//! Pure decisions for the viewport feedback layer.
//!
//! Everything the 2D overlay shows on top of the drawing — the OS cursor
//! shape, the snap marker's ink, the crosshair's form, a grip's focus — is a
//! function of the current frame's state alone. No clock, no easing, no
//! allocation: the same input must always yield the same output, which is what
//! keeps picking at 0 ms and lets the whole matrix be asserted one case at a
//! time.
//!
//! Drawing stays in `overlay.rs`; this module only decides *what* to draw.

use crate::app::settings::CursorType;
use crate::snap::SnapType;
use crate::ui::overlay::NavCursor;
use iced::{mouse, Point, Rectangle};

// ── Cursor ────────────────────────────────────────────────────────────────

/// What the pointer looks like over the viewport this frame.
#[derive(Clone, Copy, Debug)]
pub struct CursorInput {
    /// A modal or a pending client pick has taken the canvas over.
    pub suppressed: bool,
    /// Pointer is inside the viewport rectangle.
    pub over: bool,
    /// Which navigation tool is armed.
    pub nav: NavCursor,
    /// Middle button held (pan / zoom drag in progress).
    pub dragging: bool,
    /// Zoom tool with the out direction armed.
    pub zoom_out: bool,
    /// Pointer is over the ViewCube.
    pub viewcube_hit: bool,
    /// Pointer is over a constraint glyph.
    pub glyph_hit: bool,
    /// The user's configured cursor for the drawing area.
    pub cursor_type: CursorType,
}

pub fn cursor(input: CursorInput) -> mouse::Interaction {
    if input.suppressed {
        return mouse::Interaction::default();
    }
    if input.nav != NavCursor::None && input.over {
        return match input.nav {
            NavCursor::Pan => {
                if input.dragging {
                    mouse::Interaction::Grabbing
                } else {
                    mouse::Interaction::Grab
                }
            }
            NavCursor::Orbit => mouse::Interaction::AllScroll,
            NavCursor::Zoom => {
                if input.dragging && input.zoom_out {
                    mouse::Interaction::ZoomOut
                } else {
                    mouse::Interaction::ZoomIn
                }
            }
            NavCursor::None => unreachable!("guarded by the `!= None` test above"),
        };
    }
    if input.viewcube_hit {
        return mouse::Interaction::None;
    }
    if input.glyph_hit {
        return mouse::Interaction::Pointer;
    }
    if input.over && input.cursor_type == CursorType::Crosshair {
        mouse::Interaction::Hidden
    } else {
        mouse::Interaction::default()
    }
}

// ── Snap marker ───────────────────────────────────────────────────────────

/// Ink and weight for a snap marker. Colours are the user's own (or the
/// canvas-adaptive fallback); this type only carries them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapStyle {
    pub rgb: [f32; 3],
    pub width: f32,
}

pub fn snap_style(kind: SnapType, user: Option<[u8; 3]>, over_light_canvas: bool) -> SnapStyle {
    let rgb = if kind == SnapType::ObjectPick {
        [0.95, 0.50, 0.08]
    } else if let Some([r, g, b]) = user {
        [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
    } else if over_light_canvas {
        // Dark amber: the classic yellow vanishes on a light sheet.
        [0.80, 0.45, 0.0]
    } else {
        [1.0, 0.9, 0.1]
    };
    SnapStyle {
        rgb,
        width: if kind == SnapType::ObjectPick { 2.0 } else { 1.5 },
    }
}

/// Keep a marker's ink inside the viewport by at least `margin` px, so the
/// stroke never lands on the clip edge and disappears at 1× DPI.
pub fn clamp_marker(center: Point, radius: f32, viewport: Rectangle) -> Point {
    // One extra pixel of slack so the stroke's own width never lands on the
    // clip edge, where it would half-vanish.
    let inset = (radius + 1.0).max(0.0);
    let x = center.x.clamp(
        viewport.x + inset,
        (viewport.x + viewport.width - inset).max(viewport.x + inset),
    );
    let y = center.y.clamp(
        viewport.y + inset,
        (viewport.y + viewport.height - inset).max(viewport.y + inset),
    );
    Point::new(x, y)
}

// ── Crosshair ─────────────────────────────────────────────────────────────

/// The crosshair's four visual forms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrosshairForm {
    /// Nothing draws (suppressed canvas).
    Hidden,
    /// Client pick waiting: pickbox only, no arms.
    PendingPick,
    /// Point mode: a dot instead of a box.
    Point,
    /// Normal crosshair: arms plus pickbox.
    Arms,
}

pub fn crosshair_form(suppressed: bool, pick_pending: bool, point_mode: bool) -> CrosshairForm {
    if suppressed {
        CrosshairForm::Hidden
    } else if pick_pending {
        CrosshairForm::PendingPick
    } else if point_mode {
        CrosshairForm::Point
    } else {
        CrosshairForm::Arms
    }
}

/// Whether the crosshair's arms draw in this form. Point mode swaps the
/// pickbox for a dot; it keeps the arms, which is what distinguishes it from
/// a pending pick.
pub fn crosshair_arms(form: CrosshairForm) -> bool {
    matches!(form, CrosshairForm::Arms | CrosshairForm::Point)
}

pub fn crosshair_rgb(form: CrosshairForm, base: [f32; 3], pending_blue: [f32; 3]) -> [f32; 3] {
    match form {
        CrosshairForm::PendingPick => pending_blue,
        CrosshairForm::Hidden | CrosshairForm::Point | CrosshairForm::Arms => base,
    }
}

// ── Grips ─────────────────────────────────────────────────────────────────

/// A grip's three discrete states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GripFocus {
    Idle,
    Hovered,
    Hot,
}

pub fn grip_focus(is_hot: bool, is_hovered: bool) -> GripFocus {
    if is_hot {
        GripFocus::Hot
    } else if is_hovered {
        GripFocus::Hovered
    } else {
        GripFocus::Idle
    }
}

/// Whether a grip centred at `center` with `half_extent` px of ink overlaps the
/// viewport. Drawn only when true; the test below pins the boundary.
pub fn grip_in_view(viewport: Rectangle, center: Point, half_extent: f32) -> bool {
    center.x + half_extent >= viewport.x
        && center.x - half_extent <= viewport.x + viewport.width
        && center.y + half_extent >= viewport.y
        && center.y - half_extent <= viewport.y + viewport.height
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::motion;
    use iced::Size;

    const ALL_SNAP: [SnapType; 22] = [
        SnapType::Endpoint,
        SnapType::Midpoint,
        SnapType::Center,
        SnapType::GeometricCenter,
        SnapType::Node,
        SnapType::Quadrant,
        SnapType::Intersection,
        SnapType::Extension,
        SnapType::Insertion,
        SnapType::Perpendicular,
        SnapType::Tangent,
        SnapType::Nearest,
        SnapType::ApparentIntersection,
        SnapType::Parallel,
        SnapType::Grid,
        SnapType::ObjectPick,
        SnapType::Vertex,
        SnapType::EdgeMidpoint,
        SnapType::FaceCenter,
        SnapType::Knot,
        SnapType::FacePerpendicular,
        SnapType::NearestFace,
    ];

    fn base_cursor() -> CursorInput {
        CursorInput {
            suppressed: false,
            over: true,
            nav: NavCursor::None,
            dragging: false,
            zoom_out: false,
            viewcube_hit: false,
            glyph_hit: false,
            cursor_type: CursorType::Crosshair,
        }
    }

    #[test]
    fn snap_type_matrix_is_exhaustive() {
        assert_eq!(ALL_SNAP.len(), 22, "the enum grew; extend this matrix");
        let mut special = 0;
        for kind in ALL_SNAP {
            let s = snap_style(kind, Some([12, 200, 99]), false);
            assert!(s.rgb.iter().all(|c| c.is_finite() && (0.0..=1.0).contains(c)));
            if kind == SnapType::ObjectPick {
                assert_eq!(s.rgb, [0.95, 0.50, 0.08], "object pick keeps its orange");
                assert_eq!(s.width, 2.0);
                special += 1;
            } else {
                assert_eq!(
                    s.rgb,
                    [12.0 / 255.0, 200.0 / 255.0, 99.0 / 255.0],
                    "{kind:?} must honour the user's snap colour"
                );
                assert_eq!(s.width, 1.5);
            }
        }
        assert_eq!(special, 1, "only ObjectPick carries the heavy orange marker");
    }

    #[test]
    fn snap_falls_back_per_canvas_lightness() {
        assert_eq!(snap_style(SnapType::Endpoint, None, false).rgb, [1.0, 0.9, 0.1]);
        assert_eq!(snap_style(SnapType::Endpoint, None, true).rgb, [0.80, 0.45, 0.0]);
        assert_eq!(
            snap_style(SnapType::ObjectPick, Some([1, 2, 3]), true).rgb,
            [0.95, 0.50, 0.08],
            "the object-pick marker never takes the user colour"
        );
    }

    #[test]
    fn cursor_matrix_covers_eight_interactions() {
        use mouse::Interaction;
        let mut got = Vec::new();

        let mut i = base_cursor();
        i.suppressed = true;
        got.push(cursor(i));
        assert_eq!(cursor(i), Interaction::default());

        i = base_cursor();
        i.nav = NavCursor::Pan;
        got.push(cursor(i));
        assert_eq!(cursor(i), Interaction::Grab);
        i.dragging = true;
        got.push(cursor(i));
        assert_eq!(cursor(i), Interaction::Grabbing);

        i = base_cursor();
        i.nav = NavCursor::Orbit;
        got.push(cursor(i));
        assert_eq!(cursor(i), Interaction::AllScroll);

        i = base_cursor();
        i.nav = NavCursor::Zoom;
        i.dragging = true;
        i.zoom_out = true;
        got.push(cursor(i));
        assert_eq!(cursor(i), Interaction::ZoomOut);
        i.zoom_out = false;
        got.push(cursor(i));
        assert_eq!(cursor(i), Interaction::ZoomIn);

        i = base_cursor();
        i.viewcube_hit = true;
        got.push(cursor(i));
        assert_eq!(cursor(i), Interaction::None);

        i = base_cursor();
        i.glyph_hit = true;
        got.push(cursor(i));
        assert_eq!(cursor(i), Interaction::Pointer);

        i = base_cursor();
        got.push(cursor(i));
        assert_eq!(cursor(i), Interaction::Hidden);

        i = base_cursor();
        i.over = false;
        got.push(cursor(i));
        assert_eq!(cursor(i), Interaction::default());

        i = base_cursor();
        i.cursor_type = CursorType::Pointer;
        got.push(cursor(i));
        assert_eq!(cursor(i), Interaction::default());

        let mut unique: Vec<Interaction> = Vec::new();
        for g in got {
            if !unique.contains(&g) {
                unique.push(g);
            }
        }
        assert_eq!(
            unique.len(),
            8,
            "the canvas must reach eight distinct interactions: {unique:?}"
        );
    }

    #[test]
    fn nav_only_applies_inside_the_viewport() {
        let mut i = base_cursor();
        i.nav = NavCursor::Pan;
        i.over = false;
        assert_eq!(cursor(i), mouse::Interaction::default());
    }

    #[test]
    fn crosshair_has_four_forms_and_one_colour_rule() {
        let blue = [0.30, 0.55, 0.98];
        let base = [0.9, 0.9, 0.9];
        assert_eq!(crosshair_form(true, true, true), CrosshairForm::Hidden);
        assert_eq!(crosshair_form(false, true, true), CrosshairForm::PendingPick);
        assert_eq!(crosshair_form(false, false, true), CrosshairForm::Point);
        assert_eq!(crosshair_form(false, false, false), CrosshairForm::Arms);

        assert!(!crosshair_arms(CrosshairForm::Hidden));
        assert!(!crosshair_arms(CrosshairForm::PendingPick));
        assert!(
            crosshair_arms(CrosshairForm::Point),
            "point mode is a dot instead of a box, not a bare point"
        );
        assert!(crosshair_arms(CrosshairForm::Arms));

        assert_eq!(crosshair_rgb(CrosshairForm::Arms, base, blue), base);
        assert_eq!(crosshair_rgb(CrosshairForm::Point, base, blue), base);
        assert_eq!(crosshair_rgb(CrosshairForm::PendingPick, base, blue), blue);
        assert_eq!(crosshair_rgb(CrosshairForm::Hidden, base, blue), base);
    }

    #[test]
    fn grip_focus_is_discrete() {
        assert_eq!(grip_focus(false, false), GripFocus::Idle);
        assert_eq!(grip_focus(false, true), GripFocus::Hovered);
        assert_eq!(grip_focus(true, false), GripFocus::Hot);
        assert_eq!(grip_focus(true, true), GripFocus::Hot, "hot wins over hover");
    }

    #[test]
    fn grip_culling_matches_the_draw_bounds() {
        let vp = Rectangle::new(Point::new(0.0, 0.0), Size::new(800.0, 600.0));
        assert!(grip_in_view(vp, Point::new(400.0, 300.0), 5.0));
        assert!(grip_in_view(vp, Point::new(-4.0, 300.0), 5.0), "half on screen");
        assert!(!grip_in_view(vp, Point::new(-6.0, 300.0), 5.0));
        assert!(!grip_in_view(vp, Point::new(806.0, 300.0), 5.0));
    }

    #[test]
    fn marker_is_clamped_inside_the_viewport() {
        let vp = Rectangle::new(Point::new(0.0, 0.0), Size::new(800.0, 600.0));
        assert_eq!(clamp_marker(Point::new(400.0, 300.0), 6.0, vp), Point::new(400.0, 300.0));
        assert_eq!(clamp_marker(Point::new(-40.0, 300.0), 6.0, vp), Point::new(7.0, 300.0));
        assert_eq!(clamp_marker(Point::new(400.0, 900.0), 6.0, vp), Point::new(400.0, 593.0));
    }

    #[test]
    fn canvas_feedback_starts_no_motion_track() {
        motion::set_enabled(true);
        for _ in 0..1000 {
            let mut i = base_cursor();
            i.nav = NavCursor::Pan;
            std::hint::black_box(cursor(i));
            std::hint::black_box(snap_style(SnapType::Endpoint, None, false));
            std::hint::black_box(crosshair_form(false, true, false));
            std::hint::black_box(grip_focus(true, false));
        }
        assert!(
            !motion::any_active(),
            "canvas feedback must not own a motion track"
        );
    }

    /// The decisions above are only 0 ms if the module cannot schedule work:
    /// no clock, no easing, no heap. This scans the production half of the
    /// file so a future edit cannot quietly reintroduce them.
    #[test]
    fn production_half_has_no_clock_motion_or_heap() {
        let src = include_str!("canvas_fx.rs");
        let prod = src.split("#[cfg(test)]").next().unwrap();
        for banned in ["Instant", "motion::", "format!", "Vec::", "Box::leak", "String"] {
            assert!(
                !prod.contains(banned),
                "canvas_fx production code must not mention `{banned}`"
            );
        }
    }
}
