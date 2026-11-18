//! Properties panel — DraftFlowCAD-style editable object properties.
//!
//! Shows two sections (General + Geometry) for the selected entity.
//! • Layer      → combo_box  (options from document layer table)
//! • Color      → inline color picker  (ByLayer / ByBlock / ACI palette)
//! • Lineweight → combo_box  (standard CAD lineweight list)
//! • Linetype   → read-only for now
//! • Geometry   → text_input per coordinate / dimension field

use crate::ui::style::tokens;
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use std::{fmt, sync::Arc};

use crate::t;
use crate::ui::ROW_H;
use codec::types::{Color as AcadColor, LineWeight};
use codec::Handle;
use iced::widget::{canvas, column, combo_box, container, mouse_area, row, text, text_input, tooltip};
use crate::ui::kit::plain_button;
use iced::widget::text::{Ellipsis, Wrapping};
use iced::{
    mouse, Color, Element, Length, Padding, Point, Rectangle, Size, Theme,
};

// ── Row-height-derived constants ─────────────────────────────────────────
// The row's type comes from the ladder, not from a fraction of the row: at
// ROW_H * 0.42 the panel shipped 10.9 px, which the owner read as too small on
// 2026-11-02. `TEXT_LABEL` (13) is the dense-table rung; the combo's vertical
// inset is derived from it so the control still fills the 26 px row.
const FONT_SZ: f32 = tokens::TEXT_LABEL;
const COMBO_PAD_V: f32 = (ROW_H - FONT_SZ * 1.3 - 2.0) / 2.0; // fills combo to ROW_H
const PATTERN_CARD_W: f32 = 158.0;
const PATTERN_PREVIEW_H: f32 = 58.0;
// `pub(crate)` because the Graphic Attributes palette opens the same picker as
// a flyout and has to size its drop-down from the picker's own box; the old
// literals there (352 / 724) were this box plus the 2 px inset of a second
// plate that no longer exists.
pub(crate) const PATTERN_PICKER_W: f32 = 348.0;
pub(crate) const PATTERN_PICKER_H: f32 = 720.0;
const LINETYPE_MENU_W: f32 = 220.0;
use crate::app::Message;
use crate::scene::model::object::{PropSection, PropValue};
use crate::ui::kit;

const VARIES_LABEL: &str = "*VARIES*";

// ── Linetype item (name + ASCII art for combo_box) ───────────────────────

#[derive(Clone, PartialEq, Debug)]
pub struct LinetypeItem {
    pub name: String,
    pub art: String,
}

pub fn linetype_display_name(name: &str) -> String {
    if name.is_empty() || name.eq_ignore_ascii_case("ByLayer") {
        crate::t!("ByLayer").into_owned()
    } else if name.eq_ignore_ascii_case("ByBlock") {
        crate::t!("ByBlock").into_owned()
    } else {
        name.to_string()
    }
}

impl fmt::Display for LinetypeItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.name.is_empty() && !self.art.is_empty() {
            return f.write_str(&self.art);
        }
        let name = linetype_display_name(&self.name);
        if self.art.is_empty() {
            write!(f, "{name}")
        } else {
            write!(f, "{name}  {}", self.art)
        }
    }
}

// ── Lineweight wrapper (needs ToString for combo_box) ─────────────────────

#[derive(Clone, PartialEq, Debug)]
pub struct LwItem(pub LineWeight);

impl fmt::Display for LwItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            LineWeight::ByLayer => write!(f, "{}", crate::t!("ByLayer")),
            LineWeight::ByBlock => write!(f, "{}", crate::t!("ByBlock")),
            LineWeight::Default => write!(f, "{}", crate::t!("Default")),
            LineWeight::Value(v) => write!(f, "{:.2} mm", v as f64 / 100.0),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct SelectionGroup {
    pub label: String,
    pub handles: Vec<Handle>,
}

impl fmt::Display for SelectionGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[derive(Clone)]
struct HatchPatternPreview {
    pattern: crate::scene::model::hatch_model::HatchPattern,
    padding: f32,
}

impl canvas::Program<Message> for HatchPatternPreview {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        use crate::scene::model::hatch_model::{HatchModel, HatchPattern};

        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let palette = theme.palette();
        let pad = self.padding;
        let sample = canvas::Path::rectangle(
            Point::new(pad, pad),
            Size::new(
                (bounds.width - pad * 2.0).max(0.0),
                (bounds.height - pad * 2.0).max(0.0),
            ),
        );
        frame.fill(&sample, palette.background.base.color);

        match &self.pattern {
            HatchPattern::Solid => {
                frame.fill(&sample, palette.background.base.text.scale_alpha(0.72));
            }
            HatchPattern::Gradient { .. } => {
                frame.fill(&sample, palette.primary.weak.color);
            }
            HatchPattern::Pattern(_) => {
                let model = HatchModel {
                    pattern_origin: None,
                    render_instance: None,
                    world_origin: [0.0, 0.0],
                    boundary: Arc::new(vec![
                        [pad, pad],
                        [bounds.width - pad, pad],
                        [bounds.width - pad, bounds.height - pad],
                        [pad, bounds.height - pad],
                    ]),
                    boundary_wcs: None,
                    fill_plane: None,
                    fill_plane_boundary: None,
                    boundary_exterior: None,
                    boundary_sources: None,
                    boundary_paths: None,
                    style: codec::entities::HatchStyleType::Normal,
                    pattern: self.pattern.clone(),
                    name: String::new(),
                    color: [1.0; 4],
                    aci: 0,
                    line_weight_px: 1.0,
                    angle_offset: 0.0,
                    scale: hatch_preview_scale(&self.pattern),
                    draw_depth: 0.0,
                };
                let stroke = canvas::Stroke::default()
                    .with_color(palette.background.base.text)
                    .with_width(1.0);
                for segment in model.pattern_segments() {
                    frame.stroke(
                        &canvas::Path::line(
                            Point::new(segment[0][0] as f32, bounds.height - segment[0][1] as f32),
                            Point::new(segment[1][0] as f32, bounds.height - segment[1][1] as f32),
                        ),
                        stroke.clone(),
                    );
                }
            }
        }

        frame.stroke(
            &sample,
            canvas::Stroke::default()
                .with_color(tokens::edge(theme))
                .with_width(1.0),
        );
        vec![frame.into_geometry()]
    }
}

fn hatch_preview_scale(pattern: &crate::scene::model::hatch_model::HatchPattern) -> f32 {
    use crate::scene::model::hatch_model::HatchPattern;

    let HatchPattern::Pattern(families) = pattern else {
        return 1.0;
    };
    let spacing = families
        .iter()
        .filter_map(|family| {
            let spacing = family.dy.abs();
            (spacing > 1.0e-4).then_some(spacing)
        })
        .fold(f32::INFINITY, f32::min);
    if spacing.is_finite() {
        (8.0 / spacing).clamp(0.01, 100.0)
    } else {
        1.0
    }
}

fn hatch_pattern_preview(
    pattern: crate::scene::model::hatch_model::HatchPattern,
    height: f32,
) -> Element<'static, Message> {
    canvas(HatchPatternPreview {
        pattern,
        padding: 4.0,
    })
    .width(Length::Fill)
    .height(height)
    .into()
}

pub(crate) fn compact_hatch_pattern_preview(
    pattern: crate::scene::model::hatch_model::HatchPattern,
) -> Element<'static, Message> {
    canvas(HatchPatternPreview {
        pattern,
        padding: 0.5,
    })
    .width(Length::Fill)
    .height(ROW_H)
    .into()
}

fn hatch_pattern_matches(
    entry: &crate::scene::model::hatch_patterns::PatternEntry,
    search: &str,
) -> bool {
    let query = search.trim();
    query.is_empty()
        || entry.name.to_lowercase().contains(&query.to_lowercase())
        || entry
            .description
            .to_lowercase()
            .contains(&query.to_lowercase())
}

pub(crate) fn filtered_hatch_patterns(
    search: &str,
) -> Vec<&'static crate::scene::model::hatch_patterns::PatternEntry> {
    crate::scene::model::hatch_patterns::catalog()
        .iter()
        .filter(|entry| hatch_pattern_matches(entry, search))
        .collect()
}

/// Messages of a hatch pattern picker; the Properties panel and the Graphic
/// Attributes palette each route them to their own state.
pub(crate) struct PatternPickerMessages {
    pub search_id: &'static str,
    pub on_search: fn(String) -> Message,
    pub on_confirm: Message,
    pub on_focus: fn(usize) -> Message,
    pub on_changed: fn(String) -> Message,
}

pub(crate) fn hatch_pattern_picker_content<'a>(
    search_value: &'a str,
    focus: usize,
    current: &'a str,
    messages: PatternPickerMessages,
    theme: &Theme,
) -> Element<'a, Message> {
    let PatternPickerMessages {
        search_id,
        on_search,
        on_confirm,
        on_focus,
        on_changed,
    } = messages;
    let search = text_input(t!("Search patterns…").as_ref(), search_value)
        .id(iced::widget::Id::new(search_id))
        .on_input(on_search)
        .on_submit(on_confirm)
        .size(FONT_SZ)
        .padding(kit::PAD_FIELD)
        .width(Length::Fill);

    let mut grid = column![].spacing(tokens::GAP_CONTROL);
    let visible = filtered_hatch_patterns(search_value);
    for (row_index, pair) in visible.chunks(2).enumerate() {
        let mut cards = row![].spacing(tokens::GAP_CONTROL);
        for (column_index, entry) in pair.iter().enumerate() {
            let index = row_index * 2 + column_index;
            let selected = current.eq_ignore_ascii_case(&entry.name);
            let focused = focus == index;
            let name = entry.name.clone();
            let preview = hatch_pattern_preview(entry.gpu.clone(), PATTERN_PREVIEW_H);
            let card = plain_button(
                column![
                    preview,
                    container(crate::ui::text_util::elided(&entry.name, 20, theme, |shown| {
                        text(shown).size(FONT_SZ).into()
                    }))
                    .width(Length::Fill)
                    .align_x(iced::Center),
                ]
                .spacing(tokens::GAP_TIGHT),
            )
            .on_press(on_changed(name))
            .style(kit::chip_button_style(selected || focused))
            .padding(kit::Size::Compact.padding())
            .width(PATTERN_CARD_W);
            cards = cards.push(mouse_area(card).on_enter(on_focus(index)));
        }
        grid = grid.push(cards);
    }

    let results: Element<'_, Message> = if visible.is_empty() {
        container(
            text(t!("No matching patterns"))
                .size(FONT_SZ)
                .style(kit::faint_style),
        )
        .padding(tokens::GAP_SECTION)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
    } else {
        crate::ui::kit::themed_scrollable(grid)
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    };
    kit::glass_pane(
        container(column![search, results].spacing(tokens::GAP_ROW))
            // The picker pops off the panel and floats over the document, so it
            // reads the frame under it instead of painting an opaque plate.
            .padding(tokens::GAP_ROW)
            .width(PATTERN_PICKER_W)
            .height(Length::Fixed(PATTERN_PICKER_H))
            .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
            .into(),
        theme,
        false,
        "",
    )
}

/// All standard CAD lineweight options for the combobox.
pub fn lw_options() -> Vec<LwItem> {
    [
        LineWeight::ByLayer,
        LineWeight::ByBlock,
        LineWeight::Default,
        LineWeight::Value(0),
        LineWeight::Value(5),
        LineWeight::Value(9),
        LineWeight::Value(13),
        LineWeight::Value(15),
        LineWeight::Value(18),
        LineWeight::Value(20),
        LineWeight::Value(25),
        LineWeight::Value(30),
        LineWeight::Value(35),
        LineWeight::Value(40),
        LineWeight::Value(50),
        LineWeight::Value(53),
        LineWeight::Value(60),
        LineWeight::Value(70),
        LineWeight::Value(80),
        LineWeight::Value(90),
        LineWeight::Value(100),
        LineWeight::Value(106),
        LineWeight::Value(120),
        LineWeight::Value(140),
        LineWeight::Value(158),
        LineWeight::Value(200),
        LineWeight::Value(211),
    ]
    .iter()
    .copied()
    .map(LwItem)
    .collect()
}

/// Edit-buffer / active-field key identifying which value a row edits. Geometry
/// and common fields key on their `&'static str` field name; block attribute
/// rows key on the attribute's tag. One typed enum replaces the old
/// `\x01attr\x01…` sentinel prefix, so the two key namespaces can't collide by
/// construction.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FieldKey {
    /// A geometry / common editable value field, keyed by its field name.
    Geom(&'static str),
    /// A block attribute value field, keyed by its tag.
    Attr(String),
    /// One column of one named-parameter row, keyed by its stable
    /// `ParameterTable::iter()` position (see `PropValue::ParamRow`'s doc
    /// comment for why index rather than name).
    Param(usize, crate::ui::window::named_parameters::ParamField),
}

impl FieldKey {
    /// The text-input widget id backing this key (the shape used by the row
    /// renderer and the `PropSyncActive` handler). Geometry keys map straight
    /// onto their field id; attribute keys map onto the tag's field id.
    pub fn widget_id(&self) -> iced::widget::Id {
        match self {
            FieldKey::Geom(field) => prop_geom_field_id(field),
            FieldKey::Attr(tag) => prop_attr_field_id(tag),
            FieldKey::Param(index, field) => prop_param_field_id(*index, *field),
        }
    }
}

/// Text-input widget id for one column of a named-parameter row.
pub fn prop_param_field_id(
    index: usize,
    field: crate::ui::window::named_parameters::ParamField,
) -> iced::widget::Id {
    use crate::ui::window::named_parameters::ParamField;
    let suffix = match field {
        ParamField::Name => "name",
        ParamField::Formula => "formula",
    };
    iced::widget::Id::from(format!("props-param-field-{index}-{suffix}"))
}

/// Edit-buffer / active-field key for a block attribute value, keyed by its
/// tag. Kept in one place so the live-input handler and the row renderer agree.
pub fn attr_edit_key(tag: &str) -> FieldKey {
    FieldKey::Attr(tag.to_string())
}

/// Text-input widget id for a geometry/common editable value field. The same
/// id (built from the `&'static str` key) is used by the row renderer to tag
/// the input and by the update handler to focus / select it on click.
pub fn prop_geom_field_id(field: &str) -> iced::widget::Id {
    iced::widget::Id::from(format!("props-geom-field-{field}"))
}

/// Text-input widget id for a block attribute value field, keyed by the tag.
pub fn prop_attr_field_id(tag: &str) -> iced::widget::Id {
    iced::widget::Id::from(format!("props-attr-field-{tag}"))
}

/// Keeps the active-row highlight keyed to real keyboard focus: returns true
/// only while the currently-focused widget IS the text input of the active
/// field. Focus on any other widget (another property field, a non-property
/// widget, or nothing at all) clears the marker. Setting the key for a newly
/// focused property field is handled by the `PropSyncActive` handler before
/// this runs.
pub fn active_key_focused(
    active_key: Option<&FieldKey>,
    focused: Option<&iced::widget::Id>,
) -> bool {
    match (active_key, focused) {
        (Some(key), Some(focused_id)) => focused_id == &key.widget_id(),
        _ => false,
    }
}

/// Precomputes the focused-id → [`FieldKey`] map for every editable value row
/// in the given sections. Building it once when the panel's sections are
/// assembled lets a `PropSyncActive` event map a focused text-input id back to
/// its field key in O(1) instead of re-scanning the sections, enabling
/// select-all-on-focus for both text inputs and edit-choices (e.g. transparency).
pub fn build_field_key_map(sections: &[PropSection]) -> HashMap<iced::widget::Id, FieldKey> {
    let mut map = HashMap::default();
    for section in sections {
        for prop in &section.props {
            let key = match &prop.value {
                PropValue::EditText(_) | PropValue::PlainText(_) | PropValue::EditChoice { .. } => {
                    Some(FieldKey::Geom(prop.field))
                }
                PropValue::AttrText { tag, .. } => Some(FieldKey::Attr(tag.clone())),
                _ => None,
            };
            if let Some(key) = key {
                map.insert(key.widget_id(), key);
            }
            if let PropValue::ParamRow { index, .. } = &prop.value {
                use crate::ui::window::named_parameters::ParamField;
                for field in [ParamField::Name, ParamField::Formula] {
                    let key = FieldKey::Param(*index, field);
                    map.insert(key.widget_id(), key);
                }
            }
        }
    }
    map
}

/// Returns a task that sweeps the widget tree for the currently-focused widget
/// and reports its `Id` via [`Message::PropSyncActive`], which the update
/// handler uses to reconcile the active-row marker against real focus. Mirrors
/// the `WebFieldCopy` operation idiom: `focusable` fires for every focusable
/// widget, and we record the one that `is_focused()`.
pub fn sync_active_field_task() -> iced::Task<Message> {
    use iced::advanced::widget::operation::{Focusable, Outcome};
    use iced::advanced::widget::{Id, Operation};
    #[derive(Default)]
    struct FocusedId {
        focused: Option<Id>,
    }
    impl Operation<Option<Id>> for FocusedId {
        fn focusable(
            &mut self,
            id: Option<&Id>,
            _bounds: iced::Rectangle,
            state: &mut dyn Focusable,
        ) {
            if state.is_focused() && self.focused.is_none() {
                self.focused = id.cloned();
            }
        }
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Option<Id>>)) {
            operate(self);
        }
        fn finish(&self) -> Outcome<Option<Id>> {
            Outcome::Some(self.focused.clone())
        }
    }
    iced::advanced::widget::operate(FocusedId::default()).map(Message::PropSyncActive)
}

/// A translated choice label paired with the unchanged value stored in the
/// drawing and emitted by the properties panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalizedChoice {
    pub raw: String,
    label: String,
}

impl LocalizedChoice {
    pub fn new(raw: String) -> Self {
        let label = crate::i18n::translate(&raw).into_owned();
        Self { raw, label }
    }
}

impl fmt::Display for LocalizedChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

// ── PropertiesPanel ───────────────────────────────────────────────────────

#[derive(Clone)]
pub struct PropertiesPanel {
    pub sections: Vec<PropSection>,
    pub title: String,
    pub selection_groups: Vec<SelectionGroup>,
    pub selected_group: Option<SelectionGroup>,
    /// Linetype items (name + ASCII art) from the document — used for combo_box options.
    pub linetype_items: Vec<LinetypeItem>,
    pub selection_group_combo: combo_box::State<SelectionGroup>,
    pub choice_combos: HashMap<String, combo_box::State<LocalizedChoice>>,
    pub layer_combo: combo_box::State<String>,
    pub lineweight_combo: combo_box::State<LwItem>,
    pub linetype_combo: combo_box::State<LinetypeItem>,
    /// Whether the visual hatch-pattern picker is open.
    pub hatch_pattern_picker_open: bool,
    /// Case-insensitive filter for the visual hatch-pattern picker.
    pub hatch_pattern_search: String,
    /// Keyboard/hover focus inside the filtered visual pattern grid.
    pub hatch_pattern_focus: usize,
    /// Case-insensitive filter over the property rows shown in the panel.
    pub filter: String,
    /// In-progress text edits keyed by [`FieldKey`].
    pub edit_buf: HashMap<FieldKey, String>,
    /// Entity handles this panel was built for. `refresh_properties` compares
    /// it against the new selection to decide whether an uncommitted `edit_buf`
    /// may carry over: same selection → keep (survive a commit-triggered
    /// rebuild); different selection → drop, so a stale value can't display or
    /// commit onto a different entity (e.g. two title blocks sharing a `REV1`
    /// tag).
    pub source_handles: Vec<Handle>,
    /// Whether the quick color picker dropdown is open.
    pub color_picker_open: bool,
    /// Whether the MTEXT background-colour picker dropdown is open. Separate
    /// from `color_picker_open` so the entity colour and the background colour
    /// pickers are independent.
    pub bg_color_picker_open: bool,
    /// Field name of the generic per-field colour picker currently open (e.g. a
    /// hatch gradient colour), or `None`. Keeps each field's picker independent.
    pub open_color_field: Option<String>,
    /// Which vertex a multi-vertex entity (polyline) is focused on — driven by
    /// the Current Vertex ◀ / ▶ stepper. Reset to 0 when the selection changes.
    pub prop_vertex: usize,
    /// Draw the Current Vertex indicator only after the user changes the
    /// stepper for the current selection.
    pub prop_vertex_indicator_active: bool,
    /// Coordinate groups ("Position", "Scale", …) the user expanded into their
    /// component X/Y/Z rows. Collapsed by default; keyed `section:base` and
    /// carried across panel rebuilds so the state survives edits and selection
    /// changes.
    pub expanded_groups: HashSet<String>,
    /// Property sections the user collapsed. This is a rendered copy of the
    /// app-wide preference, allowing every open document panel to share it.
    /// Empty by default, meaning every section is initially shown.
    pub collapsed_sections: HashSet<String>,
    /// Whether the editable-dropdown (block Name) option list is open.
    pub edit_choice_open: bool,
    /// Field key of the value row currently being edited, or `None`. Marked when
    /// the user focuses an editable value field and cleared on commit / when the
    /// selection changes, so the active row can be highlighted. Geometry fields
    /// use [`FieldKey::Geom`]; attribute rows use [`FieldKey::Attr`].
    pub active_field: Option<FieldKey>,
    /// Focused-text-input id → [`FieldKey`] for the editable value rows in
    /// [`PropertiesPanel::sections`], precomputed by [`build_field_key_map`]
    /// when the panel is rebuilt. Gives the `PropSyncActive` handler an O(1)
    /// id→key lookup. Derived from `sections`: rebuild it alongside them.
    pub field_key_by_id: HashMap<iced::widget::Id, FieldKey>,
}

impl Default for PropertiesPanel {
    fn default() -> Self {
        Self {
            sections: vec![],
            title: String::new(),
            selection_groups: vec![],
            selected_group: None,
            linetype_items: vec![],
            selection_group_combo: combo_box::State::new(vec![]),
            choice_combos: HashMap::default(),
            layer_combo: combo_box::State::new(vec![]),
            lineweight_combo: combo_box::State::new(lw_options()),
            linetype_combo: combo_box::State::new(vec![]),
            hatch_pattern_picker_open: false,
            hatch_pattern_search: String::new(),
            hatch_pattern_focus: 0,
            filter: String::new(),
            edit_buf: HashMap::default(),
            source_handles: vec![],
            color_picker_open: false,
            bg_color_picker_open: false,
            open_color_field: None,
            prop_vertex: 0,
            prop_vertex_indicator_active: false,
            expanded_groups: HashSet::default(),
            collapsed_sections: HashSet::default(),
            edit_choice_open: false,
            active_field: None,
            field_key_by_id: HashMap::default(),
        }
    }
}

impl PropertiesPanel {
    pub fn empty() -> Self {
        Self {
            title: t!("No selection").into_owned(),
            ..Default::default()
        }
    }

    /// Maps a focused text-input widget id back to the [`FieldKey`] of that
    /// property row, if the id belongs to an editable value field currently
    /// shown. O(1) via the [`PropertiesPanel::field_key_by_id`] map.
    pub fn prop_field_key_for_id(&self, id: &iced::widget::Id) -> Option<FieldKey> {
        self.field_key_by_id.get(id).cloned()
    }

    pub fn selected_handles(&self) -> Vec<Handle> {
        self.selected_group
            .as_ref()
            .map(|group| group.handles.clone())
            .unwrap_or_default()
    }

    pub fn view(
        &self,
        width: f32,
        chrome: crate::ui::dock::Chrome,
        theme: &Theme,
    ) -> Element<'_, Message> {
        use crate::ui::dock::PanelId;
        // ── Header ──────────────────────────────────────────────────────────
        let header =
            crate::ui::dock::title_bar(PanelId::Properties, t!("Properties").into_owned(), chrome);

        // ── Title bar (entity type / "No Selection") ─────────────────────
        let title_content: Element<'_, Message> = if self.selection_groups.is_empty() {
            let full_title = t!(&self.title);
            crate::ui::text_util::elided(&full_title, 34, theme, |shown| {
                text(shown).size(FONT_SZ).style(kit::muted_style).into()
            })
        } else {
            combo_box(
                &self.selection_group_combo,
                "",
                self.selected_group.as_ref(),
                Message::PropSelectionGroupChanged,
            )
            .size(FONT_SZ)
            .padding(kit::PAD_FIELD)
            .input_style(kit::field_style)
            .on_open(Message::PropColorPickerClose)
            .width(Length::Fill)
            .into()
        };

        // The entity-type strip is the panel's head chrome: a full-width band,
        // squared because it meets the dock edge to edge.
        let title_bar = kit::surface(
            title_content,
            kit::Tier::Band,
            kit::PAD_BAND,
            "",
        )
        .width(Length::Fill);

        // ── Filter ──────────────────────────────────────────────────────────
        // The sheet puts a name filter at the head of the property card: the
        // list is long, and the user comes here looking for one row.
        let filter: Element<'_, Message> = if self.sections.is_empty() {
            container(column![]).height(0).into()
        } else {
            container(
                text_input(t!("Search").as_ref(), &self.filter)
                    .on_input(Message::PropFilterChanged)
                    .size(FONT_SZ)
                    .padding(kit::PAD_FIELD)
                    .style(kit::field_style)
                    .width(Length::Fill),
            )
            .width(Length::Fill)
            .padding(iced::Padding {
                top: tokens::GAP_CONTROL,
                right: tokens::GAP_CONTROL,
                bottom: tokens::GAP_HAIRLINE,
                left: tokens::GAP_CONTROL,
            })
            .into()
        };

        // ── Content ─────────────────────────────────────────────────────────
        let content: Element<'_, Message> = if self.sections.is_empty() {
            // The hint holds the sheet's slot instead of sitting as one line at
            // its head: the panel's frame, its title strip and its band do not
            // move when a selection arrives.
            kit::empty::state(
                crate::ui::motion::EMPTY_PROPS,
                crate::ui::icons::themed(crate::ui::icons::INFO, kit::empty::GLYPH),
                t!("Select an object to view properties"),
            )
        } else {
            let mut col = column![].spacing(tokens::GAP_NONE);
            let mut rows = crate::ui::dock::Rows::of(crate::ui::dock::PanelId::Properties);
            for section in &self.sections {
                col = col.push(self.render_section(section, theme, &mut rows));
            }
            crate::ui::kit::themed_scrollable(col.width(Length::Fill))
                .spacing(tokens::GAP_ROW)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        };

        // The pallet's Card wash (see `App::panel_body`) is this panel's plane:
        // the dock wears the frame's hairline on the strip band and the float
        // frame, so the body paints no second box over the surface it sits in.
        container(column![header, title_bar, filter, content])
            .width(Length::Fixed(width))
            .height(Length::Fill)
            .into()
    }

    /// Compact floating panel for Quick Properties: the title plus the same
    /// editable section rows as the docked panel, sized to its content.
    /// Returns `None` when nothing is selected.
    pub fn quick_view(&self, theme: &Theme) -> Option<Element<'_, Message>> {
        if self.source_handles.is_empty() || self.sections.is_empty() {
            return None;
        }
        let full_title = t!(&self.title);
        let title = kit::surface(
            crate::ui::text_util::elided(&full_title, 34, theme, |shown| {
                text(shown).size(FONT_SZ).style(kit::muted_style).into()
            }),
            kit::Tier::Band,
            kit::PAD_BAND,
            "",
        )
        .width(Length::Fill);

        let mut sections = column![].spacing(tokens::GAP_NONE);
        let mut rows = crate::ui::dock::Rows::of(crate::ui::dock::PanelId::Properties);
        for section in &self.sections {
            sections = sections.push(self.render_section(section, theme, &mut rows));
        }

        let content = crate::ui::kit::themed_scrollable(sections.width(Length::Fill))
            .spacing(tokens::GAP_ROW)
            .width(Length::Fill)
            .height(Length::Shrink);

        Some(kit::glass_pane(
            container(column![title, content].spacing(tokens::GAP_NONE))
                // Detached from the document and drawn over the canvas, so it
                // reads the drawing through its own pane instead of wearing a
                // solid plate with a shadow.
                .width(230)
                .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
                .into(),
            theme,
            false,
            "",
        ))
    }

    // ── Section renderer ──────────────────────────────────────────────────

    fn render_section<'a>(
        &'a self,
        section: &'a PropSection,
        theme: &Theme,
        rows: &mut crate::ui::dock::Rows,
    ) -> Element<'a, Message> {
        let needle = self.filter.trim().to_lowercase();
        let hit = |label: &str| needle.is_empty() || label.to_lowercase().contains(&needle);
        // A section with nothing matching the filter is not a section of the
        // answer: it takes no band and no space.
        if !needle.is_empty() && !section.props.iter().any(|p| hit(&p.label)) {
            return container(column![]).height(0).into();
        }
        let collapsed = self.collapsed_sections.contains(&section.title);
        let toggle = kit::icon_btn(
            if collapsed {
                crate::ui::icons::themed_arrow_right(10.0)
            } else {
                crate::ui::icons::themed_arrow_down(10.0)
            },
            kit::Role::Ghost,
            18.0,
            kit::Press::On(Message::PropSectionToggle(section.title.clone())),
            "",
        );

        // The arrow intentionally sits at the far end of the header. The rest
        // of the header responds to a double-click, leaving a single click on
        // the arrow as the precise, discoverable toggle target.
        let title = mouse_area(
            container(
                text(t!(&section.title).into_owned())
                    .size(tokens::TEXT_MICRO)
                    .font(tokens::UI_FONT_MEDIUM),
            )
            .width(Length::Fill)
            .padding(kit::PAD_BAND),
        )
        .on_double_click(Message::PropSectionToggle(section.title.clone()));
        // The group header is a wash *inside* the panel: the band fill carries
        // the separation, so it wears no second hairline on top of the panel's.
        let hdr = container(row![title, toggle,].align_y(iced::Center))
            .style(move |theme: &Theme| {
                kit::wash_style(kit::Tier::Band, theme, "")
            })
            .width(Length::Fill);

        let mut col = column![rows.wrap(|| hdr.into())].spacing(tokens::GAP_NONE);
        if collapsed && needle.is_empty() {
            return col.into();
        }

        // Consecutive "<Base> X / <Base> Y [/ <Base> Z]" text rows collapse
        // into one clickable summary row; clicking expands the components.
        let mut idx = 0;
        while idx < section.props.len() {
            let group_len = if section.title == "View" {
                0
            } else {
                coord_group_len(&section.props, idx)
            };
            if group_len >= 2
                && section.props[idx..idx + group_len]
                    .iter()
                    .any(|p| hit(&p.label))
            {
                let base = coord_base(&section.props[idx].label);
                let key = format!("{}:{}", section.title, base);
                let expanded = self.expanded_groups.contains(&key);
                let joined = section.props[idx..idx + group_len]
                    .iter()
                    .map(prop_text_value)
                    .collect::<Vec<_>>()
                    .join(", ");
                col = col.push(rows.wrap(|| render_group_row(base, key, expanded, joined, theme)));
                if expanded {
                    for prop in &section.props[idx..idx + group_len] {
                        col = col.push(
                            rows.wrap(|| self.render_prop_row(prop, coord_component(&prop.label), theme)),
                        );
                    }
                }
                idx += group_len;
            } else {
                let prop = &section.props[idx];
                if hit(&prop.label) {
                    col = col.push(rows.wrap(|| self.render_prop_row(prop, &prop.label, theme)));
                }
                idx += 1;
            }
        }

        col.into()
    }

    /// Render one property row with an explicit display label (the grouped
    /// coordinate rows shorten "Position X" to "X").
    fn render_prop_row<'a>(
        &'a self,
        prop: &'a crate::scene::model::object::Property,
        label: &'a str,
        theme: &Theme,
    ) -> Element<'a, Message> {
        match &prop.value {
            PropValue::ColorChoice(color) => {
                self.render_color_row(label, prop.field, *color, None, theme)
            }
            PropValue::NamedColorChoice { color, name } => {
                self.render_color_row(label, prop.field, *color, Some(name), theme)
            }
            PropValue::ColorVaries => self.render_color_varies_row(label, theme),
            PropValue::LayerChoice(layer) => self.render_layer_row(label, layer, theme),
            PropValue::LwChoice(lw) => self.render_lw_row(label, *lw, theme),
            PropValue::FieldLwChoice { field, value } => {
                self.render_field_lw_row(label, field, *value, theme)
            }
            PropValue::FieldLwVaries { field } => self.render_field_lw_varies_row(label, field, theme),
            PropValue::LwVaries => self.render_lw_varies_row(label, theme),
            PropValue::LinetypeChoice(lt) => self.render_linetype_row(label, lt, theme),
            PropValue::Choice { selected, options } => {
                self.render_choice_row(label, prop.field, selected, options, theme)
            }
            PropValue::EditChoice { value, options } => {
                self.render_edit_choice_row(label, prop.field, value, options, theme)
            }
            PropValue::BoolToggle { field, value } => render_bool_row(label, *field, *value, theme),
            PropValue::Stepper { display, .. } => render_stepper_row(label, display, theme),
            PropValue::EditText(val) | PropValue::PlainText(val) => {
                self.render_edit_row(label, prop.field, val, theme)
            }
            PropValue::Hyperlink(val) => render_hyperlink_row(label, val, theme),
            PropValue::ReadOnly(val) if prop.field == "annotative_scale" => {
                render_annotative_scale_row(label, val, theme)
            }
            PropValue::ReadOnly(val) => render_ro_row(label, val, theme),
            PropValue::ReadOnlyWithTooltip { value, tooltip } => {
                render_ro_with_tooltip_row(label, value, tooltip, theme)
            }
            PropValue::HatchPatternChoice(current) => self.render_hatch_pattern_row(label, current, theme),
            PropValue::AttrText { tag, value } => self.render_attr_row(tag, value, theme),
            PropValue::EntityLink {
                id,
                handles,
                conflicting,
            } => render_entity_link_row(label, *id, handles.clone(), *conflicting),
            PropValue::ParamRow {
                index,
                name,
                formula,
                resolved,
            } => self.render_param_row(*index, name, formula, resolved),
            PropValue::ParamAddRow => render_param_add_row(),
            PropValue::ParamsVisibilityToggle(value) => render_params_visibility_toggle_row(*value),
        }
    }

    // ── Layer row (combo_box) ─────────────────────────────────────────────

    fn render_layer_row<'a>(&'a self, label: &'a str, current: &'a str, theme: &Theme) -> Element<'a, Message> {
        let selected = if current == VARIES_LABEL {
            None
        } else {
            Some(current.to_string())
        };
        let combo = combo_box(
            &self.layer_combo,
            VARIES_LABEL,
            selected.as_ref(),
            Message::PropLayerChanged,
        )
        .size(FONT_SZ)
        .padding([COMBO_PAD_V, tokens::GAP_CONTROL])
        .input_style(kit::field_style)
        .on_open(Message::PropColorPickerClose)
        .width(Length::Fill);

        prop_row_widget(label, combo.into(), theme)
    }

    // ── Color row (custom picker) ─────────────────────────────────────────

    fn render_color_row<'a>(
        &'a self,
        label: &'a str,
        field: &'static str,
        color: AcadColor,
        display_name: Option<&'a str>,
        theme: &Theme,
    ) -> Element<'a, Message> {
        // MTEXT background colour uses its own picker state + messages so it
        // routes to `background_color`, not the entity's main colour.
        if field == "background_color" {
            let selector = crate::ui::color_select::color_selector(
                color,
                self.bg_color_picker_open,
                crate::ui::color_select::ColorExtras {
                    by_layer: true,
                    by_block: true,
                    ..Default::default()
                },
                Message::PropBgColorChanged,
                Message::PropBgColorPickerToggle,
                // "More Colors…" opens the full palette window targeting the
                // background colour — this used to just close the picker
                // (#415).
                Message::OpenColorWindow(crate::app::ColorPickTarget::PropertiesBg, color), theme
            );
            return prop_row_widget(label, selector, theme);
        }
        // Generic per-field colour picker — routes to the named field, not the
        // entity's main colour. Used by hatch gradient colours and the dim-line
        // colour override (Leader / Dimension). Dim colours legitimately take
        // ByLayer / ByBlock; gradient colours do not.
        if matches!(
            field,
            "gradient_color_1"
                | "gradient_color_2"
                | "dim_line_color"
                | "dim_ext_line_color"
                | "dim_text_color"
                | "dim_text_fill_color"
                | "line_color"
                | "text_color"
                | "block_content_color"
                | "background_fill_color"
                | "indicator_fill_color"
        ) {
            let open = self.open_color_field.as_deref() == Some(field);
            let fsel = field.to_string();
            let full_palette_field = field.to_string();
            let extras = if field == "dim_text_fill_color" {
                crate::ui::color_select::ColorExtras {
                    none: true,
                    background: true,
                    ..Default::default()
                }
            } else if field.starts_with("dim_")
                || matches!(field, "line_color" | "text_color" | "block_content_color")
            {
                crate::ui::color_select::ColorExtras {
                    by_layer: true,
                    by_block: true,
                    ..Default::default()
                }
            } else {
                crate::ui::color_select::ColorExtras {
                    by_layer: false,
                    by_block: false,
                    ..Default::default()
                }
            };
            let selector = crate::ui::color_select::color_selector(
                color,
                open,
                extras,
                move |c| Message::PropColorFieldChanged {
                    field: fsel.clone(),
                    color: c,
                },
                Message::PropColorFieldToggle(field.to_string()),
                Message::OpenColorWindow(
                    crate::app::ColorPickTarget::PropertiesField(full_palette_field),
                    color,
                ), theme
            );
            return prop_row_widget(label, selector, theme);
        }
        let selector = crate::ui::color_select::color_selector_with_name(
            color,
            display_name,
            self.color_picker_open,
            crate::ui::color_select::ColorExtras {
                by_layer: true,
                by_block: true,
                ..Default::default()
            },
            Message::PropColorChanged,
            Message::PropColorPickerToggle,
            Message::OpenColorWindow(crate::app::ColorPickTarget::Properties, color), theme
        );
        prop_row_widget(label, selector, theme)
    }

    fn render_color_varies_row<'a>(
        &'a self,
        label: &'a str,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let selector = crate::ui::color_select::color_selector_varies(
            self.color_picker_open,
            crate::ui::color_select::ColorExtras {
                by_layer: true,
                by_block: true,
                ..Default::default()
            },
            Message::PropColorChanged,
            Message::PropColorPickerToggle,
            Message::OpenColorWindow(crate::app::ColorPickTarget::Properties, AcadColor::ByLayer), theme
        );
        prop_row_widget(label, selector, theme)
    }

    // ── Lineweight row (combo_box) ────────────────────────────────────────

    fn render_lw_row<'a>(&'a self, label: &'a str, lw: LineWeight, theme: &Theme) -> Element<'a, Message> {
        let selected = LwItem(lw);
        let combo = combo_box(
            &self.lineweight_combo,
            "",
            Some(&selected),
            |item: LwItem| Message::PropLwChanged(item.0),
        )
        .size(FONT_SZ)
        .padding([COMBO_PAD_V, tokens::GAP_CONTROL])
        .input_style(kit::field_style)
        .on_open(Message::PropColorPickerClose)
        .width(Length::Fill);

        prop_row_widget(label, combo.into(), theme)
    }

    fn render_field_lw_row<'a>(
        &'a self,
        label: &'a str,
        field: &'static str,
        lw: LineWeight,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let selected = LwItem(lw);
        let combo = combo_box(
            &self.lineweight_combo,
            "",
            Some(&selected),
            move |item: LwItem| Message::PropFieldLwChanged {
                field,
                value: item.0,
            },
        )
        .size(FONT_SZ)
        .padding([COMBO_PAD_V, tokens::GAP_CONTROL])
        .input_style(kit::field_style)
        .on_open(Message::PropColorPickerClose)
        .width(Length::Fill);

        prop_row_widget(label, combo.into(), theme)
    }

    fn render_lw_varies_row<'a>(&'a self, label: &'a str, theme: &Theme) -> Element<'a, Message> {
        let combo = combo_box(
            &self.lineweight_combo,
            VARIES_LABEL,
            None,
            |item: LwItem| Message::PropLwChanged(item.0),
        )
        .size(FONT_SZ)
        .padding([COMBO_PAD_V, tokens::GAP_CONTROL])
        .input_style(kit::field_style)
        .on_open(Message::PropColorPickerClose)
        .width(Length::Fill);

        prop_row_widget(label, combo.into(), theme)
    }

    fn render_field_lw_varies_row<'a>(
        &'a self,
        label: &'a str,
        field: &'static str,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let combo = combo_box(
            &self.lineweight_combo,
            VARIES_LABEL,
            None,
            move |item: LwItem| Message::PropFieldLwChanged {
                field,
                value: item.0,
            },
        )
        .size(FONT_SZ)
        .padding([COMBO_PAD_V, tokens::GAP_CONTROL])
        .input_style(kit::field_style)
        .on_open(Message::PropColorPickerClose)
        .width(Length::Fill);

        prop_row_widget(label, combo.into(), theme)
    }

    // ── Linetype row (combo_box) ──────────────────────────────────────────

    fn render_linetype_row<'a>(&'a self, label: &'a str, current: &'a str, theme: &Theme) -> Element<'a, Message> {
        // Normalise: empty string = "ByLayer"
        let display = if current.is_empty() {
            "ByLayer"
        } else {
            current
        };
        let selected = self
            .linetype_items
            .iter()
            .find(|item| item.name.eq_ignore_ascii_case(display))
            .cloned();
        let combo = combo_box(
            &self.linetype_combo,
            VARIES_LABEL,
            selected.as_ref(),
            |item: LinetypeItem| Message::PropLinetypeChanged(item.name),
        )
        .size(FONT_SZ)
        .padding([COMBO_PAD_V, tokens::GAP_CONTROL])
        .input_style(kit::field_style)
        .on_open(Message::PropColorPickerClose)
        .width(Length::Fill);

        prop_row_widget(
            label,
            crate::ui::wide_menu::wide_menu(combo, LINETYPE_MENU_W),
            theme,
        )
    }

    fn render_choice_row<'a>(
        &'a self,
        label: &'a str,
        field: &'static str,
        current: &'a str,
        _options: &'a [String],
        theme: &Theme,
    ) -> Element<'a, Message> {
        let Some(state) = self.choice_combos.get(field) else {
            return render_ro_row(label, current, theme);
        };

        let selected = if current == VARIES_LABEL {
            None
        } else {
            Some(LocalizedChoice::new(current.to_string()))
        };
        let combo = combo_box(state, VARIES_LABEL, selected.as_ref(), move |choice| {
            Message::PropGeomChoiceChanged {
                field,
                value: choice.raw,
            }
        })
        .size(FONT_SZ)
        .padding([COMBO_PAD_V, tokens::GAP_CONTROL])
        .input_style(kit::field_style)
        .on_open(Message::PropColorPickerClose)
        .width(Length::Fill);

        prop_row_widget(label, combo.into(), theme)
    }

    // ── Editable geometry row (text_input) ────────────────────────────────

    fn render_edit_row<'a>(
        &'a self,
        label: &'a str,
        field: &'static str,
        entity_val: &'a str,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let key = FieldKey::Geom(field);
        let display = self
            .edit_buf
            .get(&key)
            .map(|s| s.as_str())
            .unwrap_or(entity_val);

        let active = self.active_field.as_ref() == Some(&key);
        let ti = text_input("", display)
            .id(prop_geom_field_id(field))
            .on_input(move |v| Message::PropGeomInput { field, value: v })
            .on_submit(Message::PropGeomCommit(field))
            .size(FONT_SZ)
            .style(kit::field_style)
            .padding(kit::PAD_FIELD)
            .width(Length::Fill);

        prop_row_with_active(label, ti.into(), active, theme)
    }

    /// Editable dropdown row (block reference Name): a text field with a caret
    /// button in one bordered control. Typing + Enter commits through the
    /// normal PropGeomCommit path (existing name → re-point, new name →
    /// rename); the caret opens a dropdown list of the definitions and picking one
    /// applies through PropGeomChoiceChanged. Typed text filters the list.
    fn render_edit_choice_row<'a>(
        &'a self,
        label: &'a str,
        field: &'static str,
        entity_val: &'a str,
        options: &'a [String],
        theme: &Theme,
    ) -> Element<'a, Message> {
        let key = FieldKey::Geom(field);
        let active = self.active_field.as_ref() == Some(&key);
        let typed = self.edit_buf.get(&key);
        let display = typed.map(|s| s.as_str()).unwrap_or(entity_val);

        let input = text_input("", display)
            .id(prop_geom_field_id(field))
            .on_input(move |v| Message::PropGeomInput { field, value: v })
            .on_submit(Message::PropGeomCommit(field))
            .size(FONT_SZ)
            .style(|theme: &Theme, status| kit::bare_field_style(theme, status))
            .padding(Padding {
                top: COMBO_PAD_V,
                bottom: COMBO_PAD_V,
                left: tokens::GAP_CONTROL,
                right: tokens::GAP_HAIRLINE,
            })
            .width(Length::Fill);
        let caret = plain_button(
            container(if self.edit_choice_open {
                crate::ui::icons::themed_arrow_up(9.0)
            } else {
                crate::ui::icons::themed_arrow_down(9.0)
            })
            .align_y(iced::Center),
        )
        .on_press(Message::PropEditChoiceToggle)
        .style(move |theme: &Theme, status| {
            kit::style_for(theme, status, kit::Role::Ghost, "")
        })
        .padding([COMBO_PAD_V, tokens::GAP_TIGHT]);
        let head = container(row![input, caret].align_y(iced::Center))
            // The composite field is a well: the recess is what says "type
            // here", and the row's own selection tint is what says "this one".
            .style(move |theme: &Theme| kit::Tier::Well.style(theme, ""))
            .width(Length::Fill);

        if !self.edit_choice_open {
            return prop_row_with_active(label, head.into(), active, theme);
        }

        // Open list: all definitions, filtered by any typed text.
        let filter = typed.map(|s| s.to_lowercase());
        let mut list = column![].spacing(tokens::GAP_HAIRLINE);
        for opt in options {
            if let Some(f) = &filter {
                if !opt.to_lowercase().contains(f.as_str()) {
                    continue;
                }
            }
            let value = opt.clone();
            list = list.push(
                kit::list_row(
                    text(opt.as_str()).size(FONT_SZ),
                    false,
                    kit::Press::On(Message::PropGeomChoiceChanged { field, value }),
                    "",
                ),
            );
        }
        let popup = kit::glass_pane(
            container(crate::ui::kit::themed_scrollable(list).height(Length::Shrink))
                // The choice list drops over the drawing; glass keeps the
                // geometry under it legible while the pick is being made.
                .padding(tokens::GAP_HAIRLINE)
                .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
                .into(),
            theme,
            false,
            "",
        );

        prop_row_with_active(
            label,
            crate::ui::color_select::drop_down_below(
                head.into(),
                popup.into(),
                None,
                Length::Shrink,
                Message::PropEditChoiceToggle,
            ),
            active,
            theme,
        )
    }

    /// One editable row for a block attribute: the tag is the row label and the
    /// text box edits the value. Routing rides the tag (a runtime string), so
    /// this uses the dedicated `PropAttr*` messages instead of the geometry
    /// path whose field key is `&'static str`. The row label is the tag itself.
    fn render_attr_row<'a>(&'a self, tag: &'a str, entity_val: &'a str, theme: &Theme) -> Element<'a, Message> {
        let key = attr_edit_key(tag);
        let display = self
            .edit_buf
            .get(&key)
            .map(|s| s.as_str())
            .unwrap_or(entity_val);

        let active = self.active_field.as_ref() == Some(&key);
        let tag_for_input = tag.to_string();
        let ti = text_input("", display)
            .id(prop_attr_field_id(tag))
            .on_input(move |v| Message::PropAttrInput {
                tag: tag_for_input.clone(),
                value: v,
            })
            .on_submit(Message::PropAttrCommit(tag.to_string()))
            .size(FONT_SZ)
            .style(kit::field_style)
            .padding(kit::PAD_FIELD)
            .width(Length::Fill);

        prop_row_with_active(tag, ti.into(), active, theme)
    }

    /// One named-parameter row: editable name + formula, with the live-
    /// resolved value (or error) shown alongside, embedded in the panel instead of
    /// the old separate modal. Each field commits on submit (Enter / losing
    /// focus), not per keystroke — `ParameterTable::set` validates
    /// immediately and would otherwise reject a formula mid-type (the same
    /// reasoning the old modal's buffered-Apply design used, just applied
    /// per-field instead of per-whole-table).
    fn render_param_row<'a>(
        &'a self,
        index: usize,
        name: &'a str,
        formula: &'a str,
        resolved: &'a Result<f64, String>,
    ) -> Element<'a, Message> {
        use crate::ui::window::named_parameters::ParamField;

        let name_key = FieldKey::Param(index, ParamField::Name);
        let name_display = self
            .edit_buf
            .get(&name_key)
            .map(|s| s.as_str())
            .unwrap_or(name);
        let name_active = self.active_field.as_ref() == Some(&name_key);
        let name_input = text_input("name", name_display)
            .id(name_key.widget_id())
            .on_input(move |v| Message::PropParamInput {
                index,
                field: ParamField::Name,
                value: v,
            })
            .on_submit(Message::PropParamCommit {
                index,
                field: ParamField::Name,
            })
            .size(FONT_SZ)
            .style(kit::field_style)
            .padding(kit::PAD_FIELD)
            .width(Length::FillPortion(3));

        let formula_key = FieldKey::Param(index, ParamField::Formula);
        let formula_display = self
            .edit_buf
            .get(&formula_key)
            .map(|s| s.as_str())
            .unwrap_or(formula);
        let formula_active = self.active_field.as_ref() == Some(&formula_key);
        let formula_input = text_input("formula", formula_display)
            .id(formula_key.widget_id())
            .on_input(move |v| Message::PropParamInput {
                index,
                field: ParamField::Formula,
                value: v,
            })
            .on_submit(Message::PropParamCommit {
                index,
                field: ParamField::Formula,
            })
            .size(FONT_SZ)
            .style(kit::field_style)
            .padding(kit::PAD_FIELD)
            .width(Length::FillPortion(3));

        let (value_text, is_error) = match resolved {
            Ok(v) => (format!("{v:.4}"), false),
            Err(_) => ("—".to_string(), true),
        };
        let value_label = container(text(value_text).size(FONT_SZ).style(move |theme: &Theme| {
            if is_error {
                kit::tone_style(theme, kit::Tone::Danger)
            } else {
                Default::default()
            }
        }))
        .width(Length::FillPortion(2))
        .align_x(iced::Right);

        let delete_btn = kit::icon_btn(
            text("\u{2715}").size(FONT_SZ),
            kit::Role::Ghost,
            20.0,
            kit::Press::On(Message::PropParamDelete(index)),
            "",
        );

        let active = name_active || formula_active;
        let content = container(
            row![name_input, formula_input, value_label, delete_btn]
                .spacing(tokens::GAP_TIGHT)
                .align_y(iced::Center),
        )
        .style(move |theme: &Theme| {
            // The row is a cell, not a plane: the selected one takes the
            // app's single row wash, the rest of them stay on the page.
            if active {
                kit::tint_style(kit::Tint::Selected, theme, "")
            } else {
                kit::wash_style(kit::Tier::Flush, theme, "")
            }
        })
        .padding([tokens::GAP_NONE, tokens::GAP_CONTROL])
        .width(Length::Fill);

        if let Err(err) = resolved {
            tooltip(
                content,
                text(err.as_str()).size(FONT_SZ),
                tooltip::Position::Top,
            )
            .gap(4.0)
            .padding(tokens::GAP_CONTROL)
            .style(|theme: &Theme| {
                kit::tone_plate_style_on(
                    theme,
                    kit::Tone::Danger,
                    kit::plane(theme, kit::Tier::Card),
                    "",
                )
            })
            .into()
        } else {
            content.into()
        }
    }

    fn render_hatch_pattern_row<'a>(
        &'a self,
        label: &'a str,
        current: &'a str,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let head = plain_button(
            row![
                crate::ui::text_util::elided(current, 16, theme, |shown| {
                    text(shown).size(FONT_SZ).width(Length::Fill).into()
                }),
                if self.hatch_pattern_picker_open {
                    crate::ui::icons::themed_arrow_up(FONT_SZ)
                } else {
                    crate::ui::icons::themed_arrow_down(FONT_SZ)
                },
            ]
            .align_y(iced::Center),
        )
        .on_press(Message::PropHatchPatternPickerToggle(current.to_string()))
        .style(kit::chip_button_style(self.hatch_pattern_picker_open))
        .padding([COMBO_PAD_V, tokens::GAP_CONTROL])
        .width(Length::Fill);

        if !self.hatch_pattern_picker_open {
            return prop_row_widget(label, head.into(), theme);
        }

        let popup = hatch_pattern_picker_content(
            &self.hatch_pattern_search,
            self.hatch_pattern_focus,
            current,
            PatternPickerMessages {
                search_id: "hatch-pattern-search",
                on_search: Message::PropHatchPatternSearchChanged,
                on_confirm: Message::PropHatchPatternConfirm,
                on_focus: Message::PropHatchPatternFocus,
                on_changed: Message::PropHatchPatternChanged,
            },
            theme,
        );

        prop_row_widget(
            label,
            crate::ui::color_select::drop_down_below(
                head.into(),
                popup,
                Some(Length::Fixed(PATTERN_PICKER_W)),
                Length::Fixed(PATTERN_PICKER_H),
                Message::PropHatchPatternPickerToggle(current.to_string()),
            ),
            theme,
        )
    }
}

// ── Standalone helpers ────────────────────────────────────────────────────

/// A boolean toggle button row (for "Invisible" etc.).
fn render_stepper_row<'a>(label: &'a str, display: &'a str, theme: &Theme) -> Element<'a, Message> {
    let arrow = |glyph: &'static str, delta: i8| {
        kit::icon_btn(
            text(glyph).size(FONT_SZ),
            kit::Role::Standard,
            18.0,
            kit::Press::On(Message::PropVertexStep(delta)),
            "",
        )
    };
    let widget = iced::widget::row![
        arrow("◀", -1),
        text(display)
            .size(FONT_SZ)
            .width(Length::Fill)
            .align_x(iced::Center),
        arrow("▶", 1),
    ]
    .spacing(tokens::GAP_TIGHT)
    .align_y(iced::Center);
    prop_row_widget(label, widget.into(), theme)
}

fn render_bool_row<'a>(label: &'a str, field: &'static str, value: bool, theme: &Theme) -> Element<'a, Message> {
    let btn_label = if value {
        t!("Yes").into_owned()
    } else {
        t!("No").into_owned()
    };
    let btn = plain_button(
        text(btn_label).size(FONT_SZ).style(move |theme: &Theme| {
            if value {
                kit::tone_style(theme, kit::Tone::Warning)
            } else {
                Default::default()
            }
        }),
    )
    .on_press(Message::PropBoolToggle(field))
    .style(kit::chip_button_style(value))
    .padding(kit::Size::Compact.padding())
    .width(Length::Fill);

    prop_row_widget(label, btn.into(), theme)
}

// ── Collapsible coordinate groups (Position / Start / Scale …) ────────────

/// The X/Y/Z suffix rank of a coordinate row label, with its base ("Position
/// X" → ("Position", 0)). `None` for non-coordinate labels.
fn coord_suffix(label: &str) -> Option<(&str, usize)> {
    for (rank, suf) in [" X", " Y", " Z"].iter().enumerate() {
        if let Some(base) = label.strip_suffix(suf) {
            if !base.is_empty() {
                return Some((base, rank));
            }
        }
    }
    None
}

/// Length of the coordinate group starting at `idx`: consecutive text rows
/// labelled "<Base> X", "<Base> Y" and optionally "<Base> Z". 0/1 = no group.
fn coord_group_len(props: &[crate::scene::model::object::Property], idx: usize) -> usize {
    if matches!(props[idx].field, "pl3_vertex_x" | "pm_vx") {
        return 0;
    }
    let groupable = |p: &crate::scene::model::object::Property| {
        matches!(
            p.value,
            PropValue::EditText(_) | PropValue::ReadOnly(_) | PropValue::ReadOnlyWithTooltip { .. }
        )
    };
    let Some((base, 0)) = coord_suffix(&props[idx].label) else {
        return 0;
    };
    if !groupable(&props[idx]) {
        return 0;
    }
    let mut len = 1;
    while idx + len < props.len() && len < 3 {
        match coord_suffix(&props[idx + len].label) {
            Some((b, r)) if b == base && r == len && groupable(&props[idx + len]) => len += 1,
            _ => break,
        }
    }
    if len >= 2 {
        len
    } else {
        0
    }
}

fn coord_base(label: &str) -> &str {
    coord_suffix(label).map(|(b, _)| b).unwrap_or(label)
}

/// Short component label for an expanded row ("Position X" → indented "X").
fn coord_component(label: &str) -> &'static str {
    match coord_suffix(label) {
        Some((_, 0)) => "    X",
        Some((_, 1)) => "    Y",
        _ => "    Z",
    }
}

/// Display string of a text-valued property (grouped rows are always
/// EditText / ReadOnly — see `coord_group_len`).
fn prop_text_value(prop: &crate::scene::model::object::Property) -> String {
    match &prop.value {
        PropValue::EditText(s)
        | PropValue::ReadOnly(s)
        | PropValue::ReadOnlyWithTooltip { value: s, .. } => s.clone(),
        _ => String::new(),
    }
}

/// The collapsed summary row of a coordinate group. The expand arrow leads
/// the label cell (clicking the cell toggles); the value cell is the same
/// read-only selectable field every other read-only row uses.
fn render_group_row<'a>(
    base: &str,
    key: String,
    expanded: bool,
    joined: String,
    theme: &Theme,
) -> Element<'a, Message> {
    let label_btn = kit::list_row(
        container(
            row![
                if expanded {
                    crate::ui::icons::themed_arrow_down(FONT_SZ)
                } else {
                    crate::ui::icons::themed_arrow_right(FONT_SZ)
                },
                crate::ui::text_util::elided(base, 16, theme, |shown| {
                    text(shown).size(FONT_SZ).style(kit::muted_style).into()
                }),
            ]
            .spacing(tokens::GAP_TIGHT)
            .align_y(iced::Center),
        )
        .height(Length::Fill)
        .align_y(iced::Center),
        false,
        kit::Press::On(Message::PropGroupToggle(key)),
        "",
    )
    .padding(Padding {
        top: tokens::GAP_NONE,
        bottom: tokens::GAP_NONE,
        left: tokens::GAP_TIGHT,
        right: tokens::GAP_CONTROL,
    })
    .height(Length::Fixed(ROW_H));
    let label_col = container(label_btn)
        // The label gutter is a band inside the row: square like the seam it
        // sits against, and a notch of fill below the pallet's card.
        .style(|theme: &Theme| kit::wash_style(kit::Tier::Band, theme, ""))
        // 7:5 since the 2026-11-02 ladder lift. At the old 5:6 the 13 px labels
        // wrapped in the 250 px dock ("打印样式表应用于", "UCS 图标位于原点"),
        // pushing the fixed 26 px row into a second line. The label side now
        // carries the longer half; value fields are inputs and clip, so they
        // lose the width without changing shape.
        .width(Length::FillPortion(7))
        .height(Length::Fixed(ROW_H))
        .align_y(iced::Center);

    // The field copies the value, so the locally-built `joined` is fine here.
    let value_field = crate::ui::read_only::field(&joined, FONT_SZ, Length::Fill);
    // The value column is the page itself, not a plane on it: the row's label
    // gutter is the only half that carries its own wash, and the recessed
    // read-only field paints its own well. So this one wears `Flush`.
    let value_col = container(value_field)
        .style(move |theme: &Theme| {
            kit::wash_style(kit::Tier::Flush, theme, "")
        })
        .width(Length::FillPortion(5))
        .height(Length::Fixed(ROW_H))
        .align_y(iced::Center)
        .padding([tokens::GAP_NONE, tokens::GAP_HAIRLINE]);

    container(row![label_col, value_col])
        .height(Length::Fixed(ROW_H))
        .style(|theme: &Theme| kit::seam_style(theme, ""))
        .into()
}
fn render_annotative_scale_row<'a>(label: &'a str, value: &'a str, theme: &Theme) -> Element<'a, Message> {
    let field = crate::ui::read_only::field(value, FONT_SZ, Length::Fill);

    let manage = kit::btn(
        text("...").size(FONT_SZ),
        kit::Role::Standard,
        kit::Size::Compact,
        kit::Press::On(Message::AnnoObjectScaleOpen),
        "",
    );

    let controls = row![field, manage, iced::widget::space().width(tokens::GAP_FIELD)]
        .spacing(tokens::GAP_HAIRLINE)
        .align_y(iced::Center)
        .width(Length::Fill);

    prop_row_widget(label, controls.into(), theme)
}
fn render_hyperlink_row<'a>(label: &'a str, value: &'a str, theme: &Theme) -> Element<'a, Message> {
    let field = crate::ui::read_only::field(value, FONT_SZ, Length::Fill);
    let manage = kit::btn(
        text("...").size(FONT_SZ),
        kit::Role::Standard,
        kit::Size::Compact,
        kit::Press::On(Message::PropHyperlinkOpen),
        "",
    );
    let controls = row![field, manage, iced::widget::space().width(tokens::GAP_FIELD)]
        .spacing(tokens::GAP_HAIRLINE)
        .align_y(iced::Center)
        .width(Length::Fill);
    prop_row_widget(label, controls.into(), theme)
}
fn render_ro_row<'a>(label: &'a str, value: &'a str, theme: &Theme) -> Element<'a, Message> {
    // A read-only value is shown as a non-editable but selectable field: no
    // on_input means the caret never appears, but the text can be selected
    // (carrying the full, un-truncated value) and copied with Ctrl+C.
    let field = crate::ui::read_only::field(value, FONT_SZ, Length::Fill);
    prop_row_widget(label, field, theme)
}

fn render_ro_with_tooltip_row<'a>(
    label: &'a str,
    value: &'a str,
    tooltip_text: &'a str,
    theme: &Theme,
) -> Element<'a, Message> {
    let field = crate::ui::read_only::field(value, FONT_SZ, Length::Fill);
    // The plate the tooltip used to paint is retired: the glass pane carries
    // the inset, and `Tier::Flush` on the tooltip itself stops iced from
    // drawing its own solid plate one padding wider than the glass on
    // every side.
    let tip = kit::glass_pane(
        container(text(tooltip_text).size(FONT_SZ))
            .padding(tokens::GAP_CONTROL)
            .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
            .into(),
        theme,
        false,
        "",
    );
    let wrapped = tooltip(field, tip, tooltip::Position::Top)
        .gap(4.0)
        .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""));
    prop_row_widget(label, wrapped.into(), theme)
}

// ── Constraints section row (clickable entity link) ───────────────────────

/// One persistent-constraint row in the Constraints properties section:
/// `label` is the glyph + kind name + resolved value (e.g. "↔ Distance:
/// hole_dia = 12.00"), rendered as the row's own clickable surface — there's
/// no natural separate "value" for a constraint row, unlike a Layer/Color
/// field, so this doesn't use the usual `prop_row_widget` label|value split.
/// Clicking it selects every entity in `handles`; a conflicting/redundant
/// constraint (mirrors the viewport glyph pill's own color cue) tints red.
fn render_entity_link_row<'a>(
    label: &'a str,
    id: crate::scene::parametric_constraints::ConstraintId,
    handles: Vec<Handle>,
    conflicting: bool,
) -> Element<'a, Message> {
    let link = plain_button(text(crate::t!(label)).size(FONT_SZ).width(Length::Fill))
        .on_press(Message::PropConstraintLinkClick(handles))
        .style(move |theme: &Theme, status| {
            kit::style_for(
                theme,
                status,
                if conflicting {
                    kit::Role::Danger
                } else {
                    kit::Role::Standard
                },
                "",
            )
        })
        .padding(kit::Size::Control.padding())
        .width(Length::Fill);
    let delete = kit::icon_btn(
        text("\u{2715}").size(FONT_SZ),
        kit::Role::Ghost,
        20.0,
        kit::Press::On(Message::PropConstraintDelete(id)),
        "",
    );
    container(row![link, delete].spacing(tokens::GAP_HAIRLINE).align_y(iced::Center))
        .width(Length::Fill)
        .into()
}

// ── Parameters section: "+ Add parameter" row ──────────────────────────────

/// The trailing "+ Add parameter" row: appends a fresh, uniquely-named
/// parameter (`param1`, `param2`, …) the user then renames/redefines inline
/// — avoids a separate "pending new row" concept, since every row always
/// reflects a real committed table entry.
fn render_param_add_row<'a>() -> Element<'a, Message> {
    let btn = plain_button(
        row![
            text("+").size(FONT_SZ),
            text(t!("Add parameter").into_owned()).size(FONT_SZ)
        ]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center),
    )
    .on_press(Message::PropParamAddNew)
    .style(kit::role_style(kit::Role::Ghost))
    .padding(kit::Size::Compact.padding())
    .width(Length::Fill);
    container(btn).width(Length::Fill).into()
}

// ── Parameters section: leading global visibility toggle ───────────────────

/// The Parameters section's leading header row (no-selection page): a
/// global on/off toggle for whether any constraint pill in the viewport
/// shows its driven value/parameter-name text — lives next to the
/// named-parameter table it governs.
fn render_params_visibility_toggle_row<'a>(value: bool) -> Element<'a, Message> {
    let btn_label = if value { t!("On") } else { t!("Off") }.into_owned();
    let btn = plain_button(
        row![
            crate::ui::icons::semantic(crate::ui::icons::layer_visible(value), 13.0),
            text(btn_label).size(FONT_SZ),
        ]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center),
    )
    .on_press(Message::ShowConstraintValuesChanged(!value))
    .style(kit::chip_button_style(value))
    .padding(kit::Size::Compact.padding())
    .width(Length::Fill);
    container(
        row![
            text(t!("Values").into_owned())
                .size(FONT_SZ)
                .width(Length::Fill),
            btn
        ]
        .align_y(iced::Center),
    )
    .width(Length::Fill)
    .into()
}

/// Build a label | widget property row.
fn prop_row_widget<'a>(label: &'a str, widget: Element<'a, Message>, theme: &Theme) -> Element<'a, Message> {
    prop_row_with_active(label, widget, false, theme)
}

/// Like [`prop_row_widget`] but washes the row as the currently-active edit
/// row when `active` is true, with the kit's single row tint so it reads as
/// "selected" without shouting.
fn prop_row_with_active<'a>(
    label: &'a str,
    widget: Element<'a, Message>,
    active: bool,
    theme: &Theme,
) -> Element<'a, Message> {
    let full_label = t!(label);
    let label_col = container(crate::ui::text_util::elided(&full_label, 18, theme, |shown| {
        text(shown)
            .size(FONT_SZ)
            // One line, always. A wrapped label turns the fixed 26 px row into
            // two lines and shoves every row below it down — the 2026-11-02
            // lift made several 13 px labels wider than their gutter. cosmic-
            // text ellipsizes at the exact measured width instead.
            .wrapping(Wrapping::None)
            .ellipsis(Ellipsis::End)
            .style(kit::muted_style)
            .into()
    }))
    .style(move |theme: &Theme| {
        if active {
            kit::tint_style(kit::Tint::Selected, theme, "")
        } else {
            kit::wash_style(kit::Tier::Band, theme, "")
        }
    })
    // 7:5, matching `render_group_row`: the 2026-11-02 lift made the 13 px
    // labels wider than a 5/11 gutter in the 250 px dock, and a wrapped label
    // breaks the fixed 26 px row. Values are inputs, so they clip instead.
    .width(Length::FillPortion(7))
    .height(Length::Fixed(ROW_H))
    .align_y(iced::Center)
    .padding([tokens::GAP_NONE, tokens::GAP_CONTROL]);
    let value_col = container(widget)
        // The value half is the pallet's own plane: the field inside it paints
        // its well, so this column wears no fill of its own.
        .style(move |theme: &Theme| {
            if active {
                kit::tint_style(kit::Tint::Selected, theme, "")
            } else {
                kit::wash_style(kit::Tier::Flush, theme, "")
            }
        })
        .width(Length::FillPortion(5))
        .height(Length::Fixed(ROW_H))
        .align_y(iced::Center)
        .padding([tokens::GAP_NONE, tokens::GAP_HAIRLINE]);
    container(row![label_col, value_col])
        .height(Length::Fixed(ROW_H))
        // Selection no longer changes this seam — the tint on the two halves is
        // what says "this row holds the edit". The 1px square row seam is
        // kit::seam_style; the screen supplies no numbers of its own.
        .style(|theme: &Theme| kit::seam_style(theme, ""))
        .into()
}

// ── Color display helper ──────────────────────────────────────────────────

/// Returns an (iced::Color swatch_bg, display_label) pair for an AcadColor.
pub fn acad_color_display(c: AcadColor) -> (Color, &'static str) {
    match c {
        AcadColor::None => (Color::TRANSPARENT, "None"),
        AcadColor::ByLayer => (
            Color {
                r: 0.35,
                g: 0.35,
                b: 0.35,
                a: 1.0,
            },
            "ByLayer",
        ),
        AcadColor::ByBlock => (
            Color {
                r: 0.25,
                g: 0.25,
                b: 0.45,
                a: 1.0,
            },
            "ByBlock",
        ),
        AcadColor::Index(i) => {
            let (r, g, b) = codec::types::aci_table::aci_to_rgb(i).unwrap_or((200, 200, 200));
            (
                Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0),
                aci_label(i),
            )
        }
        AcadColor::Rgb { r, g, b } => (
            Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0),
            "Custom",
        ),
    }
}

fn aci_label(idx: u8) -> &'static str {
    match idx {
        1 => "Red",
        2 => "Yellow",
        3 => "Green",
        4 => "Cyan",
        5 => "Blue",
        6 => "Magenta",
        7 => "White",
        8 => "Dark Gray",
        9 => "Light Gray",
        _ => "Index",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        active_key_focused, attr_edit_key, build_field_key_map, hatch_pattern_matches,
        hatch_preview_scale, prop_attr_field_id, prop_geom_field_id, FieldKey,
    };

    fn sample_sections() -> Vec<crate::scene::model::object::PropSection> {
        use crate::scene::model::object::{PropValue, Property};
        vec![
            crate::scene::model::object::PropSection {
                title: "Geometry".into(),
                props: vec![Property {
                    label: "X".into(),
                    field: "pos_x",
                    value: PropValue::EditText("1.0".into()),
                }],
            },
            crate::scene::model::object::PropSection {
                title: "Attributes".into(),
                props: vec![Property {
                    label: "TITLE".into(),
                    field: "0",
                    value: PropValue::AttrText {
                        tag: "TITLE".into(),
                        value: "hello".into(),
                    },
                }],
            },
            crate::scene::model::object::PropSection {
                title: "Reference".into(),
                props: vec![Property {
                    label: "Name".into(),
                    field: "name",
                    value: PropValue::EditChoice {
                        value: "Door".into(),
                        options: vec!["Door".into(), "Window".into()],
                    },
                }],
            },
        ]
    }

    #[test]
    fn focus_transition_updates_the_active_field_highlight() {
        // The highlight is keyed to real keyboard focus: it persists only while
        // the focused widget is THAT field's text input, and clears when focus
        // moves to any other widget (or nowhere).
        //
        // Focus on the active field's own input → row stays marked active.
        let focused = prop_geom_field_id("pos_x");
        assert!(active_key_focused(
            Some(&FieldKey::Geom("pos_x")),
            Some(&focused)
        ));

        // Focus moves to another property field → superseded (cleared here; the
        // manual focus-setter then marks the new field active).
        assert!(!active_key_focused(
            Some(&FieldKey::Geom("pos_x")),
            Some(&prop_geom_field_id("pos_y"))
        ));

        // Focus moves to a non-property widget / nothing → cleared.
        assert!(!active_key_focused(
            Some(&FieldKey::Geom("pos_x")),
            Some(&prop_attr_field_id("TITLE"))
        ));
        assert!(!active_key_focused(Some(&FieldKey::Geom("pos_x")), None));

        // No active field (or one whose input isn't focused) never reads active.
        assert!(!active_key_focused(
            None,
            Some(&prop_geom_field_id("pos_x"))
        ));
    }

    #[test]
    fn field_ids_are_distinct_across_geometries_and_attributes() {
        assert_ne!(prop_geom_field_id("pos_x"), prop_geom_field_id("pos_y"));
        assert_ne!(prop_geom_field_id("pos_x"), prop_attr_field_id("TITLE"));
        assert_eq!(prop_attr_field_id("TITLE"), prop_attr_field_id("TITLE"));
    }

    #[test]
    fn prop_field_key_for_id_maps_focus_ids_to_active_keys() {
        let sections = sample_sections();
        let map = build_field_key_map(&sections);

        // A focused geometry edit field maps back to its `&'static str` key,
        // and round-trips through `active_key_focused`.
        let geom_id = prop_geom_field_id("pos_x");
        assert_eq!(map.get(&geom_id), Some(&FieldKey::Geom("pos_x")));
        assert!(active_key_focused(map.get(&geom_id), Some(&geom_id)));

        // A focused attribute edit field maps back to its tag key.
        let attr_id = prop_attr_field_id("TITLE");
        assert_eq!(map.get(&attr_id), Some(&attr_edit_key("TITLE")));
        assert!(active_key_focused(map.get(&attr_id), Some(&attr_id)));

        // The caret-dropdown (EditChoice) is editable and maps to its field key.
        let name_id = prop_geom_field_id("name");
        assert_eq!(map.get(&name_id), Some(&FieldKey::Geom("name")));
        assert!(active_key_focused(map.get(&name_id), Some(&name_id)));

        // Unknown or non-property ids map to nothing.
        assert_eq!(map.get(&prop_geom_field_id("nope")), None);
        assert_eq!(map.get(&iced::widget::Id::new("elsewhere")), None);
    }

    #[test]
    fn hatch_picker_filters_names_and_descriptions() {
        let ansi31 = crate::scene::model::hatch_patterns::find("ANSI31").unwrap();

        assert!(hatch_pattern_matches(ansi31, "ansi"));
        assert!(hatch_pattern_matches(ansi31, &ansi31.description));
        assert!(!hatch_pattern_matches(ansi31, "definitely-not-a-pattern"));
    }

    #[test]
    fn hatch_preview_scale_is_finite_and_visible() {
        let ansi31 = crate::scene::model::hatch_patterns::find("ANSI31").unwrap();
        let scale = hatch_preview_scale(&ansi31.gpu);

        assert!(scale.is_finite());
        assert!((0.01..=100.0).contains(&scale));
    }
}
