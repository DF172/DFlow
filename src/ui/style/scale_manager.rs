//! Annotation-scale manager — add / delete / edit the drawing's ACAD_SCALELIST.
//!
//! Shares the style managers' frame (toolbar + list + editor + palette) so it
//! looks and behaves like Text / Dim / MLeader style managers, but scales are
//! plain `Scale` objects rather than symbol-table styles, so it drives its own
//! messages instead of the StyleKind machinery.

use crate::ui::style::tokens;
use crate::app::Message;
use crate::t;
use crate::ui::kit;
use crate::ui::kit::muted_style;
use crate::ui::style::style_manager::{hdivider, tb_button, vsep};
use iced::widget::{column, container, mouse_area, row, text, text_input, Space};
use iced::{Element, Theme};
use std::borrow::Cow;

/// Inline-rename text-input id, so the rename-start handler can focus it.
pub fn rename_input_id() -> iced::widget::Id {
    iced::widget::Id::new("scale-rename-input")
}

/// `scales` is (name, "paper:drawing" ratio label). The editor buffers hold the
/// name / paper / drawing of the row being added or edited.
pub fn view_window<'a, 'b>(
    scales: &'b [(String, String)],
    selected: &'b str,
    current: &'b str,
    rename_active: Option<&'b str>,
    rename_buf: &'a str,
    paper_buf: &'a str,
    drawing_buf: &'a str,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    // ── Toolbar: New / Delete | Set Current / Apply ───────────────────────
    let toolbar = container(
        row![
            tb_button(t!("New"), Message::ScaleManagerNew, false),
            tb_button(t!("Copy"), Message::ScaleManagerCopy, false),
            tb_button(t!("Delete"), Message::ScaleManagerDelete, false),
            Space::new().width(sizing.width),
            tb_button(t!("Set Current"), Message::ScaleManagerSetCurrent, false),
            tb_button(t!("Apply"), Message::ScaleManagerApply, true),
        ]
        .spacing(tokens::GAP_TIGHT)
        .align_y(iced::Center),
    )
    .style(kit::chrome_plate_style)
    .width(sizing.width)
    .padding(kit::Size::Compact.padding());

    // ── Left: scale list ──────────────────────────────────────────────────
    let rows: Vec<Element<'_, Message>> =
        scales
            .iter()
            .map(|(name, ratio)| {
                // The row being renamed shows an inline text field; a single click
                // selects, a double click starts renaming.
                if rename_active == Some(name.as_str()) {
                    return text_input("", rename_buf)
                        .id(rename_input_id())
                        .on_input(Message::ScaleRenameEdit)
                        .on_submit(Message::ScaleRenameCommit)
                        .style(kit::field_style)
                        .size(tokens::TEXT_LABEL)
                        .padding(crate::ui::kit::Size::Compact.padding())
                        .width(sizing.width)
                        .into();
                }
                let is_sel = name.as_str() == selected;
                let is_cur = name.eq_ignore_ascii_case(current);
                let check = crate::ui::icons::themed_check_cell(is_cur);
                let label = row![
                    check,
                    text(name.clone()).size(tokens::TEXT_LABEL).width(sizing.width),
                    text(ratio.clone()).size(tokens::TEXT_MICRO).style(muted_style),
                ]
                .spacing(tokens::GAP_TIGHT)
                .align_y(iced::Center);
                let cell = container(label).padding(crate::ui::kit::Size::Compact.padding()).width(sizing.width).style(
                    move |theme: &Theme| kit::selected_row_wash_style(theme, is_sel),
                );
                mouse_area(cell)
                    .on_press(Message::ScaleManagerSelect(name.clone()))
                    .on_double_click(Message::ScaleRenameStart(name.clone()))
                    .into()
            })
            .collect();

    let list_panel = container(
        column![
            text(t!("Scales")).size(tokens::TEXT_MICRO).style(muted_style),
            container(crate::ui::kit::themed_scrollable(column(rows).spacing(tokens::GAP_SEAM)).height(sizing.height))
                .style(kit::pane_plate_style)
                .width(sizing.width)
                .height(sizing.height)
                .padding(tokens::GAP_HAIRLINE),
        ]
        .spacing(tokens::GAP_TIGHT)
        .height(sizing.height),
    )
    .width(190)
    .height(sizing.height)
    .padding(iced::Padding {
        top: 12.0,
        right: 8.0,
        bottom: 12.0,
        left: 12.0,
    });

    // ── Right: editor (name + paper:drawing) ──────────────────────────────
    let field = |label: Cow<'static, str>,
                 ph: Cow<'static, str>,
                 value: &str,
                 on: fn(String) -> Message| {
        row![
            text(label).size(tokens::TEXT_LABEL).style(muted_style).width(96),
            text_input(ph.as_ref(), value)
                .on_input(on)
                .style(kit::field_style)
                .size(tokens::TEXT_BODY)
                .padding(kit::Size::Compact.padding())
                .width(sizing.width),
        ]
        .align_y(iced::Center)
        .spacing(tokens::GAP_CONTROL)
    };

    let editor = container(
        column![
            text(Cow::Borrowed("Scale")).size(tokens::TEXT_MICRO).style(muted_style),
            field(t!("Paper units"), Cow::Borrowed("1"), paper_buf, Message::ScaleManagerPaperBuf),
            field(t!("Drawing units"), Cow::Borrowed("50"), drawing_buf, Message::ScaleManagerDrawingBuf),
            Space::new().height(6),
            text(t!("Double-click a scale to rename it; edit its paper : drawing ratio here. New / Copy add a scale. Changes are kept only if you click Apply before closing."))
                .size(tokens::TEXT_MICRO)
                .style(muted_style),
        ]
        .spacing(tokens::GAP_ROW),
    )
    .width(sizing.width)
    .height(sizing.height)
    .padding(tokens::GAP_SECTION);

    let body = row![list_panel, vsep(sizing.height), editor].height(sizing.height);

    container(column![toolbar, hdivider(sizing.width), body])
        .width(sizing.width)
        .height(sizing.height)
        .into()
}
