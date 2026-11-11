use crate::ui::kit::{self, PAD_PLATE, Press, Role, Size, Tier};
use crate::ui::style::tokens;
use super::super::document::{DynComponent, DynFieldEntry};
use super::super::Message;
use super::*;
use iced::widget::{column, container, mouse_area, row, text, tooltip};
use iced::{Element, Length, Theme};
use std::time::Duration;

fn viewport_tooltip<'a>(
    control: impl Into<Element<'a, Message>>,
    title: String,
    command: &'static str,
    theme: &Theme,
) -> Element<'a, Message> {
    let text = format!("{title}\n{} {command}", crate::t!("Command:"));
    // The tip's plate is glass: it reads the strip and the drawing under it
    // instead of laying an opaque card over them. The widget's own inset goes to
    // zero so the pane hugs the ink, and `PAD_PLATE` becomes the blank inside
    // the plate — the same rung every other floating surface uses.
    tooltip(
        control,
        kit::glass_pane(
            kit::surface(
                crate::ui::ribbon::tooltip_content(text),
                Tier::Flush,
                kit::PAD_PLATE,
                "",
            )
            .into(),
            theme,
            false,
            "",
        ),
        tooltip::Position::Bottom,
    )
    .gap(6.0)
    .padding(tokens::GAP_NONE)
    .delay(Duration::from_millis(400))
    .style(|theme: &Theme| Tier::Flush.style(theme, ""))
    .into()
}

pub(super) fn viewport_controls<'a>(
    render_mode: codec::entities::ViewportRenderMode,
    show_grid: bool,
    snap_on: bool,
    include_split: bool,
    tile_count: usize,
    render_mode_menu_open: bool,
    render_mode_preview: Option<codec::entities::ViewportRenderMode>,
    theme: &Theme,
) -> Element<'a, Message> {
    let render_modes: Vec<RenderModeChoice> = crate::modules::view::visual_style::VISUAL_STYLES
        .iter()
        .map(|style| RenderModeChoice(style.mode))
        .collect();
    let danger_btn =
        move |bytes: &'static [u8], msg: Message, title: String, command: &'static str| {
            let button = kit::btn(
                crate::ui::icons::themed_danger(bytes, 15.0),
                Role::Danger,
                Size::Compact,
                Press::On(msg),
                "",
            );
            viewport_tooltip(button, title, command, theme)
        };

    // Borderless icon button; an `active` toggle gets an accent tint + fill.
    let icon_btn = move |bytes: &'static [u8],
                         active: bool,
                         msg: Message,
                         title: String,
                         command: &'static str| {
        let icon = if active {
            crate::ui::icons::themed_primary_weak_text(bytes, 15.0)
        } else {
            crate::ui::icons::themed(bytes, 15.0)
        };
        let button = kit::btn(
            icon,
            if active { Role::Engaged } else { Role::Ghost },
            Size::Compact,
            Press::On(msg),
            "",
        );
        viewport_tooltip(button, title, command, theme)
    };

    // The standard picker cannot show a live sample beside its rows. This
    // flyout keeps the current style unchanged while hovering; a click commits.
    let picker_head = kit::btn(
        row![
            text(RenderModeChoice(render_mode).to_string()).size(tokens::TEXT_LABEL),
            crate::ui::icons::themed_arrow_toggle(render_mode_menu_open, 9.0),
        ]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center),
        if render_mode_menu_open {
            Role::Engaged
        } else {
            Role::Ghost
        },
        Size::Compact,
        Press::On(Message::ToggleRenderModeMenu(render_mode)),
        "",
    );
    let picker_head = viewport_tooltip(
        picker_head,
        crate::t!("Visual Style").into_owned(),
        "VISUALSTYLES",
        theme,
    );

    let preview_mode = render_mode_preview.unwrap_or(render_mode);
    let mode_names: Vec<String> = render_modes.iter().map(|c| c.to_string()).collect();
    let dropdown_w = crate::ui::style::common::dropdown_popup_width(
        mode_names.iter().map(|s| s.as_str()),
        11.0,
        46.0,
        180.0,
    );

    let mut choices = column![].spacing(tokens::GAP_HAIRLINE).width(Length::Fill);
    for choice in render_modes {
        let highlighted = choice.0 == preview_mode;
        let selected = choice.0 == render_mode;
        let checkmark = crate::ui::icons::themed_check_cell(selected);
        let label = text(choice.to_string())
            .size(tokens::TEXT_LABEL)
            .wrapping(iced::advanced::text::Wrapping::None);
        let row_content = row![checkmark, label]
            .spacing(tokens::GAP_CONTROL)
            .align_y(iced::alignment::Vertical::Center);
        let option = container(row_content)
            .padding(kit::Size::Compact.padding())
            .width(Length::Fill)
            .style(move |theme: &Theme| kit::option_row_style(theme, highlighted, selected));
        choices = choices.push(
            mouse_area(option)
                .interaction(iced::mouse::Interaction::Pointer)
                .on_enter(Message::PreviewRenderMode(choice.0))
                .on_press(Message::SetRenderMode(choice.0)),
        );
    }
    // The flyout reads the drawing through it: the column keeps only the plate
    // inset, and the glass underneath supplies fill, edge and corner. Its width
    // stays on the host so the stack shrinks to exactly what it wraps.
    let popup = kit::glass_pane(
        kit::surface(choices, Tier::Flush, PAD_PLATE, "")
            .width(Length::Fixed(dropdown_w))
            .into(),
        theme,
        false,
        "",
    );
    let picker: Element<'a, Message> =
        iced_aw::DropDown::new(picker_head, popup, render_mode_menu_open)
            .width(Length::Fixed(dropdown_w))
            .alignment(iced_aw::drop_down::Alignment::Bottom)
            .offset(3.0)
            .on_dismiss(Message::DismissRenderModeMenu)
            .into();

    // Thin vertical divider between control groups.
    let sep = || -> Element<'_, Message> { kit::rule_v(16.0, "").into() };

    let mut bar = row![].spacing(tokens::GAP_HAIRLINE).align_y(iced::alignment::Vertical::Center);
    bar = bar
        .push(icon_btn(
            crate::ui::icons::GRID,
            show_grid,
            Message::ToggleGrid,
            crate::t!("Toggle Grid").into_owned(),
            "GRID",
        ))
        .push(sep())
        .push(icon_btn(
            crate::ui::icons::SNAP,
            snap_on,
            Message::ToggleGridSnap,
            crate::t!("Toggle Grid Snap").into_owned(),
            "SNAP",
        ))
        .push(sep())
        .push(picker);
    if include_split {
        bar = bar
            .push(sep())
            .push(icon_btn(
                crate::ui::icons::SPLIT_V,
                false,
                Message::SplitModelViewport(false),
                crate::tr!("viewport", "split-vertical"),
                "VPORTS 2V",
            ))
            .push(sep())
            .push(icon_btn(
                crate::ui::icons::SPLIT_H,
                false,
                Message::SplitModelViewport(true),
                crate::tr!("viewport", "split-horizontal"),
                "VPORTS 2H",
            ));
        // Drag handle + close: only meaningful with more than one model tile.
        // The handle is a `mouse_area` (not a button) so it fires on press-DOWN,
        // letting the drag continue onto the target pane to swap them (a button
        // would only fire on release). Placed just left of Close.
        if tile_count > 1 {
            let drag = mouse_area(
                container(crate::ui::icons::themed_success(
                    crate::ui::icons::MOVE,
                    15.0,
                ))
                .padding(kit::PAD_PLATE),
            )
            .interaction(iced::mouse::Interaction::Grab)
            .on_press(Message::PaneMoveStart);
            let drag = viewport_tooltip(drag, crate::tr!("viewport", "move"), "VPORTS", theme);
            bar = bar.push(sep()).push(drag).push(sep()).push(danger_btn(
                crate::ui::icons::CLOSE,
                Message::CloseModelViewport,
                crate::tr!("viewport", "close"),
                "VPORTS SINGLE",
            ));
        }
    }

    kit::surface(bar, Tier::Plate, PAD_PLATE, "").into()
}

// ── Dynamic-input field formatting ─────────────────────────────────────────

/// Short prefix shown before a dynamic-input box's value.
/// The string shown inside a dynamic-input box: the typed buffer when the
/// field is locked, otherwise the live value derived from the cursor
/// world position (and the base point for polar quantities).
pub(super) fn dyn_component_value(
    f: &DynFieldEntry,
    w: glam::DVec3,
    base: Option<glam::DVec3>,
    xf: &super::super::helpers::UcsXform,
    comma_cartesian: bool,
    absolute: bool,
) -> String {
    if let Some(b) = &f.buffer {
        return b.clone();
    }
    let b = base.unwrap_or(glam::DVec3::ZERO);
    let p = xf.to_ucs(w);
    // Relative deltas and the polar angle read in the active UCS plane. The
    // delta is offset-invariant, so only the axis rotation matters (identity
    // xf reproduces the world-frame deltas).
    let d = xf.vec_to_ucs(w - b);
    let dx = d.x as f64;
    let dy = d.y as f64;
    // When a base point exists (DYN-on after the first pick) the cartesian
    // fields show relative deltas — matching the typed-value convention
    // in `dyn_resolve_point` so the live preview and the committed
    // coordinate use the same frame. See #35.
    let relative = base.is_some() && !absolute;
    // Width / Height read as unsigned magnitudes (the sign is taken from the
    // cursor side on commit), matching the rectangle's two-edge entry. But once
    // the user separates the values with `,` the entry is a cartesian
    // coordinate pair, so the fields read as signed X/Y deltas to match the
    // committed point (#269).
    let wh = matches!(
        f.role,
        crate::command::DynRole::Width | crate::command::DynRole::Height
    ) && relative
        && !comma_cartesian;
    match f.component {
        DynComponent::X if relative => {
            crate::entities::common::format_length(if wh { dx.abs() } else { dx })
        }
        DynComponent::Y if relative => {
            crate::entities::common::format_length(if wh { dy.abs() } else { dy })
        }
        DynComponent::Z if relative => crate::entities::common::format_length(0.0),
        DynComponent::X => crate::entities::common::format_length(p.x),
        DynComponent::Y => crate::entities::common::format_length(p.y),
        DynComponent::Z => crate::entities::common::format_length(p.z),
        // Scaled by the role so a diameter box reads twice the radius.
        DynComponent::Distance => crate::entities::common::format_length(
            (dx * dx + dy * dy).sqrt() * f.role.value_scale() as f64,
        ),
        // Shared rule: unsigned magnitude of the short angle, so CW (below the
        // reference axis) reads positive (e.g. 30°, not -30°/330°). The
        // committed value stays signed (see dyn_resolve_point).
        DynComponent::Angle => crate::entities::common::format_angle(
            (crate::command::dyn_display_angle_deg(dy.atan2(dx) as f32) as f64).to_radians(),
        ),
        // Typed-only scalar — no geometric value to track when empty.
        DynComponent::Scalar => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codec::entities::ViewportRenderMode as M;

    #[test]
    fn test_viewport_controls_construction() {
        let light = crate::ui::style::draftflow_theme::draftflow_light();
        let dark = crate::ui::style::draftflow_theme::draftflow_dark();
        for style in crate::modules::view::visual_style::VISUAL_STYLES {
            // Both palettes, because the bar's tip and flyout are glass now and
            // a pane resolves its colours at build time — a theme that cannot
            // build one of these is a crash waiting on the next toggle.
            for theme in [&light, &dark] {
                let _el = viewport_controls(style.mode, true, true, true, 1, false, None, theme);
                let _el_open = viewport_controls(
                    style.mode,
                    false,
                    false,
                    false,
                    2,
                    true,
                    Some(M::Wireframe3D),
                    theme,
                );
            }
        }
    }

    use crate::ui::style::common::wcag_contrast;

    #[test]
    fn test_visual_style_dropdown_contrast_across_all_themes() {
        for theme in iced::Theme::ALL {
            let p = theme.palette();
            let bg = p.background.weak.color;

            // 1. Text contrast (WCAG standard AA for normal text is >= 4.5:1, large/ui >= 3:1):
            let strong_text = p.background.strong.text;
            let text_contrast = wcag_contrast(strong_text, bg);
            assert!(
                text_contrast >= 4.5,
                "Theme {:?} selected text contrast {:.2} on background is too low",
                theme,
                text_contrast
            );

            let base_text = p.background.base.text;
            let base_contrast = wcag_contrast(base_text, bg);
            assert!(
                base_contrast >= 4.5,
                "Theme {:?} unselected text contrast {:.2} on background is too low",
                theme,
                base_contrast
            );

            // 2. Checkmark contrast:
            let pri = p.primary.base.color;
            let pri_contrast = wcag_contrast(pri, bg);
            let tick_color = if pri_contrast >= 2.0 {
                pri
            } else {
                p.background.base.text
            };
            let final_tick_contrast = wcag_contrast(tick_color, bg);
            assert!(
                final_tick_contrast >= 2.0,
                "Theme {:?} tick contrast {:.2} on background is too low",
                theme,
                final_tick_contrast
            );
        }
    }

    #[test]
    fn test_visual_style_dropdown_width_accommodates_all_items() {
        let max_label_len = crate::modules::view::visual_style::VISUAL_STYLES
            .iter()
            .map(|c| c.label.chars().count())
            .max()
            .unwrap_or(0);
        let dropdown_w = crate::ui::style::common::dropdown_popup_width(
            crate::modules::view::visual_style::VISUAL_STYLES
                .iter()
                .map(|c| c.label),
            11.0,
            46.0,
            180.0,
        );

        // Longest default label is "Gouraud Shaded + Edges" (22 chars)
        assert_eq!(max_label_len, 22);
        // Computed width should easily fit the 22-char label + checkmark + padding
        assert!(dropdown_w >= 210.0);

        // CJK labels count wide characters double
        let cjk_labels = ["二维线框", "着色+边"];
        let cjk_w = crate::ui::style::common::dropdown_popup_width(
            cjk_labels.iter().copied(),
            11.0,
            46.0,
            180.0,
        );
        assert_eq!(cjk_w, 180.0);

        for style in crate::modules::view::visual_style::VISUAL_STYLES {
            assert!(!style.label.contains('\n'), "Label should be a single line");
        }
    }
}
