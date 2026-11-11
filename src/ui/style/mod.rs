pub mod anno_object_scale;
pub mod common;
pub mod dimstyle;
pub mod draftflow_theme;
pub mod fusion_theme;
pub mod mleaderstyle;
pub mod mlstyle;
pub mod plotstyle;
pub mod point_style;
pub mod scale_manager;
pub mod style_list;
pub mod style_manager;
pub mod tablestyle;
pub mod textstyle;
pub mod tokens;

// Ratchet guard for the ui/kit teardown: violation counts may only shrink.
// Registers as a sibling test module so `cargo test -p DraftFlowCAD style::`
// picks it up; see D:\tmp\kit-baseline.csv for the audited baseline.
#[cfg(test)]
mod kit_contract_tests;

#[cfg(test)]
mod common_tests {
    // The `common::muted_style` forwarder was deleted in the 36th freeze
    // (2026-10-21): the 25 files that imported it and the full-path call sites
    // now name `ui::kit::muted_style` directly. The one-ink contract stays pinned
    // by the muted-style sweep in `theme_accessibility_tests`.

    #[test]
    fn the_plotstyle_button_forwarder_stays_deleted() {
        // This test used to be a tautology: `let divergent = true; assert!(…)`,
        // documenting that two `btn_s` locals differed and must not be merged. Both
        // of those locals have since been deleted — `style_manager.rs`'s on screen 21,
        // `plotstyle.rs`'s on screen 23, each drained onto `kit::btn` — so the premise
        // the boolean was standing in for no longer exists. What is worth asserting
        // now is the deletion itself, read out of the sources rather than from a
        // constant: the acceptance for this mission is that the old implementation is
        // gone, not that a new layer was added beside it.
        let plotstyle = include_str!("plotstyle.rs");
        let style_manager = include_str!("style_manager.rs");
        let layout_manager = include_str!("../window/layout_manager.rs");
        assert!(
            !plotstyle.contains("fn btn_s"),
            "plotstyle.rs re-grew a private button-plate forwarder after screen 23 deleted it"
        );
        assert!(
            !style_manager.contains("fn btn_s"),
            "style_manager.rs re-grew a private button-plate forwarder after screen 21 deleted it"
        );
        assert!(
            plotstyle.contains("kit::btn("),
            "plotstyle.rs's plates must come from the L1 primitive that takes role, size and press"
        );
        // The third local went this run, inside the parallel batch that drained 21
        // screens' hand-typed plates: layout_manager.rs now calls `kit::button_style`
        // and `kit::chip_button_style` directly. So the shape is gone from all three
        // files that ever carried it, and the claim is now the stronger one — a
        // re-grown forwarder in any of the three reads red, and the drained file must
        // still get its plates from L1.
        assert!(
            !layout_manager.contains("fn btn_s"),
            "layout_manager.rs re-grew a private button-plate forwarder after this run deleted it"
        );
        assert!(
            layout_manager.contains("kit::button_style("),
            "layout_manager.rs's plates must come from the L1 button style that takes the accent flag"
        );
    }
}
