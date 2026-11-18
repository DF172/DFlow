// Replacement A: the plate never offers an exit. It shows a static notice,
// leaves on the normal schedule, and the graphics warning dialog carries both
// exits afterwards. So exactly two Message arms may restart the editor, and the
// dialog must wire exactly those two.
//
// Lives outside `mod.rs` so `include_str!("mod.rs")` cannot match these
// assertions and inflate the counts it is checking.

#[test]
fn failure_page_has_both_exits_and_no_third_hidden_path() {
    let update = include_str!("mod.rs");
    assert!(
        update.contains("Message::GpuWarningSilence =>"),
        "the scan must cover the update table"
    );
    assert_eq!(
        update.matches("restart_with_gpu_choice(").count(),
        2,
        "only the retry and safe-mode arms may restart the editor"
    );
    assert!(update.contains("Message::GpuRetry => self.restart_with_gpu_choice(&[])"));
    assert!(update.contains("Message::GpuSafeMode => self.restart_with_gpu_choice(&[\"--safe-mode\"])"));
    assert_eq!(
        update.matches("\"--safe-mode\"").count(),
        1,
        "only one exit asks for safe mode"
    );

    let modal = include_str!("../view/modal.rs");
    assert_eq!(
        modal.matches("Message::GpuRetry").count(),
        1,
        "the dialog must offer exactly one retry button"
    );
    assert_eq!(
        modal.matches("Message::GpuSafeMode").count(),
        1,
        "the dialog must offer exactly one safe-mode button"
    );
    assert!(
        modal.contains("Message::GpuWarningSilence"),
        "the session-level silence exit must stay wired"
    );
}
