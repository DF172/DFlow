// #141 / 批 042 guard: the DWG and DXF file-type icons are a delivered asset
// set, and three separate consumers depend on them staying intact — the Windows
// Shell (via the ProgIDs written in `src/io/file_association.rs`), the MSI
// (`packaging/windows/main.wxs`), and the portable build (which materializes
// them from `include_bytes!`). Before this file nothing pinned any of that:
// `main.wxs` referenced `dwg.ico` / `dxf.ico` that did not exist in the tree and
// the build still succeeded, because WiX only fails at ICE 48 validation time.
//
// Every figure here is parsed out of the shipped bytes, not typed: the size set,
// the entry count and the WXS references are read back the way the Shell reads
// them. `the_directory_reader_detects_a_doctored_container` exists so a broken
// reader cannot pass the other tests silently — a gate that cannot fail is not
// evidence.

use std::collections::BTreeSet;

const DWG_ICO: &[u8] = include_bytes!("../packaging/windows/dwg.ico");
const DXF_ICO: &[u8] = include_bytes!("../packaging/windows/dxf.ico");
const APP_ICO: &[u8] = include_bytes!("../packaging/windows/AppIcon.ico");
const MAIN_WXS: &str = include_str!("../packaging/windows/main.wxs");
const FILE_ASSOC: &str = include_str!("../src/io/file_association.rs");

/// The signed frame set (spec 批 042 §2). A Shell that finds no exact match for
/// the size it wants picks the nearest entry, so a missing 16px silently makes
/// every list view render a rescaled 24/32 instead of the redrawn plate.
const SIZES: &[u32] = &[16, 24, 32, 48, 64, 128, 256];

/// Read `ICONDIR` + the directory entries: returns `(width, height, offset, len)`
/// per entry. `None` for anything that is not a well-formed icon container.
fn directory(ico: &[u8]) -> Option<Vec<(u32, u32, u32, u32)>> {
    if ico.len() < 6 {
        return None;
    }
    let reserved = u16::from_le_bytes([ico[0], ico[1]]);
    let kind = u16::from_le_bytes([ico[2], ico[3]]);
    let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;
    if reserved != 0 || kind != 1 || ico.len() < 6 + 16 * count {
        return None;
    }
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let at = 6 + 16 * i;
        // 0 in the width byte means 256, not 0.
        let w = if ico[at] == 0 { 256 } else { ico[at] as u32 };
        let h = if ico[at + 1] == 0 { 256 } else { ico[at + 1] as u32 };
        let offset = u32::from_le_bytes([
            ico[at + 12],
            ico[at + 13],
            ico[at + 14],
            ico[at + 15],
        ]);
        let len = u32::from_le_bytes([ico[at + 8], ico[at + 9], ico[at + 10], ico[at + 11]]);
        if (offset as usize) + (len as usize) > ico.len() {
            return None;
        }
        out.push((w, h, offset, len));
    }
    Some(out)
}

fn size_set(ico: &[u8], label: &str) -> BTreeSet<u32> {
    let entries = directory(ico)
        .unwrap_or_else(|| panic!("{label} is not a well-formed ICONDIR container"));
    entries
        .iter()
        .map(|(w, h, _, _)| {
            assert_eq!(w, h, "{label} has a non-square {w}x{h} entry");
            *w
        })
        .collect()
}

#[test]
fn the_two_drawing_icons_ship_and_are_real_containers() {
    for (label, ico) in [("dwg.ico", DWG_ICO), ("dxf.ico", DXF_ICO)] {
        assert!(!ico.is_empty(), "{label} is empty");
        assert!(directory(ico).is_some(), "{label} does not parse");
    }
}

#[test]
fn both_icons_carry_exactly_the_signed_size_set() {
    for (label, ico) in [("dwg.ico", DWG_ICO), ("dxf.ico", DXF_ICO)] {
        let want: BTreeSet<u32> = SIZES.iter().copied().collect();
        let got = size_set(ico, label);
        assert_eq!(got, want, "{label} frame set drifted from the signed {SIZES:?}");
    }
}

#[test]
fn the_file_icons_and_the_app_icon_share_one_frame_ladder() {
    // Different assets, same ladder: the Shell's per-size pick behaves
    // differently when the two icon sets diverge.
    let app = size_set(APP_ICO, "AppIcon.ico");
    for (label, ico) in [("dwg.ico", DWG_ICO), ("dxf.ico", DXF_ICO)] {
        assert_eq!(
            size_set(ico, label),
            app,
            "{label} and AppIcon.ico must offer the same sizes"
        );
    }
}

#[test]
fn wxs_declares_one_file_element_per_icon_and_that_file_exists() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for (file_id, rel) in [
        ("DwgIco", "packaging/windows/dwg.ico"),
        ("DxfIco", "packaging/windows/dxf.ico"),
    ] {
        let needle = format!("<File Id='{file_id}' ");
        let at = MAIN_WXS
            .find(&needle)
            .unwrap_or_else(|| panic!("main.wxs lost the {file_id} File element"));
        let element = &MAIN_WXS[at..(at + 240).min(MAIN_WXS.len())];
        let src_start = element
            .find("Source='")
            .map(|i| i + "Source='".len())
            .unwrap_or_else(|| panic!("{file_id} has no Source attribute"));
        let tail = &element[src_start..];
        let src = &tail[..tail.find('\'').unwrap()];
        // WiX paths are backslash-y; the tree is checked forward-slash.
        let normalized = src.replace('\\', "/");
        assert_eq!(
            normalized, rel,
            "{file_id} reads from {src}, not the signed {rel} — the asset the \
             installer ships would not be the asset that was measured"
        );
        assert!(
            root.join(&normalized).is_file(),
            "{file_id} points at {src}, which is not in the tree — the MSI would \
             build a package that ships no drawing icon"
        );
        assert!(
            MAIN_WXS.contains(&format!("Icon='{file_id}'")),
            "no ProgId/Shortcut references {file_id}, so the declared file would go unused"
        );
    }
}

#[test]
fn no_drawing_progid_advertises_the_executable_as_its_icon() {
    // The defect this guard exists for: `"<exe>",0` renders AppIcon, i.e. the
    // application's own badge, on a user's drawing. After the fix exactly one
    // such write may remain, and it must belong to the app's own entry —
    // "Open with" has to show its icon.
    const EXE_ICON: &str = "{exe}\\\",0";
    let hits: Vec<&str> = FILE_ASSOC
        .lines()
        .filter(|line| line.contains(EXE_ICON))
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected the exe-as-icon write once, got {}:\n{}",
        hits.len(),
        hits.join("\n")
    );

    // Proximity, not same-line: the surviving write sits inside the
    // `Applications\DraftFlowCAD.exe` DefaultIcon call, which spans lines.
    let app_default = FILE_ASSOC
        .find(r"{APP_BASE}\DefaultIcon")
        .expect("the app's own DefaultIcon entry is gone — \"Open with\" would show a blank");
    let exe_icon = FILE_ASSOC
        .find(EXE_ICON)
        .expect("no exe-as-icon write left at all");
    assert!(
        exe_icon > app_default && exe_icon - app_default < 200,
        "the surviving exe-as-icon write is at byte {exe_icon}, {app_default} is \
         the app entry: it must be the app's own DefaultIcon, not some ProgID's"
    );

    // And the ProgID writer — the one that decides what a drawing looks like —
    // must take its icon from a path argument. Read the function body rather
    // than count lines, so re-wrapping the source cannot flip this green by
    // accident.
    let header = "fn register_progid(";
    let at = FILE_ASSOC
        .find(header)
        .expect("register_progid is gone; nothing writes the drawing ProgIDs");
    let body = &FILE_ASSOC[at..at + FILE_ASSOC[at..].find("Ok(())").map(|i| i + 6).unwrap_or(1200)];
    assert!(
        !body.contains(EXE_ICON),
        "register_progid writes the exe as the drawing's icon again — that is \
         DEF-DWG-ICON-001 itself"
    );
    assert!(
        body.contains("icon.display()") && body.contains(r"\DefaultIcon"),
        "register_progid no longer writes a DefaultIcon from the .ico path"
    );
    assert!(
        body.contains("if let Some(icon)"),
        "register_progid lost its Option guard; a missing .ico would now write \
         an empty DefaultIcon instead of leaving the key absent"
    );
    assert!(
        FILE_ASSOC.contains("include_bytes!(\"../../packaging/windows/dwg.ico\")"),
        "the portable build must still carry dwg.ico inside the binary"
    );
    assert!(
        FILE_ASSOC.contains("include_bytes!(\"../../packaging/windows/dxf.ico\")"),
        "the portable build must still carry dxf.ico inside the binary"
    );
}

#[test]
fn the_directory_reader_detects_a_doctored_container() {
    // Self-check for the instrument above: swap one entry's size and the signed
    // set must stop matching, otherwise `both_icons_carry_exactly…` would pass
    // on any bytes at all. Both the width and the height byte move, or the
    // squareness assert fires first and the test proves only that it exists.
    let mut broken = DWG_ICO.to_vec();
    broken[6] = 15; // first entry: 16 -> 15
    broken[7] = 15;
    let got = size_set(&broken, "doctored dwg.ico");
    assert!(
        !got.contains(&16),
        "the reader still reported a 16px frame after it was renamed to 15px"
    );
    let signed: BTreeSet<u32> = SIZES.iter().copied().collect();
    assert_ne!(got, signed, "a doctored container passed the signed set");
}
