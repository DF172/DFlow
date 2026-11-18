// Missing-fonts prompt — shown after a drawing opens with `.shx` fonts this
// machine doesn't have. DraftFlow CAD makes no network requests, so the
// prompt lists the missing files and where to put them: the local fonts
// folder (see `io::font_repo::fonts_dir`). Text re-resolution picks up any
// file dropped there on the next open.

use crate::app::Message;
use crate::ui::kit;
use iced::widget::{column, container, row, text, Space};
use iced::{Element, Fill, Length, Theme};
use crate::ui::style::tokens;

/// The prompt: the missing-font list and a close action.
pub fn view_window<'a>(
    fonts: &'a [String],
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let content_width = if matches!(sizing.width, Length::Fill) {
        Fill
    } else {
        Length::Fixed(586.0)
    };
    let listed: Vec<Element<'a, Message>> = fonts
        .iter()
        .map(|name| text(format!("  {name}")).size(tokens::TEXT_BODY).into())
        .collect();
    let fonts_dir = crate::io::font_repo::fonts_dir()
        .map(|dir| dir.display().to_string())
        .unwrap_or_default();
    let actions = row![
        Space::new().width(content_width),
        kit::dialog_button_styled_opt(
            crate::t!("Close"),
            Some(Message::MissingFontsDismiss),
            kit::secondary_style,
        ),
    ]
    .spacing(tokens::GAP_ROW);

    container(
        column![
            text(crate::t!(
                "This drawing uses fonts that are not installed on this machine. Place the missing files into the fonts folder and reopen the drawing: %{folder}",
                folder = fonts_dir
            ))
            .size(tokens::TEXT_LABEL)
            .style(kit::muted_style),
            // The right padding keeps the names clear of the scrollbar, which
            // a list as wide as its longest name would otherwise draw over.
            container(
                crate::ui::kit::themed_scrollable(column(listed).spacing(tokens::GAP_HAIRLINE).padding(iced::Padding {
                    right: 14.0,
                    ..iced::Padding::ZERO
                }))
                .height(Length::Fixed(140.0)),
            )
                .padding(kit::PAD_CARD)
                .width(content_width)
                .height(Length::Fixed(140.0))
                .style(|theme: &Theme| kit::Tier::Well.style(theme, "")),
            actions,
        ]
        .spacing(tokens::GAP_FIELD)
        .width(content_width)
        .height(sizing.height),
    )
    .width(content_width)
    .padding([tokens::GAP_SECTION, tokens::GAP_GUTTER])
    .into()
}
