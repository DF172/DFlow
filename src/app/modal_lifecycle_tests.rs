//! B4: one lifecycle for every dialog.
//!
//! The requirement was to give all `ModalKind` variants the same exit, focus
//! return and keyboard gate without touching 56 files. That only holds if the
//! field itself has two doors, so the first test scans the tree for the doors
//! and the rest exercise them. `kind_name` is the compile-time half: adding a
//! variant fails to build here until it is named, which is what keeps
//! `all_kinds` honest.

use super::{DraftFlowCAD, Message, ModalKind};
use crate::ui::{focus, motion};
use iced::time::Instant;
use std::path::Path;

/// Every variant of [`ModalKind`], platform differences included.
fn all_kinds() -> Vec<ModalKind> {
    use ModalKind::*;
    let mut kinds = vec![
        Shortcuts,
        LayerStateManager,
        LayerTranslator,
        DrawingUnits,
        BlockDefinition,
        PdfAttach,
        PointCloudAttach,
        PointCloudColorMap,
        PcSection,
        UnderlayLayers,
        PdfImportSettings,
        PdfImportFile,
        XrefAttach,
        WriteBlock,
        GeometricTolerance,
        AttDef,
        AttDefEdit,
        CountInvalidArea,
        Field,
        DraftingSettings,
        AutoConstrainSettings,
        LayerStateEditor,
        Plot,
        PrintAll,
        LayoutManager,
        Plotstyle,
        TextStyle,
        TableStyle,
        MlStyle,
        MLeaderStyle,
        DimStyle,
        Unsaved,
        SaveDialog,
        Recovery,
        MissingFonts,
        RecoveryPrompt,
        Options,
        FindReplace,
        AecDropWarning,
        PointStyle,
        AttributeEditor,
        LayerDeleteWarning,
        Aliases,
        NamedParameters,
        ScaleManager,
        AnnoObjectScale,
        Hyperlink,
        InsertTable,
        DataLinkManager,
        DataExtraction,
        GpuWarning,
        XrefHelp,
        SheetSet,
    ];
    #[cfg(not(target_arch = "wasm32"))]
    kinds.extend([FileInUse, ExternalChange]);
    #[cfg(target_arch = "wasm32")]
    kinds.push(AutosaveRestore);
    kinds
}

/// Exhaustive on purpose: a new variant is a compile error here, not a dialog
/// that quietly skips the lifecycle.
fn kind_name(kind: ModalKind) -> &'static str {
    use ModalKind::*;
    match kind {
        Shortcuts => "Shortcuts",
        LayerStateManager => "LayerStateManager",
        LayerTranslator => "LayerTranslator",
        DrawingUnits => "DrawingUnits",
        BlockDefinition => "BlockDefinition",
        PdfAttach => "PdfAttach",
        PointCloudAttach => "PointCloudAttach",
        PointCloudColorMap => "PointCloudColorMap",
        PcSection => "PcSection",
        UnderlayLayers => "UnderlayLayers",
        PdfImportSettings => "PdfImportSettings",
        PdfImportFile => "PdfImportFile",
        XrefAttach => "XrefAttach",
        WriteBlock => "WriteBlock",
        GeometricTolerance => "GeometricTolerance",
        AttDef => "AttDef",
        AttDefEdit => "AttDefEdit",
        CountInvalidArea => "CountInvalidArea",
        Field => "Field",
        DraftingSettings => "DraftingSettings",
        AutoConstrainSettings => "AutoConstrainSettings",
        LayerStateEditor => "LayerStateEditor",
        Plot => "Plot",
        PrintAll => "PrintAll",
        LayoutManager => "LayoutManager",
        Plotstyle => "Plotstyle",
        TextStyle => "TextStyle",
        TableStyle => "TableStyle",
        MlStyle => "MlStyle",
        MLeaderStyle => "MLeaderStyle",
        DimStyle => "DimStyle",
        Unsaved => "Unsaved",
        SaveDialog => "SaveDialog",
        Recovery => "Recovery",
        MissingFonts => "MissingFonts",
        RecoveryPrompt => "RecoveryPrompt",
        Options => "Options",
        FindReplace => "FindReplace",
        AecDropWarning => "AecDropWarning",
        #[cfg(not(target_arch = "wasm32"))]
        FileInUse => "FileInUse",
        #[cfg(not(target_arch = "wasm32"))]
        ExternalChange => "ExternalChange",
        PointStyle => "PointStyle",
        AttributeEditor => "AttributeEditor",
        LayerDeleteWarning => "LayerDeleteWarning",
        Aliases => "Aliases",
        NamedParameters => "NamedParameters",
        ScaleManager => "ScaleManager",
        AnnoObjectScale => "AnnoObjectScale",
        Hyperlink => "Hyperlink",
        InsertTable => "InsertTable",
        DataLinkManager => "DataLinkManager",
        DataExtraction => "DataExtraction",
        GpuWarning => "GpuWarning",
        XrefHelp => "XrefHelp",
        SheetSet => "SheetSet",
        #[cfg(target_arch = "wasm32")]
        AutosaveRestore => "AutosaveRestore",
    }
}

fn collect_rs(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("source directory must be readable") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// The `fn` a line belongs to, so the door gate can say *which* door it found
/// instead of only counting assignments.
fn enclosing_fn(lines: &[&str], at: usize) -> String {
    for line in lines[..=at].iter().rev() {
        let mut text = line.trim_start();
        for prefix in [
            "pub(in crate::app) ",
            "pub(crate) ",
            "pub(super) ",
            "pub ",
            "async ",
        ] {
            if let Some(rest) = text.strip_prefix(prefix) {
                text = rest;
            }
        }
        if let Some(rest) = text.strip_prefix("fn ") {
            return rest
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .next()
                .unwrap_or_default()
                .to_string();
        }
    }
    String::new()
}

/// The 56 dialogs are only "collected" if the field has one creator and one
/// destroyer. A direct assignment anywhere else is a dialog that skipped the
/// fade, the focus return and the keyboard gate, so it fails here by file and
/// line rather than by a screenshot the reviewer has to notice.
#[test]
fn the_dialog_field_has_exactly_two_doors() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs(&root, &mut files);
    files.sort();

    let mut closes: Vec<(String, String, String)> = Vec::new();
    let mut opens: Vec<(String, String, String)> = Vec::new();
    for file in &files {
        // This file only quotes the needles; it never touches the field.
        if file.ends_with("modal_lifecycle_tests.rs") {
            continue;
        }
        let text = std::fs::read_to_string(file).expect("source file must be readable");
        let lines: Vec<&str> = text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            for (needle, sink) in [
                ("active_modal = None", &mut closes),
                ("active_modal = Some(", &mut opens),
            ] {
                if line.contains(needle) {
                    sink.push((
                        file.display().to_string(),
                        enclosing_fn(&lines, index),
                        format!("{}:{}", file.display(), index + 1),
                    ));
                }
            }
        }
    }

    assert_eq!(
        closes.len(),
        2,
        "a dialog must close through close_modal_now or teardown_modal, found: {closes:?}"
    );
    assert_eq!(
        opens.len(),
        1,
        "a dialog must open through open_modal, found: {opens:?}"
    );
    let mut close_doors: Vec<&str> = closes.iter().map(|(_, f, _)| f.as_str()).collect();
    close_doors.sort();
    assert_eq!(
        close_doors,
        vec!["close_modal_now", "teardown_modal"],
        "the two closers moved: {closes:?}"
    );
    for (path, _, at) in &closes {
        assert!(
            path.ends_with("app\\update\\mod.rs") || path.ends_with("app/update/mod.rs"),
            "a closer lives outside the chokepoint file: {at}"
        );
    }
    assert_eq!(opens[0].1, "open_modal", "the opener moved: {opens:?}");
}

/// Every variant must fade before it is torn down, whatever its payload.
#[test]
fn every_dialog_kind_fades_before_it_is_torn_down() {
    motion::set_enabled(false);
    let mut app = DraftFlowCAD::new_for_test();
    let kinds = all_kinds();
    assert_eq!(
        kinds.len(),
        if cfg!(target_arch = "wasm32") { 56 } else { 55 },
        "the variant list and the enum disagree"
    );
    for (index, kind) in kinds.iter().enumerate() {
        for other in &kinds[index + 1..] {
            assert_ne!(kind, other, "the variant list repeats a kind");
        }
        assert!(!kind_name(*kind).is_empty());
        app.open_modal(*kind);
        assert_eq!(app.active_modal, Some(*kind), "{kind:?} did not open");
        app.close_active_modal();
        assert!(
            app.modal_exit,
            "{kind:?} vanished instead of entering its exit"
        );
        assert_eq!(
            app.active_modal,
            Some(*kind),
            "{kind:?} left before its fade could paint"
        );
        app.finish_modal_exit();
        assert!(app.active_modal.is_none(), "{kind:?} survived its teardown");
        assert!(!app.modal_exit);
    }
    motion::set_enabled(false);
}

#[test]
fn a_dismissed_dialog_stays_painted_until_its_fade_lands() {
    motion::set_enabled(true);
    let mut app = DraftFlowCAD::new_for_test();
    app.open_modal(ModalKind::Options);
    assert!(motion::running(motion::MODAL_FADE), "the entrance is in flight");

    app.close_active_modal();
    assert!(app.modal_exit);
    assert_eq!(
        app.active_modal,
        Some(ModalKind::Options),
        "the sheet still owns the frame while it fades"
    );

    std::thread::sleep(std::time::Duration::from_millis(motion::EXIT + 40));
    let _ = app.update(Message::MotionTick(Instant::now()));
    assert!(
        app.active_modal.is_none(),
        "the motion tick is what lands the exit"
    );
    assert!(!app.modal_exit);
    assert_eq!(motion::value(motion::MODAL_FADE), 0.0);
}

#[test]
fn a_real_action_commits_a_pending_exit_before_it_is_dispatched() {
    motion::set_enabled(true);
    let mut app = DraftFlowCAD::new_for_test();
    app.open_modal(ModalKind::Options);
    app.close_active_modal();
    assert!(app.modal_exit);

    // A dialog that is only visually leaving must not keep the keyboard.
    let _ = app.update(Message::Noop);
    assert!(
        app.active_modal.is_none(),
        "the next action must not land inside a vanishing dialog"
    );
    assert!(!app.modal_exit);
}

#[test]
fn a_commit_door_drops_the_dialog_in_the_same_frame() {
    motion::set_enabled(true);
    let mut app = DraftFlowCAD::new_for_test();
    app.open_modal(ModalKind::Options);
    app.close_modal_now();
    assert!(app.active_modal.is_none());
    assert!(!app.modal_exit);
    assert_eq!(motion::value(motion::MODAL_FADE), 0.0);
}

#[test]
fn a_chained_open_swaps_the_sheet_without_replaying_the_entrance() {
    motion::set_enabled(true);
    let mut app = DraftFlowCAD::new_for_test();
    app.open_modal(ModalKind::Options);
    app.modal_entrance = iced::Vector::new(motion::ENTRANCE_REACH, 0.0);
    app.close_active_modal();
    assert!(app.modal_exit);

    app.open_modal(ModalKind::Shortcuts);
    assert_eq!(app.active_modal, Some(ModalKind::Shortcuts));
    assert!(!app.modal_exit, "the pending exit was committed, not inherited");
    assert_eq!(
        app.modal_entrance,
        iced::Vector::ZERO,
        "a swap is not an arrival, so it does not slide"
    );
    assert_eq!(
        motion::value(motion::MODAL_FADE),
        1.0,
        "the sheet never dips between two dialogs"
    );
}

#[test]
fn the_trigger_wakes_up_when_the_dialog_closes() {
    motion::set_enabled(false);
    focus::begin_pass();
    focus::clear();
    focus::enroll("b4-test:trigger");
    assert!(focus::advance(true));
    assert_eq!(focus::current(), Some("b4-test:trigger"));

    let mut app = DraftFlowCAD::new_for_test();
    app.open_modal(ModalKind::Options);
    app.close_modal_now();
    assert_eq!(
        focus::current(),
        None,
        "the ring belonged to the closed dialog and must be dropped"
    );

    focus::begin_pass();
    focus::enroll("b4-test:trigger");
    assert_eq!(
        focus::current(),
        Some("b4-test:trigger"),
        "the stop goes back to whatever opened the dialog"
    );
}
