//! Centralized UI Theme Accessibility & Contrast Test Suite.
//!
//! Evaluates WCAG 2.1 contrast ratios across every selectable theme — the 22
//! built-ins plus the hand-authored DraftFlow and Fusion schemes — for every
//! major DraftFlowCAD UI domain:
//! - Core Theme Palettes (base, weak, strong, weakest)
//! - Ribbon Bar (tabs, buttons, dropdown popups, contextual layout tab)
//! - Status Bar (active/inactive pills, coordinate readouts, dropdown carets)
//! - Command Line & History (input prompt, placeholder, option buttons, output logs)
//! - Properties & Dock Panels (field labels, section headers, muted text)
//! - Modals & Action Buttons (dialog body, Primary, Danger, Success buttons)
//! - Dropdowns & Selection Overlays (visual style picker, item checkmarks)
//! - Block Palette (card labels in normal/hover/pressed/placing states,
//!   header icon buttons, empty-state muted text)

use crate::ui::style::common::{accessible_accent, accessible_accent_threshold, wcag_contrast};
use iced::{Color, Theme};

/// Composite a semi-transparent foreground color over a solid background color.
///
/// Delegates to [`crate::ui::style::tokens::over`]: the rule the guards measure
/// with has to be the rule the paint is built with, and two copies of one blend
/// is how a guarantee and its check start disagreeing.
fn composite_over(fg: Color, bg: Color) -> Color {
    crate::ui::style::tokens::over(fg, bg)
}

/// A colour as the six hex digits a screenshot would show, for measurement
/// reports that have to be checkable against a pixel dump rather than re-run.
fn hex_of(c: Color) -> String {
    let [r, g, b, _] = c.into_rgba8();
    format!("#{r:02X}{g:02X}{b:02X}")
}

/// A colour including its alpha, for the lines that have to distinguish *the ink
/// a style function hands to iced* from the pixel that ink composites to on one
/// named plane. Two rungs of the voice ladder can print the same composited hex
/// while being two different inks on two different planes; the alpha is what
/// tells the difference, and the ladder-collapses check
/// (`the_quiet_voice_of_a_row_still_reads`) needs the ink, not the sample.
fn sig_of(c: Color) -> String {
    let [r, g, b, _] = c.into_rgba8();
    format!("#{r:02X}{g:02X}{b:02X}@{:.2}", c.a)
}

/// Every theme the user can actually select, `Theme::Custom` schemes included.
///
/// `iced::Theme::ALL` omits custom themes, so iterating it leaves the
/// hand-authored DraftFlow and Fusion palettes unchecked — exactly the themes
/// with no upstream guarantees, where a single seed typo is the only thing that
/// would drop a surface below AA.
fn selectable_themes() -> Vec<Theme> {
    crate::app::config::all_themes()
}

/// The kit measured as painted, not as intended.
///
/// This suite spent its whole life checking palette pairs and per-file helpers,
/// which meant every migration wave silently moved paint out of its coverage:
/// `ui::kit::style_for` is the authority now and nothing was measuring it. The
/// probe this replaces found, over the selectable themes, a hovered Danger label
/// at 1.85:1 (`danger.base.text` — the ramp's own answer — painted on the
/// `danger.weak` tint) and a resting Ghost label at 3.09:1 (`INK_MUTED` applied
/// to interactive ink). Both are gone; this is the version that fails the build.
///
/// Contrast is taken where it lands: a wash or a faded plate is composited over
/// the page first, the ink over that. A linear gradient is sampled at both of
/// its stops and the worse one rules, because a label sits wherever it sits.
///
/// `Disabled` is exempt — WCAG 1.4.3 excludes inactive components, and the kit's
/// disabled rung (`tokens::INK_DISABLED` on both fills, shadow dropped) is meant
/// to read as unavailable. The `Danger` *resting* plate is the neutral `weak`
/// surface with the page's own ink: it clears AA today by 0.01, which is why it
/// is asserted here rather than trusted.
#[test]
fn kit_roles_hold_aa_contrast() {
    use crate::ui::kit::{style_for, Role};
    use crate::ui::style::tokens;
    use iced::widget::button;
    use iced::{Background, Gradient};
    let roles = [
        Role::Primary,
        Role::Standard,
        Role::Engaged,
        Role::Ghost,
        Role::Danger,
        Role::QuietDanger,
    ];
    let statuses = [
        button::Status::Active,
        button::Status::Hovered,
        button::Status::Pressed,
    ];
    let mut fails: Vec<String> = Vec::new();
    let mut checked = 0usize;
    for theme in selectable_themes() {
        let p = theme.palette();
        let ground = p.background.base.color;
        for role in roles {
            for status in statuses {
                let s = style_for(&theme, status, role, "");
                let plates: Vec<Color> = match s.background {
                    Some(Background::Gradient(Gradient::Linear(l))) => l
                        .stops
                        .iter()
                        .flatten()
                        .map(|st| composite_over(st.color, ground))
                        .collect(),
                    Some(Background::Color(c)) => vec![composite_over(c, ground)],
                    _ => vec![ground],
                };
                let worst_of = |ink: Color| {
                    plates
                        .iter()
                        .map(|pl| wcag_contrast(composite_over(ink, *pl), *pl))
                        .fold(f32::INFINITY, f32::min)
                };
                let measured = worst_of(s.text_color);
                checked += 1;
                if measured >= tokens::AA_INK_CONTRAST {
                    continue;
                }
                // Diagnostic: what the best available ink would have reached,
                // so a failure says whether the plate or the choice was wrong.
                let best = [
                    p.background.base.text,
                    p.background.strong.text,
                    p.background.weak.text,
                    p.danger.weak.text,
                    p.danger.base.text,
                    tokens::PLATE_INK_LIGHT,
                    tokens::PLATE_INK_DARK,
                ]
                .iter()
                .map(|c| worst_of(*c))
                .fold(0.0f32, f32::max);
                fails.push(format!(
                    "{theme} / {} / {status:?}: {:.2}:1 (best ink available would reach {:.2}:1)",
                    role_label(role),
                    measured,
                    best
                ));
            }
        }
    }
    if !fails.is_empty() {
        panic!(
            "{} of {checked} kit role/state/theme combinations fail AA (>= {:.1}:1):\n{}",
            fails.len(),
            tokens::AA_INK_CONTRAST,
            fails.join("\n")
        );
    }
}

fn role_label(role: crate::ui::kit::Role) -> &'static str {
    use crate::ui::kit::Role;
    match role {
        Role::Primary => "Primary",
        Role::Standard => "Standard",
        Role::Engaged => "Engaged",
        Role::Ghost => "Ghost",
        Role::Danger => "Danger",
        Role::QuietDanger => "QuietDanger",
    }
}

#[test]
fn test_theme_core_surfaces_contrast() {
    let themes = selectable_themes();
    for theme in &themes {
        let p = theme.palette();

        // Base surface & text
        let base_contrast = wcag_contrast(p.background.base.text, p.background.base.color);
        assert!(
            base_contrast >= 4.5,
            "Theme {:?} base text ({:.2}:1) fails WCAG AA on base background",
            theme,
            base_contrast
        );

        // Weak surface & text
        let weak_contrast = wcag_contrast(p.background.weak.text, p.background.weak.color);
        assert!(
            weak_contrast >= 4.5,
            "Theme {:?} weak text ({:.2}:1) fails WCAG AA on weak background",
            theme,
            weak_contrast
        );

        // Strong surface & text
        let strong_contrast = wcag_contrast(p.background.strong.text, p.background.strong.color);
        assert!(
            strong_contrast >= 4.5,
            "Theme {:?} strong text ({:.2}:1) fails WCAG AA on strong background",
            theme,
            strong_contrast
        );

        // Weakest surface & text
        let weakest_contrast = wcag_contrast(p.background.weakest.text, p.background.weakest.color);
        assert!(
            weakest_contrast >= 4.5,
            "Theme {:?} weakest text ({:.2}:1) fails WCAG AA on weakest background",
            theme,
            weakest_contrast
        );
    }
}

#[test]
fn test_ribbon_contrast() {
    let themes = selectable_themes();
    for theme in &themes {
        let p = theme.palette();

        // 1. Active Tab: rendered on background.weakest.color
        let active_tab_bg = p.background.weakest.color;
        let active_tab_text = p.background.weakest.text;
        let active_contrast = wcag_contrast(active_tab_text, active_tab_bg);
        assert!(
            active_contrast >= 4.5,
            "Theme {:?} active ribbon tab text ({:.2}:1) fails WCAG AA",
            theme,
            active_contrast
        );

        // 2. Inactive Tab: rendered on ribbon background.base.color with 0.72 alpha
        let ribbon_bg = p.background.base.color;
        let inactive_tab_text = composite_over(p.background.base.text.scale_alpha(0.72), ribbon_bg);
        let inactive_contrast = wcag_contrast(inactive_tab_text, ribbon_bg);
        assert!(
            inactive_contrast >= 3.0,
            "Theme {:?} inactive ribbon tab text ({:.2}:1) fails secondary text threshold (>=3.0:1)",
            theme,
            inactive_contrast
        );

        // 3. Hovered Tab: rendered on background.weak.color
        let hovered_tab_bg = p.background.weak.color;
        let hovered_tab_text = p.background.weak.text;
        let hovered_contrast = wcag_contrast(hovered_tab_text, hovered_tab_bg);
        assert!(
            hovered_contrast >= 4.5,
            "Theme {:?} hovered ribbon tab text ({:.2}:1) fails WCAG AA",
            theme,
            hovered_contrast
        );

        // 4. Ribbon Button normal & hover
        let btn_normal = wcag_contrast(p.background.base.text, ribbon_bg);
        assert!(
            btn_normal >= 4.5,
            "Theme {:?} normal ribbon button text ({:.2}:1) fails WCAG AA",
            theme,
            btn_normal
        );

        let btn_hover_bg = p.background.weak.color;
        let btn_hover_text = p.background.weak.text;
        let btn_hover = wcag_contrast(btn_hover_text, btn_hover_bg);
        assert!(
            btn_hover >= 4.5,
            "Theme {:?} hovered ribbon button text ({:.2}:1) fails WCAG AA",
            theme,
            btn_hover
        );

        // 5. Ribbon Dropdown Popup Panels
        let popup_bg = p.background.base.color;
        let popup_row_normal = wcag_contrast(p.background.base.text, popup_bg);
        assert!(
            popup_row_normal >= 4.5,
            "Theme {:?} ribbon popup normal row ({:.2}:1) fails WCAG AA",
            theme,
            popup_row_normal
        );

        let popup_row_hover_bg = p.background.weak.color;
        let popup_row_hover_text = p.background.weak.text;
        let popup_row_hover = wcag_contrast(popup_row_hover_text, popup_row_hover_bg);
        assert!(
            popup_row_hover >= 4.5,
            "Theme {:?} ribbon popup hovered row ({:.2}:1) fails WCAG AA",
            theme,
            popup_row_hover
        );
    }
}

/// The status bar's mode pills, measured where their paint is decided.
///
/// The test this replaces asserted `primary.weak.text` on `primary.weak.color`
/// for the lit pill and `base.text` at 0.72 on `weakest.color` for the resting
/// one. Neither plate is painted any more: the pill ladder moved to the aurora
/// wash, [`crate::ui::kit::surface::pill_style`] handed iced no ink at all
/// (`text_color: None`), and every call site guessed — `themed_primary_weak_text`
/// for the icon, `tone_style(Accent)` for the polar angle, a third hand-mixed
/// `tokens::surface(Rank::Band)` plate for the resting toggle, `primary.base`
/// for the space-mode capsule's hover. The old test stayed green through all of
/// it because it was arithmetic on palette slots rather than on the painter. An
/// assertion that measures a function nobody calls is worse than none: it
/// reports the defect as checked.
///
/// So this calls [`crate::ui::kit::surface::pill_style`] and
/// [`crate::ui::kit::surface::pill_button_style`] the way the bar calls them,
/// over every selectable theme, in both lit states, at both pointer rungs and
/// all three button statuses, and holds the ink to AA on the plate it actually
/// lands on. Three further invariants are the ones that broke:
///
/// - the painter must hand iced an ink, not `None` (`None` is the hole L3 fell
///   into);
/// - the container and the button are one rung, so at the same hover they must
///   produce the same plate — `Pressed` legitimately lifts the pointer channel
///   to its top, which is the brightest the plate ever gets;
/// - the ink an `svg` icon asks for through [`crate::ui::kit::surface::pill_ink`]
///   (an `svg` style closure cannot inherit) must be the ink the plate hands the
///   label, or one pill reads two rungs.
///
/// Hover rides the motion registry with one id per rung, because `pill_hover`
/// reads `hover:<id>` and a shared id would let the last write win.
#[test]
fn status_pills_are_measured_where_they_are_painted() {
    use crate::ui::kit::surface::{pill_button_style, pill_ink, pill_style};
    use crate::ui::motion;
    use crate::ui::style::tokens;
    use iced::widget::button;
    use iced::{Background, Gradient};

    const RUNGS: [(&'static str, f32); 2] = [
        ("aatest:pill-rest", 0.0),
        ("aatest:pill-lift", 1.0),
    ];
    let statuses = [
        button::Status::Active,
        button::Status::Hovered,
        button::Status::Pressed,
    ];
    let mut fails: Vec<String> = Vec::new();
    let mut checked = 0usize;
    for theme in selectable_themes() {
        let p = theme.palette();
        let ground = p.background.base.color;

        // 1. The bar's readouts — coordinates, scale, layout name — are page ink
        //    on the bar ground: normal text, so AA.
        let label_contrast = wcag_contrast(p.background.base.text, ground);
        assert!(
            label_contrast >= 4.5,
            "Theme {theme:?} statusbar coordinate/scale text ({label_contrast:.2}:1) fails WCAG AA",
        );

        // A plate as the pixels it lands on, gradient stops included: a label sits
        // wherever it sits, so the worse stop rules.
        let plates_of = |bg: &Option<Background>| -> Vec<Color> {
            match bg {
                Some(Background::Gradient(Gradient::Linear(l))) => l
                    .stops
                    .iter()
                    .flatten()
                    .map(|st| composite_over(st.color, ground))
                    .collect(),
                Some(Background::Color(c)) => vec![composite_over(*c, ground)],
                _ => vec![ground],
            }
        };
        let sig_list = |plates: &[Color]| -> Vec<String> {
            plates.iter().map(|c| sig_of(*c)).collect()
        };
        let contrast_against = |ink: Color, plates: &[Color]| {
            plates
                .iter()
                .map(|pl| wcag_contrast(composite_over(ink, *pl), *pl))
                .fold(f32::INFINITY, f32::min)
        };

        for (id, hover) in RUNGS {
            motion::set(motion::hover_key(id), hover, 0);
            for active in [false, true] {
                let c = pill_style(id, active, &theme, "");
                let cplates = plates_of(&c.background);
                for status in statuses {
                    let b = pill_button_style(id, active, &theme, status, "");
                    let bplates = plates_of(&b.background);
                    let tag = format!("{theme} / {id} / active={active} / {status:?}");
                    // `container::Style` carries an `Option<Color>`; the ladder
                    // now fills it (it used to hand iced `None`, which is the
                    // hole L3 fell into). `button::Style` carries a plain
                    // `Color`, so the ink there cannot be omitted at all.
                    let Some(c_ink) = c.text_color else {
                        fails.push(format!(
                            "{tag}: the pill painter returned no text_color — the plate is painted but the ink is left to L3, which is how the lit pill ended up wearing primary.weak.text"
                        ));
                        continue;
                    };
                    let b_ink = b.text_color;
                    checked += 1;

                    let worst = contrast_against(c_ink, &cplates)
                        .min(contrast_against(b_ink, &bplates));
                    if worst < tokens::AA_INK_CONTRAST {
                        let best = [
                            p.background.base.text,
                            p.background.strong.text,
                            p.background.weak.text,
                            p.background.weakest.text,
                        ]
                        .iter()
                        .map(|cand| {
                            contrast_against(*cand, &cplates)
                                .max(contrast_against(*cand, &bplates))
                        })
                        .fold(0.0f32, f32::max);
                        fails.push(format!(
                            "{tag}: ink {worst:.2}:1 (container {} on [{}], button {} on [{}]); best ink in the ramp would reach {best:.2}:1",
                            sig_of(c_ink),
                            sig_list(&cplates).join(", "),
                            sig_of(b_ink),
                            sig_list(&bplates).join(", "),
                        ));
                    }

                    // One rung, two shapes. A press is the pointer channel at
                    // full, so at hover 1.0 even a press must still agree.
                    let same_rung = !matches!(status, button::Status::Pressed) || hover > 0.99;
                    if same_rung && sig_list(&cplates) != sig_list(&bplates) {
                        fails.push(format!(
                            "{tag}: the container pill and the button pill painted different plates at hover {hover}: [{}] vs [{}]",
                            sig_list(&cplates).join(", "),
                            sig_list(&bplates).join(", "),
                        ));
                    }

                    // The icon asks; the plate answers. Two answers means the
                    // icon and the label of one pill are on different rungs.
                    let asked = sig_of(pill_ink(id, active, &theme, ""));
                    let given = sig_of(c_ink);
                    if asked != given {
                        fails.push(format!(
                            "{tag}: pill_ink hands an svg icon {asked} but the same plate hands its label {given}"
                        ));
                    }
                }
            }
        }
    }
    if !fails.is_empty() {
        panic!(
            "{} of {checked} status-pill states fail AA (>= {:.1}:1) or break one-ladder:\n{}",
            fails.len(),
            tokens::AA_INK_CONTRAST,
            fails.join("\n")
        );
    }
}

/// The numbers behind the #73 sample sheet (`D:\tmp\pull-pill-73.html`).
///
/// It prints, per theme, the plate and ink the kit's pill ladder paints now,
/// beside the plate/ink pairs the deleted status-bar code actually put on
/// screen — reconstructed from those call sites verbatim (`primary.weak.text`
/// for a lit icon, `base.text` at 0.72 for a resting one, `tone_style(Accent)`
/// for the polar angle, `primary.weak/base.color` for the space-mode capsule,
/// `tokens::surface(Rank::Band)` for the resting toggle plate). Every colour
/// block in the sheet is therefore a measured pixel, not a re-drawn guess.
/// Re-run with
/// `cargo test --lib status_pill_ladder_measurements -- --nocapture`.
///
/// Rows are `theme|case|state|plate|ink|ratio`. A gradient plate lists both
/// stops joined by `+` and the ratio is the worse one, matching
/// [`kit_roles_hold_aa_contrast`]. Alpha travels with the hex because a plate is
/// composited over the bar ground before an ink is measured against it.
#[test]
fn status_pill_ladder_measurements() {
    use crate::ui::kit::{self, surface, Role};
    use crate::ui::motion;
    use crate::ui::style::tokens;
    use iced::widget::button;
    use iced::{Background, Gradient};

    fn stop_colors(bg: &Option<Background>) -> Vec<Color> {
        match bg {
            Some(Background::Gradient(Gradient::Linear(l))) => {
                l.stops.iter().flatten().map(|st| st.color).collect()
            }
            Some(Background::Color(c)) => vec![*c],
            _ => Vec::new(),
        }
    }

    for theme in selectable_themes() {
        let p = theme.palette();
        let ground = p.background.base.color;
        let row = |case: &str, state: &str, plates: &[Color], ink: Color| {
            let comp: Vec<Color> = if plates.is_empty() {
                vec![ground]
            } else {
                plates.iter().map(|c| composite_over(*c, ground)).collect()
            };
            let ratio = comp
                .iter()
                .map(|pl| wcag_contrast(composite_over(ink, *pl), *pl))
                .fold(f32::INFINITY, f32::min);
            println!(
                "{theme}|{case}|{state}|{}|{}|{ratio:.2}",
                comp.iter().map(|c| sig_of(*c)).collect::<Vec<_>>().join("+"),
                sig_of(ink),
            );
        };

        // ── after: the ladder, as the bar calls it ──────────────────────────
        for (id, hover) in [("m:pill-0", 0.0_f32), ("m:pill-1", 1.0_f32)] {
            motion::set(motion::hover_key(id), hover, 0);
            for active in [false, true] {
                let state = format!("{}@{hover:.0}", if active { "lit" } else { "rest" });
                let c = surface::pill_style(id, active, &theme, "");
                row(
                    "kit-container",
                    &state,
                    &stop_colors(&c.background),
                    c.text_color.expect("the ladder must ink its own plate"),
                );
                let b = surface::pill_button_style(id, active, &theme, button::Status::Active, "");
                row("kit-button", &state, &stop_colors(&b.background), b.text_color);
                let i = surface::pill_ink(id, active, &theme, "");
                row("kit-icon", &state, &stop_colors(&c.background), i);
            }
        }

        // ── before: pairs the deleted code painted ──────────────────────────
        motion::set(motion::hover_key("m:pill-0"), 0.0, 0);
        let engaged = kit::style_for(&theme, button::Status::Active, Role::Engaged, "");
        let eplate = stop_colors(&engaged.background);
        let rest = surface::pill_style("m:pill-0", false, &theme, "");
        let rplate = stop_colors(&rest.background);
        let faded = p.background.base.text.scale_alpha(0.72);
        row("old-icon-lit", "lit@0", &eplate, p.primary.weak.text);
        row("old-angle-lit", "lit@0", &eplate, kit::tone_style(&theme, kit::Tone::Accent).color.expect("tone ink"));
        row("old-angle-rest", "rest@0", &rplate, kit::muted_style(&theme).color.expect("muted ink"));
        row("old-icon-rest", "rest@0", &rplate, faded);
        row("old-toggle-rest", "rest@0", &[tokens::surface(&theme, tokens::Rank::Band)], faded);
        // The space-mode capsule, and the pair the deleted assertion checked:
        // they were the same numbers, which is why it never saw the defect.
        row("old-space-rest", "lit@0", &[p.primary.weak.color], p.primary.weak.text);
        row("old-space-hover", "lit@0", &[p.primary.base.color], p.primary.weak.text);
        let sub = button::subtle(&theme, button::Status::Active);
        row("old-space-inactive", "rest@0", &stop_colors(&sub.background), sub.text_color);
    }
}

/// The second half of #73: the three sub-items the status-pill pass left open.
///
/// * **The band's edge.** [`the_verdict_band_feeds_its_own_words`] says so in its
///   own doc: it measures the band's words and deliberately does not measure the
///   one-pixel `swatch.base` edge against the tier the band was dropped into.
///   That is a WCAG 1.4.11 question about a non-text part, so it is measured
///   against [`tokens::AA_COMPONENT_CONTRAST`], and both the fill and the edge
///   are printed: a band whose fill vanishes into its neighbour never gets to the
///   edge.
/// * **`signal_button_style`'s private resting alpha.** The painter is called
///   directly, so these are the numbers of whatever ships. Before the fix that
///   was `pair.color` at 0.82 over the ground, an opaque `pair.text` on top of a
///   translucent plate, and a border that is the *same* colour as the plate, so
///   the chip had no edge at all; `D:/tmp/s73b-measure.log` holds that snapshot
///   (251 of 728 resting inks and 226 of 728 borders below their floor). After
///   the fix the plate is one of L0's `painted()` rungs — opaque, so its contrast
///   against a tier is a property of the pixel and not of what is underneath —
///   and the edge is a solved boundary. All three are printed for both rungs and
///   every tier, because the boundary law is a disjunction: the chip's fill reads
///   against the tier, *or* its edge does.
/// * **The canvas constraint glyphs.** Printed on both sides. `glyph-*` rows
///   reconstruct the literals the overlay shipped before this item (`#676D76`
///   badge, `Color::WHITE` ink, `#2391E6` coincident, `#D64C4C` object-lock and
///   datum, `#48C774` equal, `#E0823E` point marker, pure red hover cross, and
///   the danger / primary-strong palette pair for conflict and selection ring).
///   `glyph2-*` rows call the replacement,
///   [`crate::ui::kit::surface::canvas_glyphs`], so the sheet compares the two
///   against the same grounds. The canvas grounds are `Scene::default()`'s own
///   values (model `#212830`, paper `#FFFFFF`), not invented ones, because a
///   glyph drawn on paper has nothing to do with the theme's page colour.
///
/// Rows are `theme|case|state|plate|ink|ratio|ground`. `plate` is the composited
/// pixel the part is measured *against*; `ink` is the part as the style hands it
/// over, alpha included, so a translucent border still reads as translucent in
/// the sheet and the generator can re-composite it; `ground` is whatever was
/// underneath. Ratio is `contrast(over(ink, plate), plate)` — the same
/// composition the painter will perform on screen.
///
/// Re-run with
/// `cargo test --lib band_signal_glyph_measurements -- --nocapture`.
#[test]
fn band_signal_glyph_measurements() {
    use crate::ui::kit::{self, surface, Tier, Tone};
    use iced::widget::button;

    const TONES: [Tone; 4] = [Tone::Accent, Tone::Success, Tone::Warning, Tone::Danger];
    const TIERS: [Tier; 7] = [
        Tier::Flush,
        Tier::Band,
        Tier::Plate,
        Tier::Card,
        Tier::Float,
        Tier::Well,
        Tier::Sink,
    ];

    for theme in selectable_themes() {
        let p = theme.palette();

        let row = |case: &str, state: &str, backdrop: Color, ground: Color, part: Color| {
            let plate = composite_over(backdrop, ground);
            let painted = composite_over(part, plate);
            let ratio = wcag_contrast(painted, plate);
            println!(
                "{theme}|{case}|{state}|{}|{}|{ratio:.2}|{}",
                sig_of(plate),
                sig_of(part),
                sig_of(ground),
            );
        };

        // ── the verdict band: fill and edge against every tier beside it ────
        // One painter call per tier, because the band is now asked which plane it
        // sits on: the plate and the words are ground-independent, the hairline is
        // not. Printing all three per tier is what lets the sheet show that.
        for tone in TONES {
            for tier in TIERS {
                let ground = kit::plane(&theme, tier);
                let style = surface::tone_plate_style_on(&theme, tone, ground, "");
                let fill = match style.background {
                    Some(iced::Background::Color(c)) => c,
                    other => panic!("{theme} {tone:?} verdict band paints no ground: {other:?}"),
                };
                let ink = style
                    .text_color
                    .unwrap_or_else(|| panic!("{theme} {tone:?} verdict band gives its words no ink"));
                let edge = style.border.color;
                let at = format!("{tone:?}@{tier:?}");
                row("band-fill", &at, ground, ground, fill);
                row("band-edge", &at, ground, ground, edge);
                row("band-ink", &at, fill, ground, ink);
            }
        }

        // ── signal_button_style: the painter itself, rest and lit ───────────
        for tone in TONES {
            for tier in TIERS {
                let paint = kit::signal_button_style(tone, tier);
                for (case, status) in [
                    ("signal", button::Status::Active),
                    ("signal-lit", button::Status::Hovered),
                ] {
                    let style = paint(&theme, status);
                    let badge = match style.background {
                        Some(iced::Background::Color(c)) => c,
                        other => panic!("{theme} {tone:?} {case} button paints no plate: {other:?}"),
                    };
                    let ground = kit::plane(&theme, tier);
                    let at = format!("{tone:?}@{tier:?}");
                    row(&format!("{case}-ink"), &at, badge, ground, style.text_color);
                    // The chip's own fill against the tier it is dropped into:
                    // the boundary law is `this reads` OR `its edge reads`, so
                    // both halves have to be on the sheet.
                    row(&format!("{case}-fill"), &at, ground, ground, badge);
                    row(&format!("{case}-edge"), &at, ground, ground, style.border.color);
                }
            }
        }

        // ── the canvas constraint glyphs: what overlay.rs used to paint ─────
        let normal_bg = Color::from_rgb8(103, 109, 118);
        let normal_fg = Color::WHITE;
        let coincident_bg = Color::from_rgb8(35, 145, 230);
        let lock_red = Color::from_rgb8(214, 76, 76);
        let equal_green = Color::from_rgb8(72, 199, 116);
        let marker_orange = Color::from_rgb8(224, 130, 62);
        let hover_red = Color::from_rgb(1.0, 0.0, 0.0);
        let conflict_bg = p.danger.base.color;
        let conflict_fg = p.danger.base.text;
        let selected_ring = p.primary.strong.color;

        for (case, backdrop, part) in [
            ("glyph-badge", normal_bg, normal_fg),
            ("glyph-coincident", coincident_bg, normal_fg),
            ("glyph-conflict", conflict_bg, conflict_fg),
            ("glyph-equal", normal_bg, equal_green),
            ("glyph-equal-on-conflict", conflict_bg, equal_green),
            ("glyph-lock-object", normal_bg, lock_red),
            ("glyph-lock-point", normal_bg, normal_fg),
            ("glyph-vertical-datum", normal_bg, lock_red),
            ("glyph-point-marker", normal_bg, marker_orange),
            ("glyph-ring", normal_bg, selected_ring),
            ("glyph-ring-on-conflict", conflict_bg, selected_ring),
        ] {
            row(case, "badge", backdrop, backdrop, part);
        }
        for (at, canvas) in [("model", Color::from_rgb8(33, 40, 48)), ("paper", Color::from_rgb8(255, 255, 255))] {
            // A dynamic dimension draws no badge (`overlay.rs:2238`), so the
            // white shackle, the grey keyhole, the selection ring and the hover
            // X are laid straight onto the drawing.
            row("glyph-bare-shackle", at, canvas, canvas, normal_fg);
            row("glyph-bare-keyhole", at, canvas, canvas, normal_bg);
            row("glyph-ring-on-canvas", at, canvas, canvas, selected_ring);
            row("glyph-hover-on-canvas", at, canvas, canvas, hover_red);
            // The replacement: one solve for one canvas pixel, then every part
            // measured on the ground it is actually drawn over.
            let g = surface::canvas_glyphs(canvas, &theme);
            for (state, b) in [
                ("ordinary", &g.ordinary),
                ("coincident", &g.coincident),
                ("conflict", &g.conflict),
            ] {
                let at2 = format!("{state}@{at}");
                row("glyph2-plate", &at2, canvas, canvas, b.plate);
                row("glyph2-ink", &at2, b.plate, canvas, b.ink);
                row("glyph2-equal", &at2, b.plate, canvas, b.equal);
                row("glyph2-datum", &at2, b.plate, canvas, b.datum);
                row("glyph2-marker", &at2, b.plate, canvas, b.marker);
                row("glyph2-ring", &at2, b.plate, canvas, b.ring);
            }
            row("glyph2-bare-shackle", at, canvas, canvas, g.bare);
            row("glyph2-bare-keyhole", at, g.bare, canvas, g.bare_keyhole);
            row("glyph2-ring-bare", at, canvas, canvas, g.bare_ring);
            row("glyph2-hover", at, canvas, canvas, g.hover);
        }
    }
}

/// The automation chip painted its own rung of the ladder: `pair.color` at a
/// private 0.82 for rest, 1.0 for the pointer, and a border set to *that same
/// translucent colour*, so the chip had no edge — it was a wash whose depth
/// depended on whatever tier it was dropped into. Measured over 26 themes × 4
/// tones × 7 tiers (`D:/tmp/s73b-measure.log`, 2026-10-17 00:0x), 251 of 728
/// resting inks fell below AA on the plate they were handed (worst 3.07:1,
/// Fusion Black `Danger` at `Sink`) and 226 of 728 borders fell below
/// AA_COMPONENT against their tier.
///
/// The fix hands the rung to L0 (`tokens::SIGNAL_DIP_REST` / `_LIT` through
/// `tokens::painted`) and gives the chip a solved boundary. This guard holds the
/// four laws that makes the failure impossible rather than the four numbers that
/// happen to pass today:
///
///   1. the plate is opaque, so its contrast against a plane is a property of the
///      pixel, not of what is underneath it;
///   2. the ink clears [`tokens::AA_INK_CONTRAST`] on that plate at **both** rungs,
///      and neither the plate nor the ink moves when the ground does — a chip whose
///      depth depended on the tier underneath it is the 0.82 bug back again;
///   3. the boundary is a disjunction, measured **one plane at a time**: on each of
///      the seven tiers a screen can drop the chip on, the fill reads *or* the edge
///      does, at [`tokens::AA_COMPONENT_CONTRAST`]. The ground is an argument to the
///      painter ([`crate::ui::kit::control::signal_button_style`]) because a
///      placement-free hairline cannot serve this law, and that is arithmetic, not
///      a solver bug: of the 312 band and chip cases swept on 2026-10-17, **39 admit
///      no colour whatsoever**, and the exact supremum over every pixel in every
///      colourspace in those 39 tops out at 2.651:1 — Tokyo Night's `Danger` chip at
///      the pointer rung (`D:/tmp/s73-probe2.log`;
///      [`a_placement_free_edge_is_provably_impossible`] re-derives the bound
///      inside this suite). Two earlier answers are on the record as the wrong
///      questions they were. Solving the edge against the chip's own plate left
///      **62 of 1456** tier readings with neither cue. Solving it against its own
///      plate *and* every plane in [`crate::ui::kit::label::kit_planes`] — a
///      conjunction, one hairline answering for 23 grounds — left **397 of 4784**,
///      and asked a colour to clear two pixels at once on **106 of the 2893**
///      plate/plane pairs, where the maths says none can. Both limbs are retired:
///      the plate a chip outlines is not a plane it is dropped on, and the plane it
///      is dropped on is the only ground it is measured against.
///   4. the pointer actually moves the pixel: a lit plate identical to the
///      resting one is the 0.82 bug wearing a different coat;
///   5. disabled dims both fills by [`tokens::INK_DISABLED`] and leaves the
///      border where it was. The drafts' tick ships this status every time the
///      alias or the command is empty (`on_press_maybe(done_ok.then_some(..))`),
///      and a plate at full strength there is a chip that says "ready" while it
///      cannot answer a press. This arm was absent until 2026-10-21 and only a
///      screenshot of the real executable caught it.
#[test]
fn the_signal_chip_carries_its_own_words() {
    use crate::ui::kit::{self, Tier, Tone};
    use crate::ui::style::tokens;
    use iced::widget::button;

    const TONES: [Tone; 4] = [Tone::Accent, Tone::Success, Tone::Warning, Tone::Danger];
    // Every plane a screen can drop a chip on. Not `kit_planes`: those include the
    // kit's own wash composites, and no call site hands the chip one of those — a
    // `Tier` is the vocabulary the painter was given, so it is the vocabulary the
    // guard sweeps.
    const GROUNDS: [Tier; 7] = [
        Tier::Flush,
        Tier::Band,
        Tier::Plate,
        Tier::Card,
        Tier::Float,
        Tier::Well,
        Tier::Sink,
    ];

    let mut fails: Vec<String> = Vec::new();
    let mut checked = 0usize;
    let mut rungs = 0usize;
    let themes = selectable_themes();
    let theme_count = themes.len();
    // The debt this guard owes, totalled from the loop structure before any
    // measurement happens: four tones, two rungs, seven grounds. Computed here
    // rather than accumulated inside the sweep, so a theme, rung or ground that
    // silently stopped being measured shows up short instead of self-consistent.
    let expected = theme_count * TONES.len() * GROUNDS.len() * 2;
    let expected_rungs = theme_count * TONES.len() * 2;

    for theme in themes {
        for tone in TONES {
            let hue = kit::label::tone_pair(&theme, tone).color;
            let plate_of = |style: &button::Style, what: &str| match style.background {
                Some(iced::Background::Color(c)) => c,
                other => panic!("{theme} {tone:?} {what} chip paints no plate: {other:?}"),
            };

            for (what, status) in [
                ("resting", button::Status::Active),
                ("lit", button::Status::Hovered),
            ] {
                // The reference painter, asked on one ground. The plate and the
                // words answer to the tone and the rung and never to the plane
                // underneath; that is asserted per ground below, and it is what
                // makes one plate measurable against seven planes instead of
                // having to sweep seven plates.
                let reference = kit::signal_button_style(tone, Tier::Band)(&theme, status);
                let plate = plate_of(&reference, what);
                let ink = reference.text_color;
                rungs += 1;

                if plate.a != 1.0 {
                    fails.push(format!(
                        "{theme} {tone:?} {what} plate {} is translucent (a {:.2})",
                        hex_of(plate),
                        plate.a
                    ));
                }
                let on_plate = wcag_contrast(composite_over(ink, plate), plate);
                if on_plate < tokens::AA_INK_CONTRAST {
                    fails.push(format!(
                        "{theme} {tone:?} {what} ink {} on {}: {on_plate:.2}:1",
                        hex_of(ink),
                        hex_of(plate)
                    ));
                }

                for ground in GROUNDS {
                    let placed = kit::signal_button_style(tone, ground)(&theme, status);
                    let plane = kit::plane(&theme, ground);
                    let edge = placed.border.color;
                    checked += 1;

                    // A chip that changes colour with the tier under it is a wash,
                    // and the disjunction below would then be measuring a different
                    // pixel on every ground — the law would hold and the chip would
                    // still have no fixed depth.
                    if plate_of(&placed, what) != plate {
                        fails.push(format!(
                            "{theme} {tone:?} {what} chip on {ground:?} is a wash: the plate moved \
                             from {} to {}",
                            hex_of(plate),
                            hex_of(plate_of(&placed, what))
                        ));
                    }
                    if placed.text_color != ink {
                        fails.push(format!(
                            "{theme} {tone:?} {what} chip on {ground:?} re-solved its own words for \
                             the plane underneath: {} became {}",
                            hex_of(ink),
                            hex_of(placed.text_color)
                        ));
                    }

                    let fill_cue = wcag_contrast(plate, plane);
                    let edge_cue = wcag_contrast(composite_over(edge, plane), plane);
                    if fill_cue < tokens::AA_COMPONENT_CONTRAST
                        && edge_cue < tokens::AA_COMPONENT_CONTRAST
                    {
                        fails.push(format!(
                            "{theme} {tone:?} {what} chip on {ground:?} at {}: neither fills \
                             ({fill_cue:.2}:1) nor edges ({edge_cue:.2}:1)",
                            hex_of(plane)
                        ));
                    }
                    // The economy rule, now per ground: where the fill already
                    // separates the chip from the plane it sits on, the theme
                    // author's hue stays exactly as shipped. A hue moved for a
                    // boundary that was never missing is a colour lost and a
                    // measurement nobody asked for.
                    if fill_cue >= tokens::AA_COMPONENT_CONTRAST && edge != hue {
                        fails.push(format!(
                            "{theme} {tone:?} {what} chip on {ground:?} filled at {fill_cue:.2}:1 \
                             and its edge was still moved from {} to {}",
                            hex_of(hue),
                            hex_of(edge)
                        ));
                    }

                    println!(
                        "QC73C {theme}|{tone:?}|{what}|on {ground:?} {}|plate {}|ink {} {on_plate:.2}:1|edge {} fills {fill_cue:.2} edges {edge_cue:.2}",
                        hex_of(plane),
                        hex_of(plate),
                        hex_of(ink),
                        hex_of(edge),
                    );
                }
            }

            let rest_plate = plate_of(
                &kit::signal_button_style(tone, Tier::Band)(&theme, button::Status::Active),
                "resting",
            );
            let lit_plate = plate_of(
                &kit::signal_button_style(tone, Tier::Band)(&theme, button::Status::Hovered),
                "lit",
            );
            if rest_plate == lit_plate {
                fails.push(format!(
                    "{theme} {tone:?} pointer changes nothing: {} at both rungs",
                    hex_of(rest_plate)
                ));
            }

            // Disabled is not a third rung of that ladder: it is the resting chip
            // with both fills dimmed by the kit's one disabled treatment, border
            // untouched. The two draft rows ship exactly this status while their
            // input is incomplete, so an undimmed plate there is the chip saying
            // "ready" next to a press it will refuse.
            let resting =
                kit::signal_button_style(tone, Tier::Band)(&theme, button::Status::Active);
            let disabled =
                kit::signal_button_style(tone, Tier::Band)(&theme, button::Status::Disabled);
            let disabled_plate = plate_of(&disabled, "disabled");
            if disabled_plate.a != tokens::INK_DISABLED {
                fails.push(format!(
                    "{theme} {tone:?} disabled plate {} carries alpha {:.2}, not INK_DISABLED \
                     ({:.2})",
                    hex_of(disabled_plate),
                    disabled_plate.a,
                    tokens::INK_DISABLED
                ));
            }
            if disabled.text_color.a != tokens::INK_DISABLED {
                fails.push(format!(
                    "{theme} {tone:?} disabled ink {} carries alpha {:.2}, not INK_DISABLED \
                     ({:.2})",
                    hex_of(disabled.text_color),
                    disabled.text_color.a,
                    tokens::INK_DISABLED
                ));
            }
            if disabled.border != resting.border {
                fails.push(format!(
                    "{theme} {tone:?} disabled chip moved its border: {:?} became {:?}",
                    resting.border, disabled.border
                ));
            }
        }
    }

    println!("QC73C checked {checked} chip/plane pairings, {} failures", fails.len());
    if !fails.is_empty() {
        panic!(
            "{} signal-chip readings below their floor:\n{}",
            fails.len(),
            fails.join("\n")
        );
    }
    assert_eq!(checked, expected);
    assert_eq!(rungs, expected_rungs);
}

/// The constraint badges on the drawing were eight `Color::from_rgb8` literals
/// and one palette pair, painted identically whether the document was the model
/// canvas or paper. Measured before the fix (`D:/tmp/s73b-measure.log`): a dynamic
/// dimension's white lock on paper is **1.00:1**, invisible in 26 of 26 themes;
/// its grey keyhole on the model canvas 2.85:1; the selection ring against its own
/// badge 1.01:1; the object-mode lock, the vertical datum, the point marker and
/// the `Equal` label between 1.24:1 and 2.41:1 on the badge they all shared.
///
/// `kit::surface::canvas_glyphs` now solves every part for the canvas pixel in
/// hand, one part per ground. This guard asserts the ground is the *only* thing
/// the part can be measured against — a mark on a plate is measured on that plate,
/// not on a plate it never lands on — and that nothing ships translucent, because
/// a translucent glyph on a user-chosen `COLORBG` blends into the drawing it is
/// supposed to sit on top of.
#[test]
fn the_canvas_glyphs_feed_their_own_drawing() {
    use crate::ui::kit::surface::{canvas_glyphs, CanvasBadge};
    use crate::ui::style::{common::wcag_luminance, tokens};

    // `Scene::default()`'s two shipped canvases, not invented ones: model and
    // paper bracket what a user can set COLORBG to.
    let canvases = [
        ("model", Color::from_rgb8(33, 40, 48)),
        ("paper", Color::from_rgb8(255, 255, 255)),
    ];
    let mut fails: Vec<String> = Vec::new();
    let mut checked = 0usize;
    let themes = selectable_themes();
    let theme_count = themes.len();

    for theme in themes {
        for (at, canvas) in canvases {
            let g = canvas_glyphs(canvas, &theme);
            let mut measure = |what: &str, ground: Color, part: Color, min: f32| {
                let ratio = wcag_contrast(composite_over(part, ground), ground);
                checked += 1;
                if part.a != 1.0 {
                    fails.push(format!(
                        "{theme} on {at}: {what} {} is translucent (a {:.2})",
                        hex_of(part),
                        part.a
                    ));
                }
                if ratio < min {
                    fails.push(format!(
                        "{theme} on {at}: {what} {} on {}: {ratio:.2}:1 (< {min})",
                        hex_of(part),
                        hex_of(ground)
                    ));
                }
                println!(
                    "QC73D {theme}|{at}|{what}|{} on {}|{ratio:.2}",
                    hex_of(part),
                    hex_of(ground)
                );
            };
            for (state, b) in [
                ("ordinary", &g.ordinary),
                ("coincident", &g.coincident),
                ("conflict", &g.conflict),
            ] {
                let CanvasBadge {
                    plate,
                    ink,
                    equal,
                    datum,
                    marker,
                    ring,
                } = *b;
                measure(&format!("{state} plate"), canvas, plate, tokens::AA_COMPONENT_CONTRAST);
                measure(&format!("{state} ink"), plate, ink, tokens::AA_INK_CONTRAST);
                measure(&format!("{state} equal"), plate, equal, tokens::AA_INK_CONTRAST);
                measure(&format!("{state} datum"), plate, datum, tokens::AA_INK_CONTRAST);
                measure(&format!("{state} marker"), plate, marker, tokens::AA_INK_CONTRAST);
                measure(&format!("{state} ring"), plate, ring, tokens::AA_COMPONENT_CONTRAST);
            }
            measure("bare lock", canvas, g.bare, tokens::AA_INK_CONTRAST);
            // The keyhole is struck *inside* the lock body, so its ground is the
            // body's pixel rather than the drawing behind it.
            measure("bare keyhole", g.bare, g.bare_keyhole, tokens::AA_INK_CONTRAST);
            measure("bare ring", canvas, g.bare_ring, tokens::AA_COMPONENT_CONTRAST);
            measure("hover cross", canvas, g.hover, tokens::AA_COMPONENT_CONTRAST);
            // A solved part that is the canvas itself has vanished into it, even
            // if the maths above says the ratio holds at some alpha.
            if wcag_luminance(g.bare) == wcag_luminance(canvas) {
                fails.push(format!("{theme} on {at}: the bare lock is the canvas"));
            }
        }
    }

    println!(
        "QC73D checked {checked} canvas-glyph readings, {} failures",
        fails.len()
    );
    if !fails.is_empty() {
        panic!(
            "{} canvas-glyph readings below their floor:\n{}",
            fails.len(),
            fails.join("\n")
        );
    }
    // Six readings per badge (plate, ink, three marks, ring) × three badges, plus
    // the four parts struck straight on the drawing.
    assert_eq!(checked, theme_count * canvases.len() * (6 * 3 + 4));
}

#[test]
fn test_command_line_contrast() {
    let themes = selectable_themes();
    for theme in &themes {
        let p = theme.palette();
        let cli_bg = p.background.base.color;

        // 1. Input Value Text (normal text: >= 4.5:1)
        let input_contrast = wcag_contrast(p.background.base.text, cli_bg);
        assert!(
            input_contrast >= 4.5,
            "Theme {:?} command line input text ({:.2}:1) fails WCAG AA",
            theme,
            input_contrast
        );

        // 1b. Prompt ("Command:") text (normal text: >= 4.5:1)
        let prompt_color =
            accessible_accent_threshold(p.success.base.color, cli_bg, p.background.base.text, 4.5);
        let prompt_contrast = wcag_contrast(prompt_color, cli_bg);
        assert!(
            prompt_contrast >= 4.5,
            "Theme {:?} command prompt text ({:.2}:1) fails WCAG AA (>=4.5:1)",
            theme,
            prompt_contrast
        );

        // 2. Placeholder Text (0.72 alpha, secondary text: >= 3.0:1)
        let placeholder = composite_over(p.background.base.text.scale_alpha(0.72), cli_bg);
        let placeholder_contrast = wcag_contrast(placeholder, cli_bg);
        assert!(
            placeholder_contrast >= 3.0,
            "Theme {:?} command line placeholder text ({:.2}:1) fails secondary text threshold (>=3.0:1)",
            theme,
            placeholder_contrast
        );

        // 3. Option Keyword Button: normal is weakest pair, hovered is primary.weak pair
        let opt_normal_contrast =
            wcag_contrast(p.background.weakest.text, p.background.weakest.color);
        assert!(
            opt_normal_contrast >= 4.5,
            "Theme {:?} option button normal text ({:.2}:1) fails WCAG AA",
            theme,
            opt_normal_contrast
        );

        let opt_hover_contrast = wcag_contrast(p.primary.weak.text, p.primary.weak.color);
        assert!(
            opt_hover_contrast >= 3.0,
            "Theme {:?} option button hover text ({:.2}:1) fails UI component threshold (>=3.0:1)",
            theme,
            opt_hover_contrast
        );

        // 4. Command History Entries:
        // EntryKind::Command -> background.base.text (normal text: >= 4.5:1)
        let cmd_contrast = wcag_contrast(p.background.base.text, cli_bg);
        assert!(
            cmd_contrast >= 4.5,
            "Theme {:?} history command text ({:.2}:1) fails WCAG AA",
            theme,
            cmd_contrast
        );

        // EntryKind::Output -> background.base.text at 0.72 alpha (secondary text: >= 3.0:1)
        let output_text = composite_over(p.background.base.text.scale_alpha(0.72), cli_bg);
        let output_contrast = wcag_contrast(output_text, cli_bg);
        assert!(
            output_contrast >= 3.0,
            "Theme {:?} history output text ({:.2}:1) fails secondary text threshold (>=3.0:1)",
            theme,
            output_contrast
        );

        // EntryKind::Info -> primary.base.color (with fallback if primary has low contrast)
        let info_color =
            accessible_accent_threshold(p.primary.base.color, cli_bg, p.background.base.text, 4.5);
        let info_contrast = wcag_contrast(info_color, cli_bg);

        // EntryKind::Error -> danger.base.color
        let error_contrast = wcag_contrast(p.danger.base.color, cli_bg);
        assert!(
            info_contrast >= 4.5,
            "Theme {:?} info text ({:.2}:1) fails WCAG AA normal text threshold (>=4.5:1)",
            theme,
            info_contrast
        );
        assert!(
            error_contrast >= 2.0,
            "Theme {:?} error text ({:.2}:1) fails non-text indicator threshold (>=2.0:1)",
            theme,
            error_contrast
        );
    }
}

#[test]
fn test_properties_and_dock_contrast() {
    let themes = selectable_themes();
    for theme in &themes {
        let p = theme.palette();
        let dock_bg = p.background.base.color;

        // 1. Property Field Labels & Values
        let label_contrast = wcag_contrast(p.background.base.text, dock_bg);
        assert!(
            label_contrast >= 4.5,
            "Theme {:?} properties label text ({:.2}:1) fails WCAG AA",
            theme,
            label_contrast
        );

        // 2. Muted Helper Text: muted_style uses background.base.text at 0.68 alpha
        let muted_text = composite_over(p.background.base.text.scale_alpha(0.68), dock_bg);
        let muted_contrast = wcag_contrast(muted_text, dock_bg);
        assert!(
            muted_contrast >= 3.0,
            "Theme {:?} properties muted text ({:.2}:1) fails secondary text threshold (>=3.0:1)",
            theme,
            muted_contrast
        );

        // 3. Section Header: rendered on background.weakest.color
        let header_bg = p.background.weakest.color;
        let header_text = p.background.base.text;
        let header_contrast = wcag_contrast(header_text, header_bg);
        assert!(
            header_contrast >= 4.0,
            "Theme {:?} properties header text ({:.2}:1) fails header contrast threshold (>=4.0:1)",
            theme,
            header_contrast
        );
    }
}

#[test]
fn test_modals_and_action_buttons_contrast() {
    let themes = selectable_themes();
    for theme in &themes {
        let p = theme.palette();
        let modal_bg = p.background.base.color;

        // 1. Modal Body Text (normal text: >= 4.5:1)
        let body_contrast = wcag_contrast(p.background.base.text, modal_bg);
        assert!(
            body_contrast >= 4.5,
            "Theme {:?} modal body text ({:.2}:1) fails WCAG AA",
            theme,
            body_contrast
        );

        // 2. Primary Action Button: UI button component (WCAG 1.4.11 >= 3.0:1)
        let primary_btn_contrast = wcag_contrast(p.primary.base.text, p.primary.base.color);
        assert!(
            primary_btn_contrast >= 3.0,
            "Theme {:?} Primary button text ({:.2}:1) fails button threshold (>=3.0:1)",
            theme,
            primary_btn_contrast
        );

        // 3. Danger Action Button: UI button component (WCAG 1.4.11 >= 3.0:1)
        let danger_btn_contrast = wcag_contrast(p.danger.base.text, p.danger.base.color);
        assert!(
            danger_btn_contrast >= 3.0,
            "Theme {:?} Danger button text ({:.2}:1) fails button threshold (>=3.0:1)",
            theme,
            danger_btn_contrast
        );

        // 4. Success Action Button: UI button component (WCAG 1.4.11 >= 3.0:1)
        let success_btn_contrast = wcag_contrast(p.success.base.text, p.success.base.color);
        assert!(
            success_btn_contrast >= 3.0,
            "Theme {:?} Success button text ({:.2}:1) fails button threshold (>=3.0:1)",
            theme,
            success_btn_contrast
        );

        // 5. Modal Close Button Hover: danger.strong pair (WCAG 1.4.11 >= 3.0:1)
        let close_hover_contrast = wcag_contrast(p.danger.strong.text, p.danger.strong.color);
        assert!(
            close_hover_contrast >= 3.0,
            "Theme {:?} close button hover text ({:.2}:1) fails button threshold (>=3.0:1)",
            theme,
            close_hover_contrast
        );
    }
}

/// Deleted 2026-10-16 in favour of `kit_roles_hold_aa_contrast` above. This one
/// walked the two raw aurora ends at the 3:1 UI-component threshold and checked
/// `Color::WHITE` by hand, which is precisely how a 3.18:1 hovered hero passed a
/// "primary button contrast" test for two release waves. The replacement measures
/// the plate `style_for` actually paints, at the 4.5:1 bar, with the ink the
/// plate is paired with.

#[test]
fn test_dropdowns_and_selection_overlays() {
    let themes = selectable_themes();
    for theme in &themes {
        let p = theme.palette();

        // 1. Visual Style Dropdown Popup: background is weak.color
        let dropdown_bg = p.background.weak.color;

        // Selected / hovered text uses strong.text
        let selected_contrast = wcag_contrast(p.background.strong.text, dropdown_bg);
        assert!(
            selected_contrast >= 4.5,
            "Theme {:?} visual style selected text ({:.2}:1) fails WCAG AA",
            theme,
            selected_contrast
        );

        // Inactive item text uses base.text
        let inactive_contrast = wcag_contrast(p.background.base.text, dropdown_bg);
        assert!(
            inactive_contrast >= 4.5,
            "Theme {:?} visual style inactive text ({:.2}:1) fails WCAG AA",
            theme,
            inactive_contrast
        );

        // 2. Active Checkmark with Dynamic Fallback
        // Dropdown panel is background.weak.color; highlighted row can be strong.color.
        let pri = p.primary.base.color;
        let bg_weak = dropdown_bg;
        let bg_strong = p.background.strong.color;
        let fallback = p.background.weak.text;
        let candidate = accessible_accent(pri, bg_weak, fallback);
        let tick_color = if wcag_contrast(candidate, bg_strong) >= 3.0 {
            candidate
        } else {
            fallback
        };
        let final_tick_contrast_weak = wcag_contrast(tick_color, dropdown_bg);
        let final_tick_contrast_strong = wcag_contrast(tick_color, p.background.strong.color);
        assert!(
            final_tick_contrast_weak >= 3.0,
            "Theme {:?} checkmark ({:.2}:1) against weak background fails UI threshold (>=3.0:1)",
            theme,
            final_tick_contrast_weak
        );
        assert!(
            final_tick_contrast_strong >= 3.0,
            "Theme {:?} checkmark ({:.2}:1) against strong background fails UI threshold (>=3.0:1)",
            theme,
            final_tick_contrast_strong
        );
    }
}

#[test]
fn test_viewport_controls_toggle_buttons_contrast() {
    let themes = selectable_themes();
    for theme in &themes {
        let p = theme.palette();

        // 1. Active Toggle Button (Grid & Snap):
        // Background is primary.weak.color, icon uses primary.weak.text
        let active_bg = p.primary.weak.color;
        let active_icon = p.primary.weak.text;
        let active_contrast = wcag_contrast(active_icon, active_bg);
        assert!(
            active_contrast >= 3.0,
            "Theme {:?} active viewport toggle button icon ({:.2}:1) fails UI threshold (>=3.0:1)",
            theme,
            active_contrast
        );

        // 2. Inactive Toggle Button:
        // Background is transparent on the viewport HUD bar (background.base.color)
        let inactive_icon = p.background.base.text;
        let inactive_contrast = wcag_contrast(inactive_icon, p.background.base.color);
        assert!(
            inactive_contrast >= 4.5,
            "Theme {:?} inactive viewport toggle button icon ({:.2}:1) fails WCAG AA (>=4.5:1)",
            theme,
            inactive_contrast
        );
    }
}

#[test]
fn test_block_palette_contrast() {
    use crate::ui::kit::{picker_tile_border, picker_tile_colors};
    // Both answers moved into L1 with the twenty-second screen, which drained the
    // palette's last hand-written `container::Style`. What this test holds now is
    // the single decision, not the palette's private copy of it — so a state pair
    // that fails AA fails the tile everywhere it is ever drawn.
    use iced::widget::button::Status as BtnStatus;

    let themes = selectable_themes();
    for theme in &themes {
        let p = theme.palette();

        // ── 1. Card label pairs resolve to the theme's own text colors ──
        // Regression guard for the light-theme bug where labels were hardcoded
        // to `Color::WHITE` (unreadable white-on-light). If anyone reintroduces
        // a hardcoded foreground, these equality checks fail on every theme
        // whose surface text is not pure white.
        let (normal_bg, normal_fg) = picker_tile_colors(theme, false, BtnStatus::Active);
        // Cards sit on the dock's base surface, so they take the weak pair
        // to read as cards.
        assert_eq!(
            normal_bg, p.background.weak.color,
            "Theme {:?} block card normal background drifted from weak pair",
            theme
        );
        assert_eq!(
            normal_fg, p.background.weak.text,
            "Theme {:?} block card normal label must use weak text (was hardcoded WHITE)",
            theme
        );

        let (_, hover_fg) = picker_tile_colors(theme, false, BtnStatus::Hovered);
        assert_eq!(
            hover_fg, p.background.strong.text,
            "Theme {:?} block card hover label must use strong text (was hardcoded WHITE)",
            theme
        );

        let (_, pressed_fg) = picker_tile_colors(theme, false, BtnStatus::Pressed);
        assert_eq!(
            pressed_fg, p.background.strong.text,
            "Theme {:?} block card pressed label must use strong text (was hardcoded WHITE)",
            theme
        );

        // Default-branch statuses (Active/Disabled/Focused/...) share the weak pair.
        let (_, disabled_fg) = picker_tile_colors(theme, false, BtnStatus::Disabled);
        assert_eq!(
            disabled_fg, p.background.weak.text,
            "Theme {:?} block card default-state label must use weak text",
            theme
        );

        // Placing (selected) cards always use the primary pair, regardless of
        // hover — the background does not switch on hover while placing.
        for status in [BtnStatus::Active, BtnStatus::Hovered, BtnStatus::Pressed] {
            let (bg, fg) = picker_tile_colors(theme, true, status);
            assert_eq!(
                bg, p.primary.base.color,
                "Theme {:?} placing card bg must be primary.base",
                theme
            );
            assert_eq!(
                fg, p.primary.base.text,
                "Theme {:?} placing card label must be primary.base.text",
                theme
            );
        }

        // ── 2. Card label contrast: normal / hover / pressed ──
        // 11px single-line labels are normal text → WCAG AA >= 4.5:1.
        let normal_contrast = wcag_contrast(normal_fg, normal_bg);
        assert!(
            normal_contrast >= 4.5,
            "Theme {:?} block card label ({:.2}:1) fails WCAG AA on card background",
            theme,
            normal_contrast
        );

        let (hover_bg, hover_fg) = picker_tile_colors(theme, false, BtnStatus::Hovered);
        let hover_contrast = wcag_contrast(hover_fg, hover_bg);
        assert!(
            hover_contrast >= 4.5,
            "Theme {:?} block card hovered label ({:.2}:1) fails WCAG AA on hovered background",
            theme,
            hover_contrast
        );

        let (pressed_bg, pressed_fg) = picker_tile_colors(theme, false, BtnStatus::Pressed);
        let pressed_contrast = wcag_contrast(pressed_fg, pressed_bg);
        assert!(
            pressed_contrast >= 4.5,
            "Theme {:?} block card pressed label ({:.2}:1) fails WCAG AA on pressed background",
            theme,
            pressed_contrast
        );

        // ── 3. Placing card contrast ──
        // Selected-card label on `primary.base`: UI button component threshold
        // (>= 3.0:1 per WCAG 1.4.11, matching the Primary-button test above).
        let (placing_bg, placing_fg) = picker_tile_colors(theme, true, BtnStatus::Active);
        let placing_contrast = wcag_contrast(placing_fg, placing_bg);
        assert!(
            placing_contrast >= 3.0,
            "Theme {:?} placing block card label ({:.2}:1) fails button threshold (>=3.0:1)",
            theme,
            placing_contrast
        );
        // Placing + hover must not silently drop contrast (bg is sticky).
        let (placing_hover_bg, placing_hover_fg) =
            picker_tile_colors(theme, true, BtnStatus::Hovered);
        let placing_hover_contrast = wcag_contrast(placing_hover_fg, placing_hover_bg);
        assert!(
            placing_hover_contrast >= 3.0,
            "Theme {:?} placing+hover block card label ({:.2}:1) fails button threshold (>=3.0:1)",
            theme,
            placing_hover_contrast
        );

        // ── 4. Card border is theme-driven (no invisible borders) ──
        assert_eq!(
            picker_tile_border(theme, false),
            crate::ui::style::tokens::edge(theme),
            "Theme {:?} block card border must use the shared edge token",
            theme
        );
        assert_eq!(
            picker_tile_border(theme, true),
            p.primary.base.color,
            "Theme {:?} placing block card border must use primary color",
            theme
        );

        // ── 5. Header icon buttons ──
        // Deleted 2026-10-15 with `block_icon_button_text_color`: the palette's
        // icon buttons are `kit::btn(.., Role::Ghost, ..)` now, and
        // `kit_roles_hold_aa_contrast` below measures the real stylesheet
        // instead of a second copy of its intent.

        // ── 6. Empty-state muted text ("No blocks in this drawing") ──
        // Rendered as `base.text` at 0.72 alpha over the dock `base`
        // background → secondary text threshold >= 3.0:1. (The old hardcoded
        // 0.55-gray failed this on light themes.)
        let empty_text = composite_over(
            p.background.base.text.scale_alpha(0.72),
            p.background.base.color,
        );
        let empty_contrast = wcag_contrast(empty_text, p.background.base.color);
        assert!(
            empty_contrast >= 3.0,
            "Theme {:?} block palette empty-state text ({:.2}:1) fails secondary text threshold (>=3.0:1)",
            theme, empty_contrast
        );

        // ── 7. Hardcoded-white regression probe ──
        // The exact reported bug: pure white labels on the light card
        // background. Every theme's card background must NOT be near-white
        // with white text — i.e. white-on-card must fail while the real
        // theme pair passes (proves the fix matters on light themes and the
        // test would catch a WHITE hardcode).
        let white_on_normal = wcag_contrast(Color::WHITE, normal_bg);
        if white_on_normal < 4.5 {
            assert!(
                normal_contrast >= 4.5,
                "Theme {:?} needs theme-aware labels: white-on-card is {:.2}:1 but theme pair is {:.2}:1",
                theme, white_on_normal, normal_contrast
            );
        }
    }
}

/// The refused-value field (`kit::error_field_style`) paints its text on the
/// palette's danger recess rather than on the neutral page, and that pairing
/// had never been measured. Both windows whose duplicate the primitive replaced
/// inked the value at `danger.base.text` over `danger.weak.color` — a pairing
/// inherited from when the same ink was painted on `danger.base`. Measured, that
/// is 80 failures out of 104 checks across the selectable themes.
///
/// So this guard calls the primitive and measures what it hands back, the way
/// [`kit_roles_hold_aa_contrast`] measures `style_for`: text at AA, the plate's
/// edge and icon at the 1.4.11 component bar, the value again on top of the
/// selection it lands in when the user drags across it, and the ring at the
/// width that makes it the focus affordance. Translucent inks are composited
/// onto the plate first, because that is where they land.
#[test]
fn the_error_field_reads_on_its_own_danger_recess() {
    use crate::ui::style::tokens;
    use iced::widget::text_input;

    let mut fails: Vec<String> = Vec::new();
    let mut checked = 0usize;
    for theme in selectable_themes() {
        for (name, status) in [
            ("rest", text_input::Status::Active),
            ("focus", text_input::Status::Focused { is_hovered: true }),
        ] {
            let style = crate::ui::kit::error_field_style(&theme, status);
            let plate = match style.background {
                iced::Background::Color(color) => color,
                other => {
                    let line = format!("{theme} {name}: the plate is not a solid color: {other:?}");
                    fails.push(line);
                    continue;
                }
            };

            let value = wcag_contrast(style.value, plate);
            checked += 1;
            if value < tokens::AA_INK_CONTRAST {
                fails.push(format!("{theme} {name} value ink: {value:.2}:1"));
            }

            let placeholder = wcag_contrast(composite_over(style.placeholder, plate), plate);
            checked += 1;
            if placeholder < tokens::AA_INK_CONTRAST {
                fails.push(format!("{theme} {name} placeholder: {placeholder:.2}:1"));
            }

            // Dragging a cursor through an invalid value puts the text on the
            // selection block, so the highlight has to keep carrying it.
            let on_selection =
                wcag_contrast(style.value, composite_over(style.selection, plate));
            checked += 1;
            if on_selection < tokens::AA_INK_CONTRAST {
                fails.push(format!("{theme} {name} text on selection: {on_selection:.2}:1"));
            }

            // The edge and the icon are parts of a component, not text: the
            // 1.4.11 bar, measured over the plate they enclose.
            let edge = wcag_contrast(composite_over(style.border.color, plate), plate);
            checked += 1;
            if edge < tokens::AA_COMPONENT_CONTRAST {
                fails.push(format!("{theme} {name} danger edge: {edge:.2}:1"));
            }

            let icon = wcag_contrast(composite_over(style.icon, plate), plate);
            checked += 1;
            if icon < tokens::AA_COMPONENT_CONTRAST {
                fails.push(format!("{theme} {name} status icon: {icon:.2}:1"));
            }

            // The ring is what says "this one has focus": at the hairline a
            // keyboard user cannot see it, and the corner has to be the one every
            // other field in the app already uses or a refused field reads as a
            // different control sitting next to the valid ones.
            let ring = if name == "focus" {
                tokens::EDGE_RING
            } else {
                tokens::EDGE_HAIRLINE
            };
            checked += 1;
            if style.border.width != ring {
                fails.push(format!(
                    "{theme} {name} ring is {} px, expected {ring} px",
                    style.border.width
                ));
            }
            let corners = [
                style.border.radius.top_left,
                style.border.radius.top_right,
                style.border.radius.bottom_right,
                style.border.radius.bottom_left,
            ];
            checked += 1;
            if corners.iter().any(|c| *c != tokens::RADIUS_FIELD) {
                fails.push(format!(
                    "{theme} {name} corner {corners:?} is not every other field's {}",
                    tokens::RADIUS_FIELD
                ));
            }

            println!(
                "error field {theme:?} {name}: value {value:.2} placeholder {placeholder:.2} edge {edge:.2} icon {icon:.2} on-selection {on_selection:.2}"
            );
        }
    }
    assert!(
        fails.is_empty(),
        "{} of {checked} error-field measurements fail on the danger recess:\n{}",
        fails.len(),
        fails.join("\n")
    );
}

/// Contrast of a field's value ink where it is actually painted.
///
/// The highlight goes on the plate first, then the glyphs go on the highlight,
/// because a text selection has no ink of its own — iced reuses the style's
/// `value` colour for the covered characters and does not adjust it. So a
/// highlight is only legal if the text still reads on top of it.
fn marked_text_contrast(value: Color, selection: Color, plate: Color) -> f32 {
    let highlight = composite_over(selection, plate);
    wcag_contrast(composite_over(value, highlight), highlight)
}

/// The neutral field family carries the same constraint the refused field had,
/// and it had never been measured: `field_style`, `read_only_style` and
/// `editor_style` each set `selection` to the theme's primary at a fixed 0.5
/// wash (the deleted `SELECTION_TINT`), which is a decision about the highlight's
/// own colour and says nothing about the text on top of it. `bare_field_style` is
/// not measured separately: it spreads `..field_style(theme, status)`, so its
/// value and highlight are the same two colours and only its plate moves to
/// whatever the caller paints.
///
/// This guard measures one number per theme per entry point and names it. It is
/// written before the fix, so the failure list is the measurement report — the
/// first run on 2026-10-16 came out 112 of 156 under AA, and every selection
/// then went through [`crate::ui::style::tokens::backdrop_for`]. Note that
/// `composite_over` treats a translucent plate as a colour, not as a layer over
/// the page: after the fix every input here is opaque (a translucent ink or
/// highlight would be resolved by L1 first), so the plate term drops out of the
/// arithmetic and the limitation is inert.
///
/// What the `QC` line it prints is for is the other axis: a band has to be
/// *seen* as well as carried. The run printed four pale schemes where the solved
/// band lands within 12 L\* of its recess (Kanagawa Lotus 6.9, Solarized Light
/// 7.7, Tokyo Night Light 11.5, Catppuccin Latte 11.8) where the deleted 0.5 wash
/// sat 17.9–23.7 L\* away. That is not the solver failing; it is the theme's own
/// ink, and the printed numbers say so: the band is at its forced optimum, the
/// floor the ink imposes. Lotus marks ink L\* 36.3 and prints band L\* 83.4,
/// which is exactly `4.8 × (Y(ink) + 0.05) − 0.05` expressed in L\*; the same
/// holds for Solarized Light (ink 38.8, floor 86.9, printed band 86.7), Tokyo
/// Night Light (26.8, 71.8, 71.7) and Catppuccin Latte (34.3, 80.8, 80.8). Their
/// recesses are already at 90.3 / 94.5 / 83.2 / 92.6, so the whole band has to
/// live inside that last few points of lightness — and the darker direction is
/// arithmetically shut on all four, since a band under the ink would need
/// `(Y(ink) + 0.05) / 4.8 − 0.05` to stay non-negative and the four score
/// −0.0205, −0.0176, −0.0290 and −0.0226. A theme with near-black ink gets a
/// strong band for free (iced `Light`: ink L\* 0, band 46.8 L\* below its recess),
/// so the lever here is the seed palette's ink rung, not this function.
#[test]
fn the_marked_text_survives_its_own_highlight() {
    use crate::ui::style::tokens;
    use iced::widget::{text_editor, text_input};
    use iced::Background;

    let mut fails: Vec<String> = Vec::new();
    let mut checked = 0usize;

    let solid = |b: Background| match b {
        Background::Color(color) => Some(color),
        _ => None,
    };

    for theme in selectable_themes() {
        // Quality control, not an assertion: carrying the AA promise moved the
        // highlight off the accent, and the question a screenshot has to answer
        // is whether the band still reads as a band. So dump the actual pixels:
        // the solved band, the one the deleted 0.5 wash used to paint, the
        // recess under them, the ink on top, plus the two separations that
        // matter — CIE L* apart from the recess (shape visibility at the dark
        // end, where a WCAG ratio of 1.5:1 can still be an obvious edge) and the
        // channel spread, which is how much hue survived the pull.
        {
            use crate::ui::style::common::lstar;
            let label = {
                let d = format!("{theme:?}");
                match d.find("name: \"") {
                    Some(i) => {
                        let rest = &d[i + 7..];
                        rest.split('"').next().unwrap_or("custom").to_string()
                    }
                    None => d,
                }
            };
            let palette = theme.palette();
            let recess = tokens::inset(&theme);
            let style = crate::ui::kit::field_style(&theme, text_input::Status::Active);
            let hex = |c: Color| {
                format!(
                    "{:02X}{:02X}{:02X}",
                    (c.r.clamp(0.0, 1.0) * 255.0).round() as u8,
                    (c.g.clamp(0.0, 1.0) * 255.0).round() as u8,
                    (c.b.clamp(0.0, 1.0) * 255.0).round() as u8
                )
            };
            let spread = |c: Color| {
                let hi = c.r.max(c.g).max(c.b);
                let lo = c.r.min(c.g).min(c.b);
                hi - lo
            };
            let band = style.selection;
            let old = composite_over(
                palette.primary.base.color.scale_alpha(0.5),
                recess,
            );
            println!(
                "QC {label}|band={}|was={}|recess={}|ink={}|accent={}|dBandRecessL*={:.1}|dOldRecessL*={:.1}|spreadBand={:.3}|spreadRecess={:.3}|spreadAccent={:.3}|accentL*={:.1}|bandL*={:.1}|recessL*={:.1}|inkL*={:.1}",
                hex(band),
                hex(old),
                hex(recess),
                hex(style.value),
                hex(palette.primary.base.color),
                lstar(band) - lstar(recess),
                lstar(old) - lstar(recess),
                spread(band),
                spread(recess),
                spread(palette.primary.base.color),
                lstar(palette.primary.base.color),
                lstar(band),
                lstar(recess),
                lstar(style.value),
            );
        }

        for (name, status) in [
            ("field rest", text_input::Status::Active),
            ("field hover", text_input::Status::Hovered),
            ("field focus", text_input::Status::Focused { is_hovered: true }),
        ] {
            let style = crate::ui::kit::field_style(&theme, status);
            let plate = solid(style.background).expect("field_style paints a solid recess");
            checked += 1;
            let c = marked_text_contrast(style.value, style.selection, plate);
            println!("{theme:?} {name}: marked value {c:.2}:1");
            if c < tokens::AA_INK_CONTRAST {
                fails.push(format!("{theme} {name}: {c:.2}:1"));
            }
        }

        let ro = crate::ui::kit::read_only_style(&theme, text_input::Status::Active);
        let ro_plate = solid(ro.background).expect("read_only_style paints a solid recess");
        checked += 1;
        let c = marked_text_contrast(ro.value, ro.selection, ro_plate);
        println!("{theme:?} read-only: marked value {c:.2}:1");
        if c < tokens::AA_INK_CONTRAST {
            fails.push(format!("{theme} read-only: {c:.2}:1"));
        }

        for (name, status) in [
            ("editor rest", text_editor::Status::Active),
            ("editor disabled", text_editor::Status::Disabled),
        ] {
            let ed = crate::ui::kit::editor_style(&theme, status);
            let ed_plate = solid(ed.background).expect("editor_style paints a solid recess");
            checked += 1;
            let c = marked_text_contrast(ed.value, ed.selection, ed_plate);
            println!("{theme:?} {name}: marked value {c:.2}:1");
            if c < tokens::AA_INK_CONTRAST {
                fails.push(format!("{theme} {name}: {c:.2}:1"));
            }
        }
    }

    assert!(
        fails.is_empty(),
        "{} of {checked} marked-text measurements fail: the selection paints over \
         the value instead of under it:\n{}",
        fails.len(),
        fails.join("\n")
    );
}

/// A verdict word used to be painted with a *plate* colour, so it could only be
/// judged on the plane it lands on. This is the guard that judges it.
///
/// [`crate::ui::kit::label::tone_style`] used to hand back
/// `tone_pair(theme, tone).color`, which is `palette.<role>.base.color` — the
/// swatch's own fill, the colour a chip is painted — to `text::Style.color`, and
/// 17 screens called it (`grep -rn "tone_style(" src` on 2026-10-16). Its doc
/// cited `draftflow_semantic_colors_read_as_text` as the guarantee that a tone
/// can be body text, and that test is not the guarantee the sentence claimed: it
/// covers three of the four tones (`Accent` is missing), against one plane
/// (`background.base`), for the two DraftFlow themes (`draftflow_themes()`), at
/// `AA_COMPONENT_CONTRAST` — 3.0, the bar for a control's edge, not the 4.5 an
/// ink has to clear. Measured over the planes L1 actually hands a call site, 333
/// of 728 pairs fell under 4.5:1, the worst being iced's own `Dark` success green
/// at 1.09:1 on `Float`.
///
/// `tone_style` now solves the ink against [`crate::ui::kit::label::kit_planes`],
/// so this guard measures the pairing as shipped: the colour that goes into the
/// `text::Style`, over every plane in the same list the solver reads. Two extra
/// laws ride along, because both are things a solver can get wrong silently:
///
/// - a tone already legible everywhere must come back **untouched** — an ink that
///   passes must not be pulled "for safety", or every theme's verdict colours
///   drift one release later than the reason;
/// - a word written on a plane outside the ladder (the document canvas, whose
///   colour is the drawing's) must clear AA on *that* plane, which is
///   [`crate::ui::kit::label::tone_style_on`]'s whole job.
///
/// Coverage claim, stated exactly: `kit_planes` is the *flat* vocabulary, and two
/// of the 17 call sites write a verdict word on an engaged plate — the active
/// polar pill (`statusbar/mod.rs:911`) and an engaged Yes/No button
/// (`properties.rs:1569`), both on `tokens::aurora_gradient`, which has two stops
/// and no single resting pixel. Their ink is solved against the ladder, so it is
/// no worse than the plate colour it replaced, but a gradient is not measured by
/// this guard and nothing else measures it either.
///
/// The `QC3` line prints, per theme and tone, the worst plane before the solve,
/// the worst after, and the two colours — the hexes are there so a report can be
/// checked against a pixel dump instead of re-run.
#[test]
fn the_verdict_word_reads_on_the_plane_it_is_given() {
    use crate::ui::kit::label::{kit_planes, tone_pair, tone_style, tone_style_on, Tone};
    use crate::ui::style::tokens;

    let tones = [Tone::Accent, Tone::Success, Tone::Warning, Tone::Danger];
    // The canvas is user data: a DWG can set any background, so these are the
    // ends a drawing is actually shipped with — pure black, the DraftFlow paper,
    // a stock ACAD model space, and white. No plate in the middle of that range
    // is solvable by any ink (see `tokens::legible_on` and the dead band), and
    // the print column below is what would name one if a drawing ever asked.
    let canvases = [
        ("black", Color::from_rgb8(0x00, 0x00, 0x00)),
        ("acad", Color::from_rgb8(0x1E, 0x1E, 0x1E)),
        ("paper", Color::from_rgb8(0xF7, 0xF9, 0xFB)),
        ("white", Color::from_rgb8(0xFF, 0xFF, 0xFF)),
    ];

    let worst_over = |color: Color, planes: &[Color]| -> (f32, usize) {
        let mut worst = (f32::INFINITY, 0usize);
        for (i, plate) in planes.iter().enumerate() {
            let c = wcag_contrast(composite_over(color, *plate), *plate);
            if c < worst.0 {
                worst = (c, i);
            }
        }
        worst
    };

    let mut fails: Vec<String> = Vec::new();
    let mut checked = 0usize;

    for theme in selectable_themes() {
        let planes = kit_planes(&theme);
        for tone in tones {
            let raw = tone_pair(&theme, tone).color;
            let ink = tone_style(&theme, tone)
                .color
                .expect("tone_style always paints an ink");

            let (before, _) = worst_over(raw, &planes);
            let (after, at) = worst_over(ink, &planes);
            checked += planes.len();
            for plate in &planes {
                let c = wcag_contrast(composite_over(ink, *plate), *plate);
                if c < tokens::AA_INK_CONTRAST {
                    fails.push(format!(
                        "{theme} {tone:?} solved ink={} on {}: {c:.2}:1",
                        hex_of(ink),
                        hex_of(*plate)
                    ));
                }
            }

            // The palette's own answer must survive a pass, unchanged, rung and all.
            if before >= tokens::AA_INK_CONTRAST && ink != raw {
                fails.push(format!(
                    "{theme} {tone:?} was already legible at {before:.2}:1 and got moved \
                     from {} to {}",
                    hex_of(raw),
                    hex_of(ink)
                ));
            }

            for (name, canvas) in canvases {
                let on_canvas = tone_style_on(&theme, tone, canvas)
                    .color
                    .expect("tone_style_on always paints an ink");
                let c = wcag_contrast(composite_over(on_canvas, canvas), canvas);
                checked += 1;
                if c < tokens::AA_INK_CONTRAST {
                    fails.push(format!(
                        "{theme} {tone:?} on canvas {name} {}: {c:.2}:1",
                        hex_of(canvas)
                    ));
                }
            }

            println!(
                "QC3 {theme}|{tone:?}|before={before:.2}:1|after={after:.2}:1 on plane {at}|worst-plate={}|ink={}->{}",
                hex_of(planes[at]),
                hex_of(raw),
                hex_of(ink)
            );
        }
    }

    assert!(
        fails.is_empty(),
        "{} of {checked} verdict-word measurements fail: a verdict word is \
         painted with the colour a chip would be filled with, on planes nobody \
         measured:\n{}",
        fails.len(),
        fails.join("\n")
    );
}

/// A verdict **band** carries its own ground, so its words are measured on that
/// ground and not on whatever tier the screen dropped it into.
///
/// Nothing measured this. `kit_roles_hold_aa_contrast` walks
/// [`crate::ui::kit::style_for`], and a tone plate is not one of the `Role`s that
/// table answers; `the_error_field_reads_on_its_own_danger_recess` measures an
/// input's recess, not a block of prose. Meanwhile four dialogs spelled the band
/// and disagreed about it: `attdef_dialog.rs`, `block_definition.rs` and
/// `wblock.rs` each mixed `danger.base` at 0.18 over their own background (two
/// drawing an edge, one none, two overriding their text with
/// `background.strongest.text`), while `tone_plate_style` filled itself with
/// `danger.weak` and inked its words with `weak.text`. Four verdict bands, zero
/// measurements.
///
/// The first version of this guard solved the band's ink against its wash
/// composited over every tier — 22 plates — and it failed 173 of 2288 times. The
/// failure was not an ink that needed pulling; it was a plate set **no** ink can
/// serve: `success.base` at 0.18 over Dracula's tiers spans `Y` 0.142–0.242, and
/// clearing 4.5:1 at the pale end takes an ink to `Y ≥ 1.26` while the deep end
/// takes it to `Y ≤ −0.007`. The solver's answer was to hand back the chip colour
/// untouched at 3.27:1, which is the correct answer to an impossible question and
/// the wrong thing to ship. So the band paints an opaque ground instead, and this
/// guard asserts the two laws that make it legible anywhere: the ground is opaque,
/// and its ink clears AA on it.
///
/// The band's *edge* is asserted here too, and that closes the sweep this guard
/// used to declare out of scope. It is a WCAG 1.4.11 question about a non-text
/// part, so it is measured on `tokens::AA_COMPONENT_CONTRAST` and not on the
/// words' 4.5.
///
/// Two wrong questions are on the record. The pre-fix painter left an untouched
/// `swatch.base` edge under 3:1 against the tier beside it on 128 of 728 band ×
/// tier pairs, worst Kanagawa Dragon Accent on Card at 1.00:1 — a hairline that was
/// the same pixel as its neighbour. The first fix then asked one placement-free
/// hairline to clear the band's own ground *and* every plane the kit can drop it
/// on. That is a conjunction, and this guard is what proved no single pixel serves
/// it: **158 of the 2392 readings** that sweep takes failed with neither cue. iced
/// Dark's Success band on `#545556` filled 1.10:1 with a 2.99:1 hairline; Nord's
/// Accent band on `#4E5159` filled 2.73:1 with a 1.21:1 one. A hairline that is 3:1
/// from its own plate can still vanish into a tier a few steps off that plate, and
/// on 106 of the plate/plane pairs behind those readings the exact bound says no
/// colour in any colourspace could have cleared both at once
/// ([`a_placement_free_edge_is_provably_impossible`]).
///
/// So the plane the band is dropped on entered the signature
/// ([`crate::ui::kit::surface::tone_plate_style_on`]) and the law asserted below is
/// answered one ground at a time: on every plane in [`kit_planes`], plus the dialog
/// body's own control rung, either the wash separates from the plane or the
/// hairline does. Both failing on the same plane is a failure. The band's own plate
/// is *not* one of those planes — a hairline outlines a plate, it is never dropped
/// onto it — which is why the old "edge ≥3 against its own ground" limb is deleted
/// from this guard rather than left in weakened: it was never a boundary anyone had
/// to see, and keeping it is what made the sweep unsolvable.
#[test]
fn the_verdict_band_feeds_its_own_words() {
    use crate::ui::kit::label::{kit_planes, tone_pair, tone_swatch, Tone};
    use crate::ui::kit::surface::{dialog_body_plate, tone_band_plate, tone_plate_style_on};
    use crate::ui::style::tokens;

    let tones = [Tone::Accent, Tone::Success, Tone::Warning, Tone::Danger];
    let mut fails: Vec<String> = Vec::new();
    let mut checked = 0usize;
    let mut bands = 0usize;
    let mut hue_kept = 0usize;
    let themes = selectable_themes();
    let theme_count = themes.len();
    // Owed before the sweep: one reading per band per ground, the grounds being
    // that theme's plane vocabulary plus the dialog body rung. Accumulated outside
    // the loop so a ground that silently stopped being swept shows up short.
    let expected: usize = themes
        .iter()
        .map(|theme| (kit_planes(theme).len() + 1) * tones.len())
        .sum();

    for theme in themes {
        let mut grounds = kit_planes(&theme);
        // Not a tier, and not in `kit_planes`: every modal band in the app is
        // pushed into a dialog body, which paints a control rung.
        grounds.push(dialog_body_plate(&theme));

        for tone in tones {
            let plate = tone_band_plate(&theme, tone);
            let chip = tone_pair(&theme, tone).color;
            let swatch = tone_swatch(&theme, tone);
            let reference = tone_plate_style_on(&theme, tone, grounds[0], "");
            let ink = reference
                .text_color
                .unwrap_or_else(|| panic!("{theme} {tone:?} verdict band gives its words no ink"));
            let painted = match reference.background {
                Some(iced::Background::Color(c)) => c,
                other => panic!("{theme} {tone:?} verdict band paints no ground: {other:?}"),
            };
            bands += 1;
            // The law the solve depends on, checked where it could break: the
            // ground the style paints and the plate the ink was solved for are one
            // colour, so no tier is in either measurement.
            if painted != plate {
                fails.push(format!(
                    "{theme} {tone:?} band paints {} but inked for {}",
                    hex_of(painted),
                    hex_of(plate)
                ));
            }

            let over = |c: Color| wcag_contrast(composite_over(c, plate), plate);
            let solved = over(ink);
            let mut kept_hue = 0usize;

            if solved < tokens::AA_INK_CONTRAST {
                fails.push(format!(
                    "{theme} {tone:?} band ink {} on its own ground {}: {solved:.2}:1",
                    hex_of(ink),
                    hex_of(plate)
                ));
            }
            // An ink moved for no reason is a hue the theme author chose and a
            // measurement nobody asked for.
            if over(chip) >= tokens::AA_INK_CONTRAST && ink != chip {
                fails.push(format!(
                    "{theme} {tone:?} band was readable at {:.2}:1 and its ink was moved \
                     from {} to {}",
                    over(chip),
                    hex_of(chip),
                    hex_of(ink)
                ));
            }

            // ── the boundary law (WCAG 1.4.11, `AA_COMPONENT_CONTRAST`) ──────
            // One ground at a time, which is the only form of this law that has an
            // answer. The plate and the words must not move with it — a band whose
            // own colour depended on the tier under it would hand the depth back to
            // the screens and this guard would be measuring 23 different plates.
            for plane in &grounds {
                let style = tone_plate_style_on(&theme, tone, *plane, "");
                let edge = style.border.color;
                checked += 1;

                let ground = match style.background {
                    Some(iced::Background::Color(c)) => c,
                    other => panic!("{theme} {tone:?} band on {} paints no ground: {other:?}", hex_of(*plane)),
                };
                if ground != plate {
                    fails.push(format!(
                        "{theme} {tone:?} band on {} is a wash: its ground moved from {} to {}",
                        hex_of(*plane),
                        hex_of(plate),
                        hex_of(ground)
                    ));
                }
                if style.text_color != Some(ink) {
                    fails.push(format!(
                        "{theme} {tone:?} band on {} re-solved its own words for the plane \
                         underneath: {} became {:?}",
                        hex_of(*plane),
                        hex_of(ink),
                        style.text_color.map(hex_of),
                    ));
                }

                let fill_cue = wcag_contrast(plate, *plane);
                let edge_cue = wcag_contrast(composite_over(edge, *plane), *plane);
                if edge == swatch.base.color {
                    kept_hue += 1;
                }
                if fill_cue < tokens::AA_COMPONENT_CONTRAST
                    && edge_cue < tokens::AA_COMPONENT_CONTRAST
                {
                    fails.push(format!(
                        "{theme} {tone:?} band invisible on {}: fill {fill_cue:.2}:1 and \
                         edge {} {edge_cue:.2}:1",
                        hex_of(*plane),
                        hex_of(edge)
                    ));
                }
                // The economy rule, per ground: the theme author's hue is left
                // alone wherever *this* plane already reads by its fill. A hue
                // moved for a boundary that was not missing here is a colour lost.
                if fill_cue >= tokens::AA_COMPONENT_CONTRAST && edge != swatch.base.color {
                    fails.push(format!(
                        "{theme} {tone:?} band edge {} already carried this plane at \
                         {fill_cue:.2}:1 and got moved to {}",
                        hex_of(swatch.base.color),
                        hex_of(edge)
                    ));
                }

                println!(
                    "QC4 {theme}|{tone:?}|plate {} (Y {:.3})|on {}|ink {} scores {solved:.2}:1 \
                     | chip {} scored {:.2}:1 | edge {} fills {fill_cue:.2} edges {edge_cue:.2}",
                    hex_of(plate),
                    crate::ui::style::common::wcag_luminance(plate),
                    hex_of(*plane),
                    hex_of(ink),
                    hex_of(chip),
                    over(chip),
                    hex_of(edge),
                );
            }
            hue_kept += kept_hue;
        }
    }

    assert!(
        fails.is_empty(),
        "{} of {checked} verdict-band measurements fail: a band that carries its \
         own ground was inked without measuring it:\n{}",
        fails.len(),
        fails.join("\n")
    );
    assert_eq!(checked, expected);
    assert_eq!(bands, theme_count * tones.len());
    // Printed, not asserted: how often the plane already read and the theme's own
    // hue shipped untouched. The economy rule above makes every *moved* hue a
    // answered question, and this line is what shows the per-ground solve stopped
    // dragging colours that nothing asked to be dragged.
    println!("QC4 checked {checked} band/plane pairings over {bands} bands, {hue_kept} kept the theme hue");
}

/// The field family's quiet inks, measured for the first time.
///
/// Every previous guard in this file asks about the **value** — the word a field
/// carries once there is something in it. Task #69 solved the value and the
/// selection and stopped there, and #70 solved the words a `tone_style` paints.
/// What nobody measured was the ink a control uses when it is *not* carrying a
/// value: `placeholder` (the hint that stands in for an empty field), `icon` (a
/// trailing clear/search glyph), and a dropdown's `placeholder_color` +
/// `handle_color`. Those four primitives spelled them as `background.base.text`
/// times a private alpha — `INK_PLACEHOLDER` 0.62, `INK_MUTED` 0.68, `INK_FAINT`
/// 0.48 — and a disabled editor multiplied a second alpha on top
/// (`INK_PLACEHOLDER * off` = 0.31), which is precisely the double-translucency
/// class #69 had to fix for values.
///
/// The rule is measured where it lands, with no exception carved for a
/// translucent plate: the style's own plate is composited onto every plane in
/// [`crate::ui::kit::label::kit_planes`] (the same 22 the #70 solver solves
/// against), then the ink onto that, and the **worst** ground rules. An opaque
/// recess makes that a no-op — `tokens::over` returns the foreground at a=1 —
/// so `field_style` is measured against exactly the one pixel it paints, while
/// `bare_field_style` (a plate the wrapped container supplies) and the disabled
/// editor (recess at 0.5) get the whole ladder, which is the honest reading of
/// "a field can be dropped anywhere".
///
/// Two floors, because two kinds of thing are painted:
/// - **Placeholder is text.** A hint that stands in for an empty value is read,
///   not glanced at, so it carries [`tokens::AA_INK_CONTRAST`] (4.5:1) — the
///   same bar as a value.
/// - **An icon or a handle is a graphic.** WCAG 1.4.11 asks 3:1 of a user
///   interface *component*, so [`tokens::AA_COMPONENT_CONTRAST`].
///
/// `Disabled` keeps the WCAG 1.4.3 exemption for its **text** (an inactive
/// control is allowed to recede; that is the point of it) but is held to the
/// 3:1 component floor here, so "unavailable" never degrades into "invisible".
///
/// Red run, 2026-10-16, before the L1 fix: **136 of 338** measurements failed
/// (`D:\tmp\s74c-red.log`), and the shape of the failure is the double alpha the
/// note above describes, per entry —
/// `editor_disabled` placeholder 26/26, worst 1.53:1 (Solarized Light);
/// `read_only` placeholder 25/26 at 2.07:1 (Kanagawa Lotus);
/// `bare_field` placeholder 25/26 at 2.04:1 (Tokyo Night);
/// `bare_field` icon 16/26 at 2.18:1 (Tokyo Night);
/// `field` / `editor` / `pick_list` placeholder 10/26 each at 2.66:1 and
/// `pick_list_disabled` placeholder 4/26 at the same 2.66:1 (Kanagawa Lotus);
/// `pick_list_disabled` text 8/26 at 2.14:1 (Kanagawa Lotus);
/// and three singletons at the component bar (`field` icon, `pick_list` handle,
/// `pick_list_disabled` handle, all 2.98:1 on Kanagawa Lotus). One failure is
/// invisible in that list of printed numbers: Kanagawa Wave's bare-field icon
/// sits just under 3.0:1 and prints `3.00`, so a grep of the log reads 135. The
/// guard compares the unrounded value.
///
/// What the fix is allowed to touch: a style's `placeholder`, `icon`, `handle`
/// and, for `bare_field_style` only, its `value` — because a bare field is the
/// one entry whose plate belongs to the container rather than to itself, so it
/// gets the same plane-set sweep as its hint. `pick_list`'s enabled value was
/// already the palette's ink at 5.79:1 worst and is left alone, which is the
/// point of measuring first: the run above says which inks were faded past the
/// bar and which were only *assumed* to be.
///
/// The guard also caught its own fix being wrong, and that is worth keeping in
/// the record. The first attempt solved the disabled editor's placeholder against
/// the style's `recess`, which is that sunken inset composited onto the *page* —
/// one ground, while the control paints a translucent plate and can be dropped on
/// twenty-two. The re-run took `editor_disabled placeholder` from 26 failures to
/// 25 and left every one of them under 3:1 on a plane the solver had never seen
/// (1.96:1 worst). [`crate::ui::kit::label::grounds`] folds the plate through the
/// plane set before solving, which is what this test measures, and the remaining
/// single failure (Kanagawa Wave's bare-field icon, clearing both extremes of the
/// ladder and missing on an interior tier) is why
/// [`crate::ui::kit::label::resolve_on`] re-solves against whichever plane is
/// worst instead of trusting its two endpoints.
#[test]
fn the_quiet_ink_of_a_field_still_reads() {
    use crate::ui::kit::control::{
        bare_field_style, editor_style, field_style, pick_list_style, read_only_style,
    };
    use crate::ui::kit::label::kit_planes;
    use crate::ui::style::tokens;
    use iced::widget::{pick_list, text_editor, text_input};
    use iced::Background;

    let mut fails: Vec<String> = Vec::new();
    let mut checked = 0usize;

    // Worst-case contrast of one ink over one plate, across every plane the
    // plate can be dropped on. Returns `(worst score, the plane that produced
    // it)` so a failure names its own ground. (Plain comments, not `///`: a doc
    // comment on a `let` of a closure is an unused doc comment, and rustc says
    // so on every build.)
    let worst_over_planes = |ink: Color, plate: Color, planes: &[Color]| -> (f32, Color) {
        let mut worst = (f32::INFINITY, planes[0]);
        for plane in planes {
            let ground = composite_over(plate, *plane);
            let c = wcag_contrast(composite_over(ink, ground), ground);
            if c < worst.0 {
                worst = (c, ground);
            }
        }
        worst
    };

    for theme in selectable_themes() {
        let planes = kit_planes(&theme);
        let name = format!("{theme}");

        let push = |label: &'static str,
                    role: &'static str,
                    ink: Color,
                    plate: Color,
                    floor: f32,
                    fails: &mut Vec<String>,
                    checked: &mut usize,
                    name: &str,
                    planes: &[Color]| {
            let (score, ground) = worst_over_planes(ink, plate, planes);
            *checked += 1;
            if score < floor {
                fails.push(format!(
                    "{name} {label} {role}: {} on {} = {score:.2}:1 (floor {floor:.1}:1)",
                    hex_of(composite_over(ink, ground)),
                    hex_of(ground)
                ));
            }
            println!(
                "QC5 {name}|{label}|{role}|ink {}|plate {}|on {} = {score:.2}:1 (floor {floor:.1})",
                hex_of(ink),
                hex_of(plate),
                hex_of(ground)
            );
        };

        let inputs = [
            (
                "field",
                field_style(&theme, text_input::Status::Active),
                tokens::AA_INK_CONTRAST,
            ),
            (
                "read_only",
                read_only_style(&theme, text_input::Status::Active),
                tokens::AA_INK_CONTRAST,
            ),
            (
                "bare_field",
                bare_field_style(&theme, text_input::Status::Active),
                tokens::AA_INK_CONTRAST,
            ),
        ];
        for (label, style, placeholder_floor) in inputs {
            let Background::Color(plate) = style.background else {
                panic!("{name} field primitive {label} stopped painting a flat plate");
            };
            push(
                label,
                "placeholder",
                style.placeholder,
                plate,
                placeholder_floor,
                &mut fails,
                &mut checked,
                &name,
                &planes,
            );
            // A trailing glyph, so the component floor. `read_only_style`
            // deliberately draws no icon (`Color::TRANSPARENT`), and a
            // transparent ink composites to its ground: measuring it would
            // report 1.00:1 as a defect when the design fact is *nothing is
            // painted there*. Absence is not a contrast failure.
            if style.icon.a > 0.0 {
                push(
                    label,
                    "icon",
                    style.icon,
                    plate,
                    tokens::AA_COMPONENT_CONTRAST,
                    &mut fails,
                    &mut checked,
                    &name,
                    &planes,
                );
            }
            // A bare field's plate is the container's, so its *value* is exposed
            // to the whole plane ladder exactly like its hint is. The two styles
            // that paint their own recess had their value settled by #69; this
            // one never had, and an ink solved for a plate that is not painted
            // here is not solved at all.
            if label == "bare_field" {
                push(
                    label,
                    "value",
                    style.value,
                    plate,
                    tokens::AA_INK_CONTRAST,
                    &mut fails,
                    &mut checked,
                    &name,
                    &planes,
                );
            }
        }

        let editors = [
            (
                "editor",
                editor_style(&theme, text_editor::Status::Active),
                tokens::AA_INK_CONTRAST,
            ),
            (
                "editor_disabled",
                editor_style(&theme, text_editor::Status::Disabled),
                tokens::AA_COMPONENT_CONTRAST,
            ),
        ];
        for (label, style, floor) in editors {
            let Background::Color(plate) = style.background else {
                panic!("{name} {label} stopped painting a flat plate");
            };
            push(
                label,
                "placeholder",
                style.placeholder,
                plate,
                floor,
                &mut fails,
                &mut checked,
                &name,
                &planes,
            );
        }

        let lists = [
            (
                "pick_list",
                pick_list_style(&theme, pick_list::Status::Active),
                tokens::AA_INK_CONTRAST,
            ),
            (
                "pick_list_disabled",
                pick_list_style(&theme, pick_list::Status::Disabled),
                tokens::AA_COMPONENT_CONTRAST,
            ),
        ];
        for (label, style, text_floor) in lists {
            let Background::Color(plate) = style.background else {
                panic!("{name} {label} stopped painting a flat plate");
            };
            push(
                label,
                "placeholder",
                style.placeholder_color,
                plate,
                text_floor,
                &mut fails,
                &mut checked,
                &name,
                &planes,
            );
            push(
                label,
                "handle",
                style.handle_color,
                plate,
                tokens::AA_COMPONENT_CONTRAST,
                &mut fails,
                &mut checked,
                &name,
                &planes,
            );
            push(
                label,
                "text",
                style.text_color,
                plate,
                text_floor,
                &mut fails,
                &mut checked,
                &name,
                &planes,
            );
        }
    }

    assert!(
        fails.is_empty(),
        "{} of {checked} quiet-ink measurements fail: a field's placeholder, icon \
         or handle was faded by an alpha that was never resolved against the plane \
         it lands on:\n{}",
        fails.len(),
        fails.join("\n")
    );
}

/// Every voice the type ladder paints, measured for the first time.
///
/// #74 covered the ink a *control* uses when it is empty. This covers the ink a
/// *row* uses when it is secondary: the rungs of
/// [`crate::ui::kit::label::VOICE_RUNGS`], which is the list the builders in
/// that module actually paint with, plus the two style-function forms of it.
/// Before #75 these were `background.base.text` multiplied by an alpha and
/// handed straight to iced, unsolved.
///
/// The exposure is not marginal, which is why it is its own guard instead of one
/// more entry in the test above. Measured on 2026-10-16 with grep over `src/`:
/// [`crate::ui::kit::label::muted_style`] appears on **207** lines (37 of them
/// written as the full `crate::ui::kit::muted_style` path, and
/// `common::muted_style` was a two-line forwarder onto the kit's until the
/// 36th freeze deleted it, so every one of
/// them lands here), [`crate::ui::kit::label::faint_style`] on **5**. A property
/// panel's labels, a ribbon's captions and a status line's counts are painted by
/// these two functions, and the `muted` / `caption` / `faint` / `group` / `key`
/// builders repeat the same arithmetic in widget form.
///
/// The red state this opened with, measured over the 26 selectable themes and the
/// 22 kit planes: `faint` failed **26 of 26** themes (worst 1.75:1 Tokyo Night,
/// best 3.32:1 DraftFlow Light — not one theme cleared it), `muted` failed
/// **24 of 26** (worst 2.18:1 Tokyo Night). That is 50 of 52 measurements, and
/// the two that passed were the two lightest palettes in the app.
///
/// All of it is prose, so all of it owes [`tokens::AA_INK_CONTRAST`] (4.5:1) —
/// WCAG 1.4.3 has no quieter bar for a secondary word, only for a large one, and
/// these are `TEXT_BODY` and `TEXT_HEADING` sizes. The plate is the plane they
/// are written on: a style function paints no ground, so the sweep runs over
/// [`crate::ui::kit::label::kit_planes`] exactly as `tone_style`'s solver does.
#[test]
fn the_quiet_voice_of_a_row_still_reads() {
    use crate::ui::kit::label::{faint_style, kit_planes, muted_style, voice_ink, VOICE_RUNGS};
    use crate::ui::style::tokens;

    let mut fails: Vec<String> = Vec::new();
    let mut checked = 0usize;

    for theme in selectable_themes() {
        let name = format!("{theme}");
        let planes = kit_planes(&theme);

        let mut entries: Vec<(String, Color, f32)> = VOICE_RUNGS
            .iter()
            .map(|&rung| (format!("rung {rung:.2}"), voice_ink(&theme, rung), rung))
            .collect();
        // The two style functions are the same ladder in `text::Style` form, so
        // they are measured as themselves *and* required to be the shared path —
        // a style fn that quietly goes back to `scale_alpha` would otherwise
        // paint the old pixel while this guard printed green.
        assert_eq!(
            muted_style(&theme).color,
            Some(voice_ink(&theme, tokens::INK_MUTED)),
            "{name}: muted_style is no longer the shared voice solver"
        );
        assert_eq!(
            faint_style(&theme).color,
            Some(voice_ink(&theme, tokens::INK_FAINT)),
            "{name}: faint_style is no longer the shared voice solver"
        );
        entries.push(("muted_style".into(), voice_ink(&theme, tokens::INK_MUTED), tokens::INK_MUTED));
        entries.push(("faint_style".into(), voice_ink(&theme, tokens::INK_FAINT), tokens::INK_FAINT));

        for (label, ink, rung) in entries {
            // A solved ink is opaque by construction (`tokens::legible_on` folds
            // the alpha into the colour), so the rung only survives as the painted
            // alpha in the themes that never needed solving. Print the difference
            // as a NOTE, not a failure: it is the ladder doing its job, and the
            // quietest rungs converge on one readable pixel — #69's law, stated.
            if ink.a != rung {
                println!(
                    "QC6 {name}|{label}|NOTE the requested alpha was {rung:.2}, the painted one is {:.2} (solved)",
                    ink.a
                );
            }            let mut worst = (f32::INFINITY, planes[0]);
            for plane in &planes {
                let ground = composite_over(ink, *plane);
                let score = wcag_contrast(ground, *plane);
                if score < worst.0 {
                    worst = (score, *plane);
                }
            }
            checked += 1;
            let (score, ground) = worst;
            println!(
                "QC6 {name}|{label}|ink {}|on {}|ink0 {} = {score:.2}:1 (floor {:.1})",
                hex_of(composite_over(ink, ground)),
                hex_of(ground),
                sig_of(ink),
                tokens::AA_INK_CONTRAST
            );
            if score < tokens::AA_INK_CONTRAST {
                fails.push(format!(
                    "{name} {label}: {} on {} = {score:.2}:1 (floor {:.1}:1)",
                    hex_of(composite_over(ink, ground)),
                    hex_of(ground),
                    tokens::AA_INK_CONTRAST
                ));
            }
        }
    }

    assert!(
        fails.is_empty(),
        "{} of {checked} quiet-voice measurements fail: a secondary row label was \
         faded by an alpha that was never resolved against the plane it is written \
         on:\n{}",
        fails.len(),
        fails.join("\n")
    );
}

/// #72: moving a colour to a readable luminance must not take the colour with it.
///
/// [`tokens::pull_to_luminance`] is the only move L0 makes when a colour sits at
/// the wrong brightness, and every promise in the kit rests on it: a verdict word
/// (`label::tone_ink_on`), a prose rung (`label::voice_ink`), a text-selection
/// band (`tokens::backdrop_for`), the aurora's own two stops (`tokens::hero`).
/// It moved along a straight line in linear light toward black or toward white,
/// and toward white that line is not a lightness change at all — it lifts the
/// small channels proportionally more than the large one, so the colour both
/// pales and rotates. Measured before this guard existed, over the 26 selectable
/// themes (`the_marked_text_survives_its_own_highlight`, QC lines, oklch
/// recomputed offline from the printed hex): Kanagawa Lotus `C 0.0862 → 0.0162`,
/// `C/L` ×0.114; Catppuccin Latte hue `+14.1°`; toward black on Tokyo Night the
/// absolute chroma fell to ×0.445 of the input.
///
/// The law asserted here is the one the fix has to satisfy, written so it cannot
/// be met by cheating in either direction:
///
/// - the hue angle may not move (tolerance below), because hue is what the eye
///   reads as *which* colour a band is;
/// - the chroma may fall only as far as the sRGB gamut at the *landed* lightness
///   and the *original* hue forbids. That ceiling is searched for here rather
///   than asserted away: approaching white, `C` genuinely has to shrink, and a
///   guard that ignored it would demand a pixel no display can show.
///
/// The hue law is skipped unless the pixel carries chroma at *both* ends of the
/// move, where `atan2` is arithmetic noise and a drift would be a fabricated
/// defect. The chroma law needs no such gate: a near-neutral input has a ceiling
/// of its own tiny `C`, so it can only ever pass.
#[test]
fn the_pull_keeps_the_colour_it_was_given() {
    use crate::ui::kit::label::{tone_ink_on, tone_pair, voice_ink, Tone, VOICE_RUNGS};
    use crate::ui::style::{common, tokens};
    use iced::widget::text_input;

    // Widest oklch chroma any sRGB colour reaches is ~0.33, so 0.40 brackets the
    // search from above without needing a per-hue table. This is the guard's own
    // reading of the display's limit, deliberately not a call into the code under
    // test: a ceiling copied from the implementation could only ever agree with it.
    fn gamut_chroma(l: f32, h: f32) -> f32 {
        let (rad, mut lo, mut hi) = (h.to_radians(), 0.0f32, 0.40f32);
        for _ in 0..24 {
            let mid = (lo + hi) * 0.5;
            if common::oklab_in_gamut([l, mid * rad.cos(), mid * rad.sin()]) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        lo
    }

    fn hue_drift(a: f32, b: f32) -> f32 {
        ((b - a + 540.0).rem_euclid(360.0) - 180.0).abs()
    }

    /// Half a degree of hue. Set from the 2026-10-16 sweep of 1332 pulls: the
    /// implemented pull moves a readable hue by at most 0.13°, while the straight
    /// line it replaced moved one by 17.59°. The bar sits between the two, so it
    /// cannot be met by anything that still rotates the colour on the way past.
    const HUE_TOL: f32 = 0.5;
    /// A pixel is only *a* hue while it carries chroma at both ends of the move.
    /// Below this the angle is `atan2` of a near-zero vector, and the probe
    /// measured exactly how misleading that is: an untrimmed sweep printed a
    /// 119° "drift" on a pull that had been gamut-forced into a neutral.
    const HUE_READABLE: f32 = 0.01;

    let mut fails: Vec<String> = Vec::new();
    let mut pulled = 0usize;
    let mut untouched = 0usize;
    let mut gamut_trimmed = 0usize;
    let mut worst_hue = (0.0f32, String::new());
    let mut worst_chroma = (1.0f32, String::new());

    for theme in selectable_themes() {
        let name = format!("{theme}");
        let palette = theme.palette();

        // Every entry is a colour the kit actually asks L0 to move, taken from
        // the call the widget makes — not a probe hue invented for the test.
        let mut cases: Vec<(&str, Color, Color)> = Vec::new();
        for &rung in &VOICE_RUNGS {
            cases.push(("voice", palette.background.base.text, voice_ink(&theme, rung)));
        }
        for tone in [Tone::Accent, Tone::Success, Tone::Warning, Tone::Danger] {
            let plate = crate::ui::kit::surface::tone_band_plate(&theme, tone);
            cases.push(("tone", tone_pair(&theme, tone).color, tone_ink_on(&theme, tone, plate)));
        }
        cases.push((
            "select",
            palette.primary.base.color,
            crate::ui::kit::field_style(&theme, text_input::Status::Active).selection,
        ));
        // The danger field's *ring*, not its selection band: `error_field_style`
        // resolves `selection` from `palette.primary` (the recess is already the
        // danger colour, so the highlight is the only part free to move), which
        // would make a danger→selection pairing two unrelated hues and a drift
        // the pull never caused. What L0 is asked to move here is the danger hue
        // itself, at the 3.0 component floor rather than the 4.5 ink floor.
        cases.push((
            "danger-edge",
            palette.danger.base.color,
            crate::ui::kit::error_field_style(&theme, text_input::Status::Active)
                .border
                .color,
        ));
        // The aurora's two ends and a danger tint, moved by `painted` — the same
        // `pull_stops` maths `hero` runs, reachable without matching on a
        // gradient struct.
        for (tag, hue) in [
            ("hero-hi", tokens::aurora_hi(&theme)),
            ("hero-lo", tokens::aurora_lo(&theme)),
        ] {
            let (plate, _) = tokens::painted(hue, 0.0);
            cases.push((tag, hue, plate));
        }

        for (family, before, after) in cases {
            if (before.r - after.r).abs() < 1e-6
                && (before.g - after.g).abs() < 1e-6
                && (before.b - after.b).abs() < 1e-6
            {
                untouched += 1;
                continue;
            }
            pulled += 1;

            let (l0, c0, h0) = common::oklch_of(before);
            let (l1, c1, h1) = common::oklch_of(after);
            // Reported separately from the gate: `neutral` only describes the input,
            // which is what the printed suffix is about, while `readable` is the
            // both-ends condition the hue law actually needs.
            let neutral = c0 < HUE_READABLE;
            let readable = c0 >= HUE_READABLE && c1 >= HUE_READABLE;
            let dh = hue_drift(h0, h1);
            let ceiling = c0.min(gamut_chroma(l1, h0));
            let trimmed = c1 < c0 - 1e-6;
            if trimmed {
                gamut_trimmed += 1;
            }
            let ratio = if c0 > 1e-6 { c1 / c0 } else { 1.0 };

            println!(
                "PULL72 {name}|{family}|in {}|out {}|L {l0:.3}->{l1:.3}|C {c0:.4}->{c1:.4}|h \
                 {h0:.1}->{h1:.1}|dh {dh:.2}|Cmax@L1 {ceiling:.4}|ratio {ratio:.3}{}",
                hex_of(before),
                hex_of(after),
                if !readable {
                    if neutral { "|neutral-input" } else { "|neutral-output" }
                } else if trimmed {
                    "|gamut-trimmed"
                } else {
                    ""
                }
            );

            if readable && dh > worst_hue.0 {
                worst_hue = (dh, format!("{name} {family}"));
            }
            if ratio < worst_chroma.0 {
                worst_chroma = (ratio, format!("{name} {family}"));
            }
            if readable && dh > HUE_TOL {
                fails.push(format!(
                    "{name} {family}: hue moved {dh:.2}° ({} → {}), tol {HUE_TOL}",
                    hex_of(before), hex_of(after)
                ));
            }
            if ceiling - c1 > tokens::CHROMA_SLACK {
                fails.push(format!(
                    "{name} {family}: chroma {c0:.4} → {c1:.4} but the gamut at the landed \
                     lightness would have carried {ceiling:.4} (slack {})",
                    tokens::CHROMA_SLACK
                ));
            }
        }
    }

    println!(
        "PULL72 SUM|pulled={pulled}|untouched={untouched}|gamut_trimmed={gamut_trimmed}|worst_hue=\
         {:.2}°@{}|worst_chroma_ratio={:.3}@{}",
        worst_hue.0, worst_hue.1, worst_chroma.0, worst_chroma.1
    );

    assert!(
        fails.is_empty(),
        "{} of {pulled} pulls ({} left alone) moved the colour as well as the \
         luminance:\n{}",
        fails.len(),
        untouched,
        fails.join("\n")
    );
}

/// The exact arithmetic behind the #73 ruling, kept as a guard rather than as a
/// note in a commit message: a placement-free boundary edge is not merely hard, it
/// is impossible, and the per-plane solve is not merely easier, it always has an
/// answer.
///
/// The 16³ cube search said 39 of 312 band/chip cases have no single opaque pixel
/// that carries the boundary law. A cube can only under-report, so that number was
/// an upper bound on the debt, not a verdict. This sweep answers the question
/// exactly: contrast between two colours depends on nothing but their relative
/// luminances, so
///
///   g(y) = min over needing planes p of (max(y,p)+0.05)/(min(y,p)+0.05)
///
/// is a function of one variable. Between two adjacent plane luminances every
/// branch is monotone, so g's peak sits either at y=0, y=1, or where the
/// "above" branch of the darker neighbour crosses the "below" branch of the
/// lighter one: y+0.05 = sqrt((a+0.05)(b+0.05)). Enumerating those candidates
/// gives the exact supremum over ALL colours, greys included — and since every
/// luminance in [0,1] is reachable by a real grey pixel, `bound < bar` means
/// provably no pixel of any kind satisfies the law, while `bound >= bar` means
/// the solver, not the maths, is at fault.
///
/// Three assertions, all structural, so a theme added later can only move the
/// counts on the printed line:
///
///   * `impossible > 0` — some band or chip cannot be served by a painter that
///     cannot see its ground. That is why
///     [`crate::ui::kit::surface::tone_plate_style_on`] and
///     [`crate::ui::kit::control::signal_button_style`] take the plane as an
///     argument, and no future "simplify the signature back out" refactor can claim
///     the old form was merely conservative;
///   * `conjunction_dead > 0` — the retired limb (one hairline must also read on
///     the plate it outlines) is unsolvable on real plate/plane pairs, so what
///     [`the_verdict_band_feeds_its_own_words`] stopped asserting was a false
///     requirement, not a weakened one;
///   * `single_plane_dead == 0` — asked about *one* plane, the law always has a
///     colour. The per-ground solve can never be blocked by arithmetic, only by a
///     solver bug.
#[test]
fn a_placement_free_edge_is_provably_impossible() {
    use crate::ui::kit::label::{kit_planes, Tone};
    use crate::ui::kit::surface::tone_band_plate;
    use crate::ui::kit::{self, Tier};
    use crate::ui::style::common::wcag_luminance;
    use crate::ui::style::tokens;
    use iced::widget::button;

    let bar = tokens::AA_COMPONENT_CONTRAST;
    let ratio = |x: f32, y: f32| (x.max(y) + 0.05) / (x.min(y) + 0.05);

    // The formula has to be the same function the guard measures with, or the
    // sweep proves nothing about the failures it is counting.
    for (c, p) in [
        (Color::BLACK, Color::WHITE),
        (Color::from_rgb8(0x77, 0x77, 0x77), Color::WHITE),
        (Color::from_rgb8(0x29, 0x2D, 0x33), Color::from_rgb8(0x7F, 0x84, 0x8E)),
    ] {
        let a = wcag_luminance(c);
        let b = wcag_luminance(p);
        assert!(
            (ratio(a, b) - wcag_contrast(c, p)).abs() < 1e-4,
            "bound formula disagrees with wcag_contrast: {} vs {}",
            ratio(a, b),
            wcag_contrast(c, p)
        );
    }

    // sup over one luminance variable of the worst reading in a needing set.
    let bound = |ys: &[f32]| -> (f32, f32) {
        let mut cands = vec![0.0f32, 1.0];
        for &a in ys {
            for &b in ys {
                let t = ((a + 0.05) * (b + 0.05)).sqrt() - 0.05;
                if t >= 0.0 && t <= 1.0 {
                    cands.push(t);
                }
            }
        }
        let mut best = (0.0f32, 0.0f32);
        for y in cands {
            let g = ys
                .iter()
                .map(|p| ratio(y, *p))
                .fold(f32::INFINITY, f32::min);
            if g > best.0 {
                best = (g, y);
            }
        }
        best
    };
    let grey_at = |y: f32| -> Color {
        let lin = if y <= 0.0031308 {
            12.92 * y
        } else {
            1.055 * y.powf(1.0 / 2.4) - 0.055
        };
        let byte = (lin * 255.0).round().clamp(0.0, 255.0) as u8;
        Color::from_rgb8(byte, byte, byte)
    };

    let tones = [Tone::Accent, Tone::Success, Tone::Warning, Tone::Danger];
    let mut impossible = 0usize;
    let mut solvable = 0usize;
    // Placement-aware: for each hosting plane, could an edge clear *it* and the
    // part's own plate at the same time? That two-pixel conjunction is the limb
    // the ruling retired.
    let mut conjunction_dead = 0usize;
    let mut conjunction_total = 0usize;
    // The form the signature now asks about: one plane, nothing else.
    let mut single_dead = 0usize;
    let mut single_total = 0usize;
    let mut worst_worst = (f32::INFINITY, String::new());
    let mut lines: Vec<String> = Vec::new();

    for theme in selectable_themes() {
        let planes = kit_planes(&theme);
        for tone in tones {
            // ── the band plate, and the chip's at both rungs ─────────────────
            let cases: Vec<(String, Color)> = {
                let mut c = vec![(format!("{theme}|band|{tone:?}"), tone_band_plate(&theme, tone))];
                let paint = kit::signal_button_style(tone, Tier::Band);
                for (what, status) in [
                    ("rest", button::Status::Active),
                    ("lit", button::Status::Hovered),
                ] {
                    let s = paint(&theme, status);
                    if let Some(iced::Background::Color(cp)) = s.background {
                        c.push((format!("{theme}|chip|{tone:?}|{what}"), cp));
                    }
                }
                c
            };

            for (label, p) in cases {
                let py = wcag_luminance(p);
                let needing_planes: Vec<Color> = planes
                    .iter()
                    .cloned()
                    .filter(|plane| wcag_contrast(p, *plane) < bar)
                    .chain(std::iter::once(p))
                    .collect();
                let needing: Vec<f32> = needing_planes.iter().map(|c| wcag_luminance(*c)).collect();
                let (b, y) = bound(&needing);
                if b + 1e-6 < bar {
                    impossible += 1;
                    if b < worst_worst.0 {
                        worst_worst = (b, label.clone());
                    }
                } else {
                    solvable += 1;
                }
                lines.push(format!(
                    "BOUND73 {label}|plate {} (Y {py:.3})|needing={}|bound={:.3}@{} Y{y:.3}|{}",
                    hex_of(p),
                    needing.len(),
                    b,
                    hex_of(grey_at(y)),
                    if b + 1e-6 < bar { "IMPOSSIBLE" } else { "SOLVABLE" },
                ));
                // The retired limb, pair by pair: a colour asked to separate the
                // part's own plate *and* the plane beside it.
                for &plane in &needing_planes {
                    let fy = wcag_luminance(plane);
                    conjunction_total += 1;
                    if bound(&[py, fy]).0 + 1e-6 < bar {
                        conjunction_dead += 1;
                    }
                }
            }
        }

        // The law as the painter is now allowed to ask it: this one plane.
        for &plane in &planes {
            let fy = wcag_luminance(plane);
            single_total += 1;
            let (b, _) = bound(&[fy]);
            if b + 1e-6 < bar {
                single_dead += 1;
                lines.push(format!(
                    "BOUND73 DEAD-SINGLE {theme}|{} (Y {fy:.3})|bound={b:.3}",
                    hex_of(plane)
                ));
            }
        }
    }

    println!("{}", lines.join("\n"));
    println!(
        "BOUND73 SUMMARY cases={}|impossible={impossible}|solvable={solvable}|conjunction pairs={conjunction_total} unsolvable={conjunction_dead}|single-plane grounds={single_total} unsolvable={single_dead}|worst bound={:.3}@{}|bar={bar}",
        impossible + solvable,
        worst_worst.0,
        worst_worst.1,
    );

    assert!(
        impossible > 0,
        "{impossible} of {} cases need no pixel: a placement-free edge serves the \
         boundary law in every case, which contradicts the 2026-10-17 sweep that \
         moved the ground into the painter signature. Either the plane vocabulary \
         changed or the signature is being asked to answer for planes it is never \
         handed.",
        impossible + solvable,
    );
    assert!(
        conjunction_dead > 0,
        "no plate/plane pair is unsolvable by any colour: the retired own-plate \
         limb would be a real requirement after all, and \
         the_verdict_band_feeds_its_own_words must restore it rather than keep \
         asserting the disjunction per ground.",
    );
    assert_eq!(
        single_dead, 0,
        "{single_dead} of {single_total} single planes admit no boundary colour: the \
         per-ground solve is not a matter of arithmetic but of a solver that has been \
         handed more than one plane.",
    );
}

