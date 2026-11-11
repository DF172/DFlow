//! Annotation Object Scale dialog — add / remove the annotation scales a single
//! selected object carries a per-object representation for.
//!
//! Each drawing scale is a row; a checkmark marks the scales the object is a
//! member of. Clicking a row toggles membership: adding synthesizes a per-scale
//! context (`AcDb*ObjectContextData`) at that scale, removing drops it. Shares
//! the style / scale managers' frame so it looks consistent.

use crate::ui::style::tokens;
use crate::app::Message;
use crate::t;
use crate::ui::kit;
use crate::ui::kit::muted_style;
use crate::ui::style::style_manager::{hdivider, tb_button};
use iced::widget::{column, container, mouse_area, row, text, Space};
use iced::{Element};

/// `scales` is `(name, "paper:drawing" ratio, is_member)`. Every label is cloned
/// into the widget tree, so the returned element borrows nothing from the args.
pub fn view_window(
    object_label: &str,
    scales: &[(String, String, bool)],
    sizing: crate::ui::shell::Sizing,
) -> Element<'static, Message> {
    let toolbar = container(
        row![
            text(t!("Object: %{object_label}", object_label = object_label)).size(tokens::TEXT_LABEL),
            Space::new().width(sizing.width),
            tb_button(t!("Close"), Message::CloseModal, true),
        ]
        .spacing(tokens::GAP_TIGHT)
        .align_y(iced::Center),
    )
    .style(kit::chrome_plate_style)
    .width(sizing.width)
    .padding(kit::Size::Compact.padding());

    let rows: Vec<Element<'_, Message>> = scales
        .iter()
        .map(|(name, ratio, member)| {
            let check = crate::ui::icons::themed_check_cell(*member);
            let label = row![
                check,
                text(name.clone()).size(tokens::TEXT_LABEL).width(sizing.width),
                text(ratio.clone()).size(tokens::TEXT_MICRO).style(muted_style),
            ]
            .spacing(tokens::GAP_TIGHT)
            .align_y(iced::Center);
            let cell = container(label).padding(crate::ui::kit::Size::Compact.padding()).width(sizing.width);
            mouse_area(cell)
                .on_press(Message::AnnoObjectScaleToggle(name.clone()))
                .into()
        })
        .collect();

    let list = container(crate::ui::kit::themed_scrollable(column(rows).spacing(tokens::GAP_SEAM)).height(sizing.height))
        .style(kit::pane_plate_style)
        .width(sizing.width)
        .height(sizing.height)
        .padding(tokens::GAP_HAIRLINE);

    let body = container(
        column![
            text(t!(
                "Click a scale to add or remove the object's representation for it."
            ))
            .size(tokens::TEXT_MICRO)
            .style(muted_style),
            list,
        ]
        .spacing(tokens::GAP_CONTROL)
        .height(sizing.height),
    )
    .width(sizing.width)
    .height(sizing.height)
    .padding(tokens::GAP_SECTION);

    container(column![toolbar, hdivider(sizing.width), body])
        .width(sizing.width)
        .height(sizing.height)
        .into()
}
