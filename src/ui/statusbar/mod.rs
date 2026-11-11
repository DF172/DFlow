//! Bottom status bar — Model/Layout tabs + OSNAP toggle + status info

pub(crate) mod spacemouse;
pub mod status_menu;
pub mod statusbar_config;
pub mod statusbar_menu;

use iced::widget::tooltip::Position as TipPos;
use iced::widget::{column, container, mouse_area, row, text, text_input, tooltip};
use crate::ui::kit::plain_button;
use iced::{Element, Length, Theme};
use iced_aw::ContextMenu;
use std::sync::Arc;

/// Scrollable id of the status-bar layout-tab strip (retained so the existing
/// `Message::ScrollLayoutTabs` handler still resolves; the strip now flex-wraps
/// instead of scrolling).
pub const LAYOUT_TABS_SCROLL_ID: &str = "statusbar-layout-tabs";

/// Widget id of the inline layout-rename text input, so the rename can grab
/// keyboard focus the moment it opens (issue #86).
pub const LAYOUT_RENAME_INPUT_ID: &str = "layout_rename_input";

use crate::app::Message;
use crate::snap::Snapper;
use crate::t;
use crate::ui::kit;
use crate::ui::style::draftflow_theme::draftflow_colors;
use crate::ui::style::tokens;
use crate::ui::statusbar::status_menu::Entry as StatusMenuEntry;
use crate::ui::statusbar::statusbar_config::{StatusBarConfig, StatusPill};
use crate::ui::wrap_bar::WrapBar;

/// Height of one status-bar row. Matches the drawing-tab strip above it so the
/// three horizontal strips — tabs, status bar, command line — line up, and it
/// is also the reach a status-bar menu keeps around itself (see
/// `status_menu::menu_bar`).
pub const ROW_HEIGHT: f32 = 26.0;

/// A lit toggle's underline stops this far short of the plate on each side.
const RULE_PILL_INSET: f32 = 5.0;

const ST_ANNO_VISIBILITY: &[u8] = include_bytes!("../../../assets/icons/scale_list.svg");
const ST_ANNO_AUTO_ADD: &[u8] = include_bytes!("../../../assets/icons/add_scale.svg");
const ST_VP_SCALE_SYNC: &[u8] = include_bytes!("../../../assets/icons/sync.svg");

pub struct StatusMenuData<'a> {
    /// Cached layout names (shared `Arc`, moved in — never deep-cloned).
    pub layout_names: Arc<Vec<String>>,
    pub polar_custom_input: &'a str,
    pub scale_is_model: bool,
    pub current_scale_name: String,
    /// Cached scale picker list (shared `Arc`, moved in).
    pub scale_list: Arc<Vec<(String, f32, f64)>>,
    pub has_selection: bool,
    /// Entity-type names in the current layout (shared `Arc`, moved in).
    pub selection_types: Arc<Vec<String>>,
    pub selection_filter: &'a rustc_hash::FxHashSet<String>,
    pub tooltip_hidden: bool,
}

/// The lit rung of every pill in the band, as the app currently sees it.
///
/// One value per pill, built once per message by the app
/// (`DraftFlowCAD::status_lights`) and read by BOTH the bar's paint and the
/// update-side motion sync. That is the point of the struct: a pill's plate
/// eases toward `engage`, and `engage` is a *track*, so whoever starts the
/// track and whoever paints the plate must be looking at the same booleans or
/// the pill settles at a rung its own state never asked for.
#[derive(Clone, Copy, Default, Debug)]
pub struct StatusLights {
    pub ortho: bool,
    pub lwt: bool,
    pub polar: bool,
    pub dyn_input: bool,
    pub otrack: bool,
    pub osnap: bool,
    pub space: bool,
    pub anno_visibility: bool,
    pub anno_auto_add: bool,
    /// `Some` only while the active layout has a viewport whose annotation
    /// scale can follow; the pill itself is hidden until then.
    pub vp_scale_sync: Option<bool>,
    pub transparency: bool,
    pub isolate: bool,
    pub quick_props: bool,
    pub filter: bool,
    pub sel_cycle: bool,
    pub clean_screen: bool,
}

/// Every pill that can be lit, by its motion id.
///
/// `kit::surface::pill_style` / `pill_button_style` key their engagement track
/// on `state_key(id)`, and the update-side sync
/// (`DraftFlowCAD::sync_motion`) starts those tracks from [`StatusLights`].
/// A pill painted with an id that is not listed here never eases; an id listed
/// here that no pill paints keeps a track nobody reads. The gate
/// `status_pills_and_lights_agree` compares this list against the ids the bar
/// actually paints, so the two cannot drift.
pub const LIT_PILLS: &[&str] = &[
    "ST_ORTHO",
    "ST_LWT",
    "ST_POLAR",
    "ST_DYN",
    "ST_OTRACK",
    "ST_OSNAP",
    "ST_SPACE",
    "ST_ANNO_VISIBILITY",
    "ST_ANNO_AUTO_ADD",
    "ST_VP_SCALE_SYNC",
    "ST_TRANSPARENCY",
    "ST_ISOLATE",
    "ST_QUICKPROPS",
    "ST_FILTER",
    "ST_SELCYCLE",
    "ST_CLEANSCREEN",
];

impl StatusLights {
    /// Whether `id` is lit. The ids are [`LIT_PILLS`]'s; an unknown id is dark.
    #[must_use]
    pub fn lit(&self, id: &str) -> bool {
        match id {
            "ST_ORTHO" => self.ortho,
            "ST_LWT" => self.lwt,
            "ST_POLAR" => self.polar,
            "ST_DYN" => self.dyn_input,
            "ST_OTRACK" => self.otrack,
            "ST_OSNAP" => self.osnap,
            "ST_SPACE" => self.space,
            "ST_ANNO_VISIBILITY" => self.anno_visibility,
            "ST_ANNO_AUTO_ADD" => self.anno_auto_add,
            "ST_VP_SCALE_SYNC" => self.vp_scale_sync.unwrap_or(false),
            "ST_TRANSPARENCY" => self.transparency,
            "ST_ISOLATE" => self.isolate,
            "ST_QUICKPROPS" => self.quick_props,
            "ST_FILTER" => self.filter,
            "ST_SELCYCLE" => self.sel_cycle,
            "ST_CLEANSCREEN" => self.clean_screen,
            _ => false,
        }
    }
}

#[derive(Clone, Default)]
pub struct StatusBar {
    #[allow(dead_code)]
    pub coord_display: String,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            coord_display: "MODEL".into(),
        }
    }

    pub fn view<'a>(
        &'a self,
        snapper: &'a Snapper,
        lights: &StatusLights,
        polar_increment_deg: f32,
        isometric_drafting: bool,
        iso_plane: crate::app::settings::IsoPlane,
        layouts: Arc<Vec<String>>,
        block_tabs: Vec<String>,
        current_layout: &'a str,
        active_block: Option<String>,
        // Start/welcome view has no drawing to own layouts.
        is_start: bool,
        // If `Some((original, edit_value))`, the named tab shows a text input.
        rename_state: Option<&'a (String, String)>,
        // Scale of the first user viewport in the active paper layout.
        viewport_scale: Option<f64>,
        // Number of user viewports in the current paper layout (0 = model space).
        viewport_count: usize,
        // Whether the layout tabs (Model/Paper) are visible (LAYOUTTAB).
        show_layout_tabs: bool,
        // Current annotation scale for model space (1.0 = 1:1, 50.0 = 1:50, etc.).
        annotation_scale: f32,
        // True when the scale pill is interactive (always model space; paper space only when a viewport is active/selected).
        scale_pill_enabled: bool,
        // Live cursor position in model coordinates, for the coordinate readout.
        cursor_world: glam::DVec3,
        // $COORDS readout mode: 0 = static (updates only on a pick), 1 = live
        // absolute, 2 = polar (distance<angle from the last point while picking).
        coords_mode: i16,
        // The last committed point, for the static (0) and polar (2) readouts.
        last_point: Option<glam::DVec3>,
        // True while a command is prompting for a point (enables the polar readout).
        picking: bool,
        // LUNITS — how lengths are written, for the units pill.
        linear_format: i16,
        // Which pills the user has chosen to show on the bar.
        config: &'a StatusBarConfig,
        menu_data: StatusMenuData<'a>,
        // Remaining degrees of freedom for the current parametric scope.
        // `None` when the scope
        // has no `ParametricConstraintSet` yet (nothing constrained), so the
        // badge stays invisible until it's actually relevant.
        parametric_dof: Option<usize>,
        // Number of redundant/conflicting constraints in the current scope.
        // Zero hides
        // the pill entirely, so an ordinarily/fully-constrained drawing sees
        // no new clutter.
        parametric_conflicts: usize,
        // What is drawing the scene. Only a degraded verdict shows anything.
        gpu_status: &'a crate::scene::pipeline::GpuStatus,
        spacemouse: Option<Element<'a, Message>>,
        theme: &'a Theme,
    ) -> Element<'a, Message> {
        let StatusMenuData {
            layout_names,
            polar_custom_input,
            scale_is_model,
            current_scale_name,
            scale_list,
            has_selection,
            selection_types,
            selection_filter,
            tooltip_hidden,
        } = menu_data;
        // The bar paints from the same `StatusLights` the update-side motion
        // sync starts its engagement tracks from, so a pill's plate and the
        // track it eases on cannot disagree about what "lit" means.
        let StatusLights {
            ortho: ortho_mode,
            lwt: lineweight_display,
            polar: polar_mode,
            dyn_input,
            otrack,
            osnap: osnap_active,
            space: in_mspace,
            anno_visibility: annotation_all_visible,
            anno_auto_add: annotation_auto_add,
            vp_scale_sync: viewport_scale_synced,
            transparency: transparency_display,
            isolate: isolation_active,
            quick_props: quick_properties,
            filter: selection_filter_active,
            sel_cycle: selection_cycling,
            clean_screen,
        } = *lights;

        // Leftmost hamburger: opens a dropdown listing Model + every layout, so
        // a layout can be picked directly even when the tab strip is scrolled.
        let menu_icon = if is_start {
            crate::ui::icons::themed_disabled(crate::ui::icons::MENU, 16.0)
        } else {
            crate::ui::icons::themed_secondary(crate::ui::icons::MENU, 16.0)
        };
        let menu_button = plain_button(menu_icon).style(kit::subtle_style).padding(kit::Size::Compact.padding());
        let menu_btn = if is_start {
            tip(
                menu_button.into(),
                t!("Open or create a drawing to manage layouts."), theme
            )
        } else {
            status_menu::menu_bar(
                menu_tip(
                    menu_button
                        .on_press(Message::StatusMenuTooltipHidden(true))
                        .into(),
                    t!("Model and layout list"),
                    tooltip_hidden, theme
                ),
                statusbar_menu::layout_entries(&layout_names, current_layout),
                200.0,
            )
        };

        let add_button = plain_button(text("+").size(tokens::TEXT_BODY))
            .style(kit::subtle_style)
            .padding(kit::Size::Compact.padding());
        let add_btn = if is_start {
            tip(
                add_button.into(),
                t!("Open or create a drawing to add a layout."), theme
            )
        } else {
            add_button.on_press(Message::LayoutCreate).into()
        };

        // ── Right side ────────────────────────────────────────────────────

        let vp_label = if viewport_count > 0 {
            t!("%{n} VP", n = viewport_count).into_owned()
        } else {
            String::new()
        };
        // Scale pill: opens the scale picker popup.
        // Model space: always interactive, shows annotation scale.
        // Paper space: interactive only when a viewport is active/selected.
        // Keep its text identical to the active drawing-defined scale. Rebuilding
        // the label from the numeric factor turns an architectural
        // `1/2" = 1'-0"` scale into `1:24`, mixing formats in the same control.
        let scale_label = if current_scale_name.is_empty() {
            active_scale_label(
                scale_is_model,
                annotation_scale,
                viewport_scale,
                &scale_list,
            )
            .unwrap_or_else(|| {
                if scale_is_model {
                    format_scale(Some(1.0 / annotation_scale as f64))
                } else {
                    format_scale(viewport_scale)
                }
            })
        } else {
            current_scale_name.clone()
        };
        // The value alone ("1:1") made the pill unreadable to anyone who did
        // not already know what it measured; the item's own translated name
        // rides next to the value here, the same as every other pill.
        let scale_text = format!("{} {}", t!(StatusPill::Scale.label()), scale_label);
        let scale_element: Element<'_, Message> = if scale_pill_enabled {
            status_menu::menu_bar(
                menu_tip(
                    popup_pill(&scale_text),
                    t!("Annotation / Viewport Scale\nClick to change"),
                    tooltip_hidden, theme
                ),
                crate::ui::popup::scale_popup::menu_entries(
                    scale_is_model,
                    &current_scale_name,
                    viewport_scale,
                    &scale_list,
                ),
                if scale_is_model { 150.0 } else { 120.0 },
            )
        } else {
            status_pill("ST_SCALE_READOUT", scale_text).into()
        };
        // Build the right-side pills, honouring the user's per-pill visibility.
        // They live in a flex-wrap flow (WrapFlow) so they spill onto extra rows
        // when the width can't hold them all on one line.
        let vis = |p: StatusPill| config.is_visible(p);
        let mut pills: Vec<Element<'_, Message>> = Vec::new();
        // Indexes of group separators inside `pills`, so ones that end up
        // leading, doubled, or trailing can be dropped after assembly.
        let mut divs: Vec<usize> = Vec::new();
        if vis(StatusPill::SpaceMouse) {
            if let Some(pill) = spacemouse {
                pills.push(pill);
            }
        }
        // Not a `StatusPill` and not hideable: the scene is on a software
        // rasterizer or not drawn at all, and the popup that said so has been
        // dismissed. This stays for the session and reopens it. Many people
        // never read the command line, so this is the one place the fact
        // remains visible.
        if let Some(label) = gpu_pill_label(gpu_status) {
            let detail = match gpu_status {
                crate::scene::pipeline::GpuStatus::Software(adapter) => {
                    format!("{}\n{}", adapter.name, crate::tr!("gpu", "pill-tip"))
                }
                _ => crate::tr!("gpu", "pill-tip"),
            };
            pills.push(tip(warning_pill(label, Message::GpuWarningOpen), detail.into(), theme).into());
        }
        // The coordinate readout leads the bar, next to the layout tabs — it is
        // a readout, not a mode, so it does not belong among the toggles on
        // the right, and it is always on: the bar is where the pointer's
        // position is read, so it is not a user-hideable pill.
        let coords_element: Element<'_, Message> = {
            let coords_label = format_coords(cursor_world, last_point, coords_mode, picking);
            tip(
                action_pill(&coords_label, Message::CycleCoordsMode),
                t!("Cursor coordinates ($COORDS)\nClick to cycle: static / live / polar"), theme
            )
            .into()
        };
        // Not a `StatusPill` (so not user-hideable yet): invisible until the
        // current scope actually has a ParametricConstraintSet, so a drawing
        // that never uses parametric constraints sees no new clutter.
        if let Some(dof) = parametric_dof {
            let pill = if dof == 0 {
                success_pill(crate::tf!("DOF: {dof}").into_owned())
            } else {
                status_pill("ST_DOF", crate::tf!("DOF: {dof}").into_owned())
            };
            pills.push(
                tip(
                    pill,
                    if dof == 0 {
                        t!("Fully constrained — no remaining degrees of freedom")
                    } else {
                        t!("Remaining degrees of freedom in the current parametric constraint scope")
                    }, theme
                )
                .into(),
            );
        }
        // Each click removes one flagged constraint.
        if parametric_conflicts > 0 {
            pills.push(
                tip(
                    action_pill(
                        crate::tf!("⚠ {parametric_conflicts} conflicting").into_owned(),
                        Message::ResolveOneParametricConflict,
                    ),
                    t!("One or more parametric constraints conflict or are redundant\nClick to remove one and re-solve"), theme
                )
                .into(),
            );
        }
        if vis(StatusPill::Ortho) {
            pills.push(
                tip(
                    toggle_pill(
                        "ST_ORTHO",
                        crate::ui::icons::ST_ORTHO,
                        t!(StatusPill::Ortho.label()).into_owned(),
                        ortho_mode,
                        Message::ToggleOrtho,
                    ),
                    t!("Orthogonal Mode\nF8"), theme
                )
                .into(),
            );
        }
        if vis(StatusPill::Lwt) {
            pills.push(
                tip(
                    toggle_pill(
                        "ST_LWT",
                        crate::ui::icons::ST_LWT,
                        t!(StatusPill::Lwt.label()).into_owned(),
                        lineweight_display,
                        Message::ToggleLineweightDisplay,
                    ),
                    t!("Show Lineweight\nLWDISPLAY"), theme
                )
                .into(),
            );
        }
        if vis(StatusPill::Polar) {
            pills.push(
                polar_pill(
                    polar_mode,
                    polar_increment_deg,
                    tooltip_hidden,
                    crate::ui::popup::polar_popup::menu_entries(
                        polar_increment_deg,
                        polar_custom_input,
                    ), theme
                )
                .into(),
            );
        }
        if vis(StatusPill::Dyn) {
            pills.push(
                tip(
                    toggle_pill(
                        "ST_DYN",
                        crate::ui::icons::ST_DYN,
                        t!(StatusPill::Dyn.label()).into_owned(),
                        dyn_input,
                        Message::ToggleDynInput,
                    ),
                    t!("Dynamic Input\nF12"), theme
                )
                .into(),
            );
        }
        if vis(StatusPill::Otrack) {
            pills.push(
                tip(
                    toggle_pill(
                        "ST_OTRACK",
                        crate::ui::icons::ST_OTRACK,
                        t!(StatusPill::Otrack.label()).into_owned(),
                        otrack,
                        Message::ToggleOTrack,
                    ),
                    t!("Object Snap Tracking\nF11"), theme
                )
                .into(),
            );
        }
        if vis(StatusPill::Osnap) {
            pills.push(
                osnap_btn(
                    osnap_active,
                    snapper.snap_enabled,
                    tooltip_hidden,
                    crate::ui::popup::snap_popup::menu_entries(
                        snapper,
                        isometric_drafting,
                        iso_plane,
                    ), theme
                )
                .into(),
            );
        }
        // Drawing aids | annotation group.
        divs.push(pills.len());
        pills.push(kit::rule_v(ROW_HEIGHT, "").into());
        if vis(StatusPill::Space) {
            pills.push(
                tip(
                    space_mode_btn(current_layout, in_mspace),
                    t!("PAPER: double-click viewport to enter MSPACE\nMODEL: click to switch to Model Space"), theme
                )
                .into(),
            );
        }
        if vis(StatusPill::Scale) {
            pills.push(scale_element);
        }
        if vis(StatusPill::AnnoVisibility) {
            pills.push(
                tip(
                    toggle_pill(
                        "ST_ANNO_VISIBILITY",
                        ST_ANNO_VISIBILITY,
                        t!(StatusPill::AnnoVisibility.label()).into_owned(),
                        annotation_all_visible,
                        Message::ToggleAnnotationVisibility,
                    ),
                    t!("Show Annotation Objects"), theme
                )
                .into(),
            );
        }
        if vis(StatusPill::AnnoAutoAdd) {
            pills.push(
                tip(
                    toggle_pill(
                        "ST_ANNO_AUTO_ADD",
                        ST_ANNO_AUTO_ADD,
                        t!(StatusPill::AnnoAutoAdd.label()).into_owned(),
                        annotation_auto_add,
                        Message::ToggleAnnotationAutoAdd,
                    ),
                    t!("Automatically Add Scales"), theme
                )
                .into(),
            );
        }
        if vis(StatusPill::VpScaleSync) {
            if let Some(synced) = viewport_scale_synced {
                pills.push(
                    tip(
                        toggle_pill(
                            "ST_VP_SCALE_SYNC",
                            ST_VP_SCALE_SYNC,
                            t!(StatusPill::VpScaleSync.label()).into_owned(),
                            synced,
                            Message::SyncViewportAnnotationScale,
                        ),
                        t!("Viewport / Annotation Scale Sync"), theme
                    )
                    .into(),
                );
            }
        }
        if vis(StatusPill::Units) {
            pills.push(
                status_menu::menu_bar(
                    menu_tip(
                        popup_pill(t!(crate::modules::draw::units::linear_format_short(
                            linear_format
                        ))),
                        t!("Units (LUNITS)\nHow lengths are written\nClick to change"),
                        tooltip_hidden, theme
                    ),
                    crate::ui::popup::units_popup::menu_entries(linear_format),
                    140.0,
                )
                .into(),
            );
        }
        if vis(StatusPill::Transparency) {
            pills.push(
                tip(
                    toggle_pill(
                        "ST_TRANSPARENCY",
                        crate::ui::icons::ST_TRANSPARENCY,
                        t!(StatusPill::Transparency.label()).into_owned(),
                        transparency_display,
                        Message::ToggleTransparencyDisplay,
                    ),
                    t!("Show Transparency\nForce opaque when off"), theme
                )
                .into(),
            );
        }
        // Annotation group | selection group.
        divs.push(pills.len());
        pills.push(kit::rule_v(ROW_HEIGHT, "").into());
        if vis(StatusPill::Isolate) {
            pills.push(
                status_menu::menu_bar(
                    menu_tip(
                        toggle_pill(
                            "ST_ISOLATE",
                            crate::ui::icons::ST_ISOLATE,
                            t!(StatusPill::Isolate.label()).into_owned(),
                            isolation_active,
                            Message::StatusMenuTooltipHidden(true),
                        ),
                        t!("Isolate Objects\nClick for Isolate / Hide / End"),
                        tooltip_hidden, theme
                    ),
                    crate::ui::popup::isolate_popup::menu_entries(has_selection, isolation_active),
                    160.0,
                )
                .into(),
            );
        }
        if vis(StatusPill::QuickProps) {
            pills.push(
                tip(
                    toggle_pill(
                        "ST_QUICKPROPS",
                        crate::ui::icons::ST_QUICKPROPS,
                        t!(StatusPill::QuickProps.label()).into_owned(),
                        quick_properties,
                        Message::ToggleQuickProperties,
                    ),
                    t!("Quick Properties\nFloating panel on selection"), theme
                )
                .into(),
            );
        }
        if vis(StatusPill::SelFilter) {
            pills.push(
                status_menu::menu_bar(
                    menu_tip(
                        toggle_pill(
                            "ST_FILTER",
                            crate::ui::icons::ST_FILTER,
                            t!(StatusPill::SelFilter.label()).into_owned(),
                            selection_filter_active,
                            Message::StatusMenuTooltipHidden(true),
                        ),
                        t!("Selection Filtering\nLimit which object types can be picked"),
                        tooltip_hidden, theme
                    ),
                    crate::ui::popup::selection_filter_popup::menu_entries(
                        &selection_types,
                        selection_filter,
                    ),
                    180.0,
                )
                .into(),
            );
        }
        if vis(StatusPill::SelCycle) {
            pills.push(
                tip(
                    toggle_pill(
                        "ST_SELCYCLE",
                        crate::ui::icons::ST_SELCYCLE,
                        t!(StatusPill::SelCycle.label()).into_owned(),
                        selection_cycling,
                        Message::ToggleSelectionCycling,
                    ),
                    t!("Selection Cycling\nRepeat-click to step through overlapping objects"), theme
                )
                .into(),
            );
        }
        // Selection group | screen group.
        divs.push(pills.len());
        pills.push(kit::rule_v(ROW_HEIGHT, "").into());
        if vis(StatusPill::Vp) && !vp_label.is_empty() {
            pills.push(
                tip(
                    status_pill("ST_VP_COUNT", vp_label).into(),
                    t!("Viewport count in active layout"), theme
                )
                .into(),
            );
        }
        if vis(StatusPill::CleanScreen) {
            pills.push(
                tip(
                    toggle_pill(
                        "ST_CLEANSCREEN",
                        crate::ui::icons::ST_CLEANSCREEN,
                        t!(StatusPill::CleanScreen.label()).into_owned(),
                        clean_screen,
                        Message::ToggleCleanScreen,
                    ),
                    t!("Clean Screen\nHide ribbon and panels"), theme
                )
                .into(),
            );
        }
        // Screen group | customization handle.
        divs.push(pills.len());
        pills.push(kit::rule_v(ROW_HEIGHT, "").into());
        // Customization handle: opens the pill show/hide menu.
        pills.push(
            status_menu::menu_bar(
                menu_tip(
                    customize_btn(),
                    t!("Customization\nShow or hide status-bar items"),
                    tooltip_hidden, theme
                ),
                statusbar_menu::customization_entries(config),
                200.0,
            )
            .into(),
        );
        // Drop separators that would lead, double up, or trail: the pills
        // around them are user-hideable, so grouping shifts at runtime.
        let div_set: rustc_hash::FxHashSet<usize> = divs.iter().copied().collect();
        let mut cleaned: Vec<Element<'_, Message>> = Vec::with_capacity(pills.len());
        let mut prev_div = true;
        for (i, el) in pills.into_iter().enumerate() {
            let is_div = div_set.contains(&i);
            if is_div && prev_div {
                continue;
            }
            prev_div = is_div;
            cleaned.push(el);
        }
        if prev_div {
            cleaned.pop();
        }
        let right_status = iced::widget::Row::with_children(cleaned)
            .spacing(tokens::GAP_CONTROL)
            .align_y(iced::Center)
            .wrap()
            .vertical_spacing(0.0)
            .align_x(iced::alignment::Horizontal::Right);

        // Left area: hamburger menu + Model/layout tabs in a flex-wrap flow, so
        // they spill onto lower rows when narrow (no scroll arrows). The pills
        // use the remaining space on the final tab row when they fit; otherwise
        // WrapBar adds another right-aligned row.
        let mut left: Vec<Element<'_, Message>> = Vec::new();
        left.push(menu_btn);
        left.push(coords_element);
        if show_layout_tabs {
            // Paper layouts only: `layouts[0]` is always "Model". Derived once
            // from the shared `Arc` and cloned (cheap refcount bump) per tab.
            let reorderable_layouts: Arc<[String]> = Arc::from(layouts.get(1..).unwrap_or(&[]));
            // Shared once per frame: block tabs carry no reorder set, so reuse
            // one empty Arc (refcount bump) instead of allocating per tab.
            let empty_reorderable: Arc<[String]> = Arc::from([] as [String; 0]);
            for name in layouts.iter() {
                let is_active = active_block.is_none() && name.as_str() == current_layout;
                let renaming = rename_state
                    .filter(|(orig, _)| orig.as_str() == name.as_str())
                    .map(|(_, edit)| edit.as_str());
                let switch_msg = Message::LayoutSwitch(name.to_owned());
                left.push(
                    space_tab(
                        name.to_owned(),
                        is_active,
                        renaming,
                        !is_start,
                        true,
                        reorderable_layouts.clone(),
                        switch_msg,
                        "SB_LAYOUT_TAB", theme
                    )
                    .into(),
                );
            }
            for name in block_tabs.iter() {
                let is_active = active_block.as_deref() == Some(name.as_str());
                let switch_msg = Message::BlockEditSwitch(name.to_owned());
                left.push(
                    space_tab(
                        name.to_owned(),
                        is_active,
                        None,
                        !is_start,
                        false,
                        empty_reorderable.clone(),
                        switch_msg,
                        "SB_BLOCK_TAB", theme
                    )
                    .into(),
                );
            }
            left.push(add_btn.into());
        }
        let left_area = iced::widget::Row::with_children(left)
            .spacing(tokens::GAP_HAIRLINE)
            .align_y(iced::Center)
            .wrap()
            .vertical_spacing(0.0);

        let wrap = WrapBar::new(left_area.into(), right_status.into())
            .min_row_h(ROW_HEIGHT)
            .justify_end(true);

        // Under the sheet the bar is the page itself: no fill, no seam, no
        // box — one row of small readouts floating under the canvas, which is
        // where the sheet puts it. The vendor themes keep their banded strip
        // and the hairline that closes it.
        let sheet = crate::ui::style::draftflow_theme::draftflow_colors(theme).is_some();
        let bar = kit::strip(
            wrap,
            iced::Padding::from(if sheet {
                [tokens::GAP_CONTROL, tokens::GAP_SECTION]
            } else {
                [tokens::GAP_HAIRLINE, tokens::GAP_CONTROL]
            }),
            "",
        )
        .style(move |theme: &Theme| {
            if sheet {
                kit::Tier::Flush.style(theme, "")
            } else {
                kit::sheet_strip_style(kit::Tier::Sink, theme, "")
            }
        })
        .width(Length::Fill);
        if sheet {
            bar.into()
        } else {
            column![kit::sheet_rule(), bar].width(Length::Fill).into()
        }
    }
}

// ── Coordinate readout ────────────────────────────────────────────────────

/// The readout, in the drawing's own linear units.
///
/// Lengths went out at four decimal places whatever the drawing asked for, so a
/// drawing set to architectural units read its coordinates in decimals, and one
/// asking for two places got four. `format_length` is the same helper the
/// properties panel formats through, so both now say a length the same way.
fn format_coords(
    cursor: glam::DVec3,
    last: Option<glam::DVec3>,
    mode: i16,
    picking: bool,
) -> String {
    use crate::entities::common::format_length as len;
    let abs = |p: glam::DVec3| format!("{}, {}, {}", len(p.x), len(p.y), len(p.z));
    match mode {
        // Static: show the last picked point; the readout freezes between picks.
        0 => abs(last.unwrap_or(cursor)),
        // Polar: distance < angle relative to the last point while a command is
        // prompting for a point; absolute otherwise.
        2 => match (picking, last) {
            (true, Some(l)) => {
                let d = cursor - l;
                let dist = (d.x * d.x + d.y * d.y).sqrt();
                // A bearing, so it is counted from the drawing's zero and runs
                // the way the drawing says — and comes back out in the same
                // form the polar input accepts.
                let ang = d.y.atan2(d.x);
                format!(
                    "{} < {}",
                    len(dist),
                    crate::entities::common::format_direction(ang)
                )
            }
            _ => abs(cursor),
        },
        // 1 (default) and anything else: live absolute.
        _ => abs(cursor),
    }
}

// ── Customization handle ──────────────────────────────────────────────────

fn customize_btn() -> Element<'static, Message> {
    plain_button(crate::ui::icons::themed_secondary(
        crate::ui::icons::MENU,
        18.0,
    ))
    .on_press(Message::StatusMenuTooltipHidden(true))
    .style(kit::subtle_style)
    .padding(kit::Size::Compact.padding())
    .into()
}

// ── Tooltip helper ────────────────────────────────────────────────────────

fn tip<'a>(
    content: Element<'a, Message>,
    label: std::borrow::Cow<'static, str>,
    theme: &Theme,
) -> Element<'a, Message> {
    tip_node(content, text(label).size(tokens::TEXT_LABEL).into(), theme)
}

/// A menu root shows its tooltip until clicked. Moving from the root into the
/// opened menu resets suppression for the next hover without covering the menu.
fn menu_tip<'a>(
    content: Element<'a, Message>,
    label: std::borrow::Cow<'static, str>,
    hidden: bool,
    theme: &Theme,
) -> Element<'a, Message> {
    let content = if hidden { content } else { tip(content, label, theme) };

    mouse_area(content)
        .on_exit(Message::StatusMenuTooltipHidden(false))
        .into()
}

/// Like [`tip`] but the tooltip body is any element — used to embed an SVG
/// glyph (e.g. the dropdown caret) instead of a Unicode character that renders
/// as tofu on the web. (#138)
///
/// `theme` is the palette the plate is mixed from. A glass pane resolves its
/// colours while it is built — the shader that draws it is never handed a theme
/// of its own — and the tooltip widget is told to paint nothing so the pane is
/// the only plate: its default class would draw a solid box 5 px wider than the
/// glass on every side.
fn tip_node<'a>(
    content: Element<'a, Message>,
    body: Element<'a, Message>,
    theme: &Theme,
) -> Element<'a, Message> {
    tooltip(
        content,
        kit::glass_pane(
            container(body)
                .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
                .padding(kit::Size::Compact.padding())
                .into(),
            theme,
            false,
            "",
        ),
        TipPos::Top,
    )
    .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
    .into()
}

// ── Simple toggle pill ────────────────────────────────────────────────────

/// A status-bar toggle: an 18 px tinted icon and the item's translated name,
/// side by side. Icon-only was the earlier answer to issue #216 (size-10 text
/// was unreadable) but left the whole row unnameable at a glance, so the name
/// is back at the readable rung and the row is spaced by `GAP_CONTROL`. The
/// tooltip each call site wraps it with still carries the shortcut and the
/// longer explanation. `key` feeds the motion registry so the pill's hover
/// tint fades in instead of stepping.
///
/// Both rungs come from the kit's pill ladder: [`kit::surface::pill_button_style`]
/// paints the plate and the ink together, and the icon asks the same ladder for
/// its colour. The old code did neither — it painted the active state from
/// `Role::Engaged`, then re-spelled the resting state by hand over
/// `button::subtle` with a third plate (`tokens::surface(Rank::Band)`) that no
/// other pill used, and inked the icon from `primary.weak.text`, a slot the
/// pill stopped painting when the ladder moved to the aurora wash.
fn toggle_pill(
    key: &'static str,
    icon: &'static [u8],
    label: String,
    active: bool,
    msg: Message,
) -> Element<'static, Message> {
    let hover_key = crate::ui::motion::hover_key(key);
    let ink = move |theme: &Theme| kit::surface::pill_ink(key, active, theme, "");
    let pill = mouse_area(
        plain_button(
            row![
                crate::ui::icons::themed_by(icon, 18.0, ink.clone()),
                text(label)
                    .size(tokens::TEXT_LABEL)
                    .style(move |theme: &Theme| kit::color_style(ink(theme))),
            ]
            .spacing(tokens::GAP_TIGHT)
            .align_y(iced::Center),
        )
            .on_press(msg)
            .style(move |theme: &Theme, status| {
                kit::surface::pill_button_style(key, active, theme, status, "")
            })
            .padding([tokens::GAP_TIGHT, tokens::GAP_CONTROL])
            .height(Length::Fixed(ROW_HEIGHT - kit::RULE_THICKNESS)),
    )
    .on_enter(Message::UiHover {
        key: hover_key,
        entered: true,
    })
    .on_exit(Message::UiHover {
        key: hover_key,
        entered: false,
    });
    kit::rule_under_inset_at(
        pill,
        crate::ui::motion::engage(crate::ui::motion::state_key(key), active),
        RULE_PILL_INSET,
    )
}

// ── Split pill (toggle + dropdown caret) ──────────────────────────────────
//
// Shared chrome for every status-bar pill that pairs a toggle with a picker
// popup (OSNAP, POLAR, …): one outer border wraps the caller's `main` region
// and a dropdown caret, so both halves read as a single integrated control and
// the border / background / caret / sizing live in exactly ONE place.

/// Wrap a caller-built, already-click-wired `main` element together with its
/// menu-bearing dropdown caret.
fn split_pill<'a>(
    key: &'static str,
    main: Element<'static, Message>,
    caret: Element<'a, Message>,
    active: bool,
) -> Element<'a, Message> {
    let hover_key = crate::ui::motion::hover_key(key);
    let pill = mouse_area(
        container(row![main, caret].spacing(tokens::GAP_TIGHT).align_y(iced::Center))
            .style(move |theme: &Theme| kit::pill_style(key, active, theme, ""))
            .padding(kit::PAD_PLATE)
            .center_y(Length::Fixed(ROW_HEIGHT - kit::RULE_THICKNESS)),
    )
    .on_enter(Message::UiHover {
        key: hover_key,
        entered: true,
    })
    .on_exit(Message::UiHover {
        key: hover_key,
        entered: false,
    });
    kit::rule_under_inset_at(
        pill,
        crate::ui::motion::engage(crate::ui::motion::state_key(key), active),
        RULE_PILL_INSET,
    )
}

// ── Polar tracking pill ───────────────────────────────────────────────────
//
// Main: left-click toggles polar on/off; right-click cycles the increment.
// Caret: opens the angle picker.

fn polar_pill<'a>(
    active: bool,
    increment_deg: f32,
    tooltip_hidden: bool,
    entries: Vec<StatusMenuEntry<'a>>,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let angle = crate::ui::popup::polar_popup::angle_label(increment_deg);
    let tooltip_text = t!(
        "Polar Tracking (%{angle})\nF10 — left-click on/off\nRight-click cycles · ▾ picks angle",
        angle = angle,
    );

    // Right-click quick-cycles through the same increments the picker lists.
    const CYCLE: &[f32] = &[90.0, 45.0, 30.0, 22.5, 18.0, 15.0, 10.0, 5.0, 1.0];
    let next_angle = match CYCLE.iter().position(|&a| (a - increment_deg).abs() < 1e-3) {
        Some(i) => CYCLE[(i + 1) % CYCLE.len()],
        None => 45.0,
    };

    // Every ink inside this pill comes from the ladder that paints the plate
    // under it (`split_pill` → `kit::pill_style`), asked by id so the caret, the
    // icon and the angle read the same pointer channel the plate walks.
    let ink = move |theme: &Theme| kit::surface::pill_ink("ST_POLAR", active, theme, "");
    let polar_icon = crate::ui::icons::themed_by(crate::ui::icons::ST_POLAR, 18.0, ink.clone());
    let main = mouse_area(
        row![
            polar_icon,
            text(t!(StatusPill::Polar.label()).into_owned())
                .size(tokens::TEXT_LABEL)
                .style(move |theme: &Theme| kit::color_style(ink(theme))),
            text(angle).size(tokens::TEXT_LABEL).style(move |theme: &Theme| {
                kit::color_style(ink(theme))
            }),
        ]
        .spacing(tokens::GAP_HAIRLINE)
        .align_y(iced::Center),
    )
    .on_press(Message::TogglePolar)
    .on_right_press(Message::SetPolarAngle(next_angle));
    let main = tooltip(
        main,
        kit::glass_pane(
            container(text(tooltip_text).size(tokens::TEXT_LABEL))
                .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
                .padding(kit::Size::Compact.padding())
                .into(),
            theme,
            false,
            "",
        ),
        TipPos::Top,
    )
    .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""));

    let caret = status_menu::menu_bar(
        menu_tip(
            mouse_area(
                container(crate::ui::icons::themed_down_by(
                    9.0,
                    move |theme: &Theme| kit::surface::pill_ink("ST_POLAR", active, theme, ""),
                ))
                .padding([tokens::GAP_TIGHT, tokens::GAP_CONTROL]),
            )
            .on_press(Message::StatusMenuTooltipHidden(true))
            .into(),
            t!("Polar angle\nClick to choose"),
            tooltip_hidden, theme
        ),
        entries,
        120.0,
    );

    split_pill("ST_POLAR", main.into(), caret, active)
}

// ── OSNAP pill ─────────────────────────────────────────────────────────────
//
// Main: toggles the global snap on/off. Caret: opens the snap-type dropdown.

fn osnap_btn<'a>(
    active: bool,
    snap_enabled: bool,
    tooltip_hidden: bool,
    entries: Vec<StatusMenuEntry<'a>>,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let on = active || snap_enabled;
    let ink = move |theme: &Theme| kit::surface::pill_ink("ST_OSNAP", on, theme, "");
    let snap_icon = crate::ui::icons::themed_by(crate::ui::icons::ST_OSNAP, 18.0, ink.clone());
    let main = tip(
        mouse_area(
            row![
                snap_icon,
                text(t!(StatusPill::Osnap.label()).into_owned())
                    .size(tokens::TEXT_LABEL)
                    .style(move |theme: &Theme| kit::color_style(ink(theme))),
            ]
            .spacing(tokens::GAP_TIGHT)
            .align_y(iced::Center),
        )
            .on_press(Message::ToggleSnapEnabled)
            .into(),
        t!("Object Snap: toggle on/off\nF3"), theme
    );

    let caret = status_menu::menu_bar(
        menu_tip(
            mouse_area(
                container(crate::ui::icons::themed_down_by(
                    9.0,
                    move |theme: &Theme| kit::surface::pill_ink("ST_OSNAP", on, theme, ""),
                ))
                .padding([tokens::GAP_TIGHT, tokens::GAP_CONTROL]),
            )
            .on_press(Message::StatusMenuTooltipHidden(true))
            .into(),
            t!("Object Snap list\nClick to choose snap types"),
            tooltip_hidden, theme
        ),
        entries,
        210.0,
    );

    split_pill("ST_OSNAP", main, caret, on)
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn layout_tab_context_menu(name: String, theme: &Theme) -> Element<'static, Message> {
    let rename = mouse_area(
        container(text(t!("Rename")).size(tokens::TEXT_BODY))
            .padding(kit::PAD_MENU_ROW)
            .width(Length::Fill),
    )
    .on_press(Message::LayoutRenameStart(name.clone()))
    .interaction(iced::mouse::Interaction::Pointer);

    let delete = mouse_area(
        container(text(t!("Delete")).size(tokens::TEXT_BODY))
            .padding(kit::PAD_MENU_ROW)
            .width(Length::Fill),
    )
    .on_press(Message::LayoutDelete(name))
    .interaction(iced::mouse::Interaction::Pointer);

    // A right-click menu floats over the drawing, so its plate reads the frame
    // behind it instead of plating an opaque box. The host keeps its inset and
    // width and paints nothing itself.
    kit::glass_pane(
        container(column![rename, delete].spacing(tokens::GAP_NONE).width(160))
            .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
            .padding([tokens::GAP_TIGHT, tokens::GAP_NONE])
            .into(),
        theme,
        false,
        "",
    )
}

/// A layout tab button.
///
/// When `rename_edit` is `Some(value)` the tab shows an inline text input
/// instead of the normal button.  The tab is not renameable when it is the
/// "Model" tab (callers simply never pass `Some` for that name).
fn space_tab<'a>(
    label: String,
    is_active: bool,
    rename_edit: Option<&'a str>,
    enabled: bool,
    localizes_name: bool,
    reorderable_layouts: Arc<[String]>,
    switch_msg: Message,
    report_key_prefix: &'static str,
    theme: &'a Theme,
) -> Element<'a, Message> {
    // A layout tab's ink is the one place its name may read Chinese: `label` stays
    // the stored name, because the reorder set, the context menu and the switch
    // message all compare against it. Block tabs pass `false` -- a block someone
    // named `Model` is not the model space tab.
    let shown = if localizes_name {
        crate::i18n::app_name_display(&label)
    } else {
        label.clone()
    };
    // A tab's ink and its plate are two separate kit rungs. Under the sheet the
    // lit tab is a panel chip with a 2 px ink rule on its top edge and body ink
    // for its label; every other tab is a faint name that walks up to ink under
    // the pointer. The vendor themes keep their accent wash.
    let report_key = format!("{report_key_prefix}:{label}");
    let hover_key = crate::ui::motion::glow_key_any(&report_key);
    let hover = move || crate::ui::motion::value(hover_key).clamp(0.0, 1.0);
    let ink = move |theme: &Theme| {
        if let Some(colors) = draftflow_colors(theme) {
            if !enabled {
                return kit::color_style(colors.faint);
            }
            if is_active {
                return kit::color_style(colors.ink);
            }
            return kit::color_style(crate::ui::motion::mix(colors.faint, colors.ink, hover()));
        }
        if !enabled {
            kit::faint_style(theme)
        } else if is_active {
            kit::tone_style(theme, kit::Tone::Accent)
        } else {
            kit::muted_style(theme)
        }
    };
    let paint = move |theme: &Theme| kit::sheet_chip_style(theme, is_active, hover(), "");

    if !enabled {
        let mut label_text = text(shown.clone()).size(tokens::TEXT_LABEL).style(ink);
        if is_active {
            label_text = label_text.font(tokens::UI_FONT_MEDIUM);
        }
        let display = container(label_text)
            .style(paint)
            .padding(iced::Padding::from([
                tokens::GAP_NONE,
                tokens::GAP_SECTION,
            ]))
            // The rule lands on the row's own edge only when the tab is one
            // `RULE` shorter than the row; `WrapBar` centres its blocks.
            .center_y(Length::Fixed(ROW_HEIGHT - kit::RULE_THICKNESS));
        crate::ui::wrap_bar::PosReport::owned(
            report_key,
            tip(
                display.into(),
                t!("Open or create a drawing to switch layouts."), theme
            ),
        )
        .into()
    } else if let Some(edit_val) = rename_edit {
        // Inline rename text input with a cancel (✕) button.
        let input = text_input("", edit_val)
            .id(iced::widget::Id::new(LAYOUT_RENAME_INPUT_ID))
            .on_input(Message::LayoutRenameEdit)
            .on_submit(Message::LayoutRenameCommit)
            .size(tokens::TEXT_BODY)
            .padding(kit::PAD_FIELD)
            .width(Length::Fixed(90.0));

        let cancel_btn = plain_button(crate::ui::icons::themed_secondary(
            crate::ui::icons::CLOSE,
            10.0,
        ))
        .on_press(Message::LayoutRenameCancel)
        .style(kit::subtle_style)
        .padding([tokens::GAP_TIGHT, tokens::GAP_TIGHT]);

        row![input, cancel_btn]
            .spacing(tokens::GAP_NONE)
            .align_y(iced::Center)
            .into()
    } else {
        // Normal clickable tab — left click switches. Paper-layout tabs are
        // wrapped in `ContextMenu`, which owns right-click handling.
        let mut label_text = text(shown.clone()).size(tokens::TEXT_LABEL).style(ink);
        if is_active {
            label_text = label_text.font(tokens::UI_FONT_MEDIUM);
        }
        let display = container(label_text)
            .style(paint)
            .padding(iced::Padding::from([
                tokens::GAP_NONE,
                tokens::GAP_SECTION,
            ]))
            .center_y(Length::Fixed(ROW_HEIGHT - kit::RULE_THICKNESS));

        let has_context_menu = reorderable_layouts.contains(&label);
        // The rule is drawn by a widget, not by a fill-width strip: `WrapBar`
        // measures its blocks under unbounded limits, where `Fill` has no width
        // to resolve against.
        let tab = mouse_area(kit::rule_over(display, is_active))
            .on_press(switch_msg)
            .on_enter(Message::UiHover {
                key: hover_key,
                entered: true,
            })
            .on_exit(Message::UiHover {
                key: hover_key,
                entered: false,
            });
        let tab: Element<'a, Message> = if has_context_menu {
            crate::ui::wrap_bar::ReorderTab::layout(label.clone(), reorderable_layouts, tab).into()
        } else {
            tab.into()
        };

        let tab = if has_context_menu {
            ContextMenu::new(tab, move || layout_tab_context_menu(label.clone(), theme)).into()
        } else {
            tab
        };
        crate::ui::wrap_bar::PosReport::owned(report_key, tab).into()
    }
}

/// Space-mode toggle button in the status bar.
///
/// - Model tab            → "MODEL"  (non-clickable, informational)
/// - Layout, PSPACE       → "PAPER"  (click → MspaceCommand: enter MSPACE)
/// - Layout, MSPACE       → "MODEL"  (click → ExitViewport: return to PSPACE)
fn space_mode_btn(current_layout: &str, in_mspace: bool) -> Element<'static, Message> {
    let is_model_tab = current_layout == "Model";

    // Labels and styling follow the conventional drafting layout:
    //   PAPER = currently in paper-space editing
    //   MODEL = currently in model-space editing (either the Model tab or MSPACE)
    let (label, active, on_press) = if is_model_tab {
        (t!("MODEL"), false, None::<Message>)
    } else if in_mspace {
        (t!("MODEL"), true, Some(Message::ExitViewport))
    } else {
        (t!("PAPER"), false, Some(Message::MspaceCommand))
    };

    let mut btn = plain_button(text(label).size(tokens::TEXT_LABEL))
        .style(move |theme: &Theme, status| {
        // The lit rung of this capsule is the same pill ladder every other
        // mode pill in the band rides (`split_pill` → `kit::pill_style`), so it
        // gets the kit's painter rather than a hand-mixed pair of primary slots.
        // The old code painted `primary.weak.color` with `primary.weak.text` and
        // jumped to `primary.base.color` on hover — a rung that exists nowhere
        // else in the app, and an ink contrast solved for a plate the rest of
        // the bar had already stopped drawing.
        kit::surface::pill_button_style("ST_SPACE", active, theme, status, "")
    })
    .padding([tokens::GAP_TIGHT, tokens::GAP_CONTROL])
    .height(Length::Fixed(ROW_HEIGHT - kit::RULE_THICKNESS));

    if let Some(msg) = on_press {
        btn = btn.on_press(msg);
    }

    kit::rule_under_inset_at(
        btn,
        crate::ui::motion::engage(crate::ui::motion::state_key("ST_SPACE"), active),
        RULE_PILL_INSET,
    )
}

fn status_pill(id: &'static str, label: impl Into<String>) -> Element<'static, Message> {
    container(text(label.into()).size(tokens::TEXT_LABEL))
        .style(move |theme: &Theme| kit::pill_style(id, false, theme, ""))
        .padding(kit::Size::Compact.padding())
        .into()
}

/// A success-colored status pill used when no degrees of freedom remain.
/// The status-bar text for a degraded graphics verdict; `None` when there is
/// nothing to say, which is the case the bar must not clutter.
fn gpu_pill_label(status: &crate::scene::pipeline::GpuStatus) -> Option<String> {
    use crate::scene::pipeline::GpuStatus;
    match status {
        GpuStatus::Software(_) => Some(crate::tr!("gpu", "pill-software")),
        GpuStatus::NoRenderer => Some(crate::tr!("gpu", "pill-no-renderer")),
        GpuStatus::Unknown | GpuStatus::Hardware(_) => None,
    }
}

/// A pill in the theme's warning colours: the only status-bar item that
/// means "something is wrong", so it must not look like a toggle.
fn warning_pill(label: impl Into<String>, msg: Message) -> Element<'static, Message> {
    plain_button(text(label.into()).size(tokens::TEXT_LABEL))
        .on_press(msg)
        .style(kit::signal_button_style(kit::Tone::Warning, kit::Tier::Band))
        .padding(kit::Size::Compact.padding())
        .into()
}

fn success_pill(label: impl Into<String>) -> Element<'static, Message> {
    // The verdict is carried by the ink (the kit's success tone); the plate it
    // sits on is the same group rung as every other capsule in the band. The
    // `[4, 8]` is the pill grid, shared with the toggles beside it.
    kit::wash(
        text(label.into())
            .size(tokens::TEXT_LABEL)
            .style(|theme: &Theme| kit::tone_style(theme, kit::Tone::Success)),
        kit::Tier::Plate,
        iced::Padding::from([4, 8]),
        "",
    )
    .into()
}

// ── Scale popup button ────────────────────────────────────────────────────

/// Shared visual root for labelled status-bar menus (units, scale, …).
fn popup_pill(label: impl Into<String>) -> Element<'static, Message> {
    action_pill(label, Message::StatusMenuTooltipHidden(true))
}

fn action_pill(label: impl Into<String>, msg: Message) -> Element<'static, Message> {
    let label = label.into();
    plain_button(text(label).size(tokens::TEXT_LABEL))
        .on_press(msg)
        .style(kit::readout_style)
        .padding([tokens::GAP_TIGHT, tokens::GAP_CONTROL])
        .into()
}

// ── Scale display ─────────────────────────────────────────────────────────

/// Formats a viewport scale factor as a human-readable ratio string.
///
/// - `None`  → "1:1"  (model space or no viewport yet)
/// - `1.0`   → "1:1"
/// - `0.02`  → "1:50"
/// - `2.0`   → "2:1"
fn format_scale(scale: Option<f64>) -> String {
    let s = match scale {
        None => return "1:1".to_string(),
        Some(v) if v <= 0.0 => return "1:1".to_string(),
        Some(v) => v,
    };

    // Try to express as a clean integer ratio.
    if s >= 1.0 {
        let n = s.round() as u32;
        if (s - n as f64).abs() < 0.01 * s {
            return if n == 1 {
                "1:1".to_string()
            } else {
                format!("{}:1", n)
            };
        }
    } else {
        let inv = (1.0 / s).round() as u32;
        if (s - 1.0 / inv as f64).abs() < 0.01 * s {
            return format!("1:{}", inv);
        }
    }

    // Fall back to a decimal string.
    format!("{:.4}", s)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn active_scale_label(
    is_model: bool,
    annotation_scale: f32,
    viewport_scale: Option<f64>,
    scales: &[(String, f32, f64)],
) -> Option<String> {
    scales
        .iter()
        .find(|(_, anno_scale, vp_scale)| {
            if is_model {
                (annotation_scale - *anno_scale).abs() < 0.001 * annotation_scale.max(0.001)
            } else {
                viewport_scale
                    .map(|current| (current - *vp_scale).abs() < 0.001 * vp_scale.max(0.001))
                    .unwrap_or(false)
            }
        })
        .map(|(label, _, _)| label.clone())
}

#[cfg(test)]
mod b5_e3_tests {
    use super::*;

    /// Ids spelled as the first string argument of `marker(...)` in `src`.
    /// The definition of the helper itself does not start with a quote, so it
    /// is skipped; only real call sites contribute.
    fn ids_after<'a>(src: &'a str, marker: &str) -> Vec<&'a str> {
        let mut out = Vec::new();
        for chunk in src.split(marker).skip(1) {
            let Some(rest) = chunk.trim_start().strip_prefix('"') else {
                continue;
            };
            if let Some(end) = rest.find('"') {
                out.push(&rest[..end]);
            }
        }
        out
    }

    /// The list the update side chases and the pills the bar actually paints
    /// must be the same set.
    ///
    /// `kit::surface::pill_style` keys its engagement track on `state_key(id)`:
    /// a pill painted with an unlisted id never eases (it steps), and a listed
    /// id with no pill leaves a track nobody reads. Neither can be seen in a
    /// type, so the witness is the source shape plus the set comparison.
    #[test]
    fn status_pills_and_lights_agree() {
        let file = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/ui/statusbar/mod.rs"
        ))
        .expect("statusbar source must be readable");
        // Only the paint side. The assertions below spell the shapes they
        // forbid, so a scan that reached this module would always find them.
        let src = file.split("\n#[cfg(test)]").next().unwrap_or(&file);

        let mut painted = ids_after(src, "toggle_pill(");
        painted.extend(ids_after(src, "split_pill("));
        painted.extend(ids_after(src, "pill_button_style("));
        painted.sort_unstable();
        painted.dedup();

        let mut listed = LIT_PILLS.to_vec();
        listed.sort_unstable();
        listed.dedup();
        assert_eq!(listed.len(), LIT_PILLS.len(), "LIT_PILLS has a duplicate id");

        assert_eq!(
            painted, listed,
            "a pill the band paints is missing from LIT_PILLS (it will step instead of \
             easing) or LIT_PILLS names a pill the band no longer paints"
        );
        assert!(
            painted.len() >= 16,
            "the band paints {} lit pills; this witness was sized on the 16 that existed",
            painted.len()
        );

        assert!(
            !src.contains("pill_style(\"\""),
            "an empty motion id is shared by every read-only pill, so one of them \
             settling would light the rest"
        );
        assert!(
            !src.contains("rule_under_inset(pill, active"),
            "the toggle pill's rule reads the raw bool again: it must read \
             `engage(state_key(key), active)` through `rule_under_inset_at`"
        );
        assert!(
            !src.contains("rule_under_inset(btn, active"),
            "the space-mode pill's rule reads the raw bool again (rule_under_inset_at \
             with state_key(\"ST_SPACE\"))"
        );
    }

    /// Every listed id must have a light. A missing arm in `StatusLights::lit`
    /// reads as "dark" for ever, so the pill would look dead in exactly the
    /// state it is meant to show.
    #[test]
    fn every_listed_pill_has_a_light() {
        let all = StatusLights {
            ortho: true,
            lwt: true,
            polar: true,
            dyn_input: true,
            otrack: true,
            osnap: true,
            space: true,
            anno_visibility: true,
            anno_auto_add: true,
            vp_scale_sync: Some(true),
            transparency: true,
            isolate: true,
            quick_props: true,
            filter: true,
            sel_cycle: true,
            clean_screen: true,
        };
        for &id in LIT_PILLS {
            assert!(
                all.lit(id),
                "{id} is listed in LIT_PILLS but StatusLights::lit has no arm for it"
            );
        }
        assert!(!all.lit("ST_NOT_A_PILL"));
    }

    /// The sync that starts these tracks must live on the update side: the
    /// frame subscription is collected at the end of `update`, before `view`,
    /// so a track started while painting never receives a frame.
    #[test]
    fn the_lights_are_chased_from_update() {
        let sync = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/app/update/motion.rs"
        ))
        .expect("app/update/motion.rs must be readable");
        assert!(
            sync.contains("for &id in LIT_PILLS"),
            "the sync no longer walks LIT_PILLS, so listed pills lose their ease"
        );
        assert!(
            sync.contains("motion::chase(motion::state_key(id)"),
            "the sync no longer chases the pill's own state track"
        );
        let update = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/app/update/mod.rs"
        ))
        .expect("app/update/mod.rs must be readable");
        assert!(
            update.contains("self.sync_motion();"),
            "update() stopped calling sync_motion(); every state track would freeze"
        );
    }
}
