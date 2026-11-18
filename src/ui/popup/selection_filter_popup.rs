//! Selection-filter status menu — choose which entity types are selectable.
//! A checked row means that type can be picked; unchecking it excludes the
//! type from interactive selection. Opened from the FILTER status pill.

use rustc_hash::FxHashSet as HashSet;

use crate::t;
use iced::widget::{container, row, text};
use crate::ui::kit::plain_button;
use iced::{Element, Fill, Theme};

use crate::app::Message;
use crate::ui::statusbar::status_menu::Entry;
use crate::ui::kit;
use crate::ui::style::tokens;

/// - `types`: entity-type names present in the current layout.
/// - `excluded`: types currently filtered out (unchecked).
pub fn menu_entries(types: &[String], excluded: &HashSet<String>) -> Vec<Entry<'static>> {
    // "Select All / Clear All" header, mirroring the OSNAP popup: Select All
    // clears every exclusion, Clear All excludes every present type.
    let has_types = !types.is_empty();
    let all_included = excluded.is_empty();
    let all_excluded = has_types && types.iter().all(|t| excluded.contains(t));
    let header = row![
        header_btn(
            "Select All",
            Message::SelectionFilterSelectAll,
            has_types && !all_included,
        ),
        header_btn(
            "Clear All",
            Message::SelectionFilterClearAll,
            has_types && !all_excluded,
        ),
    ]
    .spacing(tokens::GAP_SEAM)
    .padding(kit::Size::Compact.padding());

    let divider = container(iced::widget::Space::new().height(1))
        .style(kit::chrome_plate_style)
        .width(Fill)
        .padding(kit::PAD_BADGE);

    let rows: Vec<Entry<'static>> = if types.is_empty() {
        vec![Entry::stay(empty_row())]
    } else {
        types
            .iter()
            .map(|name| {
                let included = !excluded.contains(name);
                Entry::stay(type_row(name.clone(), included))
            })
            .collect()
    };

    let mut entries = vec![Entry::stay(header), Entry::stay(divider)];
    entries.extend(rows);
    entries
}

fn type_row(name: String, included: bool) -> Element<'static, Message> {
    let check = crate::ui::icons::themed_check_cell(included);

    let lbl = text(t!(name.as_str())).size(tokens::TEXT_LABEL);

    let content = row![check, lbl].spacing(tokens::GAP_CONTROL).align_y(iced::Center);

    plain_button(content)
        .on_press(Message::ToggleSelectionFilterType(name))
        .style(kit::subtle_style)
        .width(Fill)
        .padding(kit::Size::Control.padding())
        .into()
}

fn empty_row() -> Element<'static, Message> {
    container(
        text(t!("No objects"))
            .size(tokens::TEXT_LABEL)
            .style(|theme: &Theme| {
                kit::color_style(theme.palette().background.base.text.scale_alpha(0.42))
            }),
    )
    .padding(kit::Size::Control.padding())
    .into()
}

fn header_btn(label: &str, msg: Message, enabled: bool) -> Element<'_, Message> {
    let b = plain_button(text(t!(label)).size(tokens::TEXT_MICRO));
    let b = if enabled { b.on_press(msg) } else { b };
    b.style(kit::secondary_style).padding(kit::PAD_TRIGGER).into()
}
