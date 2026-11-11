#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod graphics;
pub(crate) mod spacemouse;
use crate::ui::style::tokens;
use crate::app::config::UiThemeConfig;
use crate::app::settings;
use crate::app::settings::{CursorType, RightClickMode};
use crate::app::Message;
use crate::ui::kit;
use iced::widget::{column, container, row, slider, text, text_input, Space};
use crate::ui::kit::plain_button;
use iced::{Element, Fill, Theme};
use std::fmt;


/// Width of the vertical tab rail. Wide enough for the longest translated
/// page name; German and Russian are the ones that set it.
const TAB_RAIL_WIDTH: f32 = 178.0;
/// The dialog's own size when the modal is laid out intrinsically.
const DIALOG_WIDTH: f32 = 880.0;
const DIALOG_HEIGHT: f32 = 620.0;

/// Which page of the Options dialog is showing.
///
/// Persisted: with nine pages, reopening on General every time means hunting
/// for the one you were last in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OptionsTab {
    #[default]
    General,
    Files,
    OpenAndSave,
    Display,
    Drafting,
    Modeling,
    Selection,
    UserPreferences,
    Graphics,
    About,
}

/// Application preferences the dialog reads that are plain scalars on the app.
///
/// Gathered rather than passed one by one: `view_window` already carries a
/// long positional list, and several of these are adjacent `bool`s that would
/// swap silently at the call site.
#[derive(Clone, Copy)]
pub struct AppPrefs {
    /// SAVETIME, minutes between recovery saves; 0 disables.
    pub savetime_min: i32,
    /// ISAVEBAK: keep a `.bak` when overwriting.
    pub backup_on_save: bool,
    /// TEXTFILL: fill TrueType glyphs rather than drawing them hollow.
    pub textfill: bool,
    /// CLIPROMPTLINES: prompt lines shown above the command window.
    pub cliprompt_lines: i32,
    /// COMMANDLINEFADETIME: how long overlay history lines stay visible, in ms.
    pub commandline_fade_ms: i32,
    /// ZOOMWHEEL: reverse the mouse-wheel zoom direction.
    pub zoom_wheel_reversed: bool,
    /// Scroll pans and Ctrl+scroll zooms (touchpads).
    pub touchpad_pan: bool,
    /// Interface scale in percent.
    pub ui_scale_percent: u16,
    /// ZOOMFACTOR. The slider sets the range the system variable has; the
    /// field beside it reaches `settings::ZOOM_FACTOR_MAX`.
    pub zoom_factor: i32,
    /// TEXTEDITMODE: TEXTEDIT keeps prompting for the next object.
    pub texteditmode: bool,
    /// DIMCONTINUEMODE: continued dimensions inherit the base dimension's style.
    pub dimension_continue_mode: i16,
    /// QDIM extension-origin priority: 0 endpoints, 1 intersections.
    pub qdim_snap_priority: u8,
    /// ANNOAUTOSCALE, -4..=4. The sign is on/off; the magnitude selects which
    /// objects a newly added scale reaches.
    pub annotation_auto_scale: i8,
    /// Polar tracking increment in degrees.
    pub polar_increment_deg: f32,
    /// NAVVCUBE: show the navigation cube.
    pub show_viewcube: bool,
    /// UCSICON: show the UCS icon.
    pub show_ucs_icon: bool,
    /// UCSICON ORigin: draw it at the origin rather than the corner.
    pub ucs_icon_at_origin: bool,
    /// SHORTCUTMENU: what a right-click in the drawing area does.
    pub right_click_mode: RightClickMode,
    /// SHORTCUTMENUDURATION: time-sensitive hold threshold, ms.
    pub right_click_hold_ms: i32,
    /// Open the Plot / Page Setup dialog for every new layout.
    pub page_setup_on_new_layout: bool,
}

/// The support folders the Files page lists. `None` where the platform could
/// not give one — the row then says so and its button is disabled rather than
/// opening nothing. `*_custom` says the user pointed that folder somewhere
/// other than the default, which is what shows the "Restore default" button.
#[derive(Clone, Default)]
pub struct Folders {
    pub config: Option<String>,
    pub config_custom: bool,
    pub plot_styles: Option<String>,
    pub plot_styles_custom: bool,
    pub autosave: Option<String>,
    pub autosave_custom: bool,
    /// Where `.shx` / `.ttf` fonts dropped in by the user (or downloaded for
    /// a drawing) are found.
    pub fonts: Option<String>,
    pub fonts_custom: bool,
}

/// Which support folder a Files-page action targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderKind {
    Config,
    PlotStyles,
    Autosave,
    Fonts,
}

/// Values read from the current drawing's header rather than from preferences.
///
/// These are saved in the DWG, so they follow the drawing rather than the
/// application — the page labels them as such. `available` is false when no
/// drawing is open and the controls are not drawn at all.
#[derive(Clone, Copy)]
pub struct DrawingPrefs {
    pub available: bool,
    /// ISOLINES.
    pub isolines: i16,
    /// DISPSILH.
    pub display_silhouette: bool,
    /// SURFU / SURFV.
    pub surface_u: i16,
    pub surface_v: i16,
    /// SURFTYPE: 5, 6 or 8.
    pub surface_type: i16,
    /// SOLIDHIST.
    pub record_solid_history: bool,
    /// SHOWHIST, 0..=2.
    pub show_solid_history: i16,
}

/// The Selection-card settings that live on `UserSettings` rather than on the
/// model-space theme.
///
/// Gathered into one value instead of four more positional parameters: the
/// function already takes seventeen, and `pick_add` / `pick_drag_rect` are two
/// adjacent `bool`s that would swap silently at the call site.
#[derive(Clone, Copy)]
pub struct SelectionPrefs {
    /// PICKBOX, 0..=50.
    pub pick_box: i32,
    /// PICKADD: true = a click adds to the selection.
    pub pick_add: bool,
    /// PICKDRAG: true = press-drag draws a rectangle instead of a lasso.
    pub pick_drag_rect: bool,
    /// GRIPOBJLIMIT, 0..=32767; 0 = no limit.
    pub grip_object_limit: i32,
    /// Selection cycling: a click where objects overlap opens a picker.
    pub selection_cycling: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Labelled<T> {
    value: T,
    label: String,
}

impl<T> fmt::Display for Labelled<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

/// A background swatch that opens the colour wheel when clicked.
///
/// The wheel sits alongside the hex field rather than replacing it: typing
/// `#1E1E1E` stays the fastest way to reproduce an exact colour, while the
/// wheel is for choosing one by eye.
fn bg_swatch_picker<'a>(
    target: crate::app::BgTarget,
    rgb: [u8; 3],
    open: Option<crate::app::BgTarget>,
) -> Element<'a, Message> {
    let colour = iced::Color::from_rgb8(rgb[0], rgb[1], rgb[2]);
    // The fill is the user's colour, i.e. content rather than chrome, but the
    // plate is still a *swatch*, not the drawing: it needs the theme hairline
    // or a white field on the light panel (and a black one on the dark panel)
    // disappears into it. `kit::swatch_style` is that pair — caller's colour as
    // data, `tokens::edge` as chrome.
    let swatch = container(Space::new())
        .style(move |theme: &Theme| kit::swatch_style(theme, colour, ""))
        .width(28)
        .height(22);
    let underlay = plain_button(swatch)
        .on_press(Message::BgPickerOpen(target))
        .padding(tokens::GAP_NONE)
        .style(kit::text_style);
    iced_aw::ColorPicker::new(
        open == Some(target),
        colour,
        underlay,
        Message::BgPickerCancel,
        Message::BgPickerSubmit,
    )
    .into()
}

#[allow(clippy::too_many_arguments)]
pub fn view_window<'a>(
    default_save_format: &'a str,
    file_assoc_enabled: bool,
    show_constraint_values: bool,
    ui_theme: &'a UiThemeConfig,
    theme_color_inputs: &'a [String; 6],
    language: crate::i18n::Language,
    active_tab: OptionsTab,
    cursor_size: i32,
    selection: SelectionPrefs,
    prefs: AppPrefs,
    spacemouse: Element<'a, Message>,
    graphics: Element<'a, Message>,
    snap_angle_input: &'a str,
    zoom_factor_input: &'a str,
    drawing_prefs: DrawingPrefs,
    folders: Folders,
    qnew_template: &'a str,
    double_click_block_refedit: bool,
    double_click_block_attedit: bool,
    cursor_type: CursorType,
    crosshair_color: Option<[u8; 3]>,
    crosshair_color_input: &'a str,
    snap_marker_color: Option<[u8; 3]>,
    snap_marker_color_input: &'a str,
    command_text_color: Option<[u8; 3]>,
    command_text_color_input: &'a str,
    lineweight_display_scale: i32,
    model_space: &'a crate::app::config::ModelSpaceThemeConfig,
    model_bg_input: &'a str,
    paper_bg_input: &'a str,
    desk_bg_input: &'a str,
    bg_picker: Option<crate::app::BgTarget>,
    dirty: bool,
    close_confirm: bool,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let selected_format = crate::io::SAVE_FORMAT_OPTIONS
        .iter()
        .copied()
        .find(|candidate| *candidate == default_save_format);

    let theme_options = crate::app::config::all_themes()
        .iter()
        .map(ToString::to_string)
        .chain(std::iter::once("Custom".to_string()))
        .map(|value| Labelled {
            label: match value.as_str() {
                "Light" => crate::t!("Light").into_owned(),
                "Dark" => crate::t!("Dark").into_owned(),
                "Custom" => crate::t!("Custom").into_owned(),
                _ => value.clone(),
            },
            value,
        })
        .collect::<Vec<_>>();
    let selected_theme = theme_options
        .iter()
        .find(|choice| choice.value == ui_theme.name)
        .cloned();

    let language_options = crate::i18n::Language::ALL
        .into_iter()
        .map(|value| Labelled {
            label: value.label(),
            value,
        })
        .collect::<Vec<_>>();
    let selected_language = language_options
        .iter()
        .find(|choice| choice.value == language)
        .cloned();

    let cursor_options = CursorType::ALL
        .into_iter()
        .map(|value: CursorType| Labelled {
            label: crate::t!(value.label()).into_owned(),
            value,
        })
        .collect::<Vec<_>>();
    let selected_cursor = cursor_options
        .iter()
        .find(|choice| choice.value == cursor_type)
        .cloned();

    let right_click_options = RightClickMode::ALL
        .into_iter()
        .map(|value: RightClickMode| Labelled {
            label: crate::t!(value.label()).into_owned(),
            value,
        })
        .collect::<Vec<_>>();
    let selected_right_click = right_click_options
        .iter()
        .find(|choice| choice.value == prefs.right_click_mode)
        .cloned();

    let palette = ui_theme.palette.to_iced();
    let colors = [
        (
            crate::tr!("options", "color-background"),
            palette.background,
        ),
        (crate::tr!("options", "color-text"), palette.text),
        (crate::tr!("options", "color-primary"), palette.primary),
        (crate::tr!("options", "color-success"), palette.success),
        (crate::tr!("options", "color-warning"), palette.warning),
        (crate::tr!("options", "color-danger"), palette.danger),
    ];

    let mut color_controls = column![].spacing(tokens::GAP_ROW);
    for (index, (label, color)) in colors.into_iter().enumerate() {
        let swatch = container(Space::new())
            .style(move |theme: &Theme| kit::swatch_style(theme, color, ""))
            .width(28)
            .height(22);
        color_controls = color_controls.push(
            row![
                text(label).size(tokens::TEXT_BODY).width(110),
                swatch,
                text_input("#RRGGBB", theme_color_inputs[index].as_str())
                    .on_input(move |value| Message::OptionsThemeColorChanged(index, value))
                    .width(130),
            ]
            .spacing(tokens::GAP_FIELD)
            .align_y(iced::Center),
        );
    }

    // Changes show at once but are committed by OK / Apply; Close puts them
    // back (asking first when there is something to lose).
    let ok = kit::dialog_button(crate::t!("OK"), Message::OptionsOk, true);
    let apply = kit::dialog_button_styled_opt(
        crate::t!("Apply"),
        dirty.then_some(Message::OptionsApply),
        if dirty {
            kit::secondary_style
        } else {
            kit::text_style
        },
    );
    let close = kit::dialog_button(crate::tr!("action", "close"), Message::OptionsClose, false);

    let general = column![
        kit::heading(crate::tr!("options", "language-section"), ""),
        Space::new().height(10),
        row![
            text(crate::tr!("options", "language-label"))
                .size(tokens::TEXT_BODY)
                .width(150),
            crate::ui::kit::themed_pick_list(selected_language, language_options, |choice| choice
                .label
                .clone(),)
            .on_select(|choice| Message::LanguageChanged(choice.value))
            .width(sizing.width),
        ]
        .spacing(tokens::GAP_SECTION)
        .align_y(iced::Center),
        Space::new().height(24),
        kit::heading(crate::t!("Applications"), ""),
        Space::new().height(10),
        row![
            text(crate::t!("Keyboard shortcuts")).size(tokens::TEXT_BODY).width(Fill),
            plain_button(text(crate::t!("Keyboard Shortcuts…")).size(tokens::TEXT_LABEL))
                .on_press(Message::ShortcutsPanelOpen)
                .padding(kit::Size::Control.padding())
                .style(kit::secondary_style),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
        Space::new().height(10),
        row![
            text(crate::t!("Command aliases")).size(tokens::TEXT_BODY).width(Fill),
            plain_button(text(crate::t!("Command Aliases…")).size(tokens::TEXT_LABEL))
                .on_press(Message::AliasEditorOpen)
                .padding(kit::Size::Control.padding())
                .style(kit::secondary_style),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
        Space::new().height(22),
        kit::heading(crate::t!("Plotting"), ""),
        Space::new().height(10),
        row![
            text(crate::t!("Plot device, paper, scale and plot styles"))
                .size(tokens::TEXT_BODY)
                .width(Fill),
            plain_button(text(crate::t!("Plot and Page Setup…")).size(tokens::TEXT_LABEL))
                .on_press(Message::PlotDialogOpen)
                .padding(kit::Size::Control.padding())
                .style(kit::secondary_style),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
        Space::new().height(10),
        row![
            iced::widget::checkbox(prefs.page_setup_on_new_layout)
                .on_toggle(Message::PageSetupOnNewLayoutChanged)
                .size(tokens::MARK_SQUARE),
            text(crate::t!("Show the page setup for new layouts")).size(tokens::TEXT_BODY),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
    ]
    .spacing(tokens::GAP_NONE)
    .width(sizing.width);

    // Saving preferences, gathered onto one page. The format and the file
    // association were on General, which had become a page of three unrelated
    // controls; the autosave interval and the backup toggle have existed since
    // #205 with no control at all.
    let open_and_save = column![
        kit::heading(crate::t!("Open and Save"), ""),
        Space::new().height(10),
        Space::new().height(10),
        row![
            text(crate::tr!("options", "default-save-format-label"))
                .size(tokens::TEXT_BODY)
                .width(150),
            crate::ui::kit::themed_pick_list(
                selected_format,
                // Drawings, not the template entry (the list's last).
                &crate::io::SAVE_FORMAT_OPTIONS[..crate::io::SAVE_FORMAT_OPTIONS.len() - 1],
                |value| value.to_string(),
            )
            .on_select(|format: &str| Message::DefaultSaveFormatChanged(format.to_string()))
            .width(sizing.width),
        ]
        .spacing(tokens::GAP_SECTION)
        .align_y(iced::Center),
        Space::new().height(8),
        text(crate::tr!("options", "default-save-format-help"))
            .size(tokens::TEXT_LABEL)
            .width(sizing.width),
        Space::new().height(14),
        row![
            iced::widget::checkbox(file_assoc_enabled)
                .on_toggle(Message::FileAssocChanged)
                .size(tokens::MARK_SQUARE),
            text(crate::t!("Open .dwg and .dxf files with DraftFlow CAD")).size(tokens::TEXT_BODY),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
        Space::new().height(6),
        text(crate::t!(
            "Also installs the application and file-type icons the desktop shows."
        ))
        .size(tokens::TEXT_LABEL)
        .width(sizing.width),
        Space::new().height(14),
        row![
            iced::widget::checkbox(show_constraint_values)
                .on_toggle(Message::ShowConstraintValuesChanged)
                .size(tokens::MARK_SQUARE),
            text(crate::t!(
                "Show values and parameter names on constraint markers"
            ))
            .size(tokens::TEXT_BODY),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
        Space::new().height(6),
        text(crate::t!(
            "On by default. Turn off to show just the constraint glyph in the viewport — \
             enough to see that a constraint is present — without the driven value or named \
             parameter text covering nearby geometry."
        ))
        .size(tokens::TEXT_LABEL)
        .width(sizing.width),
        Space::new().height(22),
        kit::heading(crate::t!("File Safety"), ""),
        Space::new().height(10),
        row![
            text(crate::t!("Automatic save")).size(tokens::TEXT_BODY).width(150),
            slider(
                0..=120,
                prefs.savetime_min.clamp(0, 120),
                Message::SaveTimeChanged
            )
            .step(1)
            .width(Fill),
            text(if prefs.savetime_min <= 0 {
                crate::t!("Off").into_owned()
            } else {
                crate::tf!("{} min", prefs.savetime_min).into_owned()
            })
            .size(tokens::TEXT_LABEL)
            .width(52),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
        Space::new().height(6),
        text(crate::t!(
            "Minutes between recovery saves to a .sv$ file; 0 turns it off (SAVETIME)."
        ))
        .size(tokens::TEXT_LABEL)
        .width(sizing.width),
        Space::new().height(14),
        row![
            iced::widget::checkbox(prefs.backup_on_save)
                .on_toggle(Message::BackupOnSaveChanged)
                .size(tokens::MARK_SQUARE),
            text(crate::t!(
                "Keep a .bak copy when overwriting a drawing (ISAVEBAK)"
            ))
            .size(tokens::TEXT_BODY),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
    ]
    .spacing(tokens::GAP_NONE)
    .width(sizing.width);

    let crosshair_rgb = crosshair_color.unwrap_or([255, 255, 255]);
    let crosshair_colour =
        iced::Color::from_rgb8(crosshair_rgb[0], crosshair_rgb[1], crosshair_rgb[2]);
    let crosshair_swatch = container(Space::new())
        .style(move |theme: &Theme| kit::swatch_style(theme, crosshair_colour, ""))
        .width(28)
        .height(22);

    let command_rgb = command_text_color.unwrap_or([220, 220, 220]);
    let command_colour =
        iced::Color::from_rgb8(command_rgb[0], command_rgb[1], command_rgb[2]);
    let command_swatch = container(Space::new())
        .style(move |theme: &Theme| kit::swatch_style(theme, command_colour, ""))
        .width(28)
        .height(22);

    let snap_rgb = snap_marker_color.unwrap_or([255, 230, 26]);
    let snap_colour = iced::Color::from_rgb8(snap_rgb[0], snap_rgb[1], snap_rgb[2]);
    let snap_swatch = container(Space::new())
        .style(move |theme: &Theme| kit::swatch_style(theme, snap_colour, ""))
        .width(28)
        .height(22);

    let model_bg_rgb = model_space
        .custom_bg
        .unwrap_or(crate::app::config::CLASSIC_CAD_DARK_BG);

    let paper_bg_rgb = model_space
        .custom_paper_bg
        .unwrap_or(crate::app::config::DEFAULT_PAPER_BG);

    let desk_bg_rgb = model_space
        .custom_desk_bg
        .unwrap_or(crate::app::config::DEFAULT_DESK_BG);

    let mode_options = crate::app::config::ModelSpaceMode::ALL
        .into_iter()
        .map(|value| Labelled {
            label: crate::t!(value.label()).into_owned(),
            value,
        })
        .collect::<Vec<_>>();
    let selected_mode = mode_options
        .iter()
        .find(|choice| choice.value == model_space.mode)
        .cloned();

    let color_choice_options = [
        (0u8, "0: Theme Default"),
        (1, "1: Red"),
        (2, "2: Yellow"),
        (3, "3: Green"),
        (4, "4: Cyan"),
        (5, "5: Blue"),
        (6, "6: Magenta"),
        (7, "7: White/Black"),
    ]
    .into_iter()
    .map(|(value, label)| Labelled {
        value,
        label: crate::t!(label).into_owned(),
    })
    .collect::<Vec<_>>();

    let selected_window_color = color_choice_options
        .iter()
        .find(|c| c.value == model_space.selection_window_color)
        .cloned()
        .unwrap_or_else(|| Labelled {
            value: model_space.selection_window_color,
            label: format!("ACI {}", model_space.selection_window_color),
        });

    let selected_crossing_color = color_choice_options
        .iter()
        .find(|c| c.value == model_space.selection_crossing_color)
        .cloned()
        .unwrap_or_else(|| Labelled {
            value: model_space.selection_crossing_color,
            label: format!("ACI {}", model_space.selection_crossing_color),
        });

    let selected_highlight_color = color_choice_options
        .iter()
        .find(|c| c.value == model_space.selection_highlight_color)
        .cloned()
        .unwrap_or_else(|| Labelled {
            value: model_space.selection_highlight_color,
            label: format!("ACI {}", model_space.selection_highlight_color),
        });

    let selected_grip_color = color_choice_options
        .iter()
        .find(|c| c.value == model_space.grip_color)
        .cloned()
        .unwrap_or_else(|| Labelled {
            value: model_space.grip_color,
            label: format!("ACI {}", model_space.grip_color),
        });

    let selected_grip_hot = color_choice_options
        .iter()
        .find(|c| c.value == model_space.grip_hot)
        .cloned()
        .unwrap_or_else(|| Labelled {
            value: model_space.grip_hot,
            label: format!("ACI {}", model_space.grip_hot),
        });

    let selected_grip_hover = color_choice_options
        .iter()
        .find(|c| c.value == model_space.grip_hover)
        .cloned()
        .unwrap_or_else(|| Labelled {
            value: model_space.grip_hover,
            label: format!("ACI {}", model_space.grip_hover),
        });

    let mut display = column![
        kit::heading(crate::tr!("options", "theme-section"), ""),
        Space::new().height(10),
        row![
            text(crate::tr!("options", "theme-label"))
                .size(tokens::TEXT_BODY)
                .width(150),
            crate::ui::kit::themed_pick_list(selected_theme, theme_options, |choice| choice.label.clone(),)
                .on_select(|choice| Message::OptionsThemeChanged(choice.value))
                .width(sizing.width),
        ]
        .spacing(tokens::GAP_SECTION)
        .align_y(iced::Center),
        Space::new().height(8),
        text(crate::tr!("options", "theme-help"))
            .size(tokens::TEXT_LABEL)
            .width(sizing.width),
        Space::new().height(12),
        color_controls,
        Space::new().height(16),
        // A list, not a slider: the dialog rescales under the cursor as the
        // value changes, which would drag a slider along with it.
        row![
            text(crate::t!("Interface scale")).size(tokens::TEXT_BODY).width(150),
            crate::ui::kit::themed_pick_list(
                Some(format!("{}%", prefs.ui_scale_percent)),
                [75u16, 100, 125, 150, 175, 200]
                    .map(|p| format!("{p}%"))
                    .to_vec(),
                |label: &String| label.clone(),
            )
            .on_select(|label: String| {
                Message::UiScaleChanged(label.trim_end_matches('%').parse().unwrap_or(100))
            })
            .width(sizing.width),
        ]
        .spacing(tokens::GAP_SECTION)
        .align_y(iced::Center),
        Space::new().height(24),
        row![
            kit::heading(crate::t!("Model Space Appearance"), ""),
            Space::new().width(Fill),
            plain_button(text(crate::t!("Restore Defaults")).size(tokens::TEXT_LABEL))
                .on_press(Message::RestoreModelSpaceDisplayDefaults)
                .padding(kit::Size::Control.padding())
                .style(kit::secondary_style),
        ]
        .align_y(iced::Center),
        Space::new().height(10),
        row![
            text(crate::t!("Canvas mode")).size(tokens::TEXT_BODY).width(140),
            crate::ui::kit::themed_pick_list(selected_mode, mode_options, |choice| choice.label.clone(),)
                .on_select(|choice| Message::ModelSpaceModeChanged(choice.value))
                .width(Fill),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
    ];

    if model_space.mode == crate::app::config::ModelSpaceMode::Custom {
        display = display.push(Space::new().height(10)).push(
            row![
                text(crate::t!("Model background")).size(tokens::TEXT_BODY).width(140),
                bg_swatch_picker(crate::app::BgTarget::Model, model_bg_rgb, bg_picker),
                text_input("#RRGGBB", model_bg_input)
                    .on_input(Message::ModelSpaceBgChanged)
                    .width(150),
            ]
            .spacing(tokens::GAP_FIELD)
            .align_y(iced::Center),
        );
    }

    display = display.push(Space::new().height(10)).push(
        row![
            text(crate::t!("Paper background")).size(tokens::TEXT_BODY).width(140),
            bg_swatch_picker(crate::app::BgTarget::Paper, paper_bg_rgb, bg_picker),
            text_input("#RRGGBB", paper_bg_input)
                .on_input(Message::PaperSpaceBgChanged)
                .width(150),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
    );

    display = display.push(Space::new().height(10)).push(
        row![
            text(crate::t!("Desk surround")).size(tokens::TEXT_BODY).width(140),
            bg_swatch_picker(crate::app::BgTarget::Desk, desk_bg_rgb, bg_picker),
            text_input("#RRGGBB", desk_bg_input)
                .on_input(Message::DeskSpaceBgChanged)
                .width(150),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
    );

    display = display
        .push(Space::new().height(10))
        .push(
            row![
                text(crate::t!("Grid opacity")).size(tokens::TEXT_BODY).width(140),
                slider(5..=100, model_space.grid_opacity.clamp(5, 100), Message::GridOpacityChanged)
                    .step(1)
                    .width(Fill),
                text(format!("{}%", model_space.grid_opacity.clamp(5, 100)))
                    .size(tokens::TEXT_LABEL)
                    .width(44),
            ]
            .spacing(tokens::GAP_FIELD)
            .align_y(iced::Center),
        )
        .push(Space::new().height(6))
        .push(
            text(match model_space.mode {
                crate::app::config::ModelSpaceMode::MatchTheme => {
                    crate::t!("Canvas background, grid, and default line colors automatically adapt to the active theme.")
                }
                crate::app::config::ModelSpaceMode::ClassicDark => {
                    crate::t!("Model space canvas remains locked to classic CAD dark charcoal (#212830).")
                }
                crate::app::config::ModelSpaceMode::Custom => {
                    crate::t!("Custom background colors set above are applied to Model and Paper space.")
                }
            })
            .size(tokens::TEXT_LABEL)
            .width(sizing.width),
        )
        .push(Space::new().height(24))
        .push(kit::heading(crate::t!("Crosshair"), ""))
        .push(Space::new().height(10))
        .push(
            row![
                text(crate::t!("Crosshair size")).size(tokens::TEXT_BODY).width(140),
                slider(1..=100, cursor_size.clamp(1, 100), Message::CursorSizeChanged)
                    .step(1)
                    .width(Fill),
                text(format!("{}%", cursor_size.clamp(1, 100)))
                    .size(tokens::TEXT_LABEL)
                    .width(44),
            ]
            .spacing(tokens::GAP_FIELD)
            .align_y(iced::Center),
        )
        .push(Space::new().height(10))
        .push(
            row![
                text(crate::t!("Cursor type")).size(tokens::TEXT_BODY).width(140),
                crate::ui::kit::themed_pick_list(
                    selected_cursor,
                    cursor_options,
                    |choice| choice.label.clone(),
                )
                .on_select(|choice| Message::CursorTypeChanged(choice.value))
                .width(Fill),
            ]
            .spacing(tokens::GAP_FIELD)
            .align_y(iced::Center),
        )
        .push(Space::new().height(10))
        .push(
            row![
                text(crate::t!("Crosshair color")).size(tokens::TEXT_BODY).width(140),
                crosshair_swatch,
                text_input(crate::t!("#RRGGBB or blank").as_ref(), crosshair_color_input)
                    .on_input(Message::CrosshairColorChanged)
                    .width(150),
            ]
            .spacing(tokens::GAP_FIELD)
            .align_y(iced::Center),
        )
        .push(Space::new().height(10))
        .push(
            row![
                text(crate::t!("Snap marker color")).size(tokens::TEXT_BODY).width(140),
                snap_swatch,
                text_input(crate::t!("#RRGGBB or blank").as_ref(), snap_marker_color_input)
                    .on_input(Message::SnapMarkerColorChanged)
                    .width(150),
            ]
            .spacing(tokens::GAP_FIELD)
            .align_y(iced::Center),
        )
        .push(Space::new().height(10))
        .push(
            row![
                text(crate::t!("Command line text color")).size(tokens::TEXT_BODY).width(140),
                command_swatch,
                text_input(crate::t!("#RRGGBB or blank").as_ref(), command_text_color_input)
                    .on_input(Message::CommandTextColorChanged)
                    .width(150),
            ]
            .spacing(tokens::GAP_FIELD)
            .align_y(iced::Center),
        )
        .push(Space::new().height(6))
        .push(
            text(crate::t!("Leave the color blank to keep automatic viewport contrast."))
                .size(tokens::TEXT_LABEL)
                .width(sizing.width),
        )
        .push(Space::new().height(24))
        .push(kit::heading(crate::t!("Lineweight"), ""))
        .push(Space::new().height(10))
        .push(
            row![
                text(crate::t!("Model display scale")).size(tokens::TEXT_BODY).width(140),
                slider(
                    25..=200,
                    lineweight_display_scale.clamp(25, 200),
                    Message::LineweightDisplayScaleChanged,
                )
                .step(1)
                .width(Fill),
                text(format!("{}%", lineweight_display_scale.clamp(25, 200)))
                    .size(tokens::TEXT_LABEL)
                    .width(44),
            ]
            .spacing(tokens::GAP_FIELD)
            .align_y(iced::Center),
        )
        .push(Space::new().height(6))
        .push(
            text(crate::t!("Changes the on-screen width in Model without affecting plotted output."))
                .size(tokens::TEXT_LABEL)
                .width(sizing.width),
        )
        .push(Space::new().height(14))
        .push(
            row![
                iced::widget::checkbox(prefs.textfill)
                    .on_toggle(Message::TextFillChanged)
                    .size(tokens::MARK_SQUARE),
                text(crate::t!("Fill TrueType glyphs (TEXTFILL)")).size(tokens::TEXT_BODY),
            ]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center),
        )
        .push(Space::new().height(24))
        .push(kit::heading(crate::t!("Command Line"), ""))
        .push(Space::new().height(10))
        .push(
            row![
                text(crate::t!("Prompt lines")).size(tokens::TEXT_BODY).width(140),
                slider(
                    0..=50,
                    prefs.cliprompt_lines.clamp(0, 50),
                    Message::ClipromptLinesChanged,
                )
                .step(1)
                .width(Fill),
                text(prefs.cliprompt_lines.clamp(0, 50).to_string())
                    .size(tokens::TEXT_LABEL)
                    .width(44),
            ]
            .spacing(tokens::GAP_FIELD)
            .align_y(iced::Center),
        )
        .push(Space::new().height(6))
        .push(
            text(crate::t!(
                "Temporary prompt lines shown above the command window (CLIPROMPTLINES)."
            ))
            .size(tokens::TEXT_LABEL)
            .width(sizing.width),
        )
        .push(Space::new().height(12))
        .push(
            row![
                text(crate::t!("History fade time")).size(tokens::TEXT_BODY).width(140),
                slider(
                    0..=60000,
                    prefs.commandline_fade_ms.clamp(0, 60000),
                    Message::CommandLineFadeChanged,
                )
                .step(250)
                .width(Fill),
                text(if prefs.commandline_fade_ms <= 0 {
                    crate::t!("Never").into_owned()
                } else {
                    format!("{:.1} s", prefs.commandline_fade_ms as f32 / 1000.0)
                })
                .size(tokens::TEXT_LABEL)
                .width(44),
            ]
            .spacing(tokens::GAP_FIELD)
            .align_y(iced::Center),
        )
        .push(Space::new().height(6))
        .push(
            text(crate::t!(
                "How long overlay history lines stay visible; 0 means never fade (COMMANDLINEFADETIME)."
            ))
            .size(tokens::TEXT_LABEL)
            .width(sizing.width),
        );

    let display_element = display.spacing(tokens::GAP_NONE).width(sizing.width);

    let selection = column![
        kit::heading(crate::t!("Selection"), ""),
        Space::new().height(10),
        row![
            text(crate::t!("Pick box size")).size(tokens::TEXT_BODY).width(140),
            slider(0..=50, selection.pick_box.clamp(0, 50), Message::PickBoxChanged)
                .step(1)
                .width(Fill),
            text(selection.pick_box.clamp(0, 50).to_string())
                .size(tokens::TEXT_LABEL)
                .width(44),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
        Space::new().height(8),
        text(crate::t!(
            "Controls both the visible selection box and the click aperture."
        ))
        .size(tokens::TEXT_LABEL)
        .width(sizing.width),
        Space::new().height(24),
        kit::heading(crate::t!("Selection Modes"), ""),
        Space::new().height(10),
        row![
            // Checked is the inverse of PICKADD: plain clicks replace and Shift adds.
            iced::widget::checkbox(!selection.pick_add)
                .on_toggle(Message::ShiftToAddToggled)
                .size(tokens::MARK_SQUARE),
            text(crate::t!("Use Shift to add to selection (PICKADD)")).size(tokens::TEXT_BODY),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
        Space::new().height(10),
        row![
            iced::widget::checkbox(selection.pick_drag_rect)
                .on_toggle(Message::PickDragRectToggled)
                .size(tokens::MARK_SQUARE),
            text(crate::t!(
                "Press and drag draws a rectangle instead of a lasso (PICKDRAG)"
            ))
            .size(tokens::TEXT_BODY),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
        Space::new().height(10),
        row![
            iced::widget::checkbox(selection.selection_cycling)
                .on_toggle(Message::SelectionCyclingChanged)
                .size(tokens::MARK_SQUARE),
            text(crate::t!("Clicking overlapping objects opens a picker")).size(tokens::TEXT_BODY),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
        Space::new().height(24),
        row![
            kit::heading(crate::t!("Visual Effect Settings"), ""),
            Space::new().width(Fill),
            plain_button(text(crate::t!("Restore Defaults")).size(tokens::TEXT_LABEL))
                .on_press(Message::RestoreSelectionVisualDefaults)
                .padding(kit::Size::Control.padding())
                .style(kit::secondary_style),
        ]
        .align_y(iced::Center),
        Space::new().height(10),
        row![
            iced::widget::checkbox(model_space.selection_area)
                .on_toggle(Message::SelectionAreaToggled)
                .size(tokens::MARK_SQUARE),
            text(crate::t!("Indicate selection area with transparent fill (SELECTIONAREA)"))
                .size(tokens::TEXT_BODY),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
        Space::new().height(10),
        row![
            text(crate::t!("Selection area opacity")).size(tokens::TEXT_BODY).width(140),
            slider(
                0..=100,
                model_space.selection_opacity.clamp(0, 100),
                Message::SelectionOpacityChanged,
            )
            .step(1)
            .width(Fill),
            text(format!("{}%", model_space.selection_opacity.clamp(0, 100)))
                .size(tokens::TEXT_LABEL)
                .width(44),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
        Space::new().height(6),
        text(crate::t!(
            "Transparency of the window and crossing selection areas (SELECTIONAREAOPACITY)."
        ))
        .size(tokens::TEXT_LABEL)
        .width(sizing.width),
        Space::new().height(10),
        row![
            text(crate::t!("Window selection color")).size(tokens::TEXT_BODY).width(140),
            crate::ui::kit::themed_pick_list(
                Some(selected_window_color),
                color_choice_options.clone(),
                |choice| choice.label.clone(),
            )
            .on_select(|choice| Message::SelectionWindowColorChanged(choice.value))
            .width(Fill),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
        Space::new().height(10),
        row![
            text(crate::t!("Crossing selection color")).size(tokens::TEXT_BODY).width(140),
            crate::ui::kit::themed_pick_list(
                Some(selected_crossing_color),
                color_choice_options.clone(),
                |choice| choice.label.clone(),
            )
            .on_select(|choice| Message::SelectionCrossingColorChanged(choice.value))
            .width(Fill),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
        Space::new().height(10),
        row![
            iced::widget::checkbox(model_space.selection_effect)
                .on_toggle(Message::SelectionEffectToggled)
                .size(tokens::MARK_SQUARE),
            text(crate::t!("Show selection effect (SELECTIONEFFECT)")).size(tokens::TEXT_BODY),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
        Space::new().height(10),
        row![
            text(crate::t!("Selection highlight color")).size(tokens::TEXT_BODY).width(140),
            crate::ui::kit::themed_pick_list(
                Some(selected_highlight_color),
                color_choice_options.clone(),
                |choice| choice.label.clone(),
            )
            .on_select(|choice| Message::SelectionHighlightColorChanged(choice.value))
            .width(Fill),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
        Space::new().height(24),
        kit::heading(crate::t!("Preview"), ""),
        Space::new().height(10),
        row![
            // SELECTIONPREVIEW is a bitmask; bit 1 is the idle rollover and
            // bit 2 the one that runs while a command is gathering objects.
            iced::widget::checkbox(model_space.selection_preview & 1 != 0)
                .on_toggle(Message::SelectionPreviewIdleToggled)
                .size(tokens::MARK_SQUARE),
            text(crate::t!("Preview selection when no command is active")).size(tokens::TEXT_BODY),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
        Space::new().height(10),
        row![
            iced::widget::checkbox(model_space.selection_preview & 2 != 0)
                .on_toggle(Message::SelectionPreviewCommandToggled)
                .size(tokens::MARK_SQUARE),
            text(crate::t!("Preview selection during a command")).size(tokens::TEXT_BODY),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
        Space::new().height(6),
        text(crate::t!(
            "Highlights the object under the cursor before it is picked (SELECTIONPREVIEW)."
        ))
        .size(tokens::TEXT_LABEL)
        .width(sizing.width),
        Space::new().height(24),
        kit::heading(crate::t!("Grip Settings"), ""),
        Space::new().height(10),
        row![
            text(crate::t!("Grip size")).size(tokens::TEXT_BODY).width(140),
            slider(
                1..=25,
                model_space.grip_size.clamp(1, 25),
                Message::GripSizeChanged,
            )
            .step(1)
            .width(Fill),
            text(format!("{} px", model_space.grip_size.clamp(1, 25)))
                .size(tokens::TEXT_LABEL)
                .width(44),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
        Space::new().height(10),
        row![
            text(crate::t!("Object limit for grips")).size(tokens::TEXT_BODY).width(140),
            // Values above the slider range remain available through SETVAR.
            slider(
                0..=1000,
                selection.grip_object_limit.clamp(0, 1000),
                Message::GripObjectLimitChanged,
            )
            .step(1)
            .width(Fill),
            text(if selection.grip_object_limit == 0 {
                crate::t!("Unlimited").into_owned()
            } else {
                selection.grip_object_limit.to_string()
            })
            .size(tokens::TEXT_LABEL)
            .width(44),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
        Space::new().height(6),
        text(crate::t!(
            "Past this many selected objects no grips are drawn; 0 removes the limit (GRIPOBJLIMIT)."
        ))
        .size(tokens::TEXT_LABEL)
        .width(sizing.width),
        Space::new().height(10),
        row![
            text(crate::t!("Unselected grip color")).size(tokens::TEXT_BODY).width(140),
            crate::ui::kit::themed_pick_list(
                Some(selected_grip_color),
                color_choice_options.clone(),
                |choice| choice.label.clone(),
            )
            .on_select(|choice| Message::GripColorChanged(choice.value))
            .width(Fill),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
        Space::new().height(10),
        row![
            text(crate::t!("Selected/hot grip color")).size(tokens::TEXT_BODY).width(140),
            crate::ui::kit::themed_pick_list(
                Some(selected_grip_hot),
                color_choice_options.clone(),
                |choice| choice.label.clone(),
            )
            .on_select(|choice| Message::GripHotChanged(choice.value))
            .width(Fill),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
        Space::new().height(10),
        row![
            text(crate::t!("Hover grip color")).size(tokens::TEXT_BODY).width(140),
            crate::ui::kit::themed_pick_list(
                Some(selected_grip_hover),
                color_choice_options,
                |choice| choice.label.clone(),
            )
            .on_select(|choice| Message::GripHoverChanged(choice.value))
            .width(Fill),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
    ]
    .spacing(tokens::GAP_NONE)
    .width(sizing.width);

    // ANNOAUTOSCALE's magnitude decides which objects a newly added scale
    // reaches — 1 skips layers that are off, frozen, locked or frozen in the
    // viewport, 2 keeps locked ones, 3 skips only locked, 4 takes everything.
    // The sign is the on/off the status-bar pill flips, so the mode survives
    // being switched off and back on.
    let auto_scale_options = [
        (0i8, "Off"),
        (1, "Skip objects on layers that are off, frozen or locked"),
        (2, "Skip objects on layers that are off or frozen"),
        (3, "Skip objects on locked layers"),
        (4, "All annotative objects"),
    ]
    .into_iter()
    .map(|(value, label)| Labelled {
        value,
        label: crate::t!(label).into_owned(),
    })
    .collect::<Vec<_>>();
    let selected_auto_scale = auto_scale_options
        .iter()
        .find(|choice| choice.value == prefs.annotation_auto_scale.max(0))
        .cloned();

    let qdim_options = [(0u8, "Endpoints"), (1, "Intersections")]
        .into_iter()
        .map(|(value, label)| Labelled {
            value,
            label: crate::t!(label).into_owned(),
        })
        .collect::<Vec<_>>();
    let selected_qdim = qdim_options
        .iter()
        .find(|choice| choice.value == prefs.qdim_snap_priority)
        .cloned();

    let user_prefs = column![
        kit::heading(crate::t!("User Preferences"), ""),
        Space::new().height(10),
        spacemouse,
        Space::new().height(12),
        kit::heading(crate::t!("Zoom"), ""),
        Space::new().height(10),
        row![
            iced::widget::checkbox(prefs.zoom_wheel_reversed)
                .on_toggle(Message::ZoomWheelReversedChanged)
                .size(tokens::MARK_SQUARE),
            text(crate::t!("Reverse mouse wheel zoom (ZOOMWHEEL)")).size(tokens::TEXT_BODY),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
        Space::new().height(8),
        row![
            iced::widget::checkbox(prefs.touchpad_pan)
                .on_toggle(Message::TouchpadPanChanged)
                .size(tokens::MARK_SQUARE),
            text(crate::t!("Touchpad: scroll pans, Ctrl+scroll or pinch zooms"))
                .size(tokens::TEXT_BODY),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
        Space::new().height(12),
        row![
            text(crate::t!("Zoom factor")).size(tokens::TEXT_BODY).width(150),
            slider(
                settings::ZOOM_FACTOR_MIN..=settings::ZOOM_FACTOR_SYSVAR_MAX,
                prefs
                    .zoom_factor
                    .clamp(settings::ZOOM_FACTOR_MIN, settings::ZOOM_FACTOR_SYSVAR_MAX),
                Message::ZoomFactorChanged,
            )
            .step(1)
            .width(Fill),
            text_input("60", zoom_factor_input)
                .on_input(Message::ZoomFactorInputChanged)
                .width(52),
            text(crate::tf!(
                "{:.1}% a notch",
                settings::zoom_notch_percent(prefs.zoom_factor)
            ))
            .size(tokens::TEXT_LABEL)
            .width(76),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
        Space::new().height(6),
        text(crate::t!("How far one wheel notch zooms (ZOOMFACTOR)."))
            .size(tokens::TEXT_LABEL)
            .width(sizing.width),
        Space::new().height(4),
        text(crate::tf!(
            "The slider covers the system variable's {} to {}; the field takes up to {} for a faster wheel.",
            settings::ZOOM_FACTOR_MIN,
            settings::ZOOM_FACTOR_SYSVAR_MAX,
            settings::ZOOM_FACTOR_MAX
        ))
        .size(tokens::TEXT_LABEL)
        .width(sizing.width),
        Space::new().height(24),
        kit::heading(crate::t!("Text and Dimensions"), ""),
        Space::new().height(10),
        row![
            iced::widget::checkbox(prefs.texteditmode)
                .on_toggle(Message::TextEditModeChanged)
                .size(tokens::MARK_SQUARE),
            text(crate::t!("TEXTEDIT edits one object and ends (TEXTEDITMODE)"))
                .size(tokens::TEXT_BODY),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
        Space::new().height(12),
        row![
            iced::widget::checkbox(prefs.dimension_continue_mode == 1)
                .on_toggle(Message::DimContinueModeChanged)
                .size(tokens::MARK_SQUARE),
            text(crate::t!(
                "Continued dimensions inherit the base dimension's layer and style (DIMCONTINUEMODE)"
            ))
            .size(tokens::TEXT_BODY),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
        Space::new().height(12),
        row![
            text(crate::t!("QDIM origin priority")).size(tokens::TEXT_BODY).width(150),
            crate::ui::kit::themed_pick_list(selected_qdim, qdim_options, |choice| choice.label.clone())
                .on_select(|choice| Message::QdimSnapPriorityChanged(choice.value))
                .width(Fill),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
        Space::new().height(6),
        text(crate::t!(
            "Which points QDIM measures from. Also settable inside the command."
        ))
        .size(tokens::TEXT_LABEL)
        .width(sizing.width),
        Space::new().height(24),
        kit::heading(crate::t!("Annotation"), ""),
        Space::new().height(10),
        row![
            text(crate::t!("Add scales automatically")).size(tokens::TEXT_BODY).width(150),
            crate::ui::kit::themed_pick_list(selected_auto_scale, auto_scale_options, |choice| {
                choice.label.clone()
            })
            .on_select(|choice| Message::AnnoAutoScaleChanged(choice.value))
            .width(Fill),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
        Space::new().height(6),
        text(crate::t!(
            "Which annotative objects pick up a newly set annotation scale (ANNOAUTOSCALE)."
        ))
        .size(tokens::TEXT_LABEL)
        .width(sizing.width),
        Space::new().height(24),
        kit::heading(crate::t!("Right-click Customization"), ""),
        Space::new().height(10),
        row![
            text(crate::t!("Right-click in drawing area")).size(tokens::TEXT_BODY).width(150),
            crate::ui::kit::themed_pick_list(
                selected_right_click,
                right_click_options,
                |choice| choice.label.clone(),
            )
            .on_select(|choice| Message::RightClickModeChanged(choice.value))
            .width(Fill),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
        Space::new().height(12),
        row![
            text(crate::t!("Hold duration")).size(tokens::TEXT_BODY).width(150),
            slider(
                100..=1000,
                prefs.right_click_hold_ms.clamp(100, 1000),
                Message::RightClickHoldMsChanged
            )
            .step(50)
            .width(Fill),
            text(crate::tf!("{} ms", prefs.right_click_hold_ms.clamp(100, 1000)))
                .size(tokens::TEXT_LABEL)
                .width(52),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
        Space::new().height(6),
        text(crate::t!(
            "Shortcut menu: right-click always opens the menu. Time-sensitive: a quick click is Enter, holding longer opens the menu (SHORTCUTMENUDURATION)."
        ))
        .size(tokens::TEXT_LABEL)
        .width(sizing.width),
        Space::new().height(24),
        kit::heading(crate::t!("Block Edit"), ""),
        Space::new().height(10),
        row![
            iced::widget::checkbox(double_click_block_refedit)
                .on_toggle(Message::DoubleClickBlockRefeditChanged)
                .size(tokens::MARK_SQUARE),
            text(crate::t!("Double-click to edit block in-place (REFEDIT)"))
                .size(tokens::TEXT_BODY),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
        Space::new().height(18),
        row![
            iced::widget::checkbox(double_click_block_attedit)
                .on_toggle(Message::DoubleClickBlockAtteditChanged)
                .size(tokens::MARK_SQUARE),
            text(crate::t!("Double-click attributed blocks to edit attributes (ATTEDIT)"))
                .size(tokens::TEXT_BODY),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
        Space::new().height(24),
        kit::heading(crate::t!("Drawing Units"), ""),
        Space::new().height(10),
        row![
            text(crate::t!("Length, angle and insertion units"))
                .size(tokens::TEXT_BODY)
                .width(Fill),
            plain_button(text(crate::t!("Drawing Units…")).size(tokens::TEXT_LABEL))
                .on_press(Message::OpenDrawingUnits)
                .padding(kit::Size::Control.padding())
                .style(kit::secondary_style),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
    ]
    .spacing(tokens::GAP_NONE)
    .width(sizing.width);

    // Snap modes, grid and object snap stay in the Drafting Settings dialog.
    // These two have no home: Drafting Settings shows the rotation angle but
    // offers only Reset, and the polar increment lives solely in a status-bar
    // pop-up.
    let polar_options = [90.0f32, 45.0, 30.0, 22.5, 18.0, 15.0, 10.0, 5.0, 1.0]
        .into_iter()
        .map(|value| Labelled {
            label: format!("{}°", crate::app::settings::format_snap_angle(value)),
            value,
        })
        .collect::<Vec<_>>();
    let selected_polar = polar_options
        .iter()
        .find(|choice| (choice.value - prefs.polar_increment_deg).abs() < 1e-4)
        .cloned()
        .unwrap_or_else(|| Labelled {
            value: prefs.polar_increment_deg,
            label: format!(
                "{}°",
                crate::app::settings::format_snap_angle(prefs.polar_increment_deg)
            ),
        });

    let drafting = column![
        kit::heading(crate::t!("Drafting"), ""),
        Space::new().height(10),
        row![
            text(crate::t!("Drafting rotation")).size(tokens::TEXT_BODY).width(150),
            text_input("0", snap_angle_input)
                .on_input(Message::SnapAngleInputChanged)
                .width(110),
            text(crate::t!("degrees")).size(tokens::TEXT_LABEL),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
        Space::new().height(6),
        text(crate::t!(
            "Rotates the crosshair and the snap grid in the active UCS (SNAPANG)."
        ))
        .size(tokens::TEXT_LABEL)
        .width(sizing.width),
        Space::new().height(14),
        row![
            text(crate::t!("Polar tracking increment"))
                .size(tokens::TEXT_BODY)
                .width(150),
            crate::ui::kit::themed_pick_list(Some(selected_polar), polar_options, |choice| {
                choice.label.clone()
            })
            .on_select(|choice| Message::PolarIncrementChanged(choice.value))
            .width(Fill),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
        Space::new().height(24),
        kit::heading(crate::t!("Snap and Grid"), ""),
        Space::new().height(10),
        row![
            text(crate::t!("Snap modes, grid spacing and object snap"))
                .size(tokens::TEXT_BODY)
                .width(Fill),
            plain_button(text(crate::t!("Drafting Settings…")).size(tokens::TEXT_LABEL))
                .on_press(Message::ToggleSnapPopup)
                .padding(kit::Size::Control.padding())
                .style(kit::secondary_style),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
    ]
    .spacing(tokens::GAP_NONE)
    .width(sizing.width);

    let surface_type_options = [
        (5i16, "Quadratic B-spline"),
        (6, "Cubic B-spline"),
        (8, "Bezier"),
    ]
    .into_iter()
    .map(|(value, label)| Labelled {
        value,
        label: format!("{} ({value})", crate::t!(label)),
    })
    .collect::<Vec<_>>();
    let selected_surface_type = surface_type_options
        .iter()
        .find(|choice| choice.value == drawing_prefs.surface_type)
        .cloned();

    let show_history_options = [(0i16, "Never"), (1, "As set per solid"), (2, "Always")]
        .into_iter()
        .map(|(value, label)| Labelled {
            value,
            label: crate::t!(label).into_owned(),
        })
        .collect::<Vec<_>>();
    let selected_show_history = show_history_options
        .iter()
        .find(|choice| choice.value == drawing_prefs.show_solid_history.clamp(0, 2))
        .cloned();

    let mut modeling = column![
        kit::heading(crate::t!("3D Modeling"), ""),
        Space::new().height(10),
        kit::heading(crate::t!("Display Tools"), ""),
        Space::new().height(10),
        row![
            iced::widget::checkbox(prefs.show_viewcube)
                .on_toggle(Message::ShowViewCubeChanged)
                .size(tokens::MARK_SQUARE),
            text(crate::t!("Show the navigation cube (NAVVCUBE)")).size(tokens::TEXT_BODY),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
        Space::new().height(10),
        row![
            iced::widget::checkbox(prefs.show_ucs_icon)
                .on_toggle(Message::ShowUcsIconChanged)
                .size(tokens::MARK_SQUARE),
            text(crate::t!("Show the UCS icon (UCSICON)")).size(tokens::TEXT_BODY),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
        Space::new().height(10),
        row![
            iced::widget::checkbox(prefs.ucs_icon_at_origin)
                .on_toggle(Message::UcsIconAtOriginChanged)
                .size(tokens::MARK_SQUARE),
            text(crate::t!(
                "Place the UCS icon at the origin (UCSICON ORigin)"
            ))
            .size(tokens::TEXT_BODY),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
    ];

    if drawing_prefs.available {
        modeling = modeling
            .push(Space::new().height(24))
            .push(
                row![
                    kit::heading(crate::t!("Display Resolution"), ""),
                    Space::new().width(10),
                    text(crate::t!("applies to the current drawing")).size(tokens::TEXT_LABEL),
                ]
                .align_y(iced::Center),
            )
            .push(Space::new().height(10))
            .push(
                row![
                    text(crate::t!("Isolines per surface")).size(tokens::TEXT_BODY).width(150),
                    // Retessellate once when dragging ends.
                    slider(
                        0..=64,
                        drawing_prefs.isolines.clamp(0, 64),
                        Message::IsolinesChanged,
                    )
                    .step(1i16)
                    .on_release(Message::IsolinesReleased)
                    .width(Fill),
                    text(drawing_prefs.isolines.clamp(0, 64).to_string())
                        .size(tokens::TEXT_LABEL)
                        .width(44),
                ]
                .spacing(tokens::GAP_FIELD)
                .align_y(iced::Center),
            )
            .push(Space::new().height(10))
            .push(
                row![
                    iced::widget::checkbox(drawing_prefs.display_silhouette)
                        .on_toggle(Message::DispSilhChanged)
                        .size(tokens::MARK_SQUARE),
                    text(crate::t!("Show silhouette edges on solids (DISPSILH)"))
                        .size(tokens::TEXT_BODY),
                ]
                .spacing(tokens::GAP_ROW)
                .align_y(iced::Center),
            )
            .push(Space::new().height(10))
            .push(
                row![
                    text(crate::t!("Surface density U")).size(tokens::TEXT_BODY).width(150),
                    slider(
                        0..=200,
                        drawing_prefs.surface_u.clamp(0, 200),
                        Message::SurfaceUChanged,
                    )
                    .step(1i16)
                    .width(Fill),
                    text(drawing_prefs.surface_u.clamp(0, 200).to_string())
                        .size(tokens::TEXT_LABEL)
                        .width(44),
                ]
                .spacing(tokens::GAP_FIELD)
                .align_y(iced::Center),
            )
            .push(Space::new().height(10))
            .push(
                row![
                    text(crate::t!("Surface density V")).size(tokens::TEXT_BODY).width(150),
                    slider(
                        0..=200,
                        drawing_prefs.surface_v.clamp(0, 200),
                        Message::SurfaceVChanged,
                    )
                    .step(1i16)
                    .width(Fill),
                    text(drawing_prefs.surface_v.clamp(0, 200).to_string())
                        .size(tokens::TEXT_LABEL)
                        .width(44),
                ]
                .spacing(tokens::GAP_FIELD)
                .align_y(iced::Center),
            )
            .push(Space::new().height(10))
            .push(
                row![
                    text(crate::t!("Surface type")).size(tokens::TEXT_BODY).width(150),
                    crate::ui::kit::themed_pick_list(
                        selected_surface_type,
                        surface_type_options,
                        |choice| choice.label.clone(),
                    )
                    .on_select(|choice| Message::SurfaceTypeChanged(choice.value))
                    .width(Fill),
                ]
                .spacing(tokens::GAP_FIELD)
                .align_y(iced::Center),
            )
            .push(Space::new().height(24))
            .push(
                row![
                    kit::heading(crate::t!("Solid History"), ""),
                    Space::new().width(10),
                    text(crate::t!("applies to the current drawing")).size(tokens::TEXT_LABEL),
                ]
                .align_y(iced::Center),
            )
            .push(Space::new().height(10))
            .push(
                row![
                    iced::widget::checkbox(drawing_prefs.record_solid_history)
                        .on_toggle(Message::SolidHistChanged)
                        .size(tokens::MARK_SQUARE),
                    text(crate::t!(
                        "Record the history of composite solids (SOLIDHIST)"
                    ))
                    .size(tokens::TEXT_BODY),
                ]
                .spacing(tokens::GAP_ROW)
                .align_y(iced::Center),
            )
            .push(Space::new().height(10))
            .push(
                row![
                    text(crate::t!("Show solid history")).size(tokens::TEXT_BODY).width(150),
                    crate::ui::kit::themed_pick_list(
                        selected_show_history,
                        show_history_options,
                        |choice| choice.label.clone(),
                    )
                    .on_select(|choice| Message::ShowHistChanged(choice.value))
                    .width(Fill),
                ]
                .spacing(tokens::GAP_FIELD)
                .align_y(iced::Center),
            );
    }

    let modeling = modeling.spacing(tokens::GAP_NONE).width(sizing.width);

    // The application has no support-file search path, so this page does not
    // pretend to offer one. It shows where each folder actually lives and lets
    // the user move it anywhere — which is the question people are really
    // asking when they go looking for a Files page.
    let folder_row = |label: std::borrow::Cow<'a, str>,
                      path: Option<String>,
                      kind: FolderKind,
                      custom: bool| {
        let shown = path
            .clone()
            .unwrap_or_else(|| crate::t!("Not available").into_owned());
        let mut open = plain_button(text(crate::t!("Open folder")).size(tokens::TEXT_LABEL))
            .padding(kit::Size::Control.padding())
            .style(kit::secondary_style);
        if let Some(path) = path {
            open = open.on_press(Message::OpenFolder(path));
        }
        let change = plain_button(text(crate::t!("Change...")).size(tokens::TEXT_LABEL))
            .padding(kit::Size::Control.padding())
            .style(kit::secondary_style)
            .on_press(Message::FolderChange(kind));
        let mut actions = row![change, open].spacing(tokens::GAP_HAIRLINE);
        if custom {
            actions = row![
                plain_button(text(crate::t!("Restore default")).size(tokens::TEXT_LABEL))
                    .padding(kit::Size::Control.padding())
                    .style(kit::text_style)
                    .on_press(Message::FolderReset(kind)),
                actions,
            ]
            .spacing(tokens::GAP_HAIRLINE);
        }
        row![
            column![text(label).size(tokens::TEXT_BODY), text(shown).size(tokens::TEXT_LABEL),]
                .spacing(tokens::GAP_HAIRLINE)
                .width(Fill),
            actions,
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center)
    };

    let files = column![
        kit::heading(crate::t!("Files"), ""),
        Space::new().height(6),
        text(crate::t!(
            "Where the application keeps its own files. Each folder can be moved anywhere."
        ))
        .size(tokens::TEXT_LABEL)
        .width(sizing.width),
        Space::new().height(16),
        folder_row(
            crate::t!("Configuration"),
            folders.config.clone(),
            FolderKind::Config,
            folders.config_custom,
        ),
        Space::new().height(12),
        folder_row(
            crate::t!("Plot styles"),
            folders.plot_styles.clone(),
            FolderKind::PlotStyles,
            folders.plot_styles_custom,
        ),
        Space::new().height(12),
        folder_row(
            crate::t!("Autosave files"),
            folders.autosave.clone(),
            FolderKind::Autosave,
            folders.autosave_custom,
        ),
        Space::new().height(12),
        folder_row(
            crate::t!("Fonts"),
            folders.fonts.clone(),
            FolderKind::Fonts,
            folders.fonts_custom,
        ),
        Space::new().height(16),
        text(crate::t!("Default template for new drawings")).size(tokens::TEXT_BODY),
        Space::new().height(4),
        row![
            text_input(crate::t!("Blank drawing").as_ref(), qnew_template)
                .on_input(Message::QnewTemplateChanged)
                .size(tokens::TEXT_LABEL)
                .width(Fill),
            plain_button(text(crate::t!("Browse...")).size(tokens::TEXT_LABEL))
                .padding(kit::Size::Control.padding())
                .style(kit::secondary_style)
                .on_press(Message::QnewTemplateBrowse),
        ]
        .spacing(tokens::GAP_FIELD)
        .align_y(iced::Center),
    ]
    .spacing(tokens::GAP_NONE)
    .width(sizing.width);

    let content: Element<'a, Message> = match active_tab {
        OptionsTab::General => general.into(),
        OptionsTab::Display => display_element.into(),
        OptionsTab::Selection => selection.into(),
        OptionsTab::Files => files.into(),
        OptionsTab::OpenAndSave => open_and_save.into(),
        OptionsTab::Drafting => drafting.into(),
        OptionsTab::Modeling => modeling.into(),
        OptionsTab::UserPreferences => user_prefs.into(),
        OptionsTab::Graphics => graphics,
        OptionsTab::About => crate::ui::window::about::view_page(),
    };

    // A vertical rail rather than a horizontal strip: the tab names are
    // translated into 21 languages, and a row of them stops fitting long
    // before the list of pages is complete.
    let tab_button = |label, tab| {
        let selected = active_tab == tab;
        plain_button(text(label).size(tokens::TEXT_BODY))
            .on_press(Message::OptionsTabChanged(tab))
            .padding([tokens::GAP_CONTROL, tokens::GAP_SECTION])
            .width(Fill)
            .style(if selected {
                kit::primary_style
            } else {
                kit::text_style
            })
    };
    let tabs = column![
        tab_button(crate::t!("General"), OptionsTab::General),
        tab_button(crate::t!("Files"), OptionsTab::Files),
        tab_button(crate::t!("Open and Save"), OptionsTab::OpenAndSave),
        tab_button(crate::t!("Display"), OptionsTab::Display),
        tab_button(crate::t!("Drafting"), OptionsTab::Drafting),
        tab_button(crate::t!("3D Modeling"), OptionsTab::Modeling),
        tab_button(crate::t!("Selection"), OptionsTab::Selection),
        tab_button(crate::t!("User Preferences"), OptionsTab::UserPreferences),
    ];
    // The web build has no backend to choose, so it has no Graphics page.
    #[cfg(not(target_arch = "wasm32"))]
    let tabs = tabs.push(tab_button(crate::t!("Graphics"), OptionsTab::Graphics));
    let tabs = tabs.push(tab_button(crate::t!("About"), OptionsTab::About));
    let tabs = tabs.spacing(tokens::GAP_HAIRLINE).width(iced::Length::Fixed(TAB_RAIL_WIDTH));

    let pane = column![
        // Keep the scrollbar in its own lane instead of floating over the
        // controls at the trailing edge of the Options content.
        crate::ui::kit::themed_scrollable(content).spacing(tokens::GAP_ROW).height(Fill),
        Space::new().height(12),
        row![Space::new().width(Fill), ok, apply, close].spacing(tokens::GAP_ROW),
    ]
    .width(Fill)
    .height(sizing.height);

    let body = row![tabs, Space::new().width(18), pane]
        .width(sizing.width)
        .height(sizing.height);

    let intrinsic = sizing.width == crate::ui::shell::Sizing::INTRINSIC.width;
    let main = container(body)
        .style(|theme: &Theme| kit::dialog_body_style(theme))
        .padding([tokens::GAP_GUTTER, tokens::GAP_GUTTER])
        .width(if intrinsic {
            iced::Length::Fixed(DIALOG_WIDTH)
        } else {
            sizing.width
        })
        .height(if intrinsic {
            iced::Length::Fixed(DIALOG_HEIGHT)
        } else {
            sizing.height
        });
    if !close_confirm {
        return main.into();
    }
    crate::ui::shell::discard_guard(
        main,
        Message::OptionsCloseDiscard,
        Message::OptionsCloseKeep,
    )
}
