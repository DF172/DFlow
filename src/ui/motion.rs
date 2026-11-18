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
/// Empty-state fades: one track per surface, not per row.
pub const EMPTY_LAYERS: &str = "empty:layers";
pub const EMPTY_PROPS: &str = "empty:props";
pub const EMPTY_MATCHES: &str = "empty:matches";

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

/// Latched press: the sink-and-rebound twin of [`hover`].
///
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
    TRACKS.with(|t| t.borrow().values().any(|tr| !tr.finished(now)))
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
