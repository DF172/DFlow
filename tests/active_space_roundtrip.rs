// The active space (Model vs a paper layout) a drawing was saved in must
// round-trip: `set_current_layout` mirrors it into the document as $TILEMODE
// (`header.show_model_space`) and the CTAB current-tab variable, and the loader
// restores it. Regression guard for the "always reopens in Model / the first
// paper layout" bugs.

use DraftFlowCAD::scene::Scene;

#[test]
fn switching_to_paper_records_tilemode_and_ctab() {
    let mut scene = Scene::new();

    // A paper layout is active → TILEMODE says paper, CTAB names the tab.
    scene.set_current_layout("Layout1".to_string());
    assert!(
        !scene.document.header.show_model_space,
        "$TILEMODE should record paper space when a layout is active"
    );
    assert_eq!(
        DraftFlowCAD::io::saved_active_layout(&scene.document).as_deref(),
        Some("Layout1"),
        "CTAB must be created/updated so the exact paper tab round-trips (not \
         just the first paper layout)"
    );

    // Back to Model → TILEMODE flips, CTAB follows.
    scene.set_current_layout("Model".to_string());
    assert!(
        scene.document.header.show_model_space,
        "$TILEMODE should record model space in the Model tab"
    );
    assert_eq!(
        DraftFlowCAD::io::saved_active_layout(&scene.document).as_deref(),
        Some("Model"),
    );
}

#[test]
fn active_paper_layout_survives_a_dxf_save_and_reload() {
    let mut scene = Scene::new();
    scene.set_current_layout("Layout1".to_string());

    // Full file round-trip: write to DXF bytes, read them back.
    let bytes = DraftFlowCAD::io::save_to_bytes(&scene.document, "dxf", scene.document.version)
        .expect("save to DXF bytes");
    let doc = DraftFlowCAD::io::load_bytes("roundtrip.dxf", bytes).expect("reload DXF bytes");

    assert!(
        !doc.header.show_model_space,
        "$TILEMODE must persist paper space across a DXF save/reload"
    );
    assert_eq!(
        DraftFlowCAD::io::saved_active_layout(&doc).as_deref(),
        Some("Layout1"),
        "CTAB must persist the exact active tab across a DXF save/reload"
    );
}

#[test]
fn ctab_is_created_when_absent_then_updated_in_place() {
    let mut scene = Scene::new();
    let doc = &mut scene.document;

    // A brand-new document carries no CTAB entry.
    assert_eq!(DraftFlowCAD::io::saved_active_layout(doc), None);

    // First write creates it; a second write must update in place, not stack a
    // duplicate entry that a reader could resolve to the stale value.
    DraftFlowCAD::io::set_saved_active_layout(doc, "Layout2");
    assert_eq!(
        DraftFlowCAD::io::saved_active_layout(doc).as_deref(),
        Some("Layout2")
    );
    DraftFlowCAD::io::set_saved_active_layout(doc, "Layout3");
    assert_eq!(
        DraftFlowCAD::io::saved_active_layout(doc).as_deref(),
        Some("Layout3")
    );
}
