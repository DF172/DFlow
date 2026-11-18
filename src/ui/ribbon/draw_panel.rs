//! Additional drawing tools, reached from the Draw panel's title.

use iced::widget::{column, container, row, text, tooltip};
use crate::ui::kit::plain_button;
use iced::{Element, Fill, Theme};

use super::widgets::{
    hover_tracked, make_icon, option_mark, option_mark_lift, ribbon_tip, tool_btn_style,
    LARGE_DROPDOWN_ARROW_SIZE,
};
use super::{dropdown_backdrop, position_ribbon_dropdown, Ribbon};
use crate::app::Message;
use crate::modules::IconKind;
use crate::t;
use crate::ui::kit;
use crate::ui::{icons, wrap_bar::PosReport};
use crate::ui::style::tokens;

pub(super) const PANEL_ID: &str = "draw_extension";
const TITLE_ID: &str = "draw_extension_title";
const SCALE: f32 = 0.7;
/// A slide-out tool is a full cell — icon over name, the same reading order as
/// a strip tool. A bare glyph asked the reader to hover every square to learn
/// what it was.
const CELL_W: f32 = 88.0;
const CELL_H: f32 = 64.0;
const CELL_GAP: f32 = 4.0;
const MAX_COLS: usize = 4;
const OPTION_HEIGHT: f32 = 30.0 * SCALE;
const GROUP_TITLE_ARROW_SIZE: f32 = LARGE_DROPDOWN_ARROW_SIZE * 1.5;

pub(super) struct Tool {
    pub command: &'static str,
    pub label: &'static str,
    pub icon: &'static [u8],
    pub options: &'static [(&'static str, &'static str)],
}

const TOOLS: &[Tool] = &[
    Tool {
        command: "SPLINE",
        label: "Spline Fit",
        icon: include_bytes!("../../../assets/icons/spline.svg"),
        options: &[],
    },
    Tool {
        command: "SPLINECV",
        label: "Spline CV",
        icon: include_bytes!("../../../assets/icons/spline_cv.svg"),
        options: &[],
    },
    Tool {
        command: "XLINE",
        label: "Construction Line",
        icon: include_bytes!("../../../assets/icons/xline.svg"),
        options: &[],
    },
    Tool {
        command: "RAY",
        label: "Ray",
        icon: include_bytes!("../../../assets/icons/ray.svg"),
        options: &[],
    },
    Tool {
        command: "DIVIDE",
        label: "Divide",
        icon: include_bytes!("../../../assets/icons/divide.svg"),
        options: &[],
    },
    Tool {
        command: "MEASURE",
        label: "Measure",
        icon: include_bytes!("../../../assets/icons/measure.svg"),
        options: &[],
    },
    Tool {
        command: "REGION",
        label: "Region",
        icon: include_bytes!("../../../assets/icons/region.svg"),
        options: &[],
    },
    Tool {
        command: "BOUNDARY",
        label: "Boundary",
        icon: include_bytes!("../../../assets/icons/boundary.svg"),
        options: &[],
    },
    Tool {
        command: "HELIX",
        label: "Helix",
        icon: include_bytes!("../../../assets/icons/helix.svg"),
        options: &[],
    },
    Tool {
        command: "DONUT",
        label: "Donut",
        icon: include_bytes!("../../../assets/icons/donut.svg"),
        options: &[],
    },
    Tool {
        command: "MULTIPOINT",
        label: "Multiple Points",
        icon: include_bytes!("../../../assets/icons/multipoint.svg"),
        options: &[
            ("POINT", "Single Point"),
            ("MULTIPOINT", "Multiple Points"),
            ("DDPTYPE", "Point Style"),
        ],
    },
];

#[derive(Clone, Copy)]
struct Panel {
    id: &'static str,
    /// A command the owning ribbon group must hold, for titles more than one
    /// tab uses ("Block" on Home and Insert).
    anchor: Option<&'static str>,
    title_id: &'static str,
    title: &'static str,
    tools: &'static [Tool],
}

/// The Reference panel's slide-out: Edit Reference, then the xref fading
/// switch and amount (drawn by `reference_overlay`).
const REFERENCE_TOOLS: &[Tool] = &[Tool {
    command: "REFEDIT",
    label: "Edit Reference",
    icon: include_bytes!("../../../assets/icons/edit_block.svg"),
    options: &[],
}];
pub(super) const REFERENCE_PANEL_ID: &str = "reference_extension";

const fn icon_tool(command: &'static str, label: &'static str, icon: &'static [u8]) -> Tool {
    Tool {
        command,
        label,
        icon,
        options: &[],
    }
}

/// Home > Block slide-out.
const HOME_BLOCK_TOOLS: &[Tool] = &[
    icon_tool(
        "ATTDEF",
        "Define Attributes",
        include_bytes!("../../../assets/icons/attdef.svg"),
    ),
    icon_tool(
        "ATTMAN",
        "Manage Attributes",
        include_bytes!("../../../assets/icons/attman.svg"),
    ),
    icon_tool(
        "ATTSYNC",
        "Synchronize",
        include_bytes!("../../../assets/icons/attsync.svg"),
    ),
    icon_tool(
        "BASE",
        "Set Base Point",
        include_bytes!("../../../assets/icons/base_point.svg"),
    ),
];

/// Insert > Block Definition slide-out.
const BLOCK_DEFINITION_TOOLS: &[Tool] = &[
    icon_tool(
        "BASE",
        "Set Base Point",
        include_bytes!("../../../assets/icons/base_point.svg"),
    ),
    icon_tool(
        "ATTSYNC",
        "Synchronize",
        include_bytes!("../../../assets/icons/attsync.svg"),
    ),
];

const PANELS: &[Panel] = &[
    Panel {
        id: PANEL_ID,
        anchor: None,
        title_id: TITLE_ID,
        title: "Draw",
        tools: TOOLS,
    },
    Panel {
        id: "modify_extension",
        anchor: None,
        title_id: "modify_extension_title",
        title: "Modify",
        tools: super::modify_panel::TOOLS,
    },
    Panel {
        id: REFERENCE_PANEL_ID,
        anchor: None,
        title_id: "reference_extension_title",
        title: "Reference",
        tools: REFERENCE_TOOLS,
    },
    Panel {
        id: "home_block_extension",
        anchor: Some("BEDIT"),
        title_id: "home_block_extension_title",
        title: "Block",
        tools: HOME_BLOCK_TOOLS,
    },
    Panel {
        id: "block_definition_extension",
        anchor: None,
        title_id: "block_definition_extension_title",
        title: "Block Definition",
        tools: BLOCK_DEFINITION_TOOLS,
    },
];

/// Command, label and icon of every tool in the expanded Draw and Modify
/// panels.
pub(crate) fn tools() -> impl Iterator<Item = (&'static str, &'static str, &'static [u8])> {
    PANELS
        .iter()
        .flat_map(|panel| panel.tools)
        .map(|tool| (tool.command, tool.label, tool.icon))
}

fn panel_for_dropdown(id: &str) -> Option<Panel> {
    PANELS.iter().copied().find(|panel| {
        panel.id == id
            || panel
                .tools
                .iter()
                .any(|tool| tool.command == id && !tool.options.is_empty())
    })
}

pub(super) fn owns_dropdown(id: &str) -> bool {
    panel_for_dropdown(id).is_some()
}

/// The slide-out panel id of ribbon group `title`, when it has one. The sheet's
/// one-row tool strip keeps its groups short and offers the rest through a
/// small ▾ at the group's edge; this is how that ▾ finds its panel.
pub(super) fn group_panel_id(title: &str, group_ids: &[&'static str]) -> Option<&'static str> {
    panel_for_group(title, group_ids).map(|panel| panel.id)
}

pub(super) fn parent_panel(id: &str) -> Option<&'static str> {
    panel_for_dropdown(id).map(|panel| panel.id)
}

/// The slide-out under the title of the ribbon group `title`, whose items
/// are `group_ids`.
fn panel_for_group(title: &str, group_ids: &[&'static str]) -> Option<Panel> {
    PANELS.iter().copied().find(|panel| {
        panel.title == title
            && !panel.tools.is_empty()
            && panel
                .anchor
                .is_none_or(|anchor| group_ids.contains(&anchor))
    })
}

/// Every command the slide-out of the group `title` offers: each tool and
/// each of its submenu options.
pub(super) fn slide_out_commands(
    title: &str,
    group_ids: &[&'static str],
) -> impl Iterator<Item = &'static str> {
    panel_for_group(title, group_ids)
        .into_iter()
        .flat_map(|panel| panel.tools)
        .flat_map(|tool| {
            std::iter::once(tool.command).chain(tool.options.iter().map(|(cmd, _)| *cmd))
        })
}

/// A slide-out tool is lit while it or one of its submenu options runs.
fn tool_is_active(tool: &Tool, active_tool: Option<&str>) -> bool {
    active_tool.is_some_and(|active| {
        tool.command == active || tool.options.iter().any(|(cmd, _)| *cmd == active)
    })
}

pub(super) fn group_title<'a>(
    title: &'static str,
    group_ids: &[&'static str],
    open: &Option<String>,
    active_tool: Option<&str>,
    theme: &Theme,
) -> Element<'a, Message> {
    let Some(panel) = panel_for_group(title, group_ids) else {
        return container(
            text(t!(title))
                .size(tokens::TEXT_LABEL)
                .style(kit::faint_style),
        )
        .padding([tokens::GAP_SEAM, tokens::GAP_TIGHT])
        .into();
    };
    let expanded = open.as_deref().and_then(parent_panel) == Some(panel.id);
    // A command running from the closed slide-out lights its title, the way a
    // dropdown lights while one of its items runs.
    let lit = expanded
        || panel
            .tools
            .iter()
            .any(|tool| tool_is_active(tool, active_tool));
    let arrow = if expanded {
        icons::themed_arrow_up(GROUP_TITLE_ARROW_SIZE)
    } else {
        icons::themed_arrow_down(GROUP_TITLE_ARROW_SIZE)
    };
    let hover_key = crate::ui::motion::hover_key(panel.id);
    let glow_key = crate::ui::motion::glow_key(panel.id);
    // One Pen carries no mark beside a group label: the label alone names the
    // panel, and the arrow alone says it opens.
    let title_button: Element<'a, Message> = PosReport::new(
        panel.id,
        hover_tracked(
            hover_key,
            plain_button(
                row![
                    PosReport::new(
                        panel.title_id,
                        text(t!(title))
                            .size(tokens::TEXT_LABEL)
                            .style(kit::faint_style)
                    ),
                    arrow
                ]
                .spacing(tokens::GAP_TIGHT)
                .align_y(iced::Center),
            )
            .on_press(Message::ToggleRibbonDropdown(panel.id.to_string()))
            .style(move |theme: &Theme, status| {
                tool_btn_style(
                    theme,
                    lit,
                    status,
                    crate::ui::motion::value(glow_key),
                    crate::ui::motion::value(hover_key),
                )
            })
            .padding([tokens::GAP_SEAM, tokens::GAP_TIGHT]),
        ),
    )
    .into();
    if panel.id != REFERENCE_PANEL_ID {
        return title_button;
    }
    // The corner arrow opens the External References palette.
    let launcher_hover = crate::ui::motion::hover_key("EXTERNALREFERENCES");
    let launcher_glow = crate::ui::motion::glow_key("EXTERNALREFERENCES");
    let launcher = ribbon_tip(
        // A right arrow turned to point into the corner.
        hover_tracked(
            launcher_hover,
            plain_button(
                iced::widget::svg(icons::themed_handle(icons::ARROW_LONG_RIGHT, 10.0))
                    .width(10)
                    .height(10)
                    .rotation(iced::Radians(std::f32::consts::FRAC_PI_4))
                    .style(|theme: &Theme, _| iced::widget::svg::Style {
                        color: Some(theme.palette().background.base.text),
                    }),
            )
            .on_press(Message::RibbonToolClick {
                tool_id: "EXTERNALREFERENCES".to_string(),
                event: crate::modules::ModuleEvent::Command("EXTERNALREFERENCES".to_string()),
            })
            .style(move |theme: &Theme, status| {
                tool_btn_style(
                    theme,
                    false,
                    status,
                    crate::ui::motion::value(launcher_glow),
                    crate::ui::motion::value(launcher_hover),
                )
            })
            .padding([tokens::GAP_NONE, tokens::GAP_TIGHT]),
        ),
        crate::tf!(
            "{}\n{} EXTERNALREFERENCES",
            t!("External References"),
            t!("Command:")
        ).into_owned(),
        tooltip::Position::Bottom,
        tokens::GAP_NONE,
        theme,
    );
    // Shrink-sized: a Fill here would make the panel claim the whole row.
    row![title_button, launcher]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center)
        .into()
}

fn tool_button(
    tool: &Tool,
    active: bool,
    panel_id: &'static str,
    theme: &Theme,
) -> Element<'static, Message> {
    let hover_key = crate::ui::motion::hover_key(tool.command);
    let glow_key = crate::ui::motion::glow_key(tool.command);
    let has_options = !tool.options.is_empty();
    let body = column![
        container(make_icon(IconKind::Svg(tool.icon), 20.0))
            .width(Fill)
            .height(iced::Length::Fixed(20.0))
            .align_x(iced::Center)
            .align_y(iced::Center),
        text(t!(tool.label).into_owned())
            .size(tokens::TEXT_LABEL)
            .width(Fill)
            .align_x(iced::Center)
            .wrapping(iced::advanced::text::Wrapping::Word),
    ]
    .spacing(tokens::GAP_HAIRLINE)
    .align_x(iced::Center)
    .width(Fill);
    let face = plain_button(body)
        .on_press(Message::DropdownSelectItem {
            dropdown_id: panel_id,
            cmd: tool.command,
        })
        .style(move |theme: &Theme, status| {
            tool_btn_style(
                theme,
                active,
                status,
                crate::ui::motion::value(glow_key),
                crate::ui::motion::value(hover_key),
            )
        })
        .width(CELL_W)
        .height(CELL_H)
        .padding(iced::Padding {
            top: tokens::GAP_TIGHT,
            right: tokens::GAP_TIGHT,
            bottom: tokens::GAP_TIGHT,
            left: tokens::GAP_TIGHT,
        });
    let tip = format!("{}\n{} {}", t!(tool.label), t!("Command:"), tool.command);
    let face: Element<'static, Message> = ribbon_tip(
        face,
        tip,
        tooltip::Position::Bottom,
        tokens::GAP_NONE,
        theme,
    );
    if !has_options {
        return hover_tracked(hover_key, face);
    }
    // The same bottom mark the strip uses — one wording for "there are more
    // choices behind this" everywhere in the ribbon, lifted to trail the name
    // instead of hanging on the taller cell's bottom edge.
    let mark = option_mark(
        tool.command,
        format!("{} {}", t!(tool.label), t!("options")),
        hover_key,
        false,
        option_mark_lift(CELL_H, tokens::GAP_TIGHT, 20.0, tokens::TEXT_LABEL),
        theme,
    );
    // One hover area for the cell and its mark, so the pointer lights the whole
    // tile — including over the caret.
    hover_tracked(hover_key, iced::widget::stack![face, mark])
}

/// The Reference slide-out: Edit Reference, and xref fading on/off with its
/// amount (XDWGFADECTL; the switch keeps the amount as a negative value).
fn reference_overlay<'a>(
    ribbon: &Ribbon,
    panel: Panel,
    win: (f32, f32),
    theme: &Theme,
) -> Element<'a, Message> {
    let width: f32 = 260.0_f32.min((win.0 - 8.0).max(120.0));
    let (x, anchor_top) = ribbon.dd_anchor(panel.id, width, win.0);
    let left = x.clamp(
        0.0,
        (win.0 - (width + 2.0 * tokens::GAP_CONTROL)).max(0.0),
    );
    let top = anchor_top.min((win.1 - 80.0).max(0.0));
    let tool = &panel.tools[0];
    let edit = plain_button(
        row![
            make_icon(IconKind::Svg(tool.icon), 16.0),
            text(t!(tool.label)).size(tokens::TEXT_LABEL)
        ]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center),
    )
    .on_press(Message::DropdownSelectItem {
        dropdown_id: panel.id,
        cmd: tool.command,
    })
    .style(kit::ghost_style)
    .width(Fill)
    .padding(kit::PAD_FIELD);
    let fade = ribbon.xref_fade;
    let on = fade > 0;
    let xf_hover = crate::ui::motion::hover_key("XREFFADE");
    let xf_glow = crate::ui::motion::glow_key("XREFFADE");
    let switch = hover_tracked(
        xf_hover,
        plain_button(make_icon(
            IconKind::Svg(include_bytes!("../../../assets/icons/underlay_frames.svg")),
            16.0,
        ))
        .on_press(Message::XrefFadeToggle)
        .style(move |theme: &Theme, status| {
            tool_btn_style(
                theme,
                on,
                status,
                crate::ui::motion::value(xf_glow),
                crate::ui::motion::value(xf_hover),
            )
        })
        .padding(tokens::GAP_HAIRLINE),
    );
    let amount = fade.unsigned_abs().min(90) as u8;
    let fading = row![
        ribbon_tip(
            switch,
            crate::tf!(
                "{}\n{} XDWGFADECTL",
                t!("Xref fading"),
                t!("Command:")
            ).into_owned(),
            tooltip::Position::Bottom,
            tokens::GAP_NONE,
            theme,
        ),
        text(t!("Xref fading")).size(tokens::TEXT_LABEL),
        iced::widget::slider(0..=90u8, amount, Message::XrefFadeSlide)
            .on_release(Message::XrefFadeCommit)
            .width(Fill),
        text(amount.to_string()).size(tokens::TEXT_LABEL).width(22),
    ]
    .spacing(tokens::GAP_CONTROL)
    .align_y(iced::Center);
    // The host keeps its inset and width; `popup_plate` paints the paper — the
    // `Float` rung with the field's edge, so the menu carries a contour over
    // the drawing instead of dissolving into it.
    let body = kit::popup_plate(
        container(column![edit, fading].spacing(tokens::GAP_TIGHT).padding(tokens::GAP_CONTROL))
            .width(width)
            .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
            .into(),
        "",
    );
    dropdown_backdrop(position_ribbon_dropdown(body, left, top))
}

pub(super) fn overlay<'a>(
    ribbon: &Ribbon,
    id: &str,
    win: (f32, f32),
    theme: &Theme,
) -> Element<'a, Message> {
    let panel = panel_for_dropdown(id).expect("registered ribbon extension");
    if panel.id == REFERENCE_PANEL_ID {
        return reference_overlay(ribbon, panel, win, theme);
    }
    let tools = panel.tools;
    let width = (MAX_COLS as f32 * (CELL_W + CELL_GAP) - CELL_GAP + 12.0)
        .min((win.0 - 8.0).max(CELL_W + 12.0));
    let (x, anchor_top) = ribbon.dd_anchor(panel.id, width, win.0);
    let outer_w = width + 2.0 * tokens::GAP_CONTROL;
    let anchor = crate::ui::wrap_bar::dropdown_bounds(panel.title_id);
    let left = anchor
        .map_or(x, |b| b.x + (b.width - outer_w) / 2.0)
        .clamp(0.0, (win.0 - outer_w).max(0.0));
    let top = anchor_top.min((win.1 - 80.0).max(0.0));
    let available_height = (win.1 - top - 4.0).max(1.0);
    let cols = (((width - 12.0 + CELL_GAP) / (CELL_W + CELL_GAP)).floor() as usize)
        .clamp(1, MAX_COLS);
    let option_tool = tools
        .iter()
        .find(|tool| tool.command == id && !tool.options.is_empty());
    let content_height = option_tool.map_or_else(
        || tools.len().div_ceil(cols) as f32 * (CELL_H + CELL_GAP) - CELL_GAP + 12.0,
        |tool| tool.options.len() as f32 * OPTION_HEIGHT,
    );
    let contents: Element<'static, Message> = if let Some(tool) = option_tool {
        column(
            tool.options
                .iter()
                .map(|&(cmd, label)| {
                    plain_button(text(t!(label)).size(tokens::TEXT_LABEL))
                        .on_press(Message::DropdownSelectItem {
                            dropdown_id: tool.command,
                            cmd,
                        })
                        .style(kit::ghost_style)
                        .width(Fill)
                        .height(OPTION_HEIGHT)
                        .padding([tokens::GAP_HAIRLINE, tokens::GAP_CONTROL])
                        .into()
                })
                .collect::<Vec<Element<'static, Message>>>(),
        )
        .into()
    } else {
        column(
            tools
                .chunks(cols)
                .map(|tools| {
                    row(tools
                        .iter()
                        .map(|tool| {
                            tool_button(
                                tool,
                                tool_is_active(tool, ribbon.active_tool.as_deref()),
                                panel.id,
                                theme,
                            )
                        })
                        .collect::<Vec<_>>())
                    .spacing(CELL_GAP)
                    .into()
                })
                .collect::<Vec<Element<'static, Message>>>(),
        )
        .spacing(CELL_GAP)
        .padding(tokens::GAP_CONTROL)
        .into()
    };
    // Same paper as the slide-out above: the host keeps the inset and width,
    // `popup_plate` paints the plate and its contour.
    let panel = kit::popup_plate(
        container(
            kit::themed_scrollable(contents).height(content_height.min(available_height)),
        )
        .width(width)
        .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
        .into(),
        "",
    );
    dropdown_backdrop(position_ribbon_dropdown(panel, left, top))
}

#[cfg(test)]
mod tests {
    use super::{parent_panel, PANEL_ID, TOOLS};

    #[test]
    fn extension_submenus_resolve_to_their_parent_panel() {
        assert_eq!(parent_panel("MULTIPOINT"), Some(PANEL_ID));
        assert_eq!(parent_panel("DRAWORDER_FRONT"), Some("modify_extension"));
        assert_eq!(parent_panel("unknown"), None);
    }

    #[test]
    fn split_tools_follow_plain_tools() {
        let first_split = TOOLS
            .iter()
            .position(|tool| !tool.options.is_empty())
            .unwrap_or(TOOLS.len());
        assert!(TOOLS[..first_split]
            .iter()
            .all(|tool| tool.options.is_empty()));
        assert!(TOOLS[first_split..]
            .iter()
            .all(|tool| !tool.options.is_empty()));
    }
}
