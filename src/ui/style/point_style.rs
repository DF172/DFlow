//! Point Style (DDPTYPE) dialog — pick a PDMODE glyph from a grid and set the
//! point size (PDSIZE), relative to the screen or in absolute units.
//!
//! Changes apply live to the active document header; the renderer rebuilds the
//! point glyphs (see `entities::point`). Command entry (`PDMODE` / `PDSIZE`)
//! does the same without the dialog.

use crate::ui::style::tokens;
use crate::app::Message;
use crate::t;
use crate::ui::kit::muted_style;
use crate::ui::kit;
use iced::widget::{canvas, column, container, row, text, text_input, Space};
use crate::ui::kit::plain_button;
use iced::{mouse, Element, Length, Point, Rectangle, Size, Theme};
use std::borrow::Cow;

const CELL_PX: f32 = 44.0;

/// Glyph-grid columns (low-nibble shape) × rows (enclosure bits):
///   shapes:     0=dot, 1=none, 2='+', 3='×', 4='|'
///   enclosures: 0=none, 32=circle, 64=square, 96=both
const ENCLOSURES: [i16; 4] = [0, 32, 64, 96];
const SHAPES: [i16; 5] = [0, 1, 2, 3, 4];

/// Canvas that renders a single PDMODE glyph inside a grid cell.
struct GlyphCanvas {
    mode: i16,
}

impl canvas::Program<Message> for GlyphCanvas {
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
        let glyph = theme.palette().background.base.text;
        let (cx, cy) = (bounds.width * 0.5, bounds.height * 0.5);
        let r = bounds.width.min(bounds.height) * 0.30;
        let stroke = canvas::Stroke {
            width: 1.4,
            style: canvas::Style::Solid(glyph),
            ..Default::default()
        };
        let line = |a: Point, b: Point| canvas::Path::line(a, b);

        match self.mode & 0x0F {
            0 => frame.fill(&canvas::Path::circle(Point::new(cx, cy), 2.4), glyph),
            1 => {}
            2 => {
                frame.stroke(
                    &line(Point::new(cx - r, cy), Point::new(cx + r, cy)),
                    stroke.clone(),
                );
                frame.stroke(
                    &line(Point::new(cx, cy - r), Point::new(cx, cy + r)),
                    stroke.clone(),
                );
            }
            3 => {
                frame.stroke(
                    &line(Point::new(cx - r, cy - r), Point::new(cx + r, cy + r)),
                    stroke.clone(),
                );
                frame.stroke(
                    &line(Point::new(cx - r, cy + r), Point::new(cx + r, cy - r)),
                    stroke.clone(),
                );
            }
            4 => frame.stroke(
                &line(Point::new(cx, cy), Point::new(cx, cy - r)),
                stroke.clone(),
            ),
            _ => {}
        }
        if self.mode & 32 != 0 {
            frame.stroke(&canvas::Path::circle(Point::new(cx, cy), r), stroke.clone());
        }
        if self.mode & 64 != 0 {
            let sq =
                canvas::Path::rectangle(Point::new(cx - r, cy - r), Size::new(2.0 * r, 2.0 * r));
            frame.stroke(&sq, stroke.clone());
        }
        vec![frame.into_geometry()]
    }
}

fn cell<'a>(value: i16, selected: bool) -> Element<'a, Message> {
    let glyph = canvas(GlyphCanvas { mode: value })
        .width(Length::Fixed(CELL_PX))
        .height(Length::Fixed(CELL_PX));
    plain_button(glyph)
        .padding(tokens::GAP_NONE)
        .on_press(Message::PointStyleSetMode(value))
        .style(move |theme: &Theme, status| {
            kit::glyph_cell_style(theme, status, selected, tokens::aurora_gradient(theme, 0.14))
        })
        .into()
}

pub fn view_window<'a>(
    pdmode: i16,
    relative: bool,
    size_buf: &str,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let height = sizing.height;
    // The sample sheet opens at this measure: the glyph grid is left-aligned
    // as one centred band, so the dialog is not sized by its 5-cell row.
    let frame_width = if matches!(sizing.width, Length::Fill) {
        Length::Fill
    } else {
        Length::Fixed(459.0)
    };
    let width = frame_width;
    // Glyph grid: a row per enclosure, a cell per shape.
    let mut grid = column![].spacing(tokens::GAP_CONTROL);
    for enc in ENCLOSURES {
        let mut r = row![].spacing(tokens::GAP_CONTROL);
        for sh in SHAPES {
            let value = enc + sh;
            r = r.push(cell(value, value == pdmode));
        }
        grid = grid.push(r);
    }

    let size_row = row![
        text(t!("Point Size:")).size(tokens::TEXT_META),
        Space::new().width(10),
        text_input("0", size_buf)
            .on_input(Message::PointStyleSizeInput)
            .on_submit(Message::PointStyleApplySize)
            .style(kit::field_style)
            .size(tokens::TEXT_META)
            .width(110),
        Space::new().width(6),
        text(if relative {
            Cow::Borrowed("%")
        } else {
            t!("units")
        })
        .size(tokens::TEXT_BODY)
        .style(muted_style),
    ]
    .align_y(iced::Center);

    let radios = column![
        kit::form_radio(
            t!("Set Size Relative to Screen"),
            true,
            Some(relative),
            Message::PointStyleSizeRelative,
        ),
        kit::form_radio(
            t!("Set Size in Absolute Units"),
            false,
            Some(relative),
            Message::PointStyleSizeRelative,
        ),
    ]
    .spacing(tokens::GAP_CONTROL);

    let ok = kit::dialog_button(t!("OK"), Message::PointStyleOk, true);

    let body = column![
        grid,
        Space::new().height(tokens::GAP_SECTION),
        size_row,
        Space::new().height(tokens::GAP_ROW),
        radios,
    ]
    .spacing(tokens::GAP_TIGHT);

    container(
        column![
            container(body).width(width).align_x(iced::alignment::Horizontal::Center),
            Space::new().height(tokens::GAP_SECTION),
            row![Space::new().width(width), ok].width(width),
        ]
        .spacing(tokens::GAP_TIGHT)
        .padding(tokens::PAD_PAGE)
        .width(width)
        .height(height),
    )
    .width(width)
    .height(height)
    .into()
}
