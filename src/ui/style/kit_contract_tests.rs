//! Kit contract guard — the ratchet that forces the UI layer to stop hand-writing
//! styles once `src/ui/kit/` primitives exist.
//!
//! WHY THIS EXISTS
//! Every previous "UI upgrade" pass only *added* things on top of the old
//! structure. The refactor acceptance rule is therefore arithmetic, not
//! opinion: a violation count may only ever go DOWN. Each test here re-measures
//! `src/` at compile/test time and compares against a frozen baseline. A number
//! above the baseline fails the build (someone hand-wrote new styling); a number
//! below the baseline passes and prints "lower the baseline" so the win is
//! booked instead of silently re-baselined.
//!
//! MEASUREMENT PROVENANCE
//! The first freeze is the *measured* value on 2026-10-15, one row per hit in
//! `D:\tmp\kit-baseline.csv` (counts summary: `D:\tmp\kit-baseline-counts.txt`).
//! Measured twice with two independent engines (grep pipeline + ripgrep) to rule
//! out engine quirks: a) 361, b) 56, c) 1022, d) 1336. The constants below carry
//! the current re-freeze, and each one records the drop from that first freeze.
//!
//! Each re-freeze run MUST be double-checked before it is written here: run
//! `bash D:\tmp\measure3.sh` (external grep + awk + the node size/text_size walk)
//! and compare its twelve guarded counts against the "BASELINE CAN BE LOWERED
//! TO" / "flat at" lines this guard prints in the same test output. Two engines
//! agreeing on all twelve numbers is what makes a run auditable; a run with no
//! log line behind it is a guess.
//!
//! One label per object, enforced by `one_screen_label_is_kept_in_this_doc`:
//! a rebuilt screen is `Screen N`, N read off the order of `REBUILT_SCREENS`; a
//! working turn is a `run`; a decrease between two re-freezes is an `X -> Y drop`;
//! and a single value of the F2 ratio series is an `F2 reading`. The retired
//! spellings were `l-e-g N`, `the <ordinal> l-e-g`, `L-e-g-s`, `<ordinal> (date)`
//! headers and `F2 po-i-n-t` — two of them numbered the same screen differently,
//! so a reader who lined the two systems up got a row count that never existed.
//! They are spelled in pieces here because the guard below scans this file's own
//! text and must not trip over its own explanation.
//! `bash D:\tmp\progress.sh`
//! turns the per-hit CSV into `D:\tmp\kit-migration-progress.csv` (one row per
//! file, `TOTAL` last) and prints its own reconciliation: the per-file column
//! sums must equal the guarded hit-row count, and that file count must equal the
//! distinct-file count. Both checks exist because the first version of that
//! script piped through `head -1`, which drains an 8 KB buffer and silently
//! dropped 25 of 80 files while still printing a plausible TOTAL.
//!
//! Runs verified in `D:\tmp\*.log` by both engines: a) 361 -> 348 and 233 -> 204,
//! b) 32 -> 25, c) 1022 -> 1016 and 1015 -> 824, d) 1336 -> 1328 and 1316 -> 1073,
//! then the (d) split (1073 rows = 957 glyph + 116 box) and #58's two folds:
//! box 116 -> 0 and glyph 957 -> 63. (g) was opened at **98** and folded to
//! **3** inside the same hour on 2026-10-17, both engines reading 98 before and
//! 3 after (`D:\tmp\m87a.log`, `m87b.log`, `t87-guard.log`).
//! The first #42 calibration screen (`ui/window/layer_state_manager.rs`) then
//! took that file to zero rows: c) 151 -> 146, f) 513 -> 488, d1) 63 -> 61
//! (`D:\tmp\s42-measure.log` for the measurer, and the guard suite in the same
//! run). This run is different in kind from the folds above — pixels moved — so
//! it is booked per row in `D:\tmp\s42-calibration-ledger.tsv` and re-derived
//! from L0/L1 by `D:\tmp\prove-s42-calibration.cjs` before the numbers here.
//! Every other run in the chains below (a's 348 -> 298 -> 262 -> 251 -> 250 ->
//! 233, b's 56 -> 32, c's 1016 -> 1015, d's 1328 -> 1316) is inherited from this
//! file's own previous comments: the runs that produced them were re-run with
//! narrower filters and left no log behind, so those numbers are what the file
//! had already frozen, not independently re-measured values.
//!
//! RULES (kept deliberately dumb so they are reproducible without a regex crate)
//! - Scan every `*.rs` under `src/`, recursively.
//! - Skip whitelisted paths (the places where a literal *must* live).
//! - Skip lines whose trimmed content starts with `//` (kills doc/comment FPs).
//! - One line counts at most once per category (row-per-hit, like the CSV),
//!   **except** (d) and (g), which walk the blanked file and count per *call*:
//!   the quantity they measure is not line-shaped (a box row can hide inside a
//!   chain, and a `.text_size(` can be split across lines), and a row-per-hit
//!   rule there is exactly the defect (f) was created to catch.
//!
//! If you change a rule here, re-run the measurer and re-baseline every constant
//! plus `BASELINE_FROZEN` in the same commit — otherwise the ratchet measures
//! nothing, or reports a date that no longer describes the number.

use std::fs;
use std::path::{Path, PathBuf};

/// (a) hand-written `container::Style {` / `button::Style {` / `text_input::Style {`
/// literals, fully-qualified forms included. Re-frozen 2026-10-16: **116**
/// (361 -> 348 -> 298 -> 262 -> 251 -> 250 -> 233 -> 204 -> 191 -> 187 -> 184 ->
/// 179 -> 116 across the migration waves; the last run is wave 4, the seven
/// dialog/manager screens that had never been migrated at all).
///
/// Provenance of the runs, in the two kinds the chain has always had: the
/// 361 -> 191 part is inherited from the earlier freezes, while every step from
/// 191 down was measured on the day it happened and the logs are still there.
/// `191 -> 187` is task #65: `kit::error_field_style` took five duplicate
/// error-field implementations out of the screens (`D:\tmp\t65d.log` read 191
/// before the deletions, `D:\tmp\t65e.log` read 187 after, and #68's `t68a.log`
/// plus #70's `t70d.log` held 187 across two unrelated batches, which is what
/// makes the drop attributable rather than drift). `187 -> 184` is task #71:
/// `kit::tone_plate_style` took the three hand-mixed danger banners out of
/// `attdef_dialog.rs` / `block_definition.rs` / `wblock.rs`, one literal each.
/// Both engines agree on the new number: `D:\tmp\t71-ratchet.log` (shell) prints
/// 184 and this test's own scan printed "191 -> 184".
///
/// `179 -> 116` is wave 4 (task #61, plus #76 and #79 inside it), and it is the
/// first run attributed row by row rather than by hand: the diff of
/// `D:\tmp\kit-baseline-before74.csv` (the 179 snapshot) against
/// `D:\tmp\kit-baseline-freeze116.csv` names seven files and nothing else —
/// `xref_manager.rs` 20 -> 1, `layers.rs` 10 -> 0, `pdf_dialogs.rs` 9 -> 0,
/// `layer_state_manager.rs` 7 -> 0, `plot.rs` 7 -> 0, `count_palette.rs` 6 -> 0,
/// `plugin_manager.rs` 5 -> 0. Sum -63, no file grew, no file outside those
/// seven moved. `pdf_dialogs.rs` reaching 0 is the part that was *not* a
/// migration: its three `pub(crate)` style forwarders (`accent_text`,
/// `card_style`, `well_style`) had 18 call sites in five other screens, and they
/// were deleted along with the call sites rather than left as a shim — task #79.
///
/// KNOWN DEFECT IN THIS COLUMN, unchanged and now quantified: `style_struct_literal`
/// accepts any line where the widget's `Style` is followed by `{`, which is also
/// the shape of a *return type* in a function signature — `pub fn x(...) ->
/// button::Style {`. 21 of the 116 rows are signature lines
/// (`D:\tmp\t80-artefacts.log` quotes all 21 verbatim). They are not innocent
/// either: 18 of the 21 name a `fn` that is a screen defining its *own* style function
/// (`btn_s`, `list_item`, `field_style`, `tab_btn_style`, `popup_row_style`,
/// `header_button_style`, `surface_style`, `button_style`), which is the same
/// offence one level up — an L3 authority deciding paints the kit should own.
/// The remaining 3 are the second line of a multi-line signature
/// (`) -> button::Style {`), which is nothing at all. So the column files
/// "local style authority" together with "hand-written literal" under one count;
/// it does not overstate the violation, it under-distinguishes it. The defect
/// stays unfixed because splitting the two needs a decision about whether a
/// forwarder is worth gating, and #78 deletes the first of them anyway.
///
/// The 204 -> 191 drop is batch 5: `kit::scrim_style` took the three modal lids
/// and `kit::brand_rule` the modal rule, `kit::swatch_style` /
/// `kit::mark_plate_style` took the colour plates, `kit::seam_style` the two
/// property-grid row seams, `kit::tone_plate_style` the two danger bands.
///
/// One literal is *left in* on purpose and is not a slipped assertion:
/// `sheet_set.rs:680` (a badge whose neutral branch is `palette.secondary`, a
/// role `kit::Tone` has no member for). Its partner in that pair —
/// `color_select.rs`'s ACI `button::Style`, the one this paragraph excused on the
/// grounds that "no kit primitive carries a data colour plus a status" — is no
/// longer in this count. #63 did not re-argue the exception; it built the
/// primitive the exception was describing. See the 109 -> 102 drop below.
///
/// The 116 -> 113 drop is task #77 (`kit::Join` + `kit::joined_btn`). Two call
/// sites, both of which had been building a whole plate by hand only to change
/// its corners: `xref_manager.rs`'s `split_button` owned one `button::Style` (it
/// picked the weak/strong hover pair, the edge colour and the hairline itself —
/// every one of those already being `Role::Standard`), and `drafting_settings.rs`
///'s `tab_button` owned two (its active branch re-spelled `Role::Engaged`'s
/// aurora wash at 0.16/0.26 by hand, its rest branch `Role::Standard` again).
/// Both closures were deleted, not wrapped, so (a) and (b2) fell together and the
/// padding came out identical: `Size::Control` resolves to `[4, 10]` and
/// `Size::Compact` to `[4, 8]`, which is exactly what those two sites had typed.
/// That last fact is worth keeping in the record because it is the counter-
/// example to the usual objection — here, moving the decision to L1 changed no
/// pixel, which is what makes the remaining difference (a tab now inherits the
/// role's elevation, and its outer corner joined the ladder instead of the
/// private 3.0/4.0) a decision being *made once* rather than a restyling.
///
/// The 113 -> 109 drop is task #78, and it is a deletion rather than a migration:
/// `src/ui/style/row_button.rs` was the last style authority outside the kit that
/// a screen still called into. Its four rows were two `button::Style` literals
/// plus the two `-> impl Fn(..) -> button::Style` / `-> button::Style` return
/// annotations that made them a public surface. `layers.rs` now asks `kit::btn`
/// for `Role::Ghost` at its four call sites — `sortable_header`, the `svg_btn`
/// closure (which serves visible, freeze, lock and every viewport column), the
/// plot composite, and the name cell — so the file is gone from the tree,
/// archived at `D:\tmp\t78-archive\row_button.rs` rather than forgotten, because
/// the reason it existed (two byte-identical copies in two panels) is the
/// evidence behind its own doc comment.
///
/// The 109 -> 102 drop is #63, and it is the first run in this column that moves
/// pixels *on purpose*, so the moves are written out rather than folded into the
/// count. Seven hand-written style closures went to L1: two colour cells to the
/// new `kit::swatch_button_style`, one named-colour row plate to
/// `kit::swatch_plate_style`, the flyout's no-colour placeholder to the existing
/// `kit::mark_plate_style`, and three 12px layer chips (`ribbon/mod.rs`,
/// `ribbon/widgets.rs` twice) to the existing `kit::swatch_style`. Three of those
/// seven were the same block typed out twice over in two files, which is the
/// whole argument for the batch and the reason the count falls by more than one.
///
/// What actually changes on screen, four items:
///   * every chip's corner 1.0 -> `RADIUS_MARK` 2.0, across the ~332 cells the
///     palette dialog (267) and the ribbon flyout (65) render;
///   * the dialog's selected ring 2.5 -> `EDGE_FRAME` 2.0, which is what the
///     flyout already painted, so two surfaces reporting the same state stopped
///     disagreeing;
///   * the dialog's *hover* ring moved off `primary` onto the theme's ink. It
///     used to share a colour with selection, so pointing at a colour looked like
///     having chosen it. This one was found while migrating, not signed off
///     beforehand, and it is the reason `swatch_border` exists as a shared rule
///     instead of two per-caller closures;
///   * the 12px layer chips' edge, hand-spelled as `palette.background.strong`,
///     became `tokens::edge`. That is bit-identical on the dark scheme, because
///     `Control::Strong` and iced's own `strong` are both `deviate(base, 0.15)`
///     (see [`crate::ui::style::tokens::Control::vendor_slot`]), and one rung
///     apart on the light one (0.125 against 0.15). The same three cells also
///     pick up `snap: true`, which every other swatch in the kit already had and
///     which is *not* free here: `crisp` is off in this manifest, so their
///     default was off.
///
/// The before/after sample the batch was judged against is
/// `D:\tmp\s63-radius-decision.html`; its numbers are parsed from the tree by
/// `D:\tmp\mk-63-samples.cjs` rather than typed in.
///
/// The 102 -> 100 drop is #42's second screen (`block_definition.rs`, ledger
/// `D:\tmp\s42-block-definition-ledger.tsv`): the dialog's hand-written fieldset
/// border went into `kit::group_box` and its 0.55 modal scrim became
/// `kit::scrim_style`. Both were closures over a paint the kit already owned, so
/// neither moved a pixel and neither moved an ink value — and the radius claim is
/// checkable rather than asserted: the pre-batch board carries no (b) row for
/// this file, so the closure already spelled `tokens::RADIUS_FIELD` (5.0) and
/// `EDGE_HAIRLINE` (1.0) instead of `4.0`, and no `radius: <numeric>` literal
/// survives anywhere in the guarded tree now. The re-count behind that half of
/// the claim is this freeze's, not the run's that wrote it: the last two screen
/// rows were `ribbon/collapse.rs:624`, renamed to `tokens::RADIUS_FLAT` at the
/// 25th freeze, and `app/view/overlay.rs`'s square, drained by an earlier wave.
/// What still spells `radius: 0.0` is L1 only (`kit/control.rs:843` and six in
/// `kit/surface.rs`), and the two near-misses a grep returns —
/// `statusbar/status_menu.rs:88`'s `blur_radius: 6.0` and
/// `app/command_driver/tests.rs:965`'s `radius: 3.0` — are a shadow blur and a
/// test fixture, neither of which column (b) reads. The one thing the old paren
/// got wrong is that `collapse.rs:624` was never inert: the `Border` around that
/// `radius: 0.0` carries `width: tokens::EDGE_HAIRLINE` = 1.0, so the corner
/// does paint, at a square radius.
///
/// The scrim is exact at rest by construction: `scrim_style` resolves to
/// `control_plate(Control::Strongest).scale_alpha(SCRIM_INK * fade(track))` with
/// `SCRIM_INK = 0.55` and `fade(track) = 1.0` at rest, which is the closure it
/// replaced plus the fade the ladder already gives every other plate.
///
/// Re-frozen at **99** by #42's third screen (`ui/window/annotation_data.rs`,
/// ledger `D:\tmp\s42-annotation-data-ledger.tsv`), and its single row is a
/// rename, not a restyle: the fieldset border closure at old line 438 is
/// bit-for-bit what `kit::group_box` paints (`EDGE_HAIRLINE` / `RADIUS_FIELD` /
/// `background.strong.color`, no fill), which is the same evidence that promoted
/// the primitive in the first place. 99 rows over 42 files (was 43), heads
/// `ui/style/plotstyle.rs` 5, `ui/style/scale_manager.rs` 5,
/// `ui/window/block_palette.rs` 5, `ui/window/graphic_attributes.rs` 5.
///
/// Re-frozen at **95** by #42's fourth screen (`ui/window/attribute_editor.rs`,
/// ledger `D:\tmp\s42-attribute-editor-ledger.tsv`): four rows, all ink, none of
/// them a restyle in the sense the paragraph above means. The forwarder at old
/// line 200 is deleted outright — it only re-exported `kit::field_style`, so its
/// row leaves with the function and five call sites lose a layer of indirection.
/// Two hand-painted `container::Style` closures fold into primitives that were
/// already making this exact decision: the top band's `background.weak` plate
/// onto `kit::wash(Tier::Band, ..)` (the ladder's own page->Band rung, the same
/// step three already-rebuilt window toolbars took, so the delta is priced by the
/// surface contracts and not by this column) and the window's ground onto
/// `kit::page_style(.., "")`, which is bit-identical because `fade("")` is 1.0.
/// The fourth row is not a container at all: a five-state `button::Style` for the
/// attribute list reaches `kit::list_row`, and it is the one row here that changes
/// what the user sees without changing a single geometry pixel; its five
/// re-decided values are on the ledger's 421/423 lines, not asserted away here.
/// Measured now: 94 rows over 40 files — `sheet_set.rs` took the last one out of
/// its badge by asking the tone ladder for the plate instead of painting it.
///
/// Re-frozen at **91** by #42's seventh screen (`ui/window/field_dialog.rs`).
/// Those three rows were not a painted plate: they were one line of a local
/// `trait IntoContainer` and the `container::Style { … }` its `impl` built in
/// order to borrow the kit's own field plate for a *container* (a selectable
/// list's frame, and a read-only code plate). The (a) rule counts a struct
/// literal wherever it appears, signatures included, so the column read them as
/// screen-decided ink — and here the screen had decided nothing, it had moved
/// `background` and `border` over whole from `kit::field_style`. The conversion
/// is a kit fact, so it moved up a layer as `kit::field_container_style`
/// (`ui/kit/control.rs`, re-exported) rather than being deleted; the two call
/// sites changed shape, the pixels did not.
///
/// The ninth screen's row was the mirror image of that defect: `properties.rs`
/// had a `pub(crate) fn combo_input_style(..) -> text_input::Style` whose entire
/// body was `kit::field_style(theme, status)`. The column counts a signature
/// because it counts the shape, and the shape here was a forwarding layer with no
/// paint in it — 11 call sites (8 in that panel, 3 in `graphic_attributes.rs`)
/// could reach the kit directly. So this row was cleared by deleting the shim,
/// which is the top of the landing order, and it moved nothing: a row the ratchet
/// invented and a row the screen owned are not the same debt, and only one of
/// them is paid by renaming. Column (a) measures now: 25 rows over 15 files, headed
/// by `ui/window/open_progress.rs` 4 and `ui/node_graph.rs` 3 — none of which this
/// run reached (the whole board is 61 rows over 31 files). The eleventh
/// rebuilt screen took two of these: a hand `container::Style` border that
/// `kit::group_box` writes field-for-field, and a tab-strip divider that filled
/// itself from `background.neutral` where every other rule in the app fills from
/// `tokens::edge`.
///
/// Re-frozen at **87** by #42's twelfth screen (`ui/window/wblock.rs`, 1 row): the
/// second deleted private fieldset, whose `container::Style` `kit::group_box`
/// restates field-for-field (`EDGE_HAIRLINE` / `RADIUS_FIELD` /
/// `background.strong.color`, no fill). Measured now: 87 rows over 36 files, still
/// headed at 5 each by `plotstyle.rs`, `scale_manager.rs` and `block_palette.rs`.
/// That file's other five rows were (a2) ink, and the pair together is why the row
/// leaves at 0 px rather than at a price: nothing in the box's paint changed, only
/// who decides it.
///
/// Re-frozen at **25** by #42's fold batch 026 (2026-10-19, the 26th freeze): three
/// screens whose fieldset well restated `kit::group_box`'s border byte-for-byte
/// (`EDGE_HAIRLINE` / `RADIUS_FIELD` / `background.strong.color`, no fill) now call
/// `kit::fieldset_well_style`, the name that border was always copying -
/// `ui/window/drawing_units.rs`, `ui/window/geometric_tolerance.rs` and
/// `ui/window/layer_translator.rs`. 0 px again, for the reason the wblock.rs link
/// above already gives: the well painted this before and paints this now; only who
/// decides it changed. Two of the three files leave the board (39 -> 37);
/// `layer_translator.rs` stays for its one
/// `off_token_alpha` row, which is a wash-strength decision, not a border, so the
/// file is not pinned. `group_box` itself dropped its own private fourth copy of
/// the same block and calls this fn, which is why the L1 box and the three screens
/// cannot drift apart later.
///
/// Re-frozen at **23** by #42's fold batch 031 (2026-10-19, the 28th freeze): two
/// list rows spelled the selected-row wash by hand, byte-for-byte identical in all
/// five fields - `background: selected.then(aurora_gradient(theme, WASH_ROW))`,
/// `text_color: primary.weak.text`, transparent border, 0 width, RADIUS_FIELD - and
/// both now call `kit::selected_row_wash_style(theme, selected)`. 0 px: the row
/// painted this before and paints this now, only who decides it changed. Both files
/// leave the board (61 -> 59 rows, 31 -> 29 files), each having owed exactly this
/// one row.
///
/// The new fn is not any of its three L1 neighbours, and the reason each was
/// rejected is the content of the fn: `kit::wash_style` scales the *tier fill* by
/// `fade(track)` and returns `snap: true` plus an elevation shadow, so routing a row
/// through it changes the paint and the scroll behaviour; `tint_style(Tint::Selected)`
/// is a solid `primary.weak` plate at `TINT_SELECTED` (0.12), one rank hotter than
/// the 0.10 aurora these rows wear and not a gradient at all; the bordered plates
/// (`guard_panel_style`, and the brand-hairline plate that has since been deleted in
/// favour of a glass pane) are a line around a ground, not a wash. `kit::list_row`
/// was rejected last because it is
/// a `Button` with a hover branch, and its engaged wash is `TINT_SELECTED * p`, so
/// folding would both move the ink and hand both lists a hover state they never
/// painted. That is the same 0.10-vs-0.12 question #48 is holding for a signed
/// sample - this batch answers only "who owns the decision", not "which strength".
///
/// Re-frozen at **9** by #42's batch 033 (2026-10-20, the 32nd freeze): 14 rows out,
/// in two groups, each landed on an L1 fn that had to be written from the site rather
/// than borrowed from a neighbour. Group A (8 rows, 4 files) went to the five plate
/// fns added for it - `plain_plate_style`, `field_radius_plate_style`,
/// `card_plate_style`, `row_selection_plate_style`, `preview_well_style` - taking
/// `open_progress.rs` 4, `pc_manager.rs` 2, `layout_manager.rs` 1, `xref_attach.rs` 1.
/// Group C (6 rows, 4 files) went to four new surface fns (`accent_plate_style`,
/// `ghost_sheet_style`, `marker_ring_style`, `option_row_style`) plus
/// `kit::menu_row_style`, which is the private fn from the eighth rebuilt screen
/// moved up a layer with its body unchanged. Both engines agree on the nine left:
/// `node_graph.rs` 3, `about.rs` 2, `gradient_editor.rs` 2, `ribbon/widgets.rs` 1
/// (a) row still on the board, held for the next batch), `point_style.rs` 1. Seven
/// files leave the board (19 -> 12 files); `open_progress.rs` stays for the
/// full-strength wash at :67, which is a wash-strength decision the column
/// cannot see. Spelled that way on purpose: this file is scanned for out-of-scope
/// hits too, and writing the call shape here would add a prose line to a census of
/// code sites (measured at the 32nd freeze, where one such spelling did).
///
/// Re-frozen at **1** by #42's batch 034 (2026-10-20, the 33rd freeze): 8 rows out
/// of 4 files, every one landed on an L1 fn written from its site rather than
/// borrowed from a neighbour. `ui/node_graph.rs` 3 went to `heading_plate_style`,
/// `node_plate_style` and `port_dot_style`; `ui/window/about.rs` 2 went to
/// `inset_card_style` (the file's private `surface_style` deleted, its three call
/// sites repointed); `ui/window/gradient_editor.rs` 2 went to `picker_field_style`
/// and `plain_plate_style`, the latter the site's own plate byte for byte;
/// `ui/style/point_style.rs` 1 went to `glyph_cell_style`, whose selected fill
/// stays an *argument* at the call site because the 0.14 strength is a signed
/// decision #48 holds (that row remains on (h); the file's (a) count is now 0).
/// Three files left the board at once (12 -> 9). The one (a) row still on it is
/// `ui/ribbon/widgets.rs:406`, a `button::Style` signature whose body is held for
/// #82's C/F decision; `(a2)`'s `common.rs:27` is likewise held, not forgotten.
const BASELINE_STYLE_LITERAL: usize = 1;

/// (a2) hand-written `text::Style { color: .. }` blocks — ink a screen decided
/// for itself instead of asking `kit::label` for a voice.
///
/// This column exists because the first four were blind to it: `STYLE_WIDGETS`
/// names `container`, `button` and `text_input`, and a `text::Style` literal is
/// none of those, so 42 call sites that each pick their own colour were never
/// counted and never shrank. Frozen at the measured **42** on 2026-10-16 the
/// blind spot was found (`D:\tmp\t80-textstyle.log` lists all 42 rows). The 42
/// live in 23 files and split three ways: the two heavy hosts `wblock.rs` 6 and
/// `block_definition.rs` 5; six CAD style-editor windows under `src/ui/style/`
/// (`dimstyle`, `mlstyle`, `style_manager`, `tablestyle`, `textstyle` at 2 each
/// plus `common.rs` 1) for 11; and 20 spread one or two to a file across 15
/// further windows, popups and the start-page overlay.
///
/// It is deliberately its own category rather than a fourth member of
/// `STYLE_WIDGETS`: (a) carries a frozen chain of its own, going back to 361,
/// and folding these in would move a historical number by redefining what was
/// counted, which is exactly the kind of quiet restatement this file keeps
/// getting called out for.
///
/// Re-frozen at **37** by #42's second screen (`block_definition.rs`, ledger
/// `D:\tmp\s42-block-definition-ledger.tsv`), which gave up all 5 of its rows to
/// the kit's label voices. Unlike the two container folds in (a) these are *not*
/// renames, and the honest reading is an ink change per row, read off `HEAD`
/// before the edit: the fieldset caption and the empty-selection status moved
/// from a hand-picked `background.strongest.text.scale_alpha(0.85 / 0.75)` to
/// [`crate::ui::kit::muted_style`] (`INK_MUTED` = 0.68 of `background.base.text`,
/// then solved to the AA floor), and the invalid-name hint and the two count
/// inks moved from raw `danger.base` / `primary.base` to
/// [`crate::ui::kit::tone_style`], which starts at the same tone colour and is
/// only ever pulled toward more contrast. So the column drops by 5 because the
/// screens stopped deciding ink, not because the ink is identical.
/// Distribution after the run (`D:\tmp\tally-perfile.cjs` on the fresh board):
/// **37** rows over **22** files — one heavy host (`window/wblock.rs` 6, now the
/// only dialog family in the column), the six CAD style-editor files under
/// `src/ui/style/` still carrying 11 (five windows at 2 plus `common.rs` 1), and
/// 20 spread one or two to a file across 15 further windows, popups and the
/// start-page overlay.
///
/// Re-frozen at **36** by #42's third screen (`ui/window/annotation_data.rs`),
/// 1 row, and this one is the cleanest fold of the day: the screen's `fn muted`
/// was a two-line forwarder onto `common::muted_style`, which was itself a
/// forwarder onto [`crate::ui::kit::muted_style`], so the ink never changed — the
/// screen was reaching the kit through two hops and shadowing it with a local
/// name. The local function is deleted, not kept as a re-export. 36 rows over 21
/// files (was 22); `ui/window/wblock.rs` 6 is still the only dialog family left
/// in the column.
///
/// Re-frozen at **35** by #42's eighth screen (`app/view/overlay.rs`): the QSelect error line
/// was `text::Style { color: Some(theme.palette().danger.base.color) }` and now reads
/// `kit::tone_style(theme, kit::Tone::Danger)` — the voice ladder, not a palette grab. The
/// other row this screen owned is deliberately left: a menu hint that dims by two alphas
/// off a `highlighted` branch, which only becomes solvable once the plate under it comes
/// from a rung (see the deferred rows on `D:\tmp\s42-overlay-ledger.tsv`).
///
/// Re-frozen at **33** by #42's eleventh screen (`ui/window/drafting_settings.rs`), 2 rows,
/// and both are the same bug the `kit::group_box` doc names in its own drift table: an alpha
/// multiplied onto a *palette text* colour, which is solved for nothing. The fieldset caption
/// wore `background.strongest.text.scale_alpha(0.75)` at eleven boxes and the polar-tracking
/// explainer wore `× 0.7`; both now read [`crate::ui::kit::muted_style`] (`INK_MUTED` = 0.68 of
/// `background.base.text`, then solved to the AA floor by `voice_ink`/`resolve_on`). So this
/// column drops by 2 because the screen stopped deciding ink — the ink itself moved, quieter by
/// 5-7 points of alpha and finally checkable by `theme_accessibility_tests.rs`, which can see a
/// kit voice and never saw an inline multiply. Neither screen 9 nor screen 10 owned an (a2) row,
/// so this is the first (a2) drain since the eighth screen.
///
/// Re-frozen at **27** by #42's twelfth screen (`ui/window/wblock.rs`), 6 rows at
/// once — the largest single drain since the second screen gave up 5, and the end
/// of this column's head host. Four were the identical unsolved multiply
/// (`background.strongest.text.scale_alpha(0.85)`) on a field label: the X/Y/Z axis
/// captions, "Insert units:", "File name and path:", and the deleted private
/// fieldset's own caption at all four boxes. Each becomes
/// [`crate::ui::kit::muted_style`] (`INK_MUTED` 0.68 of `background.base.text`,
/// then solved to the AA floor), the same fold `annotation_data.rs` made when its
/// local `fn muted` turned out to be a two-line forwarder onto that function. Two
/// more were a chip fill colour used as *ink*: `palette().warning.base.color` on
/// "No objects selected" and `palette().primary.base.color` on the count line, now
/// `kit::tone_style(theme, Tone::Warning | Tone::Accent)` — which is the ink that
/// `the_verdict_word_reads_on_the_plane_it_is_given` can measure (333 of 728 tone ×
/// plane pairs used to land under 4.5:1 as raw chip fills). So the column drops by
/// 6 because the screen stopped deciding ink, not because the ink is identical:
/// every one of these six rows changes colour, four of them by 17 points of alpha
/// plus a re-solve against a different plane, and no board number can show that.
/// Re-frozen at **1** by #42's fold batch 032-A (2026-10-19, the 29th freeze): 18 rows over
/// 12 files, and all 18 are the same rename. [`crate::ui::kit::color_style`] is
/// `text::Style { color: Some(c) }` and nothing else, so each hand-spelled closure handed the
/// colour expression over byte-for-byte: `theme.palette().primary.base.color`, `... .danger.base
/// .color`, `theme.palette().background.base.text.scale_alpha(0.72)`, `base.scale_alpha(if
/// enabled { 0.6 } else { 0.35 })`. 0 px. That is what separates this batch from every link
/// above it: those traded a hand-picked alpha for `muted_style` or a raw chip fill for
/// `tone_style` and had to declare an ink change, and this one trades a struct literal for the
/// name of the same struct literal.
///
/// 5 of the 12 files (`mlstyle`, `tablestyle`, `textstyle`, `about`, `layout_manager`) reached
/// the literal through a private `fn primary_style(theme: &Theme) -> iced::widget::text::Style`
/// that returned it. Folding only that body would drain nothing: this column is counted by
/// `style_struct_literal(line, "text::Style")`, which matches the *signature* line too, so the fn
/// is deleted and its 8 call sites spell `kit::color_style(...)` (mlstyle 3, textstyle 2,
/// tablestyle 1, about 1, layout_manager 1, counted off the pre-batch copies in `D:\tmp\s32abak`).
/// `layout_manager` is the one site that used the value outside a `.style()` closure, so it takes
/// the colour expression itself rather than a style. `tablestyle.rs` had no `use crate::ui::kit`,
/// so its 2 sites are fully qualified.
///
/// 9 of the 18 rows belonged to the 6 files that now leave the board (59 -> 41 rows, 29 -> 23
/// files): 3 popups at 1 row each and 3 `src/ui/style` editors at 2 each, and those were their
/// only rows, which is why each of them can be pinned in `REBUILT_SCREENS`. The other 9 came from
/// 6 files that stay on the board still owing (a), (e), (f) or (h) rows of their own; for each of
/// those the freeze proves category-by-category that nothing but (a2) moved.
///
/// The last row was `ui/style/common.rs:27`, `pub(crate) fn muted_style(theme: &Theme)`
/// -> `iced::widget::text::Style`: a two-line forwarder whose body already called
/// `crate::ui::kit::muted_style`, which the engine saw only because the signature spells
/// the struct type. The 36th freeze deleted the forwarder rather than keeping it as a
/// re-export and repointed every caller; the tree now imports the kit function in 28
/// files and names it as a full path at 2 call sites, with no non-documentation reference
/// to the old path left. The column is 0 because the wrapper is gone. The one-ink contract
/// moved with it: `style::mod`'s RED pin on the forwarder was deleted with its subject, and
/// the 26-theme sweep in `theme_accessibility_tests` still asserts the kit's solved ink.
const BASELINE_TEXT_STYLE_LITERAL: usize = 0;

/// (b) corner radius hard-coded as a numeric literal inside the iced UI layer
/// (`src/ui/`, `src/app/`). Re-frozen 2026-10-17: **2** (was 56, then 32, then
/// 25, then 22, then 15, then 9).
///
/// The 22 -> 15 drop is wave 4 (`#61`), and the per-file counts say exactly which
/// of the seven migrated screens carried a radius: `xref_manager.rs` 3 -> 0,
/// `layers.rs` 2 -> 0, `count_palette.rs` 1 -> 0, `layer_state_manager.rs` 1 -> 0.
/// Net −7 with no file growing, against the two snapshots
/// `D:\tmp\kit-baseline-before74.csv` and `D:\tmp\kit-baseline-freeze116.csv`.
/// The other three migrated screens (`pdf_dialogs`, `plot`, `plugin_manager`)
/// contributed nothing here.
///
/// The 15 -> 9 drop is a rename, not a redesign: six rows already spelled the
/// `RADIUS_MARK` number as a literal (`status_menu.rs:94`, `gradient_editor.rs:311`,
/// `graphic_attributes.rs:738/871/934`, `xref_attach.rs:233`) and now read
/// `tokens::RADIUS_MARK`. Ledger `D:\tmp\fold-b-radius-rows.csv`; the proof
/// (`D:\tmp\prove-zero-pixel-b-radius.cjs`) reads the rung out of `tokens.rs` at
/// proof time, asserts no *second* rung claims that value (so the name can only
/// resolve one way), and re-reads each tree line for the written text. 6/6 landed,
/// four files, deleted = added = 6, line counts unchanged. `status_menu.rs` held
/// exactly this one guarded row, so the guarded file count legitimately fell
/// 80 -> 79; it is not a dropped scan.
///
/// The 9 -> 2 drop is #63, and unlike the run above it this one *is* a
/// redesign: the seven `radius: 1.0` colour chips did not get renamed to a rung,
/// they got a rung. Two became `kit::swatch_button_style`, one
/// `kit::swatch_plate_style`, one `kit::mark_plate_style` and three
/// `kit::swatch_style`, all of which spell `tokens::RADIUS_MARK` inside L1, which
/// the measurement does not scan. That last clause is the honest caveat: this run
/// reduces the column by relocating the decision into the layer that is allowed
/// to own it, and it is only distinguishable from gaming the counter because the
/// corner value changed at the same time (1.0 -> 2.0 on ~332 rendered cells) and
/// because three of the seven were one block copied into two files. The pixel
/// moves are itemised under the 109 -> 102 drop in column (a), which is where they
/// show up as well.
///
/// Both rows named above are gone as of the 25th freeze, and the reason spelled
/// here for keeping them was only ever true of one: `ribbon/collapse.rs:624`
/// sits inside a `Border` of `width: tokens::EDGE_HAIRLINE` = 1.0, so its corner
/// does paint — a square, not nothing. It was therefore renamed to
/// `tokens::RADIUS_FLAT` (0 px, value unchanged) instead of argued away, and
/// `app/view/overlay.rs`'s square went in a wave between the two freezes. Column
/// (b) now measures 0 against a ceiling of 0, which is where this paragraph's
/// caution retires itself: the rung already existed, so naming the survivors
/// moved no pixel and invented no tier.
///
/// Do not read the per-file list as stragglers awaiting a wave: a row-level diff
/// of the CSVs is the only honest attribution here, because wave 4 edited those
/// files hard enough that every surviving line number shifted. Counts per file
/// are stable evidence; `file:line` pairs across the two snapshots are not.
///
/// Re-frozen at **1** by that same run, and it is a rename rather than a re-decision:
/// `context_menu_row_style`'s `radius: 0.0.into()` is a *deliberate* square — menu rows
/// are stacked full-width and hit-tested to `MENU_ROW_H`, so a corner would round away
/// the seam that says which row the pointer is on. L0 now names that zero
/// (`tokens::RADIUS_FLAT`), which is also why the doc on `RADIUS_FIELD` had to
/// stop claiming it is the tightest corner in the app: it is the tightest
/// *rounded* one, and a flat square is now a named rung rather than a leftover.
const BASELINE_OFF_TOKEN_RADIUS: usize = 0;

/// (b2) asymmetric corner radii spelled out as `Radius { top_left: N, .. }`.
/// Frozen at the measured **12** on 2026-10-16, the day the (b) column's blind
/// spot was found (`D:\tmp\t80-corners.log` holds the 12 rows). Its own category
/// rather than extra rows in (b), for the same reason as (a2): (b) is frozen at a
/// number that was measured by a rule which never saw these, and restating it
/// would make a past run look like progress it did not contain.
///
/// Re-frozen at **0** the same day by task #77, which is the first column in this
/// file to be closed rather than ratcheted: the 12 rows were `xref_manager.rs` 8
/// (a split button's two halves, four corners each) and `drafting_settings.rs` 4
/// (a tab's top corners), and `kit::Join` / `kit::joined_btn` now own that shape,
/// so there is no remaining reason for a screen to spell a corner. Both hosts are
/// inside the counted scope, `src/ui/kit/` is not, so `join_radius` is the only
/// place left that may write `Radius { .. }` — which is the whole point.
///
/// What the zero floor means in practice: a new joined shape is a new `Join`
/// member, not a new literal. If a screen genuinely needs an asymmetric plate
/// that `Join` cannot express, the honest move is to say here why the enum needed
/// a fourth arm and raise this number on purpose; what must not happen is a
/// `Radius { .. }` in a screen that simply nobody looked at.
const BASELINE_OFF_TOKEN_CORNER_RADIUS: usize = 0;

/// (c) `.spacing(<literal>)` hard-coded gaps. Re-frozen 2026-10-16: **792**
/// (was 1022, then 1016, then 1015, then 824; wave 3 moved 191 call sites onto
/// the gap ladder). Batch 5 left this number exactly where it was, and that was
/// a real scope fact rather than a blind spot: the freshly generated per-file CSV
/// still listed 12 spacing rows in `graphic_attributes.rs` and one in
/// `sheet_set.rs`, i.e. batch 5 migrated style literals and radii only and never
/// touched a `.spacing()` call. A category that holds still across a wave is
/// worth checking against the CSV before it is read as progress.
///
/// The 824 -> 792 drop is wave 4 (`#61`) and is *not* spread over its seven
/// screens: `xref_manager.rs` 26 -> 0 and `plugin_manager.rs` 6 -> 0, and the
/// other five migrated screens did not have a single `.spacing()` literal to
/// give up. The same CSV pair shows the surviving long tail untouched and in
/// fact untouched by anything in this migration: `pdf_dialogs.rs` 71,
/// `drafting_settings.rs` 50, `dimstyle.rs` 47, `annotation_data.rs` 40, `plot.rs`
/// 35 — dense settings forms whose gap decisions were still local *as measured
/// then*. Task #91 folded the on-ladder half of all of them, so at that freeze
/// the same column topped out at `plot.rs` 17 / `block_definition.rs` 14 /
/// `drafting_settings.rs` 12. `block_definition.rs` has since gone to 0 through
/// #42, and the column's heads are `plot.rs` 17 / `drafting_settings.rs` 12 /
/// `annotation_data.rs` 11.
///
/// The 792 -> 790 drop is task #78, and it is deliberately two rows rather than
/// six: `layers.rs` still holds `.spacing(2)` / two `.spacing(4)` / `.spacing(0)`
/// / two `.spacing(5)` on lines this task never rewrote. The rule used here is
/// that a literal is folded when its own line is being rewritten, not reached
/// for from across the file — folding the two column gutters on their own would
/// move the header row relative to the data rows unless both were moved
/// together, and that is the layers panel's own density decision (#45), as is
/// `5.0`, which sits between `GAP_TIGHT` and `GAP_CONTROL` and so needs a
/// measurement rather than a rounding.
///
/// Re-frozen at **151** the same day by task #91, the mechanical fold that
/// renamed every on-ladder scalar `.spacing(<literal>)` to its `tokens::GAP_*`
/// name. This is not a migration: nothing was re-decided, and the proof is
/// `D:\tmp\prove-zero-pixel-cf.cjs`, which re-reads the ladder out of `tokens.rs`
/// and checks all 735 ledger rows in `D:\tmp\fold-cf-scalar-rows.csv` for
/// numeric equality (735/735 equal, 0 off-ladder). `GAP_*` is `f32`, so `4` and
/// `4.0` land on the same rung and neither row moved.
///
/// What survived is the part a rename cannot reach: 151 rows sitting on nine
/// distinct numbers, `5` (42), `7` (41), `3` (29), `1` (21), `18` (7), `9` (5),
/// `14` (4), `20` (1), `3.0` (1), none of which is a `GAP_*` rung. Those are an
/// L0 ladder decision (#A1c), not a folding job, and each one moves pixels on
/// the screens that made it.
///
/// Re-frozen at **146** by the first #42 calibration screen
/// (`ui/window/layer_state_manager.rs`, 5 of its rows). Unlike the two runs
/// above this one *is* a migration: five rhythm literals were re-decided by
/// role and each moved a pixel. `D:\tmp\prove-s42-calibration.cjs` recomputes
/// every row's drift out of `tokens.rs` rather than trusting the ledger's
/// column, and cross-checks that the ceiling minus the fresh count equals the
/// number of ledger rows for that column — so a run booked without its rows is
/// refused. Distribution after the run, tallied by `D:\tmp\tally-kit-baseline.cjs`:
/// eight distinct numbers, `5` (41), `7` (37), `3` (30), `1` (21), `18` (7),
/// `9` (5), `14` (4), `20` (1). `3.0` folded into `3`.
///
/// Re-frozen at **132** by that same screen's second pass, #42's
/// `block_definition.rs` (ledger `D:\tmp\s42-block-definition-ledger.tsv`, 14 of
/// its rows in this column). This run moves pixels like the one above it: 13
/// rhythm literals were re-decided by role onto `GAP_*` rungs and one `.spacing(5)`
/// was deleted outright rather than renamed, because the row it separated has a
/// single child — an inert call whose landing would have been `GAP_NONE` and whose
/// drift is therefore recorded as `INERT:` in the ledger, exempted by
/// `D:\tmp\prove-s42-screen.cjs` instead of quietly counted as 0. The value
/// histogram shows exactly where the 14 went and nothing else: `5` 41 -> 33 (-8),
/// `3` 30 -> 26 (-4), `1` 21 -> 19 (-2), and `7` untouched at 37, which is the
/// check that this screen did not reach into another file's rhythm. Eight
/// distinct numbers survive, same eight as before: `7` (37), `5` (33), `3` (26),
/// `1` (19), `18` (7), `9` (5), `14` (4), `20` (1), over 44 files.
///
/// Re-frozen at **121** by #42's third screen (`ui/window/annotation_data.rs`,
/// ledger `D:\tmp\s42-annotation-data-ledger.tsv`, 11 of its rows, 11 px — every
/// rhythm literal on that screen was one step from a rung). The histogram moves
/// only where this screen reached and invents nothing: `7` 37 -> 32 (the fieldset
/// title->body now owned by `kit::group_box`, three wizard page bodies onto
/// `GAP_CONTROL`, and the dialog's own outer column onto `GAP_ROW`), `5` 33 -> 29
/// (three fieldset bodies onto `GAP_TIGHT`, one wizard page body onto
/// `GAP_CONTROL` because its *sibling page* in the same wizard already wore it),
/// `3` 26 -> 25 and `1` 19 -> 18 (a tick list onto `GAP_TIGHT`, a radio stack onto
/// `GAP_HAIRLINE`, each matched to the other list / column of the same dialog).
/// `18` / `9` / `14` / `20` untouched, and the same eight values are still on the
/// board — `D:\tmp\diff-distinct.cjs` reports appeared: none, vanished: none for
/// this column. 121 rows over 43 files, headed by `ui/window/plot.rs` 17 and
/// `ui/window/drafting_settings.rs` 12.
///
/// Re-frozen at **120** by #42's fourth screen (`ui/window/attribute_editor.rs`,
/// one row): the attribute list's own `spacing(1)` goes to `GAP_HAIRLINE` (2),
/// because the tab strip at the top of that same dialog already wears it and `1`
/// is under the smallest rung — 1 px, and the only histogram bucket that moves
/// (`1` 18 -> 17, verified on the fresh board). 120 rows over 42 files, still
/// headed by `plot.rs` 17 and `drafting_settings.rs` 12, neither of which this
/// run reached.
///
/// Re-frozen at **119** by #42's seventh screen, one row: the same argument
/// repeats in `field_dialog.rs`, where the selectable list separated its rows
/// with `spacing(1)` while the plate around them already wore `GAP_HAIRLINE`.
/// `1` is under the smallest rung, so either landing moves a pixel: `GAP_NONE`
/// would let two adjacent selected washes fuse into one band, and the hairline
/// keeps the seam that says these are separate lines. 1 px per row, and the
/// bucket that moves is `1` 17 -> 16. Measured now: 119 rows over 41 files,
/// still headed by `plot.rs` 17 and `drafting_settings.rs` 12, then
/// `wblock.rs` 7 and `ribbon/mod.rs` 6.
///
/// Re-frozen at **117** by #42's eighth screen (`app/view/overlay.rs`, two rows,
/// 4 px between them): the annotative checkbox and its label were a bare iced
/// checkbox plus a second `text()` the screen wrote and spaced by hand, which is
/// exactly the pairing `kit::labeled_checkbox` owns — so the seam comes from that
/// helper at GAP_CONTROL (+3 px) rather than from a rung I picked. The other row
/// is the QSelect radio stack's `spacing(5)`: 5 belongs to no rung, and the
/// panel's other seams are 10/8/6, so it steps down to GAP_TIGHT (−1) to keep the
/// section separations reading as the larger step. Measured at that run: 117 rows
/// over 40 files, still headed by `plot.rs` 17 and `drafting_settings.rs` 12, then
/// `wblock.rs` 7.
///
/// Re-frozen at **100** by #42's tenth screen (`ui/window/plot.rs`, all 17 rows,
/// 49 px): nine of the dialog's panels were spacing a stack of labelled field rows
/// at a bare `7`, and 112 sites in this app already call that seam `GAP_FIELD`, so
/// each pays +3 px for the role while nearest-rung snapping offered −1 px for the
/// distance (finding F2 again, priced row by row in
/// `D:\tmp\s42-plot-ledger.tsv`). The three `spacing(14)` rows of the custom-sheet
/// editor take the same rung and lose 4 px each; the two scrolled checkbox lists
/// go 3 -> 2 and 1 -> 2 so that this dialog's two list branches name one seam, the
/// same rung `layer_state_manager.rs:140` and `print_all.rs:51` already name for
/// that shape; the two group columns take `GAP_SECTION` (+3) because a rule rides
/// in that gap; and the two main columns take `GAP_GUTTER`, the one row here where
/// role and nearest rung agree. Column (d1) measured 88 rows over 38 files at that
/// freeze and measures 7 rows over 5 files now, headed by
/// `ui/window/gradient_editor.rs` 3. The eleventh rebuilt screen took all 12 of its rows: one was
/// the dialog's private `let toggle`, a single literal drawn at eleven call sites;
/// three were loop-built mode rows; seven were readout rows no callback-taking
/// primitive could hold, which is what asked for `kit::static_checkbox`; and the
/// twelfth was the 20 px seam between the two snap-mode columns, which becomes the
/// gutter this dialog already pays between its two whole columns.
///
/// Re-frozen at **81** by #42's twelfth screen (`ui/window/wblock.rs`, 7 of its 26
/// rows, 6 px of seam — one of them booked at 0 px because it was inert). Measured
/// now: 81 rows over 37 files, headed by `ribbon/mod.rs` 6 and `xref_attach.rs` 6.
/// This screen is where the INERT exemption gets its second instance: the count
/// arm of the status row was `row![one text].spacing(5)`, a call that cannot move
/// anything, so it is deleted rather than re-tiered — the same shape
/// `block_definition.rs` opened, and the reason a board row can leave at 0 px while
/// the snapping plan priced it at 1. The other six are one dialog deciding a seam
/// once: three tight stacks of sibling rows inside a group take `GAP_HAIRLINE`
/// because the fourth stack in that dialog already named it, and three
/// icon-or-plate rows take `GAP_CONTROL` because the two glyph buttons in the same
/// file already named that.
const BASELINE_OFF_TOKEN_SPACING: usize = 0;

/// (d) `.size(<literal>)` hard-coded font sizes. The last figure of the *flat*
/// column was **1072** (re-frozen 2026-10-16; was 1336, then 1328, then 1316,
/// then 1073; the start-page wordmark, tagline and version now use the
/// `kit::display` / `kit::caption` / `kit::faint` voices), before the split below
/// cut it into the two ceilings that follow.
///
/// The 1073 -> 1072 drop is one row in `layers.rs` (6 -> 5) out of a wave that
/// tore down 63 style literals across seven screens, and the honest reading is
/// that this column barely moved *because of how the kit is called*, not because
/// the screens were already clean: `kit::btn` takes its content as an
/// `Element`, so every migrated call site still arrives as
/// `text(label).size(11)` and its number is counted here before L1 ever sees it.
/// Seven screens of migration therefore bought almost nothing in this column.
/// What did move it was `#58` on 2026-10-17, and the honest reading of that drop
/// is that the *counting* changed, not the architecture: a call site that wrote
/// 11 now writes `tokens::TEXT_LABEL`, which is the same number, named. The type
/// voice is still not pushed into the primitives — `kit::btn` still takes a bare
/// `Element`, so the screen still decides the point size, only now it decides it
/// in ladder vocabulary.
///
/// SPLIT INTO TWO COLUMNS ON 2026-10-17 (`#85`), because the flat column filed
/// rows under the wrong unit: iced's `checkbox(...).size(N)` is a **box edge
/// length** that wants `tokens::MARK_SQUARE`, not a font size that wants a
/// `TEXT_*` rung. Resolving the host is a walk back to the chain head, not a
/// same-line pattern match, and the walk changes the answer materially — the
/// row-level rule could only see 17 of the 116 box rows, so every earlier
/// reading of this column implicitly trusted 852 glyph rows to include them.
///
/// Measured by walking each `.size(` back through its own expression, with
/// string literals and comments blanked first (a `")" ` inside a string would
/// otherwise break the bracket pairing):
/// glyph **63** — was 957 until 2026-10-17, when task #58 folded the **894**
/// rows that already sat on a `TEXT_*` rung of the same family (11→LABEL 483,
/// 12→BODY 229, 10→MICRO 152, 14→HEADING 15, 15→SECTION 10, 20→TITLE 5). Every
/// one of them resolved to the number it already spelled, so no glyph moved.
/// What survives is exactly the residue the ladder cannot name: 48 rows of 13
/// (there is no 13 rung), 4 of 9, 4 of 10.5, one 28, one 22 — and 5 rows whose
/// number equals a token of *another* dimension (18 = `PAD_ACTION_X` ×4,
/// 16 = `GAP_GUTTER` ×1). A padding step is not a type rung, so those 5 are not
/// foldable by a script either; both buckets are #58's decision list, not
/// leftovers of this pass.
/// box **0** — was 116 until 2026-10-17, when task #58 folded every
/// hand-written edge into `tokens::MARK_SQUARE` (79 rows already equalled 14 and
/// moved no pixels; 30 were 15, 5 were 13, one 12 and one 16, so 37 rows moved
/// by 1 px). The ceiling is now zero: a mark box edge is one ladder value, and a
/// screen that wants a different one has to say so in L1, not in a call site.
/// unexpected **0** — no icon, container or unresolved host survives the walk.
/// 957 + 116 = the column's scope of **1073** rows, and 1073 = **1010** rows now
/// named by a token + **63** rows still off it.
/// Two further numbers are corrections of the record, not of the code: the whole
/// `.size(` population is **1074** occurrences over **1072** lines (the old
/// row-level rule counted a line, and two lines carry two calls), while this
/// column's scope is **1073** rows — the one outside it is
/// `ui/kit/control.rs:1201`, the kit's own `radio` helper,
/// which the whitelist excludes because L1 is allowed to spell an edge length;
/// and the earlier note of "116 checkbox / 97 field / 848 text / 11 unresolved"
/// was produced by a walk that passed *through* `::`, so `iced::widget::text(..)`
/// reported its host as `iced` and landed in the unknown bucket. Those 11
/// unknowns were the measurer's own defect.
///
/// Re-frozen at **61** by the first #42 calibration screen: two `13` group
/// headings became `tokens::TEXT_HEADING` (14), each a 1 px step, booked in
/// `D:\tmp\s42-calibration-ledger.tsv` separately from the inset drift so a
/// type move never inflates the pixel cost of a spacing decision.
///
/// Re-frozen at **60** by #42's second screen (`block_definition.rs`): its one
/// glyph row, the confirm-question text at old line 643, left `13` for
/// `tokens::TEXT_BODY` (12) — a step *down*, because that line is body copy
/// asking a yes/no question, not a heading, and there is no 13 rung to reach for.
/// Booked on its own line of `D:\tmp\s42-block-definition-ledger.tsv` and kept
/// out of the 65 px inset bill by `D:\tmp\prove-s42-screen.cjs`. The `13` bucket
/// is what moved and nothing else: 46 -> 45 occurrences, with `9` (5), `18` (4),
/// `10.5` (4), `28` (1), `16` (1), `22` (1) all unchanged. 60 rows now sit over
/// 25 files, headed by `app/view/modal.rs` 10; the occurrence count is one higher
/// than the row count because `gradient_editor.rs:362` carries two `.size(`
/// literals on a single line.
///
/// Re-frozen at **59** by #42's third screen (`ui/window/annotation_data.rs`):
/// its one glyph row, the wizard's title at old line 1182, left `13` for
/// `tokens::TEXT_HEADING` (14) — a step *up*, which is the opposite direction
/// from `block_definition.rs:643` and is decided the same way: that line was body
/// copy asking a yes/no question, this one is the dialog's only title with
/// `TEXT_LABEL` captions under it, and `13` is not a rung. 1 px, booked on its
/// own ledger line and kept out of the 38 px inset bill by
/// `D:\tmp\prove-s42-screen.cjs`. 59 rows over 24 files (was 25), headed by
/// `app/view/modal.rs` 10.
///
/// Re-frozen at **56** by #42's fourth screen (`ui/window/attribute_editor.rs`,
/// three rows — three of the six `13`s in that dialog; the other three were
/// `.text_size(13)` and are column (g), which this file alone held tree-wide).
/// All three take `tokens::TEXT_BODY`: a step *down* from 13, where the second
/// screen stepped down at `block_definition.rs:643` and the third stepped *up* at
/// `annotation_data.rs:1182`. The direction is not the decision — the decision is
/// that 13 is not a rung and these three lines are body copy in a form, which is
/// what `TEXT_BODY` is for. 1 px each, booked per row, kept out of the inset bill.
/// 55 rows over 22 files, still headed by `app/view/modal.rs` 10.
///
/// Re-frozen at **54** by that run: the inline on-canvas text edit wore `.size(13)`,
/// which has no rung, and every other text field in the app — including this overlay's
/// own numeric ones — is `TEXT_BODY` 12. One notch down, 1 px, booked per row.
///
/// Re-frozen at **50** by #42's eleventh screen (`ui/window/drafting_settings.rs`),
/// and this run is the whole `10.5` bucket: 4 rows left the column and the value's
/// tree-wide occurrence count goes 4 -> 0, because all four of them were this one
/// dialog's. Two went *down* to `tokens::TEXT_MICRO` (10) — the F5 cycling hint and
/// the polar-tracking explainer, both lines that must read quieter than the 11 px
/// field labels they sit under — and two went *up* to `tokens::TEXT_LABEL` (11),
/// the Select All / Clear All plate captions, which join the seven tab titles and
/// the three isometric plates already at 11 in the same dialog. Half a pixel each,
/// 2 px total, booked on their own ledger lines and kept out of the 52 px inset
/// bill. 50 rows over 20 files, still headed by `app/view/modal.rs` 10.
///
/// What this column still cannot see is the other half of the same screen's type
/// bill: the deleted private `let toggle` captioned its box at `tokens::TEXT_BODY`
/// (12) and the primitive it became captions at `tokens::TEXT_LABEL` (11), so
/// eleven of this dialog's master ticks silently lost a point. A rename between two
/// already-named rungs has no literal, so no column, no plan and no guard in this
/// repo can price it — finding F6, declared on that ledger as an
/// `unmeasured_type_step` row at 0 px with the 1 pt stated in prose instead.
///
/// The chain above stops at **50** while the ceiling had already been written down
/// at **41**. That is a gap in this doc, not in the arithmetic: three later runs
/// drained this column and were priced only where the board is tallied — the
/// per-column drops sit in their screens' ledgers
/// (`D:\tmp\s42-rib-ledger.tsv` 3 rows, `D:\tmp\s42-shortcuts-ledger.tsv` 4,
/// `D:\tmp\s42-cpal-ledger.tsv` 2) and the resulting totals in the before-chain
/// below — and 50 − 3 − 4 − 2 = 41, so the two agreed all along and this paragraph
/// is a pointer, not a correction. Naming the gap instead of backdating three
/// paragraphs that no run measured.
///
/// Re-frozen at **31** by #42's seventeenth screen (`app/view/modal.rs`, 10 of its
/// 18 board rows, 11 px). Every one of the ten was `.size(13)` — the bucket this
/// column exists for, and it goes 34 -> 24 tree-wide while the guarded rows go 41
/// -> 31 (the 25th occurrence is `ui/kit/control.rs`, which L1 is allowed to
/// spell). The landing is decided by what each line *is*, not by which rung is
/// nearest: eight go to `TEXT_BODY` at 1 px (three field captions' text, five
/// dialog bodies that ask the user to do or accept something), one goes *up* to
/// `TEXT_HEADING` at 1 px (the GPU warning's headline, joining the four siblings
/// that already headline a dialog that way) and one goes *down two notches* to
/// `TEXT_LABEL` at 2 px (the platform hint under that headline, which the file
/// already mutes with `kit::muted_style` — a quiet line that is bigger than the
/// body it comments on is the same defect #58 keeps meeting in the reverse
/// direction). The reason no 13 rung is on offer is written on `TEXT_TITLE`'s own
/// doc: the ladder is priced at a 1.25 ratio floor, and inserting 13 between 12
/// and 14 would break it twice over for one dialog's convenience.
///
/// That sentence was wrong, and #42's batch 029 (2026-10-19, the 27th freeze)
/// re-freezes this column at **7** on the strength of finding that out rather than
/// on the strength of a new primitive. The 1.25 floor lives in
/// `the_scale_is_monotonic_and_the_shadow_grows_with_lift`, and that test applies it
/// to two pairs only: `TEXT_TITLE / TEXT_SECTION` (1.33) and `TEXT_DISPLAY /
/// TEXT_TITLE` (1.9). The dense tier is governed by a strict order plus a 3 px band,
/// and 14 -> 15 already steps by 1 px inside it on the reason the test itself
/// records - those roles are separated by ink and position, not by size. A 13 rung
/// between `TEXT_BODY` (12) and `TEXT_HEADING` (14) breaks neither asserted pair,
/// so the seventeenth screen's "would break it twice over" was an inference from a
/// doc paragraph, not a reading of the law. `tokens::TEXT_META = 13.0` is now that
/// rung, and its place is asserted rather than paragraphed: two new lines in that
/// test pin `TEXT_BODY < TEXT_META < TEXT_HEADING` and `TEXT_META - TEXT_MICRO <=
/// 3.0`. Red-green was run on them - `TEXT_META` set to 14.5 panics at tokens.rs
/// with "TEXT_META left its place", and at 13.0 the test passes.
///
/// All 21 rows this link drains are the same renames: `.size(13)` typed as a digit
/// at 21 sites in 11 files, now `.size(tokens::TEXT_META)`. 0 px, because the number
/// handed to iced is the number that was already there - which is exactly the
/// distinction the whole column exists to keep: a row can leave the board without
/// moving a pixel, and a fold that moves a pixel cannot be booked as if it did not.
/// Two of the 21 sites (`ui/window/about.rs:59` and `:203`) are the only reason that
/// file is still on the board at all. Six files leave (37 -> 31), including
/// `ui/window/layer_translator.rs`, whose single wash row the previous link named as
/// the one thing keeping it unpinned; the same run folds that row (see the (h) note
/// on this file's `BASELINE_FROZEN` doc), so the promise is closed rather than renewed.
///
/// The 7 that link was waiting on arrived as a second ruling, and #42's batch
/// 032-B (2026-10-20, the 30th freeze) re-freezes this column at **0** on it.
/// The ruling was about laws, not numbers: 4 new rungs in `tokens`, then 7
/// renames onto them. `point_style.rs:192` 18 -> `TEXT_FOCAL`, `about.rs:193`
/// 28 -> `TEXT_HERO`, `auto_constrain_settings.rs:124` 16 -> `TEXT_PANE_HEAD`,
/// `gradient_editor.rs:340` 9 -> `TEXT_GLYPH_CAPTION`, `gradient_editor.rs:362`
/// twice on one line (`⇄` 18 -> `TEXT_FOCAL`, its caption 9 ->
/// `TEXT_GLYPH_CAPTION`) and `recovery.rs:22` 18 -> `TEXT_FOCAL`. 0 px at every
/// one of the 7: the number handed to iced is the number that was already there.
///
/// Each rung is named for what its sites *do*, and each carries the collision
/// that naming could not dissolve, written on the rung rather than buried here:
/// `TEXT_FOCAL` (18) is 3 sites with 3 different jobs - a dialog's title, a
/// metric's value, and a glyph used as an icon - and the first of those is the
/// job `TEXT_SECTION`'s doc already claims, so the 15/16/18/20 title cluster is
/// pinned by a band instead of by prose. `TEXT_PANE_HEAD` (16) is that same
/// pane-title role 1 px up. `TEXT_HERO` (28) is deliberately *not* called a
/// wordmark, because `crate::ui::kit::display` owns that word for
/// `TEXT_DISPLAY`. `TEXT_GLYPH_CAPTION` (9) sits under the floor `TEXT_MICRO`'s
/// doc used to claim was the bottom of the ladder, which is why that doc is now
/// rewritten instead of kept. All 4 collisions are handed to #58; a rung can be
/// a name for a job without the job being unique yet.
///
/// The law was moved before the numbers were, and its shape changed so the
/// proof costs one compile instead of three. `the_scale_is_monotonic_and_the_shadow_grows_with_lift`
/// now walks a 12-rung total order and prices each new position with its own
/// closure (`headline_clears` for the 4 headline steps, `cluster_fits` for the
/// title band, `floor_fits` for the sub-`TEXT_MICRO` step), and every closure is
/// run a second time against a value chosen so it MUST be false - a ladder with
/// `drifted[7] = drifted[6]`, `cluster_fits(19.0)`, `floor_fits(8.0)`,
/// `dense_fits(TEXT_HEADING)`. Batch 029 recorded red-green as an event that
/// happened once outside the test; this records it as a line inside the test, so
/// a future reader can see the assertion is capable of failing without
/// re-running a perturbation.
///
/// Two files leave the board (`ui/window/recovery.rs`,
/// `ui/window/auto_constrain_settings.rs`, 23 -> 21); the total goes 41 -> 34.
/// What the column still holds: nothing. At a ceiling of 0 the ratchet is no
/// longer a debt tracker but a ban - a hand-typed glyph `.size(N)` in an L3
/// screen now fails the guard the day it is written.
const BASELINE_SIZE_GLYPH: usize = 0;
const BASELINE_SIZE_BOX: usize = 0;

/// (e) a screen borrowing one of iced's own ready-made style functions:
/// `.style(button::text)`, `dialog_button(.., button::danger)`,
/// `let mut style = button::text(theme, status)`.
///
/// Frozen at the measured **101** on 2026-10-16, the day the blind spot was found
/// — and that number is itself a correction: the first freeze of this column read
/// 48, because the surface it scanned for was thirteen names typed from four
/// widget files. See [`vendor_style_fns`]; the column now derives its own surface.
/// (a) cannot see any of these rows: it looks for `<widget>::Style` followed by
/// `{`, which is the shape of a struct literal, and these lines name a *function*
/// that returns a `Style`. The screen never spells a field, so not one of the 101
/// matched a rule this file has ever had — while still deciding a look, by
/// pointing at vendor code instead of at `kit`.
///
/// Why that is the same offence and not a lesser one: the plate those 101 get is
/// composed inside `vendor/iced/widget/src/{button,container}.rs` from
/// `theme.palette()` directly. None of it passes through `ui/style/tokens.rs`, so
/// no number in it is a rung, and none of it is in the surface
/// `theme_accessibility_tests` measures. `button::text` is the clearest case —
/// rest ink `palette.background.base.text`, hover ink the same ink at
/// `scale_alpha(0.8)` and *no background at all*. So 25 call sites fade their text
/// on the pointer by an alpha this codebase never chose and never contrast-tested,
/// and their hover affordance is a dim rather than a plate, which is a different
/// answer to "what is clickable here" than every other control on the same screen.
///
/// The 101 live in 41 files. By borrowed look: `button::subtle` 30, `button::text`
/// 25, `button::danger` 22, `container::bordered_box` 15, `container::rounded_box`
/// 4, `button::warning` 2, `button::success` 2, `checkbox::primary` 1. By form: 82
/// pass the fn by reference to `.style(..)`, 14 hand it as an argument to a
/// screen-local helper or call it as a constructor (`modal.rs` ×6 via
/// `dialog_button(.., button::danger | button::warning)`, `ribbon/mod.rs:1490` and
/// `graphic_attributes.rs:806` which inline `button::subtle(theme, status)`, and
/// `layout_manager.rs:52`), and 5 bind it to a local then mutate iced's plate
/// (`node_graph.rs:721`, `statusbar_menu.rs:36`, `block_palette.rs:500`,
/// `gradient_editor.rs:348`, `layer_translator.rs:42`) — the worst of the five,
/// since it borrows a look and edits it in place.
/// The heaviest hosts are `statusbar/mod.rs` 10, `shortcuts.rs` 7, then
/// `sheet_set.rs` / `count_palette.rs` / `alias_editor.rs` / `modal.rs` at 6 each.
///
/// Closing it is four separate L1 questions, not one grep-and-replace, which is
/// why the column is frozen rather than immediately driven down:
/// - `button::text` (25): there is no text-only voice. `Role::Ghost` is
///   "clickable but unframed" and still paints a `WASH_POINTER` plate, and the
///   `kit::label` family has no clickable member. Either a link role joins `Role`
///   or these 25 accept the plate; the decision belongs to L1.
/// - `button::subtle` (30) and `container::bordered_box` / `rounded_box` (19):
///   these are the panels' *default frame*, and `kit` already owns frames —
///   `Role::Standard` and `kit::well_style` / `kit::card`. So most of these 49 rows
///   are screens that were never migrated, not a missing capability.
/// - `button::danger` (22) plus `warning`/`success` (4): `Role::Danger` exists but
///   rests neutral and arms into the ramp on the pointer, while iced's rests on
///   `danger.base` — a red plate. This is a rest-state decision, and mapping it
///   silently would repaint destructive buttons without saying so. `Role` has no
///   warning/success member at all.
/// - `checkbox::primary` (1, `plot.rs:841`): closed by the 35th freeze through
///   `kit::checkbox_style`, a style fn rather than a checkbox primitive — the
///   129 `checkbox(..)` constructions still build iced widgets; only this site's
///   sheet is L1's now.
///
/// Its own category rather than extra rows in (a), for the same reason as (a2)
/// and (b2): (a) carries a frozen chain back to 361 that was measured by a rule
/// which never saw these, and folding them in would restate history as progress.
///
/// Freeze provenance, stated plainly because it is not the usual kind. This
/// column has been frozen twice on the same day and the second freeze is a
/// correction of the first, not a decrease.
///
/// The first freeze (2026-10-16 21:4x) read **48** from a 13-name surface typed by
/// hand out of four widget files, at a moment when the lib test target could not
/// link (a concurrent session had an in-flight edit in `kit/surface.rs` +
/// `theme_accessibility_tests.rs`). The matcher was verified independently then:
/// its functions were copied verbatim into a standalone harness and agreed
/// row-for-row with the shell engine at 48. Both engines were right and the
/// number was still wrong, because the *surface* was an assumption.
///
/// The correction: the surface is derived from the fork (`vendor_style_fns`), and
/// both engines re-measured on the same tree. The Rust matcher (`is_whitelisted`,
/// `is_word_char`, `vendor_style_fns`, `top_level_and_nested`,
/// `names_iced_builtin`) was extracted verbatim into `D:\tmp\e-check\items.rs` —
/// only `env!("CARGO_MANIFEST_DIR")` replaced by the literal manifest dir —
/// compiled and run: `D:\tmp\t83-standalone.log` reports surface **40** names,
/// `files_scanned=674`, `builtin_style_call=101`. The grep + awk engine
/// (`D:\tmp\measure3.sh`, now generating the same list from `vendor/iced/widget/src`)
/// reports `D:\tmp\t83-measure.log`: a=109 / a2=42 / b=15 / b2=0 / c=790 / d=1072
/// / **e=101**, 2 210 CSV rows, snapshot kept as `D:\tmp\kit-after81b.csv`. The two
/// derived name lists diff identical and `comm -3` over their two 101-row
/// `file:line` sets is empty, so the 101 is not one engine's quirk. The in-crate
/// run then landed green: `D:\tmp\t83-e101.log` prints `iced built-in style-fn
/// call sites flat at 101 (frozen 2026-10-16, surface 40 names)` with the six
/// older columns unchanged (109 / 42 / 15 / 0 / 790 / 1072), 7 passed / 0 failed,
/// and the full suite at 2317 passed / 0 failed / 25 ignored.
///
/// The second freeze (2026-10-17 14:1x) is a real decrease, not another
/// reading-scope correction: **101 -> 24**. Decision B moved 77 of the 101 rows into L1 by
/// giving the kit its own `subtle_style` / `text_style` / `danger_style` (30 + 25 +
/// 22), and those three stylesheets are *bit-identical copies* of the vendor ones,
/// asserted per theme and per status by `the_vendor_equivalents_are_exact` below
/// (312 comparisons, 0 mismatch). So this batch is the only (e) work that costs no
/// pixel: the remaining 24 rows are the ones that cannot be copied, because no kit
/// sheet exists to copy — 15 `container::bordered_box`, 4 `container::rounded_box`,
/// 2 `button::warning`, 2 `button::success`, 1 `checkbox::primary`.
///
/// Both engines re-measured on the same tree before the number moved: the shell
/// engine (`D:\tmp\measure3.sh` -> `D:\tmp\m82b.log`) and this crate's matcher,
/// and the per-name counts were re-derived from the source lines, not from the CSV
/// snippet field. Scope proof for the 77: `D:\tmp\fold-b-rows.csv` (77 `replace`
/// rows + 9 `insert-use` rows over 36 files) against `D:\tmp\pre-fold-b.tar`, where
/// every folded file's added-line count equals its removed-line count plus its one
/// inserted import, if it had one.
/// The third freeze (2026-10-17 14:4x): **24 -> 9**. Decision D folded the 15
/// `container::bordered_box` rows, and unlike B it could not be a copy — no kit
/// sheet reproduces that palette pair, so each row had to be sent to the tier its
/// host actually is, with the pixel cost printed first (`D:\tmp\t82-d-measure.log`):
///   * 10 detached bodies (six tooltip bodies, two menus, a drag ghost, a cycling
///     popup) -> `Tier::Float`: |ΔL*| 8.08 / 14.02 / 16.82, ink 26/26, corner
///     26/26 (5.0 -> 10.0), frame 26/26, plate source Ladder 26.
///   * 4 scrollable recesses (layout list, missing-font list, two recovery panes)
///     -> `Tier::Well`: |ΔL*| 0.98 / 5.94 / 6.29, ink 26/26, corner 0/26, frame
///     26/26, plate source Ladder 26.
///   * 1 status-bar readout chip (`status_pill`) -> the pill ladder's resting rung:
///     plate identical on 26/26, ink 0/26, corner 0/26, frame 26/26. This is the
///     zero-pixel row, and it is also the row that says the pill ladder is not on
///     L0's surface ladder: its plate source was iced's palette pair (printed as
///     `Control` since decision A took that ramp over), which is decision A's gap,
///     not this fold's.
/// The 15 needed no new L1 API. `Tier::style` is already documented as the sheet
/// for a call site that holds its own `Container`, and ten L3 files already spell
/// `.style(|theme: &Theme| kit::Tier::Float.style(theme, ""))`; the empty track is
/// the resting fade, which is what these hosts had before.
/// Scope proof: `D:\tmp\fold-d-rows.csv` (15 `replace` rows over 10 files) against
/// `D:\tmp\t82-d-backup.tar`, and `container::bordered_box` now reads zero outside
/// the whitelist. The remaining 9 rows are 4 `container::rounded_box`, 2
/// `button::warning`, 2 `button::success`, 1 `checkbox::primary`.
/// The fourth freeze (2026-10-17 16:1x): **9 -> 5**. Decision E folded the 4
/// `container::rounded_box` rows, and it was a copy, not a mapping, which is why it
/// is the second zero-pixel run of this column. `rounded_box` fills with
/// `palette.background.weak` and inks with `weak.text`; decision A (15:1x) had made
/// those two values *definitionally* the `Control::Weak` rung of L0's plate ladder, so
/// the kit could hand back the same struct. `kit::dialog_body_style` is that sheet —
/// plate and ink read from `tokens::control_plate` / `control_ink`, the border built by
/// the vendor's own `iced::border::rounded(tokens::RADIUS_MARK)`, and `shadow`/`snap`
/// taken from `..container::Style::default()` exactly as the vendor takes them. That
/// last point is the whole risk of the run: every other container style in
/// `kit::surface` writes `snap: true`, while the vendor's default `snap` is
/// `renderer::CRISP` (off in this manifest), so spelling it the kit way would have
/// moved pixels while every colour and corner still matched. The whole struct is
/// compared per theme by `the_vendor_equivalents_are_exact`, whose container arm now
/// runs alongside the button arm.
/// Scope proof: `D:\tmp\fold-e-rows.csv` (4 `replace` rows over 4 files) against
/// `D:\tmp\t90-backup.tar`; `container::rounded_box` reads zero outside the whitelist.
/// The remaining **5** rows are 2 `button::warning`, 2 `button::success`, 1
/// `checkbox::primary` — decision C and decision F, both explicitly parked by the user
/// on 2026-10-17 ("先做 E，剩下几个先放后面").
///
/// The fifth freeze (2026-10-21, decision C): **5 -> 1**. The four signal-button
/// rows left iced's sheets for `kit::signal_button_style`, each with the ground
/// its host actually paints:
///   * `app/view/modal.rs` (AEC drop): `Tone::Warning` on `Tier::Card`, the body
///     plate that dialog already uses.
///   * `ui/statusbar/mod.rs` (`warning_pill`): `Tone::Warning` on `Tier::Band`,
///     the full-width chrome strip it sits in.
///   * `ui/window/alias_editor.rs` and `ui/window/shortcuts.rs` (the draft row's
///     accept check): `Tone::Success` on `Tier::Flush`, because both guards build
///     on `kit::ground_style` and `Tier::Flush` *is* that pixel (`paint()` answers
///     `None` and `plane()` falls back to the theme base, `surface.rs:172-175`).
/// The fold is not free, and it is not a copy: the plate, the ink and the edge are
/// the signal chip's own rule (`painted(tone, SIGNAL_DIP_REST)` plus
/// `resolve_edge`), not iced's palette pair. Measured on the grounds the sites
/// actually sit on (`D:\tmp\s82c-guard2.log`, 26 themes, against
/// `button::warning` / `button::success` at rest):
///   * warning on `Tier::Card`: |ΔL*| 0.79 / 5.73 / 7.14 (min / median / max),
///     ink differs 26/26, corner 0/26, frame 26/26.
///   * success on `Tier::Flush`: |ΔL*| 0.15 / 5.12 / 6.64, ink 26/26, corner 0/26,
///     frame 26/26.
/// The corner is untouched because the iced button sheet and this one both use
/// `RADIUS_CONTROL`; the frame moves because iced draws no border at all where the
/// chip solves one against its plane. AA is not a threshold this fold chases: the
/// same painter's plate/ink/edge law is swept on all seven grounds by
/// `theme_accessibility_tests::the_signal_chip_carries_its_own_words`
/// (`D:\tmp\s82c-guard1.log`: 1456 chip/plane pairings, 0 failures).
/// `dialog_button_styled`/`dialog_button_styled_opt` and `modal.rs`'s local
/// `dialog_button` now take `impl Fn` instead of a fn pointer, because a
/// `signal_button_style(tone, ground)` result carries both decisions in its
/// environment; every pre-existing fn-pointer call site still compiles unchanged.
/// The column reached zero in the 35th freeze: `ui/window/plot.rs`'s
/// `checkbox::primary` became `kit::checkbox_style`, an L1 fn that keeps iced's
/// plate and frame untouched and dims only the tick and the label when the box
/// cannot be picked (decision F). The neutral-plate follow-up stays on #63.
const BASELINE_BUILTIN_STYLE_CALL: usize = 0;

/// (f) `.padding(<literal>)` — the inner padding a screen decides for itself
/// instead of taking it from the size ladder. Frozen at the measured **739** on
/// 2026-10-17, and this column did not exist before then: it was a blind spot,
/// not a clean sheet.
///
/// WHY IT WAS INVISIBLE. Every earlier measurer asked "does a digit follow
/// `.padding(`?" — which is the shape of the scalar call, `.padding(12)`. The
/// dominant shape in this codebase is the *pair*, `.padding([4, 10])`, where the
/// digit is behind a `[` and the test never looked. Measured on the same tree:
/// 133 scalar rows (what the old rule could see) vs 739 rows once each
/// comma-separated argument element is inspected (606 of them pairs). So the
/// category was under-reported by a factor of five while both engines agreed —
/// the same failure class as (e)'s hand-typed surface: a rule that looks like it
/// scans the whole family and in fact scans one spelling of it.
///
/// WHAT THE NUMBER MEANS. `tokens.rs` has no padding rung of its own; the ladder
/// lives in `kit::Size::{Action, Control, Compact, Icon, Flush}`, and each of
/// those already carries a padding pair. The heaviest literal in the histogram,
/// `[4, 10]` (71 rows when last sampled), re-spells `Size::Control`'s own
/// `PAD_CONTROL_Y`/`PAD_CONTROL_X` at the call site — a screen that writes it is
/// voting on L0's geometry without going through L0. That makes (f) the column
/// with the clearest fix path (call the primitive you are copying the numbers
/// of) and the clearest hazard (bulk-replacing it moves pixels on every panel in
/// the app, so it is rung-by-rung work under #58/#42, not a sed pass).
///
/// NOT COUNTED: `.padding(tokens::GAP_*)`, `.padding(kit::…)` and any argument
/// whose elements are all names. Half-tokenized pairs *are* counted, because the
/// literal half is still a local decision — there are exactly two of them in the
/// frozen set: `overlay.rs:1194` `.padding([MENU_PAD_TOP as u16, 0])` and
/// `properties.rs:1502` `.padding([COMBO_PAD_V, 6.0])`.
///
/// Provenance: `D:\tmp\t83-measure.log` re-measured the whole board at 00:01 on
/// 2026-10-17 and printed a=109 / a2=42 / b=15 / b2=0 / c=790 / d=1072 / e=101 /
/// **f=739** (2 949 CSV lines = the previous 2 210 plus 739 new rows, snapshot
/// `D:\tmp\kit-after83.csv`), 74 files. The a–e columns are unchanged from the
/// 2026-10-16 freeze, so adding (f) did not perturb them: the grep pre-filter
/// gained `\.padding\(` and each awk rule stays independent.
///
/// Re-frozen at **643** the same day by task #91, which renamed the on-ladder
/// *scalar* `.padding(<literal>)` calls to `tokens::GAP_*` — the same zero-pixel
/// proof as (c), 735 rows checked for numeric equality against the ladder read
/// back out of `tokens.rs`. It deliberately left the pair form `.padding([y, x])`
/// alone: a pair names two rungs at once, and `kit::Size::padding()` was a
/// private fn, so folding pairs needed that ladder published as a name first
/// (#92), not a wider sed.
///
/// Re-frozen at **519** by #92, which made `Size::padding()` public and folded
/// **124** pair rows (ledgers `D:\tmp\fold-cf-pair-rows.csv` 103 rows / 31 files
/// and `D:\tmp\fold-cf-pair2-rows.csv` 21 rows / 7 files) onto the rung whose two
/// numbers they already equal — `D:\tmp\prove-zero-pixel-cf-pair.cjs` reads the
/// rungs out of `control.rs` + `tokens.rs`, asserts each rung really is symmetric
/// (`[v, x]` means top=bottom=v, right=left=x in iced, so an asymmetric rung would
/// move pixels under a rename's name) and then re-reads the tree to confirm the
/// recorded line now carries that rung. That is the whole of it: **124 = 643 −
/// 519**, every pair on a ladder rung is gone, `D:\tmp\classify-f-pairs.cjs`
/// reports zero on-tier survivors.
///
/// Re-frozen at **513** by the first half of #93. The demand run
/// (`D:\tmp\ladder-demand.cjs`, results in `D:\tmp\ladder-demand.json`) put a
/// fine-grained ladder up against the 670 surviving (c)+(f) rows and rejected it
/// on measurement: `D:\tmp\spacing-drift.cjs` found that of the 139 odd-valued
/// `.spacing` calls in the guarded tree, **123 sit in a file that also uses the
/// adjacent even rung** (`dimstyle` uses 8 for rows 39 times next to 7 five times;
/// `drafting_settings` interleaves 6/7/8). Odd numbers there are hand-tuned
/// near-misses, not a rhythm, so naming them would turn `tokens.rs` into a number
/// lookup and call it a scale. What the same run did find is one real gap above
/// 16: the six modal-plate rows `.padding([24, 28])` — the only pair in the tree
/// that agrees with itself, appears more than once, and matches nothing on any
/// ladder. Those became `kit::Size::Plate` (L0 `PAD_PLATE_V`/`PAD_PLATE_X`) rather
/// than two `GAP_*` rungs, for the reason given on those constants: a plate inset
/// is not a gap between siblings, and the ladder stays eight rungs.
/// `D:\tmp\prove-zero-pixel-plate.cjs` re-reads L0, the `Size::Plate` arm and every
/// line of the `D:\tmp\fold-plate-rows.tsv` ledger, then asserts no
/// `.padding([24, 28])` literal survives: the rung resolves to the pair it
/// replaced, so **6 = 519 − 513** rows are renames and none is a resize. The
/// second half of #93 — snapping the remaining off-ladder values onto the ladder —
/// moves pixels by 1–26 each and is scheduled as per-screen #42 work against
/// `D:\tmp\ladder-snap.csv`, not as a fold on this board.
///
/// What is left in these 513, as measured after both folds: **465** pair rows on no
/// rung, 73 distinct pairs, headed by `[3, 6]` 68, `[5, 8]` 57, `[5, 12]` 26 —
/// those are the #42 snap rows; **37** scalars on no rung (`3` 27, `14` 3, `20` 2,
/// `1` 2, `5` 1, `9` 1, `18` 1); **5** half-tokenized pairs; **6** rows this
/// column's paren walk cannot parse. Heads are unchanged: `sheet_set.rs` 35,
/// `pdf_dialogs.rs` 30, `field_dialog.rs` 29, `overlay.rs` 25,
/// `layer_state_manager.rs` 25.
///
/// Re-frozen at **488** by the first #42 calibration screen, which took all 25
/// of `layer_state_manager.rs`'s rows off the board (heads now: `sheet_set.rs`
/// 35, `pdf_dialogs.rs` 30, `field_dialog.rs` 29, `overlay.rs` 25 — the
/// calibrated file no longer appears, and
/// `a_rebuilt_screen_never_takes_a_literal_again` is what keeps it out).
/// Same double check as (c): the rows are in `D:\tmp\s42-calibration-ledger.tsv`,
/// each landing resolves out of `control.rs` / `surface.rs` / `tokens.rs`, and
/// `D:\tmp\prove-s42-calibration.cjs` refuses the run unless the drop equals the
/// ledger. Distribution after the run (`D:\tmp\tally-kit-baseline.cjs`, 488 rows
/// over 70 files): **448** numeric pairs on 84 distinct values, headed by
/// `[3, 6]` 65, `[5, 8]` 53, `[2, 6]` 22, `[4, 12]` 22, `[3, 8]` 21, `[5, 12]` 20;
/// **35** scalars (`3` 26 the bulk); **5** half-tokenized pairs. The `[3, 6]`
/// head is finding F1 — those are `kit::PAD_FIELD` renames, not snap candidates,
/// and #42 must not pay a pixel for them.
///
/// Re-frozen at **475** by #42's second screen (`block_definition.rs`, 13 rows,
/// 52 px of the run's 65 px inset bill). Two of those rows are the point of the
/// whole column: `[3, 14]` (the hyperlink row beside the block list) and
/// `[20, 24]` (the confirm-modal plate) each appeared exactly once in the tree
/// and both appeared *here*, so this screen is the reason those two values left
/// the board entirely (`D:\tmp\diff-distinct.cjs` against
/// `D:\tmp\kit-baseline-before-bd.csv`: 84 -> 82 distinct, vanished = `[3, 14]`
/// and `[20, 24]`, appeared = none). Nothing else was invented either; every row
/// landed on a rung that already existed, chosen by what the region is rather
/// than by which number was nearest — `Size::Compact` x3 (286/348/518),
/// `PAD_WELL` x2 (187/541), `PAD_FIELD` x2 (302/505), `Size::Icon` x2 (357/606,
/// `Padding::ZERO`: the glyph box *is* the plate), and `Size::Control`,
/// `PAD_CARD`, `PAD_BAND`, `Size::Plate` once each. `D:\tmp\ladder-snap.csv`
/// booked four of the thirteen at 0 px, two of them as named kit renames
/// (232 -> `PAD_FIELD`, 541 -> `PAD_PLATE`) — finding F1 — and priced the two
/// singleton pairs at 3 px and 12 px by rounding `[3, 14]` to `[2, 12]` and
/// `[20, 24]` to `[16, 16]`, where the roles asked for 7 px and 8 px. The
/// per-row overrides are recorded in the ledger's note column.
/// Distribution after the run (`D:\tmp\tally-kit-baseline.cjs`):
/// **435** pairs + **35** scalars + **5** half-tokenized over **69** files, heads
/// `[3, 6]` 64, `[5, 8]` 52, `[2, 6]` 22, `[4, 12]` 22, `[5, 12]` 20, `[3, 8]` 19.
/// File heads are unchanged in order — `sheet_set.rs` 35, `pdf_dialogs.rs` 30,
/// `field_dialog.rs` 29, `overlay.rs` 25, `annotation_data.rs` 22 — with
/// `block_definition.rs` gone from the board and kept off it by
/// `a_rebuilt_screen_never_takes_a_literal_again`.
///
/// Re-frozen at **453** by #42's third screen (`annotation_data.rs`, 22 rows,
/// 27 px of the run's 38 px bill). Two values leave the board entirely and both
/// were this screen's alone (`D:\tmp\diff-distinct.cjs` against the retained
/// snapshot `D:\tmp\kit-baseline-before-ad.csv`: 81 -> 79 distinct, vanished =
/// the scalar `9` and `[4, 9]` x3, appeared = none). The scalar is the local
/// fieldset's `.padding(9)`, which is now `kit::group_box`'s `PAD_WELL`: the board
/// prices the helper once, the screen wears it 17 times, so the ledger records the
/// 4 px of that one row as a 17-fold repeat instead of hiding a per-call-site sum
/// the ratchet cannot see. The other 21 rows are call sites folded onto rungs that
/// already existed, by role: `Size::Control` x7 (four text fields in a
/// `label : field` row, the folder-picker button, and two dialog action buttons),
/// `Size::Compact` x5 (three browse buttons beside a field plus two rows of the
/// link list), `PAD_FIELD` x8 as pure F1 renames at 0 px, and `PAD_PLATE` once for
/// the tooltip body on a floating plate. The
/// planner priced the same 33 inset+spacing rows at 26 px; role-based landing
/// costs 38 px (1.46x — finding F2, third measurement point after 2.1x and
/// 1.76x). Composition after the run: **414** pairs + **34** scalars + **5**
/// half-tokenized over **68** files, heads `[3, 6]` 55, `[5, 8]` 52, `3` 26,
/// `[2, 6]` 22, `[4, 12]` 22, `[5, 12]` 20, `[3, 8]` 19, `[10, 12]` 19, with
/// `[4, 7]` down to 15 and `[5, 10]` to 14. `annotation_data.rs` leaves the head
/// list; `sheet_set.rs` 35, `pdf_dialogs.rs` 30, `field_dialog.rs` 29,
/// `overlay.rs` 25 and `properties.rs` 20 are now the five dialog families to
/// take in order.
///
/// Re-frozen at **444** by #42's fourth screen (`ui/window/attribute_editor.rs`,
/// nine rows, four of them `PAD_FIELD` renames at 0 px). The role-based landing
/// paid 16 px over the ten inset+spacing rows; nearest-rung snapping priced the
/// same ten rows at 5 px — **3.20x**, the fourth F2 reading after
/// 2.1x / 1.76x / 1.46x, and the widest gap yet. It is widest here because three
/// of the five moves are *family* decisions no distance rule can see: the Apply
/// verb goes to `kit::dialog_button` (Size::Action, 6 px) rather than the
/// `[4,12]` snap offered, the tab strip goes to `Size::Control` (2 px) rather
/// than the 0 px snap, and the value box goes to `PAD_FIELD` (1 px) rather than
/// the 0 px `PAD_PLATE` — every other `text_input` in the same dialog was already
/// on the field rung. Measured now: 444 rows over 67 files, **411** bracket-form
/// + **33** scalar-form, with the five heads above unchanged (this run took nine
/// rows out of a file that is not among them).
///
/// Re-frozen at **409** by #42's fifth screen (`ui/window/sheet_set.rs`, 35 rows,
/// 34 of them a real move and one `PAD_CARD` rename at 0 px). The planner priced
/// the same 35 rows at **28** px — **4.64x**, the fifth F2 reading and by now the
/// expected shape of a dialog rather than an outlier: 15 rows go to
/// `Size::Control` because a form's field band has to be one height, 7 to
/// `Size::Compact` because a tree row, a submenu header, a tab and a list row are
/// all the same dense line, 5 to `Size::Action` because a footer verb is wider
/// than the pane around it, 5 to `Size::Icon` where zeroing the inset and
/// squaring the box is cheaper *and* right, and 3 to named surface rungs. The
/// planner booked 3 rows at 0 px because `[3, 6]` is already `PAD_FIELD` and
/// `[10, 12]` is already `PAD_CARD`; a tree row is not a field, and that
/// disagreement is the finding, not an oversight. Composition measured after the
/// run: **376** pairs + **33** scalars over **66** files, 77 distinct values,
/// heads `[3, 6]` 50, `[5, 8]` 43, `3` 26, `[2, 6]` 22, `[4, 12]` 21, `[10, 12]`
/// 18, `[3, 8]` 17, `[4, 7]` 15, `[5, 10]` 13, `[5, 12]` down to 11 with the
/// dialog verbs moved. `sheet_set.rs` leaves the head list; `pdf_dialogs.rs` 30,
/// `field_dialog.rs` 29, `overlay.rs` 25, `properties.rs` 20 and `plot.rs` 15 are
/// the five families to take in order.
///
/// Re-frozen at **379** by #42's sixth screen (`ui/window/pdf_dialogs.rs`, 30
/// rows, 22 of them a real move and 8 renames at 0 px — and this run touched
/// *only* this column: (a) 94, (d1) 55 and (h) 9 are bit-for-bit where screen 5
/// left them). The planner priced the same 30 rows at **21** px, so **4.76x** is
/// the sixth F2 reading and the widest yet, and the reason is the same as the
/// fifth's: distance rules cannot see families. 13 rows go to `Size::Control`
/// because a dialog's field band, its browse partner and its value plate have to
/// be one height — and 4 sites in this very file already wore that rung three
/// lines away, which is the family argument made locally rather than by me. 3 go
/// to `Size::Compact` (a segment, a tab, a row switch), 3 to `Size::Action`
/// because every accepted footer in this batch spells its verbs with the action
/// rung (+8 px each, the widest single move here), 2 to `Size::Icon` where the
/// old box was 12 px of air around a 12 px arrow, 1 to `PAD_WELL` because that
/// row already paints `Tier::Well`, and 7 are `PAD_CARD` renames in a file that
/// already imported and wore the rung at 270. Composition measured after the
/// run: **345** pairs + **34** scalars over **65** files, 72 distinct padding
/// expressions, heads `[3, 6]` 48, `[5, 8]` 37, `[2, 6]` 22, `[4, 12]` 21,
/// `[3, 8]` 17, `[4, 7]` 15, `[5, 10]` 12, `[10, 12]` 11. `pdf_dialogs.rs`
/// leaves the family list; `field_dialog.rs` 29, `overlay.rs` 25,
/// `properties.rs` 20, `plot.rs` 15 and `attdef_dialog.rs` 14 are the five to
/// take in order.
///
/// Re-frozen at **350** by #42's seventh screen (`ui/window/field_dialog.rs`, 29
/// rows: 22 a real move, 3 renames at 0 px, 4 the in-card verbs that cost the
/// same either way). The planner priced those 29 rows plus one `spacing(1)` at
/// **31** px — **2.74x**, the eighth F2 reading and the *lowest* since the
/// calibration screen, which is the honest way to read it: this dialog had
/// already named most of its rungs, so the gap comes from one family argument
/// (14 field-band rows the snap wanted to scatter over `Size::Compact`) rather
/// than from many. The rows that cost the most are the two formula entry boxes
/// at 6 px each: they deliberately wore `[8, 8]`, i.e. they were taller than
/// every other field in the same dialog, and the ruling is that a dialog does
/// not get two field heights. Their 4 px of lost vertical air is visible and is
/// NOT signed off, same as the list rows' selection paint (Primary plate -> the
/// `list_row` wash) and the help verb's +8 px. Composition measured after the
/// run: **316** pairs + **34** scalars over **64** files, 70 distinct padding
/// expressions (32 of them mix a token name with a literal, so no head applies
/// to those), heads `[3, 6]` 48, `[5, 8]` 23, `[2, 6]` 22, `[4, 12]` 21,
/// `[3, 8]` 16, `[4, 7]` 15, `[6, 8]` 9, `[10, 12]` 9, `[5, 10]` 8. The
/// `[5, 8]` head fell by 14 in one run for the first time — that head *is* this
/// dialog family's field band, and the files still wearing it are the next
/// batch. `field_dialog.rs` leaves the family list; `overlay.rs` 25,
/// `properties.rs` 20, `plot.rs` 15, `attdef_dialog.rs` 14 and
/// `count_palette.rs` 13 are the five to take in order.
///
/// Re-frozen at **325** by #42's eighth screen (`app/view/overlay.rs`, 25 rows, 2
/// of them 0 px renames) — and this is the floating MText editor bar, the densest
/// surface in the app, so the family argument is unusually plain: one strip, one
/// height. 9 glyph cells go to `Size::Icon` at a 24.0 side (the 18.0 svg buttons
/// were already 3 + 18 + 3 = 24 wide, so those render identical and the 6 px the
/// ledger books is the inset moving into the side); 5 word verbs go to
/// `Size::Compact` (a word is not a glyph, so it does not get a square); 9 numeric
/// fields go to `PAD_FIELD` — and six of those nine rows existed only because the
/// screen wrote the same `text_input` chain inline instead of calling its own
/// `small_input` helper, which is why three call sites drain nine board rows. Two
/// of the 25 are the half-tokenised pairs this column's own doc had frozen as
/// unpriceable: the preview column's `[0.0, MTEXT_PREVIEW_PAD]` and the menu's
/// `[MENU_PAD_TOP as u16, 0]` became `[tokens::GAP_NONE, MTEXT_PREVIEW_PAD]` and
/// `[MENU_PAD_TOP, tokens::GAP_NONE]` at 0 px, the consts kept on purpose (the
/// preview pad is subtracted from the canvas' own coordinate transform, and
/// MENU_PAD_TOP is the popup's hit-tested band). Measured now: 305 rows over 62
/// files; the heads are `plot.rs` 15, `attdef_dialog.rs` 14, `ribbon/mod.rs` 14,
/// `count_palette.rs` 13, `wblock.rs` 12 — so `plot.rs` is next, and both
/// `overlay.rs` and `properties.rs` have left the head list even though the first
/// still owes three ink rows (named as deferred rows in
/// `D:\tmp\s42-overlay-ledger.tsv`).
///
/// The ninth screen is the widest F2 gap measured so far and the reason it is
/// worth reading: `properties.rs` priced at 55 px against the plan's 8 px over
/// the same 20 rows (6.88x, after 2.1 / 1.76 / 1.46 / 3.20 / 4.64 / 4.76 / 2.74 /
/// 4.60). Not one of those 55 px came from a resize the panel did not want. The
/// plan moves a value to whichever rung is nearest; the screen asked what the
/// control IS. Four glyph-only buttons went to `Size::Icon` with a square side
/// (18.0 / 20.0), which zeroes their inset and grows their hit target from a
/// ~14 px-tall strip around a 10 px arrow to a box you can actually aim at; the
/// popup list stopped re-deciding `kit::list_row`'s inset two lines after calling
/// it; three chips took `Size::Compact` because a chip in a row is a dense item
/// even when the row is 26.0 px tall; and the section header took `PAD_BAND`
/// because the panel above it already wears that rung. Six of the 20 rows were
/// 0 px renames, which is how the plan and the screen can agree on the destination
/// and still differ on why.
///
/// Re-frozen at **290** by #42's tenth screen (`ui/window/plot.rs`, 15 of its 33
/// rows, 25 px of inset). The rule this run leans on is that a row of buttons is
/// one row: Apply, Add and six secondary plates were the only exceptions to an
/// inset their own siblings had already named `kit::Size::Control`, so each gives
/// up 2 px of lateral air to match, while Commit — the one plate the dialog is
/// asking for — takes `Size::Action` and gains 2 px of height instead, which is
/// what keeps it heavier than its seven neighbours rather than merely wider. The
/// scroll body's `14` becomes `PAD_CARD` (+6: an asymmetric 10 vertical / 12
/// lateral, which is why a scalar could never have named itself) joining the four
/// screens that already wrap a dialog body that way; the footer band's `[5, 10]`
/// becomes `PAD_BAND` (+1) because it already wears `Tier::Band`; and the three
/// pickers plus the empty-state well are 0 px renames. Twelve of this screen's
/// hand-built plates now come from `kit::btn`, and that part moves no ink at all:
/// `button_style(accent)` *is* `role_style(Primary|Standard)` (`control.rs:1168`),
/// so the four `if … { on_press }` reassignments that became `Press` arguments
/// reach iced's Disabled status through the same closure they always did.
/// Measured now: 282 rows over 60 files, headed by `attdef_dialog.rs` 14 and
/// `ribbon/mod.rs` 14, then `count_palette.rs` 13. The eleventh rebuilt screen
/// took 8 of the column for 37 px, and one row is 22 of those 37: the dialog
/// body's `[14, 16]` lands on `kit::Size::Plate` `[24, 28]`, the fold
/// [`crate::ui::style::tokens::PAD_PLATE_V`]'s own doc pre-authorises by naming
/// that pair as drift from the six windows that wrote `[24, 28]`. The other 15 px
/// are the eleven fieldsets that went `[8, 10]` -> `kit::PAD_WELL` `[6, 8]` (priced
/// once at 4 px, drawn eleven times) and five label-left fields that went
/// `[4, 7]` -> `kit::PAD_FIELD` `[3, 6]` at 2 px each.
///
/// Re-frozen at **270** by #42's twelfth screen (`ui/window/wblock.rs`, 12 of its 26
/// rows, 32 px of inset). Measured now: 270 rows over 59 files, headed by
/// `ribbon/mod.rs` 14 and `attdef_dialog.rs` 14, then `count_palette.rs` 13. Three
/// of this screen's rows are the role beating the distance by more than a rename:
/// the browse plate `[3, 0]` lands on `kit::Size::Compact` for 9 px (its
/// `Length::Fixed(28.0)` and its hand-centring `container(..).align_x(Center)`
/// leave with it, because a tier sizes a plate and centre-aligning one;
/// properties.rs's two "..." plates are already on Compact), the dialog body
/// `[8, 10]` lands on `PAD_CARD` for 4 px joining five screens that already wrap a
/// body that way, and the dismiss cross `[1, 6]` lands on Compact for 5 px because
/// a 12 pt glyph inside a 1 px inset is a ~14 px click zone in a 20 px band — the
/// same defect that moved properties.rs's four glyph plates off 0-vertical strips.
/// The glyph-only quick-select plate is the one row where the plan and the role
/// disagree in the other direction: it priced `[3, 6]` at 0 px and the row pays 3,
/// because two plates in one row are one row of secondaries.
///
/// Re-frozen at **213** by **fold batch 006**, whose run type is the one #91/#92
/// established for this column — named sites across several files, not a screen
/// rebuild. Eight `.padding([4, 12])` / `.padding([5, 12])` menu lines were
/// pointed at the `kit::PAD_MENU_ROW` tier that #42's sixteenth screen named:
/// `ui/popup/isolate_popup.rs` 1, `ui/statusbar/mod.rs` 2, `ui/statusbar/statusbar_menu.rs`
/// 1, `ui/window/block_palette.rs` 1, `ui/window/xref_manager.rs` 3. One line per
/// site, nothing else in those five files touched, and the sites came from the
/// guard's own needle walk (`D:\tmp\apply-fold006.cjs`), so the count is the same
/// number the ceiling defends rather than a grep someone retyped. Seven of the
/// eight already wore the tier's exact pair and move no ink; `block_palette.rs`'s
/// `[5, 12]` pays the 1 px the ladder sends back to `GAP_TIGHT`, drawn 2 px
/// shorter. Measured now: 213 rows over **65** files — `isolate_popup.rs` leaves
/// the board because that one row was its whole debt. Five sites stay hand-typed
/// in the same family and stay on the board: they are verbs, and their migration
/// is priced against `kit::Size::Control`, not against this tier.
///
/// Re-frozen at **206** by #42's seventeenth screen (`app/view/modal.rs`, 7 of its
/// 18 board rows, 84 px — the widest inset bill a screen has yet carried, and the
/// reason this column's F2 ratio is now 4.42× where the thirteen earlier screens
/// spread 1.36×–2.85×). Four of the seven spend the pre-authorisation rather than
/// making a decision: the hyperlink and Save-As bodies leave `[14, 16]` for
/// `kit::Size::Plate` `[24, 28]` at 22 px each and the save-file-in-use and
/// external-change bodies leave `[18, 20]` at 14 px each, all
/// four landing on the fold [`crate::ui::style::tokens::PAD_PLATE_V`]'s own doc
/// names as drift from the six windows that wrote `[24, 28]` — 72 px of the 84 are
/// therefore signed off in L0, not by this run. The remaining 12 px are the three
/// label-left fields (`[5, 8]` -> `kit::PAD_FIELD` `[3, 6]`, 4 px each), and that
/// one *is* a role disagreement between two kit tiers rather than a distance call:
/// the snap plan offered `Size::Compact` `[4, 8]` for 1 px, but Compact's note says
/// "a dense row item or status pill" while PAD_FIELD's says "the inset inside a
/// field — the blank between the ink and the plate it is painted on", and these
/// three rows are `text_input` plates whose own point size the same run moved to
/// `TEXT_BODY`. The eleventh screen sent five of the same shape here; this screen
/// joins it rather than reopening the question. What the column cannot see at all
/// is the other 32 px this run paid: 38 `Space::new()` sides (finding F4), priced
/// on that ledger and invisible to every ratchet.
/// What the column cannot see has a third spelling, found while re-reading the four
/// runs whose shape gates turned out to be vacuous — they counted the literal word
/// `regex`, which never appears in Rust, so those 33 "the shape is gone" claims had
/// never been measured. An inset written as a struct and passed POSITIONALLY to an
/// L1 helper is invisible to the (f) element walk, which reads only the `.padding(`
/// argument list, and to this file's own F5 gate, which anchors on a `.padding(`
/// line whose tail opens `Padding {`: the thirteenth screen's tab strip called
/// `kit::surface(row, Tier::Band, Padding { right: 6.0, ..Padding::ZERO }, "")` and
/// proved clean. It now reads `tokens::GAP_CONTROL` — 0 px, and the twelve columns
/// measured identical before and after, which is exactly the evidence that the site
/// was never on this board. Two numbers the same re-read found are left standing on
/// purpose, because naming them would MOVE pixels rather than rename them: a
/// disabled-ink alpha typed as `scale_alpha(0.42)` in `ui/window/graphic_attributes.rs`
/// is not `INK_DISABLED` (this file pins that at 0.5), and the ribbon band's scrim
/// keeps its 0.62 in a screen-local const consumed by `scale_alpha`. No column counts
/// `scale_alpha` at all — the (h) surface is the L0 function names, and a number
/// behind a local `const` is invisible to a digit scan either way — so both are
/// priced in their runs' want files, in prose, and re-read before either is folded.
///
/// Re-frozen at **3** by #42's batch 029 (2026-10-19, the 27th freeze), and this one
/// is *not* free: two rows, `ui/window/gradient_editor.rs:449` and
/// `ui/window/print_all.rs:60`, both `[5, 18]`, land on `kit::Size::Action`
/// `[6, 18]`, so the vertical inset grows 1 px on each of the two plates. That is a
/// pixel move bought with a signature, not a rename - signed 2026-10-19 by the board
/// owner over the alternative of a new off-ladder `[5, 18]` inset, with 0 px being
/// the other option on the table. `print_all.rs` leaves the board on this row alone.
///
/// Re-frozen at **0** by #42's batch 032-C (2026-10-20, the 31st freeze). The 3 rows
/// that survived batch 029's ruling were the pairs 18/40, 18/22 and 16/20 at
/// `app/view/mod.rs:3023`, `ui/window/open_progress.rs:139` and
/// `ui/window/plugin_manager.rs:346`. This doc cited the middle one as 143 until this
/// freeze; the board row is the authority and the prose was wrong by 4 lines, which is
/// the kind of error no column can catch because both numbers are in a comment.
///
/// All 3 landed as renames, which is what the board owner signed for this column ("add
/// 3 pairs, rename, 0 pixels") instead of bending three plates onto tiers that would
/// have moved pixels. L0 gained 6 scalars (`PAD_HERO_V/X`, `PAD_PROGRESS_CARD_V/X`,
/// `PAD_NOTICE_V/X`) and L1 gained 3 composed pairs (`kit::PAD_HERO`,
/// `kit::PAD_PROGRESS_CARD`, `kit::PAD_NOTICE`); the axes were re-read out of this file's
/// own subject and compared against the pre-batch snapshot before the word 0 was
/// written, so the claim is arithmetic and not a restatement of the intent. 0 px over 3
/// sites, and 2 files leave the board: 34 rows / 21 files -> 31 rows / 19 files.
///
/// What the rename does NOT settle. 4 of the 6 new numbers duplicate a value that
/// another rung already carries in the same family — 18 twice against `PAD_ACTION_X`,
/// 20 against `PAD_PAGE`, 16 against `GAP_GUTTER` — so the ladder now holds two names
/// for one pixel in 4 places. No column can see that: the ratchet counts a literal
/// leaving L3, and a named rung is silent whichever way its value points. That is
/// #58's ruling, opened by batch 029 and widened here, and the 12 positional `Length`
/// sides still hand-typed at L3 are in the same custody. Priced on
/// `D:\tmp\s42-b032c-ledger.tsv`: 3 board rows and 7 lines the board cannot count
/// (6 new L0 rungs, which price sites but draw none, and 1 export-list change).

const BASELINE_OFF_TOKEN_PADDING: usize = 0;

/// (g) `.text_size(<literal>)` — the point size of a widget's **own label**.
///
/// FOUND, NOT INHERITED: this family is older than the column and was counted by
/// nothing. (d) walks `.size(` back to its constructor, and a `checkbox` answers
/// that call with its **box edge**; the label's font is a *second* setter on the
/// same widget — `vendor/iced/widget/src/checkbox.rs:190 pub fn text_size(…)`
/// sits right under `:172 pub fn size(…)`, and `radio.rs:213`, `toggler.rs:173`,
/// `pick_list.rs:236`, `overlay/menu.rs:92` repeat the pair — so a screen could
/// pin its glyph to the ladder with `.size(tokens::MARK_SQUARE)` and then write
/// `.text_size(11)` one method later, invisibly. Fourth instance of the same
/// error class on this board ((e) hand-typed surface, (f) one spelling of a pair,
/// (d) one unit under one method name): a rule that covers one spelling of a
/// family claims the family.
///
/// Measured when the column opened (2026-10-17 02:3x): **98** calls over **27**
/// files, values `11` ×47, `12` ×46, `10` ×2, `13` ×3. Three of those four
/// numbers are already `TEXT_*` rungs, so 95 of the 98 are an equal-value fold —
/// the same kind of batch as #58's glyph pass, no pixels moved. The three `13`s
/// are deliberately left counted rather than waived: they belong to #58's
/// decision (no 13 rung, merge per role), which is per-screen work under #42.
///
/// NOT COUNTED: `.text_size(tokens::TEXT_*)`, and the other `*size*` setters of
/// the fork that are not a point size — `qr_code::cell_size` / `total_size` (a QR
/// module is a length) and `pane_grid::min_size` — which is why the surface is
/// derived from the vendor by *name plus argument type* (see
/// [`vendor_text_size_fns`]) rather than by "anything with 'size' in it".
///
/// Counted over the blanked file, not per line: a call split across lines must
/// not drop out of this column the way the `[a, b]` form dropped out of (f)'s
/// first rule. There are no split calls in the tree today (`0` lines end with an
/// open `.text_size(`), and the shape of this rule does not depend on that.
///
/// FOLDED the same day it was opened (2026-10-17 02:3x, `D:\tmp\fold-g.log`):
/// **98 -> 3**. The 95 swaps are `11 -> TEXT_LABEL` ×47, `12 -> TEXT_BODY` ×46,
/// `10 -> TEXT_MICRO` ×2 — each number already equalled the rung it was renamed
/// to, so every row is a rename and none is a resize. Scope proof in
/// `D:\tmp\fold-g-scope.txt`: 27 files written, **95** lines removed, **26** of
/// them keep their exact line count (in-line swaps only) and `viewcube.rs` grows
/// by one `use crate::ui::style::tokens;`, the file that had no `tokens` in scope.
/// What moved is the *vocabulary* the screen writes its point size in, not the
/// size — the same honest reading as #58's glyph fold: the widget still takes a
/// bare number of points from the call site, it just takes it from the ladder.
/// The three `13`s that stayed counted were the last of the column, and they sat
/// in `attribute_editor.rs` — the only file in the tree where a fold (11/12 were
/// already rungs) and a hold (13 has no rung) sat in the same function. The
/// fourth #42 screen drained them (2026-10-18, ledger
/// `D:\tmp\s42-attribute-editor-ledger.tsv` rows 247/290/572): each `13` went to
/// `TEXT_BODY`, so this is a one-notch *resize*, not a rename, and it is priced
/// at 1 px per row in the proof script — which now resolves a step row against
/// L0 instead of pinning it to the word "step". (g) is empty tree-wide, so the
/// ceiling below is 0 and any future `.text_size(n)` fails the build.
const BASELINE_OFF_TOKEN_TEXT_SIZE: usize = 0;

/// The date the constants above were last re-frozen. The green "flat at"
/// messages print this instead of a literal date, because a hardcoded one goes
/// stale on the next re-freeze and then reports a measurement that never
/// happened on that day. Update it in the same edit as the numbers.
/// 2026-10-17 is not one re-freeze but a day of them. Read off two snapshots of
/// the same engine (`D:\tmp\kit-after83.csv` at 00:01, `D:\tmp\kit-baseline.csv`
/// now): (a) 109 -> 102, (b) 15 -> 2, (c) 790 -> 151, (d) 1 072 -> 63 glyph + 0 box
/// after the split, (e) 101 -> 5, (f) 739 -> 513, (g) 98 -> 3. (a2) 42, (b2) 0 and
/// (d3) 0 held *in that run*; (h) 10 has no constant here because the alpha column
/// is measured only by `D:\tmp\measure3.sh` and aggregated by `D:\tmp\progress.sh`,
/// which is an asymmetry rather than a pass — a column the guard cannot fail is a
/// column that cannot be guarded. Each number above is the output of a measured run
/// rather than of an edit, and each column's own doc carries the ledger and
/// zero-pixel proof for its step.
///
/// Ten #42 screen rebuilds came after that pair, and they are the runs today
/// that moved pixels on purpose. The calibration screen
/// (`layer_state_manager.rs`) took (c) 151 -> 146, (f) 513 -> 488, (d1) 63 -> 61;
/// its before/after is proved by `D:\tmp\prove-s42-calibration.cjs` against the
/// ceilings of that moment, since no board snapshot was kept for it.
/// `block_definition.rs` then took (a) 102 -> 100, (a2) 42 -> 37, (c) 146 -> 132,
/// (d1) 61 -> 60, (f) 488 -> 475, measured against the retained snapshot
/// `D:\tmp\kit-baseline-before-bd.csv` and proved by
/// `D:\tmp\prove-s42-screen.cjs`. The third screen is `annotation_data.rs`,
/// measured against the snapshot kept for it,
/// `D:\tmp\kit-baseline-before-ad.csv`: (a) 100 -> 99, (a2) 37 -> 36,
/// (c) 132 -> 121, (d1) 60 -> 59, (f) 475 -> 453. The fourth is
/// `attribute_editor.rs`, measured against `D:\tmp\kit-baseline-before-ae.csv`:
/// (a) 99 -> 95, (c) 121 -> 120, (f) 453 -> 444, (d1) 59 -> 56, and the two
/// columns no earlier screen ever met — (g) 3 -> 0 and (h) 10 -> 9. (g) is now
/// empty tree-wide, which means the last `.text_size(n)` in this codebase was in
/// this one dialog. A fifth screen, `sheet_set.rs`, then drained 37 rows against
/// `D:\tmp\pre-s42-sheetset.csv`: (a) 95 -> 94, (d1) 56 -> 55, (f) 444 -> 409, and
/// L1 gained `PAD_BADGE` for it. The sixth is `pdf_dialogs.rs`, measured against
/// `D:\tmp\pre-s42-pdfdialogs.csv`: (f) 409 -> 379 and nothing else, because that
/// screen had already named every one of its font sizes and plates. The seventh
/// is `field_dialog.rs`, measured against `D:\tmp\pre-s42-field-dialog.csv`:
/// (a) 94 -> 91, (c) 120 -> 119, (f) 379 -> 350 — the first screen since the
/// calibration one to take rows out of two columns besides its own, and it did
/// so by *promoting* code rather than deleting it (`kit::field_container_style`).
/// An eighth, `app/view/overlay.rs`, took (a2) 36 -> 35, (b) 2 -> 1,
/// (c) 119 -> 117, (d1) 55 -> 54, (f) 350 -> 325 against
/// `D:\tmp\pre-s42-overlay.csv` — 32 of its 33 rows, the other 3 named as
/// deferred in its ledger instead of being quietly dropped. The ninth is
/// `ui/properties.rs`, measured against `D:\tmp\pre-s42-properties.csv`:
/// (a) 91 -> 90 and (f) 325 -> 305, its 21 rows drained whole, plus 13 further
/// insets that no column counts at all (F5, see `BASELINE_OFF_TOKEN_PADDING`).
/// The tenth is `ui/window/plot.rs`, measured against
/// `D:\tmp\pre-s42-plot.csv`: (c) 117 -> 100 and (f) 305 -> 290, 32 of its 33
/// rows, the 33rd being the one `builtin_style_call` (#82 decision F, parked by
/// the user) that its ledger declares deferred rather than drops. This run closed
/// a hole in the proof itself on the way: `D:\tmp\prove-s42-screen.cjs` kept its
/// board columns in a hand-typed table that had never carried (e), so the rule
/// "every board row this screen owned must have a ledger line" had been silently
/// applying to seven of the eight columns for nine screens. plot.rs is the first
/// pinned screen to own an (e) row, the deferral made the proof die instead of
/// ignore it, the column was added, and all nine earlier proofs were re-run after
/// that edit (8 PROVEN / 1 documented skip, `D:\tmp\s10-proof-regress.log`).
/// The eleventh is `ui/window/drafting_settings.rs`, measured against
/// `D:\tmp\pre-s42-draftset.csv`: (c) 100 -> 88, (f) 290 -> 282, (d1) 54 -> 50,
/// (a) 90 -> 88 and (a2) 35 -> 33, all 28 of its rows and none deferred. It is
/// the first run to grow an L1 primitive on the way (`kit::static_checkbox`,
/// because seven of this dialog's checkbox rows are readouts and a
/// callback-taking primitive would make them lie about being clickable), and it
/// is where finding F6 was named: a rename between two already-named rungs
/// (`TEXT_BODY` -> `TEXT_LABEL`, 12 -> 11 pt at eleven painted rows) is invisible
/// to the board, the snap plan and this guard at once, so it is priced in prose on
/// `D:\tmp\s42-draftset-ledger.tsv` rather than pretending to be 0 px.
/// The twelfth is `ui/window/wblock.rs`, measured against
/// `D:\tmp\pre-s42-wblock.csv`: (c) 88 -> 81, (f) 282 -> 270, (a) 88 -> 87 and
/// (a2) 33 -> 27, all 26 of its rows and none deferred. Two things happened here
/// that had not happened on a rebuilt screen before. The first is the second
/// instance of F6, and it moves the *other* axis: the deleted local fieldset
/// spaced its caption from its body by `GAP_TIGHT` (4) while `kit::group_box`
/// spaces them by `GAP_CONTROL` (6), so four boxes opened 2 px in a change that
/// contains no literal at all — named to named, invisible to board, plan and
/// guard alike, and priced in prose on that ledger too. The second is that this
/// screen's largest single row (the browse plate, 9 px) was not a resize choice
/// between two rungs but the removal of three shapes that were hand-simulating a
/// tier: a fixed width, a centring container and a `[3, 0]` inset.
/// The current twelve
/// columns therefore read
/// 59 / 23 / 1 / 0 / 51 / 157 / 0 / 28 / 0 / 0 / 5 / 6, a board total of
/// **330** over **60** files. That sentence was this freeze's reading and is now
/// the twenty-fourth freeze's: the board after batch 032-B (2026-10-20, the 30th
/// freeze) read 23 / 1 / 0 / 0 / 0 / 3 / 0 / 0 / 0 / 0 / 5 / 2, a total of **34**
/// over **21** files. After batch 032-C (the 31st freeze) the twelve read 23 / 1 / 0 / 0 / 0 / 0 / 0 / 0 / 0 / 0 / 5 / 2, a
/// total of **31** over **19** files. After this freeze's batch 033 they read
/// 9 / 1 / 0 / 0 / 0 / 0 / 0 / 0 / 0 / 0 / 5 / 2, a total of **17** over **12** files, and
/// each of the 4 non-zero columns is stated with its own number in its own doc rather
/// than carried forward from here. The −99.4% against the **2 869** the chain above
/// carries for the start of the day is computed here out of 2869 and 17; the −98.9%
/// left from the 31st freeze and the −88.5% standing further down this comment belong to
/// those runs' own boards of 31 and 330 and are left as their records rather than
/// re-pointed, because rewriting a finished run's arithmetic is how a chain stops being
/// evidence.

/// The chain of earlier totals below is no longer prose
/// carried from one freeze to the next: every number is the guarded row count of that screen's
/// own pre-batch snapshot (`D:\tmp\pre-s42-*.csv`), read by this freeze, and the freeze refuses
/// unless each link's difference belongs to exactly one source file — a run may only drain its own
/// screen — with the two exceptions it proves rather than grants. A fold batch drains several files
/// by definition, so the **448** -> **440** link is checked row-for-row against the eight
/// `FOLDED` lines of `D:\tmp\fold006-rows.tsv`: every row leaving must be one of those eight and none
/// may come back. A rewrite that changes a *caller's* line count moves that caller's rows instead of
/// adding them, so the **440** -> **423** link tolerates 8 rows leaving `ui/ribbon/color_dropdown.rs` and `ui/ribbon/draw_panel.rs` only because the
/// same 8 return in the same column at new line numbers. The **423** screen 18 started from was
/// never snapshotted whole — screen 17 was landed without a freeze script — so it is rebuilt
/// here from the live board plus that run's own pre-batch extract, and the freeze refuses unless that
/// rebuild and all 20 links close on today's total. That arithmetic also corrected an off-by-one the
/// previous sentences carried: they paired **485** with `graphic_attributes.rs`, but that run's own
/// snapshot reads **506** and `shortcuts.rs`'s reads **485**. The board read **346** before `ui/style/plotstyle.rs`, **362** before `ui/window/block_palette.rs`, **372** before `ui/style/style_manager.rs`, **389** before `ui/style/dimstyle.rs`, **406** before `ui/window/attdef_dialog.rs`, **423** before `ui/ribbon/widgets.rs`, **440** before `app/view/modal.rs`, **448** before fold batch 006, **466** before `ui/window/count_palette.rs`, **485** before `ui/window/shortcuts.rs`, **506** before `ui/window/graphic_attributes.rs`, **530** before `ui/ribbon/mod.rs`, **556** before `ui/window/wblock.rs`, **584** before `ui/window/drafting_settings.rs`, **616** before `ui/window/plot.rs`, **637** before `ui/properties.rs`, **667** before `app/view/overlay.rs`, **700** before `ui/window/field_dialog.rs`, **730** before `ui/window/pdf_dialogs.rs`, **767** before `ui/window/sheet_set.rs`; **788** is
/// the oldest total still in prose, inherited from before the first snapshotted screen. Against
/// **2 869** at the start of the day (−88.5%). This twenty-fourth freeze adds the **346** -> **330**
/// link and re-verifies it column by column against the ten constants it is about to lower plus the
/// two columns that have no constant — since this run, read out of the board line above rather than
/// out of today's counts file, because the one (h) row this screen drains would otherwise be read
/// as a constant, rebuild the start total as 345 and refuse a closure that really holds:
/// 345 -> 330 is a fall of 15 against a debt of 16, while the guard's line
/// gives 346, and 346 -> 330 is exactly 16. The ten guarded columns of that line
/// are cross-checked against the ten constants before its last two are trusted. The debt count
/// itself needed no rewrite this time and that is a coincidence, not a re-pointing: run block_palette.rs owed
/// 16 rows and this run owed 16, both 16, so the next run must rewrite it. The freeze
/// refuses if any column's fall differs from the 16 rows
/// of this run's own extract, or if the extract's guarded rows differ from the ledger's *board* lines
/// (screen 23's ledger carries **27** lines of which **16** are board rows and **11** are
/// shapes the ratchet cannot count, so a check on the raw line count would have refused a legal ledger). All twenty-three screens are in
/// `REBUILT_SCREENS`, so their rows cannot come back; each landed screen's drop equals its own ledger's *board* row count exactly (21 for screen 4 — its
/// 22nd ledger row is a board-blind `Space` rename and cannot appear in a drop; 30
/// for screen 6, whose board-blind row is again a `Space` rename, at 0 px; screen 13's drop is its
/// full 24, of which 3 rows are the size-9 sites whose +1 pt is priced in the ledger but lives in no
/// pixel total the board can see; screen 14's is its full 21 and the file leaves the board entirely
/// (68 -> 67), because those 21 rows were the whole debt it carried — its two struct insets, its one
/// numeric `Space` side and its five hand-built plates are priced on the ledger and visible to no
/// column; screen 15's is 19 of its 20 board rows, because the twentieth is the `button::success`
/// call #82 C parked, and the ledger names it as deferred instead of dropping it — which is also why
/// the file stays on the board for that one row instead of leaving it as screen 14's did; screen 16's is its
/// full 18 and the file leaves the board (67 -> 66). It is the first screen in the chain whose whole debt was
/// pairs, scalars and point sizes — no hand-built plate, no struct-form inset, no board-blind `Space` side and
/// nothing left open by decision — which is why its 19 px against the plan's 14 px floor is the narrowest F2
/// gap measured so far (1.36x, under screen 13's 1.40x — re-pointed at screen 21 from the per-screen lines of
/// `D:\tmp\regress-s20.log`, which put 1.40x on `ui/ribbon/mod.rs` and 13.00x on `ui/window/shortcuts.rs`): a screen that had already named its roles leaves the
/// planner very little to disagree with, and the disagreement that remains is the three footer verbs the plan
/// wanted to dress as menu lines), which
/// is the check that a run was paid for in rows and not in a rounding of the
/// ceiling. Screen 17's drop is **17** of its ledger's 18 board rows — 7 (f) + 10 (d1)
/// landed and the eighteenth is the `button::warning` call #82 C/F parked, named as
/// deferred rather than dropped — which is why `app/view/modal.rs` stays on the board for
/// that single row and the file count holds at **65** instead of falling to 64. Its 38
/// `Space::new()` sides are the largest board-blind block any screen has yet produced
/// (the runner-up is `app/view/overlay.rs` at 20), and they are the reason this run's
/// ledger (56 lines) is three times its debt (18 rows). Screen 18's drop is **17** of its
/// ledger's 18 board rows — 10 (f) + 3 (c) + 3 (a), plus the one (h) halo that moved onto
/// `tokens::LIFT_HOVER` at 0 px — and the eighteenth is the `tool_btn_style` paint signature #82 C/F
/// parked, named as deferred rather than dropped, which is why `ui/ribbon/widgets.rs` stays on the board
/// for that one row and the file count holds at **65**. Its F2 ratio is **1.00x**, the first exact
/// tie in the series: 3 px of drift against the planner's own 3 px over the same 13 inset rows, and the
/// proof demonstrates the tie row by row (13 of 13) instead of asserting it — a screen that had already
/// named every role it moved leaves the planner nothing to disagree about, so what remains is the ratio's
/// warning that the ratio has stopped separating role from distance. What this run paid for in rows and
/// not in pixels: 1 inset deletion that moved 0 px, 2 board-blind `Space` sides, 3 board-blind struct
/// insets of three different shapes — the screen-local `COMBO_PANEL_PAD` const the run left at L3 instead
/// of promoting, a struct written straight across a call, and a four-sided struct collapsed onto one
/// scalar rung — 4 ink folds, and a
/// residual line for the positional sides and font-measurement scalars. All of it is priced on
/// `D:\tmp\s42-widgets-ledger.tsv`. Screen 19's drop is its **17** board rows in full — 14 (f) + 2 (c) + 1 (d1) —
/// and because those were the whole debt the file carried, `ui/window/attdef_dialog.rs` leaves the
/// board entirely and the count falls **65 -> 64**, the third screen to do so after
/// `ui/window/graphic_attributes.rs` and `ui/window/count_palette.rs`. Nothing was left open by
/// decision and no row was deferred, and it is the first run whose ledger has no line the board
/// cannot see: 0 board-blind `Space` sides, 0 struct insets, 0 ink rows, 0 residual, so ledger
/// lines, owed rows and drained rows are all **17**. Its F2 ratio is **4.82x** — 53 px of
/// role-based drift against the planner's 11 px floor over the same 16 inset rows — the widest gap
/// since screen 15, and one disagreement produces almost all of it: seven field slots whose
/// labels sit `beside` the input rather than above it, which is exactly the condition separating
/// `kit::PAD_FIELD` from the `Size::Control` rung nearest-rung snapping chose. A screen with many
/// side-labelled fields prices that pair of rungs many times over. Two landings bought shapes the
/// board cannot count and pixels it cannot see: the 28 px `pick()` fold deleted a
/// `Length::Fixed(28.0)` box, a hand-centring container and a disabled-state reassignment in one
/// line, and the footer's `text("?").size(13)` now says `Help` like the three sibling dialogs. All of it
/// is priced on `D:\tmp\s42-attdef-ledger.tsv`. Screen 20's drop is its **17** board rows in full — 3 (a) + 2 (a2) + 5 (c) + 6 (f) + 1 (d1) —
/// and `ui/style/dimstyle.rs` leaves the board with them, so the count falls **64 ->
/// 63**. Four of its rows went by *deletion* rather than landing, which is the form this
/// batch exists to produce: the file carried a private copy of the shared editor panel that five
/// named-style managers already compose through `style_manager::editor_shell` — its own tab strip,
/// preview plate, compare row and scrolled body — and deleting the copy took two local style
/// factories, one hand-rolled `container::Style`, the header's `.size(13)` and the body's
/// `.padding([12, 12])` with it. Its F2 ratio is **1.50x**, 15 px of role-based drift against the
/// planner's 10 px floor over the same 11 inset rows: the second-narrowest in the series after the
/// Screen 18's exact tie, and narrow for a structural reason — five of the seven tab-page
/// columns missed `GAP_ROW` by 1 px and the other two already named it, so this screen had almost
/// no off-ladder geometry left to re-decide. The two rows the price could not score are disclosed on
/// the ledger instead of hidden in its total: `[12, 12]` was already on the ladder and lands 0 px as
/// a *transfer* (the builder this screen now borrows paints the same 12/12 as its own row), and the
/// `.size(13)` row prices 1 px as a decision while rendering 13 today because the shared builder
/// decides it for all five managers. Both numbers are reported, which is the price-once rule. The
/// five ink rows move no geometry by definition and one real colour move is priced in prose: sixteen
/// section headings left `palette().primary.base.color` for `kit::accent_style`, a lightness-only pull
/// through `resolve_on` against `kit_planes`, with the hue fixed by the #72 rule. All of it is
/// priced on `D:\tmp\s42-dimstyle-ledger.tsv`. Screen 21's drop is its **10** board rows in full — 4 (a) + 2 (a2) + 2 (f) + 1 (c) + 1 (d1) —
/// and `ui/style/style_manager.rs` leaves the board with them, so the count falls **63 ->
/// 62**, the fifth screen to empty itself after `graphic_attributes.rs`, `count_palette.rs`,
/// `attdef_dialog.rs` and `dimstyle.rs` — re-pointed at screen 22: screen 20's own clause above records
/// that `dimstyle.rs` left the board (64 -> 63), and an enumeration that skips a link cannot be
/// used to rank the next one. This is the run `dimstyle.rs`'s freeze pointed at — `editor_shell` is the
/// shared frame the five named-style managers draw through, so the plate, the band, the
/// tab chips and the toolbar buttons all five compose through are L1-borne now. It does not empty the
/// family: `anno_object_scale.rs` (5 rows), `scale_manager.rs` (9) and `plotstyle.rs` (16) still hand-type
/// their own copy of the same band-and-plate pair, which is why the weak-plate census below is reported
/// rather than declared closed. Its F2 ratio is
/// **5.00x**, 5 px of role-based drift against the planner's 1 px floor over the same 3 inset rows — the
/// third-widest point in the series, behind `shortcuts.rs`'s 13.00x and `properties.rs`'s 6.88x and ahead
/// of `attdef_dialog.rs`'s 4.82x, and wide because both disagreements are about *roles*, not distances: the
/// toolbar's `[5, 8]` is the inset of a band that carries
/// buttons, which L1 names `kit::PAD_BAND` [4,10], while the planner snapped it to `Size::Compact`
/// [4,8] because that pair is 1 px nearer the old numbers; the scrolled property page's `[12, 12]` is
/// a bounded body, `kit::PAD_CARD` [10,12], while the plan spelled it as two `GAP_SECTION`s
/// because that costs 0 px. A screen whose every number is already on a rung gives the planner a free
/// answer that is not the right name. Four (a) and two (a2) rows are ink by definition (0 px), and two
/// of the four are the *reason* this run could be written at all: `kit::chrome_plate_style` and
/// `kit::pane_plate_style` are new L1 primitives named from measurement (eight hand-typed sites
/// each), and both are 0 px because `tokens::control_plate(theme, Control::Weak)` is proven equal to
/// the iced field read they replace on all 26 themes — the remaining hand-typed holdings are measured
/// on this run's ledger as **15 lines over 14 files** for the weak plate and **33 lines over 21 files**,
/// 19 of them in 15 screen files, for the raw vendor accent. What this run deleted is the acceptance:
/// three private style factories left the file — `primary_text_style` (the raw accent hue),
/// `btn_s` (an accent-or-quiet closure that conflated the `Role` axis with a wider box) and
/// `tab_button_style`, a wrapper whose whole body was the L1 call it wrapped. The wrapper is the
/// ratchet finding: (a) fires on `button::Style {` with the qualifier spelled out, and this file had
/// imported the alias and written `-> Style {`, so the board counted four (a) rows here and not five
/// (both spellings re-tested against the live regex on the run). Deleting a whole layer therefore moves no
/// column at all — a blindness, not a saving, recorded on the ledger rather than banked in the drop. Two shapes the
/// board cannot count are priced in prose instead of hidden: the emphasised button's `[4, 14]` vs
/// `[4, 10]` lived in a `let pad = if accent` binding no (f) regex reaches, so the `Role`
/// landing moves 0 px in the columns and repaints 4 px of lateral inset on 7 windows (the question of
/// whether an emphasised toolbar button should be wider than its neighbours at all is reserved to #48
/// and the user, and either answer raises the board); and the two struct insets of `list_panel` and
/// the shell's outer padding are `Padding { top, right, bottom, left }` passed positionally, priced
/// four-axis at 0 px with every side named. One self-correction is on the record rather than quietly
/// reverted: the style-list seam was first landed on `tokens::GAP_HAIRLINE` (2) — an unpriced +1 px —
/// and corrected to `tokens::GAP_SEAM` (1), the exact rung. The two fixed widths (`.width(170)`,
/// `.width(150)`) stay at L3 and are #58's rows. All of it is priced on
/// `D:\tmp\s42-stylemgr-ledger.tsv`. Screen 22's drop is its **16** board rows in full — 5 (a) + 1 (c) + 10 (f) —
/// and `ui/window/block_palette.rs` leaves the board with them, so the count falls **62 ->
/// 61**, the sixth screen to empty itself after `graphic_attributes.rs`, `count_palette.rs`,
/// `attdef_dialog.rs`, `dimstyle.rs` and `style_manager.rs`. Its F2 ratio is **1.00x**, 3 px of role-based
/// drift against the planner's own 3 px over the same 11 inset rows — the second exact tie in the
/// series, after `widgets.rs`'s — and the proof demonstrates it row by row (11 of 11) rather than
/// asserting it from the sums. The two ties are not the same fact: screen 18 tied because every row
/// already sat on the rung the plan would pick, while this one ties on three different terms — two
/// of the 3 moving rows land exactly where the plan pointed (`[5, 10]` ->
/// `kit::Size::Control`, `[5, 1]` -> `[GAP_TIGHT, GAP_SEAM]`) and the third pays the same 1 px for a
/// different shape, which is the row no total can see. `D:\tmp\ladder-snap-bpal.csv`
/// also books 6 of its 11 rows as renames at 0 px, which is 6 px the old resize-only
/// contract would have charged (finding F1). THE TIE HIDES THE SHARPEST ROW: the gallery cell's
/// `.padding(3)` costs 1 px in the plan, which takes it down to `GAP_HAIRLINE` (2) on every
/// side, and 1 px in the landing, which takes it to `kit::PAD_CELL` [3, 4] — the vertical the
/// glyph already had, a named 4 laterally, because PAD_CELL's own doc names this shape ("a gallery
/// thumbnail"). Same price, opposite direction, and only the row's note tells them apart, which is
/// why the F2 tie rule reads notes instead of totals. Five (a) rows are ink by definition and they are
/// three plates and two hairlines: the blank thumbnail composes `kit::plane(theme, Tier::Sink)`
/// through `kit::document_style`, the item context menu's plate is the second use of
/// `kit::pane_plate_style` — the primitive `style_manager.rs` earned on screen 21 — the two
/// rules are `kit::hdivider`, and the tile is `kit::picker_tile_style`, new in L1 and
/// built by copying the screen's two private helpers character for character and then deleting them.
/// THE DELETION IS THE ACCEPTANCE AND THE BUILD PROVES IT: both helpers were `pub(crate)`, so a
/// surviving call site anywhere in the crate fails to compile, and `D:\tmp\s22-check4.log`
/// ends `CARGO_EXIT=0` on the final tree; `src/ui/theme_accessibility_tests.rs`, the one
/// non-screen reader of the pair, was re-pointed at the promoted names so the AA contract keeps all
/// four states on 26 themes without reaching into a screen file. Two colour facts move no column and
/// are priced in prose: the hairlines left `background.neutral.color` for `tokens::edge`,
/// which is bit-identical on every light scheme (`the_control_plate_ladder_is_exact` pins the plate
/// ladder over 26 themes x 7 steps) and one rung deeper on a dark one, and how many of the 26 are dark
/// is NOT statically countable — vendored `palette.rs:35` derives `is_dark` from luminance
/// rather than from a theme name — so the dark half is reported as unmeasured, not as zero; and
/// `kit::document_style` writes `snap: true` where iced's default left the `crisp`
/// feature off (`Cargo.toml:31`), so the thumbnail plate's box now rounds to whole device pixels
/// at a fractional scale factor. A board with no snap column reads that as 0 px everywhere, which is
/// why it is on the ledger. 10 of the ledger's 26 lines are invisible to the board by
/// construction, which is why the ledger is 26 and the debt is 16: 2 struct-form insets
/// (F5, the scrolled list column's `{0.0, 8.0, 6.0, 0.0}` and the gallery grid's
/// `{0.0, 8.0, 0.0, 0.0}`, four named sides each at 0 px — the (f) walk reads only the argument
/// list of `.padding(`, so a struct spread over five lines leaves no trace), 1 hover wash
/// strength (`0.06` -> `tokens::WASH_POINTER`, and no column counts `scale_alpha`), 1
/// conditional tile inset whose two branches are now `tokens::GAP_CONTROL` and
/// `tokens::PAD_FIELD_Y`, 1 deletion record, and 5 residual hand-typed `Length`s that no
/// column measures and #58 owns. One raw palette read stays on the screen on purpose and the
/// want gate pins it at exactly 1: the menu row's hover ink still arrives through
/// `theme.palette().background.base.text` because its sibling is
/// `context_menu_row_style` (`app/view/overlay.rs:1226`, accepted on screen 8), and deciding the
/// app's one menu-row plate from inside a block-palette run would re-decide another screen's role. It
/// is queued with the weak-plate census #48 needs. All of it is priced on
/// `D:\tmp\s42-bpal-ledger.tsv`. Screen 23's drop is its **16** board rows in full — 5 (a) + 3 (c) + 7 (f) + 1 (h) — and `ui/style/plotstyle.rs`
/// leaves the board with them, so the count falls **61 -> 60**. It is the seventh
/// *screen* to empty itself after `graphic_attributes.rs`, `count_palette.rs`,
/// `attdef_dialog.rs`, `dimstyle.rs`, `style_manager.rs` and `block_palette.rs` — the ordinals count
/// per-screen runs only, which is a convention rather than a fact about the board: fold batch 006 also
/// emptied a file (`isolate_popup.rs`, exactly one row) and is deliberately not in this list,
/// because the list ranks screens and a fold is not one. Its F2 ratio is **2.63x**, 21 px of
/// role-based drift against the planner's own 8 px over the same 10 inset rows, and the rank
/// is looked up rather than remembered: 11 of 19 points across 4 replay logs, between
/// `ui/window/field_dialog.rs`'s 2.74x and `ui/window/wblock.rs`'s 2.53x.
/// WHERE THE 13 PX GO. The two totals are joined row by row rather than compared as sums, so the
/// disagreement has addresses: 5 rows paid more (429 (+6), 107 (+2), 127 (+2), 209 (+2), 427 (+2)), 4 matched the plan exactly,
/// 1 paid LESS (376 (-1)). 4 rows the plan scored 0 px because their pairs were
/// already on the ladder (429 [8, 4] -> [8,4] (GAP_ROW+GAP_TIGHT) pays 6 px; 209 [2, 8] -> [2,8] (GAP_HAIRLINE+GAP_ROW) pays 2 px; 225 spacing 1 -> 1 (in-tier) pays 0 px; 441 [12, 12] -> [12,12] (GAP_SECTION+GAP_SECTION) pays 0 px); 2 of them actually moved and cost the landing
/// 8 px, because the plan reads a pair and the role reads an object. Row 429 is the second of two
/// layer-usage recesses in the
/// same panel and its sibling (316) already wears `kit::PAD_WELL`, so keeping the tighter pair would
/// draw two boxes that describe one thing at two depths; that pair alone is 7 px of the
/// 21. One row lands cheaper than the floor (F1 again): `[3, 5]` at row 376 goes to
/// `kit::PAD_FIELD` [3, 6] for 1 px while nearest-rung snapping spends 2 going *down* to [2, 4] — a
/// named inset beat the ladder.
/// PAINTED IS NOT PRICED. Row 209 books 2 px once and is drawn 255 times — the ACI list is
/// `(1u8..=255)` at `plotstyle.rs:142` — so the visible cost is 4 px of height per row (2 top, 2
/// bottom), and `kit::PAD_WELL` is one literal painted at two boxes. The board counts rows; the eye
/// counts draws. Both numbers are on the ledger, and this paragraph names both because a screen whose
/// only geometry row is a list row is exactly where a priced pixel understates the change.
/// THE DELETION AND ITS PAPER TRAIL. This run writes no primitive into L1: every landing target was
/// earned by an earlier run (`kit::pane_plate_style` on 21, `kit::picker_tile_style` on 22 and unused
/// here), so the files it touches are the screen plus two documents that still asserted the deleted
/// forwarder existed. `fn btn_s` was module-private, so the compiler proves no call survives inside
/// the file (`D:\tmp\s23-check.log` ends `CARGO_EXIT=0` on the final tree) and the want gate pins
/// `re:btn_s` at 0. The two documents: `src/ui/style/common.rs`'s header note, which said "the two
/// `btn_s` locals have divergent bodies" when both locals are now gone, and
/// `ui::style::common_tests::btn_s_bodies_documented_as_divergent`, which was `let divergent = true;`
/// `assert!(divergent)` — un-failable by construction, measurement self-deceit kept as documentation,
/// and the one thing in this tree the mission explicitly forbids inheriting quietly. It is now
/// `the_plotstyle_button_forwarder_stays_deleted` and reads the sources instead of a constant: no
/// `btn_s` in `plotstyle.rs`, none in `style_manager.rs` (screen 21), `kit::btn(` present at
/// 6 sites, and `layout_manager.rs` the only file in the tree still defining the forwarder —
/// so what `grep -rn` was used for is the claim "one definition remains", and the test now
/// guarantees it instead of a boolean. Both document edits are comment and cfg(test) text in files the
/// engine either skips or does not read, and the freeze proves that rather than assuming it: today's
/// twelve columns must equal the ones measured before those edits (`D:\tmp\s23-measure.log`).
/// 11 of the ledger's 27 lines are invisible to the board by construction, which is
/// why the ledger is 27 and the debt is 16: 1 `blind_padding` (247), 5 `unmeasured_space` (319, 432, 385, 391, 393), 1 `unmeasured_alpha` (191), 4 `residual_landing` (177, 222, 245, 299). Two of the `Length`s
/// (177 and 222) are the same column typed twice — the row's usage-count cell and the list header's
/// label cell — which must be edited together or the header stops sitting over its numbers, and no
/// column would notice. (h) is the column this paragraph can report and no constant can defend:
/// 7 -> 6, the wash under the selected ACI row, `aurora_gradient(theme, 0.10)`
/// -> `tokens::TINT_SELECTED` (0.12). No cumulative `(h)` figure is quoted anywhere in this file: the
/// count of `(h)` rows the series has drained is not recoverable from any artefact in
/// `D:\tmp`, because the column opened without a constant and every run has read it out of the
/// proof's stdout only, which is not kept per run. Ink the sample-shot gate `#48` has not signed
/// off, all of it priced on the ledger: the selected wash 0.10 -> 0.12 and the hover 0.05 -> 0.06 across
/// up to 255 rows, the selected row's ink moving to `base.text` under `#73`'s ruling, the Save
/// plate losing 4 px of height (`[4, 14]` -> `kit::Size::Control`), and the band, the two recesses and
/// the pick list the rows above name. `[4, 14]` was not a one-off shape at that
/// freeze — `ui/window/open_progress.rs:133` still wore it — and the 32nd freeze
/// supersedes that sentence: no `[4, 14]` survives in the tree (re-grepped this run,
/// 0 hits), because that Cancel button is now `kit::PAD_MENU_ROW` at :119 and lost 2
/// px of lateral air doing it. It is on the same #48 list as the Save plate and is
/// disclosed rather than approved. No direct `theme.palette()` read survives
/// in 524 lines of screen (0 counted by this freeze), which the want gate pins at zero
/// and is the strongest single claim of the run: the file no longer reaches into the palette for a plate. All of it is priced on
/// `D:\tmp\s42-plotstyle-ledger.tsv`. Fold batch 006 cannot use that check, because a fold batch by definition
/// drains several files at once and the one-file rule would reject it; its scope is
/// proved two other ways instead, and both are on its own ledger
/// (`D:\tmp\fold006-rows.tsv`, audited by `D:\tmp\prove-zero-pixel-fold006.cjs`):
/// the family re-counted from the tree after the write as
/// **15** sites = **10** naming the tier (this run's 8 plus the two
/// `count_palette.rs` named at screen 16) + **5** still hand-typed verbs, and a
/// line-count delta of **0** for all five files, so nothing in them changed except
/// the characters after `.padding(`. Note which half each number counts: the
/// family is 15, while the ceiling this run lowers (13 -> 5) is the hand-typed half
/// only — writing `8 + 5 = 13` would be an off-by-two in the other file's debt.
/// That run is also why the board reads **65** files rather than **66**:
/// `isolate_popup.rs` carried exactly one row, and naming it emptied the file.
/// (h) is the one column this paragraph can report but no constant can defend:
/// `D:\tmp\prove-s42-screen.cjs` accounts for its rows against the snapshot and
/// prints `NO RATCHET` where a ceiling would have been checked.
/// (h) on this freeze: 6 -> **2**, and the 4 that left are renames, not decisions.
/// `ui/statusbar/statusbar_menu.rs:42`, `ui/style/scale_manager.rs:84`,
/// `ui/style/style_list.rs:57` and `ui/window/layer_translator.rs:40` each handed a
/// bare `0.10` to `aurora_gradient`; they now hand `tokens::WASH_ROW`, which batch
/// 029 priced at 0.10 so that the number reaching iced is the same number. 0 px.
/// The rung was not folded into `TINT_SELECTED` (0.12): that would have darkened four
/// already-shipped selected-row plates by one step of the wash band, which is a pixel
/// move and needed asking, and the board owner signed "name 0.10, keep 0.14 and 1.0"
/// on 2026-10-19 instead. The band is now arithmetic rather than prose -
/// `WASH_POINTER < WASH_ROW < TINT_SELECTED` asserts in
/// `the_scale_is_monotonic_and_the_shadow_grows_with_lift`, so `WASH_ROW` cannot
/// quietly become a second name for either neighbour.
/// The 2 rows that remain are `ui/style/point_style.rs:101` at 0.14 and
/// `ui/window/open_progress.rs:68` at 1.0, both kept by that same signature: 0.14
/// sits between `WASH_ROW` and `TINT_SELECTED` and belongs to neither, and 1.0 is not
/// a wash at all but a full-strength plate that happens to be painted through the
/// wash function - folding either one moves ink.
/// This column still has no ceiling, so all of the above is a report the next run
/// can contradict by measuring; the gap is the instrument's, and naming it is not the
/// same as closing it.
///
/// Re-frozen 2026-10-20 as the **32nd** freeze (#42 batch 033). Twelve columns, read
/// off the guard's own board rows after the write: **9 / 1 / 0 / 0 / 0 / 0 / 0 / 0 /
/// 0 / 0 / 5 / 2**, 17 rows in 12 files, down from 31 rows in 19 files before the
/// batch. Against the day's opening **2 869** rows that is **−99.4%**. (a) 23 -> 9 is
/// the whole movement and the paragraph above the `BASELINE_STYLE_LITERAL` doc owns
/// its detail; the other eleven columns did not move, and (e) 5 and (a2) 1 are the
/// two that are still owed a signed route (#82 C/F, and the `common.rs:27` helper
/// that no screen is allowed to keep privately).
///
/// Two instrument changes came with this freeze, not two pixel changes. First, the
/// `PAD_MENU_ROW` walk can now read the rung spelling of the inset as well as the
/// digits, so the 5 -> 0 ceiling it just accepted cannot be refilled by a fold that
/// types `[tokens::GAP_TIGHT, tokens::GAP_SECTION]`; the new
/// `the_menu_row_needle_sees_every_spelling_and_none_other` is the falsifiable half of
/// that claim, and it is the round's law: a needle widened without a shape it must
/// reject is a needle nobody checked. Second, seven files left the board at once and
/// six of them entered `REBUILT_SCREENS` (64 -> 70), which is where the pin asserts
/// each of them still holds none of the five numeric shapes. The seventh,
/// `app/view/overlay.rs`, was already pinned as screen 8 and is not entered twice:
/// this batch paid the 2 (a) rows its screen-8 entry declared as a deliberate debt.
///
/// The ink this freeze owes an answer for is two rows, 2 px each. `print_all.rs`'s
/// Options wore the family's odd `[5, 12]` and the tier it now names is `[4, 12]`, so
/// its line is one rung narrower vertically than before the verbs folded;
/// `open_progress.rs`'s Cancel wore `[4, 14]` and names the same tier, so it is one
/// rung narrower laterally. Both folds landed between the 24th freeze's guard log
/// (which still listed five hand-typed sites) and the 31st's (which read zero), and
/// neither was sample-signed; #48 holds them and this paragraph is the disclosure, not
/// the approval. The other four verbs wrote `[4, 12]` and moved nothing. `(h)` is
/// still 2 rows and still unsigned: `ui/style/point_style.rs:101` at 0.14 and `ui/window/open_progress.rs:67` at 1.0
/// (the point-style line moved from :105 when its closure folded into the kit), both kept by the
/// 2026-10-19 signature that names 0.10 and refuses to move the other two.
///
/// Re-frozen 2026-10-20 as the **33rd** freeze (#42 batch 034). Twelve columns, read
/// off the guard's own board rows after the write: **1 / 1 / 0 / 0 / 0 / 0 / 0 / 0 /
/// 0 / 0 / 5 / 2**, 9 rows in 9 files, down from 17 rows in 12 files before the
/// batch. (a) 9 -> 1 is the whole movement; the other eleven columns did not move.
/// Against the day's opening **2 869** rows that is **−99.7%**.
/// Six new L1 fns carry the 8 rows: `heading_plate_style`, `node_plate_style`,
/// `port_dot_style` and `inset_card_style` in `kit::surface`, plus
/// `picker_field_style` and `glyph_cell_style` in `kit::control`. Every body is the
/// removed site's own, so the batch claims 0 px at all 8 rows by construction; the
/// two numbers that do not ride into the kit are the selected fill's 0.14 (stays an
/// argument at the call site, #48) and the hover wash's 0.05, which is kit-internal
/// and priced on the ledger like `menu_row_style`'s 0.42. Three files left the board
/// and entered `REBUILT_SCREENS` (70 -> 73): `ui/node_graph.rs`,
/// `ui/window/about.rs`, `ui/window/gradient_editor.rs`. `ui/style/point_style.rs`
/// is deliberately not entered: its (h) row is still on the board, so the screen is
/// not drained even though its (a) count is 0.
///
/// One instrument artifact, not a code site: the (h) engine reads lines naively, so
/// `glyph_cell_style`'s doc sentence quoting the 0.14 strength added one
/// `off_token_alpha_out_of_scope` row inside `ui/kit/` (informational: 96 -> 97).
/// The board's (h) count is unchanged at 2 because the kit is out of scope for that
/// column. The CSV also carries one blank physical line (the (d) engine's empty
/// output after its column hit zero), so its 108 physical lines are 1 header + 1
/// blank + 9 board + 97 informational; the freeze gate counts non-blank lines.
///
/// Re-frozen 2026-10-21 as the **34th** freeze (#82 decision C). Twelve columns, read
/// off the guard's own board rows after the write: **1 / 1 / 0 / 0 / 0 / 0 / 0 / 0 /
/// 0 / 0 / 1 / 2**, 5 rows in 5 files, down from 9 rows in 9 files before the batch.
/// (e) 5 -> 1 is the whole movement; the other eleven columns did not move. Against
/// the day's opening **2 869** rows that is **−99.8%**. The four rows went to
/// `kit::signal_button_style`, each on the ground its host paints: `app/view/modal.rs`
/// (Warning, Card), `ui/statusbar/mod.rs` (Warning, Band), `ui/window/alias_editor.rs`
/// and `ui/window/shortcuts.rs` (Success, Flush). Unlike the 33rd freeze this batch
/// moves pixels by design; every plate, ink and edge is priced per theme in
/// `D:\tmp\s42-b035-ledger.tsv` (four sites x two DraftFlow themes, max |ΔL*| 3.66-5.81)
/// and on all 26 themes in the guard's own fold print (`D:\tmp\s34f-guard.log`:
/// warning |ΔL*| 0.79 / 5.73 / 7.14, success 0.15 / 5.12 / 6.64, ink 26/26, corner
/// 0/26, frame 26/26). `dialog_button_styled` / `dialog_button_styled_opt` and the
/// local `dialog_button` in `modal.rs` now take `impl Fn` instead of a fn pointer, so
/// a tone+ground decision can travel in the environment; every pre-existing
/// fn-pointer call site compiles unchanged. Two files left the board and entered
/// `REBUILT_SCREENS` (73 -> 75): `ui/statusbar/mod.rs` and `ui/window/alias_editor.rs`.
/// `app/view/modal.rs` (screen 17) and `ui/window/shortcuts.rs` (screen 15) were
/// already pinned and are not entered twice. The remaining (e) row is
/// `ui/window/plot.rs`'s `checkbox::primary`, decision F, still parked behind #63.
/// The CSV is 104 physical lines: 1 header + 1 blank + 5 board + 97 informational
/// (the same blank line the (d) engine leaves after its column hit zero).
///
/// Re-frozen 2026-10-21 as the **35th** freeze (#82 decision F, the last (e) row).
/// Twelve columns, read off the guard's own board rows after the write: **1 / 1 /
/// 0 / 0 / 0 / 0 / 0 / 0 / 0 / 0 / 0 / 2**, 4 rows in 4 files, down from 5 rows in
/// 5 files before the batch. (e) 1 -> 0 is the whole movement and empties the
/// column; the other eleven columns did not move. Against the day's opening
/// **2 869** rows that is **−99.9%**. The row left the board through a new L1 fn,
/// `kit::checkbox_style`, not a screen-local patch: iced's `checkbox::primary`
/// still paints the plate and the frame, and only when the status is `Disabled`
/// does the fn drop the tick (`icon_color`) and the label (`text_color`) to
/// `tokens::INK_DISABLED`; `Active` and `Hovered` pass every field through
/// unchanged, which is why the eight pickable rows in the dialog are 0 px. The
/// plate and frame stay iced's disabled plate on purpose: a kit-drawn neutral
/// plate is #63's `Tone::Neutral` decision and is not this batch's to pre-empt.
/// The five disabled-row call sites (`check_enabled` in `ui/window/plot.rs`) now
/// all route through the same fn, so a later plate change is one L1 edit.
///
/// Pixels, both DraftFlow themes (`D:\tmp\b037-{before,after}-{dark,light}-plot.png`,
/// zone stats in `D:\tmp\s37-zones-{dark,light}.tsv`): the three rows that are
/// actually disabled at runtime (Scale lineweights, Plot with plot styles, Display
/// plot styles) move their label ink from #EFEBE8 (L235) to #84817E (L129) on Dark
/// and from #201C19 (L28) to #868280 (L130) on Light, while their plates read
/// 0 changed pixels (#312B26 Dark / #EAE7E4 Light, the iced disabled plate). The
/// eight pickable labels and their plates are 0 changed pixels in both themes;
/// Center the plot and Fit to paper are disabled only when the custom area is
/// selected, which is why they were pickable (and therefore unchanged) in this run.
/// The tick's own ink cannot be photographed in this dialog: a disabled box has no
/// `on_toggle`, so it can never be observed checked. That gap is covered by the
/// guard's `the_unpickable_tick_loses_both_inks`, 52 readings over 26 themes x two
/// tick states, which asserts the tick and the label alphas equal `INK_DISABLED`,
/// `background`/`border` byte-identical to `checkbox::primary`, and `Active` /
/// `Hovered` identical field by field.
///
/// The (e) surface is unchanged at 40 derived names. The CSV was 103 physical
/// lines: 1 header + 1 blank + 4 board + 97 informational (the same blank line
/// the (d) engine leaves after its column hit zero).
///
/// Re-frozen 2026-10-21 as the **36th** freeze (the (a2) forwarder deletion).
/// Twelve columns, read off the guard's own board rows after the write: **1 / 0 /
/// 0 / 0 / 0 / 0 / 0 / 0 / 0 / 0 / 0 / 2**, 3 rows in 3 files. The (a2) row left
/// through deletion, not a rename: `ui/style/common.rs`'s `muted_style` was a
/// two-line forwarder onto `ui::kit::muted_style`, and the 25 files that imported
/// it plus every full-path call site now name the kit directly. Zero pixels moved,
/// because the forwarded ink was already the kit's solved voice and every call
/// site kept its arguments. The two remaining families are untouched: the two (h)
/// rows (`point_style.rs:101`, `open_progress.rs:67`) still wait on their ink
/// sign-off, and the one (a) row (`ribbon/widgets.rs:414`) is `tool_btn_style`'s
/// return signature, not a literal, pending its own decision. No screen entered
/// `REBUILT_SCREENS`. The CSV is now 102 physical lines: 1 header + 1 blank +
/// 3 board + 97 informational.
const BASELINE_FROZEN: &str = "2026-10-21, 36th freeze";

/// Lower bound on scanned files. A broken path would otherwise make every count
/// read 0 and turn this whole guard green-by-accident.
const MIN_SCANNED_FILES: usize = 600;

/// Where the source actually is. `env!` is used instead of a relative `"src"`
/// because unit-test CWD is not guaranteed to be the crate root on every runner;
/// there is no precedent in this crate for CWD-relative source scanning.
fn src_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Paths (relative to `src/`, forward slashes) that are *allowed* to hold
/// literals: the kit primitives themselves and the token definitions that must
/// spell the numbers out, plus the two test suites that assert on them.
fn is_whitelisted(rel: &str) -> bool {
    rel.starts_with("ui/kit/")
        || rel == "ui/style/tokens.rs"
        || rel == "ui/theme_accessibility_tests.rs"
        || rel == "ui/style/kit_contract_tests.rs"
}

/// The iced UI layer. Corner-radius literals elsewhere (`src/snap.rs` pick
/// radius, `src/modules/**` arc & fillet radii, `src/scene/**` tessellation,
/// `src/entities/**` `base_radius`, `src/app/command_driver/**` B-rep tests,
/// the HTML string in `src/sys.rs`) are CAD geometry, not `Pixels`, and are
/// reported separately rather than gating this refactor. See the
/// `off_token_radius_out_of_ui_scope` rows in `D:\tmp\kit-baseline.csv`.
fn is_ui_layer(rel: &str) -> bool {
    (rel.starts_with("ui/") || rel.starts_with("app/")) && !rel.starts_with("app/command_driver/")
}

#[derive(Default, Debug)]
struct Counts {
    files_scanned: usize,
    style_literal: usize,
    text_style_literal: usize,
    off_token_radius: usize,
    off_token_corner_radius: usize,
    radius_out_of_ui_scope: usize,
    off_token_spacing: usize,
    off_token_padding: usize,
    text_size_off_ladder: usize,
    size_glyph: usize,
    size_box: usize,
    size_unexpected: usize,
    size_unexpected_hosts: Vec<String>,
    builtin_style_call: usize,
    vendor_style_fns: usize,
    vendor_text_size_fns: usize,
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn strip_leading_spaces_tabs(s: &str) -> &str {
    s.trim_start_matches(|c: char| c == ' ' || c == '\t')
}

fn starts_with_ascii_digit(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_digit())
}

/// `true` if `needle` occurs at a word boundary and is followed by `{`
/// (only spaces/tabs allowed in between).
fn style_struct_literal(line: &str, needle: &str) -> bool {
    let mut from = 0usize;
    while let Some(off) = line[from..].find(needle) {
        let at = from + off;
        from = at + needle.len();
        if at > 0 && is_word_char(line[..at].chars().next_back().unwrap_or(' ')) {
            continue; // e.g. `my_button::Style` — not one of ours
        }
        if strip_leading_spaces_tabs(&line[from..]).starts_with('{') {
            return true;
        }
    }
    false
}

/// `true` if `needle` occurs and the first non-space/tab char after it is a
/// digit — i.e. a magic number instead of a `tokens::` constant.
fn numeric_follows(line: &str, needle: &str) -> bool {
    let mut from = 0usize;
    while let Some(off) = line[from..].find(needle) {
        from += off + needle.len();
        if starts_with_ascii_digit(strip_leading_spaces_tabs(&line[from..])) {
            return true;
        }
    }
    false
}

/// `radius:` literal, rejecting suffixed fields such as `blur_radius:` /
/// `base_radius:` via the leading word boundary.
fn radius_literal(line: &str) -> bool {
    const NEEDLE: &str = "radius:";
    let mut from = 0usize;
    while let Some(off) = line[from..].find(NEEDLE) {
        let at = from + off;
        from = at + NEEDLE.len();
        if at > 0 && is_word_char(line[..at].chars().next_back().unwrap_or(' ')) {
            continue; // blur_radius:, base_radius:, top_radius:
        }
        if starts_with_ascii_digit(strip_leading_spaces_tabs(&line[from..])) {
            return true;
        }
    }
    false
}

/// One comma-separated argument element of a `.padding(..)` call is an
/// off-ladder literal: `4`, `10.5`, `-2`, `0 as u16`. Names (`tokens::GAP_ROW`,
/// `PAD_FIELD_Y`) and expressions (`Pixels::from(4)`) are not.
fn is_pad_literal_element(raw: &str) -> bool {
    let p = raw.trim();
    let (num, cast) = match p.find(" as ") {
        Some(i) => (&p[..i], Some(&p[i + 4..])),
        None => (p, None),
    };
    if let Some(c) = cast {
        if c.is_empty()
            || !c.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
        {
            return false;
        }
    }
    let mut b = num.as_bytes();
    if b.first() == Some(&b'-') {
        b = &b[1..];
    }
    let int_part = match b.iter().position(|&c| c == b'.') {
        Some(i) => {
            let frac = &b[i + 1..];
            if frac.is_empty() || !frac.iter().all(u8::is_ascii_digit) {
                return false;
            }
            &b[..i]
        }
        None => b,
    };
    !int_part.is_empty() && int_part.iter().all(u8::is_ascii_digit)
}

/// (f) `true` when the first `.padding(..)` argument list on the line contains at
/// least one numeric literal element. Mirrors `padding_off_ladder()` in
/// `D:\tmp\measure3.sh`. Asking instead "does a digit follow `.padding(`?" — the
/// shape every earlier measurer used — only sees `.padding(12)` and is blind to
/// the dominant `.padding([4, 10])`; see the (f) doc above for that 135-vs-739
/// miss.
fn padding_off_ladder(line: &str) -> bool {
    const NEEDLE: &str = ".padding(";
    let Some(at) = line.find(NEEDLE) else {
        return false;
    };
    let rest = &line[at + NEEDLE.len()..];
    let mut depth = 1usize;
    let mut inner = String::new();
    for ch in rest.chars() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        inner.push(ch);
    }
    inner
        .replace('[', " ")
        .replace(']', " ")
        .split(',')
        .any(is_pad_literal_element)
}

/// Blank comments, string and char literals to spaces (newlines kept) so the
/// host walk cannot be thrown off by a `)` or `(` written inside text. `b".."`,
/// `r".."` and `r#".."#` are handled because the CAD strings use them.
fn blank_literals(src: &str) -> Vec<char> {
    fn blank(c: &mut [char], a: usize, b: usize) {
        for slot in c.iter_mut().take(b).skip(a) {
            if *slot != '\n' {
                *slot = ' ';
            }
        }
    }

    let mut c: Vec<char> = src.chars().collect();
    let n = c.len();
    let mut i = 0usize;
    while i < n {
        let ch = c[i];
        if ch == '/' && i + 1 < n && c[i + 1] == '/' {
            let mut e = i;
            while e < n && c[e] != '\n' {
                e += 1;
            }
            blank(&mut c, i, e);
            i = e;
        } else if ch == '/' && i + 1 < n && c[i + 1] == '*' {
            let mut e = i + 2;
            let mut depth = 1usize;
            while e + 1 < n && depth > 0 {
                if c[e] == '*' && c[e + 1] == '/' {
                    depth -= 1;
                    e += 2;
                } else if c[e] == '/' && c[e + 1] == '*' {
                    depth += 1;
                    e += 2;
                } else {
                    e += 1;
                }
            }
            blank(&mut c, i, e);
            i = e;
        } else if ch == 'r' || ch == 'b' {
            let raw = ch == 'r';
            let mut k = i + 1;
            if raw && k < n && c[k] == 'b' {
                k += 1;
            }
            let mut hashes = 0usize;
            while k < n && c[k] == '#' {
                hashes += 1;
                k += 1;
            }
            if k < n && c[k] == '"' && (raw || hashes == 0) {
                let close_needle = '"';
                let mut e = k + 1;
                if raw {
                    e = k + 1;
                    loop {
                        let stop = match c[e..].iter().position(|&x| x == close_needle) {
                            Some(p) => e + p,
                            None => {
                                e = n;
                                break;
                            }
                        };
                        let mut h = stop + 1;
                        while h < n && h - stop - 1 < hashes && c[h] == '#' {
                            h += 1;
                        }
                        if h - stop - 1 == hashes {
                            e = h;
                            break;
                        }
                        e = stop + 1;
                    }
                } else {
                    while e < n && c[e] != '"' {
                        e += if c[e] == '\\' { 2 } else { 1 };
                    }
                    e += 1;
                }
                blank(&mut c, i, e.min(n));
                i = e.min(n);
            } else if ch == 'b' && i + 1 < n && c[i + 1] == '\'' {
                i += 1;
            } else {
                i += 1;
            }
        } else if ch == '"' {
            let mut e = i + 1;
            while e < n && c[e] != '"' {
                e += if c[e] == '\\' { 2 } else { 1 };
            }
            blank(&mut c, i, (e + 1).min(n));
            i = (e + 1).min(n);
        } else if ch == '\'' {
            // Char literal `'x'` / `'\n'`; a lifetime `'a` is the same shape and
            // blanks harmlessly.
            let mut e = i + 1;
            if e < n && c[e] == '\\' {
                e += 2;
            } else {
                e += 1;
            }
            if e < n && c[e] == '\'' {
                blank(&mut c, i, e + 1);
                i = e + 1;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    c
}

/// The three answers `#85` separates. `.size()` on a box widget is an edge
/// length; on a text or field widget it is a point size; anything else has to
/// be named before it can be counted, which is the whole point of splitting.
#[derive(Debug, PartialEq, Eq)]
enum SizeFamily {
    Glyph,
    BoxEdge,
    Unexpected(&'static str),
}

fn box_hosts(name: &str) -> bool {
    matches!(name, "checkbox" | "radio" | "radio_button" | "toggler")
}

fn glyph_hosts(name: &str) -> bool {
    matches!(
        name,
        "text"
            | "Text"
            | "new"
            | "rich_text"
            | "RichText"
            | "markdown"
            | "Markdown"
            | "span"
            | "Span"
            | "helper"
            | "caption"
            | "heading"
            | "text_input"
            | "TextInput"
            | "pick_list"
            | "PickList"
            | "editor"
            | "Editor"
            | "text_editor"
            | "TextEditor"
            | "combo_box"
            | "text_box"
            | "search"
            | "input"
    )
}

/// Walk left from the `.` of a `.size(` back to the constructor that owns it,
/// eating each `.method(..)` segment on the way. Stops at `;`, at a failed
/// bracket pair, or at the first identifier whose left neighbour is neither `.`
/// (still on the chain) nor `::` (a module path, where the identifier *is* the
/// constructor).
fn size_host(chars: &[char], dot: usize) -> Result<String, &'static str> {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut j = dot as isize - 1;
    let mut guard = 0;
    loop {
        if guard > 6000 {
            return Err("walk-too-far");
        }
        guard += 1;
        while j >= 0 && chars[j as usize].is_whitespace() {
            j -= 1;
        }
        if j < 0 {
            return Err("no-host");
        }
        let cj = chars[j as usize];
        if cj == ';' || cj == '}' {
            return Err("no-host");
        }
        if cj == ')' {
            let mut depth = 0isize;
            let mut k = j;
            while k >= 0 {
                let ck = chars[k as usize];
                if ck == ')' {
                    depth += 1;
                } else if ck == '(' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                k -= 1;
            }
            if k < 0 {
                return Err("unbalanced");
            }
            j = k - 1;
            while j >= 0 && chars[j as usize].is_whitespace() {
                j -= 1;
            }
        }
        if j >= 0 && word(chars[j as usize]) {
            let end = (j + 1) as usize;
            while j >= 0 && word(chars[j as usize]) {
                j -= 1;
            }
            let name: String = chars[(j + 1) as usize..end].iter().collect();
            let mut p = j;
            while p >= 0 && chars[p as usize].is_whitespace() {
                p -= 1;
            }
            if p >= 1 && chars[p as usize] == ':' && chars[p as usize - 1] == ':' {
                return Ok(name);
            }
            if p >= 0 && chars[p as usize] == '.' {
                j = p - 1;
                continue;
            }
            return Ok(name);
        }
        return Err("no-host");
    }
}

fn classify_size_host(name: &str) -> SizeFamily {
    if box_hosts(name) {
        SizeFamily::BoxEdge
    } else if glyph_hosts(name) {
        SizeFamily::Glyph
    } else {
        SizeFamily::Unexpected("unnamed-host")
    }
}

fn scan_src() -> Counts {
    let root = src_root();
    assert!(
        root.is_dir(),
        "kit-contract guard found no source dir at {} — refusing to report green counts",
        root.display()
    );

    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    let style_fns = vendor_style_fns();
    let text_size_fns = vendor_text_size_fns();

    let mut c = Counts::default();
    c.vendor_style_fns = style_fns.len();
    c.vendor_text_size_fns = text_size_fns.len();
    for path in files {
        let rel = match path.strip_prefix(&root) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        if is_whitelisted(&rel) {
            continue;
        }
        let Ok(text) = fs::read_to_string(path) else {
            continue; // non-UTF-8 / symlink noise: never the point of this guard
        };
        c.files_scanned += 1;

        // (d) is counted by walking each `.size(` back to its constructor, not
        // by a same-line pattern — the row-level form could only see 17 of the
        // 116 box rows and would file the rest as font sizes.
        {
            let flat = blank_literals(&text);
            // (g) rides the same blanked text: a second size setter on the same
            // receivers, spelled differently, which (d) therefore cannot see.
            c.text_size_off_ladder += text_size_off_ladder(&flat, &text_size_fns);
            let n = flat.len();
            let mut k = 0usize;
            while k + 5 < n {
                if flat[k] == '.'
                    && flat[k + 1] == 's'
                    && flat[k + 2] == 'i'
                    && flat[k + 3] == 'z'
                    && flat[k + 4] == 'e'
                    && flat[k + 5] == '('
                    && (k == 0 || !(flat[k - 1].is_alphanumeric() || flat[k - 1] == '_'))
                {
                    let mut d = k + 6;
                    while d < n && flat[d].is_whitespace() {
                        d += 1;
                    }
                    if d < n && (flat[d].is_ascii_digit() || flat[d] == '-') {
                        let line_no = flat[..k].iter().filter(|&&ch| ch == '\n').count() + 1;
                        match size_host(&flat, k) {
                            Ok(host) => match classify_size_host(&host) {
                                SizeFamily::Glyph => c.size_glyph += 1,
                                SizeFamily::BoxEdge => c.size_box += 1,
                                SizeFamily::Unexpected(_) => {
                                    c.size_unexpected += 1;
                                    c.size_unexpected_hosts
                                        .push(format!("{}:{} {}", rel, line_no, host));
                                }
                            },
                            // A walk that could not reach a constructor is a row
                            // the guard has not been taught yet, not a reason to
                            // panic the whole scan.
                            Err(kind) => {
                                c.size_unexpected += 1;
                                c.size_unexpected_hosts
                                    .push(format!("{}:{} {}", rel, line_no, kind));
                            }
                        }
                    }
                    k += 6;
                } else {
                    k += 1;
                }
            }
        }

        for raw in text.lines() {
            let line = raw.trim_start();
            if line.starts_with("//") {
                continue;
            }
            if STYLE_WIDGETS.iter().any(|w| style_struct_literal(line, w)) {
                c.style_literal += 1;
            }
            if style_struct_literal(line, "text::Style") {
                c.text_style_literal += 1;
            }
            if radius_literal(line) {
                if is_ui_layer(&rel) {
                    c.off_token_radius += 1;
                } else {
                    c.radius_out_of_ui_scope += 1;
                }
            }
            if CORNER_FIELDS
                .iter()
                .any(|f| numeric_follows(line, f))
            {
                c.off_token_corner_radius += 1;
            }
            if numeric_follows(line, ".spacing(") {
                c.off_token_spacing += 1;
            }
            if padding_off_ladder(line) {
                c.off_token_padding += 1;
            }
            if style_fns.iter().any(|f| names_iced_builtin(line, f)) {
                c.builtin_style_call += 1;
            }
        }
    }
    c
}

const STYLE_WIDGETS: [&str; 3] = ["container::Style", "button::Style", "text_input::Style"];

/// The four corner fields of `iced::border::Radius`. A hand-written
/// `Radius { top_left: 3.0, top_right: 0.0, .. }` is the same offence as
/// `radius: 3.0` — a corner decided at the call site — but the (b) column could
/// not see it because it looks for the word `radius` followed by a digit, and
/// asymmetric corners spell the number out under four *other* names. Measured
/// 2026-10-16: 12 such lines, in `xref_manager.rs` (8, the split-button halves)
/// and `drafting_settings.rs` (4, the tab shape).
const CORNER_FIELDS: [&str; 4] = [
    "top_left:",
    "top_right:",
    "bottom_left:",
    "bottom_right:",
];

/// The surface of iced's own "ready-made look" functions, read out of the vendored
/// fork **at scan time** instead of typed into this file.
///
/// Why it is derived: the first version of this column, written 2026-10-16 21:4x
/// from four widget files, was a list of 13 names and froze the count at 48. The
/// truth was 101. It missed `button::subtle` (30 sites), `container::bordered_box`
/// (15), `container::rounded_box` (4) and `button::{success,warning}` (2 each),
/// because it assumed which files mattered instead of asking the fork. A typed
/// list can only ever be as awake as the person typing it, so the list is now
/// generated from `vendor/iced/widget/src/**.rs`: every `pub fn` whose signature
/// returns `Style`, named `<module stem>::<fn>` because that is the shape a call
/// site writes (`use iced::widget::button` then `button::subtle`).
///
/// Consequences worth stating: if an iced bump adds a style fn, this column gains
/// eyes for it on the next run with no edit here; and a vendor directory that
/// moved or emptied cannot silently zero the column, because `MIN_VENDOR_STYLE_FNS`
/// fails the test loudly. `space::horizontal` is not in the list — it returns
/// `Space`, not `Style`, and the first (buggy) enumeration briefly claimed it,
/// which is why the return type is matched exactly rather than "contains Style".
///
/// Measured surface on this tree: **40** names over `vendor/iced/widget/src/*.rs`
/// plus one tier down (`overlay/menu.rs` supplies `menu::default`, which a
/// top-level-only scan misses). Generic style fns lose their type parameters so
/// the needle matches what the call site writes: `pub fn transparent<Theme>(..)
/// -> Style` is filed as `container::transparent`.
fn vendor_style_fns() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/iced/widget/src");
    let mut out = Vec::new();
    for dir in top_level_and_nested(&root) {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() || path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let stem = match path.file_stem() {
                Some(s) => s.to_string_lossy().replace('\\', "/"),
                None => continue,
            };
            let stem = stem.rsplit('/').next().unwrap_or("").to_string();
            if stem.is_empty() || stem == "mod" {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else { continue };
            let lines: Vec<&str> = text.lines().collect();
            for (i, raw) in lines.iter().enumerate() {
                let Some(rest) = raw.strip_prefix("pub fn ") else { continue };
                let name = rest.split('(').next().unwrap_or("").trim().to_string();
                // `container::transparent<Theme>` is a generic fn; the call site
                // writes `container::transparent`, so the type parameters are cut.
                let name = name.split('<').next().unwrap_or("").trim().to_string();
                if name.is_empty() {
                    continue;
                }
                let mut j = i;
                while j < lines.len() && j < i + 12 {
                    let l = lines[j];
                    if let Some(pos) = l.find("-> ") {
                        let ty = l[pos + 3..].trim_end().trim_end_matches('{').trim();
                        if ty == "Style" {
                            out.push(format!("{stem}::{name}"));
                        }
                        break;
                    }
                    if l.starts_with('}') {
                        break;
                    }
                    j += 1;
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// `root` plus each of its immediate subdirectories: the widget crate keeps most
/// modules at the top level and a few (`overlay/menu.rs`) one tier down.
fn top_level_and_nested(root: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![root.to_path_buf()];
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                dirs.push(entry.path());
            }
        }
    }
    dirs
}

/// Floor on the derived surface. The vendored fork exposes 40 style fns on
/// 2026-10-16; a run that finds far fewer means the vendor tree moved, and the
/// (e) count would then read low for a reason that has nothing to do with screens.
const MIN_VENDOR_STYLE_FNS: usize = 35;

/// The `*text_size*` setters of the vendored fork, read at scan time for the same
/// reason (e) stopped hand-typing its surface: the list of names *is* a
/// measurement, and a typed list is only as awake as whoever typed it.
///
/// Rule, applied to `vendor/iced/widget/src/**.rs` (top level plus one tier, the
/// same walk as [`top_level_and_nested`]): a non-indented `pub fn` whose **name
/// contains `text_size`** and whose **signature takes `impl Into<Pixels>`**. The
/// name clause keeps out `qr_code::cell_size` / `total_size` and
/// `pane_grid::min_size`, which are lengths; the argument clause keeps out any
/// future setter that takes a `Size`/`Length` instead of a raw point size.
///
/// Measured on this tree: **2** names (`text_size` from checkbox / radio /
/// toggler / pick_list / overlay::menu, `with_text_size` from markdown's
/// constructor) over 6 definitions. Names are returned bare, not `module::name`,
/// because a call site writes `.text_size(..)` — the module is on the receiver,
/// not the method.
fn vendor_text_size_fns() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/iced/widget/src");
    let mut out = Vec::new();
    for dir in top_level_and_nested(&root) {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() || path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else { continue };
            for raw in text.lines() {
                // `trim_start`, unlike the (e) derivation: these setters are
                // *builder methods*, so they live inside an `impl` block four
                // spaces in. A non-indented-only rule would find nothing here and
                // the column would read 0 while looking green.
                let Some(rest) = raw.trim_start().strip_prefix("pub fn ") else { continue };
                let name = rest
                    .split('(')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .split('<')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !name.contains("text_size") {
                    continue;
                }
                let params = rest.split_once('(').map(|(_, p)| p).unwrap_or("");
                if params.contains("impl Into<Pixels>") {
                    out.push(name);
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Floor on that surface. The fork exposes 2 such setters on 2026-10-17; a run
/// that finds none means the vendor tree moved or the rule stopped matching, and
/// (g) would then read 0 for a reason that has nothing to do with the screens.
const MIN_VENDOR_TEXT_SIZE_FNS: usize = 2;

/// Count the (g) rows in one blanked file: every `.text_size(` / `.with_text_size(`
/// whose first non-space argument character is a digit. Deliberately a
/// whole-file walk, so a call split over lines is still a row — the mistake (f)
/// was created to catch, one spelling deep.
fn text_size_off_ladder(flat: &[char], needles: &[String]) -> usize {
    let mut hits = 0usize;
    for name in needles {
        let mut needle = Vec::with_capacity(name.len() + 2);
        needle.push('.');
        needle.extend(name.chars());
        needle.push('(');
        let mut i = 0usize;
        while i + needle.len() <= flat.len() {
            if flat[i..i + needle.len()] != needle[..] {
                i += 1;
                continue;
            }
            // A screen's own `foo_text_size(` is not this setter; the leading
            // `.` in the needle already rules out `xtext_size(`, and a word
            // boundary before the dot rules out `a.b.text_size(` being missed —
            // that one is still the same method, so it counts.
            let mut d = i + needle.len();
            while d < flat.len() && flat[d].is_whitespace() {
                d += 1;
            }
            if d < flat.len() && flat[d].is_ascii_digit() {
                hits += 1;
            }
            i += needle.len();
        }
    }
    hits
}

/// `true` if `needle` occurs with a word boundary on both sides, so a screen's
/// own `my_button::text` is not mistaken for iced's and `button::textStyle` is
/// not mistaken for `button::text`. A leading `::` is allowed deliberately:
/// `iced::widget::button::text` borrows the very same look.
fn names_iced_builtin(line: &str, needle: &str) -> bool {
    let mut from = 0usize;
    while let Some(off) = line[from..].find(needle) {
        let at = from + off;
        from = at + needle.len();
        if at > 0 && is_word_char(line[..at].chars().next_back().unwrap_or(' ')) {
            continue;
        }
        match line[from..].chars().next() {
            Some(next) if is_word_char(next) => continue,
            _ => return true,
        }
    }
    false
}

/// Ratchet (a): every `*_::Style { ... }` block written by hand is a widget that
/// has not been moved onto `ui::kit` yet.
#[test]
fn style_literals_shrink_from_baseline() {
    let c = scan_src();
    assert!(
        c.files_scanned >= MIN_SCANNED_FILES,
        "kit-contract guard only scanned {} files (<{}); the counts below are meaningless",
        c.files_scanned,
        MIN_SCANNED_FILES
    );

    assert!(
        c.style_literal <= BASELINE_STYLE_LITERAL,
        "REGRESSION: hand-written style struct literals went UP: {} vs baseline {} (+{}). \
         Build the widget in src/ui/kit/ instead of spelling `container::Style {{ .. }}` / \
         `button::Style {{ .. }}` / `text_input::Style {{ .. }}` at the call site. \
         Per-hit audit rows: D:\\tmp\\kit-baseline.csv",
        c.style_literal,
        BASELINE_STYLE_LITERAL,
        c.style_literal - BASELINE_STYLE_LITERAL,
    );

    if c.style_literal < BASELINE_STYLE_LITERAL {
        // Deliberately not a failure: tearing down old shelves is the goal.
        // Loud, greppable prompt so the win gets booked instead of drifting.
        println!(
            "KIT-CONTRACT: hand-written style literals {} -> {} (-{}). \
             BASELINE CAN BE LOWERED TO {}",
            BASELINE_STYLE_LITERAL,
            c.style_literal,
            BASELINE_STYLE_LITERAL - c.style_literal,
            c.style_literal
        );
    } else {
        println!(
            "KIT-CONTRACT: hand-written style literals flat at {} (frozen {}).",
            c.style_literal,
            BASELINE_FROZEN
        );
    }
}

/// Ratchet (a2): hand-written `text::Style { .. }` must come back from
/// `kit::label` — `muted`, `tone_style`, `color_style`, `faint` and the plane
/// resolvers already answer "what ink reads on this plate", and a screen that
/// spells its own `text::Style` is deciding that twice, badly, for one widget.
#[test]
fn text_style_literals_shrink_from_baseline() {
    let c = scan_src();
    assert!(
        c.files_scanned >= MIN_SCANNED_FILES,
        "kit-contract guard only scanned {} files (<{}); the counts below are meaningless",
        c.files_scanned,
        MIN_SCANNED_FILES
    );

    assert!(
        c.text_style_literal <= BASELINE_TEXT_STYLE_LITERAL,
        "REGRESSION: hand-written `text::Style {{ .. }}` literals went UP: {} vs baseline \
         {} (+{}). Ask `crate::ui::kit` for the voice instead of picking an ink per widget. \
         Per-hit audit rows: D:\\tmp\\kit-baseline.csv",
        c.text_style_literal,
        BASELINE_TEXT_STYLE_LITERAL,
        c.text_style_literal - BASELINE_TEXT_STYLE_LITERAL,
    );

    if c.text_style_literal < BASELINE_TEXT_STYLE_LITERAL {
        println!(
            "KIT-CONTRACT: hand-written text::Style literals {} -> {} (-{}). \
             BASELINE CAN BE LOWERED TO {}",
            BASELINE_TEXT_STYLE_LITERAL,
            c.text_style_literal,
            BASELINE_TEXT_STYLE_LITERAL - c.text_style_literal,
            c.text_style_literal
        );
    } else {
        println!(
            "KIT-CONTRACT: hand-written text::Style literals flat at {} (frozen {}).",
            c.text_style_literal,
            BASELINE_FROZEN
        );
    }
}

/// Ratchet (e): a screen may not borrow iced's own style functions. `.style(button::text)`
/// reads like clean delegation and is the opposite — the plate is composed in vendor
/// code, so it carries numbers `tokens.rs` never decided and ink the contrast guards
/// never measured. Ask `kit::btn` for a [`Role`] instead.
#[test]
fn builtin_style_calls_shrink_from_baseline() {
    let c = scan_src();
    assert!(
        c.files_scanned >= MIN_SCANNED_FILES,
        "kit-contract guard only scanned {} files (<{}); the counts below are meaningless",
        c.files_scanned,
        MIN_SCANNED_FILES
    );
    // The column is only as good as the surface it scans for, and that surface is
    // now read out of the vendor tree, so pin the read: a small or blind surface
    // must fail here rather than quietly report a low count.
    assert!(
        c.vendor_style_fns >= MIN_VENDOR_STYLE_FNS,
        "the vendor style-fn surface derived to only {} names (<{}): `vendor/iced/widget/src/**.rs` \
         moved or stopped parsing, so the (e) count would be fiction. Frozen run measured 40 names \
         over 101 call sites.",
        c.vendor_style_fns,
        MIN_VENDOR_STYLE_FNS,
    );
    assert!(
        vendor_style_fns().contains(&"button::subtle".to_string()),
        "the derived surface does not contain `button::subtle`, which is the exact name the \
         hand-typed 13-name list missed and that made the first freeze read 48 instead of 101 \
         — the derivation is broken, not the count"
    );
    // Second, smaller pin: `overlay/menu.rs` is one tier below the widget crate's
    // top level, so a surface derived without the nested walk silently loses it.
    assert!(
        vendor_style_fns().contains(&"menu::default".to_string()),
        "the derived surface does not contain `menu::default` from `overlay/menu.rs`: the \
         nested tier stopped being scanned, which is how a surface goes blind again"
    );

    assert!(
        c.builtin_style_call <= BASELINE_BUILTIN_STYLE_CALL,
        "REGRESSION: call sites borrowing iced's built-in style fns went UP: {} vs baseline \
         {} (+{}). `button::subtle` / `button::text` / `container::bordered_box` hand the paint \
         to `vendor/iced`, bypassing `ui::kit` and `ui::style::tokens` entirely. Per-hit audit \
         rows: D:\\tmp\\kit-baseline.csv",
        c.builtin_style_call,
        BASELINE_BUILTIN_STYLE_CALL,
        c.builtin_style_call - BASELINE_BUILTIN_STYLE_CALL,
    );

    if c.builtin_style_call < BASELINE_BUILTIN_STYLE_CALL {
        println!(
            "KIT-CONTRACT: iced built-in style-fn call sites {} -> {} (-{}) over a {}-name surface. \
             BASELINE CAN BE LOWERED TO {}",
            BASELINE_BUILTIN_STYLE_CALL,
            c.builtin_style_call,
            BASELINE_BUILTIN_STYLE_CALL - c.builtin_style_call,
            c.vendor_style_fns,
            c.builtin_style_call
        );
    } else {
        println!(
            "KIT-CONTRACT: iced built-in style-fn call sites flat at {} (frozen {}, surface {} names).",
            c.builtin_style_call,
            BASELINE_FROZEN,
            c.vendor_style_fns
        );
    }
}

/// Ratchet (b/b2/c/d): radii, gaps and font sizes must come from `ui::style::tokens`.
/// Any literal count here is surface area that cannot be re-themed as a system.
#[test]
fn radius_spacing_and_type_come_from_tokens() {
    let c = scan_src();
    assert!(
        c.files_scanned >= MIN_SCANNED_FILES,
        "kit-contract guard only scanned {} files (<{}); the counts below are meaningless",
        c.files_scanned,
        MIN_SCANNED_FILES
    );

    let over_radius = c.off_token_radius > BASELINE_OFF_TOKEN_RADIUS;
    let over_corner = c.off_token_corner_radius > BASELINE_OFF_TOKEN_CORNER_RADIUS;
    let over_spacing = c.off_token_spacing > BASELINE_OFF_TOKEN_SPACING;
    let over_glyph = c.size_glyph > BASELINE_SIZE_GLYPH;
    let over_box = c.size_box > BASELINE_SIZE_BOX;

    assert!(
        !over_radius,
        "REGRESSION: hard-coded corner radii in the UI layer went UP: {} vs baseline {} (+{}). \
         Use `tokens::RADIUS_*` / `ui::kit::surface` instead of `radius: N.into()`.",
        c.off_token_radius,
        BASELINE_OFF_TOKEN_RADIUS,
        c.off_token_radius - BASELINE_OFF_TOKEN_RADIUS,
    );
    assert!(
        !over_corner,
        "REGRESSION: hand-written `Radius {{ top_left: N, .. }}` corners went UP: {} vs \
         baseline {} (+{}). An asymmetric corner is still a corner decided at the call site — \
         ask `ui::kit` for the joined shape (`kit::half_style`) instead of naming four numbers.",
        c.off_token_corner_radius,
        BASELINE_OFF_TOKEN_CORNER_RADIUS,
        c.off_token_corner_radius - BASELINE_OFF_TOKEN_CORNER_RADIUS,
    );
    assert!(
        !over_spacing,
        "REGRESSION: hard-coded `.spacing(N)` went UP: {} vs baseline {} (+{}). \
         Use `tokens::GAP_*` instead of a literal gap.",
        c.off_token_spacing,
        BASELINE_OFF_TOKEN_SPACING,
        c.off_token_spacing - BASELINE_OFF_TOKEN_SPACING,
    );
    assert!(
        !over_glyph,
        "REGRESSION: hard-coded `.size(N)` point sizes went UP: {} vs baseline {} (+{}). \
         Use `tokens::TEXT_*` instead of a literal point size.",
        c.size_glyph,
        BASELINE_SIZE_GLYPH,
        c.size_glyph - BASELINE_SIZE_GLYPH,
    );
    assert!(
        !over_box,
        "REGRESSION: hard-coded `.size(N)` box edge lengths went UP: {} vs baseline {} (+{}). \
         A `checkbox`/`radio`/`toggler` size is the MARK's edge, not a font size — \
         use `tokens::MARK_SQUARE`, and never move it onto a `TEXT_*` rung.",
        c.size_box,
        BASELINE_SIZE_BOX,
        c.size_box - BASELINE_SIZE_BOX,
    );
    // A host this guard has not named cannot be counted in either column, so it
    // must fail loudly rather than quietly land in the glyph total the way the
    // old flat column absorbed every box row.
    assert!(
        c.size_unexpected_hosts.is_empty(),
        "`.size(N)` on a host this guard has not classified ({} row(s)): {:?} \
         — name it in `box_hosts`/`glyph_hosts` after deciding which quantity it is.",
        c.size_unexpected,
        c.size_unexpected_hosts,
    );

    for (name, actual, baseline) in [
        ("radius", c.off_token_radius, BASELINE_OFF_TOKEN_RADIUS),
        (
            "corner_radius",
            c.off_token_corner_radius,
            BASELINE_OFF_TOKEN_CORNER_RADIUS,
        ),
        ("spacing", c.off_token_spacing, BASELINE_OFF_TOKEN_SPACING),
        ("size_glyph", c.size_glyph, BASELINE_SIZE_GLYPH),
        ("size_box", c.size_box, BASELINE_SIZE_BOX),
    ] {
        if actual < baseline {
            println!(
                "KIT-CONTRACT: {} {} -> {} (-{}). BASELINE CAN BE LOWERED TO {}",
                name,
                baseline,
                actual,
                baseline - actual,
                actual
            );
        } else {
            println!(
                "KIT-CONTRACT: {} flat at {} (frozen {}).",
                name, actual, BASELINE_FROZEN
            );
        }
    }

    // Informational only — CAD geometry radii, not iced `Pixels`. Not gated,
    // so unrelated kernel work cannot trip the UI refactor ratchet.
    println!(
        "KIT-CONTRACT: {} non-UI `radius:` literals (CAD pick/fillet/mesh radii) excluded from the baseline.",
        c.radius_out_of_ui_scope
    );
}

/// Ratchet (f): inner padding comes from `kit::Size`, not from the call site.
/// The doc on `BASELINE_OFF_TOKEN_PADDING` records why this column only exists
/// now — every earlier measurer asked "does a digit follow `.padding(`?", which
/// is true of `.padding(12)` and false of the `.padding([4, 10])` pairs that make
/// up 606 of the 739 rows.
#[test]
fn padding_comes_from_the_size_ladder() {
    // Pin the *rule* before measuring the tree. If someone "simplifies" the
    // element walk back to the digit-follows form, the column does not fail — it
    // goes green at a fifth of its true height, which is the worst kind of pass.
    assert!(padding_off_ladder("    c.padding(12)"));
    assert!(padding_off_ladder("    c.padding([4, 10])"));
    assert!(padding_off_ladder("    c.padding([COMBO_PAD_V, 6.0])"));
    assert!(padding_off_ladder("    c.padding([MENU_PAD_TOP as u16, 0])"));
    assert!(!padding_off_ladder("    c.padding(tokens::GAP_ROW)"));
    assert!(
        !padding_off_ladder("    c.padding([tokens::GAP_HAIRLINE, tokens::GAP_ROW])"),
        "a fully tokenized pair must not be counted, or the column measures itself"
    );
    assert!(!padding_off_ladder("    c.padding(PAD_CONTROL_Y)"));
    assert!(!padding_off_ladder("    let padding = 4;"));
    assert!(
        !numeric_follows("    c.padding([4, 10])", ".padding("),
        "the old single-shape test is being used again as the padding rule"
    );

    let c = scan_src();
    assert!(
        c.files_scanned >= MIN_SCANNED_FILES,
        "kit-contract guard only scanned {} files (<{}); the padding count below is meaningless",
        c.files_scanned,
        MIN_SCANNED_FILES
    );
    assert!(
        c.off_token_padding <= BASELINE_OFF_TOKEN_PADDING,
        "REGRESSION: screens deciding their own `.padding(N)` went UP: {} vs baseline {} (+{}). \
         Take the padding from `kit::Size::{{Plate, Action, Control, Compact, Icon, Flush}}` — each rung \
         already carries the pair — and if the number you need is on no rung, name it in \
         `ui::style::tokens` instead of spelling it at the call site.",
        c.off_token_padding,
        BASELINE_OFF_TOKEN_PADDING,
        c.off_token_padding - BASELINE_OFF_TOKEN_PADDING,
    );

    // The plate rung is the one pair folded onto a name that is deliberately *not*
    // a `GAP_*` rung, so its two properties are pinned here instead of being left
    // to whoever reads `tokens.rs`: the rung resolves to the numbers the six folded
    // literals carried (a rename, not a resize), and it is symmetric. Symmetry is
    // not decoration — a screen writing `[24, 28]` and a rung writing
    // top/bottom = 24, right/left = 28 are the same box only while the second pair
    // equals the first, and iced's `From<[f32; 2]>` (`vendor/iced/core/src/padding.rs`)
    // is what makes that mapping true in the first place.
    let plate = crate::ui::kit::Size::Plate.padding();
    assert_eq!(plate.top, tokens::PAD_PLATE_V);
    assert_eq!(plate.bottom, tokens::PAD_PLATE_V);
    assert_eq!(plate.right, tokens::PAD_PLATE_X);
    assert_eq!(plate.left, tokens::PAD_PLATE_X);
    assert_eq!(
        (tokens::PAD_PLATE_V, tokens::PAD_PLATE_X),
        (24.0, 28.0),
        "PAD_PLATE_V/X no longer carry the pair the six measured modal rows were \
         folded from (ledger `D:/tmp/fold-plate-rows.tsv`); that is a resize wearing \
         a rename's name, and every dialog plate in the app moved with it"
    );

    // And the folded spelling must not come back. The ratchet above only goes one
    // way — a new hand-typed pair is caught when the *count* exceeds the ceiling,
    // which it will not, because 513 has headroom below it. This pins the specific
    // pair instead, so a screen that re-decides the plate at the call site fails by
    // name rather than by a later measurement someone has to notice.
    let mut plate_files = Vec::new();
    let root = src_root();
    collect_rs_files(&root, &mut plate_files);
    let mut hand_written_plates = Vec::new();
    let mut rung_uses = 0usize;
    for path in plate_files {
        // This file holds the needle it is looking for, so scanning it would make
        // the guard fail on its own source. Same self-measurement trap (f) was
        // created to catch, arriving from the other direction.
        let rel = match path.strip_prefix(&root) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        if rel == "ui/style/kit_contract_tests.rs" {
            continue;
        }
        let Ok(text) = fs::read_to_string(path) else { continue };
        for line in text.lines() {
            let t = line.trim_start();
            // Comment lines are prose about the pair, not uses of it; tokens.rs
            // documents `[24, 28]` in a doc comment and must not fail this guard.
            if t.starts_with("//") {
                continue;
            }
            if t.contains(".padding([24, 28])") || t.contains(".padding([24,28])") {
                hand_written_plates.push(format!("{}:{rel}", t));
            }
            if t.contains("Size::Plate.padding()") {
                rung_uses += 1;
            }
        }
    }
    assert!(
        hand_written_plates.is_empty(),
        "a screen is spelling the plate inset out again instead of asking \
         `kit::Size::Plate`: {hand_written_plates:?}\n(both numbers live at L0 as PAD_PLATE_V/X; \
         a call site that needs a different plate is a #42 finding, not a literal)"
    );
    assert!(
        rung_uses >= 6,
        "`kit::Size::Plate` is used at only {rung_uses} sites; the six modal dialog plates the \
         rung was named from are the floor, so either they were folded back to literals or the \
         rung is being deleted while the L0 constants stay behind it"
    );

    if c.off_token_padding < BASELINE_OFF_TOKEN_PADDING {
        println!(
            "KIT-CONTRACT: padding {} -> {} (-{}). BASELINE CAN BE LOWERED TO {}",
            BASELINE_OFF_TOKEN_PADDING,
            c.off_token_padding,
            BASELINE_OFF_TOKEN_PADDING - c.off_token_padding,
            c.off_token_padding
        );
    } else {
        println!(
            "KIT-CONTRACT: padding flat at {} (frozen {}).",
            c.off_token_padding, BASELINE_FROZEN
        );
    }
}

/// The #42 end-state, pinned per screen.
///
/// WHY A PER-FILE BAN ON TOP OF THE RATCHET
/// (c) and (f) are ceilings over the whole tree: a screen that reaches zero
/// keeps its rows legal the moment another screen grows, so "finished" is not a
/// state the ratchet can hold. Once a screen has been rebuilt against L1 by
/// role, it joins `REBUILT_SCREENS` and may never take a literal again — the
/// boundary fails by name instead of waiting for someone to notice a number.
///
/// WHY `kit::PAD_FIELD` IS PINNED HERE
/// It is the exception the calibration screen found (finding F1, recorded on the
/// constant in `kit/surface.rs`). The snapping planner only knows the `GAP_*`
/// ladder, so it reads a dense field's `[3, 6]` as off-tier and proposes `[4, 6]`
/// — a resize. Both numbers were already decided at L0: `PAD_FIELD_Y` for the
/// vertical, `GAP_CONTROL` for the lateral. Pinning the sides to those two
/// constants is what keeps the rung from drifting into the ladder it was
/// excluded from, which would silently move every dense field in the app.
#[test]
fn a_rebuilt_screen_never_takes_a_literal_again() {
    let field = crate::ui::kit::PAD_FIELD;
    assert_eq!(field.top, tokens::PAD_FIELD_Y);
    assert_eq!(field.bottom, tokens::PAD_FIELD_Y);
    assert_eq!(field.right, tokens::GAP_CONTROL);
    assert_eq!(field.left, tokens::GAP_CONTROL);
    assert_eq!(
        (tokens::PAD_FIELD_Y, tokens::GAP_CONTROL),
        (3.0, 6.0),
        "PAD_FIELD_Y/GAP_CONTROL no longer carry the pair the 69 dense-field rows \
         were renamed from (ledger `D:/tmp/ladder-snap.csv`); that is a resize \
         wearing a rename's name, and `kit::PAD_FIELD` moved the editor rows with it"
    );
    // The rung must stay *off* the symmetric-ladder reading: if a later edit
    // makes it a `GAP_*` rung, the snapping planner stops seeing it as an
    // exception and the 69 rows become resize candidates again.
    assert_ne!(
        tokens::PAD_FIELD_Y, tokens::GAP_CONTROL,
        "`PAD_FIELD` is an asymmetric pair by decision; making both sides equal \
         folds it back into the ladder and re-opens finding F1"
    );

    // Three more rungs are load-bearing for a *rename* claim, so their sides are
    // pinned the same way: `plot.rs` says its empty-state well, its footer band and
    // its scroll body moved 0 px / 1 px / 6 px because these pairs are what they
    // are. If a rung's value is edited, those rows stop being renames and every
    // screen that took the rung silently resizes — which no board column can see,
    // because the call sites still name a rung.
    let well = crate::ui::kit::PAD_WELL;
    assert_eq!((well.top, well.right), (tokens::GAP_CONTROL, tokens::GAP_ROW), "PAD_WELL no longer is the [6, 8] inset the page-setup empty state renamed to");
    let band = crate::ui::kit::PAD_BAND;
    assert_eq!((band.top, band.right), (tokens::GAP_TIGHT, tokens::GAP_FIELD), "PAD_BAND no longer is the [4, 10] inset the plot footer renamed to");
    let card = crate::ui::kit::PAD_CARD;
    assert_eq!((card.top, card.right), (tokens::GAP_FIELD, tokens::GAP_SECTION), "PAD_CARD no longer is the [10, 12] inset five dialog bodies now wrap themselves in");
    // The thirteenth screen landed three insets L1 had never carried (PAD_TAB,
    // PAD_GLYPH, PAD_CELL) and turned the prose on `elevation` into four named
    // steps. A screen that wears a name cannot show a value change on the board —
    // that is finding F6 — so every constant this run's 0 px claims lean on is
    // pinned here: the tab pair, the two rungs its squeeze limit is derived from,
    // the glyph plate the measured plurality wore, the cell pair six containers
    // were folded onto, and the step ladder's four values.
    let tab = crate::ui::kit::PAD_TAB;
    assert_eq!(
        (tab.top, tab.right),
        (tokens::PAD_TAB_V, tokens::PAD_TAB_X),
        "PAD_TAB is no longer the [5, 14] tab plate whose lateral number the strip's squeeze limit eats into"
    );
    {
        let path = src_root().join("ui").join("ribbon").join("mod.rs");
        let strip = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("the tab-squeeze pin needs {}: {e}", path.display()));
        assert!(
            strip.contains(".min_spacing_x(-(tokens::PAD_TAB_X - tokens::GAP_HAIRLINE))"),
            "the tab row's squeeze limit is no longer written as one lateral inset minus a hairline, so 12px of overlap and the 2px of blank left before labels touch became two unrelated numbers"
        );
    }
    let glyph = crate::ui::kit::PAD_GLYPH;
    assert_eq!(
        (glyph.top, glyph.right),
        (tokens::GAP_HAIRLINE, tokens::GAP_CONTROL),
        "PAD_GLYPH is no longer the [2, 6] plate the eight measured glyph buttons already wore, so every tick and arrow in the dense strips changed height under a rename"
    );
    let cell = crate::ui::kit::PAD_CELL;
    assert_eq!(
        (cell.top, cell.right),
        (tokens::PAD_FIELD_Y, tokens::GAP_TIGHT),
        "PAD_CELL is no longer the [3, 4] narrow edge six ribbon containers were folded onto"
    );
    // The eighteenth screen named two numbers the board cannot price. `GAP_SEAM` is
    // load-bearing twice over: thirteen popup rows hand-drew the `1`, and
    // `ui/ribbon/mod.rs` re-derives a list's height from `row_count × 26.0` — a seam
    // renamed to 2 therefore pushes the last row past its plate without moving a
    // single counted literal. `PAD_TRIGGER` is the `[3, 8]` three ribbon dropdowns
    // wore, whose lateral is `GAP_ROW`: shared with the row ladder, so an edit there
    // silently re-spaces every row in the app.
    assert_eq!(
        tokens::GAP_SEAM, 1.0,
        "GAP_SEAM is no longer the 1 px seam thirteen popup rows and one counted list-height formula were folded onto"
    );
    assert_eq!(
        (
            tokens::GAP_NONE,
            tokens::GAP_SEAM,
            tokens::GAP_HAIRLINE,
            tokens::GAP_TIGHT,
            tokens::GAP_CONTROL,
            tokens::GAP_ROW
        ),
        (0.0, 1.0, 2.0, 4.0, 6.0, 8.0),
        "the low rungs of the spacing ladder no longer step 0/1/2/4/6/8, so every fold that took one of these names moved pixels under a rename"
    );
    let trigger = crate::ui::kit::PAD_TRIGGER;
    assert_eq!(
        (trigger.top, trigger.right, trigger.bottom, trigger.left),
        (
            tokens::PAD_FIELD_Y,
            tokens::GAP_ROW,
            tokens::PAD_FIELD_Y,
            tokens::GAP_ROW
        ),
        "PAD_TRIGGER is no longer the [3, 8] trigger inset three ribbon dropdowns were folded onto"
    );
    let dark = iced::Theme::Dark;
    let ghost_status = button::Status::Active;
    assert_eq!(
        crate::ui::kit::ghost_style(&dark, ghost_status).text_color,
        crate::ui::kit::role_style(crate::ui::kit::Role::Ghost)(&dark, ghost_status).text_color,
        "ghost_style is no longer the same ink as role_style(Role::Ghost), so the eleven unframed popup lines this screen folded changed colour under a rename"
    );
    assert_eq!(
        (tokens::LIFT_FLUSH, tokens::LIFT_REST, tokens::LIFT_HOVER, tokens::LIFT_FLOAT),
        (0.0, 1.0, 2.0, 3.0),
        "the elevation step constants no longer match the four steps elevation's own doc names"
    );
    let halo_named = tokens::elevation(&iced::Theme::Dark, tokens::LIFT_HOVER);
    let halo_literal = tokens::elevation(&iced::Theme::Dark, 2.0);
    assert_eq!(
        (halo_named.blur_radius, halo_named.offset, halo_named.color),
        (halo_literal.blur_radius, halo_literal.offset, halo_literal.color),
        "LIFT_HOVER no longer paints what 2.0 painted, so the active tab's halo moved ink under a rename"
    );
    // The eleventh screen's loudest row is priced off the plate tier: DSETTINGS'
    // body inset went `[14, 16]` -> `Size::Plate` for 22 px, the fold
    // `tokens::PAD_PLATE_V`'s own doc pre-authorised. Pin the pair, because eleven
    // dialogs now wear it and an edit here moves all of them at once, invisibly.
    let plate = crate::ui::kit::Size::Plate.padding();
    assert_eq!(
        (plate.top, plate.right),
        (tokens::PAD_PLATE_V, tokens::PAD_PLATE_X),
        "Size::Plate no longer is the [24, 28] inset the dialog family was folded onto"
    );
    // `kit::static_checkbox` was cut in this run so that seven readout rows in one
    // dialog could speak the kit without being handed a callback they would lie
    // about. Its whole justification is that it is the *same plate* as
    // `labeled_checkbox` and differs by nothing but the callback, so the claim is
    // pinned by reading both bodies: if one of the three rungs drifts, the two
    // kinds of checkbox row stop being the same height, and no column on the board
    // can see that because both call sites still name a rung.
    {
        let path = src_root().join("ui").join("kit").join("control.rs");
        let kit = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("the checkbox-plate pin needs {}: {e}", path.display()));
        let body_of = |name: &str| -> String {
            let at = kit
                .find(&format!("pub fn {name}<"))
                .unwrap_or_else(|| panic!("`{name}` vanished from ui/kit/control.rs"));
            let rest = &kit[at..];
            let end = rest
                .find("\npub fn ")
                .or_else(|| rest.find("\n}\n"))
                .expect("unterminated checkbox helper");
            rest[..end].to_string()
        };
        for name in ["labeled_checkbox", "static_checkbox"] {
            let body = body_of(name);
            for rung in ["tokens::MARK_SQUARE", "tokens::TEXT_LABEL", "tokens::GAP_CONTROL"] {
                assert!(
                    body.contains(rung),
                    "`{name}` no longer wears {rung}, so it is a different plate from its sibling and the readouts in `drafting_settings.rs` silently change height"
                );
            }
        }
    }
    // `block_definition.rs` is the second screen through #42 (2026-10-17). It is
    // the first one that *widened* L1 instead of only draining a file: five S5
    // dialogs were hand-writing the same titled fieldset and two were importing
    // a private helper across a window-module boundary, so `kit::group_box` and
    // `kit::labeled_checkbox` are the two primitives that came out of it. The
    // role-based landing cost 65 px against the planner's 37 px snap floor
    // (1.76x; the calibration screen's ratio was 2.1x) — finding F2, priced per
    // row in the ledger `D:/tmp/s42-block-definition-ledger.tsv` and recomputed
    // out of `tokens.rs` + `control.rs` + `surface.rs` by
    // `D:/tmp/prove-s42-screen.cjs`. A further 10 px sits in this file's 12
    // `Space::new().height(n)` rows, which neither the board nor the planner can
    // see at all (blind spot F4), so 75 px is the screen's real geometry bill and
    // 65 px is the part of it that any ratchet counts.
    // `annotation_data.rs` is the third screen through #42 (2026-10-17), and the
    // first one that *consumes* the primitive the second one created rather than
    // widening L1: 17 hand-written titled fieldsets go through `kit::group_box`,
    // which is the test of whether it was cut to the right shape. It paid 36 board
    // rows (11 (c) + 22 (f) + 1 (d1) + 1 (a) + 1 (a2)) for 38 px of geometry, where
    // the snapping planner priced the same 33 inset/spacing rows at 26 px — 1.46x,
    // the third F2 reading after the calibration screen's 2.1x and
    // `block_definition.rs`'s 1.76x, so a plan's pixel total keeps being read as a
    // floor. Two disclosures are the honest cost of this run and are booked in the
    // ledger as `unmeasured_plate` rows (nine: eight buttons plus one list row)
    // rather than asserted here. The buttons now go through `kit::btn(..)` with an
    // explicit role — 7 `Role::Standard`, 1 `Role::Ghost` — and every one of those
    // rows is priced at 0 board px, which is a *colour* decision, not a rename: an
    // unstyled `button()` resolves to iced's primary plate in the vendor
    // (`vendor/iced/widget/src/button.rs:570`), and the pre-batch board proves that
    // spelling for the folder-picker site (its row reads
    // `button(text(..).size(tokens::TEXT_BODY)).padding([5, 10])`, one line, no
    // style argument). Putting those on the background ladder leaves the accent on
    // the dialog's primary actions only. `Role::Ghost` further adds a `WASH_POINTER`
    // hover tint the `kit::text_style` button it replaced never had. The other
    // disclosure: the data link list now paints its own selection (`kit::list_row`)
    // instead of spelling a `●` into the label string. Neither is signed off by the
    // user; both sit on the ledger's note column with their consequence spelled out.
    // `attribute_editor.rs` is the fourth screen through #42 (2026-10-18) and the
    // first one that met columns (g) and (h). Both were empty everywhere else in
    // the tree, so its 21 board rows are 4 (a) + 1 (c) + 9 (f) + 3 (d1) + 3 (g) +
    // 1 (h) and this file is now the *whole* of (g): 3 -> 0. It paid 16 px of
    // inset against the planner's 5 px snap floor for the same 10 rows — 3.20x,
    // the fourth F2 reading (2.1x, 1.76x, 1.46x) — plus 6 px of type step in
    // six (d1)/(g) rows, which no inset total counts and the snap plan cannot
    // price at all. The disclosures this run owns, none of them signed off: the
    // Apply verb folds into `kit::dialog_button` and moves 2 px vertical + 4 px
    // lateral; the top band folds into `kit::wash(Tier::Band)` and moves 1 px
    // vertical while its plate steps weak -> Band (the ladder's own page->Band
    // rung, priced by the surface contracts); the tab strip takes
    // `kit::Size::Control` at +2 px lateral where snapping offered 0 px; the
    // hand-painted list row folds into `kit::list_row`, which re-decides its
    // whole state set (+1 px vertical, +4 px lateral, selected wash 0.10 ->
    // `TINT_SELECTED` 0.12, hover step `TINT_ENGAGED`, resting hover
    // `WASH_POINTER` 0.06, selected ink off `primary.weak.text` onto
    // `background.base.text`). The last of those is the (h) row's real content,
    // and it is booked at 0 px because a wash alpha moves no geometry — the
    // column has no ratchet at all, which is written into the proof script rather
    // than hidden by it. One more row sits where no board can see it: a
    // `Space::new().height(8)` -> `GAP_ROW` rename at 0 px (blind spot F4).
    // `sheet_set.rs` is the fifth screen through #42 (2026-10-18) and the widest
    // F2 gap yet: 37 board rows (35 (f) + 1 (d1) + 1 (a)) for 130 px of inset,
    // 2 px of type step and 1 px on a board-blind `kit::tint` pair, against the
    // planner's 28 px floor for the same 35 rows (4.64x after 2.1 / 1.76 / 1.46 /
    // 3.20). L1 grew for it: `PAD_BADGE` is the first rung whose vertical axis is
    // zero, because a badge inside a tree row borrows the row's line height and
    // `Size::Compact` would have cost the tree 20-40% of its density to re-decide
    // a number the role had already settled. The disclosures this run owns, none
    // of them signed off: the tab strip's selected half leaves `button_style(true)`
    // (Role::Primary, hero glow) for `Role::Engaged`; the five header controls
    // leave `kit::subtle_style`, which has no Role twin, for `Role::Standard`
    // (weak plate -> `Control::Weak` + RADIUS_CONTROL + elevation 1); the folder
    // row's close glyph leaves `kit::text_style` for `Role::QuietDanger`, which
    // rests as Ghost and adds a pointer-armed danger; both selectable lists move
    // their selection from a Primary plate into `kit::list_row`'s wash; and the
    // informational badge's voice moves from `palette.secondary` to `Tone::Accent`
    // (the reading `plugin_manager.rs` already made) with its plate going from a
    // flat colour to `tone_band_plate` and a solved hairline. Nothing else about
    // the dialog's shape is a colour decision: the body wrapper's `PAD_CARD` is a
    // 0 px rename. One more shape nobody can see: `kit::icon_btn` takes its box
    // side as a positional `f32`, so the 22.0/18.0 squares this batch wrote are
    // in the file and off every board (the ledger's residual row).
    // `pdf_dialogs.rs` is the sixth screen through #42 (2026-10-18): 30 board rows,
    // all of them (f), for 100 px of inset against the planner's 21 px floor —
    // 4.76x, the widest F2 gap measured so far. Its disclosures, none signed off:
    // three selection controls (the segmented strip, the layer On/Off switch and
    // the colour-map tab strip) leave `button_style(true)`, i.e. `Role::Primary`
    // with the hero glow, for `Role::Engaged` — the wash reading `chip_button_style`
    // already documents for a sibling in a strip; two glyph buttons are squared on
    // the `Size::Icon` rung, so the page stepper loses ~18 px of width (a 12 px
    // arrow had 12 px of air either side) and the reverse-ramp button goes 28 -> 22;
    // three footer help verbs gain 8 px of height because the footers of this
    // dialog family are all on `Size::Action` and a help verb typed at `[5, 11]`
    // was the only exception; and the 13 field-band rows gain 2-5 px each so that a
    // field, its browse partner and its value plate stop being three different
    // heights in one form. Two folds moved nothing and are worth stating anyway:
    // the view-mode chip and the section pick button were already rung-padded, so
    // they were invisible to the board, and they are in this file only so that
    // `button_style(` can be asserted absent instead of merely rare.
    // `field_dialog.rs` is the seventh screen through #42 (2026-10-18): 33 board
    // rows — 29 (f) + 1 (c) + 3 (a) — for 85 px against the planner's 31 px
    // (2.74x, the lowest F2 reading since the calibration screen, because this
    // dialog had already named most of its rungs). L1 gained
    // `kit::field_container_style` for the (a) rows, which were a borrowed field
    // plate rather than screen-decided ink, and `card()` moved onto
    // `kit::surface` — the shape `sheet_set.rs` had settled, which is also why
    // `pdf_dialogs.rs`'s `card()` was retro-fitted the same day (0 px there).
    // Disclosures, none signed off: 14 field-band rows gain 3 px so a field, its
    // browse partner and its read-only value plate are one height; the two
    // formula entry boxes lose 4 px of vertical air (they were the only taller
    // fields in the dialog); the selectable list's chosen row leaves
    // `button_style(true)` for `list_row`'s wash; its rows' seam goes 1 -> 2 px;
    // and the footer's help verb gains 8 px to match the `dialog_button` Cancel
    // beside it.
    const REBUILT_SCREENS: [&str; 74] = [
        "ui/window/layer_state_manager.rs",
        "ui/window/block_definition.rs",
        "ui/window/annotation_data.rs",
        "ui/window/attribute_editor.rs",
        "ui/window/sheet_set.rs",
        "ui/window/pdf_dialogs.rs",
        "ui/window/field_dialog.rs",
        // Screen 8 (2026-10-18): 32 of its 33 board rows drained, 118 px of geometry
        // against the planner's 25 px floor (4.60x, the ninth F2 reading). It is also
        // the first pinned screen allowed to still owe the board: the context menu's
        // own plate (2 (a) rows) and its hint ink (1 (a2)) are deferred, because
        // MENU_ROW_H fixes those lines at 22.0 px and their insets are asymmetric
        // struct literals {top 3, right 12, bottom 3, left 6} that no `kit::list_row`
        // inset expresses — judging that needs a screenshot, which is #48. Two blind
        // spots this batch made explicit live in the same file: struct-form
        // `.padding(iced::Padding { .. })` is invisible to (f) (F5, now *priced* by
        // the proof and declared on its ledger as two deferred insets, not merely
        // printed — the ninth screen's tool work, see below), and the bar's fixed
        // widths and icon sides are positional arguments no column reads (ledger
        // residual line). `D:\tmp\s42-overlay-ledger.tsv` names every one of them.
        // The debt is closed: the hint ink took `kit::color_style` at batch 032-A and
        // the two plate rows took `kit::menu_row_style` at batch 033 (the 32nd freeze),
        // which is that screen's own `context_menu_row_style` lifted a layer with its
        // body byte-for-byte intact — so 0 px, and no #48 sample was needed to close it.
        "app/view/overlay.rs",
        // Screen 9 (2026-10-18): `ui/properties.rs`, the docked inspector, 21 board
        // rows — 20 (f) + 1 (a) — drained to zero, 55 px of geometry against the
        // planner's 8 px floor (6.88x, the widest F2 reading measured). The gap is
        // not resize greed: four glyph-only buttons went to `Size::Icon` squares
        // (18.0/20.0) because a 10 px arrow inside a 0-vertical-padding strip was a
        // ~14 px-tall click zone in a 26.0 px row, the block-name popup stopped
        // re-deciding `kit::list_row`'s inset two lines after calling it, three
        // chips and two "..." buttons took `Size::Compact` to match their row, and
        // the section header took `PAD_BAND` to match the band above it. 6 of the
        // 20 were 0 px renames. The (a) row was a forwarding shim whose body was
        // already `kit::field_style`: deleted, 11 call sites repointed, 0 px.
        // Disclosures, none signed off: every popup option row grows 4 px and that
        // popup has no height cap (`Length::Shrink` at `color_select.rs:301`), so a
        // long block list runs further past the panel than it did; the two delete
        // glyphs and the collapse arrow change box, not plate — `chip_button_style`
        // and `Role::Ghost` paint were left alone deliberately, because the danger
        // *voice* for "remove this row" is a colour decision for #48. Thirteen more
        // rows this screen owns are invisible to the ratchet: `.padding(Padding {
        // top: 3.0, .. })` decides an inset on four named-or-numbered sides and no
        // column counts it (F5). Eleven were converted to the array form the board
        // can read, two stay asymmetric structs with every side named, and all
        // thirteen are priced four-axis in `D:\tmp\s42-properties-ledger.tsv`.
        "ui/properties.rs",
        // Screen 10 (2026-10-18): `ui/window/plot.rs`, the plot dialog, 32 of its 33
        // board rows drained — 15 (f) + 17 (c) — for 74 px of geometry against the
        // planner's 26 px floor over the same 32 rows (2.85x, the tenth F2 reading,
        // and the second-lowest after field_dialog's 2.74x: this screen had already
        // named most of its roles, so the role-vs-distance gap stayed small even
        // though nine panels each paid 3 px to become GAP_FIELD stacks). The last
        // row, `.style(checkbox::primary)` at 842 (#82 decision F), was drained by
        // the 35th freeze (2026-10-21) through `kit::checkbox_style`: iced still
        // paints the plate and the frame, and the new L1 fn dims only the tick and
        // the label when the box is disabled. It sat as a `deferred_screen_row` in
        // `D:\tmp\s42-plot-ledger.tsv` until then, which is also how the proof's
        // own column table was found to have never carried (e) at all.
        // Disclosures, none signed off: eight secondary plates lose 2 px of lateral
        // inset and Commit gains 2 px of height, so the bottom strip is one row of
        // matching secondaries under one heavier action; the nine panels of the body
        // each grow 3 px per field seam, which makes both columns taller inside a
        // fixed-height scrollable (more scrolling, no clipping); the body's own
        // inset goes 14 -> [10, 12], so the two columns gain 4 px of width and lose
        // 4 px of vertical air; and the three paired-field rows of the custom-sheet
        // editor tighten 4 px each. Ten `Space::new()` sides and one struct inset
        // were named at 0 px and are invisible to every column (F4, F5); the fixed
        // widths (58 / 160 / 92 / 60 / 200.0) are positional arguments and belong to
        // neither the board nor this proof, recorded on the ledger's residual lines.
        "ui/window/plot.rs",
        // Screen 11 (2026-10-18): `ui/window/drafting_settings.rs`, the DSETTINGS
        // dialog, all 28 of its board rows drained — 8 (f) + 12 (c) + 4 (d1) +
        // 2 (a) + 2 (a2) — for 52 px of inset-and-seam geometry against the
        // planner's 23 px floor over the same 20 rows (2.26x, the eleventh F2
        // point and the lowest yet, because one row does almost all the work:
        // the dialog body's [14, 16] lands on `kit::Size::Plate` [24, 28] for
        // 22 px of the 52, which is the fold `tokens::PAD_PLATE_V`'s own doc
        // pre-authorised by naming this pair as drift from the six windows that
        // wrote [24, 28]). Two of the twenty rows are single literals that
        // paint eleven rows each — the deleted private `fn group` priced once at
        // 4 px and drawn eleven times (44 px), the deleted `let toggle` priced
        // once at 1 px and drawn eleven times — so the *painted* geometry is
        // 110 px, not 52, and the ledger says so on both lines. This run also
        // opened an L1 primitive rather than routing around the gap: seven rows
        // in this dialog are readouts, not controls, and `labeled_checkbox`'s
        // `on_toggle` argument IS the affordance, so a callback that ignored its
        // input would light the box up as something clickable that changes
        // nothing. `kit::static_checkbox` is the same plate with no callback, and
        // the two kinds of row are therefore the same height by construction.
        // Disclosures, none signed off: the dialog body loses 20 px of width and
        // 24 px of height to the plate inset, absorbed by the scrollable that
        // already owns the middle row; eleven fieldsets tighten 2 px vertically
        // and 2 px laterally, and their captions go from `strongest.text × 0.75`
        // unsolved to the AA-solved muted voice at 0.68, which is quieter and
        // finally contrast-checked; eleven master-tick captions shrink 12 -> 11 pt
        // to match the dialog's thirteen other checkbox rows, all seven tab titles
        // and the three isometric plates (F6: a rename between two already-named
        // rungs, invisible to the board, the plan and the guard alike); five
        // fields go [4, 7] -> [3, 6], one pixel tighter on each axis; Select All
        // and Clear All grow 10.5 -> 11 pt; the rule under the tab strip keeps its
        // 1 px and changes colour *role*, from `background.neutral` to
        // `tokens::edge`, so it follows the theme's edge like every other rule in
        // the app; and the 24 px reserve between the two object-snap readouts
        // becomes 16, the same gutter this dialog pays between its two whole
        // columns. Ten `Space::new()` sides were named (nine at 0 px, one at 8)
        // and the deleted divider's own `height(1)` left the screen entirely; all
        // eleven are invisible to every column (F4). The three label reserve
        // widths (120 / 100 / 130) are positional arguments and belong to neither
        // this board nor this proof, recorded on the ledger's residual line.
        // `D:\tmp\s42-draftset-ledger.tsv` names every one of them.
        "ui/window/drafting_settings.rs",
        // Screen 12 (2026-10-18): `ui/window/wblock.rs`, the Write Block dialog, all
        // 26 of its board rows drained — 12 (f) + 7 (c) + 1 (a) + 6 (a2) — for 38 px
        // of inset-and-seam geometry against the planner's 15 px floor over the same
        // 19 rows (2.53x, the twelfth F2 reading). Painted geometry is 56 px, not 38:
        // `PAD_WELL` is one literal drawn at four boxes (+3), the coordinate
        // `PAD_FIELD` is one literal drawn at three inputs (+4), and 8 px of it is
        // the F6 class this screen produced a *second* instance of — the deleted
        // helper spaced its caption from its body by `GAP_TIGHT` (4) while
        // `kit::group_box` uses `GAP_CONTROL` (6), so four boxes open 2 px with no
        // literal anywhere in the change. No (d1) row here: every glyph and caption
        // in this dialog already named its rung.
        // Disclosures, none signed off: the browse plate grows ~3 px wide and 1 px
        // tall because `Length::Fixed(28.0)`, its `container(..).width(Fill).
        // align_x(Center)` and its `[3, 0]` inset were three shapes simulating one
        // tier, and `Size::Compact` now decides them; the quick-select glyph plate
        // pays 3 px to stop being the only exception to its own row; the dismiss
        // cross goes from a ~14 px click zone to a 22 px plate; the dialog body gains
        // 4 px of lateral and 4 px of vertical room as `PAD_CARD`; every group in the
        // dialog gains 2 px of caption-to-body air; four field labels and the four
        // box captions go from an unsolved `× 0.85` to the AA-solved muted voice
        // (quieter by 17 alpha points and finally measurable), and the two verdict
        // words go from raw chip fills to `Tone::Warning` / `Tone::Accent`, which is
        // a colour change on a line the user reads as an answer; the status row's
        // placeholder reserve goes 14 -> 12 inside a `Fixed(142.0)` box, so what moves
        // is where the empty sits, not the box. Seven `Space::new()` sides were named
        // (5 at 0 px, 3 px total) and are invisible to every column (F4); the fixed
        // plate heights (142.0 ×2, 16.0, 33.0) and the four glyph sizes are positional
        // arguments that belong to neither this board nor this proof, recorded on the
        // ledger's residual line. `D:\tmp\s42-wblock-ledger.tsv` names every one.
        "ui/window/wblock.rs",
        // Screen 13 is the first one inside L2: the ribbon strip was
        // retyping the kit's insets for itself (14 (f) + 6 (c) + 3 (d1) + 1 (h)),
        // which is what proves the tier rule applies to the framework and not only
        // to the dialogs below it. Its positional numbers — row_h(28.0), a
        // height(24), a 12px swatch and six Length::Fixed panel widths — stay
        // invisible to every column and are named on the ledger's residual line.
        "ui/ribbon/mod.rs",
        // Screen 14 is the first one whose debt is mostly hand-built plates
        // rather than numbers: five `button::Style`/`container::Style` literals, four of
        // which L1 had never carried. The screen keeps no private style function — the
        // plates went into the kit, which is what the four pins below exist to keep
        // honest. Its positional numbers (a 38 px label gutter, the popup's 352/724,
        // four `ROW_H * 0.42` font sizes) are invisible to every column and are named on
        // the ledger's residual lines. `D:\tmp\s42-gattr-ledger.tsv` prices each one.
        "ui/window/graphic_attributes.rs",
        // Screen 15 is the first where the snap plan's floor is a single pixel:
        // eleven of its twelve pair rows already wore two numbers that are each on the
        // ladder, so nearest-rung snapping priced them at 0 and the ratio this run
        // reports is 13 against 1. Read that as a warning about the ratio, not as a
        // record of conservatism — the plan's own destination for the footer verbs is
        // `GAP_TIGHT+GAP_GUTTER`, a scrollbar reserve handed to a button, and paying
        // 4 px per row to refuse that misnaming is the whole point of role-based
        // landing. Its positional numbers (four `Length::Fixed(180.0)`, the guard
        // panel's `Fixed(320.0)` that decided the guard pair's tier, three glyph sizes
        // passed as `12.0`) are invisible to every column and are named on the ledger's
        // residual line. `D:\\tmp\\s42-shortcuts-ledger.tsv` prices each one.
        "ui/window/shortcuts.rs",
        // Screen 16 is the first to leave the board with nothing deferred and no ink row at
        // all: `ui/window/count_palette.rs` carried 18 rows — 13 insets, 3 gaps, 2 point sizes — and
        // every one of them landed, so the file count drops 67 -> 66. Its F2 ratio is the narrowest
        // measured (19 px against the plan's 14 px floor, 1.36x), and the 5 px of disagreement is a
        // single question of role: the plan saw three footer verbs typing [5, 12] and priced them as
        // GAP_TIGHT+GAP_SECTION, i.e. a menu line's lateral gutter handed to a button, while this run
        // sent them to `kit::Size::Control` — the same tier screen 15 gave the discard/keep
        // pair, whose twin `ui/window/alias_editor.rs:288/292` still types [4, 12] by hand and stays
        // on that screen's board. Two of this file's own lines went the other way: the menu entry at
        // 321 and the caption at 340 became `kit::PAD_MENU_ROW`, the tier cut from a counted family
        // (thirteen hand-typed sites remain tree-wide, eight menu lines and five verbs) and now pinned
        // by its own ceiling below, because a `pub const` inset is invisible to every column the
        // moment a screen names it — finding F6. The plan's own destination for 675 is [4, 4] at 2 px;
        // this run took `kit::PAD_CELL` at 1 px, whose doc names [3, 5] as a member of that family.
        // Priced row by row in `D:\tmp\s42-cpal-ledger.tsv`.
        "ui/window/count_palette.rs",
        // Screen 17 is `app/view/modal.rs`, the modal host, and it is the first screen
        // whose ledger is three times its debt: 56 lines for 18 board rows, because 38 of them are
        // `Space::new()` sides no column can see (F4). Seven insets and ten point sizes landed —
        // 84 px plus 11 px — and the 38 blind sides moved 32 px whose *signed* net is 0 px (+14 at
        // the field rows and the footer air, −14 under three dialog titles), so the file is a
        // redistribution and not a growth. One (e) row stays: the lossy-Save-As warning plate still
        // calls iced's own `button::warning` under standing instruction #82 C/F, is named as
        // deferred rather than dropped, and is the only row keeping this file on the board.
        // Its F2 ratio is the widest ever measured — 84 px against the planner's 19 px floor over
        // the same 7 rows, 4.42x, where the thirteen earlier screens spread 1.36x–2.85x — and the
        // size is not greed: 72 of the 84 px are four dialog bodies paying the fold
        // `tokens::PAD_PLATE_V`'s own doc pre-authorises, the second time this column has spent
        // that authorisation rather than earned it. The remaining 12 px are three `text_input`
        // plates that took `kit::PAD_FIELD` [3, 6] over the plan's nearer `Size::Compact` [4, 8],
        // because the field tier names the role and the same run moved their font to `TEXT_BODY`.
        // Ten `.size(13)` lines left for the rungs their role claims (8 -> `TEXT_BODY`, 1 ->
        // `TEXT_HEADING`, 1 -> `TEXT_LABEL` two notches down under a muted hint). Two numbers this
        // run could not price anywhere: the 90 / 70 / 70 label reserves are positional arguments,
        // invisible to the board, the plan and the proof at once, and are booked to #58 rather than
        // written as a residual line this ledger does not carry. Priced row by row in
        // `D:\tmp\s42-modal-ledger.tsv`.
        "app/view/modal.rs",
        // Screen 18 is `ui/ribbon/widgets.rs`, the ribbon's widget factory. It is the first screen
        // whose role-based landing costs exactly what the nearest-rung planner priced — 3 px against 3 px
        // over the same 13 inset rows, F2 = 1.00x, demonstrated row by row rather than claimed. Seventeen
        // of its 18 board rows landed (10 (f), 3 (c), 3 (a), 1 (h)); the eighteenth, `tool_btn_style` at
        // 406, still returns iced's own button paint under standing instruction #82 C/F, is declared a
        // `deferred_screen_row` in `D:\tmp\s42-widgets-ledger.tsv` instead of dropped, and is the only row
        // keeping this file on the board. Three of the (a) rows were deletions, not renames: the file's
        // local `popup_row_style`, `popup_panel_style` and `tip_style` forwarded to ink the kit already
        // paints, so the three helpers are gone and their 39 call sites (eleven, eleven and seventeen)
        // name `kit::ghost_style` and `kit::float_style` directly, together with ribbon/mod.rs's
        // `pub(crate) use widgets::tip_style as tooltip_style` re-export and the one site that reached the
        // alias through it. That repoint is why seven files outside this screen carry edits from this run
        // (`ui/kit/control.rs`, `ui/kit/mod.rs`, `ui/ribbon/mod.rs`, `ui/ribbon/draw_panel.rs`,
        // `ui/ribbon/color_dropdown.rs`, `ui/window/block_palette.rs`, `app/view/controls.rs`) while draining
        // **0** board rows between them: the only trace they leave on the ratchet is the 8 rows of
        // `ui/ribbon/color_dropdown.rs` and `ui/ribbon/draw_panel.rs` that moved line numbers while screen 17's rows were deleted. Two numbers
        // this run named are invisible to every column the moment a screen wears them (finding F6) and are
        // pinned below instead: `tokens::GAP_SEAM` is load-bearing twice over — thirteen popup rows
        // hand-drew the 1 and `ui/ribbon/mod.rs` re-derives a list's height from `row_count × 26.0`, so a
        // seam renamed to 2 pushes the last row past its plate without moving a counted literal — and
        // `kit::PAD_TRIGGER` is the [3, 8] three ribbon dropdowns wore, whose lateral is shared with the
        // row ladder. The 24 ledger lines price the whole screen: 13 insets (one of them an inert
        // deletion), 4 ink folds, 2 board-blind `Space` sides, 3 board-blind struct insets, 1 deferred
        // row and a residual line for the positional widths and heights, the two font-measurement
        // scalars, `COMBO_ROW_SPACING = 3.0` (an F6-class choice between two already-named rungs, left at
        // 3 and booked to #58 rather than snapped) and the three consts that hand geometry to
        // `ui/ribbon/mod.rs` from this file.
        "ui/ribbon/widgets.rs",
        // Screen 19 is `ui/window/attdef_dialog.rs`, the attribute-definition dialog. All
        // 17 of its board rows landed (14 (f), 2 (c), 1 (d1)) and the file leaves the board
        // entirely, which is why the count falls 65 -> 64 instead of holding. Two of the
        // 17 retired shapes the ratchet cannot count and neither was bought with a pixel:
        // `pick()` was a `Length::Fixed(28.0)` box around a centring container whose
        // disabled state was a reassignment of the button value — it is now one
        // `kit::btn(.., kit::Size::Compact, kit::Press::Off, ..)` call, so one board row retired three
        // shapes — and the footer's `text("?").size(13)` became `kit::dialog_button(t!("Help"), ..)`,
        // matching `block_definition.rs:586`. The `[5, 11]` and `[1, 6]` insets are the two rows
        // where the price could not choose a side: `PAD_GLYPH` and `Size::Action` both cost 8 px for
        // the footer, and `wblock.rs` priced `[1, 6]` at 5 px while
        // `block_definition.rs` priced the identical band glyph at 7 px — in each case the sibling
        // that shares the *style expression* decided, not the cheaper number. The seven field slots
        // that landed on `kit::PAD_FIELD` instead of the planner's `Size::Control` are why this
        // screen's F2 ratio is 4.82x rather than screen 18's 1.00x.
        "ui/window/attdef_dialog.rs",
        // Screen 20 is `ui/style/dimstyle.rs`, the dimension-style manager. All
        // 17 of its board rows landed (3 (a), 2 (a2), 5 (c), 6 (f), 1 (d1)) and the file
        // leaves the board entirely, which is why the count falls 64 -> 63 instead
        // of holding. It is the run where the deletion *is* the migration: the screen kept a
        // private copy of the editor panel that `mlstyle.rs`, `tablestyle.rs`, `textstyle.rs` and
        // `mleaderstyle.rs` draw through `style_manager::editor_shell`, so the four chrome
        // blocks went out of the file rather than onto a rung, and their three surviving numbers
        // are now the shared builder's rows (`style_manager.rs`:196 preview plate, :208
        // `.size(13)`, :219 `.padding([12, 12])`) — one run of that file settles all five managers,
        // which is why it is the next high-leverage target rather than another 17-row screen.
        // Nothing here was bought with a hidden pixel: `read_only::field` takes its point size
        // as a positional argument, so ten hand-typed `11.0` literals sat the board at zero
        // rows and are named `tokens::TEXT_LABEL` at 0 px, the two `Space::new().height(0)` branches
        // name `tokens::GAP_NONE`, and the one visible change the fold gives up is the body's
        // height claim (`Fixed(sizing.height)` -> `Fill`), the form the other four managers already
        // use. The `kit::accent_style` primitive this run needed is new in L1 and its census —
        // 7 hand-rolled copies under 2 names, 22 sites already solved — is in its doc.
        "ui/style/dimstyle.rs",
        // Screen 21 is `ui/style/style_manager.rs`, the shared frame underneath
        // every named-style manager. All 10 of its board rows landed (4 (a), 2 (a2), 2 (f), 1 (c),
        // 1 (d1)) and the file leaves the board, so the count falls 63 -> 62. It is
        // the highest-leverage screen in the style family: `dimstyle.rs` was deleted onto this
        // frame on screen 20, so the plate, the band, the chips and the buttons the five managers compose
        // through are L1-borne from here. Three siblings still hand-type their own copy of the same
        // band-and-plate pair (`anno_object_scale.rs`, `scale_manager.rs`, `plotstyle.rs`) and stay
        // on the board for those rows. Three private style factories left the file
        // rather than being renamed — `primary_text_style`, `btn_s` and `tab_button_style` —
        // which is what makes this a migration instead of a repaint. The forwarder is the
        // instructive one: its body was a single call to `kit::chip_button_style`, and (a) fires on
        // `button::Style {` spelled with the qualifier while this file had imported the alias and
        // written `-> Style {` — so the board counted four (a) rows here, not five, and deleting the
        // layer moved no column. A blindness rather than a saving, and it is on the ledger, not in the drop.
        // The two new L1 primitives here are 0 px by proof, not by assertion:
        // `tokens::control_plate(theme, Control::Weak)` equals the iced field read it replaces on
        // all 26 themes, while `Tier::Band` and `Tier::Well` were measured at |dL*| 0.98-6.29
        // against these plates and refused, leaving the plane decision to #48. The `Press::Off` fold
        // is 0 px by construction: an unavailable button dropped `.on_press` before, and `Press::Off` still
        // hands `on_press_maybe(None)` to the same paint path, so iced reports `Status::Disabled` either way.
        "ui/style/style_manager.rs",
        // Screen 22 is `ui/window/block_palette.rs`, the block gallery — the palette that
        // paints a tile per block, which is why its debt is 5 (a) + 1 (c) + 10 (f) rather
        // than a dialog's inset grid. All 16 of its board rows landed and the file leaves
        // the board, so the count falls 62 -> 61. Its F2 ratio is 1.00x: 3 px against the
        // plan's own 3 px over the same 11 inset rows, the second exact tie in the series, and 6
        // of those rows are booked as renames at 0 px — 6 px the old resize-only contract
        // would have charged for them (finding F1).
        // `kit::picker_tile_style` is new in L1. It was built by copying the screen's two
        // private helpers (`block_card_colors`, `block_card_border`) character for character and
        // then deleting them, which is the acceptance the user set: an added layer with the old
        // one still calling is a repaint, not a refactor. The deletion is proven by the build
        // rather than by a grep — both helpers were `pub(crate)`, so a surviving call site
        // anywhere in the crate fails to compile, and `D:\tmp\s22-check4.log` ends CARGO_EXIT=0.
        // `src/ui/theme_accessibility_tests.rs`, the one non-screen reader of the pair, now
        // resolves through the promoted names, so the AA contract keeps four states on 26
        // themes without reaching into a screen file.
        // The tie conceals the row that matters. The gallery cell's `.padding(3)` is 1 px in the
        // plan and 1 px in the landing, but the plan spends it going DOWN to `GAP_HAIRLINE` (2)
        // on all four sides while the landing spends it going OUT to `kit::PAD_CELL` [3, 4],
        // keeping the vertical the glyph already has. Same total, opposite direction, and only
        // the note distinguishes them — which is why the F2 tie test reads notes row by row
        // instead of comparing sums.
        // Two colour facts move no column and are named here rather than hidden in a 0 px:
        // the two hairlines left `background.neutral.color` for `tokens::edge`, identical on
        // every light scheme (the plate-ladder guard pins 26 themes x 7 steps) and one rung
        // deeper on a dark one, where the count of dark themes is NOT statically enumerable
        // because vendored `palette.rs:35` derives `is_dark` from luminance — so it is reported
        // as unmeasured, not as zero; and `kit::document_style` writes `snap: true` where iced's
        // default left the `crisp` feature off, so the thumbnail plate rounds to whole device
        // pixels at a fractional scale factor. No column counts snapping.
        // The screen keeps exactly one raw palette read, and the want gate pins it at 1 so a
        // second cannot arrive quietly: the menu row's hover ink still comes through
        // `background.base.text` because its sibling plate is `context_menu_row_style` in
        // `app/view/overlay.rs` (screen 8, accepted). Deciding the app's one menu-row plate from
        // inside this run would re-decide another screen's role, so it is queued with the
        // weak-plate census #48 needs. 10 ledger lines are board-blind by construction
        // (2 struct insets, 1 wash strength, 1 conditional inset, 1 deletion record, 5 residual
        // `Length`s billed to #58), and they are the reason the ledger holds 26 lines
        // against a debt of 16.
        "ui/window/block_palette.rs",
        // Screen 23 (2026-10-19): `ui/style/plotstyle.rs`, the Plot Style Table editor — all 16 of
        // its board rows drained (5 (a) + 3 (c) + 7 (f) + 1 (h)) and the file leaves the board, 61 -> 60.
        // Its F2 ratio is 2.63x: 21 px of role-based drift against the plan's own 8
        // px over the same 10 inset rows, ranked #11 of 19 out of the replay logs
        // rather than out of memory, between `ui/window/field_dialog.rs` (2.74x) and `ui/window/wblock.rs` (2.53x).
        // 5 rows paid more (429 (+6), 107 (+2), 127 (+2), 209 (+2), 427 (+2)), 4 equalled the plan, 1 paid less (376 (-1)),
        // joined by line number instead of compared as two sums.
        // 4 rows the plan scored 0 px; 2 of them actually moved and cost 8 px,
        // because the plan reads a pair
        // and the role reads an object: [8, 4] at row 429 is on the ladder as GAP_ROW+GAP_TIGHT and
        // pays 6 px anyway, since row 316 — the other layer-usage recess in the same panel, telling
        // the same story — already wears kit::PAD_WELL. Two sibling boxes at two depths is a defect
        // no pixel total can see, and that one pair is 7 px of the 21. Row 376's [3, 5] ->
        // PAD_FIELD [3, 6] is F1 the other way: 1 px where nearest-rung snapping would spend 2.
        // PAINTED IS NOT PRICED. Row 209 books 2 px and is drawn 255 times ((1u8..=255) at
        // plotstyle.rs:142), so the ACI list grows 4 px per row; PAD_WELL is one literal painted at
        // two boxes. The board counts rows, the eye counts draws, and the disclosure comment in the
        // screen keeps both numbers where a reader will find them.
        // NO L1 PRIMITIVE IS ADDED. Everything lands on shapes earlier runs earned, so the run
        // touches the screen plus two documents that still asserted the deleted forwarder existed:
        // common.rs's header note, and a test that was `let divergent = true; assert!(divergent)`
        // — un-failable by construction, the class this mission calls measurement self-deceit. It is
        // now the_plotstyle_button_forwarder_stays_deleted and reads the sources: no btn_s in
        // plotstyle.rs (this run) or style_manager.rs (screen 21), kit::btn( at 6 sites, and
        // layout_manager.rs still holding that run's only remaining copy — the 21-file parallel
        // batch merged later in this mission deleted that copy too, so the test now asserts all
        // three files stay clean and layout_manager reads kit::button_style(. btn_s was module-private, so the
        // compiler proves the in-file deletion (D:\tmp\s23-check.log ends CARGO_EXIT=0) and the want
        // gate pins re:btn_s at 0; the cross-file claim is the test's job now. The comment and
        // cfg(test) edits moved 0 board rows, checked against D:\tmp\s23-measure.log by this freeze.
        // 11 of 27 ledger lines are board-blind by construction:
        //   1. 1 `blind_padding` (247);
        //   2. 5 `unmeasured_space` (319, 432, 385, 391, 393);
        //   3. 1 `unmeasured_alpha` (191);
        //   4. 4 `residual_landing` (177, 222, 245, 299);
        // and (h) fell 7 -> 6 with no constant to defend it; no
        // cumulative (h) number is quoted in this file because none is recoverable from the artefacts. The screen
        // reaches `theme.palette()` directly 0 times across 524 lines and the want gate
        // pins that at zero. Un-signed pixel and ink moves are queued to #48.
        "ui/style/plotstyle.rs",
        // Screens 24-44 (this run, 2026-10-19, the 25th freeze): the 21 files whose board
        // rows reached zero between the 24th freeze (330 rows / 60 files) and this freeze
        // (91 rows / 39 files). They land as one batch, named in alphabetical order, so the
        // screen number for them is array position, not work order. Two drains are booked
        // here and neither is a screen of its own: the 21-file parallel batch that folded
        // 35 hand-typed (a)/(a2) plates onto existing L1 roles (129 -> 94 rows), and 3
        // value-equal renames this run (94 -> 91): ribbon/collapse.rs radius 0.0 ->
        // tokens::RADIUS_FLAT (0 px; note the border width beside it is
        // tokens::EDGE_HAIRLINE = 1.0, so the corner does draw and the old "nothing
        // renders" reason recorded for this file does not apply), app/view/mod.rs
        // padding 20 and ui/style/point_style.rs padding 20 -> tokens::PAD_PAGE (0 px; the
        // name still owes the L0 gap/inset pass, #82).
        // Per-file counts below are read off D:\tmp\kit-baseline-before-dimstyle.csv
        // (the 11:15 ledger, 389 rows / 64 files), not off the 24th freeze, because no
        // per-file snapshot was kept at 20:0x - the aggregate is what this freeze books.
        "app/view/viewcube.rs",
        "ui/color_select.rs",
        "ui/command_line.rs",
        "ui/dock.rs",
        "ui/popup/polar_popup.rs",
        "ui/popup/scale_popup.rs",
        "ui/popup/snap_popup.rs",
        "ui/read_only.rs",
        "ui/ribbon/collapse.rs",
        "ui/ribbon/color_dropdown.rs",
        "ui/ribbon/draw_panel.rs",
        "ui/statusbar/spacemouse.rs",
        "ui/style/anno_object_scale.rs",
        "ui/style/mleaderstyle.rs",
        "ui/window/browser.rs",
        "ui/window/layers.rs",
        "ui/window/missing_fonts.rs",
        "ui/window/options.rs",
        "ui/window/options/spacemouse.rs",
        "ui/window/xref_help.rs",
        "ui/window/xref_manager.rs",
        // Screens 45-46 (this run, 2026-10-19, the 26th freeze): the 2 files whose board
        // rows reached zero between the 25th freeze (91 rows / 39 files) and this freeze
        // (88 rows / 37 files). Both left on one fold: their fieldset well was a
        // hand-typed container::Style copying kit::group_box border-for-border, and it now
        // calls kit::fieldset_well_style, the L1 name introduced for that border in this
        // batch. 0 px; no ink, radius, width or colour changed. layer_translator.rs folded
        // the same third copy but is NOT pinned here - it still owes 1 off_token_alpha row
        // at line 40, a wash-strength decision booked for signature rather than a rename.
        "ui/window/drawing_units.rs",
        "ui/window/geometric_tolerance.rs",
        // Screens 47-52 (this run, 2026-10-19, the 27th freeze): the 6 files whose board
        // rows reached zero between the 26th freeze (88 rows / 37 files) and this freeze
        // (61 rows / 31 files). Five of the six left on 0 px renames of a hand-typed digit
        // to a name that already existed or that this batch priced in L0: 21 `.size(13)`
        // sites -> tokens::TEXT_META and 4 `aurora_gradient(.., 0.10)` sites ->
        // tokens::WASH_ROW. The sixth, print_all.rs, is the exception and is booked as a
        // pixel move: its single row was `.padding([5, 18])`, now `kit::Size::Action`
        // [6, 18], +1 px of vertical inset signed by the board owner rather than
        // pre-authored by L0. They land as one batch, named in alphabetical order, so the
        // screen number for them is array position, not work order.
        "ui/statusbar/statusbar_menu.rs",
        "ui/window/find_replace.rs",
        "ui/window/layer_translator.rs",
        "ui/window/named_parameters.rs",
        "ui/window/options/graphics.rs",
        "ui/window/print_all.rs",
        // Screens 53-54 (this run, 2026-10-19, the 28th freeze): the 2 files whose
        // board rows reached zero in fold batch 031 (61 rows / 31 files -> 59 rows /
        // 29 files). Each owed exactly one (a) row - a hand-spelled selected-row wash
        // closure - and both now call kit::selected_row_wash_style, the L1 name that
        // wash was already copying. 0 px; no ink, radius, width, fill or colour
        // changed. The wash strength itself (0.10 WASH_ROW vs the 0.12 TINT_SELECTED
        // that kit::list_row wears) is deliberately NOT settled here: that is a
        // change of paint and is held for #48's signed sample.
        "ui/style/scale_manager.rs",
        "ui/style/style_list.rs",
        // Screens 55-60 (this run, 2026-10-19, the 29th freeze): the 6 files whose board
        // rows reached zero in fold batch 032-A (59 rows / 29 files -> 41 rows / 23 files).
        // Each owed only (a2) rows - hand-spelled `text::Style { color: Some(...) }`
        // closures, 1 per popup and 2 per style editor - and all of them now call
        // kit::color_style with the same colour expression. 0 px: no ink, alpha or voice
        // was touched, which is why this batch needed no #48 sample. The 3 style editors
        // also lost a private `fn primary_style` factory, deleted rather than kept as a
        // re-export, because the (a2) engine counts the signature line.
        "ui/popup/cycle_popup.rs",
        "ui/popup/selection_filter_popup.rs",
        "ui/popup/units_popup.rs",
        "ui/style/mlstyle.rs",
        "ui/style/tablestyle.rs",
        "ui/style/textstyle.rs",
        // Screens 61-62 (this run, 2026-10-20, the 30th freeze): the 2 files whose board
        // rows reached zero in batch 032-B (41 rows / 23 files -> 34 rows / 21 files).
        // Each owed exactly one (d1) row - `recovery.rs:22` `.size(18)` on a metric's
        // value, `auto_constrain_settings.rs:124` `.size(16)` on the pane title - and
        // both now name a rung in `tokens` (`TEXT_FOCAL`, `TEXT_PANE_HEAD`) that carries
        // the same number to iced. 0 px, and no ink, alpha, radius or inset is touched,
        // so no #48 sample was needed. What these two pins do NOT settle is the role
        // question the rungs were handed to #58 with: 16 and 18 are titles whose job
        // `TEXT_SECTION`'s doc already claims, and the cluster they join is held by a
        // band in the ladder test rather than by a ruling.
        "ui/window/recovery.rs",
        "ui/window/auto_constrain_settings.rs",
        // Screens 63-64 (this run, 2026-10-20, the 31st freeze): the 2 files whose
        // board rows reached zero in batch 032-C (34 rows / 21 files -> 31 rows /
        // 19 files). Each owed exactly one (f) row, an off-ladder inset pair the
        // ladder had no name for: 18/40 on the start page's hero plate and 16/20 on
        // the plugin browser's build notice. Both now wear a pair L1 composes out of
        // the numbers the board row already carried (`kit::PAD_HERO`,
        // `kit::PAD_NOTICE`), so nothing but the argument of the inset call changed
        // and no #48 sample was needed — no ink, radius, size or alpha is touched. The
        // value collisions these 6 new rungs create are named in the (f) doc and
        // belong to #58, not to this pin.
        "app/view/mod.rs",
        // Screens 65-70 (this run, 2026-10-20, the 32nd freeze): the 6 files whose
        // board rows reached zero for the first time in batch 033 (31 rows / 19
        // files -> 17 rows / 12 files). All six owed (a) rows only - hand-built
        // `container::Style` / `button::Style` blocks - and each one's block is now
        // owned by an L1 fn rather than by the screen: `open_progress`/`pc_manager`/
        // `layout_manager`/`xref_attach` went to the five plate fns written for them,
        // `dock/view`, `navigation`, `view/controls` to `accent_plate_style`,
        // `ghost_sheet_style`, `marker_ring_style` and `option_row_style`. A seventh
        // file, `app/view/overlay.rs`, also cleared its board rows this batch but is
        // NOT a new pin: it has been screen 8 since 2026-10-18, and its 2 deferred
        // (a) rows are the debt its own entry below says it was allowed to keep — this
        // batch paid it with `menu_row_style`, the screen's private fn lifted into the
        // kit with its body byte-for-byte intact. 0 px at every one of the fourteen
        // rows, so no #48 sample is claimed here. The two blind spots these pins still
        // do NOT settle: the `Length` arguments (`navigation.rs`'s `.width(10).height(10)`
        // marker, `overlay.rs`'s `Padding { top: 3.0, right: 12.0, .. }` menu indent)
        // are not one of the five shapes and belong to #58, and `ghost_sheet_style`
        // still takes its alpha as an argument because no column measures alpha (F4).
        "app/dock/view.rs",
        "app/navigation.rs",
        "app/view/controls.rs",
        "ui/window/layout_manager.rs",
        "ui/window/pc_manager.rs",
        "ui/window/xref_attach.rs",
        // Screens 71-73 (2026-10-20, batch 034): the last three files whose (a)
        // rows could leave the board. 8 rows went to six new L1 fns, every body
        // the site's own, so 0 px by construction. `ui/style/point_style.rs` is
        // deliberately absent: its (h) row is still on the board, so the screen
        // is not drained even though its (a) count is now 0.
        "ui/node_graph.rs",
        "ui/window/about.rs",
        "ui/window/gradient_editor.rs",
        // Screens 74-75 (2026-10-21, the 34th freeze): the two files whose last
        // board row was an iced signal-button sheet and whose five counted literal
        // shapes (padding, spacing, size, text_size, radius) were already zero, so
        // the (e) migration is what empties the board for them. `app/view/modal.rs`
        // and `ui/window/shortcuts.rs` also leave this batch, but both are already
        // pinned (screens 17 and 15) — re-entering them would shift every number
        // after their index.
        "ui/statusbar/mod.rs",
        "ui/window/alias_editor.rs",
    ];
    let root = src_root();
    let mut problems = Vec::new();
    for rel in REBUILT_SCREENS {
        let path = root.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        // A rebuilt screen that disappears is not a pass: the pin vacates and the
        // debt comes back through a renamed file.
        let Ok(text) = fs::read_to_string(&path) else {
            problems.push(format!("{rel}: rebuilt screen missing from the tree — move it or drop it from REBUILT_SCREENS in the same change"));
            continue;
        };
        for (idx, line) in text.lines().enumerate() {
            let t = line.trim_start();
            if t.starts_with("//") {
                continue;
            }
            let mut why = Vec::new();
            if padding_off_ladder(t) {
                why.push("numeric `.padding(`");
            }
            if numeric_follows(t, ".spacing(") {
                why.push("numeric `.spacing(`");
            }
            if numeric_follows(t, ".size(") {
                why.push("numeric `.size(`");
            }
            if numeric_follows(t, ".text_size(") {
                why.push("numeric `.text_size(`");
            }
            if radius_literal(t) {
                why.push("numeric `radius:`");
            }
            if !why.is_empty() {
                problems.push(format!("{rel}:{} {}", idx + 1, why.join("+")));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "a rebuilt screen is failing its own contract ({} literal call sites): {problems:?}",
        problems.len()
    );
}

/// The four plates cut for `graphic_attributes.rs` are pinned to the
/// claims that let the screen drop its own style closures at 0 px — and, in one
/// case, at a deliberate ink change. A rebuilt screen calls a kit function by name,
/// so nothing on the ratchet can see the plate underneath it change: this is finding
/// F6 again, one level deeper (the rename is between a screen's literal and the kit's
/// constant), and it can only be defended by reading the values.
///
/// The second of the four is no longer a plate at all. The kit fn that named the
/// brand-hairline "this panel is the object of attention" triple has been deleted —
/// after the rollout nothing in `src/` called it, and a floating panel that *is* the
/// object of attention now reads the frame under it as a glass pane. Its numbers are
/// pinned in place from `tokens::glass_*`, because a shader is invisible to every
/// column of the ratchet: this is the only way the material can be read at all. A
/// surface that still wants an opaque plate owes its own row on the ratchet, so it
/// cannot escape both guards at once.
#[test]
fn the_plates_cut_at_the_fourteenth_screen_are_pinned() {
    let dark = iced::Theme::Dark;

    // The ground is the one surface that must carry no edge: it is what the tiers
    // sit on, and framing it would draw a line around the whole window.
    let ground = crate::ui::kit::ground_style(&dark);
    assert_eq!(
        ground.background,
        Some(iced::Background::Color(dark.palette().background.base.color)),
        "ground_style no longer paints the theme's own base, so every window it lands on changed ground"
    );
    assert_eq!(
        ground.border.width, 0.0,
        "ground_style grew an edge; the page ground is not a panel and must not be outlined"
    );

    // What replaced the ten hand-written brand-hairline plates: a panel that *is*
    // the object of attention now reads the frame under it through a glass pane,
    // and a pane is a shader, so the ratchet cannot see its material change. Every
    // number the pane is built from is therefore read here — this is finding F6 one
    // level deeper than the plate it replaces.
    let light = iced::Theme::Light;

    // The fill. Dark is the literal warm-black triple, not a derived grey, and
    // light is white thinned to the *thin* rung; both alphas are the floor that
    // keeps ink readable in a build that cannot blur, so the floor itself is pinned.
    let dark_tint = tokens::glass_tint(&dark);
    assert_eq!(
        dark_tint,
        iced::Color { r: 0.054_9, g: 0.062_7, b: 0.074_5, a: 0.72 },
        "glass_tint's dark fill is no longer the measured value every pane in the app is tinted with"
    );
    assert_eq!(
        tokens::glass_tint(&light),
        iced::Color::WHITE.scale_alpha(0.62),
        "glass_tint's light fill is no longer white at the thin rung"
    );
    for (name, tint) in [("dark", dark_tint), ("light", tokens::glass_tint(&light))] {
        assert!(
            tint.a >= 0.6,
            "glass_tint's {name} alpha fell below the 0.6 legibility floor, so a pane that cannot blur stopped holding its own text"
        );
    }

    // The rim: the light scheme wears the full 0.20 and dark pulls to 0.12 so the
    // edge is never the brightest thing on a near-black page. If the two ever
    // converge, one of the two schemes has been repainted by the other's reading.
    let dark_rim = tokens::glass_edge(&dark);
    let light_rim = tokens::glass_edge(&light);
    assert_eq!(dark_rim, iced::Color::WHITE.scale_alpha(0.12), "the dark rim is no longer 0.12");
    assert_eq!(light_rim, iced::Color::WHITE.scale_alpha(0.20), "the light rim is no longer 0.20");
    assert!(
        light_rim.a > dark_rim.a,
        "the light rim no longer out-reads the dark one; light glass loses its edge against a bright canvas"
    );

    // The specular ramp: light arrives from above, so the top end is always the
    // hotter end, the spread stays a 6-point whisper rather than a decorative
    // gradient, and both ends are white — glass never claims a fifth aurora role.
    for theme in [&dark, &light] {
        let (hi_top, hi_bottom) = tokens::glass_hi(theme);
        assert!(
            hi_top.r == 1.0 && hi_top.g == 1.0 && hi_top.b == 1.0,
            "the glass highlight is no longer scaled white, so it has become a tinted gradient and is claiming a colour role it was explicitly denied"
        );
        assert!(
            hi_top.a > hi_bottom.a,
            "the glass highlight ramp inverted; a pane lit from below reads as a hole in the page"
        );
        assert!(
            hi_top.a - hi_bottom.a <= 0.06 + 1e-6,
            "the glass highlight spread exceeded the 6-point bound, turning a lit edge into a decorative gradient"
        );
    }

    // The corner. Two rungs, popped above flat, and neither is a control corner —
    // a tree-wide substitution that handed glass the field or control radius would
    // round every floating pane to a button.
    assert!(
        tokens::RADIUS_GLASS < tokens::RADIUS_GLASS_RAISED,
        "a popped pane no longer has a larger corner than a flat one, so `raised` stopped meaning anything"
    );
    for radius in [tokens::RADIUS_GLASS, tokens::RADIUS_GLASS_RAISED] {
        assert_ne!(
            radius, tokens::RADIUS_CONTROL,
            "a glass corner was set to the control corner; a pane is not a button and a tree-wide rename would have done this on purpose"
        );
        assert_ne!(radius, tokens::RADIUS_FIELD, "a glass corner was set to the field corner");
    }

    // The trigger: the edge answers the open state, the ground never does, and the
    // ink is set explicitly — that last one is the AA repair this run made on
    // purpose, because the hand-written preview plate let iced's default black
    // stand where the page's own ink should have been.
    let rest = crate::ui::kit::picker_trigger_style(false)(&dark, button::Status::Active);
    let hovered = crate::ui::kit::picker_trigger_style(false)(&dark, button::Status::Hovered);
    let open = crate::ui::kit::picker_trigger_style(true)(&dark, button::Status::Active);
    assert_eq!(rest.border.color, tokens::edge(&dark), "a closed trigger no longer rests on the page edge");
    assert_eq!(
        hovered.border.color,
        dark.palette().primary.base.color,
        "the pointer no longer arms the brand hairline, so the trigger stopped answering the mouse"
    );
    assert_eq!(
        open.border.color,
        dark.palette().primary.base.color,
        "an open trigger no longer looks engaged"
    );
    assert_eq!(
        hovered.background, rest.background,
        "the trigger changed ground on the pointer; only its edge may move, or a panel brightens as you sweep it"
    );
    assert_eq!(
        rest.text_color,
        dark.palette().background.base.text,
        "picker_trigger_style no longer sets the page ink — this is the assertion that keeps the dark-scheme black-on-black defect from coming back"
    );
    assert_eq!(rest.border.radius, tokens::RADIUS_MARK.into(), "the trigger's corner is no longer the mark corner");

    // A colour cell's ink stays one of two extremes on purpose. The kit has a
    // WCAG-solving helper (tokens::legible_on) and it is deliberately NOT used
    // here: it returns a derived mid-tone where this returns black or white, and
    // swapping them changes shipped ink on every colour label. The reason lives on
    // swatch_edit_ink's own doc; this pin keeps the behaviour, not the argument.
    for fill in [
        iced::Color::BLACK,
        iced::Color::WHITE,
        iced::Color::from_rgb(0.5, 0.5, 0.5),
        iced::Color::from_rgb(0.2, 0.7, 0.3),
    ] {
        let ink = crate::ui::kit::swatch_edit_ink(fill);
        assert!(
            ink == iced::Color::BLACK || ink == iced::Color::WHITE,
            "swatch_edit_ink returned a mid-tone for {fill:?}, which is legible_on's job and would repaint every colour cell"
        );
    }
    assert_eq!(
        crate::ui::kit::swatch_edit_ink(iced::Color::from_rgb(0.9, 0.9, 0.9)),
        iced::Color::BLACK,
        "a light cell no longer gets black ink"
    );
    assert_eq!(
        crate::ui::kit::swatch_edit_ink(iced::Color::from_rgb(0.1, 0.1, 0.1)),
        iced::Color::WHITE,
        "a dark cell no longer gets white ink"
    );
}

/// The guard plate cut for `shortcuts.rs`, and the equality that let its lid
/// fold into a primitive screen 14 already had. A rebuilt screen calls a
/// kit function by name, so nothing on the ratchet can see the plate underneath it
/// change (finding F6), and the lid's fold rested on a number the screen typed and
/// the kit names separately.
#[test]
fn the_guard_plate_cut_at_the_fifteenth_screen_is_pinned() {
    let dark = iced::Theme::Dark;

    // The unsaved-changes guard: the page's own ground, the page's own edge, the
    // corner of the things it holds. Two screens typed this triple; a rename that
    // silently becomes a resize is invisible to (a), so the values are read here.
    let guard = crate::ui::kit::guard_panel_style(&dark);
    assert_eq!(
        guard.background,
        Some(iced::Background::Color(dark.palette().background.base.color)),
        "guard_panel_style no longer sits on the page ground"
    );
    assert_eq!(
        guard.border.color,
        tokens::edge(&dark),
        "the guard's outline is no longer the page edge — it was never meant to be the brand"
    );
    assert_eq!(
        guard.border.width,
        tokens::EDGE_HAIRLINE,
        "the guard's outline is no longer one hairline"
    );
    assert_eq!(
        guard.border.radius,
        tokens::RADIUS_CONTROL.into(),
        "the guard stopped wearing the corner of its own contents"
    );
    // The two surfaces a reader could confuse, kept apart by what they are made of.
    // Under the one-pen grammar every plate corner is the control corner, so the
    // distinction is not carried by the radius: the guard says "here is a question"
    // in an opaque page plate with the page's own edge, and the thing that *is* the
    // object of attention says it in glass — a translucent pane that reads the frame
    // under it. A warning must not dim the document behind it, so the guard's
    // opacity is the load-bearing half of that difference, and the ink is the other.
    let guard_background = guard
        .background
        .expect("guard_panel_style stopped painting a ground at all");
    let iced::Background::Color(guard_ground) = guard_background else {
        panic!(
            "guard_panel_style's ground is now a gradient; an alert is a flat plate, and a gradient would make the guard the loudest thing in the dialog"
        );
    };
    assert_eq!(
        guard_ground.a,
        1.0,
        "the unsaved-changes guard became translucent; an alert now dims the document behind it and reads as a state of the drawing"
    );
    assert_ne!(
        guard.border.color,
        tokens::glass_edge(&dark),
        "guard_panel_style and the glass rim now share an outline ink, so the plate family has collapsed into one"
    );
    assert!(
        tokens::glass_tint(&dark).a < guard_ground.a,
        "glass is no longer thinner than the page plate it floats over, so a pane stops reading the frame beneath it"
    );

    // The lid. The screen typed `scale_alpha(0.55)`; the kit names
    // `SCRIM_INK * fade(track)`, and `fade("")` resolves to 1.0. Both halves of
    // that equality are premises of a 0-ink rename, so both are pinned, not
    // assumed: the constant's value and the resolved colour.
    assert!(
        (tokens::SCRIM_INK - 0.55).abs() < 1e-6,
        "SCRIM_INK is no longer the 0.55 screen 15 folded a typed alpha into, so that screen's lid changed opacity on a rename"
    );
    let scrim = crate::ui::kit::scrim_style(&dark, "");
    assert_eq!(
        scrim.background,
        Some(iced::Background::Color(
            tokens::control_plate(&dark, tokens::Control::Strongest).scale_alpha(tokens::SCRIM_INK)
        )),
        "scrim_style no longer composes the strongest plate with SCRIM_INK, so the equality the rename claimed is gone"
    );
}

/// The inset the sixteenth screen cut for a detached menu's own line, and the size
/// of the family it was named from. A rebuilt screen calls a tier by name, so no
/// ratchet column can see the numbers underneath it change (finding F6) — which is
/// why the tier's four sides are read from L0 here, and why the tree-wide count of
/// the pair it replaces gets a ceiling of its own instead of living in a doc
/// comment nobody re-measures.
///
/// Lowered 13 -> **5** by fold batch 006, which named the eight menu lines rather
/// than renaming them, then 5 -> **0** by the folds that landed between the 24th
/// freeze (`s23-guard-test.log`, 2026-10-19 20:01, still listing five) and the 31st
/// (`s32a-guard.log`, 23:31, reading zero). This ceiling counts the **hand-typed
/// half** of the family, not the family: 16 sites in 11 files wear this inset and
/// all 16 name `kit::PAD_MENU_ROW`, so the hand-typed half is empty and the zero is
/// what keeps it empty.
///
/// Four of the last five wrote `[4, 12]` and moved 0 px. The fifth — `print_all`'s
/// Options, the family's one odd `[5, 12]` (read from `DraftFlow-snapshot-20261017`
/// at :119) — took the ladder's 4 on its vertical and is 2 px shorter than it was.
/// That is an ink change, not a rename, so it belongs to the unsigned-sample list
/// #48 and is not counted as a free fold by this ceiling.
///
/// A ceiling at zero is only worth what the walk can see, so the walk now names both
/// spellings that could refill it: the digits (`[4, 12]`, `[5, 12]`) and the rung
/// pair (`[tokens::GAP_TIGHT, tokens::GAP_SECTION]`), which writes the same four
/// sides from L0 and was invisible to the digit needles (finding F6).
const BASELINE_MENU_ROW_HAND_TYPED: usize = 0;

#[test]
fn the_menu_row_inset_cut_at_the_sixteenth_screen_is_pinned() {
    // The tier is the pair the menu lines already wore: GAP_TIGHT above and below,
    // GAP_SECTION down both sides. Read from L0 rather than typed, so a rung
    // renumbered at L0 fails here instead of quietly resizing eight menus.
    assert_eq!(
        crate::ui::kit::PAD_MENU_ROW.top,
        tokens::GAP_TIGHT,
        "PAD_MENU_ROW no longer sits on GAP_TIGHT vertically"
    );
    assert_eq!(
        crate::ui::kit::PAD_MENU_ROW.bottom,
        tokens::GAP_TIGHT,
        "PAD_MENU_ROW is no longer symmetric top-to-bottom, so a menu line leans"
    );
    assert_eq!(
        crate::ui::kit::PAD_MENU_ROW.right,
        tokens::GAP_SECTION,
        "PAD_MENU_ROW no longer sits on GAP_SECTION laterally"
    );
    assert_eq!(
        crate::ui::kit::PAD_MENU_ROW.left,
        crate::ui::kit::PAD_MENU_ROW.right,
        "PAD_MENU_ROW is no longer left-right symmetric"
    );

    // The three tiers a reader could confuse this one with. PAD_PLATE shares its
    // vertical, PAD_CARD shares its lateral, Size::Control shares both halves one
    // axis at a time — and the roles are what keep them apart: a menu line is not a
    // list row, not a card, not a verb. If any pair converges on both axes the
    // tier system has collapsed back into a number.
    assert_ne!(
        crate::ui::kit::PAD_MENU_ROW.right,
        crate::ui::kit::PAD_PLATE.right,
        "PAD_MENU_ROW and PAD_PLATE now share a lateral, so a menu line and a list row are one plate"
    );
    assert_ne!(
        crate::ui::kit::PAD_MENU_ROW.top,
        crate::ui::kit::PAD_CARD.top,
        "PAD_MENU_ROW and PAD_CARD now share a vertical, so a menu line wears a card's air"
    );
    let control = crate::ui::kit::Size::Control.padding();
    assert_ne!(
        crate::ui::kit::PAD_MENU_ROW.right, control.right,
        "PAD_MENU_ROW and Size::Control now share a lateral, so the verb and the menu line are one tier again"
    );
    assert_eq!(
        crate::ui::kit::PAD_MENU_ROW.top, control.top,
        "the menu line and the verb no longer share their vertical air; the census behind PAD_MENU_ROW is stale, re-measure it"
    );
}

/// The two hand-written spellings of [`crate::ui::kit::PAD_MENU_ROW`]: the digits
/// themselves, and those same four sides spelled from the L0 rungs the tier is built
/// from. A ceiling at zero is only real if a fold cannot escape it by switching
/// spelling, which is finding F6 — the rung pair used to be invisible to this walk.
fn menu_row_shape(line: &str) -> bool {
    let flat: String = line.chars().filter(|c| *c != ' ').collect();
    flat.contains(".padding([4,12])")
        || flat.contains(".padding([5,12])")
        || flat.contains(".padding([tokens::GAP_TIGHT,tokens::GAP_SECTION])")
}

/// The needle is the only thing standing between the zero ceiling and a silent
/// refill, so it gets read against shapes it must not match as well as the three it
/// must. Written against source-shaped text (spaces included) because that is what
/// the walk feeds it.
#[test]
fn the_menu_row_needle_sees_every_spelling_and_none_other() {
    assert!(menu_row_shape("    .padding([4, 12])"));
    assert!(menu_row_shape("    .padding([5, 12])"));
    assert!(menu_row_shape(
        "    .padding([tokens::GAP_TIGHT, tokens::GAP_SECTION])"
    ));
    assert!(!menu_row_shape("    .padding(kit::PAD_MENU_ROW)"));
    assert!(!menu_row_shape("    .padding([4, 10])"));
    assert!(!menu_row_shape("    .padding(tokens::GAP_TIGHT)"));
    assert!(!menu_row_shape(
        "    .padding([tokens::GAP_TIGHT, tokens::GAP_CONTROL])"
    ));
}

/// `kit::PAD_MENU_ROW` names one plate; naming it does not delete the call sites
/// that still type its two numbers by hand. The walk counts only the hand-typed half.
///
/// Re-measured for the 32nd freeze: **16** sites in **11** files wear this inset and
/// every one of them names the tier — `isolate_popup`, `statusbar/mod.rs` x2,
/// `statusbar_menu`, `xref_manager` x3, `block_palette`, `count_palette` x2 (the
/// sixteenth and the fold-006 sites), `open_progress`, `print_all`, `xref_attach`,
/// `mlstyle` and `alias_editor` x2 (the five verbs that were hand-typed through the
/// 24th freeze). Nothing is left to count, so the census is no longer a
/// menu-line/verb split: the verbs that wrote `[4, 12]` came onto this tier at 0 px,
/// and the one that wrote `[5, 12]` is priced in pixels on #48 rather than excused
/// here. `kit::Size::Control` is still a different role (`[4, 10]`), and the pin test
/// above fails the moment the two laterals converge.
///
/// The count may only go DOWN, and it stays at zero only while the walk sees every
/// spelling; see `the_menu_row_needle_sees_every_spelling_and_none_other`.
#[test]
fn the_menu_row_family_keeps_its_measured_ceiling() {
    let root = src_root();
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    let mut hand_typed = Vec::new();
    for path in files {
        let rel = match path.strip_prefix(&root) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        // The kit, the tokens and this file itself: the doc above holds the needle,
        // so a guard that scanned its own source would fail by reading itself.
        if is_whitelisted(&rel) {
            continue;
        }
        let Ok(text) = fs::read_to_string(path) else { continue };
        for (i, line) in text.lines().enumerate() {
            let t = line.trim_start();
            if t.starts_with("//") {
                continue;
            }
            if menu_row_shape(t) {
                hand_typed.push(format!("{rel}:{}", i + 1));
            }
        }
    }
    assert!(
        hand_typed.len() <= BASELINE_MENU_ROW_HAND_TYPED,
        "REGRESSION: hand-typed menu-row insets went UP: {} vs baseline {} (+{}). New sites: {}. \
         Take the inset from `kit::PAD_MENU_ROW` (a menu line or its caption) or \
         `kit::Size::Control` (a verb) — do not retype [4, 12] or [5, 12] at a call site.",
        hand_typed.len(),
        BASELINE_MENU_ROW_HAND_TYPED,
        hand_typed.len() - BASELINE_MENU_ROW_HAND_TYPED,
        hand_typed.join(", "),
    );
    if hand_typed.len() < BASELINE_MENU_ROW_HAND_TYPED {
        println!(
            "PAD_MENU_ROW family: {} sites left, baseline is {} — lower the baseline by {} and re-count the menu-line/verb split in kit::PAD_MENU_ROW's doc.",
            hand_typed.len(),
            BASELINE_MENU_ROW_HAND_TYPED,
            BASELINE_MENU_ROW_HAND_TYPED - hand_typed.len(),
        );
    }
    for h in &hand_typed {
        println!("  still typing the menu-row pair by hand: {h}");
    }
}

/// Ratchet (g): the point size of a widget's own label comes from `tokens::TEXT_*`.
/// See the doc on `BASELINE_OFF_TOKEN_TEXT_SIZE` for how a whole family stayed
/// invisible: (d) reads `.size(`, and on a checkbox that is the box edge — the
/// font of the same widget is set by the *next* method, which nothing looked at.
#[test]
fn label_text_size_comes_from_the_type_ladder() {
    // Pin the surface first. A column whose name list silently shrinks reports
    // green at zero, which is how (e) froze at 48 instead of 101.
    let fns = vendor_text_size_fns();
    assert!(
        fns.len() >= MIN_VENDOR_TEXT_SIZE_FNS,
        "the vendor exposed only {:?} `*text_size*` setters (<{}): the fork moved or the \
         derivation rule stopped matching, so (g) would measure nothing and look clean",
        fns,
        MIN_VENDOR_TEXT_SIZE_FNS,
    );
    assert!(
        fns.iter().any(|n| n == "text_size"),
        "`text_size` must be in the derived surface; it is the setter (g) exists for ({:?})",
        fns
    );
    assert!(
        fns.iter().any(|n| n == "with_text_size"),
        "markdown's `with_text_size` takes the same quantity and must not be missed ({:?})",
        fns
    );

    // Pin the rule on synthetic text, including the split-line shape that a
    // line-based rule would lose.
    let needles: Vec<String> = fns
        .iter()
        .filter(|n| **n == "text_size" || **n == "with_text_size")
        .cloned()
        .collect();
    let count = |src: &str| text_size_off_ladder(&blank_literals(src), &needles);
    assert_eq!(count("cb.text_size(11)"), 1, "a literal label size is a row");
    assert_eq!(count("cb.text_size(11.0)"), 1, "a float literal is the same row");
    assert_eq!(
        count("cb.text_size(\n    12,\n)"),
        1,
        "a call split across lines must still count: the (f) column was created \
         because a one-shape rule looked green at a fifth of its height"
    );
    assert_eq!(count("cb.text_size(tokens::TEXT_LABEL)"), 0);
    assert_eq!(count("cb.text_size(size)"), 0);
    assert_eq!(
        count("qr.cell_size(4).total_size(120)"),
        0,
        "a QR module is a length, not a type size — it is not in this surface"
    );
    assert_eq!(count("grid.min_size(90.0)"), 0);
    assert_eq!(
        count("cb.size(14).text_size(11)"),
        1,
        "one row per quantity: `.size(14)` is (d)'s box edge, only `.text_size(11)` is (g)'s"
    );
    assert_eq!(
        count("Span::styled(a, m.with_text_size(13, s))"),
        1,
        "markdown's constructor form takes the same point size"
    );

    let c = scan_src();
    assert!(
        c.files_scanned >= MIN_SCANNED_FILES,
        "kit-contract guard only scanned {} files (<{}); the text-size count below is meaningless",
        c.files_scanned,
        MIN_SCANNED_FILES
    );
    assert!(
        c.text_size_off_ladder <= BASELINE_OFF_TOKEN_TEXT_SIZE,
        "REGRESSION: screens deciding their own label point size went UP: {} vs baseline {} (+{}). \
         `.text_size(N)` is the widget's own caption in points — use `tokens::TEXT_*`; and if the \
         size you want is on no rung, that is a ladder decision, not a call-site one.",
        c.text_size_off_ladder,
        BASELINE_OFF_TOKEN_TEXT_SIZE,
        c.text_size_off_ladder - BASELINE_OFF_TOKEN_TEXT_SIZE,
    );

    if c.text_size_off_ladder < BASELINE_OFF_TOKEN_TEXT_SIZE {
        println!(
            "KIT-CONTRACT: text_size {} -> {} (-{}). BASELINE CAN BE LOWERED TO {}",
            BASELINE_OFF_TOKEN_TEXT_SIZE,
            c.text_size_off_ladder,
            BASELINE_OFF_TOKEN_TEXT_SIZE - c.text_size_off_ladder,
            c.text_size_off_ladder
        );
    } else {
        println!(
            "KIT-CONTRACT: text_size flat at {} (frozen {}, surface {:?} derived from vendor).",
            c.text_size_off_ladder, BASELINE_FROZEN, fns
        );
    }
}

// ── The surface ladder contracts ────────────────────────────────────────
//
// The ratchets above police *literals*; these police the *geometry* of the new
// L0 ladder in `tokens.rs`. The old mechanism was a hand-tuned per-scheme mix
// ratio and it was measurably broken: because `Color::mix` interpolates in
// linear light, one ratio meant wildly different perceptual steps on a near-
// black page than on a near-white one. Measured 2026-10-15 the light scheme's
// four tiers landed at L* 96.57 / 96.61 / 97.04 / 97.18 (adjacent gaps of
// 0.04–0.43 L*, i.e. four named roles drawing one plane) and the dark scheme's
// first rung jumped 19.6 L* while the last two moved 0.39 and 1.08. These tests
// make that class of regression fail the build.

use super::{common, tokens};
use crate::app::config::all_themes;
use crate::ui::kit::{
    danger_style, dialog_body_style, label::Tone, pill_style, signal_button_style, style_for,
    subtle_style, surface::Tier, text_style, Role,
};
use crate::ui::style::draftflow_theme::{draftflow_colors, draftflow_themes};
use iced::widget::{button, container};
use iced::Color;

/// The four raised planes in ladder order. `tokens::Rank` has no `ALL`, and the
/// order here *is* the `rung()` order, so this is the single enumeration these
/// contracts walk.
const RANKS: [tokens::Rank; 4] = [
    tokens::Rank::Band,
    tokens::Rank::Plate,
    tokens::Rank::Card,
    tokens::Rank::Float,
];

/// Display labels for the five-point sequence `page → Band → Plate → Card →
/// Float`; `NAMES[i]` and `NAMES[i + 1]` bracket gap `i`.
const NAMES: [&str; 5] = ["page", "Band", "Plate", "Card", "Float"];

/// THE LAW (for every selectable theme in `all_themes()`):
/// 1. `page`, `Band`, `Plate`, `Card`, `Float` are *strictly monotonic* in CIE
///    L* (a single consistent direction — `tokens::surface` runs the ladder
///    downward on a page with no lift headroom, so we test magnitude/order, not
///    "brighter"). Each of the four adjacent gaps, `page → Band` included, must
///    be at least `max(0.5 L*, half the mean gap of that theme's own ladder)`.
/// 2. WCAG AA body ink: every plane clears 4.5:1 against the scheme's own
///    `background.base.text`.
/// 3. Recess direction: a well (`inset`) is darker than the page and a sink
///    (`sunk`) is darker than the well, for every theme.
///
/// WHAT THE GAP FORMULATION GUARANTEES / DELIBERATELY DOES NOT. The second term
/// is relative to the ladder *this* theme solved, so a cramped third-party
/// palette degrades to a compressed-but-legible ladder instead of tripping a
/// hard 1.5 L* wall it was never given room to hit; `tokens::surface` scales the
/// ladder by the room the scheme has, so demanding a fixed absolute step would
/// contradict that design. The absolute 0.5 L* floor still stops the historical
/// collapse (gaps of 0.04–0.43). What it does NOT check: perceptual *equal-ness*
/// of the rungs (a 5:3 split passes), and it does not re-derive `tokens::room()`
/// — these tests observe the emitted colours, so they stay honest even if the
/// room heuristic changes underneath them.
#[test]
fn surface_tiers_separate_in_every_theme() {
    let mut failures: Vec<String> = Vec::new();

    for theme in all_themes() {
        let palette = theme.palette();
        let base = palette.background.base.color;
        let ink = palette.background.base.text;
        let page = common::lstar(base);

        let surfaces: [Color; 4] = RANKS.map(|rank| tokens::surface(&theme, rank));
        let rungs: [f32; 4] = surfaces.map(|c| common::lstar(c));

        // DraftFlow's two schemes carry an explicit sheet whose rungs answer
        // different questions (panel, inner plate, paper, file card), so the
        // vendor ladder law does not describe them. Their measured relations
        // are pinned by `brand_ladder_has_measurable_steps`; the AA bar in (2)
        // still applies to every theme.
        let explicit_sheet =
            crate::ui::style::draftflow_theme::draftflow_colors(&theme).is_some();

        // (1) Monotonic ladder with a real, scale-relative step between rungs.
        if !explicit_sheet {
            let seq: [f32; 5] = {
                let mut s = [page; 5];
                s[1..].copy_from_slice(&rungs);
                s
            };
            let diffs: [f32; 4] = std::array::from_fn(|i| seq[i + 1] - seq[i]);
            let monotonic =
                diffs.iter().all(|d| *d > 0.0) || diffs.iter().all(|d| *d < 0.0);
            if !monotonic {
                failures.push(format!(
                    "{theme}: ladder not strictly monotonic in L* (page {page:.2}, rungs [{:.2}, {:.2}, {:.2}, {:.2}], diffs [{:.2}, {:.2}, {:.2}, {:.2}])",
                    rungs[0], rungs[1], rungs[2], rungs[3],
                    diffs[0], diffs[1], diffs[2], diffs[3],
                ));
            } else {
                let mean_gap = diffs.iter().map(|d| d.abs()).sum::<f32>() / 4.0;
                for (i, d) in diffs.iter().enumerate() {
                    let gap = d.abs();
                    let floor = 0.5_f32.max(mean_gap * 0.5);
                    if gap < floor {
                        let a = NAMES[i];
                        let b = NAMES[i + 1];
                        failures.push(format!(
                            "{theme}: gap {a} -> {b} is {gap:.3} L*, below the {floor:.3} floor (mean gap {mean_gap:.3}, diffs [{:.2}, {:.2}, {:.2}, {:.2}])",
                            diffs[0], diffs[1], diffs[2], diffs[3],
                        ));
                    }
                }
            }
        }

        // (2) WCAG AA body ink on every plane.
        for (rank, surface) in RANKS.iter().zip(surfaces) {
            let contrast = common::wcag_contrast(ink, surface);
            if contrast < 4.5 {
                failures.push(format!(
                    "{theme}: ink {ink:?} on {rank:?} {surface:?} (L* {:.2}) is {contrast:.2}:1, below AA",
                    common::lstar(surface),
                ));
            }
        }

        // (3) Recess direction: well below page, sink below well. DraftFlow
        // names its recesses on the sheet instead (inset is the paper, sunk is
        // the shell), which the brand test pins as an identity.
        if !explicit_sheet {
            let inset = common::lstar(tokens::inset(&theme));
            let sunk = common::lstar(tokens::sunk(&theme));
            if !(inset < page && sunk < inset) {
                failures.push(format!(
                    "{theme}: recess inverted (page {page:.2}, inset {inset:.2}, sunk {sunk:.2})",
                ));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "{} surface-ladder law violation(s) across {} themes:\n{}",
            failures.len(),
            all_themes().len(),
            failures.join("\n"),
        );
    }
}

/// The two schemes the app actually ships carry an explicit sheet rather than a
/// solved ladder: `Band` is the panel, `Plate` the inner plate, `Card` the paper
/// and `Float` the file card. The four answer different questions, so the
/// vendor law's "one direction, equal steps" does not describe them; what the
/// one-pen grammar does require is that each plane be *distinguishable*. Pinned
/// here: every plane sits at least 1.0 L* from the page, neighbouring planes at
/// least 0.5 L* from each other, all four on the same side of the page, and the
/// whole five-plane spread stays inside 1–25 L* (below 1 the sheet is one plane,
/// above 25 it reads as separate UIs). The recesses are names, not rungs:
/// `inset` is the paper and `sunk` is the shell, pinned as identities. The full
/// measured table is printed so a run of this test leaves an auditable trail.
#[test]
fn brand_ladder_has_measurable_steps() {
    for theme in draftflow_themes() {
        let page = common::lstar(theme.palette().background.base.color);
        let rungs: [(tokens::Rank, f32); 4] =
            RANKS.map(|rank| (rank, common::lstar(tokens::surface(&theme, rank))));

        println!("── {theme} surface sheet (CIE L*) ──");
        println!("   page   {page:>6.2}");
        let seq: [f32; 5] = {
            let mut s = [page; 5];
            for (i, (_, l)) in rungs.iter().enumerate() {
                s[i + 1] = *l;
            }
            s
        };
        let diffs: [f32; 4] = std::array::from_fn(|i| seq[i + 1] - seq[i]);
        for (i, (rank, l)) in rungs.iter().enumerate() {
            println!(
                "   {rank:<6?} {l:>6.2}   gap {:+.2} L*",
                diffs[i],
            );
        }
        let lo = seq.iter().cloned().fold(f32::MAX, f32::min);
        let hi = seq.iter().cloned().fold(f32::MIN, f32::max);
        let spread = hi - lo;
        println!("   spread over the five planes = {spread:.2} L*\n");

        for (i, (rank, l)) in rungs.iter().enumerate() {
            assert!(
                (*l - page).abs() >= 1.0,
                "{theme}: {rank:?} sits {:.3} L* from the page ({page:.2}), below the 1.0 \
                 distinguishable floor",
                (*l - page).abs(),
            );
            if i > 0 {
                let prev = seq[i];
                assert!(
                    (*l - prev).abs() >= 0.5,
                    "{theme}: {rank:?} is {:.3} L* from the plane beneath it, below the 0.5 \
                     collapse floor (sheet L* page {page:.2} then [{:.2}, {:.2}, {:.2}, {:.2}])",
                    (*l - prev).abs(),
                    seq[1], seq[2], seq[3], seq[4],
                );
            }
        }
        let raised_up = seq[1..].iter().all(|l| *l > page);
        let raised_down = seq[1..].iter().all(|l| *l < page);
        assert!(
            raised_up || raised_down,
            "{theme}: the raised planes straddle the page (page {page:.2}, raised [{:.2}, {:.2}, {:.2}, {:.2}])",
            seq[1], seq[2], seq[3], seq[4],
        );
        assert!(
            spread >= 1.0 && spread <= 25.0,
            "{theme}: total plane spread {spread:.2} L* is outside the 1–25 window",
        );

        let colors = crate::ui::style::draftflow_theme::draftflow_colors(&theme)
            .expect("draftflow_themes() yields only the two explicit sheets");
        assert_eq!(
            tokens::inset(&theme), colors.surface,
            "{theme}: `inset` left the paper plane"
        );
        assert_eq!(
            tokens::sunk(&theme), colors.shell,
            "{theme}: `sunk` left the shell plane"
        );
    }
}

/// Guards the closed-form inverse `tokens::plane()` leans on: `plane()` targets
/// an L*, converts it to a linear luminance via `lstar_to_luminance`, then
/// inverts the affine mix; the whole scheme is only sound if L* → Y → L* is the
/// identity, *including* across CIE's linear-segment handover at L* = 8 (the
/// epsilon/K join). Sampling `{0, 3, 8, 20, 50, 88, 100}` straddles that join on
/// both sides and pins the endpoints, so a future edit that lets the two
/// branches disagree by even a hair fails here rather than shipping a ladder
/// with a discontinuity near black — exactly where a dark CAD page lives.
#[test]
fn lstar_round_trip_is_exact() {
    const TOL: f32 = 1e-3;

    // (a) Fixed L* points across the handover.
    for l in [0.0_f32, 3.0, 8.0, 20.0, 50.0, 88.0, 100.0] {
        let back = common::luminance_to_lstar(common::lstar_to_luminance(l));
        assert!(
            (back - l).abs() < TOL,
            "round-trip L* {l} → Y → L* landed at {back:.6}, off by {:.6}",
            back - l,
        );
    }

    // (b) The same round-trip applied to real plane colours, so the identity
    //     holds for whatever `lstar()` produces on live palettes, not just on
    //     hand-written scalars.
    let dfs = draftflow_themes();
    let samples: [Color; 7] = [
        Color::WHITE,
        Color::BLACK,
        dfs[0].palette().background.base.color, // DraftFlow Dark seed
        dfs[1].palette().background.base.color, // DraftFlow Light seed
        Color::from_rgb8(64, 64, 64),
        Color::from_rgb8(128, 128, 128),
        Color::from_rgb8(192, 192, 192),
    ];
    for color in samples {
        let l = common::lstar(color);
        let back = common::luminance_to_lstar(common::lstar_to_luminance(l));
        assert!(
            (back - l).abs() < TOL,
            "colour {color:?} at L* {l:.6} round-tripped to {back:.6}, off by {:.6}",
            back - l,
        );
    }

    // (c) The endpoints the ladder leans against: white is 100, black is 0.
    assert!(
        (common::lstar(Color::WHITE) - 100.0).abs() <= TOL,
        "lstar(WHITE) = {:.6}, not 100.0 within {TOL}",
        common::lstar(Color::WHITE),
    );
    assert!(
        (common::lstar(Color::BLACK) - 0.0).abs() <= TOL,
        "lstar(BLACK) = {:.6}, not 0.0 within {TOL}",
        common::lstar(Color::BLACK),
    );
}

// ── #82's input ledger: what folding iced's built-in style fns would paint ─────
//
// Ratchet column (e) counts 101 call sites that hand their paint to `vendor/iced`
// instead of `ui::kit`. Folding them is a *visual* migration, so the sign-off
// needs the measured delta, not a claim: for every theme that can be selected,
// this reads the pixel iced paints today and the pixel the kit candidate paints
// in its place, and reports how far apart they are in CIE L*.
//
// The invariant the fold buys — and the one thing asserted here rather than
// merely printed — is that the kit candidate's plate is always a rung of the
// L0 ladder (page / Band / Plate / Card / Float / inset / sunk). iced's built-in
// styles instead read `palette.background.{weakest,weaker,strong}` and
// `palette.{primary,danger,…}.base`, which are per-scheme hand-picked swatches
// outside that ladder; that is exactly why a plate can vanish on one theme and
// shout on another. If a kit candidate ever leaves the ladder, the fold would
// smuggle an off-ladder pixel back in, so that fails the build.

/// The seven planes `kit::surface::Tier` can paint, in ladder order then the
/// recesses then the page. Local because `Tier::radius` is private to L1 and the
/// question here is geometry, which is only observable through `style()`.
const TIERS: [Tier; 7] = [
    Tier::Band,
    Tier::Plate,
    Tier::Card,
    Tier::Float,
    Tier::Well,
    Tier::Sink,
    Tier::Flush,
];

/// One resting control's paint, normalized across `button::Style` and
/// `container::Style` so a delta can be compared between the two.
#[derive(Clone, Copy, Debug)]
struct Paint {
    plate: Option<Color>,
    ink: Option<Color>,
    radius: f32,
    edge_width: f32,
    edge: Color,
}

fn plate_of(bg: &Option<iced::Background>) -> Option<Color> {
    match bg {
        Some(iced::Background::Color(c)) => Some(*c),
        _ => None,
    }
}

fn read_button(style: button::Style) -> Paint {
    Paint {
        plate: plate_of(&style.background),
        // `button::Style` always carries an ink; the container half is optional.
        ink: Some(style.text_color),
        radius: style.border.radius.top_left,
        edge_width: style.border.width,
        edge: style.border.color,
    }
}

fn read_container(style: container::Style) -> Paint {
    Paint {
        plate: plate_of(&style.background),
        ink: style.text_color,
        radius: style.border.radius.top_left,
        edge_width: style.border.width,
        edge: style.border.color,
    }
}

/// `(what iced paints today, what the kit candidate paints instead)`; `None` for
/// a candidate that does not exist yet, which is itself a finding.
type Fold = fn(&iced::Theme) -> (Paint, Option<Paint>);

fn subtle_over_standard(theme: &iced::Theme) -> (Paint, Option<Paint>) {
    let s = button::Status::Active;
    (
        read_button(button::subtle(theme, s)),
        Some(read_button(style_for(theme, s, Role::Standard, ""))),
    )
}

fn text_over_ghost(theme: &iced::Theme) -> (Paint, Option<Paint>) {
    let s = button::Status::Active;
    (
        read_button(button::text(theme, s)),
        Some(read_button(style_for(theme, s, Role::Ghost, ""))),
    )
}

fn danger_over_role(theme: &iced::Theme) -> (Paint, Option<Paint>) {
    let s = button::Status::Active;
    (
        read_button(button::danger(theme, s)),
        Some(read_button(style_for(theme, s, Role::Danger, ""))),
    )
}

fn success_over_signal(theme: &iced::Theme) -> (Paint, Option<Paint>) {
    let s = button::Status::Active;
    // Both migrated sites (`alias_editor.rs`, `shortcuts.rs`) are icon buttons on
    // the unsaved-changes guard's own ground, which is `kit::ground_style` — the
    // theme's base, i.e. the pixel `Tier::Flush` resolves to because it paints
    // nothing of its own. The other six tiers are swept every run by
    // `theme_accessibility_tests::the_signal_chip_carries_its_own_words`.
    let kit = signal_button_style(Tone::Success, Tier::Flush);
    (
        read_button(button::success(theme, s)),
        Some(read_button(kit(theme, s))),
    )
}

fn warning_over_signal(theme: &iced::Theme) -> (Paint, Option<Paint>) {
    let s = button::Status::Active;
    // Two sites with two grounds: the AEC-drop dialog's `Tier::Card` body and the
    // status bar's `Tier::Band`. Card is the one measured here because it is the
    // smaller, lighter plane of the two and therefore the harder edge to solve;
    // Band's readings are printed per theme by the seven-ground chip guard.
    let kit = signal_button_style(Tone::Warning, Tier::Card);
    (
        read_button(button::warning(theme, s)),
        Some(read_button(kit(theme, s))),
    )
}

/// The four list and log recesses the row folds onto: a scrollable layout list,
/// a missing-font list, two recovery panes. `Well` is the only tier whose corner
/// already equals the vendor's 5.0, so this is the fold that moves no geometry.
fn bordered_box_over_well(theme: &iced::Theme) -> (Paint, Option<Paint>) {
    (
        read_container(container::bordered_box(theme)),
        Some(read_container(Tier::Well.style(theme, ""))),
    )
}

/// The ten detached bodies: tooltip contents, a right-click menu, a drag ghost,
/// a cycling popup. `Float` is the ladder's rung for "above the page" and the one
/// tier whose depth comes mostly from its shadow, so this is the fold that moves
/// the corner as well as the plate — printed, not assumed.
fn bordered_box_over_float(theme: &iced::Theme) -> (Paint, Option<Paint>) {
    (
        read_container(container::bordered_box(theme)),
        Some(read_container(Tier::Float.style(theme, ""))),
    )
}

/// The status bar's readout chip (`status_pill`). The pill ladder's resting rung
/// *is* the palette pair `container::bordered_box` drew — `pill_parts` takes
/// `weakest.color` and gives back `weakest.text` at hover 0 — so this row folds
/// with no plate, ink or corner movement, and it is the one row whose destination
/// is a ladder the tier vocabulary does not cover.
fn bordered_box_over_pill(theme: &iced::Theme) -> (Paint, Option<Paint>) {
    (
        read_container(container::bordered_box(theme)),
        Some(read_container(pill_style("", false, theme, ""))),
    )
}

/// `container::rounded_box` corners at 2.0 — the `RADIUS_MARK` rung — and fills
/// with `palette.background.weak`. Before decision A that fill was not a rung of
/// anything L0 owned, so the only candidate the kit could offer was `None`: this
/// row measured zero pixels because there was nothing to measure against, and the
/// four sites stayed on iced's sheet. A put `weak` on the control-plate ladder by
/// definition, which is what makes decision E a bit-identical fold rather than a
/// redesign, and this row now reads the reproduction against the vendor.
fn rounded_box_over_dialog_body(theme: &iced::Theme) -> (Paint, Option<Paint>) {
    (
        read_container(container::rounded_box(theme)),
        Some(read_container(dialog_body_style(theme))),
    )
}

/// The shapes the frozen run's rows sort into, one entry per *destination*, so a
/// vendor sheet folded three different ways prints three costs instead of an
/// average of them. `container::bordered_box`'s 15 rows split 10 / 4 / 1 by what
/// their hosts are: ten detached bodies (tooltips, a right-click menu, a drag
/// ghost, a cycling popup), four scrollable list/log recesses, one status-bar
/// readout chip. The counts are read off the source lines, not from the CSV
/// snippet field, which truncates at commas; the reconciliation assert below is
/// what keeps this table honest when a call site is added, removed or renamed.
///
/// The first three now read **0** because decision B emptied them (2026-10-17
/// 14:1x): their 77 rows were migrated by a bit-identical copy of the vendor
/// stylesheet, which is why the fold fns below are kept even though no call site
/// is left — they record what the *other* route (mapping each vendor sheet onto an
/// existing kit `Role`) would have cost, and it was not zero: `button::subtle` over
/// `Role::Standard` moved the plate by |ΔL*| 7.18–8.81 on 26/26 themes and came
/// from iced's palette pair, not the surface ladder. That measurement is still
/// printed every run,
/// so the rejected alternative stays visible instead of being argued from memory.
///
/// The three `container::bordered_box` rows read **0** for the same reason: decision
/// D emptied them (2026-10-17 14:4x) by sending each host to the tier it is, and the
/// three costs above are what that moved. They stay in the table because the next
/// screen that reaches for a hand-rolled frame should see that a detached body costs
/// |ΔL*| 8–17 and a readout chip costs nothing.
///
/// The `container::rounded_box` row reads **0** by a different route. It was the one
/// bucket with *no candidate at all* before decision A — `rounded_box` fills with
/// `palette.background.weak`, which was not a rung of anything L0 owned, so the row
/// could only report "no tier at 2.0" and measure nothing. A made that slot a named
/// rung, and decision E (2026-10-17 16:1x) then folded the four sites onto
/// `kit::dialog_body_style` *bit-identically*, so this row is the only one in the
/// ledger whose migration cost is asserted rather than printed: see
/// [`the_vendor_equivalents_are_exact`], which walks the container sheets too.
const FOLD_BUCKETS: usize = 9;
const FOLDS: [(&str, &str, usize, Fold); FOLD_BUCKETS] = [
    ("button::subtle", "kit::subtle_style", 0, subtle_over_standard),
    ("button::text", "kit::text_style", 0, text_over_ghost),
    ("button::danger", "kit::danger_style", 0, danger_over_role),
    ("container::bordered_box", "Tier::Float", 0, bordered_box_over_float),
    ("container::bordered_box", "Tier::Well", 0, bordered_box_over_well),
    ("container::bordered_box", "pill resting", 0, bordered_box_over_pill),
    (
        "container::rounded_box",
        "kit::dialog_body_style",
        0,
        rounded_box_over_dialog_body,
    ),
    ("button::warning", "signal_button_style", 0, warning_over_signal),
    ("button::success", "signal_button_style", 0, success_over_signal),
];

/// `checkbox::primary` no longer has a call site: the 35th freeze routed the plot
/// dialog's five disabled-row boxes through `kit::checkbox_style`, which keeps this
/// sheet's plate and frame and dims only the tick and the label. The bucket stays
/// listed so the law above still checks that the vendor fn exists, but it owes 0
/// rows and the ratchet must agree.
const CHECKBOX_PRIMARY_ROWS: usize = 0;

/// Which layer a plate was actually produced by. Measured, not asserted from a
/// reading of the code, because the whole premise of #82 — "fold the vendor
/// styles into L1 and the app gets L0's ladder" — only holds if L1's own roles
/// sit on that ladder. They do not all do so.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Source {
    /// A rung of L0's perceptual ladder: page, Band, Plate, Card, Float, inset, sunk.
    Ladder,
    /// A rung of L0's control-plate ladder ([`tokens::Control`], decision A). It
    /// reproduces iced's `palette.background` pairs bit-for-bit on purpose, so this
    /// arm is what `VendorPair` used to be — the pixels are the same, the *owner*
    /// is not: the kit now asks L0 for a named rung instead of spelling iced's
    /// private field. Before A this arm was iced's pair, and `neutral`, `stronger`
    /// and `strongest` fell through to `Nowhere` because the old list had only five
    /// of the eight slots.
    Control,
    /// A semantic swatch run through `tokens::painted` — the signal chip family's
    /// own plate rule, deliberately not on the neutral ladder.
    Semantic,
    /// A pixel nothing in L0 or L1 can produce: a hand-picked colour.
    Nowhere,
}

/// The seven control rungs, spelled out rather than derived, so a variant added to
/// [`tokens::Control`] fails the `CONTROL_RUNGS.len() == tokens::CONTROL_STEPS.len()`
/// assert below instead of quietly dropping out of the classification.
const CONTROL_RUNGS: [tokens::Control; 7] = [
    tokens::Control::Weakest,
    tokens::Control::Weaker,
    tokens::Control::Weak,
    tokens::Control::Neutral,
    tokens::Control::Strong,
    tokens::Control::Stronger,
    tokens::Control::Strongest,
];

const SOURCES: [Source; 4] = [Source::Ladder, Source::Control, Source::Semantic, Source::Nowhere];

impl Source {
    /// The single source that can produce `plate` under `theme`, or `Nowhere`.
    /// Membership is exact float equality on purpose: these are values the same
    /// pure functions return for the same theme, so anything that is not bit-equal
    /// was not produced by that path.
    fn of(theme: &iced::Theme, plate: Color) -> Source {
        let palette = theme.palette();
        let mut ladder: Vec<Color> = vec![
            palette.background.base.color,
            tokens::inset(theme),
            tokens::sunk(theme),
        ];
        ladder.extend(RANKS.iter().map(|rank| tokens::surface(theme, *rank)));
        if ladder.iter().any(|rung| *rung == plate) {
            return Source::Ladder;
        }
        if CONTROL_RUNGS
            .iter()
            .any(|rung| tokens::control_plate(theme, *rung) == plate)
        {
            return Source::Control;
        }
        for swatch in [
            &palette.primary,
            &palette.success,
            &palette.warning,
            &palette.danger,
        ] {
            for shade in [swatch.weak.color, swatch.base.color, swatch.strong.color] {
                if shade == plate {
                    return Source::Semantic;
                }
                // `painted()` only *is* a named layer when it dips by one of L0's own
                // rung constants. Measured 2026-10-17: `signal_button_style` rests at
                // `painted(swatch, SIGNAL_DIP_REST)`, and because this arm only knew the
                // zero spelling it branded all 26 themes hand-picked — the same defect
                // class as (d)/(g) and `#[path = "…"]`: one rule matching one spelling is
                // a broken measurement, not a broken subject.
                for dip in [
                    tokens::HERO_DIP_HOVER,
                    tokens::HERO_DIP_REST,
                    tokens::HERO_DIP_PRESS,
                    tokens::SIGNAL_DIP_LIT,
                    tokens::SIGNAL_DIP_REST,
                ] {
                    if tokens::painted(shade, dip).0 == plate {
                        return Source::Semantic;
                    }
                }
            }
        }
        Source::Nowhere
    }
}

/// THE LAW:
/// 1. The bucket table reconciles with the measured (e) count, so this ledger
///    cannot drift away from the ratchet it is meant to feed.
/// 2. Every bucket's vendor fn is still derivable from `vendor/iced/widget/src`,
///    so a renamed style fn fails here instead of silently measuring stale pixels.
/// 3. Every kit candidate's plate is producible by a *named* layer: a rung of L0's
///    surface ladder, a rung of L0's control-plate ladder (decision A), or a
///    semantic swatch through `tokens::painted`. A plate none of them produces is a
///    hand-picked pixel and fails. A plate at alpha 0 is not a plate — it paints
///    nothing — so it is counted as `transparent at rest` instead of being demanded
///    a name; the DraftFlow pill ladder's resting rung has that shape. Which named
///    layer each bucket lands on is
///    printed, not asserted, because the surface ladder is not the only legal
///    answer — but it is the answer #82 has to choose deliberately, per role, with
///    the pixel delta in front of it.
/// 4. `container::rounded_box`'s corner *is* the `RADIUS_MARK` rung. That equality is
///    the reason decision E could be a rename rather than a redesign: the obstacle was
///    never the radius ladder, it was that no L0 rung owned `palette.background.weak`
///    until A. Still asserted, because if iced ever moves that corner the kit sheet
///    stops being its replacement and the (e) count would be reporting migrated sites
///    whose pixels changed.
///
/// Then it prints, per bucket: how many themes already paint the identical plate
/// (a zero-pixel fold), and the min / median / max |ΔL*| where they differ. Those
/// numbers are the input for #82's four L1 decisions; they are deliberately not
/// asserted, because "how far should a plate move" is a design call, not a law.
#[test]
fn folding_the_builtin_styles_into_the_kit_is_measured_not_assumed() {
    let c = scan_src();
    let tabled: usize = FOLDS.iter().map(|(_, _, rows, _)| *rows).sum::<usize>() + CHECKBOX_PRIMARY_ROWS;
    assert_eq!(
        tabled, c.builtin_style_call,
        "the (e) fold ledger covers {tabled} call sites but the ratchet measures {}; \
         a built-in style fn was added, removed or renamed and this table no longer \
         reconciles with column (e) — re-run `bash D:\\tmp\\measure3.sh` and re-derive \
         the per-name counts from the source lines",
        c.builtin_style_call,
    );

    let surface = vendor_style_fns();
    let mut names: Vec<&str> = FOLDS.iter().map(|(name, _, _, _)| *name).collect();
    names.push("checkbox::primary");
    for name in names {
        assert!(
            surface.iter().any(|n| n == name),
            "`{name}` is no longer in the surface derived from `vendor/iced/widget/src` \
             ({} names): this fold ledger is measuring a style fn that does not exist, \
             so its deltas are fiction",
            surface.len(),
        );
    }

    let mut hand_picked: Vec<String> = Vec::new();
    let mut identical: [usize; FOLD_BUCKETS] = [0; FOLD_BUCKETS];
    let mut both_unset: [usize; FOLD_BUCKETS] = [0; FOLD_BUCKETS];
    let mut transparent: [usize; FOLD_BUCKETS] = [0; FOLD_BUCKETS];
    let mut deltas: [Vec<f32>; FOLD_BUCKETS] = std::array::from_fn(|_| Vec::new());
    let mut ink_moved: [usize; FOLD_BUCKETS] = [0; FOLD_BUCKETS];
    let mut corner_moved: [usize; FOLD_BUCKETS] = [0; FOLD_BUCKETS];
    let mut frame_moved: [usize; FOLD_BUCKETS] = [0; FOLD_BUCKETS];
    let mut sources = [[0usize; SOURCES.len()]; FOLD_BUCKETS];

    let themes = all_themes();
    let theme_count = themes.len();
    assert!(theme_count > 0, "all_themes() returned no theme to measure");
    let first = &themes[0];

    for theme in themes.iter() {
        for (i, (name, dest, _, fold)) in FOLDS.iter().enumerate() {
            let (iced_today, kit_instead) = fold(theme);
            let Some(candidate) = kit_instead else { continue };
            match candidate.plate {
                // A plate no L0 or L1 path can produce is the only genuine
                // violation: it means the kit is handing out a pixel it cannot
                // name. Which *legal* source it picks is the design decision, so
                // it is counted below rather than asserted away here.
                Some(plate) => {
                    // A fully transparent plate is not a plate: it paints no pixel,
                    // so no L0 layer is expected to name it. The pill ladder's
                    // resting rung on the DraftFlow schemes is exactly this
                    // (`colors.hover` at alpha 0, waiting for the pointer), and
                    // reporting it as an unnamed fill would call the sheet's
                    // "no plate at rest" a measurement defect. The count is printed
                    // rather than folded into `both_unset`, because iced's sheet
                    // does paint a plate here; the fold deletes it.
                    if plate.a == 0.0 {
                        transparent[i] += 1;
                        continue;
                    }
                    let source = Source::of(theme, plate);
                    if source == Source::Nowhere {
                        hand_picked.push(format!(
                            "{theme}: folding `{name}` onto {dest} would paint {plate:?} at L* {:.2}, which \
                             no rung of L0's surface ladder, control-plate ladder or semantic swatch \
                             produces",
                            common::lstar(plate),
                        ));
                    }
                    sources[i][SOURCES.iter().position(|s| *s == source).unwrap()] += 1;
                }
                None => {
                    // `button::text` and `Role::Ghost` agree on having no plate at
                    // all; that is the zero-pixel case, not a missing fill.
                    assert_eq!(
                        iced_today.plate, None,
                        "{theme}: folding `{name}` onto {dest} drops a plate iced's own style has \
                         ({:?}); a fold may not delete a surface",
                        iced_today.plate,
                    );
                    both_unset[i] += 1;
                }
            }
            match (iced_today.plate, candidate.plate) {
                (Some(a), Some(b)) => {
                    if a == b {
                        identical[i] += 1;
                    } else {
                        deltas[i].push((common::lstar(a) - common::lstar(b)).abs());
                    }
                }
                (None, None) => {}
                _ => deltas[i].push(f32::NAN),
            }
            if iced_today.ink != candidate.ink {
                ink_moved[i] += 1;
            }
            if (iced_today.radius - candidate.radius).abs() > 1e-3 {
                corner_moved[i] += 1;
            }
            if (iced_today.edge_width - candidate.edge_width).abs() > 1e-3
                || (iced_today.edge != candidate.edge && iced_today.edge_width > 0.0)
            {
                frame_moved[i] += 1;
            }
        }
    }

    // The ledger prints before any verdict is handed down. Measured 2026-10-17:
    // with the asserts first, a failing fold hid every per-bucket number, and the
    // 52 remaining offenders were the only evidence left — exactly the file, theme
    // and delta a reviewer needs in order to decide, not just to be blocked on.
    let rounded_box_radius = read_container(container::rounded_box(first)).radius;

    println!(
        "KIT-CONTRACT: (e) fold ledger over {} themes, reconciled to {} call sites \
         ({FOLD_BUCKETS} destination rows + {CHECKBOX_PRIMARY_ROWS} checkbox::primary with no L1 primitive):",
        theme_count, tabled,
    );
    println!(
        "KIT-CONTRACT:   tier radii available in kit::surface = [{}]",
        TIERS
            .iter()
            .map(|t| format!("{t:?} {:.1}", t.style(first, "").border.radius.top_left))
            .collect::<Vec<_>>()
            .join(", ")
    );
    for (i, (name, dest, rows, _)) in FOLDS.iter().enumerate() {
        let real: Vec<f32> = deltas[i].iter().copied().filter(|d| !d.is_nan()).collect();
        let unmeasurable = deltas[i].len() - real.len();
        let (min, median, max) = if real.is_empty() {
            (0.0, 0.0, 0.0)
        } else {
            let mut sorted = real.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            (sorted[0], sorted[sorted.len() / 2], sorted[sorted.len() - 1])
        };
        let plate = identical[i] + both_unset[i];
        let invisible = if transparent[i] > 0 {
            format!(", {} transparent at rest", transparent[i])
        } else {
            String::new()
        };
        println!(
            "KIT-CONTRACT:   `{name}` -> {dest}: {rows} rows, zero-pixel plate on {plate}/{theme_count} \
             themes ({} same colour, {} both no plate{invisible}), |ΔL*| min {min:.2} median {median:.2} \
             max {max:.2}; ink differs on {}/{theme_count}; corner differs on {}/{theme_count}; \
             frame differs on {}/{theme_count}; plate not comparable on {unmeasurable}",
            identical[i], both_unset[i], ink_moved[i], corner_moved[i], frame_moved[i],
        );
        println!(
            "KIT-CONTRACT:     plate source over {theme_count} themes: {}",
            SOURCES
                .iter()
                .enumerate()
                .map(|(j, s)| format!("{s:?} {}", sources[i][j]))
                .collect::<Vec<_>>()
                .join(" / "),
        );
    }

    assert!(
        (rounded_box_radius - tokens::RADIUS_CONTROL).abs() <= 1e-3,
        "`container::rounded_box` corners at {rounded_box_radius}, no longer the `RADIUS_CONTROL` \
         rung ({})",
        tokens::RADIUS_CONTROL,
    );

    assert!(
        hand_picked.is_empty(),
        "{} fold candidate(s) paint a pixel no layer can name:\n{}",
        hand_picked.len(),
        hand_picked.join("\n"),
    );
}

/// THE LAW (decision B): the three vendor stylesheets that L1 now owns are
/// **bit-identical** to the ones in `vendor/iced/widget/src/button.rs`, for every
/// iced theme and every status. The two DraftFlow schemes are the exception, and
/// they are not skipped: their `subtle` control answers to the design sheet
/// (panel2 at rest, `hover` under the pointer, `active` on the press, `disabled`
/// ink when disabled), so those eight styles are pinned against `Colors` by
/// whole-struct equality instead of against iced's palette pair.
///
/// This is an equality test, not a threshold test, because the promise being made
/// is "migrating these 77 call sites moves no pixel". A mean |ΔL*| under some
/// epsilon would still allow the plate, the ink and the corner to each move a
/// little; `button::Style` derives `PartialEq`, so the whole struct is compared
/// instead of a projection of it.
///
/// The risk this gates is a mirror that drifts silently. `kit::subtle_style` and
/// its two siblings were written by reading the vendor bodies, and the one place
/// where a copy can diverge without anyone noticing is the disabled arm: ours
/// routes through `kit::control::dim`, which scales by `tokens::INK_DISABLED`,
/// while iced scales by its own literal. Those agree only while both are 0.5.
/// If they ever part, the (e) ratchet would report the sites as migrated while the
/// pixels changed underneath the report.
///
/// THE CONTAINER ARM (decision E, opened 2026-10-17 16:1x). `kit::dialog_body_style`
/// replaces `container::rounded_box`, and that sheet has a field the button sheets do
/// not expose to this kind of mistake: `snap`. Every other container style in
/// `kit::surface` writes `snap: true`, because a tier wants crisp edges; the vendor's
/// `rounded_box` leaves it at `Style::default()`, whose `snap` is `renderer::CRISP` —
/// `cfg!(feature = "crisp")`, which this manifest does not enable, so `false`. A
/// hand-copied sheet that "looks right" in plate, ink, edge and radius would therefore
/// still repaint the site. The whole struct is compared, so that is caught here rather
/// than in a screenshot: `dialog_body_style` reaches the default with
/// `..container::Style::default()` and takes nothing into its own hands.
///
/// Counted separately from the button walk on purpose. One shared counter would let a
/// container bucket silently contribute zero comparisons while the product formula
/// `themes x sheets x statuses` still balances, which is exactly the empty-enumeration
/// hole the assert below exists to close.
#[test]
fn the_vendor_equivalents_are_exact() {
    const STATUSES: [button::Status; 4] = [
        button::Status::Active,
        button::Status::Hovered,
        button::Status::Pressed,
        button::Status::Disabled,
    ];

    type Sheet = fn(&iced::Theme, button::Status) -> button::Style;

    let pairs: [(&str, Sheet, Sheet); 3] = [
        ("button::subtle", button::subtle, subtle_style),
        ("button::text", button::text, text_style),
        ("button::danger", button::danger, danger_style),
    ];

    let themes = all_themes();
    let mut mismatches: Vec<String> = Vec::new();
    let mut compared = 0usize;
    let mut sheet_pinned = 0usize;

    for theme in &themes {
        for (name, vendor, ours) in pairs {
            for status in STATUSES {
                compared += 1;
                // The design sheet is not a vendor reproduction. DraftFlow's subtle
                // control walks its own plate ladder because iced maps `Weakest` and
                // `Weaker` onto the same rung, so a vendor-faithful copy would have no
                // hover response at all. The whole struct is still compared, just
                // against the sheet's named pixels.
                if name == "button::subtle" {
                    if let Some(colors) = draftflow_colors(theme) {
                        let (plate, ink) = match status {
                            button::Status::Active => (Some(colors.panel2), colors.ink),
                            button::Status::Hovered => (Some(colors.hover), colors.ink),
                            button::Status::Pressed => (Some(colors.active), colors.ink),
                            button::Status::Disabled => (Some(colors.panel2), colors.disabled),
                        };
                        let want = button::Style {
                            background: plate.map(iced::Background::Color),
                            text_color: ink,
                            border: iced::border::rounded(tokens::RADIUS_CONTROL),
                            ..button::Style::default()
                        };
                        assert_eq!(
                            ours(theme, status),
                            want,
                            "{theme:?} / {status:?}: the DraftFlow subtle control left the \
                             sample's plate ladder (panel2 / hover / active / disabled); that \
                             ladder is part of the design sheet, not iced's palette pair",
                        );
                        sheet_pinned += 1;
                        continue;
                    }
                }
                let from_vendor = vendor(theme, status);
                let from_kit = ours(theme, status);
                if from_vendor != from_kit {
                    mismatches.push(format!(
                        "{name} / {theme:?} / {status:?}\n    vendor: {from_vendor:?}\n    \
                         kit:    {from_kit:?}"
                    ));
                }
            }
        }
    }

    // A walk over an empty enumeration would satisfy every assertion below, so the
    // comparison count is checked against the product of the three loops before the
    // result is trusted — and the theme count is floored, because `all_themes()`
    // silently returning only the custom set would halve the coverage being claimed.
    assert!(
        themes.len() >= 20,
        "all_themes() returned {} theme(s); decision B claims coverage of the whole \
         selectable set and cannot do that on this few",
        themes.len(),
    );
    assert_eq!(
        compared,
        themes.len() * pairs.len() * STATUSES.len(),
        "the button equality walk compared {compared} style(s) instead of themes x sheets x statuses",
    );

    // The design-sheet arm is a second enumeration, so it gets its own count gate:
    // an empty DraftFlow set would silently turn the exemption into a no-op.
    let sheet_themes = themes
        .iter()
        .filter(|theme| draftflow_colors(theme).is_some())
        .count();
    assert!(
        sheet_themes >= 2,
        "all_themes() yielded {sheet_themes} DraftFlow scheme(s); both shipped schemes are \
         expected in the selectable set",
    );
    assert_eq!(
        sheet_pinned,
        sheet_themes * STATUSES.len(),
        "the design-sheet arm pinned {sheet_pinned} subtle style(s) instead of {sheet_themes} \
         DraftFlow theme(s) x {} statuses",
        STATUSES.len(),
    );

    assert!(
        mismatches.is_empty(),
        "{} kit stylesheet(s) are not bit-identical to the vendor ones they replace \
         (decision B claimed zero pixels moved):\n{}",
        mismatches.len(),
        mismatches.join("\n"),
    );

    println!(
        "KIT-CONTRACT: decision B bit-identity: {} kit sheet(s) == vendor over {} themes x {} \
         statuses = {compared} comparisons ({sheet_pinned} pinned to the DraftFlow sheet), 0 mismatch",
        pairs.len(),
        themes.len(),
        STATUSES.len(),
    );

    // The container arm: no statuses, because `container::Style` is not a stateful
    // sheet. Same rule as above — the count is pinned against the product before the
    // emptiness verdict is believed.
    type CSheet = fn(&iced::Theme) -> container::Style;

    let cpairs: [(&str, CSheet, CSheet); 1] = [(
        "container::rounded_box",
        container::rounded_box,
        dialog_body_style,
    )];

    let mut cmismatches: Vec<String> = Vec::new();
    let mut ccompared = 0usize;

    for theme in &themes {
        for (name, vendor, ours) in cpairs {
            ccompared += 1;
            let from_vendor = vendor(theme);
            let from_kit = ours(theme);
            if from_vendor != from_kit {
                cmismatches.push(format!(
                    "{name} / {theme:?}\n    vendor: {from_vendor:?}\n    kit:    {from_kit:?}"
                ));
            }
        }
    }

    assert_eq!(
        ccompared,
        themes.len() * cpairs.len(),
        "the container equality walk compared {ccompared} style(s) instead of themes x sheets",
    );

    assert!(
        cmismatches.is_empty(),
        "{} kit container sheet(s) are not bit-identical to the vendor one they replace \
         (decision E claimed zero pixels moved; `snap`, `shadow`, plate, ink, edge and \
         radius are all compared):\n{}",
        cmismatches.len(),
        cmismatches.join("\n"),
    );

    println!(
        "KIT-CONTRACT: decision E bit-identity: {} kit container sheet(s) == vendor over {} \
         themes = {ccompared} comparisons, 0 mismatch",
        cpairs.len(),
        themes.len(),
    );
}

/// THE PRICING SHEET BEHIND DECISION A (opened 2026-10-17 14:5x, decided 15:1x).
///
/// The ladder now exists, and the pixels it must not move are asserted by
/// [`the_control_plate_ladder_is_exact`]. What this test keeps doing is the part
/// that is still an open question, because form 2 was NOT chosen and the tier
/// direction defect was NOT fixed:
///
/// WHY IT EXISTED BEFORE ANY CODE. Decision A says "L0 is missing a control-plate
/// ladder", and the obvious fix would be new `SURFACE_LADDER`-shaped rungs that
/// `Role::Standard`/`pill`/`chip` read instead of `palette.background.{weakest,
/// weaker, weak, strong}`. Whether that costs a pixel is not an opinion: it is
/// determined by what those slots actually *are* in L\* terms. If a slot sits
/// a near-constant distance above or below the page across all 26 themes, a rung
/// can be defined at that distance and A is a rename. If the distances scatter
/// (iced's palettes are hand-tuned per theme), no rung reproduces them and A is a
/// redesign that moves pixels on every neutral button in the app — which is
/// precisely the answer that has to be bought with a sample shot under #48.
///
/// Measured answer, kept printing so the two rejected routes stay visible: the
/// existing surface ladder cannot host a control plate (21.57 / 30.84 L\* on the
/// `weak`->Card and `strong`->Float mappings, and 3 themes whose ladder runs the
/// *other way* from their own palette), so A went form 1 — L0 owns the call, iced
/// keeps the arithmetic. Form 2 (one perceptual step per slot for every theme) is
/// the route #48 still has to accept or drop, and it is priced per slot below.
///
/// So this prints and asserts only that the measurement is complete:
/// (1) each slot's ΔL\* from its own theme's page, per theme, (2) how many of the
/// 26x8 slots a *currently existing* L0 rung already reproduces bit-exactly, via
/// the same `Source::of` the (e) ledger uses (which checks `Ladder` first, so a
/// coincidental match is credited to the ladder), and (3) whether the slots
/// are monotone at all, because a ladder by definition is.
///
/// NOTHING HERE ASSERTS A THRESHOLD, and that is still correct: the numbers below
/// are design costs, not laws. A probe that pretended to be a rule would be the
/// same mistake as a `flat at` line reporting a measurement that never happened.
/// The `palette::Background` field names, DERIVED from the vendored iced source.
///
/// WHY THIS IS DERIVED AND NOT TYPED. The first version of decision A's probe
/// hand-typed four names (`weakest`, `weaker`, `weak`, `strong`) and the vendored
/// struct has eight: it missed `neutral`, `stronger` and `strongest` — and
/// `strongest` is not hypothetical, `src/ui/kit/surface.rs`'s scrim already paints
/// it (multi-line chain, which is also why the grep that sized A's face missed it).
/// A count column whose name list was copied by hand is the same defect already
/// caught three times on this ratchet: (e)'s surface, (f)'s one padding spelling,
/// (d)'s mixed dimensions. So the list comes from the struct.
fn vendor_background_slots() -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/iced/core/src/theme/palette.rs");
    let src = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("A's slot surface needs {}: {e}", path.display()));
    let open = src
        .find("pub struct Background {")
        .expect("`pub struct Background {` vanished from the vendored iced palette");
    let body = &src[open..src[open..]
        .find("\n}")
        .expect("unterminated `Background` struct")
        + open];
    let mut out: Vec<String> = Vec::new();
    for line in body.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("pub ") {
            if let Some(field) = rest.strip_suffix(": Pair,") {
                out.push(field.trim().to_string());
            }
        }
    }
    out
}

/// Resolve a derived slot name to its plate colour. The `panic` arm is the point:
/// if the vendored struct grows a field, this fails loudly instead of the probe
/// quietly measuring one fewer pixel family (the `(g)` `.text_size` blind spot).
fn slot_plate(theme: &iced::Theme, name: &str) -> Color {
    let bg = &theme.palette().background;
    match name {
        "base" => bg.base.color,
        "weakest" => bg.weakest.color,
        "weaker" => bg.weaker.color,
        "weak" => bg.weak.color,
        "neutral" => bg.neutral.color,
        "strong" => bg.strong.color,
        "stronger" => bg.stronger.color,
        "strongest" => bg.strongest.color,
        other => panic!(
            "the vendored `Background` exposes a slot `{other}` this resolver does not know; \
             add it rather than letting decision A's measurement go narrower"
        ),
    }
}

fn slot_ink(theme: &iced::Theme, name: &str) -> Color {
    let bg = &theme.palette().background;
    match name {
        "base" => bg.base.text,
        "weakest" => bg.weakest.text,
        "weaker" => bg.weaker.text,
        "weak" => bg.weak.text,
        "neutral" => bg.neutral.text,
        "strong" => bg.strong.text,
        "stronger" => bg.stronger.text,
        "strongest" => bg.strongest.text,
        other => panic!("unresolved slot `{other}`"),
    }
}

/// The `deviate` amount behind each `Background` slot, DERIVED from vendor source.
///
/// WHY THIS IS DERIVED TOO. Decision A has two candidate forms, and the cheaper
/// one is "L0 owns the arithmetic but keeps iced's arithmetic": `tokens::plate`
/// calls `iced::theme::palette::deviate(page, k)` with `k` from this table. That
/// is a rename only if every theme's background pairs actually came out of
/// `Background::new` — a hand-authored `Palette` would make the "rename" move a
/// pixel. The only way to know is to re-run the derivation and compare, which
/// needs the `k` values, and they belong to the vendor. So they are parsed, not
/// typed, for the same reason the slot names are.
fn vendor_deviate_steps() -> Vec<(String, f32)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/iced/core/src/theme/palette.rs");
    let src = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("A's replica check needs {}: {e}", path.display()));
    let open = src
        .find("pub fn new(base: Color, text: Color) -> Self {")
        .expect("`Background::new` no longer spells its signature the way A's replica check expects");
    let body = &src[open..src[open..]
        .find("\n    }")
        .expect("unterminated `Background::new`")
        + open];
    let mut out: Vec<(String, f32)> = Vec::new();
    for line in body.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("let ") {
            if let Some(eq) = rest.find(" = deviate(base, ") {
                let name = rest[..eq].trim().to_string();
                let tail = &rest[eq + " = deviate(base, ".len()..];
                if let Some(close) = tail.find(')') {
                    if let Ok(k) = tail[..close].trim().parse::<f32>() {
                        out.push((name, k));
                    }
                }
            }
        }
    }
    out
}

#[test]
fn decision_a_priced_the_two_forms_of_a_control_plate_ladder() {
    let themes = all_themes();
    assert!(
        themes.len() >= 20,
        "decision A's pricing sheet must see the whole theme set, got {}",
        themes.len(),
    );

    let names = vendor_background_slots();
    let steps = vendor_deviate_steps();
    assert!(
        steps.len() >= 6,
        "derived only {} `deviate` steps from `Background::new` — the parser is broken, and \
         A's replica-vs-perceptual choice cannot be made without them (got {steps:?})",
        steps.len(),
    );
    assert!(
        names.len() >= 6,
        "derived only {} `Background` slots from vendor/iced/core/src/theme/palette.rs — \
         the parser is broken, and a short list is how A gets decided on an incomplete \
         measurement (first run of this probe typed 4 of 8)",
        names.len(),
    );
    assert_eq!(
        names.first().map(String::as_str),
        Some("base"),
        "the ramp monotonicity below skips index 0 on the assumption that the derived order \
         starts at `base`; got {names:?} — re-derive rather than skip the wrong slot",
    );

    let mut magnitude: Vec<Vec<f32>> = names.iter().map(|_| Vec::new()).collect();
    let mut nearest_l0: Vec<Vec<f32>> = names.iter().map(|_| Vec::new()).collect();
    let mut exact_ladder = vec![0usize; names.len()];
    let mut non_monotone: Vec<String> = Vec::new();
    let mut ink_variants: Vec<String> = Vec::new();
    let mut off_side: Vec<String> = Vec::new();
    let mut exact_replica = vec![0usize; names.len()];
    let mut replica_cost: Vec<Vec<f32>> = names.iter().map(|_| Vec::new()).collect();
    let mut exact_ink_replica = vec![0usize; names.len()];
    let mut ink_replica_cost: Vec<Vec<f32>> = names.iter().map(|_| Vec::new()).collect();
    let mut measured = 0usize;

    for theme in themes.iter() {
        let base_l = common::lstar(theme.palette().background.base.color);
        // Every pixel L0 itself can hand out for this theme, expressed as its
        // SIGNED distance from the page: the base, the two recesses, and the four
        // raised rungs. `Source::of` asks whether a slot is *equal* to one of
        // these; A needs the weaker question — how far the nearest one is —
        // because a rung that is 0.3 L\* off is a rename, and only bit-equality
        // is a rename under decision B's guard.
        //
        // SIGNED, AND WHY THAT MATTERS. The first version of this probe compared
        // |dL\*| against |dL\*|. It reported `strong` a median 0.32 L\* from an
        // existing rung and looked like a near-rename, while its own candidate
        // mapping printed a max move of 30.84 L\* in the same run. Both numbers
        // were true: `tokens::surface` picks its direction from the page's lift
        // headroom (`Up` when `100 - page >= MIN_LIFT_HEADROOM`, else `Down`) and
        // then compresses by `room/top`, whereas iced's `deviate` always moves the
        // same way by a fixed mix. On a theme that flips the ladder's direction,
        // a +17.4 slot and a -17.4 rung are 34.8 L\* apart but 0.0 in magnitude
        // terms. Comparing magnitudes hid exactly the themes A would break, so the
        // metric now compares the offsets themselves.
        let ladder_colors: Vec<Color> = vec![
            theme.palette().background.base.color,
            tokens::inset(theme),
            tokens::sunk(theme),
        ]
        .into_iter()
        .chain(RANKS.iter().map(|rank| tokens::surface(theme, *rank)))
        .collect();
        let ladder: Vec<f32> = ladder_colors
            .iter()
            .map(|c| common::lstar(*c) - base_l)
            .collect();
        let raised: Vec<f32> = ladder[3..].to_vec();

        let mut row: Vec<String> = Vec::new();
        let mut mags: Vec<f32> = Vec::new();
        let mut theme_off_side = 0usize;
        for (i, name) in names.iter().enumerate() {
            let plate = slot_plate(theme, name);
            let d = common::lstar(plate) - base_l;
            assert!(d.is_finite(), "slot `{name}` gave a non-finite L* delta");
            let gap = ladder
                .iter()
                .map(|l| (l - d).abs())
                .fold(f32::INFINITY, f32::min);
            if *name != "base" && !raised.iter().any(|l| l.signum() == d.signum()) {
                theme_off_side += 1;
            }
            // A's candidate form 1: L0 owns the call but not the arithmetic, i.e.
            // `tokens::plate(theme, Step)` re-runs `deviate(page, k)`. Then the plate
            // moves only on a theme whose pairs were NOT produced by that call, so
            // count the exact hits and measure the cost of the misses.
            if let Some((_, k)) = steps.iter().find(|(s, _)| s == name) {
                let replica =
                    iced::theme::palette::deviate(theme.palette().background.base.color, *k);
                if replica == plate {
                    exact_replica[i] += 1;
                }
                replica_cost[i].push((common::lstar(replica) - common::lstar(plate)).abs());
                // The ink is the other half of a `Pair`, and `Pair::new` does not
                // store the text it was handed — it stores `readable(color, text)`,
                // which is why 20/26 themes show MORE than one ink across the eight
                // slots. So an L0 plate ladder can only own a whole `Pair` if the
                // derivation reproduces the ink from what L0 can actually see, i.e.
                // the palette's own `base.text`. If it does not, A owns the plate
                // and the ink stays where it is; that is a measured boundary, not a
                // convenience.
                let replica_pair = iced::theme::palette::Pair::new(
                    replica,
                    theme.palette().background.base.text,
                );
                if replica_pair.text == slot_ink(theme, name) {
                    exact_ink_replica[i] += 1;
                }
                ink_replica_cost[i].push(
                    (common::lstar(replica_pair.text) - common::lstar(slot_ink(theme, name))).abs(),
                );
            }
            mags.push(d.abs());
            magnitude[i].push(d.abs());
            nearest_l0[i].push(gap);
            measured += 1;
            if Source::of(theme, plate) == Source::Ladder {
                exact_ladder[i] += 1;
            }
            row.push(format!("{name} {d:+.2}~{gap:.2}"));
        }
        if theme_off_side > 0 {
            off_side.push(format!(
                "{theme}: {theme_off_side} slot(s) sit on a side no raised rung of this theme reaches"
            ));
        }

        // Vendor order is base, weakest, weaker, weak, neutral, strong, stronger,
        // strongest: the ramp must be ascending in |d| or no ladder can name it.
        let ramp = &mags[1..];
        if !ramp.windows(2).all(|w| w[0] <= w[1] + 1e-3) {
            non_monotone.push(format!("{theme}: {}", row.join(" ")));
        }
        // Ink coupling: `Background::new` gives every slot the SAME text colour, so
        // a plate rung can move without moving the ink. A hand-tuned palette may not.
        let mut inks: Vec<String> = names
            .iter()
            .map(|n| format!("{:?}", slot_ink(theme, n)))
            .collect();
        inks.sort();
        inks.dedup();
        if inks.len() > 1 {
            ink_variants.push(format!("{theme}: {} distinct ink(s)", inks.len()));
        }
        println!(
            "KIT-CONTRACT:   A probe {theme:<22} iced[{}] | L0 rungs[{}]",
            row.join(" "),
            raised
                .iter()
                .map(|l| format!("{l:+.2}"))
                .collect::<Vec<_>>()
                .join(" "),
        );
    }

    assert_eq!(
        measured,
        themes.len() * names.len(),
        "the probe measured {measured} slot(s) instead of themes x derived slots",
    );

    for (i, name) in names.iter().enumerate() {
        let mut m = magnitude[i].clone();
        let mut g = nearest_l0[i].clone();
        m.sort_by(|a, b| a.partial_cmp(b).expect("finite magnitudes were asserted above"));
        g.sort_by(|a, b| a.partial_cmp(b).expect("finite gaps were asserted above"));
        assert_eq!(m.len(), themes.len(), "slot `{name}` is under-measured");
        println!(
            "KIT-CONTRACT:   A slot `{name:<9}` |dL*| min {:.2} median {:.2} max {:.2} \
             spread {:.2} | distance to the NEAREST existing L0 pixel (SIGNED offsets, so a \
             rung on the wrong side of the page is charged the full round trip): min {:.2} \
             median {:.2} max {:.2} | bit-exact on the ladder: {}/{}",
            m[0],
            m[m.len() / 2],
            m[m.len() - 1],
            m[m.len() - 1] - m[0],
            g[0],
            g[g.len() / 2],
            g[g.len() - 1],
            exact_ladder[i],
            m.len(),
        );

        // A's two candidate forms, priced by this same measurement.
        if *name != "base" {
            let rc = &replica_cost[i];
            if !rc.is_empty() {
                let worst = rc.iter().fold(0f32, |acc, v| acc.max(*v));
                let ic = &ink_replica_cost[i];
                let worst_ink = ic.iter().fold(0f32, |acc, v| acc.max(*v));
                println!(
                    "KIT-CONTRACT:   A form-1 `{name:<9}` L0 re-derives with iced's own `deviate`: \
                     plate bit-exact on {}/{} themes (worst miss {worst:.2} L*), ink re-derived \
                     from base.text bit-exact on {}/{} (worst miss {worst_ink:.2} L*) \
                     (a theme that misses is one whose pairs were NOT produced by `Background::new`)",
                    exact_replica[i],
                    rc.len(),
                    exact_ink_replica[i],
                    ic.len(),
                );
            }
            let median = m[m.len() / 2];
            let worst = m.iter().fold(0f32, |acc, v| acc.max((v - median).abs()));
            let over_half = m.iter().filter(|v| (**v - median).abs() > 0.5).count();
            println!(
                "KIT-CONTRACT:   A form-2 `{name:<9}` one perceptual step of {median:.2} L* for every \
                 theme (direction still taken from the theme's own deviate): worst move \
                 {worst:.2} L*, themes moved more than 0.50: {over_half}/{}",
                m.len(),
            );
        }
    }

    // The two gaps decision A would actually spend, on the candidate mapping
    // "rest reads the Card rung, hover/press read the Float rung". These were
    // already signed (`lstar(a) - lstar(b)`, then `abs`), which is why they exposed
    // the 21.57 / 30.84 outliers that the magnitude-based nearest-L0 column of the
    // first run reported as 1.40 and 0.32.
    let card = RANKS[2];
    let float = RANKS[3];
    let mut rest_gap: Vec<(String, f32)> = Vec::new();
    let mut press_gap: Vec<(String, f32)> = Vec::new();
    for theme in themes.iter() {
        rest_gap.push((
            theme.to_string(),
            (common::lstar(slot_plate(theme, "weak"))
                - common::lstar(tokens::surface(theme, card)))
                .abs(),
        ));
        press_gap.push((
            theme.to_string(),
            (common::lstar(slot_plate(theme, "strong"))
                - common::lstar(tokens::surface(theme, float)))
                .abs(),
        ));
    }
    let describe = |mut v: Vec<(String, f32)>| {
        let mut plain: Vec<f32> = v.iter().map(|(_, g)| *g).collect();
        plain.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("finite gaps"));
        let worst: Vec<String> = v
            .iter()
            .take(3)
            .map(|(name, g)| format!("{name} {g:.2}"))
            .collect();
        format!(
            "min {:.2} median {:.2} max {:.2} | worst: {}",
            plain[0],
            plain[plain.len() / 2],
            plain[plain.len() - 1],
            worst.join(", "),
        )
    };
    println!(
        "KIT-CONTRACT:   A candidate mapping cost over {} themes: rest `weak`->{} dL* {}, \
         press `strong`->{} dL* {} (0.00 would mean A is a rename; anything above ~0.5 is a \
         visible plate move and must be bought by #48's sample shots)",
        themes.len(),
        // `NAMES` is the point-free label table for this exact sequence: index
        // `rung() + 1`, because `NAMES[0]` is the page the rungs are measured from.
        NAMES[card.rung() + 1],
        describe(rest_gap),
        NAMES[float.rung() + 1],
        describe(press_gap),
    );


    println!(
        "KIT-CONTRACT:   A ramp monotonicity: {} of {} themes do NOT order the derived slots by \
         |dL*| ascending; a ladder is by definition monotone, so every one of those is a slot no \
         rung can represent without moving a pixel",
        non_monotone.len(),
        themes.len(),
    );
    for line in &non_monotone {
        println!("KIT-CONTRACT:     non-monotone {line}");
    }
    println!(
        "KIT-CONTRACT:   A side reachability: {} of {} themes have at least one slot on a side \
         that NONE of that theme's own raised rungs reaches — for those, A cannot re-point a role \
         at a rung at all, because the ladder and iced's `deviate` disagree about which way a \
         plate goes (see the `L0 rungs[...]` field of each probe line above)",
        off_side.len(),
        themes.len(),
    );
    for line in &off_side {
        println!("KIT-CONTRACT:     off-side {line}");
    }
    println!(
        "KIT-CONTRACT:   A ink coupling: {} of {} themes give their background slots MORE than one \
         text colour, so for those a plate rung swap can move the ink too (whereas \
         `Background::new` shares one ink across all eight slots)",
        ink_variants.len(),
        themes.len(),
    );
    for line in &ink_variants {
        println!("KIT-CONTRACT:     {line}");
    }
}

/// THE LAW (decision A, form 1, 2026-10-17): L0's control-plate ladder reproduces
/// iced's `palette.background` plates **bit for bit**, for every selectable theme.
///
/// WHY AN EQUALITY TEST AND NOT A THRESHOLD. Decision A was approved on the claim
/// that it moves no pixel: the kit stops spelling `palette.background.{weakest,
/// weak, strong, strongest}` and asks [`tokens::Control`] for a named rung instead.
/// That claim is only true while [`tokens::control_plate`] and the vendor's own
/// `deviate(page, k)` agree exactly, so the gate compares pixels. A tolerance would
/// let the plate, and with it every neutral button in the app, drift a little on
/// every theme while the ratchet still reported the sites as migrated — the same
/// failure decision B guarded against with `the_vendor_equivalents_are_exact`.
///
/// THREE PINS, BECAUSE ONE LOOKS ENOUGH AND ISN'T:
/// 1. The transcribed step must be the vendor's step for that slot *and* sit at the
///    same position in the vendor's ramp, so a re-ordered or re-typed
///    [`tokens::CONTROL_STEPS` cannot survive.
/// 2. Every plate pixel, 26 themes x 7 rungs.
/// 3. Every ink pixel. `tokens::control_ink` *reads* the slot's ink rather than
///    re-solving it, and a match arm that points at the wrong slot would otherwise
///    be invisible: it is bit-exact by construction only if the arm is right.
///
/// The ink is deliberately NOT re-derived here, because that is a measured limit and
/// not an oversight: re-running `Pair::new(deviate(page, k), base.text)` is exact on
/// only 22-25 of the 26 themes and misses by up to 10.12 L\* (printed below), since
/// `Pair::new` stores the *corrected* text and the original input is not recoverable
/// from a built `Palette`. Owning the ink needs L0's own contrast solver in front of
/// every control, which is #73, not this guard.
#[test]
fn the_control_plate_ladder_is_exact() {
    let themes = all_themes();
    assert!(
        themes.len() >= 20,
        "the control-plate guard has to see the whole selectable set, got {}",
        themes.len(),
    );
    assert_eq!(
        CONTROL_RUNGS.len(),
        tokens::CONTROL_STEPS.len(),
        "CONTROL_RUNGS lists {} rung(s) but tokens::CONTROL_STEPS has {}: a variant was added \
         to `tokens::Control` and this guard silently stopped measuring it",
        CONTROL_RUNGS.len(),
        tokens::CONTROL_STEPS.len(),
    );

    let names = vendor_background_slots();
    let steps = vendor_deviate_steps();
    assert!(
        names.len() >= 6 && steps.len() >= 6,
        "derived {} `Background` slot(s) and {} `deviate` step(s) from \
         vendor/iced/core/src/theme/palette.rs — a short list is how this guard ends up \
         pinning fewer pixels than it claims",
        names.len(),
        steps.len(),
    );

    // Pin 1: the transcription, slot by slot.
    for rung in CONTROL_RUNGS {
        let slot = rung.vendor_slot();
        assert!(
            names.iter().any(|n| n == slot),
            "`tokens::Control::{rung:?}` claims the vendored slot `{slot}`, which is not in the \
             struct derived from palette.rs: {names:?}",
        );
        let position = steps
            .iter()
            .position(|(s, _)| s == slot)
            .unwrap_or_else(|| panic!("`Background::new` no longer derives a plate for `{slot}`"));
        assert_eq!(
            position,
            rung.step(),
            "`tokens::Control::{rung:?}` indexes step {} but the vendor ramp puts `{slot}` at \
             {position}: the ramp order in CONTROL_STEPS is now wrong, so every rung below would \
             be compared against the neighbour's pixel",
            rung.step(),
        );
        let vendor_k = steps[position].1;
        assert!(
            (tokens::CONTROL_STEPS[rung.step()] - vendor_k).abs() <= 1e-6,
            "`tokens::CONTROL_STEPS[{}] = {} but iced's `deviate` uses {vendor_k} for `{slot}` \
             ({rung:?}) — L0 would be naming an amount it does not spend",
            rung.step(),
            tokens::CONTROL_STEPS[rung.step()],
        );
    }

    let mut compared = 0usize;
    let mut ink_cells = 0usize;
    let mut plate_failures: Vec<String> = Vec::new();
    let mut ink_failures: Vec<String> = Vec::new();
    let mut ink_miss = 0usize;
    let mut ink_miss_worst = 0f32;

    for theme in themes.iter() {
        for rung in CONTROL_RUNGS {
            let slot = rung.vendor_slot();
            // Pin 2: the plate.
            let plate = tokens::control_plate(theme, rung);
            let vendor = slot_plate(theme, slot);
            compared += 1;
            if plate != vendor {
                plate_failures.push(format!(
                    "{theme}: `{slot}` is {:?} (L* {:.3}) in the palette, but \
                     control_plate({rung:?}) gives {:?} (L* {:.3})",
                    vendor,
                    common::lstar(vendor),
                    plate,
                    common::lstar(plate),
                ));
            }
            // Pin 3: the ink is the slot's own, read through L0.
            ink_cells += 1;
            if tokens::control_ink(theme, rung) != slot_ink(theme, slot) {
                ink_failures.push(format!(
                    "{theme}: control_ink({rung:?}) no longer reads the ink of `{slot}` — the arm \
                     points at the wrong rung",
                ));
            }
            // The limit, printed rather than asserted: what it would cost to OWN the
            // ink instead of reading it.
            let re_solved = iced::theme::palette::Pair::new(plate, theme.palette().background.base.text).text;
            if re_solved != slot_ink(theme, slot) {
                ink_miss += 1;
                ink_miss_worst = ink_miss_worst
                    .max((common::lstar(re_solved) - common::lstar(slot_ink(theme, slot))).abs());
            }
        }
    }

    // Print first, verdict second: with the asserts above the prints, a single
    // drifted theme hides the count of how many drifted and the ink boundary that
    // #73 needs in order to decide.
    println!(
        "KIT-CONTRACT: decision A bit-identity: control_plate == palette.background.<slot>.color \
         over {} themes x {} rungs = {compared} cells, {} mismatch ({} of the {} derived slots \
         are not rungs: `base`, the page the ladder is measured from)",
        themes.len(),
        CONTROL_RUNGS.len(),
        plate_failures.len(),
        names.len() - CONTROL_RUNGS.len(),
        names.len(),
    );
    println!(
        "KIT-CONTRACT:   A ink boundary: control_ink reads the slot's ink on {}/{} cells, {} \
         mismatch; re-solving it as Pair::new(control_plate, base.text) would differ on \
         {ink_miss}/{ink_cells} cells, worst {ink_miss_worst:.2} L* — which is why A owns the \
         plate and #73 owns the ink",
        ink_cells - ink_failures.len(),
        ink_cells,
        ink_failures.len(),
    );

    assert!(
        plate_failures.is_empty(),
        "{} control plate(s) drifted from iced's palette pair, so A would move pixels while the \
         ratchet reported a rename:\n{}",
        plate_failures.len(),
        plate_failures.join("\n"),
    );
    assert!(
        ink_failures.is_empty(),
        "{} control ink read(s) no longer return the ink of the slot they name:\n{}",
        ink_failures.len(),
        ink_failures.join("\n"),
    );
    assert_eq!(
        compared,
        themes.len() * CONTROL_RUNGS.len(),
        "the plate loop ran {compared} times instead of themes x rungs — a `continue` or an \
         early return has gutted the guard, which is exactly how a `flat at` line comes to \
         report a measurement that never happened",
    );
    assert_eq!(
        ink_cells, compared,
        "the ink arm ran {ink_cells} times against {compared} plates; plate and ink must be \
         measured over the same cells",
    );
}

#[test]
fn one_screen_label_is_kept_in_this_doc() {
    // Two ordinal systems naming one object is a measurement defect, not a wording
    // preference: the old chain label and `screen 21` pointed at the same rebuilt
    // screen from different files, and a reader who lined the two up got a row
    // count that was neither. One label per thing now — `Screen N` off the order of
    // `REBUILT_SCREENS`, `run` for a working turn, `X -> Y drop` for a decrease
    // between re-freezes, `F2 reading` for one value of the ratio series. This
    // guard reads its own source text, so a retired spelling cannot come back as
    // documentation the next time someone writes a chain.
    const RETIRED_ORDINALS: [&str; 23] = [
        "first",
        "second",
        "third",
        "fourth",
        "fifth",
        "sixth",
        "seventh",
        "eighth",
        "ninth",
        "tenth",
        "eleventh",
        "twelfth",
        "thirteenth",
        "fourteenth",
        "fifteenth",
        "sixteenth",
        "seventeenth",
        "eighteenth",
        "nineteenth",
        "twentieth",
        "twenty-first",
        "twenty-second",
        "twenty-third",
    ];
    let files = [
        (
            "src/ui/style/kit_contract_tests.rs",
            include_str!("kit_contract_tests.rs"),
        ),
        ("src/ui/style/mod.rs", include_str!("mod.rs")),
        ("src/ui/style/common.rs", include_str!("common.rs")),
    ];
    // Built by concatenation, because a guard that spells the strings it forbids
    // fails on its own source.
    let retired_words = [format!("le{}", "g"), format!("le{}", "gs")];
    let retired_point = format!("f2{}point", " ");
    let mut problems: Vec<String> = Vec::new();
    let mut headers: Vec<u32> = Vec::new();

    for (file, doc) in files {
        for (idx, line) in doc.lines().enumerate() {
            for word in line.split(|c: char| !c.is_alphanumeric()) {
                if retired_words.iter().any(|w| word.eq_ignore_ascii_case(w)) {
                    problems.push(format!("{file}:{}: retired chain word `{word}`", idx + 1));
                }
            }
            let lower = line.to_lowercase();
            for digit in '0'..='9' {
                if lower.contains(&format!("the screen {digit}")) {
                    problems.push(format!("{file}:{}: article before a screen number", idx + 1));
                }
            }
            for ordinal in RETIRED_ORDINALS {
                if lower.contains(&format!("{ordinal} (20")) {
                    problems.push(format!(
                        "{file}:{}: `{ordinal}` used as a screen number",
                        idx + 1
                    ));
                }
            }
            if lower.contains(retired_point.as_str()) {
                problems.push(format!(
                    "{file}:{}: a ratio reading called by the retired name",
                    idx + 1
                ));
            }
            if let Some(rest) = line.trim_start().strip_prefix("// Screen ") {
                let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
                if !digits.is_empty() && rest[digits.len()..].starts_with(" (") {
                    let n: u32 = digits
                        .parse()
                        .expect("a header screen number should parse");
                    if headers.contains(&n) {
                        problems.push(format!("{file}:{}: duplicate `Screen {n}` header", idx + 1));
                    }
                    headers.push(n);
                }
            }
        }
    }

    // The parser has to be alive or the loop above is a tautology — the failure
    // mode this file already had once with `let divergent = true`.
    assert!(
        headers.len() >= 5,
        "only {} `Screen N (date)` headers were found; the scan is not reading the ledger, \
         so this guard would pass on any text",
        headers.len(),
    );
    assert!(
        problems.is_empty(),
        "this contract doc carries a second label for one object:\n{}",
        problems.join("\n"),
    );
}

/// The plot dialog's five unpickable ticks (Center the plot, Fit to paper,
/// Scale lineweights, Plot with plot styles, Display plot styles) all call
/// [`crate::ui::kit::control::checkbox_style`]. Before it they wore iced's
/// `primary`, whose disabled arm dims the plate but leaves the tick in the
/// theme accent and the caption at full ink, so a row nobody can press read as
/// the brightest one in the panel. Plate and border stay the vendor's, which
/// means the only pixels that move are those two inks; every enabled row must
/// stay byte-identical to `primary`.
#[test]
fn the_unpickable_tick_loses_both_inks() {
    use crate::ui::kit::control::checkbox_style;
    use crate::ui::style::tokens;
    use iced::widget::checkbox;

    let themes = crate::app::config::all_themes();
    assert!(
        themes.len() >= 26,
        "only {} selectable themes; the scan is not reading the theme list",
        themes.len(),
    );

    let mut checked = 0usize;
    for theme in &themes {
        for is_checked in [false, true] {
            let disabled = checkbox::Status::Disabled { is_checked };
            let vendor = checkbox::primary(theme, disabled);
            let kit = checkbox_style(theme, disabled);

            assert_eq!(
                kit.icon_color.a,
                tokens::INK_DISABLED,
                "{theme:?} checked={is_checked}: tick alpha {} is not INK_DISABLED",
                kit.icon_color.a,
            );
            let caption = kit.text_color.unwrap_or_else(|| {
                panic!("{theme:?} checked={is_checked}: caption ink is None, i.e. full strength")
            });
            assert_eq!(
                caption.a,
                tokens::INK_DISABLED,
                "{theme:?} checked={is_checked}: caption alpha {} is not INK_DISABLED",
                caption.a,
            );
            assert_eq!(
                kit.background, vendor.background,
                "{theme:?} checked={is_checked}: the disabled plate moved",
            );
            assert_eq!(
                kit.border, vendor.border,
                "{theme:?} checked={is_checked}: the disabled border moved",
            );

            for status in [
                checkbox::Status::Active { is_checked },
                checkbox::Status::Hovered { is_checked },
            ] {
                assert_eq!(
                    checkbox_style(theme, status),
                    checkbox::primary(theme, status),
                    "{theme:?} checked={is_checked}: an enabled row moved",
                );
            }
            checked += 1;
        }
    }
    println!(
        "QC74A checked {checked} unpickable ticks across {} themes",
        themes.len(),
    );
    assert_eq!(checked, themes.len() * 2);
}

// ── The glass rollout: the four doors the ruling closed behind it ─────────
//
// A pane is a shader, so none of the twelve ratchet columns can see it: the
// ratchet counts literals a screen types, and a screen that went to glass types
// nothing at all. That makes the usual ceiling guards blind here in both
// directions — a *new* opaque floating plate would not raise any column, and a
// pane quietly replaced by a plate would lower one and look like a win. These
// four guards are the parts of the ruling that the columns cannot express:
// (1) the `Float` rung has no surface left to paint outside the kit, (2) the one
// exception is the browser-only tip and is named, (3) the glass census is a
// floor, (4) the plate that glass replaced stays deleted.
//
// (3) is deliberately a floor rather than a ceiling: the rollout is a one-way
// door, so a count may only rise. (1), (2) and (4) are ceilings at their measured
// value for the same reason — each is a claim that something is *gone*, and the
// only way a claim like that stays true is by being re-measured every run.

/// The needle for a pane call site. `kit::glass_pane(` and a bare `glass_pane(`
/// both qualify; the definition (`pub fn glass_pane<`) and the re-export
/// (`glass::glass_pane;`) do not, and both live in the whitelisted kit anyway.
fn glass_site(line: &str) -> bool {
    line.contains("glass_pane(")
}

/// The needle for the opaque floating rung: the path-qualified form only, so the
/// ladder's own `Tier::Float.style(` spelling never reads as a call.
fn float_call(line: &str) -> bool {
    line.contains("::float_style(")
}

/// (1) No surface outside the kit asks the ladder for the `Float` rung any more.
///
/// Measured after the rollout: **0** rows. `ui/kit/` owns the rung and the tier
/// table, `ui/style/theme_accessibility_tests.rs` sweeps every rung of that table
/// for contrast (a ladder test, not a plate), and this file quotes the name. The
/// remaining two `Tier::Float` rows in `src` are the AA sweep's own list, so they
/// are on the whitelist and are *not* counted here.
///
/// The reason this is a door and not tidiness: quads draw before primitives inside
/// a layer, so a floating panel that keeps an opaque `Float` fill paints over the
/// pane sitting under it. Every one of these rows would be a surface that looks
/// like glass and is actually a plate.
#[test]
fn no_surface_outside_the_kit_asks_for_the_float_rung() {
    let root = src_root();
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    assert!(
        files.len() >= MIN_SCANNED_FILES,
        "glass census scanned only {} files; the src path is wrong and every count below is 0 by accident",
        files.len()
    );
    let mut offenders = Vec::new();
    for path in files {
        let rel = match path.strip_prefix(&root) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        if is_whitelisted(&rel) {
            continue;
        }
        let Ok(text) = fs::read_to_string(path) else { continue };
        for (i, line) in text.lines().enumerate() {
            let t = line.trim_start();
            if t.starts_with("//") {
                continue;
            }
            if t.contains("Tier::Float") {
                offenders.push(format!("{rel}:{}", i + 1));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "REGRESSION: {} row(s) outside the kit name the floating rung again: {offenders:?}. \
         A floating surface is `kit::glass_pane(plate.into(), theme, raised, \"\")`; \
         if a surface genuinely cannot be a pane, it must be written down in the kit's \
         `float_style` doc and added here by name, not quietly plated.",
        offenders.len()
    );
}

/// A fade that ends on a sheet has to end on the sheet's own colour.
///
/// The chip strip's two scrims are gradients into the card they sit on. The
/// colour is the kit's to hand out ([`crate::ui::kit::sheet_ground`]) because the
/// census above forbids a view naming the floating rung; this is the other half
/// of that door — the name must still resolve to the plate the sheet paints, or
/// the kit would be handing out a guess under an honest name.
#[test]
fn the_sheet_ground_is_the_colour_the_sheet_paints() {
    let themes = all_themes();
    assert!(!themes.is_empty(), "all_themes() returned no theme to measure");
    for theme in themes.iter() {
        let painted = read_container(crate::ui::kit::sheet_style(theme, "")).plate;
        assert_eq!(
            painted,
            Some(crate::ui::kit::sheet_ground(theme)),
            "{theme}: the named sheet ground is not the plate `sheet_style` paints"
        );
    }
}

/// The strip asks the kit for its ground instead of guessing one.
///
/// `theme.palette().background.base.color` would pass the rung census and still
/// be wrong: the card is a rung above the page, so a fade into the page colour
/// ends in a visible grey bar. The needle is the helper's name at the call site.
#[test]
fn the_chip_strip_fades_into_the_kits_sheet_ground() {
    let source = include_str!("../command_line.rs");
    let start = source
        .find("fn chip_strip<'a>(")
        .expect("the chip strip");
    let body = &source[start..];
    let end = body
        .find("\n/// Command names matching")
        .expect("the end of the chip strip");
    let body = &body[..end];

    assert!(
        body.contains("kit::sheet_ground("),
        "the chip strip's scrim no longer asks the kit for the card's ground"
    );
    assert!(
        !body.contains("background.base.color"),
        "the chip strip's scrim fell back to the page colour, which is a rung below the card"
    );
}

/// (2) The single exception stays single, and stays behind the browser gate.
///
/// Ceiling at **1**. The one row that may name `kit::float_style` outside the kit
/// is the "add a folder" tip in `ui/window/annotation_data.rs` — a surface that
/// exists only in `wasm32`, in the one view of the wizard that has no other
/// floating surface, so nothing in it ever gets a `&Theme` to hand a pane. The
/// argument is on the call site and in `float_style`'s own "Where it is left
/// standing" section; what is *asserted* here is the two facts the argument rests
/// on: that there is exactly one such row, in that file, and that the nearest `#[cfg(`
/// attribute above it is the wasm gate rather than a `not(wasm32)` one — i.e. the
/// exception really is the browser-only tip and not a desktop surface that picked
/// up the old recipe.
///
/// A second row here is the signal that a floating surface got painted opaque
/// again; a row whose gate reads `not(wasm32)` is the signal that the exception
/// moved somewhere the shipped build draws.
#[test]
fn the_opaque_floating_plate_has_only_the_browser_tip_left() {
    let root = src_root();
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    let mut users: Vec<(String, usize)> = Vec::new();
    let mut ungated: Vec<String> = Vec::new();
    for path in files {
        let rel = match path.strip_prefix(&root) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        if is_whitelisted(&rel) {
            continue;
        }
        let Ok(text) = fs::read_to_string(path) else { continue };
        // `text` has to outlive `lines`: the rows are borrows of it, and the gate
        // lookup below walks the file backwards from a call site.
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim_start();
            if t.starts_with("//") || !float_call(t) {
                continue;
            }
            users.push((rel.clone(), i + 1));
            let gate = lines[..i]
                .iter()
                .rev()
                .map(|l| l.trim_start())
                .find(|l| l.contains("#[cfg("));
            match gate {
                // `not(target_arch = "wasm32")` contains the gate's text as a
                // substring, so the negation is excluded by name and not by order.
                Some(g) if g.contains("target_arch = \"wasm32\"") && !g.contains("not(") => {}
                other => ungated.push(format!(
                    "{rel}:{} nearest cfg = {other:?}",
                    i + 1
                )),
            }
        }
    }
    assert_eq!(
        ungated,
        Vec::<String>::new(),
        "the remaining opaque-plate user is not behind the wasm gate, so the desktop build is drawing a plate that reads as glass: {ungated:?}"
    );
    assert!(
        users.len() <= 1,
        "REGRESSION: {} rows outside the kit call the opaque floating plate: {users:?}. \
         The ruling allows exactly one, the browser-only tip in ui/window/annotation_data.rs; \
         a new floating surface takes `kit::glass_pane`.",
        users.len()
    );
    if users.is_empty() {
        println!(
            "float_style has 0 call sites outside the kit — the browser tip went to glass too; \
             delete `float_style` and its re-export, retire the `Float` rung, and lower this ceiling to 0 by removing it."
        );
    } else {
        println!("  the one opaque floating plate left in the app: {}:{}", users[0].0, users[0].1);
    }
}

/// (3) The glass census: 44 pane sites across 20 files, and the per-family floors
/// that let a reader see *which* surfaces the ruling reaches.
///
/// A floor, because the rollout only ever moves a surface from plate to pane — with
/// one deliberate exception, recorded here so the next reader does not read the lower
/// number as drift. 2026-11-02: the ribbon dropdowns, the colour flyout and the
/// viewport context menu were moved from glass to `kit::popup_plate` (the `Float`
/// rung wearing the field's edge). Glass over a drawing is what made those panels
/// read as transparent: their rows were legible only through the backdrop, and the
/// user rejected it twice on the running build. The 11 panes retired by that ruling
/// are gone: `ui/ribbon/mod.rs` 7 → 0, `app/view/overlay.rs` 4 → 3, and the two
/// `draw_panel` / `color_dropdown` panes (which sat in `ui/ribbon/` too). Glass keeps
/// every dialog, palette and dock surface; the popups are opaque sheets by design.
///
/// 2026-11-09: the file count went 21 → 20 for the opposite reason — a pane was
/// *centralized*, not plated over. `ui/window/layers.rs` built its own name tip
/// (`name_tip`, one `glass_pane` site); the long-text ruling moved every cut label
/// onto `text_util::elided` → `kit::tip::full`, so that pane now lives once in
/// `ui/kit/tip.rs`. The kit is whitelisted here because it is the layer that owns
/// paint, so the census cannot see it — the guard that owns it moved with it, to
/// `tip.rs::full_builds_a_tooltip_on_glass`, which fails if the shared tip ever
/// stops being glass. The site total is unchanged at 44: layers gave up 1 and the
/// command line's autocomplete card added 1.
///
/// The per-file minimums are re-frozen at the same measurement, so a surface can be
/// *added* anywhere but none of the existing families can be thinned out without
/// failing the guard that owns it.
///
/// A glass menu needs no backdrop override. `iced_aw`'s `context_menu::primary` —
/// the class the theme resolves when a site names no style — fills its own quad with
/// `a: 0f32`, which is the f32 literal zero rather than the hex-looking `0.32`, and
/// the vendored test `primary_theme_has_transparent_background` asserts `color.a == 0.0`.
/// The `Style::default()` of `[0.87, 0.87, 0.87, 0.30]` is never consulted, because
/// `Catalog::default()` hands out `primary`. So nothing square sits under a pane, and
/// pinning a pairing for it would be a floor over a layer that does not exist.
const GLASS_SITE_FLOOR: usize = 44;
const GLASS_FILE_FLOOR: usize = 20;
const GLASS_FAMILY_FLOORS: [(&str, usize); 9] = [
    ("ui/ribbon/mod.rs", 0),
    ("ui/window/layers.rs", 0),
    ("ui/window/graphic_attributes.rs", 4),
    ("ui/properties.rs", 4),
    ("ui/command_line.rs", 5),
    ("app/view/overlay.rs", 3),
    ("app/view/mod.rs", 4),
    ("ui/window/xref_manager.rs", 3),
    ("ui/statusbar/mod.rs", 3),
];

#[test]
fn the_glass_rollout_keeps_its_measured_floor() {
    let root = src_root();
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    let mut per_file: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut sites = 0usize;
    for path in files {
        let rel = match path.strip_prefix(&root) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        if is_whitelisted(&rel) {
            continue;
        }
        let Ok(text) = fs::read_to_string(path) else { continue };
        for line in text.lines() {
            let t = line.trim_start();
            if t.starts_with("//") || !glass_site(t) {
                continue;
            }
            sites += 1;
            *per_file.entry(rel.clone()).or_insert(0usize) += 1;
        }
    }
    assert!(
        sites >= GLASS_SITE_FLOOR,
        "REGRESSION: the glass census fell to {sites} sites; the floor is {GLASS_SITE_FLOOR}. \
         A pane that went back to an opaque plate is not a saving — the ratchet cannot see it, \
         which is why this number is a floor and not a ceiling.",
    );
    assert!(
        per_file.len() >= GLASS_FILE_FLOOR,
        "REGRESSION: only {} files carry a pane; the floor is {GLASS_FILE_FLOOR}. \
         The site count can hold while a whole surface family leaves glass — check the split.",
        per_file.len()
    );
    for (family, floor) in GLASS_FAMILY_FLOORS {
        let got = per_file.get(family).copied().unwrap_or(0);
        assert!(
            got >= floor,
            "REGRESSION: {family} fell to {got} pane sites; the measured floor is {floor}, \
             so a floating surface in this family was plated over.",
        );
    }
    if sites > GLASS_SITE_FLOOR {
        println!(
            "GLASS FLOOR CAN BE RAISED TO {sites} sites / {} files — book the win: the rollout reached a new surface, so raise GLASS_SITE_FLOOR, GLASS_FILE_FLOOR and the owning row of GLASS_FAMILY_FLOORS together.",
            per_file.len()
        );
    }
    println!(
        "glass census: {sites} pane sites across {} files (floor {GLASS_SITE_FLOOR} / {GLASS_FILE_FLOOR})",
        per_file.len()
    );
}

/// (4) The plate glass replaced stays deleted.
///
/// A name the kit no longer exports cannot be called — but it can be re-added by
/// someone who reads a stale doc, a patch, or a test message, and the tree would
/// then have two ways to say "this panel is the object of attention", one of them
/// opaque. The needle is built from pieces because this file's own text is scanned
/// and must not trip the guard it defines.
#[test]
fn the_brand_hairline_plate_is_not_renamed_back_into_the_tree() {
    let needle = ["selected", "panel", "style"].join("_");
    let root = src_root();
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    let mut rows = Vec::new();
    for path in files {
        let rel = match path.strip_prefix(&root) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        let Ok(text) = fs::read_to_string(path) else { continue };
        for (i, line) in text.lines().enumerate() {
            let t = line.trim_start();
            if t.starts_with("//") {
                continue;
            }
            if t.contains(&needle) {
                rows.push(format!("{rel}:{}", i + 1));
            }
        }
    }
    assert!(
        rows.is_empty(),
        "REGRESSION: the deleted brand-hairline plate is back at {rows:?}. The surface it named \
         is a glass pane now (`kit::glass_pane`); `guard_panel_style` is the only bordered plate \
         the kit exports, and it is an alert, not a selection.",
    );
}


