//! DraftFlowCAD-style OSNAP status menu.

use iced::widget::{checkbox, column, container, row, text};
use crate::ui::kit::plain_button;
use iced::{Element, Fill, Length};

use crate::app::settings::IsoPlane;
use crate::app::Message;
use crate::snap::{SnapType, Snapper, ALL_SNAP_MODES};
use crate::t;
use crate::ui::statusbar::status_menu::Entry;
use crate::ui::kit;
use crate::ui::style::tokens;

pub fn menu_entries<'a>(
    snapper: &'a Snapper,
    isometric: bool,
    iso_plane: IsoPlane,
) -> Vec<Entry<'a>> {
    let all_on = snapper.all_on();
    let none_on = snapper.none_on();

    let header = row![
        header_btn("Select All", Message::SnapSelectAll, !all_on),
        header_btn("Clear All", Message::SnapClearAll, !none_on),
    ]
    .spacing(tokens::GAP_SEAM)
    .padding(kit::Size::Compact.padding());

    // Divider
    let divider = container(iced::widget::Space::new().height(1))
        .style(kit::chrome_plate_style)
        .width(Fill)
        .padding(kit::PAD_BADGE);

    let mut iso_planes = row![].spacing(tokens::GAP_TIGHT);
    for plane in IsoPlane::ALL {
        iso_planes = iso_planes.push(
            plain_button(text(t!(plane.label())).size(tokens::TEXT_MICRO))
                .on_press(Message::SetIsoPlane(plane))
                .style(if isometric && iso_plane == plane {
                    kit::primary_style
                } else {
                    kit::secondary_style
                })
                .padding(kit::PAD_TRIGGER),
        );
    }
    let drafting = column![
        row![
            checkbox(isometric)
                .on_toggle(|_| Message::ToggleIsometricDrafting)
                .size(tokens::MARK_SQUARE),
            text(t!("Isometric drafting")).size(tokens::TEXT_LABEL),
        ]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center),
        iso_planes,
        text(t!("F5 cycles the active plane.")).size(tokens::TEXT_MICRO),
    ]
    .spacing(tokens::GAP_CONTROL)
    .padding(kit::Size::Compact.padding());
    let drafting_divider = container(iced::widget::Space::new().height(1))
        .style(kit::chrome_plate_style)
        .width(Fill)
        .padding(kit::PAD_BADGE);

    let mut entries = vec![
        Entry::stay(header),
        Entry::stay(divider),
        Entry::stay(drafting),
        Entry::stay(drafting_divider),
    ];
    for &(snap_type, _glyph, label) in ALL_SNAP_MODES {
        entries.push(Entry::stay(snap_row(
            snap_type,
            label,
            snapper.is_on(snap_type),
        )));
    }
    entries
}

// ── Individual snap row ───────────────────────────────────────────────────

fn snap_row<'a>(snap_type: SnapType, label: &'a str, active: bool) -> Element<'a, Message> {
    let checkmark = crate::ui::icons::themed_check_cell(active);

    // SVG marker (not a Unicode glyph) so the symbols render on the web build,
    // whose bundled font lacks them and showed tofu boxes. (#138)
    let icon_el = container(crate::ui::icons::themed_success::<Message>(
        crate::ui::icons::osnap(snap_type),
        13.0,
    ))
    .width(Length::Fixed(16.0))
    .align_x(iced::Center);

    let label_el = text(t!(label)).size(tokens::TEXT_LABEL);

    let content = row![checkmark, icon_el, label_el]
        .spacing(tokens::GAP_TIGHT)
        .align_y(iced::Center);

    plain_button(content)
        .on_press(Message::ToggleSnap(snap_type))
        .style(kit::subtle_style)
        .width(Fill)
        .padding(kit::PAD_TRIGGER)
        .into()
}

fn header_btn(label: &str, msg: Message, enabled: bool) -> Element<'_, Message> {
    let b = plain_button(text(t!(label)).size(tokens::TEXT_MICRO));
    let b = if enabled { b.on_press(msg) } else { b };
    b.style(kit::secondary_style).padding(kit::PAD_TRIGGER).into()
}
