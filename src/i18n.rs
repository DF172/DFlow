//! Application language selection and embedded Fluent resources.

use i18n_embed::fluent::{fluent_language_loader, FluentLanguageLoader};
#[cfg(all(not(target_arch = "wasm32"), not(test)))]
use i18n_embed::DesktopLanguageRequester;
use i18n_embed::LanguageLoader;
#[cfg(target_arch = "wasm32")]
use i18n_embed::WebLanguageRequester;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::OnceLock;

#[path = "locale_catalog.rs"]
mod locale_catalog;

#[derive(RustEmbed)]
#[folder = "locales/"]
struct Localizations;

/// User-selectable UI language. `System` keeps following the platform's
/// preferred locale while explicit choices remain stable across restarts.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    #[serde(rename = "system")]
    System,
    #[serde(rename = "en-US")]
    EnUs,
    #[serde(rename = "bg-BG")]
    BgBg,
    #[serde(rename = "tr-TR")]
    TrTr,
    #[serde(rename = "nl-NL")]
    NlNl,
    #[serde(rename = "fr-FR")]
    FrFr,
    #[serde(rename = "de-DE")]
    DeDe,
    #[serde(rename = "hi-IN")]
    HiIn,
    #[serde(rename = "ru-RU")]
    RuRu,
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "es-ES")]
    EsEs,
    #[serde(rename = "pt-BR")]
    PtBr,
    #[serde(rename = "ar-SA")]
    ArSa,
    #[serde(rename = "ja-JP")]
    JaJp,
    #[serde(rename = "ko-KR")]
    KoKr,
    #[serde(rename = "lo-LA")]
    LoLa,
    #[serde(rename = "cs-CZ")]
    CsCz,
    #[serde(rename = "it-IT")]
    ItIt,
    #[serde(rename = "fi-FI")]
    FiFi,
    #[serde(rename = "hu-HU")]
    HuHu,
    #[serde(rename = "pl-PL")]
    PlPl,
    #[serde(rename = "zh-TW")]
    ZhTw,
    #[serde(rename = "el-GR")]
    ElGr,
}

impl Language {
    pub const ALL: [Language; 23] = [
        Language::System,
        Language::EnUs,
        Language::BgBg,
        Language::PtBr,
        Language::CsCz,
        Language::NlNl,
        Language::FrFr,
        Language::FiFi,
        Language::DeDe,
        Language::ElGr,
        Language::HuHu,
        Language::ItIt,
        Language::JaJp,
        Language::KoKr,
        Language::LoLa,
        Language::PlPl,
        Language::RuRu,
        Language::ZhCn,
        Language::EsEs,
        Language::ZhTw,
        Language::TrTr,
        Language::HiIn,
        Language::ArSa,
    ];

    fn requested(self) -> Vec<i18n_embed::unic_langid::LanguageIdentifier> {
        match self {
            Language::System => system_languages(),
            Language::EnUs => vec!["en-US".parse().expect("valid locale")],
            Language::BgBg => vec!["bg-BG".parse().expect("valid locale")],
            Language::TrTr => vec!["tr-TR".parse().expect("valid locale")],
            Language::NlNl => vec!["nl-NL".parse().expect("valid locale")],
            Language::FrFr => vec!["fr-FR".parse().expect("valid locale")],
            Language::DeDe => vec!["de-DE".parse().expect("valid locale")],
            Language::HiIn => vec!["hi-IN".parse().expect("valid locale")],
            Language::RuRu => vec!["ru-RU".parse().expect("valid locale")],
            Language::ZhCn => vec!["zh-CN".parse().expect("valid locale")],
            Language::EsEs => vec!["es-ES".parse().expect("valid locale")],
            Language::PtBr => vec!["pt-BR".parse().expect("valid locale")],
            Language::ArSa => vec!["ar-SA".parse().expect("valid locale")],
            Language::JaJp => vec!["ja-JP".parse().expect("valid locale")],
            Language::KoKr => vec!["ko-KR".parse().expect("valid locale")],
            Language::LoLa => vec!["lo-LA".parse().expect("valid locale")],
            Language::CsCz => vec!["cs-CZ".parse().expect("valid locale")],
            Language::ItIt => vec!["it-IT".parse().expect("valid locale")],
            Language::FiFi => vec!["fi-FI".parse().expect("valid locale")],
            Language::HuHu => vec!["hu-HU".parse().expect("valid locale")],
            Language::PlPl => vec!["pl-PL".parse().expect("valid locale")],
            Language::ZhTw => vec!["zh-TW".parse().expect("valid locale")],
            Language::ElGr => vec!["el-GR".parse().expect("valid locale")],
        }
    }

    pub fn label(self) -> String {
        match self {
            Language::System => crate::tr!("language", "system"),
            Language::EnUs => crate::tr!("language", "english"),
            Language::BgBg => crate::tr!("language", "bulgarian"),
            Language::TrTr => crate::tr!("language", "turkish"),
            Language::NlNl => crate::tr!("language", "dutch"),
            Language::FrFr => crate::tr!("language", "french"),
            Language::DeDe => crate::tr!("language", "german"),
            Language::HiIn => crate::tr!("language", "hindi"),
            Language::RuRu => crate::tr!("language", "russian"),
            Language::ZhCn => crate::tr!("language", "chinese-simplified"),
            Language::EsEs => crate::tr!("language", "spanish"),
            Language::PtBr => crate::tr!("language", "portuguese"),
            Language::ArSa => crate::tr!("language", "arabic"),
            Language::JaJp => crate::tr!("language", "japanese"),
            Language::KoKr => crate::tr!("language", "korean"),
            Language::LoLa => crate::tr!("language", "lao"),
            Language::CsCz => crate::tr!("language", "czech"),
            Language::ItIt => crate::tr!("language", "italian"),
            Language::FiFi => crate::tr!("language", "finnish"),
            Language::HuHu => crate::tr!("language", "hungarian"),
            Language::PlPl => crate::tr!("language", "polish"),
            Language::ZhTw => crate::tr!("language", "chinese-traditional"),
            Language::ElGr => crate::tr!("language", "greek"),
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = self.label();
        f.write_str(&label)
    }
}

fn system_languages() -> Vec<i18n_embed::unic_langid::LanguageIdentifier> {
    // The unit tests compare English prompts and messages through one
    // process-wide loader, and every `apply_config` of a default config asks
    // for the system language, so the machine's locale must never reach
    // them — not even from a test that runs in parallel.
    #[cfg(test)]
    {
        vec!["en-US"
            .parse()
            .expect("en-US is a valid language identifier")]
    }

    #[cfg(not(test))]
    {
        #[cfg(target_arch = "wasm32")]
        let mut requested = WebLanguageRequester::requested_languages();
        #[cfg(not(target_arch = "wasm32"))]
        let requested = DesktopLanguageRequester::requested_languages();

        // `navigator.languages` may be empty in privacy-restricted browser
        // contexts. The singular preference is still exposed by mainstream
        // browsers, so keep it as the first fallback before English.
        #[cfg(target_arch = "wasm32")]
        if requested.is_empty() {
            if let Some(language) = web_sys::window()
                .and_then(|window| window.navigator().language())
                .and_then(|language| language.parse().ok())
            {
                requested.push(language);
            }
        }

        requested
    }
}

fn load_language(
    loader: &FluentLanguageLoader,
    language: Language,
) -> Result<(), i18n_embed::I18nEmbedError> {
    let mut requested = language.requested();
    if requested.is_empty() {
        requested.push(loader.fallback_language().clone());
    }
    #[allow(unused_variables)]
    let selected = i18n_embed::select(loader, &Localizations, &requested)?;
    #[cfg(target_arch = "wasm32")]
    if let Some(language) = selected.first() {
        if let Some(root) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.document_element())
        {
            let _ = root.set_attribute("lang", &language.to_string());
            let direction = if language.to_string().starts_with("ar") {
                "rtl"
            } else {
                "ltr"
            };
            let _ = root.set_attribute("dir", direction);
        }
    }
    Ok(())
}

pub fn loader() -> &'static FluentLanguageLoader {
    static LOADER: OnceLock<FluentLanguageLoader> = OnceLock::new();
    LOADER.get_or_init(|| {
        let loader = fluent_language_loader!();
        if let Err(error) = load_language(&loader, Language::System) {
            eprintln!("Unable to load system UI language: {error}");
            loader
                .load_languages(&Localizations, &[loader.fallback_language().clone()])
                .expect("fallback UI language must be embedded");
        }
        loader
    })
}

/// Apply a user preference process-wide. The loader swaps resources atomically,
/// so the next Iced view pass immediately receives the new language.
pub fn set_language(language: Language) -> Result<(), i18n_embed::I18nEmbedError> {
    load_language(loader(), language)
}

pub fn active_language_tag() -> String {
    loader()
        .current_languages()
        .into_iter()
        .next()
        .unwrap_or_else(|| loader().fallback_language().clone())
        .to_string()
}

/// Translate an application-facing source label from the complete UI catalog.
///
/// Stable semantic ids remain preferable for new code. This compatibility
/// layer lets existing UI surfaces move to Fluent without turning command
/// tokens, property ids, file-format values, or plug-in supplied text into
/// translatable data.
pub fn translate(source: impl AsRef<str>) -> Cow<'static, str> {
    let source = source.as_ref();
    locale_catalog::message_attribute(source)
        .map(|(message_id, attribute_id)| Cow::Owned(loader().get_attr(message_id, attribute_id)))
        .unwrap_or_else(|| Cow::Owned(source.to_string()))
}

/// Translate a catalog message and replace the named values used by legacy
/// command prompts. Markers preserve values through Fluent parsing.
pub fn translate_args(source: impl AsRef<str>, args: &[(&str, String)]) -> Cow<'static, str> {
    let source = source.as_ref();
    let mut translated = locale_catalog::message_attribute(source)
        .map(|(message_id, attribute_id)| loader().get_attr(message_id, attribute_id))
        .unwrap_or_else(|| source.to_string());
    for (name, value) in args {
        translated = translated.replace(&format!("__ocs_arg_{name}__"), value);
        translated = translated.replace(&format!("%{{{name}}}"), value);
    }
    Cow::Owned(translated)
}

/// Translate a Rust formatting template after its values have been rendered.
/// The catalog stores positional markers, while this function recovers each
/// rendered value from the English template and places it in the localized
/// sentence. File names, handles, counts, and command values therefore remain
/// data instead of being sent through translation.
pub fn translate_format(template: &str, rendered: String) -> Cow<'static, str> {
    let Some((message_id, attribute_id)) = locale_catalog::message_attribute(template) else {
        return Cow::Owned(rendered);
    };
    let Some(values) = format_values(template, &rendered) else {
        return Cow::Owned(rendered);
    };
    let mut translated = loader().get_attr(message_id, attribute_id);
    for (index, value) in values.iter().enumerate() {
        translated = translated.replace(&format!("__ocs_fmt_{index}__"), value);
    }
    Cow::Owned(translated)
}

fn format_values(template: &str, rendered: &str) -> Option<Vec<String>> {
    let mut literals = vec![String::new()];
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        match (ch, chars.peek().copied()) {
            ('{', Some('{')) => {
                chars.next();
                literals.last_mut()?.push('{');
            }
            ('}', Some('}')) => {
                chars.next();
                literals.last_mut()?.push('}');
            }
            ('{', _) => {
                let mut closed = false;
                for inner in chars.by_ref() {
                    if inner == '}' {
                        closed = true;
                        break;
                    }
                }
                if !closed {
                    return None;
                }
                literals.push(String::new());
            }
            ('}', _) => return None,
            _ => literals.last_mut()?.push(ch),
        }
    }

    let mut cursor = 0;
    if !rendered.starts_with(&literals[0]) {
        return None;
    }
    cursor += literals[0].len();
    let mut values = Vec::with_capacity(literals.len().saturating_sub(1));
    for index in 1..literals.len() {
        let separator = &literals[index];
        if index + 1 == literals.len() && separator.is_empty() {
            values.push(rendered[cursor..].to_string());
            cursor = rendered.len();
        } else {
            let offset = rendered[cursor..].find(separator)?;
            values.push(rendered[cursor..cursor + offset].to_string());
            cursor += offset + separator.len();
        }
    }
    (cursor == rendered.len()).then_some(values)
}

/// The names this app mints itself -- the default layout list, the WCS sentinel
/// the viewcube picker injects, the untitled document tab -- are shown through
/// the catalog while the stored value stays English. `Model` / `Layout1` are what
/// `LAYOUT` and `MSPACE` compare against and what DXF writes back, and `Drawing3`
/// seeds both the `Save as` default name and the MCP document list, so mapping the
/// data would break a consumer, not a label.
///
/// A user-renamed layout or named UCS falls to the last arm untouched: it has no
/// catalog entry to land on, and the whitelist here is the only thing keeping a
/// layout someone named `Model` from being painted `模型`.
pub fn app_name_display(raw: &str) -> String {
    match raw {
        "Model" | "WCS" => translate(raw).into_owned(),
        other if is_default_numbered_name(other, "Layout") => {
            translate_format("Layout{}", other.to_string()).into_owned()
        }
        other if is_default_numbered_name(other, "Drawing") => {
            translate_format("Drawing{}", other.to_string()).into_owned()
        }
        other => other.to_string(),
    }
}

/// `Layout12` is the default-name shape; `LayoutFoo` is someone else's name.
fn is_default_numbered_name(raw: &str, prefix: &str) -> bool {
    match raw.strip_prefix(prefix) {
        Some(tail) => !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()),
        None => false,
    }
}

/// Built-in ribbon modules have stable ids; plug-ins keep their supplied title
/// until they provide their own localization bundle.
pub fn ribbon_module_title(id: &str, fallback: &str) -> String {
    match id {
        "draw" => crate::tr!("ribbon-tab", "draw"),
        "parametric" => crate::tr!("ribbon-tab", "parametric"),
        "annotate" => crate::tr!("ribbon-tab", "annotate"),
        "insert" => crate::tr!("ribbon-tab", "insert"),
        "model" => crate::tr!("ribbon-tab", "model"),
        "layout" => crate::tr!("ribbon-tab", "layout"),
        "manage" => crate::tr!("ribbon-tab", "manage"),
        "view" => crate::tr!("ribbon-tab", "view"),
        _ => fallback.to_string(),
    }
}

#[macro_export]
macro_rules! tr {
    ($message_id:literal, $attribute_id:literal $(,)?) => {
        i18n_embed_fl::fl!($crate::i18n::loader(), $message_id, $attribute_id)
    };
    ($message_id:literal, $attribute_id:literal, $($name:ident = $value:expr),+ $(,)?) => {
        i18n_embed_fl::fl!(
            $crate::i18n::loader(),
            $message_id,
            $attribute_id,
            $($name = $value),+
        )
    };
    ($message_id:literal $(,)?) => {
        i18n_embed_fl::fl!($crate::i18n::loader(), $message_id)
    };
    ($message_id:literal, $($name:ident = $value:expr),+ $(,)?) => {
        i18n_embed_fl::fl!(
            $crate::i18n::loader(),
            $message_id,
            $($name = $value),+
        )
    };
}

/// Source-catalog compatibility macro used while the existing interface is
/// migrated to semantic Fluent ids. Named arguments mirror the command prompt
/// placeholders already present in the source catalog.
#[macro_export]
macro_rules! t {
    ($source:expr $(,)?) => {
        $crate::i18n::translate($source)
    };
    ($source:expr, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::i18n::translate_args(
            $source,
            &[$((stringify!($name), ($value).to_string())),+],
        )
    };
}

/// Localized counterpart of `format!` for application-facing messages.
#[macro_export]
macro_rules! tf {
    ($template:literal $($args:tt)*) => {{
        let rendered = format!($template $($args)*);
        $crate::i18n::translate_format($template, rendered)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn donation_text_is_translated_in_every_locale() {
        for language in Language::ALL
            .into_iter()
            .filter(|language| *language != Language::System)
        {
            let loader = FluentLanguageLoader::new("draftflow", "en-US".parse().unwrap());
            load_language(&loader, language).expect("Fluent resources must parse");
            let attributes: BTreeSet<String> =
                loader.with_message_iter(&language.requested()[0], |messages| {
                    messages
                        .filter(|message| message.id.name == "donation")
                        .flat_map(|message| {
                            message
                                .attributes
                                .iter()
                                .map(|attr| attr.id.name.to_owned())
                        })
                        .collect()
                });
            assert_eq!(
                attributes,
                BTreeSet::from(["title", "heading", "body", "decline"].map(String::from))
            );
            for attribute in attributes {
                assert!(!loader.get_attr("donation", &attribute).is_empty());
            }
        }
    }

    #[test]
    fn every_catalog_covers_and_formats_the_source_catalog() {
        let loader = FluentLanguageLoader::new("draftflow", "en-US".parse().unwrap());
        load_language(&loader, Language::EnUs).expect("English Fluent resources must parse");
        let keys = |loader: &FluentLanguageLoader, language: Language| {
            loader.with_message_iter(&language.requested()[0], |messages| {
                let entries: Vec<_> = messages
                    .flat_map(|message| {
                        std::iter::once((message.id.name.to_owned(), String::new())).chain(
                            message.attributes.iter().map(|attribute| {
                                (message.id.name.to_owned(), attribute.id.name.to_owned())
                            }),
                        )
                    })
                    .collect();
                let unique: BTreeSet<_> = entries.iter().cloned().collect();
                assert_eq!(
                    entries.len(),
                    unique.len(),
                    "duplicate keys in {language:?}"
                );
                unique
            })
        };
        let source = keys(&loader, Language::EnUs);
        assert!(!source.is_empty());

        let english = Localizations::get("en-US/draftflow.ftl").unwrap();
        let english = std::str::from_utf8(&english.data).unwrap();
        let variables: Vec<_> = english
            .split('$')
            .skip(1)
            .map(|tail| {
                tail.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-')
                    .next()
                    .unwrap()
            })
            .collect();
        let args = variables.into_iter().map(|name| (name, 2)).collect();
        let render = |loader: &FluentLanguageLoader, message: &str, attribute: &str| {
            loader
                .with_fluent_message_and_bundle(message, |fluent_message, bundle| {
                    let pattern = if attribute.is_empty() {
                        fluent_message.value()
                    } else {
                        fluent_message
                            .get_attribute(attribute)
                            .map(|attr| attr.value())
                    };
                    pattern
                        .map(|pattern| {
                            let mut errors = Vec::new();
                            let text = bundle.format_pattern(pattern, Some(&args), &mut errors);
                            assert!(errors.is_empty(), "{message}.{attribute}: {errors:?}");
                            text.into_owned()
                        })
                        .unwrap_or_default()
                })
                .unwrap()
        };
        let markers = |text: &str| -> BTreeSet<String> {
            text.split("__ocs_")
                .skip(1)
                .filter_map(|tail| tail.split_once("__").map(|(marker, _)| marker.to_owned()))
                .collect()
        };
        for language in Language::ALL.into_iter().filter(|l| *l != Language::System) {
            let resource =
                Localizations::get(&format!("{}/draftflow.ftl", language.requested()[0]))
                    .unwrap();
            let text = std::str::from_utf8(&resource.data).unwrap();
            if let Err((_, errors)) = fluent_syntax::parser::parse(text) {
                panic!("Invalid Fluent syntax in {language:?}: {errors:?}");
            }
            let localized = FluentLanguageLoader::new("draftflow", "en-US".parse().unwrap());
            load_language(&localized, language).expect("Fluent resources must parse");
            let actual = keys(&localized, language);
            let extra: Vec<_> = actual.difference(&source).collect();
            assert!(extra.is_empty(), "extra keys in {language:?}: {extra:?}");
            let missing: Vec<_> = source.difference(&actual).collect();
            assert!(
                missing.is_empty(),
                "{} missing keys in {language:?}: {:?}",
                missing.len(),
                &missing[..missing.len().min(10)]
            );
            for (message, attribute) in &source {
                let expected = render(&loader, message, attribute);
                let translated = render(&localized, message, attribute);
                assert_eq!(
                    markers(&translated),
                    markers(&expected),
                    "{language:?}: {message}.{attribute}"
                );
                assert_eq!(
                    translated.is_empty(),
                    expected.is_empty(),
                    "{language:?}: {message}.{attribute}"
                );
            }
        }
    }

    /// Technical names, protocol ids and command tokens are data, not application-facing
    /// prose, so they stay English by decision and never enter the catalog.
    const STAYS_ENGLISH: &[&str] = &[
        "CPU",
        "RAM",
        "GPU",
        "GPU (backend)",
        "SpaceMouse",
        "getpoint {}: {}",
        // Product, licence and engine names. A Chinese drawing session still reads these unchanged.
        "DraftFlow CAD",
        "DraftFlow CAD {}",
        "GNU GPL v3",
        "Mental Ray",
        "Rapid RT",
        "SpaceMouse · {}",
        // Graphics backends: the row shows the API the driver selected, matched out of
        // `active.backend` (`Some("dx12") => "DirectX 12"` in options/graphics.rs), so the value is an
        // identifier, not prose.
        "DirectX 12",
        "Vulkan",
        "OpenGL",
        "Metal",
        // Sentinels and identifiers. `*VARIES*` is `VARIES_LABEL` in three modules and is compared by
        // string (`text_input(if varies { "*VARIES*" } …)`), so translating it would break the
        // "several values" state of every combo it appears in.
        "*VARIES*",
        "API {}",
        "GUID",
        "WKT",
        "ACI {}",
        "{} px",
        "fx",
        // System-variable echoes: the name is what SETVAR/INQUIRE consumers read back.
        "STYLE",
        "DONUTID",
        "CETRANSPARENCY = {}",
        "ATTACH: {e}",
        "FILLETEDGE: {error}",
        "CHAMFEREDGE: {error}",
        // Developer performance overlay (tessellation timing strip), not a user-facing string.
        "tess {:.1} ms · {} wires · epoch {}",
        // #190 prompt() literals with no prose to translate: the visible text is a value,
        // a {variable} another arm already decides, or a bracket group. The `[Option/Listing]`
        // part really is invisible -- strip_option_listing (src/ui/command_line.rs:104) deletes it
        // from the shown line and the chips come from the Rust CmdOption list, which #188 decides.
        // The `<default>` echo inside those same rows is NOT invisible: nothing in the app parses
        // angle brackets, so it reaches the screen as written. That exposure is #194 (measured
        // 88 zh-CN slots), not a reason to translate these three rows here -- the same masking rule
        // that protects the keyword casing hides the group from the term pass too.
        "{} <{value:.4}>{suffix}",
        "{prompt} <{value:.4}>",
        "BOUNDARY  {} <{}>:",
        "[Use current origin/Set new origin/Default to boundary extents] <Use current origin>:",
        "[bottom Left/bottom Right/top rIght/top lEft/Center] <bottom Left>:",
        "[Select polyline/Polygonal/Rectangular/Invert clip] <Rectangular>:",
        "{base} <{}>:",
        // #188 command-line option buttons that are not prose:
        // axis / plane names (chairman kept N/E/S/W-class designations 10-25 13:3x),
        // surface-continuity grades (the armed LOFT prompts print G0/G1 verbatim), and
        // DIMCONTINUE's button, exempted by the chairman 10-25 12:47:32.
        "Continuous",
        "G0",
        "G1",
        "X",
        "XY",
        "XYZ",
        "Y",
        "YZ",
        "Z",
        "ZX",
    ];

    /// #178 guard: a `t!`/`tf!` call whose source argument is a literal must have a
    /// catalog entry. Without one the call renders English in every language and the
    /// parity gate above cannot see it, because no key is missing.
    #[test]
    fn every_literal_macro_source_is_a_catalog_entry() {
        // The continuation rule itself, asserted here so a future edit cannot hand back keys with
        // phantom newline-and-indent runs that no call ever produces.
        assert_eq!(
            literal_macro_sources("t!(\"First \\\n             Second\")"),
            vec![(1, "First Second".to_string())]
        );
        assert_eq!(
            literal_macro_sources("t!(\"A \\\r\n B\")"),
            vec![(1, "A B".to_string())]
        );
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        collect_catalog_guard_files(&root, &mut files);
        files.sort();
        let mut checked = 0_usize;
        let mut missing = Vec::new();
        for file in &files {
            let text = std::fs::read_to_string(file).expect("source file must be readable");
            for (line, source) in literal_macro_sources(&text) {
                checked += 1;
                let decided = locale_catalog::message_attribute(&source).is_some()
                    || STAYS_ENGLISH.contains(&source.as_str())
                    || source.starts_with("SETVAR: ");
                if !decided {
                    missing.push(format!("{}:{line}: {source}", file.display()));
                }
            }
        }
        assert!(
            checked > 5_000,
            "the scanner saw only {checked} macro sources, so it is broken"
        );
        assert!(
            missing.is_empty(),
            "{} literal t!/tf! sources have no catalog entry:\n{}",
            missing.len(),
            missing.join("\n")
        );
    }

    /// #185 guard: five names printed English on the main window in a Chinese
    /// session -- `Drawing1`, `Model`, `Layout1`, `WCS`, `MCP`. None of the three
    /// leak gates can see them: the ink is built from a stored `String` or a bare
    /// `text("MCP")`, so no literal reaches `t!` in the files those gates scan. The
    /// fix lives at the ink sites, which means a regression is someone editing one
    /// of those five files and the only evidence is a screenshot. Pin each site to
    /// its witness (the data still flowing there) and its mapper (the ink mapping it
    /// through), so a reverted site fails here instead of on the chairman's screen.
    #[test]
    fn the_default_names_are_mapped_at_every_ink_site() {
        struct Site {
            file: &'static str,
            witness: &'static str,
            mapper: &'static str,
        }
        let sites = [
            Site {
                file: "src/app/view/viewcube.rs",
                witness: "\"WCS\".to_string()",
                mapper: "app_name_display(value)",
            },
            Site {
                file: "src/ui/statusbar/mod.rs",
                witness: "fn space_tab<'a>(",
                mapper: "app_name_display(&label)",
            },
            Site {
                file: "src/app/document.rs",
                witness: "tab_title: format!(\"Drawing{}\"",
                mapper: "app_name_display(&self.tab_title)",
            },
            Site {
                file: "src/ui/command_line.rs",
                witness: "mcp_status(",
                mapper: "text(t!(\"MCP\"))",
            },
            Site {
                file: "src/app/commands/layerprops.rs",
                witness: "\"WCS\".into()",
                mapper: "app_name_display(&active_name)",
            },
        ];
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut dead = Vec::new();
        for site in &sites {
            let text = std::fs::read_to_string(root.join(site.file))
                .expect("ink-site source must be readable");
            if !text.contains(site.witness) {
                dead.push(format!(
                    "{}: {} no longer flows to this file, so the pinned site moved -- re-measure it",
                    site.file, site.witness
                ));
                continue;
            }
            if !text.contains(site.mapper) {
                dead.push(format!(
                    "{}: {} renders raw English again",
                    site.file, site.witness
                ));
            }
        }
        assert!(
            dead.is_empty(),
            "{} #185 ink sites are unmapped:\n{}",
            dead.len(),
            dead.join("\n")
        );
    }

    /// The mapping itself, in the two shapes only a default name has. Asserted
    /// against the catalog rather than a rendered locale so the test does not depend
    /// on the host language: a deleted arm, or a `Layout{}` template the lookup no
    /// longer uses, shows up here as a name with nothing to land on.
    #[test]
    fn default_names_have_catalog_entries_and_renames_do_not() {
        for template in ["Model", "WCS", "MCP", "Layout{}", "Drawing{}"] {
            assert!(
                locale_catalog::message_attribute(template).is_some(),
                "{template} has no catalog arm, so the name renders English in all 22 locales"
            );
        }
        assert!(is_default_numbered_name("Layout1", "Layout"));
        assert!(is_default_numbered_name("Drawing12", "Drawing"));
        assert!(!is_default_numbered_name("Layout", "Layout"));
        assert!(!is_default_numbered_name("LayoutFoo", "Layout"));
        assert!(!is_default_numbered_name("Model", "Layout"));
        assert_eq!(app_name_display("LayoutFoo"), "LayoutFoo");
        assert_eq!(app_name_display("A3 封面"), "A3 封面");
    }

    /// #182 guard: the ribbon tooltip is built with `t!(tool.label)`, so those strings are
    /// catalog keys carried in a struct field instead of a macro literal. The #178 gate above
    /// only sees literal first arguments, so a label added without a catalog arm renders
    /// English in all 22 languages and no parity test notices.
    #[test]
    fn every_ribbon_tool_label_is_a_catalog_entry() {
        assert_eq!(
            field_string_literals("        label: \"Align Vertical Centers\",", "label"),
            vec![(1, "Align Vertical Centers".to_string())]
        );
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui/ribbon");
        let mut files = Vec::new();
        for entry in std::fs::read_dir(&dir).expect("ribbon source directory must be readable") {
            let path = entry.expect("directory entry").path();
            if path.extension().is_some_and(|name| name == "rs") {
                files.push(path);
            }
        }
        files.sort();
        let mut checked = 0_usize;
        let mut missing = Vec::new();
        for file in &files {
            let text = std::fs::read_to_string(file).expect("source file must be readable");
            for (line, label) in field_string_literals(&text, "label") {
                checked += 1;
                if locale_catalog::message_attribute(&label).is_none() {
                    missing.push(format!("{}:{line}: {label}", file.display()));
                }
            }
        }
        assert!(
            checked >= 30,
            "the scanner saw only {checked} ribbon labels, so it is broken"
        );
        assert!(
            missing.is_empty(),
            "{} ribbon tool labels have no catalog entry and render English in every locale:\n{}",
            missing.len(),
            missing.join("\n")
        );
    }

    /// Every ribbon module's vocabularies are struct fields (`label:`, dropdown
    /// item triples), not `label:` lines inside `src/ui/ribbon`, so the source
    /// scanner above never sees them. The annotate tab shipped
    /// "Reassociate Center Object" as English in all 22 locales for exactly that
    /// reason. Walk the registry the way the ribbon does and assert the catalog
    /// answers for every name a tab can draw.
    #[test]
    fn every_module_ribbon_label_is_a_catalog_entry() {
        fn item_labels(item: &crate::modules::RibbonItem, out: &mut Vec<(&'static str, String)>) {
            use crate::modules::RibbonItem as I;
            match item {
                I::Tool(t) | I::LabeledTool(t) | I::LargeTool(t) => out.push((t.label, "tool".into())),
                I::Dropdown { items, .. } => out.extend(
                    items
                        .iter()
                        .map(|(_, label, _)| (*label, "dropdown item".to_string())),
                ),
                I::LabeledDropdown { label, items, .. }
                | I::LargeDropdown { label, items, .. } => {
                    // An empty face label shows the chosen item's name instead.
                    if !label.is_empty() {
                        out.push((*label, "dropdown face".into()));
                    }
                    out.extend(
                        items
                            .iter()
                            .map(|(_, item_label, _)| (*item_label, "dropdown item".to_string())),
                    );
                }
                I::ToolGrid { columns } => {
                    for tool in columns.iter().flatten() {
                        out.push((tool.label, "grid tool".into()));
                    }
                }
                I::LayerComboGroup { row2, row3 } => {
                    for tool in row2.iter().chain(row3) {
                        out.push((tool.label, "layer row".into()));
                    }
                }
                I::PropertiesGroup { match_prop } => {
                    out.push((match_prop.label, "properties face".into()));
                }
                I::StyleComboGroup { rows, .. } => {
                    for tool in rows.iter().flatten() {
                        out.push((tool.label, "style row".into()));
                    }
                }
            }
        }

        let mut checked = 0_usize;
        let mut missing = Vec::new();
        for module in crate::modules::registry::all_modules() {
            for group in module.ribbon_groups() {
                if locale_catalog::message_attribute(group.title).is_none() {
                    missing.push(format!("group title {:?}", group.title));
                }
                let mut labels = Vec::new();
                for item in &group.tools {
                    item_labels(item, &mut labels);
                }
                for (label, kind) in labels {
                    checked += 1;
                    if locale_catalog::message_attribute(label).is_none() {
                        missing.push(format!("{kind} {label:?} (module {})", module.id()));
                    }
                }
            }
        }

        assert!(
            checked >= 100,
            "the registry walk saw only {checked} labels, so it is broken"
        );
        assert!(
            missing.is_empty(),
            "{} ribbon labels have no catalog entry and render English in every locale:\n{}",
            missing.len(),
            missing.join("\n")
        );
    }


    /// Sinks whose argument is drawn on screen: the command line, a widget, or a display-named field.
    /// Whitespace between the token and `format!(` is ignored by the scanner, so no entry ends in a
    /// space; the char before a match has to be a real boundary.
    const DISPLAY_SINKS: &[&str] = &[
        "push_info(&",
        "push_info(",
        "push_error(&",
        "push_error(",
        "push_error_once(&",
        "push_error_once(",
        "push_warning(&",
        "push_warning(",
        "push_warning_once(&",
        "push_warning_once(",
        "push_output(&",
        "push_output(",
        "push_command(&",
        "push_command(",
        "text(&",
        "text(",
        "set_text(",
        "caption:",
        "helper:",
        "placeholder:",
        "tooltip:",
        "label:",
        "title:",
        "description:",
    ];

    /// Templates at a display-named field whose value is also read back as data: the sheet-set form
    /// title is written into the `.dst` file through `set_prop("Title", …)` and compared with
    /// `c.prop("Name") == Some(…)`, so translating it breaks the round trip. #184 classified them as
    /// data instead; the third field of each entry is the line that has to stay true for that to hold.
    const DUAL_USE_SITES: &[(&str, &str, &str)] = &[(
        "src/app/commands/sheet_set.rs",
        "New Subset ({n})",
        "el.set_prop(\"Title\", f.title.trim());",
    )];

    /// #184 guard: `format!` builds its text before the catalog is ever consulted, so an English
    /// template at a display sink prints English in all 22 locales whatever the catalog holds. #184
    /// rewrote 55 of these to `tf!`; the census that found them matched on the call shape and missed
    /// `format!("automation {op}: {code} — {detail}")`, whose arguments are all inline captures.
    /// This scan keys on the sink instead, and it is what turned that one up.
    #[test]
    fn no_bare_format_at_a_display_sink() {
        // fixtures before the tree: a broken scanner and a clean tree both report zero, so the firing
        // has to be shown here.
        assert_eq!(
            bare_format_at_display_sink("self.command_line.push_info(&format!(\"Kicked {} objects\", n));"),
            vec![(1, "Kicked {} objects".to_string())]
        );
        assert_eq!(
            bare_format_at_display_sink("text(\n    format!(\"Done {} times\", n),\n)"),
            vec![(2, "Done {} times".to_string())]
        );
        // the sink in front is what makes it a screen; without one the same template is internal data
        assert!(bare_format_at_display_sink("let s = format!(\"Kicked {} objects\", n);").is_empty());
        // a sink whose template holds nothing prose-shaped
        assert!(bare_format_at_display_sink("text(format!(\"{} / {}\", a, b));").is_empty());
        // the exemption list is consulted, and a comment is not a call site
        assert!(bare_format_at_display_sink("// text(format!(\"Kicked {} objects\", n));").is_empty());
        assert!(bare_format_at_display_sink("text(format!(\"{} px\", w));").is_empty());

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        collect_catalog_guard_files(&root, &mut files);
        files.sort();
        let mut scanned = 0_usize;
        let mut excluded = 0_usize;
        let mut hits = Vec::new();
        for file in &files {
            let text = std::fs::read_to_string(file).expect("source file must be readable");
            scanned += 1;
            let rel = file
                .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                .unwrap_or(file.as_path())
                .to_string_lossy()
                .replace('\\', "/");
            for (line, template) in bare_format_at_display_sink(&text) {
                let dual = DUAL_USE_SITES
                    .iter()
                    .find(|(site, tpl, _)| rel == *site && *tpl == template.as_str());
                if let Some((_, _, evidence)) = dual {
                    // the exclusion is only as good as the line that proves the value is read back
                    if !text.contains(evidence) {
                        hits.push(format!(
                            "{}:{line}: DUAL-USE evidence `{}` is gone, so `{template}` is no longer data",
                            file.display(),
                            evidence
                        ));
                    } else {
                        excluded += 1;
                    }
                    continue;
                }
                hits.push(format!("{}:{line}: format!(\"{template}\")", file.display()));
            }
        }
        assert!(scanned > 500, "only {scanned} source files were walked, so the scan is broken");
        assert!(
            excluded > 0,
            "the dual-use exclusion branch never fired, so DUAL_USE_SITES is stale"
        );
        assert!(
            hits.is_empty(),
            "{} display sinks still build their text with `format!`, which cannot be translated:\n{}",
            hits.len(),
            hits.join("\n")
        );
    }

    /// #184 guard: `ConstraintKind::label()` feeds `t!(kind.label())` in the viewport echo row, so those
    /// literals reach the screen as a function return -- invisible to #178 (literal arguments only) and
    /// to #182 (struct fields only). A constraint kind added without a catalog arm shows `Colinear` in a
    /// Chinese session, which is how the `{kind:?}` row this replaced leaked.
    #[test]
    fn every_constraint_kind_label_is_a_catalog_entry() {
        assert_eq!(
            label_fn_returns("pub const fn label(self) -> &'static str {\n        Self::A => \"Alpha\",\n    }"),
            vec![(2, "Alpha".to_string())]
        );
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/scene/parametric_constraints.rs");
        let text = std::fs::read_to_string(&path).expect("constraint source must be readable");
        let labels = label_fn_returns(&text);
        assert!(
            labels.len() >= 20,
            "only {} literals parsed out of label(), so the extractor is stale",
            labels.len()
        );
        let missing: Vec<String> = labels
            .iter()
            .filter(|(_, value)| locale_catalog::message_attribute(value).is_none())
            .map(|(line, value)| format!("{}:{line}: {value}", path.display()))
            .collect();
        assert!(
            missing.is_empty(),
            "{} ConstraintKind labels reach a screen with no catalog entry:\n{}",
            missing.len(),
            missing.join("\n")
        );
    }

    /// A literal that appears in two arms makes the second one dead -- `match` takes the first, so the
    /// slots the second points at can never be read in any of the 22 locales, while the batch that added
    /// it believes it shipped a translation. rustc only warns here (`unreachable_pattern`), and a warning
    /// has never stopped a delivery, so the count needs a test of its own.
    #[test]
    fn catalog_has_no_duplicate_literal_arms() {
        let fixture = "\"a\" => Some((\"p\", \"x\")),\n\"a\" => Some((\"q\", \"y\")),\n";
        let arms = catalog_literal_arms(fixture);
        assert_eq!(arms.len(), 2, "fixture arms parsed: the extractor stopped seeing arms");
        assert_eq!(arms[1], (2, "a".to_string()), "fixture line numbering");
        let fired = duplicate_literal_arms(fixture);
        assert_eq!(fired.len(), 1, "the duplicate branch never opened fire");

        let text = include_str!("locale_catalog.rs");
        let counted = catalog_literal_arms(text);
        assert!(
            counted.len() >= 6_000,
            "only {} arms parsed out of the catalog, so the extractor is stale",
            counted.len()
        );
        let dups = duplicate_literal_arms(text);
        assert!(
            dups.is_empty(),
            "{} literal(s) claimed by two catalog arms -- the later one is dead code and its 22 slots \
             are never read:\n{}",
            dups.len(),
            dups.iter().map(|(line, lit)| format!("{line}: {lit}")).collect::<Vec<_>>().join("\n")
        );
    }

    /// Every `"literal" => Some(("parent", "slug")),` arm, with its line number.
    fn catalog_literal_arms(text: &str) -> Vec<(usize, String)> {
        let mut out = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with('"') || !trimmed.contains("=> Some((") {
                continue;
            }
            if let Some(lit) = quoted_literal(trimmed) {
                out.push((index + 1, lit));
            }
        }
        out
    }

    /// The later occurrence of each repeated literal -- the one `match` can never reach.
    fn duplicate_literal_arms(text: &str) -> Vec<(usize, String)> {
        let mut seen: Vec<String> = Vec::new();
        let mut dups = Vec::new();
        for (line, lit) in catalog_literal_arms(text) {
            if seen.iter().any(|existing| existing == &lit) {
                dups.push((line, lit));
            } else {
                seen.push(lit);
            }
        }
        dups
    }

    /// Every locale file has to parse under the same Fluent parser the runtime uses. A header written with
    /// no value and no attributes (`annotate =`) does not merely lose one entry: the parser recovers by
    /// folding the following lines into `Entry::Junk`, so a whole block goes missing while its bytes stay in
    /// the file -- and a missing attribute makes `translate` hand back the English source, the #184 symptom.
    /// Shapes below measured with an out-of-tree decoder (`fluent_syntax::parser::parse`, fluent-syntax
    /// 0.12), because the obvious assumption failed: an attribute at column 0 is *legal* -- it attaches to
    /// the preceding message with no error, which is how lo-LA really writes
    /// `.mleader-specify-landing-distance`. Indentation is not the invariant; the parse is.
    #[test]
    fn every_locale_file_parses_without_junk() {
        assert_eq!(
            parse_health("annotate =\nother = ok\n"),
            (1, 1, 0),
            "the empty-header fixture must be junk with an error, or this guard proves nothing"
        );
        assert_eq!(
            parse_health("annotate = x\n    .slot-a = one\n.slot-b = two\n"),
            (0, 0, 2),
            "a column-0 attribute must stay legal and reach its message, or this guard would condemn the \
             shape lo-LA ships"
        );

        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
        let mut paths: Vec<_> = std::fs::read_dir(&dir)
            .expect("locales directory must exist")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.join("draftflow.ftl").is_file())
            .collect();
        paths.sort();
        let mut junk = 0;
        let mut errors = 0;
        let mut attributes = 0;
        let mut problems = Vec::new();
        for path in &paths {
            let file = path.join("draftflow.ftl");
            let text = std::fs::read_to_string(&file).expect("locale must be readable");
            let (j, e, a) = parse_health(&text);
            attributes += a;
            junk += j;
            errors += e;
            if j != 0 || e != 0 {
                problems.push(format!("{}: junk={j} errors={e}", file.display()));
            }
        }
        assert_eq!(paths.len(), 22, "expected 22 locale directories");
        assert!(
            attributes > 100_000,
            "only {attributes} attributes parsed across {} files, so the check is vacuous",
            paths.len()
        );
        assert!(
            problems.is_empty(),
            "{} locale file(s) do not parse clean ({junk} junk, {errors} errors):\n{}",
            problems.len(),
            problems.join("\n")
        );
    }

    /// A catalog arm is a promise that `translate` will find something. When the attribute is absent from
    /// the active locale the runtime does not fall back to the English source: `i18n_embed::fluent` renders
    /// `No localization for message id: "model" and attribute id: "select-a-solid-edge"` into the sink, so a
    /// gap is worse than the leak it would replace. Parity is read off the same Fluent AST the runtime
    /// builds, never off an indentation assumption -- a `^ {4}\.` line scan reported 59 existing slots
    /// missing (20 locales indent at 3 spaces, fr-FR's insert block at 7, lo-LA puts one at column 0), and
    /// backfilling that verdict would have shadowed real translations with the source string.
    #[test]
    fn every_catalog_arm_has_an_attribute_in_every_locale() {
        let pairs = catalog_arm_pairs(include_str!("locale_catalog.rs"));
        assert!(
            pairs.len() > 6_000,
            "the catalog scan collapsed to {} pairs, so this guard proves nothing",
            pairs.len()
        );

        let (count, fixture) = locale_slots("annotate =\n    .slot-a = one\n.slot-b = two\n   .slot-c = three\n");
        assert_eq!(count, 3, "attributes at 0, 3 and 4 spaces must all attach to their message");
        for slug in ["slot-a", "slot-b", "slot-c"] {
            assert!(
                fixture.contains(&format!("annotate.{slug}")),
                "the fixture dropped {slug}, so the scan is shape-sensitive again"
            );
        }

        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
        let mut paths: Vec<_> = std::fs::read_dir(&dir)
            .expect("locales directory must exist")
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.join("draftflow.ftl").is_file())
            .collect();
        paths.sort();
        let mut offenders = Vec::new();
        let mut attributes = 0;
        for path in &paths {
            let locale = path.file_name().and_then(|name| name.to_str()).unwrap_or("?");
            let text = std::fs::read_to_string(path.join("draftflow.ftl")).expect("locale must be readable");
            let (count, have) = locale_slots(&text);
            attributes += count;
            offenders.extend(
                pairs
                    .iter()
                    .filter(|pair| !have.contains(*pair))
                    .map(|pair| format!("{locale}: {pair}")),
            );
        }
        assert_eq!(paths.len(), 22, "expected 22 locale directories");
        assert!(
            attributes > 100_000,
            "only {attributes} attributes parsed across 22 files, so the check is vacuous"
        );
        assert!(
            offenders.is_empty(),
            "{} catalog arm(s) have no attribute in at least one locale:\n{}",
            offenders.len(),
            offenders.join("\n")
        );
    }

    /// The plate's status line does not go through the source catalog: `SplashStatus::text`
    /// calls `crate::tr!("splash", …)` with the id written out, so the catalog scan above
    /// cannot see it. The key set is read off `src/ui/splash.rs` itself -- a phase added
    /// without 22 translations would otherwise render Fluent's own error on the plate.
    #[test]
    fn every_splash_status_key_has_a_slot_in_all_22_locales() {
        const PREFIX: &str = "crate::tr!(\"splash\", \"";
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut keys = std::collections::BTreeSet::new();
        // The plate's own strings, plus the restart failure the boot module
        // reports when the retry/safe-mode exits cannot spawn a process. Both
        // are `tr!` direct lookups, which the catalog-arm guard cannot see.
        for file in ["src/ui/splash.rs", "src/app/startup.rs"] {
            let source = std::fs::read_to_string(root.join(file))
                .unwrap_or_else(|error| panic!("{file} must be readable: {error}"));
            // The test module quotes keys too; production is what ships.
            let production = source.split("\n#[cfg(test)]").next().unwrap_or(&source);
            let mut rest = production;
            while let Some(at) = rest.find(PREFIX) {
                rest = &rest[at + PREFIX.len()..];
                let end = rest.find('"').expect("a splash key must be closed");
                keys.insert(rest[..end].to_string());
                rest = &rest[end..];
            }
        }
        assert!(
            keys.len() >= 12,
            "only {} splash key(s) found in src/ui/splash.rs + src/app/startup.rs, so this \
             guard proves nothing",
            keys.len()
        );

        let dir = root.join("locales");
        let mut paths: Vec<_> = std::fs::read_dir(&dir)
            .expect("locales directory must exist")
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.join("draftflow.ftl").is_file())
            .collect();
        paths.sort();
        assert_eq!(paths.len(), 22, "expected 22 locale directories");

        let mut offenders = Vec::new();
        for path in &paths {
            let locale = path.file_name().and_then(|name| name.to_str()).unwrap_or("?");
            let text = std::fs::read_to_string(path.join("draftflow.ftl"))
                .expect("locale must be readable");
            let slots = attribute_values(&text, "splash");
            for key in &keys {
                match slots.get(key) {
                    Some(value) if !value.trim().is_empty() => {}
                    Some(_) => offenders.push(format!("{locale}: splash.{key} is empty")),
                    None => offenders.push(format!("{locale}: splash.{key} is missing")),
                }
            }
            // The graphics warning's safe-mode exit is also a direct lookup.
            let gpu = attribute_values(&text, "gpu");
            match gpu.get("safe-mode") {
                Some(value) if !value.trim().is_empty() => {}
                Some(_) => offenders.push(format!("{locale}: gpu.safe-mode is empty")),
                None => offenders.push(format!("{locale}: gpu.safe-mode is missing")),
            }
        }
        assert!(
            offenders.is_empty(),
            "{} splash slot(s) are missing or empty across the 22 locales:\n{}",
            offenders.len(),
            offenders.join("\n")
        );
    }

    /// A message block's attributes with their values. Continuation lines
    /// (indented, not starting with `.`) extend the previous attribute, so a
    /// wrapped translation is not mistaken for an empty slot.
    fn attribute_values(text: &str, message: &str) -> std::collections::HashMap<String, String> {
        let mut out = std::collections::HashMap::new();
        let mut inside = false;
        let mut current: Option<String> = None;
        for line in text.lines() {
            if !inside {
                inside = line.trim_end() == format!("{message} =");
                continue;
            }
            if line.trim().is_empty() {
                continue;
            }
            if !line.starts_with(char::is_whitespace) {
                break;
            }
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix('.') {
                if let Some((name, value)) = rest.split_once('=') {
                    let name = name.trim().to_string();
                    out.insert(name.clone(), value.trim().to_string());
                    current = Some(name);
                    continue;
                }
            }
            if let Some(name) = &current {
                let value = out.entry(name.clone()).or_default();
                if !value.is_empty() {
                    value.push(' ');
                }
                value.push_str(trimmed);
            }
        }
        out
    }

    /// The `("message", "attribute")` pairs the catalog hands `loader().get_attr`, as `message.attribute`.
    fn catalog_arm_pairs(text: &str) -> std::collections::HashSet<String> {
        let mut out = std::collections::HashSet::new();
        for line in text.lines() {
            let Some(at) = line.find("=> Some((\"") else {
                continue;
            };
            let names = quoted_literals(&line[at + 9..], 2);
            if names.len() == 2 {
                out.insert(format!("{}.{}", names[0], names[1]));
            }
        }
        out
    }

    /// The next `want` double-quoted literals, with `\\` escapes resolved the way the catalog writes them.
    fn quoted_literals(text: &str, want: usize) -> Vec<String> {
        let mut out = Vec::new();
        let mut current = String::new();
        let mut inside = false;
        let mut chars = text.chars();
        while let Some(ch) = chars.next() {
            if !inside {
                inside = ch == '"';
            } else if ch == '\\' {
                current.push(chars.next().unwrap_or('\\'));
            } else if ch == '"' {
                inside = false;
                out.push(std::mem::take(&mut current));
                if out.len() == want {
                    break;
                }
            } else {
                current.push(ch);
            }
        }
        out
    }

    /// `(attribute count, {"message.attribute"})` for one FTL source, straight from the parser.
    fn locale_slots(text: &str) -> (usize, std::collections::HashSet<String>) {
        let ast = match fluent_syntax::parser::parse_runtime(text) {
            Ok(ast) => ast,
            Err((ast, _errors)) => ast,
        };
        let mut have = std::collections::HashSet::new();
        for entry in &ast.body {
            if let fluent_syntax::ast::Entry::Message(message) = entry {
                for attribute in &message.attributes {
                    have.insert(format!("{}.{}", message.id.name, attribute.id.name));
                }
            }
        }
        let count = have.len();
        (count, have)
    }

    /// `(junk, errors, attributes)` for one FTL source. `fluent_syntax::Result` carries the partial resource
    /// inside the error, so both halves have to be read or a file can fail with its junk never counted. The
    /// attribute total is read off the same parse: the top level only carries one entry per parent block, so
    /// counting entries would let a file that collapsed to almost nothing pass as healthy.
    fn parse_health(text: &str) -> (usize, usize, usize) {
        let (ast, errors) = match fluent_syntax::parser::parse_runtime(text) {
            Ok(ast) => (ast, 0),
            Err((ast, errs)) => (ast, errs.len()),
        };
        let junk = ast
            .body
            .iter()
            .filter(|entry| matches!(entry, fluent_syntax::ast::Entry::Junk { .. }))
            .count();
        let attributes = ast
            .body
            .iter()
            .map(|entry| match entry {
                fluent_syntax::ast::Entry::Message(message) => message.attributes.len(),
                fluent_syntax::ast::Entry::Term(term) => term.attributes.len(),
                _ => 0,
            })
            .sum();
        (junk, errors, attributes)
    }

    /// The literals `label()` can hand back: the lines between its signature and the next column-0-ish
    /// closing brace, keeping only `=> "…"` arms.
    fn label_fn_returns(text: &str) -> Vec<(usize, String)> {
        let marker = "pub const fn label(self) -> &'static str";
        let mut out = Vec::new();
        let mut started = false;
        for (index, line) in text.lines().enumerate() {
            if !started {
                started = line.contains(marker);
                continue;
            }
            if started && line.trim() == "}" {
                break;
            }
            if let Some(at) = line.find("=> \"") {
                if let Some(value) = quoted_literal(&line[at + 3..]) {
                    out.push((index + 1, value));
                }
            }
        }
        out
    }

    /// A `format!(` whose nearest non-whitespace token in front is a display sink, with the template
    /// that would reach the screen. Exempted exactly as #178 exempts literals: `STAYS_ENGLISH` and the
    /// `SETVAR: ` prefix.
    fn bare_format_at_display_sink(text: &str) -> Vec<(usize, String)> {
        let chars: Vec<char> = text.chars().collect();
        let needle: Vec<char> = "format!(".chars().collect();
        let mut out = Vec::new();
        let mut i = 0_usize;
        while i + needle.len() <= chars.len() {
            if !chars[i..].starts_with(needle.as_slice()) {
                i += 1;
                continue;
            }
            let open = i;
            i += needle.len();
            // the up-to-24 non-blank characters in front, oldest first. Whitespace is dropped rather
            // than treated as a stop, because a field reads `title: format!(…)` and a call reads
            // `self.command_line\n    .push_error(&format!(…)` -- both have to reach their sink token.
            let mut back = open as isize - 1;
            let mut window = String::new();
            while back >= 0 && window.chars().count() < 24 {
                let c = chars[back as usize];
                if !c.is_whitespace() {
                    window.insert(0, c);
                }
                back -= 1;
            }
            let sinked = DISPLAY_SINKS.iter().any(|sink| {
                if !window.ends_with(*sink) {
                    return false;
                }
                // the character in front of the sink token, when there is one: `push_info(` may not be
                // the tail of a longer name (`my_push_info(`) or of a closed string.
                let at = window.len() - sink.len();
                if at == 0 {
                    return true;
                }
                let b = window.as_bytes()[at - 1];
                !(b.is_ascii_alphanumeric() || b == b'_' || b == b'"')
            });
            if !sinked {
                continue;
            }
            let mut start = open;
            while start > 0 && chars[start - 1] != '\n' {
                start -= 1;
            }
            let head: String = chars[start..open].iter().collect();
            if head.trim_start().starts_with("//") || head.trim_start().starts_with('!') {
                continue;
            }
            let line = 1 + chars[..open].iter().filter(|c| **c == '\n').count();
            let mut j = i;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if chars.get(j) != Some(&'"') {
                continue; // a raw string or a variable template: not the shape this guard demands
            }
            let Some(template) = quoted_literal(&chars[j..].iter().collect::<String>()) else {
                continue;
            };
            let mut prose = String::new();
            let mut in_group = false;
            for c in template.chars() {
                if c == '{' {
                    in_group = true;
                } else if c == '}' {
                    in_group = false;
                } else if !in_group {
                    prose.push(c);
                }
            }
            // two adjacent letters is the shape of a word; a unit is one letter against digits
            let has_word = prose.as_bytes().windows(2).any(|w| {
                (w[0] as char).is_ascii_alphabetic() && (w[1] as char).is_ascii_alphabetic()
            });
            if !has_word || STAYS_ENGLISH.contains(&template.as_str()) || template.starts_with("SETVAR: ") {
                continue;
            }
            out.push((line, template));
        }
        out
    }

    /// The decoded body of a string literal that starts at the beginning of `text`.
    fn quoted_literal(text: &str) -> Option<String> {
        let mut out = String::new();
        let mut chars = text.chars();
        if chars.next() != Some('"') {
            return None;
        }
        while let Some(c) = chars.next() {
            match c {
                '"' => return Some(out),
                '\\' => match chars.next() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    Some(other) => out.push(other),
                    None => return None,
                },
                other => out.push(other),
            }
        }
        None
    }

    /// Single-line `field: "literal",` pairs, escapes honoured, raw strings refused.
    fn field_string_literals(text: &str, field: &str) -> Vec<(usize, String)> {
        let mut out = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let prefix = format!("{field}: ");
            let Some(at) = line.find(&prefix) else { continue };
            let rest = &line[at + prefix.len()..];
            let Some(body) = rest.strip_prefix('"') else {
                assert!(
                    !rest.starts_with('r'),
                    "raw string at {field} is not handled by this scanner"
                );
                continue;
            };
            let mut value = String::new();
            let mut chars = body.chars();
            let mut closed = false;
            while let Some(c) = chars.next() {
                match c {
                    '\\' => value.push(chars.next().unwrap_or('\\')),
                    '"' => {
                        closed = true;
                        break;
                    }
                    other => value.push(other),
                }
            }
            if closed {
                out.push((index + 1, value));
            }
        }
        out
    }

    fn collect_catalog_guard_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("source directory must be readable") {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                collect_catalog_guard_files(&path, out);
            } else if path.extension().is_some_and(|name| name == "rs")
                // The catalog is the table itself; this module holds the patterns to match.
                && !path.ends_with("locale_catalog.rs")
                && !path.ends_with("i18n.rs")
            {
                out.push(path);
            }
        }
    }

    /// Every string literal appearing as the first argument of a `t!` or `tf!` call,
    /// decoded to the runtime value the catalog matches against, with the line it starts on.
    fn literal_macro_sources(text: &str) -> Vec<(usize, String)> {
        let chars: Vec<char> = text.chars().collect();
        let mut out = Vec::new();
        let mut index = 0;
        let mut line = 1_usize;
        let mut walked = 0_usize;
        while index + 4 <= chars.len() {
            let long = chars[index] == 't' && chars[index + 1] == 'f' && chars[index + 2] == '!' && chars[index + 3] == '(';
            let short = chars[index] == 't' && chars[index + 1] == '!' && chars[index + 2] == '(';
            if !(long || short) {
                index += 1;
                continue;
            }
            let after = index + if long { 4 } else { 3 };
            if index > 0 && (chars[index - 1].is_alphanumeric() || chars[index - 1] == '_') {
                index = after;
                continue;
            }
            let mut cursor = after;
            while cursor < chars.len() && chars[cursor].is_whitespace() {
                cursor += 1;
            }
            if cursor >= chars.len() || chars[cursor] != '"' {
                index = after;
                continue;
            }
            let mut value = String::new();
            cursor += 1;
            let mut closed = false;
            while cursor < chars.len() {
                let current = chars[cursor];
                if current == '"' {
                    closed = true;
                    break;
                }
                if current == '\\' {
                    let Some(&next) = chars.get(cursor + 1) else {
                        break;
                    };
                    match next {
                        'n' => value.push('\n'),
                        'r' => value.push('\r'),
                        't' => value.push('\t'),
                        'u' => {
                            let (decoded, used) = decode_unicode_escape(&chars[cursor + 2..]);
                            value.push(decoded);
                            cursor += used;
                        }
                        // A backslash just before a newline is rustc's line continuation: the
                        // newline and the whole run of whitespace around it are dropped, so the
                        // runtime value is the two fragments glued with no separator. A guard that
                        // keeps them would look for a catalog key that no call ever produces.
                        '\n' | '\r' | ' ' | '\t' => {
                            let mut jump = cursor + 1;
                            let mut line_break = false;
                            while jump < chars.len() && chars[jump].is_whitespace() {
                                line_break |= matches!(chars[jump], '\n' | '\r');
                                jump += 1;
                            }
                            if !line_break {
                                value.push(next);
                                cursor += 2;
                                continue;
                            }
                            cursor = jump;
                            continue;
                        }
                        other => value.push(other),
                    }
                    cursor += 2;
                } else {
                    value.push(current);
                    cursor += 1;
                }
            }
            if !closed {
                index = after;
                continue;
            }
            while walked < index {
                if chars[walked] == '\n' {
                    line += 1;
                }
                walked += 1;
            }
            out.push((line, value));
            index = cursor;
        }
        out
    }

    /// `\u{XXXXXX}` — the digits start after the brace has been checked here.
    fn decode_unicode_escape(rest: &[char]) -> (char, usize) {
        if rest.first() != Some(&'{') {
            return ('u', 0);
        }
        let end = rest.iter().position(|&c| c == '}').unwrap_or(0);
        let digits: String = rest[1..end].iter().collect();
        let used = end + 1;
        match u32::from_str_radix(&digits, 16).ok().and_then(char::from_u32) {
            Some(decoded) => (decoded, used),
            None => ('u', 0),
        }
    }

    /// Every literal given as a `CmdOption` button label, with its line and the char offset
    /// of the opening quote.
    /// `CmdOption::new`/`enter` run `t!` on their *parameter*, so #178's literal scanner is
    /// blind here: this is the only thing that can see the fifth display channel (#188).
    fn cmd_option_labels(text: &str) -> Vec<(usize, usize, String)> {
        let chars: Vec<char> = text.chars().collect();
        let mut out = Vec::new();
        let mut index = 0_usize;
        while index + 11 <= chars.len() {
            if !chars_at(&chars, index, "CmdOption::") {
                index += 1;
                continue;
            }
            let tail = index + 11;
            let method = if chars_at(&chars, tail, "new(") {
                4
            } else if chars_at(&chars, tail, "enter(") {
                6
            } else {
                index = tail;
                continue;
            };
            let mut cursor = tail + method;
            while chars.get(cursor).is_some_and(|c| c.is_whitespace()) {
                cursor += 1;
            }
            let Some(value) = read_rust_string_literal(&chars, cursor) else {
                index = cursor;
                continue;
            };
            let line = chars[..cursor].iter().filter(|&&c| c == '\n').count() + 1;
            out.push((line, cursor, value));
            index = cursor;
        }
        out
    }

    fn chars_at(chars: &[char], at: usize, needle: &str) -> bool {
        needle
            .chars()
            .enumerate()
            .all(|(offset, c)| chars.get(at + offset) == Some(&c))
    }

    /// Decode one Rust string literal, applying escapes and `\`+newline continuation.
    /// Returns `None` when the opening quote is missing or the literal never closes.
    fn read_rust_string_literal(chars: &[char], at: usize) -> Option<String> {
        if chars.get(at) != Some(&'"') {
            return None;
        }
        let mut value = String::new();
        let mut cursor = at + 1;
        loop {
            let current = *chars.get(cursor)?;
            if current == '"' {
                return Some(value);
            }
            if current == '\\' {
                let next = *chars.get(cursor + 1)?;
                match next {
                    'n' => value.push('\n'),
                    'r' => value.push('\r'),
                    't' => value.push('\t'),
                    'u' => {
                        let (decoded, used) = decode_unicode_escape(&chars[cursor + 2..]);
                        value.push(decoded);
                        cursor += used;
                    }
                    other if other.is_whitespace() => {
                        let mut jump = cursor + 1;
                        let mut line_break = false;
                        while let Some(&w) = chars.get(jump) {
                            if !w.is_whitespace() {
                                break;
                            }
                            line_break |= matches!(w, '\n' | '\r');
                            jump += 1;
                        }
                        if !line_break {
                            value.push(other);
                            cursor += 2;
                            continue;
                        }
                        cursor = jump;
                        continue;
                    }
                    other => value.push(other),
                }
                cursor += 2;
            } else {
                value.push(current);
                cursor += 1;
            }
        }
    }

    /// DEF-194 (input half). `every_cmd_option_label_is_decided` proves the chip's *label* can be
    /// localized; that is only half the contract, because a label that is translated on screen but not
    /// accepted on the keyboard is a worse defect than an English one -- the person reads 关闭, types it,
    /// and gets "Invalid". This pins the other half: the reverse lookup resolves a typed label to its
    /// keyword, refuses to guess when two chips share one label, ignores the international `_` prefix and
    /// the finish-chip's empty keyword, and the dispatch funnel still calls it before any routing.
    #[test]
    fn a_chip_label_typed_back_reaches_its_keyword() {
        use crate::command::{localized_keyword, CmdOption};
        let chip = |label: &str, keyword: &str| CmdOption {
            label: label.to_string(),
            keyword: keyword.to_string(),
        };
        let one = vec![chip("关闭", "CLOSE"), chip("Above", "ABOVE")];
        assert_eq!(localized_keyword(&one, "关闭").as_deref(), Some("CLOSE"));
        assert_eq!(
            localized_keyword(&one, "_above").as_deref(),
            Some("ABOVE"),
            "the international form must still reach the same chip"
        );
        assert_eq!(localized_keyword(&one, "  ").as_deref(), None);
        assert_eq!(
            localized_keyword(&one, "no such chip").as_deref(),
            None,
            "an unknown token must be dispatched untouched"
        );
        let enter = vec![chip("关闭", "")];
        assert_eq!(
            localized_keyword(&enter, "关闭").as_deref(),
            None,
            "a finish chip submits with Enter; rewriting to an empty keyword would eat the token"
        );
        let shared = vec![chip("关闭", "CLOSE"), chip("关闭", "OFF")];
        assert_eq!(
            localized_keyword(&shared, "关闭").as_deref(),
            None,
            "two chips read alike: never guess"
        );

        let body = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/app/commands/mod.rs"),
        )
        .expect("dispatch file must be readable");
        let at = body
            .find("fn dispatch_command_inner")
            .expect("the funnel must exist");
        let window = &body[at..at + 2000];
        let call = window
            .find("crate::command::localized_keyword(&c.options(), cmd)")
            .expect("the funnel must reverse a typed chip label before routing it");
        let alias = window
            .find("resolve_alias(")
            .expect("alias resolution is the first routing step");
        assert!(
            call < alias,
            "the keyword must be settled before any routing: call at {call}, alias at {alias}"
        );
    }

    /// #194 guard, the other end of the same contract: the funnel in `dispatch_command_inner` only sees
    /// the *first* line of a command. Once a command is active, the text goes to
    /// `TabState::on_text_input` of the step, and there the reverse lookup was missing -- a prompt that
    /// read `[关闭 / 以上]` accepted only CLOSE/ABOVE. This pins that both step sites consult
    /// `step_keyword` and that no step call takes the raw text.
    #[test]
    fn a_chip_label_typed_at_an_active_step_reaches_its_keyword() {
        let body = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/app/update/command.rs"),
        )
        .expect("step handling file must be readable");
        let defined_at = body
            .find("fn step_keyword(")
            .expect("the step reverse map must exist");
        assert!(
            body[defined_at..]
                .find("crate::command::localized_keyword(&cmd.options(), text)")
                .is_some(),
            "step_keyword must be the same lookup the funnel uses, not a second implementation"
        );
        let calls: Vec<&str> = body
            .lines()
            .filter(|l| l.contains(".on_text_input("))
            .collect();
        assert!(
            calls.len() >= 2,
            "measured fewer step calls than the two known sites: {calls:?}"
        );
        for line in &calls {
            assert!(
                line.contains("on_text_input(&token)"),
                "a step must receive the resolved keyword, never the typed label: {line}"
            );
        }
        assert!(
            !body.contains("on_text_input(&text)"),
            "raw text must not be handed to a step"
        );
    }

    /// #188 guard: the command-line option buttons. A label with no catalog entry and no
    /// exemption prints English on a button the chairman can click, and none of the other
    /// four leak gates can see it, because the `t!` argument is a function parameter.
    #[test]
    fn every_cmd_option_label_is_decided() {
        let wrapped = "CmdOption::new(\"A \\\n                B\", \"X\")";
        let quote_at = wrapped[..wrapped.find("\"A").unwrap()].chars().count();
        assert_eq!(cmd_option_labels(wrapped), vec![(1, quote_at, "A B".to_string())]);
        assert!(
            cmd_option_labels("CmdOption::new(name, \"X\")").is_empty(),
            "a variable label is not measurable here"
        );
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        collect_catalog_guard_files(&root, &mut files);
        files.sort();
        let mut checked = 0_usize;
        let mut missing = Vec::new();
        for file in &files {
            let text = std::fs::read_to_string(file).expect("source file must be readable");
            // A label after `#[cfg(test)]` is an assert fixture, not ink on the screen.
            let tests_at = text.find("#[cfg(test)]").map(|byte| text[..byte].chars().count());
            for (line, offset, label) in cmd_option_labels(&text) {
                if tests_at.is_some_and(|at| offset >= at) {
                    continue;
                }
                checked += 1;
                if locale_catalog::message_attribute(&label).is_none()
                    && !STAYS_ENGLISH.contains(&label.as_str())
                {
                    missing.push(format!("{}:{line}: {label}", file.display()));
                }
            }
        }
        assert!(
            checked > 600,
            "the scanner saw only {checked} button labels, so it is broken"
        );
        assert!(
            missing.is_empty(),
            "{} CmdOption button labels reach the screen undecided:\n{}",
            missing.len(),
            missing.join("\n")
        );
        // The skip branch must fire on the real fixture it was written for, otherwise the
        // guard is untested and one more `enter("<0.5000>")` style site hides behind it.
        let fixture = std::fs::read_to_string(root.join("ui/popup/context_menu.rs"))
            .expect("context menu source must be readable");
        let tests_at = fixture
            .find("#[cfg(test)]")
            .map(|byte| fixture[..byte].chars().count())
            .expect("context_menu.rs must hold a test module");
        let labels = cmd_option_labels(&fixture);
        assert!(
            labels.iter().any(|(_, _, value)| value == "<0.5000>"),
            "the Enter-row fixture label must still be scannable"
        );
        assert!(
            labels.iter().all(|(_, offset, _)| *offset >= tests_at),
            "every CmdOption in context_menu.rs must sit in the test module"
        );
    }

    /// #190: the sixth display channel. A prompt function hands its result to the command line,
    /// which translates it at the sink (`push_output`/`push_info` run `t!(msg)` on the finished
    /// string), so nothing at the call site is a `t!`/`tf!` argument and the #178, #181, #184 and
    /// #188 gates are all blind here. Two ways to put English on the screen: prose with no catalog
    /// entry, and prose with a `{}` hole still wrapped in `format!` -- the sink then translates a
    /// string whose values are already baked in, which matches no arm even after the arm is added.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct PromptSite {
        line: usize,
        macro_name: Option<String>,
        via_const: Option<String>,
        value: String,
    }

    /// The same prose test the #184 gate uses, so this guard and that one cannot disagree about
    /// what counts as a sentence: a space, three letters in a row, and either a colon or three
    /// words. `"{:.4}"` and `"CCW"` fail it and are not ink the chairman reads as untranslated.
    fn is_prompt_prose(value: &str) -> bool {
        value.chars().any(|c| c.is_whitespace())
            && value.as_bytes().windows(3).any(|w| {
                w.iter().all(|b| (*b as char).is_ascii_alphabetic())
            })
            && (value.contains(':') || value.split_whitespace().count() >= 3)
    }

    /// Nothing a translator could write: the string is placeholders and chip listings only. `[..]`
    /// is the option listing the chip row renders, `<..>` the default echo, `{..}` a hole, and a
    /// token without a lowercase letter is either a separator or the command name the sink already
    /// printed (`BOUNDARY  {} <{}>:`, `[Select polyline/Polygonal] <Rectangular>:`). Those seven
    /// rows are the #190 plan's exemptions, and they are exempt because of their *shape*, not
    /// because a table says so -- so the skip stays true when an eighth row of the same shape
    /// arrives, and dies loudly if the stripping itself breaks.
    fn prompt_is_placeholders_only(value: &str) -> bool {
        let mut residue = String::new();
        let mut depth = 0_usize;
        for ch in value.chars() {
            match ch {
                '[' | '<' | '{' => depth += 1,
                ']' | '>' | '}' => depth = depth.saturating_sub(1),
                _ if depth == 0 => residue.push(ch),
                _ => {}
            }
        }
        residue
            .split_whitespace()
            .all(|tok| tok.chars().filter(|c| c.is_alphabetic()).all(|c| !c.is_lowercase()))
    }

    /// Braces the sink cannot fill. `%{name}` belongs to `t!`/`translate_args`, which substitutes
    /// by name, so a prompt written entirely in that protocol is already decided and must not be
    /// counted as a hole.
    fn positional_holes(value: &str) -> usize {
        let chars: Vec<char> = value.chars().collect();
        (1..chars.len())
            .filter(|i| chars[*i] == '{' && chars[i - 1] != '%')
            .count()
    }

    fn is_promptish(name: &str) -> bool {
        name == "prompt"
            || name.ends_with("_prompt")
            || name.ends_with("_prompts")
            || name.starts_with("prompt_")
    }

    fn word_before(chars: &[char], at: usize) -> bool {
        at == 0 || !(chars[at - 1].is_alphanumeric() || chars[at - 1] == '_')
    }

    /// A raw string starting at `at`: `r"…"`, `r#"…"#`, `r##"…"##`. Its content is the runtime
    /// value verbatim -- no escapes -- so it is the arm key as written.
    fn raw_string_at(chars: &[char], at: usize) -> Option<(String, usize)> {
        if chars.get(at) != Some(&'r') || !word_before(chars, at) {
            return None;
        }
        let mut j = at + 1;
        let mut hashes = 0_usize;
        while chars.get(j) == Some(&'#') {
            hashes += 1;
            j += 1;
        }
        if chars.get(j) != Some(&'"') {
            return None;
        }
        let open = j + 1;
        let mut k = open;
        while k < chars.len() {
            if chars[k] == '"' {
                let mut matched = 0_usize;
                while matched < hashes && chars.get(k + 1 + matched) == Some(&'#') {
                    matched += 1;
                }
                if matched == hashes {
                    return Some((chars[open..k].iter().collect(), k + 1 + hashes));
                }
            }
            k += 1;
        }
        None
    }

    /// One comment, string, char literal or lifetime starting at `at`, returning the offset past
    /// it; ordinary code advances by one. Without this, a quote inside a `//` note reads as a
    /// template and the guard invents catalog entries nobody calls.
    fn skip_trivia(chars: &[char], at: usize) -> usize {
        if chars_at(chars, at, "//") {
            let mut j = at + 2;
            while j < chars.len() && chars[j] != '\n' {
                j += 1;
            }
            return j;
        }
        if chars_at(chars, at, "/*") {
            let mut depth = 1_usize;
            let mut j = at + 2;
            while j < chars.len() && depth > 0 {
                if chars_at(chars, j, "/*") {
                    depth += 1;
                    j += 2;
                } else if chars_at(chars, j, "*/") {
                    depth -= 1;
                    j += 2;
                } else {
                    j += 1;
                }
            }
            return j;
        }
        if let Some((_, end)) = raw_string_at(chars, at) {
            return end;
        }
        if chars.get(at) == Some(&'"') {
            let mut j = at + 1;
            while j < chars.len() {
                match chars[j] {
                    '\\' => j += 2,
                    '"' => return j + 1,
                    _ => j += 1,
                }
            }
            return j;
        }
        if chars.get(at) == Some(&'\'') {
            // `'a` is a lifetime, `'x'` and `'\''` are chars; only the second one closes.
            let mut j = at + 1;
            while j < chars.len() && j <= at + 4 {
                if chars[j] == '\\' {
                    j += 2;
                    continue;
                }
                if chars[j] == '\'' {
                    return j + 1;
                }
                j += 1;
            }
            return at + 1;
        }
        at + 1
    }

    /// The decoded value of the literal at `at` and the offset past it.
    fn literal_at(chars: &[char], at: usize) -> Option<(String, usize)> {
        if let Some(raw) = raw_string_at(chars, at) {
            return Some(raw);
        }
        let value = read_rust_string_literal(chars, at)?;
        let mut j = at + 1;
        while j < chars.len() {
            match chars[j] {
                '\\' => j += 2,
                '"' => {
                    j += 1;
                    break;
                }
                _ => j += 1,
            }
        }
        Some((value, j))
    }

    /// The macro an argument belongs to, walked back from its opening offset: whitespace, then
    /// `(`, then `!`, then the name. A comma or `=` behind the quote means the literal is not that
    /// macro's template, which is what separates a prompt from an option list.
    fn macro_of_first_arg(chars: &[char], at: usize) -> Option<String> {
        let mut i = at;
        while i > 0 && chars.get(i - 1).is_some_and(|c| c.is_whitespace()) {
            i -= 1;
        }
        if i == 0 {
            return None;
        }
        i -= 1;
        if chars.get(i) != Some(&'(') {
            return None;
        }
        if i == 0 {
            return None;
        }
        i -= 1;
        if chars.get(i) != Some(&'!') {
            return None;
        }
        let mut end = i;
        while end > 0 {
            end -= 1;
            if !(chars[end].is_alphanumeric() || chars[end] == '_') {
                end += 1;
                break;
            }
        }
        let name: String = chars[end..i].iter().collect();
        if name.is_empty() {
            None
        } else {
            Some(name)
        }
    }

    fn skip_ws(chars: &[char], at: usize) -> usize {
        let mut j = at;
        while chars.get(j).is_some_and(|c| c.is_whitespace()) {
            j += 1;
        }
        j
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum ConstInit {
        Literal(String),
        Alias(String),
    }

    /// Identifier runs inside a span, comments and literals skipped.
    fn ident_runs(chars: &[char], from: usize, to: usize) -> Vec<(usize, usize, String)> {
        let mut out = Vec::new();
        let mut i = from;
        while i < to {
            let c = chars[i];
            if c == '/' || c == '"' || c == '\'' || (c == 'r' && raw_string_at(chars, i).is_some()) {
                i = skip_trivia(chars, i);
                continue;
            }
            if c.is_alphabetic() || c == '_' {
                let start = i;
                while i < to && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                out.push((start, i, chars[start..i].iter().collect()));
                continue;
            }
            i += 1;
        }
        out
    }

    /// Same-file `const`/`static` items with a SCREAMING name: the initializer's literals joined
    /// (that is what `concat!("a", "b")` holds at runtime), or an alias to another item.
    fn declared_consts(chars: &[char], limit: usize) -> Vec<(String, ConstInit)> {
        let mut out = Vec::new();
        let mut i = 0_usize;
        while i + 6 < limit {
            if !chars_at(chars, i, "const ") || !word_before(chars, i) {
                i = skip_trivia(chars, i);
                continue;
            }
            let name_at = skip_ws(chars, i + 6);
            let mut j = name_at;
            while j < limit && (chars[j].is_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            let name: String = chars[name_at..j].iter().collect();
            if name.is_empty() || !name.chars().all(|c| !c.is_lowercase()) || name.len() < 3 {
                i = j.max(i + 1);
                continue;
            }
            // to the initializer's `=`, then to its `;` at bracket depth zero
            let mut k = j;
            while k < limit {
                if chars[k] == '=' && chars.get(k + 1) != Some(&'=') && !matches!(chars[k - 1], '=' | '!' | '<' | '>') {
                    break;
                }
                if chars[k] == ';' {
                    break;
                }
                k = skip_trivia(chars, k);
            }
            if chars.get(k) != Some(&'=') {
                i = k.max(i + 1);
                continue;
            }
            let rhs_at = k + 1;
            let mut depth = 0_isize;
            let mut m = rhs_at;
            while m < limit {
                let c = chars[m];
                if matches!(c, '(' | '[' | '{') {
                    depth += 1;
                } else if matches!(c, ')' | ']' | '}') {
                    depth -= 1;
                } else if c == ';' && depth <= 0 {
                    break;
                }
                m = skip_trivia(chars, m);
            }
            let rhs = &chars[rhs_at..m];
            let lits = literals_in(rhs, 0, rhs.len());
            let init = if lits.is_empty() {
                ident_runs(rhs, 0, rhs.len())
                    .into_iter()
                    .map(|(_, _, name)| name)
                    .find(|name| name.len() >= 3 && name.chars().all(|c| !c.is_lowercase()))
                    .map(ConstInit::Alias)
            } else {
                Some(ConstInit::Literal(
                    lits.into_iter().map(|(_, value)| value).collect(),
                ))
            };
            if let Some(init) = init {
                out.push((name, init));
            }
            i = m.max(i + 1);
        }
        out
    }

    /// Every string literal in a span, with the offset of its opening quote.
    fn literals_in(chars: &[char], from: usize, to: usize) -> Vec<(usize, String)> {
        let mut out = Vec::new();
        let mut i = from;
        while i < to {
            let c = chars[i];
            if c == '"' || (c == 'r' && raw_string_at(chars, i).is_some()) {
                if let Some((value, end)) = literal_at(chars, i) {
                    out.push((i, value));
                    i = end;
                    continue;
                }
            }
            i = skip_trivia(chars, i);
        }
        out
    }

    /// Follow an alias chain to the literal it ends on. Five hops is more than the batch uses and
    /// less than anything that could loop: `const A: &str = B;` names another item, not prose.
    fn const_text(consts: &[(String, ConstInit)], name: &str) -> Option<String> {
        let mut current = name.to_string();
        for _ in 0..5 {
            match consts.iter().find(|(found, _)| found == &current)?.1.clone() {
                ConstInit::Literal(value) => return Some(value),
                ConstInit::Alias(next) => current = next,
            }
        }
        None
    }

    /// Body spans of every `fn` in the file.
    struct FnSpan {
        name: String,
        start: usize,
        end: usize,
    }

    fn fn_spans(chars: &[char], limit: usize) -> Vec<FnSpan> {
        let mut out = Vec::new();
        let mut i = 0_usize;
        while i + 3 < limit {
            if !chars_at(chars, i, "fn ") || !word_before(chars, i) {
                i = skip_trivia(chars, i);
                continue;
            }
            let name_at = skip_ws(chars, i + 3);
            let mut j = name_at;
            while j < limit && (chars[j].is_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            let name: String = chars[name_at..j].iter().collect();
            // the signature ends at the body's `{`; a `;` first means a trait declaration with no body
            let mut k = j;
            while k < limit && chars[k] != '{' && chars[k] != ';' && chars[k] != '}' {
                k = skip_trivia(chars, k);
            }
            if chars.get(k) != Some(&'{') {
                i = k.max(i + 1);
                continue;
            }
            let depth_start = k;
            let mut depth = 0_isize;
            let mut m = k;
            while m < limit {
                let c = chars[m];
                if matches!(c, '(' | '[' | '{') {
                    depth += 1;
                } else if matches!(c, ')' | ']' | '}') {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                m = skip_trivia(chars, m);
            }
            out.push(FnSpan {
                name,
                start: depth_start + 1,
                end: m.min(limit),
            });
            i = depth_start + 1;
        }
        out
    }

    /// The brace span of every item marked `#[cfg(test)]`. Cutting a file at the first marker is
    /// what the count probe did, and it swallowed 27 real prompt functions in 9 files: in
    /// `command.rs` the `working_plane_tests` module opens at line 146 and twelve production
    /// `fn prompt` impls follow it. Only the marked item's own body is test material.
    fn cfg_test_spans(chars: &[char]) -> Vec<(usize, usize)> {
        let marker = "#[cfg(test)]";
        let mut out = Vec::new();
        let mut i = 0_usize;
        while i + marker.chars().count() <= chars.len() {
            if !chars_at(chars, i, marker) {
                i = skip_trivia(chars, i);
                continue;
            }
            let mut k = skip_ws(chars, i + marker.chars().count());
            while k < chars.len() && chars[k] != '{' && chars[k] != ';' && chars[k] != '}' {
                k = skip_trivia(chars, k);
            }
            if chars.get(k) != Some(&'{') {
                i = k.max(i + marker.chars().count());
                continue;
            }
            let open = k;
            let mut depth = 0_isize;
            let mut m = open;
            while m < chars.len() {
                let c = chars[m];
                if matches!(c, '(' | '[' | '{') {
                    depth += 1;
                } else if matches!(c, ')' | ']' | '}') {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                m = skip_trivia(chars, m);
            }
            out.push((open, m.min(chars.len())));
            i = (m + 1).max(i + 1);
        }
        out
    }

    fn inside_spans(spans: &[(usize, usize)], at: usize) -> bool {
        spans.iter().any(|(start, end)| at >= *start && at < *end)
    }

    /// Every literal a prompt function can hand to the command line: the literals in each promptish
    /// body, the bodies reached through another `*prompt*` call, and the same-file `const` aliases
    /// those bodies read. `attdef.rs` builds its attribute-mode prompt from `MODES_PROMPT`, which
    /// is why a scan of `fn prompt` bodies alone counted 253 rows where the channel holds 334.
    fn prompt_sites(text: &str) -> Vec<PromptSite> {
        let chars: Vec<char> = text.chars().collect();
        let tests = cfg_test_spans(&chars);
        let in_tests = |at: usize| inside_spans(&tests, at);
        let fns = fn_spans(&chars, chars.len());
        let consts = declared_consts(&chars, chars.len());
        let mut out: Vec<PromptSite> = Vec::new();
        let mut queue: Vec<usize> = fns
            .iter()
            .enumerate()
            .filter(|(_, span)| is_promptish(&span.name))
            .map(|(index, _)| index)
            .collect();
        let mut seen = vec![false; fns.len()];
        let mut cursor = 0_usize;
        while cursor < queue.len() {
            let index = queue[cursor];
            cursor += 1;
            if seen[index] {
                continue;
            }
            seen[index] = true;
            let span = &fns[index];
            for (at, value) in literals_in(&chars, span.start, span.end) {
                if value.trim().is_empty() || in_tests(at) {
                    continue;
                }
                out.push(PromptSite {
                    line: 1 + chars[..at].iter().filter(|c| **c == '\n').count(),
                    macro_name: macro_of_first_arg(&chars, at),
                    via_const: None,
                    value,
                });
            }
            for (start, end, name) in ident_runs(&chars, span.start, span.end) {
                if name.contains("prompt") && chars.get(skip_ws(&chars, end)) == Some(&'(') {
                    if let Some(next) = fns.iter().position(|other| other.name == name) {
                        queue.push(next);
                    }
                    continue;
                }
                if name.len() < 3 || !name.chars().all(|c| !c.is_lowercase()) || in_tests(start) {
                    continue;
                }
                if let Some(value) = const_text(&consts, &name) {
                    out.push(PromptSite {
                        line: 1 + chars[..start].iter().filter(|c| **c == '\n').count(),
                        macro_name: macro_of_first_arg(&chars, start),
                        via_const: Some(name),
                        value,
                    });
                }
            }
        }
        out
    }

    /// #190 guard: the command line echoes whatever a prompt function returns, and the only place
    /// that string is translated is the sink. A prose prompt with no catalog entry is English in
    /// all 22 locales; the same prose with a `{}` hole left inside `format!` is English even after
    /// the entry exists, because the sink receives the finished string with the values baked in.
    #[test]
    fn every_prompt_literal_is_a_catalog_entry() {
        let sample = "const MODES_PROMPT: &str =\n    \"Enter an option [Invisible/Constant] <done>:\";\n\
            fn prompt(&self) -> String { format!(\"Pick {} corners\", n) }\n\
            fn width_prompt() -> String { MODES_PROMPT.to_string() }\n\
            fn unrelated() -> String { \"Nothing measured here\".to_string() }\n";
        let sites = prompt_sites(sample);
        let values: Vec<&str> = sites.iter().map(|site| site.value.as_str()).collect();
        assert!(
            values.contains(&"Pick {} corners"),
            "the template inside a prompt body was lost: {values:?}"
        );
        assert!(
            values.contains(&"Enter an option [Invisible/Constant] <done>:"),
            "the const alias reached through a promptish sibling was lost: {values:?}"
        );
        assert!(
            !values.contains(&"Nothing measured here"),
            "a body that is not promptish must not be counted: {values:?}"
        );
        assert_eq!(
            sites
                .iter()
                .find(|site| site.value == "Pick {} corners")
                .expect("template site")
                .macro_name
                .as_deref(),
            Some("format"),
            "the guard needs the macro to tell a bare format! from a tf!"
        );
        assert_eq!(
            sites
                .iter()
                .find(|site| site.via_const.is_some())
                .expect("alias site")
                .via_const
                .as_deref(),
            Some("MODES_PROMPT")
        );
        assert!(is_prompt_prose("Pick {} corners"));
        assert!(
            !is_prompt_prose("{:.4}"),
            "a bare format specifier is not a sentence the chairman reads"
        );
        assert_eq!(
            positional_holes("RENAME %{type}  Enter the current name:"),
            0,
            "a %-named placeholder is filled by t!, not a hole the sink drops"
        );
        assert_eq!(positional_holes("Specify {} or %{type}"), 1);
        // The exemption branch the #190 plan leans on, exercised on its own rows rather than by
        // trust: every one of the seven is placeholders plus a command name, and the prose rows that
        // differ from them by one word must still be demanded as arms.
        for only in [
            "{} <{value:.4}>{suffix}",
            "{prompt} <{value:.4}>",
            "BOUNDARY  {} <{}>:",
            "[Use current origin/Set new origin/Default to boundary extents] <Use current origin>:",
            "[bottom Left/bottom Right/top rIght/top lEft/Center] <bottom Left>:",
            "[Select polyline/Polygonal/Rectangular/Invert clip] <Rectangular>:",
            "{base} <{}>:",
        ] {
            assert!(
                prompt_is_placeholders_only(only),
                "an exempt-shaped prompt was not skipped: {only:?}"
            );
        }
        for prose in [
            "Enter an option [Invisible/Constant] <done>:",
            "Pick {} corners",
            "Specify the second corner:",
        ] {
            assert!(
                !prompt_is_placeholders_only(prose),
                "a row with real words was skipped as if it were a placeholder: {prose:?}"
            );
        }
        assert!(
            prompt_sites("#[cfg(test)]\nmod checks {\n    fn prompt() -> String { \"Only In A Test Here:\".to_string() }\n}\n")
                .is_empty(),
            "a prompt fn inside the test module is a fixture, not ink"
        );
        // ... but a prompt impl that sits AFTER that module in the same file is production code, and
        // the count probe's whole-file cut swallowed 27 of them across 9 files.
        let late = prompt_sites(
            "#[cfg(test)]\nmod checks {\n    fn prompt() -> String { \"Only In A Test Here:\".to_string() }\n}\nfn prompt(&self) -> String { \"Specify the second corner:\".to_string() }\n",
        );
        assert_eq!(
            late.iter().map(|site| site.value.as_str()).collect::<Vec<_>>(),
            vec!["Specify the second corner:"],
            "the test exclusion must cover the marked item only, not the rest of the file"
        );

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        collect_catalog_guard_files(&root, &mut files);
        files.sort();
        let mut checked = 0_usize;
        let mut exempt_hits = 0_usize;
        let mut placeholder_skips = 0_usize;
        let mut missing = Vec::new();
        let mut bare = Vec::new();
        for file in &files {
            let text = std::fs::read_to_string(file).expect("source file must be readable");
            for site in prompt_sites(&text) {
                if !is_prompt_prose(&site.value) {
                    continue;
                }
                checked += 1;
                if prompt_is_placeholders_only(&site.value) {
                    placeholder_skips += 1;
                    continue;
                }
                if STAYS_ENGLISH.contains(&site.value.as_str()) || site.value.starts_with("SETVAR: ") {
                    exempt_hits += 1;
                    continue;
                }
                if locale_catalog::message_attribute(&site.value).is_none() {
                    missing.push(format!("{}:{}: {}", file.display(), site.line, site.value));
                }
                // Only a POSITIONAL hole makes a site bare. `%{name}` is the named-argument
                // protocol `t!`/`translate_args` fills (i18n.rs:281), so a prompt whose braces are
                // all `%{...}` is already decided -- 136 of them were being reported as bare.
                let holes = positional_holes(&site.value);
                if holes > 0 && site.macro_name.as_deref() != Some("tf") {
                    bare.push(format!(
                        "{}:{}: {holes} holes inside {:?}: {}",
                        file.display(),
                        site.line,
                        site.macro_name,
                        site.value
                    ));
                }
            }
        }
        // Printed before every assert, because the assert that fires first otherwise hides the
        // other two tallies and the delta against the batch plan cannot be measured.
        println!(
            "GUARD checked={checked} stays_english={exempt_hits} placeholder_only={placeholder_skips} missing={} bare={} files={}",
            missing.len(),
            bare.len(),
            files.len()
        );
        let esc = |s: &String| s.replace('\\', "\\\\").replace('\n', "\\n").replace('\t', "\\t");
        for line in &missing {
            // Escaped so one hit stays one line: 8 of the batch prompts carry a real newline and the
            // harvest below is read back line by line.
            println!("GUARD_MISS {}", esc(line));
        }
        for line in &bare {
            println!("GUARD_BARE {}", esc(line));
        }
        assert!(
            checked > 330,
            "the scanner saw only {checked} prompt sentences, so it is broken"
        );
        // The batch's seven exemptions are exempt by shape, not by name: none of them is a brand or
        // an identifier, so asserting a STAYS_ENGLISH hit count here tested a branch that has never
        // fired for a prompt (measured exempt=0 on the red run). This asserts the branch the plan
        // actually walks through, on the same seven strings the self-test above pins.
        assert!(
            placeholder_skips >= 7,
            "only {placeholder_skips} prompt sentences are placeholders-only, but the batch registers 7 exemptions"
        );
        assert!(
            missing.is_empty(),
            "{} prompt sentences reach the command line undecided:\n{}",
            missing.len(),
            missing.join("\n")
        );
        assert!(
            bare.is_empty(),
            "{} prompt templates still hand values to the sink before translating:\n{}",
            bare.len(),
            bare.join("\n")
        );
        // The alias branch must fire on the file it was written for, otherwise the shape that hid
        // 81 rows from the body scan is invisible again.
        let attdef = std::fs::read_to_string(root.join("modules/draw/draw/attdef.rs"))
            .expect("attdef source must be readable");
        assert!(
            prompt_sites(&attdef)
                .iter()
                .any(|site| site.via_const.as_deref() == Some("MODES_PROMPT")),
            "attdef.rs no longer yields its prompt through MODES_PROMPT: update this witness"
        );
    }

    /// DEF-195. `every_prompt_literal_is_a_catalog_entry` asks whether the *literal* inside a prompt body
    /// has an arm, and that is the wrong side of the boundary for one shape: `format!("{name}  Select
    /// line:")` interpolates first, so the string that reaches the sink is `DCALIGNED  Select line:` --
    /// an arm keyed by the template never matches it, `translate` returns the source, and the command
    /// line prints English on a Chinese screen. Measured on the #190 round-2 build
    /// (s190b-dcaligned-2560-cmd.png) while every catalog-side gate stayed green.
    #[test]
    fn no_catalog_template_is_interpolated_before_it_is_translated() {
        let mut templates: Vec<String> = Vec::new();
        for line in include_str!("locale_catalog.rs").lines() {
            let Some(at) = line.find("=> Some((\"") else {
                continue;
            };
            // The raw spelling, not the unescaped one: the scan compares against source text, which keeps
            // `\"` inside the literal, so an unescaped needle would silently match nothing.
            let raw = line[..at].trim();
            if !(raw.contains("{name}") || raw.contains("{cmd}")) {
                continue;
            }
            templates.push(format!("format!({raw}"));
        }
        assert!(
            templates.len() > 100,
            "self-check: only {} captured-name templates in the catalog, the scan is broken",
            templates.len()
        );
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        collect_catalog_guard_files(&root, &mut files);
        files.sort();
        let mut dead = Vec::new();
        for file in &files {
            let text = std::fs::read_to_string(file).expect("source file must be readable");
            for hit in templates.iter().filter(|needle| text.contains(needle.as_str())) {
                dead.push((file.display().to_string(), hit.clone()));
            }
        }
        assert!(
            dead.is_empty(),
            "{} catalog template(s) are interpolated by format! before the lookup, so their arm is dead: {dead:?}",
            dead.len()
        );
    }

    #[test]
    fn greek_language_setting_round_trips() {
        let loader = FluentLanguageLoader::new("draftflow", "en-US".parse().unwrap());
        load_language(&loader, Language::ElGr).expect("Greek Fluent resources must parse");
        assert_eq!(loader.get_attr("language", "greek"), "Ελληνικά");
        assert_eq!(serde_json::to_string(&Language::ElGr).unwrap(), "\"el-GR\"");
    }
}
