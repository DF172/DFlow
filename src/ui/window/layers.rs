//! Layer Manager — floating window.

use crate::app::Message;
use crate::t;
use crate::ui::kit;
use crate::ui::properties::{lw_options, LinetypeItem, LwItem};
use crate::ui::kit::muted_style;
use crate::ui::style::tokens;
use crate::ui::ROW_H;
use codec::tables::layer::Layer as DocLayer;
use codec::tables::Table;
use codec::types::aci_table::aci_to_rgb;
use codec::types::{Color as AcadColor, LineWeight};
use codec::Handle;
use iced::widget::{column, combo_box, container, mouse_area, row, text, text_input};
use iced::Padding;
use iced::{Color, Element, Fill, Length, Theme};
use std::borrow::Cow;

// ── Per-viewport column descriptor ───────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct VpCol {
    pub handle: Handle,
    pub label: String,
}

/// Sortable column in the Layer Manager table. Clicking a header sorts by
/// that column; clicking the active header again flips the direction (#133).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LayerSortCol {
    Name,
    On,
    Freeze,
    Lock,
    Plot,
    Color,
    Linetype,
    Lineweight,
    Transparency,
}

// ── Row-height-derived constants ─────────────────────────────────────────
/// SVG icon size inside a layer-table cell.
const ICON_SZ: f32 = ROW_H * 0.62; // ≈16 px at ROW_H=26
/// Font size for cell text: the dense-table rung, not a fraction of the row.
/// `ROW_H * 0.42` shipped 10.9 px until the 2026-11-02 ladder lift.
const FONT_SZ: f32 = tokens::TEXT_LABEL;
/// Vertical padding for combo_box / text_input so their total height = ROW_H.
const COMBO_PAD_V: f32 = (ROW_H - FONT_SZ * 1.3 - 2.0) / 2.0;
const LINETYPE_MENU_W: f32 = 220.0;
/// Right-rail width when a dock pallet is wide enough to carry it.
const RAIL_WIDTH: f32 = 200.0;
/// Widget id for the layer-table so a freshly created layer can be
/// scrolled into view after it is added (#271).
pub const LAYER_TABLE_SCROLL_ID: &str = "layer-manager-table-scroll";

// ── Layer data ────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    pub frozen: bool,
    pub locked: bool,
    pub plottable: bool,
    pub color: AcadColor,
    pub color_name: Option<String>,
    pub book_name: Option<String>,
    pub linetype: String,
    pub lineweight: LineWeight,
    pub transparency: i32,
    /// Freeze state per-viewport, indexed parallel to LayerPanel::vp_cols.
    pub vp_frozen: Vec<bool>,
}

impl Layer {
    pub fn new(name: &str, color: AcadColor) -> Self {
        Self {
            name: name.to_string(),
            visible: true,
            frozen: false,
            locked: false,
            plottable: true,
            color,
            color_name: None,
            book_name: None,
            linetype: "Continuous".to_string(),
            lineweight: LineWeight::Default,
            transparency: 0,
            vp_frozen: vec![],
        }
    }
}

// ── Panel state ───────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct LayerPanel {
    pub layers: Vec<Layer>,
    #[allow(dead_code)]
    pub visible: bool,
    /// Anchor row: the last row clicked. Drives the editable combos and the
    /// Shift-range origin. Its layer is always part of `selected_multi`.
    pub selected: Option<usize>,
    /// All selected rows (Ctrl/Shift extend it). Bulk property changes and
    /// deletion act on every row here. Empty ⇔ nothing selected.
    pub selected_multi: Vec<usize>,
    /// Layer being renamed (row text-input open). Keyed by NAME so a
    /// rebuild/sort between start and commit cannot retarget it (#44);
    /// `selected`/`selected_multi` below stay index-based and are re-resolved.
    pub editing: Option<String>,
    pub edit_buf: String,
    pub current_layer: String,
    pub linetype_items: Vec<LinetypeItem>,
    /// Row whose color swatch is open. Name-keyed for the same reason (#44).
    pub color_picker_row: Option<String>,
    pub color_full_palette: bool,
    pub linetype_combo: combo_box::State<LinetypeItem>,
    pub lw_combo: combo_box::State<LwItem>,
    /// Per-viewport columns (only populated when in a paper layout with viewports).
    pub vp_cols: Vec<VpCol>,
    /// Active sort column, or `None` for document order.
    pub sort_col: Option<LayerSortCol>,
    /// Sort direction; `true` = ascending.
    pub sort_asc: bool,
    /// Live name filter from the search box; empty shows every layer (#343).
    pub filter: String,
    /// Bumped whenever `layers` changes shape (rebuild or sort), so the row
    /// cache knows its filtered index list went stale.
    rows_rev: u64,
    /// Vertical scroll offset of the table's own scrollable, in px.
    pub table_offset_y: f32,
    /// Visible table height in px; zero until the scrollable reports once.
    pub table_height: f32,
    /// Filtered row indices, reused across frames — a 2000-layer table then
    /// costs one lowercase pass per edit instead of one per frame.
    row_cache: std::cell::RefCell<LayerTableCache>,
}

/// First-frame height guess for the table viewport. The scrollable reports
/// its real bounds on the first layout whenever the content overflows, but
/// a short table (fewer rows than a pallet) never overflows — this guess
/// keeps those rows visible until the first report arrives.
const DEFAULT_TABLE_H: f32 = 900.0;

/// Filtered row indices for the layer table.
#[derive(Clone, Debug, Default)]
pub struct LayerTableCache {
    filter_lower: String,
    revision: u64,
    indices: Vec<usize>,
    builds: u32,
}

impl LayerTableCache {
    /// Row indices passing `filter`, in document order. Rebuilt only when the
    /// filter text or the row list revision changed.
    pub fn rows(&mut self, layers: &[Layer], filter: &str, revision: u64) -> &[usize] {
        let filter_lower = filter.to_lowercase();
        if self.filter_lower != filter_lower || self.revision != revision {
            self.indices.clear();
            self.indices.extend(
                layers
                    .iter()
                    .enumerate()
                    .filter(|(_, layer)| {
                        filter_lower.is_empty() || layer.name.to_lowercase().contains(&filter_lower)
                    })
                    .map(|(i, _)| i),
            );
            self.filter_lower = filter_lower;
            self.revision = revision;
            self.builds += 1;
        }
        &self.indices
    }

    pub fn build_count(&self) -> u32 {
        self.builds
    }
}

impl Default for LayerPanel {
    fn default() -> Self {
        Self {
            visible: false,
            layers: vec![Layer::new("0", AcadColor::Index(7))],
            selected: None,
            selected_multi: Vec::new(),
            editing: None,
            edit_buf: String::new(),
            current_layer: "0".to_string(),
            linetype_items: vec![LinetypeItem {
                name: "Continuous".into(),
                art: String::new(),
            }],
            color_picker_row: None,
            color_full_palette: false,
            linetype_combo: combo_box::State::new(vec![LinetypeItem {
                name: "Continuous".into(),
                art: String::new(),
            }]),
            lw_combo: combo_box::State::new(lw_options()),
            vp_cols: vec![],
            // Default to alphabetical (Name) order in both the Layer Manager
            // table and the ribbon's quick layer dropdown (#270). A header
            // click still re-sorts by any other column.
            sort_col: Some(LayerSortCol::Name),
            sort_asc: true,
            filter: String::new(),
            rows_rev: 0,
            table_offset_y: 0.0,
            table_height: 0.0,
            row_cache: std::cell::RefCell::new(LayerTableCache::default()),
        }
    }
}

impl LayerPanel {
    /// Sync layers + update per-viewport freeze columns.
    /// `vp_info`: list of (vp_handle, vp_label, frozen_layer_handles) from scene.
    pub fn sync_with_viewports(
        &mut self,
        doc_layers: &Table<DocLayer>,
        vp_info: Vec<(Handle, String, Vec<Handle>)>,
    ) {
        // The rebuild below re-indexes rows, so capture the selection by name
        // first and re-resolve it after (indices alone would go stale).
        let anchor_name = self
            .selected
            .and_then(|i| self.layers.get(i))
            .map(|l| l.name.clone());
        let multi_names: Vec<String> = self
            .selected_multi
            .iter()
            .filter_map(|&i| self.layers.get(i).map(|l| l.name.clone()))
            .collect();

        self.vp_cols = vp_info
            .iter()
            .map(|(h, label, _)| VpCol {
                handle: *h,
                label: label.clone(),
            })
            .collect();

        self.layers = doc_layers
            .iter()
            // The reference's hidden system layers (`*ADSK_CONSTRAINTS`) stay out.
            .filter(|l| !l.name.starts_with('*'))
            .map(|l| {
                let layer_handle = l.handle;
                let vp_frozen = vp_info
                    .iter()
                    .map(|(_, _, frozen_handles)| frozen_handles.contains(&layer_handle))
                    .collect();
                Layer {
                    name: l.name.clone(),
                    visible: !l.flags.off,
                    frozen: l.flags.frozen,
                    locked: l.flags.locked,
                    plottable: l.is_plottable,
                    color: l.color,
                    color_name: l.color_name.clone(),
                    book_name: l.book_name.clone(),
                    linetype: if l.line_type.is_empty() {
                        "Continuous".to_string()
                    } else {
                        l.line_type.clone()
                    },
                    lineweight: l.line_weight,
                    transparency: (l.transparency.as_percent() * 100.0).round() as i32,
                    vp_frozen,
                }
            })
            .collect();

        // Re-resolve the selection against the rebuilt rows by name.
        self.selected = anchor_name.and_then(|n| self.layers.iter().position(|l| l.name == n));
        self.selected_multi = multi_names
            .iter()
            .filter_map(|n| self.layers.iter().position(|l| l.name == *n))
            .collect();

        self.apply_sort();
        self.rows_rev = self.rows_rev.wrapping_add(1);
    }

    /// Set/flip the sort column from a header click, then re-sort.
    pub fn sort_by(&mut self, col: LayerSortCol) {
        if self.sort_col == Some(col) {
            self.sort_asc = !self.sort_asc;
        } else {
            self.sort_col = Some(col);
            self.sort_asc = true;
        }
        self.apply_sort();
    }

    pub fn refresh_sort(&mut self) {
        self.apply_sort();
    }

    /// Reorder `self.layers` by the active sort column, preserving the current
    /// selection by name. No-op in document order (`sort_col == None`).
    fn apply_sort(&mut self) {
        // The row order may change below, so the cached index list is stale
        // from here on regardless of whether a sort column is active.
        self.rows_rev = self.rows_rev.wrapping_add(1);
        let Some(col) = self.sort_col else {
            return;
        };
        let asc = self.sort_asc;
        let sel_name = self
            .selected
            .and_then(|i| self.layers.get(i))
            .map(|l| l.name.clone());
        // Sort reorders rows, so the multi-selection (stored as indices) must
        // be re-resolved by name afterward or it would point at the wrong rows.
        let multi_names: Vec<String> = self
            .selected_multi
            .iter()
            .filter_map(|&i| self.layers.get(i).map(|l| l.name.clone()))
            .collect();

        use std::cmp::Ordering;
        self.layers.sort_by(|a, b| {
            let ord = match col {
                LayerSortCol::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                LayerSortCol::On => a.visible.cmp(&b.visible),
                LayerSortCol::Freeze => a.frozen.cmp(&b.frozen),
                LayerSortCol::Lock => a.locked.cmp(&b.locked),
                LayerSortCol::Plot => a.plottable.cmp(&b.plottable),
                LayerSortCol::Color => color_sort_key(a.color).cmp(&color_sort_key(b.color)),
                LayerSortCol::Linetype => a.linetype.to_lowercase().cmp(&b.linetype.to_lowercase()),
                LayerSortCol::Lineweight => a.lineweight.value().cmp(&b.lineweight.value()),
                LayerSortCol::Transparency => a.transparency.cmp(&b.transparency),
            };
            // Stable tie-break by name so equal keys keep a predictable order.
            let ord = ord.then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            if asc {
                ord
            } else {
                match ord {
                    Ordering::Less => Ordering::Greater,
                    Ordering::Greater => Ordering::Less,
                    Ordering::Equal => Ordering::Equal,
                }
            }
        });

        if let Some(n) = sel_name {
            self.selected = self.layers.iter().position(|l| l.name == n);
        }
        self.selected_multi = multi_names
            .iter()
            .filter_map(|n| self.layers.iter().position(|l| l.name == *n))
            .collect();
    }

    pub fn sync_linetypes(&mut self, items: Vec<LinetypeItem>) {
        self.linetype_combo = combo_box::State::new(items.clone());
        self.linetype_items = items;
    }

    /// Row indices passing the live name filter — the rows the view actually
    /// draws. Index math over "what the user sees" (shift-ranges) must use
    /// these; raw `selected_multi` ranges would sweep in hidden rows (#50).
    pub fn visible_indices(&self) -> Vec<usize> {
        self.row_cache
            .borrow_mut()
            .rows(&self.layers, &self.filter, self.rows_rev)
            .to_vec()
    }

    /// How many times the filtered index list has been rebuilt. A gate reads
    /// this to prove a steady frame does no scan work.
    pub fn row_cache_builds(&self) -> u32 {
        self.row_cache.borrow().build_count()
    }

    /// Whether the table currently has no row to show.
    ///
    /// Reads the same cached index list the paint uses, so the empty state and
    /// the rows cannot disagree, and a 2000-row table is not re-filtered for
    /// every message — the cache rebuilds only when the filter or the row list
    /// changes. Update-side only: `view_content` holds the cache borrow for the
    /// whole row loop and asks its own slice instead.
    pub fn is_empty(&self) -> bool {
        self.row_cache
            .borrow_mut()
            .rows(&self.layers, &self.filter, self.rows_rev)
            .is_empty()
    }

    /// The slice of filtered rows this frame builds. Everything outside it is
    /// covered by the two spacers, so the content keeps its full height.
    pub fn table_window(&self) -> crate::ui::virtual_list::Window {
        let total = {
            let mut cache = self.row_cache.borrow_mut();
            cache
                .rows(&self.layers, &self.filter, self.rows_rev)
                .len()
        };
        let viewport_h = if self.table_height > 0.0 {
            self.table_height
        } else {
            DEFAULT_TABLE_H
        };
        crate::ui::virtual_list::window(total, ROW_H, self.table_offset_y, viewport_h)
    }

    /// The Layer Manager as a dock pallet: its title bar, the toolbar across
    /// the pallet (search box on the right), then the table, at least as wide
    /// as the pallet and scrolling sideways when the pallet is narrower.
    pub fn view_panel(
        &self,
        name_col_w: f32,
        width: f32,
        chrome: crate::ui::dock::Chrome,
        theme: &Theme,
    ) -> Element<'_, Message> {
        use crate::ui::dock::PanelId;
        let title_bar = crate::ui::dock::title_bar(
            PanelId::Layers,
            crate::tr!("modal", "layer-manager"),
            chrome,
        );
        // Inside the pallet's 1 px border on either side.
        let body = self.view_content(name_col_w, (width - 2.0).max(0.0), theme);
        container(column![title_bar, body])
            .width(Length::Fixed(width))
            .height(Length::Fill)
            // The panel is the page: `Flush` paints no fill and no edge and lets
            // the dock's own `Tier::Card` plate and 1 px border show through.
            .style(|theme: &Theme| kit::wash_style(kit::Tier::Flush, theme, ""))
            .into()
    }

    /// Toolbar and table for a pallet `inner_w` wide.
    fn view_content(&self, name_col_w: f32, inner_w: f32, theme: &Theme) -> Element<'_, Message> {
        let has_sel = self.selected.is_some();
        let sel_is_zero = self
            .selected
            .map(|i| self.layers.get(i).map(|l| l.name == "0").unwrap_or(false))
            .unwrap_or(false);
        let can_set_current = self
            .selected
            .and_then(|i| self.layers.get(i))
            .is_some_and(|layer| layer.name != self.current_layer);

        // ── Toolbar ───────────────────────────────────────────────────────
        let toolbar = container(
            row![
                toolbar_btn(crate::ui::icons::PLUS, t!("New"), Message::LayerNew),
                toolbar_btn_cond(
                    crate::ui::icons::TRASH,
                    t!("Delete"),
                    Message::LayerDelete,
                    has_sel && !sel_is_zero,
                ),
                toolbar_btn_cond(
                    crate::ui::icons::CHECK,
                    t!("Set Current"),
                    Message::LayerSetCurrent,
                    can_set_current,
                ),
                iced::widget::Space::new().width(Length::Fill),
                // Search box: filters rows by name as the user types (#343).
                text_input(t!("Search…").as_ref(), &self.filter)
                    .on_input(Message::LayerManagerFilterChanged)
                    .size(FONT_SZ)
                    .padding(kit::PAD_FIELD)
                    .width(Length::Fixed(180.0))
                    .style(kit::field_style),
            ]
            .spacing(tokens::GAP_HAIRLINE)
            .align_y(iced::Center),
        )
        // The toolbar is a hole in the panel, not another strip of chrome:
        // `Sink` is the deep recess, the same rung the ribbon's tool field owns,
        // so the buttons read as plates set into it.
        .style(|theme: &Theme| kit::wash_style(kit::Tier::Sink, theme, ""))
        .width(Length::Fill)
        .padding(kit::Size::Compact.padding());

        // ── Column header ─────────────────────────────────────────────────
        let sc = self.sort_col;
        let sa = self.sort_asc;
        let mut header_row = row![
            text(t!("Status")).size(tokens::TEXT_MICRO).style(muted_style).width(COL_STATUS),
            sortable_header(
                t!("Name"),
                LayerSortCol::Name,
                Length::Fixed(name_col_w),
                sc,
                sa
            ),
            // Draggable divider: adjusts the Name column width (#359).
            iced::widget::mouse_area(kit::rule_v(14.0, "").width(Length::Fixed(2.0)))
                .on_press(Message::LayerNameColGrab)
                .interaction(iced::mouse::Interaction::ResizingHorizontally),
            sortable_header(t!("On"), LayerSortCol::On, Length::Fixed(COL_ICON), sc, sa),
            sortable_header(
                t!("Freeze"),
                LayerSortCol::Freeze,
                Length::Fixed(COL_ICON),
                sc,
                sa
            ),
            sortable_header(
                t!("Lock"),
                LayerSortCol::Lock,
                Length::Fixed(COL_ICON),
                sc,
                sa
            ),
            sortable_header(
                t!("Plot"),
                LayerSortCol::Plot,
                Length::Fixed(COL_ICON),
                sc,
                sa
            ),
            sortable_header(
                t!("Color"),
                LayerSortCol::Color,
                Length::Fixed(COL_COLOR),
                sc,
                sa
            ),
            sortable_header(
                t!("Linetype"),
                LayerSortCol::Linetype,
                Length::Fixed(COL_LT),
                sc,
                sa
            ),
            sortable_header(
                t!("Lineweight"),
                LayerSortCol::Lineweight,
                Length::Fixed(COL_LW),
                sc,
                sa
            ),
            sortable_header(
                t!("Transparency"),
                LayerSortCol::Transparency,
                Length::Fixed(COL_TRANS),
                sc,
                sa
            ),
        ]
        .spacing(tokens::GAP_TIGHT)
        .align_y(iced::Center);

        for vp in &self.vp_cols {
            header_row = header_row.push(
                text(crate::t!(&vp.label))
                    .size(tokens::TEXT_MICRO)
                    .style(muted_style)
                    .width(Length::Fixed(COL_ICON)),
            );
        }

        let col_header = container(header_row)
            // A header band meets the table on three sides, so it takes the one
            // chrome rung that draws a separating hairline and no box:
            // `strip_style` (Band fill + 1px edge, square, unlit).
            .style(|theme: &Theme| kit::strip_style(theme, ""))
            .padding(kit::Size::Compact.padding())
            .width(Length::Fill);

        // ── Layer rows ────────────────────────────────────────────────────
        // Only the rows inside the scroll viewport (plus one screen of
        // overscan) are built; zero-height spacers above and below keep the
        // content extent — and with it the scrollbar — exactly as tall as the
        // full filtered list would be.
        let win = self.table_window();
        let mut cache = self.row_cache.borrow_mut();
        let rows = cache.rows(&self.layers, &self.filter, self.rows_rev);
        // An empty table keeps the table's slot: the empty state is built in
        // the same box the scrollable would have filled, so the header band,
        // the status line and the pallet's own frame do not move.
        let no_rows = rows.is_empty();
        let mut rows_col = column![].spacing(tokens::GAP_NONE);
        if win.top_spacer > 0.0 {
            rows_col = rows_col.push(
                iced::widget::Space::new().height(Length::Fixed(win.top_spacer)),
            );
        }
        for (row, &i) in rows[win.start..win.end].iter().enumerate() {
            let layer = &self.layers[i];
            // Highlight every selected row; show the editable combos only on the
            // anchor (a single shared combo state can't drive several rows).
            let is_anchor = self.selected == Some(i);
            let is_sel = is_anchor || self.selected_multi.contains(&i);
            let is_current = layer.name == self.current_layer;
            let is_editing = self.editing.as_deref() == Some(layer.name.as_str());
            let color_open = self.color_picker_row.as_deref() == Some(layer.name.as_str());

            let (ltc, lwc) = if is_anchor {
                (Some(&self.linetype_combo), Some(&self.lw_combo))
            } else {
                (None, None)
            };

            // The row is built inside its entrance scope: every kit control
            // and glyph below is painted at this alpha, and the row itself
            // wears the same fade on its plate.
            rows_col = rows_col.push(crate::ui::kit::alpha::entrance(
                crate::ui::motion::entrance_value(
                    crate::ui::dock::entrance_keys(crate::ui::dock::PanelId::Layers),
                    row,
                ),
                || {
                    layer_row(
                        i,
                        layer,
                        is_sel,
                        is_current,
                        is_editing,
                        &self.edit_buf,
                        color_open,
                        ltc,
                        lwc,
                        &self.vp_cols,
                        name_col_w,
                        theme,
                    )
                },
            ));
        }
        if win.bottom_spacer > 0.0 {
            rows_col = rows_col.push(
                iced::widget::Space::new().height(Length::Fixed(win.bottom_spacer)),
            );
        }

        let table: Element<'_, Message> = if no_rows {
            // The no-rows case says which of the two it is: a filter that
            // matched nothing can be cleared, a table with nothing in it is
            // the drawing's own state. Both hold the scrollable's slot.
            let label = if self.filter.is_empty() {
                t!("Empty table")
            } else {
                t!("No matches")
            };
            kit::empty::state(
                crate::ui::motion::EMPTY_LAYERS,
                crate::ui::icons::themed(crate::ui::icons::INFO, kit::empty::GLYPH),
                label,
            )
        } else {
            crate::ui::kit::themed_scrollable(rows_col)
                .id(iced::advanced::widget::Id::new(LAYER_TABLE_SCROLL_ID))
                .width(Length::Fill)
                .height(Length::Fill)
                .on_scroll(Message::LayerTableScroll)
                .into()
        };

        // The header and table stretch to the wider of their columns and the
        // pallet (the zero-height spacer), so the header's grey band and
        // borders always run to the pallet's edge; a narrower pallet scrolls
        // the table sideways while the toolbar stays put.
        // A dock pallet widened past the table's own width gains the property
        // rail the manager window carries: the selected layer's attributes and
        // the drawing's layer statistics. Narrow pallets keep the table alone.
        // The rail takes precedence over a fully expanded table (the reference
        // manager shows both); below the rail's threshold the table's sideways
        // scrollbar covers the columns that no longer fit.
        let wide = inner_w >= 760.0;
        let table_w = if wide {
            (inner_w - RAIL_WIDTH - tokens::GAP_SECTION).max(220.0)
        } else {
            inner_w
        };

        let grid = column![
            col_header,
            table,
            iced::widget::Space::new()
                .width(Length::Fixed(table_w))
                .height(0),
        ]
        .height(Length::Fill);
        let grid = crate::ui::kit::themed_scrollable(grid)
            .direction(iced::widget::scrollable::Direction::Horizontal(
                iced::widget::scrollable::Scrollbar::new(),
            ))
            .width(Length::Fill)
            .height(Length::Fill);

        let anchor = self
            .selected
            .and_then(|i| self.layers.get(i))
            .or_else(|| self.layers.iter().find(|l| l.name == self.current_layer))
            .or_else(|| self.layers.first());
        let status = container(
            row![
                text(crate::tr!(
                    "layers",
                    "status-line",
                    current = self.current_layer.clone(),
                    count = self.layers.len(),
                ))
                .size(tokens::TEXT_LABEL)
                .style(muted_style),
            ]
            .width(Length::Fill),
        )
        .style(|theme: &Theme| kit::strip_style(theme, ""))
        .padding(kit::Size::Compact.padding())
        .width(Length::Fill);

        let body: Element<'_, Message> = if wide {
            row![
                container(column![toolbar, grid].height(Length::Fill)).width(Length::Fill),
                layer_rail(self, anchor),
            ]
            .spacing(tokens::GAP_SECTION)
            .height(Length::Fill)
            .into()
        } else {
            column![toolbar, grid].height(Length::Fill).into()
        };
        column![body, status].height(Length::Fill).into()
    }
}

/// The manager window's right rail: the anchor (or current) layer's attributes
/// and the drawing's layer totals. Every value reads a live `Layer` field; the
/// rail invents no per-layer object count, because the panel has no such data.
fn layer_rail<'a>(panel: &'a LayerPanel, anchor: Option<&'a Layer>) -> Element<'a, Message> {
    let card = |title: String, body: Element<'a, Message>| -> Element<'a, Message> {
        kit::surface(
            column![
                kit::group(title, ""),
                iced::widget::Space::new().height(tokens::GAP_FIELD),
                body,
            ]
            .width(Length::Fill),
            kit::Tier::Well,
            kit::Size::Compact.padding(),
            "",
        )
        .width(Length::Fill)
        .into()
    };

    let props: Element<'a, Message> = if let Some(layer) = anchor {
        let fill = iced_color_from_acad(&layer.color);
        // Same fallback the table's colour cell uses, so the rail and the row
        // never disagree on a layer's colour ("White" vs "#FFFFFF").
        let color_label = layer
            .color_name
            .clone()
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| crate::ui::color_select::color_display_name(layer.color));
        let swatch = container(
            iced::widget::Space::new()
                .width(tokens::GAP_FIELD)
                .height(tokens::GAP_FIELD),
        )
        .style(move |theme: &Theme| kit::swatch_style(theme, fill, ""));
        column![
            rail_row(t!("Name"), layer.name.clone()),
            rail_row_left(t!("Color"), row![swatch, text(color_label).size(FONT_SZ)].spacing(tokens::GAP_CONTROL).align_y(iced::Center).into()),
            rail_row(t!("Linetype"), layer.linetype.clone()),
            rail_row(t!("Lineweight"), LwItem(layer.lineweight).to_string()),
            rail_row(t!("Transparency"), format!("{}%", layer.transparency)),
            rail_row(
                t!("Plot"),
                if layer.plottable {
                    t!("Yes").to_string()
                } else {
                    t!("No").to_string()
                },
            ),
        ]
        .spacing(tokens::GAP_CONTROL)
        .width(Length::Fill)
        .into()
    } else {
        text(crate::t!("<none>"))
            .size(FONT_SZ)
            .style(muted_style)
            .into()
    };

    let on = panel.layers.iter().filter(|l| l.visible).count();
    let frozen = panel.layers.iter().filter(|l| l.frozen).count();
    let locked = panel.layers.iter().filter(|l| l.locked).count();
    let stats = column![
        rail_row(crate::tr!("layers", "total-layers"), panel.layers.len().to_string()),
        rail_row(t!("On"), on.to_string()),
        rail_row(t!("Freeze"), frozen.to_string()),
        rail_row(t!("Lock"), locked.to_string()),
    ]
    .spacing(tokens::GAP_CONTROL)
    .width(Length::Fill)
    .into();

    kit::themed_scrollable(
        column![
            card(crate::tr!("layers", "layer-properties"), props),
            iced::widget::Space::new().height(tokens::GAP_SECTION),
            card(crate::t!("Statistics").to_string(), stats),
        ]
        .width(Length::Fill),
    )
    .width(Length::Fixed(RAIL_WIDTH))
    .height(Length::Fill)
    .into()
}

/// One label/value line in the property rail, value first-class on the right.
fn rail_row<'a>(label: impl Into<String>, value: String) -> Element<'a, Message> {
    let label: String = label.into();
    row![
        text(label).size(tokens::TEXT_LABEL).style(muted_style),
        iced::widget::Space::new().width(Length::Fill),
        text(value).size(FONT_SZ),
    ]
    .width(Length::Fill)
    .align_y(iced::Center)
    .into()
}

fn rail_row_left<'a>(
    label: impl Into<String>,
    value: Element<'a, Message>,
) -> Element<'a, Message> {
    let label: String = label.into();
    row![
        text(label).size(tokens::TEXT_LABEL).style(muted_style),
        iced::widget::Space::new().width(Length::Fill),
        value,
    ]
    .width(Length::Fill)
    .align_y(iced::Center)
    .into()
}

// ── Sorting helpers ─────────────────────────────────────────────────────────

/// Packed RGB key for ordering colours deterministically by hue-ish bytes.
fn color_sort_key(c: AcadColor) -> u32 {
    let c = iced_color_from_acad(&c);
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
    (q(c.r) << 16) | (q(c.g) << 8) | q(c.b)
}

/// A clickable column header that sorts the table by `col`. Shows an up/down
/// SVG arrow when it is the active sort column (#133).
fn sortable_header<'a>(
    label: Cow<'static, str>,
    col: LayerSortCol,
    width: Length,
    active: Option<LayerSortCol>,
    asc: bool,
) -> Element<'a, Message> {
    let mut content = row![text(label).size(tokens::TEXT_MICRO).style(muted_style)]
        .spacing(tokens::GAP_TIGHT)
        .align_y(iced::Center);
    if active == Some(col) {
        content = content.push(if asc {
            crate::ui::icons::themed_arrow_up(8.0)
        } else {
            crate::ui::icons::themed_arrow_down(8.0)
        });
    }
    kit::btn(
        content,
        kit::Role::Ghost,
        kit::Size::Flush,
        kit::Press::On(Message::LayerSort(col)),
        "",
    )
    .width(width)
    .into()
}

// ── Toolbar buttons ───────────────────────────────────────────────────────

fn toolbar_btn<'a>(
    icon: &'static [u8],
    label: Cow<'static, str>,
    msg: Message,
) -> Element<'a, Message> {
    kit::btn(
        row![crate::ui::icons::themed(icon, 12.0), text(label).size(tokens::TEXT_LABEL),]
            .spacing(tokens::GAP_TIGHT)
            .align_y(iced::Center),
        kit::Role::Standard,
        kit::Size::Control,
        kit::Press::On(msg),
        "",
    )
    .into()
}

fn toolbar_btn_cond<'a>(
    icon: &'static [u8],
    label: Cow<'static, str>,
    msg: Message,
    enabled: bool,
) -> Element<'a, Message> {
    // Disabled is not an ink choice: `Press::Off` is the kit's single inert
    // state, and its `dim` step fades plate and label together.
    let press = if enabled {
        kit::Press::On(msg)
    } else {
        kit::Press::Off
    };
    kit::btn(
        row![
            if enabled {
                crate::ui::icons::themed(icon, 12.0)
            } else {
                crate::ui::icons::themed_disabled(icon, 12.0)
            },
            text(label).size(tokens::TEXT_LABEL),
        ]
        .spacing(tokens::GAP_TIGHT)
        .align_y(iced::Center),
        kit::Role::Standard,
        kit::Size::Control,
        press,
        "",
    )
    .into()
}

// ── Layer row ─────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
/// Hover popup showing a layer's full name when the cell truncates it.
fn layer_row<'a>(
    index: usize,
    layer: &'a Layer,
    is_selected: bool,
    is_current: bool,
    is_editing: bool,
    edit_buf: &'a str,
    color_picker_open: bool,
    lt_combo: Option<&'a combo_box::State<LinetypeItem>>,
    lw_combo_state: Option<&'a combo_box::State<LwItem>>,
    vp_cols: &'a [VpCol],
    name_col_w: f32,
    theme: &Theme,
) -> Element<'a, Message> {
    // #22: messages carry the layer name (resolved at dispatch time) so a
    // resort between view() and the handler cannot mistarget the row.
    let lname = layer.name.clone();
    let svg_btn = |bytes: &'static [u8], on_press: Message| -> Element<'a, Message> {
        kit::btn(
            crate::ui::icons::semantic(bytes, ICON_SZ),
            kit::Role::Ghost,
            kit::Size::Icon(ROW_H),
            kit::Press::On(on_press),
            "",
        )
        .into()
    };

    let vis_svg = crate::ui::icons::layer_visible(layer.visible);
    let frz_svg = crate::ui::icons::layer_freeze(layer.frozen);
    let lck_svg = crate::ui::icons::layer_lock(layer.locked);
    let plot_icon: Element<'_, Message> = if layer.plottable {
        crate::ui::icons::themed(crate::ui::icons::PRINT, ICON_SZ)
    } else {
        row![
            crate::ui::icons::themed(crate::ui::icons::PRINT, ICON_SZ),
            crate::ui::icons::themed_danger(crate::ui::icons::CLOSE, ICON_SZ * 0.65),
        ]
        .spacing(tokens::GAP_NONE)
        .align_y(iced::Center)
        .into()
    };

    let plot_btn: Element<'_, Message> = kit::btn(
        plot_icon,
        kit::Role::Ghost,
        kit::Size::Icon(ROW_H),
        kit::Press::On(Message::LayerTogglePlot(lname.clone())),
        "",
    )
    .into();

    let status_dot: Element<'_, Message> = if is_current {
        crate::ui::icons::themed_success(crate::ui::icons::CHECK, 13.0)
    } else {
        crate::ui::icons::themed_secondary(crate::ui::icons::DOT, 9.0)
    };

    // Name cell
    let name_cell: Element<'_, Message> = if is_editing {
        text_input("", edit_buf)
            .on_input(Message::LayerRenameEdit)
            .on_submit(Message::LayerRenameCommit)
            .size(FONT_SZ)
            .padding(Padding {
                top: COMBO_PAD_V,
                bottom: COMBO_PAD_V,
                left: 4.0,
                right: 4.0,
            })
            .style(kit::field_style)
            .width(Length::Fixed(name_col_w))
            .into()
    } else {
        // ~6 px per glyph at the 10 px row font — track the column width.
        let name_budget = ((name_col_w / 6.0) as usize).max(8);
        // A truncated name keeps its whole self on hover, so the column can
        // stay narrow without hiding what the row is called.
        kit::btn(
            crate::ui::text_util::elided(&layer.name, name_budget, theme, |shown| {
                text(shown).size(FONT_SZ).into()
            }),
            kit::Role::Ghost,
            kit::Size::Flush,
            kit::Press::On(Message::LayerRenameStart(lname.clone())),
            "",
        )
        .height(Length::Fixed(ROW_H))
        .width(Length::Fixed(name_col_w))
        .into()
    };

    // Color cell — looks like a combo_box input; click opens swatch dropdown below row.
    // Shared colour selector. Layers carry a concrete colour (no ByLayer /
    // ByBlock); true colours stay RGB instead of being collapsed to ACI 7.
    let color_cell: Element<'_, Message> =
        container(crate::ui::color_select::color_selector_with_name(
            layer.color,
            layer.color_name.as_deref(),
            color_picker_open,
            crate::ui::color_select::ColorExtras {
                by_layer: false,
                by_block: false,
                ..Default::default()
            },
            Message::LayerColorSet,
            Message::LayerColorPickerToggle(lname.clone()),
            Message::OpenColorWindow(
                crate::app::ColorPickTarget::Layer(lname.clone()),
                layer.color,
            ),
            theme,
        ))
        .width(Length::Fixed(COL_COLOR))
        .into();

    // Linetype cell — uses LinetypeItem (with ASCII art) same as Properties panel
    let cur_lt_item = LinetypeItem {
        name: layer.linetype.clone(),
        art: String::new(), // art comes from combo state items; just match by name
    };
    let lt_cell: Element<'_, Message> = if let Some(state) = lt_combo {
        crate::ui::wide_menu::wide_menu(
            combo_box(
                state,
                t!("linetype").as_ref(),
                Some(&cur_lt_item),
                |item: LinetypeItem| Message::LayerLinetypeSet(item.name),
            )
            .size(FONT_SZ)
            .padding(Padding {
                top: COMBO_PAD_V,
                bottom: COMBO_PAD_V,
                left: 4.0,
                right: 4.0,
            })
            .width(Length::Fixed(COL_LT))
            .input_style(kit::field_style),
            LINETYPE_MENU_W,
        )
    } else {
        text(layer.linetype.as_str())
            .size(FONT_SZ)
            .style(muted_style)
            .width(Length::Fixed(COL_LT))
            .into()
    };

    // Lineweight cell
    let cur_lw_item = LwItem(layer.lineweight);
    let lw_cell: Element<'_, Message> = if let Some(state) = lw_combo_state {
        combo_box(
            state,
            t!("lineweight").as_ref(),
            Some(&cur_lw_item),
            |item: LwItem| Message::LayerLineweightSet(item.0),
        )
        .size(FONT_SZ)
        .padding(Padding {
            top: COMBO_PAD_V,
            bottom: COMBO_PAD_V,
            left: 4.0,
            right: 4.0,
        })
        .width(Length::Fixed(COL_LW))
        .input_style(kit::field_style)
        .into()
    } else {
        text(cur_lw_item.to_string())
            .size(FONT_SZ)
            .style(muted_style)
            .width(Length::Fixed(COL_LW))
            .into()
    };

    // Transparency cell
    let trans_str = layer.transparency.to_string();
    let lname_trans = lname.clone();
    let trans_cell = text_input("0", &trans_str)
        .on_input(move |s| Message::LayerTransparencyEdit(lname_trans.clone(), s))
        .size(FONT_SZ)
        .padding(Padding {
            top: COMBO_PAD_V,
            bottom: COMBO_PAD_V,
            left: 4.0,
            right: 4.0,
        })
        // The row's own stripe is this field's plate, so it takes the kit's
        // plate-less field: same solved inks, no box inside the striped row.
        .style(kit::bare_field_style)
        .width(Length::Fixed(COL_TRANS));

    let mut row_content = row![
        container(status_dot)
            .width(COL_STATUS)
            .align_x(iced::Center)
            .align_y(iced::Center),
        name_cell,
        iced::widget::Space::new().width(2),
        container(svg_btn(vis_svg, Message::LayerToggleVisible(lname.clone())))
            .width(Length::Fixed(COL_ICON))
            .align_x(iced::Center),
        container(svg_btn(frz_svg, Message::LayerToggleFreeze(lname.clone())))
            .width(Length::Fixed(COL_ICON))
            .align_x(iced::Center),
        container(svg_btn(lck_svg, Message::LayerToggleLock(lname.clone())))
            .width(Length::Fixed(COL_ICON))
            .align_x(iced::Center),
        container(plot_btn)
            .width(Length::Fixed(COL_ICON))
            .align_x(iced::Center),
        color_cell,
        lt_cell,
        lw_cell,
        trans_cell,
    ]
    .spacing(tokens::GAP_TIGHT)
    .width(Fill)
    .align_y(iced::Center);

    // Per-viewport freeze columns
    for (vp_idx, _vp_col) in vp_cols.iter().enumerate() {
        let is_vp_frozen = layer.vp_frozen.get(vp_idx).copied().unwrap_or(false);
        let vp_frz_svg = crate::ui::icons::layer_vp_freeze(is_vp_frozen);
        row_content = row_content.push(
            container(svg_btn(
                vp_frz_svg,
                Message::LayerToggleVpFreeze(lname.clone(), vp_idx),
            ))
            .width(Length::Fixed(COL_ICON))
            .align_x(iced::Center),
        );
    }

    mouse_area(
        container(row_content)
            .style(move |theme: &Theme| {
                // The kit composes these two at the call site on purpose: a
                // selected row shows the app's only "this one" wash, an
                // unselected one wears its stripe.
                if is_selected {
                    kit::tint_style(kit::Tint::Selected, theme, "")
                } else {
                    kit::stripe_style(theme, index, "")
                }
            })
            .padding(Padding {
                top: 0.0,
                bottom: 0.0,
                left: 8.0,
                right: 8.0,
            })
            .height(Length::Fixed(ROW_H))
            .width(Fill),
    )
    .on_press(Message::LayerSelect(lname))
    .into()
}

// ── Display helpers ───────────────────────────────────────────────────────

pub fn iced_color_from_acad(c: &AcadColor) -> Color {
    match c {
        AcadColor::Index(i) => {
            let (r, g, b) = aci_to_rgb(*i).unwrap_or((200, 200, 200));
            Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
        }
        AcadColor::Rgb { r, g, b } => {
            Color::from_rgb(*r as f32 / 255.0, *g as f32 / 255.0, *b as f32 / 255.0)
        }
        _ => Color::WHITE,
    }
}

// ── Column widths ─────────────────────────────────────────────────────────

// Name column width is user-adjustable via the header divider drag (#359);
// the app passes the current width into `view_window`.
const COL_STATUS: f32 = 44.0;
const COL_ICON: f32 = 32.0;
const COL_COLOR: f32 = 84.0;
const COL_LT: f32 = 96.0;
const COL_LW: f32 = 80.0;
const COL_TRANS: f32 = 72.0;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::motion;

    /// 2000 empty layers — the fps gate's fixture. Built through `Layer::new`
    /// so it never touches a document (#B3 fixture).
    fn fixture(n: usize) -> LayerPanel {
        let mut panel = LayerPanel::default();
        panel.layers = (0..n)
            .map(|i| Layer::new(&format!("LAYER_{i:04}"), AcadColor::Index(7)))
            .collect();
        panel.rows_rev = panel.rows_rev.wrapping_add(1);
        panel.table_height = 700.0;
        panel
    }

    fn build_visible_rows(panel: &LayerPanel, theme: &Theme) -> usize {
        let win = panel.table_window();
        let mut cache = panel.row_cache.borrow_mut();
        let rows = cache.rows(&panel.layers, &panel.filter, panel.rows_rev);
        let mut built = 0;
        for &i in &rows[win.start..win.end] {
            let layer = &panel.layers[i];
            let _ = layer_row(
                i,
                layer,
                false,
                false,
                false,
                "",
                false,
                None,
                None,
                &panel.vp_cols,
                220.0,
                theme,
            );
            built += 1;
        }
        built
    }

    #[test]
    fn layer_table_window_2000() {
        let mut panel = fixture(2000);
        let total_h = 2000.0 * ROW_H;
        let visible = (panel.table_height / ROW_H).ceil();
        for offset in [0.0, total_h / 2.0, total_h - panel.table_height] {
            panel.table_offset_y = offset;
            let win = panel.table_window();
            assert!(
                win.built() as f32 <= 3.0 * visible + 1.0,
                "built {} rows at offset {offset}; budget is {}",
                win.built(),
                3.0 * visible
            );
            assert!(win.built() < 200, "2000 rows must never be built whole");
            let covered = win.extent(ROW_H);
            assert!(
                (covered - total_h).abs() < 0.5,
                "spacers + rows = {covered}, content must stay {total_h}"
            );
        }
        panel.table_offset_y = total_h / 2.0;
        let win = panel.table_window();
        assert!(win.start > 0 && win.end < 2000, "mid-list window is a slice");
        assert!(win.top_spacer > 0.0 && win.bottom_spacer > 0.0);
    }

    #[test]
    fn layer_filter_cache_rebuilds_only_on_change() {
        let mut panel = fixture(2000);
        let _ = panel.visible_indices();
        let after_first = panel.row_cache_builds();
        for _ in 0..100 {
            let rows = panel.visible_indices();
            assert_eq!(rows.len(), 2000);
        }
        assert_eq!(
            panel.row_cache_builds(),
            after_first,
            "100 steady frames must not rebuild the index list"
        );
        panel.filter = "layer_01".to_string();
        assert_eq!(panel.visible_indices().len(), 100);
        assert_eq!(
            panel.row_cache_builds(),
            after_first + 1,
            "one filter change, exactly one rebuild"
        );
    }

    #[test]
    fn layer_row_build_count_2000() {
        let mut panel = fixture(2000);
        let theme = Theme::Dark;
        panel.table_offset_y = 2000.0 * ROW_H / 2.0;
        let built = build_visible_rows(&panel, &theme);
        let visible = (panel.table_height / ROW_H).ceil();
        assert!(
            built as f32 <= 3.0 * visible + 1.0,
            "built {built} row widgets for a {visible}-row viewport"
        );
        assert!(built < 2000 / 10);
    }

    #[test]
    fn layer_scroll_registers_no_motion_key() {
        motion::set_enabled(true);
        let mut panel = fixture(2000);
        let theme = Theme::Dark;
        let before = motion::track_count();
        for step in 0..40 {
            panel.table_offset_y = step as f32 * 500.0;
            let _ = build_visible_rows(&panel, &theme);
        }
        assert_eq!(
            motion::track_count(),
            before,
            "scrolling must not mint a motion key per row or per frame"
        );
    }

    #[test]
    fn fps_probe_2000_scroll() {
        let mut panel = fixture(2000);
        let theme = Theme::Dark;
        let total_h = 2000.0 * ROW_H;
        let travel = total_h - panel.table_height;
        let mut samples: Vec<std::time::Duration> = Vec::with_capacity(120);
        for frame in 0..120 {
            let t = frame as f32 / 119.0;
            let eased = 0.5 - 0.5 * (std::f32::consts::PI * t).cos();
            panel.table_offset_y = travel * eased;
            let started = std::time::Instant::now();
            // The real frame path: the whole panel body, not just the window
            // arithmetic. Building all 2000 rows here would blow the budget.
            std::hint::black_box(panel.view_content(220.0, 900.0, &theme));
            samples.push(started.elapsed());
        }
        samples.sort();
        let p95 = samples[(samples.len() as f32 * 0.95) as usize];
        let worst = *samples.last().unwrap();
        println!(
            "fps_probe_2000: p95 = {:.2} ms, worst = {:.2} ms, {} frames",
            p95.as_secs_f64() * 1000.0,
            worst.as_secs_f64() * 1000.0,
            samples.len()
        );
        assert!(
            p95 <= std::time::Duration::from_millis(20),
            "p95 {p95:?} over budget"
        );
        assert!(
            worst <= std::time::Duration::from_millis(50),
            "one frame took {worst:?}"
        );
    }

    /// The paint picks the empty state from the same index list it draws, so
    /// the two cannot disagree about whether there is a row to show.
    #[test]
    fn the_empty_state_reads_the_same_rows_the_table_paints() {
        let mut panel = fixture(3);
        assert!(!panel.is_empty(), "three layers are three rows");

        // A filter that keeps one row is not empty; one that drops all of
        // them is, and the cache must notice the filter change on its own.
        panel.filter = "LAYER_0001".to_string();
        assert!(!panel.is_empty());
        panel.filter = "NO SUCH LAYER".to_string();
        assert!(panel.is_empty(), "a dead filter leaves no rows");

        panel.filter.clear();
        panel.layers.clear();
        assert!(
            panel.is_empty(),
            "a document with no layers has an empty table even unfiltered"
        );
    }
}
