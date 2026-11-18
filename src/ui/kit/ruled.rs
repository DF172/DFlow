//! A rule that rides one edge of the widget it wraps.
//!
//! The sheet marks a lit tab or a lit status toggle with a single 2 px bar on
//! one edge, not with a plate. iced's `Border` is uniform on all four sides, and
//! a `Length::Fill` strip under the content cannot be used here either: the
//! status bar measures its pills through `WrapBar`, which lays blocks out at
//! their natural width under unbounded limits, where `Fill` resolves to
//! infinity. So the bar is painted by the wrapper itself, from its own bounds,
//! which are the content's bounds plus the bar.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{self, Widget};
use iced::advanced::{mouse, overlay, renderer, Renderer as _, Shell};
use iced::{
    Background, Border, Color, Element, Event, Length, Point, Rectangle, Size, Theme, Vector,
};

use crate::ui::style::draftflow_theme::draftflow_colors;
use crate::ui::style::tokens;

/// The bar's thickness, in px.
pub const RULE: f32 = tokens::RULE_BRAND;

/// Which edge of the content the bar rides.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Top,
    Bottom,
}

/// Which ink the bar is painted in.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// The sheet's text ink: a lit tab, a lit status toggle.
    Ink,
    /// The one saturated signal: the command field's own edge.
    Accent,
}

/// Wraps `content` and paints a [`RULE`]-thick bar on `edge` while `lit`.
///
/// `lit` is a strength, not a switch: a toggle that engages over 120 ms hands
/// the same scalar to its bar and its plate, so the two cannot disagree about
/// how far the control has travelled. Unlit, the widget is transparent and only
/// ever costs the bar's thickness of height — the sheet's own grid, so a pill
/// does not jump when it lights up. Non-DraftFlow themes paint nothing at all:
/// their lit rung is a plate.
pub struct Ruled<'a, M> {
    content: Element<'a, M>,
    edge: Edge,
    lit: f32,
    tone: Tone,
    thickness: f32,
    inset: f32,
}

impl<'a, M> Ruled<'a, M> {
    pub fn new(content: impl Into<Element<'a, M>>, edge: Edge, lit: bool) -> Self {
        Self::at(content, edge, if lit { 1.0 } else { 0.0 })
    }

    /// The continuous twin of [`Ruled::new`].
    pub fn at(content: impl Into<Element<'a, M>>, edge: Edge, lit: f32) -> Self {
        Self {
            content: content.into(),
            edge,
            lit: lit.clamp(0.0, 1.0),
            tone: Tone::Ink,
            thickness: RULE,
            inset: 0.0,
        }
    }

    /// Pull the bar in from both ends, when the sheet's rule stops short of
    /// the plate it marks (the status bar's toggles inset theirs by 5 px).
    #[must_use]
    pub fn inset(mut self, inset: f32) -> Self {
        self.inset = inset;
        self
    }

    #[must_use]
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    #[must_use]
    pub fn thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }
}

impl<M> Widget<M, Theme, iced::Renderer> for Ruled<'_, M> {
    fn diff(&mut self, tree: &mut widget::Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.content));
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.content.as_widget().size().width, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let child = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        let child_size = child.size();
        let own_size = Size::new(child_size.width, child_size.height + self.thickness);
        let offset = match self.edge {
            Edge::Top => Point::new(0.0, self.thickness),
            Edge::Bottom => Point::new(0.0, 0.0),
        };
        layout::Node::with_children(own_size, vec![child.move_to(offset)])
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        let child_layout = layout.children().next().expect("ruled has one child");
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            child_layout,
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
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let child_layout = layout.children().next().expect("ruled has one child");
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            child_layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        let child_layout = layout.children().next().expect("ruled has one child");
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            child_layout,
            renderer,
            operation,
        );
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let child_layout = layout.children().next().expect("ruled has one child");
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            child_layout,
            cursor,
            viewport,
        );

        let ink = match draftflow_colors(theme) {
            Some(colors) if self.lit > 0.0 => match self.tone {
                Tone::Ink => colors.ink,
                Tone::Accent => colors.accent,
            }
            .scale_alpha(self.lit),
            _ => Color::TRANSPARENT,
        };
        if ink.a <= 0.0 {
            return;
        }
        let bounds = layout.bounds();
        let x = bounds.x + self.inset;
        let width = (bounds.width - self.inset * 2.0).max(0.0);
        let bar = match self.edge {
            Edge::Top => Rectangle {
                x,
                width,
                height: self.thickness,
                ..bounds
            },
            Edge::Bottom => Rectangle {
                x,
                width,
                y: bounds.y + bounds.height - self.thickness,
                height: self.thickness,
                ..bounds
            },
        };
        renderer.fill_quad(
            renderer::Quad {
                bounds: bar,
                border: Border::default(),
                ..renderer::Quad::default()
            },
            Background::Color(ink),
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, iced::Renderer>> {
        let child_layout = layout.children().next().expect("ruled has one child");
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            child_layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, M: 'a> From<Ruled<'a, M>> for Element<'a, M> {
    fn from(widget: Ruled<'a, M>) -> Self {
        Element::new(widget)
    }
}

/// A rule under `content`, painted while `lit`.
pub fn rule_under<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    lit: bool,
) -> Element<'a, M> {
    Ruled::new(content, Edge::Bottom, lit).into()
}

/// A rule over `content`, painted while `lit`.
pub fn rule_over<'a, M: 'a>(content: impl Into<Element<'a, M>>, lit: bool) -> Element<'a, M> {
    Ruled::new(content, Edge::Top, lit).into()
}

/// A rule over `content` at a continuous strength.
pub fn rule_over_at<'a, M: 'a>(content: impl Into<Element<'a, M>>, lit: f32) -> Element<'a, M> {
    Ruled::at(content, Edge::Top, lit).into()
}

/// A rule under `content`, inset `inset` px from both ends while `lit`.
pub fn rule_under_inset<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    lit: bool,
    inset: f32,
) -> Element<'a, M> {
    rule_under_inset_at(content, if lit { 1.0 } else { 0.0 }, inset)
}

/// A rule under `content` at a continuous strength.
pub fn rule_under_at<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    lit: f32,
) -> Element<'a, M> {
    Ruled::at(content, Edge::Bottom, lit).into()
}

/// A rule under `content` at a continuous strength, inset from both ends.
pub fn rule_under_inset_at<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    lit: f32,
    inset: f32,
) -> Element<'a, M> {
    Ruled::at(content, Edge::Bottom, lit)
        .inset(inset)
        .into()
}

/// The command field's own edge: a hairline of accent under `content`, always
/// lit. Non-DraftFlow themes paint nothing and keep their field plate instead.
pub fn accent_under<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    Ruled::at(content, Edge::Bottom, 1.0)
        .tone(Tone::Accent)
        .thickness(tokens::EDGE_HAIRLINE)
        .into()
}
