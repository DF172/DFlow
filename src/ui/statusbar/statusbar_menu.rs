//! Status-bar customization and layout-list menu entries.

use crate::ui::kit;
use iced::widget::{button, row, text};
use crate::ui::kit::plain_button;
use iced::{Element, Fill, Theme};

use crate::app::Message;
use crate::t;
use crate::ui::statusbar::status_menu::Entry;
use crate::ui::statusbar::statusbar_config::{StatusBarConfig, StatusPill};
use crate::ui::style::tokens;

pub fn customization_entries(config: &StatusBarConfig) -> Vec<Entry<'static>> {
    StatusPill::ALL
        .iter()
        // The coordinate readout is always on (see `StatusBar::view`), so the
        // menu would otherwise offer a switch that cannot take effect.
        .filter(|&&pill| pill != StatusPill::Coords)
        .map(|&pill| {
            Entry::stay(menu_row(
                t!(pill.label()),
                config.is_visible(pill),
                Message::ToggleStatusPill(pill),
            ))
        })
        .collect()
}

pub fn layout_entries<'a>(layouts: &[String], current: &str) -> Vec<Entry<'a>> {
    layouts
        .iter()
        .map(|name| Entry::close(layout_row(name.clone(), name == current)))
        .collect()
}

fn layout_row<'a>(name: String, is_current: bool) -> Element<'a, Message> {
    let lbl = text(name.clone()).size(tokens::TEXT_LABEL);
    plain_button(row![lbl].align_y(iced::Center))
        .on_press(Message::LayoutSwitch(name))
        .style(move |theme: &Theme, status| {
            let mut style = kit::subtle_style(theme, status);
            if is_current && status == button::Status::Active {
                match crate::ui::style::draftflow_theme::draftflow_colors(theme) {
                    Some(colors) => {
                        style.background = Some(iced::Background::Color(colors.selected_row));
                        style.text_color = colors.ink;
                    }
                    None => {
                        let palette = theme.palette();
                        style.background = Some(crate::ui::style::tokens::aurora_gradient(
                            theme,
                            tokens::WASH_ROW,
                        ));
                        style.text_color = palette.primary.weak.text;
                    }
                }
            }
            style
        })
        .width(Fill)
        .padding(kit::PAD_MENU_ROW)
        .into()
}

fn menu_row(
    label: std::borrow::Cow<'static, str>,
    checked: bool,
    msg: Message,
) -> Element<'static, Message> {
    let check = crate::ui::icons::themed_check_cell(checked);

    let lbl = text(label).size(tokens::TEXT_LABEL);

    let content = row![check, lbl].spacing(tokens::GAP_CONTROL).align_y(iced::Center);

    plain_button(content)
        .on_press(msg)
        .style(kit::subtle_style)
        .width(Fill)
        .padding(kit::Size::Control.padding())
        .into()
}
