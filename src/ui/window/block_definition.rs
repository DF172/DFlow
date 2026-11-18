//! Block Definition dialog window — create or redefine a block definition.
//! Prompted when the user runs the `BLOCK` or `BMAKE` command.

use crate::ui::style::tokens;
use std::fmt;

use codec::Handle;
use iced::widget::{
    checkbox, column, combo_box, container, mouse_area, row, stack, text, text_editor,
    text_input, Space,
};
use iced::{Background, Border, Element, Fill, Length, Theme};

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
pub enum BlockObjectMode {
    Retain,
    Convert,
    Delete,
}

impl Default for BlockObjectMode {
    fn default() -> Self {
        BlockObjectMode::Convert
    }
}

/// Dropdown entry for the Block Unit picker.
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

/// The Block Definition dialog's working copy.
pub struct BlockDefinitionState {
    pub name: String,
    pub existing_names: Vec<String>,
    pub name_combo: combo_box::State<String>,
    pub base_point_specify_onscreen: bool,
    pub base_point_x: String,
    pub base_point_y: String,
    pub base_point_z: String,
    pub objects_specify_onscreen: bool,
    pub selected_handles: Vec<Handle>,
    pub object_mode: BlockObjectMode,
    pub annotative: bool,
    pub match_orientation: bool,
    pub scale_uniformly: bool,
    pub allow_exploding: bool,
    pub unit: i16,
    pub description: String,
    pub description_content: text_editor::Content,
    pub hyperlink_url: String,
    pub hyperlink_desc: String,
    pub error_message: Option<String>,
    pub confirm_redefine: Option<String>,
    pub desc_placeholder: String,
}

impl Clone for BlockDefinitionState {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            existing_names: self.existing_names.clone(),
            name_combo: self.name_combo.clone(),
            base_point_specify_onscreen: self.base_point_specify_onscreen,
            base_point_x: self.base_point_x.clone(),
            base_point_y: self.base_point_y.clone(),
            base_point_z: self.base_point_z.clone(),
            objects_specify_onscreen: self.objects_specify_onscreen,
            selected_handles: self.selected_handles.clone(),
            object_mode: self.object_mode,
            annotative: self.annotative,
            match_orientation: self.match_orientation,
            scale_uniformly: self.scale_uniformly,
            allow_exploding: self.allow_exploding,
            unit: self.unit,
            description: self.description.clone(),
            description_content: text_editor::Content::with_text(&self.description),
            hyperlink_url: self.hyperlink_url.clone(),
            hyperlink_desc: self.hyperlink_desc.clone(),
            error_message: self.error_message.clone(),
            confirm_redefine: self.confirm_redefine.clone(),
            desc_placeholder: self.desc_placeholder.clone(),
        }
    }
}

impl BlockDefinitionState {
    pub fn new(existing_names: Vec<String>, pre_selected: Vec<Handle>, default_unit: i16) -> Self {
        Self {
            name: String::new(),
            name_combo: combo_box::State::new(existing_names.clone()),
            existing_names,
            base_point_specify_onscreen: false,
            base_point_x: "0.0000".to_string(),
            base_point_y: "0.0000".to_string(),
            base_point_z: "0.0000".to_string(),
            objects_specify_onscreen: false,
            selected_handles: pre_selected,
            object_mode: BlockObjectMode::Convert,
            annotative: false,
            match_orientation: false,
            scale_uniformly: false,
            allow_exploding: true,
            unit: default_unit,
            description: String::new(),
            description_content: text_editor::Content::new(),
            hyperlink_url: String::new(),
            hyperlink_desc: String::new(),
            error_message: None,
            confirm_redefine: None,
            desc_placeholder: crate::t!("Enter block description...").into_owned(),
        }
    }

    pub fn parse_base_point(&self) -> glam::DVec3 {
        let x = self.base_point_x.trim().parse::<f64>().unwrap_or(0.0);
        let y = self.base_point_y.trim().parse::<f64>().unwrap_or(0.0);
        let z = self.base_point_z.trim().parse::<f64>().unwrap_or(0.0);
        glam::DVec3::new(x, y, z)
    }

    /// Prefill the base point fields from an existing definition, so
    /// redefining a block starts from the base it already has (0.0000 for a
    /// new block). Formatted the same way the pick point button writes them.
    pub fn set_base_point(&mut self, base: glam::DVec3) {
        self.base_point_x = format!("{:.4}", base.x);
        self.base_point_y = format!("{:.4}", base.y);
        self.base_point_z = format!("{:.4}", base.z);
    }

    pub fn is_existing_name(&self) -> bool {
        let trimmed = self.name.trim();
        !trimmed.is_empty()
            && self
                .existing_names
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(trimmed))
    }

    pub fn invalid_name_char(&self) -> Option<char> {
        self.name.chars().find(|&c| {
            matches!(
                c,
                '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '=' | '`'
            )
        })
    }
}

pub fn view_window<'a>(
    state: &'a BlockDefinitionState,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    // ── Name Row ─────────────────────────────────────────────────────────────
    let name_combo = combo_box(
        &state.name_combo,
        t!("Block name").as_ref(),
        if state.name.is_empty() {
            None
        } else {
            Some(&state.name)
        },
        |chosen: String| Message::BlockDefNameSelect(chosen),
    )
    .on_input(Message::BlockDefName)
    .size(tokens::TEXT_LABEL)
    // The name is the one field this dialog asks you to type into, with its
    // label above it: `kit::PAD_FIELD` `[3, 6]` is the dense-pair inset for
    // fields packed in a row or a cell, and PAD_FIELD's own doc sends a primary
    // field to `Size::Control` `[4, 10]` instead. 5px, and it is the same
    // landing the #42 calibration screen gave the "New layer state" box, so the
    // two dialogs' primary fields now wear one inset.
    .padding(kit::Size::Control.padding())
    .width(Fill);

    let name_hint: Option<Element<'a, Message>> = if let Some(invalid_char) =
        state.invalid_name_char()
    {
        Some(
            text(crate::tf!("Invalid character '{invalid_char}' in block name (\\ / : * ? \" < > | = ` are not allowed)"))
                .size(tokens::TEXT_MICRO)
                .style(|theme: &Theme| kit::tone_style(theme, kit::Tone::Danger))
                .into(),
        )
    } else if state.is_existing_name() {
        Some(
            text(crate::tf!(
                "Block \"{}\" already exists — will redefine upon OK",
                state.name.trim()
            ))
            .size(tokens::TEXT_MICRO)
            .style(|theme: &Theme| kit::tone_style(theme, kit::Tone::Accent))
            .into(),
        )
    } else {
        None
    };

    let mut name_column = column![text(t!("Name:")).size(tokens::TEXT_LABEL).style(kit::muted_style), name_combo,]
        .spacing(tokens::GAP_TIGHT)
        .width(Fill);

    if let Some(hint) = name_hint {
        name_column = name_column.push(hint);
    }

    let name_section = name_column;

    // ── Group: Base Point ─────────────────────────────────────────────────────
    let base_onscreen = kit::labeled_checkbox(
        t!("Specify On-screen"),
        state.base_point_specify_onscreen,
        Message::BlockDefBaseOnScreen,
    );

    let pick_pt_icon = crate::ui::icons::semantic(ICON_PICK_POINT, 15.0);

    // `kit::btn` owns the plate: `[3, 8]` became `Size::Compact` `[4, 8]` (1px),
    // and the disabled half is now `Press::Off` instead of a rebuild of the same
    // button two lines later — which is also what dropped the hand-written
    // `.style(kit::button_style(false))` at every one of this screen's buttons.
    let pick_pt_btn = kit::btn(
        row![
            pick_pt_icon,
            text(t!("Pick point")).size(tokens::TEXT_LABEL)
        ]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center),
        kit::Role::Standard,
        kit::Size::Compact,
        if state.base_point_specify_onscreen {
            kit::Press::Off
        } else {
            kit::Press::On(Message::BlockDefPickPoint)
        },
        "",
    );

    let coord_row = |axis: &'static str, val: &'a str, on_input: fn(String) -> Message| {
        let is_enabled = !state.base_point_specify_onscreen;
        let input: Element<'a, Message> = if is_enabled {
            text_input("", val)
                .on_input(on_input)
                .on_submit(Message::BlockDefApply)
                .style(kit::field_style)
                .size(tokens::TEXT_LABEL)
                // A coordinate edit is a dense field packed in a row beside its
                // axis letter: `kit::PAD_FIELD` `[3, 6]` (2px), the inset
                // `labeled_field_compact` already writes for the same job.
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
            base_onscreen,
            Space::new().height(tokens::GAP_HAIRLINE),
            pick_pt_btn,
            Space::new().height(tokens::GAP_HAIRLINE),
            coord_row("X:", &state.base_point_x, Message::BlockDefBaseX),
            coord_row("Y:", &state.base_point_y, Message::BlockDefBaseY),
            coord_row("Z:", &state.base_point_z, Message::BlockDefBaseZ),
        ]
        .spacing(tokens::GAP_TIGHT),
    )
    .height(Length::Fixed(152.0));

    // ── Group: Objects ────────────────────────────────────────────────────────
    let objects_onscreen = kit::labeled_checkbox(
        t!("Specify On-screen"),
        state.objects_specify_onscreen,
        Message::BlockDefObjectsOnScreen,
    );

    let sel_obj_icon = crate::ui::icons::semantic(ICON_SELECT_OBJECTS, 15.0);

    let sel_obj_btn = kit::btn(
        row![
            sel_obj_icon,
            text(t!("Select objects")).size(tokens::TEXT_LABEL)
        ]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center),
        kit::Role::Standard,
        kit::Size::Compact,
        if state.objects_specify_onscreen {
            kit::Press::Off
        } else {
            kit::Press::On(Message::BlockDefSelectObjects)
        },
        "",
    );

    let qsel_icon = crate::ui::icons::semantic(ICON_QUICK_SELECT, 15.0);
    // The quick-select glyph gets a square plate from `kit::icon_btn`, which
    // takes the inset off the call site entirely (`Size::Icon` is `Padding::ZERO`
    // and centres the glyph in both axes). 21.0 is the side the old `[3, 5]`
    // padding around the same 15px glyph already produced vertically; it now
    // loses 4px of width and gains 0 of height, so the pair it sits next to —
    // `sel_obj_btn` at `Size::Compact` — is the taller of the two by 4px instead
    // of the plate being a 25 x 21 rectangle against a 25-tall neighbour.
    let qsel_btn = kit::icon_btn(
        qsel_icon,
        kit::Role::Standard,
        21.0,
        if state.objects_specify_onscreen {
            kit::Press::Off
        } else {
            kit::Press::On(Message::BlockDefQuickSelect)
        },
        "",
    );

    let btns_row = row![sel_obj_btn, qsel_btn]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center);

    let radios = column![
        kit::form_radio(
            t!("Retain"),
            BlockObjectMode::Retain,
            Some(state.object_mode),
            Message::BlockDefObjectMode,
        ),
        kit::form_radio(
            t!("Convert to block"),
            BlockObjectMode::Convert,
            Some(state.object_mode),
            Message::BlockDefObjectMode,
        ),
        kit::form_radio(
            t!("Delete"),
            BlockObjectMode::Delete,
            Some(state.object_mode),
            Message::BlockDefObjectMode,
        ),
    ]
    .spacing(tokens::GAP_HAIRLINE);

    let count = state.selected_handles.len();
    let status_row: Element<'a, Message> = if count == 0 {
        row![
            crate::ui::icons::semantic(ICON_WARN, 13.0),
            text(t!("No objects selected"))
                .size(tokens::TEXT_MICRO)
                .style(kit::muted_style),
        ]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center)
        .into()
    } else {
        // No `.spacing(` on this one: the row has a single child, so the call
        // could only ever have moved nothing, and a rebuilt screen keeps no
        // spacing that is not doing a job.
        row![text(crate::tf!("{count} objects selected"))
            .size(tokens::TEXT_MICRO)
            .style(|theme: &Theme| kit::tone_style(theme, kit::Tone::Accent)),]
        .align_y(iced::Center)
        .into()
    };

    let objects_group = kit::group_box(
        t!("Objects").into_owned(),
        column![
            objects_onscreen,
            Space::new().height(tokens::GAP_HAIRLINE),
            btns_row,
            Space::new().height(tokens::GAP_HAIRLINE),
            radios,
            Space::new().height(tokens::GAP_HAIRLINE),
            status_row,
        ]
        .spacing(tokens::GAP_HAIRLINE),
    )
    .height(Length::Fixed(152.0));

    // ── Group: Behavior ───────────────────────────────────────────────────────
    let annotative_cb = kit::labeled_checkbox(
        t!("Annotative"),
        state.annotative,
        Message::BlockDefAnnotative,
    );

    let match_orient_cb: Element<'a, Message> = if state.annotative {
        row![
            // The indent that hangs a child under its parent's caption: the tick
            // is `MARK_SQUARE` and `kit::labeled_checkbox` puts `GAP_CONTROL`
            // between tick and caption, so the caption starts at their sum. This
            // row had 16, which aligned to neither the old 19 nor the new 20 —
            // the two branches of this `if` were both 4px off the column they
            // mean to hang from.
            Space::new().width(tokens::MARK_SQUARE + tokens::GAP_CONTROL),
            checkbox(state.match_orientation)
                .on_toggle(Message::BlockDefMatchOrientation)
                .size(tokens::MARK_SQUARE),
            column![
                text(t!("Match block orientation")).size(tokens::TEXT_MICRO),
                text(t!("to layout")).size(tokens::TEXT_MICRO),
            ]
            .spacing(tokens::GAP_HAIRLINE),
        ]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center)
        .into()
    } else {
        row![
            Space::new().width(tokens::MARK_SQUARE + tokens::GAP_CONTROL),
            checkbox(false).size(tokens::MARK_SQUARE),
            column![
                text(t!("Match block orientation"))
                    .size(tokens::TEXT_MICRO)
                    .style(kit::muted_style),
                text(t!("to layout")).size(tokens::TEXT_MICRO).style(kit::muted_style),
            ]
            .spacing(tokens::GAP_HAIRLINE),
        ]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center)
        .into()
    };

    let scale_uniform_cb = kit::labeled_checkbox(
        t!("Scale uniformly"),
        state.scale_uniformly,
        Message::BlockDefScaleUniformly,
    );

    let allow_explode_cb = kit::labeled_checkbox(
        t!("Allow exploding"),
        state.allow_exploding,
        Message::BlockDefAllowExploding,
    );

    let behavior_group = kit::group_box(
        t!("Behavior").into_owned(),
        column![
            annotative_cb,
            Space::new().height(tokens::GAP_HAIRLINE),
            match_orient_cb,
            Space::new().height(tokens::GAP_TIGHT),
            scale_uniform_cb,
            Space::new().height(tokens::GAP_TIGHT),
            allow_explode_cb,
        ]
        .spacing(tokens::GAP_TIGHT),
    )
    .height(Length::Fixed(152.0));

    // ── Group: Settings (Column 1 Bottom) ─────────────────────────────────────
    let unit_options: Vec<UnitChoice> = units::all()
        .map(|(code, label)| UnitChoice { code, label })
        .collect();

    let selected_unit = unit_options
        .iter()
        .find(|choice| choice.code == state.unit)
        .cloned();

    let unit_picker = crate::ui::kit::themed_pick_list(selected_unit, unit_options, |choice| choice.to_string())
        .on_select(|choice| Message::BlockDefUnit(choice.code))
        .text_size(tokens::TEXT_LABEL)
        // Secondary field inside a 152px group, not the dialog's primary entry:
        // `PAD_FIELD` (2px). `Size::Control`'s 10px lateral inset would take a
        // fifth of this group's width out of the unit label's budget.
        .padding(kit::PAD_FIELD)
        .width(Fill);

    let hyperlink_btn = kit::btn(
        text(if state.hyperlink_url.is_empty() {
            t!("Hyperlink...")
        } else {
            t!("Hyperlink*")
        })
        .size(tokens::TEXT_LABEL),
        kit::Role::Standard,
        kit::Size::Compact,
        kit::Press::On(Message::BlockDefHyperlink),
        "",
    );

    let settings_group = kit::group_box(
        t!("Settings").into_owned(),
        column![
            text(t!("Block unit:")).size(tokens::TEXT_MICRO).style(kit::muted_style),
            unit_picker,
            Space::new().height(tokens::GAP_CONTROL),
            row![
                Space::new().width(Fill),
                hyperlink_btn,
                Space::new().width(Fill)
            ],
        ]
        .spacing(tokens::GAP_TIGHT),
    )
    .height(Length::Fixed(102.0));

    // ── Group: Description (Column 2 Bottom) ──────────────────────────────────
    let desc_editor = text_editor(&state.description_content)
        .on_action(Message::BlockDefDescriptionAction)
        .placeholder(&state.desc_placeholder)
        .size(tokens::TEXT_LABEL)
        // A multi-line edit is a recess the text sits *in* (it paints its own
        // ring below), so it takes the well's `[6, 8]`, not the plate's `[4, 6]`
        // it had typed. 4px, paid for the role — finding F2 of #42: the nearest
        // rung was already `[4, 6]` and cost nothing, and the nearest rung is not
        // what decides an inset here.
        .padding(kit::PAD_WELL)
        .height(Fill)
        .style(|theme: &Theme, status| {
            let palette = theme.palette();
            let border_color = match status {
                text_editor::Status::Focused { .. } => palette.primary.base.color,
                _ => tokens::edge(theme),
            };
            text_editor::Style {
                background: Background::Color(palette.background.base.color),
                border: Border {
                    color: border_color,
                    width: tokens::EDGE_HAIRLINE,
                    radius: tokens::RADIUS_FIELD.into(),
                },
                placeholder: palette.background.base.text.scale_alpha(0.48),
                value: palette.background.base.text,
                selection: palette.primary.base.color.scale_alpha(0.5),
            }
        });

    let description_group = kit::group_box(
        t!("Description").into_owned(),
        desc_editor,
    )
    .height(Length::Fixed(102.0));

    // ── Layout Assembly: Left Column & Right Column ───────────────────────────
    let left_column = column![base_point_group, settings_group,]
        .spacing(tokens::GAP_CONTROL)
        .width(Length::FillPortion(1));

    let top_right_row = row![objects_group, behavior_group,].spacing(tokens::GAP_CONTROL).width(Fill);

    let right_column = column![top_right_row, description_group,]
        .spacing(tokens::GAP_CONTROL)
        .width(Length::FillPortion(2));

    let main_body = row![left_column, right_column].spacing(tokens::GAP_CONTROL).width(Fill);

    // ── Error Banner ──────────────────────────────────────────────────────────
    // The action row now lives in the shell's full-bleed footer
    // ([`crate::ui::shell::ModalFooter`]), so this body ends at its content.
    let mut main_column = column![name_section, main_body,]
        .spacing(tokens::GAP_CONTROL)
        // The dialog's own body: `PAD_CARD` `[10, 12]`, the tier the kit names
        // for "a resting card: a dialog body, a dock panel". 4px, and it is what
        // makes this column read as the same body inset as the dialog that hosts
        // it rather than a fourth hand-tuned pair.
        .padding(kit::PAD_CARD)
        .width(sizing.width);

    if let Some(err) = &state.error_message {
        let err_banner = container(
            row![
                crate::ui::icons::semantic(ICON_WARN, 16.0),
                text(err.clone()).size(tokens::TEXT_LABEL),
                Space::new().width(Fill),
                kit::icon_btn(
                    text("×").size(tokens::TEXT_BODY),
                    kit::Role::Ghost,
                    tokens::MARK_SQUARE + tokens::GAP_CONTROL,
                    kit::Press::On(Message::BlockDefDismissError),
                    "",
                ),
            ]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center),
        )
        // A verdict band across the body: `PAD_BAND` keeps the 10px lateral it
        // already had and takes 2px out of the vertical (a band is wider than it
        // is tall), and it is one notch tighter than the `PAD_CARD` body it sits
        // in, which is the ordering the inset table is built to express.
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
            .spacing(tokens::GAP_ROW)
            .width(sizing.width)
            .padding(tokens::GAP_SECTION);
    }

    let main_content = main_column.into();

    // ── Confirm Redefinition Guard ────────────────────────────────────────────
    if let Some(name) = &state.confirm_redefine {
        let question = crate::tf!("Block \"{name}\" already exists.\nDo you want to redefine it?");
        let shield = mouse_area(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                // The same lid every modal wears: `SCRIM_INK` (0.55, which is the
                // number this closure had typed) over the strongest plate, plus
                // the `modal:fade` channel so the page under a dialog dissolves
                // instead of snapping.
                .style(|theme: &Theme| kit::scrim_style(theme, "")),
        )
        .on_press(Message::BlockDefConfirmRedefine(false));

        let panel = container(
            column![
                text(question)
                    .size(tokens::TEXT_BODY)
                    .align_x(iced::alignment::Horizontal::Center),
                Space::new().height(tokens::GAP_GUTTER),
                row![
                    kit::dialog_button(t!("Yes"), Message::BlockDefConfirmRedefine(true), false),
                    // Default / safe focus on "No"
                    kit::dialog_button(t!("No"), Message::BlockDefConfirmRedefine(false), true),
                ]
                .spacing(tokens::GAP_SECTION)
                .align_y(iced::Center),
            ]
            .spacing(tokens::GAP_NONE)
            .align_x(iced::alignment::Horizontal::Center),
        )
        // The confirmation panel is the case `Size::Plate` was named for — the
        // doc on `tokens::PAD_PLATE_V` counts six warning/confirmation windows
        // that wrote `[24, 28]` and points at the dialog rows still waiting to be
        // folded here. 8px. It lands on the same plate as `dialog_body_style`,
        // which is the sheet this panel was already wearing.
        .padding(kit::Size::Plate.padding())
        .style(|theme: &Theme| kit::dialog_body_style(theme));

        return stack![
            main_content,
            shield,
            container(panel)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Fill)
                .center_y(Fill),
        ]
        .into();
    }

    main_content
}
