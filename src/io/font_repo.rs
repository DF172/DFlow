// Missing-font detection — DWG files reference SHX fonts by bare name
// (`vnarial.shx`, `romans.shx`, …). When the file doesn't resolve on disk the
// renderer substitutes an embedded LFF stroke font, which mangles drawings
// authored with fonts it cannot stand in for. This module detects those
// references; the user resolves them by dropping the real files into the
// local fonts folder (see `fonts_dir`). DraftFlow CAD makes no network
// requests — there is no download path.

use std::path::{Path, PathBuf};

use codec::CadDocument;

/// Local store for fonts the user placed manually. Text resolution searches
/// here after the drawing folder. Options → Files can point it anywhere;
/// otherwise it is the `fonts` folder inside the config folder.
#[cfg(not(target_arch = "wasm32"))]
pub fn fonts_dir() -> Option<PathBuf> {
    if let Some(dir) = crate::config::fonts_dir_override() {
        return Some(dir);
    }
    crate::config::config_dir().map(|dir| dir.join("fonts"))
}

/// Web builds have no filesystem to cache fonts into.
#[cfg(target_arch = "wasm32")]
pub fn fonts_dir() -> Option<PathBuf> {
    None
}

/// Stable key for comparing bare font names without path or case differences.
pub fn font_key(name: &str) -> String {
    name.rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .trim()
        .to_ascii_lowercase()
}

/// Referenced-but-unresolvable `.shx` font files across the document's text
/// styles. Names are deduplicated case-insensitively and reduced to the bare
/// file name (`C:\Fonts\VNArial.shx` → `VNArial.shx`).
pub fn missing_shx_fonts(doc: &CadDocument) -> Vec<String> {
    let mut missing: Vec<String> = Vec::new();
    let base = doc
        .source_path
        .as_deref()
        .map(Path::new)
        .and_then(|p| p.parent());
    for style in doc.text_styles.iter() {
        // The primary font and the big font (Asian double-byte glyphs) are
        // both plain .shx files the style depends on.
        for file in [style.font_file.trim(), style.big_font_file.trim()] {
            if file.is_empty()
                || !file.to_ascii_lowercase().ends_with(".shx")
                || crate::io::linetypes::is_bundled_shape_file(file)
            {
                continue;
            }
            let resolved = crate::io::resolve_image_file(file, base).is_some()
                || local_font_file(file).is_some();
            if resolved {
                continue;
            }
            let name = file.rsplit(['/', '\\']).next().unwrap_or(file).to_string();
            if !missing.iter().any(|m| m.eq_ignore_ascii_case(&name)) {
                missing.push(name);
            }
        }
    }
    missing.sort_by_key(|name| name.to_ascii_lowercase());
    missing
}

/// A font file present in the local fonts directory, matched
/// case-insensitively by file name.
pub fn local_font_file(name: &str) -> Option<PathBuf> {
    let file = name.rsplit(['/', '\\']).next()?;
    let dir = fonts_dir()?;
    let direct = dir.join(file);
    if direct.is_file() {
        return Some(direct);
    }
    let lower = file.to_ascii_lowercase();
    std::fs::read_dir(&dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .find(|entry| entry.file_name().to_string_lossy().to_ascii_lowercase() == lower)
        .map(|entry| entry.path())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_keys_are_bare_trimmed_and_case_insensitive() {
        assert_eq!(font_key(r#" C:\Fonts\ROMANS.SHX "#), "romans.shx");
        assert_eq!(font_key("folder/simplex.shx"), "simplex.shx");
    }

    #[test]
    fn missing_fonts_deduplicate_by_name_case_insensitively() {
        // Two styles referencing the same missing font with different casing
        // and one whose file resolves next to the drawing must yield one hit.
        let dir = std::env::temp_dir().join("draftflow_font_repo_tests");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("exists.shx"), b"stub").unwrap();
        let mut doc = CadDocument::new();
        let mk = |name: &str, file: &str| {
            let mut style = codec::TextStyle::new(name);
            style.font_file = file.into();
            style
        };
        let _ = doc.text_styles.add(mk("A", "MISSING1.shx"));
        let _ = doc.text_styles.add(mk("B", "missing1.SHX"));
        let _ = doc.text_styles.add(mk("C", "exists.shx"));
        doc.source_path = Some(dir.join("drawing.dwg").to_string_lossy().into_owned());
        let missing = missing_shx_fonts(&doc);
        assert_eq!(missing, vec!["MISSING1.shx".to_string()]);
        let _ = std::fs::remove_file(dir.join("exists.shx"));
    }
}
