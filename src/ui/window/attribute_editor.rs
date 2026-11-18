//! Enhanced attribute editor dialog — edit the attributes of one block
//! reference (INSERT). Opened by double-clicking a block that carries
//! attributes, or by ATTEDIT with such a block selected.
//!
//! Three tabs, mirroring the standard enhanced attribute editor:
//! * **Attribute**   — the tag / prompt / value list; the selected row's value
//!                     is edited below the list.
//! * **Text Options** — the selected attribute's text formatting (style,
//!                     justification, height, rotation, width, oblique, flags).
//! * **Properties**  — the selected attribute's common entity properties
//!                     (layer, linetype, colour, lineweight).
//!
//! This module is pure layout over a working copy ([`AttrRow`]); applying the
//! edits, undo and repaint live in the update handler (`Message::AttrEditorApply`).

use crate::ui::style::tokens;
use crate::app::Message;
use crate::t;
use codec::entities::{HorizontalAlignment, VerticalAlignment};
use codec::types::{Color as AcadColor, LineWeight};
use iced::widget::{checkbox, column, container, row, text, text_input, Space};
use crate::ui::kit::plain_button;
use iced::{Element, Length};
use std::borrow::Cow;
use std::fmt;

use crate::ui::properties::{lw_options, LwItem};
use crate::ui::kit::muted_style;
use crate::ui::kit::hdivider;
use crate::ui::kit;

const LABEL_W: f32 = 120.0;

/// Which tab of the editor is showing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AttrTab {
    Attribute,
    TextOptions,
    Properties,
}

/// Editable working copy of one attribute. Numeric fields are kept as strings
/// so the user can type freely; they are parsed when Apply is pressed. Angles
/// are shown and entered in degrees.
#[derive(Clone)]
pub struct AttrRow {
    pub tag: String,
    pub prompt: String,
    pub value: String,
    // Text options
    pub text_style: String,
    pub height: String,
    pub rotation: String,
    pub width_factor: String,
    pub oblique: String,
    pub h_align: HorizontalAlignment,
    pub v_align: VerticalAlignment,
    pub backwards: bool,
    pub upside_down: bool,
    // Properties (common entity)
    pub layer: String,
    pub color: AcadColor,
    pub linetype: String,
    pub line_weight: LineWeight,
}

/// Combined justification options (horizontal × vertical), matching the
/// standard text-justification list. The stored labels are the English source
/// texts; `justify_label` / `justify_from_label` translate both directions.
pub const JUSTIFY: &[(&str, HorizontalAlignment, VerticalAlignment)] = &[
    (
        "Left",
        HorizontalAlignment::Left,
        VerticalAlignment::Baseline,
    ),
    (
        "Center",
        HorizontalAlignment::Center,
        VerticalAlignment::Baseline,
    ),
    (
        "Right",
        HorizontalAlignment::Right,
        VerticalAlignment::Baseline,
    ),
    (
        "Aligned",
        HorizontalAlignment::Aligned,
        VerticalAlignment::Baseline,
    ),
    (
        "Middle",
        HorizontalAlignment::Middle,
        VerticalAlignment::Baseline,
    ),
    ("Fit", HorizontalAlignment::Fit, VerticalAlignment::Baseline),
    (
        "Top Left",
        HorizontalAlignment::Left,
        VerticalAlignment::Top,
    ),
    (
        "Top Center",
        HorizontalAlignment::Center,
        VerticalAlignment::Top,
    ),
    (
        "Top Right",
        HorizontalAlignment::Right,
        VerticalAlignment::Top,
    ),
    (
        "Middle Left",
        HorizontalAlignment::Left,
        VerticalAlignment::Middle,
    ),
    (
        "Middle Center",
        HorizontalAlignment::Center,
        VerticalAlignment::Middle,
    ),
    (
        "Middle Right",
        HorizontalAlignment::Right,
        VerticalAlignment::Middle,
    ),
    (
        "Bottom Left",
        HorizontalAlignment::Left,
        VerticalAlignment::Bottom,
    ),
    (
        "Bottom Center",
        HorizontalAlignment::Center,
        VerticalAlignment::Bottom,
    ),
    (
        "Bottom Right",
        HorizontalAlignment::Right,
        VerticalAlignment::Bottom,
    ),
];

/// The justification label for an (h, v) pair (defaults to "Left"). Kept in
/// English (like the colour names) — the labels are standard CAD terms and
/// their keys would collide with ribbon tool labels such as "Right".
pub fn justify_label(h: HorizontalAlignment, v: VerticalAlignment) -> Cow<'static, str> {
    JUSTIFY
        .iter()
        .find(|(_, jh, jv)| *jh == h && *jv == v)
        .map(|(l, _, _)| Cow::Borrowed(*l))
        .unwrap_or(Cow::Borrowed("Left"))
}

/// Resolve a justification label back to its (h, v) pair.
pub fn justify_from_label(label: &str) -> Option<(HorizontalAlignment, VerticalAlignment)> {
    JUSTIFY
        .iter()
        .find(|(l, _, _)| *l == label)
        .map(|(_, h, v)| (*h, *v))
}

const COLOR_OPTIONS: &[&str] = &[
    "ByLayer", "ByBlock", "Red", "Yellow", "Green", "Cyan", "Blue", "Magenta", "White",
];

/// Display label for a colour (falls back to `Color <n>` for non-standard ACI).
pub fn color_label(c: AcadColor) -> String {
    match c {
        AcadColor::ByLayer => "ByLayer".into(),
        AcadColor::None => "None".into(),
        AcadColor::ByBlock => "ByBlock".into(),
        AcadColor::Index(1) => "Red".into(),
        AcadColor::Index(2) => "Yellow".into(),
        AcadColor::Index(3) => "Green".into(),
        AcadColor::Index(4) => "Cyan".into(),
        AcadColor::Index(5) => "Blue".into(),
        AcadColor::Index(6) => "Magenta".into(),
        AcadColor::Index(7) => "White".into(),
        AcadColor::Index(n) => crate::tf!("Color {n}").into_owned(),
        AcadColor::Rgb { r, g, b } => format!("{r},{g},{b}"),
    }
}

/// Resolve a colour label back to an `AcadColor` (None if unknown).
pub fn color_from_label(label: &str) -> Option<AcadColor> {
    Some(match label {
        "ByLayer" => AcadColor::ByLayer,
        "ByBlock" => AcadColor::ByBlock,
        "Red" => AcadColor::Index(1),
        "Yellow" => AcadColor::Index(2),
        "Green" => AcadColor::Index(3),
        "Cyan" => AcadColor::Index(4),
        "Blue" => AcadColor::Index(5),
        "Magenta" => AcadColor::Index(6),
        "White" => AcadColor::Index(7),
        _ => return None,
    })
}

/// One `label : widget` row with a fixed-width label column.
fn field_row<'a>(
    label: Cow<'static, str>,
    widget: Element<'a, Message>,
    width: Length,
) -> Element<'a, Message> {
    row![
        container(text(label).size(tokens::TEXT_BODY).style(muted_style)).width(LABEL_W),
        widget,
    ]
    .spacing(tokens::GAP_ROW)
    .align_y(iced::Center)
    .width(width)
    .into()
}

/// A labelled editable text field for one of the selected row's string buffers.
fn edit_field<'a>(
    label: Cow<'static, str>,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
    width: Length,
) -> Element<'a, Message> {
    let ti = text_input("", value)
        .on_input(on_input)
        .on_submit(Message::AttrEditorApply)
        .style(kit::field_style)
        .size(tokens::TEXT_BODY)
        .padding(kit::PAD_FIELD)
        .width(width);
    field_row(label, ti.into(), width)
}

/// A labelled pick_list of owned string options.
fn pick_field<'a>(
    label: Cow<'static, str>,
    options: Vec<String>,
    selected: Option<String>,
    on_select: impl Fn(String) -> Message + 'a,
    width: Length,
) -> Element<'a, Message> {
    let pl = crate::ui::kit::themed_pick_list(selected, options, |value| value.to_string())
        .on_select(on_select)
        .text_size(tokens::TEXT_BODY)
        .padding(kit::PAD_FIELD)
        .width(width);
    field_row(label, pl.into(), width)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LocalizedChoice {
    raw: String,
    label: String,
}

impl LocalizedChoice {
    fn new(raw: String) -> Self {
        let label = raw
            .strip_prefix("Color ")
            .map(|number| crate::tf!("Color {}", number).into_owned())
            .unwrap_or_else(|| crate::i18n::translate(&raw).into_owned());
        Self { raw, label }
    }
}

impl fmt::Display for LocalizedChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

/// A translated pick list which keeps the stored and emitted value unchanged.
fn localized_pick_field<'a>(
    label: Cow<'static, str>,
    options: Vec<String>,
    selected: Option<String>,
    on_select: impl Fn(String) -> Message + 'a,
    width: Length,
) -> Element<'a, Message> {
    let options = options
        .into_iter()
        .map(LocalizedChoice::new)
        .collect::<Vec<_>>();
    let selected = selected.map(LocalizedChoice::new);
    let pl = crate::ui::kit::themed_pick_list(selected, options, |value| value.to_string())
        .on_select(move |choice: LocalizedChoice| on_select(choice.raw))
        .text_size(tokens::TEXT_BODY)
        .padding(kit::PAD_FIELD)
        .width(width);
    field_row(label, pl.into(), width)
}

fn tab_button<'a>(
    label: Cow<'static, str>,
    this: AttrTab,
    active: AttrTab,
) -> Element<'a, Message> {
    let is_active = this == active;
    plain_button(text(label).size(tokens::TEXT_LABEL))
        // A tab is a sibling in a strip, and the three other tab strips in this
        // app that wear `chip_button_style` size their chips with
        // `kit::Size::Control` (`style/dimstyle.rs:397`, and `layer_state_manager`
        // for the same role). `[4, 12]` was 2px wider than the rung, so this is
        // the one place where the *role* outranks the nearest number: the snap
        // planner offered `GAP_TIGHT + GAP_SECTION` at 0px, which would have left
        // this strip's tabs a different width from every other tab in the app.
        // (drift 2px lateral)
        .padding(kit::Size::Control.padding())
        .on_press(Message::AttrEditorTab(this))
        .style(crate::ui::kit::chip_button_style(is_active))
        .into()
}

/// Build the enhanced attribute editor. `rows` is the working copy (attribute
/// order); `selected` is the highlighted row driving the Text Options /
/// Properties tabs. `layers` / `linetypes` / `styles` are the document's pick
/// lists.
pub fn view_window<'a>(
    block: &'a str,
    rows: &'a [AttrRow],
    selected: usize,
    tab: AttrTab,
    layers: Vec<String>,
    linetypes: Vec<String>,
    styles: Vec<String>,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let width = sizing.width;
    let height = sizing.height;
    // ── Top toolbar: block name on the left, Apply on the right ───────────
    // Mirrors the style-manager windows (actions left, primary action right).
    // The dialog's one commit verb. `kit::dialog_button` is its only constructor
    // in this codebase (53 sites, 25 files) and it settles three decisions at
    // once: the `Size::Action` `[6, 18]` inset, the plate — its `primary` arm
    // calls the very `primary_style` this site already wore, so the plate itself
    // does not move — and the label's size, which lands on `TEXT_BODY`, the rung
    // every other "OK / Cancel / Apply" in the app already wears.
    // (drift 2px vertical + 4px lateral inset, 1px label size)
    let apply = kit::dialog_button(t!("Apply"), Message::AttrEditorApply, true);
    // A band, by its role: full width, chrome, holding a readout and an action.
    // `Tier::Band` is the ladder rung for exactly that (`tokens::Rank::Band`,
    // "quick bar, tab strip, status bar"), so the plate stops being iced's private
    // `background.weak` slot and becomes L0's first raised rung — the same move the
    // three already-rebuilt window toolbars made. `wash` and not `strip_style`
    // because this band keeps its own explicit seam below it (`hdivider`), and a
    // strip's four-sided hairline would double it and box the window in.
    // (drift: plate moves weak → Band rung; ΔL* not re-measured here, it is the
    //  ladder's own page→Band step. `snap: true` comes with every tier.)
    let toolbar = kit::wash(
        row![
            text(t!("Block:  %{block}", block = block))
                .size(tokens::TEXT_BODY)
                .style(muted_style),
            Space::new().width(width),
            apply,
        ]
        .align_y(iced::Center),
        kit::Tier::Band,
        // `[5, 8]` was one off the rung vertically; `Size::Compact` is what the
        // other window toolbars in this app pass (`layers.rs:473`).
        // (drift 1px vertical)
        kit::Size::Compact.padding(),
        "",
    )
    .width(width);

    let tabs = row![
        tab_button(t!("Attribute"), AttrTab::Attribute, tab),
        tab_button(t!("Text Options"), AttrTab::TextOptions, tab),
        tab_button(t!("Properties"), AttrTab::Properties, tab),
    ]
    .spacing(tokens::GAP_HAIRLINE);

    let body: Element<'_, Message> = if rows.is_empty() {
        text(t!("This block has no attributes."))
            .size(tokens::TEXT_BODY)
            .style(muted_style)
            .into()
    } else {
        match tab {
            AttrTab::Attribute => attribute_tab(rows, selected, width, height),
            AttrTab::TextOptions => {
                text_options_tab(&rows[selected.min(rows.len() - 1)], styles, width, height)
            }
            AttrTab::Properties => properties_tab(
                &rows[selected.min(rows.len() - 1)],
                layers,
                linetypes,
                width,
                height,
            ),
        }
    };

    let content = container(column![tabs, body].spacing(tokens::GAP_ROW))
        .width(width)
        .height(height)
        .padding(tokens::GAP_SECTION);

    container(column![toolbar, hdivider(width), content])
        .width(width)
        .height(height)
        .into()
}

/// Attribute tab: tag / prompt / value list with row-select, plus a value box.
fn attribute_tab<'a>(
    rows: &'a [AttrRow],
    selected: usize,
    width: Length,
    height: Length,
) -> Element<'a, Message> {
    let head = row![
        container(text(t!("Tag")).size(tokens::TEXT_LABEL).style(muted_style)).width(130),
        container(text(t!("Prompt")).size(tokens::TEXT_LABEL).style(muted_style)).width(width),
        container(text(t!("Value")).size(tokens::TEXT_LABEL).style(muted_style)).width(140),
    ]
    .spacing(tokens::GAP_CONTROL);

    // `1` sat under the smallest rung; `GAP_HAIRLINE` (2) is what the tab strip at
    // the top of this same dialog already wears, so the list now breathes like the
    // chrome around it. (drift 1px per gap)
    let mut list = column![].spacing(tokens::GAP_HAIRLINE);
    for (idx, r) in rows.iter().enumerate() {
        let is_sel = idx == selected;
        let line = row![
            container(text(r.tag.as_str()).size(tokens::TEXT_BODY)).width(130),
            container(text(r.prompt.as_str()).size(tokens::TEXT_BODY).style(muted_style)).width(width),
            container(text(r.value.as_str()).size(tokens::TEXT_BODY)).width(140),
        ]
        .spacing(tokens::GAP_CONTROL);
        // The row is a selectable line in a data list, and `kit::list_row` is the
        // app's only answer to that (the layer list, the dock's file list, the
        // reference manager). Five numbers this screen owned move onto the rung:
        //   * inset `[3, 4]` → `Size::Compact` `[4, 8]` (the row's own budget,
        //     and now the same inset as the header it cannot line up with by
        //     column widths alone),
        //   * selected wash `0.10` → `tokens::TINT_SELECTED` 0.12, and the hover
        //     step becomes `TINT_ENGAGED` 0.16 instead of holding the selection
        //     wash flat,
        //   * resting hover `base.text @ 0.05` → `tokens::WASH_POINTER` 0.06,
        //   * selected ink `primary.weak.text` → `background.base.text`, i.e. the
        //     row's ink stops tracking the accent and tracks the plate it is on —
        //     the #73 pairing, solved inside the primitive now,
        //   * `Pressed` joins `Hovered` as "the pointer is on this row", and a
        //     disabled row gets the kit's `dim` step instead of staying lit.
        // None of those five were signed off; they are the price of the fold and
        // they are the reason the fold is worth making (every one of them was a
        // per-screen guess before).
        list = list.push(
            kit::list_row(
                line,
                is_sel,
                kit::Press::On(Message::AttrEditorSelect(idx)),
                "",
            )
            .width(width),
        );
    }

    let sel = selected.min(rows.len() - 1);
    let value_box = text_input("", rows[sel].value.as_str())
        .on_input(move |v| Message::AttrEditorInput { idx: sel, value: v })
        .on_submit(Message::AttrEditorApply)
        .style(kit::field_style)
        .size(tokens::TEXT_BODY)
        // This is a field in a labelled row — the same construction as
        // `edit_field` one screen-region up — so it takes `kit::PAD_FIELD`
        // `[3, 6]` and not the `[4, 6]` it had typed. The snap planner offered
        // `kit::PAD_PLATE` here at 0 px because 4 is nearer 4 than 3; that answer
        // is by number, not by role, and it would have left the dialog's five
        // fields on two different insets. (drift 1px vertical)
        .padding(kit::PAD_FIELD)
        .width(width);

    column![
        head,
        crate::ui::kit::themed_scrollable(list).width(width).height(height),
        // `8` is `GAP_ROW` to the pixel. The ratchet does not count this row (it
        // sits inside a `column![]`, not a `.padding()` call — finding F4), so
        // naming it costs nothing and removes a number the board never saw.
        Space::new().height(tokens::GAP_ROW),
        field_row(t!("Value:"), value_box.into(), width),
    ]
    .spacing(tokens::GAP_CONTROL)
    .width(width)
    .height(height)
    .into()
}

/// Text Options tab: the selected attribute's text formatting.
fn text_options_tab<'a>(
    r: &'a AttrRow,
    styles: Vec<String>,
    width: Length,
    height: Length,
) -> Element<'a, Message> {
    let style_sel = if r.text_style.is_empty() {
        None
    } else {
        Some(r.text_style.clone())
    };
    let justify_opts: Vec<String> = JUSTIFY.iter().map(|(l, _, _)| (*l).to_string()).collect();
    let justify_sel = Some(justify_label(r.h_align, r.v_align).to_string());

    column![
        pick_field(
            t!("Text Style"),
            styles,
            style_sel,
            |s| { Message::AttrEditorTextStyle(s) },
            width
        ),
        localized_pick_field(
            t!("Justification"),
            justify_opts,
            justify_sel,
            |s| { Message::AttrEditorJustify(s) },
            width
        ),
        edit_field(t!("Height"), &r.height, Message::AttrEditorHeight, width),
        edit_field(
            t!("Rotation"),
            &r.rotation,
            Message::AttrEditorRotation,
            width
        ),
        edit_field(
            t!("Width Factor"),
            &r.width_factor,
            Message::AttrEditorWidth,
            width
        ),
        edit_field(
            t!("Oblique Angle"),
            &r.oblique,
            Message::AttrEditorOblique,
            width
        ),
        field_row(
            Cow::Borrowed(""),
            checkbox(r.backwards)
                .label(t!("Backwards"))
                .on_toggle(Message::AttrEditorBackwards)
                .size(tokens::MARK_SQUARE)
                .text_size(tokens::TEXT_BODY)
                .into(),
            width,
        ),
        field_row(
            Cow::Borrowed(""),
            checkbox(r.upside_down)
                .label(t!("Upside down"))
                .on_toggle(Message::AttrEditorUpsideDown)
                .size(tokens::MARK_SQUARE)
                .text_size(tokens::TEXT_BODY)
                .into(),
            width,
        ),
    ]
    .spacing(tokens::GAP_ROW)
    .width(width)
    .height(height)
    .into()
}

/// Properties tab: the selected attribute's common entity properties.
fn properties_tab<'a>(
    r: &'a AttrRow,
    layers: Vec<String>,
    linetypes: Vec<String>,
    width: Length,
    height: Length,
) -> Element<'a, Message> {
    let layer_sel = Some(r.layer.clone());
    let lt_sel = Some(if r.linetype.is_empty() {
        "ByLayer".to_string()
    } else {
        r.linetype.clone()
    });
    let mut color_opts: Vec<String> = COLOR_OPTIONS.iter().map(|s| s.to_string()).collect();
    // The attribute may carry a colour outside the standard set (ACI 8-255 or
    // RGB). Surface its label so it displays correctly and stays selectable,
    // rather than showing blank; picking a listed colour still overrides it.
    let cur_color = color_label(r.color);
    if !color_opts.iter().any(|o| o == &cur_color) {
        color_opts.insert(0, cur_color.clone());
    }
    let color_sel = Some(cur_color);

    let lw_opts = lw_options();
    let lw_sel = LwItem(r.line_weight);
    let lw = crate::ui::kit::themed_pick_list(Some(lw_sel), lw_opts, |value| value.to_string())
        .on_select(|it: LwItem| Message::AttrEditorLineweight(it.0))
        .text_size(tokens::TEXT_BODY)
        .padding(kit::PAD_FIELD)
        .width(width);

    column![
        pick_field(
            t!("Layer"),
            layers,
            layer_sel,
            Message::AttrEditorLayer,
            width
        ),
        localized_pick_field(
            t!("Linetype"),
            linetypes,
            lt_sel,
            Message::AttrEditorLinetype,
            width
        ),
        localized_pick_field(
            t!("Color"),
            color_opts,
            color_sel,
            |s| { Message::AttrEditorColor(s) },
            width
        ),
        field_row(t!("Lineweight"), lw.into(), width),
    ]
    .spacing(tokens::GAP_ROW)
    .width(width)
    .height(height)
    .into()
}
