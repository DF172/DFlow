use crate::ui::kit;
use crate::ui::kit::read_only_style;
use iced::{Element, Length};

/// A read-only value field: a text input with no `on_input` handler.
///
/// In the pinned iced fork, an input without `on_input` is treated as
/// disabled, so the caret is never drawn and typing does nothing — but mouse
/// click/drag selection and Ctrl+C / Ctrl+A still work. This gives a field the
/// user can select and copy but never edit or place a blinker in.
///
/// The value is copied into the widget's owned buffer, so the returned
/// element does not borrow `value`: callers may hand in a transient `String`.
pub fn field<'a, Message: Clone + 'a>(
    value: &str,
    size: f32,
    width: Length,
) -> Element<'a, Message> {
    iced::widget::text_input("", value)
        .size(size)
        .style(read_only_style)
        .padding(kit::PAD_FIELD)
        .width(width)
        .into()
}
