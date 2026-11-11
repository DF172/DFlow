use crate::app::Message;
use crate::t;
use crate::ui::kit;
use iced::widget::{column, row, text, text_input, Space};
use iced::{Element, Fill, Length, Shrink};
use crate::ui::style::tokens;

pub const FIND_INPUT_ID: &str = "find-replace-search";

pub fn view_window<'a>(
    search: &'a str,
    replacement: &'a str,
    status: &'a str,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let field_width = if matches!(sizing.width, Length::Fill) {
        Fill
    } else {
        Shrink
    };
    let find_input = text_input("", search)
        .id(iced::widget::Id::new(FIND_INPUT_ID))
        .on_input(Message::FindReplaceSearchChanged)
        .on_submit(Message::FindReplaceNext)
        .padding(kit::PAD_WELL)
        .size(tokens::TEXT_META);
    let replacement_input = text_input("", replacement)
        .on_input(Message::FindReplaceReplacementChanged)
        .padding(kit::PAD_WELL)
        .size(tokens::TEXT_META);

    let enabled = !search.trim().is_empty();

    column![
        row![
            text(t!("Find:")).size(tokens::TEXT_BODY).width(90),
            find_input.width(field_width),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
        row![
            text(t!("Replace with:")).size(tokens::TEXT_BODY).width(90),
            replacement_input.width(field_width),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
        text(t!(
            "Searches Text, MText, Attribute Definitions, and block attribute values."
        ))
        .size(tokens::TEXT_LABEL),
        text(status).size(tokens::TEXT_LABEL),
        row![
            Space::new().width(field_width),
            kit::dialog_button(t!("Close"), Message::CloseModal, false),
            kit::dialog_button_styled_opt(
                t!("Replace"),
                enabled.then_some(Message::FindReplaceOne),
                kit::secondary_style,
            ),
            kit::dialog_button_styled_opt(
                t!("Replace All"),
                enabled.then_some(Message::FindReplaceAll),
                kit::danger_style,
            ),
            kit::dialog_button_styled_opt(
                t!("Find Next"),
                enabled.then_some(Message::FindReplaceNext),
                kit::primary_style,
            ),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
    ]
    .spacing(tokens::GAP_FIELD)
    .padding(tokens::GAP_SECTION)
    .width(sizing.width)
    .height(sizing.height)
    .into()
}
