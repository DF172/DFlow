//! Dock drawing: the edges (icon strip and shown group), floating pallets,
//! the + menu and the drag preview.

use crate::ui::kit;
use crate::ui::style::tokens;
use crate::app::document::DocumentTab;
use crate::app::{Message, DraftFlowCAD};
use crate::t;
use crate::ui::dock::DOCK_DIVIDER_W;
use iced::widget::{canvas, column, container, mouse_area, row, stack, text, Space};
use iced::{Element, Fill, Length, Theme};

impl DraftFlowCAD {
    /// Everything the dock draws over the workspace (the row of edges and
    /// viewport): floating pallets, an open + menu, the drag preview, and
    /// the pointer capture that feeds drags.
    pub(in crate::app) fn dock_overlay<'a>(
        &'a self,
        workspace: Element<'a, Message>,
        tab: &'a DocumentTab,
    ) -> Element<'a, Message> {
        // Floating panels sit over the workspace, back to front.
        let mut layers: Vec<Element<'_, Message>> = vec![workspace];
        for f in &self.dock.floating {
            // Once a drag is under way only its preview shows the panel.
            let dragged = self
                .dock_drag
                .and_then(|d| d.movement())
                .is_some_and(|(panel, _, target)| panel == f.id && target.is_some());
            if self.dock_panel_visible(f.id) && !dragged {
                layers.push(self.floating_panel(*f, tab));
            }
        }
        // An open pallet menu hangs beside its edge's + button, over a
        // catcher that closes it on any click elsewhere.
        if let Some(side) = self.dock_edge_menu {
            if !self.dock.groups(side).is_empty() {
                // Checked = open and docked on this edge.
                let checked: Vec<_> = crate::ui::dock::PanelId::ALL
                    .iter()
                    .map(|p| {
                        let here = self.dock.location(*p).map(|(s, _)| s) == Some(side);
                        (*p, here && self.dock_panel_visible(*p))
                    })
                    .collect();
                let (_, plus_top) = self.dock_strip_layout(side);
                let strip = crate::ui::dock::DOCK_STRIP_W;
                let x = match side {
                    crate::app::config::DockSide::Left => strip + 4.0,
                    crate::app::config::DockSide::Right => {
                        self.dock_workspace_size().0 - strip - DOCK_MENU_W - 4.0
                    }
                };
                layers.push(
                    mouse_area(Space::new().width(Fill).height(Fill))
                        .on_press(Message::Dock(crate::ui::dock::DockMsg::EdgeMenu(None)))
                        .on_right_press(Message::Dock(crate::ui::dock::DockMsg::EdgeMenu(None)))
                        .into(),
                );
                layers.push(place_at(
                    dock_edge_menu(side, &checked, &self.active_theme),
                    x.max(0.0),
                    plus_top,
                ));
            }
        }
        // An open floating title-bar menu, over its own click catcher.
        if let Some(menu) = self.dock_float_menu_overlay() {
            let close = Message::Dock(crate::ui::dock::DockMsg::FloatMenu(None));
            layers.push(
                mouse_area(Space::new().width(Fill).height(Fill))
                    .on_press(close.clone())
                    .on_right_press(close)
                    .into(),
            );
            layers.push(menu);
        }
        // While a panel is dragged, preview where it lands.
        if let Some((id, _, Some(target))) = self.dock_drag.and_then(|d| d.movement()) {
            layers.push(self.dock_drop_preview(id, target));
        }
        // Always a stack (and, below, always a mouse area), even with nothing
        // on top: iced keeps a widget's state only while the tree keeps its
        // shape, and a title bar's double-click needs its first click to
        // survive the drag that pressing the bar starts.
        let workspace: Element<'_, Message> = iced::widget::Stack::with_children(layers)
            .width(Fill)
            .height(Fill)
            .into();
        use crate::ui::dock::DockDrag;
        let capturing =
            self.dock_drag.is_some() || self.xref_col_drag.is_some() || self.xref_split_drag;
        let mut capture = mouse_area(workspace);
        if capturing {
            capture = capture
                .on_move(move |p| Message::Dock(crate::ui::dock::DockMsg::DragMove(p)))
                .on_release(Message::Dock(crate::ui::dock::DockMsg::DragRelease))
                .interaction(match self.dock_drag {
                    Some(DockDrag::FloatSize {
                        from_left: true, ..
                    }) => iced::mouse::Interaction::ResizingDiagonallyUp,
                    Some(DockDrag::FloatSize {
                        from_left: false, ..
                    }) => iced::mouse::Interaction::ResizingDiagonallyDown,
                    Some(DockDrag::Split { .. }) => iced::mouse::Interaction::ResizingVertically,
                    Some(DockDrag::Move { .. }) => iced::mouse::Interaction::Grabbing,
                    // Widths, the Layer Manager column and the xref drags.
                    _ => iced::mouse::Interaction::ResizingHorizontally,
                });
        }
        capture.into()
    }

    /// Build one edge: the vertical icon strip at the very edge plus the
    /// edge's shown group beside it. The strip lists every group as a grip
    /// bar over its pallets' icons, with a + button below; clicking an icon
    /// shows its group. The shown group's pallets stack top to bottom, each
    /// at its weighted share of the height, with a draggable splitter between
    /// neighbours. A group whose pallets all auto-hide shows only while one
    /// of its icons (or the group itself) is hovered; leaving the edge hides
    /// it again. `None` when nothing on the edge is open.
    pub(in crate::app) fn build_edge_stack<'a>(
        &'a self,
        side: crate::app::config::DockSide,
        tab: &'a DocumentTab,
    ) -> Option<Element<'a, Message>> {
        // The start page and clean-screen mode keep the drawing bare: no rail
        // there, the same as when every pallet was closed.
        if tab.is_start || self.clean_screen {
            return None;
        }
        let strip = self.dock_icon_strip(side);
        // The rail is the edge's launcher and stays even when every pallet on
        // the edge is closed: hiding it would hide the way to reopen them.
        let column_el = self
            .dock_shown_group(side)
            .filter(|_| self.dock_edge_expanded(side))
            .map(|gi| {
            let width = self.dock_group_width(side, gi);
            let group = &self.dock.groups(side)[gi];
            let mut layers: Vec<Element<'_, Message>> = Vec::new();
            let mut prev: Option<usize> = None;
            for (pos, id) in group.panels.iter().enumerate() {
                if !self.dock_panel_visible(*id) {
                    continue;
                }
                if let Some(upper) = prev {
                    layers.push(dock_splitter(side, gi, upper, pos, width));
                }
                layers.push(
                    container(self.expanded_panel(*id, side, width, tab))
                        .height(Length::FillPortion(crate::ui::dock::portion(
                            group.weights[pos],
                        )))
                        .clip(true)
                        .into(),
                );
                prev = Some(pos);
            }
            // Slide-in: the column keeps its full content width and the
            // animated viewport clips it, so labels never reflow while the
            // group reveals. `value_or(.., 1.0)` keeps groups that were
            // never hidden at full width (e.g. the ones visible at boot).
            let slide_key = crate::ui::motion::indexed_key("dockgrp:", side as usize * 8 + gi);
            let p = crate::ui::motion::value_or(slide_key, 1.0);
            container(column(layers).height(Fill))
                .width(Length::Fixed((width * p).max(1.0)))
                .clip(true)
                .into()
        });

        let mut children: Vec<Element<'_, Message>> = Vec::new();
        match side {
            crate::app::config::DockSide::Left => {
                children.push(strip);
                children.extend(column_el);
            }
            crate::app::config::DockSide::Right => {
                children.extend(column_el);
                children.push(strip);
            }
        }
        Some(
            mouse_area(row(children).height(Fill))
                .on_enter(Message::Dock(crate::ui::dock::DockMsg::HoverStay))
                .on_exit(Message::Dock(crate::ui::dock::DockMsg::HoverExit))
                .into(),
        )
    }

    /// An edge's vertical icon strip: per group its open pallets' icons in a
    /// row ended by a hairline divider, then a + button opening the pallet
    /// menu. The band along a group's window-side edge is its grip: it
    /// carries the blue bar of the shown group, lights up while hovered (or
    /// while it is a drop target) and drags the whole group; an icon shows its
    /// group on click and drags just that pallet. Heights follow
    /// `dock::strip_layout`, which drop targeting uses too.
    fn dock_icon_strip(&self, side: crate::app::config::DockSide) -> Element<'_, Message> {
        use crate::app::config::DockSide;
        use crate::ui::dock::{
            DockMsg, DropTarget, DOCK_STRIP_W, STRIP_CELL_H, STRIP_GRIP_W, STRIP_PAD,
            STRIP_PLUS_GAP,
        };
        // Only a group actually on screen is marked as shown: on an
        // auto-hiding edge nothing is, until a group is revealed.
        let shown = self
            .dock_shown_group(side)
            .filter(|_| self.dock_edge_expanded(side));
        let movement = self.dock_drag.and_then(|d| d.movement());
        let move_target = movement.and_then(|(_, _, target)| target);
        let dragging = move_target.is_some();
        let target_group = match move_target {
            Some(DropTarget::Join { side: s, group, .. }) if s == side => Some(group),
            _ => None,
        };
        let mut items: Vec<Element<'_, Message>> = Vec::new();
        // The rail lists every pallet placed on this edge, open or closed: it
        // is the launcher for the edge, so a closed pallet still needs its
        // entry. Closed ones draw dimmed and open on press.
        for gi in 0..self.dock.groups(side).len() {
            let is_shown = shown == Some(gi);
            let lit =
                target_group == Some(gi) || (!dragging && self.dock_grip_hover == Some((side, gi)));
            let group_dragged =
                dragging && movement.is_some_and(|(_, group, _)| group == Some((side, gi)));
            let panels = &self.dock.groups(side)[gi].panels;
            let row_h = 2.0 * STRIP_PAD + panels.len() as f32 * STRIP_CELL_H;

            let mut icons: Vec<Element<'_, Message>> = vec![Space::new().height(STRIP_PAD).into()];
            for &id in panels.iter() {
                let closed = !self.dock_panel_visible(id);
                let faded = closed
                    || group_dragged
                    || (dragging
                        && movement
                            .is_some_and(|(panel, group, _)| group.is_none() && panel == id));
                let glyph = if faded {
                    crate::ui::icons::themed_disabled(id.icon(), 20.0)
                } else if is_shown {
                    crate::ui::icons::themed(id.icon(), 20.0)
                } else {
                    crate::ui::icons::themed_secondary(id.icon(), 20.0)
                };
                let hovered = !dragging && self.dock_icon_hover == Some(id);
                // Icon over name: the rail is a launcher, so every pallet is
                // readable at a glance rather than a wall of bare glyphs.
                let face = column![
                    glyph,
                    text(id.rail_label())
                        .size(tokens::TEXT_MICRO)
                        .style(kit::faint_style),
                ]
                .spacing(tokens::GAP_HAIRLINE)
                .width(Fill)
                .align_x(iced::Center);
                let cell = container(face)
                    .center_x(Length::Fixed(DOCK_STRIP_W))
                    .center_y(Length::Fixed(STRIP_CELL_H - 2.0))
                    .style(move |theme: &Theme| {
                        // A click target inside the strip's chrome band: it
                        // lifts to a plate on hover and carries no paint
                        // otherwise.
                        let tier = if hovered {
                            kit::Tier::Plate
                        } else {
                            kit::Tier::Flush
                        };
                        kit::wash_style(tier, theme, "")
                    });
                let cell = mouse_area(
                    container(cell)
                        .center_x(Length::Fixed(DOCK_STRIP_W))
                        .center_y(Length::Fixed(STRIP_CELL_H)),
                )
                .on_press(Message::Dock(DockMsg::IconPress(id)))
                .on_enter(Message::Dock(DockMsg::Hover(id)))
                .on_exit(Message::Dock(DockMsg::HoverEnd(id)))
                .interaction(iced::mouse::Interaction::Pointer);
                icons.push(cell.into());
            }

            // The group grip: a band on the window side holding the edge bar.
            let bar_w = if lit { 5.0 } else { 3.0 };
            let bar_inset = 5.0;
            let bar = container(
                Space::new()
                    .width(bar_w)
                    .height(Length::Fixed(row_h - 2.0 * bar_inset)),
            )
            .style(move |theme: &Theme| {
                // The edge bar is a mark: the accent indicator paints it
                // while the group is shown or its grip is lit (the width
                // carries the difference), and it is bare otherwise.
                if lit || is_shown {
                    kit::indicator_style(theme, "")
                } else {
                    kit::wash_style(kit::Tier::Flush, theme, "")
                }
            });
            let band = mouse_area(
                container(bar)
                    .width(Length::Fixed(STRIP_GRIP_W))
                    .height(Length::Fixed(row_h))
                    .align_y(iced::alignment::Vertical::Center)
                    .align_x(match side {
                        DockSide::Left => iced::alignment::Horizontal::Left,
                        DockSide::Right => iced::alignment::Horizontal::Right,
                    }),
            )
            .on_press(Message::Dock(DockMsg::GroupGrab(side, gi)))
            .on_enter(Message::Dock(DockMsg::GripHover(side, gi)))
            .on_exit(Message::Dock(DockMsg::GripHoverEnd(side, gi)))
            .interaction(iced::mouse::Interaction::Grab);
            let band = container(band)
                .width(Fill)
                .height(Length::Fixed(row_h))
                .align_x(match side {
                    DockSide::Left => iced::alignment::Horizontal::Left,
                    DockSide::Right => iced::alignment::Horizontal::Right,
                });
            items.push(
                // A stack is as tall as its first layer, so the icon column
                // spans the whole row; the edge bar then centres on the row.
                container(stack![column(icons).height(Length::Fixed(row_h)), band])
                    .width(Length::Fixed(DOCK_STRIP_W))
                    .height(Length::Fixed(row_h))
                    .style(move |theme: &Theme| {
                        // A shown group is a selected row in the strip: the
                        // app's one wash for that, never its own alpha.
                        if lit {
                            kit::tint_style(kit::Tint::Selected, theme, "")
                        } else {
                            kit::wash_style(kit::Tier::Flush, theme, "")
                        }
                    })
                    .into(),
            );
            items.push(kit::rule_h("").into());
        }
        items.push(Space::new().height(STRIP_PLUS_GAP).into());
        let plus = kit::icon_btn(
            crate::ui::icons::themed_secondary(crate::ui::icons::PLUS, 14.0),
            kit::Role::Standard,
            28.0,
            kit::Press::On(Message::Dock(DockMsg::EdgeMenu(Some(side)))),
            "",
        );
        items.push(container(plus).center_x(Length::Fixed(DOCK_STRIP_W)).into());
        container(column(items))
            .width(Length::Fixed(DOCK_STRIP_W))
            .height(Fill)
            // The sheet's rail is its own card: a narrow launcher floating
            // beside the drawing, separate from the panels' cards.
            .style(|theme: &Theme| {
                if crate::ui::style::draftflow_theme::draftflow_colors(theme).is_some() {
                    kit::Tier::Card.style(theme, "")
                } else {
                    kit::Tier::Band.style(theme, "")
                }
            })
            .into()
    }

    /// The content of panel `id` at `width`. A `floating` panel leaves its
    /// title to the floating frame's vertical title bar.
    pub(in crate::app) fn panel_body<'a>(
        &'a self,
        id: crate::ui::dock::PanelId,
        width: f32,
        floating: bool,
        tab: &'a DocumentTab,
    ) -> Element<'a, Message> {
        let chrome = crate::ui::dock::Chrome {
            auto_collapse: self.dock.auto_hides(id),
            floating,
            title_hovered: self.dock_title_hover == Some(id),
        };
        let body = match id {
            crate::ui::dock::PanelId::Properties => tab.properties.view(width, chrome, &self.active_theme),
            crate::ui::dock::PanelId::GraphicAttributes => {
                crate::ui::window::graphic_attributes::view(
                    &self.graphic_attributes,
                    &tab.scene,
                    &tab.properties,
                    width,
                    chrome,
                    self.dock
                        .location(id)
                        .map_or(crate::app::config::DockSide::Right, |(side, _)| side),
                    matches!(
                        self.color_pick_target,
                        Some((crate::app::ColorPickTarget::Gradient(_), _))
                    ),
                    &self.active_theme,
                )
            }
            crate::ui::dock::PanelId::BlockPalette => {
                crate::ui::window::block_palette::view(&self.block_palette, width, chrome, &self.active_theme)
            }
            crate::ui::dock::PanelId::ExternalReferences => {
                self.xref_manager
                    .view(width, chrome, tab.xref_missing, &tab.scene.document, &self.active_theme)
            }
            crate::ui::dock::PanelId::Browser => crate::ui::window::browser::view(
                &tab.scene.document,
                tab.sketch_session
                    .as_ref()
                    .map(|session| session.name.as_str()),
                width,
                chrome,
            ),
            crate::ui::dock::PanelId::NodeGraph => tab.graph.panel(width, chrome),
            crate::ui::dock::PanelId::Count => crate::ui::window::count_palette::view(
                &self.count_palette,
                tab.count.as_ref(),
                &tab.scene.document,
                tab.id,
                tab.scene.geometry_epoch,
                width,
                chrome,
                &self.active_theme,
            ),
            crate::ui::dock::PanelId::PointCloudManager => crate::ui::window::pc_manager::view(
                &self.pc_manager,
                &tab.scene.document,
                &tab.scene.selected_handles_in_order(),
                width,
                chrome,
            ),
            crate::ui::dock::PanelId::SheetSetManager => {
                crate::ui::window::sheet_set::view(&self.sheet_set, width, chrome, &self.active_theme)
            }
            crate::ui::dock::PanelId::Layers => {
                tab.layers.view_panel(self.layer_name_col_w, width, chrome, &self.active_theme)
            }
        };
        // One card tier under every pallet: the panel reads as a raised sheet
        // against the canvas regardless of which of the ten views it hosts.
        // On the sheet the card also wears its own edge and lift — every
        // floating panel there is a card, and a wash with no edge would let
        // two adjacent panels merge into one field. Vendor themes keep the
        // fill wash alone, where the strip band owns the frame.
        container(body)
            .width(Fill)
            .height(Fill)
            .style(|theme: &Theme| {
                if crate::ui::style::draftflow_theme::draftflow_colors(theme).is_some() {
                    kit::Tier::Card.style(theme, "")
                } else {
                    kit::wash_style(kit::Tier::Card, theme, "")
                }
            })
            .into()
    }

    /// A pallet of the shown group: its body plus a grabbable divider
    /// against the viewport (sizing the whole group). Hovering is handled by
    /// the enclosing edge region (see `build_edge_stack`), so the body stays
    /// fully interactive without fighting the region's hover tracking.
    fn expanded_panel<'a>(
        &'a self,
        id: crate::ui::dock::PanelId,
        side: crate::app::config::DockSide,
        width: f32,
        tab: &'a DocumentTab,
    ) -> Element<'a, Message> {
        let panel = self.panel_body(id, width, false, tab);
        let divider = dock_divider(id);
        match side {
            crate::app::config::DockSide::Left => row![panel, divider].height(Fill).into(),
            crate::app::config::DockSide::Right => row![divider, panel].height(Fill).into(),
        }
    }

    /// A floating panel at its saved place: a vertical title bar on the side
    /// facing the nearer workspace edge, the panel beside it, and a corner
    /// grip to resize it. With its pin on, the panel hides down to the title
    /// bar until hovered.
    fn floating_panel<'a>(
        &'a self,
        f: crate::ui::dock::FloatPanel,
        tab: &'a DocumentTab,
    ) -> Element<'a, Message> {
        use crate::ui::dock::DockMsg;
        let FloatFrame {
            x,
            y,
            bar_left,
            hidden,
        } = self.dock_float_frame(f);
        let hovered = self.dock_title_hover == Some(f.id);
        let bar = floating_title_bar(f.id, self.dock.auto_collapse(f.id), bar_left, hovered);
        let content: Element<'_, Message> = if hidden {
            bar
        } else {
            // The resize grip sits in the bottom corner away from the bar.
            let grip_left = !bar_left;
            let grip_icon = crate::ui::icons::themed_secondary(
                if grip_left {
                    crate::ui::icons::RESIZE_LEFT
                } else {
                    crate::ui::icons::RESIZE
                },
                12.0,
            );
            let grip = mouse_area(
                container(grip_icon)
                    .center_x(Length::Fixed(14.0))
                    .center_y(Length::Fixed(14.0)),
            )
            .on_press(Message::Dock(DockMsg::FloatResizeGrab(f.id, grip_left)))
            .interaction(if grip_left {
                iced::mouse::Interaction::ResizingDiagonallyUp
            } else {
                iced::mouse::Interaction::ResizingDiagonallyDown
            });
            let body = stack![
                self.panel_body(f.id, f.w, true, tab),
                container(grip)
                    .width(Fill)
                    .height(Fill)
                    .align_x(if grip_left {
                        iced::alignment::Horizontal::Left
                    } else {
                        iced::alignment::Horizontal::Right
                    })
                    .align_y(iced::alignment::Vertical::Bottom),
            ]
            .width(Length::Fixed(f.w))
            .height(Fill);
            if bar_left {
                row![bar, body].into()
            } else {
                row![body, bar].into()
            }
        };
        // Clipped, so a panel squeezed below its content's height never
        // draws outside its frame. The panel floats over the drawing, so the
        // plate is glass reading that frame; the host keeps the clip and the
        // frame size and paints nothing of its own.
        let framed = kit::glass_pane(
            container(content)
                .height(Length::Fixed(f.h))
                .clip(true)
                .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
                .into(),
            &self.active_theme,
            false,
            "",
        );
        // The idle cursor marks the panel as opaque to the pointer, so clicks
        // on its empty areas don't fall through to the drawing below.
        let panel = mouse_area(framed)
            .on_press(Message::Dock(DockMsg::FloatRaise(f.id)))
            .on_enter(Message::Dock(DockMsg::Hover(f.id)))
            .on_exit(Message::Dock(DockMsg::HoverExit))
            .interaction(iced::mouse::Interaction::Idle);
        place_at(panel.into(), x, y)
    }

    /// Where floating panel `f` draws.
    fn dock_float_frame(&self, f: crate::ui::dock::FloatPanel) -> FloatFrame {
        let (ww, wh) = self.dock_workspace_size();
        // The bar faces the nearer workspace edge; while resizing, it stays
        // opposite the grip being dragged so it cannot flip mid-drag.
        let bar_left = match self.dock_drag {
            Some(crate::ui::dock::DockDrag::FloatSize { panel, from_left }) if panel == f.id => {
                !from_left
            }
            _ => f.x + (f.w + DOCK_FLOAT_BAR_W) * 0.5 < ww * 0.5,
        };
        let hidden = self.dock.auto_collapse(f.id) && self.dock_peek != Some(f.id);
        // A hidden panel keeps its title bar where it sits when shown.
        let x = if hidden && !bar_left { f.x + f.w } else { f.x };
        let frame = FloatFrame {
            x,
            y: f.y,
            bar_left,
            hidden,
        };
        // Keep the panel on screen when the window shrank since it was placed.
        FloatFrame {
            x: x.min(ww - frame.width(f)).max(0.0),
            y: f.y.min(wh - f.h).max(0.0),
            ..frame
        }
    }

    /// The open right-click menu of a floating title bar: Allow docking,
    /// Dock left and Dock right. It opens at the pointer, flipped to stay on
    /// screen.
    fn dock_float_menu_overlay(&self) -> Option<Element<'_, Message>> {
        use crate::app::config::DockSide;
        use crate::ui::dock::DockMsg;
        let id = self.dock_float_menu?;
        let f = self.dock.float_rect(id)?;
        if !self.dock_panel_visible(id) {
            return None;
        }
        let (ww, wh) = self.dock_workspace_size();
        let frame = self.dock_float_frame(f);
        // Where the right-click happened, in the workspace.
        let at = iced::Point::new(
            frame.bar_x(f) + self.dock_float_menu_at.x,
            frame.y + self.dock_float_menu_at.y,
        );
        let menu_x = if at.x + DOCK_MENU_W <= ww {
            at.x
        } else {
            at.x - DOCK_MENU_W
        };
        let menu_x = menu_x.clamp(0.0, (ww - DOCK_MENU_W).max(0.0));
        let menu_y = at.y.min(wh - FLOAT_MENU_H).max(0.0);
        let item =
            |label: Element<'static, Message>, press: DockMsg| -> Element<'static, Message> {
                kit::list_row(label, false, kit::Press::On(Message::Dock(press)), "").into()
            };
        let check: Element<'static, Message> = if f.docking {
            crate::ui::icons::themed(crate::ui::icons::CHECK, 12.0)
        } else {
            Space::new().width(12).height(12).into()
        };
        let docking = row![
            text(t!("Allow docking")).size(tokens::TEXT_BODY).width(Fill),
            check
        ]
        .align_y(iced::Center);
        let separator = kit::hdivider(Fill);
        let menu = column![
            item(docking.into(), DockMsg::FloatDockingToggle(id)),
            container(separator).padding(kit::PAD_CELL),
            item(
                text(t!("Dock left")).size(tokens::TEXT_BODY).into(),
                DockMsg::DockTo(id, DockSide::Left)
            ),
            item(
                text(t!("Dock right")).size(tokens::TEXT_BODY).into(),
                DockMsg::DockTo(id, DockSide::Right)
            ),
        ]
        .spacing(tokens::GAP_HAIRLINE)
        .padding(tokens::GAP_TIGHT);
        Some(place_at(
            menu_frame(menu.into(), &self.active_theme),
            menu_x,
            menu_y,
        ))
    }

    /// The live drop preview for dragged panel `id`: a tint over the target
    /// edge plus a panel-sized ghost where it lands — stacked into a group
    /// (sized as that group will be), as a new group of its own (full
    /// height), or as a floating window.
    fn dock_drop_preview(
        &self,
        id: crate::ui::dock::PanelId,
        target: crate::ui::dock::DropTarget,
    ) -> Element<'_, Message> {
        use crate::app::config::DockSide;
        use crate::ui::dock::DropTarget;
        let (ww, _) = self.dock_workspace_size();
        let strip = crate::ui::dock::DOCK_STRIP_W;
        let tint = |side: DockSide, w: f32| -> Element<'_, Message> {
            let band = container(Space::new())
                .width(Length::Fixed(w + strip))
                .height(Fill)
                // An open drop target is the loudest wash in the kit.
                .style(|theme: &Theme| kit::tint_style(kit::Tint::Dropped, theme, ""));
            container(band)
                .width(Fill)
                .height(Fill)
                .align_x(match side {
                    DockSide::Left => iced::alignment::Horizontal::Left,
                    DockSide::Right => iced::alignment::Horizontal::Right,
                })
                .into()
        };
        let mut layers: Vec<Element<'_, Message>> = Vec::new();
        let group_drag = self
            .dock_drag
            .and_then(|d| d.movement())
            .and_then(|(_, g, _)| g);
        let marks_side = match target {
            DropTarget::Edge { side, .. } | DropTarget::Join { side, .. } => Some(side),
            DropTarget::Float { x, y } => {
                let (w, h) = self.dock_float_size(id);
                layers.push(place_at(dock_ghost(id, w + DOCK_FLOAT_BAR_W, h), x, y));
                None
            }
        };
        // A single pallet also gets a ghost where it lands in the column; a
        // whole group shows only the strip feedback, as the column keeps
        // showing what is there.
        if let (Some(side), None) = (marks_side, group_drag) {
            if let Some((top, bottom, w)) = self.dock_landing(id, target) {
                let x = match side {
                    DockSide::Left => strip,
                    DockSide::Right => (ww - w - strip).max(0.0),
                };
                layers.push(tint(side, w));
                layers.push(place_at(dock_ghost(id, w, bottom - top), x, top));
            }
        }
        // On top: the strip's drop marks and a ghost of what is dragged
        // following the pointer.
        if let Some(side) = marks_side {
            layers.extend(self.dock_strip_drop_marks(side, target));
            if let Some(p) = self.dock_drag_last {
                let icons = match group_drag {
                    Some((s, g)) => self.dock_group_visible(s, g),
                    None => vec![id],
                };
                layers.push(place_at(dock_icon_ghost(&icons), p.x - 16.0, p.y - 15.0));
            }
        }
        iced::widget::Stack::with_children(layers)
            .width(Fill)
            .height(Fill)
            .into()
    }

    /// Where docked pallet `id` would sit after dropping on `target`: its
    /// span (top, bottom) in its group as laid out after the drop, and the
    /// group's width. `None` for a floating target.
    fn dock_landing(
        &self,
        id: crate::ui::dock::PanelId,
        target: crate::ui::dock::DropTarget,
    ) -> Option<(f32, f32, f32)> {
        use crate::ui::dock::DropTarget;
        let mut after = self.dock.clone();
        match target {
            DropTarget::Edge { side, index } => {
                after.dock(id, side, index);
            }
            DropTarget::Join { side, group, index } => {
                after.join_group(id, side, group, index);
            }
            DropTarget::Float { .. } => return None,
        }
        let (side, gi) = after.location(id)?;
        let group = &after.groups(side)[gi];
        let (_, top, bottom) = self
            .dock_open_spans(group, Some(id))
            .into_iter()
            .find(|(i, _, _)| group.panels[*i] == id)?;
        Some((top, bottom, after.group_width_px(side, gi, self.win_size.0)))
    }

    /// The icon strip's drop marks for `target` on `side`: a short line with
    /// end caps between a group's icons (join that group there), or a
    /// full-width line on a divider with a "+ New group" badge.
    fn dock_strip_drop_marks(
        &self,
        side: crate::app::config::DockSide,
        target: crate::ui::dock::DropTarget,
    ) -> Vec<Element<'_, Message>> {
        use crate::app::config::DockSide;
        use crate::ui::dock::{DropTarget, DOCK_STRIP_W, STRIP_CELL_H};
        let (ww, _) = self.dock_workspace_size();
        let strip_x = match side {
            DockSide::Left => 0.0,
            DockSide::Right => ww - DOCK_STRIP_W,
        };
        let (layout, _) = self.dock_strip_layout(side);
        let mut marks: Vec<Element<'_, Message>> = Vec::new();
        match target {
            DropTarget::Join { group, index, .. } => {
                let Some(g) = layout.iter().find(|g| g.group == group) else {
                    return marks;
                };
                // Icons above the insertion point, counted as the strip shows
                // them: the rail lists every pallet of the group, so the
                // insertion index is already the cell offset.
                let above = index;
                let y = g.icons_top + above as f32 * STRIP_CELL_H;
                marks.push(place_at(dock_insert_line(), strip_x + 7.0, y - 3.5));
            }
            DropTarget::Edge { index, .. } => {
                let y = layout
                    .iter()
                    .find(|g| g.group >= index)
                    .map(|g| g.top)
                    .or_else(|| layout.last().map(|g| g.bottom()))
                    .unwrap_or(0.0);
                marks.push(place_at(
                    container(Space::new())
                        .width(Length::Fixed(DOCK_STRIP_W))
                        .height(Length::Fixed(4.0))
                        .style(move |theme: &Theme| kit::indicator_style(theme, ""))
                        .into(),
                    strip_x,
                    (y - 2.0).max(0.0),
                ));
                const BADGE_W: f32 = 96.0;
                let badge = container(
                    text(format!("+ {}", t!("New group"))).size(tokens::TEXT_MICRO),
                )
                .width(Length::Fixed(BADGE_W))
                .center_x(Length::Fixed(BADGE_W))
                .padding(kit::PAD_GLYPH)
                // A count plate pinned to the strip: accent fill, accent ink.
                .style(|theme: &Theme| kit::tint_style(kit::Tint::Badge, theme, ""));
                let badge_x = match side {
                    DockSide::Left => DOCK_STRIP_W + 6.0,
                    DockSide::Right => ww - DOCK_STRIP_W - 6.0 - BADGE_W,
                };
                marks.push(place_at(badge.into(), badge_x, (y - 9.0).max(0.0)));
            }
            DropTarget::Float { .. } => {}
        }
        marks
    }
}

/// The short insertion line with end caps shown between a group's icons.
fn dock_insert_line() -> Element<'static, Message> {
    let cap = || {
        container(Space::new())
            .width(Length::Fixed(3.0))
            .height(Length::Fixed(7.0))
            .style(move |theme: &Theme| kit::indicator_style(theme, ""))
    };
    row![
        cap(),
        container(Space::new())
            .width(Length::Fixed(20.0))
            .height(Length::Fixed(3.0))
            .style(move |theme: &Theme| kit::indicator_style(theme, "")),
        cap(),
    ]
    .align_y(iced::Center)
    .height(Length::Fixed(7.0))
    .into()
}

/// The ghost following the pointer while pallets are dragged: their icons
/// in an accent-framed box.
fn dock_icon_ghost(icons: &[crate::ui::dock::PanelId]) -> Element<'static, Message> {
    let glyphs: Vec<Element<'static, Message>> = icons
        .iter()
        .map(|id| {
            container(crate::ui::icons::themed(id.icon(), 18.0))
                .center_x(Length::Fixed(32.0))
                .center_y(Length::Fixed(30.0))
                .into()
        })
        .collect();
    container(column(glyphs))
        // A thing being dragged: the kit's ghost wash — accent at ring
        // strength, control corner, no shadow of its own.
        .style(|theme: &Theme| kit::tint_style(kit::Tint::Ghost, theme, ""))
        .into()
}

/// Canvas that draws a floating title bar's label, reading upward from the
/// bar's bottom end.
struct VBarLabel {
    text: String,
}

impl canvas::Program<Message> for VBarLabel {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: iced::advanced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        frame.with_save(|frame| {
            frame.translate(iced::Vector::new(bounds.width / 2.0, bounds.height / 2.0));
            frame.rotate(iced::Radians(-std::f32::consts::FRAC_PI_2));
            // After the rotation the text runs along local +x (upward), so
            // the bar's bottom end sits at -height/2.
            frame.fill_text(canvas::Text {
                content: self.text.clone(),
                position: iced::Point::new(-bounds.height / 2.0, 0.0),
                color: theme.palette().background.base.text.scale_alpha(0.72),
                size: iced::Pixels(13.0),
                align_x: iced::advanced::text::Alignment::Left,
                align_y: iced::alignment::Vertical::Center,
                shaping: iced::advanced::text::Shaping::Advanced,
                ..Default::default()
            });
        });
        vec![frame.into_geometry()]
    }
}

/// Width of a floating panel's vertical title bar.
const DOCK_FLOAT_BAR_W: f32 = 28.0;

/// Where a floating panel draws: its top-left corner, whether its title bar
/// is on the left, and whether it is hidden down to that bar (auto-hide, not
/// hovered).
#[derive(Debug, Clone, Copy)]
struct FloatFrame {
    x: f32,
    y: f32,
    bar_left: bool,
    hidden: bool,
}

impl FloatFrame {
    /// On-screen width of panel `f` in this frame: the bar, plus the panel
    /// unless hidden.
    fn width(&self, f: crate::ui::dock::FloatPanel) -> f32 {
        if self.hidden {
            DOCK_FLOAT_BAR_W
        } else {
            f.w + DOCK_FLOAT_BAR_W
        }
    }

    /// Left edge of the title bar.
    fn bar_x(&self, f: crate::ui::dock::FloatPanel) -> f32 {
        if self.bar_left {
            self.x
        } else {
            self.x + self.width(f) - DOCK_FLOAT_BAR_W
        }
    }
}

/// A floating panel's vertical title bar: close then the hide (auto-collapse)
/// pin at the top, the panel's icon at the bottom with its title reading
/// upward above it. Pressing the bar's free area drags the panel.
fn floating_title_bar(
    id: crate::ui::dock::PanelId,
    auto_collapse: bool,
    on_left: bool,
    hovered: bool,
) -> Element<'static, Message> {
    use crate::ui::dock::DockMsg;
    let tip = if on_left {
        iced::widget::tooltip::Position::Left
    } else {
        iced::widget::tooltip::Position::Right
    };
    let title = canvas(VBarLabel {
        text: t!(id.title()).into_owned(),
    })
    .width(Fill)
    .height(Fill);
    // Close and pin only show while the pointer is over the bar, as on a
    // docked title bar.
    let mut bar = column![].spacing(tokens::GAP_HAIRLINE);
    if hovered {
        bar = bar
            .push(crate::ui::dock::close_button(id, tip))
            .push(crate::ui::dock::pin_button(id, auto_collapse, tip));
    }
    let bar = bar
        .push(container(title).width(Fill).height(Fill).padding([tokens::GAP_CONTROL, tokens::GAP_NONE]))
        .push(container(crate::ui::icons::themed(id.icon(), 16.0)).center_x(Fill))
        .padding([tokens::GAP_TIGHT, tokens::GAP_NONE])
        .align_x(iced::Center)
        .width(Length::Fixed(DOCK_FLOAT_BAR_W))
        .height(Fill);
    mouse_area(container(bar).style(move |theme: &Theme| {
        // The grab bar is the floating panel's chrome: a quiet plate at rest
        // that lifts to a card while the pointer is there to drag.
        if hovered {
            kit::Tier::Card.style(theme, "")
        } else {
            kit::Tier::Plate.style(theme, "")
        }
    }))
    .on_press(Message::Dock(DockMsg::DockGrab(id)))
    .on_right_press(Message::Dock(DockMsg::FloatMenu(Some(id))))
    .on_enter(Message::Dock(DockMsg::TitleHover(id)))
    .on_move(|p| Message::Dock(DockMsg::FloatBarPointer(p)))
    .on_exit(Message::Dock(DockMsg::TitleHoverEnd(id)))
    // Double-click docks it against the edge its title bar faces.
    .on_double_click(Message::Dock(DockMsg::DockTo(
        id,
        if on_left {
            crate::app::config::DockSide::Left
        } else {
            crate::app::config::DockSide::Right
        },
    )))
    .interaction(iced::mouse::Interaction::Grab)
    .into()
}

/// Grabbable separator between a docked pallet and the viewport: drag to
/// size its group, double-click to reset the width.
fn dock_divider(id: crate::ui::dock::PanelId) -> Element<'static, Message> {
    let line = container(Space::new())
        .width(Length::Fixed(DOCK_DIVIDER_W))
        .height(Fill)
        .style(|theme: &Theme| kit::wash_style(kit::Tier::Sink, theme, ""));
    let grab = Message::Dock(crate::ui::dock::DockMsg::ResizeGrab(id));
    let reset = Message::Dock(crate::ui::dock::DockMsg::WidthReset(id));
    mouse_area(line)
        .on_press(grab)
        .on_double_click(reset)
        .interaction(iced::mouse::Interaction::ResizingHorizontally)
        .into()
}

/// Width of the icon strip's pallet menu.
const DOCK_MENU_W: f32 = 230.0;
/// Rough height of a floating title bar's menu, to keep it on screen.
const FLOAT_MENU_H: f32 = 110.0;

/// The pallet menu of an edge's + button, styled like the right-click menu:
/// one row per pallet with its icon, its name and a check mark when it is
/// open on this edge. `checked` holds each pallet with that state.
fn dock_edge_menu(
    side: crate::app::config::DockSide,
    checked: &[(crate::ui::dock::PanelId, bool)],
    theme: &Theme,
) -> Element<'static, Message> {
    use crate::ui::dock::DockMsg;
    let rows: Vec<Element<'static, Message>> = checked
        .iter()
        .map(|(id, checked)| {
            let id = *id;
            let check: Element<'static, Message> = if *checked {
                crate::ui::icons::themed(crate::ui::icons::CHECK, 12.0)
            } else {
                Space::new().width(12).height(12).into()
            };
            kit::list_row(
                row![
                    crate::ui::icons::themed(id.icon(), 16.0),
                    text(t!(id.title())).size(tokens::TEXT_BODY).width(Fill),
                    check,
                ]
                .spacing(tokens::GAP_ROW)
                .align_y(iced::Center),
                false,
                kit::Press::On(Message::Dock(DockMsg::EdgeMenuToggle(side, id))),
                "",
            )
            .into()
        })
        .collect();
    menu_frame(
        column(rows).spacing(tokens::GAP_HAIRLINE).padding(tokens::GAP_TIGHT).into(),
        theme,
    )
}

/// The pane a dock menu sits in. The rows' host carries `Flush` — no fill, no
/// edge, no corner — so the glass underneath stays the only thing painting the
/// surface, and the menu's ink lands in the layer above the reader.
fn menu_frame(rows: Element<'static, Message>, theme: &Theme) -> Element<'static, Message> {
    let plate = container(rows)
        .width(Length::Fixed(DOCK_MENU_W))
        .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""));
    kit::glass_pane(plate.into(), theme, false, "")
}

/// Draggable bar between two stacked pallets of group `gi`; double-click
/// gives every pallet of the group the same height again. It spans exactly
/// the pallets' width (panel plus its divider): a `Fill` width would stretch
/// the whole edge column across the workspace and squeeze the drawing view.
fn dock_splitter(
    side: crate::app::config::DockSide,
    gi: usize,
    upper: usize,
    lower: usize,
    col_w: f32,
) -> Element<'static, Message> {
    let line = container(Space::new())
        .width(Length::Fixed(col_w + DOCK_DIVIDER_W))
        .height(Length::Fixed(5.0))
        .style(|theme: &Theme| kit::wash_style(kit::Tier::Sink, theme, ""));
    mouse_area(line)
        .on_press(Message::Dock(crate::ui::dock::DockMsg::SplitGrab(
            side, gi, upper, lower,
        )))
        .on_double_click(Message::Dock(crate::ui::dock::DockMsg::SplitReset(
            side, gi,
        )))
        .interaction(iced::mouse::Interaction::ResizingVertically)
        .into()
}

/// The blue drop ghost: a `w`×`h` outline of where pallet `id` lands, with
/// its name as the header.
fn dock_ghost(id: crate::ui::dock::PanelId, w: f32, h: f32) -> Element<'static, Message> {
    let header = container(
        text(t!(id.title()))
            .size(tokens::TEXT_BODY)
            .wrapping(iced::widget::text::Wrapping::None)
            .ellipsis(iced::advanced::text::Ellipsis::End),
    )
    .width(Fill)
    .padding(kit::Size::Compact.padding())
    .clip(true)
    // The kit's own accent plate: accent fill, accent ink, no edge and no corner.
    // The accent plate iced's tint ladder owns is `Tint::Badge`, whose corner is
    // `RADIUS_CONTROL`, and this bar is square because it is the head of a square
    // sheet — so it could not fold onto that one and is not folded here either.
    .style(kit::accent_plate_style);
    container(column![header, Space::new()])
        .width(Length::Fixed(w.max(1.0)))
        .height(Length::Fixed(h.max(1.0)))
        .clip(true)
        // `frame_style`'s ring with the page sheet under it at the sheet's own
        // alpha — `fade()` walks a plane in, it is not an alpha, so the transparency
        // stays this screen's argument rather than becoming a rung.
        .style(|theme: &Theme| kit::ghost_sheet_style(theme, 0.85))
        .into()
}

/// Position `content` with its top-left corner at workspace point (`x`, `y`).
fn place_at(content: Element<'_, Message>, x: f32, y: f32) -> Element<'_, Message> {
    container(content)
        .width(Fill)
        .height(Fill)
        .align_x(iced::alignment::Horizontal::Left)
        .align_y(iced::alignment::Vertical::Top)
        .padding(iced::Padding {
            top: y.max(0.0),
            right: 0.0,
            bottom: 0.0,
            left: x.max(0.0),
        })
        .into()
}
