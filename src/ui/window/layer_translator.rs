//! Layer Translator — map this drawing's layers onto a set loaded from another
//! drawing, then translate.
//!
//! Two lists side by side: what the drawing has, and what the loaded standard
//! offers. Selecting one of each and mapping them builds up the list of moves,
//! which is only applied when Translate is pressed — so a mapping can be
//! reconsidered, and a translation is one undo step rather than many.

use crate::ui::style::tokens;
use iced::widget::{checkbox, column, container, row, text, Space};
use crate::ui::kit::plain_button;
use iced::{Border, Element, Fill, Length, Theme};

use crate::app::Message;
use crate::modules::draw::layers::laytrans::{Mapping, TargetLayer};
use crate::t;
use crate::ui::kit;

/// The dialog's working state: what was loaded, what is selected, and the
/// mappings built so far. None of it touches the drawing until Translate.
#[derive(Default)]
pub struct State {
    /// Where the target set came from, shown so it is clear what is being
    /// translated to.
    pub source_file: String,
    pub targets: Vec<TargetLayer>,
    pub mappings: Vec<Mapping>,
    pub selected_from: Option<String>,
    pub selected_to: Option<String>,
    pub force_bylayer: bool,
    pub write_log: bool,
}

fn list_row<'a>(label: String, selected: bool, message: Message) -> Element<'a, Message> {
    plain_button(text(label).size(tokens::TEXT_BODY).width(Fill))
        .on_press(message)
        .style(move |theme: &Theme, status| {
            let palette = theme.palette();
            let mut style = kit::text_style(theme, status);
            if selected {
                style.background = Some(tokens::aurora_gradient(theme, tokens::WASH_ROW));
                style.text_color = palette.primary.weak.text;
            }
            style.border = Border {
                radius: tokens::RADIUS_FIELD.into(),
                ..Default::default()
            };
            style
        })
        .width(Fill)
        .padding(kit::Size::Compact.padding())
        .into()
}

fn pane<'a>(
    title: String,
    rows: Vec<Element<'a, Message>>,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let body: Element<'_, Message> = if rows.is_empty() {
        container(text(t!("(none)")).size(tokens::TEXT_LABEL).style(kit::muted_style))
            .padding(tokens::GAP_ROW)
            .into()
    } else {
        crate::ui::kit::themed_scrollable(column(rows).spacing(tokens::GAP_SEAM))
            .height(sizing.height)
            .into()
    };
    container(column![text(title).size(tokens::TEXT_LABEL).style(kit::muted_style), body].spacing(tokens::GAP_TIGHT))
        .padding(tokens::GAP_CONTROL)
        .width(sizing.width)
        .height(sizing.height)
        .style(kit::fieldset_well_style)
        .into()
}

pub fn view_window<'a>(
    state: &'a State,
    sources: Vec<String>,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    // The sample sheet's own width: measuring at the design measure keeps the
    // dialog at its proportions instead of shrink-wrapping to the panes.
    let frame_width = if matches!(sizing.width, Length::Fill) {
        Length::Fill
    } else {
        Length::Fixed(1178.0)
    };
    let half_width = if matches!(sizing.width, Length::Fill) {
        Length::Fill
    } else {
        Length::Fixed(570.0)
    };
    let half = crate::ui::shell::Sizing {
        width: half_width,
        height: sizing.height,
    };
    let full = crate::ui::shell::Sizing {
        width: frame_width,
        height: sizing.height,
    };
    // A layer already spoken for should not be offered again; mapping it twice
    // could only mean the second one wins, which is not a choice worth making
    // silently.
    let mapped: Vec<&str> = state.mappings.iter().map(|m| m.from.as_str()).collect();
    let from_rows: Vec<Element<'_, Message>> = sources
        .iter()
        .filter(|name| !mapped.contains(&name.as_str()))
        .map(|name| {
            list_row(
                name.clone(),
                state.selected_from.as_deref() == Some(name.as_str()),
                Message::LayerTranslatorSelectFrom(name.clone()),
            )
        })
        .collect();
    let to_rows: Vec<Element<'_, Message>> = state
        .targets
        .iter()
        .map(|target| {
            list_row(
                target.name.clone(),
                state.selected_to.as_deref() == Some(target.name.as_str()),
                Message::LayerTranslatorSelectTo(target.name.clone()),
            )
        })
        .collect();
    let mapping_rows: Vec<Element<'_, Message>> = state
        .mappings
        .iter()
        .map(|mapping| {
            row![
                text(format!("{}  →  {}", mapping.from, mapping.to))
                    .size(tokens::TEXT_BODY)
                    .width(Fill),
                plain_button(text(t!("Remove")).size(tokens::TEXT_LABEL))
                    .on_press(Message::LayerTranslatorUnmap(mapping.from.clone()))
                    .style(kit::text_style)
                    .padding(kit::PAD_GLYPH),
            ]
            .align_y(iced::Center)
            .padding(kit::PAD_GLYPH)
            .into()
        })
        .collect();

    let loaded: Element<'_, Message> = if state.source_file.is_empty() {
        text(t!("No standard loaded.")).size(tokens::TEXT_LABEL).style(kit::muted_style).into()
    } else {
        text(state.source_file.clone()).size(tokens::TEXT_LABEL).style(kit::muted_style).into()
    };

    let can_map = state.selected_from.is_some() && state.selected_to.is_some();
    let controls = row![
        plain_button(text(t!("Load…")).size(tokens::TEXT_BODY))
            .on_press(Message::LayerTranslatorLoad)
            .padding(kit::Size::Control.padding()),
        plain_button(text(t!("Map")).size(tokens::TEXT_BODY))
            .on_press_maybe(can_map.then_some(Message::LayerTranslatorMap))
            .padding(kit::Size::Control.padding()),
        plain_button(text(t!("Map same")).size(tokens::TEXT_BODY))
            .on_press_maybe((!state.targets.is_empty()).then_some(Message::LayerTranslatorMapSame))
            .padding(kit::Size::Control.padding()),
        Space::new().width(Fill),
        loaded,
    ]
    .spacing(tokens::GAP_CONTROL)
    .align_y(iced::Center);

    let options = row![
        checkbox(state.force_bylayer)
            .on_toggle(Message::LayerTranslatorForceByLayer)
            .size(tokens::MARK_SQUARE),
        text(t!("Force objects to ByLayer")).size(tokens::TEXT_LABEL),
        Space::new().width(tokens::GAP_FIELD),
        checkbox(state.write_log)            .on_toggle(Message::LayerTranslatorWriteLog)
            .size(tokens::MARK_SQUARE),
        text(t!("Write translation log")).size(tokens::TEXT_LABEL),
    ]
    .spacing(tokens::GAP_CONTROL)
    .align_y(iced::Center);

    let actions = row![
        plain_button(text(t!("Save mappings…")).size(tokens::TEXT_BODY))
            .on_press_maybe(
                (!state.mappings.is_empty()).then_some(Message::LayerTranslatorSaveMappings)
            )
            .padding(kit::Size::Control.padding()),
        plain_button(text(t!("Load mappings…")).size(tokens::TEXT_BODY))
            .on_press(Message::LayerTranslatorLoadMappings)
            .padding(kit::Size::Control.padding()),
        Space::new().width(Fill),
        kit::dialog_button(t!("Cancel"), Message::CloseModal, false),
        kit::dialog_button_styled_opt(
            t!("Translate"),
            (!state.mappings.is_empty()).then_some(Message::LayerTranslatorTranslate),
            kit::primary_style,
        ),
    ]
    .spacing(tokens::GAP_CONTROL)
    .align_y(iced::Center);

    let lists = row![
        pane(t!("Translate from").into_owned(), from_rows, half),
        pane(t!("Translate to").into_owned(), to_rows, half),
    ]
    .spacing(tokens::GAP_ROW)
    .width(frame_width)
    .height(sizing.height);

    column![
        controls,
        lists,
        pane(
            t!("%{n} mapping(s)", n = state.mappings.len()).into_owned(),
            mapping_rows,
            full,
        ),
        options,
        actions,
    ]
    .spacing(tokens::GAP_ROW)
    .padding(tokens::GAP_FIELD)
    .width(frame_width)
    .into()
}
