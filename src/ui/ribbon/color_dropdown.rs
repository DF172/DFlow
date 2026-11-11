//! Object Color Dropdown panel.
//!
//! Provides a 5-section flyout ribbon color control:
//! 1. ByLayer and ByBlock logical color entries
//! 2. 45-swatch Quick Pick Grid (5 rows × 9 columns)
//! 3. Standard Index Colors (ACI 1–9)
//! 4. Recent Colors row (up to 9 swatches)
//! 5. "Select Color..." action to open the full Select Color dialog

use crate::app::Message;
use crate::t;
use crate::ui::kit;
use crate::ui::properties::acad_color_display;
use crate::ui::style::tokens;
use codec::types::Color as AcadColor;
use iced::widget::{column, container, row, text, Space};
use crate::ui::kit::plain_button;
use iced::{Element, Length, Padding, Theme};

pub const PANEL_W: f32 = 202.0;
const SWATCH_SIZE: f32 = 18.0;
const SWATCH_GAP: f32 = 2.0;

/// Curated 45-swatch quick pick grid measured from the AutoCAD ribbon control.
pub const QUICK_PICK_GRID: [[(u8, u8, u8); 9]; 5] = [
    [
        (0x00, 0x00, 0x00),
        (0xF8, 0xD7, 0x31),
        (0xCD, 0x69, 0x28),
        (0x99, 0x1B, 0x1E),
        (0x64, 0x21, 0x65),
        (0x29, 0x31, 0x89),
        (0x22, 0x34, 0x6E),
        (0x13, 0x67, 0x34),
        (0x72, 0x74, 0x30),
    ],
    [
        (0x63, 0x64, 0x66),
        (0xF1, 0xEB, 0x1F),
        (0xF2, 0x67, 0x22),
        (0xCD, 0x20, 0x27),
        (0x93, 0x27, 0x8F),
        (0x45, 0x54, 0xA5),
        (0x10, 0x56, 0x89),
        (0x13, 0x9B, 0x48),
        (0x99, 0x9B, 0x37),
    ],
    [
        (0x93, 0x95, 0x98),
        (0xF4, 0xEE, 0x51),
        (0xF8, 0x99, 0x1E),
        (0xED, 0x1F, 0x24),
        (0xA9, 0x53, 0xA0),
        (0x5E, 0x67, 0xAF),
        (0x27, 0x76, 0xBB),
        (0x58, 0xBA, 0x48),
        (0xC8, 0xC9, 0x2D),
    ],
    [
        (0xC7, 0xC8, 0xCA),
        (0xF7, 0xF2, 0x81),
        (0xFE, 0xCC, 0x66),
        (0xF2, 0x71, 0x72),
        (0xD1, 0x9C, 0xC7),
        (0x98, 0x98, 0xCC),
        (0x7A, 0xAF, 0xDF),
        (0xAF, 0xD9, 0xAA),
        (0xE5, 0xE8, 0x78),
    ],
    [
        (0xFF, 0xFF, 0xFF),
        (0xF8, 0xF6, 0xB0),
        (0xFE, 0xEA, 0xB9),
        (0xF7, 0xAB, 0xAE),
        (0xDD, 0xB3, 0xD4),
        (0xCE, 0xCC, 0xE6),
        (0xAD, 0xDD, 0xF7),
        (0xD7, 0xEB, 0xD2),
        (0xED, 0xEE, 0x99),
    ],
];

/// The standard ACI 1..=9 index colors pulled directly from the ACI color table.
#[allow(dead_code)]
pub fn aci_1_9() -> [(u8, u8, u8); 9] {
    std::array::from_fn(|i| {
        codec::types::aci_table::aci_to_rgb((i + 1) as u8).unwrap_or((128, 128, 128))
    })
}

fn swatch_btn<'a>(color: AcadColor, cur_color: AcadColor) -> Element<'a, Message> {
    let (bg, _) = acad_color_display(color);
    let selected = color == cur_color;

    plain_button(
        Space::new()
            .width(Length::Fixed(SWATCH_SIZE))
            .height(Length::Fixed(SWATCH_SIZE)),
    )
    .on_press(Message::RibbonColorChanged(color))
    .style(kit::swatch_button_style(bg, selected))
    .padding(tokens::GAP_NONE)
    .into()
}

fn placeholder_swatch<'a>() -> Element<'a, Message> {
    container(
        Space::new()
            .width(Length::Fixed(SWATCH_SIZE))
            .height(Length::Fixed(SWATCH_SIZE)),
    )
    .style(|theme: &Theme| kit::mark_plate_style(theme, ""))
    .into()
}

fn divider<'a>() -> Element<'a, Message> {
    kit::rule_h("").into()
}

fn logical_row<'a>(
    color: AcadColor,
    cur_color: AcadColor,
    label: &'static str,
) -> Element<'a, Message> {
    let (bg, _) = acad_color_display(color);
    let selected = color == cur_color;

    let swatch = container(
        Space::new()
            .width(Length::Fixed(SWATCH_SIZE))
            .height(Length::Fixed(SWATCH_SIZE)),
    )
    .style(kit::swatch_plate_style(bg, selected));

    plain_button(
        row![swatch, text(t!(label)).size(tokens::TEXT_LABEL),]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center),
    )
    .on_press(Message::RibbonColorChanged(color))
    .style(kit::ghost_style)
    .padding(kit::PAD_CELL)
    .width(Length::Fill)
    .into()
}

fn more_colors_row<'a>(active_color: AcadColor) -> Element<'a, Message> {
    plain_button(row![text(t!("Select Color...")).size(tokens::TEXT_LABEL)].align_y(iced::Center))
        .on_press(Message::OpenColorWindow(
            crate::app::ColorPickTarget::Ribbon,
            active_color,
        ))
        .style(kit::ghost_style)
        .padding([tokens::GAP_TIGHT, tokens::GAP_TIGHT])
        .width(Length::Fill)
        .into()
}

/// Builds the complete 5-section color dropdown panel element.
///
/// The panel's paper is `kit::popup_plate`'s job, the same `Float` rung with
/// the field's edge that every other ribbon dropdown wears; the palette only
/// supplies the sections and swatches.
pub fn color_dropdown_panel<'a>(
    active_color: AcadColor,
    recent_colors: &[AcadColor],
) -> Element<'a, Message> {
    // 1. ByLayer and ByBlock
    let by_layer = logical_row(AcadColor::ByLayer, active_color, "ByLayer");
    let by_block = logical_row(AcadColor::ByBlock, active_color, "ByBlock");
    let section1 = column![by_layer, by_block].spacing(tokens::GAP_HAIRLINE);

    // 2. 45-swatch Quick Pick Grid (5 rows × 9 columns)
    let mut grid_rows = column![].spacing(SWATCH_GAP);
    for row_colors in QUICK_PICK_GRID {
        let mut r = row![].spacing(SWATCH_GAP);
        for (red, green, blue) in row_colors {
            let color = AcadColor::Rgb {
                r: red,
                g: green,
                b: blue,
            };
            r = r.push(swatch_btn(color, active_color));
        }
        grid_rows = grid_rows.push(r);
    }

    // 3. Standard Index Colors (ACI 1..=9)
    let index_header = text(t!("Index Color"))
        .size(tokens::TEXT_MICRO)
        .style(kit::muted_style);
    let mut standard_row = row![].spacing(SWATCH_GAP);
    for idx in 1u8..=9 {
        let color = AcadColor::Index(idx);
        standard_row = standard_row.push(swatch_btn(color, active_color));
    }
    let section3 = column![index_header, standard_row].spacing(tokens::GAP_TIGHT);

    // 4. Recent Colors row (up to 9 swatches; pad with placeholders if fewer)
    let recent_header = text(t!("Recent Colors"))
        .size(tokens::TEXT_MICRO)
        .style(kit::muted_style);
    let mut recent_row = row![].spacing(SWATCH_GAP);
    let mut count = 0;
    for &color in recent_colors.iter().take(9) {
        recent_row = recent_row.push(swatch_btn(color, active_color));
        count += 1;
    }
    while count < 9 {
        recent_row = recent_row.push(placeholder_swatch());
        count += 1;
    }
    let section4 = column![recent_header, recent_row].spacing(tokens::GAP_TIGHT);

    // 5. Select Color...
    let section5 = more_colors_row(active_color);

    let content = column![
        section1,
        divider(),
        grid_rows,
        divider(),
        section3,
        divider(),
        section4,
        divider(),
        section5,
    ]
    .spacing(tokens::GAP_CONTROL);

    kit::popup_plate(
        container(content)
            .padding(Padding {
                top: 6.0,
                bottom: 6.0,
                left: 12.0,
                right: 12.0,
            })
            .width(Length::Fixed(PANEL_W))
            .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
            .into(),
        "",
    )
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quick_pick_grid_dimensions() {
        assert_eq!(QUICK_PICK_GRID.len(), 5);
        for row in &QUICK_PICK_GRID {
            assert_eq!(row.len(), 9);
        }
    }

    #[test]
    fn test_aci_1_9() {
        let colors = aci_1_9();
        assert_eq!(colors.len(), 9);
        assert_eq!(colors[0], (255, 0, 0)); // Red
        assert_eq!(colors[1], (255, 255, 0)); // Yellow
        assert_eq!(colors[2], (0, 255, 0)); // Green
        assert_eq!(colors[3], (0, 255, 255)); // Cyan
        assert_eq!(colors[4], (0, 0, 255)); // Blue
        assert_eq!(colors[5], (255, 0, 255)); // Magenta
        assert_eq!(colors[6], (255, 255, 255)); // White
        assert_eq!(colors[7], (128, 128, 128)); // Dark Gray
        assert_eq!(colors[8], (192, 192, 192)); // Light Gray
    }

    #[test]
    fn test_panel_geometry() {
        assert_eq!(PANEL_W, 202.0);
        let grid_width = 9.0 * SWATCH_SIZE + 8.0 * SWATCH_GAP;
        assert_eq!(grid_width, 178.0);
        let padding_h = (PANEL_W - grid_width) / 2.0;
        assert_eq!(padding_h, 12.0);
    }
}
