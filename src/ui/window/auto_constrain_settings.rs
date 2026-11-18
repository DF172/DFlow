use crate::app::settings::AutoConstrainSettings;
use crate::app::Message;
use crate::ui::kit;
use iced::widget::{checkbox, column, container, row, text, text_input, Space};
use crate::ui::kit::plain_button;
use iced::{Border, Element, Fill, Length, Theme};
use crate::ui::style::tokens;

pub fn view_window<'a>(
    settings: &'a AutoConstrainSettings,
    selected_row: usize,
    distance_input: &'a str,
    angle_input: &'a str,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    // The sample sheet's own width: measuring at the design measure keeps the
    // dialog at its proportions instead of shrink-wrapping to the table.
    let frame_width = if matches!(sizing.width, Length::Fill) {
        Length::Fill
    } else {
        Length::Fixed(776.0)
    };
    let heading = row![
        text(crate::t!("Priority")).width(70),
        text(crate::t!("Constraint Type")).width(Fill),
        text(crate::t!("Apply")).width(60),
    ]
    .spacing(tokens::GAP_ROW);

    let mut list = column![heading].spacing(tokens::GAP_TIGHT);
    for (index, kind) in settings.priority.iter().copied().enumerate() {
        let enabled = settings.enabled.contains(&kind);
        let label = row![
            text(format!("{}", index + 1)).width(70),
            text(crate::t!(kind.label())).width(Fill),
            checkbox(enabled)
                .on_toggle(move |_| Message::AutoConstrainToggleKind(kind))
                .width(60),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center);
        list = list.push(
            plain_button(label)
                .on_press(Message::AutoConstrainSelectRow(index))
                .style(move |theme: &Theme, status| {
                    // The sample marks the picked row with the accent outline,
                    // not a filled plate: the row's own text stays on paper.
                    let mut style = kit::text_style(theme, status);
                    if selected_row == index {
                        style.border = Border {
                            color: theme.palette().primary.base.color,
                            width: tokens::EDGE_HAIRLINE,
                            radius: tokens::RADIUS_FIELD.into(),
                        };
                    }
                    style
                })
                .width(Fill)
                .padding([tokens::GAP_CONTROL, tokens::GAP_ROW]),
        );
    }

    let reorder = column![
        plain_button(text(crate::t!("Move Up")))
            .on_press_maybe((selected_row > 0).then_some(Message::AutoConstrainMoveUp))
            .style(kit::button_style(false))
            .width(Fill),
        plain_button(text(crate::t!("Move Down")))
            .on_press_maybe(
                (selected_row + 1 < settings.priority.len())
                    .then_some(Message::AutoConstrainMoveDown)
            )
            .style(kit::button_style(false))
            .width(Fill),
        Space::new().height(tokens::GAP_ROW),
        plain_button(text(crate::t!("Select All")))
            .on_press(Message::AutoConstrainSelectAll)
            .style(kit::button_style(false))
            .width(Fill),
        plain_button(text(crate::t!("Clear All")))
            .on_press(Message::AutoConstrainClearAll)
            .style(kit::button_style(false))
            .width(Fill),
        plain_button(text(crate::t!("Reset")))
            .on_press(Message::AutoConstrainReset)
            .style(kit::button_style(false))
            .width(Fill),
    ]
    .spacing(tokens::GAP_ROW)
    .width(180);

    let types = row![
        container(crate::ui::kit::themed_scrollable(list).height(260))
            .padding(tokens::GAP_CONTROL)
            .width(Fill),
        reorder
    ]
    .spacing(tokens::GAP_SECTION);

    let conditions = column![
        checkbox(settings.tangent_must_share_point)
            .label(crate::t!(
                "Tangent objects must share an intersection point"
            ))
            .on_toggle(|_| Message::AutoConstrainToggleTangentPoint),
        checkbox(settings.perpendicular_must_intersect)
            .label(crate::t!(
                "Perpendicular objects must share an intersection point"
            ))
            .on_toggle(|_| Message::AutoConstrainTogglePerpendicularIntersection),
    ]
    .spacing(tokens::GAP_ROW);

    let tolerances = column![
        text(crate::t!("Tolerances")).size(tokens::TEXT_HEADING),
        row![
            text(crate::t!("Distance")).width(130),
            text_input("0.05", distance_input)
                .on_input(Message::AutoConstrainDistanceChanged)
                .padding(kit::PAD_FIELD)
                .width(200),
        ]
        .align_y(iced::Center),
        row![
            text(crate::t!("Angle")).width(130),
            text_input("1", angle_input)
                .on_input(Message::AutoConstrainAngleChanged)
                .padding(kit::PAD_FIELD)
                .width(200),
            text("°"),
        ]
        .align_y(iced::Center),
    ]
    .spacing(tokens::GAP_FIELD);
    // The sheet sets the tolerances in their own outlined card.
    let tolerances = container(tolerances)
        .padding(tokens::GAP_GUTTER)
        .width(Fill)
        .style(kit::fieldset_well_style);

    let actions = row![
        Space::new().width(Fill),
        kit::dialog_button(crate::t!("OK"), Message::AutoConstrainOk, true),
        kit::dialog_button(crate::t!("Apply"), Message::AutoConstrainApply, false),
        kit::dialog_button(crate::t!("Cancel"), Message::AutoConstrainCancel, false),
    ]
    .spacing(tokens::GAP_ROW)
    .align_y(iced::Center);

    container(
        column![
            types,
            conditions,
            tolerances,
            Space::new().height(Fill),
            actions,
        ]
        .spacing(tokens::GAP_SECTION)
        .width(frame_width)
        .height(sizing.height),
    )
    .padding([tokens::GAP_SECTION, tokens::GAP_GUTTER])
    .width(frame_width)
    .height(sizing.height)
    .into()
}
