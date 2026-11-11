//! Consolidated user configuration. Native builds use one grouped JSON file
//! (`<config>/DraftFlowCAD/settings.json`); web builds keep the same JSON in
//! `localStorage`. It holds every app preference except the command aliases,
//! which use native `ocad.pgp` or a separate web storage key. Serialized via
//! serde so the data is structured and grouped, replacing the former scattered
//! flat stores (`settings.txt` / `recent.txt` / `recent_limit.txt` /
//! `statusbar.txt` / `ribbon.txt` / `plot.txt`).

use serde::{Deserialize, Serialize};

use super::settings::UserSettings;
use crate::ui::ribbon::CollapseMode;
use crate::ui::statusbar::statusbar_config::StatusBarConfig;
use crate::ui::window::plot::PlotDialogState;

/// The whole persisted config, grouped into top-level sections.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Input modes, backup, viewport background colours, …
    pub settings: UserSettings,
    /// Iced theme selection and the six base colours used by a custom theme.
    pub theme: UiThemeConfig,
    /// Recent-files list + retained count.
    pub recent: RecentConfig,
    /// Last selected section on the tabbed Start page.
    pub start: StartConfig,
    /// Which status-bar pills the user has hidden.
    pub statusbar: StatusBarConfig,
    /// General edge-stack dock layout (which panels are docked, side, order,
    /// width and auto-collapse) for the Properties panel and block palette.
    pub dock: crate::ui::dock::DockState,
    /// Whether the optional Graphic Attributes palette is visible.
    pub show_graphic_attributes: bool,
    /// Graphic Attributes creates fills on the current layer instead of the
    /// layer of the filled object.
    pub graphic_fills_on_current_layer: bool,
    /// Add a newly selected annotation scale to existing annotative objects.
    pub annotation_auto_scale: i8,
    /// Ribbon collapse density.
    pub ribbon: RibbonConfig,
    /// Print dialog preferences (only the persisted fields; runtime state is
    /// skipped by `PlotDialogState`'s serde attributes).
    pub plot: PlotDialogState,
    /// Complete editable keyboard shortcut table.
    pub shortcuts: ShortcutConfig,
    /// Model space background, grid, and selection appearance.
    pub model_space: ModelSpaceThemeConfig,
    /// Options → Files: support folders the user pointed somewhere else.
    pub paths: PathsConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            settings: UserSettings::default(),
            theme: UiThemeConfig::default(),
            recent: RecentConfig::default(),
            start: StartConfig::default(),
            statusbar: StatusBarConfig::default(),
            dock: crate::ui::dock::DockState::sheet_layout(),
            show_graphic_attributes: false,
            graphic_fills_on_current_layer: false,
            annotation_auto_scale: -4,
            ribbon: RibbonConfig::default(),
            plot: PlotDialogState::default(),
            shortcuts: ShortcutConfig::default(),
            model_space: ModelSpaceThemeConfig::default(),
            paths: PathsConfig::default(),
        }
    }
}

/// User-chosen support folders. `None` means "follow the config folder" for
/// fonts and plot styles, and "the system temp folder" for autosave.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PathsConfig {
    pub fonts: Option<String>,
    pub plot_styles: Option<String>,
    pub autosave: Option<String>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShortcutConfig {
    pub bindings: std::collections::BTreeMap<String, String>,
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        Self {
            bindings: super::shortcuts::default_bindings(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockSide {
    Left,
    Right,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiThemeConfig {
    pub name: String,
    pub palette: UiThemePalette,
}

impl Default for UiThemeConfig {
    fn default() -> Self {
        let theme = crate::ui::style::draftflow_theme::draftflow_dark();
        Self {
            name: theme.to_string(),
            palette: UiThemePalette::from_iced(theme.seed()),
        }
    }
}

impl UiThemeConfig {
    pub fn to_iced(&self) -> iced::Theme {
        if self.name == "Custom" {
            iced::Theme::custom("Custom", self.palette.to_iced())
        } else if self.name.eq_ignore_ascii_case("System") {
            resolve_system_theme()
        } else {
            builtin_theme(&self.name).unwrap_or_else(|| {
                crate::ui::style::draftflow_theme::draftflow_dark()
            })
        }
    }
}

/// Resolve the `System` color scheme (COLORSCHEME SYSTEM) to a concrete
/// theme: the browser `prefers-color-scheme` media query on the web, the
/// Windows `AppsUseLightTheme` personalization value natively, and the dark
/// default on the remaining native platforms (no cheap OS query there).
/// Read at boot (`apply_config`) and re-read whenever COLORSCHEME SYSTEM
/// runs again — there is no live preference listener by design.
pub fn resolve_system_theme() -> iced::Theme {
    #[cfg(target_arch = "wasm32")]
    {
        let light = web_sys::window().is_some_and(|window| {
            window
                .match_media("(prefers-color-scheme: light)")
                .ok()
                .flatten()
                .is_some_and(|query| query.matches())
        });
        if light {
            iced::Theme::Light
        } else {
            iced::Theme::Dark
        }
    }
    #[cfg(all(not(target_arch = "wasm32"), target_os = "windows"))]
    {
        use windows_sys::Win32::System::Registry::{
            RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD,
        };
        let sub_key: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let value: Vec<u16> = "AppsUseLightTheme"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let mut data: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        // ERROR_SUCCESS (0) with data 1 means the apps theme is Light; a
        // missing value (older Windows, wiped personalization) is Dark.
        let light = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                sub_key.as_ptr(),
                value.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                &mut data as *mut u32 as *mut core::ffi::c_void,
                &mut size,
            ) == 0
                && data == 1
        };
        if light {
            iced::Theme::Light
        } else {
            iced::Theme::Dark
        }
    }
    #[cfg(not(any(target_arch = "wasm32", target_os = "windows")))]
    {
        iced::Theme::Dark
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiThemePalette {
    pub background: [u8; 3],
    pub text: [u8; 3],
    pub primary: [u8; 3],
    pub success: [u8; 3],
    pub warning: [u8; 3],
    pub danger: [u8; 3],
}

impl Default for UiThemePalette {
    fn default() -> Self {
        Self::from_iced(crate::ui::style::draftflow_theme::draftflow_dark().seed())
    }
}

impl UiThemePalette {
    pub fn from_iced(palette: iced::theme::palette::Seed) -> Self {
        Self {
            background: color_to_rgb(palette.background),
            text: color_to_rgb(palette.text),
            primary: color_to_rgb(palette.primary),
            success: color_to_rgb(palette.success),
            warning: color_to_rgb(palette.warning),
            danger: color_to_rgb(palette.danger),
        }
    }

    pub fn to_iced(self) -> iced::theme::palette::Seed {
        iced::theme::palette::Seed {
            background: rgb_to_color(self.background),
            text: rgb_to_color(self.text),
            primary: rgb_to_color(self.primary),
            success: rgb_to_color(self.success),
            warning: rgb_to_color(self.warning),
            danger: rgb_to_color(self.danger),
        }
    }

    pub fn hex_values(self) -> [String; 6] {
        [
            rgb_to_hex(self.background),
            rgb_to_hex(self.text),
            rgb_to_hex(self.primary),
            rgb_to_hex(self.success),
            rgb_to_hex(self.warning),
            rgb_to_hex(self.danger),
        ]
    }

    pub fn set_hex(&mut self, index: usize, value: &str) -> bool {
        let Some(rgb) = parse_hex(value) else {
            return false;
        };
        match index {
            0 => self.background = rgb,
            1 => self.text = rgb,
            2 => self.primary = rgb,
            3 => self.success = rgb,
            4 => self.warning = rgb,
            5 => self.danger = rgb,
            _ => return false,
        }
        true
    }
}

/// Every theme the user may pick: iced's built-ins, the Fusion pair, then
/// DraftFlow's own dark/light schemes. The custom themes are `Theme::Custom`,
/// so they are not in `iced::Theme::ALL`; anything enumerating themes for
/// display or for test coverage must use this instead, or they silently vanish
/// from the list.
pub fn all_themes() -> Vec<iced::Theme> {
    iced::Theme::ALL
        .iter()
        .cloned()
        .chain(crate::ui::style::fusion_theme::fusion_themes())
        .chain(crate::ui::style::draftflow_theme::draftflow_themes())
        .collect()
}

pub fn builtin_theme(name: &str) -> Option<iced::Theme> {
    all_themes()
        .into_iter()
        .find(|theme| theme.to_string() == name)
}

fn color_to_rgb(color: iced::Color) -> [u8; 3] {
    [
        (color.r * 255.0).round() as u8,
        (color.g * 255.0).round() as u8,
        (color.b * 255.0).round() as u8,
    ]
}

fn rgb_to_color(rgb: [u8; 3]) -> iced::Color {
    iced::Color::from_rgb8(rgb[0], rgb[1], rgb[2])
}

pub(crate) fn rgb_to_hex(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}

pub(crate) fn parse_hex(value: &str) -> Option<[u8; 3]> {
    let value = value.trim().strip_prefix('#').unwrap_or(value.trim());
    // Walk pairs of hex digits as chars: the old byte-length guard let a
    // 6-byte multibyte string (`aébé`) through to fixed-offset slices that
    // split a character. Exactly two digits per channel, or it is not a colour.
    let mut bytes = [0u8; 3];
    let mut chars = value.chars();
    for slot in &mut bytes {
        let high = chars.next()?.to_digit(16)?;
        let low = chars.next()?.to_digit(16)?;
        *slot = ((high << 4) | low) as u8;
    }
    if chars.next().is_some() {
        return None;
    }
    Some(bytes)
}

/// Canvas mode for Model space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ModelSpaceMode {
    /// Canvas background dynamically follows the active UI theme.
    #[default]
    MatchTheme,
    /// Classic CAD dark charcoal canvas (#212830) regardless of UI theme.
    ClassicDark,
    /// Custom background colors specified by the user.
    Custom,
}

impl ModelSpaceMode {
    pub const ALL: [Self; 3] = [Self::MatchTheme, Self::ClassicDark, Self::Custom];

    pub fn label(self) -> &'static str {
        match self {
            Self::MatchTheme => "Match Theme",
            Self::ClassicDark => "Classic CAD Dark",
            Self::Custom => "Custom",
        }
    }
}

/// Standard classic CAD dark charcoal background [33, 40, 48].
pub const CLASSIC_CAD_DARK_BG: [u8; 3] = [33, 40, 48];

/// Standard paper space sheet background [255, 255, 255].
pub const DEFAULT_PAPER_BG: [u8; 3] = [255, 255, 255];

/// Standard paper space desk surround [138, 138, 138].
pub const DEFAULT_DESK_BG: [u8; 3] = [138, 138, 138];

/// Persistent configuration for Model Space appearance and selection visual effects.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelSpaceThemeConfig {
    /// How the canvas background is determined.
    pub mode: ModelSpaceMode,
    /// Custom model space canvas background [R, G, B] (0–255).
    pub custom_bg: Option<[u8; 3]>,
    /// Custom paper space sheet background [R, G, B] (0–255).
    pub custom_paper_bg: Option<[u8; 3]>,
    /// Custom paper space desk/surround background [R, G, B] (0–255).
    pub custom_desk_bg: Option<[u8; 3]>,
    /// Grid opacity percentage (5–100, default 18%).
    pub grid_opacity: u8,
    /// SELECTIONAREA: whether selection marquees have a translucent shaded fill.
    pub selection_area: bool,
    /// SELECTIONAREAOPACITY: transparency percentage of selection areas (0–100, default 12%).
    pub selection_opacity: u8,
    /// WINDOWSAREACOLOR: ACI index (0 = Theme Primary, 1..=255 = ACI).
    pub selection_window_color: u8,
    /// CROSSINGAREACOLOR: ACI index (0 = Theme Success, 1..=255 = ACI).
    pub selection_crossing_color: u8,
    /// SELECTIONEFFECTCOLOR: ACI index for selected entities highlight (0 = Theme Primary, 1..=255 = ACI).
    pub selection_highlight_color: u8,
    /// SELECTIONEFFECT: whether selected objects glow with solid highlight (true) or dash (false).
    pub selection_effect: bool,
    /// SELECTIONPREVIEW bitmask: 1 = idle, 2 = during a command, 3 = both.
    pub selection_preview: u8,
    /// GRIPSIZE: grip marker half-size in pixels (1–25, default 5).
    pub grip_size: u8,
    /// GRIPCOLOR: unselected grip ACI color (0 = Theme Primary outline, 1..=255 = ACI).
    pub grip_color: u8,
    /// GRIPHOT: selected/hot grip ACI color (0 = Theme Danger, 1..=255 = ACI).
    pub grip_hot: u8,
    /// GRIPHOVER: hovered/warm grip ACI color (0 = Theme Primary Strong, 1..=255 = ACI).
    pub grip_hover: u8,
}

impl Default for ModelSpaceThemeConfig {
    fn default() -> Self {
        Self {
            mode: ModelSpaceMode::MatchTheme,
            custom_bg: None,
            custom_paper_bg: None,
            custom_desk_bg: None,
            grid_opacity: 18,
            selection_area: true,
            selection_opacity: 12,
            selection_window_color: 0,
            selection_crossing_color: 0,
            selection_highlight_color: 0,
            selection_effect: true,
            selection_preview: 3,
            grip_size: 5,
            grip_color: 0,
            grip_hot: 0,
            grip_hover: 0,
        }
    }
}

impl ModelSpaceThemeConfig {
    /// Resolve active model space canvas background as [f32; 4] based on current theme.
    pub fn resolve_model_bg(&self, theme: &iced::Theme) -> [f32; 4] {
        let rgb = match self.mode {
            ModelSpaceMode::MatchTheme => theme_canvas_background(theme),
            ModelSpaceMode::ClassicDark => CLASSIC_CAD_DARK_BG,
            ModelSpaceMode::Custom => self.custom_bg.unwrap_or(CLASSIC_CAD_DARK_BG),
        };
        [
            rgb[0] as f32 / 255.0,
            rgb[1] as f32 / 255.0,
            rgb[2] as f32 / 255.0,
            1.0,
        ]
    }

    /// Resolve active paper space sheet background as [f32; 4].
    pub fn resolve_paper_bg(&self) -> [f32; 4] {
        let rgb = self.custom_paper_bg.unwrap_or(DEFAULT_PAPER_BG);
        [
            rgb[0] as f32 / 255.0,
            rgb[1] as f32 / 255.0,
            rgb[2] as f32 / 255.0,
            1.0,
        ]
    }

    /// Resolve active paper space desk surround background as [f32; 4].
    pub fn resolve_desk_bg(&self) -> [f32; 4] {
        let rgb = self.custom_desk_bg.unwrap_or(DEFAULT_DESK_BG);
        [
            rgb[0] as f32 / 255.0,
            rgb[1] as f32 / 255.0,
            rgb[2] as f32 / 255.0,
            1.0,
        ]
    }

    /// Resolve active selection highlight overlay color as [f32; 4].
    pub fn resolve_selection_color(&self) -> [f32; 4] {
        if self.selection_highlight_color == 0 {
            crate::scene::model::wire_model::WireModel::SELECTED
        } else if let Some((r, g, b)) =
            codec::types::aci_table::aci_to_rgb(self.selection_highlight_color)
        {
            [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
        } else {
            crate::scene::model::wire_model::WireModel::SELECTED
        }
    }
}

/// Default canvas background for a given theme variant.
pub fn theme_canvas_background(theme: &iced::Theme) -> [u8; 3] {
    match theme {
        iced::Theme::Light => [255, 255, 255],
        iced::Theme::SolarizedLight => [253, 246, 227],
        iced::Theme::GruvboxLight => [251, 241, 199],
        iced::Theme::TokyoNightLight => [225, 226, 231],
        iced::Theme::KanagawaLotus => [242, 236, 222],
        iced::Theme::Dark => [32, 34, 37],
        iced::Theme::Dracula => [40, 42, 54],
        iced::Theme::Nord => [46, 52, 64],
        iced::Theme::SolarizedDark => [0, 43, 54],
        iced::Theme::GruvboxDark => [40, 40, 40],
        iced::Theme::TokyoNight => [26, 27, 38],
        iced::Theme::TokyoNightStorm => [36, 40, 59],
        iced::Theme::KanagawaWave => [31, 31, 40],
        iced::Theme::KanagawaDragon => [24, 26, 28],
        iced::Theme::Moonfly => [8, 18, 24],
        iced::Theme::Nightfly => [1, 22, 39],
        iced::Theme::Oxocarbon => [22, 22, 22],
        iced::Theme::Ferra => [43, 41, 46],
        // The Fusion and DraftFlow pairs deliberately break the "canvas follows
        // chrome" rule: black chrome is paired with a bright canvas in Fusion,
        // and DraftFlow pins its own sheet tones, so falling through to the
        // palette below would paint model space whatever the chrome happens to
        // be and undo the point of each theme.
        other => match crate::ui::style::fusion_theme::fusion_canvas(other)
            .or_else(|| crate::ui::style::draftflow_theme::draftflow_canvas(other))
        {
            Some(rgb) => rgb,
            None => color_to_rgb(other.palette().background.base.color),
        },
    }
}

/// Parse a flexible theme name string into an `iced::Theme` variant.
/// Normalizes by stripping spaces, underscores, and dashes, case-insensitive.
pub fn parse_theme_name(s: &str) -> Option<iced::Theme> {
    let clean: String = s
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_' && *c != '-')
        .flat_map(|c| c.to_uppercase())
        .collect();
    match clean.as_str() {
        "DARK" => Some(iced::Theme::Dark),
        "LIGHT" => Some(iced::Theme::Light),
        "DRACULA" => Some(iced::Theme::Dracula),
        "NORD" => Some(iced::Theme::Nord),
        "SOLARIZEDLIGHT" => Some(iced::Theme::SolarizedLight),
        "SOLARIZEDDARK" => Some(iced::Theme::SolarizedDark),
        "GRUVBOXLIGHT" => Some(iced::Theme::GruvboxLight),
        "GRUVBOXDARK" => Some(iced::Theme::GruvboxDark),
        "TOKYONIGHT" => Some(iced::Theme::TokyoNight),
        "TOKYONIGHTSTORM" => Some(iced::Theme::TokyoNightStorm),
        "TOKYONIGHTLIGHT" => Some(iced::Theme::TokyoNightLight),
        "KANAGAWAWAVE" => Some(iced::Theme::KanagawaWave),
        "KANAGAWADRAGON" => Some(iced::Theme::KanagawaDragon),
        "KANAGAWALOTUS" => Some(iced::Theme::KanagawaLotus),
        "MOONFLY" => Some(iced::Theme::Moonfly),
        "NIGHTFLY" => Some(iced::Theme::Nightfly),
        "OXOCARBON" => Some(iced::Theme::Oxocarbon),
        "FERRA" => Some(iced::Theme::Ferra),
        "FUSIONBLACK" => Some(crate::ui::style::fusion_theme::fusion_black()),
        "FUSIONWHITE" => Some(crate::ui::style::fusion_theme::fusion_white()),
        "DRAFTFLOWDARK" | "DFDARK" => Some(crate::ui::style::draftflow_theme::draftflow_dark()),
        "DRAFTFLOWLIGHT" | "DFLIGHT" => {
            Some(crate::ui::style::draftflow_theme::draftflow_light())
        }
        _ => None,
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RecentConfig {
    /// Recently opened file paths, newest first.
    pub files: Vec<String>,
    /// How many recent files to keep.
    pub limit: usize,
}

impl Default for RecentConfig {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            limit: super::recent::RECENT_DEFAULT,
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct StartConfig {
    pub section: super::StartSection,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RibbonConfig {
    pub collapse: CollapseMode,
}

/// A settings read or write that did not do what the user asked. Both `load`
/// and `save` used to fold every one of these into "all defaults, and
/// silence", so a user whose preferences were thrown away had no way to
/// notice the app had stopped honouring them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsFault {
    /// The config folder itself could not be resolved (no `APPDATA`/`HOME`).
    NoConfigDir,
    /// The file was read but is not settings JSON. `kept` is the copy of the
    /// unusable bytes, present when the copy succeeded.
    Corrupt {
        reason: String,
        kept: Option<std::path::PathBuf>,
    },
    /// The file is there but its bytes could not be obtained at all.
    Unreadable {
        reason: String,
    },
    /// A write did not land, so this run's changes are not persisted.
    NotSaved {
        reason: String,
    },
}

impl SettingsFault {
    /// Plain English, like the GPU fallback notices: this is read at startup,
    /// before the saved language has been applied.
    pub fn message(&self) -> String {
        match self {
            SettingsFault::NoConfigDir => {
                "Settings folder unavailable; preferences will not persist.".to_string()
            }
            SettingsFault::Corrupt { reason, kept } => match kept {
                Some(kept) => format!(
                    "Settings file is not valid settings ({reason}); defaults are in use, old file kept as {}.",
                    kept.display()
                ),
                None => format!(
                    "Settings file is not valid settings ({reason}); defaults are in use and the old file could not be kept."
                ),
            },
            SettingsFault::Unreadable { reason } => format!(
                "Settings file could not be read ({reason}); defaults are in use."
            ),
            SettingsFault::NotSaved { reason } => format!(
                "Settings could not be saved ({reason}); this run's changes will be lost."
            ),
        }
    }
}

fn settings_fault_slot() -> &'static std::sync::Mutex<Option<SettingsFault>> {
    static SLOT: std::sync::OnceLock<std::sync::Mutex<Option<SettingsFault>>> =
        std::sync::OnceLock::new();
    SLOT.get_or_init(|| std::sync::Mutex::new(None))
}

/// The fault from the most recent settings *read*, taken so the notice is
/// shown once and a healthy run shows none. A failed write is not parked
/// here: [`AppConfig::save`] hands it back to the caller that has to decide
/// whether to believe the settings are on disk.
pub fn take_settings_fault() -> Option<SettingsFault> {
    settings_fault_slot()
        .lock()
        .ok()
        .and_then(|mut slot| slot.take())
}

fn remember_settings_fault(fault: SettingsFault) {
    if let Ok(mut slot) = settings_fault_slot().lock() {
        *slot = Some(fault);
    }
}

impl AppConfig {
    /// Read the saved config, or all-defaults when the file is missing or
    /// unusable — in which case [`take_settings_fault`] holds why. Unknown or
    /// missing fields fall back to their section defaults via
    /// `#[serde(default)]`.
    pub fn load() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let (config, fault) = load_from(config_path().as_deref());
        #[cfg(target_arch = "wasm32")]
        let (config, fault) = load_from_web();

        if let Some(fault) = fault {
            remember_settings_fault(fault);
        }
        config
    }

    /// Persist the config as JSON. `Err` carries why the write did not land:
    /// only the caller knows whether it may still record these settings as
    /// saved, so the verdict is handed to it instead of parked in the startup
    /// fault slot.
    ///
    /// Does nothing under `cfg(test)` unless a test has aimed the file at a path
    /// it owns. The default path is the developer's own settings file, and the
    /// suite builds whole applications and changes preferences on them — without
    /// this, running `cargo test` rewrites the settings of whoever ran it.
    pub fn save(&self) -> Result<(), SettingsFault> {
        if cfg!(test) && std::env::var_os(SETTINGS_FILE_ENV).is_none() {
            return Ok(());
        }

        #[cfg(not(target_arch = "wasm32"))]
        let fault = match serde_json::to_string_pretty(self) {
            Err(reason) => Some(SettingsFault::NotSaved {
                reason: reason.to_string(),
            }),
            Ok(json) => write_settings(config_path().as_deref(), &json),
        };

        #[cfg(target_arch = "wasm32")]
        let fault = match (
            web_sys::window().and_then(|window| window.local_storage().ok().flatten()),
            serde_json::to_string(self),
        ) {
            (None, _) => Some(SettingsFault::NoConfigDir),
            (Some(_), Err(reason)) => Some(SettingsFault::NotSaved {
                reason: format!("{reason:?}"),
            }),
            (Some(storage), Ok(json)) => storage
                .set_item(WEB_CONFIG_KEY, &json)
                .err()
                .map(|reason| SettingsFault::NotSaved {
                    reason: format!("{reason:?}"),
                }),
        };

        fault.map_or(Ok(()), Err)
    }
}

/// Read one settings file. Split out of `AppConfig::load` and given an
/// explicit path so the states that used to be indistinguishable can be
/// pinned by tests without touching anyone's real settings.
#[cfg(not(target_arch = "wasm32"))]
fn load_from(path: Option<&std::path::Path>) -> (AppConfig, Option<SettingsFault>) {
    let Some(path) = path else {
        return (AppConfig::default(), Some(SettingsFault::NoConfigDir));
    };
    match std::fs::read_to_string(path) {
        Ok(body) => match serde_json::from_str::<AppConfig>(&body) {
            Ok(mut config) => {
                // A profile still carrying the pre-sheet default layout is
                // upgraded once, in place; a customised layout is kept.
                if config.dock.is_legacy_default() {
                    config.dock = crate::ui::dock::DockState::sheet_layout();
                }
                (config, None)
            }
            Err(reason) => {
                let kept = preserve_settings(path, &body);
                (
                    AppConfig::default(),
                    Some(SettingsFault::Corrupt {
                        reason: reason.to_string(),
                        kept,
                    }),
                )
            }
        },
        // Nothing saved yet is a first run, not a lost preference.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (AppConfig::default(), None),
        Err(error) => (
            AppConfig::default(),
            Some(SettingsFault::Unreadable {
                reason: error.to_string(),
            }),
        ),
    }
}

/// Copy unusable bytes aside before a save overwrites the file, so there is
/// still something to look at. `None` means the copy itself failed.
#[cfg(not(target_arch = "wasm32"))]
fn preserve_settings(path: &std::path::Path, body: &str) -> Option<std::path::PathBuf> {
    let kept = path.with_file_name("settings.unreadable.json");
    std::fs::write(&kept, body).ok()?;
    Some(kept)
}

/// Write the settings JSON, returning the reason it did not land. The folder
/// error is folded into the write error only when the write also failed: a
/// folder that already exists makes `create_dir_all` complain harmlessly, and
/// the write is what actually decides.
#[cfg(not(target_arch = "wasm32"))]
fn write_settings(path: Option<&std::path::Path>, json: &str) -> Option<SettingsFault> {
    let Some(path) = path else {
        return Some(SettingsFault::NoConfigDir);
    };
    let folder_error = path
        .parent()
        .and_then(|dir| std::fs::create_dir_all(dir).err());
    match std::fs::write(path, json) {
        Ok(()) => None,
        Err(error) => Some(SettingsFault::NotSaved {
            reason: match folder_error {
                Some(folder_error) => format!("{error} (settings folder: {folder_error})"),
                None => error.to_string(),
            },
        }),
    }
}

#[cfg(target_arch = "wasm32")]
fn load_from_web() -> (AppConfig, Option<SettingsFault>) {
    let Some(storage) = web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    else {
        return (AppConfig::default(), Some(SettingsFault::NoConfigDir));
    };
    match storage.get_item(WEB_CONFIG_KEY) {
        Ok(None) => (AppConfig::default(), None),
        Ok(Some(body)) => match serde_json::from_str::<AppConfig>(&body) {
            Ok(config) => (config, None),
            Err(reason) => (
                AppConfig::default(),
                Some(SettingsFault::Corrupt {
                    reason: reason.to_string(),
                    kept: None,
                }),
            ),
        },
        Err(reason) => (
            AppConfig::default(),
            Some(SettingsFault::Unreadable {
                reason: format!("{reason:?}"),
            }),
        ),
    }
}

#[cfg(target_arch = "wasm32")]
const WEB_CONFIG_KEY: &str = "draftflow.settings";

/// Aim the settings file at a path the caller owns. Only tests use it: without
/// a hook, [`AppConfig::save`] can only ever write to the developer's own
/// settings, and the branch where a write is refused stays unreachable.
const SETTINGS_FILE_ENV: &str = "DRAFTFLOW_SETTINGS_FILE";

#[cfg(not(target_arch = "wasm32"))]
fn config_path() -> Option<std::path::PathBuf> {
    if let Some(path) = std::env::var_os(SETTINGS_FILE_ENV) {
        return Some(std::path::PathBuf::from(path));
    }
    Some(crate::config::config_dir()?.join("settings.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_settings_path(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("draftflow_settings_{name}_{}_{nanos}", std::process::id()))
    }

    /// The settings hook is one process-wide name, so the tests that aim it
    /// take turns instead of moving each other's paths mid-save.
    fn settings_env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// A write that never landed may not be filed as saved. Once it was, the
    /// app carried the preference in memory, showed it as applied, and never
    /// sent another write — so it silently reverted at the next start.
    #[test]
    fn a_refused_settings_write_is_retried_when_the_next_change_comes() {
        let _serial = settings_env_lock();
        let dir = scratch_settings_path("retry");
        std::fs::create_dir_all(&dir).expect("create dir");
        // A plain file sitting where the settings folder belongs: neither the
        // folder nor the file under it can be made, on any OS.
        let blocker = dir.join("blocker");
        let refused_path = blocker.join("settings.json");
        let written_path = dir.join("settings.json");
        std::fs::write(&blocker, "").expect("write blocker");
        let mut app = crate::app::DraftFlowCAD::new_for_test();

        unsafe { std::env::set_var(SETTINGS_FILE_ENV, &refused_path) };
        app.show_graphic_attributes = true;
        app.save_config();
        app.save_config();
        unsafe { std::env::set_var(SETTINGS_FILE_ENV, &written_path) };
        app.save_config();
        unsafe { std::env::remove_var(SETTINGS_FILE_ENV) };

        assert_eq!(
            app.command_line
                .history
                .iter()
                .filter(|entry| entry.text.contains("could not be saved"))
                .count(),
            1,
            "the write that did not land is said once"
        );
        assert!(
            app.last_saved_config
                .as_ref()
                .expect("a config is always seeded")
                .show_graphic_attributes,
            "only after it landed is the change filed as saved"
        );
        let on_disk = std::fs::read_to_string(&written_path)
            .expect("the retry is what puts the file on disk");
        assert!(
            on_disk.replace(' ', "").contains("\"show_graphic_attributes\":true"),
            "the retried write carries the change the user made: {on_disk}"
        );
        let _: AppConfig =
            serde_json::from_str(&on_disk).expect("a settings file the next start can read");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `load` used to answer "all defaults" identically for a first run, a file
    /// it could not open, and a file whose bytes were not settings JSON — so a
    /// user who had just lost their preferences could not tell that apart from
    /// one who never had any.
    #[test]
    fn a_lost_preference_is_reported_instead_of_passing_as_a_first_run() {
        let path = scratch_settings_path("used");

        let (config, fault) = load_from(Some(&path));
        assert!(
            config == AppConfig::default(),
            "a first run boots on defaults"
        );
        assert!(fault.is_none(), "a first run is not a fault");

        std::fs::write(&path, r#"{"show_graphic_attributes":true}"#).expect("write");
        let (config, fault) = load_from(Some(&path));
        assert!(config.show_graphic_attributes, "a saved preference loads");
        assert_eq!(fault, None);

        std::fs::write(&path, "{ truncated").expect("write");
        let (config, fault) = load_from(Some(&path));
        assert!(
            config == AppConfig::default(),
            "unusable bytes still boot on defaults"
        );
        let Some(SettingsFault::Corrupt { kept, .. }) = fault else {
            panic!("bytes that are not settings JSON must be a corrupt-file fault, got {fault:?}");
        };
        let kept = kept.expect("the unusable bytes are kept");
        assert_eq!(kept, path.with_file_name("settings.unreadable.json"));
        assert_eq!(
            std::fs::read_to_string(&kept).expect("read kept"),
            "{ truncated",
            "a save must not be able to erase the only copy of what was there"
        );

        std::fs::remove_file(&path).expect("remove file");
        std::fs::remove_file(&kept).expect("remove kept");
    }

    /// The other half of the same fold: a non-`NotFound` open failure (here a
    /// folder sitting in the file's place) is not a first run.
    #[test]
    fn a_settings_file_that_cannot_be_opened_is_not_reported_as_a_first_run() {
        let path = scratch_settings_path("blocked");
        std::fs::create_dir(&path).expect("create blocker");
        let (config, fault) = load_from(Some(&path));
        assert!(
            config == AppConfig::default(),
            "an unreadable file still boots on defaults"
        );
        assert!(
            matches!(fault, Some(SettingsFault::Unreadable { .. })),
            "an open failure must not collapse into the first-run answer: {fault:?}"
        );
        assert!(
            fault.unwrap().message().contains("defaults are in use"),
            "the notice has to say the preferences are not the user's own"
        );
        std::fs::remove_dir(&path).expect("remove blocker");
    }

    /// `save` threw away all three of its failures; `write_settings` returns
    /// them, with the folder error folded in only when the write also failed.
    #[test]
    fn a_settings_write_that_does_not_land_says_why() {
        let dir = scratch_settings_path("writable");
        std::fs::create_dir_all(&dir).expect("create dir");
        let path = dir.join("settings.json");

        assert_eq!(write_settings(Some(&path), "{}"), None);
        assert_eq!(std::fs::read_to_string(&path).expect("read back"), "{}");

        let blocker = dir.join("blocker");
        std::fs::write(&blocker, "").expect("write blocker");
        let fault = write_settings(Some(&blocker.join("settings.json")), "{}")
            .expect("writing under a file must report");
        assert!(matches!(fault, SettingsFault::NotSaved { .. }));
        assert!(
            fault.message().contains("settings folder"),
            "the folder failure is part of why nothing landed: {}",
            fault.message()
        );

        assert_eq!(write_settings(None, "{}"), Some(SettingsFault::NoConfigDir));
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn system_scheme_resolves_to_a_concrete_theme_and_keeps_its_name() {
        // Fresh configs keep the existing default; SYSTEM is opt-in.
        assert_ne!(UiThemeConfig::default().name, "System");
        let mut cfg = UiThemeConfig::default();
        cfg.name = "System".to_string();
        let resolved = cfg.to_iced();
        // Case-insensitive like the other stored names.
        cfg.name = "system".to_string();
        assert_eq!(cfg.to_iced(), resolved);
        assert!(
            matches!(resolved, iced::Theme::Light | iced::Theme::Dark),
            "SYSTEM must resolve through the platform preference"
        );
        // The stored name survives resolution; re-runs re-resolve.
        assert_eq!(cfg.name, "system");
        assert_eq!(resolve_system_theme(), resolved);
    }

    #[test]
    fn test_model_space_defaults() {
        let cfg = ModelSpaceThemeConfig::default();
        assert_eq!(cfg.mode, ModelSpaceMode::MatchTheme);
        assert_eq!(cfg.custom_bg, None);
        assert_eq!(cfg.custom_paper_bg, None);
        assert_eq!(cfg.grid_opacity, 18);
        assert!(cfg.selection_area);
        assert_eq!(cfg.selection_opacity, 12);
        assert_eq!(cfg.selection_window_color, 0);
        assert_eq!(cfg.selection_crossing_color, 0);
        assert_eq!(cfg.selection_highlight_color, 0);
        assert!(cfg.selection_effect);
        assert_eq!(cfg.selection_preview, 3);
    }

    #[test]
    fn test_resolve_model_bg_match_theme() {
        let cfg = ModelSpaceThemeConfig {
            mode: ModelSpaceMode::MatchTheme,
            ..Default::default()
        };
        // Light theme should produce pure white background [1.0, 1.0, 1.0, 1.0]
        let light_bg = cfg.resolve_model_bg(&iced::Theme::Light);
        assert_eq!(light_bg, [1.0, 1.0, 1.0, 1.0]);

        // Oxocarbon theme should produce dark background
        let oxo_bg = cfg.resolve_model_bg(&iced::Theme::Oxocarbon);
        assert!((oxo_bg[0] - 22.0 / 255.0).abs() < 1e-4);
        assert!((oxo_bg[1] - 22.0 / 255.0).abs() < 1e-4);
        assert!((oxo_bg[2] - 22.0 / 255.0).abs() < 1e-4);
        assert_eq!(oxo_bg[3], 1.0);
    }

    #[test]
    fn test_resolve_model_bg_classic_dark() {
        let cfg = ModelSpaceThemeConfig {
            mode: ModelSpaceMode::ClassicDark,
            ..Default::default()
        };
        // Even with Light theme active, ClassicDark must stay locked to [33, 40, 48]
        let bg = cfg.resolve_model_bg(&iced::Theme::Light);
        assert!((bg[0] - 33.0 / 255.0).abs() < 1e-4);
        assert!((bg[1] - 40.0 / 255.0).abs() < 1e-4);
        assert!((bg[2] - 48.0 / 255.0).abs() < 1e-4);
        assert_eq!(bg[3], 1.0);
    }

    #[test]
    fn test_resolve_model_bg_custom() {
        let cfg = ModelSpaceThemeConfig {
            mode: ModelSpaceMode::Custom,
            custom_bg: Some([10, 20, 30]),
            ..Default::default()
        };
        let bg = cfg.resolve_model_bg(&iced::Theme::Light);
        assert!((bg[0] - 10.0 / 255.0).abs() < 1e-4);
        assert!((bg[1] - 20.0 / 255.0).abs() < 1e-4);
        assert!((bg[2] - 30.0 / 255.0).abs() < 1e-4);
    }

    #[test]
    fn test_model_space_config_serde_roundtrip() {
        let mut original = AppConfig::default();
        original.model_space.mode = ModelSpaceMode::Custom;
        original.model_space.custom_bg = Some([12, 34, 56]);
        original.model_space.grid_opacity = 45;
        original.model_space.selection_opacity = 35;
        original.model_space.selection_window_color = 5;

        let serialized = serde_json::to_string(&original).expect("serialize config");
        let deserialized: AppConfig =
            serde_json::from_str(&serialized).expect("deserialize config");

        assert_eq!(deserialized.model_space.mode, ModelSpaceMode::Custom);
        assert_eq!(deserialized.model_space.custom_bg, Some([12, 34, 56]));
        assert_eq!(deserialized.model_space.grid_opacity, 45);
        assert_eq!(deserialized.model_space.selection_opacity, 35);
        assert_eq!(deserialized.model_space.selection_window_color, 5);
    }

    #[test]
    fn graphic_attributes_visibility_defaults_off_and_round_trips() {
        let missing: AppConfig = serde_json::from_str("{}").expect("deserialize old config");
        assert!(!missing.show_graphic_attributes);

        let mut original = AppConfig::default();
        original.show_graphic_attributes = true;
        let serialized = serde_json::to_string(&original).expect("serialize config");
        let restored: AppConfig = serde_json::from_str(&serialized).expect("deserialize config");
        assert!(restored.show_graphic_attributes);
    }

    #[test]
    fn test_parse_theme_name() {
        assert_eq!(parse_theme_name("light"), Some(iced::Theme::Light));
        assert_eq!(parse_theme_name("DARK"), Some(iced::Theme::Dark));
        assert_eq!(
            parse_theme_name("solarized light"),
            Some(iced::Theme::SolarizedLight)
        );
        assert_eq!(
            parse_theme_name("tokyo-night-storm"),
            Some(iced::Theme::TokyoNightStorm)
        );
        assert_eq!(
            parse_theme_name("Kanagawa_Lotus"),
            Some(iced::Theme::KanagawaLotus)
        );
        assert_eq!(parse_theme_name("1"), None);
        assert_eq!(parse_theme_name("0"), None);
        assert_eq!(parse_theme_name("nonexistent_theme"), None);
    }

    /// The old guard checked the BYTE length (`value.len() != 6`) but sliced
    /// at fixed byte offsets — a 6-byte multibyte string like `aébé` passed
    /// the guard and then split a character:
    /// `byte index 2 is not a char boundary; it is inside 'é'`. Pasting it
    /// into any Options colour field killed the process on the keystroke.
    #[test]
    fn test_parse_hex_rejects_multibyte_input_without_panicking() {
        for value in ["aébé", "#aébé", "éééééé", "#ébé#"] {
            assert_eq!(parse_hex(value), None, "{value} should not parse");
        }
        // Non-hex garbage and wrong lengths stay None.
        for value in ["gggggg", "a1b2c", "a1b2c34", "#GGHHII"] {
            assert_eq!(parse_hex(value), None, "{value} should not parse");
        }
        // Valid input keeps working, with and without '#', with whitespace.
        assert_eq!(parse_hex("a1b2c3"), Some([0xa1, 0xb2, 0xc3]));
        assert_eq!(parse_hex("#A1B2C3"), Some([0xa1, 0xb2, 0xc3]));
        assert_eq!(parse_hex("  000000  "), Some([0, 0, 0]));
    }
}
