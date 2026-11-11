//! Modal overlay shown while a CAD file is being loaded.
//!
//! Displays the file name, size, current phase, measured progress, and a
//! Cancel button.

use crate::ui::kit;
use crate::ui::style::tokens;
use crate::t;
use iced::time::Instant;
use iced::widget::{column, container, row, stack, text, Space};
use crate::ui::kit::plain_button;
use iced::{Background, Element, Fill, Length, Theme};
use std::borrow::Cow;
use std::sync::atomic::Ordering;

use crate::app::{
    Message, OpenProgress, OPEN_PHASE_CACHING, OPEN_PHASE_FINALIZING, OPEN_PHASE_PARSING,
    OPEN_PHASE_READING, OPEN_PHASE_XREF,
};

const CARD_WIDTH: f32 = 420.0;
const BAR_TRACK_WIDTH: f32 = 380.0;
const BAR_TRACK_HEIGHT: f32 = 6.0;

fn phase_label(phase: u8) -> Cow<'static, str> {
    match phase {
        OPEN_PHASE_READING => t!("Reading file…"),
        OPEN_PHASE_PARSING => t!("Parsing entities…"),
        OPEN_PHASE_XREF => t!("Loading references…"),
        OPEN_PHASE_CACHING => t!("Building scene caches…"),
        OPEN_PHASE_FINALIZING => t!("Finalizing…"),
        _ => t!("Working…"),
    }
}

fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

/// (fill, trailing) in pixels for a bar filled to `shown`.
///
/// Total on any input: an out-of-range or non-finite fraction is pinned to
/// the track instead of producing a negative spacer or a NaN width, which
/// would panic the layout.
fn bar_segments(shown: f32) -> (f32, f32) {
    let shown = if shown.is_finite() {
        shown.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let fill = BAR_TRACK_WIDTH * shown;
    (fill, (BAR_TRACK_WIDTH - fill).max(0.0))
}

/// (offset, width) in pixels of the indeterminate band at sweep phase `t`.
///
/// The band travels its own width short of the track, so it starts flush at
/// the left and ends flush at the right and never paints outside the track.
fn sweep_segment(t: f32) -> (f32, f32) {
    let t = if t.is_finite() { t.clamp(0.0, 1.0) } else { 0.0 };
    let band = BAR_TRACK_WIDTH * 0.3;
    let travel = BAR_TRACK_WIDTH - band;
    (travel * t, band)
}

/// Which of the two progress shapes the counter asks for. A file whose size
/// is not known yet reports nothing: 0 basis points means "unmeasured", and
/// the card sweeps inside the same track rather than sitting at 0 %.
fn is_indeterminate(basis_points: u16) -> bool {
    basis_points == 0
}

pub fn view<'a>(progress: &'a OpenProgress, now: Instant) -> Element<'a, Message> {
    let phase = progress.state.phase.load(Ordering::Acquire);
    let basis_points = progress
        .state
        .basis_points
        .load(Ordering::Relaxed)
        .min(10000);
    let fraction = basis_points as f32 / 10000.0;
    // The bar follows the chase track, not the raw counter: the loading
    // phases report in bursts, and a bar that stepped with them would read as
    // a stutter. `value_or` hands the first frame the measured value so the
    // stamp does not have to undo a zero on its second tick.
    let shown =
        crate::ui::motion::value_or(crate::ui::motion::PROGRESS, fraction).clamp(0.0, 1.0);

    let fill_style =
        |theme: &Theme| kit::field_radius_plate_style(tokens::aurora_gradient(theme, 1.0));

    let bar_value: Element<'_, Message> = if is_indeterminate(basis_points) {
        // The sweep is motion, so the motion preference owns it: with motion
        // off the band rests at mid-track instead of running.
        let t = if crate::ui::motion::enabled() {
            crate::ui::motion::tri_wave(
                now.saturating_duration_since(progress.started).as_millis() as u64,
                crate::ui::motion::LOOP_MS,
            )
        } else {
            0.5
        };
        let (offset, width) = sweep_segment(t);
        let band: Element<'_, Message> = container(
            Space::new()
                .width(Length::Fixed(width))
                .height(Length::Fixed(BAR_TRACK_HEIGHT)),
        )
        .style(fill_style)
        .into();
        row![
            Space::new()
                .width(Length::Fixed(offset))
                .height(Length::Fixed(BAR_TRACK_HEIGHT)),
            band,
            Space::new()
                .width(Length::Fixed(BAR_TRACK_WIDTH - offset - width))
                .height(Length::Fixed(BAR_TRACK_HEIGHT)),
        ]
        .into()
    } else {
        let (fill_width, trailing) = bar_segments(shown);
        let bar_fill: Element<'_, Message> = container(
            Space::new()
                .width(Length::Fixed(fill_width))
                .height(Length::Fixed(BAR_TRACK_HEIGHT)),
        )
        .style(fill_style)
        .into();
        row![
            bar_fill,
            Space::new()
                .width(Length::Fixed(trailing))
                .height(Length::Fixed(BAR_TRACK_HEIGHT)),
        ]
        .into()
    };

    let bar_track: Element<'_, Message> = container(
        stack![
            container(
                Space::new()
                    .width(Length::Fixed(BAR_TRACK_WIDTH))
                    .height(Length::Fixed(BAR_TRACK_HEIGHT))
            )
            .style(|theme: &Theme| {
                kit::field_radius_plate_style(Background::Color(
                    theme.palette().background.strong.color,
                ))
            }),
            bar_value,
        ]
        .width(Length::Fixed(BAR_TRACK_WIDTH))
        .height(Length::Fixed(BAR_TRACK_HEIGHT)),
    )
    .into();

    // ── Card body ────────────────────────────────────────────────────────
    let title = text(t!("Opening file")).size(tokens::TEXT_SECTION);

    let name_line = text(format!(
        "{}  ({})",
        progress.name,
        format_size(progress.size_bytes)
    ))
    .size(tokens::TEXT_META)
    .style(|theme: &Theme| kit::color_style(theme.palette().background.base.text.scale_alpha(0.82)));

    let phase_line = text(format!(
        "{}  {:.1}%",
        phase_label(phase),
        basis_points as f32 / 100.0
    ))
    .size(tokens::TEXT_BODY)
    .style(|theme: &Theme| kit::color_style(theme.palette().primary.base.color));

    let cancel_btn: Element<'_, Message> = plain_button(text(t!("Cancel")).size(tokens::TEXT_BODY))
        .on_press(Message::OpenCancel)
        .style(kit::danger_style)
        .padding(kit::PAD_MENU_ROW)
        .into();

    let cancel_row: Element<'_, Message> = container(cancel_btn).align_right(Fill).into();

    let card = container(
        column![title, name_line, bar_track, phase_line, cancel_row]
            .spacing(tokens::GAP_FIELD)
            .width(Length::Fixed(CARD_WIDTH)),
    )
    .padding(kit::PAD_PROGRESS_CARD)
    .style(kit::card_plate_style);

    // ── Backdrop (click-blocker + dim) ────────────────────────────────────
    let backdrop: Element<'_, Message> = container(Space::new().width(Fill).height(Fill))
        .style(|theme: &Theme| {
            kit::plain_plate_style(Background::Color(
                theme.palette().background.strong.color.scale_alpha(0.72),
            ))
        })
        .width(Fill)
        .height(Fill)
        .into();

    let centered: Element<'_, Message> = container(card)
        .center_x(Fill)
        .center_y(Fill)
        .width(Fill)
        .height(Fill)
        .into();

    stack![backdrop, centered].width(Fill).height(Fill).into()
}

#[cfg(test)]
mod b6_progress_tests {
    use super::*;

    /// The paint side only: the assertions below spell the shapes they look
    /// for, so a scan that reached this module would always pass.
    fn paint_side(src: &str) -> &str {
        src.split("\n#[cfg(test)]").next().unwrap_or(src)
    }

    #[test]
    fn the_bar_fills_its_track_and_never_leaves_it() {
        assert_eq!(bar_segments(0.0), (0.0, BAR_TRACK_WIDTH));
        assert_eq!(bar_segments(1.0), (BAR_TRACK_WIDTH, 0.0));
        assert_eq!(
            bar_segments(0.5),
            (BAR_TRACK_WIDTH / 2.0, BAR_TRACK_WIDTH / 2.0)
        );
        for bad in [-1.0, 2.0, f32::NAN] {
            let (fill, trailing) = bar_segments(bad);
            assert!(
                fill >= 0.0 && trailing >= 0.0 && (fill + trailing - BAR_TRACK_WIDTH).abs() < 1e-3,
                "{bad} produced fill {fill} / trailing {trailing}"
            );
        }
    }

    #[test]
    fn the_sweep_band_always_sits_inside_the_track() {
        for step in 0..=100 {
            let (offset, width) = sweep_segment(step as f32 / 100.0);
            assert!(width > 0.0, "the band must keep a width");
            assert!(offset >= 0.0, "the band starts inside the track");
            assert!(
                offset + width <= BAR_TRACK_WIDTH + 1e-3,
                "step {step} paints to {} on a {} track",
                offset + width,
                BAR_TRACK_WIDTH
            );
        }
        assert_eq!(sweep_segment(-1.0).0, 0.0, "an out-of-range phase clamps");
        assert_eq!(sweep_segment(f32::NAN).0, 0.0, "a NaN phase clamps");
    }

    #[test]
    fn only_an_unmeasured_open_sweeps() {
        assert!(is_indeterminate(0));
        assert!(!is_indeterminate(1));
        assert!(!is_indeterminate(10000));
    }

    /// The card must read the chase track and the update side must own it;
    /// landing a frame subscription is the only thing that keeps the sweep
    /// alive, and only `update` is evaluated before `subscription()`.
    #[test]
    fn the_bar_follows_a_track_the_update_side_owns() {
        let card = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/ui/window/open_progress.rs"
        ))
        .expect("open_progress source must be readable");
        let card = paint_side(&card);
        assert!(
            card.contains("motion::value_or(crate::ui::motion::PROGRESS"),
            "the bar paints the raw counter again instead of the eased track"
        );
        assert!(
            card.contains("motion::tri_wave("),
            "the indeterminate shape is gone"
        );

        let update = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/app/update/viewport.rs"
        ))
        .expect("the viewport update module must be readable");
        let update = paint_side(&update);
        assert!(
            update.contains("motion::chase(crate::ui::motion::PROGRESS"),
            "no update-side chase owns the progress track"
        );
        assert!(
            update.contains("progress_seed"),
            "a new open must seed the track at zero instead of inheriting the last file's bar"
        );
    }
}
