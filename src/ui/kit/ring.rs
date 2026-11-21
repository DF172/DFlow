//! A progress ring drawn on the rim of the control it reports on.
//!
//! The open-file progress used to be a modal card in the middle of the
//! workspace. Here it lives on the Open button itself: the same measured
//! fraction the card's bar read is stroked clockwise around the button's own
//! rectangle, so the thing spinning is the thing that was clicked.
//!
//! The ring never changes the button's layout. It strokes the rim *inside* the
//! control's bounds ([`RING_INSET`] + [`RING_WIDTH`] ≈ two pixels), because the
//! ribbon strip's height is a frozen rung and a ring that pushed the row down
//! would move every tool row under it for the duration of a file load.
//!
//! The perimeter walk is exported ([`perimeter_length`], [`perimeter_point`])
//! so the geometry can be asserted without a renderer: a ring that starts
//! anywhere but 12 o'clock or that overruns its own perimeter is a defect the
//! tests should catch, not one the eye has to.

use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Tree, Widget};
use iced::advanced::Renderer as _;
use iced::widget::canvas::{self, Frame, Path, Stroke};
use iced::{
    mouse, Color, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector,
};

/// Distance from the control's edge to the stroke's centre line.
pub const RING_INSET: f32 = 1.0;
/// Stroke width of both the track and the measured arc.
pub const RING_WIDTH: f32 = 1.8;
/// Corner radius of the rim. The chrome's quick-access buttons are 24 px high;
/// 4 px keeps the ring's corners inside theirs at every size.
pub const RING_RADIUS: f32 = 4.0;

/// What to paint on the rim this frame.
#[derive(Clone, Copy, Debug)]
pub struct Ring {
    /// Measured progress, `0.0..=1.0`. Ignored when `sweep` is set.
    pub fraction: f32,
    /// Indeterminate sweep phase, `0.0..=1.0`. `Some` while the file's size is
    /// not yet known, matching the card's rule (`basis_points == 0`).
    pub sweep: Option<f32>,
    /// Fade of the whole ring, read from the `OPEN_RING` track so the ring
    /// arrives and leaves without a hard cut.
    pub alpha: f32,
}

impl Ring {
    /// The value at rest: nothing measured, nothing drawn.
    pub const NONE: Self = Self {
        fraction: 0.0,
        sweep: None,
        alpha: 0.0,
    };

    fn visible(&self) -> bool {
        self.alpha > 0.002
    }

    /// `(start, span)` along the perimeter, in fraction units of one lap.
    fn arc(&self) -> (f32, f32) {
        if let Some(phase) = self.sweep {
            if phase.is_finite() {
                return (phase.rem_euclid(1.0), 0.25);
            }
            return (0.0, 0.25);
        }
        let f = if self.fraction.is_finite() {
            self.fraction.clamp(0.0, 1.0)
        } else {
            0.0
        };
        (0.0, f)
    }
}

/// Wraps `content` and strokes a progress rim on its rectangle.
pub fn progress_ring<'a, M: 'a>(
    content: Element<'a, M>,
    ring: Ring,
    accent: Color,
) -> Element<'a, M> {
    Element::new(RingWidget {
        content,
        ring,
        accent,
    })
}

struct RingWidget<'a, M> {
    content: Element<'a, M>,
    ring: Ring,
    accent: Color,
}

/// Length of the rounded-rectangle perimeter with corner radius `r`.
pub fn perimeter_length(w: f32, h: f32, r: f32) -> f32 {
    let r = r.min(w * 0.5).min(h * 0.5).max(0.0);
    let straight = 2.0 * (w - 2.0 * r) + 2.0 * (h - 2.0 * r);
    straight + std::f32::consts::TAU * r
}

/// The point `s` pixels along the perimeter, starting at top-centre and walking
/// clockwise. Always returns a point on or inside the `w × h` rectangle.
pub fn perimeter_point(s: f32, w: f32, h: f32, r: f32) -> Point {
    if w <= 0.0 || h <= 0.0 {
        return Point::ORIGIN;
    }
    let r = r.min(w * 0.5).min(h * 0.5).max(0.0);
    let total = perimeter_length(w, h, r);
    let mut s = if s.is_finite() {
        s.rem_euclid(total)
    } else {
        0.0
    };
    let quarter = std::f32::consts::FRAC_PI_2 * r;

    let half_top = w * 0.5 - r;
    if s <= half_top {
        return Point::new(w * 0.5 + s, 0.0);
    }
    s -= half_top;

    if s <= quarter {
        let a = -std::f32::consts::FRAC_PI_2 + s / r;
        return Point::new(w - r + r * a.cos(), r + r * a.sin());
    }
    s -= quarter;

    let side = h - 2.0 * r;
    if s <= side {
        return Point::new(w, r + s);
    }
    s -= side;

    if s <= quarter {
        let a = s / r;
        return Point::new(w - r + r * a.cos(), h - r + r * a.sin());
    }
    s -= quarter;

    let bottom = w - 2.0 * r;
    if s <= bottom {
        return Point::new(w - r - s, h);
    }
    s -= bottom;

    if s <= quarter {
        let a = std::f32::consts::FRAC_PI_2 + s / r;
        return Point::new(r + r * a.cos(), h - r + r * a.sin());
    }
    s -= quarter;

    if s <= side {
        return Point::new(0.0, h - r - s);
    }
    s -= side;

    if s <= quarter {
        let a = std::f32::consts::PI + s / r;
        return Point::new(r + r * a.cos(), r + r * a.sin());
    }
    s -= quarter;

    // The last straight run closes the lap at top-centre.
    Point::new(r + s.min(half_top), 0.0)
}

/// A partial lap as a polyline. The sample spacing is capped in pixels so a
/// long rim does not turn into a visible polygon and a small one does not
/// spend hundreds of points.
fn arc_path(start: f32, span: f32, w: f32, h: f32, r: f32, offset: Vector) -> Path {
    let total = perimeter_length(w, h, r);
    let run = span.clamp(0.0, 1.0) * total;
    let steps = ((run / 1.5).ceil() as usize).clamp(2, 512);
    Path::new(|builder| {
        for i in 0..=steps {
            let s = (start * total) + run * (i as f32 / steps as f32);
            let p = perimeter_point(s, w, h, r) + offset;
            if i == 0 {
                builder.move_to(p);
            } else {
                builder.line_to(p);
            }
        }
    })
}

impl<M> Widget<M, Theme, Renderer> for RingWidget<'_, M> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.content));
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let child = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        layout::Node::with_children(child.size(), vec![child])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        if let Some(child) = layout.children().next() {
            self.content
                .as_widget_mut()
                .operate(&mut tree.children[0], child, renderer, operation);
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut iced::advanced::Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        if let Some(child) = layout.children().next() {
            self.content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                child,
                cursor,
                renderer,
                shell,
                viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        layout
            .children()
            .next()
            .map_or(mouse::Interaction::None, |child| {
                self.content.as_widget().mouse_interaction(
                    &tree.children[0],
                    child,
                    cursor,
                    viewport,
                    renderer,
                )
            })
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let Some(child) = layout.children().next() else {
            return;
        };
        self.content
            .as_widget()
            .draw(&tree.children[0], renderer, theme, style, child, cursor, viewport);

        if !self.ring.visible() {
            return;
        }
        let bounds = layout.bounds();
        let w = bounds.width - 2.0 * RING_INSET;
        let h = bounds.height - 2.0 * RING_INSET;
        if w <= 1.0 || h <= 1.0 {
            return;
        }
        let r = RING_RADIUS.min(w * 0.5).min(h * 0.5);
        let offset = Vector::new(RING_INSET, RING_INSET);
        let alpha = self.ring.alpha.clamp(0.0, 1.0);

        let track = Path::rounded_rectangle(
            Point::new(RING_INSET, RING_INSET),
            Size::new(w, h),
            iced::border::Radius::from(r),
        );
        let (start, span) = self.ring.arc();
        let arc = arc_path(start, span, w, h, r, offset);

        let mut frame = Frame::new(renderer, bounds.size());
        frame.stroke(
            &track,
            Stroke::default()
                .with_color(self.accent.scale_alpha(0.22 * alpha))
                .with_width(RING_WIDTH),
        );
        if span > 0.0 {
            frame.stroke(
                &arc,
                Stroke::default()
                    .with_color(self.accent.scale_alpha(alpha))
                    .with_width(RING_WIDTH)
                    .with_line_cap(canvas::LineCap::Round),
            );
        }
        let geometry = frame.into_geometry();
        renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
            renderer.draw_geometry(geometry);
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, Renderer>> {
        let child = layout.children().next()?;
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            child,
            renderer,
            viewport,
            translation,
        )
    }
}

#[cfg(test)]
mod ring_tests {
    use super::*;

    #[test]
    fn the_perimeter_starts_at_twelve_oclock_and_walks_clockwise() {
        let (w, h, r) = (24.0, 24.0, 4.0);
        let p0 = perimeter_point(0.0, w, h, r);
        assert!((p0.x - w / 2.0).abs() < 1e-3 && p0.y.abs() < 1e-3, "start {p0:?}");
        // A quarter of the way round a square-ish ring is the right edge.
        let q = perimeter_point(perimeter_length(w, h, r) * 0.25, w, h, r);
        assert!(q.x > w - r - 1e-3, "a quarter lap should reach the right rim: {q:?}");
    }

    #[test]
    fn every_sample_sits_on_or_inside_the_rectangle() {
        for (w, h) in [(24.0, 24.0), (48.0, 24.0), (12.0, 24.0)] {
            let r = RING_RADIUS.min(w * 0.5).min(h * 0.5);
            let total = perimeter_length(w, h, r);
            for step in 0..=400 {
                let p = perimeter_point(total * step as f32 / 400.0, w, h, r);
                assert!(
                    p.x >= -1e-3 && p.y >= -1e-3 && p.x <= w + 1e-3 && p.y <= h + 1e-3,
                    "{w}x{h} sample {step} left the rect at {p:?}"
                );
            }
        }
    }

    #[test]
    fn bad_input_never_produces_a_panicking_geometry() {
        for s in [f32::NAN, f32::INFINITY, -5.0] {
            let p = perimeter_point(s, 0.0, 10.0, 4.0);
            assert_eq!(p, Point::ORIGIN);
        }
        let ring = Ring {
            fraction: f32::NAN,
            sweep: Some(f32::NAN),
            alpha: 1.0,
        };
        let (start, span) = ring.arc();
        assert!(start.is_finite() && span.is_finite());
    }

    #[test]
    fn an_unmeasured_open_sweeps_and_a_measured_one_fills() {
        let swept = Ring {
            fraction: 0.0,
            sweep: Some(0.3),
            alpha: 1.0,
        };
        assert_eq!(swept.arc(), (0.3, 0.25));
        let filled = Ring {
            fraction: 0.4,
            sweep: None,
            alpha: 1.0,
        };
        assert_eq!(filled.arc(), (0.0, 0.4));
    }
}
