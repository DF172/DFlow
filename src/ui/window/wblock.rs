//! Write Block (WBLOCK) dialog window — write a block, selected entities, or entire drawing to an external DWG/DXF file.

use crate::ui::style::tokens;
use std::fmt;

use codec::Handle;
use iced::widget::{column, combo_box, container, row, text, text_input, Space};
use iced::{Element, Fill, Length, Theme};

use crate::app::Message;
use crate::modules::draw::units;
use crate::t;
use crate::ui::kit;

static ICON_PICK_POINT: &[u8] = include_bytes!("../../../assets/icons/blocks/pick_point.svg");
static ICON_SELECT_OBJECTS: &[u8] =
    include_bytes!("../../../assets/icons/blocks/select_objects.svg");
static ICON_QUICK_SELECT: &[u8] = include_bytes!("../../../assets/icons/blocks/quick_select.svg");
static ICON_WARN: &[u8] = include_bytes!("../../../assets/icons/ui/warning_triangle.svg");

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WblockSourceMode {
    Block,
    EntireDrawing,
    Objects,
}

impl Default for WblockSourceMode {
    fn default() -> Self {
        WblockSourceMode::Objects
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WblockObjectMode {
    Retain,
    Convert,
    Delete,
}

impl Default for WblockObjectMode {
    fn default() -> Self {
        WblockObjectMode::Retain
    }
}

/// Dropdown entry for the Unit picker.
#[derive(Clone, PartialEq, Eq)]
pub struct UnitChoice {
    pub code: i16,
    pub label: &'static str,
}

impl fmt::Display for UnitChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = crate::i18n::translate(self.label);
        f.write_str(name.as_ref())
    }
}

/// The Write Block dialog's state.
pub struct WblockState {
    pub source_mode: WblockSourceMode,
    pub block_name: String,
    pub existing_blocks: Vec<String>,
    pub block_combo: combo_box::State<String>,

    pub base_point_x: String,
    pub base_point_y: String,
    pub base_point_z: String,

    pub selected_handles: Vec<Handle>,
    pub object_mode: WblockObjectMode,

    pub file_path: String,
    pub unit: i16,

    pub error_message: Option<String>,
}

impl Clone for WblockState {
    fn clone(&self) -> Self {
        Self {
            source_mode: self.source_mode,
            block_name: self.block_name.clone(),
            existing_blocks: self.existing_blocks.clone(),
            block_combo: self.block_combo.clone(),
            base_point_x: self.base_point_x.clone(),
            base_point_y: self.base_point_y.clone(),
            base_point_z: self.base_point_z.clone(),
            selected_handles: self.selected_handles.clone(),
            object_mode: self.object_mode,
            file_path: self.file_path.clone(),
            unit: self.unit,
            error_message: self.error_message.clone(),
        }
    }
}

impl WblockState {
    pub fn new(
        existing_blocks: Vec<String>,
        pre_selected: Vec<Handle>,
        default_unit: i16,
        default_path: String,
    ) -> Self {
        let default_block = existing_blocks.first().cloned().unwrap_or_default();
        Self {
            source_mode: WblockSourceMode::Objects,
            block_name: default_block,
            block_combo: combo_box::State::new(existing_blocks.clone()),
            existing_blocks,
            base_point_x: "0.00".to_string(),
            base_point_y: "0.00".to_string(),
            base_point_z: "0.00".to_string(),
            selected_handles: pre_selected,
            object_mode: WblockObjectMode::Retain,
            file_path: default_path,
            unit: default_unit,
            error_message: None,
        }
    }

    pub fn parse_base_point(&self) -> glam::DVec3 {
        let x = self.base_point_x.trim().parse::<f64>().unwrap_or(0.0);
        let y = self.base_point_y.trim().parse::<f64>().unwrap_or(0.0);
        let z = self.base_point_z.trim().parse::<f64>().unwrap_or(0.0);
        glam::DVec3::new(x, y, z)
    }
}

pub fn view_window<'a>(
    state: &'a WblockState,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let is_objects = state.source_mode == WblockSourceMode::Objects;
    let is_block = state.source_mode == WblockSourceMode::Block;

    // ── Source: Block Radio & Dropdown ─────────────────────────────────────────
    let block_radio = kit::form_radio(
        t!("Block:"),
        WblockSourceMode::Block,
        Some(state.source_mode),
        Message::WblockSourceMode,
    );

    let block_dropdown: Element<'a, Message> = if is_block {
        combo_box(
            &state.block_combo,
            "",
            if state.block_name.is_empty() {
                None
            } else {
                Some(&state.block_name)
            },
            |chosen: String| Message::WblockBlockSelect(chosen),
        )
        .on_input(Message::WblockBlockName)
        .size(tokens::TEXT_LABEL)
        .padding(kit::PAD_FIELD)
        .width(Fill)
        .into()
    } else {
        crate::ui::read_only::field(&state.block_name, 11.0, Fill)
    };

    let block_row = row![block_radio, block_dropdown]
        .spacing(tokens::GAP_ROW)
        .width(Fill)
        .align_y(iced::Center);

    let entire_drawing_radio = kit::form_radio(
        t!("Entire drawing"),
        WblockSourceMode::EntireDrawing,
        Some(state.source_mode),
        Message::WblockSourceMode,
    );

    let objects_radio = kit::form_radio(
        t!("Objects"),
        WblockSourceMode::Objects,
        Some(state.source_mode),
        Message::WblockSourceMode,
    );

    // ── Source: Base Point Sub-group ──────────────────────────────────────────
    let pick_pt_icon = crate::ui::icons::semantic(ICON_PICK_POINT, 14.0);
    let pick_pt_btn = kit::btn(
        row![pick_pt_icon, text(t!("Pick point")).size(tokens::TEXT_LABEL)]
            .spacing(tokens::GAP_CONTROL)
            .align_y(iced::Center),
        kit::Role::Standard,
        kit::Size::Control,
        if is_objects {
            kit::Press::On(Message::WblockPickPoint)
        } else {
            kit::Press::Off
        },
        "",
    );

    let coord_row = |axis: &'static str, val: &'a str, on_input: fn(String) -> Message| {
        let input: Element<'a, Message> = if is_objects {
            text_input("", val)
                .on_input(on_input)
                .on_submit(Message::WblockApply)
                .style(kit::field_style)
                .size(tokens::TEXT_LABEL)
                .padding(kit::PAD_FIELD)
                .width(Fill)
                .into()
        } else {
            crate::ui::read_only::field(val, 11.0, Fill)
        };
        row![
            text(axis)
                .size(tokens::TEXT_LABEL)
                .style(kit::muted_style)
                .width(Length::Fixed(16.0)),
            input,
        ]
        .spacing(tokens::GAP_TIGHT)
        .align_y(iced::Center)
    };

    let base_point_group = kit::group_box(
        t!("Base point").into_owned(),
        column![
            pick_pt_btn,
            Space::new().height(tokens::GAP_HAIRLINE),
            coord_row("X:", &state.base_point_x, Message::WblockBaseX),
            coord_row("Y:", &state.base_point_y, Message::WblockBaseY),
            coord_row("Z:", &state.base_point_z, Message::WblockBaseZ),
        ]
        .spacing(tokens::GAP_HAIRLINE),
    )
    .height(Length::Fixed(175.0));

    // ── Source: Objects Sub-group ─────────────────────────────────────────────
    let sel_obj_icon = crate::ui::icons::semantic(ICON_SELECT_OBJECTS, 14.0);
    let qsel_icon = crate::ui::icons::semantic(ICON_QUICK_SELECT, 14.0);
    let sel_obj_btn = kit::btn(
        row![sel_obj_icon, text(t!("Select objects")).size(tokens::TEXT_LABEL)]
            .spacing(tokens::GAP_CONTROL)
            .align_y(iced::Center),
        kit::Role::Standard,
        kit::Size::Compact,
        if is_objects {
            kit::Press::On(Message::WblockSelectObjects)
        } else {
            kit::Press::Off
        },
        "",
    );
    let qsel_btn = kit::btn(
        qsel_icon,
        kit::Role::Standard,
        kit::Size::Compact,
        if is_objects {
            kit::Press::On(Message::WblockQuickSelect)
        } else {
            kit::Press::Off
        },
        "",
    );

    let obj_btns_row = row![sel_obj_btn, qsel_btn]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center);

    let object_radios = column![
        kit::form_radio(
            t!("Retain"),
            WblockObjectMode::Retain,
            if is_objects {
                Some(state.object_mode)
            } else {
                None
            },
            Message::WblockObjectMode,
        ),
        kit::form_radio(
            t!("Convert to block"),
            WblockObjectMode::Convert,
            if is_objects {
                Some(state.object_mode)
            } else {
                None
            },
            Message::WblockObjectMode,
        ),
        kit::form_radio(
            t!("Delete from drawing"),
            WblockObjectMode::Delete,
            if is_objects {
                Some(state.object_mode)
            } else {
                None
            },
            Message::WblockObjectMode,
        ),
    ]
    .spacing(tokens::GAP_HAIRLINE);

    let count = state.selected_handles.len();
    let status_row: Element<'a, Message> = if !is_objects {
        Space::new().height(tokens::GAP_SECTION).into()
    } else if count == 0 {
        row![
            crate::ui::icons::semantic(ICON_WARN, 13.0),
            text(t!("No objects selected"))
                .size(tokens::TEXT_MICRO)
                .style(|theme: &Theme| kit::tone_style(theme, kit::Tone::Warning)),
        ]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center)
        .into()
    } else {
        row![text(crate::tf!("{count} objects selected"))
            .size(tokens::TEXT_MICRO)
            .style(|theme: &Theme| kit::tone_style(theme, kit::Tone::Accent))]
        .align_y(iced::Center)
        .into()
    };

    let objects_subgroup = kit::group_box(
        t!("Objects").into_owned(),
        column![
            obj_btns_row,
            Space::new().height(tokens::GAP_HAIRLINE),
            object_radios,
            Space::new().height(tokens::GAP_HAIRLINE),
            status_row,
        ]
        .spacing(tokens::GAP_HAIRLINE),
    )
    .height(Length::Fixed(175.0));

    let sub_panels = row![base_point_group, objects_subgroup]
        .spacing(tokens::GAP_ROW)
        .width(Fill);

    let source_group = kit::group_box(
        t!("Source").into_owned(),
        column![
            block_row,
            entire_drawing_radio,
            objects_radio,
            Space::new().height(tokens::GAP_TIGHT),
            sub_panels,
        ]
        .spacing(tokens::GAP_TIGHT),
    )
    .height(Length::Shrink);

    // ── Destination Group ─────────────────────────────────────────────────────
    let file_input = text_input("", &state.file_path)
        .on_input(Message::WblockFilePath)
        .on_submit(Message::WblockApply)
        .style(kit::field_style)
        .size(tokens::TEXT_LABEL)
        .padding(kit::PAD_FIELD)
        .width(Fill);

    let browse_btn = kit::btn(
        text("...").size(tokens::TEXT_LABEL),
        kit::Role::Standard,
        kit::Size::Compact,
        kit::Press::On(Message::WblockBrowsePath),
        "",
    );

    let path_row = row![file_input, browse_btn]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center);

    let unit_options: Vec<UnitChoice> = units::all()
        .map(|(code, label)| UnitChoice { code, label })
        .collect();

    let selected_unit = unit_options
        .iter()
        .find(|choice| choice.code == state.unit)
        .cloned();

    let unit_picker = crate::ui::kit::themed_pick_list(selected_unit, unit_options, |choice| choice.to_string())
        .on_select(|choice| Message::WblockUnit(choice.code))
        .text_size(tokens::TEXT_LABEL)
        .padding(kit::PAD_FIELD)
        .width(Fill);

    let units_row = row![
        text(t!("Insert units:"))
            .size(tokens::TEXT_LABEL)
            .style(kit::muted_style),
        Space::new().width(tokens::GAP_ROW),
        unit_picker,
        Space::new().width(Length::Fixed(33.0)),
    ]
    .spacing(tokens::GAP_TIGHT)
    .align_y(iced::Center);

    let destination_group = kit::group_box(
        t!("Destination").into_owned(),
        column![
            text(t!("File name and path:"))
                .size(tokens::TEXT_LABEL)
                .style(kit::muted_style),
            path_row,
            Space::new().height(tokens::GAP_TIGHT),
            units_row,
        ]
        .spacing(tokens::GAP_HAIRLINE),
    )
    .height(Length::Shrink);

    // ── Footer Bar ────────────────────────────────────────────────────────────
    let actions = row![
        Space::new().width(Fill),
        kit::dialog_button(t!("OK"), Message::WblockApply, true),
        kit::dialog_button(t!("Cancel"), Message::CloseModal, false),
        kit::dialog_button(t!("Help"), Message::WblockHelp, false),
    ]
    .spacing(tokens::GAP_ROW)
    .align_y(iced::Center);

    // ── Error Banner ──────────────────────────────────────────────────────────
    let mut main_column = column![source_group, destination_group, actions,]
        .spacing(tokens::GAP_ROW)
        .padding(kit::PAD_CARD)
        .width(sizing.width);

    if let Some(err) = &state.error_message {
        let err_banner = container(
            row![
                crate::ui::icons::semantic(ICON_WARN, 16.0),
                text(err.clone()).size(tokens::TEXT_LABEL),
                Space::new().width(Fill),
                kit::btn(
                    text("×").size(tokens::TEXT_BODY),
                    kit::Role::Standard,
                    kit::Size::Compact,
                    kit::Press::On(Message::WblockDismissError),
                    "",
                ),
            ]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center),
        )
        // The same in-place verdict band the Block Editor dialog draws
        // (`block_definition.rs`), and now the same ground: the band is pushed
        // onto the form's own column, so its hairline solves against the dialog
        // body rather than against a popup plate this window never detaches.
        .padding(kit::PAD_BAND)
        .style(|theme: &Theme| {
            crate::ui::kit::tone_plate_style_on(
                theme,
                crate::ui::kit::Tone::Danger,
                crate::ui::kit::dialog_body_plate(theme),
                "",
            )
        });

        main_column = column![err_banner, main_column]
            .spacing(tokens::GAP_CONTROL)
            .width(sizing.width)
            .padding(tokens::GAP_FIELD);
    }

    main_column.into()
}
