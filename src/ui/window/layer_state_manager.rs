//! Layer State Manager — native DWG/DXF named layer-state UI.

use crate::app::{LayerStateLayerFlag, LayerStateProperty, Message};
use crate::t;
use crate::ui::properties::{lw_options, LwItem};
use codec::{LayerState, LayerStateMask};
use iced::widget::{checkbox, column, container, row, text, text_input, Space};
use crate::ui::kit::plain_button;
use iced::{Element, Fill, Length, Theme};
use std::borrow::Cow;
use std::fmt;
use crate::ui::kit;
use crate::ui::style::tokens;

#[derive(Clone, PartialEq, Debug)]
struct TransparencyItem(Option<codec::types::Transparency>);

impl fmt::Display for TransparencyItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            None => write!(f, "{}", t!("Not set")),
            Some(value) => write!(f, "{}%", (value.as_percent() * 100.0).round() as u8),
        }
    }
}

fn transparency_options(current: Option<codec::types::Transparency>) -> Vec<TransparencyItem> {
    let mut options = vec![TransparencyItem(None)];
    for percent in (0..=90).step_by(10) {
        options.push(TransparencyItem(Some(
            codec::types::Transparency::from_percent(percent as f64 / 100.0),
        )));
    }
    let current = TransparencyItem(current);
    if !options.contains(&current) {
        options.insert(1, current);
    }
    options
}

fn mask_summary(mask: LayerStateMask) -> String {
    let properties = [
        (LayerStateMask::ON, "On/Off"),
        (LayerStateMask::FROZEN, "Freeze"),
        (LayerStateMask::LOCKED, "Lock"),
        (LayerStateMask::PLOT, "Plot"),
        (LayerStateMask::NEW_VIEWPORT, "New VP Freeze"),
        (LayerStateMask::COLOR, "Color"),
        (LayerStateMask::LINE_TYPE, "Linetype"),
        (LayerStateMask::LINE_WEIGHT, "Lineweight"),
        (LayerStateMask::PLOT_STYLE, "Plot style"),
        (LayerStateMask::TRANSPARENCY, "Transparency"),
    ];
    properties
        .into_iter()
        .filter_map(|(flag, label)| mask.contains(flag).then(|| t!(label)))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn view_window<'a>(
    states: Vec<LayerState>,
    selected: Option<&'a str>,
    name: &'a str,
    description: &'a str,
    filter: &'a str,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let selected_state = selected.and_then(|selected| {
        states
            .iter()
            .find(|state| state.name.eq_ignore_ascii_case(selected))
    });
    let query: Cow<str> = if filter.trim().is_empty() {
        Cow::Borrowed("")
    } else {
        Cow::Owned(filter.trim().to_lowercase())
    };
    let rows: Vec<Element<'_, Message>> = states
        .iter()
        .filter(|state| {
            query.is_empty()
                || state.name.to_lowercase().contains(query.as_ref())
                || state.description.to_lowercase().contains(query.as_ref())
        })
        .map(|state| {
            let is_selected =
                selected.is_some_and(|selected| state.name.eq_ignore_ascii_case(selected));
            let subtitle = if state.description.is_empty() {
                t!("%{n} layers", n = state.layers.len()).to_string()
            } else {
                state.description.clone()
            };
            kit::list_row(
                column![
                    text(state.name.clone()).size(tokens::TEXT_BODY),
                    text(subtitle).size(tokens::TEXT_MICRO).style(kit::muted_style),
                ]
                .spacing(tokens::GAP_HAIRLINE),
                is_selected,
                kit::Press::On(Message::LayerStateManagerSelect(state.name.clone())),
                "",
            )
            // No `.padding()` here, deliberately: `kit::list_row` already takes
            // `kit::Size::Compact`, and this call site had been overtyping it as
            // `[6, 9]`. A screen re-deciding the inset of the primitive it just
            // called is the exact shape of debt #42 exists to find, and it is the
            // reason the calibration screen was chosen by role rather than by row
            // count. (drift 3px/row: 6→4 vertical, 9→8 lateral)
            .width(sizing.width)
            .into()
        })
        .collect();

    let empty: Element<'_, Message> = container(
        column![
            text(if states.is_empty() {
                t!("No layer states in this drawing")
            } else {
                t!("No matching layer states")
            })
            .size(tokens::TEXT_LABEL)
            .style(kit::muted_style),
            text(t!("Choose New to capture the current layer settings."))
                .size(tokens::TEXT_MICRO)
                .style(kit::muted_style),
        ]
        .spacing(tokens::GAP_TIGHT),
    )
    // An empty block is a *block*, not the list's content, so it carries its own
    // air: `GAP_SECTION`, the rung every pane body on this screen already wears.
    // `kit::PAD_WELL` was the other candidate and is wrong here — the well inset
    // belongs to the recess at the call below, and stacking the two would make the
    // empty state's inset depend on a number the screen never wrote. (drift 2px)
    .padding(tokens::GAP_SECTION)
    .into();

    let state_list: Element<'_, Message> = if rows.is_empty() {
        empty
    } else {
        crate::ui::kit::themed_scrollable(column(rows).spacing(tokens::GAP_HAIRLINE))
            .height(sizing.height)
            .into()
    };

    let left = container(
        column![
            text_input(t!("Search layer states…").as_ref(), filter)
                .on_input(Message::LayerStateManagerFilter)
                .size(tokens::TEXT_LABEL)
                // A field with its label above it, alone in a pane: that is a
                // panel control, so it takes the control rung `[4, 10]` rather
                // than the dense `[3, 6]` the editor's inline fields use.
                // `GAP_FIELD` is literally the field's lateral inset. (drift 3px)
                .padding(kit::Size::Control.padding()),
            container(state_list)
                .width(sizing.width)
                .height(sizing.height)
                .padding(kit::PAD_WELL)
                // A list is the kit's `Well`: recess fill + hairline edge at the
                // field corner. Only the recess fill is new; the edge it already
                // spelled is the same token.
                //
                // The inset now comes from `kit::PAD_WELL` `[6, 8]` instead of a
                // typed `3`, and that is the one decision on this screen that
                // changes how it reads rather than only how it is spelled: the
                // rows stop being full-bleed bands and become pills floating
                // inside the recess. The hand-tuned `3` existed to keep a row's
                // wash nearly touching the well edge, which collides with the
                // well's own `RADIUS_FIELD` corner at the first and last row.
                // (drift 10px: 3→6 vertical, 3→8 lateral)
                .style(|theme: &Theme| kit::Tier::Well.style(theme, "")),
        ]
        .spacing(tokens::GAP_ROW)
        .height(sizing.height),
    )
    .width(280)
    .height(sizing.height)
    // The left pane of a split window is wider from the window edges than from
    // the seam it shares with the details pane, so its inset is genuinely
    // asymmetric and no `kit::Size` rung fits it. The numbers are L0's; only the
    // spelling was the screen's. This moves nothing — but note that it *was*
    // invisible: the (f) column matches `.padding(` against a number or a pair,
    // and a multi-line `iced::Padding { .. }` struct answers neither, so 26 such
    // structs across the guarded tree are debt no ratchet was counting (finding F3).
    .padding(iced::Padding {
        top: tokens::GAP_SECTION,
        right: tokens::GAP_ROW,
        bottom: tokens::GAP_SECTION,
        left: tokens::GAP_SECTION,
    });

    let details = if let Some(state) = selected_state {
        column![
            // 13px sat one pixel under `TEXT_HEADING` and one over `TEXT_BODY`.
            // This line names a group inside a pane, which is what the heading
            // rung is for; the rung, not the midpoint between two rungs. (drift 1px)
            text(t!("Saved state details")).size(tokens::TEXT_HEADING),
            row![
                text(t!("Layers")).size(tokens::TEXT_MICRO).style(kit::muted_style).width(92),
                text(state.layers.len().to_string()).size(tokens::TEXT_LABEL),
            ]
            .spacing(tokens::GAP_ROW),
            row![
                text(t!("Current layer")).size(tokens::TEXT_MICRO).style(kit::muted_style).width(92),
                text(if state.current_layer.is_empty() {
                    "—".to_string()
                } else {
                    state.current_layer.clone()
                })
                .size(tokens::TEXT_LABEL),
            ]
            .spacing(tokens::GAP_ROW),
            text(t!("Restored properties")).size(tokens::TEXT_MICRO).style(kit::muted_style),
            text(mask_summary(state.mask)).size(tokens::TEXT_LABEL),
        ]
        // 7px was hand-tuned between `GAP_TIGHT` (4) and `GAP_ROW` (8). The
        // block is a stack of label/value rows read one after another, which is
        // what the row rung means; four rungs apart would have separated the
        // pairs rather than the lines inside them. (drift 1px, and the same
        // ruling applies to every seven-pixel spacing this screen wrote)
        .spacing(tokens::GAP_ROW)
    } else {
        column![
            text(t!("New layer state")).size(tokens::TEXT_HEADING),
            text(t!(
                "Save captures the current settings of every layer in the drawing."
            ))
            .size(tokens::TEXT_LABEL)
            .style(kit::muted_style),
        ]
        .spacing(tokens::GAP_ROW)
    };

    // Three of the five actions in this window act on the *selected* state, so
    // with nothing selected they have nothing to act on. The old spelling built
    // each of them twice — once with `.on_press`, once without — which left the
    // disabled look to iced by accident instead of by decision. `kit::btn` takes
    // the question as a `Press`: [`kit::Press::Off`] is the kit's own word for
    // "unpressable because it is unavailable", and it dims through the same
    // `Status::Disabled` rung a bare pressless button was already landing on, so
    // the paint is unchanged and the duplication is gone.
    let has_state = selected_state.is_some();
    let edit = kit::btn(
        text(t!("Edit")).size(tokens::TEXT_LABEL),
        kit::Role::Standard,
        kit::Size::Control,
        if has_state {
            kit::Press::On(Message::LayerStateManagerEdit)
        } else {
            kit::Press::Off
        },
        "",
    );
    let restore = kit::btn(
        text(t!("Restore")).size(tokens::TEXT_LABEL),
        kit::Role::Primary,
        kit::Size::Control,
        if has_state {
            kit::Press::On(Message::LayerStateManagerRestore)
        } else {
            kit::Press::Off
        },
        "",
    );
    // `Delete` stays on `kit::danger_style`, and that is not an oversight left
    // mid-migration: the doc on that function records that moving it onto
    // `Role::Danger` *is* the visible half of decision #82 C, which is unsigned.
    // Folding its inset onto the rung costs nothing; folding its plate is not
    // this batch's to spend.
    let delete = {
        let plate = plain_button(text(t!("Delete")).size(tokens::TEXT_LABEL))
            .padding(kit::Size::Control.padding())
            .style(kit::danger_style);
        if has_state {
            plate.on_press(Message::LayerStateManagerDelete)
        } else {
            plate
        }
    };

    let right = container(
        column![
            // A toolbar of secondary actions at the top of a pane: `Size::Control`
            // `[4, 10]`, the rung whose doc says "a toolbar or panel control".
            // These had been typed `[5, 12]`, which is neither the control rung
            // nor the action rung — the 12 is a *gap* value (GAP_SECTION) used as
            // an inset. (drift 3px/button)
            row![
                kit::btn(
                    text(t!("New")).size(tokens::TEXT_LABEL),
                    kit::Role::Standard,
                    kit::Size::Control,
                    kit::Press::On(Message::LayerStateManagerNew),
                    "",
                ),
                edit,
                Space::new().width(sizing.width),
                restore,
                delete,
            ]
            .spacing(tokens::GAP_CONTROL)
            .align_y(iced::Center),
            kit::hdivider(sizing.width),
            text(t!("Name")).size(tokens::TEXT_MICRO).style(kit::muted_style),
            text_input(t!("Layer state name").as_ref(), name)
                .on_input(Message::LayerStateManagerName)
                .on_submit(Message::LayerStateManagerSave)
                .size(tokens::TEXT_LABEL)
                // Same ruling as the search field above: a primary field in a
                // pane wears `Size::Control`. (drift 3px)
                .padding(kit::Size::Control.padding()),
            text(t!("Description")).size(tokens::TEXT_MICRO).style(kit::muted_style),
            text_input(t!("Optional description").as_ref(), description)
                .on_input(Message::LayerStateManagerDescription)
                .size(tokens::TEXT_LABEL)
                .padding(kit::Size::Control.padding()),
            Space::new().height(tokens::GAP_ROW),
            details,
            Space::new().height(sizing.height),
            text(t!("Layer states are stored inside the drawing and remain available after reopening it."))
                .size(tokens::TEXT_MICRO)
                .style(kit::muted_style),
            // The window's single commit action, at the bottom of the pane it
            // saves: that is a dialog footer, and the footer has one
            // constructor in this codebase — `kit::dialog_button`, called at 53
            // sites across 25 files. Using it removes three decisions from this
            // screen at once (the `[6, 14]` inset, the plate style, and the
            // label's 11px), and the label lands on the rung every other
            // "OK / Cancel / Apply" verb in the app already wears.
            // (drift 4px inset + 1px label size)
            kit::dialog_button(
                if selected_state.is_some() {
                    t!("Update from Drawing")
                } else {
                    t!("Save New State")
                },
                Message::LayerStateManagerSave,
                true,
            ),
        ]
        .spacing(tokens::GAP_ROW)
        .height(sizing.height),
    )
    .width(sizing.width)
    .height(sizing.height)
    // `[12, 12]` is a symmetric pair, so this is the same box written as the
    // rung it already was: 0px. The editor half of this file already wore
    // `GAP_SECTION` here, and the two halves of one window should not differ.
    .padding(tokens::GAP_SECTION);

    container(row![left, right].height(sizing.height))
        .width(sizing.width)
        .height(sizing.height)
        .into()
}

fn mask_for(property: LayerStateProperty) -> LayerStateMask {
    match property {
        LayerStateProperty::On => LayerStateMask::ON,
        LayerStateProperty::Frozen => LayerStateMask::FROZEN,
        LayerStateProperty::Locked => LayerStateMask::LOCKED,
        LayerStateProperty::Plot => LayerStateMask::PLOT,
        LayerStateProperty::NewViewport => LayerStateMask::NEW_VIEWPORT,
        LayerStateProperty::Color => LayerStateMask::COLOR,
        LayerStateProperty::LineType => LayerStateMask::LINE_TYPE,
        LayerStateProperty::LineWeight => LayerStateMask::LINE_WEIGHT,
        LayerStateProperty::PlotStyle => LayerStateMask::PLOT_STYLE,
        LayerStateProperty::Transparency => LayerStateMask::TRANSPARENCY,
    }
}

fn mask_button<'a>(
    state: &LayerState,
    label: Cow<'static, str>,
    property: LayerStateProperty,
) -> Element<'a, Message> {
    let enabled = state.mask.contains(mask_for(property));
    plain_button(
        row![
            text(if enabled { "✓" } else { " " }).size(tokens::TEXT_MICRO),
            text(label).size(tokens::TEXT_MICRO),
        ]
        .spacing(tokens::GAP_TIGHT)
        .align_y(iced::Center),
    )
    .on_press(Message::LayerStateEditorMaskToggle(property))
    .style(kit::chip_button_style(enabled))
    // `kit::chip` takes `Size::Compact`, and this control is a chip in a strip,
    // so it wears the same inset. It does *not* go through `kit::chip` itself:
    // the quiet half of that constructor is `Role::Standard`, whose resting
    // elevation and per-theme fade are exactly what `chip_button_style` exists
    // to withhold from a dialog with a dozen engagement states. (drift 1px)
    .padding(kit::Size::Compact.padding())
    .into()
}

fn bool_cell<'a>(
    value: bool,
    index: usize,
    flag: LayerStateLayerFlag,
    width: f32,
) -> Element<'a, Message> {
    container(
        checkbox(value)
            .on_toggle(move |_| Message::LayerStateEditorLayerFlagToggle(index, flag))
            .size(tokens::MARK_SQUARE),
    )
    .center_x(Length::Fixed(width))
    .width(Length::Fixed(width))
    .into()
}

fn editor_header<'a>() -> Element<'a, Message> {
    container(
        row![
            text(t!("Layer"))
                .size(tokens::TEXT_MICRO)
                .style(kit::muted_style)
                .width(Length::Fixed(170.0)),
            text(t!("On"))
                .size(tokens::TEXT_MICRO)
                .style(kit::muted_style)
                .width(Length::Fixed(44.0)),
            text(t!("Freeze"))
                .size(tokens::TEXT_MICRO)
                .style(kit::muted_style)
                .width(Length::Fixed(54.0)),
            text(t!("Lock"))
                .size(tokens::TEXT_MICRO)
                .style(kit::muted_style)
                .width(Length::Fixed(44.0)),
            text(t!("Plot"))
                .size(tokens::TEXT_MICRO)
                .style(kit::muted_style)
                .width(Length::Fixed(44.0)),
            text(t!("New VP"))
                .size(tokens::TEXT_MICRO)
                .style(kit::muted_style)
                .width(Length::Fixed(54.0)),
            text(t!("Color"))
                .size(tokens::TEXT_MICRO)
                .style(kit::muted_style)
                .width(Length::Fixed(135.0)),
            text(t!("Linetype"))
                .size(tokens::TEXT_MICRO)
                .style(kit::muted_style)
                .width(Length::Fixed(150.0)),
            text(t!("Lineweight"))
                .size(tokens::TEXT_MICRO)
                .style(kit::muted_style)
                .width(Length::Fixed(115.0)),
            text(t!("Plot style"))
                .size(tokens::TEXT_MICRO)
                .style(kit::muted_style)
                .width(Length::Fixed(135.0)),
            text(t!("Transparency"))
                .size(tokens::TEXT_MICRO)
                .style(kit::muted_style)
                .width(Length::Fixed(105.0)),
        ]
        .spacing(tokens::GAP_TIGHT)
        .align_y(iced::Center),
    )
    .padding(kit::Size::Compact.padding())
    // The header and the rows it labels must wear the *same* lateral inset or
    // every column shifts against its own name, so the row's rung wins over the
    // band's: `PAD_BAND` `[4, 10]` was the other candidate and would have moved
    // the header 2px off the cells. `kit::strip_style` still owns the plate.
    // (drift 1px)
    // A header band meets the table on three sides: `strip_style` is the one
    // chrome rung with a separating hairline and no box (Band fill + 1px edge,
    // square, unlit).
    .style(|theme: &Theme| kit::strip_style(theme, ""))
    .width(Fill)
    .into()
}

fn editor_layer_row<'a>(
    index: usize,
    layer: &'a codec::LayerStateLayer,
    color_open: bool,
    linetypes: std::sync::Arc<[String]>,
    // The colour cell's list floats over the rows under it; the glass pane it
    // becomes is tinted from the theme the editor is painted in.
    theme: &Theme,
) -> Element<'a, Message> {
    let current_linetype = Some(layer.line_type.clone());
    let current_lineweight = Some(LwItem(layer.line_weight));
    let color = crate::ui::color_select::color_selector(
        layer.color,
        color_open,
        crate::ui::color_select::ColorExtras {
            by_layer: false,
            by_block: false,
            ..Default::default()
        },
        move |color| Message::LayerStateEditorLayerColor(index, color),
        Message::LayerStateEditorLayerColorToggle(index),
        Message::OpenColorWindow(crate::app::ColorPickTarget::LayerState(index), layer.color),
        theme,
    );

    container(
        row![
            text(layer.layer_name.as_str())
                .size(tokens::TEXT_LABEL)
                .width(Length::Fixed(170.0)),
            bool_cell(!layer.off, index, LayerStateLayerFlag::On, 44.0),
            bool_cell(layer.frozen, index, LayerStateLayerFlag::Frozen, 54.0),
            bool_cell(layer.locked, index, LayerStateLayerFlag::Locked, 44.0),
            bool_cell(layer.plottable, index, LayerStateLayerFlag::Plot, 44.0),
            bool_cell(
                layer.new_viewport_frozen,
                index,
                LayerStateLayerFlag::NewViewport,
                54.0
            ),
            container(color).width(Length::Fixed(135.0)),
            crate::ui::kit::themed_pick_list(current_linetype, linetypes, |value| value.to_string(),)
                .on_select(move |value| Message::LayerStateEditorLayerLinetype(index, value))
                .text_size(tokens::TEXT_LABEL)
                .padding(kit::PAD_FIELD)
                .width(Length::Fixed(150.0)),
            crate::ui::kit::themed_pick_list(current_lineweight, lw_options(), |value| value.to_string(),)
                .on_select(move |item: LwItem| {
                    Message::LayerStateEditorLayerLineweight(index, item.0)
                })
                .text_size(tokens::TEXT_LABEL)
                .padding(kit::PAD_FIELD)
                .width(Length::Fixed(115.0)),
            text_input(t!("Default").as_ref(), &layer.plot_style)
                .on_input(move |value| Message::LayerStateEditorLayerPlotStyle(index, value))
                .size(tokens::TEXT_LABEL)
                .padding(kit::PAD_FIELD)
                .width(Length::Fixed(135.0)),
            crate::ui::kit::themed_pick_list(
                Some(TransparencyItem(layer.transparency)),
                transparency_options(layer.transparency),
                |value| value.to_string(),
            )
            .on_select(move |item| Message::LayerStateEditorLayerTransparency(index, item.0))
            .text_size(tokens::TEXT_LABEL)
            .padding(kit::PAD_FIELD)
            .width(Length::Fixed(105.0)),
        ]
        .spacing(tokens::GAP_TIGHT)
        .align_y(iced::Center),
    )
    .padding(kit::Size::Compact.padding())
    // Two rulings, one per level of this row:
    //   * the four controls inside it (three pick lists and the plot-style
    //     field) are *fields in a dense row*, so they take `kit::PAD_FIELD`
    //     `[3, 6]` — the pair `labeled_field_compact` already uses, and the one
    //     the snap planner wanted to raise to `[4, 6]` and charge 69 pixels for
    //     app-wide (finding F1);
    //   * the row itself is a row in a list, the same role as the manager's
    //     state rows above, so it takes `Size::Compact` — which is also what
    //     `editor_header` now wears, keeping the stripes under their names.
    // (drift 1px)
    // The recess ladder owns the stripe: even rows sit in the table's own
    // `Sink`, odd rows lift one notch to `Well`.
    .style(move |theme: &Theme| kit::stripe_style(theme, index, ""))
    .width(Fill)
    .into()
}

pub fn view_editor<'a>(
    state: &'a LayerState,
    filter: &'a str,
    color_open: Option<usize>,
    linetypes: std::sync::Arc<[String]>,
    sizing: crate::ui::shell::Sizing,
    // Rows are built by `editor_layer_row`, whose colour cell drops a glass
    // list — see that function. The window passes its own theme down.
    theme: &Theme,
) -> Element<'a, Message> {
    let properties = [
        (t!("On / Off"), LayerStateProperty::On),
        (t!("Freeze"), LayerStateProperty::Frozen),
        (t!("Lock"), LayerStateProperty::Locked),
        (t!("Plot"), LayerStateProperty::Plot),
        (t!("New VP"), LayerStateProperty::NewViewport),
        (t!("Color"), LayerStateProperty::Color),
        (t!("Linetype"), LayerStateProperty::LineType),
        (t!("Lineweight"), LayerStateProperty::LineWeight),
        (t!("Plot style"), LayerStateProperty::PlotStyle),
        (t!("Transparency"), LayerStateProperty::Transparency),
    ];
    let mask_controls = properties
        .into_iter()
        // Ten chips in a strip: `GAP_TIGHT` is the dense-chrome rung, and 5 was
        // a hand-tuned value one pixel off it. The chips' own inset comes from
        // `Size::Compact` inside `mask_button`, so the two now come from
        // neighbouring rungs instead of two typed numbers. (drift 1px)
        .fold(row![].spacing(tokens::GAP_TIGHT), |row, (label, property)| {
            row.push(mask_button(state, label, property))
        });

    let layer_names: Vec<String> = state
        .layers
        .iter()
        .map(|layer| layer.layer_name.clone())
        .collect();
    let query: Cow<str> = if filter.trim().is_empty() {
        Cow::Borrowed("")
    } else {
        Cow::Owned(filter.trim().to_lowercase())
    };
    let rows = state
        .layers
        .iter()
        .enumerate()
        .filter(|(_, layer)| {
            query.is_empty() || layer.layer_name.to_lowercase().contains(query.as_ref())
        })
        .fold(column![].spacing(tokens::GAP_NONE), |rows, (index, layer)| {
            rows.push(editor_layer_row(
                index,
                layer,
                color_open == Some(index),
                std::sync::Arc::clone(&linetypes),
                theme,
            ))
        });

    container(
        column![
            row![
                text(t!("Name")).size(tokens::TEXT_MICRO).style(kit::muted_style),
                text_input(t!("Layer state name").as_ref(), &state.name)
                    .on_input(Message::LayerStateEditorName)
                    .size(tokens::TEXT_LABEL)
                    // `[3, 6]` was already `kit::PAD_FIELD` to the pixel, so
                    // these three rows are a rename and move nothing — they were
                    // counted only because the pair was typed instead of named.
                    .padding(kit::PAD_FIELD)
                    .width(Length::Fixed(220.0)),
                text(t!("Description")).size(tokens::TEXT_MICRO).style(kit::muted_style),
                text_input(t!("Optional description").as_ref(), &state.description)
                    .on_input(Message::LayerStateEditorDescription)
                    .size(tokens::TEXT_LABEL)
                    .padding(kit::PAD_FIELD)
                    .width(sizing.width),
            ]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center),
            row![
                text(t!("%{n} saved layers", n = state.layers.len()))
                    .size(tokens::TEXT_MICRO)
                    .style(kit::muted_style),
                Space::new().width(sizing.width),
                text(t!("Current layer")).size(tokens::TEXT_MICRO).style(kit::muted_style),
                crate::ui::kit::themed_pick_list(Some(state.current_layer.clone()), layer_names, |value| {
                    value.to_string()
                },)
                .on_select(Message::LayerStateEditorCurrentLayer)
                .text_size(tokens::TEXT_LABEL)
                .padding(kit::PAD_FIELD)
                .width(Length::Fixed(180.0)),
            ]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center),
            kit::hdivider(sizing.width),
            text(t!("Properties restored by this state"))
                .size(tokens::TEXT_MICRO)
                .style(kit::muted_style),
            mask_controls,
            row![
                text(t!("Saved layer values")).size(tokens::TEXT_BODY),
                Space::new().width(sizing.width),
                text_input(t!("Search layers…").as_ref(), filter)
                    .on_input(Message::LayerStateEditorFilter)
                    .size(tokens::TEXT_LABEL)
                    // A 190px box sharing its row with a heading, in a window
                    // already carrying eleven fixed columns: this is the dense
                    // field rung, not the pane field's `[4, 10]`, because the
                    // box's text budget is the scarce thing here. (drift 2px)
                    .padding(kit::PAD_FIELD)
                    .width(Length::Fixed(190.0)),
            ]
            .align_y(iced::Center),
            container(
                column![editor_header(), crate::ui::kit::themed_scrollable(rows).height(sizing.height)]
                    .spacing(tokens::GAP_NONE)
                    .height(sizing.height),
            )
            .height(sizing.height)
            // The table sits in a shallow recess: `Tier::Well` keeps the hairline
            // edge and field corner this call site already spelled and adds the
            // recess fill the striped rows are meant to sit in.
            //
            // It takes no `PAD_WELL`, unlike the list recess in the manager
            // above, and the reason is arithmetic rather than taste: the eleven
            // columns here are `Length::Fixed` and sum to 1050px plus their gaps,
            // so an 8px lateral recess inset comes straight off the text budget
            // of a table whose width the shell may hand smaller than the 1180px
            // this window was measured at. The list's rows are `Fill` and lose
            // nothing they need. A rule that only holds when there is slack is
            // written down with its slack, not applied anyway.
            .style(|theme: &Theme| kit::Tier::Well.style(theme, "")),
            row![
                text(t!(
                    "Changes affect the saved state only; the drawing is unchanged until Restore."
                ))
                .size(tokens::TEXT_MICRO)
                .style(kit::muted_style),
                Space::new().width(sizing.width),
                // Cancel / Save Changes at the foot of an editor window: the
                // footer constructor, so `[6, 18]`, `TEXT_BODY` and the plate
                // all come from L1 instead of three numbers typed here.
                // (drift 7px inset + 1px label size, each)
                kit::dialog_button(t!("Cancel"), Message::LayerStateEditorCancel, false),
                kit::dialog_button(
                    t!("Save Changes"),
                    Message::LayerStateEditorSave,
                    true,
                ),
            ]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center),
        ]
        .spacing(tokens::GAP_ROW)
        .height(sizing.height),
    )
    .padding(tokens::GAP_SECTION)
    .width(sizing.width)
    .height(sizing.height)
    .into()
}

#[cfg(test)]
mod borrowed_linetypes_tests {
    // #21 RED: view_editor must take shared ownership of the linetype list
    // (a borrow cannot cross the twice-per-frame build closure) instead of
    // cloning per row — this call fails to compile until the signatures take
    // Arc<[String]>.
    use super::view_editor;

    #[test]
    fn view_editor_accepts_shared_linetypes() {
        let state = codec::LayerState {
            name: String::new(),
            description: String::new(),
            mask: codec::LayerStateMask::default(),
            hidden: false,
            current_layer: String::new(),
            layers: Vec::new(),
        };
        let linetypes: std::sync::Arc<[String]> =
            std::sync::Arc::from(vec!["Continuous".to_string()]);
        let sizing = crate::ui::shell::Sizing {
            width: iced::Length::Fixed(1180.0),
            height: iced::Length::Fixed(560.0),
        };
        let _ = view_editor(
            &state,
            "",
            None,
            std::sync::Arc::clone(&linetypes),
            sizing,
            // The editor's colour cell drops a glass list over the rows, so the
            // view needs a theme to tint it with; the test asserts ownership of
            // the linetype list, not which theme was picked.
            &iced::Theme::Dark,
        );
        // Shared ownership: the source stays usable after the call.
        assert_eq!(linetypes.len(), 1);
    }
}
