use crate::ui::style::tokens;
use crate::app::Message;
use crate::t;
use crate::ui::kit::muted_style as muted;
use iced::widget::{checkbox, column, container, row, text, Space};
use crate::ui::kit::plain_button;
use iced::{Element, Length};
use crate::ui::kit;

pub fn view_window<'a>(
    layouts: &'a [(String, bool)],
    printer: Option<&'a str>,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let selected = layouts.iter().filter(|(_, checked)| *checked).count();
    let rows: Vec<Element<'a, Message>> = layouts
        .iter()
        .map(|(name, checked)| {
            let layout = name.clone();
            container(
                checkbox(*checked)
                    .label(name.clone())
                    .on_toggle(move |_| Message::PrintAllToggle(layout.clone()))
                    .size(tokens::MARK_SQUARE)
                    .text_size(tokens::TEXT_BODY),
            )
            .padding(kit::PAD_WELL)
            .width(Length::Fill)
            .into()
        })
        .collect();
    let list: Element<'a, Message> = if rows.is_empty() {
        container(
            text(t!("No paper layouts are available."))
                .size(tokens::TEXT_BODY)
                .style(muted),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    } else {
        crate::ui::kit::themed_scrollable(column(rows).spacing(tokens::GAP_HAIRLINE))
            .height(sizing.height)
            .into()
    };
    let printer = printer
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| t!("Default printer").into_owned());
    let summary = if selected == 1 {
        t!("1 layout selected").into_owned()
    } else {
        format!("{selected} {}", t!("layouts selected"))
    };

    let mut pdf = plain_button(text(t!("PDF")).size(tokens::TEXT_LABEL))
        .style(kit::button_style(false))
        .padding([tokens::GAP_TIGHT, tokens::GAP_GUTTER]);
    let mut print = plain_button(text(t!("Print")).size(tokens::TEXT_LABEL))
        .style(kit::button_style(true))
        .padding(kit::Size::Action.padding());
    if selected > 0 {
        pdf = pdf.on_press(Message::PrintAllPdf);
        print = print.on_press(Message::PrintAllPrint);
    }

    let list_panel = container(list)
        .width(sizing.width)
        .height(sizing.height)
        .padding(tokens::GAP_TIGHT)
        .style(kit::pane_plate_style);

    container(
        column![
            text(t!(
                "Select the layouts to output. Each page is printed in Layout mode."
            ))
            .size(tokens::TEXT_BODY),
            row![
                plain_button(text(t!("Select all")).size(tokens::TEXT_LABEL))
                    .on_press(Message::PrintAllSelectAll)
                    .style(kit::button_style(false))
                    .padding(kit::Size::Control.padding()),
                plain_button(text(t!("Select none")).size(tokens::TEXT_LABEL))
                    .on_press(Message::PrintAllSelectNone)
                    .style(kit::button_style(false))
                    .padding(kit::Size::Control.padding()),
                Space::new().width(Length::Fill),
                text(summary).size(tokens::TEXT_LABEL).style(muted),
            ]
            .spacing(tokens::GAP_CONTROL)
            .align_y(iced::Center),
            list_panel,
            row![
                text(format!("{}: {printer}", t!("Printer")))
                    .size(tokens::TEXT_LABEL)
                    .style(muted),
                Space::new().width(Length::Fill),
                plain_button(text(t!("Options…")).size(tokens::TEXT_LABEL))
                    .on_press(Message::PrintAllOptions)
                    .style(kit::button_style(false))
                    .padding(kit::PAD_MENU_ROW),
                pdf,
                print,
            ]
            .spacing(tokens::GAP_CONTROL)
            .align_y(iced::Center),
        ]
        .spacing(tokens::GAP_FIELD),
    )
    .padding(tokens::GAP_SECTION)
    .width(sizing.width)
    .height(sizing.height)
    .into()
}
