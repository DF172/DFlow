//! The press sink: a paint-only translation wrapped around a button's content.
//!
//! `button::Style` has no transform field, so a press cannot be expressed as a
//! style. Animating padding or height instead would reflow the row on every
//! frame of the click and shove the neighbouring controls sideways. This
//! wrapper takes the third route: it lays out exactly like its content and
//! then draws it through `Renderer::with_translation`, so the glyph sinks by a
//! pixel while the plate, the hit test and the layout around it never move.
//!
//! The wrapper is transparent in the widget tree — `tag`, `state` and `diff`
//! are forwarded and the same `Tree` and `Layout` are handed straight to the
//! content — so it introduces no state of its own and cannot change how the
//! content diffs, focuses or reports its intrinsic size.
//!
//! It reads the track; it does not write it. `kit::control::style_for` latches
//! the pressed state into `motion::press` once per state change, which is the
//! only place that knows whether the button is down.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{mouse, overlay, renderer, Renderer as _, Shell};
use iced::{Element, Length, Rectangle, Size, Theme, Vector};

use crate::ui::motion;

/// How far the content sinks at full press. One pixel: enough to read as a
/// physical give at 100% scaling, small enough that the glyph never leaves its
/// optical centre.
const SINK_PX: f32 = 1.0;

/// Wraps `content` so it sinks while `track` runs. An empty track draws
/// unmoved, which keeps un-migrated controls out of the system.
pub fn sink<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    track: &'static str,
) -> Sink<'a, Message> {
    Sink {
        content: content.into(),
        track,
    }
}

pub struct Sink<'a, Message> {
    content: Element<'a, Message>,
    track: &'static str,
}

/// Rounded to whole pixels so the glyph lands on the pixel grid instead of
/// being resampled at a fractional offset, which would soften every label
/// during the click.
fn offset(track: &'static str) -> f32 {
    if track.is_empty() {
        return 0.0;
    }
    (motion::value(track) * SINK_PX).round()
}

impl<Message> Widget<Message, Theme, iced::Renderer> for Sink<'_, Message> {
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
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            tree, event, layout, cursor, renderer, shell, viewport,
        );

        // A click anywhere on a kit control means the person is on the pointer:
        // the keyboard ring comes off, the tab position stays.
        if matches!(
            event,
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        ) && cursor.is_over(layout.bounds())
        {
            crate::ui::focus::set_modality(crate::ui::focus::Modality::Pointer);
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let dy = offset(self.track);
        if dy == 0.0 {
            self.content
                .as_widget()
                .draw(tree, renderer, theme, style, layout, cursor, viewport);
            return;
        }
        let content = self.content.as_widget();
        renderer.with_translation(Vector::new(0.0, dy), move |renderer| {
            content.draw(tree, renderer, theme, style, layout, cursor, viewport);
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

impl<'a, Message> From<Sink<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(sink: Sink<'a, Message>) -> Self {
        Element::new(sink)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::text;

    #[test]
    fn the_sink_is_one_pixel() {
        assert_eq!(SINK_PX, 1.0);
    }

    #[test]
    fn an_empty_track_never_moves() {
        motion::set_enabled(false);
        motion::set("", 1.0, motion::HOVER);
        assert_eq!(offset(""), 0.0);
    }

    #[test]
    fn the_offset_follows_the_track_and_stays_on_the_pixel_grid() {
        motion::set_enabled(false);
        let key = motion::press_key("sink-test");
        motion::set(key, 1.0, motion::HOVER);
        assert_eq!(offset(key), 1.0);
        motion::set(key, 0.0, motion::HOVER);
        assert_eq!(offset(key), 0.0);
        motion::set(key, 0.4, motion::HOVER);
        assert_eq!(offset(key), 0.0, "a sub-pixel value must not blur the glyph");
        motion::set(key, 0.6, motion::HOVER);
        assert_eq!(offset(key), 1.0);
    }

    #[test]
    fn the_wrapper_does_not_change_the_intrinsic_size() {
        let plain: Element<'_, ()> = text("Ok").into();
        let wrapped: Element<'_, ()> = sink(text("Ok"), "sink-test:size").into();
        assert_eq!(plain.as_widget().size(), wrapped.as_widget().size());
    }
}