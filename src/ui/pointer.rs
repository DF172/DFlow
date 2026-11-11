//! Where the pointer last was, in window space.
//!
//! `update` never sees a cursor position — iced hands events to widgets, and
//! the app only hears about the ones the subscription turns into messages. The
//! dialog entrance needs a position at the moment `open_modal` runs (which
//! control the person just used), so one passive listener records every
//! pointer move and the message handler asks for it.
//!
//! This is one `Point` for the whole process, not per window: the native build
//! is a single main window plus in-canvas dialogs, and the web build has one
//! surface. A second OS window would need a key here, and there is none.

use std::cell::Cell;

use iced::{mouse, Event, Point};

thread_local! {
    static LAST: Cell<Option<Point>> = const { Cell::new(None) };
}

/// Records a pointer move. Every other event is ignored.
pub fn note_event(event: &Event) {
    if let Event::Mouse(mouse::Event::CursorMoved { position }) = event {
        LAST.with(|p| p.set(Some(*position)));
    }
}

/// The last pointer position, or `None` before the first move.
pub fn last() -> Option<Point> {
    LAST.with(|p| p.get())
}

/// Forgets the pointer. Tests use this so a stale position cannot leak into
/// the next case; nothing in the running app needs it.
pub fn forget() {
    LAST.with(|p| p.set(None));
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::mouse::Button;

    #[test]
    fn a_move_is_remembered() {
        forget();
        note_event(&Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(120.0, 640.0),
        }));
        assert_eq!(last(), Some(Point::new(120.0, 640.0)));
    }

    #[test]
    fn other_events_do_not_move_the_pointer() {
        forget();
        note_event(&Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(4.0, 5.0),
        }));
        note_event(&Event::Mouse(mouse::Event::ButtonPressed(Button::Left)));
        note_event(&Event::Mouse(mouse::Event::ButtonReleased(Button::Left)));
        assert_eq!(last(), Some(Point::new(4.0, 5.0)));
    }
}
