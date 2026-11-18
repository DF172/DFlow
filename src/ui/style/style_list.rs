//! Shared style-manager list row.
//!
//! Every style manager renders the same left-hand list of style names where a
//! single click selects and a double click renames inline.
//!
//! The whole row is a `mouse_area` (not a `button`): an inner interactive
//! widget would capture the press and the parent `mouse_area` would never see
//! the second click, so `on_double_click` would never fire. `mouse_area`
//! carries both `on_press` (select) and `on_double_click` (rename) itself.

use crate::app::{Message, StyleKind};
use crate::t;
use crate::ui::kit::muted_style;
use crate::ui::style::tokens;
use iced::widget::{container, mouse_area, row, text, text_input, Space};
use iced::{Element, Fill, Theme};

/// Shared id for the inline rename field, so the rename-start handler can focus
/// it the moment the row turns editable.
pub fn rename_input_id() -> iced::widget::Id {
    iced::widget::Id::new("style-rename-input")
}

/// One row of the style list. Renders an editable `text_input` when `name` is
/// the style being renamed (`rename_active`), otherwise a selectable row whose
/// double click starts the rename.
///
/// The sample managers mark a row three ways: a status dot (accent for the
/// style in effect, the sheet edge otherwise), the style's name, and a
/// trailing tag — the accent "current" pill for the style in effect, the
/// muted "used in drawing" note for a style the drawing references, nothing
/// for a style that is merely available. The tag sits at the far edge so the
/// column can be scanned down its right margin.
pub fn item<'a>(
    name: &str,
    is_current: bool,
    is_used: bool,
    is_selected: bool,
    kind: StyleKind,
    on_select: Message,
    rename_active: Option<&str>,
    rename_buf: &'a str,
    can_rename: bool,
) -> Element<'a, Message> {
    if rename_active == Some(name) {
        text_input("", rename_buf)
            .id(iced::widget::Id::new("style-rename-input"))
            .on_input(Message::StyleRenameEdit)
            .on_submit(Message::StyleRenameCommit(kind))
            .style(crate::ui::kit::field_style)
            .size(tokens::TEXT_LABEL)
            .padding(crate::ui::kit::Size::Compact.padding())
            .width(Fill)
            .into()
    } else {
        let dot = container(Space::new().width(8).height(8))
            .style(move |theme: &Theme| crate::ui::kit::status_dot_style(theme, is_current));
        let tag: Element<'a, Message> = if is_current {
            crate::ui::kit::pill(
                text(t!("Current").into_owned()).size(tokens::TEXT_MICRO),
                "style-list:current",
                true,
                crate::ui::kit::PAD_BADGE,
                "modal:fade",
            )
            .into()
        } else if is_used {
            text(t!("Used in drawing").into_owned())
                .size(tokens::TEXT_MICRO)
                .style(muted_style)
                .into()
        } else {
            Space::new().width(0).height(0).into()
        };
        let label = row![
            dot,
            text(name.to_string()).size(tokens::TEXT_LABEL),
            Space::new().width(Fill),
            tag,
        ]
        .spacing(tokens::GAP_TIGHT)
        .align_y(iced::Center);
        let cell = container(label)
            .padding(crate::ui::kit::Size::Compact.padding())
            .width(Fill)
            .style(move |theme: &Theme| crate::ui::kit::selected_row_wash_style(theme, is_selected));
        let area = mouse_area(cell).on_press(on_select);
        if can_rename {
            area.on_double_click(Message::StyleRenameStart(kind, name.to_string()))
                .into()
        } else {
            area.into()
        }
    }
}
