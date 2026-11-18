//! L2 shell: the frame machinery that composes kit primitives and decides no
//! colour of its own.
//!
//! The layer boundary was drawn from measurement, not preference. The sizing
//! contract (`Sizing`, `intrinsic`) is named by 41 files across `src/ui/window/`,
//! `src/app/view/` and `src/ui/` — every dialog body threads it through its
//! `view_window` signature. The dialog chrome is named by 4 files at 14
//! positions (`modal` 5, `ModalOptions` 6, `discard_guard` 2, `backdrop` 1).
//! Neither group paints, and neither group is a screen, so both belong to the
//! shell rather than to a flat `ui::modal` that mixed them.
//!
//! `src/ui/modal.rs` is deleted, not aliased. There is no `ui::modal` path left
//! to keep the old name alive: the type formerly called `ModalSizing` is
//! `shell::Sizing`, and the callers were moved to it. Like `kit`, this module
//! names its public surface explicitly, so a new shell primitive only becomes
//! reachable once it appears in the `pub use` list below.

pub mod dialog;
pub mod frame;

pub use dialog::{backdrop, discard_guard, modal, ModalFooter, ModalOptions};
pub use frame::{intrinsic, Sizing};
