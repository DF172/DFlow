//! The kit: the app's widget primitives, and the only authority for what a
//! control or a plane looks like.
//!
//! Four layers, and dependencies only ever point down:
//!
//! ```text
//! L0  style::tokens   constants + pure colour functions, no widgets
//! L1  ui::kit         stateless primitives, generic over the message type
//! L2  ui::shell       region geometry: who is above whom, and how tall
//! L3  screens         ui/**, app/view/**, app/dock/view.rs — compose, never style
//! ```
//!
//! L3 is forbidden from writing a `container::Style` / `button::Style` /
//! `text_input::Style` literal, a numeric `radius`, or a numeric `spacing`.
//! That is not a style guideline; it is enforced by
//! `style::kit_contract_tests`, which counts the violations in the tree and
//! fails when the number *grows*. The count at the audit that opened this work
//! was 361 / 56 / 1022 / 1336 (`D:\tmp\kit-baseline.csv`, 2026-10-15); the
//! baselines are re-frozen downward after every migration wave, and the live
//! ratchet count is measured with the awk one-liner recorded in
//! `D:\tmp\kit-migration-progress.csv` rather than taken from a report.
//!
//! Two rules keep this layer from becoming the thing it replaced:
//!   * A primitive is merged only with at least one real screen calling it.
//!     An unused kit function is API drift, not architecture.
//!   * Where an older shared helper already had locked behaviour
//!     (`style::form`, and `style::common::muted_style` until its 36th-freeze
//!     deletion), the implementation moves *here* and the old path becomes a
//!     forwarding stub on its way out, never a second, subtly different authority.

pub mod alpha;
pub mod control;
pub mod empty;
pub mod feedback;
pub mod glass;
pub mod label;
pub mod plain;
pub mod press;
pub mod ruled;
pub mod stack;
pub mod surface;
pub mod tip;

pub use plain::{apply_site, plain_button};
pub use press::sink;

pub use control::{
    bare_field_style, brand_mark, btn, button_style, checkbox_style, danger_style, chip,
    state_pill,
    chip_button_style,
    dialog_button,
    dialog_button_styled, dialog_button_styled_opt, editor_style, error_field_style,
    field_container_style, field_style,
    form_radio, ghost_style, glyph_cell_style, icon_btn, joined_btn, labeled_checkbox, list_row,
    list_row_at,
    locked_style, menu_row_style, pick_list_style, picker_field_style,
    primary_style,
    read_only_style, readout_style, role_style, scrollable_style, secondary_style,
    signal_button_style, static_checkbox, style_for, subtle_style, tab_plate_style, text_style,
    themed_pick_list, themed_scrollable, tool_plate_style, Join, Press, Role, Size,
};
pub use glass::glass_pane;
pub use label::{
    accent_style, body, caption, chord_ink_style, color_style, display, faint, faint_style, group,
    grounds, heading, key, kit_planes, muted, muted_style, resolve_on, section_label, tone_ink_on,
    tone_pair, tone_style, tone_style_on, tone_swatch, Tone,
};
pub use ruled::{
    accent_under, rule_over, rule_over_at, rule_under, rule_under_at, rule_under_inset,
    rule_under_inset_at, Edge as RuleEdge, Ruled, Tone as RuleTone,
    RULE as RULE_THICKNESS,
};
pub use stack::{
    form_row, form_row_flush, hgroup, labeled_field, labeled_field_compact, labeled_field_enabled,
    labeled_pick_list, labeled_pick_list_enabled, vgroup, vstack, LABEL_WIDTH,
};
pub use surface::{
    accent_plate_style, brand_rule, canvas_glyphs, card_plate_style, chrome_plate_style,
    dialog_body_plate,
    dialog_body_style, document_plane, document_style, field_radius_plate_style,
    fieldset_well_style, fade, float_style, frame_style, ghost_sheet_style, ground_style,
    group_box, guard_panel_style, hdivider, heading_plate_style, indicator,
    indicator_style, inset_card_style,
    mark_plate_style, marker_ring_style, node_plate_style, option_row_style, page, page_style,
    paper_style,
    pane_plate_style, picker_tile_border,
    picker_tile_colors, picker_tile_style, picker_trigger_style, pill,
    plane,
    pill_style, plain_plate_style, popup_plate, port_dot_style, preview_well_style, ring_style, rule_h, rule_v,
    row_selection_plate_style, scrim_style, seam_style, selected_row_wash_style, section_head,
    status_dot_style,
    sheet_chip_style, sheet_flat_style, sheet_ground, sheet_strip_style,
    sheet_rule, sheet_rule_v, sheet_style,
    strip, strip_style,
    stripe_style, surface, swatch_button_style, swatch_edit_ink, swatch_edit_style,
    swatch_plate_style, swatch_style, tick, tint, tint_style, tone_band_plate,
    tone_plate_style_on, vsep, vseparator, wash, wash_style, CanvasBadge, CanvasGlyphs, Tint, Tier,
    PAD_BADGE, PAD_BAND, PAD_CARD, PAD_CELL, PAD_FIELD, PAD_GLYPH, PAD_HERO, PAD_MENU_ROW,
    PAD_PLATE, PAD_PROGRESS_CARD, PAD_TAB, PAD_TRIGGER, PAD_WELL,
};
