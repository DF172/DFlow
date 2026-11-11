//! Empty states: the list's slot, held open.
//!
//! Two shapes, one rule. [`state`] fills the box its list would have filled, so
//! a panel never resizes when the last row leaves and the empty state never
//! floats above a shrunken table; [`line`] is the one-row form a popup uses,
//! which keeps the popup's width and its row height rather than collapsing to
//! nothing.
//!
//! Nothing here moves. An empty list is still — a glyph and one line of faint
//! ink — so it can never be read as a list that is still loading. That matters
//! because loading is the one state in the app that sweeps: the open-progress
//! shimmer and the indeterminate segment are the only moving bars, and an empty
//! state that animated would borrow their vocabulary.
//!
//! The fade is 120 ms through the caller's key. The track is started on the
//! update side ([`crate::ui::motion::rise`]) because the frame subscription is
//! collected at the end of `update`, before `view`: a track started while
//! painting would never receive a frame. Reading through `engage` keeps the
//! first paint correct — no track yet means fully drawn, not invisible.

use iced::widget::{column, container, row, text};
use iced::{Element, Fill};

use super::{faint_style, Size};
use crate::ui::motion;
use crate::ui::style::tokens;

/// Glyph edge for the slot-filling form, matched to the start page's.
pub const GLYPH: f32 = 22.0;
/// Glyph edge for the one-row popup form.
pub const LINE_GLYPH: f32 = 14.0;

/// An empty state for a region that would otherwise hold a list.
///
/// `key` is the surface's motion track, one per surface rather than one per
/// message: switching a filter narrows and widens the same state, and the
/// element should not restart its fade every time it is rebuilt.
pub fn state<'a, M: 'a>(
    key: &'static str,
    glyph: Element<'a, M>,
    label: impl Into<String>,
) -> Element<'a, M> {
    let alpha = motion::engage(key, true);
    super::alpha::entrance(alpha, move || {
        container(
            column![glyph, text(label.into()).size(tokens::TEXT_LABEL).style(faint_style)]
                .spacing(tokens::GAP_ROW)
                .align_x(iced::Alignment::Center),
        )
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill)
        .into()
    })
}

/// The one-row form: a suggestion popup with no suggestion to offer.
///
/// It keeps the popup's width so the card under the command field does not
/// narrow to a stamp, and it keeps one row's height so the field above does not
/// jump when the list appears or disappears.
pub fn line<'a, M: 'a>(
    key: &'static str,
    glyph: Element<'a, M>,
    label: impl Into<String>,
) -> Element<'a, M> {
    let alpha = motion::engage(key, true);
    super::alpha::entrance(alpha, move || {
        container(
            row![glyph, text(label.into()).size(tokens::TEXT_LABEL).style(faint_style)]
                .spacing(tokens::GAP_CONTROL)
                .align_y(iced::Center),
        )
        .width(Fill)
        .padding(Size::Compact.padding())
        .into()
    })
}

#[cfg(test)]
mod b6_empty_tests {
    use super::*;
    use iced::widget::Space;

    fn full(key: &'static str) -> Element<'static, ()> {
        state(key, Space::new().into(), "Nothing here")
    }

    fn compact(key: &'static str) -> Element<'static, ()> {
        line(key, Space::new().into(), "No matches")
    }

    /// The iron law the batch was approved on: an empty state occupies the slot
    /// its list would have occupied, so the chrome around it never moves.
    #[test]
    fn the_full_state_fills_the_slot_it_replaces() {
        assert_eq!(
            full("b6:empty:slot").as_widget().size(),
            iced::Size::new(Fill, Fill),

            "the empty state must claim the list's box, not shrink-wrap its text"
        );
    }

    /// The popup form is the other half of the same law: keep the width, keep
    /// exactly one row of height, collapse to neither.
    #[test]
    fn the_popup_state_keeps_its_width_and_one_row() {
        assert_eq!(
            compact("b6:empty:line").as_widget().size(),
            // iced's `Fit` is the intrinsic height: exactly one row. `Shrink`
            // would let the row compress, which is the opposite of the promise.
            iced::Size::new(Fill, iced::Length::Fit),
            "a no-match popup must not narrow to its label"
        );
    }

    /// Static by construction. If this file ever grows a wave, the empty state
    /// starts borrowing the loader's vocabulary and the two become one state.
    #[test]
    fn nothing_in_an_empty_state_sweeps() {
        let file = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/ui/kit/empty.rs"
        ))
        .expect("the empty-state source must be readable");
        let paint = file.split("\n#[cfg(test)]").next().unwrap_or(&file);
        for loader in ["tri_wave", "LOOP_MS", "pulse(", "replay("] {
            assert!(
                !paint.contains(loader),
                "an empty state must not use `{loader}`: motion is what loading looks like"
            );
        }
        assert!(
            paint.contains("motion::engage(key"),
            "the empty state's fade must read its own key, or it arrives at full strength"
        );
    }

    /// Both forms hand their whole body to `alpha::entrance`, including the
    /// glyph: the icon helper bakes the scope into the svg's own opacity, so a
    /// glyph built outside the closure would stay full strength over faded text.
    #[test]
    fn both_forms_build_their_glyph_inside_the_fade() {
        let file = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/ui/kit/empty.rs"
        ))
        .expect("the empty-state source must be readable");
        let paint = file.split("\n#[cfg(test)]").next().unwrap_or(&file);
        assert_eq!(
            paint.matches("alpha::entrance(alpha").count(),
            2,
            "there are two forms, so there are two entrances"
        );
        let first_glyph_use = paint.find("glyph,").expect("the glyph is placed in a layout");
        assert!(
            first_glyph_use > paint.find("alpha::entrance(alpha").unwrap(),
            "the first glyph use must be inside the entrance closure"
        );
        assert!(
            LINE_GLYPH < GLYPH,
            "the popup's mark must be the smaller of the two"
        );
    }
}
