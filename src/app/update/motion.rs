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

        // Item 35 (C, 2026-11-20 ruling): the canvas scrim under the
        // right-click menu. The edge is detected here, on the update side,
        // because a track started while view paints never receives a frame
        // (this module's header). Reading `open_at` directly covers every
        // open/close path — including the call sites that assign
        // `open_at = None` without going through `SelectionMenu::close`.
        // Open: anchor at 0 first, then rise, so the very first opening
        // animates instead of stepping (a fresh `set` with no prior track
        // would start at its target). Close: ease back to 0; once settled,
        // the view pushes no scrim, so no backdrop reader and no blur pass.
        let menu_open = self
            .tabs
            .get(self.active_tab)
            .map(|tab| tab.scene.selection.borrow().menu.open_at.is_some())
            .unwrap_or(false);
        if menu_open != self.menu_scrim_open {
            self.menu_scrim_open = menu_open;
            if menu_open {
                motion::set(motion::MENU_SCRIM, 0.0, 0);
                motion::set(motion::MENU_SCRIM, 1.0, motion::MENU_SCRIM_IN_MS);
            } else {
                motion::set(motion::MENU_SCRIM, 0.0, motion::MENU_SCRIM_OUT_MS);
            }
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

        // The Open button's rim reports the file load on the button that
        // started it. The measured fraction is the same `PROGRESS` track the
        // old card's bar read; this track only decides whether the rim is on
        // screen, so a cancelled or failed open fades it instead of cutting.
        motion::rise(motion::OPEN_RING, self.opening.is_some(), motion::OPEN_RING_TIME);

        // Dimension text is formatted from the style on every frame, so the
        // unit switch's multiplier has to be published here, on the one path
        // every message passes through, rather than read in the formatter.
        //
        // F3 (b): the chase track is process-global, but a unit switch belongs
        // to the document that started it. Attribute a freshly running chase
        // to the active tab (the trigger always runs on the active document),
        // and publish its value only while that tab is still active. A tab
        // switch lands the chase at 1.0 instantly — item 22's contract — so a
        // background document never inherits another drawing's multiplier and
        // switching back reads no residual. This is "the background tab does
        // not chase", not per-document independence: the multiplier itself is
        // still one global value (see entities/dimension.rs DISPLAY_SCALE_BITS).
        let active = self.active_tab;
        if self.unit_chase_tab != active {
            self.unit_chase_tab = active;
            motion::set(motion::UNIT_CHASE, 1.0, 0);
            self.unit_chase_owner = usize::MAX;
        }
        let chasing = motion::running(motion::UNIT_CHASE);
        if chasing && self.unit_chase_owner == usize::MAX {
            self.unit_chase_owner = active;
        }
        if !chasing {
            self.unit_chase_owner = usize::MAX;
        }
        let scale = unit_chase_scale(
            self.unit_chase_owner,
            active,
            chasing,
            motion::value_or(motion::UNIT_CHASE, 1.0),
        );
        crate::entities::dimension::set_display_scale(scale);
    }
}

/// F3 (b): the multiplier to publish for the active tab. A running unit chase
/// is published only for the tab that owns it; every other tab reads 1.0, and
/// a settled chase reads 1.0 everywhere. Pure, so the publish rule has one
/// source the guard exercises directly instead of a second copy in the test.
///
/// This is "the background tab does not chase", NOT per-document independence:
/// the multiplier itself is still one process-global value
/// (`entities::dimension::DISPLAY_SCALE_BITS`); the rule only decides which
/// tab's chase is allowed to drive it away from 1.0.
fn unit_chase_scale(owner: usize, active: usize, chasing: bool, value: f32) -> f32 {
    if chasing && owner == active {
        value
    } else {
        1.0
    }
}

#[cfg(test)]
mod b6_sync_tests {
    /// The paint side only: the assertions below spell the shapes they look
    /// for, so a scan that reached a test module would always pass.
    ///
    /// Every inline test module body is cut out of the source, wherever it
    /// sits: these files interleave test modules with production code
    /// (`#[cfg(test)] mod x_tests;` shims, and several inline modules that hand
    /// back to the code after them), so neither "stop at the first attribute"
    /// nor "stop at the first inline module" reads the production half.
    fn paint_side(path: &str) -> String {
        let src = std::fs::read_to_string(format!("{}{path}", env!("CARGO_MANIFEST_DIR")))
            .unwrap_or_else(|e| panic!("{path} must be readable: {e}"));
        strip_test_modules(&src)
    }

    fn strip_test_modules(src: &str) -> String {
        const MARK: &str = "\n#[cfg(test)]\nmod ";
        let bytes = src.as_bytes();
        let mut out = String::new();
        let mut cursor = 0;
        while let Some(rel) = src[cursor..].find(MARK) {
            let at = cursor + rel;
            let after = at + MARK.len();
            let header_end = src[after..]
                .find('\n')
                .map(|i| after + i)
                .unwrap_or(src.len());
            let Some(open) = src[after..header_end].rfind('{').map(|i| after + i) else {
                // `#[cfg(test)] mod x_tests;` — an external file, nothing to cut.
                cursor = after;
                continue;
            };
            out.push_str(&src[cursor..at]);
            let mut depth = 0i32;
            let mut i = open;
            while i < bytes.len() {
                match bytes[i] {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    b'"' => {
                        i += 1;
                        while i < bytes.len() {
                            if bytes[i] == b'\\' {
                                i += 2;
                                continue;
                            }
                            if bytes[i] == b'"' {
                                break;
                            }
                            i += 1;
                        }
                    }
                    b'/' if bytes.get(i + 1) == Some(&b'/') => {
                        while i < bytes.len() && bytes[i] != b'\n' {
                            i += 1;
                        }
                        continue;
                    }
                    _ => {}
                }
                i += 1;
            }
            cursor = i;
        }
        out.push_str(&src[cursor..]);
        out
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

    /// Batch D (33/34/37/38/44): each item has one trigger that starts its
    /// track and one paint that reads it. A key started but never painted
    /// leaves a track nobody reads; a key painted but never started freezes at
    /// its first sample because no message asks for the next frame.
    #[test]
    fn every_batch_d_track_has_a_trigger_and_a_paint() {
        let sites: &[(&str, &[&str])] = &[
            // 33 — the dock drop preview.
            ("/src/app/dock/update.rs", &["motion::replay", "motion::DOCK_DROP"]),
            ("/src/app/dock/view.rs", &["motion::DOCK_DROP"]),
            // 34 — the properties accordion.
            (
                "/src/app/update/mod.rs",
                &["motion::section_key(", "motion::PROP_SECTION_TIME"],
            ),
            (
                "/src/ui/properties.rs",
                &["motion::section_key(", "kit::collapse("],
            ),
            // 37 — the Open rim.
            (
                "/src/app/update/motion.rs",
                &["motion::rise(motion::OPEN_RING", "motion::OPEN_RING_TIME"],
            ),
            (
                "/src/app/view/mod.rs",
                &["motion::OPEN_RING", "motion::PROGRESS"],
            ),
            ("/src/ui/ribbon/mod.rs", &["kit::progress_ring("]),
            // 38 — the large-drawing sweep.
            (
                "/src/app/update/file.rs",
                &["motion::replay(", "motion::REVEAL", "REVEAL_MIN_ENTITIES"],
            ),
            (
                "/src/app/view/mod.rs",
                &["motion::reveal_tile(", "motion::REVEAL"],
            ),
            // 44 — the unit switch's display multiplier.
            ("/src/app/update/style.rs", &["motion::unit_chase("]),
            (
                "/src/app/commands/layerprops.rs",
                &["motion::unit_chase("],
            ),
            (
                "/src/app/update/motion.rs",
                &["set_display_scale(", "motion::UNIT_CHASE"],
            ),
        ];
        for (file, needles) in sites {
            let paint = paint_side(file);
            for needle in *needles {
                assert!(
                    paint.contains(needle),
                    "{file} no longer names {needle}; batch D's track would go dark"
                );
            }
        }
    }
}

#[cfg(test)]
mod f3_unit_chase_publish_scope {
    //! F3 (b) guard: the publish rule may drive the process-global multiplier
    //! away from 1.0 only for the tab that owns the running chase. This drives
    //! the real decision function (the single source `sync_motion` calls) and
    //! checks the publish point routes through it and lands the chase at 1.0
    //! on a tab switch, so a regression that bypasses the rule is caught here.
    use super::unit_chase_scale;

    #[test]
    fn only_the_owner_tabs_running_chase_is_published() {
        let ratio = 25.4_f32;
        // Owner active and chasing: the chase value is published.
        assert_eq!(unit_chase_scale(0, 0, true, ratio), ratio);
        // A different tab is active: never inherit the owner's multiplier.
        assert_eq!(unit_chase_scale(0, 1, true, ratio), 1.0);
        // No owner attributed (boot, or just after a switch): 1.0.
        assert_eq!(unit_chase_scale(usize::MAX, 0, true, ratio), 1.0);
        // Settled chase: 1.0 even for the owner tab.
        assert_eq!(unit_chase_scale(0, 0, false, ratio), 1.0);
        assert_eq!(unit_chase_scale(0, 1, false, ratio), 1.0);
    }

    #[test]
    fn the_publish_point_routes_through_the_rule_and_resets_on_switch() {
        let src = std::fs::read_to_string(format!(
            "{}/src/app/update/motion.rs",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("motion.rs readable");
        // Production half only: everything before the first inline test module,
        // so this scan cannot match its own assertions.
        let prod = src.split("\n#[cfg(test)]").next().unwrap_or(&src);
        assert!(
            prod.contains("unit_chase_scale("),
            "sync_motion must publish through the single-source rule"
        );
        assert!(
            prod.contains("motion::set(motion::UNIT_CHASE, 1.0, 0)"),
            "a tab switch must land the chase at 1.0 instantly (no residual)"
        );
        assert!(
            prod.contains("self.unit_chase_tab != active"),
            "the publish point must detect the tab switch"
        );
    }
}
