//! Plot Style Table Editor window — fills the entire OS window.

use crate::ui::style::tokens;
use crate::app::Message;
use crate::t;
use crate::ui::kit::muted_style;
use crate::ui::kit;
use iced::widget::{column, container, row, text, text_input, Space};
use iced::Element;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
struct PlotLineweightItem {
    index: u8,
    label: String,
}

impl fmt::Display for PlotLineweightItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label)
    }
}

fn plot_lineweight_options() -> Vec<PlotLineweightItem> {
    let mut options = vec![PlotLineweightItem {
        index: 255,
        label: t!("Use object lineweight").into_owned(),
    }];

    options.extend(
        crate::io::plot_style::LW_TABLE
            .iter()
            .enumerate()
            .map(|(index, lw)| PlotLineweightItem {
                index: index as u8,
                label: crate::tf!("{lw:.2} mm", lw = lw).into_owned(),
            }),
    );

    options
}

/// Layer names bucketed by ACI (AutoCAD Color Index), index = ACI.
///
/// Pure function of `document.layers`: one entry per layer whose color is
/// `Color::Index(aci)` with `1 <= aci < 256`, each bucket sorted
/// case-insensitively. Extracted from `view_window` so the table can be
/// unit-tested and benched without building widgets. `pub` (not
/// `pub(crate)`) so the `cargo bench` harness (`benches/`, an external
/// crate) can measure it as `ui_plotstyle_layer_usage`.
pub fn build_layer_usage(document: &codec::CadDocument) -> Vec<Vec<String>> {
    let mut layer_usage = vec![Vec::<String>::new(); 256];

    for layer in document.layers.iter() {
        if let codec::types::Color::Index(aci) = &layer.color {
            let index = *aci as usize;

            if index > 0 && index < layer_usage.len() {
                layer_usage[index].push(layer.name.clone());
            }
        }
    }

    for layers in &mut layer_usage {
        if layers.len() > 1 {
            layers.sort_by_cached_key(|name| name.to_ascii_lowercase());
        }
    }
    layer_usage
}

pub fn view_window<'a>(
    document: &'a codec::CadDocument,
    table: Option<&'a crate::io::plot_style::PlotStyleTable>,
    selected_aci: u8,
    color_buf: &'a str,
    lw_buf: &'a str,
    screen_buf: &'a str,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let show_layer_usage = !table.is_some_and(|table| table.is_stb);

    let layer_usage = if show_layer_usage {
        build_layer_usage(document)
    } else {
        Vec::new()
    };
    let table_name = table
        .map(|t| t.name.clone())
        .unwrap_or_else(|| t!("(no table loaded)").into_owned());

    // ── Toolbar ───────────────────────────────────────────────────────────
    // The five plates in this bar are one row of buttons, so they wear one
    // inset. `Save` typed `[4, 14]` while its four siblings had already named
    // `kit::Size::Control`; landing it on Control gives the emphasised button 4
    // px of lateral air — the same narrowing #42's `style_manager::tb_button`
    // recorded for the five named-style managers, whose accent branch is written
    // at 3 sites and drawn on 7 windows. The plate stays Primary, which is what
    // says "this one", not the width.
    let toolbar = container(
        row![
            kit::btn(
                text(t!("Load CTB/STB")).size(tokens::TEXT_LABEL),
                kit::Role::Standard,
                kit::Size::Control,
                kit::Press::On(Message::PlotStyleLoad),
                "",
            ),
            kit::btn(
                text(t!("Save")).size(tokens::TEXT_LABEL),
                kit::Role::Primary,
                kit::Size::Control,
                kit::Press::On(Message::PlotStylePanelSaveDirect),
                "",
            ),
            kit::btn(
                text(t!("Save As…")).size(tokens::TEXT_LABEL),
                kit::Role::Standard,
                kit::Size::Control,
                kit::Press::On(Message::PlotStylePanelSave),
                "",
            ),
            kit::btn(
                text(t!("Clear Table")).size(tokens::TEXT_LABEL),
                kit::Role::Standard,
                kit::Size::Control,
                kit::Press::On(Message::PlotStyleClear),
                "",
            ),
            Space::new().width(sizing.width),
            text(table_name).size(tokens::TEXT_MICRO).style(muted_style),
        ]
        .spacing(tokens::GAP_TIGHT)
        .align_y(iced::Center),
    )
    .style(kit::chrome_plate_style)
    .width(sizing.width)
    .padding(kit::PAD_BAND);

    // ── Left: ACI list ────────────────────────────────────────────────────
    let aci_items: Vec<Element<'_, Message>> = (1u8..=255)
        .map(|aci| {
            let is_sel = aci == selected_aci;
            let usage_count = layer_usage.get(aci as usize).map(Vec::len).unwrap_or(0);

            let usage_label = if show_layer_usage && usage_count > 0 {
                format!("● {usage_count}")
            } else {
                String::new()
            };
            let (aci_color, _) =
                crate::ui::properties::acad_color_display(codec::types::Color::Index(aci));
            let has_override = table
                .and_then(|t| t.aci_entries.get(aci as usize))
                .map(|e| e.color.is_some() || e.lineweight != 255 || e.screening != 100)
                .unwrap_or(false);
            let lw_str = table
                .and_then(|t| t.aci_entries.get(aci as usize))
                .and_then(|e| {
                    if e.lineweight != 255 {
                        crate::io::plot_style::LW_TABLE
                            .get(e.lineweight as usize)
                            .map(|lw| format!("{lw:.2}mm"))
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
            let color_str = table
                .and_then(|t| t.aci_entries.get(aci as usize))
                .and_then(|e| e.color.map(|[r, g, b]| format!("#{r:02X}{g:02X}{b:02X}")))
                .unwrap_or_default();
            let label = if has_override {
                format!("{aci:>3}  {color_str:<9} {lw_str}")
            } else {
                format!("{aci:>3}  {}", t!("(default)"))
            };
            // A selectable line in a data list. `kit::list_row` is the L1 answer
            // for it and its doc names this exact row: the screen typed its own
            // pair of wash strengths (0.10 selected / 0.05 hover) where the kit
            // walks the pinned ladder (TINT_SELECTED .12 / TINT_ENGAGED .16 under
            // the pointer, WASH_POINTER .06 when idle). Two decisions the row was
            // not entitled to make change with it: the selected line's ink goes
            // from `primary.weak.text` to the kit's `base.text` on the wash
            // plate (#73's ink-on-wash ruling), and an engaged row now lifts
            // further under the pointer. Inset joins the kit's `Size::Compact`,
            // so each of the 255 rows grows 4 px of height (2 px top, 2 px
            // bottom) — priced once here, drawn 255 times. Width stays the
            // screen's because `sized_flow` builds this tree twice, once to
            // measure and once to paint, and only the caller knows which pass it
            // is in.
            kit::list_row(
                row![
                    crate::ui::color_select::swatch(aci_color),
                    text(label)
                        .size(tokens::TEXT_MICRO)
                        .font(iced::Font::MONOSPACE)
                        .width(iced::Length::Fill),
                    text(usage_label)
                        .size(tokens::TEXT_MICRO)
                        .font(iced::Font::MONOSPACE)
                        .width(42),
                ]
                .spacing(tokens::GAP_CONTROL)
                .align_y(iced::Center),
                is_sel,
                kit::Press::On(Message::PlotStylePanelSelectAci(aci)),
                "",
            )
            .width(sizing.width)
            .into()
        })
        .collect();

    let aci_list = container(
        column![
            row![
                text(t!("ACI Color Index"))
                    .size(tokens::TEXT_MICRO)
                    .style(muted_style)
                    .width(iced::Length::Fill),
                text(t!("Layers")).size(tokens::TEXT_MICRO).style(muted_style).width(42),
            ]
            .align_y(iced::Center),
            // `.spacing(1)` is the seam between two rows, not hairline air: the
            // rung is GAP_SEAM (1) and GAP_HAIRLINE (2) is a different, one-pixel
            // wider answer, so folding them together is a pixel move, not a
            // rename. The plate below is the bounded-pane triple
            // (weak control fill + theme edge + field corner) that L1 already
            // owns; `pane_plate_style`'s census named this file as one of the
            // eight screens hand-typing it, and `tokens::control_plate(Weak)`
            // equals `palette.background.weak` by definition (decision A, pinned
            // over 26 themes x 7 rungs), so the fold paints the same.
            container(crate::ui::kit::themed_scrollable(
                column(aci_items).spacing(tokens::GAP_SEAM),
            )
            .height(sizing.height))
            .style(kit::pane_plate_style)
            .width(sizing.width)
            .height(sizing.height)
            .padding(tokens::GAP_HAIRLINE),
        ]
        .spacing(tokens::GAP_TIGHT)
        .height(sizing.height),
    )
    .width(280)
    .height(sizing.height)
    .padding(iced::Padding {
        top: tokens::GAP_SECTION,
        right: tokens::GAP_ROW,
        bottom: tokens::GAP_SECTION,
        left: tokens::GAP_SECTION,
    });

    // ── Right: Edit panel ─────────────────────────────────────────────────
    let selected_layers = layer_usage
        .get(selected_aci as usize)
        .cloned()
        .unwrap_or_default();

    let selected_layers_text = if selected_layers.is_empty() {
        t!("(none)").into_owned()
    } else {
        selected_layers.join(", ")
    };
    let entry = table.and_then(|t| t.aci_entries.get(selected_aci as usize));
    let cur_color = entry
        .and_then(|e| e.color.map(|[r, g, b]| format!("#{r:02X}{g:02X}{b:02X}")))
        .unwrap_or_else(|| t!("(none)").into());
    let cur_lw = entry
        .map(|e| {
            if e.lineweight == 255 {
                t!("object").into()
            } else {
                crate::io::plot_style::LW_TABLE
                    .get(e.lineweight as usize)
                    .map(|lw| crate::tf!("{lw:.2}mm (idx {})", e.lineweight).into_owned())
                    .unwrap_or_else(|| crate::tf!("idx {}", e.lineweight).into_owned())
            }
        })
        .unwrap_or_else(|| "—".into());
    let cur_scr = entry
        .map(|e| format!("{}%", e.screening))
        .unwrap_or_else(|| "—".into());

    let lbl = |s: Cow<'static, str>| text(s).size(tokens::TEXT_LABEL).style(muted_style);
    let lw_items = plot_lineweight_options();

    let current_lw_index = lw_buf.trim().parse::<u8>().unwrap_or(255);

    let current_lw = lw_items
        .iter()
        .find(|item| item.index == current_lw_index)
        .cloned()
        .unwrap_or_else(|| lw_items[0].clone());

    let edit_panel = container(
        column![
            row![
                text(t!("ACI:")).size(tokens::TEXT_LABEL).style(muted_style).width(100),
                text(format!("{selected_aci}")).size(tokens::TEXT_LABEL),
            ]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center),
            // Two recesses answer the same question ("which layers wear this
            // ACI") at two altitudes, so they take one inset and one internal
            // gap rather than each re-deriving a cheaper pair — the planner's
            // floor could name `[5,8]`->`[4,8]` here and `[8,4]` unchanged there
            // because it prices a line, not a role. The gap is GAP_TIGHT because
            // a caption and the run it captions are the tightest pair in a
            // column whose siblings are 8 apart.
            if show_layer_usage {
                container(
                    column![
                        text(format!("{}: {}", t!("Layers"), selected_layers.len()))
                            .size(tokens::TEXT_MICRO)
                            .style(muted_style),
                        text(selected_layers_text)
                            .size(tokens::TEXT_MICRO)
                            .width(iced::Length::Fill),
                    ]
                    .spacing(tokens::GAP_TIGHT),
                )
                .padding(kit::PAD_WELL)
                .width(iced::Length::Fill)
            } else {
                container(Space::new().height(tokens::GAP_NONE))
            },
            // ── Plot color ───────────────────────────────────────────────────
            lbl(t!("Plot color:")),
            {
                let selected_color = color_buf
                    .strip_prefix('#')
                    .filter(|hex| hex.len() == 6)
                    .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                    .map(|rgb| codec::types::Color::Rgb {
                        r: ((rgb >> 16) & 0xFF) as u8,
                        g: ((rgb >> 8) & 0xFF) as u8,
                        b: (rgb & 0xFF) as u8,
                    });

                let display_color =
                    selected_color.unwrap_or(codec::types::Color::Index(selected_aci));

                let (swatch_color, _) = crate::ui::properties::acad_color_display(display_color);

                let display_text = if color_buf.is_empty() {
                    t!("Use object color").into_owned()
                } else {
                    crate::ui::color_select::color_display_name(display_color)
                };

                row![
                    container(
                        row![
                            crate::ui::color_select::swatch(swatch_color),
                            text(display_text).size(tokens::TEXT_LABEL),
                        ]
                        .spacing(tokens::GAP_CONTROL)
                        .align_y(iced::Center)
                    )
                    .padding(kit::Size::Compact.padding())
                    .width(iced::Length::Fill),
                    kit::btn(
                        text(t!("Choose color…")).size(tokens::TEXT_LABEL),
                        kit::Role::Standard,
                        kit::Size::Control,
                        kit::Press::On(Message::OpenColorWindow(
                            crate::app::ColorPickTarget::PlotStyle,
                            display_color,
                        )),
                        "",
                    ),
                    kit::btn(
                        text(t!("Reset")).size(tokens::TEXT_LABEL),
                        kit::Role::Standard,
                        kit::Size::Control,
                        kit::Press::On(Message::PlotStylePanelColorBuf(String::new())),
                        "",
                    ),
                ]
                .spacing(tokens::GAP_CONTROL)
                .align_y(iced::Center)
            },
            // ── Lineweight ───────────────────────────────────────────────────
            lbl(t!("Lineweight:")),
            // A lineweight selector is a field, and the field inset is
            // `PAD_FIELD` `[3, 6]` — which is *cheaper* than the planner's floor
            // here (it could only see the GAP_* ladder and proposed `[2, 4]`, 2
            // px). Finding F1, again: a named inset can land below the rung
            // ladder's "nearest" because the ladder is not the only vocabulary.
            crate::ui::kit::themed_pick_list(Some(current_lw), lw_items, |item| item.to_string(),)
                .on_select(|item: PlotLineweightItem| { Message::PlotStylePanelLwSet(item.index) })
                .text_size(tokens::TEXT_LABEL)
                .padding(kit::PAD_FIELD)
                .width(iced::Length::Fill),
            // ── Screening ────────────────────────────────────────────────────
            lbl(t!("Screening (0-100):")),
            text_input("100", screen_buf)
                .on_input(Message::PlotStylePanelScreenBuf)
                .style(crate::ui::kit::field_style)
                .size(tokens::TEXT_LABEL)
                .padding(kit::Size::Compact.padding()),
            Space::new().height(tokens::GAP_ROW),
            // ── Current values ───────────────────────────────────────────────
            text(t!("Current values:")).size(tokens::TEXT_MICRO).style(muted_style),
            text(t!("  Color: %{cur_color}", cur_color = cur_color)).size(tokens::TEXT_MICRO),
            text(t!("  Lineweight: %{cur_lw}", cur_lw = cur_lw)).size(tokens::TEXT_MICRO),
            text(t!("  Screening: %{cur_scr}", cur_scr = cur_scr)).size(tokens::TEXT_MICRO),
            Space::new().height(tokens::GAP_SECTION),
            kit::hdivider(iced::Length::Fill),
            Space::new().height(tokens::GAP_ROW),
            if show_layer_usage {
                container(
                    column![
                        row![
                            text(crate::tf!("Layers using ACI {selected_aci}")).size(tokens::TEXT_LABEL),
                            Space::new().width(iced::Length::Fill),
                            text(format!(
                                "{} {}",
                                selected_layers.len(),
                                if selected_layers.len() == 1 {
                                    t!("layer")
                                } else {
                                    t!("layers")
                                }
                            ))
                            .size(tokens::TEXT_MICRO)
                            .style(muted_style),
                        ]
                        .align_y(iced::Center),
                        if selected_layers.is_empty() {
                            column![text(t!("No layers use this ACI in the current drawing."))
                                .size(tokens::TEXT_MICRO)
                                .style(muted_style)]
                        } else {
                            column(
                                selected_layers
                                    .iter()
                                    .map(|name| text(format!("• {name}")).size(tokens::TEXT_MICRO).into())
                                    .collect::<Vec<Element<'_, Message>>>(),
                            )
                            .spacing(tokens::GAP_TIGHT)
                        },
                    ]
                    .spacing(tokens::GAP_TIGHT),
                )
                .padding(kit::PAD_WELL)
                .width(iced::Length::Fill)
            } else {
                container(Space::new().height(tokens::GAP_NONE))
            },
            Space::new().height(sizing.height),
        ]
        .spacing(tokens::GAP_ROW)
        .height(sizing.height),
    )
    .width(sizing.width)
    .height(sizing.height)
    // `[12, 12]` is a scalar in disguise, and the scalar has a name: the section
    // rung, 0 px.
    .padding(tokens::GAP_SECTION);

    let body = row![aci_list, kit::vsep(sizing.height), edit_panel].height(sizing.height);

    container(column![toolbar, kit::hdivider(sizing.width), body].spacing(tokens::GAP_NONE))
        .width(sizing.width)
        .height(sizing.height)
        .into()
}

#[cfg(test)]
mod layer_usage_tests {
    use super::*;

    fn indexed_layer(doc: &mut codec::CadDocument, name: &str, aci: u8) -> codec::tables::Layer {
        let mut layer = codec::tables::Layer::new(name);
        layer.handle = doc.allocate_handle();
        layer.color = codec::types::Color::Index(aci);
        layer
    }

    #[test]
    fn build_layer_usage_buckets_index_colors() {
        let mut doc = codec::CadDocument::new();
        let a = indexed_layer(&mut doc, "WALLS", 1);
        let b = indexed_layer(&mut doc, "doors", 1);
        let c = indexed_layer(&mut doc, "ROOF", 3);
        let _ = doc.layers.add(a);
        let _ = doc.layers.add(b);
        let _ = doc.layers.add(c);

        let usage = build_layer_usage(&doc);

        assert_eq!(usage.len(), 256);
        assert_eq!(usage[1], vec!["doors".to_string(), "WALLS".to_string()]);
        assert_eq!(usage[3], vec!["ROOF".to_string()]);
        assert!(usage[2].is_empty());
    }

    #[test]
    fn build_layer_usage_ignores_non_index_colors() {
        let mut doc = codec::CadDocument::new();
        let mut truecolor = codec::tables::Layer::new("TRUECOLOR");
        truecolor.handle = doc.allocate_handle();
        truecolor.color = codec::types::Color::Rgb {
            r: 10,
            g: 20,
            b: 30,
        };
        let _ = doc.layers.add(truecolor);

        let usage = build_layer_usage(&doc);

        assert!(
            usage
                .iter()
                .all(|bucket| !bucket.iter().any(|n| n == "TRUECOLOR")),
            "truecolor layer must not appear in any ACI bucket"
        );
    }
}
