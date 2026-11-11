use crate::ui::kit;
use super::status_menu::{self, Entry};
use crate::app::Message;
use crate::input::spacemouse::{NavigationMode, Preferences, Status};
use iced::widget::{row, text};
use crate::ui::kit::plain_button;
use iced::{Element, Fill};
use crate::ui::style::tokens;

pub(crate) fn view(
    preferences: Preferences,
    status: Status,
    paused: bool,
    label: String,
    sheet: bool,
) -> Element<'static, Message> {
    let root = plain_button(
        row![
            crate::ui::icons::themed(crate::ui::window::options::spacemouse::ICON, 15.),
            text(label).size(tokens::TEXT_LABEL),
            crate::ui::icons::themed_arrow_toggle(false, 8.)
        ]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center),
    )
    .padding(kit::PAD_FIELD)
    .style(kit::subtle_style);
    let mut entries = vec![Entry::stay(
        text(format!("SpaceMouse · {}", crate::t!(status.label()))).size(tokens::TEXT_LABEL),
    )];
    for mode in NavigationMode::ALL {
        let label = format!(
            "{}  {}",
            if mode == preferences.mode {
                "●"
            } else {
                "○"
            },
            crate::t!(mode.label())
        );
        entries.push(Entry::close(
            plain_button(text(label).size(tokens::TEXT_BODY))
                .width(Fill)
                .padding([tokens::GAP_CONTROL, tokens::GAP_FIELD])
                .style(kit::text_style)
                .on_press(Message::SpaceMouseMode(mode)),
        ));
    }
    if sheet {
        entries.push(Entry::stay(
            text(crate::t!("Paper sheets keep rotation locked.")).size(tokens::TEXT_LABEL),
        ));
    }
    entries.push(Entry::close(
        plain_button(
            text(crate::t!(if paused {
                "Resume SpaceMouse"
            } else {
                "Pause SpaceMouse"
            }))
            .size(tokens::TEXT_BODY),
        )
        .on_press(Message::SpaceMousePause)
        .style(kit::text_style)
        .width(Fill)
        .padding([tokens::GAP_CONTROL, tokens::GAP_FIELD]),
    ));
    for (label, message) in [
        ("SpaceMouse preferences…", Message::SpaceMousePreferences),
        ("3Dconnexion settings…", Message::SpaceMouseDriverSettings),
    ] {
        entries.push(Entry::close(
            plain_button(text(crate::t!(label)).size(tokens::TEXT_BODY))
                .on_press(message)
                .style(kit::text_style)
                .width(Fill)
                .padding([tokens::GAP_CONTROL, tokens::GAP_FIELD]),
        ));
    }
    status_menu::menu_bar(root, entries, 260.)
}
