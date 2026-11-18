//! Style helpers shared across the style windows.
//!
//! `muted_style` lived here as a two-line forwarder onto `ui::kit::muted_style`
//! until the 36th freeze deleted it and repointed the 25 importing files at the
//! kit directly. The numeric helpers below (WCAG, Oklab, dropdown width) stay.
//!
//! **`btn_s` is not deduped here, and no longer could be.** This note used to
//! record that the two locals in `plotstyle.rs` and `style_manager.rs` had
//! divergent bodies — `style_manager` handled `Status::Disabled` (text alpha
//! 0.45), `plotstyle` did not — and that unifying them would silently change the
//! plotstyle disabled appearance. Both locals are gone: `style_manager.rs`'s went
//! on screen 21, `plotstyle.rs`'s on screen 23, each drained onto `kit::btn`, which
//! takes role, size and press as arguments and dims through
//! `kit::control::Role`'s own disabled branch. `ui/window/layout_manager.rs`, the
//! third and last file that ever carried one, lost its copy in the 21-screen batch
//! this run merged, so no `btn_s` definition remains in the tree. So this file hosts
//! text and container helpers only: a button-plate forwarder here would be a
//! second L2 under the one L1 that already owns that decision. The deletion is
//! pinned by `the_plotstyle_button_forwarder_stays_deleted`.

use iced::Color;

/// Convert sRGB channel [0.0, 1.0] to linear light for WCAG luminance calculations.
pub(crate) fn to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Calculate standard WCAG 2.1 relative luminance for an opaque color.
pub(crate) fn wcag_luminance(color: Color) -> f32 {
    0.2126 * to_linear(color.r) + 0.7152 * to_linear(color.g) + 0.0722 * to_linear(color.b)
}

/// CIE L* of a colour (0 = black, 100 = white).
///
/// Why the surface ladder is measured in this unit rather than in a mix ratio:
/// `Color::mix` interpolates in *linear light* (`into_linear` / `from_linear`),
/// and linear luminance compresses brutally near black. A fixed 0.04 mix moves
/// a #10141B page 19.6 L* and the *former* light seed #ECEFF4 2.2 L* (the seed
/// is #D9DEE6 now, see `draftflow_theme::draftflow_light`), which is why one set
/// of hand-tuned mix ratios could never give the dark and light schemes the same
/// looking hierarchy — the exact failure `tokens::surface` replaced.
#[must_use]
pub(crate) fn lstar(color: Color) -> f32 {
    luminance_to_lstar(wcag_luminance(color))
}

/// The inverse of [`luminance_to_lstar`]: the linear luminance carrying `l`.
#[must_use]
pub(crate) fn lstar_to_luminance(l: f32) -> f32 {
    // CIE's linear segment below L* = 8 keeps the scale monotonic all the way
    // to black, which matters here: a near-black CAD page lives in that band.
    if l > 8.0 {
        ((l + 16.0) / 116.0).powi(3)
    } else {
        l / 903.296_3
    }
}

#[must_use]
pub(crate) fn luminance_to_lstar(y: f32) -> f32 {
    // 6/29 cubed is the CIE epsilon where the cube root hands over to the
    // linear segment; the matching K is (29/3)^3.
    const EPSILON: f32 = 0.008_856_452;
    if y > EPSILON {
        116.0 * y.cbrt() - 16.0
    } else {
        903.296_3 * y
    }
}

/// Calculate standard WCAG contrast ratio between two colors (range: 1.0 to 21.0).
pub(crate) fn wcag_contrast(c1: Color, c2: Color) -> f32 {
    let l1 = wcag_luminance(c1);
    let l2 = wcag_luminance(c2);
    (l1.max(l2) + 0.05) / (l1.min(l2) + 0.05)
}

/// The colour as Oklab: `[L 0..=1, a, b]`.
///
/// Not a nicer `lstar`: this is the coordinate system in which "move it to
/// luminance `y`" and "keep the colour" are *separable* statements. `L` is the
/// lightness and `(a, b)` is the colour, so a move that only touches `L` cannot
/// rotate the hue even by accident — which is the property
/// [`super::tokens::pull_to_luminance`] needed and did not have. CIE L\* and the
/// sRGB channel spread can only say roughly how bright and roughly how colourful
/// something is, and that vagueness is how the old pull could be documented as
/// preserving identity while the 2026-10-16 dump measured Kanagawa Lotus losing
/// nine tenths of its chroma on the way up.
///
/// Ottosson's published pair of matrices with the cube roots between; the
/// constants are his, carried to eight significant digits because the ninth
/// would be noise under an 8-bit channel.
#[must_use]
pub(crate) fn oklab_of(color: Color) -> [f32; 3] {
    let (r, g, b) = (
        to_linear(color.r),
        to_linear(color.g),
        to_linear(color.b),
    );
    // linear sRGB -> cone response -> the cube-rooted space the axes are
    // near-orthogonal in.
    let l = (0.4122_214_7 * r + 0.5363_325_4 * g + 0.0514_459_9 * b).cbrt();
    let m = (0.2119_035_0 * r + 0.6806_995_5 * g + 0.1073_969_6 * b).cbrt();
    let s = (0.0883_024_6 * r + 0.2817_188_4 * g + 0.6299_787_0 * b).cbrt();
    [
        0.2104_542_6 * l + 0.7936_177_8 * m - 0.0040_720_5 * s,
        1.9779_985_0 * l - 2.4285_922_1 * m + 0.4505_937_1 * s,
        0.0259_040_4 * l + 0.7827_717_7 * m - 0.8086_757_7 * s,
    ]
}

/// sRGB-encoded channels of an Oklab triple, *not* clamped: a channel outside
/// `0.0..=1.0` is a colour asking for something the display cannot show, and the
/// caller has to see that rather than a clamped lie.
#[must_use]
pub(crate) fn oklab_channels(lab: [f32; 3]) -> [f32; 3] {
    let [l, a, b] = lab;
    // The inverse has no guard for a negative cone response: `powi(3)` keeps the
    // sign, and a negative LMS is exactly the out-of-gamut signal above.
    let lt = l + 0.3963_377_8 * a + 0.2158_037_6 * b;
    let mt = l - 0.1055_613_5 * a - 0.0638_541_7 * b;
    let st = l - 0.0894_841_8 * a - 1.2914_855_5 * b;
    let (l_, m_, s_) = (lt.powi(3), mt.powi(3), st.powi(3));
    [
        from_linear(4.0767_416_6 * l_ - 3.3077_115_9 * m_ + 0.2309_699_3 * s_),
        from_linear(-1.2684_380_0 * l_ + 2.6097_574_0 * m_ - 0.3413_194_0 * s_),
        from_linear(-0.0041_960_9 * l_ - 0.7034_186_1 * m_ + 1.7076_147_0 * s_),
    ]
}

/// The same triple brought onto the display, which is what a pixel is.
#[must_use]
pub(crate) fn oklab_to_color(lab: [f32; 3]) -> Color {
    let [r, g, b] = oklab_channels(lab);
    Color::from_rgb(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
}

/// True when the display can show that triple as written.
#[cfg(test)]
#[must_use]
pub(crate) fn oklab_in_gamut(lab: [f32; 3]) -> bool {
    // `1e-4` of an encoded channel is 0.025 of an 8-bit step: enough to absorb
    // the two powf round trips, small enough that a real excursion still reads
    // false. Measured over a 41 x 41 x 5 sweep of in-gamut colours, the round
    // trip `oklab_to_color(oklab_of(c))` moved a channel by at most 1.9e-5, so
    // this band never swallows a colour the pixel already had.
    const EPS: f32 = 1e-4;
    oklab_channels(lab)
        .iter()
        .all(|v| (-(EPS)..(1.0 + EPS)).contains(v))
}

/// The colour as Oklch: `(L 0..=1, C, hue in degrees)`.
///
/// The polar reading of [`oklab_of`], and the form a report wants: `C` is how
/// colourful and `h` is *which* colour, which are the two things a screenshot can
/// be checked against. The pull itself works in the cartesian triple, where
/// holding the colour is arithmetic rather than a special case for a neutral.
///
/// Hue of a near-neutral colour is `atan2` of a near-zero vector, so it is
/// reported as `0.0` under [`OKLCH_HUE_IRRELEVANT`] — a guard comparing hues has
/// to compare chroma at *both* ends first or it will invent a drift the paint
/// cannot show. Measured on the probe sweep: with that gate the old pull's worst
/// readable hue move is 17.59° and the new one's is 0.13°; without it, a
/// gamut-forced neutral prints a 119° "drift" that no eye can find.
#[cfg(test)]
#[must_use]
pub(crate) fn oklch_of(color: Color) -> (f32, f32, f32) {
    let [light, a, b] = oklab_of(color);
    let chroma = (a * a + b * b).sqrt();
    let hue = if chroma < OKLCH_HUE_IRRELEVANT {
        0.0
    } else {
        b.atan2(a).to_degrees().rem_euclid(360.0)
    };
    (light, chroma, hue)
}

/// Below this `C` the hue angle is numerical noise around a neutral, and naming
/// it in a report would claim a colour the paint does not have. Measured: the
/// quietest band chroma across the 26 selectable themes is `0.0162` (Kanagawa
/// Lotus, after the old pull had washed it), so this floor sits under every real
/// colour the kit paints and only excludes true greys.
#[cfg(test)]
pub(crate) const OKLCH_HUE_IRRELEVANT: f32 = 0.004;

/// The inverse of [`to_linear`]: linear light back to the encoded value a pixel
/// carries. Negative input stays on the negative branch of the curve so an
/// out-of-gamut channel keeps its sign instead of turning into NaN.
#[must_use]
fn from_linear(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.abs().powf(1.0 / 2.4) - 0.055
    }
}

/// Returns `accent` if it achieves at least `min_contrast` against `bg`,
/// otherwise safely falls back to `fallback`.
pub(crate) fn accessible_accent_threshold(
    accent: Color,
    bg: Color,
    fallback: Color,
    min_contrast: f32,
) -> Color {
    if wcag_contrast(accent, bg) >= min_contrast {
        accent
    } else {
        fallback
    }
}

/// Returns `accent` if it achieves at least 3.0:1 contrast against `bg` (WCAG 1.4.11 for UI components),
/// otherwise safely falls back to `fallback`.
pub(crate) fn accessible_accent(accent: Color, bg: Color, fallback: Color) -> Color {
    accessible_accent_threshold(accent, bg, fallback, 3.0)
}

/// Returns true if the given canvas background color has a WCAG relative luminance > 0.5 (light surface).
/// Ignores alpha assuming canvas backgrounds are opaque (or composited against default viewport clear).
pub(crate) fn canvas_is_light(bg: [f32; 4]) -> bool {
    wcag_luminance(Color::from_rgb(bg[0], bg[1], bg[2])) > 0.5
}

/// True if a character is East Asian Wide or Fullwidth (CJK ideographs, kana, hangul, fullwidth forms).
/// Approximation of East Asian Wide/Fullwidth; not exhaustive (e.g. emoji ranges and CJK Extension B+
/// beyond 0x2FA1F omitted for the backlog).
pub(crate) fn is_wide(c: char) -> bool {
    let u = c as u32;
    matches!(
        u,
        0x1100..=0x115F   // Hangul Jamo
        | 0x2E80..=0xA4CF // CJK Radicals, Kangxi, Hiragana, Katakana, Bopomofo, Hangul Compatibility, CJK Unified Ideographs, Yi
        | 0xAC00..=0xD7A3 // Hangul Syllables
        | 0xF900..=0xFAFF // CJK Compatibility Ideographs
        | 0xFE30..=0xFE4F // CJK Compatibility Forms
        | 0xFF01..=0xFF60 // Fullwidth ASCII variants
        | 0xFFE0..=0xFFE6 // Fullwidth symbol variants
        | 0x20000..=0x2FA1F // CJK Unified Ideographs Extension B-F, Compatibility Supplement
    )
}

/// Estimated popup width for a vertical list of labels. Wide (CJK) glyphs
/// count double so localized labels via t!() don't truncate.
pub fn dropdown_popup_width<'a>(
    labels: impl Iterator<Item = &'a str>,
    font_size: f32,
    chrome: f32,
    min: f32,
) -> f32 {
    let max_units = labels
        .map(|l| {
            l.chars()
                .map(|c| if is_wide(c) { 2.0 } else { 1.0 })
                .sum::<f32>()
        })
        .fold(0.0f32, f32::max);
    (max_units * font_size * 0.68 + chrome).max(min)
}
