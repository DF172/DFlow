//! The update-side half of the interface's state motion.
//!
//! iced evaluates `subscription()` at the end of `update()`, *before* `view()`
//! (vendor/iced/winit/src/lib.rs). A track started while the view is being
//! built is therefore never seen by [`crate::ui::motion::any_active`], no frame
//! is requested, and the value freezes at its first sample. So every track
//! whose target is app state — the status-bar pills' engagement, the command
//! line's autocomplete and its sliding highlight — is retargeted here, from the
//! same values the paint reads. `view` only reads tracks.

use super::DraftFlowCAD;
use crate::ui::motion;
use crate::ui::statusbar::{LIT_PILLS, StatusLights};

impl DraftFlowCAD {
    /// Whether the command line may show its suggestion popup at all: no grip
    /// is collecting input, and either no command is active or that command
    /// publishes its own contextual suggestions.
    pub(in crate::app) fn autocomplete_allowed(&self) -> bool {
        let Some(tab) = self.tabs.get(self.active_tab) else {
            return false;
        };
        self.grip_pending.is_none()
            && (tab.active_cmd.is_none() || self.command_line.has_contextual_suggestions())
    }

    /// The lit rung of every status-bar pill, from the same fields `view`
    /// reads. Both the bar's paint and [`Self::sync_motion`] take this one
    /// value, so a plate and the track it eases on cannot disagree.
    pub(in crate::app) fn status_lights(&self) -> StatusLights {
        let Some(tab) = self.tabs.get(self.active_tab) else {
            return StatusLights::default();
        };
        StatusLights {
            ortho: self.ortho_mode,
            lwt: tab.scene.document.header.lineweight_display,
            polar: self.polar_mode,
            dyn_input: self.dyn_input,
            otrack: self.snapper.otrack_enabled,
            osnap: self.snapper.is_active() || self.snapper.snap_enabled,
            space: tab.scene.active_viewport.is_some(),
            anno_visibility: tab.scene.annotation_all_visible(),
            anno_auto_add: self.annotation_auto_scale > 0,
            vp_scale_sync: tab.scene.viewport_annotation_scale_synced(),
            transparency: tab.scene.transparency_display,
            isolate: tab.scene.is_isolation_active(),
            quick_props: self.quick_properties,
            filter: tab.scene.selection_filter_active(),
            sel_cycle: self.selection_cycling,
            clean_screen: self.clean_screen,
        }
    }

    /// Retarget every state-driven ease to where the app is now.
    ///
    /// Idempotent by construction: [`motion::chase`] runs nothing when the
    /// track already sits on the target, so this can be called on every
    /// message without minting tracks or restarting an ease that is in flight.
    pub(in crate::app) fn sync_motion(&mut self) {
        let lights = self.status_lights();
        for &id in LIT_PILLS {
            let target = if lights.lit(id) { 1.0 } else { 0.0 };
            motion::chase(motion::state_key(id), target, motion::HOVER);
        }

        let allowed = self.autocomplete_allowed();
        let showing = self.command_line.autocomplete_showing(allowed);
        motion::chase(
            motion::AUTOCOMPLETE,
            if showing { 1.0 } else { 0.0 },
            motion::HOVER,
        );
        if showing {
            let cursor = self.command_line.autocomplete_cursor.unwrap_or(0);
            motion::chase(motion::AUTOCOMPLETE_SEL, cursor as f32, motion::HOVER);
        }

        // Empty states rise on the edge only: a table that was never empty
        // must not fade on boot, and one that empties again replays the 120 ms
        // entrance. The three surfaces read the same fields their paint does.
        let (layers_empty, props_empty) = match self.tabs.get(self.active_tab) {
            Some(tab) => (tab.layers.is_empty(), tab.properties.sections.is_empty()),
            None => (false, false),
        };
        motion::rise(motion::EMPTY_LAYERS, layers_empty, motion::HOVER);
        motion::rise(motion::EMPTY_PROPS, props_empty, motion::HOVER);
        let no_match = self.command_line.no_match_showing(allowed);
        motion::rise(motion::EMPTY_MATCHES, no_match, motion::HOVER);
    }
}

#[cfg(test)]
mod b6_sync_tests {
    /// The paint side only: the assertions below spell the shapes they look
    /// for, so a scan that reached a test module would always pass.
    fn paint_side(path: &str) -> String {
        let src = std::fs::read_to_string(format!("{}{path}", env!("CARGO_MANIFEST_DIR")))
            .unwrap_or_else(|e| panic!("{path} must be readable: {e}"));
        src.split("\n#[cfg(test)]").next().unwrap_or(&src).to_string()
    }

    /// Every empty state is painted and eased under the same key. A key
    /// painted but never risen steps in instead of fading; a key risen but
    /// never painted leaves a track nobody reads.
    #[test]
    fn every_empty_state_is_painted_and_risen_under_one_key() {
        let sync = paint_side("/src/app/update/motion.rs");
        for (file, key) in [
            ("/src/ui/window/layers.rs", "EMPTY_LAYERS"),
            ("/src/ui/properties.rs", "EMPTY_PROPS"),
            ("/src/ui/command_line.rs", "EMPTY_MATCHES"),
        ] {
            let paint = paint_side(file);
            assert!(
                paint.contains(&format!("motion::{key}")),
                "{file} no longer paints {key}"
            );
            assert!(
                sync.contains(&format!("motion::rise(motion::{key}")),
                "the update-side sync does not rise {key}, so its fade never gets a frame"
            );
        }
        assert!(
            sync.contains("tab.layers.is_empty()"),
            "the layers empty state must read the same table the paint reads"
        );
        assert!(
            sync.contains("tab.properties.sections.is_empty()"),
            "the properties empty state must read the same sections the paint reads"
        );
        assert!(
            sync.contains("no_match_showing(allowed)"),
            "the command-line no-match card must read the popup's own showing rule"
        );
    }
}
