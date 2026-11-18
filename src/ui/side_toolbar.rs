//! Reusable right-edge vertical toolbar.
//!
//! A single centred column of icon buttons floated over the right edge of the
//! canvas — a lightweight, context-specific alternative to a contextual ribbon
//! tab. It is built from a flat list of [`ToolDef`]s and dispatches each tool's
//! command through the existing [`Message::RibbonToolClick`] path, so any
//! module's tools can drive it. First used for paper-space viewport / plot
//! actions; reusable for any future context action set.
//!
//! This is an L3 screen: it composes and never paints. Both plates here float
//! over the drawing, so both are glass panes — the toolbar's own pane and the
//! one-pane-above tooltip — and a tool button wears `Role::Engaged` when the
//! tool is on and `Role::Standard` at rest and on hover. Every paint decision
//! stays inside the kit; this file writes no stylesheet literal and no
//! hand-typed radius, gap or point size.

use crate::ui::kit::{body, glass_pane, icon_btn, surface, Press, Role, Tier, PAD_PLATE};
use crate::ui::style::tokens;
use iced::widget::{column, container, text, tooltip};
use iced::{Element, Length, Theme};

use crate::app::Message;
use crate::modules::{IconKind, ToolDef};

const ICON_SIZE: f32 = 22.0;
const BTN_SIZE: f32 = 38.0;
/// Gap between the toolbar and the right edge of the canvas.
const EDGE_MARGIN: f32 = 8.0;
/// Room kept free at the top right for the view cube and the UCS list; the
/// toolbar is centred in the height below it.
const TOP_RESERVE: f32 = 200.0;
/// This overlay has no entrance-fade key of its own yet, so every kit call
/// passes the empty track: `motion::value_or` answers with its default and the
/// paints stay at full strength. When the canvas learns a key, this is the only
/// place to change.
const NO_TRACK: &'static str = "";

fn icon_el(icon: IconKind) -> Element<'static, Message> {
    match icon {
        IconKind::Glyph(s) => text(s).size(ICON_SIZE * 0.85).into(),
        IconKind::Svg(bytes) => crate::ui::icons::semantic(bytes, ICON_SIZE),
    }
}

/// A tool's name, on one plane above the toolbar plate.
fn tip_panel(tip: &'static str, theme: &Theme) -> Element<'static, Message> {
    // Ribbon labels break over two lines; a tooltip reads them on one.
    let caption = crate::t!(tip).replace('\n', " ");
    // 11px became `TEXT_BODY` (14 since the 2026-11-02 lift): 11 was never on
    // the type scale, and the kit has no rung below body except `key`'s
    // arithmetic `TEXT_BODY - 2`.
    // `theme` mixes the glass: the pane resolves its colours while it is built,
    // because the shader that draws it is never handed a theme of its own.
    glass_pane(
        surface(body(caption, NO_TRACK), Tier::Flush, PAD_PLATE, NO_TRACK).into(),
        theme,
        false,
        NO_TRACK,
    )
}

/// Build the floating right-edge vertical toolbar from `tools`, vertically
/// centred over the canvas. Returns `None` when `tools` is empty so the caller
/// can skip pushing an overlay.
pub fn view(tools: &[ToolDef], theme: &Theme) -> Option<Element<'static, Message>> {
    view_with_active(tools, &|_| false, theme)
}

/// As [`view`], with the tools `active` names drawn as switched on.
pub fn view_with_active(
    tools: &[ToolDef],
    active: &dyn Fn(&str) -> bool,
    theme: &Theme,
) -> Option<Element<'static, Message>> {
    if tools.is_empty() {
        return None;
    }

    let mut col = column![].spacing(tokens::GAP_TIGHT).align_x(iced::Center);
    for t in tools {
        // An on tool wears the aurora wash + edge; a rest tool is a plain
        // framed plate that lifts under the pointer. Both are the kit's
        // numbers, decided in `kit::style_for`.
        let role = if active(t.id) { Role::Engaged } else { Role::Standard };
        let btn = icon_btn(
            icon_el(t.icon),
            role,
            BTN_SIZE,
            Press::On(Message::RibbonToolClick {
                tool_id: t.id.to_string(),
                event: t.event.clone(),
            }),
            NO_TRACK,
        );
        // Label tooltip on the left so it never runs off the right edge. The
        // tooltip widget paints nothing itself: its default class would plate
        // a solid box around the glass.
        col = col.push(
            tooltip(btn, tip_panel(t.label, theme), tooltip::Position::Left)
                .gap(tokens::GAP_TIGHT)
                .style(|theme: &Theme| Tier::Flush.style(theme, "")),
        );
    }

    // The plate the column of tools sits on. It floats over the drawing, so it
    // reads the frame behind it instead of plating an opaque box; the host keeps
    // the `PAD_PLATE` inset and wears no fill of its own.
    let panel = glass_pane(
        surface(col, Tier::Flush, PAD_PLATE, NO_TRACK).into(),
        theme,
        false,
        NO_TRACK,
    );

    // Fill the canvas, pin the panel to the right edge and centre it
    // vertically — no manual size/position math needed. This container is
    // geometry only: it wears no stylesheet.
    Some(
        container(iced::widget::opaque(panel))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::Right)
            .align_y(iced::Center)
            .padding(iced::Padding {
                right: EDGE_MARGIN,
                top: TOP_RESERVE,
                ..iced::Padding::ZERO
            })
            .into(),
    )
}
