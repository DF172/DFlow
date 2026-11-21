//! Selection-cycling list box — shown at the cursor when a click lands on
//! two or more overlapping objects. Each row shows the object's drawn color
//! swatch, its type, and (right-aligned) its layer; clicking a row adds that
//! object to the current selection. Clicking outside dismisses it.

use crate::ui::kit;
use crate::ui::kit::plain_button;
use iced::alignment::Horizontal;
use iced::widget::{column, container, mouse_area, opaque, row, rule, space, text};
use iced::{Element, Fill, Length, Theme};

use crate::app::Message;
use crate::ui::style::tokens;

/// Row content for one overlapping candidate.
pub struct CycleCandidate {
    pub handle: codec::Handle,
    pub type_name: String,
    pub layer: String,
    /// The color the object is drawn with (layer-inherited when ByLayer).
    pub color: [f32; 4],
}

/// Full-canvas overlay: the list box anchored at `anchor` (canvas
/// coordinates) plus a transparent click-catcher that cancels.
///
/// P2 batch 1 item 15: `hover` / `fade_old` / `t` carry the running candidate
/// migration (the row the pointer is on, the row it just left, and the eased
/// share of the 60 ms cross-fade). Passing them in keeps the overlay a pure
/// function of scene state — the overlay itself never starts a track.
pub fn cycle_popup_overlay(
    anchor: iced::Point,
    items: Vec<CycleCandidate>,
    theme: &Theme,
    hover: Option<codec::Handle>,
    fade_old: Option<codec::Handle>,
    t: f32,
) -> Element<'static, Message> {
    // The type column and the layer column get one shared width each (the
    // widest entry of their kind, clamped) so the separators line up.
    let type_w = clamp_col_width(
        items.iter().map(|c| c.type_name.chars().count()),
        11.0,
        40.0,
        96.0,
    );
    let layer_w = clamp_col_width(
        items.iter().map(|c| c.layer.chars().count()),
        10.0,
        28.0,
        84.0,
    );

    let t = t.clamp(0.0, 1.0);
    // Resolve the two inks once for the whole box: the accent needs a theme
    // contrast solve, and the rows below only mix between these two.
    let (base_ink, accent_ink) = cycle_inks(theme);
    let rows: Vec<Element<'static, Message>> = items
        .into_iter()
        .map(|c| {
            // The arriving row eases up to full accent, the departing one
            // eases back to resting ink; every other row is untouched. With
            // no migration running `fade_old` is `None` and the held row sits
            // at its resting weight, so nothing is drawn differently.
            let ink = row_weight(c.handle, hover, fade_old, t);
            item_row(
                c.handle,
                c.type_name,
                c.layer,
                c.color,
                type_w,
                layer_w,
                ink,
                base_ink,
                accent_ink,
            )
        })
        .collect();

    let plate = container(column(rows))
        .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
        .width(Length::Fixed(type_w + layer_w + 42.0));
    let panel = kit::glass_pane(plate.into(), theme, false, "");

    let positioned = iced::widget::pin(opaque(panel))
        .position(iced::Point::new(anchor.x.max(0.0), anchor.y.max(0.0)));

    mouse_area(positioned).on_press(Message::CycleCancel).into()
}

/// Column width for the widest label, using a per-glyph estimate with a
/// minimum so short labels don't collapse and a maximum so long ones clip.
fn clamp_col_width(labels: impl Iterator<Item = usize>, glyph_w: f32, min: f32, max: f32) -> f32 {
    let widest = labels.max().unwrap_or(0) as f32 * glyph_w;
    widest.clamp(min, max)
}

/// Highlight weight of one candidate row: `t` on the row the pointer is on,
/// `1 - t` on the row it is leaving, zero on every other row.
///
/// P2 batch 1 item 15: the two weights always sum to 1, so the migration reads
/// as one highlight sliding down the list instead of two fades overlapping.
#[must_use]
pub fn row_weight(
    handle: codec::Handle,
    hover: Option<codec::Handle>,
    fade_old: Option<codec::Handle>,
    t: f32,
) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if Some(handle) == hover {
        t
    } else if Some(handle) == fade_old {
        1.0 - t
    } else {
        0.0
    }
}

/// The two inks a candidate row can carry: the resting text ink, and the
/// accent the arriving row climbs to.
///
/// Resolving the accent costs a theme contrast solve (measured at ≈0.3 ms in
/// a debug build), so this runs ONCE per overlay build and the rows only mix —
/// see [`row_ink_mix`]. Calling it per row would spend milliseconds per frame
/// on a list that has ten rows.
#[must_use]
pub fn cycle_inks(theme: &Theme) -> (iced::Color, iced::Color) {
    let base = theme.palette().background.base.text;
    let accent = kit::accent_style(theme).color.unwrap_or(base);
    (base, accent)
}

/// The type-name ink for a row at highlight weight `w` (0 = resting, 1 =
/// accent), mixed from the two inks [`cycle_inks`] already resolved.
///
/// P2 batch 1 item 15: the two ends are mixed PER CHANNEL and the resting
/// alpha is carried through untouched. Alpha cannot carry this migration —
/// `transparency_enable` defaults off, so alpha is forced back to 1 at draw
/// time and a fade written as alpha would render as no change at all.
#[must_use]
pub fn row_ink_mix(base: iced::Color, accent: iced::Color, w: f32) -> iced::Color {
    let w = w.clamp(0.0, 1.0);
    iced::Color::from_rgba(
        base.r + (accent.r - base.r) * w,
        base.g + (accent.g - base.g) * w,
        base.b + (accent.b - base.b) * w,
        base.a,
    )
}

fn item_row(
    handle: codec::Handle,
    type_name: String,
    layer: String,
    color: [f32; 4],
    type_w: f32,
    layer_w: f32,
    ink: f32,
    base_ink: iced::Color,
    accent_ink: iced::Color,
) -> Element<'static, Message> {
    let swatch = container(
        space::Space::new()
            .width(Length::Fixed(9.0))
            .height(Length::Fixed(9.0)),
    )
    .style(move |theme: &Theme| {
        kit::swatch_style(
            theme,
            iced::Color::from_rgba(color[0], color[1], color[2], color[3]),
            "",
        )
    });

    let content = row![
        swatch,
        container(
            text(type_name)
                .size(tokens::TEXT_LABEL)
                .style(move |_: &Theme| kit::color_style(row_ink_mix(base_ink, accent_ink, ink)))
        )
        .width(Length::Fixed(type_w))
        .align_y(iced::alignment::Vertical::Center),
        container(rule::vertical(1)).height(Length::Fixed(12.0)),
        container(
            text(layer)
                .size(tokens::TEXT_MICRO)
                .align_x(Horizontal::Right)
                .style(|theme: &Theme| {
                    kit::color_style(theme.palette().background.base.text.scale_alpha(0.72))
                })
        )
        .width(Length::Fixed(layer_w))
        .align_y(iced::alignment::Vertical::Center),
    ]
    .spacing(tokens::GAP_CONTROL)
    .align_y(iced::Center);

    let btn = plain_button(content)
        .on_press(Message::CycleSelect(handle))
        .style(kit::subtle_style)
        .width(Fill)
        .padding(kit::PAD_TRIGGER);
    // Highlight the underlying object while the cursor is over this row.
    mouse_area(btn)
        .on_enter(Message::CycleHover(Some(handle)))
        .on_exit(Message::CycleHoverExit(handle))
        .into()
}

#[cfg(test)]
mod cycle_row_ink_tests {
    use super::*;

    /// Item 15: the type-name highlight migrates as an RGB mix, and alpha is
    /// carried through untouched — with `transparency_enable` off an alpha
    /// fade would be forced back to 1 and the migration would be invisible.
    #[test]
    fn cycle_row_ink_mixes_rgb_and_hands_the_weight_over() {
        let theme = Theme::Dark;
        let (base_ink, accent_ink) = cycle_inks(&theme);
        let base = theme.palette().background.base.text;
        let rest = row_ink_mix(base_ink, accent_ink, 0.0);
        let lit = row_ink_mix(base_ink, accent_ink, 1.0);

        assert_eq!(
            rest.a, base.a,
            "resting alpha must survive: alpha cannot carry this migration"
        );
        assert_eq!(lit.a, base.a, "lit alpha must stay at the resting value");
        assert!(
            rest != lit,
            "the two ends must differ, or the migration would be a no-op"
        );
        // A midpoint must be the per-channel average of the two ends.
        let half = row_ink_mix(base_ink, accent_ink, 0.5);
        assert!((half.r - (rest.r + lit.r) * 0.5).abs() < 1e-6);
        assert!((half.g - (rest.g + lit.g) * 0.5).abs() < 1e-6);
        assert!((half.b - (rest.b + lit.b) * 0.5).abs() < 1e-6);

        // The weight hand-over: the arriving row climbs to 1 while the
        // departing row falls to 0, and no other row moves at all.
        let old = codec::Handle::new(1);
        let new = codec::Handle::new(2);
        let other = codec::Handle::new(3);
        assert_eq!(row_weight(old, Some(new), Some(old), 0.0), 1.0);
        assert_eq!(row_weight(new, Some(new), Some(old), 0.0), 0.0);
        assert_eq!(row_weight(old, Some(new), Some(old), 1.0), 0.0);
        assert_eq!(row_weight(new, Some(new), Some(old), 1.0), 1.0);
        for step in 0..=4 {
            let t = step as f32 / 4.0;
            let sum =
                row_weight(old, Some(new), Some(old), t) + row_weight(new, Some(new), Some(old), t);
            assert!((sum - 1.0).abs() < 1e-6, "the pair must always sum to 1");
            assert_eq!(
                row_weight(other, Some(new), Some(old), t),
                0.0,
                "a row outside the migration must stay at resting ink"
            );
        }

        // With no hover and no departing row — the settled / no-candidate
        // state — every row reads resting ink, so nothing is drawn differently.
        assert_eq!(row_weight(old, None, None, 0.0), 0.0);
        assert_eq!(row_weight(new, None, None, 1.0), 0.0);

        // Probe A: the per-row cost a switching frame pays — one weight lookup
        // plus one channel mix. That is the whole of item 15's per-row work.
        let t0 = std::time::Instant::now();
        let mut acc = 0.0f32;
        for k in 0..200_000u32 {
            let t = (k % 60) as f32 / 60.0;
            acc += row_ink_mix(
                base_ink,
                accent_ink,
                row_weight(new, Some(new), Some(old), t),
            )
            .r;
        }
        let per_row_ns = t0.elapsed().as_secs_f64() * 1e9 / 200_000.0;

        // Probe B: the cost of resolving the two inks, paid ONCE per overlay
        // build (not per row). This is why `cycle_inks` exists.
        let t1 = std::time::Instant::now();
        for _ in 0..50 {
            let (b, a) = cycle_inks(&theme);
            acc += b.r + a.r;
        }
        let inks_once_us = t1.elapsed().as_secs_f64() * 1e6 / 50.0;
        assert!(acc > 0.0, "keep the loops observable");
        println!("CYCLEINK mix_ns_per_row={per_row_ns:.2} inks_once_us={inks_once_us:.2}");
    }
}
