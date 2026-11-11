//! Text style manager.

use crate::app::{Message, StyleKind};
use crate::t;
use crate::ui::kit::muted_style;
use iced::widget::{canvas, checkbox, column, container, row, text, text_input, Space};
use crate::ui::kit::plain_button;
use iced::{mouse, Element, Length, Point, Rectangle, Theme};
use std::borrow::Cow;
use crate::ui::style::tokens;
use crate::ui::kit;

pub struct TextStyleView<'a> {
    pub styles: Vec<String>,
    /// Parallel to `styles`: the drawing references that text style.
    pub used: Vec<bool>,
    pub selected: &'a str,
    pub current: &'a str,
    pub tab: u8,
    pub compare_name: String,
    pub compare_opts: Vec<String>,
    pub comparison_sections: Vec<String>,
    pub read_only: bool,
    pub in_use: bool,
    pub font_buf: &'a str,
    pub width_buf: &'a str,
    pub oblique_buf: &'a str,
    pub height_buf: &'a str,
    pub bigfont_buf: &'a str,
    pub ttf_buf: &'a str,
    pub backward: bool,
    pub upside_down: bool,
    pub vertical: bool,
    pub annotative: bool,
    pub rename_active: Option<&'a str>,
    pub rename_buf: &'a str,
}

const BUILTIN_FONTS: &[&str] = &[
    "Standard", "ISO", "Simplex", "RomanS", "RomanD", "RomanC", "RomanT", "ItalicC", "ItalicT",
    "ScriptS", "ScriptC", "GothGBT", "GothGRT", "GothITT", "GreekC", "Symbol", "ISO3098",
    "Unicode",
];

struct TextPreviewCanvas {
    font: String,
    big_font: String,
    width_factor: f32,
    oblique: f32,
    height: f32,
    upside_down: bool,
    vertical: bool,
    annotative: bool,
}

impl canvas::Program<Message> for TextPreviewCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let ink = theme.palette().background.base.text;
        let glyph_height = if self.height <= 0.0 {
            18.0
        } else {
            (self.height * 7.0).clamp(6.0, 52.0)
        };
        let rotation = if self.upside_down {
            std::f32::consts::PI
        } else {
            0.0
        };
        let strokes = if self.vertical {
            let mut result = Vec::new();
            for (index, character) in "Ab3".chars().enumerate() {
                let (mut glyph, _) = crate::scene::text::lff::tessellate_text_ex(
                    [0.0, -(index as f32) * glyph_height * 1.25],
                    glyph_height,
                    rotation,
                    self.width_factor,
                    self.oblique,
                    &self.font,
                    &character.to_string(),
                );
                result.append(&mut glyph);
            }
            result
        } else {
            crate::scene::text::lff::tessellate_text_ex(
                [0.0, 0.0],
                glyph_height,
                rotation,
                self.width_factor,
                self.oblique,
                &self.font,
                "AaBbCc 0123",
            )
            .0
        };
        let mut min = [f32::MAX; 2];
        let mut max = [f32::MIN; 2];
        for stroke in &strokes {
            for &[x, y] in stroke {
                min[0] = min[0].min(x);
                min[1] = min[1].min(y);
                max[0] = max[0].max(x);
                max[1] = max[1].max(y);
            }
        }
        if min[0] <= max[0] {
            let span = [(max[0] - min[0]).max(1.0), (max[1] - min[1]).max(1.0)];
            let available = [bounds.width - 24.0, bounds.height - 34.0];
            let scale = (available[0] / span[0])
                .min(available[1] / span[1])
                .min(1.0)
                .max(0.0);
            let middle = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5];
            let center = [bounds.width * 0.5, (bounds.height - 18.0) * 0.5];
            let map = |x: f32, y: f32| {
                Point::new(
                    center[0] + (x - middle[0]) * scale,
                    center[1] - (y - middle[1]) * scale,
                )
            };
            for stroke in &strokes {
                if stroke.len() < 2 {
                    continue;
                }
                let path = canvas::Path::new(|builder| {
                    builder.move_to(map(stroke[0][0], stroke[0][1]));
                    for &[x, y] in &stroke[1..] {
                        builder.line_to(map(x, y));
                    }
                });
                frame.stroke(
                    &path,
                    canvas::Stroke::default().with_color(ink).with_width(1.35),
                );
            }
        }
        let font_label = if self.big_font.trim().is_empty() {
            self.font.clone()
        } else {
            format!("{} + {}", self.font, self.big_font)
        };
        let mode = if self.annotative {
            t!("Paper height")
        } else {
            t!("Drawing height")
        };
        frame.fill_text(canvas::Text {
            content: format!("{}  ·  {} {:.3}", font_label, mode, self.height),
            position: Point::new(10.0, bounds.height - 9.0),
            color: ink.scale_alpha(0.65),
            size: iced::Pixels(10.0),
            align_y: iced::alignment::Vertical::Center,
            ..Default::default()
        });
        vec![frame.into_geometry()]
    }
}

fn input_row<'a>(
    label: Cow<'static, str>,
    placeholder: Cow<'static, str>,
    value: &'a str,
    field: &'static str,
    read_only: bool,
) -> Element<'a, Message> {
    let control: Element<'a, Message> = if read_only {
        crate::ui::read_only::field(value, 11.0, Length::Fixed(180.0))
    } else {
        text_input(placeholder.as_ref(), value)
            .on_input(move |value| Message::TextStyleEdit { field, value })
            .style(kit::field_style)
            .size(tokens::TEXT_LABEL)
            .width(180)
            .into()
    };
    row![text(label).size(tokens::TEXT_LABEL).style(muted_style).width(150), control]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center)
        .into()
}

fn toggle<'a>(
    label: Cow<'static, str>,
    value: bool,
    field: &'static str,
    read_only: bool,
) -> Element<'a, Message> {
    let checkbox = checkbox(value).label(label).size(tokens::MARK_SQUARE).text_size(tokens::TEXT_LABEL);
    if read_only {
        checkbox.into()
    } else {
        checkbox
            .on_toggle(move |_| Message::TextStyleToggle(field))
            .into()
    }
}

/// A font picker list the way the sample sets it: a bounded well with a band
/// across the top (the list's name on the left, its count or "installed" on the
/// right), then rows that carry a status dot and, on the selected row, the
/// "current" pill. The band stays fixed while the rows scroll.
fn font_list<'a>(
    title: Cow<'static, str>,
    values: Vec<String>,
    selected: &'a str,
    system: bool,
    read_only: bool,
) -> Element<'a, Message> {
    let count = values.len();
    let header = container(
        row![
            text(title).size(tokens::TEXT_MICRO).style(muted_style),
            Space::new().width(Length::Fill),
            text(if system {
                t!("Installed").into_owned()
            } else {
                count.to_string()
            })
            .size(tokens::TEXT_MICRO)
            .style(muted_style),
        ]
        .align_y(iced::Center),
    )
    .width(Length::Fill)
    .padding(iced::Padding {
        top: tokens::GAP_TIGHT,
        right: tokens::GAP_ROW,
        bottom: tokens::GAP_TIGHT,
        left: tokens::GAP_ROW,
    })
    .style(|theme: &Theme| kit::sheet_strip_style(kit::Tier::Band, theme, "modal:fade"));

    let items: Vec<Element<'a, Message>> = values
        .into_iter()
        .map(|font| {
            let active = selected.eq_ignore_ascii_case(&font);
            let dot = container(Space::new().width(8).height(8))
                .style(move |theme: &Theme| kit::status_dot_style(theme, active));
            let mut label = row![
                dot,
                text(font.clone()).size(tokens::TEXT_MICRO),
                Space::new().width(Length::Fill),
            ]
            .spacing(tokens::GAP_TIGHT)
            .align_y(iced::Center);
            if active {
                let badge: Element<'a, Message> = kit::pill(
                    text(t!("Current").into_owned()).size(tokens::TEXT_MICRO),
                    "font-list:current",
                    true,
                    kit::PAD_BADGE,
                    "modal:fade",
                )
                .into();
                label = label.push(badge);
            }
            let button = plain_button(label)
                .style(kit::chip_button_style(active))
                .padding(kit::Size::Compact.padding())
                .width(Length::Fill);
            if read_only {
                button.into()
            } else if system {
                button
                    .on_press(Message::TextStyleEdit {
                        field: "ttf",
                        value: font,
                    })
                    .into()
            } else {
                button.on_press(Message::TextStyleFontPick(font)).into()
            }
        })
        .collect();
    container(column![
        header,
        kit::hdivider(Length::Fill),
        crate::ui::kit::themed_scrollable(column(items).spacing(tokens::GAP_SEAM)).height(250)
    ])
    .width(Length::FillPortion(1))
    .style(kit::pane_plate_style)
    .clip(true)
    .into()
}

/// One of the three field cards under the font lists ("Font file", "Big font",
/// "System font"): a titled plate around a single full-width field. The sample
/// groups them because they are three answers to the same question, and a row
/// of bare label/field pairs made the third read as an afterthought.
fn field_card<'a>(
    label: Cow<'static, str>,
    placeholder: Cow<'static, str>,
    value: &'a str,
    field: &'static str,
    read_only: bool,
) -> Element<'a, Message> {
    let control: Element<'a, Message> = if read_only {
        crate::ui::read_only::field(value, tokens::TEXT_LABEL, Length::Fill)
    } else {
        text_input(placeholder.as_ref(), value)
            .on_input(move |value| Message::TextStyleEdit { field, value })
            .style(kit::field_style)
            .size(tokens::TEXT_LABEL)
            .width(Length::Fill)
            .into()
    };
    container(
        column![
            text(label).size(tokens::TEXT_MICRO).style(muted_style),
            control,
        ]
        .spacing(tokens::GAP_CONTROL),
    )
    .width(Length::FillPortion(1))
    .style(kit::pane_plate_style)
    .padding(kit::PAD_CARD)
    .into()
}

pub fn view_window<'a>(
    v: TextStyleView<'a>,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let effective_font = if v.ttf_buf.trim().is_empty() {
        v.font_buf
    } else {
        v.ttf_buf
    };
    let width = v
        .width_buf
        .trim()
        .parse::<f32>()
        .unwrap_or(1.0)
        .abs()
        .clamp(0.01, 100.0);
    let preview = canvas(TextPreviewCanvas {
        font: effective_font.to_string(),
        big_font: v.bigfont_buf.to_string(),
        width_factor: if v.backward { -width } else { width },
        oblique: v
            .oblique_buf
            .trim()
            .parse::<f32>()
            .unwrap_or(0.0)
            .to_radians(),
        height: v.height_buf.trim().parse::<f32>().unwrap_or(0.0).max(0.0),
        upside_down: v.upside_down,
        vertical: v.vertical,
        annotative: v.annotative,
    })
    .width(Length::Fill)
    .height(112);

    let content: Element<'a, Message> = match v.tab {
        0 => {
            let built_in = BUILTIN_FONTS
                .iter()
                .map(|value| (*value).to_string())
                .collect();
            // Web: the fetched Noto set plus the office/CAD family names it
            // stands in for (Arial → Noto Sans, …); desktop: installed fonts.
            let system = crate::scene::text::sysfont::picker_families();
            column![
                row![
                    font_list(t!("Stroke fonts"), built_in, v.font_buf, false, v.read_only),
                    font_list(t!("System fonts"), system, v.ttf_buf, true, v.read_only),
                ]
                .spacing(tokens::GAP_FIELD),
                row![
                    field_card(
                        t!("Font file"),
                        t!("font file…"),
                        v.font_buf,
                        "font",
                        v.read_only
                    ),
                    field_card(
                        t!("Big font"),
                        t!("big-font file…"),
                        v.bigfont_buf,
                        "bigfont",
                        v.read_only
                    ),
                    field_card(
                        t!("System font"),
                        t!("font family…"),
                        v.ttf_buf,
                        "ttf",
                        v.read_only
                    ),
                ]
                .spacing(tokens::GAP_FIELD),
            ]
            .spacing(tokens::GAP_ROW)
            .into()
        }
        _ => column![
            text(t!("Size")).size(tokens::TEXT_LABEL).style(|theme: &Theme| kit::color_style(theme.palette().primary.base.color)),
            input_row(
                if v.annotative {
                    t!("Paper text height")
                } else {
                    t!("Fixed height")
                },
                t!("0 = variable"),
                v.height_buf,
                "height",
                v.read_only,
            ),
            text(t!("Effects")).size(tokens::TEXT_LABEL).style(|theme: &Theme| kit::color_style(theme.palette().primary.base.color)),
            input_row(
                t!("Width factor"),
                "1.0".into(),
                v.width_buf,
                "width",
                v.read_only
            ),
            input_row(
                t!("Oblique angle"),
                "0.0".into(),
                v.oblique_buf,
                "oblique",
                v.read_only
            ),
            row![
                toggle(t!("Backward"), v.backward, "backward", v.read_only),
                toggle(t!("Upside down"), v.upside_down, "upside_down", v.read_only),
                toggle(
                    t!("Vertical"),
                    v.vertical,
                    "vertical",
                    v.read_only || !v.ttf_buf.trim().is_empty(),
                ),
            ]
            .spacing(tokens::GAP_GUTTER),
            toggle(t!("Annotative"), v.annotative, "annotative", v.read_only),
        ]
        .spacing(tokens::GAP_ROW)
        .into(),
    };
    let status = if v.read_only {
        t!("Referenced style · Read only")
    } else if v.selected.eq_ignore_ascii_case(v.current) {
        t!("Current style")
    } else if v.in_use {
        t!("Used in drawing")
    } else {
        t!("Available style")
    };
    let summary = if v.comparison_sections.is_empty() {
        t!("No differences").into_owned()
    } else {
        format!("{}: {}", t!("Different"), v.comparison_sections.join(", "))
    };
    let editor = crate::ui::style::style_manager::editor_shell(
        crate::ui::style::style_manager::EditorShell {
            sizing,
            selected: v.selected.to_string(),
            status,
            preview: preview.into(),
            comparison: Some(crate::ui::style::style_manager::EditorComparison {
                selected: v.compare_name,
                options: v.compare_opts,
                summary,
                on_select: Message::TextStyleDialogCompare,
            }),
            tabs: vec![
                crate::ui::style::style_manager::EditorTab {
                    label: t!("Fonts"),
                    active: v.tab == 0,
                    on_press: Message::TextStyleDialogTab(0),
                },
                crate::ui::style::style_manager::EditorTab {
                    label: t!("Size and Effects"),
                    active: v.tab == 1,
                    on_press: Message::TextStyleDialogTab(1),
                },
            ],
            content,
        },
    );
    crate::ui::style::style_manager::view(crate::ui::style::style_manager::Scaffold {
        sizing,
        kind: StyleKind::Text,
        styles: &v.styles,
        used: &v.used,
        selected: v.selected,
        current: Some(v.current),
        rename_active: v.rename_active,
        rename_buf: v.rename_buf,
        on_new: Message::TextStyleDialogNew,
        on_copy: Message::TextStyleDialogCopy,
        on_delete: Message::TextStyleDialogDelete,
        on_select: Message::TextStyleDialogSelect,
        on_set_current: Message::TextStyleDialogSetCurrent,
        on_apply: Message::TextStyleApply,
        read_only: v.read_only,
        editor,
    })
}
