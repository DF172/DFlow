//! Verdict feedback: a glyph and an ink, and nothing that moves.
//!
//! Every failure in the app is painted the same way — the tone's glyph at the
//! tone's solved ink, then the message in that same ink. There is deliberately
//! no shake, no slide, no colour pulse: motion is reserved for things the user
//! can act on, and a failure is something to read. The icon says which kind of
//! verdict this is without relying on colour alone, which is also what a
//! colour-blind user needs.

use iced::widget::{row, text};
use iced::{Element, Theme};

use super::label::{tone_ink, Tone};
use crate::ui::icons;
use crate::ui::style::tokens;

/// Glyph size of a feedback line: the label rung, so the icon never outgrows
/// the sentence it introduces.
pub const GLYPH: f32 = 14.0;

/// One verdict line: `tone`'s glyph, then `message` in `tone`'s ink.
pub fn line<'a, M: 'a>(tone: Tone, message: impl Into<String>) -> Element<'a, M> {
    row![
        icons::themed_by(tone.glyph(), GLYPH, move |theme| tone_ink(theme, tone)),
        text(message.into())
            .size(tokens::TEXT_LABEL)
            .style(move |theme: &Theme| super::tone_style(theme, tone)),
    ]
    .spacing(tokens::GAP_CONTROL)
    .align_y(iced::Center)
    .into()
}

#[cfg(test)]
mod b6_feedback_tests {
    use super::*;

    /// Every tone names a real asset, and no two share one: the glyph is the
    /// non-colour half of the verdict, so a wrong glyph is a wrong verdict.
    #[test]
    fn each_tone_has_its_own_glyph() {
        let tones = [Tone::Danger, Tone::Warning, Tone::Success, Tone::Accent];
        let mut seen: Vec<&'static [u8]> = Vec::new();
        for tone in tones {
            let glyph = tone.glyph();
            assert!(!glyph.is_empty(), "{tone:?} points at an empty glyph");
            assert!(!seen.contains(&glyph), "{tone:?} reuses another tone's glyph");
            seen.push(glyph);
        }
    }

    /// A failure is read, not performed: the module may paint ink and pick a
    /// glyph, but it must not reach for a motion track, an offset or a
    /// transform.
    #[test]
    fn feedback_never_moves() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/ui/kit/feedback.rs"
        ))
        .expect("feedback source must be readable");
        let paint = src.split("\n#[cfg(test)]").next().unwrap_or(&src);
        for forbidden in [
            "motion::",
            "::offset(",
            "translate",
            "pulse(",
            "tri_wave",
            "Vector::new",
        ] {
            assert!(
                !paint.contains(forbidden),
                "feedback paints `{forbidden}`; an error must sit still"
            );
        }
        assert!(
            paint.contains("tone.glyph()"),
            "the icon is the non-colour half of the verdict"
        );
        assert!(
            paint.contains("tone_style(theme, tone)"),
            "the message must take the solved verdict ink"
        );
    }

    /// The line is one row of glyph + text, so a caller cannot get a sentence
    /// laid out as two stacked blocks by accident.
    #[test]
    fn the_line_is_one_row() {
        let element: Element<'_, ()> = line(Tone::Danger, "boom");
        let size = element.as_widget().size();
        assert_eq!(size.width, iced::Length::Fit);
        assert_eq!(size.height, iced::Length::Fit);
    }
}
