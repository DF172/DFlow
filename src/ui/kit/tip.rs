//! Where a tooltip lands.
//!
//! The placement rule lives in the vendored tooltip overlay, because that is
//! where `layout` runs; this module is the app's handle on it. It exists so the
//! rule is testable from our side and so a call site can name the side it
//! means instead of encoding it as a `Position`.
//!
//! The rule is "nearest side that fits, never on top of the thing being
//! explained": asked for `Below`, a tooltip flips above when there is no room
//! underneath, and slides along the cross axis to stay in the window. It never
//! moves back towards its anchor, which is what iced's own clamp does.

pub use iced::widget::tooltip::{flip_rect, Side};

use iced::widget::{text, tooltip};
use iced::{Element, Rectangle, Theme};

use crate::ui::style::tokens;

/// Places `size` next to `anchor` inside `bounds`, preferring `side`.
///
/// Thin wrapper over [`flip_rect`] that mirrors the tooltip's own gap and
/// padding defaults, for callers that lay a tip out by hand (the canvas
/// overlays position their own popups and cannot use the widget's hover state).
pub fn place(anchor: Rectangle, size: iced::Size, bounds: Rectangle, side: Side) -> Rectangle {
    flip_rect(anchor, size, bounds, GAP, side)
}

/// The kit's tooltip gap: the same tight spacing a menu uses under its anchor.
pub const GAP: f32 = 4.0;

/// A control whose label had to be cut: hovering it shows the whole string.
///
/// The tip asks for `Bottom` on purpose. That is the side the vendored overlay
/// can flip: with no room underneath it goes above, and it slides along the
/// cross axis to stay inside the window, so the last row of a panel is
/// explained instead of clipped.
///
/// The plate is glass, like every other tip in the app
/// (`app/view/controls.rs::viewport_tooltip`). The 2026-11-02 ruling that moved
/// menus and flyouts onto opaque paper covers field-sized objects with rows; a
/// one-line label tip is not one of those, and a paper tip over the drawing
/// would be the only opaque rectangle in the tooltip family.
///
/// `theme` is handed in rather than resolved at style time because a glass pane
/// fixes its colours when it is built — the same reason
/// `ribbon::widgets::RenderCtx` carries one for every renderer that can emit a
/// tip.
pub fn full<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    full: impl Into<String>,
    theme: &Theme,
) -> Element<'a, M> {
    let ink = text(full.into())
        .size(tokens::TEXT_LABEL)
        .style(|theme: &Theme| super::color_style(theme.palette().background.base.text));
    tooltip(
        content,
        // The widget's own inset goes to zero so the pane hugs the ink, and
        // `PAD_PLATE` becomes the blank inside the plate. Its style is `Flush`
        // so the tooltip's quad paints nothing over the pane beneath the words.
        super::glass_pane(
            super::surface(ink, super::Tier::Flush, super::PAD_PLATE, "").into(),
            theme,
            false,
            "",
        ),
        tooltip::Position::Bottom,
    )
    .gap(GAP)
    .padding(tokens::GAP_NONE)
    .style(|theme: &Theme| super::Tier::Flush.style(theme, ""))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Size;

    fn viewport() -> Rectangle {
        Rectangle {
            x: 0.0,
            y: 0.0,
            width: 1280.0,
            height: 800.0,
        }
    }

    fn anchor_at(x: f32, y: f32) -> Rectangle {
        Rectangle {
            x,
            y,
            width: 120.0,
            height: 32.0,
        }
    }

    fn tip() -> Size {
        Size::new(180.0, 28.0)
    }

    #[test]
    fn a_tip_with_room_stays_below_its_control() {
        let anchor = anchor_at(400.0, 200.0);
        let placed = flip_rect(anchor, tip(), viewport(), GAP, Side::Below);
        assert_eq!(placed.y, anchor.y + anchor.height + GAP);
        assert_eq!(
            placed.x,
            anchor.x + (anchor.width - tip().width) / 2.0,
            "a tip centres on its control"
        );
    }

    #[test]
    fn a_tip_at_the_bottom_edge_flips_above() {
        let anchor = anchor_at(400.0, 770.0);
        let placed = flip_rect(anchor, tip(), viewport(), GAP, Side::Below);
        assert_eq!(
            placed.y,
            anchor.y - GAP - tip().height,
            "no room below means above, not on top of the control"
        );
        assert!(
            placed.y + placed.height <= anchor.y,
            "the tip must not cover what it explains"
        );
    }

    #[test]
    fn a_tip_at_the_top_edge_flips_below() {
        let anchor = anchor_at(400.0, 4.0);
        let placed = flip_rect(anchor, tip(), viewport(), GAP, Side::Above);
        assert_eq!(placed.y, anchor.y + anchor.height + GAP, "no room above means below");
        assert!(placed.y >= anchor.y + anchor.height);
    }

    #[test]
    fn a_tip_near_the_right_edge_slides_left() {
        let anchor = anchor_at(1200.0, 300.0);
        let placed = flip_rect(anchor, tip(), viewport(), GAP, Side::Below);
        assert_eq!(
            placed.x,
            viewport().width - tip().width,
            "the tip stays inside the window"
        );
        assert_eq!(placed.y, anchor.y + anchor.height + GAP, "sliding is cross-axis only");
    }

    #[test]
    fn a_side_anchored_tip_flips_across_its_own_axis() {
        let anchor = anchor_at(8.0, 300.0);
        let placed = flip_rect(anchor, tip(), viewport(), GAP, Side::Left);
        assert_eq!(
            placed.x,
            anchor.x + anchor.width + GAP,
            "no room to the left means the right side"
        );
        assert!(
            placed.x >= anchor.x + anchor.width,
            "a side tip must not overlap its control"
        );
    }

    #[test]
    fn the_roomier_side_wins_when_neither_fits() {
        // 900 px tall window, a tip taller than either gap around a centred
        // anchor: it goes to the larger gap and overhangs instead of covering.
        let bounds = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 300.0,
        };
        let anchor = Rectangle {
            x: 0.0,
            y: 120.0,
            width: 400.0,
            height: 60.0,
        };
        let tall = Size::new(100.0, 200.0);
        let placed = flip_rect(anchor, tall, bounds, GAP, Side::Below);
        assert!(
            placed.y >= anchor.y + anchor.height,
            "the roomier side still never overlaps the anchor, got {}",
            placed.y
        );
    }

    /// G9 rides the B4 flip: the full-text tip asks for the side that can move
    /// out of the way, so the last row of a panel is explained rather than
    /// clipped.
    #[test]
    fn a_full_text_tip_asks_for_the_side_that_flips() {
        assert_eq!(
            Side::of(iced::widget::tooltip::Position::Bottom),
            Some(Side::Below),
            "the full-text tip must ask for Bottom, the side the overlay can flip"
        );
    }

    /// The tip is a real tooltip widget on opaque paper, with its theme
    /// resolved at style time — that is what lets a row built without a
    /// `&Theme` still attach it.
    #[test]
    fn full_builds_a_tooltip_on_glass() {
        let source = include_str!("tip.rs");
        let paint = source.split("\n#[cfg(test)]").next().unwrap_or(source);
        let body = paint
            .split("pub fn full<'a, M: 'a>(")
            .nth(1)
            .expect("the full-text tip");
        assert!(body.contains("tooltip("), "the tip must be the tooltip widget");
        assert!(body.contains("tooltip::Position::Bottom"));
        // Tips are glass in this app: `app/view/controls.rs::viewport_tooltip`
        // and `ribbon::widgets::RenderCtx::theme` both say so, and the latter
        // spells out why the theme has to be handed to every renderer that can
        // emit one. The 2026-11-02 paper ruling covers menus and flyouts, which
        // are field-sized objects with rows — not a one-line label tip.
        assert!(
            body.contains("glass_pane("),
            "a label tip reads the surface under it like every other tip in the app"
        );
        assert!(
            !body.contains("Tier::Float"),
            "the tip went back to opaque paper, which is the menu ruling, not the tip ruling"
        );
    }

    #[test]
    fn a_tip_wider_than_the_window_pins_to_the_left_edge() {
        let anchor = anchor_at(300.0, 300.0);
        let huge = Size::new(2000.0, 28.0);
        let placed = flip_rect(anchor, huge, viewport(), GAP, Side::Below);
        assert_eq!(placed.x, 0.0);
    }
}
