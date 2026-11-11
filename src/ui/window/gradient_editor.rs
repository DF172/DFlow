//! Gradient flyout of the Graphic Attributes palette.

use crate::ui::style::tokens;
use crate::app::Message;
use crate::scene::model::hatch_model::GradientKind;
use crate::ui::window::graphic_attributes::GraphicAttributesMsg;
use codec::types::Color as AcadColor;
use iced::widget::{checkbox, column, container, image, mouse_area, row, slider, text, text_input, tooltip, Space};
use crate::ui::kit::plain_button;
use iced::{Background, Color, Element, Fill, Length, Theme};
use crate::ui::kit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientColorMode {
    One,
    Two,
}

#[derive(Debug, Clone)]
pub struct GradientEditorState {
    pub handles: Vec<codec::Handle>,
    pub color_mode: GradientColorMode,
    pub color_1: AcadColor,
    pub color_2: AcadColor,
    pub shade_tint: f32,
    pub kind: GradientKind,
    pub inverted: bool,
    pub angle: String,
    pub centered: bool,
    /// Large preview, rebuilt only when a setting it depends on changes.
    preview: std::cell::RefCell<Option<(PreviewKey, Option<iced::widget::image::Handle>)>>,
}

type PreviewKey = (GradientColorMode, AcadColor, AcadColor, u32, GradientKind, bool, String, bool);

#[derive(Debug, Clone)]
pub enum GradientMsg {
    Open,
    Cancel,
    Apply,
    ColorMode(GradientColorMode),
    /// Stop colour 1 or 2.
    Color(u8, AcadColor),
    ShadeTint(f32),
    Kind(GradientKind),
    InvertToggled,
    Angle(String),
    AngleReset,
    Centered(bool),
}

fn msg(message: GradientMsg) -> Message {
    Message::GraphicAttributes(GraphicAttributesMsg::Gradient(message))
}

impl GradientEditorState {
    pub fn from_hatch(
        handles: Vec<codec::Handle>,
        hatch: &codec::entities::Hatch,
    ) -> Self {
        let color = |index: usize| {
            hatch
                .gradient_color
                .colors
                .get(index)
                .map(|entry| entry.color)
                .unwrap_or(AcadColor::Index(7))
        };
        let (kind, inverted) = GradientKind::from_name(&hatch.gradient_color.name);
        Self {
            handles,
            color_mode: if hatch.gradient_color.is_single_color {
                GradientColorMode::One
            } else {
                GradientColorMode::Two
            },
            color_1: color(0),
            color_2: color(1),
            shade_tint: hatch.gradient_color.color_tint as f32,
            kind,
            inverted,
            angle: format!("{:.1}°", hatch.gradient_color.angle.to_degrees()),
            centered: hatch.gradient_color.shift.abs() < 1e-9,
            preview: Default::default(),
        }
    }

    pub fn update(&mut self, message: GradientMsg) {
        match message {
            GradientMsg::ColorMode(mode) => self.color_mode = mode,
            GradientMsg::Color(1, color) => self.color_1 = color,
            GradientMsg::Color(_, color) => self.color_2 = color,
            GradientMsg::ShadeTint(value) => self.shade_tint = value.clamp(0.0, 1.0),
            GradientMsg::Kind(kind) => {
                self.kind = kind;
                if kind == GradientKind::Linear {
                    self.inverted = false;
                }
            }
            GradientMsg::InvertToggled => {
                if self.kind != GradientKind::Linear {
                    self.inverted ^= true;
                }
            }
            GradientMsg::Angle(value) => self.angle = value,
            GradientMsg::AngleReset => self.angle = "0.0°".into(),
            GradientMsg::Centered(value) => self.centered = value,
            GradientMsg::Open | GradientMsg::Cancel | GradientMsg::Apply => {}
        }
    }

    pub fn angle_radians(&self) -> f64 {
        self.angle
            .trim()
            .trim_end_matches('°')
            .replace(',', ".")
            .parse::<f64>()
            .unwrap_or(0.0)
            .rem_euclid(360.0)
            .to_radians()
    }
}

fn rgb(color: &AcadColor) -> Color {
    let (r, g, b) = match color {
        AcadColor::Rgb { r, g, b } => (*r, *g, *b),
        AcadColor::Index(i) => {
            codec::types::aci_table::aci_to_rgb(*i).unwrap_or((128, 128, 128))
        }
        _ => (255, 255, 255),
    };
    Color::from_rgb8(r, g, b)
}

fn rgb_array(color: &AcadColor) -> [f32; 4] {
    let c = rgb(color);
    [c.r, c.g, c.b, 1.0]
}

fn mix_rgba(a: Color, b: Color, t: f32) -> [u8; 4] {
    let channel = |a: f32, b: f32| ((a + (b - a) * t) * 255.0).round() as u8;
    [channel(a.r, b.r), channel(a.g, b.g), channel(a.b, b.b), 255]
}

fn preview_sized(
    state: &GradientEditorState,
    width: u32,
    height: u32,
) -> Option<iced::widget::image::Handle> {
    let c1 = rgb(&state.color_1);
    let c2 = if state.color_mode == GradientColorMode::One {
        let c = crate::scene::gradient_tint_color(rgb_array(&state.color_1), state.shade_tint);
        Color::from_rgb(c[0], c[1], c[2])
    } else {
        rgb(&state.color_2)
    };
    let angle = state.angle_radians();
    let frame = kernel::geom2d::gradient_frame(
        &[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        angle,
        if state.centered { 0.0 } else { 1.0 },
        kernel::geom2d::Tolerance::default(),
    )?;
    let (sin, cos) = angle.sin_cos();
    let radial = state.kind.radial();
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        let py = y as f64 / (height - 1) as f64;
        for x in 0..width {
            let px = x as f64 / (width - 1) as f64;
            let mut t = if radial {
                ((px - frame.center[0]).hypot(py - frame.center[1]) / frame.radius).clamp(0.0, 1.0)
                    as f32
            } else {
                (((px * cos + py * sin) - frame.projection_min) / frame.projection_span)
                    .clamp(0.0, 1.0) as f32
            };
            if state.kind == GradientKind::Cylinder {
                t = 1.0 - (2.0 * t - 1.0).abs();
            } else if state.kind == GradientKind::Curved {
                t *= t;
            } else if state.kind == GradientKind::Hemispherical {
                t = t.sqrt();
            }
            if state.inverted {
                t = 1.0 - t;
            }
            pixels.extend_from_slice(&if radial {
                mix_rgba(c2, c1, t)
            } else {
                mix_rgba(c1, c2, t)
            });
        }
    }
    Some(iced::widget::image::Handle::from_rgba(width, height, pixels))
}

fn preview(state: &GradientEditorState) -> Element<'static, Message> {
    let key: PreviewKey = (
        state.color_mode,
        state.color_1,
        state.color_2,
        state.shade_tint.to_bits(),
        state.kind,
        state.inverted,
        state.angle.clone(),
        state.centered,
    );
    let mut cache = state.preview.borrow_mut();
    if cache.as_ref().is_none_or(|(cached, _)| *cached != key) {
        *cache = Some((key, preview_sized(state, 370, 80)));
    }
    preview_image(cache.as_ref().and_then(|(_, handle)| handle.clone()))
}

pub(crate) fn compact_preview(
    hatch: &codec::entities::Hatch,
) -> Option<iced::widget::image::Handle> {
    preview_sized(&GradientEditorState::from_hatch(Vec::new(), hatch), 256, 26)
}

pub(crate) fn preview_image(handle: Option<iced::widget::image::Handle>) -> Element<'static, Message> {
    match handle {
        Some(handle) => image(handle)
            .width(Fill)
            .height(Fill)
            .content_fit(iced::ContentFit::Fill)
            .into(),
        None => Space::new().width(Fill).height(Fill).into(),
    }
}

fn icon(kind: GradientKind, inverted: bool) -> &'static [u8] {
    match (kind, inverted) {
        (GradientKind::Linear, _) => {
            include_bytes!("../../../assets/icons/gradient/gradient_linear.svg")
        }
        (GradientKind::Cylinder, false) => {
            include_bytes!("../../../assets/icons/gradient/gradient_cylindrical.svg")
        }
        (GradientKind::Cylinder, true) => {
            include_bytes!("../../../assets/icons/gradient/gradient_cylindrical_inverted.svg")
        }
        (GradientKind::Spherical, false) => {
            include_bytes!("../../../assets/icons/gradient/gradient_spherical.svg")
        }
        (GradientKind::Spherical, true) => {
            include_bytes!("../../../assets/icons/gradient/gradient_spherical_inverted.svg")
        }
        (GradientKind::Hemispherical, false) => {
            include_bytes!("../../../assets/icons/gradient/gradient_hemispherical.svg")
        }
        (GradientKind::Hemispherical, true) => {
            include_bytes!("../../../assets/icons/gradient/gradient_hemispherical_inverted.svg")
        }
        (GradientKind::Curved, false) => {
            include_bytes!("../../../assets/icons/gradient/gradient_curved.svg")
        }
        (GradientKind::Curved, true) => {
            include_bytes!("../../../assets/icons/gradient/gradient_curved_inverted.svg")
        }
    }
}

pub fn view<'a>(
    state: &'a GradientEditorState,
    dimmed: bool,
    theme: &Theme,
) -> Element<'a, Message> {
    let segment = |label, mode| {
        let selected = state.color_mode == mode;
        plain_button(text(label).size(tokens::TEXT_LABEL))
            .on_press(msg(GradientMsg::ColorMode(mode)))
            .width(Fill)
            .style(move |theme: &Theme, status| {
                if selected {
                    kit::primary_style(theme, status)
                } else {
                    kit::secondary_style(theme, status)
                }
            })
    };
    let color_field = |label: std::borrow::Cow<'static, str>, index, color: AcadColor| {
        let (background, _) = crate::ui::properties::acad_color_display(color);
        let value = crate::ui::color_select::color_display_name(color);
        let picker = plain_button(
            row![
                crate::ui::color_select::swatch(background),
                text(value).size(tokens::TEXT_MICRO),
                Space::new().width(Fill),
            ]
            .spacing(tokens::GAP_CONTROL)
            .align_y(iced::Center),
        )
        .on_press(Message::OpenColorWindow(
            crate::app::ColorPickTarget::Gradient(index),
            color,
        ))
        .style(kit::picker_field_style)
        .height(crate::ui::ROW_H)
        .padding(kit::PAD_FIELD)
        .width(Fill);
        row![text(label).size(tokens::TEXT_LABEL).width(43), picker]
            .spacing(tokens::GAP_TIGHT)
            .align_y(iced::Center)
            .width(Fill)
    };
    let kinds = [
        (GradientKind::Linear, crate::t!("Linear")),
        (GradientKind::Cylinder, crate::t!("Cyl.")),
        (GradientKind::Spherical, crate::t!("Spher.")),
        (GradientKind::Hemispherical, crate::t!("Hemi.")),
        (GradientKind::Curved, crate::t!("Curved")),
    ];
    let mut type_row = row![].spacing(tokens::GAP_TIGHT);
    for (kind, label) in kinds {
        let selected = state.kind == kind;
        let b = plain_button(
            column![
                crate::ui::icons::semantic(
                    icon(kind, state.inverted && kind != GradientKind::Linear),
                    25.0
                ),
                text(label).size(tokens::TEXT_GLYPH_CAPTION)
            ]
            .align_x(iced::Center),
        )
        .on_press(msg(GradientMsg::Kind(kind)))
        .width(48)
        .height(48)
        .style(move |theme: &Theme, status| {
            let mut s = kit::subtle_style(theme, status);
            if selected {
                s.border.color = theme.palette().primary.base.color;
                s.border.width = 1.0;
            }
            s
        });
        type_row = type_row.push(tooltip(
            b,
            text(kind.choice_label(false)).size(tokens::TEXT_MICRO),
            tooltip::Position::Bottom,
        ));
    }
    let invert_selected = state.inverted && state.kind != GradientKind::Linear;
    let invert = plain_button(column![text("⇄").size(tokens::TEXT_FOCAL), text(crate::t!("Invert")).size(tokens::TEXT_GLYPH_CAPTION)].align_x(iced::Center))
        .on_press_maybe(
            (state.kind != GradientKind::Linear).then(|| msg(GradientMsg::InvertToggled)),
        )
        .width(42)
        .height(48)
        .style(move |theme: &Theme, status| {
            let mut style = kit::secondary_style(theme, status);
            if invert_selected {
                style.border.color = theme.palette().primary.base.color;
                style.border.width = 1.0;
            }
            style
        });
    let separator = container(Space::new().width(1).height(36))
        .style(|theme: &Theme| {
            kit::plain_plate_style(Background::Color(theme.palette().background.neutral.color))
        });
    type_row = type_row.push(separator).push(tooltip(
        invert,
        text(crate::t!("Invert gradient")).size(tokens::TEXT_MICRO),
        tooltip::Position::Bottom,
    ));
    let colors: Element<'_, Message> = if state.color_mode == GradientColorMode::Two {
        row![
            color_field(crate::t!("Color 1"), 1, state.color_1),
            color_field(crate::t!("Color 2"), 2, state.color_2)
        ]
        .spacing(tokens::GAP_CONTROL)
        .into()
    } else {
        row![
            color_field(crate::t!("Color 1"), 1, state.color_1),
            row![
                text(crate::t!("Tone")).size(tokens::TEXT_LABEL).width(43),
                slider(0..=100, (state.shade_tint * 100.0) as i32, |v| {
                    msg(GradientMsg::ShadeTint(v as f32 / 100.0))
                })
                .width(Fill)
            ]
            .spacing(tokens::GAP_TIGHT)
            .align_y(iced::Center)
            .height(crate::ui::ROW_H)
            .width(Fill)
        ]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center)
        .into()
    };
    let angle = row![
        text(crate::t!("Angle")).size(tokens::TEXT_LABEL).width(43),
        text_input("0.0°", &state.angle)
            .on_input(|value| msg(GradientMsg::Angle(value)))
            .size(tokens::TEXT_LABEL)
            .width(Length::Fixed(92.0)),
        tooltip(
            plain_button(crate::ui::icons::themed_undo(11.0, true))
                .on_press(msg(GradientMsg::AngleReset))
                .style(kit::subtle_style)
                .padding(tokens::GAP_HAIRLINE)
                .width(24)
                .height(crate::ui::ROW_H),
            text(crate::t!("Reset angle")).size(tokens::TEXT_MICRO),
            tooltip::Position::Bottom,
        ),
        Space::new().width(Fill),
        checkbox(state.centered)
            .label(crate::t!("Centered"))
            .on_toggle(|value| msg(GradientMsg::Centered(value)))
            .size(tokens::MARK_SQUARE)
            .text_size(tokens::TEXT_LABEL)
    ]
    .spacing(tokens::GAP_CONTROL)
    .align_y(iced::Center);
    let types = row![text(crate::t!("Type")).size(tokens::TEXT_LABEL).width(43), type_row]
        .spacing(tokens::GAP_TIGHT)
        .align_y(iced::Center);
    let actions = container(
        row![
            plain_button(text(crate::t!("Cancel")).size(tokens::TEXT_LABEL))
                .on_press(msg(GradientMsg::Cancel))
                .style(kit::secondary_style)
                .padding(kit::PAD_TAB),
            plain_button(text(crate::t!("OK")).size(tokens::TEXT_LABEL))
                .on_press(msg(GradientMsg::Apply))
                .style(kit::primary_style)
                .padding(kit::Size::Action.padding()),
        ]
        .spacing(tokens::GAP_ROW),
    )
    .width(Fill)
    .align_x(iced::alignment::Horizontal::Right);
    let content = column![
        container(preview(state)).width(Fill).height(80),
        row![
            segment(crate::t!("One Color"), GradientColorMode::One),
            segment(crate::t!("Two Colors"), GradientColorMode::Two)
        ]
        .spacing(tokens::GAP_HAIRLINE),
        colors,
        types,
        angle,
        actions
    ]
    .spacing(tokens::GAP_ROW)
    .padding(tokens::GAP_FIELD);
    // The editor hangs off the fill preview and floats over the drawing, so it
    // reads the drawing through a glass pane instead of wearing the brand-inked
    // plate it had. The 390px width and `GAP_FIELD` insets are unchanged.
    let plate = container(content)
        .width(Length::Fixed(390.0))
        .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""));
    let panel: Element<'_, Message> = kit::glass_pane(plate.into(), theme, false, "");
    let panel: Element<'_, Message> = if dimmed {
        let shield = container(Space::new())
            .width(Fill)
            .height(Fill)
            .style(|theme: &Theme| kit::scrim_style(theme, ""));
        iced::widget::stack![panel, iced::widget::opaque(shield)]
            .width(Length::Fixed(390.0))
            .into()
    } else {
        panel
    };
    mouse_area(panel)
        .interaction(iced::mouse::Interaction::Pointer)
        .into()
}
