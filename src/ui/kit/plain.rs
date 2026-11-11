//! The plain-button choke point: every raw `iced::widget::button(...)` call
//! site in the tree is migrated to [`plain_button`] so the 210 controls that
//! B0+B1 deliberately isolated finally join the motion gate, the 1 px press
//! sink and the keyboard focus ring — without touching a single style closure
//! or padding value at the call site.
//!
//! The key is free: `#[track_caller]` makes `Location::caller()` resolve to the
//! *call site*, so `file:line` is a stable, unique identity per control with no
//! hand-written ids. [`crate::ui::motion::site_key`] interns it once; the map is
//! bounded by the number of call sites, never by frames or rows.
//!
//! Assembly is deferred to `Into<Element>` so the caller's chained
//! `width / height / padding / clip / on_press / style` all land on the real
//! `Button` unchanged. The caller's style closure is *wrapped*, not replaced:
//! the wrapper latches the press and paints the ring exactly the way
//! `kit::control::style_for` does for the 187 kit controls, so both populations
//! share one visual grammar and one set of constants.
//!
//! A site with no `on_press` (the 37 disabled / display-only controls) enrolls
//! no tab stop and wears no ring: giving a dead control a focus ring would
//! advertise an action that does not exist.

use crate::ui::style::draftflow_theme::draftflow_colors;
use crate::ui::style::tokens;
use crate::ui::{focus, motion};
use iced::widget::button::{self, Status, Style};
use iced::{Element, Length, Padding, Theme};

/// A raw button awaiting assembly. Holds the caller's chained arguments so the
/// final `Button` is built once, with the site wrapper already in place.
pub struct PlainButton<'a, Message: 'a> {
    content: Element<'a, Message>,
    key: &'static str,
    width: Option<Length>,
    height: Option<Length>,
    padding: Option<Padding>,
    clip: bool,
    on_press: Option<Message>,
    style: Option<Box<dyn Fn(&Theme, Status) -> Style + 'a>>,
}

/// Drop-in replacement for `iced::widget::button` at migrated call sites.
///
/// The caller keeps chaining exactly as before; nothing else changes.
#[track_caller]
pub fn plain_button<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> PlainButton<'a, Message> {
    let loc = std::panic::Location::caller();
    let key = motion::site_key(loc.file(), loc.line());
    PlainButton {
        content: content.into(),
        key,
        width: None,
        height: None,
        padding: None,
        clip: false,
        on_press: None,
        style: None,
    }
}

impl<'a, Message: 'a> PlainButton<'a, Message> {
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = Some(height.into());
        self
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = Some(padding.into());
        self
    }

    pub fn clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }

    pub fn on_press(mut self, on_press: Message) -> Self {
        self.on_press = Some(on_press);
        self
    }

    pub fn on_press_maybe(mut self, on_press: Option<Message>) -> Self {
        self.on_press = on_press;
        self
    }

    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self {
        self.style = Some(Box::new(style));
        self
    }
}

/// The shared site paint: latch the press, then lift the border toward the
/// accent ring. Identical arithmetic to `kit::control::style_for` so the two
/// button populations cannot drift apart.
pub fn apply_site(key: &'static str, mut style: Style, theme: &Theme, status: Status) -> Style {
    if key.is_empty() {
        return style;
    }
    let pressed = matches!(status, Status::Pressed);
    let disabled = matches!(status, Status::Disabled);
    motion::press(motion::press_key(key), pressed && !disabled);

    let ring = focus::ring(key);
    if ring > 0.0 && !disabled {
        if let Some(colors) = draftflow_colors(theme) {
            style.border.color = motion::mix(style.border.color, colors.accent, ring);
            let weight = style.border.width.max(tokens::FOCUS_RING);
            style.border.width += (weight - style.border.width) * ring;
        }
    }
    style
}

impl<'a, Message: 'a + Clone> From<PlainButton<'a, Message>> for Element<'a, Message> {
    fn from(p: PlainButton<'a, Message>) -> Self {
        let key = p.key;
        // Only a live control earns a tab stop; enrolling here (assembly is
        // still view-build time) keeps paint order identical to the kit path.
        if p.on_press.is_some() {
            focus::enroll(key);
        }
        let content = if key.is_empty() {
            p.content
        } else {
            super::press::sink(p.content, motion::press_key(key)).into()
        };
        let mut b = iced::widget::button(content);
        if let Some(w) = p.width {
            b = b.width(w);
        }
        if let Some(h) = p.height {
            b = b.height(h);
        }
        if let Some(pad) = p.padding {
            b = b.padding(pad);
        }
        if p.clip {
            b = b.clip(true);
        }
        if let Some(m) = p.on_press {
            b = b.on_press(m);
        }
        let user = p.style;
        b.style(move |theme, status| {
            let base = match &user {
                Some(f) => f(theme, status),
                None => button::primary(theme, status),
            };
            apply_site(key, base, theme, status)
        })
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::text;

    #[test]
    fn a_site_key_is_stable_across_frames() {
        let a = motion::site_key("src/ui/x.rs", 10);
        let b = motion::site_key("src/ui/x.rs", 10);
        let c = motion::site_key("src/ui/x.rs", 11);
        assert!(std::ptr::eq(a, b), "the same site must intern to one key");
        assert!(!std::ptr::eq(a, c), "two sites must not share a key");
    }

    #[test]
    fn a_disabled_site_never_wears_the_ring() {
        focus::begin_pass();
        focus::set_modality(focus::Modality::Keyboard);
        let key = motion::site_key("test:disabled", 1);
        let dead: Element<'_, ()> = plain_button(text("x")).into(); // no on_press
        std::hint::black_box(&dead);
        assert_eq!(focus::ring(key), 0.0, "a dead control must not light up");
        assert!(!focus::has_focusables(), "a dead control must not enroll a stop");
    }

    #[test]
    fn the_ring_is_drawn_outside_the_padding_box() {
        focus::begin_pass();
        focus::set_modality(focus::Modality::Keyboard);
        let key = motion::site_key("test:ring", 2);
        focus::enroll(key);
        focus::advance(true);
        let base = Style {
            border: iced::Border {
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let painted = apply_site(key, base, &Theme::Dark, Status::Active);
        assert!(
            painted.border.width >= tokens::FOCUS_RING,
            "the ring must reach the outer border, not the padded content: {}",
            painted.border.width
        );
    }

    #[test]
    fn the_wrapper_keeps_the_button_size() {
        let plain: Element<'_, ()> = iced::widget::button(text("Ok")).width(80.0).height(24.0).into();
        let wrapped: Element<'_, ()> = plain_button(text("Ok")).width(80.0).height(24.0).into();
        assert_eq!(plain.as_widget().size(), wrapped.as_widget().size());
    }

    #[test]
    fn the_gate_off_lands_the_sink_in_the_same_frame() {
        motion::set_enabled(false);
        let key = motion::site_key("test:gate", 3);
        motion::press(motion::press_key(key), true);
        assert_eq!(motion::value(motion::press_key(key)), 1.0);
        motion::press(motion::press_key(key), false);
        assert_eq!(motion::value(motion::press_key(key)), 0.0);
        motion::set_enabled(true);
    }
}
