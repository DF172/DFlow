use crate::app::Message;
use crate::input::spacemouse::{NavigationMode, Preferences, Status};
use iced::widget::{checkbox, column, container, row, slider, text, Space};
use crate::ui::kit::plain_button;
use iced::{Element, Fill};
use crate::ui::kit;
use crate::ui::style::tokens;

pub(crate) const ICON: &[u8] = include_bytes!("../../../../assets/icons/ui/spacemouse.svg");

pub(crate) fn view(
    preferences: Preferences,
    status: Status,
    paused: bool,
    details: bool,
) -> Element<'static, Message> {
    let supported = cfg!(windows);
    let mut enabled = checkbox(preferences.enabled).size(tokens::MARK_SQUARE);
    if supported {
        enabled = enabled.on_toggle(Message::SpaceMouseEnabled);
    }
    let mode = crate::ui::kit::themed_pick_list(Some(preferences.mode), NavigationMode::ALL, |mode| {
        crate::t!(mode.label()).into_owned()
    })
    .width(220);
    let mode = if supported {
        mode.on_select(Message::SpaceMouseMode)
    } else {
        mode
    };
    let status_text = if !preferences.enabled {
        crate::t!("Disabled").into_owned()
    } else if paused {
        crate::t!("Paused").into_owned()
    } else {
        crate::t!(status.label()).into_owned()
    };
    let mut content = column![
        row![
            crate::ui::icons::themed(ICON, 22.),
            text(crate::t!("SpaceMouse")).size(tokens::TEXT_SECTION)
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
        row![enabled, text(crate::t!("Enable SpaceMouse")).size(tokens::TEXT_BODY)]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center),
        row![
            text(crate::t!("Status")).size(tokens::TEXT_BODY).width(150),
            text(status_text).size(tokens::TEXT_BODY)
        ]
        .spacing(tokens::GAP_FIELD),
        row![text(crate::t!("Navigation")).size(tokens::TEXT_BODY).width(150), mode]
            .spacing(tokens::GAP_FIELD)
            .align_y(iced::Center),
        text(crate::t!(preferences.mode.description()).into_owned())
            .size(tokens::TEXT_LABEL)
            .width(Fill),
    ]
    .spacing(tokens::GAP_SECTION);
    if preferences.mode != NavigationMode::Full3D && supported {
        content = content.push(
            row![
                checkbox(preferences.pan_reversed)
                    .size(tokens::MARK_SQUARE)
                    .on_toggle(Message::SpaceMousePanReversed),
                text(crate::t!("Reverse pan direction")).size(tokens::TEXT_BODY),
            ]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center),
        );
        content = content.push(
            row![
                text(crate::t!("Pan speed")).size(tokens::TEXT_BODY).width(150),
                slider(
                    10..=300,
                    preferences.pan_speed.clamp(10, 300),
                    Message::SpaceMousePanSpeed
                )
                .step(10)
                .width(160),
                text(format!("{}%", preferences.pan_speed.clamp(10, 300))).size(tokens::TEXT_LABEL),
            ]
            .spacing(tokens::GAP_FIELD)
            .align_y(iced::Center),
        );
    }
    if paused {
        content = content.push(
            plain_button(text(crate::t!("Resume SpaceMouse")).size(tokens::TEXT_LABEL))
                .on_press(Message::SpaceMousePause)
                .style(kit::secondary_style),
        );
    }
    content = content.push(
        plain_button(
            text(crate::t!(if supported {
                "Open 3Dconnexion settings…"
            } else {
                "SpaceMouse information…"
            }))
            .size(tokens::TEXT_LABEL),
        )
        .on_press(Message::SpaceMouseDriverSettings)
        .style(kit::secondary_style)
        .padding(kit::Size::Control.padding()),
    );
    content = content.push(
        text(crate::t!(
            "Configure 3D navigation, buttons, and radial menus in 3Dconnexion settings. Pan only and Pan and zoom share the push/tilt mapping and pan settings shown here."
        ))
        .size(tokens::TEXT_LABEL)
        .width(Fill),
    );
    content = content.push(
        plain_button(
            row![
                crate::ui::icons::themed_arrow_toggle(details, 9.),
                text(crate::t!("Connection details")).size(tokens::TEXT_LABEL)
            ]
            .spacing(tokens::GAP_CONTROL),
        )
        .on_press(Message::SpaceMouseDetails)
        .style(kit::text_style),
    );
    if details {
        let detail = match status {
            Status::Unavailable(reason) => reason,
            Status::Unsupported => "This version supports SpaceMouse through 3DxWare on Windows. Other platforms do not yet have a device adapter.".into(),
            Status::Disconnected => "3DxWare is running. Connect a SpaceMouse; DraftFlow CAD reconnects automatically.".into(),
            _ => "Uses the installed 3DxWare driver. Navigation pauses while a dialog is open. Your selected navigation mode is saved across drawings and sessions.".into(),
        };
        content = content.push(text(crate::t!(&detail).into_owned()).size(tokens::TEXT_LABEL).width(Fill));
    }
    container(content.push(Space::new().height(8)))
        .width(Fill)
        .into()
}
