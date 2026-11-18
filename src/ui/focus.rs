//! Keyboard focus for self-drawn chrome.
//!
//! iced's own focus system only reaches widgets that implement `Operation::focusable`
//! (text inputs, scrollables). Every plate in this app is a `button` inside a
//! canvas-heavy layout, and Tab is already spoken for by dynamic input and the
//! command line's autocomplete. So focus traversal is kept here instead: the
//! widget constructors enroll themselves in paint order during `view`, and the
//! Tab handler walks that list when — and only when — nothing upstream wants
//! the key.
//!
//! Two rules shape the registry:
//!
//! * **It is rebuilt every frame.** `begin_pass` clears it, so a control that
//!   scrolled out of view or a dialog that closed stops being a tab stop
//!   without anyone having to unregister it.
//! * **The highest populated scope wins.** With a modal up, Tab cycles the
//!   modal only; the chrome behind it is unreachable. That is the focus trap,
//!   and it falls out of the ordering rather than being a special case.
//!
//! The ring is drawn by `kit::control::style_for` from `ring(track)`. Controls
//! built without a track (the raw `button(...)` call sites) enroll nothing and
//! read zero: they are outside this system until they migrate to the kit.

use std::cell::{Cell, RefCell};

use crate::ui::motion;

/// Which layer of the composition a control belongs to. Declaration order is
/// precedence order: a modal traps focus away from the chrome beneath it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    Chrome,
    Overlay,
    Modal,
}

/// What the person last used. A pointer click hides the ring without losing
/// the position, so the next Tab continues from where they were.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modality {
    Pointer,
    Keyboard,
}

struct Registry {
    entries: Vec<(&'static str, Scope)>,
    scope: Scope,
    current: Option<&'static str>,
    modality: Modality,
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry {
        entries: Vec::new(),
        scope: Scope::Chrome,
        current: None,
        modality: Modality::Pointer,
    });
    /// The stop that opened the dialog now on top, captured by [`remember`].
    static RECALL: Cell<Option<&'static str>> = const { Cell::new(None) };
    /// A stop waiting to be reapplied, with the pass it belongs to. The frame
    /// number is what keeps a target that never comes back from landing on
    /// some far-future frame by accident.
    static RESTORE: Cell<Option<(&'static str, u32)>> = const { Cell::new(None) };
    static FRAME: Cell<u32> = const { Cell::new(0) };
}

/// Starts a frame. Call once, at the top of the main view, before any widget
/// is built. Keeps the current stop and the modality across frames.
pub fn begin_pass() {
    let frame = FRAME.with(|f| {
        let n = f.get().wrapping_add(1);
        f.set(n);
        n
    });
    // A restore is good for exactly one pass: the next view must build the
    // control, or the stop is gone for good.
    RESTORE.with(|r| {
        if let Some((_, at)) = r.get() {
            if at != frame {
                r.set(None);
            }
        }
    });
    REGISTRY.with(|r| {
        let mut reg = r.borrow_mut();
        reg.entries.clear();
        reg.scope = Scope::Chrome;
    });
}

/// Declares which layer the controls built from here on belong to.
pub fn set_scope(scope: Scope) {
    REGISTRY.with(|r| r.borrow_mut().scope = scope);
}

/// Adds a tab stop in paint order. Empty tracks are ignored: they are the
/// un-migrated controls, and giving them a shared key would light them all up
/// at once.
pub fn enroll(track: &'static str) {
    if track.is_empty() {
        return;
    }
    let recalled = RESTORE.with(|r| {
        let wanted = r.get().filter(|(key, _)| *key == track);
        if wanted.is_some() {
            r.set(None);
        }
        wanted.is_some()
    });
    REGISTRY.with(|r| {
        let mut reg = r.borrow_mut();
        if reg.entries.iter().any(|(k, _)| *k == track) {
            return;
        }
        let scope = reg.scope;
        reg.entries.push((track, scope));
        if recalled {
            reg.current = Some(track);
            if reg.modality == Modality::Keyboard {
                motion::set(motion::focus_key(track), 1.0, motion::HOVER);
            }
        }
    });
}

/// Whether the active scope has anything to focus. The Tab handler uses this
/// to decide between traversing chrome and falling through to dynamic input.
pub fn has_focusables() -> bool {
    REGISTRY.with(|r| !r.borrow().entries.is_empty())
}

/// Moves to the next (`forward`) or previous stop within the highest
/// populated scope. Returns false when there is nothing to traverse, which
/// leaves the key for whoever owns it upstream.
pub fn advance(forward: bool) -> bool {
    REGISTRY.with(|r| {
        let mut reg = r.borrow_mut();
        let Some(scope) = reg.entries.iter().map(|(_, s)| *s).max() else {
            return false;
        };
        let stops: Vec<&'static str> = reg
            .entries
            .iter()
            .filter(|(_, s)| *s == scope)
            .map(|(k, _)| *k)
            .collect();
        if stops.is_empty() {
            return false;
        }
        let at = reg.current.and_then(|c| stops.iter().position(|k| *k == c));
        let next = match at {
            Some(i) if forward => (i + 1) % stops.len(),
            Some(i) => (i + stops.len() - 1) % stops.len(),
            None if forward => 0,
            None => stops.len() - 1,
        };
        if let Some(old) = reg.current {
            if old != stops[next] {
                motion::set(motion::focus_key(old), 0.0, 0);
            }
        }
        reg.current = Some(stops[next]);
        reg.modality = Modality::Keyboard;
        motion::set(motion::focus_key(stops[next]), 1.0, motion::HOVER);
        true
    })
}

/// The control the ring is on, if any.
pub fn current() -> Option<&'static str> {
    REGISTRY.with(|r| r.borrow().current)
}

/// Ring strength for `track`, 0.0–1.0. Zero for a pointer session, for a stop
/// that is not current, and for every un-migrated control.
pub fn ring(track: &'static str) -> f32 {
    if track.is_empty() {
        return 0.0;
    }
    REGISTRY.with(|r| {
        let reg = r.borrow();
        if reg.modality != Modality::Keyboard || reg.current != Some(track) {
            return 0.0;
        }
        motion::value(motion::focus_key(track))
    })
}

/// Records what the person just used. Switching to the pointer hides the ring
/// immediately; switching back to the keyboard brings it back on the same stop.
pub fn set_modality(modality: Modality) {
    REGISTRY.with(|r| {
        let mut reg = r.borrow_mut();
        if reg.modality == modality {
            return;
        }
        reg.modality = modality;
        if let Some(key) = reg.current {
            let target = if modality == Modality::Keyboard {
                1.0
            } else {
                0.0
            };
            let ms = if modality == Modality::Keyboard {
                motion::HOVER
            } else {
                0
            };
            motion::set(motion::focus_key(key), target, ms);
        }
    });
}

/// Captures the stop that opened whatever is about to cover the chrome.
///
/// A dialog takes the keyboard and gets its own controls enrolled, so `current`
/// moves into the dialog and the trigger is forgotten. Call this once, when the
/// dialog opens; [`restore`] puts the person back.
pub fn remember() {
    let key = current();
    RECALL.with(|r| r.set(key));
}

/// Queues the remembered stop for the next pass that builds it.
///
/// Closing the dialog restarts the pass the chrome is rebuilt in, and the
/// control is enrolled again in paint order — so the restore rides on the
/// enrollment instead of needing a second lookup. It is good for exactly one
/// pass: if the trigger is gone (its panel collapsed, its tab closed) the stop
/// dies with it rather than landing on a stranger later.
pub fn restore() {
    let Some(key) = RECALL.with(|r| r.take()) else {
        return;
    };
    let at = FRAME.with(|f| f.get()).wrapping_add(1);
    RESTORE.with(|r| r.set(Some((key, at))));
}

/// Drops a queued restore without spending it.
///
/// Closing one dialog straight into another keeps a sheet on screen, so the
/// stop must not travel back to the trigger behind it.
pub fn cancel_restore() {
    RESTORE.with(|r| r.set(None));
    RECALL.with(|r| r.set(None));
}

/// Drops the stop without forgetting the modality. Escape and modal close
/// call this so a ring cannot be left glowing on a control that just went away.
pub fn clear() {
    REGISTRY.with(|r| {
        let mut reg = r.borrow_mut();
        if let Some(key) = reg.current.take() {
            motion::set(motion::focus_key(key), 0.0, 0);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset() {
        begin_pass();
        clear();
        set_modality(Modality::Pointer);
    }

    #[test]
    fn traversal_follows_paint_order_and_wraps() {
        reset();
        enroll("focus-test:a");
        enroll("focus-test:b");
        enroll("focus-test:c");
        assert!(advance(true));
        assert_eq!(current(), Some("focus-test:a"));
        advance(true);
        assert_eq!(current(), Some("focus-test:b"));
        advance(true);
        advance(true);
        assert_eq!(current(), Some("focus-test:a"), "the list must wrap");
        advance(false);
        assert_eq!(current(), Some("focus-test:c"));
    }

    #[test]
    fn a_modal_traps_focus_away_from_the_chrome() {
        reset();
        enroll("focus-test:chrome");
        set_scope(Scope::Overlay);
        enroll("focus-test:overlay");
        set_scope(Scope::Modal);
        enroll("focus-test:dialog-ok");
        enroll("focus-test:dialog-cancel");
        advance(true);
        assert_eq!(current(), Some("focus-test:dialog-ok"));
        advance(true);
        assert_eq!(current(), Some("focus-test:dialog-cancel"));
        advance(true);
        assert_eq!(
            current(),
            Some("focus-test:dialog-ok"),
            "Tab must not escape the dialog"
        );
    }

    #[test]
    fn the_ring_is_keyboard_only_and_never_lights_an_empty_track() {
        reset();
        motion::set_enabled(false);
        enroll("focus-test:ringed");
        advance(true);
        assert_eq!(ring("focus-test:ringed"), 1.0);
        assert_eq!(ring("focus-test:other"), 0.0);
        assert_eq!(ring(""), 0.0, "un-migrated controls stay outside the ring");
        set_modality(Modality::Pointer);
        assert_eq!(ring("focus-test:ringed"), 0.0);
        assert_eq!(current(), Some("focus-test:ringed"), "the stop is kept");
        set_modality(Modality::Keyboard);
        assert_eq!(ring("focus-test:ringed"), 1.0);
    }

    #[test]
    fn an_empty_pass_leaves_the_key_to_dynamic_input() {
        reset();
        assert!(!has_focusables());
        assert!(!advance(true));
        enroll("focus-test:only");
        assert!(has_focusables());
        begin_pass();
        assert!(!has_focusables(), "a frame with no controls unregisters them");
        assert!(!advance(true));
    }

    /// A dialog takes the keyboard; closing it has to hand the stop back to
    /// whichever control opened it, not to the top of the chrome.
    #[test]
    fn the_trigger_is_restored_when_the_dialog_closes() {
        reset();
        enroll("focus-test:trigger");
        enroll("focus-test:neighbour");
        advance(true);
        assert_eq!(current(), Some("focus-test:trigger"));
        remember();
        // The dialog's own controls enroll and take the ring.
        set_scope(Scope::Modal);
        enroll("focus-test:dialog-ok");
        advance(true);
        assert_eq!(current(), Some("focus-test:dialog-ok"));
        set_scope(Scope::Chrome);
        // Dialog gone: the chrome is rebuilt next pass, the trigger with it.
        restore();
        begin_pass();
        enroll("focus-test:trigger");
        enroll("focus-test:neighbour");
        assert_eq!(
            current(),
            Some("focus-test:trigger"),
            "focus must land back on the control that opened the dialog"
        );
    }

    #[test]
    fn a_restored_stop_stays_dark_for_a_pointer_session() {
        reset();
        enroll("focus-test:pointer-trigger");
        advance(true);
        remember();
        set_modality(Modality::Pointer);
        restore();
        begin_pass();
        enroll("focus-test:pointer-trigger");
        assert_eq!(current(), Some("focus-test:pointer-trigger"));
        assert_eq!(ring("focus-test:pointer-trigger"), 0.0, "pointer shows no ring");
    }

    #[test]
    fn a_restore_is_good_for_one_pass() {
        reset();
        enroll("focus-test:vanished");
        advance(true);
        remember();
        restore();
        begin_pass();
        // The control was not built this pass: the stop dies with it.
        begin_pass();
        clear();
        enroll("focus-test:vanished");
        assert_eq!(current(), None, "a stale restore must not resurface");
    }

    #[test]
    fn without_a_remember_there_is_nothing_to_restore() {
        reset();
        begin_pass();
        restore();
        begin_pass();
        enroll("focus-test:plain");
        assert_eq!(current(), None);
    }

    #[test]
    fn clear_drops_the_ring() {
        reset();
        motion::set_enabled(false);
        enroll("focus-test:cleared");
        advance(true);
        clear();
        assert_eq!(current(), None);
        assert_eq!(ring("focus-test:cleared"), 0.0);
    }
}