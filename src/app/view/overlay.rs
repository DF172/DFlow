use crate::ui::kit;
use super::super::Message;
use crate::t;
use crate::ui::popup::context_menu::{
    ContextMenu, MenuIcon, MenuItem, MenuRow, MENU_PAD_TOP, MENU_ROW_H,
};
use iced::advanced::layout;
use iced::advanced::mouse;
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget;
use iced::advanced::{Layout, Shell, Widget};
use iced::widget::{column, container, mouse_area, row, scrollable, stack, text, text_input, Space};
use crate::ui::kit::plain_button;
use iced::{
    Color, Element, Event, Fill, Length, Rectangle, Size, Theme, Vector,
};
use crate::ui::style::tokens;

pub(super) fn position_canvas_overlay<'a>(
    anchor: iced::Point,
    panel: Element<'a, Message>,
) -> Element<'a, Message> {
    iced::widget::pin(iced::widget::opaque(panel))
        .position(iced::Point::new(anchor.x.max(0.0), anchor.y.max(0.0)))
        .into()
}

// ── In-place MText editor overlay ───────────────────────────────────────────

/// Widget id for the MText editor's text area (focused when Edit mode opens).
pub(in crate::app) const MTEXT_TEXT_ID: &str = "mtext_editor_text";

/// Widget id for the in-place TEXT editor's input (focused when it opens).
pub(in crate::app) const TEXT_INLINE_ID: &str = "text_inline_input";

/// In-place single-line TEXT editor: a plain text-entry box (no formatting
/// toolbar), anchored at the insertion-point click. Enter commits; Esc cancels.
pub(super) fn text_inline_overlay<'a>(
    ed: &'a super::super::text_inline::TextInlineState,
    canvas: (f32, f32),
    theme: &Theme,
) -> Element<'a, Message> {
    let field = text_input(t!("Text").as_ref(), &ed.value)
        .id(iced::widget::Id::new(TEXT_INLINE_ID))
        .on_input(Message::TextInlineInput)
        .on_submit(Message::TextInlineOk)
        .padding(tokens::GAP_CONTROL)
        .size(tokens::TEXT_BODY)
        .width(iced::Length::Fixed(240.0))
        .style(kit::field_style);

    // The plate is no longer painted by a tier: the pane under the field reads
    // the document through it, so the host keeps only the well's inset and the
    // glass supplies fill, rim and corner.
    let panel = kit::glass_pane(
        kit::surface(field, kit::Tier::Flush, kit::PAD_WELL, "").into(),
        theme,
        false,
        "",
    );

    // Keep the box on-screen so its field stays clickable at the edges.
    const PANEL_W: f32 = 240.0 + 20.0;
    const PANEL_H: f32 = 46.0;
    let (cw, ch) = canvas;
    let anchor = iced::Point::new(
        (ed.screen_anchor.x - 6.0).clamp(0.0, (cw - PANEL_W).max(0.0)),
        (ed.screen_anchor.y - 18.0).clamp(0.0, (ch - PANEL_H).max(0.0)),
    );
    position_canvas_overlay(anchor, panel.into())
}

// Stroke-font families bundled with the renderer; see scene/lff.rs.
const MTEXT_FONTS: [&str; 10] = [
    "[Style default]",
    "Standard",
    "ISO",
    "Simplex",
    "RomanS",
    "RomanD",
    "ItalicC",
    "ScriptS",
    "GothGBT",
    "RomanC",
];
/// (label, ACI). 256 = ByLayer.
/// Canvas program that renders the tessellated MText strokes inside the
/// editor's own preview area (never on the drawing). Strokes lie in the
/// world XY plane; the program fits + vertically flips them into the box.
pub(in crate::app) const MTEXT_PREVIEW_PAD: f32 = 12.0;
pub(in crate::app) const MTEXT_PREVIEW_EM_PX: f32 = 15.0;
// Used only until the shared modal sensor reports the first real layout width.
const MTEXT_EDITOR_FALLBACK_WIDTH: f32 = 660.0;
pub(in crate::app) const MTEXT_EDITOR_WRITING_WIDTH: f32 =
    MTEXT_EDITOR_FALLBACK_WIDTH - 2.0 * MTEXT_PREVIEW_PAD;

struct MTextPreview {
    /// Disconnected polylines as (x, y) world points + colour (NaN-split done).
    segments: Vec<(Vec<(f32, f32)>, Color, f32)>,
    /// Per-visible-character boxes (world frame) for click-to-select.
    boxes: Vec<crate::entities::text_support::GlyphBox>,
    /// Current selection as a visible-char range.
    sel: Option<(usize, usize)>,
    /// Caret position as a visible-char offset.
    caret: usize,
    /// Whether the caret is in its visible blink phase.
    caret_on: bool,
    /// World-space min corner (bbox) and pixels-per-world-unit scale.
    minx: f32,
    miny: f32,
    scale: f32,
    content_h: f32,
    /// On-screen width of the MText wrapping rectangle.
    wrap_width_px: f32,
}

impl MTextPreview {
    /// Visible-char offset (0..=N) nearest the cursor point (bounds-local px).
    fn offset_at(&self, p: iced::Point) -> usize {
        if self.boxes.is_empty() {
            return 0;
        }
        let wx = self.minx + (p.x - MTEXT_PREVIEW_PAD) / self.scale;
        let wy = self.miny + (self.content_h - p.y - MTEXT_PREVIEW_PAD) / self.scale;
        let mut best = 0usize;
        let mut best_d = f32::MAX;
        for b in &self.boxes {
            let dx = if wx < b.xmin {
                b.xmin - wx
            } else if wx > b.xmax {
                wx - b.xmax
            } else {
                0.0
            };
            let dy = if wy < b.ymin {
                b.ymin - wy
            } else if wy > b.ymax {
                wy - b.ymax
            } else {
                0.0
            };
            let d = dy * 1000.0 + dx; // prefer the correct line first
            if d < best_d {
                best_d = d;
                if b.is_rtl {
                    if wx < (b.xmin + b.xmax) * 0.5 {
                        best = b.vis + 1;
                    } else {
                        best = b.vis;
                    }
                } else {
                    best = b.vis;
                    // After the glyph centre → caret sits after this char.
                    if wx > (b.xmin + b.xmax) * 0.5 {
                        best = b.vis + 1;
                    }
                }
            }
        }
        best
    }
}

#[derive(Default)]
struct MTextPreviewState {
    dragging: bool,
}

impl iced::widget::canvas::Program<Message> for MTextPreview {
    type State = MTextPreviewState;

    fn update(
        &self,
        state: &mut MTextPreviewState,
        event: &iced::Event,
        bounds: iced::Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Option<iced::widget::canvas::Action<Message>> {
        use iced::mouse::{Button, Event as Me};
        use iced::widget::canvas::Action;
        use iced::Event;
        match event {
            Event::Mouse(Me::ButtonPressed(Button::Left)) => {
                if let Some(p) = cursor.position_in(bounds) {
                    state.dragging = true;
                    let off = self.offset_at(p);
                    return Some(Action::publish(Message::MTextSelStart(off)).and_capture());
                }
            }
            Event::Mouse(Me::CursorMoved { .. }) => {
                if state.dragging {
                    if let Some(p) = cursor.position_in(bounds) {
                        let off = self.offset_at(p);
                        return Some(Action::publish(Message::MTextSelTo(off)));
                    }
                }
            }
            Event::Mouse(Me::ButtonReleased(Button::Left)) => {
                if state.dragging {
                    state.dragging = false;
                    return Some(Action::capture());
                }
            }
            _ => {}
        }
        None
    }

    fn draw(
        &self,
        _state: &MTextPreviewState,
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<iced::widget::canvas::Geometry> {
        use iced::widget::canvas::{Frame, Path, Stroke};
        let mut frame = Frame::new(renderer, bounds.size());
        let pad = MTEXT_PREVIEW_PAD;
        // Draw at the real size; flip Y (world up → screen down).
        let map = |x: f32, y: f32| {
            iced::Point::new(
                pad + (x - self.minx) * self.scale,
                self.content_h - (pad + (y - self.miny) * self.scale),
            )
        };
        // Wrap ruler: same screen-space position as the width slider handle.
        let wrap_x = (pad + self.wrap_width_px).clamp(pad, bounds.width.max(pad));
        let wrap_line = Path::new(|path| {
            path.move_to(iced::Point::new(wrap_x, 0.0));
            path.line_to(iced::Point::new(wrap_x, self.content_h));
        });
        frame.stroke(
            &wrap_line,
            Stroke::default()
                .with_color(theme.palette().primary.base.color.scale_alpha(0.65))
                .with_width(1.0),
        );
        // Selection highlight behind the glyphs.
        if let Some((a, b)) = self.sel {
            for bx in &self.boxes {
                if bx.vis >= a && bx.vis < b {
                    let p0 = map(bx.xmin, bx.ymax);
                    let p1 = map(bx.xmax, bx.ymin);
                    let rect = Path::rectangle(
                        iced::Point::new(p0.x.min(p1.x), p0.y.min(p1.y)),
                        iced::Size::new((p1.x - p0.x).abs(), (p1.y - p0.y).abs()),
                    );
                    frame.fill(&rect, theme.palette().primary.base.color.scale_alpha(0.45));
                }
            }
        }
        for (seg, col, width) in &self.segments {
            if seg.len() < 2 {
                continue;
            }
            let path = Path::new(|p| {
                p.move_to(map(seg[0].0, seg[0].1));
                for &(x, y) in &seg[1..] {
                    p.line_to(map(x, y));
                }
            });
            frame.stroke(&path, Stroke::default().with_color(*col).with_width(*width));
        }
        // Caret — a vertical bar at the caret's glyph boundary, shown when the
        // selection is empty (a plain text cursor).
        // Caret is shown only when the selection is empty and the blink is in
        // its visible phase.
        let collapsed = self.caret_on && self.sel.map(|(a, b)| a == b).unwrap_or(true);
        if collapsed && self.boxes.is_empty() {
            // Empty text: show a caret at the top-left so the user can type.
            // Its height matches the preview's constant em (see EM_PX) so the
            // empty-editor caret isn't tiny.
            let path = Path::new(|p| {
                p.move_to(iced::Point::new(MTEXT_PREVIEW_PAD, MTEXT_PREVIEW_PAD));
                p.line_to(iced::Point::new(
                    MTEXT_PREVIEW_PAD,
                    (MTEXT_PREVIEW_PAD + 40.0).min(self.content_h),
                ));
            });
            frame.stroke(
                &path,
                Stroke::default()
                    .with_color(theme.palette().warning.base.color)
                    .with_width(1.5),
            );
        } else if collapsed {
            let bar = if let Some(b) = self.boxes.iter().find(|b| b.vis == self.caret) {
                let cx = if b.is_rtl { b.xmax } else { b.xmin };
                Some((cx, b.ymin, b.ymax))
            } else if self.caret > 0 {
                self.boxes
                    .iter()
                    .find(|b| b.vis == self.caret - 1)
                    .map(|b| {
                        let cx = if b.is_rtl { b.xmin } else { b.xmax };
                        (cx, b.ymin, b.ymax)
                    })
            } else {
                self.boxes.first().map(|b| {
                    let cx = if b.is_rtl { b.xmax } else { b.xmin };
                    (cx, b.ymin, b.ymax)
                })
            };
            if let Some((cx, y0, y1)) = bar {
                let p0 = map(cx, y0);
                let p1 = map(cx, y1);
                let path = Path::new(|p| {
                    p.move_to(p0);
                    p.line_to(p1);
                });
                frame.stroke(
                    &path,
                    Stroke::default()
                        .with_color(theme.palette().warning.base.color)
                        .with_width(1.5),
                );
            }
        }
        vec![frame.into_geometry()]
    }
}

/// Split every preview WireModel into finite (x, y) polyline runs, each
/// carrying its wire's colour (so inline `\C` / the colour dropdown shows) and a
/// stroke width (bold runs carry a wider pen via `line_weight_px`).
fn mtext_preview_segments(
    ed: &super::super::mtext_editor::MTextEditorState,
) -> Vec<(Vec<(f32, f32)>, Color, f32)> {
    let mut out: Vec<(Vec<(f32, f32)>, Color, f32)> = Vec::new();
    for w in &ed.preview_wires {
        let col = Color {
            r: w.color[0],
            g: w.color[1],
            b: w.color[2],
            a: 1.0,
        };
        // Bold text wires bake a wider pen (line_weight_px ~2.4); draw them thick.
        let width = if w.line_weight_px > 1.5 { 2.6 } else { 1.4 };
        let mut run: Vec<(f32, f32)> = Vec::new();
        for p in &w.points {
            if p[0].is_finite() && p[1].is_finite() {
                run.push((p[0], p[1]));
            } else if !run.is_empty() {
                out.push((std::mem::take(&mut run), col, width));
            }
        }
        if !run.is_empty() {
            out.push((run, col, width));
        }
    }
    out
}

pub(super) fn mtext_editor_overlay<'a>(
    ed: &'a super::super::mtext_editor::MTextEditorState,
    styles: Vec<String>,
    modal_offset: iced::Vector,
    modal_resize: iced::Vector,
    modal_content_size: Option<iced::Size>,
    theme: &Theme,
) -> Element<'a, Message> {
    let writing_area_px = modal_content_size
        .map(|size| size.width - 2.0 * MTEXT_PREVIEW_PAD)
        .unwrap_or(MTEXT_EDITOR_WRITING_WIDTH)
        .max(80.0);
    let measurement = mtext_editor_content(
        ed,
        &styles,
        crate::ui::shell::Sizing::INTRINSIC,
        writing_area_px,
        (writing_area_px * 0.5).max(1.0),
        theme,
    );
    let content = mtext_editor_content(
        ed,
        &styles,
        crate::ui::shell::Sizing::FILL,
        writing_area_px,
        (writing_area_px * 0.5).max(1.0),
        theme,
    );
    let content =
        crate::ui::shell::intrinsic(measurement, content, iced::Size::INFINITE, modal_resize);

    crate::ui::shell::modal(
        iced::widget::Space::new().width(Fill).height(Fill),
        t!("Text Editor"),
        content,
        Message::MTextOk,
        modal_offset,
        crate::ui::shell::ModalOptions::STANDARD,
        None,
        theme,
    )
}

fn mtext_editor_content<'a>(
    ed: &'a super::super::mtext_editor::MTextEditorState,
    styles: &[String],
    sizing: crate::ui::shell::Sizing,
    writing_area_px: f32,
    intrinsic_preview_height: f32,
    // The toolbar's colour field drops its list over the preview; the list is a
    // glass pane and takes the theme the overlay is drawn with.
    theme: &Theme,
) -> Element<'a, Message> {
    let width = sizing.width;
    let height = sizing.height;
    let preview_height = if matches!(height, iced::Length::Fill) {
        iced::Length::Fill
    } else {
        iced::Length::Fixed(intrinsic_preview_height)
    };
    use super::super::mtext_editor::{JustifyChoice, MTextFmt, ParaAlign};
    use iced::widget::canvas;

    let icon_btn = move |bytes: &'static [u8], msg: Message| -> Element<'static, Message> {
        kit::icon_btn(
            crate::ui::icons::themed(bytes, 18.0),
            kit::Role::Standard,
            24.0,
            kit::Press::On(msg),
            "",
        )
        .into()
    };
    let lbl = |s: &str| text(crate::t!(s)).size(tokens::TEXT_LABEL);
    let small_input = |placeholder: &'static str,
                       val: &str,
                       on: fn(String) -> Message,
                       w: f32|
     -> Element<'static, Message> {
        text_input(placeholder, val)
            .on_input(on)
            .width(iced::Length::Fixed(w))
            .padding(kit::PAD_FIELD)
            .size(tokens::TEXT_BODY)
            .style(kit::field_style)
            .into()
    };

    // ── Row 1: style / font / height · format icons · colour ──────────────
    let style_opts: Vec<String> = if styles.is_empty() {
        vec!["Standard".to_string()]
    } else {
        styles.to_vec()
    };
    let style_pl = kit::themed_pick_list(Some(ed.style.clone()), style_opts, |value| {
        value.to_string()
    })
    .on_select(Message::MTextStyle)
    .text_size(tokens::TEXT_LABEL)
    .width(iced::Length::Fixed(96.0));
    let font_sel = if ed.font.trim().is_empty() {
        "[Style default]".to_string()
    } else {
        ed.font.clone()
    };
    let font_pl = kit::themed_pick_list(
        Some(font_sel),
        MTEXT_FONTS
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
        |value| t!(value).into_owned(),
    )
    .on_select(Message::MTextFont)
    .text_size(tokens::TEXT_LABEL)
    .width(iced::Length::Fixed(120.0));
    // Same colour picker as the Properties panel (named swatches + "More…" full
    // palette), applied to the selection or the whole text.
    let color_pl = iced::widget::container(crate::ui::color_select::color_selector(
        codec::types::Color::from_index(ed.color_aci as i16),
        ed.color_picker_open,
        crate::ui::color_select::ColorExtras {
            by_layer: true,
            by_block: false,
            ..Default::default()
        },
        Message::MTextColorChanged,
        Message::MTextColorPickerToggle,
        Message::OpenColorWindow(
            crate::app::ColorPickTarget::MText,
            codec::types::Color::from_index(ed.color_aci as i16),
        ),
        theme,
    ))
    .width(iced::Length::Fixed(150.0));

    let row1 = row![
        style_pl,
        kit::labeled_checkbox(t!("Annotative").into_owned(), ed.annotative, Message::MTextAnnotative),
        font_pl,
        small_input("2.5", &ed.height, Message::MTextHeight, 64.0),
        iced::widget::Space::new().width(tokens::GAP_CONTROL),
        icon_btn(
            include_bytes!("../../../assets/icons/mt_bold.svg"),
            Message::MTextFmt(MTextFmt::Bold)
        ),
        icon_btn(
            include_bytes!("../../../assets/icons/mt_italic.svg"),
            Message::MTextFmt(MTextFmt::Italic)
        ),
        icon_btn(
            include_bytes!("../../../assets/icons/mt_underline.svg"),
            Message::MTextFmt(MTextFmt::Underline)
        ),
        icon_btn(
            include_bytes!("../../../assets/icons/mt_overline.svg"),
            Message::MTextFmt(MTextFmt::Overline)
        ),
        icon_btn(
            include_bytes!("../../../assets/icons/mt_strike.svg"),
            Message::MTextFmt(MTextFmt::Strike)
        ),
        icon_btn(
            include_bytes!("../../../assets/icons/mt_upper.svg"),
            Message::MTextFmt(MTextFmt::Uppercase)
        ),
        icon_btn(
            include_bytes!("../../../assets/icons/mt_lower.svg"),
            Message::MTextFmt(MTextFmt::Lowercase)
        ),
        iced::widget::Space::new().width(width),
        color_pl,
    ]
    .spacing(tokens::GAP_TIGHT)
    .align_y(iced::Alignment::Center)
    .width(width);

    // ── Row 2: oblique / width / char-spacing · align · line spacing · OK ─
    let justify = kit::themed_pick_list(
        Some(JustifyChoice(ed.attachment)),
        JustifyChoice::ALL,
        |value| value.to_string(),
    )
    .on_select(|c| Message::MTextJustify(c.0))
    .text_size(tokens::TEXT_LABEL)
    .width(iced::Length::Fixed(112.0));
    let row2 = row![
        kit::icon_btn(lbl("↶"), kit::Role::Standard, 24.0, kit::Press::On(Message::MTextUndo), ""),
        kit::icon_btn(lbl("↷"), kit::Role::Standard, 24.0, kit::Press::On(Message::MTextRedo), ""),
        lbl("O"),
        small_input("0", &ed.oblique, Message::MTextOblique, 48.0),
        lbl("W"),
        small_input("1", &ed.width, Message::MTextWidth, 48.0),
        lbl("◊"),
        small_input("0", &ed.char_space, Message::MTextCharSpace, 48.0),
        iced::widget::Space::new().width(tokens::GAP_CONTROL),
        icon_btn(
            include_bytes!("../../../assets/icons/mt_align_left.svg"),
            Message::MTextAlign(ParaAlign::Left)
        ),
        icon_btn(
            include_bytes!("../../../assets/icons/mt_align_center.svg"),
            Message::MTextAlign(ParaAlign::Center)
        ),
        icon_btn(
            include_bytes!("../../../assets/icons/mt_align_right.svg"),
            Message::MTextAlign(ParaAlign::Right)
        ),
        icon_btn(
            include_bytes!("../../../assets/icons/mt_align_justify.svg"),
            Message::MTextAlign(ParaAlign::Justify)
        ),
        iced::widget::Space::new().width(tokens::GAP_CONTROL),
        justify,
        lbl("LS"),
        kit::icon_btn(lbl("1"), kit::Role::Standard, 24.0, kit::Press::On(Message::MTextLineSpacing(1.0)), ""),
        kit::icon_btn(lbl("1.5"), kit::Role::Standard, 24.0, kit::Press::On(Message::MTextLineSpacing(1.5)), ""),
        kit::icon_btn(lbl("2"), kit::Role::Standard, 24.0, kit::Press::On(Message::MTextLineSpacing(2.0)), ""),
        kit::btn(
            lbl("Stack"),
            kit::Role::Standard,
            kit::Size::Compact,
            kit::Press::On(Message::MTextStack),
            "",
        ),
        kit::btn(
            lbl("Clear"),
            kit::Role::Standard,
            kit::Size::Compact,
            kit::Press::On(Message::MTextClearFormatting),
            "",
        ),
        kit::icon_btn(lbl("°"), kit::Role::Standard, 24.0, kit::Press::On(Message::MTextInsert("°".to_string())), ""),
        kit::icon_btn(lbl("±"), kit::Role::Standard, 24.0, kit::Press::On(Message::MTextInsert("±".to_string())), ""),
        kit::icon_btn(lbl("⌀"), kit::Role::Standard, 24.0, kit::Press::On(Message::MTextInsert("⌀".to_string())), ""),
    ]
    .spacing(tokens::GAP_TIGHT)
    .align_y(iced::Alignment::Center)
    .width(width);

    let column_mode = match (ed.column_type, ed.column_auto_height) {
        (1, _) => "Static",
        (2, true) => "Dynamic auto",
        (2, false) => "Dynamic manual",
        _ => "No columns",
    }
    .to_string();
    let column_picker = kit::themed_pick_list(
        Some(column_mode),
        ["No columns", "Static", "Dynamic auto", "Dynamic manual"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>(),
        |value| t!(value).into_owned(),
    )
    .on_select(Message::MTextColumnMode)
    .text_size(tokens::TEXT_LABEL)
    .width(iced::Length::Fixed(120.0));
    let row3 = row![
        lbl("Columns"),
        column_picker,
        lbl("Count"),
        small_input("2", &ed.column_count, Message::MTextColumnCount, 42.0),
        lbl("Height"),
        small_input("0", &ed.rect_height, Message::MTextColumnHeight, 58.0),
        lbl("Width"),
        small_input("0", &ed.column_width, Message::MTextColumnWidth, 58.0),
        lbl("Gutter"),
        small_input("0", &ed.column_gutter, Message::MTextColumnGutter, 58.0),
        kit::labeled_checkbox(t!("Reverse").into_owned(), ed.column_flow_reversed,
            Message::MTextColumnFlowReversed),
        iced::widget::Space::new().width(tokens::GAP_ROW),
        lbl("First"),
        small_input("0", &ed.paragraph_first_indent, |value| {
            Message::MTextParagraphNumber(super::super::mtext_editor::ParaNumber::FirstIndent, value)
        }, 48.0),
        lbl("Left"),
        small_input("0", &ed.paragraph_left_indent, |value| {
            Message::MTextParagraphNumber(super::super::mtext_editor::ParaNumber::LeftIndent, value)
        }, 48.0),
        lbl("Right"),
        small_input("0", &ed.paragraph_right_indent, |value| {
            Message::MTextParagraphNumber(super::super::mtext_editor::ParaNumber::RightIndent, value)
        }, 48.0),
        lbl("Before"),
        small_input("0", &ed.paragraph_space_before, |value| {
            Message::MTextParagraphNumber(super::super::mtext_editor::ParaNumber::SpaceBefore, value)
        }, 48.0),
        lbl("After"),
        small_input("0", &ed.paragraph_space_after, |value| {
            Message::MTextParagraphNumber(super::super::mtext_editor::ParaNumber::SpaceAfter, value)
        }, 48.0),
    ]
    .spacing(tokens::GAP_TIGHT)
    .align_y(iced::Alignment::Center)
    .width(width);
    let row4 = row![
        lbl("Find"),
        text_input(&t!("Find"), &ed.find_text)
            .on_input(Message::MTextFindText)
            .width(iced::Length::Fixed(140.0))
            .padding(kit::PAD_FIELD)
            .size(tokens::TEXT_BODY)
            .style(kit::field_style),
        kit::btn(
            lbl("Next"),
            kit::Role::Standard,
            kit::Size::Compact,
            kit::Press::On(Message::MTextFindNext),
            "",
        ),
        lbl("Replace"),
        text_input(&t!("Replace"), &ed.replace_text)
            .on_input(Message::MTextReplaceText)
            .width(iced::Length::Fixed(140.0))
            .padding(kit::PAD_FIELD)
            .size(tokens::TEXT_BODY)
            .style(kit::field_style),
        kit::btn(
            lbl("Replace"),
            kit::Role::Standard,
            kit::Size::Compact,
            kit::Press::On(Message::MTextReplaceNext),
            "",
        ),
        kit::btn(
            lbl("Replace All"),
            kit::Role::Standard,
            kit::Size::Compact,
            kit::Press::On(Message::MTextReplaceAll),
            "",
        ),
    ]
    .spacing(tokens::GAP_TIGHT)
    .align_y(iced::Alignment::Center)
    .width(width);

    let preview_scale = ed.preview_scale();
    let slider_min = 1e-6_f64;
    let slider_max = f64::from(writing_area_px / preview_scale).max(slider_min * 2.0);
    let width_slider = column![
        row![
            text(t!(
                "Width: %{value}",
                value = format!("{:.3}", ed.rect_width)
            ))
            .size(tokens::TEXT_LABEL),
            Space::new().width(width),
            text(format!("{:.0}%", ed.rect_width / slider_max * 100.0)).size(tokens::TEXT_LABEL),
        ]
        .width(width),
        container(
            iced::widget::slider(
                slider_min..=slider_max,
                ed.rect_width.clamp(slider_min, slider_max),
                Message::MTextRectWidth,
            )
            .step((slider_max * 0.01).max(1e-6))
            .width(width),
        )
        .padding([tokens::GAP_NONE, MTEXT_PREVIEW_PAD])
        .width(width),
    ]
    .spacing(tokens::GAP_HAIRLINE)
    .width(width);

    // ── Body: the rendered preview (the editor is preview-only). It fills the
    // space left by the toolbars, so the resizable modal's extra height flows
    // into the text area. ─────────────────────────────────────────────────
    let body: Element<'a, Message> = if !matches!(height, iced::Length::Fill) {
        // The preview is a flexible scroll viewport. Measuring its real canvas
        // would make long/wide MText dictate the dialog size, so the intrinsic
        // pass uses an empty viewport proxy. The toolbars alone determine its
        // width; the preview height follows that measured writing width.
        kit::surface(Space::new(), kit::Tier::Well, kit::PAD_WELL, "")
            .width(width)
            .height(preview_height)
            .into()
    } else {
        let segments = mtext_preview_segments(ed);
        let (mut minx, mut miny, mut maxx, mut maxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for (seg, _, _) in &segments {
            for &(x, y) in seg {
                minx = minx.min(x);
                miny = miny.min(y);
                maxx = maxx.max(x);
                maxy = maxy.max(y);
            }
        }
        // Include glyph boxes so all-whitespace / box-only lines still anchor
        // the transform (hit-testing relies on minx/miny).
        for b in &ed.glyph_boxes {
            minx = minx.min(b.xmin);
            miny = miny.min(b.ymin);
            maxx = maxx.max(b.xmax);
            maxy = maxy.max(b.ymax);
        }
        let scale = preview_scale;
        let content_h = if maxx >= minx {
            ((maxy - miny) * scale + 2.0 * MTEXT_PREVIEW_PAD).max(40.0)
        } else {
            40.0
        };
        // Multi-column MTEXT lays its columns out side by side, so the content
        // can be wider than the editor; size the canvas to the real content
        // width and let the scroll area pan horizontally to reach later columns.
        let content_w = if maxx >= minx {
            ((maxx - minx).max(ed.rect_width as f32) * scale + 2.0 * MTEXT_PREVIEW_PAD).max(40.0)
        } else {
            ed.rect_width as f32 * scale + 2.0 * MTEXT_PREVIEW_PAD
        };
        let prog = MTextPreview {
            segments,
            boxes: ed.glyph_boxes.clone(),
            sel: ed.sel,
            caret: ed.caret,
            caret_on: ed.caret_blink_on,
            minx,
            miny,
            scale,
            content_h,
            wrap_width_px: ed.rect_width as f32 * scale,
        };
        let cv = canvas(prog)
            .width(iced::Length::Fixed(content_w))
            .height(iced::Length::Fixed(content_h));
        // The preview fills the space the toolbars leave; it scrolls in BOTH
        // directions so a taller-than-view text scrolls vertically and a
        // wider-than-view multi-column text scrolls horizontally.
        use iced::widget::scrollable::{Direction, Scrollbar};
        kit::surface(
            kit::themed_scrollable(cv)
                .direction(Direction::Both {
                    vertical: Scrollbar::default(),
                    horizontal: Scrollbar::default(),
                })
                .width(width)
                .height(preview_height),
            kit::Tier::Well,
            kit::PAD_WELL,
            "",
        )
        .width(width)
        .height(preview_height)
        .into()
    };

    // ── Top action bar: Apply keeps editing; Close saves and exits. Escape
    // remains the explicit discard path.
    let action_bar = kit::surface(
        row![
            iced::widget::Space::new().width(width),
            crate::ui::style::style_manager::tb_button(t!("Apply"), Message::MTextApply, true),
            crate::ui::style::style_manager::tb_button(
                t!("Close Text Editor"),
                Message::MTextOk,
                true
            ),
        ]
        .align_y(iced::Alignment::Center),
        kit::Tier::Band,
        kit::PAD_BAND,
        "",
    )
    .width(width);

    container(
        column![action_bar, row1, row2, row3, row4, width_slider, body]
            .spacing(tokens::GAP_CONTROL)
            .width(width)
            .height(height),
    )
    .width(width)
    .height(height)
    .into()
}

/// Position a cursor-anchored panel inside the drawing's safe rectangle. The
/// panel is laid out first, so edge flipping uses its actual translated size
/// instead of estimates. `bottom_inset` reserves overlaid controls such as the
/// command line.
///
/// `offset` shifts the preferred placement from the anchor before the fit /
/// flip logic runs (e.g. to put a specific row of a menu under the pointer).
/// When the shifted panel does not fit it flips to the other side of the
/// *original* anchor, so the pointer never ends up over an unintended row.
fn position_canvas_overlay_clamped<'a>(
    anchor: iced::Point,
    bottom_inset: f32,
    offset: Vector,
    panel: Element<'a, Message>,
) -> Element<'a, Message> {
    Element::new(ClampedPin {
        content: iced::widget::opaque(panel),
        anchor,
        bottom_inset: bottom_inset.max(0.0),
        gap: 0.0,
        offset,
    })
}

/// Keep a floating panel close to the cursor without letting it cover the
/// pointer or leave the drawing's safe rectangle.
pub(super) fn position_canvas_overlay_near_cursor<'a>(
    cursor: iced::Point,
    bottom_inset: f32,
    panel: Element<'a, Message>,
) -> Element<'a, Message> {
    Element::new(ClampedPin {
        content: iced::widget::opaque(panel),
        anchor: cursor,
        bottom_inset: bottom_inset.max(0.0),
        gap: 12.0,
        offset: Vector::ZERO,
    })
}

struct ClampedPin<'a> {
    content: Element<'a, Message>,
    anchor: iced::Point,
    bottom_inset: f32,
    gap: f32,
    /// Preferred placement shift from the anchor (applied before fitting).
    offset: Vector,
}

impl Widget<Message, Theme, iced::Renderer> for ClampedPin<'_> {
    fn tag(&self) -> widget::tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> widget::tree::State {
        self.content.as_widget().state()
    }

    fn diff(&mut self, tree: &mut widget::Tree) {
        self.content.as_widget_mut().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        const MARGIN: f32 = 4.0;

        let max = limits.max();
        let safe = Size::new(
            (max.width - MARGIN * 2.0).max(0.0),
            (max.height - self.bottom_inset - MARGIN * 2.0).max(0.0),
        );
        let mut node = self.content.as_widget_mut().layout(
            tree,
            renderer,
            &layout::Limits::new(Size::ZERO, safe),
        );
        let content = node.size();
        let bottom_inset = self.bottom_inset;

        let max_x = (max.width - MARGIN - content.width).max(MARGIN);
        let right = self.anchor.x + self.gap + self.offset.x;
        let left = self.anchor.x - self.gap - content.width;
        let x = if right <= max_x {
            right.max(MARGIN)
        } else if left >= MARGIN {
            left
        } else {
            max_x
        };

        let max_y = (max.height - bottom_inset - MARGIN - content.height).max(MARGIN);
        let below = self.anchor.y + self.gap + self.offset.y;
        let above = self.anchor.y - self.gap - content.height;
        let y = if below <= max_y {
            below.max(MARGIN)
        } else if above >= MARGIN {
            above
        } else {
            max_y
        };

        node = node.move_to(iced::Point::new(x, y));
        let size = limits.resolve(Length::Fill, Length::Fill, node.size());
        layout::Node::with_children(size, vec![node])
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.content.as_widget_mut().operate(
            tree,
            layout.children().next().unwrap(),
            renderer,
            operation,
        );
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            tree,
            event,
            layout.children().next().unwrap(),
            cursor,
            renderer,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            tree,
            layout.children().next().unwrap(),
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if let Some(clipped_viewport) = bounds.intersection(viewport) {
            self.content.as_widget().draw(
                tree,
                renderer,
                theme,
                style,
                layout.children().next().unwrap(),
                cursor,
                &clipped_viewport,
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            tree,
            layout.children().next().unwrap(),
            renderer,
            viewport,
            translation,
        )
    }
}

// ── Viewport right-click context menu ──────────────────────────────────────

/// Horizontal inset so the pointer rests inside the default row, not on the
/// panel's border, when the menu opens.
const MENU_CURSOR_INSET_X: f32 = 24.0;
/// Width of the icon gutter every row reserves so labels line up whether or
/// not the row carries a glyph (object snaps, command icons).
const MENU_GUTTER_W: f32 = 18.0;
const MENU_ICON_SIZE: f32 = 14.0;
/// Trailing check mark of checked rows; sits in the submenu caret column.
const MENU_CHECK_SIZE: f32 = 10.0;

/// The gutter cell: the row's glyph, faded with a disabled row, or empty
/// space of the same width.
fn context_menu_gutter(icon: Option<MenuIcon>, enabled: bool) -> Element<'static, Message> {
    use crate::ui::icons;
    let chrome = |bytes| {
        if enabled {
            icons::themed::<Message>(bytes, MENU_ICON_SIZE)
        } else {
            icons::themed_disabled::<Message>(bytes, MENU_ICON_SIZE)
        }
    };
    let cell: Element<'static, Message> = match icon {
        Some(MenuIcon::Snap(t)) => chrome(icons::osnap(t)),
        Some(MenuIcon::Mtp) => chrome(icons::mtp_icon()),
        Some(MenuIcon::Chrome(bytes)) => chrome(bytes),
        Some(MenuIcon::Tool(bytes)) if enabled => icons::semantic(bytes, MENU_ICON_SIZE),
        Some(MenuIcon::Tool(bytes)) => icons::semantic_disabled(bytes, MENU_ICON_SIZE),
        None => iced::widget::Space::new()
            .width(MENU_ICON_SIZE)
            .height(MENU_ICON_SIZE)
            .into(),
    };
    container(cell)
        .width(Length::Fixed(MENU_GUTTER_W))
        .align_x(iced::Center)
        .align_y(iced::Center)
        .into()
}

/// Render the right-click context menu (rows from
/// `ui::popup::context_menu::build_context_menu`). The panel is placed so the
/// default row sits under the pointer: right-click then left-click in place
/// completes the step without moving the mouse (commercial solutions anchor their shortcut
/// menu the same way). `highlighted` is the keyboard highlight, if any.
///
/// The sheet is opaque: `kit::popup_plate` paints the `Float` rung with the
/// field's edge, the same paper every ribbon dropdown is written on. The menu
/// sits over the drawing, so the drawing must not read through its rows.
pub(super) fn viewport_context_menu_overlay(
    pos: iced::Point,
    bottom_inset: f32,
    menu: &ContextMenu,
    highlighted: Option<usize>,
) -> Element<'static, Message> {
    let sep = || -> Element<'static, Message> { kit::hdivider(Fill) };
    // An expanded submenu's rows arrive on their own track: the panel is
    // already on screen, so fading the whole menu again would read as a
    // re-open. Replayed by the toggle, so collapsing and re-expanding
    // replays; a plain re-layout does not.
    let sub = crate::ui::motion::value(crate::ui::motion::MENU_SUB);

    // Index into `menu.selectable()` of the row being rendered, so the
    // keyboard highlight lands on the same row the model counts.
    let mut sel_idx = 0usize;
    let mut items: Vec<Element<'static, Message>> = Vec::new();

    for row in &menu.rows {
        match row {
            MenuRow::Item(item) => {
                let is_hl = highlighted == Some(sel_idx);
                items.push(context_menu_row(item, 0.0, is_hl));
                sel_idx += 1;
            }
            MenuRow::Separator => items.push(sep()),
            MenuRow::Submenu {
                id,
                label,
                icon,
                items: children,
                open,
            } => {
                let is_hl = highlighted == Some(sel_idx);
                let enabled = !children.is_empty();
                let caret = if !enabled {
                    crate::ui::icons::themed_disabled_arrow_right(9.0)
                } else if *open {
                    crate::ui::icons::themed_arrow_down(9.0)
                } else {
                    crate::ui::icons::themed_arrow_right(9.0)
                };
                let content = row![
                    context_menu_gutter(*icon, enabled),
                    text(label.clone()).size(tokens::TEXT_BODY),
                    iced::widget::Space::new().width(Fill),
                    caret,
                ]
                .spacing(tokens::GAP_TIGHT)
                .align_y(iced::Center);
                let mut btn = plain_button(content)
                    .padding(iced::Padding {
                        top: 3.0,
                        right: 12.0,
                        bottom: 3.0,
                        left: 6.0,
                    })
                    .width(Fill)
                    .height(Length::Fixed(MENU_ROW_H))
                    .style(move |theme: &Theme, status| {
                        kit::menu_row_style(theme, status, is_hl)
                    });
                if enabled {
                    btn = btn.on_press(Message::ContextMenuSubmenuToggle(*id));
                }
                items.push(btn.into());
                sel_idx += 1;
                if *open {
                    for child in children {
                        let is_hl = highlighted == Some(sel_idx);
                        items.push(kit::alpha::entrance(sub, || {
                            context_menu_row(child, 14.0, is_hl)
                        }));
                        sel_idx += 1;
                    }
                }
            }
        }
    }

    let menu_col = column(items).spacing(tokens::GAP_NONE).width(Length::Fixed(menu.width));
    let menu_col =
        kit::themed_scrollable(menu_col)
            .height(Length::Shrink)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(8).scroller_width(6),
            ));

    // The menu is written on the same paper as a dropdown — `popup_plate`, the
    // `Float` rung wearing the field's edge. `popup_plate` already holds its
    // content one gap rung off the plate's rounded corner; this child keeps
    // only the vertical pause above the first row, so the two insets do not
    // stack into a double margin. `Flush` on the inner pane keeps the paper the
    // only thing painting the plate, so the menu keeps its exact size.
    let plate = container(menu_col)
        .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
        .padding([MENU_PAD_TOP, tokens::GAP_NONE])
        .width(Length::Fixed(menu.width));

    // The menu arrives out of the pointer: a 6 px slide on both axes and a
    // fade, on the hover budget so it never feels heavier than pointing at a
    // row. The track is replayed when the menu opens and stays at 1.0
    // afterwards, so a menu that is merely re-laid out (a submenu toggling)
    // does not re-enter.
    let open = crate::ui::motion::value(crate::ui::motion::MENU_OPEN);
    let panel = kit::popup_plate(
        kit::alpha::entrance(open, || plate.into()),
        "",
    );

    // Shift the panel so the default row's centre line is under the pointer.
    // The top inset is the paper's own gap rung plus the row's measured offset,
    // otherwise the default row would land one padding step above the cursor.
    let offset = Vector::new(
        -MENU_CURSOR_INSET_X,
        -(tokens::GAP_CONTROL + menu.default_row_y() + MENU_ROW_H * 0.5),
    ) + crate::ui::motion::menu_slide(open);
    position_canvas_overlay_clamped(pos, bottom_inset, offset, panel)
}

/// One menu row: label (bold for the default row), the keyword hint
/// right-aligned and dimmed, and a check mark in the trailing column (where
/// submenu headers draw their caret) when checked, so checked labels stay
/// aligned with the others. Disabled rows render without a press handler and
/// with faded text and icons.
fn context_menu_row(item: &MenuItem, indent: f32, highlighted: bool) -> Element<'static, Message> {
    let mut label_text = text(crate::t!(item.label.clone())).size(tokens::TEXT_BODY);
    if item.default {
        label_text = label_text.font(tokens::UI_FONT_SEMIBOLD);
    }
    let enabled = item.enabled;
    let mut content = row![context_menu_gutter(item.icon, enabled), label_text]
        .spacing(tokens::GAP_TIGHT)
        .align_y(iced::Center);
    if let Some(hint) = item.hint.as_ref() {
        content = content.push(iced::widget::Space::new().width(Fill)).push(
            text(hint.clone()).size(tokens::TEXT_LABEL).style(move |theme: &Theme| {
                let palette = theme.palette();
                let base = if highlighted {
                    palette.primary.strong.text
                } else {
                    palette.background.base.text
                };
                kit::color_style(base.scale_alpha(if enabled { 0.6 } else { 0.35 }))
            }),
        );
    }
    if item.checked {
        if item.hint.is_none() {
            content = content.push(iced::widget::Space::new().width(Fill));
        }
        content = content.push(if enabled {
            crate::ui::icons::themed(crate::ui::icons::CHECK, MENU_CHECK_SIZE)
        } else {
            crate::ui::icons::themed_disabled(crate::ui::icons::CHECK, MENU_CHECK_SIZE)
        });
    }
    let mut btn = plain_button(content)
        .padding(iced::Padding {
            top: 3.0,
            right: 12.0,
            bottom: 3.0,
            left: 6.0 + indent,
        })
        .width(Fill)
        .height(Length::Fixed(MENU_ROW_H))
        .style(move |theme: &Theme, status| kit::menu_row_style(theme, status, highlighted));
    if enabled {
        btn = btn.on_press(Message::ContextMenuPick(item.action.clone()));
    }
    btn.into()
}

/// One-shot snap override menu (Shift+RMB, #337): a cursor-anchored grid of
/// snap ICONS only — the names show as hover tooltips. Picking one applies
/// that snap to just the next point pick; the trailing MTP cell instead
/// suspends the prompt for two picks and returns their midpoint.
pub(super) fn snap_override_overlay(pos: iced::Point, theme: &Theme) -> Element<'static, Message> {
    const COLS: usize = 4;

    let cell_icon =
        |icon: &'static [u8], label: String, msg: Message| -> Element<'static, Message> {
            let btn = kit::icon_btn(
                crate::ui::icons::themed::<Message>(icon, 16.0),
                kit::Role::Ghost,
                26.0,
                kit::Press::On(msg),
                "",
            );
            let tip = kit::glass_pane(
                kit::surface(
                    text(label).size(tokens::TEXT_LABEL),
                    kit::Tier::Flush,
                    kit::PAD_PLATE,
                    "",
                )
                .into(),
                theme,
                false,
                "",
            );
            iced::widget::tooltip(btn, tip, iced::widget::tooltip::Position::Bottom).into()
        };

    // MTP (`_M2P`) goes last, after Parallel: two picks, so not a
    // `SnapType`, but same icon-only cell with hover tooltip.
    let mut cells: Vec<Element<'static, Message>> =
        Vec::with_capacity(crate::snap::ALL_SNAP_MODES.len() + 1);
    for &(snap_type, _glyph, label) in crate::snap::ALL_SNAP_MODES {
        cells.push(cell_icon(
            crate::ui::icons::osnap(snap_type),
            label.to_string(),
            Message::SnapOverridePick(snap_type),
        ));
    }
    cells.push(cell_icon(
        crate::ui::icons::mtp_icon(),
        t!("Mid Between 2 Points (M2P)").into_owned(),
        Message::SnapOverrideMtp,
    ));
    let mut grid = column![].spacing(tokens::GAP_HAIRLINE);
    while !cells.is_empty() {
        let n = cells.len().min(COLS);
        let mut r = row![].spacing(tokens::GAP_HAIRLINE);
        for c in cells.drain(..n) {
            r = r.push(c);
        }
        grid = grid.push(r);
    }

    let panel = kit::glass_pane(
        kit::surface(grid, kit::Tier::Flush, kit::PAD_PLATE, "").into(),
        theme,
        false,
        "",
    );

    // Full-screen click-catcher closes on an outside click.
    let catcher = mouse_area(
        container(iced::widget::Space::new().width(Fill).height(Fill))
            .width(Fill)
            .height(Fill),
    )
    .on_press(Message::SnapOverrideClose)
    .on_right_press(Message::SnapOverrideClose);

    stack![catcher, position_canvas_overlay(pos, panel.into())].into()
}

// ── Quick Select panel ─────────────────────────────────────────────────────

const QSELECT_ANY_TYPE: &str = "(Any type)";
const QSELECT_ANY_PROP: &str = "(Any property)";

/// Quick Select uses the application's shared movable modal frame. Its form
/// groups scope, filter, and result behavior while reusing the same compact
/// field and button styles as the other workspace dialogs.
pub(super) fn qselect_overlay<'a>(
    state: &'a crate::app::QSelectState,
    types: &[String],
    properties: &[crate::app::QSelectPropertyChoice],
    candidate_count: usize,
    modal_offset: iced::Vector,
    modal_resize: iced::Vector,
    theme: &Theme,
) -> Element<'a, Message> {
    let measurement = qselect_content(
        state,
        types,
        properties,
        candidate_count,
        crate::ui::shell::Sizing::INTRINSIC,
    );
    let content = qselect_content(
        state,
        types,
        properties,
        candidate_count,
        crate::ui::shell::Sizing::FILL,
    );
    let content =
        crate::ui::shell::intrinsic(measurement, content, iced::Size::INFINITE, modal_resize);

    crate::ui::shell::modal(
        Space::new().width(Fill).height(Fill),
        t!("Quick Select"),
        content,
        Message::QSelectClose,
        modal_offset,
        crate::ui::shell::ModalOptions::STANDARD,
        None,
        theme,
    )
}

fn qselect_content<'a>(
    state: &'a crate::app::QSelectState,
    types: &[String],
    properties: &[crate::app::QSelectPropertyChoice],
    candidate_count: usize,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    use crate::ui::kit::{dialog_button, dialog_button_styled_opt, form_radio};
    use iced::widget::{checkbox, rule};
    let mut type_options: Vec<String> = vec![QSELECT_ANY_TYPE.to_string()];
    type_options.extend(types.iter().cloned());

    let mut prop_options: Vec<crate::app::QSelectPropertyChoice> =
        vec![crate::app::QSelectPropertyChoice {
            field: String::new(),
            label: t!(QSELECT_ANY_PROP).into_owned(),
            editor: crate::app::QSelectValueEditor::Text,
        }];
    prop_options.extend(properties.iter().cloned());

    let number_property = state
        .property
        .as_ref()
        .is_some_and(|property| matches!(property.editor, crate::app::QSelectValueEditor::Number));
    let mut op_options: Vec<crate::app::QSelectOp> =
        vec![crate::app::QSelectOp::Eq, crate::app::QSelectOp::Neq];
    if number_property {
        op_options.push(crate::app::QSelectOp::Gt);
        op_options.push(crate::app::QSelectOp::Lt);
    }
    op_options.push(crate::app::QSelectOp::Any);

    let type_sel = state
        .type_filter
        .clone()
        .unwrap_or_else(|| QSELECT_ANY_TYPE.to_string());
    let prop_sel = state
        .property
        .clone()
        .unwrap_or(crate::app::QSelectPropertyChoice {
            field: String::new(),
            label: t!(QSELECT_ANY_PROP).into_owned(),
            editor: crate::app::QSelectValueEditor::Text,
        });

    let value_enabled =
        state.property.is_some() && !matches!(state.operator, crate::app::QSelectOp::Any);
    let field_width = if matches!(sizing.width, iced::Length::Fill) {
        Fill
    } else {
        iced::Length::Shrink
    };
    let flex_width = if matches!(sizing.width, iced::Length::Fill) {
        Fill
    } else {
        iced::Length::Shrink
    };

    let label =
        |s: std::borrow::Cow<'static, str>| text(s).size(tokens::TEXT_BODY).width(iced::Length::Fixed(112.0));

    let value_editor: Element<'a, Message> = match state.property.as_ref() {
        Some(property) => match &property.editor {
            crate::app::QSelectValueEditor::Choice(options) => {
                let selected = (!state.value.is_empty()).then(|| state.value.clone());
                let picker =
                    kit::themed_pick_list(selected, options.clone(), |value| value.to_string())
                        .width(field_width);
                if value_enabled {
                    picker.on_select(Message::QSelectSetValue).into()
                } else {
                    picker.into()
                }
            }
            crate::app::QSelectValueEditor::Text | crate::app::QSelectValueEditor::Number => {
                let mut input = text_input("", &state.value).size(tokens::TEXT_BODY).width(field_width);
                if value_enabled {
                    input = input.on_input(Message::QSelectSetValue);
                }
                input.into()
            }
        },
        None => text_input("", "").size(tokens::TEXT_BODY).width(field_width).into(),
    };

    let derived_error = if candidate_count == 0 {
        Some(crate::t!("No objects are available in this scope.").into_owned())
    } else if value_enabled
        && number_property
        && crate::entities::common::parse_f64(&state.value).is_none()
    {
        Some(crate::t!("Enter a valid number.").into_owned())
    } else if value_enabled
        && state.property.as_ref().is_some_and(|property| {
            matches!(property.editor, crate::app::QSelectValueEditor::Choice(_))
        })
        && state.value.is_empty()
    {
        Some(crate::t!("Choose a value.").into_owned())
    } else if matches!(
        state.operator,
        crate::app::QSelectOp::Gt | crate::app::QSelectOp::Lt
    ) && !number_property
    {
        Some(crate::t!("This operator requires a numeric property.").into_owned())
    } else {
        None
    };
    let error = state.error.clone().or(derived_error);

    let scope_options = vec![
        crate::app::QSelectScope::CurrentSpace,
        crate::app::QSelectScope::CurrentSelection,
    ];
    let scope_picker =
        kit::themed_pick_list(Some(state.scope), scope_options, |value| value.to_string())
            .on_select(Message::QSelectSetScope)
            .width(field_width);

    let mut append = checkbox(state.append).size(tokens::MARK_SQUARE);
    if matches!(state.scope, crate::app::QSelectScope::CurrentSpace) {
        append = append.on_toggle(Message::QSelectSetAppend);
    }

    let cancel = dialog_button(t!("Cancel"), Message::QSelectClose, false);
    let apply = dialog_button_styled_opt(
        t!("Apply"),
        error.is_none().then_some(Message::QSelectApply),
        kit::primary_style,
    );

    let panel_body = column![
        kit::section_label(t!("Scope")),
        Space::new().height(tokens::GAP_TIGHT),
        row![label(t!("Apply to:")), scope_picker]
            .align_y(iced::Alignment::Center)
            .spacing(tokens::GAP_ROW)
            .width(sizing.width),
        Space::new().height(tokens::GAP_TIGHT),
        text(crate::tf!("{} candidate object(s)", candidate_count))
            .size(tokens::TEXT_LABEL)
            .style(kit::muted_style),
        Space::new().height(tokens::GAP_FIELD),
        container(rule::horizontal(1)).width(flex_width),
        Space::new().height(tokens::GAP_FIELD),
        kit::section_label(t!("Filter")),
        Space::new().height(tokens::GAP_TIGHT),
        row![
            label(t!("Object type:")),
            kit::themed_pick_list(Some(type_sel), type_options, |value| t!(value).into_owned(),)
                .on_select(|s: String| {
                    if s == QSELECT_ANY_TYPE {
                        Message::QSelectSetType(None)
                    } else {
                        Message::QSelectSetType(Some(s))
                    }
                })
                .width(field_width),
        ]
        .align_y(iced::Alignment::Center)
        .spacing(tokens::GAP_ROW)
        .width(sizing.width),
        Space::new().height(tokens::GAP_CONTROL),
        row![
            label(t!("Property:")),
            kit::themed_pick_list(Some(prop_sel), prop_options, |value| crate::t!(value.to_string()).into_owned(),)
                .on_select(|p: crate::app::QSelectPropertyChoice| {
                    if p.field.is_empty() {
                        Message::QSelectSetProperty(None)
                    } else {
                        Message::QSelectSetProperty(Some(p))
                    }
                })
                .width(field_width),
        ]
        .align_y(iced::Alignment::Center)
        .spacing(tokens::GAP_ROW)
        .width(sizing.width),
        Space::new().height(tokens::GAP_CONTROL),
        row![
            label(t!("Operator:")),
            kit::themed_pick_list(Some(state.operator), op_options, |value| value.to_string(),)
                .on_select(Message::QSelectSetOperator)
                .width(field_width),
        ]
        .align_y(iced::Alignment::Center)
        .spacing(tokens::GAP_ROW)
        .width(sizing.width),
        Space::new().height(tokens::GAP_CONTROL),
        row![label(t!("Value:")), value_editor]
            .align_y(iced::Alignment::Center)
            .spacing(tokens::GAP_ROW)
            .width(sizing.width),
        Space::new().height(tokens::GAP_FIELD),
        container(rule::horizontal(1)).width(flex_width),
        Space::new().height(tokens::GAP_FIELD),
        kit::section_label(t!("Result")),
        Space::new().height(tokens::GAP_TIGHT),
        column![
            form_radio(
                t!("Include matching objects"),
                crate::app::QSelectMode::Include,
                Some(state.mode),
                Message::QSelectSetMode,
            ),
            form_radio(
                t!("Exclude matching objects"),
                crate::app::QSelectMode::Exclude,
                Some(state.mode),
                Message::QSelectSetMode,
            ),
        ]
        .spacing(tokens::GAP_TIGHT),
        Space::new().height(tokens::GAP_ROW),
        row![
            append,
            Space::new().width(tokens::GAP_CONTROL),
            text(t!("Append result to current selection")).size(tokens::TEXT_BODY),
        ]
        .align_y(iced::Alignment::Center),
        Space::new().height(tokens::GAP_FIELD),
        if let Some(message) = error {
            text(message)
                .size(tokens::TEXT_LABEL)
                .style(|theme: &Theme| kit::tone_style(theme, kit::Tone::Danger))
        } else {
            text("").size(tokens::TEXT_LABEL)
        },
        Space::new().height(tokens::GAP_ROW),
        row![
            Space::new().width(flex_width),
            cancel,
            Space::new().width(tokens::GAP_ROW),
            apply,
        ]
        .align_y(iced::Alignment::Center),
    ]
    .spacing(tokens::GAP_NONE)
    .width(sizing.width)
    .height(sizing.height);

    // The dialog's own glass plate is behind this content: `shell::modal` puts
    // one pane under the whole body. A second plate here would be a box inside
    // the box, so the host keeps only the `PAD_CARD` inset between the glass and
    // the form and paints nothing itself.
    let panel = kit::surface(panel_body, kit::Tier::Flush, kit::PAD_CARD, "")
        .width(sizing.width)
        .height(sizing.height);

    panel.into()
}
