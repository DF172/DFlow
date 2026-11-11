//! The launch screen: one dark sheet, drawn once, gone in 1.48 s.
//!
//! This is the A plate from `D:\tmp\splash-b284\candA.html` — the drawing
//! frame, the 1–4 / A–D partition, the title block, the revision and layer
//! tables, the graphic scale, the dot field, the baseline, the wordmark — not
//! a re-interpretation of it. The geometry below quotes the sample's own
//! numbers in baseline-canvas units (1280×800, `s = 1`); everything scales by
//! the single variable described in `12-adapt.md` §1.1.
//!
//! Three constraints shaped the code:
//!
//! * **iced's canvas has no letter-spacing.** The sample sets `-0.03em` on the
//!   wordmark, `0.5em` on `CAD`, `0.14em` on sheet labels and `0.16em` on the
//!   meta row. `canvas::Text` cannot express any of them, and drawing a
//!   wordmark glyph by glyph would lose its kerning pairs. So the wordmark and
//!   the small labels are set solid (0–3 % wider than the sample), while `CAD`
//!   — where 0.5 em is structural, not decorative — is drawn one letter at a
//!   time with the advance added by hand.
//! * **No per-layer opacity.** Every colour is premultiplied by its own alpha
//!   when it is emitted; there is no group fade in the renderer.
//! * **The timeline must be visible before the window exists.** The clock
//!   starts on the first frame tick, not at construction, so the entrance never
//!   begins while the compositor is still creating the window.
//!
//! The failure state and its `MessageBoxW` hand-off were cut from the design on
//! 2026-11-05 (the backend probe has a GL fallback, so the splash never has a
//! verdict to report). What remains is one state: starting, then ready.

use std::cell::Cell;
use std::rc::Rc;

use iced::advanced::text::{
    Alignment as TextAlign, Ellipsis, LineHeight, Paragraph as _, Shaping, Text as TextSpec,
    Wrapping,
};
use iced::alignment::Vertical;
use iced::widget::canvas;
use iced::{mouse, time::Instant, Color, Element, Fill, Font, Pixels, Point, Rectangle, Size};

use crate::ui::style::tokens;

/// The design axis the entrance was drawn on: the last element (`dim`, 260 ms
/// after 1020 ms) lands at 1280 ms, and `12-adapt.md` §8 caps the plate at
/// 1.3 s. Every delay and duration below is still written in these
/// milliseconds; [`INTRO_MS`] only stretches the axis.
const TIMELINE_MS: f32 = 1300.0;
/// How long the plate actually takes on screen. The design's 1.3 s read as
/// rushed twice in the hand-off (once at 1.0×, still rushed at 1.38×), so the
/// whole axis is played at `INTRO_MS / TIMELINE_MS` (about 0.46×) speed —
/// every element keeps its order, relative delay and curve, it just moves
/// slower.
const INTRO_MS: f32 = 2800.0;
/// The ready hand-off. 180 ms in the design; slightly longer here so it pairs
/// with the stretched entrance.
const FADE_MS: f32 = 220.0;
/// What the entrance becomes when Windows' "animation effects" is off: the
/// finished plate, held still long enough to register, then a cut instead of a
/// fade. The design calls for "skip the entrance, keep the terminal state"; a
/// zero-length hold would skip the plate entirely and look like a missing
/// frame.
const QUIET_HOLD_MS: f32 = 420.0;
/// The quiet hand-off: the hold is unchanged, but its last 120 ms is a fade
/// instead of a cut. A cut over the freshly built workspace reads as a missing
/// frame on a 144 Hz panel.
const QUIET_FADE_MS: f32 = 120.0;
/// The last quarter-second of the entrance says "ready" instead of whatever
/// was true mid-entrance: the plate is about to hand off and the line is the
/// user's last signal that the launch finished rather than hung.
const READY_LEAD_MS: f32 = 250.0;

/// What the status line says. Every variant is a Fluent attribute under the
/// `splash` message, so the plate never invents a string of its own — and a
/// missing locale falls back through the loader rather than showing English.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SplashStatus {
    #[default]
    Starting,
    /// The probe fell back off the preferred backend.
    GpuFallback,
    /// No probe produced a hardware adapter; the compatibility renderer is up.
    GpuCompat,
    /// The settings file was unusable and defaults were applied.
    SettingsFault,
    /// The previous launch never cleared its crash sentinel.
    SentinelFault,
    /// A drawing is being opened; the value is its `OPEN_PHASE_*` stage.
    Phase(u8),
    /// No hardware-accelerated backend survived probing. This is the plate's
    /// failure notice: static, non-blocking, and never upgraded to "ready".
    Failed,
    Ready,
}

impl SplashStatus {
    /// Which notice wins when several are true at once. The plate has one
    /// line, so the most consequential fault is the one the user reads.
    pub fn priority(self) -> u8 {
        match self {
            SplashStatus::Starting | SplashStatus::Phase(_) | SplashStatus::Ready => 0,
            SplashStatus::GpuCompat => 1,
            SplashStatus::GpuFallback => 2,
            SplashStatus::SentinelFault => 3,
            SplashStatus::SettingsFault => 4,
            SplashStatus::Failed => 5,
        }
    }

    fn text(self) -> String {
        match self {
            SplashStatus::Starting => crate::tr!("splash", "starting"),
            SplashStatus::GpuFallback => crate::tr!("splash", "gpu-fallback"),
            SplashStatus::GpuCompat => crate::tr!("splash", "gpu-compat"),
            SplashStatus::SettingsFault => crate::tr!("splash", "settings-fault"),
            SplashStatus::SentinelFault => crate::tr!("splash", "sentinel-fault"),
            SplashStatus::Phase(phase) => match phase {
                crate::app::OPEN_PHASE_PARSING => crate::tr!("splash", "phase-parsing"),
                crate::app::OPEN_PHASE_XREF => crate::tr!("splash", "phase-references"),
                crate::app::OPEN_PHASE_CACHING => crate::tr!("splash", "phase-derived-caches"),
                crate::app::OPEN_PHASE_FINALIZING => crate::tr!("splash", "phase-finalizing"),
                _ => crate::tr!("splash", "phase-reading"),
            },
            SplashStatus::Failed => crate::tr!("splash", "startup-failed"),
            SplashStatus::Ready => crate::tr!("splash", "ready"),
        }
    }
}

/// The entrance curves from `candA.html`, one per animation class.
mod ease {
    /// `cubic-bezier(.3,.8,.4,1)` — draw, tick, axis, cap.
    pub const OUT_QUART: (f32, f32, f32, f32) = (0.3, 0.8, 0.4, 1.0);
    /// `cubic-bezier(.16,.84,.3,1)` — the two wordmark halves rising.
    pub const RISE: (f32, f32, f32, f32) = (0.16, 0.84, 0.3, 1.0);
    /// `cubic-bezier(.2,.8,.3,1)` — the three CAD letters.
    pub const CAD: (f32, f32, f32, f32) = (0.2, 0.8, 0.3, 1.0);
}

/// One shared clock. The canvas program is built on the first view pass and
/// reused by iced afterwards, so the *only* thing that reaches the later
/// frames is state behind a shared cell — the program field itself is never
/// replaced. An `Rc` rather than a borrow keeps the program valid for the few
/// milliseconds between `splash = None` and the widget tree being diffed away.
struct Clock {
    /// Elapsed entrance time, in milliseconds, clamped to `INTRO_MS`.
    t: Cell<f32>,
    /// Unclamped wall-clock time since the origin. `t` is design time and is
    /// pinned to `INTRO_MS` in quiet mode; the hand-off needs the real elapsed
    /// value to know where inside the hold it is.
    elapsed: Cell<f32>,
}

/// How this launch decided to play.
enum Play {
    /// Full entrance, then the 180 ms fade.
    Animated,
    /// Reduced motion: terminal plate, no entrance, cut away.
    Quiet,
    /// `DRAFTFLOW_SPLASH_STILL` — freeze the plate at one entrance time and hold it
    /// for screenshots. `starting` / `final` hold the terminal plate; a bare
    /// number holds that many milliseconds, which is the only way to capture a
    /// mid-motion frame without racing the clock.
    Held(f32),
}

pub struct Splash {
    clock: Rc<Clock>,
    play: Play,
    /// First frame tick. Starting the clock at construction would begin the
    /// entrance while the compositor is still mapping the window.
    origin: Option<Instant>,
    /// Whether this launch has a usable graphics backend. The plate keeps its
    /// geometry for the whole entrance, so this is set once during boot.
    verdict: SplashVerdict,
}

/// How the graphics probe ended, as far as the plate is concerned.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SplashVerdict {
    /// A hardware backend (or this launch's explicit choice) won.
    #[default]
    Ok,
    /// No hardware backend survived probing. The plate still plays in full;
    /// it only swaps its status line and its ending, and the graphics
    /// warning dialog takes over the exits after the hand-off.
    ProbesFailed,
}

impl Splash {
    /// `None` when the splash is switched off (`DRAFTFLOW_SPLASH_OFF`), which is what
    /// the control server and a screenshot pass want: no 1.5 s of canvas
    /// between them and the window they are inspecting.
    pub fn new() -> Option<Self> {
        if std::env::var_os("DRAFTFLOW_SPLASH_OFF").is_some() {
            return None;
        }
        let still = std::env::var("DRAFTFLOW_SPLASH_STILL").ok();
        let play = match still.as_deref().map(str::trim) {
            Some("starting") | Some("final") => Play::Held(INTRO_MS),
            Some(ms) => match ms.parse::<f32>() {
                Ok(t) => Play::Held(t.clamp(0.0, INTRO_MS)),
                Err(_) if crate::ui::motion::enabled() => Play::Animated,
                Err(_) => Play::Quiet,
            },
            None if crate::ui::motion::enabled() => Play::Animated,
            None => Play::Quiet,
        };
        Some(Self {
            clock: Rc::new(Clock {
                t: Cell::new(0.0),
                elapsed: Cell::new(0.0),
            }),
            play,
            origin: None,
            verdict: SplashVerdict::Ok,
        })
    }

    /// Record the graphics verdict once, during boot. Only `ProbesFailed`
    /// changes the plate; a healthy launch never calls this.
    pub fn set_verdict(&mut self, verdict: SplashVerdict) {
        self.verdict = verdict;
    }

    /// Whether this launch has no usable graphics backend.
    pub fn verdict(&self) -> SplashVerdict {
        self.verdict
    }

    /// Move the origin to `now` and rewind the plate. The hand-off calls this
    /// when it lifts the cloak: frame ticks can start before the window is
    /// shown, and starting the entrance at t=0 on the first *visible* frame is
    /// what makes the launch read as one continuous take.
    pub fn restart(&mut self, now: Instant) {
        self.origin = Some(now);
        self.clock.t.set(0.0);
        self.clock.elapsed.set(0.0);
    }

    /// Advance the clock. `false` means the plate is finished and the app
    /// should drop it and paint the start page.
    pub fn advance(&mut self, now: Instant) -> bool {
        let origin = *self.origin.get_or_insert(now);
        let elapsed = origin.elapsed().as_secs_f32() * 1000.0;
        self.clock.elapsed.set(elapsed);
        match self.play {
            Play::Held(t) => {
                self.clock.t.set(t);
                true
            }
            Play::Quiet => {
                // The entrance is skipped but the plate is not: hold the
                // terminal frame, then fade out inside the same hold.
                self.clock.t.set(INTRO_MS);
                elapsed < QUIET_HOLD_MS
            }
            Play::Animated => {
                self.clock.t.set(elapsed.min(INTRO_MS + FADE_MS));
                elapsed < INTRO_MS + FADE_MS
            }
        }
    }

    /// The whole window. `ui_scale` and the locale are read once, on the first
    /// pass, because nothing in the app can change them while the plate is up.
    pub fn view(&self, ui_scale: u16, status: SplashStatus) -> Element<'_, crate::app::Message> {
        let status = self.status_at(status);
        let view = SplashView {
            clock: self.clock.clone(),
            ui_scale,
            locale: crate::i18n::active_language_tag(),
            status: status.text(),
            failed: self.verdict == SplashVerdict::ProbesFailed,
            alpha: plate_alpha(&self.play, self.clock.elapsed.get()),
            metrics: Cell::new(None),
        };
        canvas(view).width(Fill).height(Fill).into()
    }

    /// Whether the workspace must be built underneath the plate this frame.
    /// True for the whole fade, not just its last frame: the workspace is laid
    /// out and painted behind the plate, so the first frame the user sees it
    /// is already warm and the ground dissolves onto real content.
    pub fn wants_workspace_below(&self) -> bool {
        match self.play {
            Play::Held(_) => false,
            Play::Animated => self.clock.t.get() >= INTRO_MS,
            Play::Quiet => self.clock.elapsed.get() >= QUIET_HOLD_MS - QUIET_FADE_MS,
        }
    }

    /// What the plate says at this instant. The last [`READY_LEAD_MS`] of the
    /// entrance upgrades a passive notice to "ready": the plate is about to
    /// hand off, and that phrase is the last thing the user reads. A drawing
    /// still loading keeps its phase — "ready" would be a lie, and the
    /// progress overlay takes over the moment the plate leaves.
    fn status_at(&self, status: SplashStatus) -> SplashStatus {
        if matches!(status, SplashStatus::Phase(_) | SplashStatus::Failed) {
            return status;
        }
        if self.clock.t.get() >= INTRO_MS - READY_LEAD_MS {
            SplashStatus::Ready
        } else {
            status
        }
    }
}

// ── Native hand-off ────────────────────────────────────────────────────────
//
// The window is created hidden so the plate can be on screen before the
// compositor ever sees it. Showing it straight away is what produced the white
// placeholder: DWM composites a new top-level window with an empty surface
// until the first swapchain frame arrives, and `winit` registers its class with
// no background brush (`hbrBackground: 0`), so that placeholder is white even
// though iced's clear colour is the dark theme.
//
// `DWMWA_CLOAK` is the one lever that hides a *visible* window from the
// compositor: the window keeps `WS_VISIBLE`, keeps receiving `WM_PAINT` and
// keeps presenting frames, but DWM draws nothing. Cloak first, show maximized
// second, and the first thing DWM composites is a frame that already carries
// the dark ground.

/// Hides or reveals a window at the compositor level (`DWMWA_CLOAK`). A cloaked
/// window is still `WS_VISIBLE` and still renders, which is exactly what the
/// hand-off needs: warm the first frame up while nobody can see it.
#[cfg(target_os = "windows")]
pub fn set_window_cloaked(window: &dyn iced::window::Window, cloaked: bool) {
    let Some(hwnd) = win32_hwnd(window) else { return };
    let value: i32 = i32::from(cloaked);
    unsafe {
        windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute(
            hwnd,
            windows_sys::Win32::Graphics::Dwm::DWMWA_CLOAK as u32,
            &value as *const i32 as *const core::ffi::c_void,
            core::mem::size_of::<i32>() as u32,
        );
    }
}

/// Shows a window that was created hidden and maximizes it in the same call,
/// so it never appears at the 800×600 creation size and then jumps.
#[cfg(target_os = "windows")]
pub fn show_window_maximized(window: &dyn iced::window::Window) {
    let Some(hwnd) = win32_hwnd(window) else { return };
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
            hwnd,
            windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWMAXIMIZED,
        );
    }
}

#[cfg(target_os = "windows")]
fn win32_hwnd(window: &dyn iced::window::Window) -> Option<windows_sys::Win32::Foundation::HWND> {
    use raw_window_handle::RawWindowHandle;
    match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(handle) => {
            Some(handle.hwnd.get() as windows_sys::Win32::Foundation::HWND)
        }
        _ => None,
    }
}

/// No-ops elsewhere: only Windows creates the main window hidden, so nothing
/// ever asks for a reveal on the other platforms.
#[cfg(not(target_os = "windows"))]
pub fn set_window_cloaked(_window: &dyn iced::window::Window, _cloaked: bool) {}

#[cfg(not(target_os = "windows"))]
pub fn show_window_maximized(_window: &dyn iced::window::Window) {}

/// Glyph metrics for the two wordmark halves, measured once per size:
/// `(size, DRAFT width, FLOW width)`.
type WordmarkMetrics = (f32, f32, f32);

pub struct SplashView {
    clock: Rc<Clock>,
    ui_scale: u16,
    locale: String,
    status: String,
    /// No hardware backend: the plate ends early and signs it with a cross,
    /// and the start rule is not drawn (nothing is still coming up).
    failed: bool,
    /// Plate alpha for this frame, including the ground (see [`plate_alpha`]).
    alpha: f32,
    metrics: Cell<Option<WordmarkMetrics>>,
}

impl canvas::Program<crate::app::Message> for SplashView {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let t = self.clock.t.get();
        let ctx = Ctx {
            w: bounds.width,
            h: bounds.height,
            s: scale(bounds.width, bounds.height),
            // The entrance is drawn on the 1.3 s design axis; stretching the
            // playback must not reorder or re-space anything, so every element
            // sees design-time rather than wall-clock time.
            t: t * TIMELINE_MS / INTRO_MS,
            // Everything including the ground fades as one plate, which is
            // what lets the prewarmed workspace show through instead of a cut.
            g: self.alpha,
        };
        paint(&mut frame, &ctx, self);
        vec![frame.into_geometry()]
    }
}

// ── Geometry ───────────────────────────────────────────────────────────────

/// The plate's alpha for the hand-off: 1.0 until the fade starts, 0.0 at the
/// end of it. The ground is painted with this too, so the workspace under it is
/// revealed rather than swapped in on a cut. Quiet mode fades inside its
/// unchanged hold.
fn plate_alpha(play: &Play, elapsed: f32) -> f32 {
    let (start, len) = match play {
        Play::Held(_) => return 1.0,
        Play::Animated => (INTRO_MS, FADE_MS),
        Play::Quiet => (QUIET_HOLD_MS - QUIET_FADE_MS, QUIET_FADE_MS),
    };
    if elapsed <= start {
        1.0
    } else {
        1.0 - ((elapsed - start) / len).clamp(0.0, 1.0)
    }
}


/// `12-adapt.md` §1.1. Below 800 px of width the 0.70 floor would push the
/// wordmark past the right edge, so the floor is dropped there and the plate
/// scales by width alone; §1.4 hides the small pieces at the same time.
fn scale(w: f32, h: f32) -> f32 {
    if w < 800.0 {
        w / tokens::splash::DESIGN_WIDTH
    } else {
        (h / 800.0)
            .min(w / tokens::splash::DESIGN_WIDTH)
            .max(tokens::splash::SCALE_MIN)
            .min(tokens::splash::SCALE_MAX)
    }
}

/// Which decorative layers survive a small window. The order is §1.4's, from
/// the piece whose loss costs least (the vertical dimension readout) to the
/// pieces that carry the "drawing" reading (the frame) which never drop.
struct Level {
    vertical_dim: bool,
    note: bool,
    scale_bar: bool,
    tables: bool,
    env: bool,
    texture: bool,
    title_full: bool,
    chrome: bool,
    /// The gear and its construction lines. Decoration, so it is the first
    /// thing to go when the sheet gets crowded.
    gear: bool,
}

impl Level {
    fn for_window(w: f32, h: f32) -> Self {
        let mut level = Self {
            vertical_dim: true,
            note: true,
            scale_bar: true,
            tables: true,
            env: true,
            texture: true,
            title_full: true,
            chrome: true,
            gear: true,
        };
        if w < 800.0 || h < 600.0 {
            level.vertical_dim = false;
            level.note = false;
            level.scale_bar = w >= 760.0 && h >= 560.0;
            level.tables = w >= 720.0 && h >= 520.0;
            level.env = w >= 680.0 && h >= 480.0;
            level.texture = w >= 640.0 && h >= 440.0;
            level.title_full = w >= 520.0;
            // Step 7 of §1.4: the version row and the upper-right tag are
            // hidden once their 11 px type lands under 9 physical px. Inside
            // this branch `s` is at most 800/1145 = 0.699, so 11 s < 9 always;
            // entering the branch at all is the condition.
            level.chrome = false;
        }
        // The gear is anchored between two neighbours — the revision table on
        // its left and the environment block on its right — so it is drawn
        // exactly while both survive the ladder. Without them it would float
        // alone on an otherwise empty sheet, which reads as a stray glyph.
        // At the binding case (720×520) the clearances still hold: the test
        // `the_gear_keeps_clear_of_every_neighbour_at_every_scale` measures
        // them at every rung, 720×520 included.
        level.gear = w >= 720.0 && h >= 520.0;
        level
    }
}

struct Ctx {
    w: f32,
    h: f32,
    s: f32,
    /// Entrance time in milliseconds.
    t: f32,
    /// Global plate alpha (the 180 ms hand-off).
    g: f32,
}

impl Ctx {
    fn base(&self) -> f32 {
        self.h - tokens::splash::BASE_FROM_BOTTOM * self.s
    }
    /// A length in scaled pixels, never thinner than one logical pixel.
    fn line(&self, n: f32) -> f32 {
        (n * self.s).max(1.0)
    }
    /// A sheet-layer type size: `max(7px, n × s)` per §1.4's closing note.
    fn sheet_size(&self, n: f32) -> f32 {
        (n * self.s).max(7.0).round()
    }
}

fn paint(frame: &mut canvas::Frame, c: &Ctx, view: &SplashView) {
    let level = Level::for_window(c.w, c.h);
    // Ground first, and always drawn: it carries the plate's alpha so the
    // window never flashes a light theme, and during the hand-off it
    // dissolves onto the already-laid-out workspace instead of cutting to
    // the clear colour.
    frame.fill_rectangle(
        Point::ORIGIN,
        Size::new(c.w, c.h),
        tint(tokens::splash::GROUND, c.g),
    );
    // A minimised window hands the canvas a 0–8 px box; every scaled length
    // would collapse onto the same pixel, drawing a meaningless smudge.
    if c.g <= 0.0 || c.w < 8.0 || c.h < 8.0 {
        return;
    }

    if level.texture {
        paint_dots(frame, c);
    }
    paint_axis(frame, c);
    paint_sheet(frame, c, &level, view);
    if level.texture {
        paint_ruler(frame, c);
    }
    paint_gears(frame, c, &level);
    let wordmark_right = paint_wordmark(frame, c, view);
    paint_cad(frame, c);
    // The status line is the last non-wordmark piece to go (§1.4 step 7): it is
    // the only thing that says the app is starting, so it is never gated.
    paint_status(frame, c, view);
    if level.chrome {
        paint_meta(frame, c);
        paint_tag(frame, c);
    }
    if level.vertical_dim && !view.failed {
        paint_dimension(frame, c, wordmark_right);
    }
    // The end cap marks "still coming up". On the failure plate the line ends
    // in a cross instead (§1.5), so the two never compete for the same corner.
    if !view.failed {
        paint_cap(frame, c);
    }
    paint_aux(frame, c);
    paint_baseline(frame, c, view.failed, wordmark_right);
    paint_zero(frame, c);
}

/// A filled rectangle on the pixel grid. Left/top are rounded rather than the
/// right/bottom, so a 1 px hairline never rounds away to nothing.
fn bar(frame: &mut canvas::Frame, x: f32, y: f32, w: f32, h: f32, color: Color) {
    if color.a <= 0.0 || w <= 0.0 || h <= 0.0 {
        return;
    }
    frame.fill_rectangle(
        Point::new(x.round(), y.round()),
        Size::new(w.round().max(1.0), h.round().max(1.0)),
        color,
    );
}

fn tint(color: Color, alpha: f32) -> Color {
    Color {
        a: color.a * alpha.clamp(0.0, 1.0),
        ..color
    }
}

fn paint_dots(frame: &mut canvas::Frame, c: &Ctx) {
    let alpha = c.g * track(c.t, 0.0, 600.0, |t| t);
    if alpha <= 0.0 {
        return;
    }
    let color = tint(tokens::splash::DOT, alpha);
    let size = (2.0 * c.s).max(1.0);
    let step = 20.0 * c.s;
    let (x0, x1) = (c.w * 0.48, c.w - 96.0 * c.s);
    let (y0, y1) = (250.0 * c.s, c.base() - 150.0 * c.s);
    let mut y = y0;
    while y <= y1 {
        let mut x = x0;
        while x <= x1 {
            frame.fill_rectangle(Point::new(x.round(), y.round()), Size::new(size, size), color);
            x += step;
        }
        y += step;
    }
}

fn paint_axis(frame: &mut canvas::Frame, c: &Ctx) {
    let x = 96.0 * c.s;
    let top = 107.0 * c.s;
    let bottom = c.h - 52.0 * c.s;
    let grow = track(c.t, 240.0, 420.0, |t| cubic_bezier(ease::OUT_QUART, t));
    if grow > 0.0 {
        bar(
            frame,
            x,
            top,
            1.0,
            (bottom - top) * grow,
            tint(tokens::splash::LINE_FRAME, c.g),
        );
    }
    // The five-pixel graduations every 56 s, arriving after the shaft.
    let alpha = c.g * track(c.t, 300.0, 200.0, |t| t);
    if alpha <= 0.0 {
        return;
    }
    let color = tint(tokens::splash::LINE_FRAME, alpha);
    let mut y = c.base() - 56.0 * c.s;
    while y > 120.0 * c.s {
        bar(frame, x, y, 5.0 * c.s, 1.0, color);
        y -= 56.0 * c.s;
    }
}

fn paint_sheet(frame: &mut canvas::Frame, c: &Ctx, level: &Level, view: &SplashView) {
    let alpha = c.g * track(c.t, 80.0, 700.0, |t| t);
    if alpha <= 0.0 {
        return;
    }
    let s = c.s;
    let frame_ink = tint(tokens::splash::LINE_FRAME, alpha);
    let inner_ink = tint(tokens::splash::LINE_INNER, alpha);
    let table_ink = tint(tokens::splash::LINE_TABLE, alpha);
    let label_ink = tint(tokens::splash::LABEL, alpha);
    let value_ink = tint(tokens::splash::MUTED, alpha);
    let dim_ink = tint(tokens::splash::LINE_DIM, alpha);
    let strong_ink = tint(tokens::splash::LINE_STRONG, alpha);

    let o = tokens::splash::FRAME_OUTER * s;
    let io = tokens::splash::FRAME_INNER * s;
    let (x0, x1, y0, y1) = (io, c.w - io, io, c.h - io);

    // Trim line and sheet edge.
    bar(frame, o, o, c.w - 2.0 * o, 1.0, frame_ink);
    bar(frame, o, c.h - o - 1.0, c.w - 2.0 * o, 1.0, frame_ink);
    bar(frame, o, o, 1.0, c.h - 2.0 * o, frame_ink);
    bar(frame, c.w - o - 1.0, o, 1.0, c.h - 2.0 * o, frame_ink);
    bar(frame, io, io, c.w - 2.0 * io, 1.0, inner_ink);
    bar(frame, io, c.h - io - 1.0, c.w - 2.0 * io, 1.0, inner_ink);
    bar(frame, io, io, 1.0, c.h - 2.0 * io, inner_ink);
    bar(frame, c.w - io - 1.0, io, 1.0, c.h - 2.0 * io, inner_ink);

    // 1–4 across, A–D down, marked in the 16 s band between the two frames.
    for i in 1..4 {
        let x = x0 + (x1 - x0) * i as f32 / 4.0;
        bar(frame, x, o, 1.0, io - o, table_ink);
        bar(frame, x, c.h - io, 1.0, io - o, table_ink);
    }
    for j in 1..4 {
        let y = y0 + (y1 - y0) * j as f32 / 4.0;
        bar(frame, o, y, io - o, 1.0, table_ink);
        bar(frame, c.w - io, y, io - o, 1.0, table_ink);
    }
    let band = (o + io) * 0.5;
    let tag_size = c.sheet_size(8.0);
    for i in 0..4 {
        let x = x0 + (x1 - x0) * (i as f32 + 0.5) / 4.0;
        label(
            frame,
            &(i + 1).to_string(),
            x,
            band - 5.0 * s,
            tag_size,
            label_ink,
            TextAlign::Center,
            Vertical::Top,
        );
        label(
            frame,
            &(i + 1).to_string(),
            x,
            c.h - band - 5.0 * s,
            tag_size,
            label_ink,
            TextAlign::Center,
            Vertical::Top,
        );
    }
    for j in 0..4 {
        let y = y0 + (y1 - y0) * (j as f32 + 0.5) / 4.0;
        let letter = ["A", "B", "C", "D"][j];
        label(
            frame,
            letter,
            band - 3.0 * s,
            y,
            tag_size,
            label_ink,
            TextAlign::Left,
            Vertical::Center,
        );
        label(
            frame,
            letter,
            c.w - band - 3.0 * s,
            y,
            tag_size,
            label_ink,
            TextAlign::Left,
            Vertical::Center,
        );
    }

    // Title block, flush with the inner frame's bottom-right corner.
    let tw = 320.0 * s;
    let th = 96.0 * s;
    let tx1 = c.w - io;
    let ty1 = c.h - io;
    let tx0 = tx1 - tw;
    let ty0 = ty1 - th;
    let pad = 8.0 * s;
    bar(frame, tx0, ty0, tw, 1.0, frame_ink);
    bar(frame, tx0, ty1 - 1.0, tw, 1.0, frame_ink);
    bar(frame, tx0, ty0, 1.0, th, frame_ink);
    bar(frame, tx1 - 1.0, ty0, 1.0, th, frame_ink);
    let c1 = tx0 + 140.0 * s;
    let c2 = c1 + 90.0 * s;
    let rh = th * 0.5;
    bar(frame, c1, ty0, 1.0, th, inner_ink);
    bar(frame, c2, ty0, 1.0, th, inner_ink);
    bar(frame, tx0, ty0 + rh, tw, 1.0, inner_ink);
    let caption = c.sheet_size(7.5);
    let value = c.sheet_size(11.0);
    let prose = c.sheet_size(9.0);
    if level.title_full {
        cell(
            frame, tx0, ty0, pad, 7.0 * s, caption, 22.0 * s, value, "DRAWING", "DF-1.0.0",
            label_ink, value_ink,
        );
        cell(
            frame,
            c1,
            ty0,
            pad,
            7.0 * s,
            caption,
            22.0 * s,
            value,
            "SCALE",
            "1:1",
            label_ink,
            value_ink,
        );
        cell(
            frame,
            c2,
            ty0,
            pad,
            7.0 * s,
            caption,
            22.0 * s,
            value,
            "SHEET",
            "1/1",
            label_ink,
            value_ink,
        );
        label(
            frame,
            "DRAFTFLOW CAD",
            tx0 + pad,
            ty0 + rh + 19.0 * s,
            prose,
            value_ink,
            TextAlign::Left,
            Vertical::Top,
        );
        label(
            frame,
            "GPL-3.0",
            c1 + pad,
            ty0 + rh + 21.0 * s,
            caption,
            label_ink,
            TextAlign::Left,
            Vertical::Top,
        );
        label(
            frame,
            "© 2026",
            c2 + pad,
            ty0 + rh + 21.0 * s,
            caption,
            label_ink,
            TextAlign::Left,
            Vertical::Top,
        );
    } else {
        // One row only: who, and which build.
        label(
            frame,
            "DRAFTFLOW CAD  1.0.0",
            tx0 + pad,
            ty0 + 24.0 * s,
            caption,
            value_ink,
            TextAlign::Left,
            Vertical::Top,
        );
    }

    if level.tables {
        // Revision table.
        let rx = 128.0 * s;
        let ry = 132.0 * s;
        let rw = 320.0 * s;
        let rl = 22.0 * s;
        bar(frame, rx, ry, rw, 1.0, frame_ink);
        for i in 1..=4 {
            bar(frame, rx, ry + rl * i as f32, rw, 1.0, inner_ink);
        }
        bar(frame, rx + 40.0 * s, ry, 1.0, rl * 4.0, inner_ink);
        bar(frame, rx + 140.0 * s, ry, 1.0, rl * 4.0, inner_ink);
        label(frame, "REV", rx + pad, ry + 7.0 * s, caption, label_ink, TextAlign::Left, Vertical::Top);
        label(frame, "DATE", rx + 48.0 * s, ry + 7.0 * s, caption, label_ink, TextAlign::Left, Vertical::Top);
        label(
            frame,
            "DESCRIPTION",
            rx + 148.0 * s,
            ry + 7.0 * s,
            caption,
            label_ink,
            TextAlign::Left,
            Vertical::Top,
        );
        let revisions = [
            ("A", "2026-10-12", "INITIAL"),
            ("B", "2026-10-26", "SPLASH"),
            ("C", "2026-11-04", "RELEASE"),
        ];
        for (n, (rev, date, text)) in revisions.iter().enumerate() {
            let y = ry + rl * (n as f32 + 1.0) + 7.0 * s;
            label(frame, rev, rx + pad, y, caption, label_ink, TextAlign::Left, Vertical::Top);
            label(frame, date, rx + 48.0 * s, y, caption, label_ink, TextAlign::Left, Vertical::Top);
            label(
                frame,
                text,
                rx + 148.0 * s,
                y - 0.5 * s,
                caption,
                value_ink,
                TextAlign::Left,
                Vertical::Top,
            );
        }

        // Layer table. The swatch column is the only filled block on the plate
        // that is not the accent.
        let lx = 128.0 * s;
        let ly = 268.0 * s;
        let lw = 240.0 * s;
        let ll = 22.0 * s;
        bar(frame, lx, ly, lw, 1.0, frame_ink);
        for i in 1..=4 {
            bar(frame, lx, ly + ll * i as f32, lw, 1.0, inner_ink);
        }
        bar(frame, lx + 20.0 * s, ly, 1.0, ll * 4.0, inner_ink);
        bar(frame, lx + 120.0 * s, ly, 1.0, ll * 4.0, inner_ink);
        label(frame, "LAYER", lx + 28.0 * s, ly + 7.0 * s, caption, label_ink, TextAlign::Left, Vertical::Top);
        label(frame, "LTYPE", lx + 128.0 * s, ly + 7.0 * s, caption, label_ink, TextAlign::Left, Vertical::Top);
        let layers = ["0", "DIM", "TEXT"];
        for (n, name) in layers.iter().enumerate() {
            let y = ly + ll * (n as f32 + 1.0) + 7.0 * s;
            bar(frame, lx + 8.0 * s, y + 1.0 * s, 6.0 * s, 6.0 * s, table_ink);
            label(frame, name, lx + 28.0 * s, y, caption, label_ink, TextAlign::Left, Vertical::Top);
            label(
                frame,
                "CONTINUOUS",
                lx + 128.0 * s,
                y,
                caption,
                label_ink,
                TextAlign::Left,
                Vertical::Top,
            );
        }
    }

    if level.scale_bar {
        let bx = 128.0 * s;
        let by = 388.0 * s;
        let cell_w = 60.0 * s;
        let bar_h = 8.0 * s;
        for i in 0..4 {
            if i % 2 == 0 {
                bar(frame, bx + i as f32 * cell_w, by, cell_w, bar_h, strong_ink);
            }
        }
        bar(frame, bx, by, cell_w * 4.0, 1.0, frame_ink);
        bar(frame, bx, by + bar_h - 1.0, cell_w * 4.0, 1.0, frame_ink);
        for i in 0..=4 {
            let x = bx + i as f32 * cell_w;
            bar(frame, x, by, 1.0, bar_h, frame_ink);
            label(
                frame,
                &(i * 50).to_string(),
                x,
                by + bar_h + 5.0 * s,
                tag_size,
                label_ink,
                TextAlign::Center,
                Vertical::Top,
            );
        }
        label(
            frame,
            "GRAPHIC SCALE",
            bx,
            by - 13.0 * s,
            caption,
            label_ink,
            TextAlign::Left,
            Vertical::Top,
        );
    }

    if level.env {
        // What the process actually knows before it has drawn anything. The
        // system DPI is deliberately absent: iced 0.15 exposes it only through
        // an async task, and a guessed `1.00` would be a false reading on the
        // one plate whose job is to look like an instrument.
        let rows = [
            ("PLATFORM", "WIN64".to_string()),
            ("RENDER", "AUTO".to_string()),
            ("UI SCALE", format!("{}%", view.ui_scale)),
            ("LOCALE", view.locale.clone()),
        ];
        for (n, (key, val)) in rows.iter().enumerate() {
            let y = 126.0 * s + n as f32 * 22.0 * s;
            label(
                frame,
                key,
                c.w - 190.0 * s,
                y,
                caption,
                label_ink,
                TextAlign::Right,
                Vertical::Top,
            );
            label(
                frame,
                val,
                c.w - 96.0 * s,
                y - 1.0 * s,
                prose,
                value_ink,
                TextAlign::Right,
                Vertical::Top,
            );
        }
    }

    if level.note {
        label(
            frame,
            "UNLESS OTHERWISE SPECIFIED · DIMENSIONS IN LOGICAL PX · TOLERANCE ±1",
            96.0 * s,
            c.h - 112.0 * s,
            caption,
            label_ink,
            TextAlign::Left,
            Vertical::Top,
        );
        // Footer rule: closes the bottom-left gap as the sheet's lower edge.
        let from = io;
        let to = tx0 - 24.0 * s;
        bar(frame, from, c.h - 92.0 * s, to - from, 1.0, inner_ink);
    }

    if level.vertical_dim {
        // The dimension that measures the wordmark's cap height: 112 at s=1.
        // iced exposes no glyph outline ascent, so the plate uses the ratio the
        // design measured (0.747) rather than inventing a probe.
        let cap_h = tokens::splash::CAP_RATIO * tokens::splash::WORDMARK_SIZE * s;
        let x = c.w - 64.0 * s;
        let top = c.base() - cap_h;
        bar(frame, x, top, 1.0, cap_h, dim_ink);
        bar(frame, x - 4.0 * s, top, 8.0 * s, 1.0, dim_ink);
        bar(frame, x - 4.0 * s, c.base() - 1.0, 8.0 * s, 1.0, dim_ink);
        let (draft, flow) = wordmark_metrics(view, s);
        let left = 88.0 * s + draft + flow + 8.0 * s;
        bar(frame, left, top, x + 4.0 * s - left, 1.0, dim_ink);
        label(
            frame,
            &((cap_h / s) as i32).to_string(),
            x - 44.0 * s,
            top + 26.0 * s,
            c.sheet_size(11.0),
            value_ink,
            TextAlign::Left,
            Vertical::Top,
        );
    }
}

/// One gear of the pair. Lengths are canvas px (already scaled); `phase` is
/// the rotation at the first frame and `rate` the rotation per millisecond.
struct Gear {
    cx: f32,
    cy: f32,
    tip: f32,
    root: f32,
    bore: f32,
    teeth: usize,
    phase: f32,
    rate: f32,
    /// The wheel carries the centre cross, the dash-dot pitch circle and the
    /// keyway; the pinion carries a short cross and its bore, so the pair does
    /// not turn into texture at 1×.
    detailed: bool,
}

/// Two external gears on one module: a 20-tooth wheel driving a 12-tooth
/// pinion from the lower right.
///
/// The pinion's pitch radius comes from the same module as the wheel's, its
/// centre sits `pitch_w + pitch_p` away along the mesh line, and its phase is
/// half a tooth off so its gaps face the wheel's teeth. With that, the ratio
/// `-pitch_w / pitch_p` is not a stylistic choice: it is the only rate that
/// keeps the two in mesh, and the test `the_pair_is_a_real_mesh` checks both
/// the centre distance and the ratio.
fn gear_pair(s: f32) -> (Gear, Gear) {
    let pitch = (tokens::splash::GEAR_TIP + tokens::splash::GEAR_ROOT) * 0.5;
    let addendum = tokens::splash::GEAR_TIP - pitch;
    let module = 2.0 * pitch / tokens::splash::GEAR_TEETH as f32;
    let pinion_pitch = module * tokens::splash::PINION_TEETH as f32 * 0.5;
    let angle = tokens::splash::GEAR_MESH_DEG.to_radians();
    let dist = (pitch + pinion_pitch) * s;
    let pinion_step = std::f32::consts::TAU / tokens::splash::PINION_TEETH as f32;
    let rate = tokens::splash::GEAR_TURN_RAD / INTRO_MS;
    let wheel = Gear {
        cx: tokens::splash::GEAR_X * s,
        cy: tokens::splash::GEAR_Y * s,
        tip: tokens::splash::GEAR_TIP * s,
        root: tokens::splash::GEAR_ROOT * s,
        bore: tokens::splash::GEAR_BORE * s,
        teeth: tokens::splash::GEAR_TEETH,
        phase: angle,
        rate,
        detailed: true,
    };
    let pinion = Gear {
        cx: wheel.cx + dist * angle.cos(),
        cy: wheel.cy + dist * angle.sin(),
        tip: (pinion_pitch + addendum) * s,
        root: (pinion_pitch - addendum) * s,
        bore: tokens::splash::PINION_BORE * s,
        teeth: tokens::splash::PINION_TEETH,
        phase: angle + std::f32::consts::PI - 0.5 * pinion_step,
        rate: -(pitch / pinion_pitch) * rate,
        detailed: false,
    };
    (wheel, pinion)
}

/// The pair's union box in canvas px, `tip + 18 s` around each centre. Only
/// the clearance test needs it — drawing walks the two gears directly.
#[cfg(test)]
fn gear_pair_box(s: f32) -> (f32, f32, f32, f32) {
    let (wheel, pinion) = gear_pair(s);
    let reach = |g: &Gear| g.tip + 18.0 * s;
    (
        (wheel.cx - reach(&wheel)).min(pinion.cx - reach(&pinion)),
        (wheel.cy - reach(&wheel)).min(pinion.cy - reach(&pinion)),
        (wheel.cx + reach(&wheel)).max(pinion.cx + reach(&pinion)),
        (wheel.cy + reach(&wheel)).max(pinion.cy + reach(&pinion)),
    )
}

/// The working pair, drawn as sheet figures rather than icons: true tooth
/// profiles in outline, a dash-dot pitch circle on the wheel, centre crosses
/// and bored hubs. All line work, so it reads as drafting, not as a glyph
/// pasted onto the plate.
///
/// Both turn at constant angular speed for the whole run, fade included. A
/// gear that stops before the plate leaves reads as a dropped frame.
fn paint_gears(frame: &mut canvas::Frame, c: &Ctx, level: &Level) {
    if !level.gear {
        return;
    }
    let p = track(c.t, 320.0, 420.0, |t| cubic_bezier(ease::OUT_QUART, t)) * c.g;
    if p <= 0.0 {
        return;
    }
    let (wheel, pinion) = gear_pair(c.s);
    draw_gear(frame, &wheel, p, c);
    draw_gear(frame, &pinion, p, c);
}

fn draw_gear(frame: &mut canvas::Frame, g: &Gear, alpha: f32, c: &Ctx) {
    let ss = c.s;
    let rot = g.phase + g.rate * c.t;
    let outline = tint(tokens::splash::LINE_STRONG, alpha);
    let guide = tint(tokens::splash::LINE_DIM, alpha);
    let width = c.line(1.0);
    let at = |angle: f32, radius: f32| {
        Point::new(g.cx + angle.cos() * radius, g.cy + angle.sin() * radius)
    };

    // Centre cross, extended past the tip so it reads as a construction line.
    let reach = g.tip + if g.detailed { 18.0 } else { 8.0 } * ss;
    frame.stroke(
        &canvas::Path::line(Point::new(g.cx - reach, g.cy), Point::new(g.cx + reach, g.cy)),
        canvas::Stroke::default().with_color(guide).with_width(1.0),
    );
    frame.stroke(
        &canvas::Path::line(Point::new(g.cx, g.cy - reach), Point::new(g.cx, g.cy + reach)),
        canvas::Stroke::default().with_color(guide).with_width(1.0),
    );

    if g.detailed {
        // Dash-dot pitch circle: six dashes, one per 60°, at the mean radius.
        let pitch = (g.tip + g.root) * 0.5;
        let mut dashes: Vec<Point> = Vec::with_capacity(18);
        for k in 0..6 {
            let base = k as f32 * (std::f32::consts::TAU / 6.0) + rot;
            for j in 0..=2 {
                dashes.push(at(base + 0.30 * j as f32, pitch));
            }
        }
        let dash_path = canvas::Path::new(|b| {
            for run in dashes.chunks(3) {
                if let Some(first) = run.first() {
                    b.move_to(*first);
                    for pt in &run[1..] {
                        b.line_to(*pt);
                    }
                }
            }
        });
        frame.stroke(
            &dash_path,
            canvas::Stroke::default().with_color(guide).with_width(1.0),
        );
    }

    // Tooth profile. Per tooth: root arc, flank, tip arc, flank, root arc.
    // Flanks are the straight joins between the sampled arcs, which is what a
    // 1 px pen renders anyway at this radius.
    let step = std::f32::consts::TAU / g.teeth as f32;
    let mut ring: Vec<Point> = Vec::with_capacity(g.teeth * 8);
    for i in 0..g.teeth {
        let a = i as f32 * step + rot;
        for k in 0..=2 {
            ring.push(at(a - step * 0.5 + step * 0.20 * k as f32 / 2.0, g.root));
        }
        for k in 0..=3 {
            ring.push(at(a - step * 0.13 + step * 0.26 * k as f32 / 3.0, g.tip));
        }
        for k in 1..=2 {
            ring.push(at(a + step * 0.30 + step * 0.20 * k as f32 / 2.0, g.root));
        }
    }
    let ring_path = canvas::Path::new(|b| {
        if let Some(first) = ring.first() {
            b.move_to(*first);
            for pt in &ring[1..] {
                b.line_to(*pt);
            }
            b.close();
        }
    });
    frame.stroke(
        &ring_path,
        canvas::Stroke::default()
            .with_color(outline)
            .with_width(width),
    );

    // Bore, plus a keyway on the wheel so the hub is not a plain circle.
    frame.stroke(
        &canvas::Path::circle(Point::new(g.cx, g.cy), g.bore),
        canvas::Stroke::default().with_color(outline).with_width(1.0),
    );
    if g.detailed {
        let key = std::f32::consts::FRAC_PI_4 + rot;
        frame.stroke(
            &canvas::Path::line(at(key, g.bore - width * 0.5), at(key, g.bore + 6.0 * ss)),
            canvas::Stroke::default().with_color(guide).with_width(1.0),
        );
    }
}

/// One title-block cell: the key at +7 s and the value at +22 s, as the
/// sample's `cell()` does.
#[allow(clippy::too_many_arguments)]
fn cell(
    frame: &mut canvas::Frame,
    x: f32,
    y: f32,
    pad: f32,
    key_dy: f32,
    key_size: f32,
    value_dy: f32,
    value_size: f32,
    key: &str,
    value: &str,
    key_ink: Color,
    value_ink: Color,
) {
    label(
        frame,
        key,
        x + pad,
        y + key_dy,
        key_size,
        key_ink,
        TextAlign::Left,
        Vertical::Top,
    );
    label(
        frame,
        value,
        x + pad,
        y + value_dy,
        value_size,
        value_ink,
        TextAlign::Left,
        Vertical::Top,
    );
}

fn paint_ruler(frame: &mut canvas::Frame, c: &Ctx) {
    let top = c.base() + 1.5 * c.s;
    let mut x = c.w - 96.0 * c.s;
    let left = x - 300.0 * c.s;
    let mut i = 0;
    while x >= left - 0.01 {
        let long = i % 5 == 0;
        let grow = track(c.t, 120.0 + i as f32 * 10.0, 260.0, |t| {
            cubic_bezier(ease::OUT_QUART, t)
        });
        let height = (if long { 9.0 } else { 4.0 }) * c.s * grow;
        let ink = if long {
            tokens::splash::LINE_STRONG
        } else {
            tokens::splash::LINE_TABLE
        };
        bar(frame, x, top, 1.0, height, tint(ink, c.g));
        x -= 14.0 * c.s;
        i += 1;
    }
}

/// Draws DRAFT and FLOW and returns the measured right edge, which the
/// horizontal dimension and the vertical extension both anchor to.
fn paint_wordmark(frame: &mut canvas::Frame, c: &Ctx, view: &SplashView) -> f32 {
    let size = tokens::splash::WORDMARK_SIZE * c.s;
    let left = 88.0 * c.s;
    let bottom = c.h - 198.0 * c.s;
    let (draft_w, flow_w) = wordmark_metrics(view, c.s);
    let d = track(c.t, 240.0, 420.0, |t| cubic_bezier(ease::RISE, t));
    let f = track(c.t, 310.0, 420.0, |t| cubic_bezier(ease::RISE, t));
    let rise = 26.0 * c.s;
    label_font(
        frame,
        "DRAFT",
        left,
        bottom + (1.0 - d) * rise,
        size,
        tint(tokens::splash::INK, d * c.g),
        tokens::splash::WORDMARK_BOLD,
        TextAlign::Left,
        Vertical::Bottom,
    );
    label_font(
        frame,
        "FLOW",
        left + draft_w,
        bottom + (1.0 - f) * rise,
        size,
        tint(tokens::splash::INK, f * c.g),
        tokens::splash::WORDMARK_LIGHT,
        TextAlign::Left,
        Vertical::Bottom,
    );
    left + draft_w + flow_w
}

fn paint_cad(frame: &mut canvas::Frame, c: &Ctx) {
    let size = 30.0 * c.s;
    let font = tokens::UI_FONT;
    let mut x = 96.0 * c.s;
    for (i, letter) in ["C", "A", "D"].iter().enumerate() {
        let p = track(c.t, 600.0 + i as f32 * 50.0, 220.0, |t| {
            cubic_bezier(ease::CAD, t)
        });
        label_font(
            frame,
            letter,
            x,
            c.h - 174.0 * c.s + (1.0 - p) * 8.0 * c.s,
            size,
            tint(tokens::splash::MUTED, p * c.g),
            font,
            TextAlign::Left,
            Vertical::Top,
        );
        // The 0.5 em tracking is structure, so it is added by hand; no canvas
        // text spacing exists to ask for it.
        x += measure(letter, size, font) + size * 0.5;
    }
}

fn paint_status(frame: &mut canvas::Frame, c: &Ctx, view: &SplashView) {
    let p = track(c.t, 760.0, 240.0, |t| t) * c.g;
    // The rule has its own fade because it starts 60 ms before its label: on
    // the shared `p` the fill would jump in already one tenth done.
    let rule = track(c.t, 700.0, 240.0, |t| t) * c.g;
    if p <= 0.0 && rule <= 0.0 {
        return;
    }
    let size = 12.0 * c.s;
    let box_size = (11.0 * c.s).max(8.0);
    let center_y = c.h - 52.0 * c.s - size * 0.5;
    let stroke = (1.5 * c.s).max(1.0);
    let x = 96.0 * c.s;
    let top = center_y - box_size * 0.5;
    let ink = tint(tokens::splash::ACCENT, p);
    // An outlined square, not a filled one: four edges, so the ground shows
    // through the middle exactly as the sample's border does.
    bar(frame, x, top, box_size, stroke, ink);
    bar(frame, x, top + box_size - stroke, box_size, stroke, ink);
    bar(frame, x, top, stroke, box_size, ink);
    bar(frame, x + box_size - stroke, top, stroke, box_size, ink);
    label(
        frame,
        &view.status,
        x + box_size + 10.0 * c.s,
        center_y,
        size,
        tint(tokens::splash::MUTED, p),
        TextAlign::Left,
        Vertical::Center,
    );

    // The start rule. It runs on the entrance clock, not on a byte count: the
    // splash has no measurable download, so the bar says "the plate is still
    // coming up" and leaves with it. It fills across the whole time the status
    // line is readable (700 → 1300 ms) instead of reserving a burst for the
    // end; a rule that sits still until the last third reads as broken. On a
    // locale whose status line leaves no room the rule is dropped, not
    // squeezed.
    let text_w = measure(&view.status, size, tokens::UI_FONT);
    if view.failed {
        // A static notice: the plate is not waiting on anything, so a rule
        // that keeps filling would promise progress that is not happening.
        return;
    }
    if let Some((x0, x1)) = progress_span(c, text_w) {
        paint_progress(frame, c, rule, x0, x1, center_y);
    }
}

/// The horizontal span the start rule may occupy: right of the status text,
/// left of the title block, with the same 24 s of air the plate uses
/// elsewhere. `None` when what is left is shorter than
/// [`tokens::splash::PROGRESS_MIN`] — the status text alone is the state then.
fn progress_span(c: &Ctx, text_w: f32) -> Option<(f32, f32)> {
    let s = c.s;
    let x0 = 96.0 * s + (11.0 * s).max(8.0) + 10.0 * s + text_w + 18.0 * s;
    let title_left = c.w - tokens::splash::FRAME_INNER * s - 320.0 * s;
    let x1 = title_left - 24.0 * s;
    (x1 - x0 >= tokens::splash::PROGRESS_MIN * s).then_some((x0, x1))
}

fn paint_progress(
    frame: &mut canvas::Frame,
    c: &Ctx,
    alpha: f32,
    x0: f32,
    x1: f32,
    center_y: f32,
) {
    let s = c.s;
    let h = c.line(tokens::splash::PROGRESS_HEIGHT);
    let y = center_y - h * 0.5;
    let len = x1 - x0;
    bar(
        frame,
        x0,
        y,
        len,
        h,
        tint(tokens::splash::LINE_TABLE, alpha * 0.55),
    );
    for k in 1..4 {
        bar(
            frame,
            x0 + len * k as f32 / 4.0,
            y - 3.0 * s,
            1.0,
            h + 6.0 * s,
            tint(tokens::splash::LINE_TABLE, alpha * 0.45),
        );
    }
    // The run is linear and reaches the end at `INTRO_MS`, so the plate hands
    // off on a full rule rather than a stalled 96 % one, and every frame
    // between 700 ms and the fade shows a different fill.
    let prog = track(c.t, 700.0, 600.0, |t| t);
    let run = len * prog;
    if run > 0.0 {
        bar(frame, x0, y, run, h, tint(tokens::splash::ACCENT, alpha));
        if prog < 1.0 {
            bar(
                frame,
                x0 + run - 1.0,
                y - 4.0 * s,
                1.0,
                h + 8.0 * s,
                tint(tokens::splash::ACCENT, alpha),
            );
        }
    }
}

fn paint_meta(frame: &mut canvas::Frame, c: &Ctx) {
    let p = track(c.t, 800.0, 240.0, |t| t) * c.g;
    if p <= 0.0 {
        return;
    }
    let size = 11.0 * c.s;
    let mut x = 96.0 * c.s;
    let y = 84.0 * c.s;
    for text in ["1.0.0", "GPL-3.0", "WIN64"] {
        label(
            frame,
            text,
            x,
            y,
            size,
            tint(tokens::splash::MUTED, p),
            TextAlign::Left,
            Vertical::Top,
        );
        x += measure(text, size, tokens::UI_FONT) + 20.0 * c.s;
    }
}

fn paint_tag(frame: &mut canvas::Frame, c: &Ctx) {
    let p = track(c.t, 840.0, 240.0, |t| t) * c.g;
    if p <= 0.0 {
        return;
    }
    label(
        frame,
        "DWG · 1:1",
        c.w - 96.0 * c.s,
        84.0 * c.s,
        11.0 * c.s,
        tint(tokens::splash::MUTED, p),
        TextAlign::Right,
        Vertical::Top,
    );
}

fn paint_dimension(frame: &mut canvas::Frame, c: &Ctx, wordmark_right: f32) {
    let p = track(c.t, 1020.0, 260.0, |t| t) * c.g;
    if p <= 0.0 {
        return;
    }
    let x1 = wordmark_right + 24.0 * c.s;
    let x2 = c.w - 114.0 * c.s;
    let y = c.base() - 72.0 * c.s;
    // Narrow windows drop the dimension rather than crowding the right edge
    // (§1.3: 800×600 and 1024×768 both hide it).
    if x2 - x1 <= 80.0 * c.s || x2 >= c.w - 20.0 * c.s {
        return;
    }
    let ink = tint(tokens::splash::LINE_DIM, p);
    bar(frame, x1, y, x2 - x1, 1.0, ink);
    bar(frame, x1, y - 4.5 * c.s, 1.0, 9.0 * c.s, ink);
    bar(frame, x2, y - 4.5 * c.s, 1.0, 9.0 * c.s, ink);
    label(
        frame,
        &(((x2 - x1) / c.s).round() as i32).to_string(),
        (x1 + x2) * 0.5,
        y - 8.0 * c.s,
        11.0 * c.s,
        tint(tokens::splash::MUTED, p),
        TextAlign::Center,
        Vertical::Top,
    );
}

fn paint_cap(frame: &mut canvas::Frame, c: &Ctx) {
    let p = track(c.t, 900.0, 220.0, |t| cubic_bezier(ease::OUT_QUART, t)) * c.g;
    if p <= 0.0 {
        return;
    }
    let size = 18.0 * c.s;
    let slide = (1.0 - p) * 24.0 * c.s;
    let left = c.w - 96.0 * c.s - size + slide;
    let top = c.base() - size * 0.5;
    bar(frame, left, top, size, size, tint(tokens::splash::ACCENT, p));
    bar(
        frame,
        left + 7.0 * c.s,
        top + 7.0 * c.s,
        4.0 * c.s,
        4.0 * c.s,
        tint(tokens::splash::GROUND, p),
    );
    // The 1 px halo 3 s outside the block, at 42 % — four edges, so the ground
    // shows through the middle exactly as the sample's border does.
    let ring = tint(tokens::splash::ACCENT, p * 0.42);
    let (rx, ry) = (left - 3.0 * c.s, top - 3.0 * c.s);
    let (rw, rh) = (size + 6.0 * c.s, size + 6.0 * c.s);
    bar(frame, rx, ry, rw, 1.0, ring);
    bar(frame, rx, ry + rh - 1.0, rw, 1.0, ring);
    bar(frame, rx, ry, 1.0, rh, ring);
    bar(frame, rx + rw - 1.0, ry, 1.0, rh, ring);
}

fn paint_aux(frame: &mut canvas::Frame, c: &Ctx) {
    let grow = track(c.t, 480.0, 380.0, |t| cubic_bezier(ease::OUT_QUART, t));
    if grow <= 0.0 {
        return;
    }
    bar(
        frame,
        0.0,
        c.h - 196.0 * c.s,
        c.w * grow,
        1.0,
        tint(tokens::splash::LINE_FRAME, c.g),
    );
}

/// Where the failure plate's baseline stops (§1.5). Derived from the measured
/// wordmark, not from `W − 280 s` alone: on a narrow window the default cut
/// lands under the wordmark and the cross reads as hanging off a letter.
fn failure_cut(w: f32, s: f32, wordmark_right: f32) -> f32 {
    (w - 280.0 * s).max(wordmark_right + 24.0 * s)
}

fn paint_baseline(frame: &mut canvas::Frame, c: &Ctx, failed: bool, wordmark_right: f32) {
    let grow = track(c.t, 0.0, 420.0, |t| cubic_bezier(ease::OUT_QUART, t));
    if grow <= 0.0 {
        return;
    }
    let end = if failed {
        failure_cut(c.w, c.s, wordmark_right)
    } else {
        c.w
    };
    let top = c.h - tokens::splash::BASE_FROM_BOTTOM * c.s - c.line(1.5);
    bar(
        frame,
        0.0,
        top,
        end * grow,
        c.line(3.0),
        tint(tokens::splash::ACCENT, c.g),
    );
    if failed {
        paint_failure_mark(frame, c, end, grow);
    }
}

/// The cross that closes the baseline on the failure plate: centre `21 s`
/// past the cut, arms `9 s`, the same 3 px stroke as the line it ends.
fn paint_failure_mark(frame: &mut canvas::Frame, c: &Ctx, cut: f32, alpha: f32) {
    let ink = tint(tokens::splash::ACCENT, alpha);
    let stroke = canvas::Stroke::default()
        .with_color(ink)
        .with_width(c.line(3.0));
    let cx = cut + 21.0 * c.s;
    let cy = c.base();
    let arm = 9.0 * c.s;
    for (dx, dy) in [(1.0, 1.0), (1.0, -1.0)] {
        frame.stroke(
            &canvas::Path::line(
                Point::new(cx - arm * dx, cy - arm * dy),
                Point::new(cx + arm * dx, cy + arm * dy),
            ),
            stroke,
        );
    }
}

fn paint_zero(frame: &mut canvas::Frame, c: &Ctx) {
    let p = track(c.t, 400.0, 200.0, |t| t) * c.g;
    if p <= 0.0 {
        return;
    }
    bar(
        frame,
        96.0 * c.s,
        c.base(),
        c.line(3.0),
        14.0 * c.s,
        tint(tokens::splash::ACCENT, p),
    );
}

// ── Text ───────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn label(
    frame: &mut canvas::Frame,
    content: &str,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
    align_x: TextAlign,
    align_y: Vertical,
) {
    label_font(
        frame,
        content,
        x,
        y,
        size,
        color,
        tokens::UI_FONT,
        align_x,
        align_y,
    );
}

/// Same as [`label`], with an explicit face. Only the wordmark needs it: every
/// other string on the plate is the UI face.
#[allow(clippy::too_many_arguments)]
fn label_font(
    frame: &mut canvas::Frame,
    content: &str,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
    font: Font,
    align_x: TextAlign,
    align_y: Vertical,
) {
    if color.a <= 0.0 || size <= 0.0 {
        return;
    }
    frame.fill_text(canvas::Text {
        content: content.to_string(),
        position: Point::new(x.round(), y.round()),
        // `canvas::Text::draw_with` pins `Wrapping::default()` (word wrap)
        // regardless of the field, so the only way to keep a label on one line
        // is a bound wide enough that no break is ever considered.
        max_width: 8000.0,
        color,
        size: Pixels(size),
        line_height: LineHeight::Absolute(Pixels(size)),
        font,
        align_x,
        align_y,
        shaping: Shaping::default(),
        wrapping: Wrapping::None,
        ellipsis: Ellipsis::None,
    });
}

/// Measure a run with the app's real face. Used for the wordmark halves (the
/// dimension line starts at the glyph edge, not at a guessed 910 px) and for
/// the CAD advance.
fn measure(content: &str, size: f32, font: Font) -> f32 {
    let paragraph = <iced::Renderer as iced::advanced::text::Renderer>::Paragraph::with_text(TextSpec {
        content,
        bounds: Size::new(8000.0, 8000.0),
        size: Pixels(size),
        line_height: LineHeight::Absolute(Pixels(size)),
        font,
        align_x: TextAlign::Left,
        align_y: Vertical::Top,
        shaping: Shaping::default(),
        wrapping: Wrapping::None,
        ellipsis: Ellipsis::None,
        hint_factor: None,
    });
    paragraph.min_width()
}

/// Cached half-widths for the current scale. The canvas program survives every
/// frame, so this is paid twice per window size, not twice per frame.
fn wordmark_metrics(view: &SplashView, s: f32) -> (f32, f32) {
    let size = tokens::splash::WORDMARK_SIZE * s;
    if let Some((cached_size, draft, flow)) = view.metrics.get() {
        if (cached_size - size).abs() < 0.01 {
            return (draft, flow);
        }
    }
    let draft = measure("DRAFT", size, tokens::splash::WORDMARK_BOLD);
    let flow = measure("FLOW", size, tokens::splash::WORDMARK_LIGHT);
    view.metrics.set(Some((size, draft, flow)));
    (draft, flow)
}

// ── Animation ──────────────────────────────────────────────────────────────

/// Progress of one track: `0` before `delay`, `ease` once `dur` has passed.
fn track(t: f32, delay: f32, dur: f32, ease: impl Fn(f32) -> f32) -> f32 {
    if t <= delay {
        0.0
    } else {
        ease(((t - delay) / dur).clamp(0.0, 1.0))
    }
}

/// CSS `cubic-bezier(x1, y1, x2, y2)`. The sample's curves are quoted by their
/// control points, so solving them here keeps the entrances identical instead
/// of approximating each one with a named Rust easing function.
fn cubic_bezier(curve: (f32, f32, f32, f32), x: f32) -> f32 {
    let (x1, y1, x2, y2) = curve;
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let sample_x = |t: f32| {
        3.0 * (1.0 - t) * (1.0 - t) * t * x1 + 3.0 * (1.0 - t) * t * t * x2 + t * t * t
    };
    let sample_dx = |t: f32| {
        3.0 * (1.0 - t) * (1.0 - t) * x1
            + 6.0 * (1.0 - t) * t * (x2 - x1)
            + 3.0 * t * t * (1.0 - x2)
    };
    let sample_y = |t: f32| {
        3.0 * (1.0 - t) * (1.0 - t) * t * y1 + 3.0 * (1.0 - t) * t * t * y2 + t * t * t
    };

    // Newton-Raphson on the x polynomial, falling back to bisection where the
    // derivative is flat (the vertical-ish handles in `rise`).
    let mut t = x;
    for _ in 0..6 {
        let dx = sample_x(t) - x;
        if dx.abs() < 1e-5 {
            return sample_y(t);
        }
        let slope = sample_dx(t);
        if slope.abs() < 1e-6 {
            break;
        }
        t = (t - dx / slope).clamp(0.0, 1.0);
    }
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..20 {
        t = (lo + hi) * 0.5;
        if sample_x(t) < x {
            lo = t;
        } else {
            hi = t;
        }
    }
    sample_y(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_matches_the_measured_matrix() {
        // 12-adapt §1.3's window matrix.
        for (w, h, want) in [
            (800.0, 600.0, 0.70),
            (1024.0, 768.0, 0.894),
            (1280.0, 800.0, 1.0),
            (1920.0, 1080.0, 1.35),
            (3840.0, 2160.0, 2.70),
        ] {
            let got = scale(w, h);
            assert!(
                (got - want).abs() < 0.005,
                "{w}×{h} scaled to {got}, expected {want}"
            );
        }
    }

    #[test]
    fn a_width_below_800_drops_the_scale_floor() {
        // The 0.70 floor would leave the wordmark wider than the window; below
        // 800 the plate scales by width alone instead.
        for w in [400.0, 500.0, 640.0, 799.0] {
            assert!(scale(w, 600.0) < 0.70);
        }
    }

    #[test]
    fn the_entrance_ends_before_the_fade_starts() {
        // Every delay in `paint` is bounded by the design axis, so an element
        // added after it would be cut off by the hand-off instead of animating.
        // The start rule is the latest piece: 700 + 600 = 1300, ending exactly
        // on the axis rather than stalling three quarters of the way across.
        let last_delay = 700.0_f32;
        let last_duration = 600.0_f32;
        assert!(last_delay + last_duration <= TIMELINE_MS);
        assert!(last_delay + last_duration >= TIMELINE_MS - 1.0);
        // The playback only stretches: a shorter figure would cut the tail off.
        assert!(INTRO_MS >= TIMELINE_MS);
    }

    /// A stretched plate must keep the same figure: the mapping from wall-clock
    /// to design time is linear, so the last element lands on `INTRO_MS` and
    /// nothing crosses the fade.
    #[test]
    fn the_plate_is_stretched_not_retimed() {
        let speed = INTRO_MS / TIMELINE_MS;
        // Twice the design's speed was the second rejected pace; anything at or
        // below it is back to "rushed". Retuning is allowed, regressing is not.
        assert!(speed >= 2.0, "speed {speed}");
        let last_element = 700.0_f32 + 600.0;
        assert!((last_element * speed - INTRO_MS).abs() < 1e-3);
    }

    #[test]
    fn bezier_hits_both_ends_and_stays_monotonic() {
        for curve in [ease::OUT_QUART, ease::RISE, ease::CAD] {
            assert!(cubic_bezier(curve, 0.0).abs() < 1e-4);
            assert!((cubic_bezier(curve, 1.0) - 1.0).abs() < 1e-4);
            let mut previous = 0.0;
            for step in 0..=100 {
                let y = cubic_bezier(curve, step as f32 / 100.0);
                assert!(
                    y + 1e-3 >= previous,
                    "curve {curve:?} went backwards at {step}: {previous} then {y}"
                );
                previous = y;
            }
        }
    }

    #[test]
    fn the_gear_is_dropped_with_its_two_neighbours() {
        assert!(Level::for_window(1280.0, 800.0).gear);
        assert!(Level::for_window(800.0, 574.0).gear);
        // The last rung where both neighbours still fit under the gear.
        assert!(Level::for_window(720.0, 520.0).gear);
        assert!(!Level::for_window(719.0, 600.0).gear);
        assert!(!Level::for_window(900.0, 519.0).gear);
        assert!(!Level::for_window(560.0, 420.0).gear);
    }

    #[test]
    fn the_gear_keeps_clear_of_every_neighbour_at_every_scale() {
        // The measured box is the pair's union: `tip + 18 s` around each
        // centre, which is where the wheel's construction lines end and well
        // outside the pinion's hub. 24 s of air is kept on every side, the
        // same gap the plate uses elsewhere. Left: revision table (448 s).
        // Right: environment block (w - 190 s). Above: inner frame (40 s).
        // Below: dot field (250 s).
        for (w, h) in [
            (720.0, 520.0),
            (760.0, 560.0),
            (800.0, 574.0),
            (800.0, 600.0),
            (1024.0, 768.0),
            (1280.0, 800.0),
            (1440.0, 900.0),
            (1920.0, 1080.0),
            (2560.0, 1440.0),
            (3840.0, 2160.0),
            (900.0, 1600.0),
        ] {
            assert!(Level::for_window(w, h).gear, "{w}×{h} should keep the gear");
            let s = scale(w, h);
            let (x0, y0, x1, y1) = gear_pair_box(s);
            assert!(
                x0 >= 448.0 * s + 24.0 * s,
                "{w}×{h}: gear overlaps the revision table"
            );
            assert!(
                x1 <= w - 190.0 * s - 24.0 * s,
                "{w}×{h}: gear overlaps the environment block"
            );
            assert!(
                y0 >= 40.0 * s + 8.0 * s,
                "{w}×{h}: gear crosses the inner frame"
            );
            assert!(
                y1 <= 250.0 * s - 8.0 * s,
                "{w}×{h}: gear reaches the dot field"
            );
        }
    }

    #[test]
    fn the_pair_is_a_real_mesh() {
        for s in [0.6_f32, 1.0, 1.8, 3.0] {
            let (wheel, pinion) = gear_pair(s);
            let pitch_w = (wheel.tip + wheel.root) * 0.5;
            let pitch_p = (pinion.tip + pinion.root) * 0.5;
            // Centre distance is the sum of the pitch radii: the two roll on
            // each other instead of overlapping or floating apart.
            let dist = ((pinion.cx - wheel.cx).powi(2) + (pinion.cy - wheel.cy).powi(2)).sqrt();
            assert!(
                (dist - (pitch_w + pitch_p)).abs() < 1e-3 * s,
                "s={s}: centre distance {dist} vs {}",
                pitch_w + pitch_p
            );
            // One module, so the tooth counts alone set the pitch radii.
            assert!(
                (pitch_w / pitch_p - wheel.teeth as f32 / pinion.teeth as f32).abs() < 1e-4,
                "s={s}: the pinion is on a different module"
            );
            // The driven gear turns the other way at exactly the meshing rate.
            let expected = -(pitch_w / pitch_p) * wheel.rate;
            assert!(
                (pinion.rate - expected).abs() < 1e-6,
                "s={s}: rate {} vs {expected}",
                pinion.rate
            );
            // Half a tooth of phase offset turns a wheel tooth into a pinion
            // gap, which is what "in mesh" looks like at t = 0.
            let step_p = std::f32::consts::TAU / pinion.teeth as f32;
            let offset = (pinion.phase - wheel.phase).rem_euclid(step_p);
            assert!(
                (offset - step_p * 0.5).abs() < 1e-4,
                "s={s}: phase offset {offset} is not half a pinion tooth"
            );
        }
    }

    #[test]
    fn the_progress_rule_stays_between_the_text_and_the_title_block() {
        let ctx = |w: f32, h: f32| Ctx {
            w,
            h,
            s: scale(w, h),
            t: 0.0,
            g: 1.0,
        };
        for (w, h) in [
            (560.0, 420.0),
            (800.0, 600.0),
            (1024.0, 768.0),
            (1280.0, 800.0),
            (3840.0, 2160.0),
        ] {
            let c = ctx(w, h);
            let (x0, x1) = progress_span(&c, 110.0 * c.s)
                .unwrap_or_else(|| panic!("{w}×{h} dropped the rule"));
            assert!(x0 >= 96.0 * c.s + 11.0 * c.s, "{w}×{h}: starts on the box");
            assert!(
                x1 <= c.w - 40.0 * c.s - 320.0 * c.s,
                "{w}×{h}: runs into the title block"
            );
            assert!(x1 - x0 >= tokens::splash::PROGRESS_MIN * c.s);
        }
        // A status line long enough to eat the run leaves no stub of a rule.
        let c = ctx(560.0, 420.0);
        assert!(progress_span(&c, 600.0).is_none());
    }

    #[test]
    fn still_mode_holds_the_terminal_plate() {
        let mut splash = Splash {
            clock: Rc::new(Clock {
                t: Cell::new(0.0),
                elapsed: Cell::new(0.0),
            }),
            play: Play::Held(INTRO_MS),
            origin: None,
            verdict: SplashVerdict::Ok,
        };
        assert!(splash.advance(Instant::now()));
        assert_eq!(splash.clock.t.get(), INTRO_MS);
    }

    /// The screenshot hook has to be able to stop the plate mid-entrance:
    /// without that, every capture is the terminal frame and "the gear turns"
    /// or "the rule fills" can only be asserted, never shown.
    #[test]
    fn still_mode_can_freeze_any_instant() {
        let mut splash = Splash {
            clock: Rc::new(Clock {
                t: Cell::new(0.0),
                elapsed: Cell::new(0.0),
            }),
            play: Play::Held(650.0),
            origin: None,
            verdict: SplashVerdict::Ok,
        };
        assert!(splash.advance(Instant::now()));
        assert_eq!(splash.clock.t.get(), 650.0);
        // The clamp keeps a typo from parking the plate past the hand-off.
        let c = Ctx {
            w: 1280.0,
            h: 800.0,
            s: 1.0,
            t: 650.0,
            g: 1.0,
        };
        let (wheel_a, _) = gear_pair(c.s);
        let rot_a = wheel_a.phase + wheel_a.rate * c.t;
        let rot_b = wheel_a.phase + wheel_a.rate * c.t + 200.0 * wheel_a.rate;
        assert!(rot_b > rot_a, "the wheel must keep turning on the clock");
    }

    /// The hand-off rewinds the plate, because frame ticks start before the
    /// window is shown: without the rewind the entrance would already be a few
    /// hundred milliseconds in when the user first sees it.
    #[test]
    fn the_hand_off_rewinds_the_entrance() {
        let mut splash = Splash {
            clock: Rc::new(Clock {
                t: Cell::new(0.0),
                elapsed: Cell::new(0.0),
            }),
            play: Play::Animated,
            origin: None,
            verdict: SplashVerdict::Ok,
        };
        let start = Instant::now();
        assert!(splash.advance(start));
        std::thread::sleep(std::time::Duration::from_millis(20));
        let later = Instant::now();
        assert!(splash.advance(later));
        let before = splash.clock.t.get();
        assert!(before >= 10.0, "the entrance had not moved: {before}");
        splash.restart(later);
        assert_eq!(splash.clock.t.get(), 0.0);
        assert!(splash.advance(later));
        assert!(
            splash.clock.t.get() < before,
            "restart must rewind: {} vs {before}",
            splash.clock.t.get()
        );
    }

    #[test]
    fn quiet_mode_holds_the_terminal_plate_and_never_animates() {
        let mut splash = Splash {
            clock: Rc::new(Clock {
                t: Cell::new(0.0),
                elapsed: Cell::new(0.0),
            }),
            play: Play::Quiet,
            origin: None,
            verdict: SplashVerdict::Ok,
        };
        // The clock starts on the first tick; the entrance never runs.
        assert!(splash.advance(Instant::now()));
        assert_eq!(splash.clock.t.get(), INTRO_MS);
    }

    /// The failure notice is a static告知条: it wins the single status line,
    /// it survives the ready lead-in that upgrades everything else, and it
    /// never claims progress. Those are the three ways the plate could lie
    /// while the graphics warning dialog is still queued.
    #[test]
    fn the_failure_notice_is_never_upgraded_and_never_shows_progress() {
        for t in [
            0.0,
            INTRO_MS - READY_LEAD_MS - 1.0,
            INTRO_MS - READY_LEAD_MS,
            INTRO_MS,
            INTRO_MS + FADE_MS,
        ] {
            let splash = failed_plate_at(t);
            assert_eq!(
                splash.status_at(SplashStatus::Failed),
                SplashStatus::Failed,
                "the failure notice must not become 'ready' at t={t}"
            );
            // Everything else still goes through the normal ready upgrade.
            if t >= INTRO_MS - READY_LEAD_MS {
                assert_eq!(splash.status_at(SplashStatus::Starting), SplashStatus::Ready);
            }
        }
        let all = [
            SplashStatus::Starting,
            SplashStatus::GpuCompat,
            SplashStatus::GpuFallback,
            SplashStatus::SentinelFault,
            SplashStatus::SettingsFault,
            SplashStatus::Ready,
            SplashStatus::Phase(crate::app::OPEN_PHASE_READING),
            SplashStatus::Failed,
        ];
        for other in all {
            if other != SplashStatus::Failed {
                assert!(
                    SplashStatus::Failed.priority() > other.priority(),
                    "the failure notice must outrank {other:?}"
                );
            }
        }
        assert!(!SplashStatus::Failed.text().trim().is_empty());
        assert_ne!(SplashStatus::Failed.text(), SplashStatus::Ready.text());
    }

    /// §1.5: the baseline stops short on the failure plate and the cross sits
    /// `21 s` past the cut. The cut must clear the wordmark on every window —
    /// the narrow case is why it is not just `W − 280 s`.
    #[test]
    fn the_failure_cut_always_clears_the_wordmark_and_stops_short() {
        for (w, h) in [
            (800.0, 600.0),
            (1024.0, 768.0),
            (1280.0, 800.0),
            (1920.0, 1080.0),
            (2560.0, 1440.0),
            (3840.0, 2160.0),
        ] {
            let s = scale(w, h);
            let wordmark_right = w * 0.42;
            let cut = failure_cut(w, s, wordmark_right);
            assert!(
                cut >= wordmark_right + 24.0 * s,
                "{w}×{h}: cut {cut} lands under the wordmark ending at {wordmark_right}"
            );
            assert!(
                cut <= w,
                "{w}×{h}: cut {cut} runs past the plate"
            );
            assert!(
                cut + 21.0 * s + 9.0 * s <= w,
                "{w}×{h}: the cross hangs off the plate"
            );
        }
        // Narrow window: the default cut would sit under the wordmark, so the
        // measured wordmark wins.
        let narrow = failure_cut(800.0, 0.7, 600.0);
        assert!((narrow - 616.8).abs() < 0.01, "narrow cut was {narrow}");
        // Wide window: the default cut is already clear, and it is the one used.
        let wide = failure_cut(1920.0, 1.35, 700.0);
        assert!((wide - 1542.0).abs() < 0.01, "wide cut was {wide}");
    }

    /// The red lines from the董事长令: no skip, no shortened entrance. Pinned
    /// as constants so a "small tweak" has to break a named gate.
    #[test]
    fn the_red_lines_never_move() {
        assert_eq!(TIMELINE_MS, 1300.0);
        assert_eq!(INTRO_MS, 2800.0);
        assert_eq!(FADE_MS, 220.0);
        assert_eq!(QUIET_HOLD_MS, 420.0);
        assert_eq!(INTRO_MS + FADE_MS, 3020.0);
    }

    /// B7-3: the ground takes part in the fade and the workspace is built
    /// underneath for exactly that window, so no frame is ever the clear
    /// colour and the first workspace frame the user sees is already warm.
    #[test]
    fn handoff_fades_as_one_plate_over_a_prewarmed_workspace() {
        for t in [0.0, 700.0, INTRO_MS - 1.0, INTRO_MS] {
            let splash = plate_at(t);
            assert_eq!(plate_alpha(&splash.play, t), 1.0, "alpha before the fade at {t}");
            if t < INTRO_MS {
                assert!(
                    !splash.wants_workspace_below(),
                    "the workspace was built during the entrance at {t}"
                );
            }
        }
        let mut last = 1.0;
        for step in 0..=20 {
            let t = INTRO_MS + FADE_MS * step as f32 / 20.0;
            let splash = plate_at(t);
            let alpha = plate_alpha(&splash.play, t);
            assert!(alpha <= last + 1e-6, "alpha rose at {t}: {last} then {alpha}");
            last = alpha;
            assert!(splash.wants_workspace_below(), "no workspace below at {t}");
            if t < INTRO_MS + FADE_MS {
                assert!(
                    alpha > 0.0,
                    "the frame at {t} shows neither ground nor workspace"
                );
            }
        }
        assert!(
            last <= 1e-6,
            "the hand-off must end fully transparent, got {last}"
        );
        // Screenshot holds are evidence frames, not a launch: they never pay
        // for a workspace behind them.
        let held = Splash {
            play: Play::Held(INTRO_MS),
            ..plate_at(INTRO_MS)
        };
        assert!(!held.wants_workspace_below());
    }

    /// The prewarm happens inside the exit fade, so the launch length is
    /// unchanged: 2800 + 220 = 3020.
    #[test]
    fn handoff_adds_no_time_to_the_launch() {
        assert_eq!(INTRO_MS + FADE_MS, 3020.0);
    }

    /// B7-4: quiet mode still holds 420 ms; only its ending changed, from a
    /// cut to a 120 ms fade inside the same hold.
    #[test]
    fn quiet_fades_within_its_unchanged_hold() {
        assert_eq!(QUIET_HOLD_MS, 420.0);
        assert!(QUIET_FADE_MS > 0.0 && QUIET_FADE_MS <= 120.0);
        assert!(QUIET_FADE_MS < QUIET_HOLD_MS);
        let start = QUIET_HOLD_MS - QUIET_FADE_MS;
        assert_eq!(plate_alpha(&Play::Quiet, 0.0), 1.0);
        assert_eq!(plate_alpha(&Play::Quiet, start), 1.0);
        assert!(plate_alpha(&Play::Quiet, start + QUIET_FADE_MS / 2.0) < 1.0);
        assert!(plate_alpha(&Play::Quiet, QUIET_HOLD_MS) <= 1e-6);

        let quiet_at = |elapsed: f32| Splash {
            clock: Rc::new(Clock {
                t: Cell::new(INTRO_MS),
                elapsed: Cell::new(elapsed),
            }),
            play: Play::Quiet,
            origin: None,
            verdict: SplashVerdict::Ok,
        };
        assert!(!quiet_at(start - 1.0).wants_workspace_below());
        assert!(quiet_at(start).wants_workspace_below());
        assert!(quiet_at(QUIET_HOLD_MS - 1.0).wants_workspace_below());
    }

    #[test]
    fn the_level_order_drops_small_pieces_first() {
        let full = Level::for_window(1280.0, 800.0);
        assert!(full.vertical_dim && full.tables && full.env && full.texture);
        let small = Level::for_window(560.0, 420.0);
        assert!(!small.vertical_dim);
        assert!(!small.tables);
        assert!(!small.env);
        assert!(!small.texture);
        assert!(!small.gear);
        assert!(small.title_full);
        // §1.4 step 7: below 800 the version row would be under 9 physical px.
        assert!(!small.chrome);
        let tiny = Level::for_window(480.0, 400.0);
        assert!(!tiny.title_full);
        assert!(!tiny.chrome);
    }

    fn plate_at(t: f32) -> Splash {
        Splash {
            clock: Rc::new(Clock {
                t: Cell::new(t),
                elapsed: Cell::new(t),
            }),
            play: Play::Animated,
            origin: None,
            verdict: SplashVerdict::Ok,
        }
    }

    fn failed_plate_at(t: f32) -> Splash {
        Splash {
            verdict: SplashVerdict::ProbesFailed,
            ..plate_at(t)
        }
    }

    /// The last quarter second of the entrance is what the user reads before
    /// the workspace appears, so a passive notice upgrades to "ready" there.
    /// This is a copy change only: `advance` still ends the plate on the same
    /// tick (see `the_plate_is_stretched_not_retimed`).
    #[test]
    fn the_last_quarter_second_says_ready() {
        let splash = plate_at(INTRO_MS - READY_LEAD_MS - 1.0);
        assert_eq!(
            splash.status_at(SplashStatus::Starting),
            SplashStatus::Starting,
            "ready must not appear before its lead-in"
        );

        for t in [INTRO_MS - READY_LEAD_MS, INTRO_MS, INTRO_MS + FADE_MS] {
            let splash = plate_at(t);
            for notice in [
                SplashStatus::Starting,
                SplashStatus::GpuCompat,
                SplashStatus::GpuFallback,
                SplashStatus::SentinelFault,
                SplashStatus::SettingsFault,
            ] {
                assert_eq!(
                    splash.status_at(notice),
                    SplashStatus::Ready,
                    "{notice:?} at t={t} must read as delivered"
                );
            }
        }
    }

    /// A drawing still loading keeps its phase through the fade. "Ready" while
    /// the file is still parsing is the one lie this line can tell.
    #[test]
    fn a_drawing_still_loading_never_reads_ready() {
        let splash = plate_at(INTRO_MS + FADE_MS);
        for phase in [
            crate::app::OPEN_PHASE_READING,
            crate::app::OPEN_PHASE_PARSING,
            crate::app::OPEN_PHASE_XREF,
            crate::app::OPEN_PHASE_CACHING,
            crate::app::OPEN_PHASE_FINALIZING,
        ] {
            assert_eq!(
                splash.status_at(SplashStatus::Phase(phase)),
                SplashStatus::Phase(phase),
                "phase {phase} must survive the ready lead-in"
            );
        }
    }

    /// Every status the plate can show resolves to a non-empty line, including
    /// an unknown phase byte arriving from a future open pipeline.
    #[test]
    fn every_status_resolves_to_a_line() {
        for status in [
            SplashStatus::Starting,
            SplashStatus::GpuFallback,
            SplashStatus::GpuCompat,
            SplashStatus::SettingsFault,
            SplashStatus::SentinelFault,
            SplashStatus::Ready,
            SplashStatus::Phase(crate::app::OPEN_PHASE_READING),
            SplashStatus::Phase(crate::app::OPEN_PHASE_PARSING),
            SplashStatus::Phase(crate::app::OPEN_PHASE_XREF),
            SplashStatus::Phase(crate::app::OPEN_PHASE_CACHING),
            SplashStatus::Phase(crate::app::OPEN_PHASE_FINALIZING),
            SplashStatus::Phase(250),
        ] {
            assert!(
                !status.text().trim().is_empty(),
                "{status:?} rendered an empty status line"
            );
        }
    }
}
