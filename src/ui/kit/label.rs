//! Type: the voices a screen is allowed to use.
//!
//! Sizes come from [`tokens`], never from a literal at a call site — the audit
//! found `.size(14)` written 96 times and `.size(15)` 70 times while
//! `TEXT_HEADING`/`TEXT_SECTION` had a combined one call site, which is how two
//! dialogs end up a pixel apart. Colour is always a palette role, never a hue
//! written in code, so the dark and light schemes cannot drift — and no role is
//! painted raw: the verdict words are solved against the planes they land on
//! ([`tone_style`]) and the prose voices are solved too ([`voice_ink`]), because
//! a swatch chosen to fill a chip and an alpha chosen to rank two lines are
//! neither one a guarantee that the resulting pixel can be read. The entrance
//! fade is applied by the builders *after* that solve, so what is guaranteed is
//! the resting pixel.
//!
//! `muted` is deliberately the same 0.68 ink the earlier unification pass had
//! collected seven local copies into. It is not another copy: the old
//! `style::common::muted_style` forwarder was deleted in the 36th freeze, and the
//! call sites now name this function directly.

use super::surface::{fade, plane, surface, Tier, PAD_WELL};
use crate::ui::style::common::{wcag_contrast, wcag_luminance};
use crate::ui::style::tokens;
use iced::widget::text;
use iced::{Color, Element, Theme};
use std::borrow::Cow;

/// A screen or panel name: the top of the ladder. Semibold, because the
/// ladder's 15/14/12 rungs are one pixel apart and weight is the signal that
/// tells a title from the rows under it without spending another size.
pub fn heading<'a, M: 'a>(label: impl Into<String>, track: &'static str) -> Element<'a, M> {
    ink_weighted(
        label,
        tokens::TEXT_SECTION,
        1.0,
        tokens::UI_FONT_SEMIBOLD,
        track,
    )
}

/// A group name inside a panel — "Snap", "Units", "Recent documents".
pub fn group<'a, M: 'a>(label: impl Into<String>, track: &'static str) -> Element<'a, M> {
    ink_weighted(
        label,
        tokens::TEXT_HEADING,
        GROUP_INK,
        tokens::UI_FONT_MEDIUM,
        track,
    )
}

/// Muted section heading, the name `style::form` gave to a group title inside a
/// dialog. It is [`group`] under another name — 11px at the muted ink in the
/// old file, the group rung here — so a dialog that only needs to title a
/// section does not have to choose a rung.
pub fn section_label<'a, M: 'a>(label: Cow<'static, str>) -> Element<'a, M> {
    group(label, "")
}

/// Running text: a value, a path, a tooltip body.
pub fn body<'a, M: 'a>(label: impl Into<String>, track: &'static str) -> Element<'a, M> {
    ink(label, tokens::TEXT_BODY, 1.0, track)
}

/// Secondary text — a caption under a title, a directory under a file name.
/// [`tokens::INK_MUTED`] of the ink role: below `body` without changing colour
/// family, and the same ratio every existing muted label in the app already
/// used.
pub fn muted<'a, M: 'a>(label: impl Into<String>, track: &'static str) -> Element<'a, M> {
    ink(label, tokens::TEXT_BODY, tokens::INK_MUTED, track)
}

/// The line under a heading: a tagline, a subtitle. [`tokens::TEXT_HEADING`] at
/// the muted ink — one size above [`body`], quieter than [`heading`].
pub fn caption<'a, M: 'a>(label: impl Into<String>, track: &'static str) -> Element<'a, M> {
    ink(label, tokens::TEXT_HEADING, tokens::INK_MUTED, track)
}

/// The quietest voice in the app: a version string, a timestamp, a count that
/// should not compete with the row it belongs to.
pub fn faint<'a, M: 'a>(label: impl Into<String>, track: &'static str) -> Element<'a, M> {
    ink(label, tokens::TEXT_BODY, tokens::INK_FAINT, track)
}

/// The style form of [`muted`], for the call sites that hand iced a
/// `fn(&Theme) -> text::Style` rather than a built widget.
///
/// 207 lines of the app paint through this one function (the former
/// `style::common::muted_style` forwarder was deleted in the 36th freeze), so its
/// ink is the quietest voice with the largest surface — and it used to be
/// `background.base.text × INK_MUTED` unsolved. Measured 2026-10-16 across the 26
/// selectable themes and the 22 kit planes, that pixel reached the 4.5:1 text bar
/// on two themes only; the rest sat between 2.18:1 (Tokyo Night) and 4.4:1. See
/// `ui::theme_accessibility_tests::the_quiet_voice_of_a_row_still_reads`.
pub fn muted_style(theme: &Theme) -> text::Style {
    let paint = super::alpha::Paint::start();
    paint.finish(muted_style_solve(theme))
}

fn muted_style_solve(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(voice_ink(theme, tokens::INK_MUTED)),
    }
}

/// The style form of [`muted`], one notch quieter, for placeholder-grade hint
/// text that has to read as *not* the value sitting in the same row.
///
/// The notch below was worse than the notch above, and by a lot: at
/// [`tokens::INK_FAINT`] unsolved, this reached 4.5:1 on **none** of the 26
/// themes, worst 1.75:1 (Tokyo Night), best 3.32:1 (DraftFlow Light). It is
/// solved like `muted_style`; where the ladder cannot carry 0.48 the two rungs
/// converge onto the same readable pixel, which is the #69 law again and is
/// stated rather than hidden.
#[must_use]
pub fn faint_style(theme: &Theme) -> text::Style {
    let paint = super::alpha::Paint::start();
    paint.finish(faint_style_solve(theme))
}

fn faint_style_solve(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(voice_ink(theme, tokens::INK_FAINT)),
    }
}

/// The shortcut chord printed beside a command: dim ink on a filled plate, the
/// faint voice on a plain one.
#[must_use]
pub fn chord_ink_style(theme: &Theme, on_filled: bool) -> text::Style {
    text::Style {
        color: Some(if on_filled {
            crate::ui::style::draftflow_theme::draftflow_colors(theme)
                .map(|colors| colors.on_ink_dim)
                .unwrap_or_else(|| theme.palette().primary.base.text.scale_alpha(0.8))
        } else {
            faint_style(theme)
                .color
                .unwrap_or_else(|| theme.palette().background.base.text)
        }),
    }
}

/// The resting ink of a prose rung: `alpha` of the palette's text role, pulled
/// until it reads on every plane a word can land on.
///
/// This is the one path every voice in this module takes — [`ink`] paints with
/// it, `muted_style` and `faint_style` hand it to iced — so the guard measures
/// the ladder here and cannot drift away from what the widgets draw. The solve
/// is at the [`tokens::AA_INK_CONTRAST`] bar for every rung, including the
/// quietest: 1.4.3's lower bar is for *large* text, and these are `TEXT_BODY`
/// and `TEXT_HEADING` sizes, so a secondary word is still a word.
///
/// The entrance fade is deliberately applied by the caller *after* this, not
/// inside it: a panel dissolving in is supposed to be unreadable for 240 ms, and
/// multiplying the guarantee by a transient alpha would make the solved ink
/// translucent again — which is exactly the class of bug #69 and #74 were about.
/// What is guaranteed is the resting pixel, `fade(track) == 1.0`.
#[must_use]
pub fn voice_ink(theme: &Theme, alpha: f32) -> Color {
    resolve_on(
        theme.palette().background.base.text.scale_alpha(alpha),
        &kit_planes(theme),
        tokens::AA_INK_CONTRAST,
    )
}

/// A word carrying a verdict instead of prose: a failed count, a warning, the
/// accent on the field the panel is pointing at. These were raw `Color::from_rgb`
/// at each call site, which is how one dialog's "required" was a different red
/// than the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Accent,
    Success,
    Warning,
    Danger,
}

/// The style form of [`Tone`]: a verdict word painted with an ink that clears AA
/// on **every plane the kit can put under it** ([`kit_planes`]).
///
/// This used to hand back `tone_pair(theme, tone).color`, which is the colour a
/// *chip* is filled with. As a word it is a different job and a different
/// measurement: 333 of 728 tone × plane pairs in 26 selectable themes landed
/// under 4.5:1 (`the_verdict_word_reads_on_the_plane_it_is_given`), the worst
/// being iced's own `Dark` success green at 1.09:1 on the Float tier — a verdict
/// word that was, on that theme, invisible. No palette rung fixes it from the
/// other side: `weak` mixes toward the background, and `strong` is iced's
/// `deviate(base, 0.1)`, a 10% nudge, so a theme's swatch set cannot be searched
/// into a legible word. The ink has to be solved against the planes.
#[must_use]
pub fn tone_style(theme: &Theme, tone: Tone) -> text::Style {
    let paint = super::alpha::Paint::start();
    paint.finish(text::Style {
        color: Some(tone_ink(theme, tone)),
    })
}

/// [`tone_style`] for a word written on a plane the ladder does not own — the
/// document canvas, whose colour comes from the drawing rather than the theme,
/// so nothing in [`crate::ui::style::tokens`] can have solved for it in advance.
#[must_use]
pub fn tone_style_on(theme: &Theme, tone: Tone, plate: Color) -> text::Style {
    let paint = super::alpha::Paint::start();
    paint.finish(text::Style {
        color: Some(tone_ink_on(theme, tone, plate)),
    })
}

/// The tone's ink solved against one named plane.
///
/// [`crate::ui::kit::surface::tone_plate_style_on`] uses this for a band that
/// paints its own ground: the plate is known, so the solve has one entry and the
/// verdict cannot come out a different colour depending on which tier the band was
/// dropped into.
#[must_use]
pub fn tone_ink_on(theme: &Theme, tone: Tone, plate: Color) -> Color {
    resolve_on(tone_pair(theme, tone).color, &[plate], tokens::AA_INK_CONTRAST)
}

impl Tone {
    /// The glyph that speaks this tone without words.
    ///
    /// A verdict is not allowed to rely on colour alone: red and green are
    /// the pair a colour-blind reader loses first. Danger is the cross the
    /// dialogs already use to dismiss, warning the triangle, success the
    /// check, and the accent the info mark it already paints.
    pub fn glyph(self) -> &'static [u8] {
        match self {
            Tone::Danger => crate::ui::icons::CLOSE,
            Tone::Warning => crate::ui::icons::WARNING,
            Tone::Success => crate::ui::icons::CHECK,
            Tone::Accent => crate::ui::icons::INFO,
        }
    }
}

/// The tone's own ink: [`tone_pair`]'s fill colour, pulled until it reads on
/// every plane a kit word can land on.
#[must_use]
pub fn tone_ink(theme: &Theme, tone: Tone) -> Color {
    resolve_on(tone_pair(theme, tone).color, &kit_planes(theme), tokens::AA_INK_CONTRAST)
}

/// The accent as a word: [`tone_style`] with the one [`Tone`] that is not a
/// verdict but a pointer — the section heading the panel is emphasising, the
/// active style's name in a title line.
///
/// Named because seven screens hand-rolled the same two lines under two names:
/// `fn primary_style(theme: &Theme) -> iced::widget::text::Style { color: Some(
/// theme.palette().primary.base.color) }` in `ui/style/dimstyle.rs`, `mlstyle.rs`,
/// `tablestyle.rs`, `textstyle.rs`, `ui/window/about.rs` and `layout_manager.rs`,
/// plus `primary_text_style` in `ui/style/style_manager.rs` — **27 call sites**
/// between them, while **22** other sites already wrote the solved form
/// `tone_style(theme, Tone::Accent)` by hand. So this is not a new behaviour: it
/// is the minority spelling being folded onto the majority one.
///
/// The pixel does move, and only in the direction the guard already audits.
/// `palette().primary.base.color` is iced's raw accent swatch, chosen to be
/// recognizable as the brand and never measured as a word; `tone_ink` pulls it
/// until it clears [`tokens::AA_INK_CONTRAST`] over every entry of
/// [`kit_planes`], which is what
/// `ui::theme_accessibility_tests::the_verdict_word_reads_on_the_plane_it_is_given`
/// walks for all four tones. The hue stays put — the pull moves lightness only
/// (#72) — so the word still says "this is the app pointing at something", it
/// just says it legibly on a theme its author never rendered.
#[must_use]
pub fn accent_style(theme: &Theme) -> text::Style {
    tone_style(theme, Tone::Accent)
}

/// The planes a kit word can be written on, resolved to resting pixels.
///
/// Public because `the_verdict_word_reads_on_the_plane_it_is_given` has to
/// measure the same set the solver solves against: a guard with its own copy of
/// the plane list is a second definition of "where a word lands", and the two
/// drift the moment someone adds a tier.
///
/// The seven tiers, with `Flush` answering the page it lets through — painting
/// nothing is a decision about the plane, not an absence of one. Then the two
/// washes a verdict word is actually shown on in this app (a selected property
/// row, `properties.rs:1459`; an open drop target), each resolved over every tier
/// beneath it, because a tint is not a plane until something is under it. Then
/// [`tokens::Control::Weak`], which `sheet_set.rs:2000` still paints as a card
/// plate by hand through `pdf_dialogs::card_style`; it belongs to the ladder and
/// is carried here until that screen is moved.
pub fn kit_planes(theme: &Theme) -> Vec<Color> {
    let palette = theme.palette();
    let tiers = [
        Tier::Flush,
        Tier::Band,
        Tier::Plate,
        Tier::Card,
        Tier::Float,
        Tier::Well,
        Tier::Sink,
    ];
    let washes = [
        palette
            .primary
            .weak
            .color
            .scale_alpha(tokens::TINT_SELECTED),
        palette
            .primary
            .weak
            .color
            .scale_alpha(tokens::TINT_DROPPED),
    ];

    let mut planes: Vec<Color> = tiers.iter().map(|tier| plane(theme, *tier)).collect();
    planes.push(tokens::control_plate(theme, tokens::Control::Weak));
    for tier in tiers {
        let plate = plane(theme, tier);
        for wash in washes {
            planes.push(tokens::over(wash, plate));
        }
    }
    planes
}

/// The planes a control lands on once the plate it paints itself is folded in.
///
/// [`kit_planes`] answers "where can a word be written?", which is exact for a
/// control that paints nothing (a bare field) and wrong for one that paints a
/// *translucent* something: a disabled editor sinks its recess 0.5 toward
/// whatever is under it, so the ground its placeholder is drawn on is not the
/// recess and not the tier, but the two composited. Feeding [`resolve_on`] the
/// recess alone was measured 2026-10-16 and shipped 25 of 26 themes under the
/// component floor (`the_quiet_ink_of_a_field_still_reads`) — the solver proved
/// the ink readable on a pixel that never gets painted.
///
/// An opaque plate makes this a no-op per plane ([`tokens::over`] returns the
/// foreground at a=1), so a style that owns its ground can keep passing the
/// single plate to [`tokens::legible_on`] and skip the sweep.
#[must_use]
pub fn grounds(theme: &Theme, plate: Color) -> Vec<Color> {
    kit_planes(theme)
        .into_iter()
        .map(|plane| tokens::over(plate, plane))
        .collect()
}

/// How many times [`resolve_on`] re-solves against whichever plane is currently
/// worst. Three converged every case the 26-theme sweep produced on 2026-10-16;
/// the bound is there because a style function runs per layout, not because a
/// fourth pass was ever needed — a palette that did not converge in three would
/// be reported by the guard rather than silently shipped at a near miss.
const REFINE_PASSES: usize = 3;

/// The worst plane a colour has: contrast is smallest at the plane whose
/// luminance sits nearest the ink's, so this is the number that decides whether
/// a word can be read anywhere in the app, not just where it was measured.
///
/// "Nearest" is what makes the two extremes of the plane set enough for the
/// solver in [`resolve_on`] *only while the ink sits outside that set* — lift an
/// ink above the palest plane and every darker plane gains contrast, so the pale
/// one is the whole test. An ink whose luminance falls inside the range has its
/// worst case on an interior plane, and that is the one gap the sweep at the end
/// of [`resolve_on`] exists to close.
fn worst_on(color: Color, planes: &[Color]) -> f32 {
    planes
        .iter()
        .map(|plate| wcag_contrast(tokens::over(color, *plate), *plate))
        .fold(f32::INFINITY, f32::min)
}

/// Pull an ink until it clears `min` on every plane it will be shown on, keeping
/// the palette's answer untouched whenever nothing has to move.
///
/// `min` is a parameter and not a constant because two different WCAG bars are
/// in play in this layer: [`tokens::AA_INK_CONTRAST`] for anything that is read
/// (a verdict word, a field's placeholder hint) and
/// [`tokens::AA_COMPONENT_CONTRAST`] for anything that is a graphic affordance
/// (a dropdown handle, a trailing glyph, a disabled control's text). The solver
/// itself does not care which bar it is climbing to, and hard-coding 4.5 made
/// the whole function unusable for the 3.0 cases, which is how a field's quiet
/// inks ended up being faded by hand at every call site instead of solved.
///
/// Only the two extremes of the plane set need solving. An ink lifted until it
/// reads on the palest plane reads on every darker one by the same monotonicity
/// that made `worst_on` a single number, and an ink dropped until it reads on the
/// deepest plane reads on every lighter one — so two [`tokens::legible_on`]
/// calls enumerate the whole search space, and the better result wins. Better
/// means the higher worst-case, which also keeps the pull no larger than it has
/// to be: a word that clears AA by 4.9:1 is the same word as one that clears it
/// by 4.51:1, and the first one survives the next tier the kit adds.
///
/// The cost of the move is no longer directional: [`tokens::pull_to_luminance`]
/// holds the colour's Oklab `a`/`b` vector and changes only `L`, so an ink pulled
/// *down* and an ink pulled *up* keep the hue they were given — the 2026-10-16
/// sweep over 1332 pulls measured at most 0.13° of readable hue drift either way,
/// against 17.59° for the straight line toward white this replaced. What lifting
/// still costs is chroma, and only as far as the display actually forbids: the
/// gamut near white is narrow, so a saturated mid-tone approaching an
/// 8.5:1-in-ink target arrives paler. That is the whole remaining cost, and
/// [`crate::ui::style::tokens::CHROMA_SLACK`] plus
/// `crate::ui::theme_accessibility_tests::the_pull_keeps_the_colour_it_was_given`
/// hold L0 to it, so a verdict word cannot quietly become a grey one.
///
/// One input class deserves its own note, because it is the reason callers pass
/// a faded ink in the first place. When `ink` has alpha < 1 the early test is
/// still exact — `worst_on` composites it per plane, so it measures the pixel
/// iced will paint — and the ink is kept when that pixel already clears `min`.
/// When it does not, the pull returns an *opaque* colour: [`tokens::legible_on`]
/// solves against the ink's own RGB, and an alpha whose landing point depends on
/// which of 22 planes the caller happens to be standing on cannot carry a
/// guarantee. So a rung survives as a rung only where it reads, and a theme
/// where several rungs all fail converges them onto the same AA-bound pixel.
/// That convergence is the honest outcome, not a bug to route around: it is the
/// finding of #69 restated for the quiet end of the field family.
///
/// The two-extreme solve is followed by a short sweep, and the sweep is not
/// decoration: measured 2026-10-16, Kanagawa Wave's bare-field icon cleared both
/// ends of the plane set at the 3:1 component bar and still landed at 2.9998:1
/// on an interior tier, because the ink sits *inside* the ladder rather than
/// outside it (see [`worst_on`]). Three passes over the currently-worst plane,
/// and only ever taken when the extremes did not already clear `min`, so no
/// answer this function used to give is moved by it — the sweep fires on the
/// failures and never on the passes.
#[must_use]
pub fn resolve_on(ink: Color, planes: &[Color], min: f32) -> Color {
    if worst_on(ink, planes) >= min {
        return ink;
    }
    let (mut pale, mut deep) = (planes[0], planes[0]);
    for plate in planes {
        if wcag_luminance(*plate) > wcag_luminance(pale) {
            pale = *plate;
        }
        if wcag_luminance(*plate) < wcag_luminance(deep) {
            deep = *plate;
        }
    }
    let lifted = tokens::legible_on(ink, pale, min);
    let dropped = tokens::legible_on(ink, deep, min);
    let mut best = if worst_on(lifted, planes) >= worst_on(dropped, planes) {
        lifted
    } else {
        dropped
    };
    for _ in 0..REFINE_PASSES {
        if worst_on(best, planes) >= min {
            return best;
        }
        let mut worst_plate = planes[0];
        let mut worst_score = f32::INFINITY;
        for plate in planes {
            let score = wcag_contrast(tokens::over(best, *plate), *plate);
            if score < worst_score {
                worst_score = score;
                worst_plate = *plate;
            }
        }
        best = tokens::legible_on(best, worst_plate, min);
    }
    best
}

/// The **boundary** solve: the hairline that separates a part from the one plane
/// it is dropped on.
///
/// `resolve_on` is a conjunction — it pulls one colour until it clears `min` on
/// *every* plane it is handed. That was the wrong question, and so was the one
/// before it. A band or a chip is painted on one ground at a time, and the kit's
/// law for such a part is a disjunction per ground: it reads if **either** its
/// fill is [`tokens::AA_COMPONENT_CONTRAST`] from that ground **or** its edge is.
/// Satisfying that law with a *placement-free* painter — one hairline answering
/// for all of the kit's planes at once — is not hard, it is impossible, and the
/// 2026-10-17 sweep says so exactly: of the 312 band/chip cases, **273 admit a
/// pixel and 39 admit none**, and the best any colour can reach in those 39 is
/// between 2.651:1 and 2.998:1 (`D:/tmp/s73-probe2.log`). That is not a sampling
/// artifact. WCAG contrast is a function of two relative luminances, so what a
/// pixel can do at best is the maximum over one variable of the worst reading in
/// the needing set; between two neighbouring plane luminances every branch is
/// monotone, so the peak is either an endpoint or where the darker neighbour's
/// "above" branch crosses the lighter one's "below" branch at
/// `Y + 0.05 = sqrt((a + 0.05)(b + 0.05))`. Enumerating those is exact, and every
/// luminance in [0,1] is a real grey — so an upper bound of 2.65 means no colour
/// in any colourspace clears the bar there. Nord's Success band is the shape of
/// it: its plate at `Y 0.320` forces the edge to `Y ≤ 0.073` or `Y ≥ 1.15`,
/// while `#404F56` at `Y 0.074` forces `Y ≤ -0.009` (impossible) or
/// `Y ≥ 0.322`. Empty intersection.
///
/// Alpha cannot rescue it either: compositing moves the pixel only *between* the
/// ink and the ground, so a translucent hairline can never read better against a
/// plane than the same ink drawn opaque. The ground therefore enters the
/// signature, exactly as it already does for words on paper ([`tone_style_on`])
/// and for glyphs on the document
/// ([`crate::ui::kit::surface::canvas_glyphs`]).
///
/// With one ground the law has an answer every time. Where the fill already
/// separates the part from that plane the hairline keeps the theme author's hue:
/// moving a colour nothing asked to move is a hue lost and a measurement nobody
/// needed. Where the fill loses, [`tokens::legible_on`] pulls the hue the minimum
/// distance that clears the bar, with the headroom and slack that function exists
/// to keep.
#[must_use]
pub fn resolve_edge(hue: Color, plate: Color, ground: Color, min: f32) -> Color {
    if wcag_contrast(plate, ground) >= min {
        return hue;
    }
    tokens::legible_on(hue, ground, min)
}

/// The wordmark: the product name on the page that introduces the app. The only
/// use of [`tokens::TEXT_DISPLAY`], and the only tone-carrying voice, because a
/// brand line has to arrive *with* the card it sits on — so the fade is applied
/// here rather than multiplied into a style by the screen. Until now the start
/// page rebuilt the accent style by hand and re-scaled its alpha per call site,
/// which is L1's arithmetic living in L3.
///
/// It goes through [`tone_ink`] like every other tone word, including on the
/// large-text tier where WCAG would have accepted 3:1: display size is a property
/// of the type, and the plane it lands on is a property of the theme. Iced's own
/// `Dark` accent answers 1.63:1 on the Float tier, which is not a bar any size
/// clears.
pub fn display<'a, M: 'a>(
    label: impl Into<String>,
    tone: Tone,
    track: &'static str,
) -> Element<'a, M> {
    text::Text::new(label.into())
        .size(tokens::TEXT_DISPLAY)
        .font(tokens::UI_FONT_SEMIBOLD)
        .style(move |theme: &Theme| text::Style {
            color: Some(tone_ink(theme, tone).scale_alpha(fade(track))),
        })
        .into()
}

/// Which palette swatch a [`Tone`] names — the whole verdict, not one rung of it.
///
/// Use this when a surface needs the tone at more than one strength: a plate
/// filled with `weak` and edged with `base` has to read `tone_swatch(..).weak`
/// and `tone_swatch(..).base`, never a second `match tone`. A verdict rendered
/// by two matches can put a chip's fill and its border on different hues.
#[must_use]
pub fn tone_swatch(theme: &Theme, tone: Tone) -> iced::theme::palette::Swatch {
    let palette = theme.palette();
    match tone {
        Tone::Accent => palette.primary,
        Tone::Success => palette.success,
        Tone::Warning => palette.warning,
        Tone::Danger => palette.danger,
    }
}

/// Which palette swatch a [`Tone`] names.
///
/// Both renderings of a verdict start here — [`crate::ui::kit::control::signal_button_style`]
/// fills a plate with `pair.color` and writes on it with `pair.text`, and
/// [`tone_style`] takes `pair.color` as the *hue* of a word. Which is exactly how
/// the automation pill used to paint itself with four raw `Color::from_rgb`
/// values no theme could change.
///
/// The word and the chip are not the same pixel and do not pretend to be:
/// `pair.color` is a fill chosen against its own `pair.text`, while a word has
/// to be read against whatever plane the screen put under it, so [`tone_style`]
/// solves it. Same hue family, same verdict, different measurement — and since
/// #72 the measurement may differ but the hue may not: the pull moves only the
/// colour's lightness (see
/// [`crate::ui::style::tokens::pull_to_luminance`]), on a light scheme *and* on
/// a dark one, so a verdict word keeps naming the same verdict at either end of
/// the plane ladder.
#[must_use]
pub fn tone_pair(theme: &Theme, tone: Tone) -> iced::theme::palette::Pair {
    tone_swatch(theme, tone).base
}

/// A text style for a colour that is **data**, not a decision: a DWG entity's
/// own `$TEXTCOLOR`, a history line whose hue follows the kind of line it is.
///
/// The kit owns every other ink in the app, because ink is a style decision.
/// This is the escape hatch for the two cases where the colour arrives from
/// outside the theme and painting it in a palette role would misreport the
/// drawing. The call site still supplies no numbers of its own — it passes the
/// colour some lookup already produced.
#[must_use]
pub fn color_style(color: iced::Color) -> text::Style {
    text::Style { color: Some(color) }
}

/// The two rungs this ladder adds between the named inks in `tokens`.
///
/// They live here rather than in [`tokens`] because they describe a *voice*
/// ("a group name is one notch under full ink", "a keycap is quieter than a
/// caption") and not a general ink policy — but they are named, because two
/// unnamed alphas buried in widget builders are how the #75 sweep found out
/// that four of the seven voices had never been measured by anything. L0 can
/// adopt them; the guard in
/// `ui::theme_accessibility_tests::the_quiet_voice_of_a_row_still_reads` reads
/// them from here in the meantime, which is the point.
pub const GROUP_INK: f32 = 0.92;
pub const KEY_INK: f32 = 0.78;

/// Every alpha a voice in this module paints, in ladder order.
///
/// Single source for the guard: [`ink`] and [`key`] both go through
/// [`voice_ink`], so measuring this list measures the widgets, and a new rung
/// added to the ladder is measured the moment it is added rather than in the
/// next audit.
pub const VOICE_RUNGS: [f32; 5] = [
    1.0,
    GROUP_INK,
    tokens::INK_MUTED,
    tokens::INK_FAINT,
    KEY_INK,
];

fn ink<'a, M: 'a>(
    label: impl Into<String>,
    size: f32,
    alpha: f32,
    track: &'static str,
) -> Element<'a, M> {
    ink_weighted(label, size, alpha, tokens::UI_FONT, track)
}

/// [`ink`] with an explicit face. The body voices stay on the regular instance;
/// the two titles above name their weight here rather than at each screen, so a
/// screen cannot reintroduce a mixed face by overriding `Font` alone.
fn ink_weighted<'a, M: 'a>(
    label: impl Into<String>,
    size: f32,
    alpha: f32,
    font: iced::Font,
    track: &'static str,
) -> Element<'a, M> {
    text::Text::new(label.into())
        .size(size)
        .font(font)
        .style(move |theme: &Theme| text::Style {
            color: Some(voice_ink(theme, alpha).scale_alpha(fade(track))),
        })
        .into()
}

/// A key or shortcut shown as a plate rather than as prose: `F2`, `Ctrl+Shift+C`.
///
/// The plate is [`Tier::Well`], which is opaque and already one of the planes in
/// [`kit_planes`], so [`voice_ink`]'s sweep covers the pixel this word actually
/// lands on — no [`grounds`] needed.
pub fn key<'a, M: 'a>(combo: impl Into<String>, track: &'static str) -> Element<'a, M> {
    surface(
        text::Text::new(combo.into())
            .size(tokens::TEXT_BODY - 2.0)
            .style(move |theme: &Theme| text::Style {
                color: Some(voice_ink(theme, KEY_INK).scale_alpha(fade(track))),
            }),
        Tier::Well,
        PAD_WELL,
        track,
    )
    .into()
}
