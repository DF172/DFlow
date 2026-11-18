//! Layout Manager window — fills the entire OS window.

use crate::app::Message;
use crate::t;
use crate::ui::kit::muted_style;
use crate::ui::kit;
use iced::widget::{column, container, row, text, text_input, Space};
use crate::ui::kit::plain_button;
use iced::{Background, Element, Fill, Length, Theme};
use crate::ui::style::tokens;

pub fn view_window<'a>(
    layouts: Vec<String>,
    selected: &'a str,
    rename_buf: &'a str,
    current: String,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let is_model = selected == "Model";
    // The sample sheet's own width: measuring at the design measure keeps the
    // dialog at its proportions instead of shrink-wrapping to the list.
    let frame_width = if matches!(sizing.width, Length::Fill) {
        Length::Fill
    } else {
        Length::Fixed(1030.0)
    };

    // ── Toolbar ───────────────────────────────────────────────────────────
    let toolbar = container(
        row![
            plain_button(text(t!("New Layout")).size(tokens::TEXT_LABEL))
                .on_press(Message::LayoutManagerNew)
                .style(kit::button_style(false))
                .padding(kit::Size::Control.padding()),
            plain_button(text(t!("Delete")).size(tokens::TEXT_LABEL))
                .on_press(Message::LayoutManagerDelete)
                .style(move |theme: &Theme, status| {
                    if is_model {
                        kit::secondary_style(theme, status)
                    } else {
                        kit::danger_style(theme, status)
                    }
                })
                .padding(kit::Size::Control.padding()),
            Space::new().width(sizing.width),
            plain_button(
                row![
                    crate::ui::icons::themed_arrow_left(9.0),
                    text(t!("Move Left")).size(tokens::TEXT_LABEL),
                ]
                .spacing(tokens::GAP_TIGHT)
                .align_y(iced::Center),
            )
            .on_press(Message::LayoutManagerMoveLeft)
            .style(kit::button_style(false))
            .padding(kit::Size::Compact.padding()),
            plain_button(
                row![
                    text(t!("Move Right")).size(tokens::TEXT_LABEL),
                    crate::ui::icons::themed_arrow_right(9.0),
                ]
                .spacing(tokens::GAP_TIGHT)
                .align_y(iced::Center),
            )
            .on_press(Message::LayoutManagerMoveRight)
            .style(kit::button_style(false))
            .padding(kit::Size::Compact.padding()),
            plain_button(text(t!("Set Current")).size(tokens::TEXT_LABEL))
                .on_press(Message::LayoutManagerSetCurrent)
                .style(kit::button_style(true))
                .padding(kit::Size::Control.padding()),
        ]
        .spacing(tokens::GAP_TIGHT)
        .align_y(iced::Center),
    )
    .style(|theme: &Theme| {
        kit::plain_plate_style(Background::Color(theme.palette().background.weakest.color))
    })
    .width(frame_width)
    .padding(kit::Size::Compact.padding());

    // ── Left: Layout list ─────────────────────────────────────────────────
    // The sheet draws the list as flat full-width rows separated by hairlines,
    // not as a recessed well of pills: selection is the wash alone.
    let mut list_items = column![].spacing(tokens::GAP_NONE);
    for (idx, name) in layouts.iter().enumerate() {
        let is_sel = name.as_str() == selected;
        let is_cur = name.as_str() == current.as_str();
        if idx > 0 {
            list_items = list_items.push(kit::hdivider(Length::Fill));
        }
        let mut item_row = row![text(name.clone()).size(tokens::TEXT_BODY)]
            .spacing(tokens::GAP_TIGHT)
            .align_y(iced::Center);
        if is_cur {
            item_row = item_row.push(Space::new().width(Fill)).push(
                crate::ui::icons::themed_arrow_left(8.0),
            ).push(
                text(t!("Current"))
                    .size(tokens::TEXT_MICRO)
                    .style(|theme: &Theme| kit::color_style(theme.palette().primary.base.color)),
            );
        }
        list_items = list_items.push(kit::list_row(
            item_row,
            is_sel,
            kit::Press::On(Message::LayoutManagerSelect(name.clone())),
            "",
        ));
    }

    let layout_list = container(
        column![
            text(t!("Layouts")).size(tokens::TEXT_MICRO).style(muted_style),
            crate::ui::kit::themed_scrollable(list_items).height(sizing.height),
        ]
        .spacing(tokens::GAP_TIGHT)
        .height(sizing.height),
    )
    .width(Length::Fixed(290.0))
    .height(sizing.height)
    .padding(iced::Padding {
        top: tokens::GAP_SECTION,
        right: tokens::GAP_ROW,
        bottom: tokens::GAP_SECTION,
        left: tokens::GAP_SECTION,
    });

    // ── Right: Details + rename ───────────────────────────────────────────
    let details = container(
        column![
            text(if is_model {
                t!("Model Space")
            } else {
                t!("Paper Space Layout")
            })
            .size(tokens::TEXT_HEADING),
            Space::new().height(tokens::GAP_ROW),
            row![
                text(t!("Name:")).size(tokens::TEXT_LABEL).style(muted_style).width(110),
                text(selected).size(tokens::TEXT_LABEL),
            ]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center),
            row![
                text(t!("Status:")).size(tokens::TEXT_LABEL).style(muted_style).width(110),
                text(if selected == current.as_str() {
                    t!("Active")
                } else {
                    t!("Inactive")
                })
                .size(tokens::TEXT_LABEL)
                .style(move |theme: &Theme| {
                    if selected == current.as_str() {
                        kit::color_style(theme.palette().primary.base.color)
                    } else {
                        muted_style(theme)
                    }
                }),
            ]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center),
            Space::new().height(tokens::GAP_GUTTER),
            text(t!("Rename")).size(tokens::TEXT_MICRO).style(muted_style),
            row![
                text_input(t!("New name…").as_ref(), rename_buf)
                    .on_input(Message::LayoutManagerRenameBuf)
                    .on_submit(Message::LayoutManagerRenameCommit)
                    .size(tokens::TEXT_LABEL)
                    .padding(kit::Size::Compact.padding())
                    .width(Fill),
                plain_button(text(t!("OK")).size(tokens::TEXT_LABEL))
                    .on_press(Message::LayoutManagerRenameCommit)
                    .style(kit::button_style(true))
                    .padding(kit::Size::Control.padding()),
            ]
            .spacing(tokens::GAP_CONTROL)
            .align_y(iced::Center),
        ]
        .spacing(tokens::GAP_ROW),
    )
    .width(Fill)
    .padding([tokens::GAP_GUTTER, tokens::GAP_GUTTER]);

    let body = row![layout_list, kit::vsep(sizing.height), details].height(sizing.height);

    container(column![toolbar, kit::hdivider(frame_width), body].spacing(tokens::GAP_NONE))
        .width(frame_width)
        .height(sizing.height)
        .into()
}
