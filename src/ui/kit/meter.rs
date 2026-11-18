//! The measure line: a recessed track with a filled share.
//!
//! One owner today — the update dialog's download bar. It earns a name because
//! the same geometry is about to be asked for again (any later transfer the
//! shell shows), and because the two plates it is made of are already kit
//! answers: [`field_radius_plate_style`] for both the filled share and the
//! track, exactly as the open-progress card's bar does at canvas level.
//!
//! Two decisions are spelled here and nowhere else:
//!   * The fill is the theme's signal colour ([`tokens::focus_ring`]), so the
//!     bar agrees with the primary button under it without the caller knowing
//!     which colour that is.
//!   * The share is quantised to 1/1000 of the track. `FillPortion` takes
//!     `u16`, so a finer split is not available; 1000 steps is a tenth of a
//!     percent, well under what a 6 px line can show.
//!
//! Not [`crate::ui::kit::empty`], which owns the *sweep* for work with no
//! known length. This one is for work that reports a fraction.

use iced::widget::{container, row, Space};
use iced::{Element, Fill, Length, Theme};

use super::surface::field_radius_plate_style;
use crate::ui::style::tokens;

/// A track of `height` px with `fraction` of it filled, clamped to 0..=1.
///
/// The unfilled share is left as bare space rather than painted: the track
/// plate already carries the recess, and a second plate over it would read as
/// a border at this thickness.
pub fn meter<Message: 'static>(fraction: f32, height: f32) -> Element<'static, Message> {
    let filled = if fraction.is_finite() {
        (fraction.clamp(0.0, 1.0) * 1000.0).round() as u16
    } else {
        0
    };
    let rest = 1000u16.saturating_sub(filled);

    let mut bar = row![].height(Fill);
    if filled > 0 {
        bar = bar.push(
            container(Space::new())
                .width(Length::FillPortion(filled))
                .height(Fill)
                .style(|theme: &Theme| {
                    field_radius_plate_style(tokens::focus_ring(theme).into())
                }),
        );
    }
    if rest > 0 {
        bar = bar.push(Space::new().width(Length::FillPortion(rest)).height(Fill));
    }

    container(bar)
        .width(Length::Fill)
        .height(Length::Fixed(height))
        .style(move |theme: &Theme| field_radius_plate_style(tokens::inset(theme).into()))
        .into()
}