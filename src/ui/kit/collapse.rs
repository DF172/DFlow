//! Height-collapsing panel body: a widget whose reported height is its
//! content's natural height times a progress value, with the content clipped
//! to the shoelace.
//!
//! This exists because the properties accordion must grow and shrink *by the
//! frame*: the rows slide under the header while the scrollbar above them
//! follows, and the parent column reflows on the same tick. `max_height` is a
//! layout-time constant in iced, so switching it produces a hard cut, not an
//! interpolation — and animating the container's height would clip the child
//! without giving it its own natural pass. Here the child keeps its own layout
//! (full height, whatever the row stack measures), the parent sees `h * p`,
//! and the draw clips to that.
//!
//! Events are forwarded only while the body has height, so a closed section's
//! fields cannot take focus through a zero-height node.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Tree, Widget};
use iced::advanced::Renderer as _;
use iced::{mouse, Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector};

/// Wraps `content` so it occupies `progress ×` its natural height.
///
/// `progress` is read by the caller every frame (usually from a
/// [`crate::ui::motion`] track), so the widget itself holds no clock.
pub fn collapse<'a, M: 'a>(
    progress: f32,
    content: Element<'a, M>,
) -> Element<'a, M> {
    Element::new(Collapse {
        content,
        progress: progress.clamp(0.0, 1.0),
    })
}

struct Collapse<'a, M> {
    content: Element<'a, M>,
    progress: f32,
}

impl<M> Widget<M, Theme, Renderer> for Collapse<'_, M> {
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
        let full = child.size();
        let height = if self.progress <= 0.0 {
            0.0
        } else {
            full.height * self.progress
        };
        layout::Node::with_children(Size::new(full.width, height), vec![child])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        if self.progress <= 0.0 {
            return;
        }
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
        if self.progress <= 0.0 {
            return;
        }
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
        if self.progress <= 0.0 {
            return mouse::Interaction::None;
        }
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
        if self.progress <= 0.0 {
            return;
        }
        let Some(child) = layout.children().next() else {
            return;
        };
        let bounds = layout.bounds();
        let visible = bounds.intersection(viewport).unwrap_or(bounds);
        renderer.with_layer(visible, |renderer| {
            self.content
                .as_widget()
                .draw(&tree.children[0], renderer, theme, style, child, cursor, viewport);
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
        if self.progress <= 0.0 {
            return None;
        }
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
