//! Chrome metrics shared by the widget stylesheets: one radius scale, one
//! spacing scale, one type scale, one surface ladder, and the elevation
//! shadows that give the flat iced palette depth.
//!
//! These are deliberately plain `f32` constants rather than a struct: a call
//! site reads `tokens::RADIUS_CONTROL` and stays a constant expression, so the
//! stylesheet closures keep the `fn(&Theme, Status) -> Style` shape iced wants.
//!
//! Colour never comes from a literal. The colour-bearing functions here —
//! [`surface`], [`inset`], [`sunk`], [`edge`], [`elevation`] — all take the
//! active `&Theme` and derive from its palette, which is what keeps the dark
//! and light schemes from diverging: nothing in the UI layer can name a color,
//! only a role.
//!
//! `L5` of the UI upgrade replaced the per-file magic numbers with these. The
//! scale is coarse on purpose (four radii, eight gap steps, six type sizes):
//! more steps meant more ways for two dialogs to disagree by one pixel.
//!
//! A rung is added from a measurement, never from a wish. The gap and type
//! ladders below were extended after the 2026-10-15 histogram of every literal
//! in the guarded tree (`D:\tmp\kit-baseline.csv`), which showed six values
//! used by more than 90 screens-side sites each and served by no rung at all.
//! A value with no rung snaps to the nearest one and the delta gets reported;
//! it does not get to stay a magic number.

use super::common;
use iced::{Background, Color, Theme, Vector};

/// A deliberately square plate. The context menu's rows are stacked full-width
/// and hit-tested to a fixed 22 px line (`MENU_ROW_H`), so a corner would round
/// away the seam that tells the pointer which row it is on; the same zero is
/// what `kit::Size::Flush` means on the inset ladder. It is named instead of
/// written as `0.0` because the (b) column counts a literal here, and a zero
/// that is a decision should read like one.
pub const RADIUS_FLAT: f32 = 0.0;
/// The pencil mark's corner: a dock drop line, an insertion bar, the
/// active-tool slat. One pixel is the whole round a 0.35 mm line can carry; any
/// more and the mark reads as a pill rather than a stroke.
pub const RADIUS_MARK: f32 = 1.0;
/// Text fields and other flush controls. The sheet's `r-ctl` is 2 px for every
/// control, so a field and the button beside it share one corner.
pub const RADIUS_FIELD: f32 = 2.0;
/// Buttons, list rows, chips: anything you click. Equal to [`RADIUS_FIELD`] by
/// design — the sheet draws one 2 px corner for the whole control family, and
/// the two names stay apart because they answer different questions.
pub const RADIUS_CONTROL: f32 = 2.0;
/// Cards, panels, modals: surfaces that contain controls. The sheet's `r-panel`
/// and window frame are the only 4 px corners in the app.
pub const RADIUS_PANEL: f32 = 4.0;
/// The sheet's card corner: the ribbon, the command line, a dialog's paper, a
/// floating properties panel. The 2026-10 sheet draws every one of these as a
/// sheet floating on the page with a 16 px corner, which is also
/// [`RADIUS_GLASS`] — a card and a glass pane are the same silhouette, they
/// differ in material, not in outline. Kept apart from [`RADIUS_PANEL`] because
/// the vendor themes compare bit-for-bit against their own 4 px and must not
/// move when the sheet does.
pub const RADIUS_CARD: f32 = 16.0;
/// Tiles and wells inside a card: a tool tile, a field, a group ground. The
/// sheet nests one radius step down from the card it sits on.
pub const RADIUS_TILE: f32 = 10.0;
/// The glass plane: a floating pane that reads the pixels under it. 16 px comes
/// from the light sheet's `--radius-sheet: 16px` (`U08-l0-mapping.md` §3.1);
/// the Apple card rung is 18 and the app-screens soft band is 16–24, so this
/// takes the low end that the sheet actually names. Radius is pure paint — it
/// moves no hit target and no layout edge.
pub const RADIUS_GLASS: f32 = 16.0;
/// A glass pane raised above other glass — the menu that pops from a docked
/// panel. The dark sheet's `--radius-card: 20px`.
pub const RADIUS_GLASS_RAISED: f32 = 20.0;
/// Controls that sit *on* glass: segmented pickers, buttons, input wells. The
/// sheets' `--radius-control: 6px`, and the reason it is a rung and not a
/// literal — without it every screen would hand-write 6.
pub const RADIUS_GLASS_CONTROL: f32 = 6.0;
/// The pill. Reserved for the segmented selector and nothing else; the sheets
/// call it the signature Apple round.
pub const RADIUS_PILL: f32 = 9999.0;

/// The side of a small square plate: a checkbox's box, a colour swatch, the
/// "?" marker on a mixed-selection selector.
///
/// Measured, not chosen. 17 of the 20 checkbox call sites that set a box size
/// spelled `14` (2 spelled 15, 1 spelled 13), and the two colour-marker plates
/// in `color_select.rs` spelled 13. One rung for both families, because a swatch
/// in a property row and the checkbox in the row under it are the same kind of
/// object — a square mark the user reads at a glance — and 1px apart they make
/// the left edges of that column disagree. The three deviants snap here: the two
/// 15s and the 13 lose and gain 1px respectively, which is under the threshold
/// where a mark stops looking like its neighbour.
pub const MARK_SQUARE: f32 = 14.0;

/// Flush. Two plates that must read as one — a header glued to its body, a
/// table column against its neighbour. Any non-zero value here draws a seam
/// straight through what is meant to be a single surface.
pub const GAP_NONE: f32 = 0.0;
/// The only negative value in the ladder: two bordered halves of *one* control
/// pushed together by exactly the border they share, so the seam between them
/// draws one hairline instead of two. A split button's main half and its caret
/// use this; anything else that reaches for it is overlapping two separate
/// controls and will eat one pixel of whichever plate is narrower.
///
/// Tied to [`EDGE_HAIRLINE`] rather than spelled `-1.0`, because the amount to
/// swallow is by definition the width of the line being doubled: a future
/// 0.5px hairline for hi-DPI has to move this seam with it, and two literals
/// would let the halves drift apart and leave a gap where the border was.
pub const JOIN_OVERLAP: f32 = -EDGE_HAIRLINE;
/// The seam between two stacked rows of one dense list — a ribbon panel's
/// icon-only column, a style manager's rows, a popup's filter list.
///
/// One pixel, and it is named for the same reason [`PAD_FIELD_Y`] is: it sits one
/// pixel between two neighbours that are *both* wrong here. `GAP_NONE` fuses two
/// adjacent plates into what reads as one control (and a stacked cell that is
/// engaged while its neighbour is not then shows a single tall glow instead of a
/// highlighted row); `GAP_HAIRLINE` is 2 and is defined as "inside one control's own
/// parts", not between two pressable rows.
///
/// Measured, not composed: **thirteen** call sites in **thirteen** files type this
/// `1` by hand — `ui/ribbon/widgets.rs` (`flush_small_col`, the stacked icon column),
/// `ui/node_graph.rs`, `ui/popup/{selection_filter_popup,snap_popup}.rs`,
/// `ui/window/{block_palette,browser,layer_translator,pc_manager}.rs` and the five
/// list screens `ui/style/{anno_object_scale,plotstyle,scale_manager,style_manager,textstyle}.rs`,
/// each of them `themed_scrollable(column(rows).spacing(1)).height(sizing.height)`.
///
/// The value is load-bearing against a computed height, which is why folding it is
/// not free: `ui/ribbon/mod.rs:1065` sizes the layer list as `row_count × 26.0` while
/// every row is `ROW_H` tall, so a rung of 2 would push the last row past the panel
/// and make a list that fits look scrollable. Like `PAD_FIELD_Y`, this is a named
/// inset, **not** a ninth rung of the spacing ladder — reach for it only when the
/// surface is two stacked pressable rows, never as a general-purpose small gap.
pub const GAP_SEAM: f32 = 1.0;
/// Inside one control's own parts, where [`GAP_TIGHT`] already reads as two
/// separate controls: a stepper's button against its field, a swatch against
/// its hex.
pub const GAP_HAIRLINE: f32 = 2.0;
/// Within a group (icon and its label, a stepper's parts).
pub const GAP_TIGHT: f32 = 4.0;
/// The dense chrome's default: two adjacent controls in the ribbon, the dock
/// title strip, a property row. It exists because the app's densest surfaces
/// are not dialogs and 8 is too wide for them.
pub const GAP_CONTROL: f32 = 6.0;
/// Between sibling controls on one row, and the rhythm of a list.
pub const GAP_ROW: f32 = 8.0;
/// Between two labelled field rows in a settings-style dialog: wider than a
/// row of controls, tighter than a section.
pub const GAP_FIELD: f32 = 10.0;
/// Between a group and the next group.
pub const GAP_SECTION: f32 = 12.0;
/// The lane a scrollable table keeps clear on its right so the scrollbar never
/// lands on the last column's controls. Three windows each declared their own
/// `const GUTTER: f32 = 16.0` for this (`ui/window/alias_editor.rs`,
/// `shortcuts.rs`, `named_parameters.rs`), which is one fact with three
/// owners: a theme or a scrollbar width change had to be found three times. Wider than a section
/// because it is not a gap between things but a reserve beside them, and
/// narrower than [`PAD_PAGE`] because it is a lane inside a card while that is
/// the air between two blocks of one.
pub const GAP_GUTTER: f32 = 16.0;
/// The air between a dialog's body block and its footer row — the widest gap
/// inside a card, and the one number above the eight `GAP_*` rungs.
///
/// Measured at #42's seventeenth screen (`app/view/modal.rs`), which was the
/// first screen to need this altitude: nine of that file's windows separate a
/// block of prose or fields from the row of verbs, and they wrote it four ways
/// — `20` five times, `18` twice, `16` once, `22` once. `20` is both the
/// plurality and the number this constant already carried, so the role landed
/// there and the four off-20 spellings paid 2-4 px; with the same screen's
/// spacers folded onto their own rungs the file is net zero (+14 px on the
/// field rows and footer air, −14 px under three dialog titles).
///
/// The one-line doc this replaces called it "a dialog's outer inset". That role
/// moved to [`PAD_PLATE_V`]/[`PAD_PLATE_X`] once the six warning windows turned
/// out to want `[24, 28]`, and the name's only live user before this screen was
/// `app/view/mod.rs`, which reaches for it as a `.spacing()`. So the identifier
/// says `PAD_` while the role is a gap: the rename into the `GAP_` band is a
/// one-call-site wave that belongs to an L0 pass, not to a screen rebuild, and
/// it is recorded here instead of left to be rediscovered. A ninth `GAP_*` rung
/// was the other option and is refused for the reason [`PAD_PLATE_V`] gives.
pub const PAD_PAGE: f32 = 20.0;

/// The vertical inset *inside* a field — the blank above and below the text in
/// a text input or a pick list.
///
/// Deliberately not a `GAP_*` rung, because it answers a different question: a
/// gap separates two plates from each other, this separates the ink from the
/// plate it is painted on. Forcing it onto the ladder would have meant snapping
/// to `2` or `4`, and the measurement says both are lies — counted 2026-10-16
/// over every `.padding(...)` argument under `src/` (split per number), `3` is
/// the fourth most used padding value in the app at 150 sites, behind only
/// `4` (254), `8` (175) and `6` (158). A rung that is 1px from two neighbours
/// does not belong in a scale; a value with 150 users belongs in the codebase.
pub const PAD_FIELD_Y: f32 = 3.0;

/// The horizontal inset of the kit's `Size::Action` button — the verb in a
/// dialog footer, whose plate is as wide as its words need.
///
/// Named rather than rounded, and kept out of the ladder on purpose: 11 sites
/// app-wide against 150 for [`PAD_FIELD_Y`] does not earn a rung, but rounding
/// to `20` or `16` would move every action button 4px in total width, which is
/// a visible change to a shipped control made for the sake of a number's tidy
/// appearance. It lives here rather than in the kit because L0 is the only
/// place a number is decided; when the L3 waves fold those 11 sites into a
/// named inset this either earns a rung or disappears.
pub const PAD_ACTION_X: f32 = 18.0;

/// The inset a modal dialog's body carries inside its own plate — the widest
/// band in the file, and the only pair app-wide with six identical users.
///
/// Measured, not composed: `D:\tmp\ladder-snap.cjs` reads every off-ladder
/// spacing/padding call in the guarded tree and reports the pair `[24, 28]` at
/// six sites, all of them warning or confirmation windows in
/// `app/view/modal.rs` (`unsaved_changes`, `autosave_restore`, `aec_drop`,
/// `layer_delete_warning`, `gpu_warning`, `default_assoc`). The same file also
/// carries `[18, 20]` twice and `[14, 16]` twice for the same job, which is
/// drift, not a second band: those four rows are the dialog family's own
/// decision in `#42`, and folding them onto this inset is where their pixels
/// get signed off. Named at L0 like [`PAD_ACTION_X`] rather than added as a
/// `GAP_*` rung, because a plate inset is not a gap between siblings — the
/// ladder stays eight rungs, and every screen that asks for a twenty-fourth
/// rung is a screen that has not been rebuilt yet.
pub const PAD_PLATE_V: f32 = 24.0;
/// The horizontal mate of [`PAD_PLATE_V`]. See its note; both numbers come
/// from the same six measured rows.
pub const PAD_PLATE_X: f32 = 28.0;

/// The vertical inset inside a ribbon tab's plate.
///
/// Named rather than rounded, for the same reason as [`PAD_ACTION_X`]: a tab
/// inset is not a gap between siblings, and here the lateral number is not even
/// free — see [`PAD_TAB_X`] for the arithmetic it carries.
pub const PAD_TAB_V: f32 = 5.0;
/// The lateral inset of a ribbon tab, which doubles as the half-width the strip
/// is allowed to eat into when a row runs out of room.
///
/// The coupling is arithmetic, not taste: the tab row's `min_spacing_x` is
/// `-(PAD_TAB_X - GAP_HAIRLINE)`, so two neighbours may slide 12px into each
/// other and still leave 2px of blank before their labels touch. That expression
/// is written out at the call site so the dependency reads, and
/// `kit_contract_tests` pins it — rounding either number without the other is
/// how a tab row starts overprinting its own labels.
pub const PAD_TAB_X: f32 = 14.0;

/// The vertical inset of the start page's brand plate — the app's one deliberate
/// gradient surface, drawn by `kit::Tint::Hero` around the wordmark.
///
/// 18 px is what that plate shipped with: `app/view/mod.rs:3023` is the only site
/// in the guarded tree that types `[18, 40]` (counted 2026-10-20 with
/// `grep -rn 'padding(\[18, 40\])' src`). This is not [`PAD_ACTION_X`] wearing a
/// different hat: a button's 18 is lateral air around a verb whose plate is as
/// wide as its words, while this 18 is the band that keeps the aurora's top and
/// bottom edges far enough from the mark to read as warmth instead of as a
/// target. Merging the two constants would let the next change to a dialog button
/// re-decide the splash screen.
pub const PAD_HERO_V: f32 = 18.0;
/// The lateral mate of [`PAD_HERO_V`], and the widest inset in the app.
///
/// The plate is centred by its parent, so this number alone sets how much aurora
/// extends past the wordmark. [`PAD_PLATE_X`] (28) answers a different question —
/// how far a warning window's prose sits from its edge — and pulling the hero onto
/// it would narrow the splash by 24 px to share a number rather than to serve the
/// mark.
pub const PAD_HERO_X: f32 = 40.0;

/// The vertical inset of the open-progress card: the modal that draws the file
/// name, the byte-based bar and the phase line while a drawing loads.
///
/// Measured at its only site, `ui/window/open_progress.rs:139`. The card's width
/// is `Length::Fixed(CARD_WIDTH)`, so this inset is the only thing deciding how
/// much plate survives beside the bar, and the bar stretches to fill whatever the
/// inset spares. It carries the same 18 as [`PAD_HERO_V`] for unrelated reasons
/// and keeps its own name so the two can be re-decided apart; whether the two are
/// one band is #58's ruling, not this rename's.
pub const PAD_PROGRESS_CARD_V: f32 = 18.0;
/// The lateral mate of [`PAD_PROGRESS_CARD_V`].
///
/// 22 px, the same single site. The card's Cancel verb sits on its own row that a
/// container right-aligns, so the number is pure plate: no layout arithmetic in the
/// file divides or adds it, which is what makes it nameable at 0 px.
pub const PAD_PROGRESS_CARD_X: f32 = 22.0;

// ── Typeface ────────────────────────────────────────────────────────────
//
// One family carries the shell's Latin, Simplified Chinese, Cyrillic and Greek
// text: Noto Sans SC, embedded from `assets/fonts/NotoSansSC-VF.ttf` (SIL OFL
// 1.1) and installed as the native default in `app::run`. Before this the
// Latin came from iced's bundled Fira Sans while every CJK glyph fell through
// to a system face, so one 12 px label could be drawn in two different designs
// at the same size. The variable face is instanced at the requested weight by
// cosmic-text, which is why the three named weights below are real and not
// synthesized.
//
// The web build keeps iced's bundled sans: the wasm bundle would grow by the
// font's 17.7 MB, and the browser's own CJK fallback is the platform's choice
// there.
#[cfg(not(target_arch = "wasm32"))]
pub const UI_FONT_FAMILY: &str = "Noto Sans SC";
#[cfg(target_arch = "wasm32")]
pub const UI_FONT_FAMILY: &str = "Fira Sans";

/// The regular UI voice, named so a weight override cannot silently drop the
/// family back to iced's bundled sans — `Font::DEFAULT` is `SansSerif`, not the
/// app default, and a literal `..Font::DEFAULT` would opt a label out of the
/// embedded face.
pub const UI_FONT: iced::Font = iced::Font::new(UI_FONT_FAMILY);
/// Group names and table captions: one step up from body.
pub const UI_FONT_MEDIUM: iced::Font = UI_FONT.weight(iced::font::Weight::Medium);
/// Panel, section and dialog titles.
pub const UI_FONT_SEMIBOLD: iced::Font = UI_FONT.weight(iced::font::Weight::Semibold);
/// Error lines and other verdicts that must outrank body text.
pub const UI_FONT_BOLD: iced::Font = UI_FONT.weight(iced::font::Weight::Bold);

/// The launch screen's own palette and metrics.
///
/// The splash is a brand plate, not window content: it keeps the dark sheet's
/// ground on every theme (a light theme must not flash white before the start
/// page paints), and its colours are the dark scheme's exact values rather
/// than roles read from the active theme. The design record with the
/// contrast measurements lives in `D:\tmp\splash-b284\06-tokens.md`.
pub mod splash {
    use iced::{Color, font, Font};

    /// The dark sheet's shell — the same value the start page sits on.
    pub const GROUND: Color = Color::from_rgb8(0x14, 0x16, 0x19);
    /// The wordmark. 16.73:1 on [`GROUND`].
    pub const INK: Color = Color::from_rgb8(0xF4, 0xF6, 0xF8);
    /// CAD, status line, version row, values. 5.75:1.
    pub const MUTED: Color = Color::from_rgb8(0x8A, 0x92, 0x9B);
    /// Sheet labels, table captions, partition numbers. 5.33:1.
    pub const LABEL: Color = Color::from_rgb8(0x84, 0x8C, 0x96);
    /// The one saturated signal: baseline, block, zero, cursor, marker. 5.98:1.
    pub const ACCENT: Color = Color::from_rgb8(0xF2, 0x6A, 0x45);
    /// Hairlines of the inner sheet edge, axis and footer rule (1.40:1).
    pub const LINE_INNER: Color = Color::from_rgb8(0x2C, 0x32, 0x3A);
    /// Outer frame, auxiliary rule, left axis. 1.55:1.
    pub const LINE_FRAME: Color = Color::from_rgb8(0x32, 0x39, 0x42);
    /// Table rules and short ruler ticks. 2.29:1.
    pub const LINE_TABLE: Color = Color::from_rgb8(0x4A, 0x52, 0x5C);
    /// Long ruler ticks and filled scale blocks. 3.16:1.
    pub const LINE_STRONG: Color = Color::from_rgb8(0x5E, 0x67, 0x72);
    /// Dimension lines and extensions. 2.36:1.
    pub const LINE_DIM: Color = Color::from_rgb8(0x4C, 0x54, 0x5E);
    /// The upper-right dot field. 1.65:1.
    pub const DOT: Color = Color::from_rgb8(0x36, 0x3D, 0x46);

    /// Wordmark faces. The variable face is instanced at these weights by
    /// cosmic-text, so the contrast is a real instance, not a synthesised one.
    pub const WORDMARK_BOLD: Font = Font::new(super::UI_FONT_FAMILY).weight(font::Weight::ExtraBold);
    pub const WORDMARK_LIGHT: Font = Font::new(super::UI_FONT_FAMILY).weight(font::Weight::Light);

    // ── Geometry, in baseline-canvas units (1280×800) ──────────────────────
    /// Baseline centre measured up from the bottom edge.
    pub const BASE_FROM_BOTTOM: f32 = 202.0;
    /// Wordmark size at s = 1.
    pub const WORDMARK_SIZE: f32 = 150.0;
    /// The glyph cap height as a fraction of the size (measured 112 at 150).
    pub const CAP_RATIO: f32 = 0.747;
    /// Outer frame inset (trim line) and inner frame inset (sheet edge).
    pub const FRAME_OUTER: f32 = 24.0;
    pub const FRAME_INNER: f32 = 40.0;
    /// The single scale variable clamps here; the width-only branch below 800
    /// drops the floor so the wordmark never bleeds off a dragged-in window.
    pub const SCALE_MIN: f32 = 0.70;
    pub const SCALE_MAX: f32 = 3.2;
    pub const DESIGN_WIDTH: f32 = 1145.0;

    // ── The "working" pair, added 2026-11-07 ───────────────────────────────
    /// The pair is two external gears on one module: a 20-tooth wheel driving
    /// a 12-tooth pinion from the lower right. Centre of the wheel in baseline
    /// units, and the direction the pinion sits in. The union box (`tip + 18`
    /// around the wheel, `tip + 18` around the pinion) is 668–893 × 58–224, so
    /// it clears the revision table (448), the environment block (`w − 190`),
    /// the inner frame (40) and the dot field (250) at every rung of the
    /// ladder. `splash.rs` asserts that, it is not eyeballed.
    pub const GEAR_X: f32 = 750.0;
    pub const GEAR_Y: f32 = 140.0;
    pub const GEAR_MESH_DEG: f32 = 18.0;
    /// Teeth and the radii that bound the wheel's teeth: tip, root (the pitch
    /// circle is the mean, drawn dash-dot) and the bore. The module is derived
    /// from these, so the pinion meshes instead of merely touching.
    pub const GEAR_TEETH: usize = 20;
    pub const GEAR_TIP: f32 = 64.0;
    pub const GEAR_ROOT: f32 = 52.0;
    pub const GEAR_BORE: f32 = 15.0;
    pub const PINION_TEETH: usize = 12;
    pub const PINION_BORE: f32 = 10.0;
    /// Revolution of the wheel across the whole entrance, radians (103°). The
    /// pinion turns 1.667× faster the other way — the ratio the pitch radii
    /// dictate, so the two stay in mesh for the full run. Anything under about
    /// 1.2 rad reads as a still image at 1.3 s; a full turn reads as a spinner.
    pub const GEAR_TURN_RAD: f32 = 1.8;
    /// The start rule beside the status line. Two px so it stays under the
    /// 3 px accent baseline in weight; the floor is the shortest run worth
    /// drawing before the bar is replaced by the status text alone.
    pub const PROGRESS_HEIGHT: f32 = 2.0;
    pub const PROGRESS_MIN: f32 = 140.0;
}

// The whole ladder was lifted on 2026-11-02: the owner read the shipped
// screens and said every line was too small. Each rung moved by roughly the
// same factor (the dense tier by 2, the headline tier by 1.2 to 1.25), so the
// guard below still asserts the same order, bands and ratios with new numbers.
// The paragraphs keep the history of how each rung was priced; where they
// quote a pixel figure that changed, the current value is what they now say.

/// The word under a glyph in a tile button. Two sites, both in the gradient
/// editor's kind row: an 11 px label beneath a 25 px glyph inside a 48 px tile.
///
/// It is a rung *below* [`TEXT_MICRO`], and [`TEXT_MICRO`]'s doc used to claim
/// nothing lives under it. The claim was a readability floor, not a measurement;
/// these two lines shipped under it anyway, so the ladder now records what the
/// app draws and the floor sentence moved to this paragraph. #58 owns the ruling
/// on which is right: either the caption is the floor and MICRO loses its
/// rationale, or these two captions go up to MICRO and pay 1 px of legibility
/// inside a fixed 48 px tile. Signed 2026-10-19 as a 0-px rename; the number is
/// what was already there, lifted from 9 to 11 by the 2026-11-02 batch.
pub const TEXT_GLYPH_CAPTION: f32 = 11.0;
/// The floor of the type ladder as it was priced: metadata, units, the word
/// inside a chip. A 12px line is the size below which a 4K panel at 100% scale
/// stops being readable, so quieter sites step *up*. It is no longer the bottom
/// rung - [`TEXT_GLYPH_CAPTION`] sits under it, hand-typed at 2 sites - and the
/// floor claim now lives in that doc, where the exception is measurable.
pub const TEXT_MICRO: f32 = 12.0;
/// Dense labels: the property table's rows, a field's caption. One step under
/// [`TEXT_BODY`] because a dock panel with forty rows of 14px text cannot be
/// scanned; the step is in the type, not in the colour.
pub const TEXT_LABEL: f32 = 13.0;
/// Field text: the app's default control size.
pub const TEXT_BODY: f32 = 14.0;
/// The line that sits under a caption and above a field value: a readout,
/// a file name in a list, a version string. 21 call sites across 11 files
/// spelled 13 by hand, which is why the rung is priced here rather than the
/// 21 screens each keeping their own number.
///
/// The rung is a *dense-tier* neighbour, not a headline step: 14 -> 15 -> 17
/// steps by one or two pixels and is exempt from the 1.25 floor the headline
/// tier is held to in `the_scale_is_monotonic_and_the_shadow_grows_with_lift`
/// for the same reason 17 -> 18 already is - these roles are separated by ink
/// and position, not by size. Its own place in the order is pinned in that test
/// rather than left to this paragraph, because a rung nothing asserts can be
/// silently reordered by the next run that finds it inconvenient.
/// Signed 2026-10-19 by the board owner as a 0-px drain of column (d1);
/// the alternative was shrinking 21 lines of already-shipped text by 1 px.
pub const TEXT_META: f32 = 15.0;
/// Section headings inside a panel.
pub const TEXT_HEADING: f32 = 17.0;
/// Panel titles (Recent Documents, Properties).
pub const TEXT_SECTION: f32 = 18.0;
/// The title line of a settings pane: `Auto Constrain` in the constrain dialog.
///
/// This is [`TEXT_SECTION`]'s role at 19, one pixel up. The rung exists only to
/// drain a hand-typed number at 0 px; it is *not* a ruling that a pane title is
/// 1 px louder than a panel title, and the guard says so by pinning the whole
/// 18/19/20/24 cluster rather than letting each rung keep its own paragraph.
/// #58 owns collapsing it: folding this onto SECTION moves 1 px, which a rename
/// batch is not allowed to do. Signed 2026-10-19.
pub const TEXT_PANE_HEAD: f32 = 19.0;
/// The biggest line inside a compact surface: 20 px, and the app puts three
/// different jobs on it - the name a style dialog edits itself by
/// (`ui/style/point_style.rs`), the numeric value of a metric readout under a
/// [`TEXT_MICRO`] label (`ui/window/recovery.rs`), and a character drawn as an
/// icon (`"⇄"` in `ui/window/gradient_editor.rs`).
///
/// The name is deliberately about altitude, not about a job, because no single
/// job owns this number. Two of the three uses collide with a rung that already
/// exists: a dialog heading is [`TEXT_SECTION`]'s job per [`TEXT_TITLE`]'s doc
/// (20 -> 18 is a 2 px move) and a glyph-as-icon arguably belongs to
/// [`crate::ui::kit::Role::Icon`] geometry rather than to the type ladder. Both
/// go to #58; landing them here moves 0 px and keeps the number honest.
pub const TEXT_FOCAL: f32 = 20.0;
/// A window's own headline: the title that names the whole surface, not a
/// section inside it.
///
/// Three call sites spelled 20 by hand (Missing-fonts, the recovery dialog's
/// heading and its prompt). Window titles, not
/// dialog titles: a dialog's heading is a panel title and stays on
/// [`TEXT_SECTION`]. The rung was lifted to 24 on 2026-11-02 with the rest of
/// the ladder.
///
/// The original paragraph here claimed "a rung below this would collapse it onto
/// [`TEXT_SECTION`] (15 → 18 is a 1.2 ratio, under the ladder's 1.25 floor), and
/// 20/15 = 1.33 clears it, which is why this is the step and not a smaller one".
/// That was an inference from prose, and #42 batch 032-B checked it against the
/// law: the 1.25 floor is asserted over two named transitions
/// (`TEXT_TITLE / TEXT_SECTION`, `TEXT_DISPLAY / TEXT_TITLE`), not over every
/// adjacent pair, and 17 → 18 already steps 1 px inside it. So [`TEXT_PANE_HEAD`]
/// (19) and [`TEXT_FOCAL`] (20) do exist below this rung. They are 0-px drains
/// of numbers that ship, each documented with the role it borrows, and the
/// cluster they form is pinned in
/// `the_scale_is_monotonic_and_the_shadow_grows_with_lift`
/// so a fifth step cannot join on the strength of a paragraph either.
pub const TEXT_TITLE: f32 = 24.0;
/// The product name in the About hero, 32 px.
///
/// The same job [`crate::ui::kit::display`] does on the start page at
/// [`TEXT_DISPLAY`], one tier down, because the About hero is a card inside a
/// window rather than a page. The rung is named for that surface (the hero band,
/// the same word `TINT_HERO` uses for its wash) and not called a wordmark:
/// `kit::display`'s doc owns that word and the name it carries, and two rungs
/// claiming one job would be the defect #58 is for. Folding About up to
/// [`TEXT_DISPLAY`] moves 10 px, so it waits for a sample; landing it here moves
/// 0. Signed 2026-10-19.
pub const TEXT_HERO: f32 = 32.0;
/// The wordmark. The one rung above the panel ladder, used only where the app
/// introduces itself (the start page) — a size decided here rather than a
/// number typed into a screen. [`TEXT_HERO`] is the smaller sibling that the
/// About hero spells by hand; this rung belongs to [`crate::ui::kit::display`].
pub const TEXT_DISPLAY: f32 = 42.0;

/// Elevation for a chrome surface. `lift` is a step, not pixels: 0 sits flush,
/// 1 is a resting control, 2 is a hovered control, 3 is a floating overlay.
///
/// The DraftFlow schemes are drawn with a single pen: depth comes from the
/// grey ladder and a hairline, never from a cast shadow. Every other theme
/// keeps the vendor-style halo.
///
/// A black shadow is what reads on a light scheme; on a dark one it vanishes
/// against a near-black background, so the dark branch pushes the alpha and
/// blur up until the edge registers as depth instead of nothing.
pub fn elevation(theme: &Theme, lift: f32) -> iced::Shadow {
    if super::draftflow_theme::draftflow_colors(theme).is_some() {
        return iced::Shadow::default();
    }
    let lift = lift.clamp(0.0, 3.0);
    let palette = theme.palette();
    let (color, blur) = if palette.is_dark {
        // `scale_alpha` does not clamp, so the steepest step of the ladder has
        // to stay inside 1.0 on its own — an out-of-gamut alpha draws a solid
        // black slab behind the panel instead of a soft falloff.
        (Color::BLACK.scale_alpha(0.18 * lift), 2.0 + 4.0 * lift)
    } else {
        (
            palette.background.base.text.scale_alpha(0.10 * lift),
            2.0 + 2.5 * lift,
        )
    };
    iced::Shadow {
        color,
        offset: Vector::new(0.0, lift.max(0.5)),
        blur_radius: blur,
    }
}

/// The four steps [`elevation`] already documents in prose, as names.
///
/// `elevation` takes a *step*, not pixels, and its doc says so: 0 sits flush, 1
/// is a resting control, 2 is a hovered one, 3 is a floating overlay. Until the
/// ratchet's (h) column counted the bare literals handed to it, that sentence
/// was the only place the ladder existed — a screen could read `elevation(2.0)`
/// and had to guess whether 2 was a step, a radius, or an alpha. Naming the
/// four documented steps moves nothing: every call site keeps its number, it
/// just arrives as a word.
pub const LIFT_FLUSH: f32 = 0.0;
/// A control at rest: raised enough to separate its plate from the surface it
/// sits on, which is what a chrome strip is asking for.
pub const LIFT_REST: f32 = 1.0;
/// The engaged step — hovered, pressed, or currently selected. A selected tab
/// and its panel opener ride this one: the thing you are pointing at and the
/// thing that answers for the current selection are the same tier of raise.
pub const LIFT_HOVER: f32 = 2.0;
/// A floating overlay, the top of the ladder: a popup, a dropdown, a dialog.
pub const LIFT_FLOAT: f32 = 3.0;

/// The 1px edge that separates a control from the surface it sits on. Slightly
/// brighter than the palette's `neutral` on a dark scheme, which is what makes
/// a raised control legible when there is no shadow to lean on.
pub fn edge(theme: &Theme) -> Color {
    if let Some(colors) = super::draftflow_theme::draftflow_colors(theme) {
        return colors.line;
    }
    if palette_of(theme).is_dark {
        control_plate(theme, Control::Strong)
    } else {
        control_plate(theme, Control::Neutral)
    }
}

// ── The control plate ladder: the neutral steps a control is mounted on ──────

/// The seven amounts iced's `Background::new` deviates its page by, in OKLCH-L.
///
/// These are iced's numbers, transcribed on purpose: decision A's whole claim is
/// that L0 takes over the *naming* of the ramp its controls already spend, not
/// that L0 invents a new one. `the_control_plate_ladder_is_exact` compares
/// [`control_plate`] with `theme.palette().background.<slot>.color` for all 26
/// selectable themes × 7 slots, so if a step here drifts from the vendor's, that
/// guard fails on the first theme rather than quietly moving every neutral
/// button in the app.
pub const CONTROL_STEPS: [f32; 7] = [0.03, 0.07, 0.10, 0.125, 0.15, 0.175, 0.20];

/// A step of the control-plate ladder, named by role-ish amount instead of by
/// iced's private field names.
///
/// WHY THIS EXISTS (decision A, measured 2026-10-17, `all_themes()` = 26). Every
/// neutral control in the kit — `Role::Standard`'s rest and press plates, the
/// chip, the pill, `mark_plate_style`, the modal scrim, the canvas' neutral
/// badge, and L0's own `edge` — took its pixel by reaching into
/// `theme.palette().background.{weakest,weaker,weak,neutral,strong,,strongest}`,
/// i.e. L1 was borrowing iced's private ramp. The obvious fix, "just use the
/// surface ladder", is measurably wrong: `weak` → `Rank::Card` costs up to 21.57
/// CIE L* and `strong` → `Rank::Float` up to 30.84, because [`surface`] picks its
/// direction from the page's white headroom while iced's `deviate` picks it from
/// `is_dark`. Three themes (Tokyo Night Light, Kanagawa Lotus, DraftFlow Light)
/// put the two on opposite sides of their own page. That is a defect in the tier
/// ladder's direction rule and it is tracked separately, at #48.
///
/// What A can do without moving a pixel is own the arithmetic *call*: keep iced's
/// `deviate`, give it names, and let the kit stop spelling iced's fields. Perceptual
/// equalization — one true ΔL* step for every theme — is the other candidate, and
/// it costs up to 2.60 L* on the themes whose ramp is cramped; that is a redesign
/// and gets sample shots, not a refactor commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    /// The faintest plate: a quiet button at rest.
    Weakest,
    /// One step up: a quiet button under the pointer.
    Weaker,
    /// The resting plate of a real control.
    Weak,
    /// Between rest and press: also L0's own hairline on a light scheme.
    Neutral,
    /// The pressed plate, and the hairline on a dark scheme.
    Strong,
    /// Above `Strong`; nothing in the kit spends it yet.
    Stronger,
    /// The deepest step iced defines, which is what the modal scrim paints.
    Strongest,
}

impl Control {
    /// Index into [`CONTROL_STEPS`]. Kept as a match so adding a variant cannot
    /// silently reuse a neighbour's amount.
    #[must_use]
    pub fn step(self) -> usize {
        match self {
            Control::Weakest => 0,
            Control::Weaker => 1,
            Control::Weak => 2,
            Control::Neutral => 3,
            Control::Strong => 4,
            Control::Stronger => 5,
            Control::Strongest => 6,
        }
    }

    /// The vendor amount behind this plate.
    #[must_use]
    pub fn deviate(self) -> f32 {
        CONTROL_STEPS[self.step()]
    }

    /// iced's own field name for this plate, for diagnostics and for the guard
    /// that resolves slot names out of the vendored struct.
    #[must_use]
    pub fn vendor_slot(self) -> &'static str {
        match self {
            Control::Weakest => "weakest",
            Control::Weaker => "weaker",
            Control::Weak => "weak",
            Control::Neutral => "neutral",
            Control::Strong => "strong",
            Control::Stronger => "stronger",
            Control::Strongest => "strongest",
        }
    }
}

/// The control plate at `step`, derived from the theme's own page.
///
/// This calls the vendor's `deviate` rather than reproducing it. Reproducing it
/// would mean owning iced's OKLCH round-trip *and* its chroma bump (whose comment
/// in `vendor/iced/core/src/theme/palette.rs` reads, verbatim, "Formula empirically
/// and cluelessly derived"), and owning a formula I cannot justify is worse than
/// naming one. Measured consequence: bit-exact with the palette pair on 26/26
/// themes, 0.00 L* moved.
#[must_use]
pub fn control_plate(theme: &Theme, step: Control) -> Color {
    // Read the built slot rather than re-deriving it: iced stored exactly
    // `deviate(base, k)` in every built-in palette, while a hand-written
    // DraftFlow palette stores the sheet's named grey. Reading the slot is the
    // only form that is bit-exact for both.
    let bg = &palette_of(theme).background;
    match step {
        Control::Weakest => bg.weakest.color,
        Control::Weaker => bg.weaker.color,
        Control::Weak => bg.weak.color,
        Control::Neutral => bg.neutral.color,
        Control::Strong => bg.strong.color,
        Control::Stronger => bg.stronger.color,
        Control::Strongest => bg.strongest.color,
    }
}

/// The fill of a selected row or cell.
///
/// The sheet selects with a neutral plate (`sel-row`), not with a tinted
/// brand wash: vermilion is reserved for the drawing selection and focus.
/// Vendor themes keep the accent wash the app has always used.
#[must_use]
pub fn selection_fill(theme: &Theme) -> Color {
    if let Some(colors) = super::draftflow_theme::draftflow_colors(theme) {
        return colors.selected_row;
    }
    palette_of(theme).primary.weak.color.scale_alpha(TINT_SELECTED)
}

/// The fill of a selected row while the pointer is down on it.
#[must_use]
pub fn selection_press_fill(theme: &Theme) -> Color {
    if let Some(colors) = super::draftflow_theme::draftflow_colors(theme) {
        return colors.active;
    }
    palette_of(theme).primary.weak.color.scale_alpha(TINT_ENGAGED)
}

/// The edge of an editable field. The sheet gives fields `line-strong`, one
/// step above the hairline that separates two chrome planes.
#[must_use]
pub fn field_edge(theme: &Theme) -> Color {
    if let Some(colors) = super::draftflow_theme::draftflow_colors(theme) {
        return colors.line_strong;
    }
    edge(theme)
}

/// The keyboard-focus ring: the sheet's one saturated signal colour.
///
/// The sheet spends vermilion on exactly three things — a selection in the
/// drawing, keyboard focus, and the current value of a slider. Every other
/// engagement is carried by the ink or the neutral plate. Vendor themes keep
/// the aurora edge they have always used.
#[must_use]
pub fn focus_ring(theme: &Theme) -> Color {
    if let Some(colors) = super::draftflow_theme::draftflow_colors(theme) {
        return colors.accent;
    }
    aurora_edge(theme)
}

/// The ink iced solved for [`control_plate`]'s pixel.
///
/// READ, NOT RE-DERIVED, AND THAT IS A MEASURED LIMIT. `Pair::new` does not store
/// the text it was handed; it stores `readable(color, text)`, which is why 20 of
/// 26 themes show more than one ink across the eight background slots. Re-running
/// that solve from L0 (`Pair::new(deviate(page, k), base.text)`) is bit-exact for
/// only 22–25 of the 26 themes and misses by up to 10.12 L*, because the original
/// `text` that `Background::new` received is not recoverable from a built
/// `Palette` — `background.base.text` is already the corrected one. So the plate
/// ladder owns the surface and reads the ink, and the ink stays iced's until L0
/// has its own contrast solver in front of every control (which is decision #73's
/// territory, not this one).
#[must_use]
pub fn control_ink(theme: &Theme, step: Control) -> Color {
    let bg = &palette_of(theme).background;
    match step {
        Control::Weakest => bg.weakest.text,
        Control::Weaker => bg.weaker.text,
        Control::Weak => bg.weak.text,
        Control::Neutral => bg.neutral.text,
        Control::Strong => bg.strong.text,
        Control::Stronger => bg.stronger.text,
        Control::Strongest => bg.strongest.text,
    }
}

/// `theme.palette()` under one name, so the helpers above read the same way.
fn palette_of(theme: &Theme) -> &iced::theme::Palette {
    theme.palette()
}

// ── The surface ladder: one perceptual scale for every plane ──────────────

/// How far each raised plane stands above the page, in **cumulative** CIE L* —
/// `SURFACE_LADDER[2]` is `Card` above the *window*, not above `Plate`.
///
/// The resulting gaps between rungs are 4.0 / 3.5 / 5.5 / 4.5 L*. They are
/// solved, not eyeballed: `D:\tmp\ladder-check.mjs` runs this table against
/// every theme in `app::config::all_themes()` and reports each scheme's plane
/// colors, adjacent gaps and ink contrast. The floor the kit holds itself to is
/// 1.5 L* between neighbours (asserted in `kit_contract_tests`); the rungs sit
/// comfortably above it while the whole climb stays inside the contrast budget
/// of the scheme's own text.
pub const SURFACE_LADDER: [f32; 4] = [4.0, 7.5, 13.0, 17.5];

/// How far a recess sits *below* the page, same unit, same solve.
pub const RECESS_LADDER: [f32; 2] = [2.5, 5.5];

const SHALLOW: usize = 0;
const DEEP: usize = 1;

/// A page this close to pure white has nowhere left to lift, so [`surface`]
/// turns the ladder around and separates planes by sinking them instead.
/// iced's own `Theme::Light` (#FFFFFF) is the case this exists for.
const MIN_LIFT_HEADROOM: f32 = 6.0;

/// The last L* of any range is left unspent: a plane landing exactly on white
/// or black has no gradient left for a shadow to catch, and a further rung
/// above it would be unreachable.
const LADDER_RESERVE: f32 = 1.0;

/// The ladder may not eat so much of the page-to-ink range that the scheme's
/// own body text drops below WCAG AA on a raised plane.
const MIN_INK_CONTRAST: f32 = 4.5;

/// The raised planes of the interface, named by role instead of by amount.
///
/// The order *is* the ladder: `Band` under `Plate` under `Card`, with `Float`
/// above all of them. Taking a role rather than a float is what stops each
/// screen from voting with a private opinion about how bright its own widget
/// should be — before this, four call sites each passed their own `step` and
/// the ladder silently collapsed. Measured 2026-10-15 on the old formula, the
/// light scheme's four rungs landed at L* 96.57 / 96.61 / 97.04 / 97.18: a
/// total span of 0.6 L*, i.e. four named tiers drawing one plane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rank {
    /// A full-width chrome strip: quick bar, tab strip, status bar.
    Band,
    /// A group plate inside chrome: a ribbon tool group.
    Plate,
    /// A resting card: dialog body, dock panel, recent-file tile.
    Card,
    /// Detached from the document: popup, tooltip, modal.
    Float,
}

impl Rank {
    /// Index into [`SURFACE_LADDER`].
    #[must_use]
    pub fn rung(self) -> usize {
        match self {
            Rank::Band => 0,
            Rank::Plate => 1,
            Rank::Card => 2,
            Rank::Float => 3,
        }
    }
}

/// Which end of the luminance range a plane moves toward.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
}

impl Direction {
    fn sign(self) -> f32 {
        match self {
            Direction::Up => 1.0,
            Direction::Down => -1.0,
        }
    }
}

/// L* still available from the page in this direction after both hard limits:
/// the edge of the gamut, and the point where the scheme's own text stops
/// clearing [`MIN_INK_CONTRAST`] on the plane.
fn room(theme: &Theme, dir: Direction) -> f32 {
    let palette = theme.palette();
    let base = palette.background.base.color;
    let ink = palette.background.base.text;
    let page = common::lstar(base);
    let gamut = match dir {
        Direction::Up => 100.0 - page,
        Direction::Down => page,
    };
    let y_base = common::wcag_luminance(base);
    let y_ink = common::wcag_luminance(ink);
    let bound_y = match (dir, y_ink >= y_base) {
        // The plane climbs toward the ink: it must stop while the ratio holds.
        (Direction::Up, true) => ((y_ink + 0.05) / MIN_INK_CONTRAST - 0.05).clamp(0.0, 1.0),
        // The plane climbs away from the ink, or sinks away from it: contrast
        // only improves, so the gamut is the only binding limit.
        (Direction::Up, false) => 1.0,
        (Direction::Down, true) => 0.0,
        // The plane sinks toward a darker ink (a recess on a light page).
        (Direction::Down, false) => (MIN_INK_CONTRAST * (y_ink + 0.05) - 0.05).clamp(0.0, 1.0),
    };
    let contrast = (common::luminance_to_lstar(bound_y) - page).abs();
    (gamut.min(contrast) - LADDER_RESERVE).max(0.0)
}

/// A recessed band: a strip that reads as set *into* the window rather than
/// laid on top of it — the ribbon's tool area today, and anything else that
/// has to separate itself from the chrome around it without a border.
///
/// The palette cannot supply this. Every derived background step (`weak` …
/// `strong`) deviates *away* from `base` in the same direction, so a dark
/// scheme has no built-in surface darker than its own background; the recess
/// therefore moves *down* the same measured L* scale [`surface`] moves up,
/// which is what makes the cut identical in both schemes. The old hand-tuned
/// mix ratios were not: `sunk`'s 0.35 toward black moved the dark page 2.18 L*
/// while its 0.11 moved the light page 4.20 L*, and `inset`'s pair moved them
/// 1.00 and 1.87 — so "one notch down" meant two different things, and on the
/// dark scheme the shallow notch was below the 1.5 L* floor the kit holds its
/// own planes to.
pub fn sunk(theme: &Theme) -> Color {
    if let Some(colors) = super::draftflow_theme::draftflow_colors(theme) {
        return colors.shell;
    }
    plane(theme, -RECESS_LADDER[DEEP] * recess_scale(theme))
}

/// A shallow recess: fields, the command line, list wells. The same direction
/// as [`sunk`] at less than half the distance, so a form reads as inset without
/// becoming a hole in the page.
pub fn inset(theme: &Theme) -> Color {
    // The sheet draws an editable field as a sheet-white plate with a
    // `line-strong` edge rather than a darker well, so the recess is a surface
    // token here, not a mix toward black.
    if let Some(colors) = super::draftflow_theme::draftflow_colors(theme) {
        return colors.surface;
    }
    plane(theme, -RECESS_LADDER[SHALLOW] * recess_scale(theme))
}

/// How much of the recess ladder the page can actually spend sinking, using the
/// same budget rule as [`surface`]: a cramped scheme gets a compressed ladder,
/// never two planes that clamp onto the same colour. Measured 2026-10-15 on
/// iced's own `Moonfly` (page L\* 2.19), the unscaled rungs both hit the black
/// clamp at L\* 0.00 — a well and a sink were the same pixel, so every field on
/// that theme vanished into its own background. Scaled they land at 1.65 and
/// 1.00: 0.54 and 0.65 L\* apart, still legible as separate planes.
fn recess_scale(theme: &Theme) -> f32 {
    (room(theme, Direction::Down) / RECESS_LADDER[DEEP]).min(1.0)
}

/// The raised plane at `rank` above the current theme's page.
///
/// Implemented as a closed-form inverse of `Color::mix` rather than a search:
/// mixing toward white is affine in linear luminance
/// (`Y(t) = Y_base + t·(1 − Y_base)`), so a target L* maps back to a mix ratio
/// in one division. No loop, no cached table, no per-frame cost.
pub fn surface(theme: &Theme, rank: Rank) -> Color {
    if let Some(colors) = super::draftflow_theme::draftflow_colors(theme) {
        return colors.raised()[rank.rung()];
    }
    let page = common::lstar(theme.palette().background.base.color);
    let dir = if 100.0 - page >= MIN_LIFT_HEADROOM {
        Direction::Up
    } else {
        Direction::Down
    };
    // Spend the ladder in proportion to the room this scheme actually has, so a
    // cramped third-party palette degrades to a compressed ladder instead of
    // silently dropping rungs.
    let top = SURFACE_LADDER[SURFACE_LADDER.len() - 1];
    let scale = (room(theme, dir) / top).min(1.0);
    plane(theme, SURFACE_LADDER[rank.rung()] * scale * dir.sign())
}

/// A flat colour `l` CIE L* away from the page; negative is recessed. The only
/// place that turns a perceptual distance into a mix ratio.
fn plane(theme: &Theme, l: f32) -> Color {
    let base = theme.palette().background.base.color;
    let dir = if l >= 0.0 { Direction::Up } else { Direction::Down };
    let target = (common::lstar(base) + l).clamp(0.0, 100.0);
    let y_base = common::wcag_luminance(base);
    let y_target = common::lstar_to_luminance(target);
    // Invert the linear-light lerp: toward white the span is (1 − Y_base),
    // toward black it is Y_base.
    let t = match dir {
        Direction::Up => (y_target - y_base) / (1.0 - y_base),
        Direction::Down => (y_base - y_target) / y_base,
    };
    let toward = match dir {
        Direction::Up => Color::WHITE,
        Direction::Down => Color::BLACK,
    };
    base.mix(toward, t.clamp(0.0, 1.0))
}

// ── Aurora: the brand gradient ("light in flow") ──────────────────────────

/// The bright end of the aurora pair: blue flowing into violet along a
/// control's diagonal. Each scheme keeps its own step so the gradient holds
/// contrast against its chrome — and under white ink, which the primary
/// button paints straight onto the fill: the original 0x4B9EF1 measured only
/// 2.8:1 against white at this end, so the dark step sits one notch deeper.
pub fn aurora_hi(theme: &Theme) -> Color {
    if theme.palette().is_dark {
        Color::from_rgb8(0x44, 0x93, 0xEA)
    } else {
        Color::from_rgb8(0x24, 0x78, 0xCC)
    }
}

/// The deep end of the aurora pair.
pub fn aurora_lo(theme: &Theme) -> Color {
    if theme.palette().is_dark {
        Color::from_rgb8(0x62, 0x5F, 0xE6)
    } else {
        Color::from_rgb8(0x4A, 0x4A, 0xC8)
    }
}

/// Diagonal aurora fill scaled to `alpha` — the signature reserved for
/// engaged states: the active tool, the primary action, the focused field.
pub fn aurora_gradient(theme: &Theme, alpha: f32) -> Background {
    let linear = iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_4))
        .add_stop(0.0, aurora_hi(theme))
        .add_stop(1.0, aurora_lo(theme));
    Background::Gradient(iced::Gradient::Linear(linear.scale_alpha(alpha.clamp(0.0, 1.0))))
}

/// The single-colour stand-in for the aurora on 1px accents (borders, ticks,
/// focus rings), where a gradient cannot be expressed.
pub fn aurora_edge(theme: &Theme) -> Color {
    aurora_hi(theme)
}

/// How much the backdrop's colour is pushed once it is read through glass.
/// 1.7 is the `saturate(1.7)` the batch-046 sheets pair with the blur; a blurred
/// backdrop without it reads as grey mud, which is why the blur and the
/// saturation are one decision and not two.
pub const GLASS_SATURATE: f32 = 1.7;

/// The glass fill. Light takes the *thin* rung, dark the *thick* one: 62% white
/// over a bright canvas, 72% near-black over a near-black one. The dark side is
/// a warm black and not the navy/slate that a translucent panel usually reaches
/// for, because a cool wash under white ink kills the ink's contrast.
///
/// This is also the degradation path: the alpha is high enough that a pane which
/// cannot blur still holds its own text. It is not a shadow.
pub fn glass_tint(theme: &Theme) -> Color {
    if theme.palette().is_dark {
        Color {
            r: 0.054_9,
            g: 0.062_7,
            b: 0.074_5,
            a: 0.72,
        }
    } else {
        Color::WHITE.scale_alpha(0.62)
    }
}

/// The 1px rim that makes a glass pane read as an object rather than a smudge.
/// The light scheme needs the full 0.20 — it is the single attribute the
/// reference is unmistakably built from — and the dark scheme pulls it to 0.12
/// so the rim does not become the brightest thing on screen.
pub fn glass_edge(theme: &Theme) -> Color {
    if theme.palette().is_dark {
        Color::WHITE.scale_alpha(0.12)
    } else {
        Color::WHITE.scale_alpha(0.20)
    }
}

/// The top-face specular, as the two ends of a vertical ramp across the pane.
/// Kept inside a 6-point alpha spread: enough to read as a lit top edge, not
/// enough to be a decorative gradient — and deliberately *not* the aurora, so
/// glass never claims a fifth aurora role.
pub fn glass_hi(theme: &Theme) -> (Color, Color) {
    if theme.palette().is_dark {
        (
            Color::WHITE.scale_alpha(0.06),
            Color::WHITE.scale_alpha(0.01),
        )
    } else {
        (
            Color::WHITE.scale_alpha(0.08),
            Color::WHITE.scale_alpha(0.02),
        )
    }
}

// ── Painted plates: the colour math that keeps a label readable ───────────

/// The two inks that can sit on a **painted** plate.
///
/// Ink on a *surface* comes from the palette ramp (`background.base.text` and
/// friends). Ink on a saturated plate cannot: there the plate, not the scheme,
/// decides what is legible, and this app ships 26 selectable palettes so the
/// palette's own answer is not one number. The 2026-10-15 probe of
/// `kit::style_for` across all of them measured 1.85:1 for exactly that
/// shortcut (`danger.base.text` painted on `danger.weak.color`).
pub const PLATE_INK_LIGHT: Color = Color::WHITE;
/// `#121217`, not pure black: on a saturated violet a pure-black label reads as
/// a hole punched through the button rather than as ink.
pub const PLATE_INK_DARK: Color = Color::from_rgb(0.07, 0.07, 0.09);

/// The bar an interactive plate has to clear for its label. The guard
/// `ui::theme_accessibility_tests::kit_roles_hold_aa_contrast` asserts with this
/// same constant, so the number the paint is built to and the number it is
/// checked against cannot drift apart.
pub const AA_INK_CONTRAST: f32 = 4.5;

/// The bar a **non-text** part of a component has to clear against the ground
/// next to it — a border, a status icon, a ring (WCAG 1.4.11). Kept beside
/// [`AA_INK_CONTRAST`] because a plate's edge and a plate's ink are decided in
/// the same call and must not be decided against two different invented bars.
pub const AA_COMPONENT_CONTRAST: f32 = 3.0;

/// How far past the bar the paint aims. Not a safety blanket: the pull is a
/// closed form, so aiming at 4.5 lands the plate at `1.05 / (4.5)` to within
/// float noise and the guard then measures 4.4998 on its own round trip. The
/// first run of that guard reported every dark theme's hovered hero as a
/// failure at "4.50:1" for exactly this reason.
pub const AA_INK_HEADROOM: f32 = 0.3;

/// The minimum aim: how far past the bar the fallback pull has to reach, in
/// contrast units. [`AA_INK_HEADROOM`] is the preferred aim and is out of range
/// on a mid-tone plate (the band where [`legible_bound`] answers `None`), and
/// falling back to exactly `min` there reproduces the float-noise failure the
/// headroom exists to avoid. Measured 2026-10-16: Moonfly's danger recess sits
/// in that band, the fallback landed on the bar, and
/// `the_error_field_reads_on_its_own_danger_recess` reported its placeholder at
/// 4.4998:1 against its own 4.5:1 check.
///
/// The size is bounded by arithmetic, not taste. A plate closes *both*
/// directions at once when `1.05 / target - 0.05 < 0.05 * target - 0.05`, i.e.
/// when `target > √21 ≈ 4.583`; so `AA_INK_CONTRAST + AA_INK_SLACK` must stay
/// under that ceiling for the fallback to be always reachable, and under
/// `AA_INK_CONTRAST + AA_INK_HEADROOM` to ever be reached past. 0.05 puts the
/// aim at 4.55, inside both.
pub const AA_INK_SLACK: f32 = 0.05;

/// The luminance a plate may not exceed if [`PLATE_INK_LIGHT`] is to clear
/// `min`: invert `(1.0 + 0.05) / (Y + 0.05) >= min`.
fn light_ink_ceiling(min: f32) -> f32 {
    1.05 / min - 0.05
}

/// ... and the luminance it must reach for [`PLATE_INK_DARK`].
fn dark_ink_floor(min: f32) -> f32 {
    let ink = common::wcag_luminance(PLATE_INK_DARK);
    min * (ink + 0.05) - 0.05
}

/// The deepest rung any painted plate's own ladder reaches, which is what the
/// bounds are sized against. A dip toward black helps a light ink and hurts a
/// dark one, so only the dark floor has to be divided back out.
const PLATE_LADDER_DEEPEST_DIP: f32 = HERO_DIP_PRESS;

/// Move `c` to WCAG luminance `y` without moving its colour.
///
/// Two statements, and only Oklab can make them separately: `L` is the
/// lightness, `(a, b)` is the colour. So this holds the `a`/`b` *vector* and
/// walks `L`, which means the hue cannot drift by construction rather than by
/// luck, and the chroma is kept unless the display physically cannot show it.
///
/// What it replaces was a straight line in linear light toward black or white
/// (`Color::mix`), and that line was not a lightness move in one of the two
/// directions: toward black every linear channel drops by the same factor, so
/// the colour happened to survive; toward white the small channels rise
/// proportionally more than the large one, so the colour paled *and* rotated.
/// The 2026-10-16 dump measured it over the 26 selectable themes — Kanagawa
/// Lotus `C 0.0862 → 0.0162` (`C/L` ×0.114), Catppuccin Latte hue `+14.1°`,
/// Tokyo Night's pulled band down to ×0.445 of its chroma — and the doc here
/// claimed "Chroma is untouched" while it did that. The comparable figure for
/// this implementation, measured 2026-10-16 over 1332 pulls across 12 hues, 5
/// chroma levels and 4 lightness levels: worst hue move **0.13°** on any pull
/// whose endpoints carry readable chroma (both above `C 0.01`), worst landing
/// error `|Y − y| = 6.6e-7`, and the old line on the same sweep moved a hue by
/// up to **17.59°**. Where a pull is near-neutral at either end the hue angle is
/// `atan2` of a zero vector and is not reported at all (see
/// [`crate::ui::style::common::oklch_of`]).
///
/// The one cost this form has is the display's own limit: on a fixed hue ray,
/// sRGB runs out of room as `L` approaches white, so a band pulled that far
/// loses chroma it cannot keep anywhere. That is a bound, not a choice, and it is
/// searched rather than assumed — `the_pull_keeps_the_colour_it_was_given`
/// recomputes the same ceiling independently and fails if this function gives up
/// more than [`CHROMA_SLACK`] under it.
#[must_use]
pub fn pull_to_luminance(c: Color, y: f32) -> Color {
    let yc = common::wcag_luminance(c);
    if (yc - y).abs() < 1e-5 {
        return c;
    }
    let [l0, a, b] = common::oklab_of(c);

    // The ray's own limit, stated *without* the tolerance
    // [`common::oklab_in_gamut`] carries. That `EPS` is right for a report — it
    // absorbs the `powf` round trip so a real colour is not called impossible —
    // but here a channel sitting at `-5e-5` would still be clamped by
    // [`common::oklab_to_color`], and clamping one channel is a *different*
    // colour vector: the 2026-10-16 run measured Solarized Dark's warning swatch
    // pulled to `Y 0.02` landing on `#010000` with its hue rotated 2.45° for
    // exactly that reason. Strict here costs at most `EPS` of chroma and makes
    // the hue preservation total rather than approximate.
    let fits = |lab: [f32; 3]| -> bool {
        common::oklab_channels(lab)
            .iter()
            .all(|v| (0.0..=1.0).contains(v))
    };

    // The lightness that lands on `y` once the colour vector is scaled by
    // `gamma`. Bisection, not the closed form the mix line had: that one was
    // only one division *because* it moved along a line the perceptual axes
    // happen to be straight on. `L` is monotone in luminance along a fixed ray,
    // so 32 halvings put the landing inside 7e-10 of `y` — well under the
    // 4.4998:1 rounding band [`AA_INK_SLACK`] exists to absorb.
    let solve = |gamma: f32| -> f32 {
        let (mut lo, mut hi) = if y < yc { (0.0f32, l0) } else { (l0, 1.0) };
        for _ in 0..32 {
            let mid = (lo + hi) * 0.5;
            let probe = common::oklab_to_color([mid, a * gamma, b * gamma]);
            if common::wcag_luminance(probe) < y {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        (lo + hi) * 0.5
    };

    // Fast path: the colour fits at full strength, which is every downward pull
    // and most upward ones.
    let mut gamma = 1.0;
    let mut l = solve(gamma);
    if !fits([l, a, b]) {
        // Widest colour vector still displayable on this ray, re-solving the
        // lightness each step because shrinking the chroma moves the
        // luminance-per-unit-L as well. The search assumes the sRGB slice at a
        // fixed `L` is star-shaped about its neutral point — true for every hue
        // the 2026-10-16 sweep crossed, and the guard's independent ceiling
        // search makes the same assumption, so a violation would show up as a
        // failing chroma law rather than as a quiet wrong answer.
        let (mut glo, mut ghi) = (0.0f32, 1.0f32);
        for _ in 0..20 {
            let mid = (glo + ghi) * 0.5;
            if fits([solve(mid), a * mid, b * mid]) {
                glo = mid;
            } else {
                ghi = mid;
            }
        }
        gamma = glo;
        l = solve(gamma);
    }
    common::oklab_to_color([l, a * gamma, b * gamma])
}

/// How much chroma [`pull_to_luminance`] may give up beyond the display's own
/// ceiling on the hue ray before the guard calls it a defect. One fortieth of a
/// channel step is the rounding the two `powf` conversions cannot avoid; this is
/// the slack the guard measures against, not a licence to wash a colour out.
#[cfg(test)]
pub(crate) const CHROMA_SLACK: f32 = 0.006;

/// Composite `ink` over an opaque `plate` the way the renderer draws it.
///
/// Source-over in the colour space the pixels are stored in, which is the rule
/// the whole AA suite already measures with
/// (`theme_accessibility_tests::composite_over`). L0 states it once so that a
/// guarantee written here and a guard written there cannot disagree about where
/// a translucent ink actually lands.
#[must_use]
pub fn over(ink: Color, plate: Color) -> Color {
    let a = ink.a.clamp(0.0, 1.0);
    Color {
        r: ink.r * a + plate.r * (1.0 - a),
        g: ink.g * a + plate.g * (1.0 - a),
        b: ink.b * a + plate.b * (1.0 - a),
        a: 1.0,
    }
}

/// Resolve an ink so that what actually gets painted clears `min` on `plate`.
///
/// The palette's own answer wins whenever it is already good enough: an ink that
/// measures out returns untouched, rung and all. Only a measured failure moves
/// anything, and then the move is [`pull_to_luminance`] to the nearer reachable
/// bound — which changes the lightness and nothing else the display can still
/// carry, in either direction, so a rung solved on a dark scheme names the same
/// colour as the same rung on a light one.
///
/// Alpha is resolved before the bounds are computed, because the renderer blends
/// a translucent ink in the space the pixels live in, not in linear light — so
/// what has to be measured is the composite, not the colour behind it. The
/// 2026-10-16 run of `the_error_field_reads_on_its_own_danger_recess` is what
/// fixed the direction of this: [`INK_PLACEHOLDER`] (0.62) of a near-black ink
/// over a near-white recess paints at 6.19:1 and keeps its rung, while the same
/// 0.62 of a mid red over a mid pink tint paints at 1.39:1 and cannot. Where the
/// rung is what failed the ink is pulled, opaque, to the bound — a visible
/// change, reported, never hidden.
///
/// If neither bound is reachable the plate sits in the dead band this file has
/// met before (see [`hero`]: `Y ≈ 0.19`, where no ink reaches 4.5:1); the
/// stronger extreme is returned so the caller's guard fails loudly instead of
/// shipping a colour that quietly cannot work.
#[must_use]
pub fn legible_on(ink: Color, plate: Color, min: f32) -> Color {
    let painted = over(ink, plate);
    if common::wcag_contrast(painted, plate) >= min {
        return ink;
    }

    let y_plate = common::wcag_luminance(plate);
    let y_ink = common::wcag_luminance(ink);
    let opaque = Color {
        r: ink.r,
        g: ink.g,
        b: ink.b,
        a: 1.0,
    };
    // Aim past the bar, by [`AA_INK_HEADROOM`], for the reason that constant was
    // measured: this is a closed form, so aiming at exactly `min` lands the ink
    // on `min` and the guard's own round trip then reports 4.4998:1. When even
    // the aim has no room on this plate, settle for [`AA_INK_SLACK`] past the
    // bar rather than jumping to a plate extreme — at `Y 0.18` a light red still
    // reaches 4.5:1 although 4.8:1 is out of range, and the hue is worth keeping.
    let pulled = legible_bound(y_ink, y_plate, min + AA_INK_HEADROOM)
        .or_else(|| legible_bound(y_ink, y_plate, min + AA_INK_SLACK))
        .map(|y| pull_to_luminance(opaque, y));
    if let Some(color) = pulled {
        return color;
    }

    // Neither direction reaches `min` at all: the plate sits in the dead band
    // this file has already met (see [`hero`]). Hand back the stronger extreme so
    // the caller's guard names the plate instead of shipping a colour that
    // quietly cannot work.
    if common::wcag_contrast(PLATE_INK_LIGHT, plate)
        >= common::wcag_contrast(PLATE_INK_DARK, plate)
    {
        PLATE_INK_LIGHT
    } else {
        PLATE_INK_DARK
    }
}

/// The luminance an ink has to reach to sit `target`:1 from a plate of
/// `y_plate`, taking the cheaper of the two directions — or `None` when neither
/// end of the ramp is open, which is the dead band [`legible_on`] documents.
fn legible_bound(y_ink: f32, y_plate: f32, target: f32) -> Option<f32> {
    // `(Y + 0.05) / (Yp + 0.05) >= target` and its reciprocal, each solved for Y:
    // the light bound above the plate and the dark bound below it.
    let above = target * (y_plate + 0.05) - 0.05;
    let below = (y_plate + 0.05) / target - 0.05;
    match (above <= 1.0, below >= 0.0) {
        (true, true) => {
            let start = common::luminance_to_lstar(y_ink);
            let up = (common::luminance_to_lstar(above) - start).abs();
            let down = (common::luminance_to_lstar(below) - start).abs();
            Some(if up <= down { above } else { below })
        }
        (true, false) => Some(above),
        (false, true) => Some(below),
        (false, false) => None,
    }
}

/// Paint a backdrop that carries `ink` at `min`, in `hue`'s own family.
///
/// The inverted case of [`legible_on`], and it exists because one widget cannot
/// be solved any other way: a text selection is painted *under* the value, in the
/// value's single fixed colour — iced's `text_input` has no per-run ink. So the
/// highlight has to move to the far side of the ink rather than the ink moving to
/// the highlight. Measured on 2026-10-16 across the selectable themes, the
/// brand's own 0.5 accent wash over a danger recess painted the refused value at
/// 1.63:1 (Nord) to 3.30:1 (Light): a highlight that swallows the text it is
/// supposed to be marking. The same run over the three neutral field styles came
/// out worse — 112 of 156 marked-text checks under AA: 16 of 26 themes for the
/// editable field (2.45:1 Tokyo Night to 4.24:1 Nord), 22 of 26 for the
/// read-only one (1.92:1 Tokyo Night), and all 26 for a disabled command log,
/// where the plate and the wash are both translucent and the faded ink had
/// nowhere to go. That measurement is what moved the whole field family here.
///
/// `ink` is used by its luminance, so it has to be the colour that actually gets
/// painted: a caller holding a faded rung as an alpha resolves it through
/// [`over`] first, because an ink that lets its backdrop through cannot be
/// carried by any single backdrop — it would have to solve for itself.
///
/// `hue` supplies the identity, the same cheaper-direction rule as
/// [`legible_bound`] supplies the luminance, and the result is opaque: a
/// translucent backdrop would leave the plate in the arithmetic and the ink's
/// margin would then depend on which tint the theme happened to pick.
#[must_use]
pub fn backdrop_for(ink: Color, hue: Color, min: f32) -> Color {
    let y_ink = common::wcag_luminance(ink);
    let y_hue = common::wcag_luminance(hue);
    let bound = legible_bound(y_hue, y_ink, min + AA_INK_HEADROOM)
        .or_else(|| legible_bound(y_hue, y_ink, min + AA_INK_SLACK));
    match bound {
        Some(y) => pull_to_luminance(hue, y),
        // The ink itself sits mid-ramp, so no backdrop reaches `min`. Take the
        // extreme furthest from it and let the guard say so out loud.
        None => {
            if y_ink < 0.5 {
                PLATE_INK_LIGHT
            } else {
                PLATE_INK_DARK
            }
        }
    }
}

/// Decide the ink for `stops` and pull only the stops that need it.
///
/// The cost of each candidate ink is the largest CIE L\* move any stop has to
/// make so the *dimmest* rung of the ladder clears AA — a control that dims
/// under the finger has to stay readable while it is being pressed, which is
/// what made the old alpha-modulated hero fail. Cheapest wins; a tie goes to
/// the light ink, which is the read the brand was drawn with.
///
/// On the dark schemes the cheap path is the dark ink: white would have cost
/// the aurora's bright end 11.8 L\* (a muddy blue), near-black costs the deep
/// end 5.7 L\*. That is the call the aurora was named for — a hero that reads
/// as a lit plate you see *through*, not a slab.
fn pull_stops(stops: &mut [Color]) -> Color {
    let target = AA_INK_CONTRAST + AA_INK_HEADROOM;
    let keep = (1.0 - PLATE_LADDER_DEEPEST_DIP).max(1e-3);
    let white_max = light_ink_ceiling(target);
    let dark_min = dark_ink_floor(target) / keep;
    let mut cost_light = 0.0f32;
    let mut cost_dark = 0.0f32;
    for &s in stops.iter() {
        let y = common::wcag_luminance(s);
        let l = common::luminance_to_lstar(y);
        if y > white_max {
            cost_light = cost_light.max(l - common::luminance_to_lstar(white_max));
        }
        if y < dark_min {
            cost_dark = cost_dark.max(common::luminance_to_lstar(dark_min) - l);
        }
    }
    let light = cost_light <= cost_dark;
    let bound = if light { white_max } else { dark_min };
    for s in stops.iter_mut() {
        let y = common::wcag_luminance(*s);
        if (light && y > bound) || (!light && y < bound) {
            *s = pull_to_luminance(*s, bound);
        }
    }
    if light {
        PLATE_INK_LIGHT
    } else {
        PLATE_INK_DARK
    }
}

/// The plate and ink for a single-colour painted control, given where in its
/// own ladder it stands (`0.0` is the full-strength rung).
#[must_use]
pub fn painted(plate: Color, rung: f32) -> (Color, Color) {
    let mut stops = [plate];
    let ink = pull_stops(&mut stops);
    (stops[0].mix(Color::BLACK, rung.clamp(0.0, 1.0)), ink)
}

/// The Primary plate: the aurora at full strength, held opaque, pulled only as
/// far as its own ladder needs and paired with the ink that clears AA on the
/// result. `rung` is one of [`HERO_DIP_REST`] / [`HERO_DIP_HOVER`] /
/// [`HERO_DIP_PRESS`].
///
/// Opaque on purpose. A translucent hero lets the page into the arithmetic, so
/// its contrast would vary with the scheme it is drawn on — the 2026-10-15
/// probe found the 0.82-alpha press over a dark ground landing in the dead band
/// (`Y ≈ 0.19`) where neither ink reaches 4.5:1. The ladder is carried by
/// `rung`, which moves luminance and leaves hue alone.
#[must_use]
pub fn hero(theme: &Theme, rung: f32) -> (Background, Color) {
    let mut stops = [aurora_hi(theme), aurora_lo(theme)];
    let ink = pull_stops(&mut stops);
    for s in stops.iter_mut() {
        *s = s.mix(Color::BLACK, rung.clamp(0.0, 1.0));
    }
    let linear = iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_4))
        .add_stop(0.0, stops[0])
        .add_stop(1.0, stops[1]);
    (Background::Gradient(iced::Gradient::Linear(linear)), ink)
}

/// How much the hero gives back for its non-hover rungs. These are the alpha
/// rungs the kit had already converged on (0.92 rest / 1.0 hover / 0.82 press),
/// re-expressed as luminance dips: `1 − alpha`, same perceptual step, opaque
/// plate.
pub const HERO_DIP_REST: f32 = 0.08;
/// Hover is the full-strength brand plate, so nothing is given back.
pub const HERO_DIP_HOVER: f32 = 0.0;
pub const HERO_DIP_PRESS: f32 = 0.18;

/// How much a **signal chip** gives back at rest, by the same conversion as
/// [`HERO_DIP_REST`]: the status pill in the command line shipped as
/// `tone_pair.color` at alpha 0.82 and went to 1.0 under the pointer, and `1 −
/// 0.82 = 0.18` is that step expressed on an opaque plate.
///
/// Opaque is the point, not a tidy-up. A translucent chip puts whatever tier it
/// was dropped on into its own contrast arithmetic, and measured 2026-10-17 over
/// the 26 selectable themes × 4 tones × 7 tiers that made the resting chip
/// unreadable on 251 of 728 ink checks (worst 3.07:1, Fusion Black's Danger on
/// the Sink tier) with its border — literally the same colour at the same alpha —
/// under the 1.4.11 bar on 226 more. One rung of luminance buys back the quiet
/// rest state without lending the page to the measurement.
///
/// Bounded by [`PLATE_LADDER_DEEPEST_DIP`] because [`painted`] only compensates
/// its ink for the deepest dip on that ladder; a rung past it would be a promise
/// the plate cannot keep.
pub const SIGNAL_DIP_REST: f32 = 0.18;
/// The pointer brings the verdict to full strength: nothing given back.
pub const SIGNAL_DIP_LIT: f32 = 0.0;

// ── Accent washes ─────────────────────────────────────────────────────────

/// How strong an accent **wash** is, as alpha over the accent colour.
///
/// A wash answers "which one" without painting the whole panel in the brand:
/// the gradient stays reserved for engaged controls (`aurora_gradient`), and a
/// wash uses the flat accent at one of these four rungs. The numbers are the
/// ones the app had already converged on by hand before the kit (audited
/// 2026-10-15 across the dock strips, the properties palette and the start
/// page) — collected here so a fifth screen cannot invent a fifth strength and
/// so the fatigue ceiling the owner asked for is one edit away.
/// Brand tint under a hero card: colour you read as warmth, not as a target.
pub const TINT_HERO: f32 = 0.08;
/// The row or cell the cursor is on / the field that holds the value.
pub const TINT_SELECTED: f32 = 0.12;
/// A thing being dragged: still itself, but no longer part of the page.
pub const TINT_GHOST: f32 = 0.22;
/// A drop target open under the pointer — the loudest wash in the app, and
/// only ever on screen for as long as a drag is over it.
pub const TINT_DROPPED: f32 = 0.35;
/// An **engaged** control (the active snap mode, the selected tab) carrying the
/// gradient rather than a flat accent: the gradient is the one wash that says
/// "this is a target you can leave", not "this row is under the cursor".
pub const TINT_ENGAGED: f32 = 0.16;
/// The same control under the pointer or mid-press.
pub const TINT_ENGAGED_ACTIVE: f32 = 0.26;
/// The wash under a **verdict band**: a block of explanatory text that reports
/// a validation result inside a dialog.
///
/// 0.18 is not a taste call — three dialogs written independently over four
/// months (`attdef_dialog.rs`, `block_definition.rs`, `wblock.rs`) all mixed the
/// tone's `base` at 0.18 by hand, and the two that drew an edge drew it in the
/// same `base` at 1px. That agreement is the measurement; this constant is where
/// it stops being re-derived per screen. It sits above an engaged control at
/// rest (a band has to be noticed) and below a drag ghost (it never has to
/// outshout the thing being moved).
pub const TONE_WASH: f32 = 0.18;
/// The border every ordinary edge draws: a field's resting frame, a swatch's
/// hairline, a table seam, a tone plate's outline.
///
/// This was the loudest missing rung in L0. 87 call sites outside `ui/kit/` and
/// this file spelled `width: 1.0,` / `1.5,` / `2.0,` by hand, and 94 of them
/// spelled `width: 1.0,` — the single most-repeated number in the UI layer, and
/// the one the ratchet could not see, because the guard counts `radius:`,
/// `.spacing(` and `.size(` but not border weights. Naming it does not just
/// tidy: it makes a future 0.5px hairline for hi-DPI seams a decision in one
/// place instead of 94.
pub const EDGE_HAIRLINE: f32 = 1.0;
/// Border weight of a focus ring or a ghost frame, where 1px reads as a
/// shadow of the control rather than its edge.
pub const EDGE_RING: f32 = 1.5;
/// Border weight of the frame around the live viewport.
pub const EDGE_FRAME: f32 = 2.0;
/// Thickness of the brand rule: the full-width aurora bar along the top edge of
/// a floating panel. Numerically equal to [`EDGE_FRAME`] and deliberately a
/// separate rung — that one is a border weight, this one is a height, and a
/// change to viewport framing must not move the brand mark.
pub const RULE_BRAND: f32 = 2.0;
/// How loud the brand rule's gradient is allowed to be.
///
/// 0.9 rather than 1.0 because a 2px bar has no interior for the eye to settle
/// in: at full strength the violet end turns the dialog's top edge into a
/// highlighter stroke and fights the title for attention. Measured as the only
/// value `ui/modal.rs` ever used for it, so this names a decision that was
/// already being made — at one call site, by hand.
pub const AURORA_RULE: f32 = 0.9;
/// Border weight of the keyboard focus ring.
///
/// Numerically equal to [`EDGE_FRAME`] and deliberately its own rung: that one
/// frames the viewport, this one frames whichever control Tab is on, and a
/// change to viewport framing must not silently resize the ring. 2px rather
/// than [`EDGE_RING`]'s 1.5 because the ring is drawn *over* the control's own
/// border, in the accent colour, and has to survive being laid on top of a
/// plate that is already outlined.
pub const FOCUS_RING: f32 = 2.0;

// ── Ink, washes and selection ─────────────────────────────────────────────

/// How much of the base ink a quieter line of text keeps.
///
/// These were bare numbers: 0.68 written twice in [`crate::ui::kit::label`]
/// and twice in [`crate::ui::kit::control`], 0.5 for disabled in two more
/// places, 0.62 for a placeholder in two. Not preferences — a caption and a
/// deactivated row have to be the *same distance* from full ink, or a dense
/// panel reads as two competing hierarchies.
/// A caption under a title, a secondary label, an unframed control at rest.
pub const INK_MUTED: f32 = 0.68;
/// Hint-grade text: quieter than [`INK_MUTED`], still deliberately readable.
pub const INK_FAINT: f32 = 0.48;
/// A control that cannot be used right now. Different fact from
/// [`INK_FAINT`]: faint text is a choice about emphasis, this is a thing that
/// will not answer a press.
pub const INK_DISABLED: f32 = 0.5;
/// A field's placeholder, above [`INK_FAINT`] so it reads as the example it is
/// rather than as decoration.
pub const INK_PLACEHOLDER: f32 = 0.62;
/// How much of the window's darkest tone a modal scrim keeps.
///
/// Three call sites in `ui/modal.rs` spelled `0.55` by hand (the plain
/// backdrop, the unsaved-changes guard, and the resize/pin sheet over it), each
/// multiplying it into the `modal:fade` track itself. No other rung covers this
/// fact: [`INK_DISABLED`] (0.5) is ink *of* a thing that will not answer and
/// [`INK_PLACEHOLDER`] (0.62) is ink *of* a suggestion, whereas a scrim is a lid
/// over the whole page — it has to dim what is under it without erasing it,
/// which is a different question from both. A selection highlight used to sit
/// here as a 0.5 wash under text; it is now solved per theme by
/// [`backdrop_for`], because a fixed opacity cannot promise the text above it
/// stays readable on a plate it does not know.
pub const SCRIM_INK: f32 = 0.55;
/// The pointer's neutral wash on an unframed control, and the resting groove of
/// a scroll rail. Two shapes, one fact: the page's own ink at 6%, the faintest
/// paint in the app.
pub const WASH_POINTER: f32 = 0.06;
/// The wash a selected row of a list gets, and the one an engaged menu line
/// gets: 4 sites spelled 0.10 by hand. It sits between the pointer's 6% and
/// the selection plate's 12% - quieter than a plate, louder than a hover -
/// which is what a row that is *chosen* but not *raised* needs.
///
/// Named rather than folded onto [`TINT_SELECTED`]: the fold would brighten
/// every one of the four rows, and a colour change on 4 screens is not what
/// a rename batch is allowed to do. Signed 2026-10-19 by the board owner.
pub const WASH_ROW: f32 = 0.10;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::config::all_themes;

    #[test]
    fn the_scale_is_monotonic_and_the_shadow_grows_with_lift() {
        // The sheet draws one corner for the control family, so field and
        // control are deliberately equal; only the panel step is larger.
        assert!(RADIUS_MARK < RADIUS_FIELD && RADIUS_FIELD <= RADIUS_CONTROL);
        assert!(RADIUS_CONTROL < RADIUS_PANEL);
        // A square plate has to stay wider than twice the corner drawn on it, or
        // the four corners meet in the middle and the mark becomes a circle. The
        // corner a mark plate uses is [`RADIUS_MARK`], so that is the pairing
        // held: 14 px against a 2 px corner, with the field/control/panel corners
        // all too round for it (which is why `swatch` spells `RADIUS_MARK`).
        assert!(MARK_SQUARE > RADIUS_MARK * 2.0);
        // The gap ladder is a total order and every step is a real one: no two
        // rungs may be one pixel apart, or a screen choosing between them is
        // choosing nothing.
        assert!(
            GAP_NONE < GAP_HAIRLINE
                && GAP_HAIRLINE < GAP_TIGHT
                && GAP_TIGHT < GAP_CONTROL
                && GAP_CONTROL < GAP_ROW
                && GAP_ROW < GAP_FIELD
                && GAP_FIELD < GAP_SECTION
                && GAP_SECTION < PAD_PAGE
        );
        // Every law below is a closure, so the ladder's own numbers and a
        // deliberately wrong number go through the *same expression*. An inline
        // assert can only be shown to bite by breaking a constant and paying a
        // compile (and a green run then proves nothing about the drift nobody
        // typed); a closure is shown to bite in the same run, and the ladder is
        // read through it. `TEXT_META`'s two lines below predate this and are
        // converted with it, because a law that cannot fail is the defect #42 has
        // now caught three times in its own guards.
        let strictly_below = |a: f32, b: f32| a < b;
        let headline_clears = |big: f32, small: f32| big / small >= 1.25;
        let cluster_fits = |focal: f32| focal - TEXT_SECTION <= 3.0;
        let floor_fits = |caption: f32| TEXT_MICRO - caption <= 1.0;

        let ladder = [
            TEXT_GLYPH_CAPTION,
            TEXT_MICRO,
            TEXT_LABEL,
            TEXT_BODY,
            TEXT_META,
            TEXT_HEADING,
            TEXT_SECTION,
            TEXT_PANE_HEAD,
            TEXT_FOCAL,
            TEXT_TITLE,
            TEXT_HERO,
            TEXT_DISPLAY,
        ];
        for w in ladder.windows(2) {
            assert!(
                strictly_below(w[0], w[1]),
                "the type ladder lost its total order at {}/{} when batch 032-B's four 0-px rungs joined",
                w[0],
                w[1]
            );
        }
        // ...and the walk is not decoration: two equal neighbours is exactly the
        // drift a fifth "one pixel louder" title would produce, and the same
        // predicate refuses it.
        {
            let mut drifted = ladder;
            drifted[7] = drifted[6];
            assert!(
                !drifted.windows(2).all(|w| strictly_below(w[0], w[1])),
                "the ladder walk accepts two equal rungs, so it asserts nothing"
            );
        }
        // The type ladder is two tiers, and the two tiers are held to different
        // laws — asserting one floor over all six rungs would fail on the dense
        // tier and be a fake law, not a guard.
        //
        // The *headline* tier names a level of importance on its own, so its
        // steps have to clear 1.25 to read as a step at all: TEXT_SECTION ->
        // TEXT_TITLE is 1.33 and TEXT_TITLE -> TEXT_DISPLAY is 1.75.
        for [big, small] in [
            [TEXT_TITLE, TEXT_SECTION],
            [TEXT_DISPLAY, TEXT_TITLE],
            [TEXT_HERO, TEXT_TITLE],
            [TEXT_DISPLAY, TEXT_HERO],
        ] {
            let step = big / small;
            assert!(
                headline_clears(big, small),
                "headline type step {big}/{small} = {step:.3} is below 1.25"
            );
        }
        // Two of the four rungs batch 032-B added DO clear the headline floor on
        // both sides, so they are held to it: 32/24 = 1.33 and 42/32 = 1.31. The
        // other two do not, and that is the honest part of this paragraph:
        // 19/18 = 1.06 and 20/19 = 1.05 sit inside the tier the loop names only
        // four transitions for. They are not an exemption won by argument, they
        // are 0-px drains of numbers that ship, and what keeps them from becoming
        // a scale is the band below: a title may now be 18, 19, 20 or 24, and no
        // fifth step joins that cluster without failing this line.
        assert!(
            !headline_clears(TEXT_FOCAL, TEXT_PANE_HEAD) && !headline_clears(TEXT_PANE_HEAD, TEXT_SECTION),
            "the 19/20 title rungs suddenly clear 1.25, so the cluster band below is guarding nothing"
        );
        assert!(
            cluster_fits(TEXT_FOCAL),
            "the title cluster (18/19/20) has grown past the 3px band #58 agreed to carry"
        );
        assert!(
            !cluster_fits(TEXT_SECTION + 4.0),
            "the title-cluster band can never fail, so it is decoration: SECTION + 4 over {} should not fit",
            TEXT_SECTION
        );
        assert!(
            floor_fits(TEXT_GLYPH_CAPTION),
            "the type ladder grew a second rung under TEXT_MICRO's readability floor"
        );
        assert!(
            !floor_fits(8.0),
            "the sub-floor step can never fail, so it is decoration: 8 under {} should not fit",
            TEXT_MICRO
        );
        // The *dense* tier (12 / 13 / 14) steps by one pixel and is allowed to:
        // those three roles are separated by ink and by what they sit next to,
        // not by size alone — a 14px field value and a 13px caption in a 40-row
        // property table cannot be 25% apart without the table stopping fitting.
        // What is checked here is the part that must stay true: the tier is a
        // strict order and stays inside the 3px band a dense panel can hold.
        assert!(strictly_below(TEXT_MICRO, TEXT_LABEL) && strictly_below(TEXT_LABEL, TEXT_BODY));
        let dense_fits = |top: f32| top - TEXT_MICRO <= 3.0;
        assert!(dense_fits(TEXT_BODY));
        // `TEXT_META` (15) joined the dense tier on 2026-10-19 as a 0-px rename,
        // so the tier is now four rungs wide and the band has to be re-measured
        // against the widest one, not against the three this test was written for.
        // 15 - 12 = 3 still fits; the assert is what keeps the next rung from
        // being added on the strength of the prose alone.
        assert!(
            strictly_below(TEXT_BODY, TEXT_META) && strictly_below(TEXT_META, TEXT_HEADING),
            "TEXT_META left its place between the field text and the panel heading"
        );
        assert!(
            dense_fits(TEXT_META),
            "the dense tier has grown past the 3px band a dense panel can hold"
        );
        // Same expression, one number the ladder does not ship: if 17 over a 12
        // floor still "fits", the band is decoration and the line above proves
        // nothing.
        assert!(
            !dense_fits(TEXT_HEADING),
            "the dense band can never fail, so it is decoration: {} over {} should not fit",
            TEXT_HEADING,
            TEXT_MICRO
        );
        // TEXT_HEADING (17) -> TEXT_SECTION (18) is a 1.06 step and sits between
        // the two tiers: it is exempt from the 1.25 floor because a panel title
        // is bold and a section heading is not, so the roles differ by weight.
        assert!(TEXT_BODY < TEXT_HEADING && TEXT_HEADING < TEXT_SECTION);
        // The ink ladder has to stay a ladder: if the placeholder ends up
        // quieter than faint hint text, a field reads as empty twice over.
        assert!(INK_FAINT < INK_PLACEHOLDER && INK_PLACEHOLDER < INK_MUTED);
        // `SCRIM_INK` is not part of the ink ladder — it is a lid's opacity, and
        // it only has to stay clear of the two washes it could be confused with.
        assert!(
            INK_DISABLED < SCRIM_INK && SCRIM_INK < INK_PLACEHOLDER,
            "the scrim has to read as neither disabled ink nor a placeholder"
        );
        // Border weights, thin to thick. `RULE_BRAND` is not on this ladder (it
        // is a height), but it must not be so thin that it disappears at 100%
        // scale on a 1080p panel.
        assert!(EDGE_HAIRLINE < EDGE_RING && EDGE_RING < EDGE_FRAME);
        assert!(RULE_BRAND >= EDGE_HAIRLINE);
        // `WASH_POINTER` is documented as the faintest paint in the app.
        assert!(WASH_POINTER < TINT_HERO && TINT_HERO < TINT_SELECTED);
        // The row wash has to stay between the hover it beats and the plate above
        // it, or `WASH_ROW` is just a second name for one of them.
        assert!(
            WASH_POINTER < WASH_ROW && WASH_ROW < TINT_SELECTED,
            "WASH_ROW left the wash/plate band"
        );
        assert!(TINT_SELECTED < TINT_ENGAGED && TINT_ENGAGED < TINT_ENGAGED_ACTIVE);
        assert!(TINT_GHOST < TINT_DROPPED && TINT_ENGAGED_ACTIVE < TINT_DROPPED);
        // A verdict band is louder than an engaged control at rest — it has to be
        // noticed — and quieter than a drag ghost, which is the thing the eye is
        // actually following.
        assert!(TINT_ENGAGED < TONE_WASH && TONE_WASH < TINT_GHOST);
        // A signal chip's rung has to stay inside the dip [`painted`] compensates
        // its ink for; past [`PLATE_LADDER_DEEPEST_DIP`] the solve is calibrated to
        // a shallower plate than the one that ships, and the rest state goes back
        // to being unreadable — which is the defect `SIGNAL_DIP_REST` replaced.
        assert!(SIGNAL_DIP_LIT < SIGNAL_DIP_REST);
        assert!(SIGNAL_DIP_REST <= PLATE_LADDER_DEEPEST_DIP);
        // Deliberately *not* asserted: one total order over all six rungs. A drag
        // ghost (0.22) sits between the two engaged strengths — an engaged pill
        // under the pointer is the loudest thing on screen except an open drop
        // target. Naming that keeps the guard honest about what the ladder is.

        // The DraftFlow sheet carries no cast shadow at any lift: depth is the
        // grey ladder plus a hairline. This is the binding form of that rule,
        // not an absence of a guard.
        for scheme in [
            crate::ui::style::draftflow_theme::draftflow_dark(),
            crate::ui::style::draftflow_theme::draftflow_light(),
        ] {
            for lift in [0.0f32, 1.0, 2.0, 3.0, 9.0] {
                let shadow = elevation(&scheme, lift);
                assert_eq!(
                    (shadow.color.a, shadow.blur_radius, shadow.offset.y),
                    (0.0, 0.0, 0.0),
                    "{scheme} at lift {lift} drew a shadow, which the pen-only scheme forbids"
                );
            }
        }
        // Third-party themes keep the vendor halo, and its ladder still grows.
        let theme = Theme::Dark;
        let rest = elevation(&theme, 1.0);
        let hover = elevation(&theme, 2.0);
        assert!(hover.offset.y > rest.offset.y);
        assert!(hover.blur_radius > rest.blur_radius);
        assert!(hover.color.a > rest.color.a);
        // An out-of-range step saturates rather than inverting the ladder.
        assert_eq!(elevation(&theme, 9.0).blur_radius, elevation(&theme, 3.0).blur_radius);
        assert_eq!(elevation(&theme, -2.0).color.a, 0.0);
        // Every reachable step has to stay inside `a <= 1.0`. `scale_alpha`
        // does not clamp, so an out-of-range alpha would draw a solid slab
        // behind the panel rather than a soft shadow.
        for lift in [0.0f32, 1.0, 2.0, 3.0, 9.0] {
            let a = elevation(&theme, lift).color.a;
            assert!(a >= 0.0 && a <= 1.0, "{theme} at lift {lift}: alpha {a} escapes gamut");
        }
    }

    /// The painted-plate maths has to hold for any hue, not only for the two
    /// aurora ends: `painted()` is also what `Role::Danger` hands to a palette's
    /// `danger.weak` tint, and the 26 selectable palettes are not ours to
    /// choose. The sweep deliberately walks through `Y ≈ 0.17 – 0.27`, the band
    /// where neither plate ink reaches AA on its own and the plate has to move.
    #[test]
    fn every_painted_plate_carries_its_own_ink() {
        use crate::ui::style::common;
        let hues = [
            Color::from_rgb(0.9, 0.2, 0.2), // the danger family
            Color::from_rgb(0.2, 0.5, 0.9), // the hero family
            Color::from_rgb(0.4, 0.8, 0.4), // and one the kit does not use yet
        ];
        for hue in hues {
            for step in 0..20 {
                let y = 0.02 + step as f32 * 0.05;
                let plate = pull_to_luminance(hue, y);
                for rung in [0.0f32, HERO_DIP_REST, SIGNAL_DIP_REST, PLATE_LADDER_DEEPEST_DIP] {
                    let (p, ink) = painted(plate, rung);
                    let c = common::wcag_contrast(p, ink);
                    assert!(
                        c >= AA_INK_CONTRAST,
                        "painted(Y {y:.2}, rung {rung}) returned {p:?} with {ink:?}: \
                         only {:.2}:1",
                        c
                    );
                    // Pulling must not rotate the hue: which channel leads is
                    // what the eye reads as the colour, and toward black the
                    // linear channels scale together, while toward white they
                    // converge (a lighter, softer plate — the cost the dark-ink
                    // path pays, and why it is only taken when it is cheaper).
                    let led = (0..3).max_by_key(|i| (before_lin(hue)[*i] * 1e6) as i32);
                    let leads = (0..3).max_by_key(|i| (before_lin(p)[*i] * 1e6) as i32);
                    assert_eq!(
                        led, leads,
                        "pulling Y {y:.2} moved the leading channel of {hue:?} from \
                         {led:?} to {leads:?}"
                    );
                }
            }
        }
    }

    /// [`legible_on`] has three jobs and this test covers all three: leave a
    /// good ink alone, fix a bad one to the bar it failed, and never do either by
    /// rotating the hue. The 3:1 bar is solvable on every plate (`Y ≤ 0.30` takes
    /// the light ink, `Y ≥ 0.10` takes the dark one, and the two bands overlap),
    /// while AA has the dead band [`hero`] already documents — so the AA sweep
    /// asserts the guarantee everywhere except the band, and asserts the band is
    /// the *only* exception.
    #[test]
    fn legible_ink_is_only_moved_when_it_has_to_be() {
        use crate::ui::style::common;
        let plates = [
            Color::from_rgb(0.98, 0.98, 0.99), // near-white recess
            Color::from_rgb(0.75, 0.55, 0.55),  // a light danger tint
            Color::from_rgb(0.30, 0.16, 0.17),  // a dark one
            Color::from_rgb(0.06, 0.06, 0.08),  // near-black page
            Color::from_rgb(0.47, 0.47, 0.47),  // Y 0.187: inside the headroom
            // aim's dead band, the shape Moonfly's danger recess has — the plate
            // that made the fallback land exactly on the bar.
        ];
        for plate in plates {
            for hue in [
                Color::from_rgb(0.9, 0.2, 0.2),
                Color::from_rgb(0.2, 0.5, 0.9),
                Color::from_rgb(0.1, 0.1, 0.1),
                Color::from_rgb(0.95, 0.95, 0.95),
            ] {
                for rung in [1.0f32, INK_PLACEHOLDER, INK_MUTED] {
                    let ink = hue.scale_alpha(rung);
                    let out = legible_on(ink, plate, AA_COMPONENT_CONTRAST);
                    let painted = over(out, plate);
                    assert!(
                        common::wcag_contrast(painted, plate) >= AA_COMPONENT_CONTRAST,
                        "legible_on({ink:?} on {plate:?}, 3.0) handed back {out:?} \
                         painting at {:.2}:1",
                        common::wcag_contrast(painted, plate)
                    );
                    // Identity kept where the palette's answer was already good.
                    if common::wcag_contrast(over(ink, plate), plate) >= AA_COMPONENT_CONTRAST {
                        assert_eq!(out, ink, "an ink that measured out got moved");
                    }

                    let aa = legible_on(ink, plate, AA_INK_CONTRAST);
                    let painted_aa = over(aa, plate);
                    let c = common::wcag_contrast(painted_aa, plate);
                    let y = common::wcag_luminance(plate);
                    // Strict, with no band exempted, because the aim arithmetic
                    // makes it possible: both pull directions close on a plate at
                    // once only above `√21 ≈ 4.583`, so the headroom aim (4.8) can
                    // fail while the slack aim (4.55) cannot, and `pull_to_luminance`
                    // lands the target luminance exactly. The one real plate that
                    // exploited this — Moonfly's danger recess, `Y 0.18`, measured
                    // at 4.4998:1 on 2026-10-16 — is now solved in L0 rather than
                    // excused here. A future rung that pushes the fallback past
                    // `√21` fails this assert rather than the app.
                    assert!(
                        c >= AA_INK_CONTRAST,
                        "legible_on({ink:?} on {plate:?}, AA) gave {:.2}:1 on a plate \
                         at Y {y:.3}",
                        c
                    );
                    // Hue preserved: the leading channel of the resolved colour
                    // is the leading channel of the candidate it started from,
                    // unless the candidate was already the answer.
                    if out != ink {
                        let was = (0..3).max_by_key(|i| (before_lin(ink)[*i] * 1e6) as i32);
                        let now = (0..3).max_by_key(|i| (before_lin(Color { a: 1.0, ..out })[*i] * 1e6) as i32);
                        assert_eq!(
                            was, now,
                            "pulling {ink:?} to clear 3.0 on {plate:?} moved its leading \
                             channel from {was:?} to {now:?}"
                        );
                    }
                }
            }
        }

        // The two things the sweep cannot say because they need a named case:
        // a rung that *measures* out is kept, and a rung that does not is paid
        // for with opacity. Both numbers are the 2026-10-16 probe's, and the
        // second one is the correction — the assumption had been that no 0.62
        // ink can clear AA anywhere, which is only true of a linear-light blend.
        // The renderer blends in sRGB, so the same rung is fine at one end of the
        // ramp and hopeless at the other.
        let light = Color::from_rgb(0.98, 0.98, 0.99);
        let kept = legible_on(
            Color::BLACK.scale_alpha(INK_PLACEHOLDER),
            light,
            AA_INK_CONTRAST,
        );
        assert_eq!(
            kept.a, INK_PLACEHOLDER,
            "a rung painting at {:.2}:1 on a near-white recess got pulled anyway",
            common::wcag_contrast(over(kept, light), light)
        );

        let mid_tint = Color::from_rgb(0.75, 0.55, 0.55);
        let failed = legible_on(
            Color::from_rgb(0.9, 0.2, 0.2).scale_alpha(INK_PLACEHOLDER),
            mid_tint,
            AA_INK_CONTRAST,
        );
        assert_eq!(failed.a, 1.0, "an ink that cannot reach AA stayed translucent");
        assert!(
            common::wcag_contrast(failed, mid_tint) >= AA_INK_CONTRAST,
            "the pull on a mid pink tint landed at {:.2}:1",
            common::wcag_contrast(failed, mid_tint)
        );
    }

    /// A selection has to carry the ink that sits on it, and `backdrop_for` is
    /// the only place that decision is made. The sweep is the 2026-10-16 probe's
    /// own grounds: the brand's `SELECTION_TINT` over a danger recess measured
    /// 1.63:1 (Nord) to 3.30:1 (Light) under the refused value, and the fix must
    /// not buy that back by painting a highlight the plate cannot be told from.
    #[test]
    fn every_selection_backdrop_still_carries_its_ink() {
        use crate::ui::style::common;
        let grounds = [
            Color::from_rgb(0.75, 0.55, 0.55), // a light danger tint
            Color::from_rgb(0.30, 0.16, 0.17), // a dark one
            Color::from_rgb(0.98, 0.98, 0.99), // near-white page
            Color::from_rgb(0.06, 0.06, 0.08), // near-black page
            Color::from_rgb(0.47, 0.47, 0.47), // Y 0.187: the headroom aim's dead band
        ];
        let inks = [
            legible_on(Color::BLACK, grounds[0], AA_INK_CONTRAST),
            legible_on(Color::WHITE, grounds[1], AA_INK_CONTRAST),
            legible_on(Color::BLACK, grounds[2], AA_INK_CONTRAST),
            legible_on(Color::WHITE, grounds[3], AA_INK_CONTRAST),
            legible_on(Color::BLACK, grounds[4], AA_INK_CONTRAST),
        ];
        let hues = [
            Color::from_rgb(0.4, 0.5, 0.9), // the brand's aurora end
            Color::from_rgb(0.8, 0.25, 0.3),
        ];
        for plate in grounds {
            for ink in inks {
                for hue in hues {
                    let backdrop = backdrop_for(ink, hue, AA_INK_CONTRAST);
                    let c = common::wcag_contrast(ink, backdrop);
                    assert!(
                        c >= AA_INK_CONTRAST,
                        "backdrop_for({ink:?}, {hue:?}) on a plate at {plate:?} gave \
                         {backdrop:?}, which carries its ink at only {:.2}:1",
                        c
                    );
                    // Visible as a highlight at all: the backdrop must not simply
                    // become the plate it is drawn over.
                    assert_ne!(
                        backdrop, plate,
                        "the selection collapsed into the plate it marks"
                    );
                }
            }
        }
    }

    /// The linear-light components of a colour, for the hue test above.
    fn before_lin(c: Color) -> [f32; 3] {
        let l = c.into_linear();
        [l[0], l[1], l[2]]
    }

    #[test]
    fn every_selectable_theme_gets_a_visible_edge() {
        for theme in &all_themes() {
            let edge = edge(theme);
            let base = theme.palette().background.base.color;
            assert_ne!(edge, Color::TRANSPARENT, "{theme} edge is invisible");
            // The edge has to differ from the surface it bounds, or a raised
            // control and its background merge into one flat plane.
            let delta = (f32::from(LinearChannel(edge)) - f32::from(LinearChannel(base))).abs();
            assert!(delta > 0.004, "{theme}: edge {edge:?} on base {base:?}");
        }
    }

    /// Luminance proxy for the edge comparison above.
    struct LinearChannel(Color);
    impl From<LinearChannel> for f32 {
        fn from(value: LinearChannel) -> Self {
            crate::ui::style::common::wcag_luminance(value.0)
        }
    }

    /// A recess only reads as a recess if it is *below* the surface it sits in,
    /// in both schemes — a palette step would have gone the other way on dark.
    ///
    /// The two DraftFlow schemes are exempt from the vendor direction law by
    /// construction: they name their recesses on an explicit sheet, and `sunk`
    /// is the shell (DESIGN.md §2.1 — the shell is the darkest rung). Demanding
    /// `sunk < base` there would demand a seventh, darker surface the spec does
    /// not have, so the test takes the identity branch for those two themes —
    /// the same split `kit_contract_tests::brand_ladder_has_measurable_steps`
    /// pins — and keeps the direction law on the other 24 themes.
    #[test]
    fn the_recessed_band_sits_below_its_window_in_every_theme() {
        for theme in &all_themes() {
            let palette = theme.palette();
            let sunk = sunk(theme);
            let base = crate::ui::style::common::wcag_luminance(palette.background.base.color);
            if let Some(colors) = crate::ui::style::draftflow_theme::draftflow_colors(theme) {
                assert_eq!(
                    sunk, colors.shell,
                    "{theme}: an explicit sheet names `sunk` as the shell"
                );
                assert_eq!(
                    palette.background.base.color, colors.shell,
                    "{theme}: the page is the shell, so no rung sits below it"
                );
                continue;
            }
            let recess = crate::ui::style::common::wcag_luminance(sunk);
            assert!(
                recess < base,
                "{theme}: recess {sunk:?} is not darker than base {:?}",
                palette.background.base.color
            );
            // Measured as a ratio, not an absolute step: linear luminance
            // compresses hard near black, so a scheme whose surface is already
            // near-black cannot move a fixed 0.004 without punching a hole to
            // pure black — yet the cut it does make is what the eye reads at
            // that end of the range.
            assert!(
                recess <= base * 0.96,
                "{theme}: recess {sunk:?} does not read below base {:?}",
                palette.background.base.color
            );
        }
    }
}
