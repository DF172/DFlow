use crate::ui::style::tokens;
use crate::app::settings::IsoPlane;
use crate::app::Message;
use crate::snap::{SnapType, ALL_3D_SNAP_MODES, ALL_SNAP_MODES};
use iced::widget::{column, container, row, text, text_input, Space};
use iced::{Element, Fill, Theme};
use std::borrow::Cow;
use crate::ui::kit;

/// Active tab in the Drafting Settings dialog.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DraftingSettingsTab {
    #[default]
    SnapAndGrid,
    PolarTracking,
    ObjectSnap,
    ObjectSnap3D,
    DynamicInput,
    QuickProperties,
    SelectionCycling,
}

/// Working buffer of drafting settings edited in the dialog.
#[derive(Clone, Debug, PartialEq)]
pub struct DraftingSettingsState {
    pub active_tab: DraftingSettingsTab,
    // Snap and Grid
    pub snap_on: bool,
    pub grid_on: bool,
    pub snap_x_input: String,
    pub snap_y_input: String,
    pub snap_equal: bool,
    pub grid_x_input: String,
    pub grid_y_input: String,
    pub grid_major_input: String,
    pub grid_adaptive: bool,
    pub grid_beyond_limits: bool,
    pub isometric: bool,
    pub iso_plane: IsoPlane,
    pub snap_angle_deg: f32,
    // Polar Tracking
    pub polar_on: bool,
    pub ortho_on: bool,
    pub polar_increment_deg: f32,
    // Object Snap
    pub osnap_on: bool,
    pub otrack_on: bool,
    pub snap_modes: rustc_hash::FxHashSet<SnapType>,
    // 3D Object Snap
    pub osnap3d_on: bool,
    pub snap3d_modes: rustc_hash::FxHashSet<SnapType>,
    // Dynamic Input
    pub dyn_input_on: bool,
    // Quick Properties
    pub quick_props_on: bool,
    // Selection Cycling
    pub selection_cycling_on: bool,
}

impl DraftingSettingsState {
    /// Check whether settings were modified compared to the saved snapshot.
    /// Excludes `active_tab` so tab navigation does not mark the dialog dirty.
    pub fn is_dirty(&self, saved: &Self) -> bool {
        self.snap_on != saved.snap_on
            || self.grid_on != saved.grid_on
            || self.snap_x_input != saved.snap_x_input
            || self.snap_y_input != saved.snap_y_input
            || self.snap_equal != saved.snap_equal
            || self.grid_x_input != saved.grid_x_input
            || self.grid_y_input != saved.grid_y_input
            || self.grid_major_input != saved.grid_major_input
            || self.grid_adaptive != saved.grid_adaptive
            || self.grid_beyond_limits != saved.grid_beyond_limits
            || self.isometric != saved.isometric
            || self.iso_plane != saved.iso_plane
            || (self.snap_angle_deg - saved.snap_angle_deg).abs() > 0.001
            || self.polar_on != saved.polar_on
            || self.ortho_on != saved.ortho_on
            || (self.polar_increment_deg - saved.polar_increment_deg).abs() > 0.001
            || self.osnap_on != saved.osnap_on
            || self.otrack_on != saved.otrack_on
            || self.snap_modes != saved.snap_modes
            || self.osnap3d_on != saved.osnap3d_on
            || self.snap3d_modes != saved.snap3d_modes
            || self.dyn_input_on != saved.dyn_input_on
            || self.quick_props_on != saved.quick_props_on
            || self.selection_cycling_on != saved.selection_cycling_on
    }
}

/// Format a snap spacing value for the dialog's text inputs.
pub fn format_snap_spacing(v: f32) -> String {
    if (v - v.round()).abs() < 1e-4 {
        format!("{}", v.round() as i32)
    } else {
        format!("{v}")
    }
}

/// Parse a snap spacing input. Spacings must be positive and finite.
pub fn parse_snap_spacing(s: &str) -> Option<f32> {
    let v: f32 = s.trim().parse().ok()?;
    if v.is_finite() && v > 0.0 && v <= 1e9 {
        Some(v)
    } else {
        None
    }
}

/// Parse a major-line interval. Must be an integer in 2..=100.
pub fn parse_grid_major(s: &str) -> Option<u32> {
    let v: u32 = s.trim().parse().ok()?;
    (2..=100).contains(&v).then_some(v)
}

pub fn view_window<'a>(
    state: &'a DraftingSettingsState,
    _dirty: bool,
    close_confirm: bool,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    // ── Tab Bar ──────────────────────────────────────────────────────────
    // Real tab strip: a single non-wrapping row of tab-shaped buttons with a
    // divider underneath, matching the style-manager editor shell. The row
    // never wraps — it clips on overflow so "Selection Cycling" can't spill
    // onto a second row.
    let tab_button = |tab: DraftingSettingsTab, label: Cow<'a, str>| {
        let is_active = state.active_tab == tab;
        kit::joined_btn(
            text(label).size(tokens::TEXT_LABEL),
            if is_active {
                kit::Role::Engaged
            } else {
                kit::Role::Standard
            },
            kit::Size::Compact,
            kit::Press::On(Message::DraftingSettingsTabChanged(tab)),
            kit::Join::Top,
            "",
        )
    };

    let tab_row = row![
        tab_button(DraftingSettingsTab::SnapAndGrid, crate::t!("Snap and Grid")),
        tab_button(
            DraftingSettingsTab::PolarTracking,
            crate::t!("Polar Tracking")
        ),
        tab_button(DraftingSettingsTab::ObjectSnap, crate::t!("Object Snap")),
        tab_button(
            DraftingSettingsTab::ObjectSnap3D,
            crate::t!("3D Object Snap")
        ),
        tab_button(
            DraftingSettingsTab::DynamicInput,
            crate::t!("Dynamic Input")
        ),
        tab_button(
            DraftingSettingsTab::QuickProperties,
            crate::t!("Quick Properties")
        ),
        tab_button(
            DraftingSettingsTab::SelectionCycling,
            crate::t!("Selection Cycling")
        ),
    ]
    .spacing(tokens::GAP_HAIRLINE)
    .width(Fill)
    .clip(true);

    let tab_divider = kit::hdivider(Fill);

    let tab_bar = column![tab_row, tab_divider].spacing(tokens::GAP_NONE).width(Fill);

    // ── Tab 1: Snap and Grid ─────────────────────────────────────────────
    let snap_and_grid_view = {
        // Left Column: Snap settings
        let snap_on_toggle = kit::labeled_checkbox(
            crate::t!("Snap On (F9)"),
            state.snap_on,
            |_| Message::DraftingSettingsToggleSnap,
        );

        let snap_spacing_group = kit::group_box(
            crate::t!("Snap spacing"),
            column![
                row![
                    text(crate::t!("Snap X spacing:")).size(tokens::TEXT_LABEL).width(120),
                    text_input("10", &state.snap_x_input)
                        .on_input(Message::DraftingSettingsSnapXChanged)
                        .size(tokens::TEXT_LABEL)
                        .padding(kit::PAD_FIELD)
                        .width(100),
                ]
                .spacing(tokens::GAP_ROW)
                .align_y(iced::Center),
                row![
                    text(crate::t!("Snap Y spacing:")).size(tokens::TEXT_LABEL).width(120),
                    text_input("10", &state.snap_y_input)
                        .on_input(Message::DraftingSettingsSnapYChanged)
                        .size(tokens::TEXT_LABEL)
                        .padding(kit::PAD_FIELD)
                        .width(100),
                ]
                .spacing(tokens::GAP_ROW)
                .align_y(iced::Center),
                kit::labeled_checkbox(
                    crate::t!("Equal X and Y spacing"),
                    state.snap_equal,
                    |_| Message::DraftingSettingsToggleEqualSnap,
                ),
            ]
            .spacing(tokens::GAP_CONTROL),
        );

        let mut iso_planes = row![].spacing(tokens::GAP_TIGHT);
        for plane in IsoPlane::ALL {
            iso_planes = iso_planes.push(kit::btn(
                text(crate::t!(plane.label())).size(tokens::TEXT_LABEL),
                if state.isometric && state.iso_plane == plane {
                    kit::Role::Primary
                } else {
                    kit::Role::Standard
                },
                kit::Size::Control,
                kit::Press::On(Message::DraftingSettingsSetIsoPlane(plane)),
                "",
            ));
        }

        let snap_type_group = kit::group_box(
            crate::t!("Snap type"),
            column![
                kit::labeled_checkbox(
                    crate::t!("Enable isometric drafting"),
                    state.isometric,
                    |_| Message::DraftingSettingsToggleIsometric,
                ),
                iso_planes,
                text(crate::t!("F5 cycles Left, Top, and Right.")).size(tokens::TEXT_MICRO),
                Space::new().height(tokens::GAP_TIGHT),
                row![
                    text(crate::t!(
                        "Rotation: %{angle}°",
                        angle = state.snap_angle_deg
                    ))
                    .size(tokens::TEXT_LABEL),
                    kit::btn(
                        text(crate::t!("Reset rotation")).size(tokens::TEXT_MICRO),
                        kit::Role::Standard,
                        kit::Size::Compact,
                        kit::Press::On(Message::DraftingSettingsResetRotation),
                        "",
                    ),
                ]
                .spacing(tokens::GAP_ROW)
                .align_y(iced::Center),
            ]
            .spacing(tokens::GAP_CONTROL),
        );

        let left_col = column![snap_on_toggle, snap_spacing_group, snap_type_group]
            .spacing(tokens::GAP_FIELD)
            .width(Fill);

        // Right Column: Grid settings
        let grid_on_toggle = kit::labeled_checkbox(
            crate::t!("Grid On (F7)"),
            state.grid_on,
            |_| Message::DraftingSettingsToggleGrid,
        );

        let grid_spacing_group = kit::group_box(
            crate::t!("Grid spacing"),
            column![
                row![
                    text(crate::t!("Grid X spacing:")).size(tokens::TEXT_LABEL).width(120),
                    text_input("10", &state.grid_x_input)
                        .on_input(Message::DraftingSettingsGridXChanged)
                        .size(tokens::TEXT_LABEL)
                        .padding(kit::PAD_FIELD)
                        .width(100),
                ]
                .spacing(tokens::GAP_ROW)
                .align_y(iced::Center),
                row![
                    text(crate::t!("Grid Y spacing:")).size(tokens::TEXT_LABEL).width(120),
                    text_input("10", &state.grid_y_input)
                        .on_input(Message::DraftingSettingsGridYChanged)
                        .size(tokens::TEXT_LABEL)
                        .padding(kit::PAD_FIELD)
                        .width(100),
                ]
                .spacing(tokens::GAP_ROW)
                .align_y(iced::Center),
                row![
                    text(crate::t!("Major line every:")).size(tokens::TEXT_LABEL).width(120),
                    text_input("5", &state.grid_major_input)
                        .on_input(Message::DraftingSettingsGridMajorChanged)
                        .size(tokens::TEXT_LABEL)
                        .padding(kit::PAD_FIELD)
                        .width(100),
                ]
                .spacing(tokens::GAP_ROW)
                .align_y(iced::Center),
            ]
            .spacing(tokens::GAP_CONTROL),
        );

        let grid_behavior_group = kit::group_box(
            crate::t!("Grid behavior"),
            column![
                kit::labeled_checkbox(
                    crate::t!("Adaptive grid"),
                    state.grid_adaptive,
                    |_| Message::DraftingSettingsToggleAdaptiveGrid,
                ),
                kit::labeled_checkbox(
                    crate::t!("Display grid beyond Limits"),
                    state.grid_beyond_limits,
                    |_| Message::DraftingSettingsToggleBeyondLimits,
                ),
            ]
            .spacing(tokens::GAP_CONTROL),
        );

        let right_col = column![grid_on_toggle, grid_spacing_group, grid_behavior_group]
            .spacing(tokens::GAP_FIELD)
            .width(Fill);

        row![left_col, right_col].spacing(tokens::GAP_GUTTER).width(Fill)
    };

    // ── Tab 2: Polar Tracking ────────────────────────────────────────────
    let polar_tracking_view = {
        let polar_on_toggle = kit::labeled_checkbox(
            crate::t!("Polar Tracking On (F10)"),
            state.polar_on,
            |_| Message::DraftingSettingsTogglePolar,
        );

        let ortho_on_toggle = kit::labeled_checkbox(
            crate::t!("Ortho mode (F8)"),
            state.ortho_on,
            |_| Message::DraftingSettingsToggleOrtho,
        );

        let polar_angle_group = kit::group_box(
            crate::t!("Polar Angle Settings"),
            column![
                row![
                    text(crate::t!("Increment angle:")).size(tokens::TEXT_LABEL).width(130),
                    text(format!("{:.1}°", state.polar_increment_deg)).size(tokens::TEXT_LABEL),
                ]
                .spacing(tokens::GAP_ROW)
                .align_y(iced::Center),
                text(crate::t!(
                    "Polar Tracking guides cursor movement along specified angles. Ortho mode constrains movement to orthogonal axes."
                ))
                .size(tokens::TEXT_MICRO)
                .style(kit::muted_style),
            ]
            .spacing(tokens::GAP_CONTROL),
        );

        column![
            polar_on_toggle,
            ortho_on_toggle,
            Space::new().height(tokens::GAP_TIGHT),
            polar_angle_group,
        ]
        .spacing(tokens::GAP_FIELD)
        .width(Fill)
    };

    // ── Tab 3: Object Snap ───────────────────────────────────────────────
    let object_snap_view = {
        let osnap_on_toggle = kit::labeled_checkbox(
            crate::t!("Object Snap On (F3)"),
            state.osnap_on,
            |_| Message::DraftingSettingsToggleOsnap,
        );

        let otrack_on_toggle = kit::labeled_checkbox(
            crate::t!("Object Snap Tracking On (F11)"),
            state.otrack_on,
            |_| Message::DraftingSettingsToggleOtrack,
        );

        let action_buttons = row![
            kit::btn(
                text(crate::t!("Select All")).size(tokens::TEXT_LABEL),
                kit::Role::Standard,
                kit::Size::Control,
                kit::Press::On(Message::DraftingSettingsSnapSelectAll),
                "",
            ),
            kit::btn(
                text(crate::t!("Clear All")).size(tokens::TEXT_LABEL),
                kit::Role::Standard,
                kit::Size::Control,
                kit::Press::On(Message::DraftingSettingsSnapClearAll),
                "",
            ),
        ]
        .spacing(tokens::GAP_ROW);

        // Two columns of snap modes:
        let total = ALL_SNAP_MODES.len();
        let mid = (total + 1) / 2;
        let (first_half, second_half) = ALL_SNAP_MODES.split_at(mid);

        let mut col1 = column![].spacing(tokens::GAP_CONTROL).width(Fill);
        for &(snap_type, _, label) in first_half {
            let is_checked = state.snap_modes.contains(&snap_type);
            col1 = col1.push(
                kit::labeled_checkbox(
                    crate::t!(label),
                    is_checked,
                    move |_| Message::DraftingSettingsToggleSnapMode(snap_type),
                ),
            );
        }

        let mut col2 = column![].spacing(tokens::GAP_CONTROL).width(Fill);
        for &(snap_type, _, label) in second_half {
            let is_checked = state.snap_modes.contains(&snap_type);
            col2 = col2.push(
                kit::labeled_checkbox(
                    crate::t!(label),
                    is_checked,
                    move |_| Message::DraftingSettingsToggleSnapMode(snap_type),
                ),
            );
        }

        let snap_modes_group = kit::group_box(
            crate::t!("Object Snap modes"),
            row![col1, col2].spacing(tokens::GAP_GUTTER).width(Fill),
        );

        column![
            row![osnap_on_toggle, Space::new().width(tokens::GAP_GUTTER), otrack_on_toggle].align_y(iced::Center),
            action_buttons,
            Space::new().height(tokens::GAP_TIGHT),
            snap_modes_group,
        ]
        .spacing(tokens::GAP_FIELD)
        .width(Fill)
    };

    // ── Tab 4: 3D Object Snap ────────────────────────────────────────────
    let object_snap_3d_view = {
        let osnap3d_toggle = kit::labeled_checkbox(
            crate::t!("3D Object Snap On (F4)"),
            state.osnap3d_on,
            |_| Message::DraftingSettingsToggle3dOsnap,
        );

        // Implemented 3D modes follow the same checkbox pattern as the
        // Object Snap tab, driven by ALL_3D_SNAP_MODES.
        let mut modes_col = column![].spacing(tokens::GAP_CONTROL);
        for &(snap_type, _, label) in ALL_3D_SNAP_MODES {
            let is_checked = state.snap3d_modes.contains(&snap_type);
            modes_col = modes_col.push(
                kit::labeled_checkbox(
                    crate::t!(label),
                    is_checked,
                    move |_| Message::DraftingSettingsToggleSnapMode3d(snap_type),
                ),
            );
        }

        let modes_group = kit::group_box(crate::t!("3D Object Snap modes"), modes_col);

        column![osnap3d_toggle, Space::new().height(tokens::GAP_TIGHT), modes_group]
            .spacing(tokens::GAP_FIELD)
            .width(Fill)
    };

    // ── Tab 5: Dynamic Input ─────────────────────────────────────────────
    let dynamic_input_view = {
        let dyn_input_toggle = kit::labeled_checkbox(
            crate::t!("Enable Pointer Input (F12)"),
            state.dyn_input_on,
            |_| Message::DraftingSettingsToggleDynInput,
        );

        let pointer_group = kit::group_box(
            crate::t!("Pointer Input"),
            column![
                kit::static_checkbox(
                    crate::t!("Display coordinate input near crosshairs"),
                    true,
                ),
                kit::static_checkbox(
                    crate::t!("Enable dimension input fields"),
                    true,
                ),
            ]
            .spacing(tokens::GAP_CONTROL),
        );

        let prompt_group = kit::group_box(
            crate::t!("Dynamic Prompts"),
            column![kit::static_checkbox(
                crate::t!(
                    "Show command prompting and command input near crosshairs"
                ),
                true,
            )]
            .spacing(tokens::GAP_CONTROL),
        );

        column![
            dyn_input_toggle,
            Space::new().height(tokens::GAP_TIGHT),
            pointer_group,
            prompt_group,
        ]
        .spacing(tokens::GAP_FIELD)
        .width(Fill)
    };

    // ── Tab 6: Quick Properties ──────────────────────────────────────────
    let quick_properties_view = {
        let quick_props_toggle = kit::labeled_checkbox(
            crate::t!("Display Quick Properties palette on selection"),
            state.quick_props_on,
            |_| Message::DraftingSettingsToggleQuickProps,
        );

        let palette_group = kit::group_box(
            crate::t!("Palette Location"),
            column![
                kit::static_checkbox(
                    crate::t!("Cursor-dependent position"),
                    true,
                ),
                kit::static_checkbox(
                    crate::t!("Static quadrant location"),
                    false,
                ),
            ]
            .spacing(tokens::GAP_CONTROL),
        );

        column![quick_props_toggle, Space::new().height(tokens::GAP_TIGHT), palette_group,]
            .spacing(tokens::GAP_FIELD)
            .width(Fill)
    };

    // ── Tab 7: Selection Cycling ─────────────────────────────────────────
    let selection_cycling_view = {
        let sel_cycling_toggle = kit::labeled_checkbox(
            crate::t!("Allow selection cycling"),
            state.selection_cycling_on,
            |_| Message::DraftingSettingsToggleSelCycling,
        );

        let cycling_group = kit::group_box(
            crate::t!("Display Selection Cycling List Box"),
            column![
                kit::static_checkbox(
                    crate::t!("Show cycling badge when objects overlap"),
                    true,
                ),
                kit::static_checkbox(
                    crate::t!("Display selection candidate list box on click"),
                    true,
                ),
            ]
            .spacing(tokens::GAP_CONTROL),
        );

        column![sel_cycling_toggle, Space::new().height(tokens::GAP_TIGHT), cycling_group,]
            .spacing(tokens::GAP_FIELD)
            .width(Fill)
    };

    // ── Active Tab Content ───────────────────────────────────────────────
    let tab_content: Element<'a, Message> = match state.active_tab {
        DraftingSettingsTab::SnapAndGrid => snap_and_grid_view.into(),
        DraftingSettingsTab::PolarTracking => polar_tracking_view.into(),
        DraftingSettingsTab::ObjectSnap => object_snap_view.into(),
        DraftingSettingsTab::ObjectSnap3D => object_snap_3d_view.into(),
        DraftingSettingsTab::DynamicInput => dynamic_input_view.into(),
        DraftingSettingsTab::QuickProperties => quick_properties_view.into(),
        DraftingSettingsTab::SelectionCycling => selection_cycling_view.into(),
    };

    // ── Body Layout ──────────────────────────────────────────────────────
    // The action row lives on the shell's full-bleed footer
    // (`DraftFlowCAD::modal_footer`), which also owns the dirty check.
    let body = column![
        tab_bar,
        Space::new().height(tokens::GAP_FIELD),
        crate::ui::kit::themed_scrollable(tab_content).spacing(tokens::GAP_CONTROL).height(Fill),
    ]
    .width(sizing.width)
    .height(sizing.height);

    let main_content = container(body)
        .style(|theme: &Theme| kit::dialog_body_style(theme))
        .padding(kit::Size::Plate.padding())
        .width(sizing.width)
        .height(sizing.height);

    // ── Unsaved Changes Guard ────────────────────────────────────────────
    if !close_confirm {
        return main_content.into();
    }
    crate::ui::shell::discard_guard(
        main_content,
        Message::DraftingSettingsCloseDiscard,
        Message::DraftingSettingsCloseKeep,
    )
}
