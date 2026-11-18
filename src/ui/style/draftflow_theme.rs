//! DraftFlow CAD's own two color schemes: a dark professional workbench and a
//! light engineering sheet, switched with one control.
//!
//! Both are `iced::Theme::Custom`, so they are absent from `iced::Theme::ALL`.
//! Anything enumerating themes must go through [`crate::app::config::all_themes`],
//! and anything checking contrast must iterate [`draftflow_themes`] — the
//! centralized suite in `ui::theme_accessibility_tests` walks `Theme::ALL` and
//! would never see these.
//!
//! The palette is written out token by token from the design system
//! (`D:\tmp\ui-s36\round2-brand\DESIGN.md`, "单笔制图 / One Pen") instead of
//! being derived from a seed: iced's generator deviates every surface from the
//! page in OKLCH, which cannot hit the six named greys the sheet is drawn with.
//! [`Theme::custom_with_fn`] lets the exact pairs through, and `Pair` literals
//! (not `Pair::new`) keep iced from re-solving the text color, so the ink the
//! widgets draw is the ink the spec names.
//!
//! Two rules carry the whole scheme:
//!   * chrome is graphite and grey paper — no brand hue, no gradient, no shadow;
//!   * the only saturated signal is vermilion (`accent`), spent on the drawing
//!     selection, keyboard focus, and the current slider value. Nothing else.
//!
//! The split with the canvas is deliberate: dark chrome does not force a dark
//! drawing surface. DraftFlow Dark paints model space as a lighter charcoal than
//! its shell, and DraftFlow Light paints a near-white sheet.

use iced::theme::palette::{Background, Palette, Pair, Seed, Swatch};
use iced::{Color, Theme};

/// Display name of the dark scheme. Persisted verbatim in `settings.json` as
/// `theme.name`, so changing it orphans a user's choice.
pub const DRAFTFLOW_DARK: &str = "DraftFlow Dark";

/// Display name of the light scheme.
pub const DRAFTFLOW_LIGHT: &str = "DraftFlow Light";

/// Model-space canvas for [`DRAFTFLOW_DARK`] — a charcoal sheet held above the
/// chrome so the drawing area reads as paper laid on the bench.
pub const DRAFTFLOW_DARK_CANVAS: [u8; 3] = [0x23, 0x27, 0x2C];

/// Model-space canvas for [`DRAFTFLOW_LIGHT`] — paper white, a step above the
/// shell but quieter than a card.
pub const DRAFTFLOW_LIGHT_CANVAS: [u8; 3] = [0xF6, 0xF7, 0xF9];

/// Every named colour of one DraftFlow scheme, straight from the design sheet.
///
/// The struct exists so the kit stops re-deriving these from a seed: a surface
/// token is a decision in the sheet, not the output of a formula. Fields are
/// named after the sheet's token names so a call site can quote the spec.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colors {
    /// The window frame behind every panel.
    pub shell: Color,
    /// A full-width chrome strip: ribbon, dialog body.
    pub panel: Color,
    /// A group plate inside chrome: ribbon group, options navigation.
    pub panel2: Color,
    /// Model space.
    pub canvas: Color,
    /// A resting card, a field, a dialog control.
    pub surface: Color,
    /// A document surface: the start page's recent list, a file tile.
    pub file: Color,
    /// The selected row of a list.
    pub selected_row: Color,
    /// A row under the pointer.
    pub hover: Color,
    /// A pressed row or control.
    pub active: Color,
    /// Primary ink.
    pub ink: Color,
    /// The ink plate under the pointer: one step past `ink`, away from the
    /// page. A filled button has nowhere to go but deeper, because lightening
    /// it would make the label recede at the moment the control is being
    /// reached for.
    pub ink_hover: Color,
    /// Secondary ink: captions, paths.
    pub muted: Color,
    /// Quietest ink: metadata, placeholders.
    pub faint: Color,
    /// Ink for a disabled control.
    pub disabled: Color,
    /// Ink for a shortcut printed on a filled plate.
    pub on_ink_dim: Color,
    /// Hairline between chrome and content.
    pub line: Color,
    /// A field's edge, a dialog's frame.
    pub line_strong: Color,
    /// The only saturated colour in the interface.
    pub accent: Color,
    /// Ink on a saturated accent fill.
    pub on_accent: Color,
    /// Ink on an `ink`-filled plate — the primary button's label.
    pub on_ink: Color,
    /// A passing state, used as text and as a status glyph.
    pub positive: Color,
    /// A caution.
    pub warning: Color,
    /// A destructive action.
    pub danger: Color,
    /// The dot grid of model space.
    pub grid_dot: Color,
    /// The axis lines of model space.
    pub grid_axis: Color,
    /// The modal scrim.
    pub scrim: Color,
}

impl Colors {
    /// The scheme's shell, panel, and surface tokens in one tuple, in the order
    /// [`crate::ui::style::tokens::surface`] walks them.
    #[must_use]
    pub fn raised(self) -> [Color; 4] {
        [self.panel, self.panel2, self.surface, self.file]
    }
}

const LIGHT: Colors = Colors {
    shell: Color::from_rgb8(0xF0, 0xF1, 0xF3),
    panel: Color::from_rgb8(0xF5, 0xF6, 0xF8),
    panel2: Color::from_rgb8(0xF8, 0xF9, 0xFB),
    canvas: Color::from_rgb8(0xF6, 0xF7, 0xF9),
    surface: Color::from_rgb8(0xFF, 0xFF, 0xFF),
    file: Color::from_rgb8(0xFB, 0xFC, 0xFD),
    selected_row: Color::from_rgb8(0xFB, 0xEA, 0xE3),
    hover: Color::from_rgb8(0xED, 0xEF, 0xF2),
    active: Color::from_rgb8(0xD9, 0xDD, 0xE1),
    ink: Color::from_rgb8(0x19, 0x1C, 0x20),
    ink_hover: Color::from_rgb8(0x00, 0x03, 0x06),
    muted: Color::from_rgb8(0x59, 0x61, 0x6B),
    faint: Color::from_rgb8(0x5F, 0x67, 0x71),
    disabled: Color::from_rgb8(0x7C, 0x84, 0x8E),
    on_ink_dim: Color::from_rgb8(0xAE, 0xB4, 0xBB),
    line: Color::from_rgb8(0xD5, 0xD9, 0xDE),
    line_strong: Color::from_rgb8(0xC3, 0xC8, 0xCE),
    accent: Color::from_rgb8(0xB8, 0x43, 0x1F),
    on_accent: Color::from_rgb8(0xFB, 0xFB, 0xFC),
    on_ink: Color::from_rgb8(0xF1, 0xF2, 0xF4),
    positive: Color::from_rgb8(0x2C, 0x6E, 0x4F),
    warning: Color::from_rgb8(0x8A, 0x5A, 0x00),
    danger: Color::from_rgb8(0xA3, 0x22, 0x36),
    grid_dot: Color::from_rgb8(0xDD, 0xE1, 0xE6),
    grid_axis: Color::from_rgb8(0xE7, 0xEA, 0xEE),
    scrim: Color::from_rgba(139.0 / 255.0, 159.0 / 255.0, 191.0 / 255.0, 0.55),
};

const DARK: Colors = Colors {
    shell: Color::from_rgb8(0x14, 0x16, 0x19),
    panel: Color::from_rgb8(0x1A, 0x1D, 0x21),
    panel2: Color::from_rgb8(0x20, 0x24, 0x2A),
    canvas: Color::from_rgb8(0x23, 0x27, 0x2C),
    surface: Color::from_rgb8(0x26, 0x2A, 0x30),
    file: Color::from_rgb8(0x28, 0x2C, 0x32),
    selected_row: Color::from_rgb8(0x3A, 0x2A, 0x26),
    hover: Color::from_rgb8(0x26, 0x2B, 0x31),
    active: Color::from_rgb8(0x2E, 0x34, 0x3B),
    ink: Color::from_rgb8(0xE8, 0xEB, 0xEF),
    ink_hover: Color::from_rgb8(0xF7, 0xF9, 0xFB),
    muted: Color::from_rgb8(0xA7, 0xAE, 0xB7),
    faint: Color::from_rgb8(0x8C, 0x94, 0x9E),
    disabled: Color::from_rgb8(0x6F, 0x77, 0x81),
    on_ink_dim: Color::from_rgb8(0x5F, 0x68, 0x72),
    line: Color::from_rgb8(0x2A, 0x2F, 0x35),
    line_strong: Color::from_rgb8(0x3A, 0x40, 0x48),
    accent: Color::from_rgb8(0xF2, 0x6A, 0x45),
    on_accent: Color::from_rgb8(0x1A, 0x1D, 0x21),
    on_ink: Color::from_rgb8(0x15, 0x18, 0x1B),
    positive: Color::from_rgb8(0x59, 0xC0, 0x8C),
    warning: Color::from_rgb8(0xE0, 0xA9, 0x4A),
    danger: Color::from_rgb8(0xF0, 0x73, 0x7E),
    grid_dot: Color::from_rgb8(0x3A, 0x40, 0x48),
    grid_axis: Color::from_rgb8(0x32, 0x38, 0x3F),
    scrim: Color::from_rgba(55.0 / 255.0, 69.0 / 255.0, 95.0 / 255.0, 0.55),
};

/// The exact token set for `theme`, or `None` for any other theme.
#[must_use]
pub fn draftflow_colors(theme: &Theme) -> Option<Colors> {
    match theme.to_string().as_str() {
        DRAFTFLOW_DARK => Some(DARK),
        DRAFTFLOW_LIGHT => Some(LIGHT),
        _ => None,
    }
}

/// The color for a custom theme is whatever the sheet says, not what the
/// generator would derive from a seed. Pairs are written as literals so
/// `readable()` cannot nudge the ink away from the named token.
fn palette_of(colors: Colors, is_dark: bool) -> Palette {
    let pair = |color: Color, text: Color| Pair { color, text };
    Palette {
        background: Background {
            base: pair(colors.shell, colors.ink),
            weakest: pair(colors.panel2, colors.ink),
            weaker: pair(colors.panel2, colors.ink),
            weak: pair(colors.hover, colors.ink),
            neutral: pair(colors.selected_row, colors.ink),
            strong: pair(colors.active, colors.ink),
            stronger: pair(colors.line, colors.ink),
            strongest: pair(colors.surface, colors.ink),
        },
        primary: Swatch {
            base: pair(colors.accent, colors.on_accent),
            weak: pair(colors.selected_row, colors.ink),
            strong: pair(colors.active, colors.ink),
        },
        secondary: Swatch {
            base: pair(colors.muted, colors.shell),
            weak: pair(colors.selected_row, colors.ink),
            strong: pair(colors.active, colors.ink),
        },
        success: Swatch {
            base: pair(colors.positive, colors.on_accent),
            weak: pair(colors.selected_row, colors.positive),
            strong: pair(colors.active, colors.positive),
        },
        warning: Swatch {
            base: pair(colors.warning, colors.on_accent),
            weak: pair(colors.selected_row, colors.warning),
            strong: pair(colors.active, colors.warning),
        },
        danger: Swatch {
            base: pair(colors.danger, colors.on_accent),
            weak: pair(colors.selected_row, colors.danger),
            strong: pair(colors.active, colors.danger),
        },
        is_dark,
    }
}

/// The seed is carried only because the vendor API asks for one; [`palette_of`]
/// replaces every field it would have generated.
fn seed(colors: Colors) -> Seed {
    Seed {
        background: colors.shell,
        text: colors.ink,
        primary: colors.accent,
        success: colors.positive,
        warning: colors.warning,
        danger: colors.danger,
    }
}

/// Graphite workbench chrome around a charcoal sheet.
#[must_use]
pub fn draftflow_dark() -> Theme {
    Theme::custom_with_fn(DRAFTFLOW_DARK, seed(DARK), |_| palette_of(DARK, true))
}

/// Grey-paper engineering desk: graphite chrome, white cards, a paper sheet.
#[must_use]
pub fn draftflow_light() -> Theme {
    Theme::custom_with_fn(DRAFTFLOW_LIGHT, seed(LIGHT), |_| palette_of(LIGHT, false))
}

/// Both DraftFlow schemes, dark first because it is the default.
#[must_use]
pub fn draftflow_themes() -> Vec<Theme> {
    vec![draftflow_dark(), draftflow_light()]
}

/// The scheme on the other side of the light/dark split from `current`.
///
/// Toggling is defined against *luminance*, not against this pair's names: a
/// user who picked Nord and then hits the toggle gets DraftFlow Dark or Light,
/// which is the behaviour the control advertises.
#[must_use]
pub fn toggle_theme(current: &Theme) -> Theme {
    if current.palette().is_dark {
        draftflow_light()
    } else {
        draftflow_dark()
    }
}

/// The canvas colour a DraftFlow theme asks for, or `None` for any other theme.
/// Keyed by name because `Theme::Custom` carries no discriminant to match on.
#[must_use]
pub fn draftflow_canvas(theme: &Theme) -> Option<[u8; 3]> {
    draftflow_colors(theme).map(|c| {
        let [r, g, b, _] = c.canvas.into_rgba8();
        [r, g, b]
    })
}

/// Selection marquee colours as `(crossing, window)` for a DraftFlow scheme.
/// `None` for every other theme, which keeps their curated CAD pairs.
///
/// Both marquees are vermilion: the pencil-line accent is the only signal the
/// scheme spends on "this is what you picked", and a second hue would break it.
/// The two rectangles stay tellable apart by line pattern, as they always were.
#[must_use]
pub fn draftflow_selection_colors(theme: &Theme) -> Option<(Color, Color)> {
    draftflow_colors(theme).map(|c| (c.accent, c.accent))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::style::common::wcag_contrast;

    /// The same bar `theme_accessibility_tests` holds the built-in themes to.
    /// These themes are hand-written rather than generated, so nothing else
    /// would catch a token typo that drops a surface below AA.
    #[test]
    fn draftflow_themes_meet_wcag_aa_on_every_surface() {
        for theme in draftflow_themes() {
            let p = theme.palette();
            assert!(p.is_dark == (theme.to_string() == DRAFTFLOW_DARK));
            for (name, pair) in [
                ("base", p.background.base),
                ("weakest", p.background.weakest),
                ("weak", p.background.weak),
                ("neutral", p.background.neutral),
                ("strong", p.background.strong),
                ("strongest", p.background.strongest),
            ] {
                let contrast = wcag_contrast(pair.text, pair.color);
                assert!(
                    contrast >= 4.5,
                    "{theme} {name} surface text is {contrast:.2}:1, below AA"
                );
            }
        }
    }

    /// The palette must be the sheet, not a seed derivation: every background
    /// slot the kit reads is one of the named tokens, unchanged.
    #[test]
    fn the_palette_carries_the_named_tokens_exactly() {
        for (theme, colors) in [
            (draftflow_dark(), DARK),
            (draftflow_light(), LIGHT),
        ] {
            let p = theme.palette();
            assert_eq!(p.background.base.color, colors.shell);
            assert_eq!(p.background.weakest.color, colors.panel2);
            assert_eq!(p.background.weak.color, colors.hover);
            assert_eq!(p.background.neutral.color, colors.selected_row);
            assert_eq!(p.background.strong.color, colors.active);
            assert_eq!(p.background.stronger.color, colors.line);
            assert_eq!(p.background.strongest.color, colors.surface);
            assert_eq!(p.background.base.text, colors.ink);
            assert_eq!(p.primary.base.color, colors.accent);
            assert_eq!(p.primary.base.text, colors.on_accent);
        }
    }

    /// The accent is what shows a selection and the slider's current value, so
    /// it has to survive being dropped straight onto the drawing surface.
    #[test]
    fn draftflow_accent_is_visible_on_the_canvas() {
        for (theme, canvas) in [
            (draftflow_dark(), DRAFTFLOW_DARK_CANVAS),
            (draftflow_light(), DRAFTFLOW_LIGHT_CANVAS),
        ] {
            let canvas = Color::from_rgb8(canvas[0], canvas[1], canvas[2]);
            let contrast = wcag_contrast(theme.palette().primary.base.color, canvas);
            assert!(
                contrast >= 3.0,
                "{theme} accent is {contrast:.2}:1 on its canvas, too faint \
                 to show a selection"
            );
        }
    }

    /// A selection marquee is the only thing saying what a window encloses, so
    /// it has to clear the canvas it is drawn on — in both schemes.
    #[test]
    fn draftflow_marquee_reads_on_the_canvas() {
        for (theme, canvas, colors) in [
            (draftflow_dark(), DRAFTFLOW_DARK_CANVAS, DARK),
            (draftflow_light(), DRAFTFLOW_LIGHT_CANVAS, LIGHT),
        ] {
            let (crossing, window) = draftflow_selection_colors(&theme)
                .expect("a DraftFlow scheme should carry a curated marquee pair");
            assert_eq!((crossing, window), (colors.accent, colors.accent));
            let canvas = Color::from_rgb8(canvas[0], canvas[1], canvas[2]);
            for (name, color) in [("crossing", crossing), ("window", window)] {
                let contrast = wcag_contrast(color, canvas);
                assert!(
                    contrast >= 3.0,
                    "{theme} {name} marquee is {contrast:.2}:1 on its canvas"
                );
            }
        }
    }

    /// Semantic colours are used as raw text in the command line and as status
    /// hints, bypassing the `readable` guard. Errors have to stay legible.
    #[test]
    fn draftflow_semantic_colors_read_as_text() {
        for theme in draftflow_themes() {
            let p = theme.palette();
            for (name, color) in [
                ("success", p.success.base.color),
                ("warning", p.warning.base.color),
                ("danger", p.danger.base.color),
            ] {
                let contrast = wcag_contrast(color, p.background.base.color);
                assert!(
                    contrast >= 3.0,
                    "{theme} {name} is {contrast:.2}:1 on the base surface, \
                     too low to use as text"
                );
            }
        }
    }

    /// The canvas must stay distinguishable from the chrome around it, or the
    /// drawing area loses its edge when the theme changes.
    #[test]
    fn draftflow_canvas_reads_against_chrome() {
        for (theme, canvas) in [
            (draftflow_dark(), DRAFTFLOW_DARK_CANVAS),
            (draftflow_light(), DRAFTFLOW_LIGHT_CANVAS),
        ] {
            let chrome = theme.palette().background.base.color;
            let canvas = Color::from_rgb8(canvas[0], canvas[1], canvas[2]);
            let delta = (chrome.relative_luminance() - canvas.relative_luminance()).abs();
            assert!(
                delta >= 0.01,
                "{theme} canvas is only {delta:.4} luminance from the chrome \
                 — the drawing area would have no edge"
            );
        }
    }

    #[test]
    fn toggle_flips_between_the_two_schemes() {
        assert_eq!(
            toggle_theme(&draftflow_dark()).to_string(),
            DRAFTFLOW_LIGHT
        );
        assert_eq!(
            toggle_theme(&draftflow_light()).to_string(),
            DRAFTFLOW_DARK
        );
        // A third-party dark theme still lands on the light scheme.
        assert_eq!(
            toggle_theme(&Theme::Oxocarbon).to_string(),
            DRAFTFLOW_LIGHT
        );
        assert_eq!(toggle_theme(&Theme::Light).to_string(), DRAFTFLOW_DARK);
    }

    #[test]
    fn draftflow_canvas_is_keyed_by_name() {
        assert_eq!(
            draftflow_canvas(&draftflow_dark()),
            Some(DRAFTFLOW_DARK_CANVAS)
        );
        assert_eq!(
            draftflow_canvas(&draftflow_light()),
            Some(DRAFTFLOW_LIGHT_CANVAS)
        );
        assert_eq!(draftflow_canvas(&Theme::Dark), None);
    }
}
