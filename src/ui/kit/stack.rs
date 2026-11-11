//! Stacks: the only place a screen is allowed to ask for a gap.
//!
//! Spacing literals were the single largest violation class in the audit
//! (`.spacing(<number>)` 1022 times), and they are why two panels that hold the
//! same kind of content sit 3px apart. These helpers take no numbers: the
//! rhythm comes from [`tokens`], and a screen that needs a gap not on the
//! ladder is a screen with a layout problem rather than a spacing one.
//!
//! The `labeled_*` rows moved here from `style::form` rather than to
//! [`super::control`], because what they own is the row: a label column of
//! [`LABEL_WIDTH`] and one control. The control's chrome is borrowed from
//! [`super::control`] and the label's voice from [`super::label`], so this is
//! the only module that composes across the kit rather than painting.

use super::control::{field_style, pick_list_style};
use super::label;
use crate::ui::read_only;
use crate::ui::style::tokens;
use iced::widget::{container, pick_list, row, text_input, Column, Row, Space};
use iced::{Element, Fill, Length};
use std::borrow::Cow;

/// Width of the label column in a `label : control` row. One value for the
/// whole app, so a form's colon column lines up across dialogs.
pub const LABEL_WIDTH: f32 = 92.0;

/// A vertical group at section rhythm.
pub fn vstack<'a, M: 'a>(children: Vec<Element<'a, M>>) -> Column<'a, M> {
    Column::with_children(children)
        .spacing(tokens::GAP_SECTION)
        .width(Fill)
}

/// A vertical group at row rhythm — the tighter step, for a label and its body.
pub fn vgroup<'a, M: 'a>(children: Vec<Element<'a, M>>) -> Column<'a, M> {
    Column::with_children(children)
        .spacing(tokens::GAP_ROW)
        .width(Fill)
}

/// A horizontal group of sibling controls.
pub fn hgroup<'a, M: 'a>(children: Vec<Element<'a, M>>) -> Row<'a, M> {
    Row::with_children(children)
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center)
}

/// A `label : control` form row. The label column is fixed so the controls
/// align; `control` takes the rest of the width.
pub fn form_row<'a, M: 'a>(
    label: impl Into<Element<'a, M>>,
    control: impl Into<Element<'a, M>>,
) -> Row<'a, M> {
    Row::with_children(vec![
        label.into(),
        Space::new().width(Length::Fixed(LABEL_WIDTH - tokens::GAP_ROW)).into(),
        control.into(),
    ])
    .align_y(iced::Center)
}

/// A `label : control` row where the control hugs its own content and the
/// remaining width is pushed to the right edge.
pub fn form_row_flush<'a, M: 'a>(
    label: impl Into<Element<'a, M>>,
    control: impl Into<Element<'a, M>>,
) -> Row<'a, M> {
    Row::with_children(vec![
        label.into(),
        Space::new().width(Fill).into(),
        control.into(),
    ])
    .spacing(tokens::GAP_ROW)
    .align_y(iced::Center)
}

// ── The `label : control` rows ────────────────────────────────────────────
//
// Moved out of `style::form` with their geometry intact: a 92px label column,
// [`tokens::GAP_ROW`] between the two, centred on the cross axis. They take no
// `track` because `form`'s 62 call sites do not have one to hand; the label
// voice is therefore resolved at full ink (`fade("")` is 1.0). When a dialog
// learns its entrance key, pass it down and these rows follow.

/// The label column of a `label : control` row: [`label::muted`], the same
/// 0.68 ink `form` painted with, pinned to `width` so the controls line up.
fn row_label<'a, M: 'a>(label: Cow<'static, str>, width: f32) -> Element<'a, M> {
    container(label::muted(label, "")).width(width).into()
}

/// A `label : dropdown` row.
pub fn labeled_pick_list<'a, T, M>(
    label: Cow<'static, str>,
    options: Vec<T>,
    selected: Option<T>,
    on_select: impl Fn(T) -> M + 'a,
    width: Length,
) -> Element<'a, M>
where
    T: ToString + Clone + PartialEq + 'a,
    M: Clone + 'a,
{
    let list = pick_list(selected, options, |value| value.to_string())
        .on_select(on_select)
        .text_size(tokens::TEXT_BODY)
        .padding([tokens::PAD_FIELD_Y, tokens::GAP_ROW])
        .width(width)
        .style(pick_list_style);
    row![row_label(label, LABEL_WIDTH), list]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center)
        .into()
}

/// A `label : dropdown` row that renders read-only when `enabled` is false.
pub fn labeled_pick_list_enabled<'a, T, M>(
    label: Cow<'static, str>,
    options: Vec<T>,
    selected: Option<T>,
    on_select: impl Fn(T) -> M + 'a,
    width: Length,
    enabled: bool,
) -> Element<'a, M>
where
    T: ToString + Clone + PartialEq + 'a,
    M: Clone + 'a,
{
    if enabled {
        return labeled_pick_list(label, options, selected, on_select, width);
    }
    row![
        row_label(label, LABEL_WIDTH),
        read_only::field(
            selected
                .map(|choice| choice.to_string())
                .unwrap_or_default()
                .as_str(),
            tokens::TEXT_BODY,
            width,
        ),
    ]
    .spacing(tokens::GAP_ROW)
    .align_y(iced::Center)
    .into()
}

/// A `label : text field` row.
pub fn labeled_field<'a, M>(
    label: Cow<'static, str>,
    value: &'a str,
    on_input: impl Fn(String) -> M + 'a,
    width: f32,
) -> Element<'a, M>
where
    M: Clone + 'a,
{
    row![
        row_label(label, LABEL_WIDTH),
        text_input("", value)
            .on_input(on_input)
            .style(field_style)
            .size(tokens::TEXT_BODY)
            .width(width),
    ]
    .spacing(tokens::GAP_ROW)
    .align_y(iced::Center)
    .into()
}

/// A `label : text field` row that renders read-only when `enabled` is false.
pub fn labeled_field_enabled<'a, M>(
    label: Cow<'static, str>,
    value: &'a str,
    on_input: impl Fn(String) -> M + 'a,
    width: f32,
    enabled: bool,
) -> Element<'a, M>
where
    M: Clone + 'a,
{
    if enabled {
        return labeled_field(label, value, on_input, width);
    }
    row![
        row_label(label, LABEL_WIDTH),
        read_only::field(value, tokens::TEXT_BODY, Length::Fixed(width)),
    ]
    .spacing(tokens::GAP_ROW)
    .align_y(iced::Center)
    .into()
}

/// A `label : text field` pair with its own label width, for rows that put
/// several small values side by side.
pub fn labeled_field_compact<'a, M>(
    label: Cow<'static, str>,
    value: &'a str,
    on_input: impl Fn(String) -> M + 'a,
    label_width: f32,
    field_width: f32,
) -> Element<'a, M>
where
    M: Clone + 'a,
{
    row![
        row_label(label, label_width),
        text_input("", value)
            .on_input(on_input)
            .style(field_style)
            .size(tokens::TEXT_BODY)
            .padding([tokens::PAD_FIELD_Y, tokens::GAP_CONTROL])
            .width(field_width),
    ]
    // 6px is `GAP_CONTROL`: the note here claimed this was the one number on
    // the row that was not on the ladder, which stopped being true when the
    // ladder grew a rung for the dense chrome (批 #4 定档). Literal removed so
    // the ratchet cannot re-grow it.
    .spacing(tokens::GAP_CONTROL)
    .align_y(iced::Center)
    .into()
}
