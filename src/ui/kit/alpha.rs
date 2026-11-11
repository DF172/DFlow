//! Entrance alpha: the one number a row is built at.
//!
//! iced 0.15 exposes an opacity knob on `svg` and `image` only — there is no
//! layer that fades an arbitrary subtree — and every kit control paints its
//! own ink, so a dock panel cannot fade a row by wrapping it. The two ink
//! sources a row is made of are both built through the kit, so they read this
//! scope instead: the panel opens it once per row and every control and glyph
//! inside arrives at the same alpha.
//!
//! Plain `text()` follows the container's `text_color`, so it needs no help;
//! text the caller coloured itself has to ask [`ink`] for the faded ink.
//!
//! The scope is a thread-local rather than an argument because the kit's
//! constructors are called from ~300 sites that have no opinion about
//! entrances. Nothing outside [`at`] observes a value other than 1.0.

use std::cell::Cell;

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{tree, Tree, Widget};
use iced::widget::{button, checkbox, container, pick_list, text, text_input};
use iced::{
    Background, Color, Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse,
};

thread_local! {
    static ALPHA: Cell<f32> = const { Cell::new(1.0) };
    static PAINTING: Cell<bool> = const { Cell::new(false) };
}

/// Ink multiplier for the row being built; 1.0 outside any scope.
pub fn current() -> f32 {
    ALPHA.with(|a| a.get())
}

/// Builds `f` at `alpha`.
///
/// Nested scopes restore the outer value, so a row inside a faded panel can
/// still paint one cell at full strength (`at(1.0, …)`), and an out-of-range
/// alpha clamps instead of leaking.
pub fn at<T>(alpha: f32, f: impl FnOnce() -> T) -> T {
    let prev = ALPHA.replace(alpha.clamp(0.0, 1.0));
    let out = f();
    ALPHA.set(prev);
    out
}

/// `color` at the current entrance alpha.
pub fn ink(color: Color) -> Color {
    color.scale_alpha(current())
}

/// A widget style that can be painted at less than full strength.
///
/// Implemented for every style a dock row is built from, so the kit's style
/// functions fade their own colours and a panel never has to enumerate them.
pub trait Fade: Sized {
    fn faded(self, alpha: f32) -> Self;
}

/// The one kit style computation that owns a paint.
///
/// Kit style functions call each other — `stripe_style` asks `wash_style` for
/// its plate, `secondary_style` asks `role_style` — so a naive fade would dim
/// the inner plate and then dim the composite again. The outermost function is
/// the owner and fades what it finally returns; the inner ones build their
/// colours untouched. `PAINTING` is cleared by the owner's drop, so a panicking
/// style cannot leave the next frame dimmed.
pub struct Paint {
    owner: bool,
}

impl Paint {
    pub fn start() -> Self {
        Paint {
            owner: !PAINTING.with(|p| p.replace(true)),
        }
    }

    pub fn finish<S: Fade>(&self, style: S) -> S {
        if self.owner {
            style.faded(current())
        } else {
            style
        }
    }
}

impl Drop for Paint {
    fn drop(&mut self) {
        if self.owner {
            PAINTING.with(|p| p.set(false));
        }
    }
}

fn background(background: Option<Background>, alpha: f32) -> Option<Background> {
    background.map(|b| match b {
        Background::Color(color) => Background::Color(color.scale_alpha(alpha)),
        Background::Gradient(gradient) => Background::Gradient(gradient.scale_alpha(alpha)),
    })
}

fn border(border: iced::Border, alpha: f32) -> iced::Border {
    iced::Border {
        color: border.color.scale_alpha(alpha),
        ..border
    }
}

fn shadow(shadow: iced::Shadow, alpha: f32) -> iced::Shadow {
    iced::Shadow {
        color: shadow.color.scale_alpha(alpha),
        ..shadow
    }
}

impl Fade for text::Style {
    fn faded(mut self, alpha: f32) -> Self {
        self.color = self.color.map(|c| c.scale_alpha(alpha));
        self
    }
}

impl Fade for container::Style {
    fn faded(mut self, alpha: f32) -> Self {
        self.text_color = self.text_color.map(|c| c.scale_alpha(alpha));
        self.background = background(self.background, alpha);
        self.border = border(self.border, alpha);
        self.shadow = shadow(self.shadow, alpha);
        self
    }
}

impl Fade for button::Style {
    fn faded(mut self, alpha: f32) -> Self {
        self.background = background(self.background, alpha);
        self.text_color = self.text_color.scale_alpha(alpha);
        self.border = border(self.border, alpha);
        self.shadow = shadow(self.shadow, alpha);
        self
    }
}

impl Fade for text_input::Style {
    fn faded(mut self, alpha: f32) -> Self {
        self.background = match self.background {
            Background::Color(color) => Background::Color(color.scale_alpha(alpha)),
            Background::Gradient(gradient) => Background::Gradient(gradient.scale_alpha(alpha)),
        };
        self.border = border(self.border, alpha);
        self.icon = self.icon.scale_alpha(alpha);
        self.placeholder = self.placeholder.scale_alpha(alpha);
        self.value = self.value.scale_alpha(alpha);
        self.selection = self.selection.scale_alpha(alpha);
        self
    }
}

impl Fade for pick_list::Style {
    fn faded(mut self, alpha: f32) -> Self {
        self.text_color = self.text_color.scale_alpha(alpha);
        self.placeholder_color = self.placeholder_color.scale_alpha(alpha);
        self.handle_color = self.handle_color.scale_alpha(alpha);
        self.background = match self.background {
            Background::Color(color) => Background::Color(color.scale_alpha(alpha)),
            Background::Gradient(gradient) => Background::Gradient(gradient.scale_alpha(alpha)),
        };
        self.border = border(self.border, alpha);
        self
    }
}

impl Fade for checkbox::Style {
    fn faded(mut self, alpha: f32) -> Self {
        self.background = match self.background {
            Background::Color(color) => Background::Color(color.scale_alpha(alpha)),
            Background::Gradient(gradient) => Background::Gradient(gradient.scale_alpha(alpha)),
        };
        self.icon_color = self.icon_color.scale_alpha(alpha);
        self.text_color = self.text_color.map(|c| c.scale_alpha(alpha));
        self.border = border(self.border, alpha);
        self
    }
}

/// Builds one arriving row at `alpha`.
///
/// The scope is opened twice because a row is painted in two passes: the
/// constructors run now (an `svg` carries its opacity as a built field), and
/// the style closures run while the tree draws, long after this scope closed.
/// [`Entrance`] reopens it around the child's draw so the kit's ink solvers
/// see the same number, and hands the child a faded inherited text colour for
/// the plain `text()` that does not ask the kit for one.
pub fn entrance<'a, M: 'a>(
    alpha: f32,
    content: impl FnOnce() -> Element<'a, M>,
) -> Element<'a, M> {
    let alpha = alpha.clamp(0.0, 1.0);
    let content = at(alpha, content);
    Element::new(Entrance { content, alpha })
}

struct Entrance<'a, M> {
    content: Element<'a, M>,
    alpha: f32,
}

impl<'a, M> Widget<M, Theme, Renderer> for Entrance<'a, M> {
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn diff(&mut self, tree: &mut Tree) {
        self.content.as_widget_mut().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
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
        self.content.as_widget_mut().update(
            tree, event, layout, cursor, renderer, shell, viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
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
        let prev = ALPHA.replace(self.alpha);
        let faded = renderer::Style {
            text_color: style.text_color.scale_alpha(self.alpha),
            ..*style
        };
        self.content
            .as_widget()
            .draw(tree, renderer, theme, &faded, layout, cursor, viewport);
        ALPHA.set(prev);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outside_a_scope_nothing_is_faded() {
        assert_eq!(current(), 1.0);
        let ink_color = Color::from_rgb(0.5, 0.5, 0.5);
        assert_eq!(ink(ink_color), ink_color);
    }

    #[test]
    fn a_scope_fades_ink_and_restores_the_previous_value() {
        let full = Color::WHITE;
        let half = at(0.5, || {
            assert!((current() - 0.5).abs() < 1e-6);
            ink(full)
        });
        assert!((half.a - 0.5).abs() < 1e-6, "ink alpha {} is not 0.5", half.a);
        assert_eq!(current(), 1.0, "the scope must restore");
    }

    #[test]
    fn nested_scopes_restore_in_order() {
        at(0.25, || {
            at(0.75, || assert!((current() - 0.75).abs() < 1e-6));
            assert!((current() - 0.25).abs() < 1e-6, "inner scope leaked");
        });
        assert_eq!(current(), 1.0);
    }

    #[test]
    fn opaque_scopes_do_not_stack_up() {
        let c = at(1.0, || at(1.0, current));
        assert_eq!(c, 1.0);
        let c = at(0.0, || at(2.0, current));
        assert_eq!(c, 1.0, "an out-of-range alpha clamps to opaque");
        let c = at(0.0, || at(-3.0, current));
        assert_eq!(c, 0.0, "a negative alpha clamps to invisible");
    }

    /// The kit styles call each other, so only the outermost may apply the
    /// entrance alpha or a stripe would be dimmed twice.
    #[test]
    fn nested_paints_fade_exactly_once() {
        at(0.5, || {
            let outer = Paint::start();
            let inner = Paint::start();
            let solved = inner.finish(text::Style {
                color: Some(Color::WHITE),
            });
            assert_eq!(solved.color.unwrap().a, 1.0, "a nested style must stay whole");
            drop(inner);
            let painted = outer.finish(solved);
            assert!(
                (painted.color.unwrap().a - 0.5).abs() < 1e-6,
                "the owner must fade once, got {:?}",
                painted.color
            );
        });
        assert!(!PAINTING.with(|p| p.get()), "the paint must be released");
    }

    /// A paint reads the scope where the style is solved, which is the row's
    /// draw when the kit is driven from a panel.
    #[test]
    fn a_paint_reads_the_scope_it_is_solved_in() {
        let inside = at(0.4, || {
            let paint = Paint::start();
            paint.finish(text::Style {
                color: Some(Color::WHITE),
            })
        });
        assert!((inside.color.unwrap().a - 0.4).abs() < 1e-6);
        let paint = Paint::start();
        let outside = paint.finish(text::Style {
            color: Some(Color::WHITE),
        });
        assert_eq!(outside.color.unwrap().a, 1.0, "an unscoped paint is opaque");
    }

    /// Every fadeable field has to move, or a row shows a full-strength plate
    /// under faded text.
    #[test]
    fn fading_covers_plate_border_and_ink() {
        let alpha = 0.5;
        let plate = Background::Color(Color::from_rgba(0.2, 0.2, 0.2, 1.0));
        let frame = iced::Border {
            color: Color::WHITE,
            width: 1.0,
            radius: 2.0.into(),
        };
        let drop = iced::Shadow {
            color: Color::BLACK,
            offset: iced::Vector::new(0.0, 1.0),
            blur_radius: 2.0,
        };
        let style = container::Style {
            text_color: Some(Color::WHITE),
            background: Some(plate),
            border: frame,
            shadow: drop,
            snap: true,
        }
        .faded(alpha);
        let bg = style.background.unwrap();
        assert!(matches!(bg, Background::Color(c) if (c.a - 0.5).abs() < 1e-6));
        assert!((style.border.color.a - 0.5).abs() < 1e-6);
        assert!((style.shadow.color.a - alpha).abs() < 1e-6);
        assert!((style.text_color.unwrap().a - 0.5).abs() < 1e-6);
        assert!(style.snap, "fading must not disturb non-colour fields");
    }
}
