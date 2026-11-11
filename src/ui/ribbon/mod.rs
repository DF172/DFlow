// Ribbon — tab bar + 3-row tool area.
//
// Button sizes:
//   LargeTool / LargeDropdown  — full ribbon height (3 rows), icon + label [+ ▾]
//   Tool / Dropdown            — 1-row height, icon only [+ ▾ on right]
//
// Dropdown items within a group are collected into columns of 3 rows.

use crate::ui::kit;
use crate::ui::kit::{style_for, Role};
use crate::ui::style::tokens;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use rustc_hash::FxHashMap as HashMap;

use codec::types::{Color as AcadColor, LineWeight};
use iced::widget::{column, container, mouse_area, row, text, Space};
use crate::ui::kit::plain_button;
use iced::{Background, Color, Element, Fill, Length, Padding, Theme};

use crate::app::Message;
use crate::modules::{CadModule, IconKind, ModuleEvent, RibbonGroup, RibbonItem, ToolDef};
use crate::ui::properties::{linetype_display_name, lw_options, LinetypeItem};

mod draw_panel;
pub(crate) mod widgets;
pub(crate) use draw_panel::tools as panel_tools;
mod color_dropdown;
mod context_tools;
mod modify_panel;
pub use context_tools::{pdf_underlay_tools, point_cloud_tools, xref_tools, UnderlayContext};
use widgets::{StyleContext, *};
pub(crate) use widgets::{REDO_HISTORY_ID, UNDO_HISTORY_ID};
mod collapse;
mod locate;
use crate::t;
use crate::ui::wrap_bar::{PosReport, WrapBar, WrapFlow};
pub use collapse::CollapseMode;
use collapse::{CollapsePanels, Panel};

pub(crate) fn tooltip_content(text: String) -> Element<'static, Message> {
    widgets::make_tip(text)
}

// A tip is the ink only: the plate it sits on is a glass pane, applied by the
// renderer that can hand a theme to it (`app::view::controls`, `ribbon::widgets`).
// The ribbon exports the ink under this name so the viewport strip and the ribbon
// strip cannot drift into two different tip typefaces.

/// Alpha of the shield the Start tab lays over the tool field. Not a plane:
/// the page showing through at partial alpha is what keeps the recessed band
/// reading as recessed.
const RIBBON_SCRIM_A: f32 = 0.62;

/// The 1px rule the sheet draws between the ribbon's bands. Vendor themes
/// separate bands by plane contrast instead, so this paints nothing there
/// rather than stacking a second edge on a difference that already reads.
pub fn ribbon_hairline<'a>() -> Element<'a, Message> {
    kit::sheet_rule()
}

/// The vertical twin of [`ribbon_hairline`], standing between two panels.
fn ribbon_group_rule<'a>() -> Element<'a, Message> {
    kit::sheet_rule_v()
}

// ── Ribbon state ───────────────────────────────────────────────────────────

pub struct Ribbon {
    modules: Vec<Box<dyn CadModule>>,
    active: usize,
    active_tool: Option<String>,
    pub wireframe: bool,
    pub ortho_mode: bool,
    /// ViewCube (NAVVCUBE) visibility — drives the View Cube button highlight.
    pub show_viewcube: bool,
    /// UCS icon (UCSICON) visibility — drives the UCS Icon button highlight.
    pub show_ucs_icon: bool,
    /// Properties panel (PROPERTIES) visibility — drives the Properties button highlight.
    pub show_properties: bool,
    /// The Count palette is open (View › Palettes › Count).
    pub show_count_palette: bool,
    /// Sheet Set Manager palette shown (SSMSTATE).
    pub show_sheet_set: bool,
    /// Graphic Attributes panel visibility — drives its ribbon button highlight.
    pub show_graphic_attributes: bool,
    /// File tabs (FILETAB) visibility — drives the File Tabs button highlight.
    pub show_file_tabs: bool,
    /// Layout tabs (LAYOUTTAB) visibility — drives the Layout Tabs button highlight.
    pub show_layout_tabs: bool,
    pub open_dropdown: Option<String>,
    /// Title of the collapsed panel whose flyout is currently open, if any.
    pub collapsed_open: Option<String>,
    /// Last tool used from each panel (panel title → tool id). A collapsed panel
    /// shows this tool on its button, defaulting to the panel's first tool.
    last_panel_tool: HashMap<&'static str, &'static str>,
    last_cmd: HashMap<&'static str, &'static str>,
    pub active_layer: String,
    pub layer_infos: Vec<LayerInfo>,
    /// Live filter for the layer dropdown's search box (#343). Cleared on
    /// close so the next open starts unfiltered.
    pub layer_filter: String,
    /// Active object color — ACI / ByLayer / ByBlock.
    pub active_color: AcadColor,
    /// Active linetype override ("ByLayer", "Continuous", …).
    pub active_linetype: String,
    /// Active lineweight.
    pub active_lineweight: LineWeight,
    /// Linetypes loaded from the current document (with ASCII art).
    pub available_linetypes: Vec<LinetypeItem>,
    /// Whether the full ACI palette is expanded inside the color picker overlay.
    pub prop_color_palette_open: bool,
    // ── Style selector state ──────────────────────────────────────────────
    pub text_style_names: Vec<String>,
    pub active_text_style: String,
    pub dim_style_names: Vec<String>,
    pub active_dim_style: String,
    pub mleader_style_names: Vec<String>,
    pub active_mleader_style: String,
    pub table_style_names: Vec<String>,
    pub active_table_style: String,
    /// Measured tab-bar height (28 on one row, 56 when tabs wrap). Written by
    /// the `WrapBar` widget during layout, read when anchoring dropdowns.
    tab_bar_h: Arc<AtomicU32>,
    /// Measured tool-area height (TOOL_BAR_H at full density, shorter once every
    /// panel is collapsed). Written by `CollapsePanels`, read when anchoring
    /// dropdowns below the ribbon.
    tool_bar_h: Arc<AtomicU32>,
    /// User-chosen panel density (persisted). `Auto` sizes by window width.
    collapse_mode: CollapseMode,
    /// Set by `CollapsePanels` when the tool row is in its tight state; the mode
    /// selector hides itself then to give the cramped tab row its space back.
    collapse_tight: Arc<AtomicBool>,
    /// The selected PDF underlay's switches while only underlays are selected.
    underlay_ctx: Option<UnderlayContext>,
    /// Only xrefs are selected.
    xref_ctx: bool,
    /// Only point clouds are selected: whether the first shows its crops.
    point_cloud_ctx: Option<bool>,
    /// XDWGFADECTL as the Reference slide-out shows it (negative = off).
    pub xref_fade: i32,
}

/// Per-layer display data shown in the ribbon layer dropdown.
#[derive(Clone, Debug)]
pub struct LayerInfo {
    pub name: String,
    pub color: Color,
    pub visible: bool,
    pub frozen: bool,
    pub locked: bool,
    /// Layout with an active viewport: (its Layer Manager column, frozen there).
    pub vp_frozen: Option<(usize, bool)>,
}

/// Full-screen backdrop for an open ribbon dropdown: closes the dropdown when
/// the user clicks outside the panel. (Cursor motion leaking to the viewport
/// beneath is blocked in `on_viewport_move`, not here — iced 0.14's mouse_area
/// can only capture button presses, never CursorMoved. #227.)
fn dropdown_backdrop<'a>(positioned: Element<'a, Message>) -> Element<'a, Message> {
    mouse_area(positioned)
        .on_press(Message::CloseRibbonDropdown)
        .into()
}

/// Position a ribbon dropdown `panel` in a full-window container just below its
/// widget, centred on it (see [`Ribbon::dd_anchor`]).
fn position_ribbon_dropdown<'a>(
    panel: Element<'a, Message>,
    left: f32,
    top: f32,
) -> Element<'a, Message> {
    let pad = Padding {
        top,
        bottom: 0.0,
        left,
        right: 0.0,
    };
    container(panel)
        .align_top(Fill)
        .width(Fill)
        .height(Fill)
        .padding(pad)
        .align_left(Fill)
        .into()
}

/// Left edge for a `panel_w` panel centred under a button whose centre is
/// `centre_x`. The panel never crosses the window (4 px off the left edge,
/// 2 px off the right); a panel wider than the window starts at 4 px.
fn dropdown_left(centre_x: f32, panel_w: f32, win_w: f32) -> f32 {
    let max_left = (win_w - panel_w - 2.0).max(4.0);
    (centre_x - panel_w / 2.0).clamp(4.0, max_left)
}

impl Ribbon {
    pub fn new() -> Self {
        Self {
            modules: crate::modules::registry::all_modules(),
            active: 0,
            active_tool: None,
            wireframe: false,
            ortho_mode: true,
            show_viewcube: true,
            show_ucs_icon: true,
            show_properties: true,
            show_count_palette: false,
            show_sheet_set: false,
            show_graphic_attributes: false,
            show_file_tabs: true,
            show_layout_tabs: true,
            open_dropdown: None,
            collapsed_open: None,
            last_panel_tool: HashMap::default(),
            last_cmd: HashMap::default(),
            // Empty until a document is open — populated by sync_ribbon_layers.
            active_layer: String::new(),
            layer_infos: vec![],
            layer_filter: String::new(),
            active_color: AcadColor::ByLayer,
            active_linetype: "ByLayer".to_string(),
            active_lineweight: LineWeight::ByLayer,
            available_linetypes: vec![LinetypeItem {
                name: "Continuous".to_string(),
                art: String::new(),
            }],
            prop_color_palette_open: false,
            // Empty until a document is open — the Annotate style dropdowns
            // are populated from the active document by sync_ribbon_styles.
            text_style_names: vec![],
            active_text_style: String::new(),
            dim_style_names: vec![],
            active_dim_style: String::new(),
            mleader_style_names: vec![],
            active_mleader_style: String::new(),
            table_style_names: vec![],
            active_table_style: String::new(),
            tab_bar_h: Arc::new(AtomicU32::new(TAB_BAR_H.to_bits())),
            tool_bar_h: Arc::new(AtomicU32::new(TOOL_BAR_H.to_bits())),
            collapse_mode: CollapseMode::default(),
            collapse_tight: Arc::new(AtomicBool::new(false)),
            underlay_ctx: None,
            xref_ctx: false,
            point_cloud_ctx: None,
            xref_fade: 50,
        }
    }

    /// Change the tool-panel density. Persistence is handled by the caller via
    /// the consolidated app config (`save_config`).
    pub fn set_collapse_mode(&mut self, mode: CollapseMode) {
        self.collapse_mode = mode;
    }

    /// The current tool-panel density (for saving into the app config).
    pub fn collapse_mode(&self) -> CollapseMode {
        self.collapse_mode
    }

    /// Current tab-bar height as last measured by the `WrapBar` widget.
    fn tab_bar_height(&self) -> f32 {
        f32::from_bits(self.tab_bar_h.load(Ordering::Relaxed))
    }

    /// Current tool-area height as last measured by the `DensitySwap` widget.
    fn tool_bar_height(&self) -> f32 {
        f32::from_bits(self.tool_bar_h.load(Ordering::Relaxed))
    }

    /// `(left, top)` to anchor the open dropdown `id`'s overlay just below its
    /// widget. The panel hangs from the button's *centre*, not from its left
    /// edge, so a wide list reads as belonging to the button above it; it is
    /// clamped so it never leaves the window. Falls back to the window centre
    /// under the ribbon when the widget's position hasn't been recorded yet.
    ///
    /// `body_w` is the width the panel's own content declares. Every dropdown
    /// body is wrapped in `kit::popup_plate`, which insets it by
    /// `GAP_CONTROL` on every side, so the *sheet* is `2 · GAP_CONTROL` wider
    /// than the body. Centring the body's width alone would push the visible
    /// paper half an inset to the right of the button — the exact drift this
    /// anchor exists to remove.
    fn dd_anchor(&self, id: &str, body_w: f32, win_w: f32) -> (f32, f32) {
        self.dd_anchor_outer(id, body_w + 2.0 * tokens::GAP_CONTROL, win_w)
    }

    /// [`Self::dd_anchor`] for a panel that carries its own surface and is
    /// placed at its full width (the block gallery), so no sheet inset is
    /// added.
    fn dd_anchor_outer(&self, id: &str, outer_w: f32, win_w: f32) -> (f32, f32) {
        match crate::ui::wrap_bar::dropdown_bounds(id) {
            Some(b) => (
                dropdown_left(b.center_x(), outer_w, win_w),
                b.y + b.height + tokens::GAP_HAIRLINE,
            ),
            None => (
                dropdown_left(win_w / 2.0, outer_w, win_w),
                self.tab_bar_height() + self.tool_bar_height(),
            ),
        }
    }

    pub fn set_styles(
        &mut self,
        text: Vec<String>,
        active_text: &str,
        dim: Vec<String>,
        active_dim: &str,
        mleader: Vec<String>,
        active_mleader: &str,
        table: Vec<String>,
        active_table: &str,
    ) {
        self.text_style_names = text;
        self.active_text_style = active_text.to_string();
        self.dim_style_names = dim;
        self.active_dim_style = active_dim.to_string();
        self.mleader_style_names = mleader;
        self.active_mleader_style = active_mleader.to_string();
        self.table_style_names = table;
        self.active_table_style = active_table.to_string();
    }

    pub fn set_layers(&mut self, infos: Vec<LayerInfo>, active: &str) {
        self.active_layer = active.to_string();
        self.layer_infos = infos;
    }

    pub fn set_available_linetypes(&mut self, items: Vec<LinetypeItem>) {
        self.available_linetypes = items;
    }

    pub fn select(&mut self, index: usize) {
        if index < self.modules.len() {
            self.active = index;
        }
    }

    /// Replace the tab list. Clamps the active tab so it stays in range.
    pub fn set_modules(&mut self, modules: Vec<Box<dyn CadModule>>) {
        self.modules = modules;
        if self.active >= self.modules.len() {
            self.active = self.modules.len().saturating_sub(1);
        }
        self.release_tool_glow();
        self.open_dropdown = None;
    }
    pub fn activate_tool(&mut self, id: &str) {
        if self.active_tool.as_deref() != Some(id) {
            self.release_tool_glow();
        }
        self.active_tool = Some(id.to_string());
        crate::ui::motion::pulse(
            crate::ui::motion::glow_key_any(id),
            1.0,
            crate::ui::motion::REST_GLOW,
            crate::ui::motion::PULSE_RISE,
            crate::ui::motion::PULSE_FALL,
        );
    }
    #[cfg(test)]
    pub(crate) fn active_tool(&self) -> Option<&str> {
        self.active_tool.as_deref()
    }
    /// Hand the previous tool's glow track off before `active_tool` changes
    /// hands. A track has exactly one owner, and motion tracks are never
    /// pruned: an unreleased one parks at `REST_GLOW`, which `tool_btn_style`
    /// reads as *engaged* — so the button stays lit for the whole session
    /// even though it no longer runs anything.
    fn release_tool_glow(&mut self) {
        if let Some(id) = self.active_tool.take() {
            crate::ui::motion::set(
                crate::ui::motion::glow_key_any(&id),
                0.0,
                crate::ui::motion::NORMAL,
            );
        }
    }
    pub fn deactivate_tool(&mut self) {
        self.release_tool_glow();
    }
    /// Clear `active_tool` only when it currently equals `id`. Used by the
    /// window-close path to deactivate the tool that owned a popup window
    /// without disturbing a different tool the user picked in the
    /// meantime. See #40.
    pub fn deactivate_tool_if(&mut self, id: &str) {
        if self.active_tool.as_deref() == Some(id) {
            self.release_tool_glow();
        }
    }
    pub fn set_wireframe(&mut self, w: bool) {
        self.wireframe = w;
    }
    pub fn set_ortho(&mut self, ortho: bool) {
        self.ortho_mode = ortho;
    }
    pub fn set_viewcube(&mut self, on: bool) {
        self.show_viewcube = on;
    }
    pub fn set_ucs_icon(&mut self, on: bool) {
        self.show_ucs_icon = on;
    }
    pub fn set_properties(&mut self, on: bool) {
        self.show_properties = on;
    }
    pub fn set_count_palette(&mut self, on: bool) {
        self.show_count_palette = on;
    }
    pub fn set_sheet_set(&mut self, on: bool) {
        self.show_sheet_set = on;
    }
    pub fn set_graphic_attributes(&mut self, on: bool) {
        self.show_graphic_attributes = on;
    }
    pub fn set_file_tabs(&mut self, on: bool) {
        self.show_file_tabs = on;
    }
    pub fn set_layout_tabs(&mut self, on: bool) {
        self.show_layout_tabs = on;
    }
    /// Snapshot of every ribbon toggle's live state, for the render path.
    /// The Block Palette highlight is threaded in from the app's authoritative
    /// `show_block_palette` rather than a ribbon copy, so it can't drift from
    /// the panel's true visibility.
    fn toggle_state(&self, show_block_palette: bool) -> widgets::ToggleState {
        use widgets::ToggleState;
        ToggleState {
            ortho_mode: self.ortho_mode,
            show_viewcube: self.show_viewcube,
            show_ucs_icon: self.show_ucs_icon,
            show_properties: self.show_properties,
            show_count_palette: self.show_count_palette,
            show_graphic_attributes: self.show_graphic_attributes,
            show_block_palette,
            show_sheet_set: self.show_sheet_set,
            show_file_tabs: self.show_file_tabs,
            show_layout_tabs: self.show_layout_tabs,
        }
    }

    /// One panel in its four densities; `lead` adds controls before the
    /// group's own tools. `first` is true for the leftmost panel, which has no
    /// rule on its left edge.
    fn panel<'a>(
        &'a self,
        first: bool,
        g: &'a RibbonGroup,
        ts: widgets::ToggleState,
        style_ctx: &StyleContext<'_>,
        theme: &'a Theme,
        lead: &dyn Fn(bool) -> Vec<Element<'a, Message>>,
    ) -> Panel<'a> {
        let group = |compact: bool| {
            render_group(
                first,
                compact,
                g,
                lead(compact),
                &self.active_tool,
                &self.open_dropdown,
                &self.last_cmd,
                ts,
                &self.layer_infos,
                &self.active_layer,
                self.active_color,
                &self.active_linetype,
                self.active_lineweight,
                style_ctx,
                theme,
            )
        };
        let collapsed = |tight: bool| {
            collapse_button(
                g,
                self.last_panel_tool.get(g.title).copied(),
                &self.active_tool,
                &self.open_dropdown,
                &self.last_cmd,
                ts,
                &self.layer_infos,
                &self.active_layer,
                self.active_color,
                &self.active_linetype,
                self.active_lineweight,
                style_ctx,
                theme,
                tight,
            )
        };
        Panel {
            id: g.title.to_string(),
            elements: [group(false), group(true), collapsed(false), collapsed(true)],
        }
    }

    pub fn toggle_dropdown(&mut self, id: &str) {
        if self.open_dropdown.as_deref() == Some(id) {
            self.open_dropdown = None;
        } else {
            self.open_dropdown = Some(id.to_string());
            if draw_panel::owns_dropdown(id) {
                self.collapsed_open = None;
            }
        }
    }
    pub fn close_dropdown(&mut self) {
        self.open_dropdown = None;
        self.collapsed_open = None;
        self.layer_filter.clear();
    }

    pub fn escape_extension(&mut self) -> bool {
        let Some(id) = self.open_dropdown.as_deref() else {
            return false;
        };
        let Some(parent) = draw_panel::parent_panel(id) else {
            return false;
        };
        if id == parent {
            self.close_dropdown();
        } else {
            self.open_dropdown = Some(parent.to_string());
        }
        true
    }

    /// Toggle the flyout of a collapsed ribbon panel (identified by its title).
    pub fn toggle_collapsed_panel(&mut self, id: &str) {
        if self.collapsed_open.as_deref() == Some(id) {
            self.collapsed_open = None;
        } else {
            self.collapsed_open = Some(id.to_string());
        }
    }

    /// Record `tool_id` as the last-used tool of whichever active-module panel
    /// contains it, so a collapsed panel shows that tool on its button.
    pub fn note_panel_tool(&mut self, tool_id: &str) {
        if let Some(module) = self.modules.get(self.active) {
            for group in module.ribbon_groups() {
                if let Some(id) = group
                    .tools
                    .iter()
                    .filter_map(item_id)
                    .find(|id| *id == tool_id)
                {
                    self.last_panel_tool.insert(group.title, id);
                    return;
                }
            }
        }
    }

    /// Returns the index of the Layout module in the modules list.
    #[allow(dead_code)] // layout-tab helpers; not yet wired
    pub fn layout_module_index(&self) -> Option<usize> {
        self.modules.iter().position(|m| m.id() == "layout")
    }

    /// Returns true if the currently active tab is the Layout module.
    #[allow(dead_code)]
    pub fn active_is_layout(&self) -> bool {
        self.modules
            .get(self.active)
            .map(|m| m.id() == "layout")
            .unwrap_or(false)
    }

    pub fn select_dropdown_item(&mut self, dropdown_id: &'static str, cmd: &'static str) {
        self.last_cmd.insert(dropdown_id, cmd);
        self.close_dropdown();
    }

    /// Show `cmd` as the current item of a dropdown that mirrors a drawing
    /// setting, without running it.
    pub fn set_dropdown_current(&mut self, dropdown_id: &'static str, cmd: &'static str) {
        if self.last_cmd.get(dropdown_id) != Some(&cmd) {
            self.last_cmd.insert(dropdown_id, cmd);
        }
    }

    // ── View ──────────────────────────────────────────────────────────────

    /// The two chrome rows the host stacks around the document tab strip: the
    /// application bar (tabs, file commands, live state) and the tool row. The
    /// sheet draws them as two separate fields with the document strip between
    /// them; the vendor themes keep the banded ribbon and join them with
    /// hairlines at the call site.
    pub fn chrome<'a>(
        &'a self,
        is_paper: bool,
        is_start: bool,
        undo_count: usize,
        redo_count: usize,
        show_block_palette: bool,
        running: Option<&'a str>,
        objects: usize,
        // Index of the start page among the open documents, so its entry can
        // live in the tab row the way the sheet draws it.
        start_tab: Option<usize>,
        theme: &'a Theme,
    ) -> (Element<'a, Message>, Element<'a, Message>) {
        // ── Quick-access file commands + undo/redo, one merged flow ────────
        let sheet = crate::ui::style::draftflow_theme::draftflow_colors(theme).is_some();
        // The native window caption already carries the product name and its
        // version, so the bar does not repeat them: the tabs take the head.

        let lead = iced::widget::Row::with_children(vec![
            quick_access_btn(crate::ui::icons::DOC_NEW, "New", "QNEW", theme).into(),
            quick_access_btn(crate::ui::icons::FOLDER_OPEN, "Open", "OPEN", theme).into(),
            quick_access_btn(crate::ui::icons::SAVE, "Save", "SAVE", theme).into(),
            quick_access_btn(crate::ui::icons::FILE_EXPORT, "Save As", "SAVEAS", theme).into(),
            quick_access_btn(crate::ui::icons::PRINT, "Print", "PRINT", theme).into(),
            render_history_control("Undo", UNDO_HISTORY_ID, undo_count, &self.open_dropdown, theme)
                .into(),
            render_history_control("Redo", REDO_HISTORY_ID, redo_count, &self.open_dropdown, theme)
                .into(),
            quick_access_btn(
                crate::ui::icons::THEME_TOGGLE,
                "Theme",
                "COLORSCHEME TOGGLE",
                theme,
            )
            .into(),
        ])
        .spacing(TOP_HIST_GAP)
        .align_y(iced::Center)
        .wrap()
        .vertical_spacing(0.0);

        // The quick-access flow and the tabs flow each flex-wrap; WrapBar stacks
        // them so a wrapped tab never shares a row with a quick-access button.

        // A running command whose button is on another tab tints that tab's
        // header with a faint wash of the button's blue, pointing at where the
        // button lives without competing with the selected tab's outline.
        let holding_tab = self.tab_holding_active_tool();
        let mut tab_items = self.modules.iter().enumerate().fold(
            Vec::<Element<'_, Message>>::new(),
            |mut acc, (i, module)| {
                // The Layout module no longer has a ribbon tab — its paper-space
                // tools live in the right-edge side toolbar (see ui::side_toolbar).
                if module.id() == "layout" {
                    return acc;
                }

                let is_active = i == self.active;
                let holds_tool = holding_tab == Some(i);
                let tab_key = crate::ui::motion::indexed_key("tab:", i);
                // The sheet marks the current tab with a 2px ink rule under the
                // label, not a filled plate: the strip is chrome and the ink is
                // the same pen the sheet is drawn with. The rule rides inside
                // the button, under the label, so its width is the tab's own
                // width rather than the row's. It must not be a `Fill` child:
                // measured at an unbounded width (the wrap bar's natural pass)
                // a Fill child reports an infinite tab, which pushes the wrap
                // bar to give every tab a row of its own.
                let face = container(
                    text(crate::i18n::ribbon_module_title(module.id(), module.title()))
                        .size(tokens::TEXT_BODY)
                        .font(tokens::UI_FONT_MEDIUM),
                )
                .padding(kit::PAD_TAB);
                // The current tab is a pill card (see `kit::tab_plate_style`),
                // so the label rides without the sheet's under-rule: the plate
                // is the signal and a second mark under it would double it.
                let btn = plain_button(kit::rule_under(face, false))
                    .on_press(Message::RibbonSelectTab(i))
                    .style(move |theme: &Theme, status| {
                        if let Some(style) =
                            kit::tab_plate_style(theme, status, is_active, holds_tool)
                        {
                            return style;
                        }
                        // The selected tab is the kit's engaged plate, a tab
                        // holding a running command is the same plate at wash
                        // strength, an idle tab is a ghost. Only the selected tab
                        // gets the motion track: `tab:` ramps when a tab becomes
                        // current, so its ring and halo fade in over NORMAL instead
                        // of jump-cutting with the click, while a `holds_tool` tab
                        // must not ride a track that reads zero.
                        let role = if is_active || holds_tool {
                            Role::Engaged
                        } else {
                            Role::Ghost
                        };
                        let mut style = style_for(
                            theme,
                            status,
                            role,
                            if is_active { tab_key } else { "" },
                        );
                        if is_active {
                            style.border.width = TAB_RING_W;
                            // A faint brand halo lifts the selected tab off the
                            // strip while its ring fades in.
                            style.shadow = iced::Shadow {
                                color: tokens::aurora_lo(theme)
                                    .scale_alpha(TAB_HALO_A * crate::ui::motion::value(tab_key)),
                                ..tokens::elevation(theme, tokens::LIFT_HOVER)
                            };
                        } else if holds_tool {
                            style.background = Some(tokens::aurora_gradient(theme, HOLDS_TOOL_WASH));
                        }
                        style
                    })
                    .padding(Padding::ZERO);
                acc.push(btn.into());
                acc
            },
        );

        // The start page is a destination of the tab row, not a document tab:
        // the sheet lists it first, beside the ribbon modules, so its entry
        // never scrolls away with the open documents.
        if sheet {
            if let Some(start) = start_tab {
                let face = container(
                    text(t!("Start"))
                        .size(tokens::TEXT_BODY)
                        .font(tokens::UI_FONT_MEDIUM),
                )
                .padding(kit::PAD_TAB);
                let start_btn = plain_button(kit::rule_under(face, false))
                    .on_press(Message::TabSwitch(start))
                    .style(move |theme: &Theme, status| {
                        if let Some(style) = kit::tab_plate_style(theme, status, is_start, false)
                        {
                            return style;
                        }
                        style_for(
                            theme,
                            status,
                            if is_start {
                                Role::Engaged
                            } else {
                                Role::Ghost
                            },
                            "",
                        )
                    })
                    .padding(Padding::ZERO);
                tab_items.insert(0, start_btn.into());
            }
        }

        // Tabs may squeeze their gaps to fit before wrapping: from the normal 6px
        // down to -12px on a narrow (e.g. phone) tab row, tucking neighbours into
        // each other's 14px side padding without overlapping the labels. When the
        // row has room the gap stays 6px, so wide/desktop layouts are unchanged.
        let tabs = WrapFlow::new(tab_items)
            .spacing_x(tokens::GAP_CONTROL)
            .min_spacing_x(-(tokens::PAD_TAB_X - tokens::GAP_HAIRLINE))
            .row_h(TAB_BAR_H);

        // Panel-density selector, pinned to the right edge of the tab row: a bare
        // ▾ button that opens a list of modes (see `dropdown_overlay`). `Auto`
        // sizes panels to the window; the others force one density. The choice is
        // persisted (see `Ribbon::set_collapse_mode`). It hides itself once the
        // tool row is tight, giving the cramped tab row its space back.
        let dd_open = self.open_dropdown.as_deref() == Some(COLLAPSE_MODE_ID);
        let mode_btn = plain_button(crate::ui::icons::themed_arrow_down(10.0))
            .on_press(Message::ToggleRibbonDropdown(COLLAPSE_MODE_ID.to_string()))
            .style(move |theme: &Theme, status| style_for(theme, status, combo_role(dd_open), ""))
            .height(24)
            .padding(kit::PAD_GLYPH);
        let mode_dd = PosReport::new(COLLAPSE_MODE_ID, mode_btn);

        let mut tab_row = row![];
        if sheet {
            // The sheet's bar is three fields: the destinations at the head,
            // the file commands in the middle, and the measured state at the
            // tail. Strip and tail share one row instead of the strip floating
            // over a full-width head: a narrow window then squeezes the strip's
            // free space (and the tabs tuck their gaps), but the tab labels can
            // never be drawn on top of the file commands' icons.
            let strip_flow: Element<'_, Message> = tabs.into();
            let strip = container(container(strip_flow).width(Length::Shrink))
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center);
            let mut head = row![].spacing(tokens::GAP_CONTROL).align_y(iced::Center);
            head = head.push(strip);
            head = head.push(lead);
            if !self.collapse_tight.load(Ordering::Relaxed) {
                head = head.push(mode_dd);
            }
            let (state, live) = match running {
                Some(cmd) => (cmd.to_owned(), true),
                None => (t!("Ready").into_owned(), false),
            };
            head = head
                .push(kit::state_pill(theme, state, live))
                .push(kit::state_pill(
                    theme,
                    format!("{} {}", objects, t!("Objects")),
                    false,
                ));
            tab_row = row![head].width(Length::Fill);
        } else {
            tab_row = tab_row
                .push(
                    container(
                        WrapBar::new(lead.into(), tabs.into())
                            .spacing(tokens::GAP_CONTROL)
                            .report_height(self.tab_bar_h.clone()),
                    )
                    .width(Length::Fill),
                )
                .spacing(tokens::GAP_CONTROL)
                .align_y(iced::Center);
            if !self.collapse_tight.load(Ordering::Relaxed) {
                tab_row = tab_row.push(mode_dd);
            }
        }

        // On the sheet the ribbon is one floating paper: the tab strip and the
        // tool field share a single card, so neither paints a band of its own —
        // the card behind them is the plate, and the rules that used to divide
        // the two fields would draw box seams inside it. Other themes keep the
        // banded chrome and its hairlines.
        // `sheet` is resolved at the head of this function: the bar branch and the
        // card branch must agree on which theme family they are drawing.
        let tab_bar = kit::surface(
            tab_row,
            if sheet {
                kit::Tier::Flush
            } else {
                kit::Tier::Band
            },
            Padding {
                right: tokens::GAP_CONTROL,
                ..Padding::ZERO
            },
            "",
        )
        .width(Length::Fill);

        // ── Tool area ─────────────────────────────────────────────────────
        let effective_active = if !is_paper
            && self
                .modules
                .get(self.active)
                .map(|m| m.id() == "layout")
                .unwrap_or(false)
        {
            0
        } else {
            self.active
        };
        let tool_area: Element<'_, Message> =
            if let Some(module) = self.modules.get(effective_active) {
                let groups = module.ribbon_groups();
                let style_ctx = StyleContext {
                    text_style_names: &self.text_style_names,
                    active_text_style: &self.active_text_style,
                    dim_style_names: &self.dim_style_names,
                    active_dim_style: &self.active_dim_style,
                    mleader_style_names: &self.mleader_style_names,
                    active_mleader_style: &self.active_mleader_style,
                    table_style_names: &self.table_style_names,
                    active_table_style: &self.active_table_style,
                };
                let ts = self.toggle_state(show_block_palette);

                if sheet {
                    // The sheet's tool area is one bare row: icon over name,
                    // no group titles, no rules. It carries the drawing's own
                    // tools (this CAD's command set), not the reference's
                    // sample set — the row is a house for our tools, not a
                    // copy of theirs.
                    let ctx = widgets::RenderCtx {
                        active_tool: &self.active_tool,
                        theme,
                        open_dd: &self.open_dropdown,
                        last_cmd: &self.last_cmd,
                        state: ts,
                        layer_infos: &self.layer_infos,
                        active_layer: &self.active_layer,
                        active_color: self.active_color,
                        active_linetype: &self.active_linetype,
                        active_lineweight: self.active_lineweight,
                        style_ctx: &style_ctx,
                        compact: false,
                    };
                    let items = sheet_strip_items(module.id(), groups, &ctx);
                    let strip = items.into_iter().fold(
                        row![].spacing(tokens::GAP_CONTROL).align_y(iced::Center),
                        |r, e| r.push(e),
                    );
                    // The band's lower edge sits one control gap higher than
                    // its upper edge: the strip pulls its baseline up 6 px
                    // without shrinking STRIP_H, so the icon, both label lines
                    // and the dropdown caret keep their full room.
                    container(strip)
                        .width(Length::Fill)
                        .padding(Padding {
                            top: tokens::GAP_CONTROL,
                            right: tokens::GAP_SECTION,
                            bottom: tokens::GAP_NONE,
                            left: tokens::GAP_SECTION,
                        })
                        .into()
                } else {
                    // Adaptive tool area: panels sit on one row; when they don't all
                    // fit they degrade from the right — a panel's large buttons first
                    // shrink to compact icon columns, then it collapses to a ▾ flyout
                    // button. See `CollapsePanels`.
                    let panels: Vec<Panel<'_>> = groups
                        .iter()
                        .enumerate()
                        .map(|(i, g)| self.panel(i == 0, g, ts, &style_ctx, theme, &|_| Vec::new()))
                        .collect();
                    CollapsePanels::new(panels, self.collapsed_open.clone(), TOOL_BAR_H)
                        .report_height(self.tool_bar_h.clone())
                        .report_tight(self.collapse_tight.clone())
                        .mode(self.collapse_mode)
                        .into()
                }
            } else {
                text("").into()
            };

        // The tool field sits on the same panel the tab strip does — the sheet
        // draws the whole ribbon as one grey band and separates the panels
        // with hairline rules, not with a second depth of field. Vendor themes
        // keep their recessed `Sink`, which is what their own ladder expects.
        let tool_bar: Element<'_, Message> = container(tool_area)
            .width(Length::Fill)
            .style(|theme: &Theme| {
                let tier =
                    if crate::ui::style::draftflow_theme::draftflow_colors(theme).is_some() {
                        kit::Tier::Flush
                    } else {
                        kit::Tier::Sink
                    };
                tier.style(theme, "")
            })
            .into();

        let tool_bar = if is_start {
            // The Start tab has no drawing context. One theme-aware shield
            // fades and blocks only the tool area; the tabs and application
            // controls above it remain available and clearly identifiable.
            let shield = mouse_area(
                container(iced::widget::Space::new())
                    .width(Fill)
                    .height(Fill)
                    .style(|theme: &Theme| {
                        // A scrim, not a plane: the kit has no translucent-shield
                        // primitive, so the paint is the kit's `Flush` plane with
                        // the page colour laid over it at partial alpha. It fades
                        // toward the page background, not toward a palette step:
                        // `strongest` deviates *away* from `base`, so on a dark
                        // scheme it would paint a lighter slab over the recessed
                        // tool band and invert the tier the band exists to create.
                        let mut plane = kit::Tier::Flush.style(theme, "");
                        plane.background = Some(Background::Color(
                            theme.palette().background.base.color.scale_alpha(RIBBON_SCRIM_A),
                        ));
                        plane
                    }),
            )
            .on_press(Message::CloseRibbonDropdown)
            .interaction(iced::mouse::Interaction::Idle);
            let shield = ribbon_tip(
                shield,
                t!("Open or create a drawing to use ribbon commands.").into_owned(),
                iced::widget::tooltip::Position::Bottom,
                tokens::GAP_CONTROL,
                theme,
            );

            iced::widget::stack![tool_bar, iced::widget::opaque(shield)].into()
        } else {
            tool_bar
        };

        // The two rows are handed back separately: the host stacks the
        // document tab strip between them (see `app::view`). The sheet turns
        // the application bar into its own floating card and leaves the tool
        // row bare on the page; the vendor themes keep the banded ribbon and
        // join the rows with their hairlines.
        (tab_bar.into(), tool_bar)
    }

    // ── Dropdown overlay ──────────────────────────────────────────────────

    /// Place `panel` (`w` wide) under dropdown `id`, closing on a click
    /// outside it. `panel` carries its own surface, so `w` is the full sheet
    /// width and no popup inset is added.
    pub fn place_dropdown<'a>(
        &self,
        id: &str,
        panel: Element<'a, Message>,
        w: f32,
        win_w: f32,
    ) -> Element<'a, Message> {
        let (left, top) = self.dd_anchor_outer(id, w, win_w);
        dropdown_backdrop(position_ribbon_dropdown(panel, left, top))
    }

    pub fn dropdown_overlay(
        &self,
        undo_labels: &[String],
        redo_labels: &[String],
        win: (f32, f32),
        is_start: bool,
        recent_colors: &[AcadColor],
        theme: &Theme,
    ) -> Option<Element<'_, Message>> {
        if is_start {
            return None;
        }
        let open_id = self.open_dropdown.as_deref()?;

        if draw_panel::owns_dropdown(open_id) {
            return Some(draw_panel::overlay(self, open_id, win, theme));
        }

        if open_id == UNDO_HISTORY_ID || open_id == REDO_HISTORY_ID {
            let is_undo = open_id == UNDO_HISTORY_ID;
            let labels = if is_undo { undo_labels } else { redo_labels };
            if labels.is_empty() {
                return None;
            }

            let rows: Vec<Element<Message>> = labels
                .iter()
                .enumerate()
                .map(|(idx, label)| {
                    let step = idx + 1;
                    plain_button(text(label.clone()).size(tokens::TEXT_LABEL))
                        .on_press(if is_undo {
                            Message::UndoMany(step)
                        } else {
                            Message::RedoMany(step)
                        })
                        .style(kit::ghost_style)
                        .width(Fill)
                        .padding(kit::Size::Control.padding())
                        .into()
                })
                .collect();

            let panel = container(column(rows))
                .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
                .width(Length::Fixed(170.0));
            // The host keeps its inset and width; the menu's paper is painted by
        // `popup_plate` — the `Float` rung wearing the field's edge.
            let panel = kit::popup_plate(panel.into(), "");

            let (left, top) = self.dd_anchor(open_id, 170.0, win.0);
            let positioned = position_ribbon_dropdown(panel.into(), left, top);

            return Some(dropdown_backdrop(positioned));
        }

        if open_id == COLLAPSE_MODE_ID {
            const W: f32 = 150.0;
            let current = self.collapse_mode;
            let rows: Vec<Element<Message>> = CollapseMode::ALL
                .iter()
                .map(|&m| {
                    plain_button(
                        row![
                            crate::ui::icons::themed_check_cell(m == current),
                            text(t!(m.label())).size(tokens::TEXT_LABEL),
                        ]
                        .spacing(tokens::GAP_TIGHT)
                        .align_y(iced::Center),
                    )
                    .on_press(Message::SetRibbonCollapseMode(m))
                    .style(kit::ghost_style)
                    .width(Fill)
                    .padding(kit::Size::Control.padding())
                    .into()
                })
                .collect();

            let panel = container(column(rows))
                .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
                .width(Length::Fixed(W));
            // The host keeps its inset and width; the menu's paper is painted by
        // `popup_plate` — the `Float` rung wearing the field's edge.
            let panel = kit::popup_plate(panel.into(), "");

            let (left, top) = self.dd_anchor(open_id, W, win.0);
            let positioned = position_ribbon_dropdown(panel.into(), left, top);
            return Some(dropdown_backdrop(positioned));
        }

        if open_id == LAYER_COMBO_ID {
            return self.layer_combo_overlay(win);
        }

        if open_id == PROP_COLOR_ID {
            return self.prop_color_overlay(win, recent_colors);
        }
        if open_id == PROP_LINETYPE_ID {
            return self.prop_linetype_overlay(win);
        }
        if open_id == PROP_LW_ID {
            return self.prop_lw_overlay(win);
        }

        // Style combo dropdowns (annotate tab) float as overlays so the list
        // isn't clipped by the fixed ribbon-row height, the way the Draw-tab
        // dropdowns already are. (#153)
        if let Some(ov) = self.style_combo_overlay(open_id, win) {
            return Some(ov);
        }

        let module = self.modules.get(self.active)?;
        let groups = module.ribbon_groups();
        let mut items_list: Option<Vec<(&'static str, &'static str, IconKind)>> = None;
        let mut dd_default = "";
        let mut dd_id: &'static str = "";

        'outer: for group in groups {
            for item in &group.tools {
                let (id, items, default) = match item {
                    RibbonItem::Dropdown {
                        id, items, default, ..
                    } => (*id, items, *default),
                    RibbonItem::LabeledDropdown {
                        id, items, default, ..
                    } => (*id, items, *default),
                    RibbonItem::LargeDropdown {
                        id, items, default, ..
                    } => (*id, items, *default),
                    _ => continue,
                };
                if id == open_id {
                    items_list = Some(items.clone());
                    dd_default = default;
                    dd_id = id;
                    break 'outer;
                }
            }
        }
        let items = items_list?;
        let last_cmd = self.last_cmd.get(dd_id).copied().unwrap_or(dd_default);

        let rows: Vec<Element<Message>> = items
            .iter()
            .map(|(cmd, label, item_icon)| {
                let is_current = *cmd == last_cmd;
                let disabled = widgets::is_disabled_item(cmd);
                let checkmark: Element<'_, Message> =
                    crate::ui::icons::themed_check_cell(is_current);
                let icon_el: Element<Message> = container(make_icon(*item_icon, 20.0))
                    .width(Length::Fixed(20.0))
                    .into();
                let label_el = text(t!(*label))
                    .size(tokens::TEXT_LABEL)
                    .wrapping(iced::advanced::text::Wrapping::None);

                // A dropdown row is a list row: the item that is running reads
                // as engaged, the rest are quiet, and a disabled item takes the
                // kit's disabled look because `Press::Off` is the same fact.
                kit::list_row(
                    row![checkmark, icon_el, label_el]
                        .spacing(tokens::GAP_TIGHT)
                        .align_y(iced::Center),
                    is_current,
                    if disabled {
                        kit::Press::Off
                    } else {
                        kit::Press::On(Message::DropdownSelectItem {
                            dropdown_id: dd_id,
                            cmd: *cmd,
                        })
                    },
                    "",
                )
                .into()
            })
            .collect();

        let labels: Vec<String> = items
            .iter()
            .map(|(_, label, _)| t!(*label).to_string())
            .collect();
        let panel_w = crate::ui::style::common::dropdown_popup_width(
            labels.iter().map(|s| s.as_str()),
            11.0,
            68.0,
            190.0,
        );

        let panel = container(column(rows))
            .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
            .width(Length::Fixed(panel_w));
        // The host keeps its inset and width; the menu's paper is painted by
        // `popup_plate` — the `Float` rung wearing the field's edge.
        let panel = kit::popup_plate(panel.into(), "");

        let (left, top) = self.dd_anchor(open_id, panel_w, win.0);
        Some(dropdown_backdrop(position_ribbon_dropdown(
            panel.into(),
            left,
            top,
        )))
    }

    fn layer_combo_overlay(&self, win: (f32, f32)) -> Option<Element<'_, Message>> {
        // A toggle icon (visible / freeze / lock) is its own button so a click
        // on it flips that state instead of bubbling up to the row's
        // make-active handler (#133).
        let icon_btn = |bytes: &'static [u8], msg: Message| -> Element<'_, Message> {
            plain_button(crate::ui::icons::semantic(bytes, 14.0))
                .on_press(msg)
                .style(kit::ghost_style)
                .padding(kit::PAD_GLYPH)
                .into()
        };
        let filter = self.layer_filter.to_lowercase();
        let rows: Vec<Element<Message>> = self
            .layer_infos
            .iter()
            .filter(|info| filter.is_empty() || info.name.to_lowercase().contains(&filter))
            .map(|info| {
                let is_active = info.name == self.active_layer;
                let lc = info.color;
                let lv = info.visible;
                let lf = info.frozen;
                let ll = info.locked;
                let name = info.name.clone();

                let swatch = container(text(""))
                    .style(move |theme: &Theme| kit::swatch_style(theme, lc, ""))
                    .width(12)
                    .height(12);

                let vis = icon_btn(
                    crate::ui::icons::layer_visible(lv),
                    Message::LayerToggleVisible(info.name.clone()),
                );
                let freeze = icon_btn(
                    crate::ui::icons::layer_freeze(lf),
                    Message::LayerToggleFreeze(info.name.clone()),
                );
                let lock = icon_btn(
                    crate::ui::icons::layer_lock(ll),
                    Message::LayerToggleLock(info.name.clone()),
                );
                // Freeze in the current viewport only (#1466).
                let vp_freeze = info.vp_frozen.map(|(column, frozen)| {
                    icon_btn(
                        crate::ui::icons::layer_vp_freeze(frozen),
                        Message::LayerToggleVpFreeze(info.name.clone(), column),
                    )
                });
                let checkmark: Element<'_, Message> =
                    crate::ui::icons::themed_check_cell(is_active);
                let label = text(&info.name).size(tokens::TEXT_LABEL);

                // The swatch + label area selects the layer as active; the
                // icon buttons above handle their own toggles. The active
                // layer's row is the engaged one.
                let select = kit::list_row(
                    row![swatch, label].spacing(tokens::GAP_TIGHT).align_y(iced::Center),
                    is_active,
                    kit::Press::On(Message::RibbonLayerChanged(name)),
                    "",
                );

                container(
                    row![checkmark, vis, freeze]
                        .push(vp_freeze)
                        .push(lock)
                        .push(select)
                        .spacing(tokens::GAP_CONTROL)
                        .align_y(iced::Center),
                )
                .padding(kit::PAD_BADGE)
                .into()
            })
            .collect();

        // Cap the panel height and make the list scrollable so a long layer
        // list stays reachable instead of running off the bottom of the
        // screen (#227). Short lists shrink to fit; longer ones scroll.
        let row_count = rows.len().max(1);
        let list_h = (row_count as f32 * 26.0).min(420.0);
        // Search box (#343): filters the list live as the user types.
        let search = iced::widget::text_input(t!("Search layers…").as_ref(), &self.layer_filter)
            .on_input(Message::RibbonLayerFilterChanged)
            .size(tokens::TEXT_LABEL)
            .padding(kit::PAD_FIELD);
        let state_manager = plain_button(
            row![
                crate::ui::icons::themed_arrow_right(9.0),
                text(t!("Layer State Manager…")).size(tokens::TEXT_LABEL),
            ]
            .spacing(tokens::GAP_TIGHT)
            .align_y(iced::Center),
        )
        .on_press(Message::LayerStateManagerOpen)
        .style(kit::ghost_style)
        .width(Fill)
        .padding(kit::Size::Control.padding());
        let panel = container(
            column![
                container(search).padding(kit::PAD_CELL),
                kit::themed_scrollable(column(rows)).height(Length::Fixed(list_h)),
                // The footer is separated by the kit's hairline, not by a
                // border drawn around the button's container.
                kit::rule_h(""),
                container(state_manager).width(Fill).padding(kit::PAD_CELL),
            ]
            .spacing(tokens::GAP_HAIRLINE),
        )
        .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
        .width(Length::Fixed(300.0));
        // The host keeps its inset and width; the menu's paper is painted by
        // `popup_plate` — the `Float` rung wearing the field's edge.
        let panel = kit::popup_plate(panel.into(), "");

        let (left, top) = self.dd_anchor(LAYER_COMBO_ID, 300.0, win.0);
        let positioned = position_ribbon_dropdown(panel.into(), left, top);

        Some(dropdown_backdrop(positioned))
    }

    /// Floating popup for an annotate-tab style combo (text / dimension /
    /// multileader / table style). Returns `None` when `open_id` is not a
    /// style combo. Built as an overlay — like the layer combo and the
    /// Draw-tab dropdowns — so the list grows to fit its entries instead of
    /// being clipped to the ribbon-row height. (#153)
    fn style_combo_overlay(
        &self,
        open_id: &str,
        win: (f32, f32),
    ) -> Option<Element<'_, Message>> {
        let groups = self.modules.get(self.active)?.ribbon_groups();

        // Locate the open style combo; capture its style key + manager command.
        let mut found: Option<(crate::modules::StyleKey, Option<&'static str>)> = None;
        'outer: for group in groups {
            for item in &group.tools {
                if let RibbonItem::StyleComboGroup {
                    style_key,
                    combo_id,
                    manager_cmd,
                    ..
                } = item
                {
                    if *combo_id == open_id {
                        found = Some((*style_key, *manager_cmd));
                        break 'outer;
                    }
                }
            }
        }
        let (style_key, manager_cmd) = found?;

        let ctx = StyleContext {
            text_style_names: &self.text_style_names,
            active_text_style: &self.active_text_style,
            dim_style_names: &self.dim_style_names,
            active_dim_style: &self.active_dim_style,
            mleader_style_names: &self.mleader_style_names,
            active_mleader_style: &self.active_mleader_style,
            table_style_names: &self.table_style_names,
            active_table_style: &self.active_table_style,
        };
        let active = ctx.active_for(style_key).to_string();

        let mut rows: Vec<Element<Message>> = ctx
            .names_for(style_key)
            .iter()
            .map(|name| {
                let is_sel = name.as_str() == active.as_str();
                let n = name.clone();
                let checkmark: Element<Message> = crate::ui::icons::themed_check_cell(is_sel);
                kit::list_row(
                    row![checkmark, text(name.clone()).size(tokens::TEXT_LABEL)]
                        .spacing(tokens::GAP_TIGHT)
                        .align_y(iced::Center),
                    is_sel,
                    kit::Press::On(Message::RibbonStyleChanged {
                        key: style_key,
                        name: n,
                    }),
                    "",
                )
                .into()
            })
            .collect();

        if let Some(mgr) = manager_cmd {
            rows.push(
                plain_button(text(t!("Manage…")).size(tokens::TEXT_LABEL))
                    .on_press(Message::Command(mgr.to_string()))
                    .style(kit::ghost_style)
                    .width(Fill)
                    .padding(kit::Size::Control.padding())
                    .into(),
            );
        }

        let panel = container(column(rows))
            .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
            .width(Length::Fixed(LARGE_W * 2.3));
        // The host keeps its inset and width; the menu's paper is painted by
        // `popup_plate` — the `Float` rung wearing the field's edge.
        let panel = kit::popup_plate(panel.into(), "");

        let (left, top) = self.dd_anchor(open_id, LARGE_W * 2.3, win.0);
        Some(dropdown_backdrop(position_ribbon_dropdown(
            panel.into(),
            left,
            top,
        )))
    }

    fn prop_color_overlay(
        &self,
        win: (f32, f32),
        recent_colors: &[AcadColor],
    ) -> Option<Element<'_, Message>> {
        let panel = color_dropdown::color_dropdown_panel(self.active_color, recent_colors);

        let (left, top) =
            self.dd_anchor(PROP_COLOR_ID, color_dropdown::PANEL_W, win.0);
        Some(dropdown_backdrop(position_ribbon_dropdown(
            panel,
            left,
            top,
        )))
    }

    fn prop_linetype_overlay(
        &self,
        win: (f32, f32),
    ) -> Option<Element<'_, Message>> {
        let active_lt = &self.active_linetype;

        let mut items: Vec<LinetypeItem> = vec![
            LinetypeItem {
                name: "ByLayer".to_string(),
                art: String::new(),
            },
            LinetypeItem {
                name: "ByBlock".to_string(),
                art: String::new(),
            },
        ];
        for lt in &self.available_linetypes {
            if lt.name != "ByLayer" && lt.name != "ByBlock" {
                items.push(lt.clone());
            }
        }

        let rows: Vec<Element<Message>> = items
            .into_iter()
            .map(|lt| {
                let is_cur = lt.name == *active_lt;
                let check: Element<'_, Message> = crate::ui::icons::themed_check_cell(is_cur);
                let name_col = text(linetype_display_name(&lt.name))
                    .size(tokens::TEXT_LABEL)
                    .width(Length::Fixed(90.0));
                let art_col = text(lt.art.clone())
                    .size(tokens::TEXT_MICRO)
                    .style(kit::muted_style);
                let name = lt.name.clone();
                kit::list_row(
                    row![check, name_col, art_col]
                        .spacing(tokens::GAP_TIGHT)
                        .align_y(iced::Center),
                    is_cur,
                    kit::Press::On(Message::RibbonLinetypeChanged(name)),
                    "",
                )
                .into()
            })
            .collect();

        let list = container(kit::themed_scrollable(column(rows)).height(Length::Fixed(200.0)))
            .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
            .width(Length::Fixed(220.0));
        // The host keeps its inset and width; the menu's paper is painted by
        // `popup_plate` — the `Float` rung wearing the field's edge.
        let list = kit::popup_plate(list.into(), "");

        let (left, top) = self.dd_anchor(PROP_LINETYPE_ID, 220.0, win.0);
        Some(dropdown_backdrop(position_ribbon_dropdown(
            list.into(),
            left,
            top,
        )))
    }

    fn prop_lw_overlay(&self, win: (f32, f32)) -> Option<Element<'_, Message>> {
        let active_lw = self.active_lineweight;
        let rows: Vec<Element<Message>> = lw_options()
            .into_iter()
            .map(|item| {
                let is_cur = item.0 == active_lw;
                let label = item.to_string();
                let check: Element<'_, Message> = crate::ui::icons::themed_check_cell(is_cur);
                kit::list_row(
                    row![check, text(label).size(tokens::TEXT_LABEL)]
                        .spacing(tokens::GAP_TIGHT)
                        .align_y(iced::Center),
                    is_cur,
                    kit::Press::On(Message::RibbonLineweightChanged(item.0)),
                    "",
                )
                .into()
            })
            .collect();

        self.prop_overlay_positioned(rows, PROP_LW_ID, 140.0, win)
    }

    fn prop_overlay_positioned<'a>(
        &'a self,
        rows: Vec<Element<'a, Message>>,
        dd_id: &str,
        width: f32,
        win: (f32, f32),
    ) -> Option<Element<'a, Message>> {
        let panel = container(column(rows))
            .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
            .width(Length::Fixed(width));
        // The host keeps its inset and width; the menu's paper is painted by
        // `popup_plate` — the `Float` rung wearing the field's edge.
        let panel = kit::popup_plate(panel.into(), "");

        let (left, top) = self.dd_anchor(dd_id, width, win.0);
        Some(dropdown_backdrop(position_ribbon_dropdown(
            panel.into(),
            left,
            top,
        )))
    }
}

/// The separator between two tool groups on the sheet's one-row strip: one
/// hairline standing in a section-wide gap. It names the group boundary without
/// a title, which is what keeps the row short.
fn group_divider<'a>() -> Element<'a, Message> {
    container(ribbon_group_rule())
        .width(Length::Fixed(tokens::GAP_SECTION))
        .height(Length::Fixed(28.0))
        .align_x(iced::Center)
        .align_y(iced::Center)
        .into()
}

/// Does a ribbon item stand for the command `want`? A dropdown answers for its
/// own id and for the command its face currently runs.
fn strip_matches(item: &RibbonItem, want: &str) -> bool {
    match item {
        RibbonItem::Tool(t) | RibbonItem::LabeledTool(t) | RibbonItem::LargeTool(t) => {
            t.id == want
        }
        RibbonItem::Dropdown { id, default, .. }
        | RibbonItem::LargeDropdown { id, default, .. }
        | RibbonItem::LabeledDropdown { id, default, .. } => *id == want || *default == want,
        _ => false,
    }
}

/// The buttons the sheet's single-row tool strip shows for `module`.
///
/// On the drawing tab the row is this CAD's everyday command set, in working
/// order: draw, then modify, then the layer and dimension entries a drafter
/// reaches for while the command is running. Other tabs lay their own tools
/// out in module order. Either way the row is the house's tools — the
/// reference's sample row is a shape to match, not a command list to copy.
fn sheet_strip_items<'a>(
    module_id: &str,
    groups: &[RibbonGroup],
    ctx: &widgets::RenderCtx<'_>,
) -> Vec<Element<'a, Message>> {
    // The drawing tab's row is grouped the way the work is: a short draw
    // cluster, a short modify cluster, the dimension and layer entries one
    // reaches for mid-command, and the output command last. Groups are
    // separated by a section gap alone — no titles, no rules, no second row.
    const HOME_GROUPS: &[(&str, &[&str])] = &[
        ("Draw", &["LINE", "PLINE", "RECT", "CIRCLE"]),
        ("Modify", &["MOVE", "COPY", "TRIM", "OFFSET"]),
        ("Annotation", &["DIMLINEAR", "TEXT"]),
        ("Layers", &["LAYERS"]),
    ];
    const OTHER_GROUP_LIMIT: usize = 4;
    const OTHER_LIMIT: usize = 16;

    // The slide-out that carries the rest of a ribbon group, keyed by the
    // group's own title and the ids it currently holds. It is offered only
    // when the row still hides a command: a group whose tools are all on the
    // row already has nothing behind a "More…" button.
    let more_button = |title: &str,
                       groups: &[RibbonGroup],
                       shown: usize,
                       ctx: &widgets::RenderCtx<'_>|
     -> Option<Element<'a, Message>> {
        let group = groups.iter().find(|g| g.title == title)?;
        let total = group
            .tools
            .iter()
            .filter(|it| widgets::strip_supported(it))
            .count();
        if shown >= total {
            return None;
        }
        let ids: Vec<&'static str> = group.tools.iter().filter_map(item_id).collect();
        draw_panel::group_panel_id(title, &ids)
            .map(|panel| widgets::render_strip_more(panel, ctx))
    };

    let mut out: Vec<Element<'a, Message>> = Vec::new();
    if module_id == "draw" {
        for (gi, (title, group)) in HOME_GROUPS.iter().enumerate() {
            let mut shown = 0;
            for want in *group {
                if let Some(item) = groups
                    .iter()
                    .flat_map(|g| g.tools.iter())
                    .find(|it| strip_matches(it, want))
                {
                    if let Some(el) = widgets::render_strip(item, ctx) {
                        out.push(el);
                        shown += 1;
                    }
                }
            }
            if shown > 0 {
                if let Some(more) = more_button(title, groups, shown, ctx) {
                    out.push(more);
                }
                if gi + 1 < HOME_GROUPS.len() {
                    out.push(group_divider());
                }
            }
        }
        // Plot is a global command, not a member of the draw module; the sheet
        // keeps it at the end of the row because it is the last thing one does.
        out.push(Space::new().width(tokens::GAP_SECTION).into());
        let plot = RibbonItem::Tool(ToolDef {
            id: "PLOT",
            label: "Plot",
            icon: IconKind::Svg(crate::ui::icons::PRINT),
            event: ModuleEvent::Command("PLOT".to_string()),
        });
        if let Some(el) = widgets::render_strip(&plot, ctx) {
            out.push(el);
        }
    } else {
        for (gi, group) in groups.iter().enumerate() {
            if out.len() >= OTHER_LIMIT {
                break;
            }
            let mut shown = 0;
            for item in &group.tools {
                if shown >= OTHER_GROUP_LIMIT {
                    break;
                }
                if let Some(el) = widgets::render_strip(item, ctx) {
                    out.push(el);
                    shown += 1;
                }
            }
            if shown > 0 {
                if let Some(more) = more_button(group.title, groups, shown, ctx) {
                    out.push(more);
                }
                if gi + 1 < groups.len() {
                    out.push(group_divider());
                }
            }
        }
    }
    out
}

/// Render a single ribbon panel (tools + group label), fixed `TOOL_BAR_H` tall.
/// When `compact`, large tools/dropdowns are drawn as small icon columns.
#[allow(clippy::too_many_arguments)]
fn render_group<'a>(
    first: bool,
    compact: bool,
    group: &RibbonGroup,
    lead: Vec<Element<'a, Message>>,
    active_tool: &Option<String>,
    open_dd: &Option<String>,
    last_cmd: &HashMap<&'static str, &'static str>,
    state: widgets::ToggleState,
    layer_infos: &'a [LayerInfo],
    active_layer: &'a str,
    active_color: AcadColor,
    active_linetype: &'a str,
    active_lineweight: LineWeight,
    style_ctx: &StyleContext<'_>,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let mut items_row: Vec<Element<Message>> = lead;
    let mut small_buf: Vec<Element<Message>> = Vec::new();

    let ctx = widgets::RenderCtx {
        active_tool,
        theme,
        open_dd,
        last_cmd,
        state,
        layer_infos,
        active_layer,
        active_color,
        active_linetype,
        active_lineweight,
        style_ctx,
        compact,
    };

    for item in &group.tools {
        let is_large = match item {
            RibbonItem::LargeTool(_) | RibbonItem::LargeDropdown { .. } => !compact,
            RibbonItem::ToolGrid { .. }
            | RibbonItem::LayerComboGroup { .. }
            | RibbonItem::PropertiesGroup { .. }
            | RibbonItem::StyleComboGroup { .. } => true,
            _ => false,
        };

        if is_large {
            flush_small_col(&mut small_buf, &mut items_row);
            items_row.push(render_large(item, &ctx));
        } else {
            small_buf.push(render_small(item, active_tool, open_dd, last_cmd, state, theme));
            if small_buf.len() == 3 {
                flush_small_col(&mut small_buf, &mut items_row);
            }
        }
    }
    flush_small_col(&mut small_buf, &mut items_row);

    let tools_el = items_row
        .into_iter()
        .fold(row![].spacing(tokens::GAP_HAIRLINE).height(Fill).align_y(iced::Top), |r, e| {
            r.push(e)
        });

    // Panels are separated by one hairline the way the sheet separates its
    // columns: no card, no second plane, no corner. The rule belongs to the
    // gap between two panels, so the first panel has none.
    let body: Element<'a, Message> = container(
        column![
            tools_el,
            draw_panel::group_title(
                group.title,
                &group.tools.iter().filter_map(item_id).collect::<Vec<_>>(),
                open_dd,
                active_tool.as_deref(),
                theme,
            ),
        ]
        .align_x(iced::Center)
        .spacing(tokens::GAP_NONE)
        .padding([tokens::GAP_TIGHT, tokens::GAP_CONTROL])
        .height(Fill),
    )
    .height(Length::Fixed(TOOL_BAR_H))
    .into();

    if first {
        body
    } else {
        row![ribbon_group_rule(), body]
            .spacing(tokens::GAP_CONTROL)
            .align_y(iced::Top)
            .height(Length::Fixed(TOOL_BAR_H))
            .into()
    }
}

/// The top-level command id of a ribbon item, if it has one.
fn item_id(it: &RibbonItem) -> Option<&'static str> {
    match it {
        RibbonItem::Tool(t) | RibbonItem::LabeledTool(t) | RibbonItem::LargeTool(t) => Some(t.id),
        RibbonItem::Dropdown { id, .. }
        | RibbonItem::LabeledDropdown { id, .. }
        | RibbonItem::LargeDropdown { id, .. } => Some(*id),
        RibbonItem::PropertiesGroup { match_prop } => Some(match_prop.id),
        _ => None,
    }
}

/// The tool a collapsed panel shows on its button: the last-used one, else the
/// panel's first tool-like item.
fn representative<'g>(group: &'g RibbonGroup, last_used: Option<&str>) -> Option<&'g RibbonItem> {
    if let Some(want) = last_used {
        if let Some(found) = group
            .tools
            .iter()
            .find(|&it| item_id(it).map_or(false, |id| id == want))
        {
            return Some(found);
        }
    }
    group.tools.iter().find(|&it| item_id(it).is_some())
}

/// The icon of a panel's first tool-like item. Shown on the tightest collapse
/// button; digs into the composite groups so Layers/Properties/Styles panels
/// still get a representative icon.
fn first_tool_icon(group: &RibbonGroup) -> Option<IconKind> {
    group.tools.iter().find_map(|it| match it {
        RibbonItem::Tool(t) | RibbonItem::LabeledTool(t) | RibbonItem::LargeTool(t) => Some(t.icon),
        RibbonItem::Dropdown { icon, .. }
        | RibbonItem::LabeledDropdown { icon, .. }
        | RibbonItem::LargeDropdown { icon, .. } => Some(*icon),
        RibbonItem::PropertiesGroup { match_prop } => Some(match_prop.icon),
        RibbonItem::LayerComboGroup { row2, .. } => row2.first().map(|t| t.icon),
        RibbonItem::StyleComboGroup { rows, .. } => {
            rows.first().and_then(|r| r.first()).map(|t| t.icon)
        }
        RibbonItem::ToolGrid { columns } => columns
            .first()
            .and_then(|column| column.first())
            .map(|tool| tool.icon),
    })
}

/// A collapsed panel. In the COLLAPSED step: a live representative-tool button
/// (updates to the last-used tool) plus a title + ▾ opener for the full flyout.
/// In the tighter `compact` (TIGHT) step: just the first tool's icon + ▾, a pure
/// flyout opener with no direct tool run. Height is natural content height
/// (shorter than a full 3-row panel), so `CollapsePanels` can shrink the ribbon.
#[allow(clippy::too_many_arguments)]
fn collapse_button<'a>(
    group: &RibbonGroup,
    last_used: Option<&str>,
    active_tool: &Option<String>,
    open_dd: &Option<String>,
    last_cmd: &HashMap<&'static str, &'static str>,
    state: widgets::ToggleState,
    layer_infos: &'a [LayerInfo],
    active_layer: &'a str,
    active_color: AcadColor,
    active_linetype: &'a str,
    active_lineweight: LineWeight,
    style_ctx: &StyleContext<'_>,
    theme: &'a Theme,
    // When set, this is the TIGHT step (reached once even the all-collapsed row
    // overflows): the whole panel becomes a single non-running button — the first
    // tool's icon + a ▾ that opens the tools flyout. Nothing runs directly here.
    compact: bool,
) -> Element<'a, Message> {
    let title = group.title;
    let localized_title = t!(title).into_owned();
    // A collapsed panel lights its opener while one of its hidden tools runs,
    // the way a dropdown lights while one of its items does.
    let active = active_tool.as_deref();
    let holds = active.is_some_and(|id| locate::group_holds(group, id));
    let opener_hover = crate::ui::motion::hover_key(title);
    let opener_glow = crate::ui::motion::glow_key(title);
    let opener_style = move |lit: bool| {
        move |theme: &Theme, status| {
            if lit {
                tool_btn_style(
                    theme,
                    true,
                    status,
                    crate::ui::motion::value(opener_glow),
                    crate::ui::motion::value(opener_hover),
                )
            } else {
                kit::subtle_style(theme, status)
            }
        }
    };

    // Tightest form: one button = the panel's FIRST tool icon + its title + ▾.
    // Clicking opens the flyout listing every tool; no tool runs directly at this
    // density (you pick one from the dropdown), so the tightest row is pure openers.
    if compact {
        let icon: Element<'_, Message> = match first_tool_icon(group) {
            Some(ik) => make_icon(ik, SMALL_ICON),
            None => text("").into(),
        };
        let content = hover_tracked(
            opener_hover,
            plain_button(
                column![
                    icon,
                    row![
                        text(localized_title.clone())
                            .size(tokens::TEXT_MICRO)
                            .width(Fill)
                            .align_x(iced::Center)
                            .wrapping(iced::advanced::text::Wrapping::Word)
                            .style(kit::muted_style),
                        crate::ui::icons::themed_secondary_arrow_down(8.0),
                    ]
                    .spacing(tokens::GAP_TIGHT)
                    .width(Fill)
                    .align_y(iced::Center),
                ]
                .align_x(iced::Center)
                .spacing(tokens::GAP_HAIRLINE)
                .width(Fill),
            )
            .on_press(Message::ToggleRibbonPanel(title.to_string()))
            .style(opener_style(holds))
            .width(Fill)
            .padding(kit::PAD_CELL),
        );
        return automatic_large_button(localized_title, content.into());
    }

    // Collapsed (not yet tight): a large representative-tool face — a live button
    // that runs the last-used tool — above a title + ▾ opener for the full flyout.
    // For a Properties panel the representative is its Match button.
    let rep = representative(group, last_used);
    // The face lights itself when it is the running tool.
    let face_lit = rep
        .zip(active)
        .is_some_and(|(item, id)| locate::item_holds(item, id));
    let face: Element<'_, Message> = match rep {
        Some(RibbonItem::PropertiesGroup { match_prop }) => render_large(
            &RibbonItem::LargeTool(match_prop.clone()),
            &widgets::RenderCtx {
                active_tool,
                theme,
                open_dd,
                last_cmd,
                state,
                layer_infos,
                active_layer,
                active_color,
                active_linetype,
                active_lineweight,
                style_ctx,
                compact: false,
            },
        ),
        Some(item) => render_large(
            item,
            &widgets::RenderCtx {
                active_tool,
                theme,
                open_dd,
                last_cmd,
                state,
                layer_infos,
                active_layer,
                active_color,
                active_linetype,
                active_lineweight,
                style_ctx,
                compact: false,
            },
        ),
        None => text("").into(),
    };

    let opener = hover_tracked(
        opener_hover,
        plain_button(
            row![
                text(localized_title.clone())
                    .size(tokens::TEXT_MICRO)
                    .width(Fill)
                    .align_x(iced::Center)
                    .wrapping(iced::advanced::text::Wrapping::Word)
                    .style(kit::muted_style),
                crate::ui::icons::themed_secondary_arrow_down(8.0),
            ]
            .spacing(tokens::GAP_TIGHT)
            .width(Fill)
            .align_y(iced::Center),
        )
        .on_press(Message::ToggleRibbonPanel(title.to_string()))
        .style(opener_style(holds && !face_lit))
        .width(Fill)
        .padding(kit::PAD_CELL),
    );
    let opener = automatic_large_button(localized_title, opener.into());

    // The large face fills a fixed slot so a collapsed panel is shorter than a full
    // 3-row panel, letting `CollapsePanels` shrink the ribbon row.
    let face_box = container(face)
        .height(Length::Fixed(COLLAPSED_FACE_H))
        .align_y(iced::Center);

    column![face_box, opener]
        .align_x(iced::Center)
        .spacing(tokens::GAP_HAIRLINE)
        .padding(kit::PAD_CELL)
        .into()
}

impl Default for Ribbon {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::widgets::is_active_tool;
    use super::{dropdown_left, Ribbon};

    /// The dropdown hangs from its button's centre, and never leaves the
    /// window. The four cases are the ones a strip can actually produce: a
    /// middle button, a button near either edge, and a list wider than the
    /// window (the layer combo on a narrow layout).
    #[test]
    fn a_dropdown_is_centred_under_its_button_and_kept_inside_the_window() {
        assert_eq!(dropdown_left(500.0, 200.0, 1200.0), 400.0);
        assert_eq!(dropdown_left(50.0, 300.0, 1200.0), 4.0);
        assert_eq!(dropdown_left(1150.0, 300.0, 1200.0), 898.0);
        assert_eq!(dropdown_left(600.0, 1500.0, 1200.0), 4.0);

        // Whatever the button position, the panel's right edge stays two
        // pixels inside the window.
        for centre in [0.0, 120.0, 600.0, 1080.0, 1200.0] {
            let left = dropdown_left(centre, 240.0, 1200.0);
            assert!(left >= 4.0 && left + 240.0 <= 1198.0, "centre {centre}: left {left}");
        }
    }

    #[test]
    fn the_sheet_inset_is_centred_not_the_body_behind_it() {
        // `popup_plate` wraps the body in `GAP_CONTROL` on every side. The
        // anchor must hand back the *sheet's* left edge, or the visible paper
        // sits half an inset right of the button while its clamped right edge
        // can leave the window by the same amount.
        use crate::ui::style::tokens;
        let body_w = 200.0;
        let outer_w = body_w + 2.0 * tokens::GAP_CONTROL;
        let left = dropdown_left(500.0, outer_w, 1200.0);
        assert_eq!(left + tokens::GAP_CONTROL + body_w / 2.0, 500.0);
        assert_eq!(left, 500.0 - outer_w / 2.0);
    }

    #[test]
    fn block_palette_highlight_follows_the_app_state_passed_to_the_view() {
        // The Block Palette button highlight is derived from the app's
        // authoritative `show_block_palette`, threaded into the view at render
        // time: the ribbon keeps no second copy that a BLOCKPALETTE / close
        // handler must remember to keep in sync.
        let ribbon = Ribbon::default();
        let on = ribbon.toggle_state(true);
        assert!(is_active_tool("BLOCKPALETTE", &None, &on));
        let off = ribbon.toggle_state(false);
        assert!(!is_active_tool("BLOCKPALETTE", &None, &off));
    }

    #[test]
    fn escape_returns_from_an_extension_submenu_then_closes_the_panel() {
        let mut ribbon = Ribbon::default();
        ribbon.open_dropdown = Some("DRAWORDER_FRONT".to_string());

        assert!(ribbon.escape_extension());
        assert_eq!(ribbon.open_dropdown.as_deref(), Some("modify_extension"));
        assert!(ribbon.escape_extension());
        assert_eq!(ribbon.open_dropdown, None);
    }

    #[test]
    fn replacing_the_active_tool_releases_the_previous_glow_track() {
        use std::{thread, time::Duration};
        // Picking tool B while A still runs must hand A's track back: any glow
        // above 0.02 renders as engaged, so a parked track is a button that
        // stays lit for the rest of the session while running nothing.
        let mut ribbon = Ribbon::default();
        ribbon.activate_tool("GLOW_OWN_A");
        ribbon.activate_tool("GLOW_OWN_B");
        assert_eq!(ribbon.active_tool(), Some("GLOW_OWN_B"));

        thread::sleep(Duration::from_millis(230));
        let released = crate::ui::motion::value(crate::ui::motion::glow_key_any("GLOW_OWN_A"));
        let held = crate::ui::motion::value(crate::ui::motion::glow_key_any("GLOW_OWN_B"));
        assert_eq!(released, 0.0, "换人后旧工具的轨道必须归零");
        assert!(held >= crate::ui::motion::REST_GLOW);
    }

    #[test]
    fn a_command_with_no_ribbon_button_releases_the_lit_tool() {
        use std::{thread, time::Duration};
        let mut ribbon = Ribbon::default();
        ribbon.activate_tool("GLOW_OWN_C");
        ribbon.show_command("GLOW_OWN_NOT_ON_ANY_TAB");
        assert_eq!(ribbon.active_tool(), None);

        thread::sleep(Duration::from_millis(230));
        assert_eq!(
            crate::ui::motion::value(crate::ui::motion::glow_key_any("GLOW_OWN_C")),
            0.0
        );
    }
}
