//! Shared per-user config directory used by the small settings stores (recent
//! files, status-bar layout, ribbon collapse mode, …). Everything lives under
//! `<platform-config>/DraftFlowCAD` so the app keeps a single tidy folder.
//!
//! The folder itself is relocatable: every store below follows a small note
//! that names the folder the user picked. The note lives outside every folder
//! this module can move — a registry value on Windows — because moving empties
//! and removes the folder it came from. Removing the note falls back to the
//! default without touching the moved files.

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

/// The platform's own config directory (not created). This one never moves:
/// a moved folder is remembered outside it (see [`pointer`]). `None` when the
/// platform config base can't be resolved (e.g. no `HOME`).
#[cfg(not(target_arch = "wasm32"))]
pub fn default_config_dir() -> Option<PathBuf> {
    // Tests must never read or write the person's real settings: a test run
    // once raced dozens of tests through an alias-table migration and left the
    // user's alias file holding a single line. Every store routes through here,
    // so one per-process scratch folder isolates them all.
    if cfg!(test) {
        return Some(std::env::temp_dir().join(format!("draftflow-test-config-{}", std::process::id())));
    }
    let base: PathBuf = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)?
    } else if cfg!(target_os = "macos") {
        let home = std::env::var_os("HOME")?;
        let mut p = PathBuf::from(home);
        p.push("Library");
        p.push("Application Support");
        p
    } else if let Some(d) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(d)
    } else {
        let home = std::env::var_os("HOME")?;
        let mut p = PathBuf::from(home);
        p.push(".config");
        p
    };
    let mut p = base;
    p.push("DraftFlowCAD");
    Some(p)
}

/// Cached answer for "did the user move the folder?" — `None` until first use,
/// then `Some(chosen)` or `Some(None)` for "default".
#[cfg(not(target_arch = "wasm32"))]
static CONFIG_OVERRIDE: Mutex<Option<Option<PathBuf>>> = Mutex::new(None);

/// Where the app remembers a moved folder. It cannot live inside the default
/// folder: a cut empties that folder and removes it, so the note would be
/// deleted by the very act it records.
///
/// On Windows it is a value under `HKCU\Software\DraftFlowCAD`, named after the
/// default folder it belongs to so an isolated profile (a test run's scratch
/// folder, a portable copy) cannot steer another one's choice. Elsewhere it is
/// one small file beside the default folder, never inside it.
#[cfg(all(not(target_arch = "wasm32"), target_os = "windows"))]
mod pointer {
    use super::default_config_dir;
    use std::path::PathBuf;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
        RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE,
        REG_SZ,
    };

    const SUBKEY: &str = r"Software\DraftFlowCAD";

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }

    /// One value name per default folder. FNV-1a is enough to keep two profiles
    /// apart; it is an identity, not a checksum.
    fn value_name() -> String {
        let base = default_config_dir()
            .map(|dir| dir.display().to_string())
            .unwrap_or_default();
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in base.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        format!("config_dir_{hash:016x}")
    }

    pub(super) fn read() -> Option<PathBuf> {
        let subkey = wide(SUBKEY);
        let mut key: HKEY = std::ptr::null_mut();
        if unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut key) } != 0
        {
            return None;
        }
        let name = wide(&value_name());
        let mut kind = 0u32;
        let mut bytes = 0u32;
        let rc = unsafe {
            RegQueryValueExW(
                key,
                name.as_ptr(),
                std::ptr::null(),
                &mut kind,
                std::ptr::null_mut(),
                &mut bytes,
            )
        };
        if rc != 0 || kind != REG_SZ || bytes < 2 {
            unsafe { RegCloseKey(key) };
            return None;
        }
        let mut buffer = vec![0u16; (bytes as usize + 1) / 2 + 1];
        let mut written = bytes;
        let rc = unsafe {
            RegQueryValueExW(
                key,
                name.as_ptr(),
                std::ptr::null(),
                &mut kind,
                buffer.as_mut_ptr() as *mut u8,
                &mut written,
            )
        };
        unsafe { RegCloseKey(key) };
        if rc != 0 {
            return None;
        }
        let len = buffer.iter().position(|unit| *unit == 0).unwrap_or(buffer.len());
        let text = String::from_utf16_lossy(&buffer[..len]);
        if text.trim().is_empty() {
            None
        } else {
            Some(PathBuf::from(text))
        }
    }

    pub(super) fn write(path: &std::path::Path) -> Result<(), String> {
        let subkey = wide(SUBKEY);
        let mut key: HKEY = std::ptr::null_mut();
        let rc = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                0,
                std::ptr::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                std::ptr::null(),
                &mut key,
                std::ptr::null_mut(),
            )
        };
        if rc != 0 {
            return Err(format!("Could not open HKCU\\{SUBKEY} ({rc})"));
        }
        let name = wide(&value_name());
        let data = wide(&path.display().to_string());
        let cb = (data.len() * std::mem::size_of::<u16>()) as u32;
        let rc =
            unsafe { RegSetValueExW(key, name.as_ptr(), 0, REG_SZ, data.as_ptr() as *const u8, cb) };
        unsafe { RegCloseKey(key) };
        if rc != 0 {
            return Err(format!("Could not store the folder location ({rc})"));
        }
        Ok(())
    }

    pub(super) fn clear() -> Result<(), String> {
        let subkey = wide(SUBKEY);
        let mut key: HKEY = std::ptr::null_mut();
        if unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_WRITE, &mut key) } != 0
        {
            return Ok(());
        }
        let name = wide(&value_name());
        let rc = unsafe { RegDeleteValueW(key, name.as_ptr()) };
        unsafe { RegCloseKey(key) };
        if rc != 0 {
            return Err(format!("Could not clear the folder location ({rc})"));
        }
        Ok(())
    }
}

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "windows")))]
mod pointer {
    use super::default_config_dir;
    use std::path::PathBuf;

    /// A sibling of the default folder, never a child of it.
    fn file() -> Option<PathBuf> {
        default_config_dir().and_then(|dir| {
            dir.parent()
                .map(|parent| parent.join("DraftFlowCAD-location"))
        })
    }

    pub(super) fn read() -> Option<PathBuf> {
        file().and_then(|path| std::fs::read_to_string(path).ok())
            .map(|text| PathBuf::from(text.trim()))
            .filter(|path| path.is_absolute())
    }

    pub(super) fn write(path: &std::path::Path) -> Result<(), String> {
        let file = file().ok_or_else(|| "No place to remember the folder".to_string())?;
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        std::fs::write(file, path.display().to_string()).map_err(|error| error.to_string())
    }

    pub(super) fn clear() -> Result<(), String> {
        if let Some(file) = file() {
            if file.exists() {
                std::fs::remove_file(file).map_err(|error| error.to_string())?;
            }
        }
        Ok(())
    }
}

/// A pointer is usable when it names an absolute, non-empty path. Anything
/// else (a half-written line, a relative path, a deleted root) is ignored and
/// the default folder wins.
#[cfg(not(target_arch = "wasm32"))]
fn parse_pointer(text: &str) -> Option<PathBuf> {
    normalize(PathBuf::from(text.trim()))
}

/// `canonicalize` answers with an extended-length path (`\\?\D:\…`). It works
/// for every file call but reads as garbage in the Files page and in the
/// pointer file, so the prefix is dropped wherever the path is only being
/// stored or shown.
#[cfg(not(target_arch = "wasm32"))]
fn strip_verbatim(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    match text.strip_prefix(r"\\?\") {
        Some(rest) => PathBuf::from(rest),
        None => path,
    }
}

/// The note left by a build that kept the pointer inside the default folder.
/// Read once, copied into the new home, then removed so no stale copy can
/// disagree with it.
#[cfg(not(target_arch = "wasm32"))]
const LEGACY_POINTER: &str = "location.txt";

#[cfg(not(target_arch = "wasm32"))]
fn stored_override() -> Option<PathBuf> {
    let mut slot = CONFIG_OVERRIDE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if slot.is_none() {
        *slot = Some(load_pointer());
    }
    slot.clone().flatten()
}

#[cfg(not(target_arch = "wasm32"))]
fn load_pointer() -> Option<PathBuf> {
    if let Some(path) = pointer::read().and_then(normalize) {
        return Some(path);
    }
    adopt_legacy_pointer()
}

/// Take over the old in-folder pointer, if this is the first launch after the
/// change: remember it in the new place, delete the old file, and drop the
/// default folder when that file was all it held.
#[cfg(not(target_arch = "wasm32"))]
fn adopt_legacy_pointer() -> Option<PathBuf> {
    let default = default_config_dir()?;
    let legacy = default.join(LEGACY_POINTER);
    let text = std::fs::read_to_string(&legacy).ok()?;
    let path = parse_pointer(&text)?;
    let _ = pointer::write(&path);
    let _ = std::fs::remove_file(&legacy);
    let _ = std::fs::remove_dir(&default);
    Some(path)
}

/// An absolute, non-empty path, with the extended-length prefix dropped.
#[cfg(not(target_arch = "wasm32"))]
fn normalize(path: PathBuf) -> Option<PathBuf> {
    if path.as_os_str().is_empty() || !path.is_absolute() {
        return None;
    }
    Some(strip_verbatim(path))
}

/// The DraftFlowCAD config directory (not created), following the user's
/// chosen folder when one is set. `None` when no platform base resolves.
/// Callers `join` their own file name onto it and `create_dir_all` its parent
/// before writing.
#[cfg(not(target_arch = "wasm32"))]
pub fn config_dir() -> Option<PathBuf> {
    stored_override().or_else(default_config_dir)
}

/// Whether the folder was moved away from the platform default.
#[cfg(not(target_arch = "wasm32"))]
pub fn config_dir_is_custom() -> bool {
    stored_override().is_some()
}

/// Cut the config folder over to `path` and make it stick.
///
/// This is a cut, not a copy: the whole tree is verified in the new folder
/// before a single source file is removed, so two live copies of
/// `settings.json` can never drift apart and a failure leaves both sides as
/// they were. Only after the move is the old folder deleted outright — an
/// empty shell left behind still reads as "my data is there". The note that
/// points at the new folder lives outside it (see [`pointer`]), so removing
/// the folder cannot destroy the record.
#[cfg(not(target_arch = "wasm32"))]
pub fn set_config_dir(path: &std::path::Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("Choose an absolute folder path".to_string());
    }
    std::fs::create_dir_all(path).map_err(|error| error.to_string())?;
    let target = strip_verbatim(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()));
    let leaving = config_dir().filter(|old| *old != target);
    if let Some(old) = &leaving {
        if target.starts_with(old) {
            return Err("Choose a folder outside the current one".to_string());
        }
    }
    if let Some(old) = &leaving {
        cut_contents(old, &target)?;
    }
    if let Err(error) = pointer::write(&target) {
        // Put everything back rather than leave data where nothing points.
        if let Some(old) = &leaving {
            let _ = cut_contents(&target, old);
        }
        return Err(error);
    }
    // The record and the cache now name the new folder, so it must win even if
    // the old shell cannot be deleted.
    *CONFIG_OVERRIDE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Some(target.clone()));
    if let Some(old) = &leaving {
        remove_folder(old, &target)?;
    }
    Ok(())
}

/// Go back to the platform default folder. The moved folder's contents travel
/// home first (the folder we are leaving wins every clash, because it holds
/// the live settings), the record is cleared as soon as they have landed, and
/// only then is the folder the user picked deleted.
#[cfg(not(target_arch = "wasm32"))]
pub fn reset_config_dir() -> Result<(), String> {
    let leaving = match (stored_override(), default_config_dir()) {
        (Some(current), Some(default)) if current != default => Some((current, default)),
        _ => None,
    };
    if let Some((current, default)) = &leaving {
        std::fs::create_dir_all(default).map_err(|error| error.to_string())?;
        cut_contents(current, default)?;
    }
    pointer::clear()?;
    *CONFIG_OVERRIDE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(None);
    if let Some((current, default)) = &leaving {
        remove_folder(current, default)?;
    }
    Ok(())
}

/// Cut a support folder's contents (fonts, plot styles, autosaves) over to
/// `to`: everything arrives or nothing moves, and on success the folder it
/// came from is gone rather than left behind empty.
#[cfg(not(target_arch = "wasm32"))]
pub fn move_dir_contents(from: &std::path::Path, to: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|error| error.to_string())?;
    let target = strip_verbatim(to.canonicalize().unwrap_or_else(|_| to.to_path_buf()));
    if target == from {
        return Ok(());
    }
    if target.starts_with(from) {
        return Err("Choose a folder outside the current one".to_string());
    }
    cut_contents(from, &target)?;
    remove_folder(from, &target)
}

/// Remove the folder a cut has just emptied. The data is already safe in
/// `kept`, so the message says where it went instead of pretending nothing
/// happened.
#[cfg(not(target_arch = "wasm32"))]
fn remove_folder(folder: &std::path::Path, kept: &std::path::Path) -> Result<(), String> {
    if !folder.exists() {
        return Ok(());
    }
    std::fs::remove_dir_all(folder).map_err(|error| {
        format!(
            "The files moved to {}, but the old folder could not be removed: {error}",
            kept.display()
        )
    })
}

/// Cut a folder over in one piece.
///
/// The whole tree is planned first, every file is written and length-checked
/// second, and only when every single one has arrived are the sources
/// deleted. A failure anywhere rolls back the files this call created, so the
/// user is never left with half a folder on each side. Files already present
/// in the destination are replaced: the folder being left is the live one.
#[cfg(not(target_arch = "wasm32"))]
fn cut_contents(from: &std::path::Path, to: &std::path::Path) -> Result<(), String> {
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    plan_move(from, to, &mut dirs, &mut files)?;

    let mut created_dirs = Vec::new();
    let mut created_files = Vec::new();
    for dir in &dirs {
        if !dir.exists() {
            if let Err(error) = std::fs::create_dir_all(dir) {
                undo(&created_files, &created_dirs);
                return Err(format!("{}: {error}", dir.display()));
            }
            created_dirs.push(dir.clone());
        }
    }
    for (src, dst) in &files {
        let existed = dst.exists();
        if let Err(error) = std::fs::copy(src, dst) {
            undo(&created_files, &created_dirs);
            return Err(format!("{}: {error}", src.display()));
        }
        let arrived = std::fs::metadata(dst)
            .map(|meta| meta.len())
            .ok()
            .zip(std::fs::metadata(src).map(|meta| meta.len()).ok())
            .is_some_and(|(written, original)| written == original);
        if !arrived {
            undo(&created_files, &created_dirs);
            return Err(format!("{} did not arrive whole", src.display()));
        }
        if !existed {
            created_files.push(dst.clone());
        }
    }

    move_sources(&files)?;
    for dir in dirs.iter().rev() {
        let _ = std::fs::remove_dir(dir);
    }
    Ok(())
}

/// List every directory and file a cut would create, without touching disk.
/// A folder that is not there yet simply has nothing to cut.
#[cfg(not(target_arch = "wasm32"))]
fn plan_move(
    from: &std::path::Path,
    to: &std::path::Path,
    dirs: &mut Vec<PathBuf>,
    files: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<(), String> {
    if !from.exists() {
        return Ok(());
    }
    let entries = std::fs::read_dir(from).map_err(|error| format!("{}: {error}", from.display()))?;
    for entry in entries.flatten() {
        let src = entry.path();
        let name = entry.file_name();
        let dst = to.join(&name);
        if src.is_dir() {
            dirs.push(dst.clone());
            plan_move(&src, &dst, dirs, files)?;
        } else {
            files.push((src, dst));
        }
    }
    Ok(())
}

/// Delete every source file. Runs only after the whole destination has been
/// verified, so a refusal here leaves a complete second copy rather than a
/// hole.
#[cfg(not(target_arch = "wasm32"))]
fn move_sources(files: &[(PathBuf, PathBuf)]) -> Result<(), String> {
    for (src, _) in files {
        if let Err(error) = std::fs::remove_file(src) {
            return Err(format!("{} could not be removed: {error}", src.display()));
        }
    }
    Ok(())
}

/// Drop the files and folders this call created, so a failed cut leaves the
/// disk exactly as it was.
#[cfg(not(target_arch = "wasm32"))]
fn undo(files: &[PathBuf], dirs: &[PathBuf]) {
    for file in files {
        let _ = std::fs::remove_file(file);
    }
    for dir in dirs.iter().rev() {
        let _ = std::fs::remove_dir(dir);
    }
}

// ── Independently relocatable support folders ────────────────────────────────
//
// Fonts, plot styles and the autosave scratch folder can each sit somewhere of
// the user's choosing without moving the config folder. The choices live in
// `settings.json` (which itself follows the config folder); these statics are
// the process-wide copy the per-file lookups read.

#[cfg(not(target_arch = "wasm32"))]
static FONTS_OVERRIDE: Mutex<Option<PathBuf>> = Mutex::new(None);
#[cfg(not(target_arch = "wasm32"))]
static PLOT_STYLES_OVERRIDE: Mutex<Option<PathBuf>> = Mutex::new(None);
#[cfg(not(target_arch = "wasm32"))]
static AUTOSAVE_OVERRIDE: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Set (or clear) the three support-folder choices in one go.
#[cfg(not(target_arch = "wasm32"))]
pub fn set_dir_overrides(
    fonts: Option<PathBuf>,
    plot_styles: Option<PathBuf>,
    autosave: Option<PathBuf>,
) {
    *FONTS_OVERRIDE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = fonts;
    *PLOT_STYLES_OVERRIDE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = plot_styles;
    *AUTOSAVE_OVERRIDE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = autosave;
}

#[cfg(not(target_arch = "wasm32"))]
fn cloned_override(store: &Mutex<Option<PathBuf>>) -> Option<PathBuf> {
    store
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn fonts_dir_override() -> Option<PathBuf> {
    cloned_override(&FONTS_OVERRIDE)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn plot_styles_dir_override() -> Option<PathBuf> {
    cloned_override(&PLOT_STYLES_OVERRIDE)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn autosave_dir_override() -> Option<PathBuf> {
    cloned_override(&AUTOSAVE_OVERRIDE)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// A pointer file is only trusted when it names an absolute folder:
    /// a stray relative line must not move every store into the working
    /// directory, and a blank file falls back to the platform default.
    #[test]
    fn only_absolute_pointers_are_followed() {
        let absolute = std::env::temp_dir().join("draftflow-pointer-test");
        assert_eq!(
            parse_pointer(&absolute.display().to_string()),
            Some(absolute)
        );
        assert_eq!(parse_pointer("   "), None);
        assert_eq!(parse_pointer("CAD/data"), None);
    }

    /// A picked folder is stored and shown without the extended-length
    /// prefix `canonicalize` adds, otherwise the Files page reads
    /// `\\?\D:\CAD` and the pointer file keeps that spelling forever.
    #[test]
    fn extended_length_prefix_is_dropped() {
        assert_eq!(
            strip_verbatim(PathBuf::from(r"\\?\D:\tmp\CAD")),
            PathBuf::from(r"D:\tmp\CAD")
        );
        assert_eq!(
            strip_verbatim(PathBuf::from(r"\\?\UNC\server\share\CAD")),
            PathBuf::from(r"\\server\share\CAD")
        );
        assert_eq!(
            strip_verbatim(PathBuf::from(r"D:\tmp\CAD")),
            PathBuf::from(r"D:\tmp\CAD")
        );
    }

    /// Moving the folder is a cut, not a copy: the live settings win every
    /// clash, files that only the destination has survive, and the folder
    /// being left is gone afterwards — no empty shell, nothing to confuse
    /// with the real data.
    #[test]
    fn moving_takes_the_content_and_removes_the_old_folder() {
        let root = std::env::temp_dir().join(format!("draftflow-move-{}", std::process::id()));
        let from = root.join("from");
        let to = root.join("to");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(from.join("plotstyles")).unwrap();
        std::fs::create_dir_all(&to).unwrap();
        std::fs::write(from.join("settings.json"), "new").unwrap();
        std::fs::write(from.join("plotstyles/named.ctb"), "ctb").unwrap();
        std::fs::write(to.join("settings.json"), "old").unwrap();
        std::fs::write(to.join("only-default.txt"), "keep").unwrap();

        move_dir_contents(&from, &to).expect("the cut should complete");
        assert_eq!(
            std::fs::read_to_string(to.join("settings.json")).unwrap(),
            "new"
        );
        assert_eq!(
            std::fs::read_to_string(to.join("only-default.txt")).unwrap(),
            "keep"
        );
        assert_eq!(
            std::fs::read_to_string(to.join("plotstyles/named.ctb")).unwrap(),
            "ctb"
        );
        assert!(!from.exists(), "the old folder must not be left behind");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The note that names the moved folder has to survive the folder it used
    /// to sit in, so it is written outside every folder this module deletes.
    #[test]
    fn the_location_note_is_stored_outside_the_folder() {
        let chosen = std::env::temp_dir().join("draftflow-location-note");
        pointer::write(&chosen).expect("the note should be written");
        assert_eq!(pointer::read(), Some(chosen));
        pointer::clear().expect("the note should be cleared");
        assert_eq!(pointer::read(), None);
    }

    /// A cut is all-or-nothing: when one folder cannot be created at the
    /// destination, nothing is copied and nothing is deleted, so the user
    /// never ends up with half a config on each side.
    #[test]
    fn a_failed_cut_leaves_everything_where_it_was() {
        let root = std::env::temp_dir().join(format!("draftflow-cut-fail-{}", std::process::id()));
        let from = root.join("from");
        let to = root.join("to");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(from.join("plotstyles")).unwrap();
        std::fs::create_dir_all(&to).unwrap();
        std::fs::write(from.join("settings.json"), "live").unwrap();
        std::fs::write(from.join("plotstyles/named.ctb"), "ctb").unwrap();
        // A file where the destination needs a folder: the cut cannot finish.
        std::fs::write(to.join("plotstyles"), "in the way").unwrap();

        let result = move_dir_contents(&from, &to);

        assert!(result.is_err(), "a blocked destination must report failure");
        assert!(from.join("settings.json").exists());
        assert!(from.join("plotstyles/named.ctb").exists());
        assert!(!to.join("settings.json").exists());
        assert_eq!(
            std::fs::read_to_string(to.join("plotstyles")).unwrap(),
            "in the way"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}

// ── Last file-dialog directory ───────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Mutex, OnceLock};

#[cfg(not(target_arch = "wasm32"))]
fn last_dir_store() -> &'static Mutex<Option<PathBuf>> {
    static STORE: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    STORE.get_or_init(|| {
        // Seed from the persisted value; discard it if the folder is gone.
        let loaded = config_dir()
            .map(|d| d.join("last_dir.txt"))
            .and_then(|f| std::fs::read_to_string(f).ok())
            .map(|s| PathBuf::from(s.trim()))
            .filter(|p| p.is_dir());
        Mutex::new(loaded)
    })
}

/// The directory the last file dialog picked or saved into, if it still
/// exists — used to seed the next dialog so pickers reopen where the user
/// left off. Persisted across runs.
#[cfg(not(target_arch = "wasm32"))]
pub fn last_dialog_dir() -> Option<PathBuf> {
    last_dir_store().lock().ok()?.clone().filter(|p| p.is_dir())
}

/// Record the directory of a path a file dialog just returned.
#[cfg(not(target_arch = "wasm32"))]
pub fn remember_dialog_dir(file_path: &Path) {
    let Some(dir) = file_path.parent().filter(|d| d.is_dir()) else {
        return;
    };
    if let Ok(mut store) = last_dir_store().lock() {
        if store.as_deref() == Some(dir) {
            return; // unchanged — skip the disk write
        }
        *store = Some(dir.to_path_buf());
    }
    if let Some(cfg) = config_dir() {
        let _ = std::fs::create_dir_all(&cfg);
        let _ = std::fs::write(cfg.join("last_dir.txt"), dir.display().to_string());
    }
}
