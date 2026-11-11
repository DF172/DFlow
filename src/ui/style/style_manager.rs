//! Shared scaffold for every style-manager window.
//!
//! All five managers (text / dimension / table / multileader / multiline)
//! share the same frame: a top toolbar (New / Copy / Delete on the left,
//! manager-specific actions such as Set Current / Apply on the right), a style
//! list on the left, and a property editor on the right. Only the editor
//! differs, so each manager builds just that and hands it to [`view`]; the
//! toolbar, list, inline-rename wiring and chrome live here once.

use crate::ui::kit::muted_style;
use crate::ui::style::tokens;
use crate::app::{Message, StyleKind};
use crate::t;
use crate::ui::kit;
use iced::widget::{column, container, row, text, Space};
use crate::ui::kit::plain_button;
use iced::{Element, Length};
use std::borrow::Cow;

/// Everything the shared frame needs. The per-manager `editor` element is the
/// only bespoke part.
///
/// The toolbar is uniform across every manager: New / Copy / Delete on the
/// left, then **Set Current** and **Apply** on the right. Each manager just
/// supplies the two messages, so the right side can never drift or go missing.
///
/// Two lifetimes: `'a` is what the returned element keeps alive (the editor and
/// the rename buffer the inline `text_input` borrows); `'b` is the transient
/// list data (`styles`, `selected`, …) that the frame only reads while building
/// rows, so callers may pass a locally-built `Vec`.
pub struct Scaffold<'a, 'b> {
    pub sizing: crate::ui::shell::Sizing,
    pub kind: StyleKind,
    pub styles: &'b [String],
    pub selected: &'b str,
    /// Parallel to `styles`: whether the drawing references that style. The
    /// list tags such rows "used in drawing", the way the sample managers do.
    /// A manager that cannot answer passes an empty slice.
    pub used: &'b [bool],
    /// Current style for this manager, marked with a dot and a pill. `None`
    /// when the manager has no "current" concept.
    pub current: Option<&'b str>,
    pub rename_active: Option<&'b str>,
    pub rename_buf: &'a str,
    pub on_new: Message,
    pub on_copy: Message,
    pub on_delete: Message,
    /// Tuple-variant constructor for the per-row select message
    /// (e.g. `Message::TextStyleDialogSelect`).
    pub on_select: fn(String) -> Message,
    /// "Set Current" action (right side). Every manager has one.
    pub on_set_current: Message,
    /// "Apply" action (right side, primary). Every manager has one.
    pub on_apply: Message,
    /// Referenced styles may be copied but not changed, renamed, deleted, or
    /// made current.
    pub read_only: bool,
    pub editor: Element<'a, Message>,
}

pub fn view<'a, 'b>(s: Scaffold<'a, 'b>) -> Element<'a, Message> {
    let width = s.sizing.width;
    let height = s.sizing.height;
    // ── Toolbar: New / Copy / Delete | … | Set Current / Apply ────────────
    let bar = row![
        tb_button(t!("New"), s.on_new, false),
        tb_button(t!("Copy"), s.on_copy, false),
        tb_button_enabled(t!("Delete"), s.on_delete, false, !s.read_only),
        Space::new().width(width),
        tb_button_enabled(t!("Set Current"), s.on_set_current, false, !s.read_only),
        tb_button_enabled(t!("Apply"), s.on_apply, true, !s.read_only),
    ]
    .spacing(tokens::GAP_TIGHT)
    .align_y(iced::Center);
    let toolbar = container(bar)
        .style(kit::chrome_plate_style)
        .width(width)
        .padding(kit::PAD_BAND);

    // ── Left: style list (single click selects, double click renames) ─────
    let rows: Vec<Element<'_, Message>> = s
        .styles
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let is_sel = name.as_str() == s.selected;
            let is_current = s.current == Some(name.as_str());
            let is_used = s.used.get(index).copied().unwrap_or(false);
            crate::ui::style::style_list::item(
                name,
                is_current,
                is_used,
                is_sel,
                s.kind,
                (s.on_select)(name.clone()),
                s.rename_active,
                s.rename_buf,
                !(s.read_only && is_sel),
            )
        })
        .collect();

    let list_panel = container(
        column![
            row![
                text(t!("Styles")).size(tokens::TEXT_MICRO).style(muted_style),
                Space::new().width(Length::Fill),
                text(s.styles.len().to_string())
                    .size(tokens::TEXT_MICRO)
                    .style(muted_style),
            ]
            .align_y(iced::Center),
            container(crate::ui::kit::themed_scrollable(
                column(rows).spacing(tokens::GAP_SEAM),
            )
            .height(height))
                .style(kit::pane_plate_style)
                .width(width)
                .height(height)
                .padding(tokens::GAP_HAIRLINE),
        ]
        .spacing(tokens::GAP_TIGHT)
        .height(height),
    )
    .width(170)
    .height(height)
    .padding(iced::Padding {
        top: tokens::GAP_SECTION,
        right: tokens::GAP_ROW,
        bottom: tokens::GAP_SECTION,
        left: tokens::GAP_SECTION,
    });

    let body = row![list_panel, vsep(height), s.editor].height(height);

    container(column![toolbar, hdivider(width), body])
        .width(width)
        .height(height)
        .into()
}

pub struct EditorTab {
    pub label: Cow<'static, str>,
    pub active: bool,
    pub on_press: Message,
}

pub struct EditorComparison {
    pub selected: String,
    pub options: Vec<String>,
    pub summary: String,
    pub on_select: fn(String) -> Message,
}

/// Shared right-hand editor composition used by every named style manager.
/// The preview stays visible while the active property page scrolls.
pub struct EditorShell<'a> {
    pub sizing: crate::ui::shell::Sizing,
    pub selected: String,
    pub status: Cow<'static, str>,
    pub preview: Element<'a, Message>,
    pub comparison: Option<EditorComparison>,
    pub tabs: Vec<EditorTab>,
    pub content: Element<'a, Message>,
}

pub fn editor_shell<'a>(shell: EditorShell<'a>) -> Element<'a, Message> {
    let tabs: Vec<Element<'a, Message>> = shell
        .tabs
        .into_iter()
        .map(|tab| {
            plain_button(text(crate::t!(&tab.label)).size(tokens::TEXT_LABEL))
                .on_press(tab.on_press)
                .style(kit::chip_button_style(tab.active))
                .padding(kit::Size::Control.padding())
                .into()
        })
        .collect();
    let comparison: Element<'a, Message> = match shell.comparison {
        Some(comparison) if !comparison.options.is_empty() => row![
            text(t!("Compare with")).size(tokens::TEXT_MICRO).style(muted_style),
            crate::ui::kit::themed_pick_list(Some(comparison.selected), comparison.options, |value| value
                .to_string(),)
            .on_select(comparison.on_select)
            .text_size(tokens::TEXT_LABEL)
            .width(150),
            text(comparison.summary).size(tokens::TEXT_MICRO).style(muted_style),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center)
        .into(),
        _ => Space::new().height(tokens::GAP_NONE).into(),
    };
    let preview = container(shell.preview)
        .width(Length::Fill)
        .style(kit::pane_plate_style);
    container(
        column![
            row![
                text(shell.selected)
                    .size(tokens::TEXT_BODY)
                    .style(kit::accent_style),
                Space::new().width(Length::Fill),
                text(shell.status).size(tokens::TEXT_MICRO).style(muted_style),
            ]
            .align_y(iced::Center),
            preview,
            comparison,
            row(tabs).spacing(tokens::GAP_HAIRLINE),
            hdivider(Length::Fill),
            crate::ui::kit::themed_scrollable(
                container(shell.content).padding(kit::PAD_CARD).width(Length::Fill)
            )
            .width(Length::Fill)
            .height(Length::Fill),
        ]
        .spacing(tokens::GAP_CONTROL)
        .height(shell.sizing.height),
    )
    .height(shell.sizing.height)
    .width(Length::Fill)
    .padding(iced::Padding {
        top: tokens::GAP_SECTION,
        right: tokens::GAP_NONE,
        bottom: tokens::GAP_SECTION,
        left: tokens::GAP_NONE,
    })
    .into()
}

// ── Shared chrome ──────────────────────────────────────────────────────────

/// A toolbar button from the two orthogonal kit axes: [`kit::Role`] says whether
/// it is the action this screen is asking for, [`kit::Size::Control`] says how
/// big a toolbar control is.
///
/// The old shape conflated them: `accent` chose both the plate *and* a wider
/// inset (`[4, 14]` against the quiet button's `[4, 10]`), so an emphasised
/// toolbar button was a different size class from its neighbours for a reason
/// the ladder does not have a name for. Landing both branches on `Control`
/// narrows the emphasised button by 4 px of horizontal inset; `kit::Role::Primary`
/// still carries the gradient and the glow, which is what actually says
/// "this one". The count of what that repaints is larger than this file: the
/// emphasised branch is *written* at 3 call sites (`view`'s Apply here,
/// `scale_manager.rs:46`, `anno_object_scale.rs:28`) and *drawn* on 7 windows,
/// because the five named-style managers all build their toolbar through `view`.
pub(crate) fn tb_button(
    label: impl Into<Cow<'static, str>>,
    msg: Message,
    accent: bool,
) -> Element<'static, Message> {
    kit::btn(
        text(label.into()).size(tokens::TEXT_LABEL),
        if accent {
            kit::Role::Primary
        } else {
            kit::Role::Standard
        },
        kit::Size::Control,
        kit::Press::On(msg),
        "",
    )
    .into()
}

pub(crate) fn tb_button_enabled(
    label: impl Into<Cow<'static, str>>,
    msg: Message,
    accent: bool,
    enabled: bool,
) -> Element<'static, Message> {
    kit::btn(
        text(label.into()).size(tokens::TEXT_LABEL),
        if accent {
            kit::Role::Primary
        } else {
            kit::Role::Standard
        },
        kit::Size::Control,
        if enabled {
            kit::Press::On(msg)
        } else {
            kit::Press::Off
        },
        "",
    )
    .into()
}

pub(crate) use crate::ui::kit::{hdivider, vsep};
