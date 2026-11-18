//! Polar-tracking angle status menu.

use crate::ui::kit;
use crate::t;
use iced::widget::{container, row, text, text_input};
use crate::ui::kit::plain_button;
use iced::{Element, Fill, Length};

use crate::app::Message;
use crate::ui::statusbar::status_menu::Entry;
use crate::ui::style::tokens;

/// Angle increments offered in the picker, in degrees. Matches the common
/// drafting set and adds the fine 1° step requested in #264.
const PRESETS: &[f32] = &[90.0, 45.0, 30.0, 22.5, 18.0, 15.0, 10.0, 5.0, 1.0];

/// Format an angle without a trailing `.0` (so `22.5°` but `15°`).
pub fn angle_label(deg: f32) -> String {
    if (deg.fract()).abs() < 1e-3 {
        format!("{:.0}°", deg)
    } else {
        format!("{deg}°")
    }
}

pub fn menu_entries<'a>(current: f32, custom: &'a str) -> Vec<Entry<'a>> {
    let mut entries: Vec<Entry<'a>> = PRESETS
        .iter()
        .map(|&deg| {
            let active = (current - deg).abs() < 1e-3;
            Entry::close(angle_row(deg, active))
        })
        .collect();

    // Free-entry custom angle: type a value and press Enter to apply.
    let custom_field = text_input(t!("Custom…").as_ref(), custom)
        .on_input(Message::PolarCustomInput)
        .on_submit(Message::SubmitPolarCustom)
        .size(tokens::TEXT_LABEL)
        .padding(kit::PAD_GLYPH)
        .width(Length::Fixed(58.0));
    let custom_row = container(
        row![custom_field, text("°").size(tokens::TEXT_LABEL),]
            .spacing(tokens::GAP_TIGHT)
            .align_y(iced::Center),
    )
    .padding(kit::Size::Control.padding());
    entries.push(Entry::stay(custom_row));
    entries
}

fn angle_row<'a>(deg: f32, active: bool) -> Element<'a, Message> {
    let check = crate::ui::icons::themed_check_cell(active);

    let lbl = text(angle_label(deg)).size(tokens::TEXT_LABEL);

    let content = row![check, lbl].spacing(tokens::GAP_CONTROL).align_y(iced::Center);

    plain_button(content)
        .on_press(Message::SetPolarAngle(deg))
        .style(kit::subtle_style)
        .width(Fill)
        .padding(kit::Size::Control.padding())
        .into()
}
