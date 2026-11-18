//! Frame sizing: the intrinsic measuring widget the dialog layer lays itself out
//! with. Owns no colour and no token value.

use crate::app::Message;
use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{self, Widget};
use iced::advanced::{mouse, overlay, renderer, Shell};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector};

/// Sizing used inside an intrinsically measured shell frame.
///
/// [`INTRINSIC`](Self::INTRINSIC) lets each widget report the most space it can
/// use, while [`FILL`](Self::FILL) distributes the resulting shared frame to
/// sibling toolbars, panes and scroll areas.
#[derive(Debug, Clone, Copy)]
pub struct Sizing {
    pub width: Length,
    pub height: Length,
}

impl Sizing {
    pub const INTRINSIC: Self = Self {
        width: Length::Fluid(iced_core::length::Constraint::Max),
        height: Length::Fluid(iced_core::length::Constraint::Max),
    };

    pub const FILL: Self = Self {
        width: Length::Fill,
        height: Length::Fill,
    };
}

/// Measure an intrinsic copy of `content`, then lay the fill copy out in the
/// resulting shared frame. `extra` grows that frame when the resize handle is
/// dragged; `max` only caps the initial intrinsic pass.
pub fn intrinsic<'a>(
    measurement: impl Into<Element<'a, Message>>,
    content: impl Into<Element<'a, Message>>,
    max: Size,
    extra: Vector,
) -> Element<'a, Message> {
    Element::new(Intrinsic {
        children: [measurement.into(), content.into()],
        max,
        extra,
    })
}

struct Intrinsic<'a> {
    children: [Element<'a, Message>; 2],
    max: Size,
    extra: Vector,
}

impl Widget<Message, Theme, Renderer> for Intrinsic<'_> {
    fn diff(&mut self, tree: &mut widget::Tree) {
        tree.diff_children(&mut self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(
            Length::Fit.max(self.max.width + self.extra.x),
            Length::Fit.max(self.max.height + self.extra.y),
        )
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let measure_max = Size::new(
            self.max.width.min(limits.max().width),
            self.max.height.min(limits.max().height),
        );
        let measure_limits = layout::Limits::new(Size::ZERO, measure_max);
        let measured = self.children[0].as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &measure_limits,
        );
        let size = Size::new(
            (measured.size().width + self.extra.x).min(limits.max().width),
            (measured.size().height + self.extra.y).min(limits.max().height),
        );
        let final_limits = layout::Limits::new(size, size);
        let content =
            self.children[1]
                .as_widget_mut()
                .layout(&mut tree.children[1], renderer, &final_limits);

        layout::Node::with_children(size, vec![measured, content])
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.children[1].as_widget_mut().operate(
            &mut tree.children[1],
            layout.child(1),
            renderer,
            operation,
        );
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.children[1].as_widget_mut().update(
            &mut tree.children[1],
            event,
            layout.child(1),
            cursor,
            renderer,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.children[1].as_widget().mouse_interaction(
            &tree.children[1],
            layout.child(1),
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.children[1].as_widget().draw(
            &tree.children[1],
            renderer,
            theme,
            style,
            layout.child(1),
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.children[1].as_widget_mut().overlay(
            &mut tree.children[1],
            layout.child(1),
            renderer,
            viewport,
            translation,
        )
    }
}
