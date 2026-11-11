//! Reference Manager help window: how the manager works on the web build
//! (read-only list, disabled mutations, no previews). Rendered as a modal
//! like About/DSettings so the text gets a proper window width — an anchored
//! toolbar dropdown squeezed it into an unreadable sliver.

use crate::app::Message;
use crate::ui::kit::muted_style;
use crate::ui::kit;
use iced::widget::{column, container, row, text, Space};
use iced::{Element, Fill, Length};
use crate::ui::style::tokens;

pub fn view_window(sizing: crate::ui::shell::Sizing) -> Element<'static, Message> {
    // Fixed window width: paragraphs wrap against this instead of the
    // toolbar, which is what made the dropdown unreadable.
    let body_w = Length::Fixed(460.0);
    let para =
        |label: String| -> Element<'static, Message> { text(label).size(tokens::TEXT_BODY).width(Fill).into() };

    let intro = para(crate::t!("How the Reference Manager works on web").into_owned());
    let list = para(
        crate::t!(
            "This panel lists the drawing's references read-only: status, type, size, date and saved path, plus the Details pane. Select rows to inspect them; List and Tree switch the presentation."
        )
        .into_owned(),
    );
    let limits = para(
        crate::t!(
            "Attach, Refresh and Change Path stay disabled here: the web build has no filesystem access and no native file pickers, so references can be neither added, reloaded nor repathed — and previews stay unavailable. Nested references are read-only on every platform, and relative paths need the drawing saved first."
        )
        .into_owned(),
    );

    let close_button = kit::dialog_button(crate::t!("Close"), Message::CloseModal, false);
    let action_row: Element<'static, Message> =
        row![Space::new().width(Fill), close_button,]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center)
            .into();

    let content = column![
        text(crate::t!("Reference Manager — Web").into_owned()).size(tokens::TEXT_HEADING),
        text(crate::t!("EXTERNALREFERENCES on the web build").into_owned())
            .size(tokens::TEXT_LABEL)
            .style(muted_style),
        intro,
        list,
        limits,
        Space::new().height(4),
        action_row,
    ]
    .spacing(tokens::GAP_FIELD)
    .padding(tokens::GAP_GUTTER)
    .width(body_w);

    container(content)
        .width(sizing.width)
        .height(sizing.height)
        .into()
}
