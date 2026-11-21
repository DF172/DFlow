//! Keyed easing registry powering chrome micro-motion.
//!
//! Update handlers push transitions in (message-driven: tab switches, tool
//! activation, dock toggles, modal opens) and style closures read interpolated
//! progress out. `Instant::now()` is sampled at trigger *and* at read, so
//! progress stays wall-clock accurate even when no frame tick has refreshed
//! the app state — the frame subscription only exists to request redraws while
//! a track is still running (`any_active`).
//!
//! Tracks are never pruned: keys are `&'static str`, so the map is bounded by
//! the number of distinct keys in the code, and a finished track keeps
//! returning its final value — which is exactly what "fade in and stay" needs.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use iced::time::{Duration, Instant};
use iced::Color;

/// Hover fade (ms).
pub const HOVER: u64 = 120;
/// Press sink and rebound (ms). Shorter than a hover so a click reads as
/// decided rather than as a second, slower animation.
pub const PRESS: u64 = 80;
/// Standard emphasis fades: tab underline, press settle, theme cross-fade.
pub const NORMAL: u64 = 200;
/// Panel slide-ins: dock columns.
pub const PANEL: u64 = 300;
/// Activation pulse: rise to full glow (ms).
pub const PULSE_RISE: u64 = 100;
/// Activation pulse: settle down to the resting micro-glow (ms).
pub const PULSE_FALL: u64 = 200;
/// Dialog fade-in (ms).
pub const MODAL: u64 = 160;
/// Dialog fade-out (ms). Shorter than the entrance on purpose: leaving must
/// read as decided, not as a second animation. Runs on the same
/// [`ease_out_quart`] curve as the entrance, played the other way.
pub const EXIT: u64 = 120;
/// How far a dialog travels toward its trigger as it arrives (px).
pub const ENTRANCE_REACH: f32 = 8.0;
/// How far a popup menu slides out of its anchor (px). Same budget as
/// [`HOVER`], so a right-click menu never feels heavier than a hover.
pub const MENU_REACH: f32 = 6.0;
/// Track every dialog fades through, entrance and exit alike.
pub const MODAL_FADE: &str = "modal:fade";
/// Track a viewport menu (context / grip / visibility) slides in on.
pub const MENU_OPEN: &str = "menu:open";
/// Track an expanded submenu's rows arrive on.
pub const MENU_SUB: &str = "menu:sub";
/// Start-page card stagger and fade (ms).
pub const STAGGER: u64 = 70;
pub const CARD: u64 = 320;
/// Resting alpha of the aurora glow on an engaged (active) control.
pub const REST_GLOW: f32 = 0.24;
/// How long a re-targeted scalar takes to close the gap. Long enough that a
/// measured counter reads as continuous, short enough that the number is never
/// visibly behind what the phase line claims.
pub const CHASE_MS: u64 = 180;
/// Period of the indeterminate sweep, in ms. One full left-to-right-to-left
/// pass; slow enough to read as waiting rather than as a progress bar.
pub const LOOP_MS: u64 = 1400;

/// Track by which the command-line autocomplete panel fades in.
pub const AUTOCOMPLETE: &str = "commandline:autocomplete";
/// Track the autocomplete's single selection highlight rides. One scalar for
/// the whole list, so moving the highlight never restarts a per-row fade.
pub const AUTOCOMPLETE_SEL: &str = "commandline:autocomplete:sel";
/// Track the open-progress fraction chases the measured value on.
pub const PROGRESS: &str = "open_progress:fraction";

/// REGEN completion fade (item 40): the canvas wears a neutral veil that
/// peaks on the first frame after the command returns and settles to
/// nothing, so a regenerated drawing fades back from grey to full colour.
pub const REGEN_FADE: &str = "regen:fade";
/// Veil fall duration (ms); the rise is 0 so the first frame reads the peak.
pub const REGEN_FADE_MS: u64 = 160;
/// Peak alpha of the completion veil over the canvas.
pub const REGEN_VEIL_ALPHA: f32 = 0.5;

/// Item 41 (2026-11-20 chairman ruling, degraded): the render-mode switch
/// veil. A true wireframe↔solid crossfade would need two flag sets and two
/// draws per object; the ruling degrades it to a single quad over the
/// switching viewport alone, in the canvas ground colour, peaking on the
/// switch frame and falling back to nothing. Per-viewport, never canvas-wide.
pub const RENDER_VEIL: &str = "render:veil";
/// Switch-veil fall duration (ms); the rise is 0 so the switch frame reads
/// the peak.
pub const RENDER_VEIL_MS: u64 = 120;
/// Peak alpha of the switch veil over its own viewport (ruling window
/// 0.25–0.4).
pub const RENDER_VEIL_ALPHA: f32 = 0.35;

/// Item 35 (2026-11-20 chairman ruling, plan C): the canvas scrim under the
/// right-click menu. The menu itself stays an opaque `popup_plate`; only the
/// canvas beneath it softens through the glass blur channel. The track is
/// edged from `sync_motion` (update side), so it always receives frames.
pub const MENU_SCRIM: &str = "menu:scrim";
/// Scrim entrance (ms); ruling window 120–160.
pub const MENU_SCRIM_IN_MS: u64 = 140;
/// Scrim exit (ms); ruling window 120–160.
pub const MENU_SCRIM_OUT_MS: u64 = 140;
/// Peak scrim opacity (ruling window 0.6–0.85), applied as the glass tint's
/// alpha and scaled by the track value.
pub const MENU_SCRIM_PEAK: f32 = 0.7;

/// Item 45 (P1, batch 5): the theme switch erase. A true cross-fade needs the
/// outgoing frame kept as a drawable texture, which iced does not have, so the
/// carrier is a single quad over the canvas in the outgoing ground colour that
/// fades out over `THEME_VEIL_MS`. Chrome and every text-bearing surface land on
/// the new theme in the switch frame itself — a ground that keeps blending under
/// living glyphs is exactly the same-colour window this batch forbids.
pub const THEME_VEIL: &str = "canvas:theme:veil";
/// Item 45: the switch window, at the item's own 200 ms (not shortened).
pub const THEME_VEIL_MS: u64 = 200;
/// Item 45: peak opacity of the outgoing ground over the canvas. Below 1.0 so
/// the drawing stays legible through the erase; above the 0.25–0.4 tint band
/// because this stands in for a cross-fade rather than a highlight.
pub const THEME_VEIL_ALPHA: f32 = 0.55;

/// Item 46 (P1, batch 5): the MATCHPROP transfer line. One replay per applied
/// transfer drives the travelling head from the source anchor to every
/// destination anchor; the line is drawn only while the track runs, so a
/// finished transfer leaves no geometry and no frames.
pub const MATCHPROP_FLOW: &str = "canvas:matchprop:flow";
/// Item 46: source→target travel time, at the item's own 120 ms.
pub const MATCHPROP_FLOW_MS: u64 = 120;

/// Item 47 (P1, batch 5): node-graph evaluation flow. The engine evaluates the
/// whole graph in one `order()` sweep and exposes no touched-node set, so this
/// carries the degraded all-edges-at-once reading rather than a per-edge
/// travelling signal.
pub const GRAPH_FLOW: &str = "graph:eval:flow";
/// Item 47: one flow beat, at the item's own 120 ms.
pub const GRAPH_FLOW_MS: u64 = 120;

// ── Viewport overlay motion ───────────────────────────────────────────────
//
// The snap marker is drawn by `ui::overlay`, but its motion is triggered from
// the update layer: the canvas half of `ui::canvas_fx` is clock-free, and a
// canvas program keeps no state between frames. One pulse key, one pair of
// position keys and one settle key cover every snap the cursor acquires.

/// Snap acquisition pulse: how far the marker swells at its peak.
pub const SNAP_PULSE_PEAK: f32 = 1.15;
/// Snap acquisition pulse: rise to the peak (ms).
pub const SNAP_PULSE_RISE: u64 = 45;
/// Snap acquisition pulse: settle back to the resting size (ms).
pub const SNAP_PULSE_FALL: u64 = 75;
/// How long the marker takes to slide between neighbouring snap candidates
/// while the cursor keeps moving along a wire (ms).
pub const SNAP_CANDIDATE: u64 = 60;
/// How long a freshly acquired snap takes to pull the marker from the pointer
/// onto the exact snapped point (ms).
pub const SNAP_CONVERGE: u64 = 80;
/// Track the snap marker's acquisition pulse rides.
pub const SNAP_PULSE: &str = "canvas:snap:pulse";
/// Tracks the marker's displayed position chases the current snap point on.
pub const SNAP_CANDIDATE_X: &str = "canvas:snap:candidate:x";
pub const SNAP_CANDIDATE_Y: &str = "canvas:snap:candidate:y";
/// Track a fresh acquisition's convergence from the pointer to the snap point.
pub const SNAP_SETTLE: &str = "canvas:snap:settle";

/// Sweep shown when ortho/polar angle constraint actually takes over the
/// cursor: a translucent sector sweeps from the free cursor angle to the
/// locked angle (item 11). Drawn only on the takeover edge, never while the
/// constraint merely stays engaged.
pub const ANGLE_SWEEP: &str = "canvas:angle:sweep";
/// Sweep duration (ms).
pub const ANGLE_SWEEP_TIME: u64 = 100;

/// Fade-in of the snap type label beside the marker (item 12). Runs on the
/// same acquisition edge as [`SNAP_PULSE`]; the label vanishes with the snap.
pub const SNAP_LABEL: &str = "canvas:snap:label";
/// Label fade-in duration (ms).
pub const SNAP_LABEL_TIME: u64 = 100;

/// One-shot ring pulse when the FIRST point of a command is committed
/// (item 13). Later points do not replay it; cancelling and restarting the
/// command clears the accepted-snap list, so the next first point replays.
pub const FIRST_POINT_PULSE: &str = "canvas:firstpoint:pulse";
/// First-point pulse duration (ms).
pub const FIRST_POINT_PULSE_TIME: u64 = 60;

/// Highlight migration when the overlap-pick candidate list preview switches
/// to another entity (item 14). RGB cross-fade: the old candidate's tint
/// decays toward its own base colour while the new one eases up to the hover
/// tint. Not a Tab cycle — the pick loop is the candidate list box.
pub const CYCLE_MIGRATE: &str = "canvas:cycle:migrate";
/// Migration cross-fade duration (ms).
pub const CYCLE_MIGRATE_TIME: u64 = 60;

/// Batch 3 item 23: a layer row's ink dims for one beat when its layer is
/// turned OFF, so the eye follows the state change instead of reading the
/// vanished geometry as lost work. The row itself stays in the tree.
pub const LAYER_OFF_TIME: u64 = 120;

/// Batch 3 item 25: an extra-transient bump on a mode pill's ink right after
/// F8 / F3 flips it. Deliberately tiny (see the consumer's blend factor):
/// these keys are hammered while drafting, so the pulse must read as a
/// confirmation, never as a flash.
pub const MODE_PULSE_TIME: u64 = 80;

/// Batch 3 item 24: the active paper-viewport border ring brightens for one
/// beat when focus enters that viewport (Message::EnterViewport).
pub const VP_RING_PULSE: &str = "chrome:vpring:pulse";
/// Viewport-ring pulse duration (ms).
pub const VP_RING_PULSE_TIME: u64 = 120;

/// Batch 3 item 26: one-shot boundary pulse when a read-only session's first
/// workspace frame lands. The faint persistent border is gated on
/// [`enabled`]; with motion reduced, the consumer shows a static 150 ms
/// wall-clock hint window instead of an eased pulse.
pub const READONLY_EDGE: &str = "chrome:readonly:edge";
/// Read-only boundary pulse duration (ms).
pub const READONLY_EDGE_TIME: u64 = 150;

/// Batch 3 item 27: the command-line card's border flashes danger for one
/// beat when a command fails. Fired from the two `push_error*` choke points
/// only — warnings are not failures and never ride this track.
pub const CMD_ERROR_PULSE: &str = "chrome:cmdline:error";
/// Command-error border pulse duration (ms).
pub const CMD_ERROR_PULSE_TIME: u64 = 150;

/// Batch 3 item 28: after an undo, the redo dropdown's top rows (the entries
/// just moved onto the redo stack) highlight for one beat. History rows have
/// no stable id, so the highlight is positional: row 0 is the last undone
/// operation, rows 0..n-1 after UndoMany(n).
pub const UNDO_HIGHLIGHT: &str = "chrome:undo:highlight";
/// Undo-highlight duration (ms).
pub const UNDO_HIGHLIGHT_TIME: u64 = 100;

/// Window↔crossing marquee cross-fade (ms). A drag that reverses direction
/// changes what the box will pick; the colour eases over to say so.
pub const SELECT_CROSSFADE: u64 = 60;
/// Blend tracks (0 = window blue, 1 = crossing green) for the box and the
/// polygon marquees.
pub const SELECT_CROSS_BOX: &str = "canvas:select:crossing:box";
pub const SELECT_CROSS_POLY: &str = "canvas:select:crossing:poly";
/// Drag-start ghost of the entity being moved: it starts at 30% opacity and
/// dissolves as the cursor carries the live geometry away, so the eye stops
/// reading it as a second copy once the drag is unmistakable (item 17).
pub const GHOST_START_ALPHA: f32 = 0.30;
/// Screen distance (px) at which the ghost has fully dissolved.
pub const GHOST_FADE_DISTANCE: f32 = 220.0;
/// Smoothing while the dissolve target moves with the cursor (ms).
pub const GHOST_FADE_TIME: u64 = 120;
/// Track the ghost's remaining-visibility factor (1 = drag start, 0 = gone).
pub const GHOST_FADE: &str = "canvas:grip:ghost:fade";

/// Remaining visibility of the drag-start ghost at a given drag distance.
/// Linear on purpose: the eye reads the outline as a fixed-opacity reference,
/// so any easing curve here would look like the ghost accelerating away.
pub fn ghost_fade_for(distance_px: f32) -> f32 {
    (1.0 - distance_px.max(0.0) / GHOST_FADE_DISTANCE).clamp(0.0, 1.0)
}

/// Property-panel field with uncommitted text: its border eases into the
/// warning colour and back, so a typed-but-not-applied value is visible at a
/// glance without an extra badge (item 21).
pub const PROP_DIRTY_TIME: u64 = 80;
pub const PROP_DIRTY: &str = "props:field:dirty";

/// Tab save-state dot: 1 = unsaved, 0 = saved. The dot fades in when the
/// document first goes dirty and dissolves when it is written, so the
/// indicator reports the document rather than blinking at it (item 22).
pub const TAB_SAVE_TIME: u64 = 120;
pub const TAB_SAVE: &str = "tab:save:dirty";

/// UCS tripod: turning to a new orientation is one short rotation of the world
/// axes, not a snap. The icon anchor stays put, so the tripod swings about the
/// origin rather than spinning around its own middle (item 31).
pub const UCS_ROT_TIME: u64 = 150;
/// Progress track 0 → 1 for the tripod rotation.
pub const UCS_ROT: &str = "ucs:tripod:rotate";

/// Association break: a dimension whose reference chain just went Broken or
/// Unresolved gets one short warning flash where its status is reported. A
/// pulse (rise→fall inside 100 ms), not a state colour, because the row keeps
/// saying "Broken reference" afterwards — the flash only marks the moment of
/// severing (item 18).
pub const ASSOC_BREAK_TIME: u64 = 100;
pub const ASSOC_BREAK: &str = "props:assoc:break";

/// Space focus (item 32): stepping into a viewport pushes the view in from
/// 1.02 to 1.0; stepping back out pulls it the other way. The track carries
/// the extra zoom fraction, 0.02 → 0, applied as a camera zoom about the
/// pane centre so the settle lands exactly on the original view.
pub const SPACE_FOCUS_TIME: u64 = 150;
pub const SPACE_FOCUS_PEAK: f32 = 0.02;
pub const SPACE_FOCUS: &str = "viewport:space:focus";

/// ViewCube orientation damping (item 5): a face/edge/corner click turns the
/// camera to the snapped orientation over this many ms instead of cutting.
/// Within the 250–350 ms brief; the house ease-out-quart curve decelerates into
/// the target so the turn reads as a settled move, not a linear spin.
pub const VIEW_ORIENT_TIME: u64 = 300;
/// Progress track 0 → 1 for the orientation turn. The update layer slerps the
/// camera quaternion between the latched start and target by this value; the
/// camera's own snap methods stay instant (mspace decodes through them).
pub const VIEW_ORIENT: &str = "viewport:orient:damp";

/// View fly-through (P2 batch 1, item 6): a 3D ZOOM EXTENTS / entity zoom
/// moves `Camera::target` and `Camera::distance` together instead of cutting.
///
/// The track carries the flight progress 0 → 1 and is read as an eased
/// timeline, the same shape item 5 uses for a ViewCube turn. It is NOT a
/// shared-registry spring, and that is deliberate: the registry's rest gates
/// are absolute (`SPRING_REST_POS` 0.05 and `SPRING_REST_VEL` 0.5), so on a 0
/// → 1 progress a spring parks on its target while still 5 % of the trip away
/// and snaps the remainder in one frame — on a ZOOM EXTENTS that last step is
/// hundreds of drawing units of zoom. Scaling the track up to buy precision
/// makes the absolute velocity gate far tighter than the spring can decay
/// (its stiffness would have to drop under 10, i.e. ≈ 0.98 s), which misses the
/// 300–450 ms brief. The eased timeline lands exactly on 1.0 at the deadline
/// instead, so the fly-through has a fixed duration, no drift, and a
/// bit-exact destination write.
pub const VIEW_FLY: &str = "view:fly:progress";
/// Fly-through duration (ms). Inside the 300–450 ms brief by construction.
pub const VIEW_FLY_TIME: u64 = 400;

/// Orbit coast (P2 batch 1, item 7): after a Shift+MMB / 3DORBIT drag the view
/// keeps turning at the release speed and decays to rest. The tracks are in
/// SCREEN PIXELS — the same ruler the drag itself uses — and the frame applies
/// them through `Camera::spin` after converting with
/// `Camera::ORBIT_RAD_PER_PX`. Radians would not work here: the registry's
/// rest gates are absolute (0.05 position, 0.5 velocity), so a radian track
/// parks 2.9° short of where it was heading and snaps that in one frame, while
/// 0.05 px is 0.014° — invisible.
pub const ORBIT_GLIDE_YAW: &str = "orbit:glide:yaw";
pub const ORBIT_GLIDE_PITCH: &str = "orbit:glide:pitch";
/// Coast stiffness (px²/s²). ω = √400 = 20 rad/s: a 200 px/s release coasts
/// 10 px ≈ 2.9° and is parked inside ≈ 9.7/ω ≈ 0.49 s — a glide long enough
/// to read as momentum, short enough not to fight the next gesture.
pub const ORBIT_GLIDE_K: f32 = 400.0;
/// Below this release speed (px/s, ≈ 3°/s at the orbit's scale) the flick is
/// hand jitter: no coast starts and the view stops exactly where the drag
/// left it.
pub const ORBIT_GLIDE_MIN: f32 = 60.0;
/// Hard cap on the coast travel (px ≈ 34° at 0.005 rad/px), so a violent
/// flick cannot spin the model past the point where the drawing is lost. The
/// release speed is clipped to `ORBIT_COAST_MAX_PX * ω` instead of shortening
/// the spring, so the decay keeps its true shape.
pub const ORBIT_COAST_MAX_PX: f32 = 120.0;

/// P2 batch 2, item 29 — out-of-range field tint. A Drafting Settings numeric
/// row outside its documented range tints its field (border + entered ink)
/// toward the theme's `warning` token. 140 ms sits inside the 120–160 ms brief.
/// The weight is moved with `chase`, which is an idempotent retarget that
/// continues from the current interpolated value — so a value that jitters
/// across the boundary keeps sliding from where it stood instead of snapping
/// back to zero and restarting. A legal field never mints a track at all.
pub const FIELD_WARN: u64 = 140;
pub const FIELD_WARN_SNAP_X: &str = "ds:warn:snapx";
pub const FIELD_WARN_SNAP_Y: &str = "ds:warn:snapy";
pub const FIELD_WARN_GRID_X: &str = "ds:warn:gridx";
pub const FIELD_WARN_GRID_Y: &str = "ds:warn:gridy";
pub const FIELD_WARN_GRID_MAJOR: &str = "ds:warn:gridmajor";

/// P2 batch 2, item 30 — grip handle hover. One track per handle, keyed
/// through [`grip_hover_key`]; only the arriving and the departing handle are
/// ever retargeted, so a multi-selection of N grips still costs two tracks per
/// transition instead of animating the whole set. 120 ms is `HOVER`, the same
/// edge the chrome buttons use — the handle is the same kind of affordance.
pub const GRIP_HOVER: u64 = HOVER;

/// Interned track key for one grip handle. `handle` is the owning entity and
/// `grip` its grip index; callers pass `"{handle}:{grip}"`.
pub fn grip_hover_key(id: &str) -> &'static str {
    interned_key("grip:", id)
}

/// P2 batch 2, item 42 — adaptive grid density cross-fade. When the adaptive
/// step jumps a power of 5 (or GRIDUNIT changes) the outgoing spacing fades out
/// while the incoming one fades in. 190 ms sits inside the 160–220 ms brief.
/// The weight is restarted (0 → 1) on every crossing, so a third density arriving
/// mid-fade replaces the old one instead of leaving three grids on screen.
pub const GRID_DENSITY: &str = "grid:density:fade";
pub const GRID_DENSITY_FADE: u64 = 190;

/// Empty-state fades: one track per surface, not per row.
pub const EMPTY_LAYERS: &str = "empty:layers";
pub const EMPTY_PROPS: &str = "empty:props";
pub const EMPTY_MATCHES: &str = "empty:matches";

/// Dock drop preview (item 33): the landing frame fades and lifts into place
/// whenever the pointer crosses into a fresh drop zone. Replayed per zone
/// change, so the wash and the ghost arrive together instead of popping.
pub const DOCK_DROP_TIME: u64 = 120;
/// How far (px) the drop ghost travels as it lands.
pub const DOCK_DROP_REACH: f32 = 8.0;
pub const DOCK_DROP: &str = "dock:drop:preview";

/// Dock landing overshoot (item 36): one underdamped spring keyed globally
/// (only one drag can land at a time). The landed panel rides it as a pure
/// visual translation; at rest the value snaps to exactly 0 and the wrapper
/// is not built, so the settled panel is pixel-identical to the plain path.
pub const DOCK_LAND: &str = "dock:land";
/// Starting displacement (px), within the 2-4 px micro-overshoot budget.
pub const DOCK_LAND_AMP: f32 = 3.0;
/// Stiffness (1/s²): with the damping below, a 3 px kick settles inside the
/// 120-180 ms landing budget (envelope decay 37.5/s, rest ≈ 167 ms).
pub const DOCK_LAND_K: f32 = 5625.0;
/// Damping = 0.5 × critical (2√k = 150): one visible overshoot (~16% of the
/// amplitude, sub-pixel after the first) then convergence, not a bounce train.
pub const DOCK_LAND_DAMP: f32 = 75.0;

/// Properties accordion (item 34): a section header opens and closes over this
/// many ms, the body's height and the scrollbar above it riding the same
/// number.
pub const PROP_SECTION_TIME: u64 = 160;

/// Open ring (item 37): the radial progress ring around the Open button fades
/// in when a load starts and out when it ends.
pub const OPEN_RING_TIME: u64 = 160;
pub const OPEN_RING: &str = "ribbon:open:ring";

/// Large-drawing reveal (item 38): tiles of the document uncover in a spatial
/// sweep. One master track drives every tile, so the whole reveal costs one
/// frame subscription rather than one per tile.
pub const REVEAL_TIME: u64 = 520;
pub const REVEAL: &str = "canvas:reveal";
/// Below this entity count a drawing is small enough that a sweep would read as
/// a glitch rather than as work finishing; those files appear at once.
pub const REVEAL_MIN_ENTITIES: usize = 20_000;

/// Unit switch (item 44): the dimension text chases the new display factor
/// instead of jumping. The track carries a multiplier that starts at
/// `old / new` and settles on 1.0, so a per-style DIMLFAC keeps its own base.
pub const UNIT_CHASE: &str = "dims:unit:chase";

#[derive(Clone, Copy)]
struct Track {
    from: f32,
    peak: f32,
    settle: f32,
    rise_ms: u64,
    fall_ms: u64,
    started: Instant,
}

impl Track {
    fn value(&self, now: Instant) -> f32 {
        let elapsed = (now - self.started).as_millis() as i64;
        if self.rise_ms == 0 {
            // A zero rise means "peak on the very first frame". `as_millis`
            // truncates a sub-millisecond read to 0, and handing back `from`
            // there would return `settle` (0 for a fresh pulse) instead of
            // the peak the caller asked for — the first frame of every
            // rise-less pulse would silently read as its own tail. Start the
            // fall curve at elapsed 0, where ease_out_quart(0) = 0 gives
            // exactly `peak`. fall_ms == 0 is the motion-reduced landing:
            // settle immediately, matching `finished`.
            if self.fall_ms == 0 || elapsed >= self.fall_ms as i64 {
                return self.settle;
            }
            let t = ease_out_quart(elapsed.max(0) as f32 / self.fall_ms as f32);
            return self.peak + (self.settle - self.peak) * t;
        }
        if elapsed <= 0 {
            return self.from;
        }
        let total = (self.rise_ms + self.fall_ms) as i64;
        if elapsed >= total {
            return self.settle;
        }
        if elapsed <= self.rise_ms as i64 {
            let t = ease_out_quart(elapsed as f32 / self.rise_ms as f32);
            self.from + (self.peak - self.from) * t
        } else {
            let t = ease_out_quart((elapsed - self.rise_ms as i64) as f32 / self.fall_ms as f32);
            self.peak + (self.settle - self.peak) * t
        }
    }

    fn finished(&self, now: Instant) -> bool {
        (now - self.started).as_millis() as i64 >= (self.rise_ms + self.fall_ms) as i64
    }
}

/// Ease-out quart: fast start, long tail. The house curve for chrome motion.
fn ease_out_quart(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let u = 1.0 - t;
    1.0 - u * u * u * u
}

thread_local! {
    static TRACKS: RefCell<HashMap<&'static str, Track>> = RefCell::new(HashMap::new());
    static HOVERED: RefCell<HashMap<&'static str, bool>> = RefCell::new(HashMap::new());
    static PRESSED: RefCell<HashMap<&'static str, bool>> = RefCell::new(HashMap::new());
    /// Latched edges for [`rise`]: an entrance that must replay on every
    /// false→true turn, not on every report of "still true".
    static RISEN: RefCell<HashMap<&'static str, bool>> = RefCell::new(HashMap::new());
    static KEY_CACHE: RefCell<HashMap<(&'static str, &'static str), &'static str>> =
        RefCell::new(HashMap::new());
    static STRING_KEY_CACHE: RefCell<HashMap<String, &'static str>> = RefCell::new(HashMap::new());
    /// `None` until the platform preference has been read. Every entry point
    /// consults it, so a caller that forgets `init_from_system` still gets the
    /// right answer instead of a silent "always animate".
    static ENABLED: Cell<Option<bool>> = const { Cell::new(None) };
}

/// Reads the platform motion preference once and latches it.
///
/// `DRAFTFLOW_MOTION=off|on` overrides the OS, which is what a screenshot pass and
/// the control server need: a deterministic terminal frame with no clock in
/// it. Called before the splash is built so the plate and the chrome behind
/// it agree on the first frame.
pub fn init_from_system() {
    let on = match std::env::var("DRAFTFLOW_MOTION").ok().as_deref().map(str::trim) {
        Some("off") | Some("0") | Some("false") => false,
        Some("on") | Some("1") | Some("true") => true,
        _ => system_animations_enabled(),
    };
    ENABLED.with(|e| e.set(Some(on)));
}

/// Whether transitions play out over time. When false every trigger lands on
/// its terminal value in the same frame.
pub fn enabled() -> bool {
    ENABLED.with(|e| match e.get() {
        Some(on) => on,
        None => {
            init_from_system();
            e.get().unwrap_or(true)
        }
    })
}

/// Overrides the gate at runtime. Turning it off settles every running track
/// immediately, so nothing is left stranded mid-fade and `any_active` stops
/// asking for frames.
pub fn set_enabled(on: bool) {
    ENABLED.with(|e| e.set(Some(on)));
    if !on {
        let now = Instant::now();
        TRACKS.with(|t| {
            for track in t.borrow_mut().values_mut() {
                let end = track.settle;
                *track = Track {
                    from: end,
                    peak: end,
                    settle: end,
                    rise_ms: 0,
                    fall_ms: 0,
                    started: now,
                };
            }
        });
        SPRINGS.with(|s| {
            for spring in s.borrow_mut().values_mut() {
                spring.pos = spring.target;
                spring.vel = 0.0;
            }
        });
    }
}

#[cfg(target_os = "windows")]
fn system_animations_enabled() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPI_GETCLIENTAREAANIMATION,
    };
    let mut on: i32 = 1;
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            &mut on as *mut i32 as *mut core::ffi::c_void,
            0,
        )
    };
    ok != 0 && on != 0
}

#[cfg(not(target_os = "windows"))]
fn system_animations_enabled() -> bool {
    true
}

/// Registry key for a widget's hover track. The formatted key is interned in a
/// thread-local cache: one small allocation per distinct id, free reads after.
pub fn hover_key(id: &'static str) -> &'static str {
    cached_key("hover:", id)
}

/// Registry key for a widget's activation-glow track.
pub fn glow_key(id: &'static str) -> &'static str {
    cached_key("glow:", id)
}

/// Registry key for a widget's press-sink track.
pub fn press_key(id: &'static str) -> &'static str {
    cached_key("press:", id)
}

/// Registry key for a control's engaged-state track. Keyed on the widget id so
/// a state bar and the plate it belongs to can never latch different values.
pub fn state_key(id: &'static str) -> &'static str {
    cached_key("state:", id)
}

/// Registry key for a widget's keyboard-focus ring track.
pub fn focus_key(id: &'static str) -> &'static str {
    cached_key("focus:", id)
}

/// Registry key for a runtime `String` tool id. Same interned-key
/// scheme, but keyed on the `String` form so callers holding an owned id do
/// not need a `&'static str` conversion.
pub fn glow_key_any(id: &str) -> &'static str {
    interned_key("glow:", id)
}

/// Registry key for a layer row's turn-off dim (batch 3 item 23). Layer names
/// are runtime `String`s from the drawing, so this is the interned twin of
/// [`state_key`], one allocation per distinct layer name.
pub fn layer_off_key(name: &str) -> &'static str {
    interned_key("layeroff:", name)
}

/// Registry key for a mode pill's one-shot toggle pulse (batch 3 item 25),
/// keyed on the same widget id as its hover / state tracks.
pub fn pulse_key(id: &'static str) -> &'static str {
    cached_key("pulse:", id)
}

/// Registry key for one properties accordion section, keyed on its title.
/// Titles are `String`s from the drawing's section list, so this is the
/// interned twin of [`state_key`].
pub fn section_key(title: &str) -> &'static str {
    interned_key("propsec:", title)
}

/// Registry key for a migrated plain-button call site, derived from
/// `#[track_caller]`'s `file:line`. One interned allocation per distinct site;
/// the map is bounded by the number of call sites, never by frames or rows.
pub fn site_key(file: &str, line: u32) -> &'static str {
    interned_key("site:", &format!("{file}:{line}"))
}

/// Decimal pool for `indexed_key`: covers every realistic tab / status index
/// with zero allocation. Anything beyond it falls back to an interned string.
const DEC: [&str; 64] = [
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
    "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29", "30", "31",
    "32", "33", "34", "35", "36", "37", "38", "39", "40", "41", "42", "43", "44", "45", "46",
    "47", "48", "49", "50", "51", "52", "53", "54", "55", "56", "57", "58", "59", "60", "61",
    "62", "63",
];

/// Registry key for an index-driven track (ribbon tabs, status capsules).
/// Small indices hit the static pool — no allocation in the hot path.
pub fn indexed_key(prefix: &'static str, index: usize) -> &'static str {
    if let Some(&dec) = DEC.get(index) {
        return cached_key(prefix, dec);
    }
    interned_key(prefix, &index.to_string())
}

fn cached_key(prefix: &'static str, id: &'static str) -> &'static str {
    KEY_CACHE.with(|c| {
        let mut cache = c.borrow_mut();
        *cache
            .entry((prefix, id))
            .or_insert_with(|| Box::leak(format!("{prefix}{id}").into_boxed_str()))
    })
}

/// The `String`-keyed twin of [`cached_key`], for ids only known at runtime.
///
/// The cache is keyed on the **finished** key, prefix included. Keying it on
/// the bare id instead — which is what this did before the channels were
/// merged — silently maps two tracks over one id onto each other, so a hover
/// reader would get the glow track back and the control would simply stop
/// reacting. Nothing can reach that today; a second `*_key_any` would.
fn interned_key(prefix: &'static str, id: &str) -> &'static str {
    let full = format!("{prefix}{id}");
    STRING_KEY_CACHE.with(|c| {
        let mut cache = c.borrow_mut();
        if let Some(&k) = cache.get(full.as_str()) {
            return k;
        }
        let leaked: &'static str = Box::leak(full.clone().into_boxed_str());
        cache.insert(full, leaked);
        leaked
    })
}

/// Starts (or smoothly redirects) `key` toward `target` over `duration_ms`.
///
/// Redirecting mid-flight continues from the current interpolated value, so
/// rapid re-triggers never jump.
pub fn set(key: &'static str, target: f32, duration_ms: u64) {
    let now = Instant::now();
    let rise_ms = if enabled() { duration_ms } else { 0 };
    TRACKS.with(|t| {
        let mut tracks = t.borrow_mut();
        let from = tracks.get(key).map(|tr| tr.value(now)).unwrap_or(target);
        tracks.insert(
            key,
            Track {
                from: if rise_ms == 0 { target } else { from },
                peak: target,
                settle: target,
                rise_ms,
                fall_ms: 0,
                started: now,
            },
        );
    });
}

/// Idempotent retarget: moves `key` toward `target` only when the settled
/// value differs.
///
/// This is what lets a state diff run on every `update` without pinning a
/// running track: [`set`] always re-anchors from the current value, so calling
/// it each frame would freeze the track wherever it had got to. A chase skips
/// that case and only fires on a real change.
///
/// A key that never ran already reads as its default at the call site, so a
/// chase to zero mints no track — boot stays silent and `any_active` does not
/// ask for frames on a state nobody changed.
pub fn chase(key: &'static str, target: f32, duration_ms: u64) {
    let settled = TRACKS.with(|t| t.borrow().get(key).map(|tr| tr.settle));
    match settled {
        Some(settle) if (settle - target).abs() < 1e-4 => {}
        Some(_) => set(key, target, duration_ms),
        None if target.abs() < 1e-4 => {}
        None => set(key, target, duration_ms),
    }
}

/// Starts the unit switch's value chase: `old_factor` → `new_factor` becomes
/// a multiplier that begins at `old / new` and settles on 1.0 over
/// [`CHASE_MS`]. Dimension text multiplies its own style's DIMLFAC by this,
/// so every style keeps its own base and only the switch itself is animated.
///
/// A no-op for equal, zero or non-finite factors: those are not a switch, and
/// minting a track for them would ask for frames on a still drawing.
pub fn unit_chase(old_factor: f64, new_factor: f64) {
    if !old_factor.is_finite()
        || !new_factor.is_finite()
        || old_factor.abs() < 1e-12
        || new_factor.abs() < 1e-12
        || (old_factor - new_factor).abs() < 1e-12
    {
        return;
    }
    let ratio = (old_factor / new_factor) as f32;
    set(UNIT_CHASE, ratio, 0);
    chase(UNIT_CHASE, 1.0, CHASE_MS);
}

/// Progress of reveal tile `index` of `tiles`, from the master sweep `m`.
///
/// The sweep travels `tiles + fade_tiles` tile-widths, and each tile fades
/// over `fade_tiles` widths, so neighbours overlap a little and the sweep
/// reads as one motion rather than a row of switches. Pure, and total: an
/// out-of-range `m` or `index` clamps instead of producing a negative alpha.
pub fn reveal_tile(m: f32, index: usize, tiles: usize) -> f32 {
    const FADE_TILES: f32 = 3.0;
    if tiles == 0 || !m.is_finite() {
        return 0.0;
    }
    let m = m.clamp(0.0, 1.0);
    let travel = tiles as f32 + FADE_TILES;
    let start = index as f32;
    ((m * travel - start) / FADE_TILES).clamp(0.0, 1.0)
}

/// Engaged progress of `key`: the track when it exists, otherwise the control's
/// own state. Reading through this is what makes the first paint correct — a
/// pill that is already on at boot draws lit instead of flashing dark.
pub fn engage(key: &'static str, lit: bool) -> f32 {
    value_or(key, if lit { 1.0 } else { 0.0 })
}

/// Triangle wave in `0..=1` with period `period_ms`, sampled at `elapsed_ms`.
/// Used by the indeterminate sweep: it must return to where it started so the
/// loop has no seam.
pub fn tri_wave(elapsed_ms: u64, period_ms: u64) -> f32 {
    if period_ms == 0 {
        return 0.0;
    }
    let t = (elapsed_ms % period_ms) as f32 / period_ms as f32;
    if t < 0.5 {
        t * 2.0
    } else {
        2.0 - t * 2.0
    }
}

/// Animates `key` from 0 to `target` over `duration_ms`, ignoring any
/// previous value. For entrances that must replay every time (modal fade-in).
pub fn replay(key: &'static str, target: f32, duration_ms: u64) {
    let now = Instant::now();
    let rise_ms = if enabled() { duration_ms } else { 0 };
    TRACKS.with(|t| {
        t.borrow_mut().insert(
            key,
            Track {
                from: if rise_ms == 0 { target } else { 0.0 },
                peak: target,
                settle: target,
                rise_ms,
                fall_ms: 0,
                started: now,
            },
        );
    });
}

/// Pulses `key` up to `peak` and back down to `settle`, in two eased phases.
pub fn pulse(key: &'static str, peak: f32, settle: f32, rise_ms: u64, fall_ms: u64) {
    let now = Instant::now();
    let (rise_ms, fall_ms) = if enabled() {
        (rise_ms, fall_ms)
    } else {
        (0, 0)
    };
    TRACKS.with(|t| {
        let mut tracks = t.borrow_mut();
        let from = tracks.get(key).map(|tr| tr.value(now)).unwrap_or(settle);
        tracks.insert(
            key,
            Track {
                from: if rise_ms == 0 && fall_ms == 0 {
                    settle
                } else {
                    from
                },
                peak,
                settle,
                rise_ms,
                fall_ms,
                started: now,
            },
        );
    });
}

/// Latched hover: animates only on a state *change*.
///
/// Style closures may call this on every layout pass; the latch makes
/// repeated calls with the same state free, and the state-change edge
/// redirects the track from its current interpolated value.
pub fn hover(key: &'static str, hovered: bool) {
    let changed = HOVERED.with(|h| {
        let mut seen = h.borrow_mut();
        let changed = seen.get(key).copied() != Some(hovered);
        if changed {
            seen.insert(key, hovered);
        }
        changed
    });
    if changed {
        set(key, if hovered { 1.0 } else { 0.0 }, HOVER);
    }
}

/// Edge-latched ramp between 0 and 1 for a state that flips on and off.
///
/// [`hover`] cannot serve a state that can flip back within one gesture: it
/// starts a fresh track from zero on every rising edge, so a value that leaves
/// and re-enters 40 ms later has to climb from zero again instead of carrying
/// on from where it stood. This keeps the latch (still free to call on every
/// layout pass, and still one track write per edge) but on the rising edge it
/// *continues* from the current value whenever a ramp is already running, and
/// only starts from zero when the handle really was at rest.
pub fn edge_ramp(key: &'static str, on: bool, duration_ms: u64) {
    let changed = HOVERED.with(|h| {
        let mut seen = h.borrow_mut();
        let changed = seen.get(key).copied() != Some(on);
        if changed {
            seen.insert(key, on);
        }
        changed
    });
    if !changed {
        return;
    }
    if on {
        if value(key) > 0.0 {
            set(key, 1.0, duration_ms);
        } else {
            replay(key, 1.0, duration_ms);
        }
    } else if value(key).abs() < 1e-4 {
        // Raised and left before a frame painted: the value is still 0, so
        // drop the track (if there is one) instead of minting a fade from 0
        // to 0. A key nobody ever raised stays absent.
        if exists(key) {
            forget(key);
        }
    } else {
        // A key nobody raised must stay absent: `chase` to zero mints nothing,
        // so a control that is legal from boot owns no track at all.
        chase(key, 0.0, duration_ms);
    }
}

/// Latched press: the sink-and-rebound twin of [`hover`].
/// The style closure runs on every layout pass and reports the same `down`
/// state dozens of times per click. Without the latch each of those calls
/// would restart the track from its current value, which pins the sink at
/// whatever it had reached and freezes it there.
pub fn press(key: &'static str, down: bool) {
    let changed = PRESSED.with(|p| {
        let mut seen = p.borrow_mut();
        let changed = seen.get(key).copied() != Some(down);
        if changed {
            seen.insert(key, down);
        }
        changed
    });
    if changed {
        set(key, if down { 1.0 } else { 0.0 }, PRESS);
    }
}

/// Edge-latched entrance: plays `key` from zero when `on` turns true, and
/// settles it when `on` turns false.
///
/// [`chase`] cannot do this job. A key nobody has run starts at its own
/// target, so chasing an empty state to 1.0 would paint it at full strength
/// instead of fading it in; and a caller that reports "still empty" on every
/// message would restart the fade through [`set`]. The latch makes the fade an
/// edge response, which is what an arrival is.
///
/// The false path goes through [`chase`], so a surface that was never empty
/// mints no track at boot.
pub fn rise(key: &'static str, on: bool, duration_ms: u64) {
    let changed = RISEN.with(|r| {
        let mut seen = r.borrow_mut();
        let changed = seen.get(key).copied() != Some(on);
        if changed {
            seen.insert(key, on);
        }
        changed
    });
    if !changed {
        return;
    }
    if on {
        replay(key, 1.0, duration_ms);
    } else {
        chase(key, 0.0, duration_ms);
    }
}

/// Interpolated value of `key`; `0.0` for tracks that never ran.
pub fn value(key: &'static str) -> f32 {
    let now = Instant::now();
    TRACKS.with(|t| t.borrow().get(key).map(|tr| tr.value(now)).unwrap_or(0.0))
}

/// Interpolated value of `key`, falling back to `default` when the track
/// never ran. Lets pre-existing chrome (docks rendered at full width at
/// boot) read "not animated yet" as "fully open" instead of "zero".
pub fn value_or(key: &'static str, default: f32) -> f32 {
    let now = Instant::now();
    TRACKS
        .with(|t| t.borrow().get(key).map(|tr| tr.value(now)).unwrap_or(default))
}

/// Whether `key` has a track at all — running or settled.
///
/// [`value_or`] cannot tell "never ran" from "ran and settled on zero", and a
/// control that seeds its first ease (the properties accordion) has to know
/// which of the two it is looking at.
pub fn exists(key: &'static str) -> bool {
    TRACKS.with(|t| t.borrow().contains_key(key))
}

/// Drops `key` outright. Unlike [`chase`], this can remove a track that was
/// minted but never painted: a fade from 0 to 0 would otherwise keep the
/// frame loop awake for a transition nobody can see.
pub fn forget(key: &'static str) {
    TRACKS.with(|t| {
        t.borrow_mut().remove(key);
    });
}

/// Whether `key` still has time left to run. A key nobody ever started is
/// settled, and so is one whose budget elapsed — both mean "safe to commit".
pub fn running(key: &'static str) -> bool {
    if !enabled() {
        return false;
    }
    let now = Instant::now();
    TRACKS.with(|t| {
        t.borrow()
            .get(key)
            .is_some_and(|tr| !tr.finished(now))
    })
}

/// Test-only snapshot of a track's firing parameters and start instant:
/// `(from, peak, settle, rise_ms, fall_ms, started)`.
///
/// Guards use this to prove "this action fired exactly this pulse" without
/// sampling `running`/`value` at an arbitrary wall-clock moment: under CPU
/// contention the triggering `update` itself can outlast a short pulse
/// window, so a settled read is legitimate and cannot distinguish "never
/// fired" from "fired and finished". Parameters plus start-instant ownership
/// can, and they also catch a wrong peak/duration that a single value sample
/// would miss.
#[cfg(test)]
pub fn track_snapshot(key: &'static str) -> Option<(f32, f32, f32, u64, u64, Instant)> {
    TRACKS.with(|t| {
        t.borrow().get(key).map(|tr| {
            (
                tr.from,
                tr.peak,
                tr.settle,
                tr.rise_ms,
                tr.fall_ms,
                tr.started,
            )
        })
    })
}

/// Where a dialog starts when it is opened from `trigger`, given the window
/// centre: [`ENTRANCE_REACH`] px toward the trigger, resolved to a unit vector
/// so a corner press and an edge press travel the same distance.
///
/// A trigger exactly at the centre has no direction to come from, and dividing
/// by a zero length would put the dialog at NaN — it gets no shift instead.
pub fn entrance_shift(trigger: iced::Point, centre: iced::Point, reach: f32) -> iced::Vector {
    let dx = trigger.x - centre.x;
    let dy = trigger.y - centre.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-3 || !len.is_finite() {
        return iced::Vector::ZERO;
    }
    iced::Vector::new(dx / len * reach, dy / len * reach)
}

/// Slide for a popup menu at `progress` (1 = landed, 0 = just opened).
///
/// The menu is pinned at its anchor, so moving it *toward* the anchor as it
/// fades in makes it read as growing out of the click point instead of
/// appearing somewhere and drifting.
pub fn menu_slide(progress: f32) -> iced::Vector {
    let left = 1.0 - progress.clamp(0.0, 1.0);
    iced::Vector::new(-MENU_REACH * left, -MENU_REACH * left)
}

/// Number of live tracks. Gates read this to prove a per-frame draw path
/// registers no new key (a per-row key would grow it without bound).
pub fn track_count() -> usize {
    TRACKS.with(|t| t.borrow().len())
}

/// True while any track is still running; drives the frame subscription.
pub fn any_active() -> bool {
    if !enabled() {
        return false;
    }
    let now = Instant::now();
    let tracks = TRACKS.with(|t| t.borrow().values().any(|tr| !tr.finished(now)));
    tracks || springs_active()
}

/// Linear color interpolation, clamped; the shared easing target for hover
/// tints that blend between two theme colors instead of fading alpha in.
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

/// Plays the start-page entrance: three staggered card fades.
pub fn start_intro() {
    let now = Instant::now();
    set("start:card0", 1.0, CARD);
    let still = !enabled();
    TRACKS.with(|t| {
        let mut tracks = t.borrow_mut();
        for (key, delay) in [("start:card1", STAGGER), ("start:card2", STAGGER * 2)] {
            tracks.insert(
                key,
                Track {
                    from: if still { 1.0 } else { 0.0 },
                    peak: 1.0,
                    settle: 1.0,
                    rise_ms: if still { 0 } else { CARD },
                    fall_ms: 0,
                    started: if still {
                        now
                    } else {
                        now + Duration::from_millis(delay)
                    },
                },
            );
        }
    });
}

/// How many distinct entrance delays a panel may ask for. Derived from the
/// panel budget rather than typed: slot `n` starts `n * STAGGER` late inside
/// the `PANEL` window, and the last slot still finishes exactly at `PANEL`.
pub const STAGGER_SLOTS: usize = (PANEL / STAGGER) as usize - 1;

/// Delay before entrance slot `slot` starts, clamped to the last slot.
pub fn stagger_delay_ms(slot: usize) -> u64 {
    slot.min(STAGGER_SLOTS.saturating_sub(1)) as u64 * STAGGER
}

/// Entrance alpha for `slot` at panel `progress` (0..=1).
///
/// Pure by construction: the caller owns one panel-level track and derives
/// every visible row's alpha from it, so a 2000-row list still runs exactly
/// one track and no row ever owns a motion key.
pub fn stagger_value(progress: f32, slot: usize) -> f32 {
    if !enabled() {
        return 1.0;
    }
    let delay = stagger_delay_ms(slot) as f32 / PANEL as f32;
    let rise = 1.0 - delay;
    if rise <= 1e-3 {
        return 1.0;
    }
    ease_out_quart(((progress - delay) / rise).clamp(0.0, 1.0))
}

/// Starts `key` on the same staggered curve as [`stagger_value`], for call
/// sites whose style closure needs a `&'static str` track (the kit's `fade`).
///
/// The two representations must not drift, so `stagger_track_matches_value`
/// samples both across the whole budget and compares.
pub fn replay_stagger(key: &'static str, slot: usize) {
    let delay = stagger_delay_ms(slot);
    let now = Instant::now();
    let still = !enabled();
    TRACKS.with(|t| {
        t.borrow_mut().insert(
            key,
            Track {
                from: if still { 1.0 } else { 0.0 },
                peak: 1.0,
                settle: 1.0,
                rise_ms: if still { 0 } else { PANEL - delay },
                fall_ms: 0,
                started: if still {
                    now
                } else {
                    now + Duration::from_millis(delay)
                },
            },
        );
    });
}

/// Replays a panel's row entrance: one track per slot, none per row.
///
/// `keys` is the panel's static slot table ([`crate::ui::dock::entrance_keys`]).
/// A 2000-row list costs `keys.len()` tracks however many rows it draws, which
/// `entrance_costs_one_track_per_slot_not_per_row` pins.
pub fn replay_entrance(keys: &'static [&'static str]) {
    for (slot, key) in keys.iter().enumerate() {
        replay_stagger(key, slot);
    }
}

/// Entrance alpha for `row` of a panel whose slot table is `keys`.
///
/// Rows cycle through the slots, so the fade reads as a wave down the visible
/// list. Full alpha until the panel replays: a panel nobody opened is drawn at
/// once, never blank.
pub fn entrance_value(keys: &'static [&'static str], row: usize) -> f32 {
    if keys.is_empty() {
        return 1.0;
    }
    value_or(keys[row % keys.len()], 1.0)
}

/// The gate is process-wide, so every test module that flips it shares this
/// lock. Declared once at file scope so `b4_tests` and `gate_tests` cannot
/// interleave with each other.
#[cfg(test)]
static GATE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Serialises tests that flip the motion gate from other modules too.
#[cfg(test)]
pub(crate) fn test_gate_lock() -> std::sync::MutexGuard<'static, ()> {
    GATE_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Physics springs (velocity-driven), distinct from the easing `Track` above.
//
// A `Track` interpolates from A to B over a fixed duration; a spring integrates
// acceleration from displacement and velocity, so it can be *kicked* with a
// release velocity and overshoot naturally. Both live in this module so
// `any_active` remains the single place that decides whether frames are needed.
// ---------------------------------------------------------------------------

/// Stiffness (1/s²) for viewport-class motion: ~60 Hz critical damping.
pub const SPRING_K: f32 = 900.0;
/// Spring is at rest when it is within this distance of its target (px).
pub const SPRING_REST_POS: f32 = 0.05;
/// ...and moving slower than this (px/s).
pub const SPRING_REST_VEL: f32 = 0.5;
/// Frames folded into the release-velocity EMA.
pub const VELOCITY_FRAMES: usize = 5;

/// Critical damping for stiffness `k`: the fastest decay without overshoot.
pub fn critical_damping(k: f32) -> f32 {
    2.0 * k.sqrt()
}

#[derive(Clone, Copy)]
struct Spring {
    pos: f32,
    vel: f32,
    target: f32,
    stiffness: f32,
    damping: f32,
}

impl Spring {
    fn at_rest(&self) -> bool {
        (self.pos - self.target).abs() <= SPRING_REST_POS && self.vel.abs() <= SPRING_REST_VEL
    }

    /// Half-implicit Euler, sub-stepped at 240 Hz so a long frame (or a hitch)
    /// stays stable; `dt` itself is clamped so a suspended window cannot
    /// integrate a huge jump.
    fn tick(&mut self, dt: f32) {
        let mut left = dt.clamp(0.0, 0.1);
        const MAX_STEP: f32 = 1.0 / 240.0;
        while left > 0.0 {
            let h = left.min(MAX_STEP);
            let accel = self.stiffness * (self.target - self.pos) - self.damping * self.vel;
            self.vel += accel * h;
            self.pos += self.vel * h;
            left -= h;
        }
        if self.at_rest() {
            self.pos = self.target;
            self.vel = 0.0;
        }
    }
}

thread_local! {
    static SPRINGS: RefCell<HashMap<&'static str, Spring>> = RefCell::new(HashMap::new());
}

/// Points `key` at `target`, keeping the current position and velocity.
///
/// Retargeting mid-flight is the normal case for a wheel zoom: a second notch
/// must add velocity, not restart from zero. With the gate off the spring lands
/// on the target in the same frame.
pub fn spring_to(key: &'static str, target: f32, stiffness: f32, damping: f32) {
    let on = enabled();
    SPRINGS.with(|s| {
        let mut springs = s.borrow_mut();
        let spring = springs.entry(key).or_insert(Spring {
            // A key that never ran has no history to preserve: it starts on
            // its target instead of flying in from the origin.
            pos: target,
            vel: 0.0,
            target,
            stiffness,
            damping,
        });
        spring.target = target;
        spring.stiffness = stiffness;
        spring.damping = damping;
        if !on {
            spring.pos = target;
            spring.vel = 0.0;
        }
    });
}

/// Starts (or re-seats) `key` with an explicit position and velocity.
/// This is the entry point that injects the release velocity `v0`.
pub fn spring_kick(key: &'static str, pos: f32, vel: f32, target: f32, stiffness: f32, damping: f32) {
    let on = enabled();
    SPRINGS.with(|s| {
        s.borrow_mut().insert(
            key,
            Spring {
                pos: if on { pos } else { target },
                vel: if on { vel } else { 0.0 },
                target,
                stiffness,
                damping,
            },
        );
    });
}

/// Whether motion is enabled right now (system preference or a runtime
/// override). Inertial gestures use this to land immediately when it is off.
pub fn animations_enabled() -> bool {
    enabled()
}

/// Current target of a spring, or `None` if the key never ran.
pub fn spring_target(key: &'static str) -> Option<f32> {
    SPRINGS.with(|s| s.borrow().get(key).map(|sp| sp.target))
}

/// Park a spring on `pos` with no velocity. Harmless on a key that is not
/// moving; it is how a new gesture cancels the previous glide.
pub fn spring_stop(key: &'static str, pos: f32) {
    SPRINGS.with(|s| {
        s.borrow_mut().insert(
            key,
            Spring {
                pos,
                vel: 0.0,
                target: pos,
                stiffness: SPRING_K,
                damping: critical_damping(SPRING_K),
            },
        );
    });
}

/// Launch an inertial glide: keep moving at `v0`, then decelerate to rest.
///
/// The target is placed `v0 / ω` ahead of `pos`, so a critically damped spring
/// coasts exactly that distance and lands without a bounce. This is what makes
/// a released drag keep travelling instead of stopping on the last sample.
pub fn glide(key: &'static str, pos: f32, v0: f32, stiffness: f32) {
    let omega = stiffness.max(1.0).sqrt();
    let target = pos + v0 / omega;
    spring_kick(key, pos, v0, target, stiffness, critical_damping(stiffness));
}

thread_local! {
    /// Wall-clock of the previous frame tick, so `tick_springs` can derive
    /// `dt` without every caller carrying a frame clock.
    static LAST_SPRING_TICK: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// Advances every spring to frame time `now`. Cheap; called from `MotionTick`.
///
/// The first tick after a quiet spell only records the clock: there is no
/// previous sample to difference against, so nothing moves that frame.
pub fn tick_springs(now: Instant) {
    let dt = LAST_SPRING_TICK.with(|c| {
        let prev = c.get();
        c.set(Some(now));
        prev.map(|p| (now - p).as_secs_f32()).unwrap_or(0.0)
    });
    if dt <= 0.0 {
        return;
    }
    tick_springs_dt(dt);
}

/// Integrates every spring by `dt` seconds. `tick_springs` clamps through
/// [`Spring::tick`]; tests drive this directly.
pub fn tick_springs_dt(dt: f32) {
    SPRINGS.with(|s| {
        for spring in s.borrow_mut().values_mut() {
            spring.tick(dt);
        }
    });
}

/// Current position of a spring, or `None` if the key never ran.
pub fn spring_at(key: &'static str) -> Option<f32> {
    SPRINGS.with(|s| s.borrow().get(key).map(|sp| sp.pos))
}

/// Current position, falling back to `default` for an unknown key.
pub fn spring_value(key: &'static str, default: f32) -> f32 {
    spring_at(key).unwrap_or(default)
}

/// Current velocity (px/s), used by tests and by reversal heuristics.
pub fn spring_velocity(key: &'static str) -> f32 {
    SPRINGS.with(|s| s.borrow().get(key).map(|sp| sp.vel).unwrap_or(0.0))
}

/// Whether the spring is still moving (not at rest).
pub fn spring_active(key: &'static str) -> bool {
    SPRINGS.with(|s| s.borrow().get(key).is_some_and(|sp| !sp.at_rest()))
}

/// Rest-state of the whole spring registry, for `any_active`.
fn springs_active() -> bool {
    SPRINGS.with(|s| s.borrow().values().any(|sp| !sp.at_rest()))
}

/// Exponential moving average of the last [`VELOCITY_FRAMES`] pointer samples.
///
/// `push` is fed from pointer moves; `velocity()` is read once at release so a
/// flick carries its speed into the spring or the pan glide.
#[derive(Clone, Copy, Default)]
pub struct VelocityEstimator {
    velocity: f32,
    last_x: Option<f32>,
    last_t: Option<Instant>,
}

impl VelocityEstimator {
    /// Forget every sample (call on gesture start).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Feed one pointer sample. Non-positive `dt` is ignored so a coalesced
    /// event cannot divide by zero.
    pub fn push(&mut self, now: Instant, x: f32) {
        let alpha = 2.0 / (VELOCITY_FRAMES as f32 + 1.0);
        if let (Some(prev_x), Some(prev_t)) = (self.last_x, self.last_t) {
            let dt = (now - prev_t).as_secs_f32();
            if dt > 0.0 {
                let instant = (x - prev_x) / dt;
                self.velocity = if self.velocity == 0.0 {
                    instant
                } else {
                    alpha * instant + (1.0 - alpha) * self.velocity
                };
            }
        }
        self.last_x = Some(x);
        self.last_t = Some(now);
    }

    /// Estimated velocity in px/s at the last sample.
    pub fn velocity(&self) -> f32 {
        self.velocity
    }
}

#[cfg(test)]
mod spring_tests {
    use super::GATE_LOCK;
    use super::*;
    use iced::time::{Duration, Instant};

    fn tick_ms(ms: f32) {
        tick_springs_dt(ms / 1000.0);
    }

    #[test]
    fn critical_spring_reaches_target_and_stops_asking_for_frames() {
        let _g = GATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_enabled(true);
        spring_kick(
            "spring:critical",
            0.0,
            0.0,
            100.0,
            SPRING_K,
            critical_damping(SPRING_K),
        );
        assert!(any_active(), "a kicked spring must ask for frames");
        for _ in 0..240 {
            tick_ms(16.0);
        }
        assert_eq!(spring_value("spring:critical", -1.0), 100.0);
        assert!(!spring_active("spring:critical"));
    }

    #[test]
    fn underdamped_spring_overshoots_then_settles() {
        let _g = GATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_enabled(true);
        spring_kick("spring:bouncy", 0.0, 0.0, 100.0, SPRING_K, 0.45 * critical_damping(SPRING_K));
        let mut peak: f32 = 0.0;
        for _ in 0..240 {
            tick_ms(16.0);
            peak = peak.max(spring_value("spring:bouncy", 0.0));
        }
        assert!(peak > 100.5, "underdamped spring must overshoot, peak={peak}");
        assert_eq!(spring_value("spring:bouncy", -1.0), 100.0);
    }

    #[test]
    fn injected_velocity_carries_through_a_retarget() {
        let _g = GATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_enabled(true);
        spring_kick("spring:kick", 0.0, 500.0, 100.0, SPRING_K, critical_damping(SPRING_K));
        tick_ms(16.0);
        let after_one = spring_value("spring:kick", 0.0);
        assert!(after_one > 5.0, "a 500 px/s kick must move on the first frame");
        spring_to("spring:kick", 200.0, SPRING_K, critical_damping(SPRING_K));
        assert!(
            spring_velocity("spring:kick") > 0.0,
            "retarget must keep the existing velocity"
        );
        for _ in 0..300 {
            tick_ms(16.0);
        }
        assert_eq!(spring_value("spring:kick", -1.0), 200.0);
    }

    #[test]
    fn gate_off_lands_springs_in_the_same_frame() {
        let _g = GATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_enabled(true);
        spring_kick("spring:gate", 0.0, 30.0, 50.0, SPRING_K, critical_damping(SPRING_K));
        set_enabled(false);
        assert_eq!(spring_value("spring:gate", -1.0), 50.0);
        assert!(spring_velocity("spring:gate").abs() < f32::EPSILON);
        assert!(!any_active());
        set_enabled(true);
    }

    /// Item 36: the dock landing spring starts at the 3 px amplitude,
    /// overshoots past zero exactly once in a visible way, settles inside
    /// the 120-180 ms budget and snaps to precisely zero (no residual).
    #[test]
    fn dock_land_spring_overshoots_once_and_lands_at_zero() {
        let _g = GATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_enabled(true);
        spring_kick(DOCK_LAND, DOCK_LAND_AMP, 0.0, 0.0, DOCK_LAND_K, DOCK_LAND_DAMP);
        assert_eq!(spring_value(DOCK_LAND, -1.0), DOCK_LAND_AMP);
        assert!(any_active(), "a kicked landing spring must ask for frames");
        let mut min = f32::MAX;
        let mut rest_ms = None;
        for step in 1..=60 {
            tick_ms(16.0);
            min = min.min(spring_value(DOCK_LAND, 0.0));
            if !spring_active(DOCK_LAND) {
                rest_ms = Some(step * 16);
                break;
            }
        }
        assert!(
            min < -0.2,
            "underdamped landing must overshoot past zero, min={min}"
        );
        let rest = rest_ms.expect("the landing spring must settle within 960 ms");
        assert!(
            (120..=180).contains(&rest),
            "landing settle {rest} ms outside the 120-180 ms budget"
        );
        assert_eq!(
            spring_value(DOCK_LAND, -1.0),
            0.0,
            "rest must snap exactly to zero, no residual offset"
        );
        assert!(!spring_active(DOCK_LAND));
        assert!(!any_active(), "a settled landing spring must stop asking for frames");
    }

    /// Item 36: with motion disabled the landing kick lands at the target
    /// (zero) in the same frame and never asks for frames.
    #[test]
    fn dock_land_gate_off_lands_in_the_same_frame() {
        let _g = GATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_enabled(false);
        spring_kick(DOCK_LAND, DOCK_LAND_AMP, 0.0, 0.0, DOCK_LAND_K, DOCK_LAND_DAMP);
        assert_eq!(spring_value(DOCK_LAND, -1.0), 0.0);
        assert!(!spring_active(DOCK_LAND));
        assert!(!any_active());
        set_enabled(true);
    }

    #[test]
    fn velocity_estimator_converges_to_constant_speed() {
        let mut est = VelocityEstimator::default();
        let t0 = Instant::now();
        // 400 px/s at 16 ms steps.
        for n in 0..12 {
            est.push(t0 + Duration::from_millis(16 * n), n as f32 * 6.4);
        }
        assert!(
            (est.velocity() - 400.0).abs() < 20.0,
            "EMA must converge to 400 px/s, got {}",
            est.velocity()
        );
        est.reset();
        assert_eq!(est.velocity(), 0.0);
    }
}


#[cfg(test)]
mod gate_tests {
    use super::*;
    use super::GATE_LOCK;

    #[test]
    fn gate_off_lands_on_the_terminal_value_in_the_same_frame() {
        let _g = GATE_LOCK.lock().unwrap();
        set_enabled(false);
        set("gate-test:set", 1.0, PANEL);
        assert_eq!(value("gate-test:set"), 1.0);
        replay("gate-test:replay", 0.75, CARD);
        assert_eq!(value("gate-test:replay"), 0.75);
        pulse("gate-test:pulse", 1.0, REST_GLOW, PULSE_RISE, PULSE_FALL);
        assert_eq!(value("gate-test:pulse"), REST_GLOW);
        set_enabled(true);
    }

    #[test]
    fn gate_off_stops_asking_for_frames() {
        let _g = GATE_LOCK.lock().unwrap();
        set_enabled(true);
        set("gate-test:running", 1.0, PANEL);
        assert!(any_active());
        set_enabled(false);
        assert!(!any_active());
        assert_eq!(value("gate-test:running"), 1.0);
        set_enabled(true);
    }

    #[test]
    fn turning_the_gate_back_on_does_not_jump_a_settled_track() {
        let _g = GATE_LOCK.lock().unwrap();
        set_enabled(false);
        set("gate-test:resume", 1.0, PANEL);
        set_enabled(true);
        assert_eq!(value("gate-test:resume"), 1.0);
        set("gate-test:resume", 0.0, HOVER);
        assert!(value("gate-test:resume") > 0.0);
        set_enabled(true);
    }

    #[test]
    fn press_only_fires_on_a_state_change() {
        let _g = GATE_LOCK.lock().unwrap();
        set_enabled(true);
        let key = press_key("gate-test-button");
        press(key, true);
        // The style closure reports the same state on every layout pass. If
        // each report restarted the track, the sink would be pinned wherever
        // it had got to and never arrive. 5 × 30ms outlasts HOVER (120ms), so
        // a restarting latch fails this and a working one passes it.
        for _ in 0..5 {
            std::thread::sleep(Duration::from_millis(30));
            press(key, true);
        }
        assert_eq!(value(key), 1.0, "repeated reports must not restart the sink");
        press(key, false);
        std::thread::sleep(Duration::from_millis(30));
        assert!(value(key) < 1.0, "release must start the rebound");
    }

    #[test]
    fn start_intro_with_the_gate_off_shows_all_three_cards() {
        let _g = GATE_LOCK.lock().unwrap();
        set_enabled(false);
        start_intro();
        for key in ["start:card0", "start:card1", "start:card2"] {
            assert_eq!(value(key), 1.0, "{key} must be fully drawn");
        }
        set_enabled(true);
    }

    #[test]
    fn stagger_ladder_fits_the_panel_budget() {
        let _g = GATE_LOCK.lock().unwrap();
        set_enabled(true);
        assert!(STAGGER_SLOTS >= 2, "one slot is not a stagger");
        for slot in 0..STAGGER_SLOTS {
            let delay = stagger_delay_ms(slot);
            assert!(delay < PANEL / 2, "slot {slot} starts too late");
            assert_eq!(stagger_value(0.0, slot), 0.0);
            assert_eq!(stagger_value(1.0, slot), 1.0);
        }
        let last = stagger_delay_ms(STAGGER_SLOTS - 1);
        assert_eq!(
            stagger_delay_ms(usize::MAX),
            last,
            "past the last slot clamps"
        );
    }

    #[test]
    fn stagger_wave_is_monotonic_in_the_slot() {
        let _g = GATE_LOCK.lock().unwrap();
        set_enabled(true);
        let mut prev = f32::INFINITY;
        for slot in 0..STAGGER_SLOTS {
            let v = stagger_value(0.5, slot);
            assert!(
                v <= prev + 1e-6,
                "slot {slot} arrived before its predecessor"
            );
            prev = v;
        }
    }

    #[test]
    fn stagger_gate_off_lands_immediately() {
        let _g = GATE_LOCK.lock().unwrap();
        set_enabled(false);
        for slot in 0..STAGGER_SLOTS {
            assert_eq!(stagger_value(0.0, slot), 1.0);
        }
        set_enabled(true);
    }

    #[test]
    fn stagger_track_matches_value() {
        let _g = GATE_LOCK.lock().unwrap();
        set_enabled(true);
        let now = Instant::now();
        for slot in 0..STAGGER_SLOTS {
            let delay = stagger_delay_ms(slot);
            let tr = Track {
                from: 0.0,
                peak: 1.0,
                settle: 1.0,
                rise_ms: PANEL - delay,
                fall_ms: 0,
                started: now + Duration::from_millis(delay),
            };
            for t in (0..=PANEL).step_by(5) {
                let track = tr.value(now + Duration::from_millis(t));
                let value = stagger_value(t as f32 / PANEL as f32, slot);
                assert!(
                    (track - value).abs() < 1e-4,
                    "slot {slot} at {t} ms: track {track} vs value {value}"
                );
            }
        }
    }

    /// The whole point of the slot table: rows read a track, they never own
    /// one. A per-row key would make this grow with the row count.
    #[test]
    fn entrance_costs_one_track_per_slot_not_per_row() {
        let _g = GATE_LOCK.lock().unwrap();
        set_enabled(true);
        let keys: &'static [&'static str] =
            &["gate:entrance:0", "gate:entrance:1", "gate:entrance:2"];
        let before = track_count();
        replay_entrance(keys);
        let after = track_count();
        assert!(
            after - before <= keys.len(),
            "entrance minted {} tracks for {} slots",
            after - before,
            keys.len()
        );
        for row in 0..2000 {
            let v = entrance_value(keys, row);
            assert!((0.0..=1.0).contains(&v), "row {row} alpha {v} is off-scale");
        }
        assert_eq!(
            track_count(),
            after,
            "drawing 2000 rows must not mint a motion key"
        );
        set_enabled(true);
    }

    /// A panel that was never opened is drawn at once — the entrance is an
    /// arrival, not a precondition for seeing the rows.
    #[test]
    fn entrance_is_full_alpha_before_it_replays() {
        let _g = GATE_LOCK.lock().unwrap();
        set_enabled(true);
        let keys: &'static [&'static str] = &["gate:entrance:unopened:0"];
        assert_eq!(entrance_value(keys, 0), 1.0);
        assert_eq!(entrance_value(keys, 1999), 1.0);
        assert_eq!(entrance_value(&[], 7), 1.0);
    }

    /// Slot 0 leads slot 2 through the whole budget, which is what makes the
    /// fade read as a wave rather than a flash.
    #[test]
    fn entrance_wave_leads_with_the_first_slot() {
        let _g = GATE_LOCK.lock().unwrap();
        set_enabled(true);
        let keys: &'static [&'static str] =
            &["gate:entrance:wave:0", "gate:entrance:wave:1", "gate:entrance:wave:2"];
        replay_entrance(keys);
        let now = Instant::now();
        for t in (0..PANEL).step_by(10) {
            let at = now + Duration::from_millis(t as u64);
            let slot = |i: usize| {
                TRACKS.with(|tr| tr.borrow().get(keys[i]).map(|k| k.value(at)).unwrap_or(1.0))
            };
            assert!(
                slot(0) + 1e-4 >= slot(2),
                "at {t} ms slot 2 ({}) overtook slot 0 ({})",
                slot(2),
                slot(0)
            );
        }
    }
}

/// B4: dialogs leave faster than they arrive, and popups share the hover
/// budget. The numbers are the contract, so they are pinned rather than
/// described.
#[cfg(test)]
mod b4_tests {
    use super::*;
    use super::GATE_LOCK;

    #[test]
    fn the_exit_is_faster_than_the_entrance() {
        assert_eq!(MODAL, 160, "the entrance budget is part of the batch contract");
        assert_eq!(EXIT, 120, "the exit budget is part of the batch contract");
        assert!(EXIT < MODAL, "a dialog must leave faster than it arrives");
    }

    #[test]
    fn a_track_reports_running_until_it_lands() {
        let _g = GATE_LOCK.lock().unwrap();
        set_enabled(true);
        assert!(!running("b4:running:never-started"));
        replay("b4:running:live", 1.0, EXIT);
        assert!(running("b4:running:live"), "a fresh replay owns frames");
        set("b4:running:live", 0.0, 0);
        assert!(!running("b4:running:live"), "a landed track owns nothing");
    }

    /// The whole point of the exit gate: with motion off the track lands in
    /// the same frame, so the caller must commit without waiting for a tick.
    #[test]
    fn the_disabled_gate_lands_the_exit_at_once() {
        let _g = GATE_LOCK.lock().unwrap();
        set_enabled(false);
        set(MODAL_FADE, 1.0, MODAL);
        set(MODAL_FADE, 0.0, EXIT);
        assert_eq!(value(MODAL_FADE), 0.0);
        assert!(!running(MODAL_FADE), "nothing may wait for frames that never come");
        set_enabled(true);
    }

    #[test]
    fn a_dialog_enters_from_the_pointer_side() {
        let centre = iced::Point::new(640.0, 400.0);
        let low_right = entrance_shift(iced::Point::new(1200.0, 780.0), centre, ENTRANCE_REACH);
        assert!(low_right.x > 0.0 && low_right.y > 0.0, "starts toward the click");
        let len = low_right.x.hypot(low_right.y);
        assert!(
            (len - ENTRANCE_REACH).abs() < 1e-3,
            "every trigger is the same 8 px away, got {len}"
        );
        let up_left = entrance_shift(iced::Point::new(40.0, 20.0), centre, ENTRANCE_REACH);
        assert!(up_left.x < 0.0 && up_left.y < 0.0);
    }

    #[test]
    fn a_dialog_opened_at_the_centre_has_no_shift() {
        let centre = iced::Point::new(640.0, 400.0);
        let shift = entrance_shift(centre, centre, ENTRANCE_REACH);
        assert_eq!(shift, iced::Vector::ZERO, "no direction to come from");
        assert!(shift.x.is_finite() && shift.y.is_finite(), "must not be NaN");
    }

    #[test]
    fn the_entrance_shift_decays_to_zero() {
        let shift = iced::Vector::new(6.0, -4.0);
        assert_eq!(shift * (1.0 - 1.0), iced::Vector::ZERO);
        let half = shift * (1.0 - 0.5);
        assert!((half.x - 3.0).abs() < 1e-6 && (half.y + 2.0).abs() < 1e-6);
    }

    #[test]
    fn a_menu_slides_out_of_its_anchor() {
        assert_eq!(menu_slide(1.0), iced::Vector::ZERO, "a landed menu is on its anchor");
        let start = menu_slide(0.0);
        assert_eq!(start, iced::Vector::new(-MENU_REACH, -MENU_REACH));
        let mid = menu_slide(0.5);
        assert!(mid.x > start.x && mid.x < 0.0, "the slide closes monotonically");
        assert_eq!(menu_slide(-1.0), start, "out-of-range progress clamps");
    }

    #[test]
    fn the_menu_budget_matches_the_hover() {
        assert_eq!(MENU_REACH, 6.0);
        assert_eq!(HOVER, 120, "a menu may not outlast a hover");
    }
}

#[cfg(test)]
mod b5b6_tests {
    use super::*;

    /// A chase is a retarget, not an entrance: re-reporting the same target
    /// must leave a running rise alone. `replay` is what starts one from zero,
    /// so that is how the test gets a track it can interrupt.
    #[test]
    fn chase_is_idempotent_for_an_unchanged_target() {
        let _g = GATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_enabled(true);
        replay("b56:chase-idem", 1.0, NORMAL);
        for _ in 0..7 {
            std::thread::sleep(Duration::from_millis(30));
            chase("b56:chase-idem", 1.0, NORMAL);
        }
        assert_eq!(
            value("b56:chase-idem"),
            1.0,
            "210 ms of repeated chases must let the 200 ms rise land; a chase that \
             re-anchored every 30 ms would still be in flight"
        );
    }

    #[test]
    fn chase_redirects_from_the_current_value() {
        let _g = GATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_enabled(true);
        replay("b56:chase-redirect", 1.0, PANEL);
        std::thread::sleep(Duration::from_millis(90));
        chase("b56:chase-redirect", 0.0, NORMAL);
        let now = value("b56:chase-redirect");
        assert!(
            now > 0.05 && now < 0.99,
            "a redirect starts where the track stood, got {now}"
        );
    }

    #[test]
    fn a_chase_to_zero_never_mints_a_track() {
        let _g = GATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_enabled(true);
        let before = track_count();
        chase("b56:chase-zero", 0.0, NORMAL);
        assert_eq!(track_count(), before, "boot must stay silent");
    }

    #[test]
    fn the_press_budget_is_eighty_ms() {
        assert_eq!(PRESS, 80, "the approved press budget");
        assert!(PRESS < HOVER, "a press resolves faster than a hover");
    }

    #[test]
    fn a_click_lands_well_inside_the_hover_window() {
        let _g = GATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_enabled(true);
        let key = press_key("b56-press-budget");
        // Land the key on zero first: a press on a key nobody ran would start
        // from its own target and prove nothing about the budget.
        set(key, 0.0, 0);
        press(key, true);
        std::thread::sleep(Duration::from_millis(90));
        assert_eq!(
            value(key),
            1.0,
            "90 ms outlasts the 80 ms sink; a 120 ms budget would still be mid-flight"
        );
        press(key, false);
        std::thread::sleep(Duration::from_millis(90));
        assert_eq!(value(key), 0.0, "the rebound uses the same budget");
    }

    #[test]
    fn the_triangle_wave_has_no_seam() {
        assert_eq!(tri_wave(0, LOOP_MS), 0.0);
        assert_eq!(tri_wave(LOOP_MS, LOOP_MS), 0.0);
        assert!((tri_wave(LOOP_MS / 2, LOOP_MS) - 1.0).abs() < 1e-6);
        assert!((tri_wave(LOOP_MS / 4, LOOP_MS) - 0.5).abs() < 1e-6);
        assert_eq!(tri_wave(123, 0), 0.0, "a zero period must not divide by zero");
    }

    #[test]
    fn an_engage_reads_lit_before_its_track_ever_runs() {
        assert_eq!(engage("b56:engage-boot", true), 1.0);
        assert_eq!(engage("b56:engage-boot", false), 0.0);
    }

    /// `rise` is the empty state's entrance: it must replay from zero on the
    /// false→true edge, not step to the target the way `chase` does on a key
    /// nobody ran.
    #[test]
    fn a_rising_edge_replays_from_zero() {
        let _g = GATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_enabled(true);
        rise("b56:rise-edge", true, NORMAL);
        assert_eq!(value("b56:rise-edge"), 0.0, "an entrance starts at zero");
        std::thread::sleep(Duration::from_millis(230));
        assert_eq!(
            value("b56:rise-edge"),
            1.0,
            "230 ms outlasts the 200 ms budget"
        );
    }

    /// Reporting the same lit state again must not restart the fade: the sync
    /// runs on every message, and a restart would pin the row at its first
    /// sample for as long as the app keeps receiving input.
    #[test]
    fn a_repeated_rise_is_free() {
        let _g = GATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_enabled(true);
        rise("b56:rise-latch", true, NORMAL);
        std::thread::sleep(Duration::from_millis(230));
        for _ in 0..5 {
            rise("b56:rise-latch", true, NORMAL);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(value("b56:rise-latch"), 1.0, "the latch must hold");
    }

    /// The falling edge and the boot case both go through `chase`, so a state
    /// that was never shown mints no track and the frame loop stays asleep.
    #[test]
    fn a_falling_rise_mints_nothing_at_boot() {
        let _g = GATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_enabled(true);
        let before = track_count();
        rise("b56:rise-dark-boot", false, NORMAL);
        assert_eq!(track_count(), before, "a dark empty state is silent");
    }

    #[test]
    fn a_settled_engage_outranks_the_caller_state() {
        let _g = GATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_enabled(false);
        chase("b56:engage-settled", 1.0, NORMAL);
        assert_eq!(
            engage("b56:engage-settled", false),
            1.0,
            "the track is the truth once it exists"
        );
        chase("b56:engage-settled", 0.0, NORMAL);
        assert_eq!(engage("b56:engage-settled", true), 0.0);
        set_enabled(true);
    }
}

/// Batch D (items 33/34/37/38/44): each surface's track must ask for frames,
/// settle where it says it will, and be started by a named trigger.
#[cfg(test)]
mod batch_d_tests {
    use super::*;
    use iced::time::Duration;

    fn settle_wait(ms: u64) {
        std::thread::sleep(Duration::from_millis(ms));
    }

    /// Item 33: a docket drag that acquires a drop target replays the preview
    /// fade. A track nobody asked frames for would freeze at its first sample.
    #[test]
    fn the_dock_drop_preview_asks_for_frames_and_settles() {
        let _g = GATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_enabled(true);
        replay(DOCK_DROP, 1.0, DOCK_DROP_TIME);
        assert!(running(DOCK_DROP), "the preview fade must start running");
        assert!(any_active(), "the preview fade must ask for frames");
        assert!(value(DOCK_DROP) < 1.0, "the fade must start before its end");
        settle_wait(DOCK_DROP_TIME as u64 + 40);
        assert_eq!(value(DOCK_DROP), 1.0);
        assert!(!running(DOCK_DROP), "the fade must stop asking for frames");
    }

    /// Item 34: the accordion eases between the two heights. The first toggle
    /// of a session has no track yet, so the handler seeds the ease from the
    /// section's own state before chasing — `exists` is what tells the two
    /// apart.
    #[test]
    fn a_section_toggle_eases_and_registers_for_frames() {
        let _g = GATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_enabled(true);
        let key = section_key("Batch D Section");
        assert!(!exists(key), "a section nobody touched has no track");
        set(key, 1.0, 0);
        chase(key, 0.0, PROP_SECTION_TIME);
        assert!(exists(key));
        assert!(running(key), "the squeeze must start running");
        assert!(any_active(), "the squeeze must ask for frames");
        settle_wait(PROP_SECTION_TIME as u64 + 40);
        assert_eq!(value(key), 0.0, "the section must reach its closed height");
        assert!(!running(key));
    }

    /// Item 37: the Open rim fades with the load, and the fill it wraps is the
    /// same eased counter the modal bar rode.
    #[test]
    fn the_open_ring_rises_on_the_open_edge_and_asks_for_frames() {
        let _g = GATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_enabled(true);
        rise(OPEN_RING, true, OPEN_RING_TIME);
        assert!(running(OPEN_RING), "the rim must fade in with the load");
        assert!(any_active(), "the rim fade must ask for frames");
        assert!(value(OPEN_RING) < 1.0);
        settle_wait(OPEN_RING_TIME as u64 + 40);
        assert_eq!(value_or(OPEN_RING, 0.0), 1.0);
        rise(OPEN_RING, false, OPEN_RING_TIME);
        settle_wait(OPEN_RING_TIME as u64 + 40);
        assert_eq!(value(OPEN_RING), 0.0, "a finished load must drop the rim");
    }

    /// Item 38: the reveal is one master track over 24 tiles. Every tile must
    /// be veiled at zero, uncovered at one, and uncovered in staggered order
    /// along the grid, so the sweep reads as travel rather than a blink.
    #[test]
    fn the_reveal_sweep_uncovers_in_order_and_asks_for_frames() {
        let _g = GATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_enabled(true);
        let tiles = 24;
        for index in 0..tiles {
            assert_eq!(
                reveal_tile(0.0, index, tiles),
                0.0,
                "tile {index} must be veiled before the sweep starts"
            );
            assert_eq!(
                reveal_tile(1.0, index, tiles),
                1.0,
                "tile {index} must be fully open once the sweep ends"
            );
        }
        assert!(
            reveal_tile(0.5, 0, tiles) > reveal_tile(0.5, tiles - 1, tiles),
            "the sweep must reach the first tile before the last"
        );
        assert_eq!(reveal_tile(0.5, 0, 0), 0.0, "zero tiles uncover nothing");
        assert_eq!(reveal_tile(f32::NAN, 0, tiles), 0.0, "a NaN phase is inert");

        replay(REVEAL, 1.0, REVEAL_TIME);
        assert!(running(REVEAL), "the sweep must start running");
        assert!(any_active(), "the sweep must ask for frames");
        settle_wait(REVEAL_TIME as u64 + 40);
        assert_eq!(value(REVEAL), 1.0);
        assert!(!running(REVEAL), "the sweep must stop asking for frames");
    }

    /// Item 44: the unit switch eases the display multiplier from the old
    /// ratio to 1.0, so the dimension text chases the new unit instead of
    /// jumping. A same-unit switch must mint nothing.
    #[test]
    fn a_unit_switch_chases_the_ratio_and_registers_for_frames() {
        let _g = GATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_enabled(true);
        unit_chase(1.0, 25.4);
        assert!(running(UNIT_CHASE), "the unit chase must start running");
        assert!(any_active(), "the unit chase must ask for frames");
        let start = value(UNIT_CHASE);
        assert!(
            start < 0.1,
            "the chase must start near the old/new ratio (1/25.4), got {start}"
        );
        settle_wait(CHASE_MS as u64 + 40);
        assert!((value(UNIT_CHASE) - 1.0).abs() < 1e-4, "the chase must land on 1.0");
        assert!(!running(UNIT_CHASE));

        let before = track_count();
        unit_chase(25.4, 25.4);
        assert_eq!(track_count(), before, "a no-op switch must not restart the chase");
    }
}

#[cfg(test)]
mod p2b2_motion_tests {
    use super::*;
    use iced::time::Duration;
    use std::thread::sleep;

    /// Sample a snapshot's track at `at_ms` after its start instant, so a
    /// budget can be checked without waiting on the wall clock.
    fn ramp_at(
        snapshot: (f32, f32, f32, u64, u64, Instant),
        at_ms: u64,
    ) -> f32 {
        let (from, peak, settle, rise_ms, fall_ms, started) = snapshot;
        Track {
            from,
            peak,
            settle,
            rise_ms,
            fall_ms,
            started,
        }
        .value(started + Duration::from_millis(at_ms))
    }

    /// Item 29 — an out-of-range value eases in over `FIELD_WARN` (inside the
    /// 120–160 ms budget) and eases back out over the same budget. Neither
    /// edge jumps: the track starts at 0 for the entry and at the value it had
    /// reached for the recovery.
    #[test]
    fn field_warn_enters_and_recovers_in_budget() {
        let _g = test_gate_lock();
        set_enabled(true);
        assert!(
            (120..=160).contains(&FIELD_WARN),
            "FIELD_WARN={FIELD_WARN} ms is outside the 120-160 ms budget"
        );
        let key = "p2b2:warn:budget";
        edge_ramp(key, true, FIELD_WARN);
        let enter = track_snapshot(key).expect("an illegal value must own a warn track");
        assert_eq!(
            (enter.0, enter.1, enter.2),
            (0.0, 1.0, 1.0),
            "warn ramp endpoints are not 0 -> 1"
        );
        assert_eq!(enter.3, FIELD_WARN, "warn entry budget moved");
        assert_eq!(enter.4, 0, "the warn ramp must have no fall leg");
        assert_eq!(ramp_at(enter, 0), 0.0, "warn entry must not jump");
        let mid = ramp_at(enter, FIELD_WARN / 2);
        assert!(
            mid > 0.0 && mid < 1.0,
            "warn entry must ease, got {mid} at half budget"
        );
        assert_eq!(ramp_at(enter, FIELD_WARN), 1.0);

        sleep(Duration::from_millis(FIELD_WARN));
        assert!((value(key) - 1.0).abs() < 1e-6, "entry did not land on 1.0");
        edge_ramp(key, false, FIELD_WARN);
        let recover = track_snapshot(key).expect("recovery must keep the track");
        assert_eq!(
            recover.0, 1.0,
            "recovery must leave from where the entry landed"
        );
        assert_eq!(recover.1, 0.0, "recovery target is not 0");
        assert_eq!(recover.3, FIELD_WARN, "recovery budget moved");
        assert_eq!(ramp_at(recover, 0), 1.0, "recovery must not jump");
        assert_eq!(ramp_at(recover, FIELD_WARN), 0.0);
    }

    /// Item 29 — a legal↔illegal flicker 40 ms apart carries on from the current
    /// value instead of restarting, so the rail never resets and the two rails
    /// cannot add up.
    #[test]
    fn field_warn_dither_continues_from_current_value() {
        let _g = test_gate_lock();
        set_enabled(true);
        let key = "p2b2:warn:dither";
        edge_ramp(key, true, FIELD_WARN);
        sleep(Duration::from_millis(40));
        let mid = value(key);
        assert!(mid > 0.05, "the warn ramp did not move in 40 ms: {mid}");
        assert!(mid < 1.0, "the warn ramp finished inside 40 ms: {mid}");

        edge_ramp(key, false, FIELD_WARN);
        let down = track_snapshot(key).expect("recovery track");
        assert_eq!(down.1, 0.0);
        assert!(
            (down.0 - mid).abs() < 0.2,
            "recovery jumped: leaves from {} but the rail stood at {mid}",
            down.0
        );
        sleep(Duration::from_millis(40));
        let low = value(key);
        edge_ramp(key, true, FIELD_WARN);
        let again = track_snapshot(key).expect("second entry must keep the track");
        assert_eq!(again.1, 1.0);
        assert!(
            again.0 > 0.0 && (again.0 - low).abs() < 0.2,
            "second entry restarted at {} but the rail stood at {low}",
            again.0
        );
        sleep(Duration::from_millis(FIELD_WARN + 20));
        assert!(
            (value(key) - 1.0).abs() < 1e-6,
            "the dithered ramp must still land on 1.0"
        );
    }

    /// Item 30 — one grip owns one 120 ms hover track. A resting handle mints
    /// nothing, and a 40 ms in→out→in scan re-anchors that same track rather
    /// than rebuilding it, then lands on 1.0.
    #[test]
    fn grip_hover_scan_keeps_one_track() {
        let _g = test_gate_lock();
        set_enabled(true);
        assert_eq!(GRIP_HOVER, HOVER, "grip hover must share the 120 ms hover budget");
        let key = grip_hover_key("p2b2:grip:scan");

        let before = track_count();
        for _ in 0..4 {
            edge_ramp(key, false, GRIP_HOVER);
        }
        assert_eq!(
            track_count(),
            before,
            "a resting grip minted a hover track on every layout pass"
        );
        assert!(!exists(key), "a resting grip must own no track");

        edge_ramp(key, true, GRIP_HOVER);
        let enter = track_snapshot(key).expect("a hovered grip must own a track");
        assert_eq!((enter.0, enter.1), (0.0, 1.0));
        assert_eq!(enter.3, HOVER, "grip hover entry budget moved");

        sleep(Duration::from_millis(40));
        edge_ramp(key, false, GRIP_HOVER);
        sleep(Duration::from_millis(40));
        let low = value(key);
        edge_ramp(key, true, GRIP_HOVER);
        assert_eq!(
            track_count(),
            before + 1,
            "a 40 ms in->out->in retired and rebuilt the hover track"
        );
        let again = track_snapshot(key).expect("the scan must keep the track");
        assert_eq!(again.1, 1.0);
        assert!(
            again.0 > 0.0 && (again.0 - low).abs() < 0.2,
            "re-entry restarted at {} but the mark stood at {low}",
            again.0
        );
        sleep(Duration::from_millis(GRIP_HOVER + 20));
        assert!(
            (value(key) - 1.0).abs() < 1e-6,
            "the scanned hover must land on 1.0"
        );
        assert!(!running(key), "a landed hover must stop asking for frames");
    }

    /// Item 42 — a still grid owns no fade track: the canvas reads the default
    /// 1.0 and asks for no frames.
    #[test]
    fn still_grid_reads_full_ink_without_a_track() {
        let _g = test_gate_lock();
        set_enabled(true);
        assert!(
            (160..=220).contains(&GRID_DENSITY_FADE),
            "GRID_DENSITY_FADE={GRID_DENSITY_FADE} ms is outside the 160-220 ms budget"
        );
        assert!(!exists(GRID_DENSITY), "a still grid owned a fade track");
        assert_eq!(value_or(GRID_DENSITY, 1.0), 1.0, "rest ink is not full");
        assert!(!running(GRID_DENSITY), "a still grid is asking for frames");
    }
}

