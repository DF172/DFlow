use super::document::DocumentTab;
use super::document::DynComponent;
use super::history::history_dropdown_labels;
use super::{ArrowKey, Message, DraftFlowCAD};
use crate::scene::pick::grip::{grips_to_screen, grips_to_screen_paper, grips_to_screen_rte};
use crate::scene::view::viewport_pane::ViewportPane;
use crate::scene::{VIEWCUBE_PAD, VIEWCUBE_REGION_PX};
use crate::t;
use crate::ui::style::tokens;
use crate::ui::wrap_bar::DensitySwap;
use iced::widget::{column, container, mouse_area, pane_grid, responsive, row, shader, stack, text, Row, Space};
use crate::ui::kit::plain_button;
use iced::window;
use iced::{keyboard, Color, Element, Fill, Subscription, Task, Theme};
use iced_aw::ContextMenu;
use std::sync::Arc;

mod controls;
mod modal;
pub(in crate::app) mod overlay;
mod viewcube;

use controls::{dyn_component_value, viewport_controls};
use overlay::{
    mtext_editor_overlay, position_canvas_overlay, position_canvas_overlay_near_cursor,
    qselect_overlay, text_inline_overlay, viewport_context_menu_overlay,
};
use viewcube::{viewcube_nav_controls, viewcube_ucs_picker, UCS_PICKER_W};
use crate::ui::kit;

// Re-export the text-input element ids so sibling modules can address them at
// the `view::` path as before the split.
pub(in crate::app) use overlay::{MTEXT_TEXT_ID, TEXT_INLINE_ID};

pub(in crate::app) const VIEWPORT_CAPTURE_BOUNDS_ID: &str = "viewport-capture-bounds";

const VIEWCUBE_HIT_SIZE: f32 = VIEWCUBE_REGION_PX;

/// Background used by drafting overlays in model or paper space.
fn crosshair_background(tab: &DocumentTab, is_paper: bool) -> [f32; 4] {
    if !is_paper {
        return tab.scene.bg_color;
    }
    tab.scene.paper_bg_color
}

/// Which navigation tool the viewport cursor should advertise. The three flags
/// are mutually exclusive — every command entry point clears all of them before
/// arming one (`crate::app::commands`) — so the order here only settles a state
/// that cannot occur.
pub(in crate::app) fn nav_cursor(tab: &DocumentTab) -> crate::ui::overlay::NavCursor {
    use crate::ui::overlay::NavCursor;
    if tab.pan_mode {
        NavCursor::Pan
    } else if tab.orbit_mode {
        NavCursor::Orbit
    } else if tab.zoom_dynamic_mode {
        NavCursor::Zoom
    } else {
        NavCursor::None
    }
}

/// Clear gap (px) kept between the render-mode bar (top-left) and the ViewCube
/// (top-right) before the cube is judged to collide and hides.
const VIEWCUBE_GAP: f32 = 12.0;

/// True when a viewport `tile_w` px wide still has room for the ViewCube beside
/// a render-mode bar of measured width `bar_w`. When it doesn't, the cube hides
/// first (the bar keeps priority); the bar itself hides separately, only when it
/// no longer fits at all (its `DensitySwap`).
fn viewcube_has_room(bar_w: f32, tile_w: f32) -> bool {
    bar_w + VIEWCUBE_GAP + VIEWCUBE_REGION_PX + VIEWCUBE_PAD <= tile_w
}

fn hatch_pattern_key_event(
    event: iced::Event,
    status: iced::event::Status,
    _window: window::Id,
) -> Option<Message> {
    if !matches!(status, iced::event::Status::Captured) {
        return None;
    }
    let iced::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event else {
        return None;
    };
    match key {
        keyboard::Key::Named(keyboard::key::Named::ArrowLeft) => {
            Some(Message::PropHatchPatternNavigate(-1))
        }
        keyboard::Key::Named(keyboard::key::Named::ArrowRight) => {
            Some(Message::PropHatchPatternNavigate(1))
        }
        keyboard::Key::Named(keyboard::key::Named::ArrowUp) => {
            Some(Message::PropHatchPatternNavigate(-2))
        }
        keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
            Some(Message::PropHatchPatternNavigate(2))
        }
        _ => None,
    }
}

fn shortcut_capture_key_event(
    event: iced::Event,
    _status: iced::event::Status,
    _window: window::Id,
) -> Option<Message> {
    let iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event else {
        return None;
    };
    // Ignore bare modifier presses so holding Ctrl+Shift before the final
    // key doesn't end the capture early.
    if let keyboard::Key::Named(named) = &key {
        if matches!(
            named,
            keyboard::key::Named::Control
                | keyboard::key::Named::Shift
                | keyboard::key::Named::Alt
                | keyboard::key::Named::Super
                | keyboard::key::Named::Meta
        ) {
            return None;
        }
    }
    // Esc backs out of capture (and abandons an unfinished draft row) —
    // binding the ESCAPE key itself is still possible via SHORTCUTS SET.
    if matches!(key, keyboard::Key::Named(keyboard::key::Named::Escape)) {
        return Some(Message::ShortcutCaptureCancel);
    }
    shortcut_key_name(&key, modifiers).map(Message::ShortcutCaptureKey)
}

fn shortcut_key_name(key: &keyboard::Key, modifiers: keyboard::Modifiers) -> Option<String> {
    let key = match key {
        keyboard::Key::Character(value) if !value.is_empty() => value.to_uppercase(),
        keyboard::Key::Named(named) => {
            let name = format!("{named:?}");
            match name.as_str() {
                "ArrowUp" => "UP".to_string(),
                "ArrowDown" => "DOWN".to_string(),
                "ArrowLeft" => "LEFT".to_string(),
                "ArrowRight" => "RIGHT".to_string(),
                "PageUp" => "PAGEUP".to_string(),
                "PageDown" => "PAGEDOWN".to_string(),
                "Enter" | "Space" | "Escape" | "Delete" | "Backspace" | "Tab" | "Home" | "End"
                | "Insert" => name.to_uppercase(),
                _ if name.starts_with('F') && name[1..].chars().all(|ch| ch.is_ascii_digit()) => {
                    name
                }
                _ => return None,
            }
        }
        _ => return None,
    };
    let mut parts = Vec::with_capacity(5);
    if modifiers.control() {
        parts.push("CTRL".to_string());
    }
    if modifiers.logo() {
        parts.push("CMD".to_string());
    }
    if modifiers.alt() {
        parts.push("ALT".to_string());
    }
    if modifiers.shift() {
        parts.push("SHIFT".to_string());
    }
    parts.push(key);
    Some(parts.join("+"))
}

/// `ViewportRenderMode` enum carries the raw DXF integers, not a label,
/// so wrap it locally with a friendly name renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RenderModeChoice(pub codec::entities::ViewportRenderMode);

impl std::fmt::Display for RenderModeChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(crate::t!(crate::modules::view::visual_style::label_for(self.0)).as_ref())
    }
}

impl DraftFlowCAD {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn view(&self, window_id: window::Id) -> Element<'_, Message> {
        // ── Floating panel windows ─────────────────────────────────────────
        // All dialogs are in-canvas modals now (Plan B); view_main stacks the
        // active one. `window_id` is unused — there is only the main window.
        let _ = window_id;
        // Pan latency on a large drawing is ~120 ms while `update`, `prepare`
        // and `encode` together account for ~1 ms of it, the GPU sits at 0-8%,
        // and encoding no scene at all changes nothing. Widget-tree
        // construction is the largest span between the input handler returning
        // and `prepare` running, and it was never measured.
        let started = crate::perf::enabled().then(iced::time::Instant::now);
        let element = self.view_main();
        if let Some(started) = started {
            let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
            if elapsed_ms >= 1.0 {
                crate::perf_record!("[perf] view {:>7.1}ms", elapsed_ms);
            }
        }
        crate::perf::note_view_finish();
        element
    }

    /// The primary window: viewport, ribbon, tab bar, status bar. Split out of
    /// `view` so the single-window web build can render it directly, bypassing
    /// the multi-window id dispatch above (the web build has no extra windows).
    pub fn view_main(&self) -> Element<'_, Message> {
        // Boot splash owns the whole frame until its fade starts. During the
        // fade the workspace is built underneath and the plate — ground
        // included — fades over it, so the reveal is a warm, laid-out frame
        // instead of a cut to the clear colour.
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(splash) = &self.splash {
            let plate = splash.view(self.ui_scale_percent, self.splash_status());
            if splash.wants_workspace_below() {
                return stack![self.view_workspace(), plate].into();
            }
            return plate;
        }
        self.view_workspace()
    }

    /// The workspace proper: viewport, ribbon, tab bar, status bar.
    fn view_workspace(&self) -> Element<'_, Message> {
        // Tab stops are collected in paint order as the kit builds its
        // controls. Rebuilding the list every frame is what lets a control
        // that scrolled away, or a dialog that closed, drop out of the cycle
        // without an unregister call anywhere.
        crate::ui::focus::begin_pass();
        // Section marks for `view-detail`. A closure over a RefCell rather
        // than one binding per section: two of these sections sit inside a
        // nested block and would otherwise be out of scope at the report.
        let view_t0 = crate::perf::enabled().then(iced::time::Instant::now);
        let view_marks: std::cell::RefCell<Vec<(&'static str, f64)>> =
            std::cell::RefCell::new(Vec::with_capacity(8));
        let mark = |name: &'static str| {
            if let Some(t0) = view_t0 {
                view_marks
                    .borrow_mut()
                    .push((name, t0.elapsed().as_secs_f64() * 1000.0));
            }
        };

        let i = self.active_tab;
        let tab = &self.tabs[i];
        // Dynamic input is built before the status bar and properties panel,
        // so seed its shared formatter here rather than relying on either of
        // those later views to have done it for this drawing/thread.
        crate::entities::common::set_unit_context(
            crate::entities::common::UnitContext::from_header(&tab.scene.document.header),
        );
        let thumbnail_capture_clean = self.thumbnail_capture_clean;
        let theme_text = self.active_theme.palette().background.base.text;
        let viewcube_text_color = [theme_text.r, theme_text.g, theme_text.b, theme_text.a];
        let is_paper = tab.scene.current_layout != "Model";
        let committed_render_mode = if is_paper {
            tab.scene
                .active_viewport_render_mode()
                .unwrap_or(tab.render_mode)
        } else {
            tab.render_mode
        };
        // Gallery hover is a non-destructive live preview: only the shader's
        // input changes. Dismissal restores the committed mode, while clicking
        // a row follows the normal SetRenderMode path and persists it.
        let viewport_render_mode = if self.render_mode_menu_open {
            self.render_mode_preview.unwrap_or(committed_render_mode)
        } else {
            committed_render_mode
        };
        // Adaptive corner widgets: the ViewCube shows only while the active
        // viewport is wide enough to hold it *beside* the render-mode bar, whose
        // real width is measured each frame by its `DensitySwap` and read back
        // here (from the previous frame). This drives the GPU cube (via the pane
        // flag), the ViewCube nav/UCS widgets, and the hover hit-test alike, so
        // they never overlap. The bar itself hides independently (its own
        // DensitySwap) only when it no longer fits at all.
        let render_bar_w =
            f32::from_bits(self.render_bar_w.load(std::sync::atomic::Ordering::Relaxed));
        let active_vp_w = if is_paper {
            tab.scene
                .active_viewport
                .and_then(|h| {
                    let (cw, ch) = tab.scene.selection.borrow().view.vp_size;
                    tab.scene.viewport_screen_rect(h, (cw, ch))
                })
                .map(|r| r.width)
                .unwrap_or(f32::INFINITY)
        } else {
            let (vw, vh) = tab.scene.selection.borrow().view.vp_size;
            tab.scene.active_model_tile_bounds(vw, vh).width
        };
        let viewcube_visible = self.show_viewcube
            && !thumbnail_capture_clean
            && !tab.is_start
            && viewcube_has_room(render_bar_w, active_vp_w);
        // Start tab: render welcome page in place of the viewport.
        // Surrounding chrome (tab bar, status bar) stays; the welcome widget
        // returned here also flags the rest of `view` to skip drawing-only
        // overlays via `tab.is_start`.
        // Unified GPU widget for both layouts. A paper layout renders through
        // the same shader as model space: a full-canvas top-locked "sheet"
        // viewport draws the layout's own geometry (white sheet + entities +
        // borders) and the floating content viewports blit on top.
        mark("viewport_3d");
        let viewport_3d: Element<'_, Message> = if self.layout_settling && !tab.is_start {
            // Omit the shader and drafting overlays for the notice frame.
            let bg = crosshair_background(tab, is_paper);
            let lum = 0.299 * bg[0] + 0.587 * bg[1] + 0.114 * bg[2];
            let ink = if lum > 0.5 {
                Color::from_rgba(0.0, 0.0, 0.0, 0.55)
            } else {
                Color::from_rgba(1.0, 1.0, 1.0, 0.55)
            };
            container(
                text(crate::tr!("viewport", "preparing-layout"))
                    .size(tokens::TEXT_SECTION)
                    .color(ink),
            )
            .width(Fill)
            .height(Fill)
            .center_x(Fill)
            .center_y(Fill)
            // The notice frame covers the canvas, so it has to wear the canvas's
            // own colour — model background or paper desk, whichever this layout
            // is settling into — and nothing else: no edge, no shadow.
            .style(move |_| {
                kit::document_style(Color::from_rgba(bg[0], bg[1], bg[2], bg[3]), "")
            })
            .into()
        } else if tab.is_start {
            start_page_view(
                &self.recent_files,
                &self.recent_thumbs,
                self.recent_limit,
                &self.recent_limit_input,
                self.start_section,
                &self.start_query,
                self.start_filter,
                &self.tabs,
                self.savetime_min,
                &self.active_theme,
            )
        } else if is_paper {
            shader(ViewportPane::model(
                &tab.scene,
                viewcube_visible,
                !thumbnail_capture_clean,
                viewport_render_mode,
                viewcube_text_color,
            ))
            .width(Fill)
            .height(Fill)
            .into()
        } else {
            // Model space: a pane_grid of per-pane shader widgets (rendering
            // only). The input mouse_areas live in a SECOND, identical pane_grid
            // layered ABOVE the crosshair overlay (`model_input_layer` below) —
            // they must sit above the selection overlay, whose `Hidden` cursor
            // interaction otherwise "levitates" the cursor and starves any layer
            // beneath it of mouse events. A separate eventless `responsive`
            // Space captures the area size to keep `vp_size` / tile rects
            // current (building the pane_grid inside `responsive` resets the
            // mouse_areas' hover state and drops their move events).
            let scene = &tab.scene;
            let show_viewcube = viewcube_visible;
            let show_interaction = !thumbnail_capture_clean;
            let render_mode = viewport_render_mode;
            let size_probe: Element<'_, Message> = responsive(move |size| {
                {
                    let mut sel = scene.selection.borrow_mut();
                    sel.view.vp_size = (size.width, size.height);
                }
                scene.sync_tiles_from_panes(size.width, size.height);
                Space::new().width(Fill).height(Fill)
            })
            .into();
            let shaders =
                pane_grid::PaneGrid::new(&scene.model_panes, move |_pane, &idx, _maximized| {
                    pane_grid::Content::new(
                        shader(ViewportPane::for_pane(
                            scene,
                            show_viewcube,
                            show_interaction,
                            render_mode,
                            idx,
                            viewcube_text_color,
                        ))
                        .width(Fill)
                        .height(Fill),
                    )
                })
                .width(Fill)
                .height(Fill)
                .min_size(scene.model_pane_min_px())
                .spacing(crate::scene::TILE_DIVIDER_PX);
            stack![size_probe, shaders].width(Fill).height(Fill).into()
        };

        // Per-pane input layer: a pane_grid of transparent mouse_areas matching
        // the shader pane_grid (same `model_panes` → identical layout). Layered
        // above the crosshair overlay so it actually receives mouse events, and
        // it owns the divider resize. Only built for the Model layout.
        mark("model_input");
        let model_input_layer: Option<Element<'_, Message>> =
            if is_paper || tab.is_start || self.layout_settling {
                None
            } else {
                let scene = &tab.scene;
                Some(
                    pane_grid::PaneGrid::new(&scene.model_panes, |_pane, &idx, _maximized| {
                        pane_grid::Content::new(pane_mouse_area(idx))
                    })
                    .width(Fill)
                    .height(Fill)
                    .min_size(scene.model_pane_min_px())
                    .spacing(crate::scene::TILE_DIVIDER_PX)
                    .on_resize(6.0, Message::PaneResized)
                    .into(),
                )
            };

        mark("grid");
        let grid_overlay = if self.layout_settling || tab.is_start {
            Space::new().into()
        } else {
            let (vw, vh) = tab.scene.selection.borrow().view.vp_size;
            let model_basis = {
                let (o, ux, uy, uz) = tab.ucs_xform().axes();
                let (ux, uy, uz) = super::helpers::drafting_axes(
                    ux,
                    uy,
                    uz,
                    self.isometric_drafting,
                    self.iso_plane,
                    self.snap_angle_deg,
                );
                (o, (ux.as_vec3(), uy.as_vec3(), uz.as_vec3()))
            };
            // The first paper frame spends seconds in this block and `grid`
            // alone cannot say which part. Reported only when it costs
            // something, so ordinary frames stay quiet.
            let t_grid = crate::perf::enabled().then(iced::time::Instant::now);
            let views = tab.scene.grid_views(vw, vh);
            let views_ms = crate::perf::elapsed_ms(t_grid);
            let view_count = views.len();
            let t_params = crate::perf::enabled().then(iced::time::Instant::now);
            let grid: Vec<crate::ui::overlay::GridParams> = views
                .into_iter()
                .map(|(bounds, cam, handle)| {
                    let (origin, mut axes): (glam::DVec3, _) = if is_paper {
                        match tab.ucs_from_viewport(handle) {
                            Some(u) => {
                                let (o, ux, uy, uz) = super::helpers::UcsXform::from_ucs(&u).axes();
                                (o, (ux.as_vec3(), uy.as_vec3(), uz.as_vec3()))
                            }
                            None => (
                                glam::DVec3::ZERO,
                                (glam::Vec3::X, glam::Vec3::Y, glam::Vec3::Z),
                            ),
                        }
                    } else {
                        model_basis
                    };
                    if is_paper {
                        let (ux, uy, uz) = super::helpers::drafting_axes(
                            axes.0.as_dvec3(),
                            axes.1.as_dvec3(),
                            axes.2.as_dvec3(),
                            self.isometric_drafting,
                            self.iso_plane,
                            self.snap_angle_deg,
                        );
                        axes = (ux.as_vec3(), uy.as_vec3(), uz.as_vec3());
                    }
                    // BUG FIX: the display step used to ignore GRIDUNIT entirely
                    // (hardcoded 1.0 base). It now resizes from the DSettings
                    // grid spacing, and honors "Display grid beyond Limits".
                    let (step_x, step_y) = crate::ui::overlay::compute_grid_steps(
                        self.grid_spacing_x,
                        self.grid_spacing_y,
                        cam.distance,
                        cam.fov_y,
                        bounds,
                        self.grid_adaptive,
                    );
                    let limits = if self.grid_beyond_limits {
                        None
                    } else {
                        tab.scene.grid_limits_for_viewport(handle)
                    };
                    crate::ui::overlay::GridParams {
                        view_rot: cam.view_proj_rte(bounds),
                        eye: cam.eye(),
                        bounds,
                        step_x,
                        step_y,
                        major_every: self.grid_major_every,
                        origin,
                        axes,
                        limits,
                    }
                })
                .collect();
            let params_ms = crate::perf::elapsed_ms(t_params);
            let t_bg = crate::perf::enabled().then(iced::time::Instant::now);
            let bg = crosshair_background(tab, is_paper);
            let bg_ms = crate::perf::elapsed_ms(t_bg);
            if views_ms + params_ms + bg_ms >= 1.0 {
                crate::perf_record!(
                    "[perf] grid-detail views={views_ms:.1}ms params={params_ms:.1}ms \
bg={bg_ms:.1}ms n={view_count}"
                );
            }
            let bg_lum = 0.299 * bg[0] + 0.587 * bg[1] + 0.114 * bg[2];
            let grid_style = crate::ui::overlay::GridStyle {
                opacity: self.model_space.grid_opacity,
                bg_luminance: bg_lum,
            };
            crate::ui::overlay::grid_overlay(grid, grid_style)
        };

        mark("selection_overlay");
        let selection_overlay = if self.layout_settling || tab.is_start {
            Space::new().into()
        } else {
            // Hold a single `Ref<'_, SelectionState>` for the whole overlay
            // block. The `vp_size` and `last_move_pos` reads below become
            // field accesses on `sel_ref` (no redundant `borrow()`s), and
            // the final widget call takes `Arc::clone(&selection)` — atomic
            // bump per frame instead of a deep `SelectionState` clone.
            // `sel_ref` is a borrowed view for field reads; the widget owns
            // the Arc clone. No `selection` mutation occurs inside this block.
            let sel_ref = tab.scene.selection.borrow();
            let snap_info = tab.snap_result.map(|s| (s.screen, s.snap_type));
            let snap_ext_base = tab.snap_result.and_then(|s| s.extension_base);
            let snap_ext_base2 = tab.snap_result.and_then(|s| s.extension_base2);

            let show_grips = tab.active_cmd.is_none()
                || tab
                    .active_cmd
                    .as_ref()
                    .is_some_and(|cmd| cmd.name() == "STRETCH");

            let grips: Vec<crate::ui::overlay::GripMarker> = if show_grips
                && !tab.selected_grips.is_empty()
            {
                let (vw, vh) = sel_ref.view.vp_size;
                // Overlays project through the active tile's camera, so
                // they must use the active tile's screen rectangle (with
                // its canvas offset) — not the whole canvas — or they
                // land in the wrong place in a tiled layout.
                // Inside a floating viewport the pane is the viewport's own
                // rect + camera; otherwise the active model tile.
                let edit_frame = tab.scene.viewport_edit_frame((vw, vh));
                let bounds = match &edit_frame {
                    Some((_, full)) => *full,
                    None => tab.scene.active_model_tile_bounds(vw, vh),
                };
                let sel_h = tab.selected_handle;
                // Mark the Properties panel's current point grip hot.
                let current_vertex_grip: Option<usize> = tab
                    .properties
                    .prop_vertex_indicator_active
                    .then(|| sel_h)
                    .flatten()
                    .and_then(|h| {
                        let indexed = match tab.scene.document.get_entity(h) {
                            Some(codec::EntityType::LwPolyline(_))
                            | Some(codec::EntityType::Polyline2D(_))
                            | Some(codec::EntityType::Polyline3D(_))
                            | Some(codec::EntityType::Spline(_))
                            | Some(codec::EntityType::Face3D(_))
                            | Some(codec::EntityType::PolygonMesh(_)) => true,
                            _ => false,
                        };
                        indexed.then_some(tab.properties.prop_vertex)
                    });
                // In-viewport grips are model-space; project them with the
                // viewport camera so they sit on the wire the GPU draws.
                // Paper entities use the 2-D paper transform; the model tab
                // uses the model camera.
                let screen_grips = if let Some((cam, _)) = &edit_frame {
                    grips_to_screen_rte(
                        &tab.selected_grips,
                        cam.view_proj_rte(bounds),
                        cam.eye(),
                        bounds,
                    )
                } else if is_paper {
                    let cam = tab.scene.camera.borrow();
                    let aspect = if vh > 0.0 { vw / vh } else { 1.0 };
                    let half_h = cam.ortho_size();
                    let half_w = half_h * aspect;
                    let tx = cam.target.x as f32;
                    let ty = cam.target.y as f32;
                    drop(cam);
                    grips_to_screen_paper(&tab.selected_grips, tx, ty, half_w, half_h, bounds)
                } else {
                    let cam = tab.scene.camera.borrow();
                    grips_to_screen(&tab.selected_grips, &cam, bounds)
                };
                screen_grips
                    .into_iter()
                    .enumerate()
                    .filter(|(_, (_, screen, _, _, _))| {
                        screen.x.is_finite()
                            && screen.y.is_finite()
                            && screen.x >= -bounds.width
                            && screen.x <= bounds.width * 2.0
                            && screen.y >= -bounds.height
                            && screen.y <= bounds.height * 2.0
                    })
                    .map(|(index, (grip_id, screen, _is_midpoint, shape, dir))| {
                        let owner = tab.selected_grip_handles.get(index).copied();
                        let is_hot = owner.is_some_and(|handle| {
                            tab.hot_grips.contains(&(handle, grip_id))
                                || tab.active_grip.as_ref().is_some_and(|edit| {
                                    edit.targets.iter().any(|target| {
                                        target.handle == handle && target.grip_id == grip_id
                                    })
                                })
                                || (Some(handle) == sel_h && Some(grip_id) == current_vertex_grip)
                        });
                        let is_hovered = owner.is_some_and(|handle| {
                            self.grip_hover.as_ref().is_some_and(|hover| {
                                hover.handle == handle && hover.grip_id == grip_id
                            })
                        });
                        crate::ui::overlay::GripMarker {
                            pos: screen,
                            shape,
                            is_hot,
                            is_hovered,
                            dir,
                        }
                    })
                    .collect()
            } else {
                vec![]
            };
            let control_polygon = tab.selected_handle.and_then(|handle| {
                let spline = match tab.scene.document.get_entity(handle) {
                    Some(codec::EntityType::Spline(spline))
                        if crate::entities::spline::shows_control_vertices(spline) =>
                    {
                        spline
                    }
                    _ => return None,
                };
                if tab
                    .selected_grip_handles
                    .iter()
                    .any(|owner| *owner != handle)
                {
                    return None;
                }
                let points: Vec<_> = grips
                    .iter()
                    .filter(|grip| grip.shape == crate::scene::model::object::GripShape::Circle)
                    .map(|grip| grip.pos)
                    .collect();
                (points.len() >= 2)
                    .then_some((points, spline.flags.closed || spline.flags.periodic))
            });
            let grip_clip = if grips.is_empty() {
                None
            } else {
                let (vw, vh) = sel_ref.view.vp_size;
                Some(
                    tab.scene
                        .viewport_edit_frame((vw, vh))
                        .map(|(_, bounds)| bounds)
                        .unwrap_or_else(|| tab.scene.active_model_tile_bounds(vw, vh)),
                )
            };

            let (vw, vh) = sel_ref.view.vp_size;
            // Active tile rectangle (canvas-offset included) so grid / UCS
            // icon / crosshair project through the active pane's camera at
            // the correct place and scale.
            let vp_bounds = tab.scene.active_model_tile_bounds(vw, vh);

            // The UCS icon shows the active pane's UCS tripod: the model view, or
            // (inside a floating viewport) projected through the viewport camera
            // at the viewport's rect so it tracks the in-viewport UCS.
            // Rotation-only projection (view_proj_rte): the icon shows axis
            // DIRECTIONS only, so the full view_proj's huge UTM translation would
            // cancel catastrophically in f32 and make the tripod jitter.
            let ucs_icons: Vec<crate::ui::overlay::UcsIconParams> = if !self.show_ucs_icon {
                vec![]
            } else if let Some((vp_cam, full)) = tab.scene.viewport_edit_frame((vw, vh)) {
                let (_, ux, uy, uz) = tab.ucs_xform().axes();
                let origin_screen = self
                    .ucs_icon_at_origin
                    .then(|| {
                        vp_cam
                            .project(tab.ucs_origin_world(), full)
                            .map(|p| iced::Point::new(full.x + p.x, full.y + p.y))
                    })
                    .flatten();
                vec![crate::ui::overlay::UcsIconParams {
                    view_proj: vp_cam.view_proj_rte(full),
                    bounds: full,
                    axes: (ux.as_vec3(), uy.as_vec3(), uz.as_vec3()),
                    origin_screen,
                    hover: self.ucs_icon_hover,
                    selected: self.ucs_icon_selected,
                }]
            } else if !is_paper {
                // One icon per Model pane — each at its own UCS origin, projected
                // through that pane's camera at its pane rect. Only the active
                // pane carries the interactive hover / selected grips.
                let (_, ux, uy, uz) = tab.ucs_xform().axes();
                let origin_w = tab.ucs_origin_world();
                let active = tab.scene.active_model_tile.get();
                let live = tab.scene.camera.borrow().clone();
                tab.scene
                    .model_tiles
                    .borrow()
                    .iter()
                    .enumerate()
                    .map(|(i, t)| {
                        let b = iced::Rectangle {
                            x: t.rect.x * vw,
                            y: t.rect.y * vh,
                            width: (t.rect.width * vw).max(1.0),
                            height: (t.rect.height * vh).max(1.0),
                        };
                        let cam = if i == active {
                            live.clone()
                        } else {
                            t.camera.clone()
                        };
                        let origin_screen = self
                            .ucs_icon_at_origin
                            .then(|| {
                                cam.project(origin_w, b)
                                    .map(|p| iced::Point::new(b.x + p.x, b.y + p.y))
                            })
                            .flatten();
                        crate::ui::overlay::UcsIconParams {
                            view_proj: cam.view_proj_rte(b),
                            bounds: b,
                            axes: (ux.as_vec3(), uy.as_vec3(), uz.as_vec3()),
                            origin_screen,
                            hover: i == active && self.ucs_icon_hover,
                            selected: i == active && self.ucs_icon_selected,
                        }
                    })
                    .collect()
            } else {
                vec![]
            };

            // OST tracking points → screen positions, projected relative-to-eye
            // so they stay precise at UTM-scale coordinates (the full
            // view-projection cancels catastrophically in f32). Must project
            // through the active pane's camera at its rect (canvas offset
            // included) — like the grips and UCS icon above — or the marker
            // lands in the wrong place (tiled layout) or off-screen (floating
            // viewport). Without the `ob.x/ob.y` offset it silently vanishes.
            // Shared OTRACK projection basis: the active pane's camera + rect
            // (canvas offset included), matching the grips / UCS icon above.
            let drafting_alignment_active = self.snapper.alignment_active()
                || (tab.active_grip.is_some() && (self.polar_mode || self.ortho_mode));

            let otrack_proj: Option<(glam::Mat4, glam::DVec3, iced::Rectangle)> =
                if drafting_alignment_active {
                    Some(
                        if let Some((vp_cam, full)) = tab.scene.viewport_edit_frame((vw, vh)) {
                            (vp_cam.view_proj_rte(full), vp_cam.eye(), full)
                        } else {
                            let cam = tab.scene.camera.borrow();
                            (cam.view_proj_rte(vp_bounds), cam.eye(), vp_bounds)
                        },
                    )
                } else {
                    None
                };
            let ost_project =
                |w: glam::DVec3, view_rot: glam::Mat4, eye: glam::DVec3, ob: iced::Rectangle| {
                    let ndc = view_rot.project_point3((w - eye).as_vec3());
                    iced::Point::new(
                        ob.x + (ndc.x + 1.0) * 0.5 * ob.width,
                        ob.y + (1.0 - ndc.y) * 0.5 * ob.height,
                    )
                };
            let ost_points: Vec<crate::ui::overlay::OstTrackPoint> =
                if let Some((view_rot, eye, ob)) = otrack_proj {
                    self.snapper
                        .tracking_points
                        .iter()
                        .filter_map(|&wp| {
                            let s = ost_project(wp, view_rot, eye, ob);
                            (s.x.is_finite() && s.y.is_finite())
                                .then_some(crate::ui::overlay::OstTrackPoint { screen: s })
                        })
                        .collect()
                } else {
                    vec![]
                };
            // The active alignment vector: a dashed guide from the acquired
            // tracking point through the locked cursor, so the user sees the
            // extension / tracking line they are snapped to (#219).
            // An intersection lock is the meeting of two tracking vectors, so
            // both are drawn — one guide alone hides what the point is (#1313).
            let otrack_lines: Vec<(iced::Point, iced::Point)> = match otrack_proj {
                Some((view_rot, eye, ob)) => {
                    let a = ost_project(tab.last_cursor_world, view_rot, eye, ob);
                    self.otrack_active
                        .into_iter()
                        .chain(self.otrack_cross)
                        .filter_map(|(base, _dir)| {
                            let b = ost_project(base, view_rot, eye, ob);
                            (b.x.is_finite()
                                && b.y.is_finite()
                                && a.x.is_finite()
                                && a.y.is_finite())
                            .then_some((b, a))
                        })
                        .collect()
                }
                None => vec![],
            };
            // The acquired Parallel-snap reference, marked on its line (#277).
            let parallel_ref_marker: Option<iced::Point> =
                match (otrack_proj, self.snapper.parallel_ref) {
                    (Some((view_rot, eye, ob)), Some((_, pt))) => {
                        let s = ost_project(pt, view_rot, eye, ob);
                        (s.x.is_finite() && s.y.is_finite()).then_some(s)
                    }
                    _ => None,
                };

            // Model-space pane dividers (none in paper / single-pane layouts).
            let dividers = if !is_paper {
                let (vw, vh) = sel_ref.view.vp_size;
                tab.scene.model_pane_dividers(vw, vh)
            } else {
                vec![]
            };

            // Pane move (drag-to-swap) visuals: source pane rect + the drop
            // target pane under the cursor.
            let (pane_move_rect, pane_drop_rect) = match self.pane_move_from {
                Some(from) if !is_paper => {
                    let (vw, vh) = sel_ref.view.vp_size;
                    let cursor = sel_ref.input.last_move_pos;
                    let tiles = tab.scene.model_tiles.borrow();
                    let px = |t: &crate::scene::ModelTile| iced::Rectangle {
                        x: t.rect.x * vw,
                        y: t.rect.y * vh,
                        width: t.rect.width * vw,
                        height: t.rect.height * vh,
                    };
                    let src = tiles.get(from).map(px);
                    let drop = cursor.and_then(|c| {
                        tiles.iter().enumerate().find(|(i, t)| {
                            *i != from
                                && c.x >= t.rect.x * vw
                                && c.x < (t.rect.x + t.rect.width) * vw
                                && c.y >= t.rect.y * vh
                                && c.y < (t.rect.y + t.rect.height) * vh
                        })
                    });
                    (src, drop.map(|(_, t)| px(t)))
                }
                _ => (None, None),
            };

            let hover_locked = tab
                .scene
                .hover_highlight
                .map(|h| tab.scene.is_layer_locked(h))
                .unwrap_or(false);
            let point_cursor = tab
                .active_cmd
                .as_ref()
                .is_some_and(|cmd| !cmd.needs_entity_pick() && !cmd.is_selection_gathering());
            let constraint_cursor_badge = if is_paper {
                None
            } else {
                tab.scene.hover_highlight.and_then(|handle| {
                    tab.scene
                        .parametric_constraint_set(tab.current_parametric_scope())
                        .and_then(|set| {
                            set.constraints_touching(handle)
                                .find(|constraint| {
                                    constraint.kind
                                        == crate::scene::parametric_constraints::ConstraintKind::Concentric
                                })
                                .or_else(|| set.constraints_touching(handle).next())
                        })
                        .map(|constraint| constraint.kind.glyph_symbol().to_string())
                })
            };
            let constraint_glyphs: std::sync::Arc<
                [crate::scene::parametric_constraints::GlyphEntry],
            > = if is_paper {
                std::sync::Arc::from([])
            } else {
                let scope = tab.current_parametric_scope();
                tab.scene.cached_glyph_placements(
                    scope,
                    sel_ref.view.vp_size,
                    self.show_constraint_values,
                    self.constraint_bar_display,
                    self.constraint_bar_mode,
                )
            };
            // Selection stays OUTSIDE the cache (applied post-hoc): a selection
            // change must not invalidate the placement memo.
            let constraint_glyph_selected: std::sync::Arc<[bool]> = constraint_glyphs
                .iter()
                .map(|entry| tab.scene.selected_constraint == Some(entry.id))
                .collect();
            crate::ui::overlay::selection_overlay(
                std::sync::Arc::clone(&tab.scene.selection),
                snap_info,
                snap_ext_base,
                snap_ext_base2,
                grips,
                control_polygon,
                grip_clip,
                ucs_icons,
                ost_points,
                otrack_lines,
                parallel_ref_marker,
                // ViewCube hover region matches the drawn cube — gone when hidden.
                !is_paper && viewcube_visible,
                dividers,
                pane_move_rect,
                pane_drop_rect,
                nav_cursor(tab),
                self.ribbon.open_dropdown.is_some(),
                hover_locked,
                crosshair_background(tab, is_paper),
                crate::ui::overlay::CrosshairOptions {
                    size_percent: self.cursor_size,
                    pick_box: self.pick_box,
                    cursor_type: self.cursor_type,
                    point_mode: point_cursor,
                    color: self.crosshair_color,
                    isometric: self.isometric_drafting,
                    iso_plane: self.iso_plane,
                    snap_angle_deg: self.snap_angle_deg,
                    pick_pending: self.pending_pick_label().is_some(),
                    // An infinite-line preview runs along the arms; drop them
                    // so the preview stays visible (#1437). A command asking
                    // for objects shows the pickbox alone, so picking reads
                    // apart from both idle and point input (#1661).
                    hide_arms: tab.active_cmd.as_ref().is_some_and(|c| {
                        matches!(c.name(), "XLINE" | "RAY")
                            || c.needs_entity_pick()
                            || c.is_selection_gathering()
                    }),
                    snap_color: self.snap_marker_color,
                },
                crate::ui::overlay::SelectionVisualOptions {
                    area: self.model_space.selection_area,
                    opacity: self.model_space.selection_opacity,
                    window_color: self.model_space.selection_window_color,
                    crossing_color: self.model_space.selection_crossing_color,
                    highlight_color: self.model_space.selection_highlight_color,
                    grip_size: self.model_space.grip_size as f32,
                    grip_color: self.model_space.grip_color,
                    grip_hot: self.model_space.grip_hot,
                    grip_hover: self.model_space.grip_hover,
                },
                constraint_glyphs,
                constraint_glyph_selected,
                self.constraint_glyph_tooltip
                    .map(|kind| crate::t!(kind.label()).into_owned()),
                constraint_cursor_badge,
            )
        };

        mark("viewport_mouse");
        let viewport_mouse = mouse_area(container(
            iced::widget::Space::new().width(Fill).height(Fill),
        ))
        .on_move(Message::ViewportMove)
        .on_press(Message::ViewportLeftPress)
        .on_release(Message::ViewportLeftRelease)
        .on_right_press(Message::ViewportRightPress)
        .on_right_release(Message::ViewportRightRelease)
        .on_middle_press(Message::ViewportMiddlePress)
        .on_middle_release(Message::ViewportMiddleRelease)
        .on_scroll(Message::ViewportScroll)
        .on_exit(Message::ViewportExit);

        let desk_bg = self.model_space.resolve_desk_bg();
        let desk_color = Color {
            r: desk_bg[0],
            g: desk_bg[1],
            b: desk_bg[2],
            a: desk_bg[3],
        };
        let bg_color = if is_paper {
            desk_color
        } else {
            let c = tab.scene.bg_color;
            Color {
                r: c[0],
                g: c[1],
                b: c[2],
                a: c[3],
            }
        };

        // Dynamic input overlay — editable boxes near the cursor, one per
        // quantity the active command is asking for (X/Y, or polar
        // distance+angle, or a single distance/angle). TAB moves focus
        // between boxes; typing locks a box to a fixed value while the
        // rest keep tracking the cursor. The field set is maintained in
        // `tab.dyn_fields` by `sync_dyn_fields`.
        // A pick step (object selection) has no input box, but still shows
        // its prompt ("Select first object …") near the cursor as a hint.
        let dyn_picks_object = tab
            .active_cmd
            .as_ref()
            .map(|c| c.needs_entity_pick() || c.needs_structure_point_pick())
            .unwrap_or(false);
        let dyn_input_overlay: Option<Element<'_, Message>> = if self.dyn_input
            && (tab.active_cmd.is_some() || tab.active_grip.is_some())
            && (!tab.dyn_fields.is_empty() || dyn_picks_object)
        {
            let w = tab.last_cursor_world;
            let base = tab.dyn_anchor.or(self.last_point);
            let label_screen = tab
                .active_cmd
                .as_ref()
                .and_then(|command| command.dyn_label_point(w))
                .and_then(|world| {
                    let (vw, vh) = tab.scene.selection.borrow().view.vp_size;
                    let (camera, bounds) =
                        tab.scene.viewport_edit_frame((vw, vh)).unwrap_or_else(|| {
                            (
                                tab.scene.camera.borrow().clone(),
                                tab.scene.active_model_tile_bounds(vw, vh),
                            )
                        });
                    camera.project(world, bounds).and_then(|point| {
                        (point.x.is_finite() && point.y.is_finite())
                            .then(|| iced::Point::new(bounds.x + point.x, bounds.y + point.y))
                    })
                });
            // A command may drive a typed scalar by mouse (e.g. a
            // perpendicular distance to a picked object); show that live
            // value in the box until the user types over it.
            let live = tab
                .active_cmd
                .as_ref()
                .and_then(|c| c.dyn_live_value(w))
                .or_else(|| {
                    let grip = tab.active_grip.as_ref()?;
                    let action = match grip.mode {
                        crate::scene::pick::grip::GripEditMode::Lengthen => {
                            crate::scene::model::object::GripMenuAction::Lengthen
                        }
                        crate::scene::pick::grip::GripEditMode::Radius => {
                            crate::scene::model::object::GripMenuAction::Radius
                        }
                        crate::scene::pick::grip::GripEditMode::ArcLength => {
                            crate::scene::model::object::GripMenuAction::ArcLength
                        }
                        crate::scene::pick::grip::GripEditMode::RectangleWidth => {
                            crate::scene::model::object::GripMenuAction::RectangleWidth
                        }
                        crate::scene::pick::grip::GripEditMode::RectangleHeight => {
                            crate::scene::model::object::GripMenuAction::RectangleHeight
                        }
                        crate::scene::pick::grip::GripEditMode::MoveParallel => {
                            crate::scene::model::object::GripMenuAction::MoveParallel
                        }
                        _ => return None,
                    };
                    let original = self
                        .grip_originals
                        .iter()
                        .find(|(handle, _)| *handle == grip.handle)
                        .map(|(_, entity)| entity)?;
                    crate::scene::view::dispatch::grip_menu_point_value(
                        original,
                        grip.grip_id,
                        action,
                        w,
                    )
                });
            let rectangle_values = tab
                .active_grip
                .as_ref()
                .and_then(|grip| grip.rectangle_frame)
                .map(|(opposite, width_axis, height_axis)| {
                    let delta = w - opposite;
                    (delta.dot(width_axis).abs(), delta.dot(height_axis).abs())
                });
            let rectangle_label_screens = tab
                .active_grip
                .as_ref()
                .and_then(|grip| grip.rectangle_frame)
                .and_then(|(opposite, width_axis, height_axis)| {
                    let delta = w - opposite;
                    let width = delta.dot(width_axis);
                    let height = delta.dot(height_axis);
                    let width_center = opposite + width_axis * (width * 0.5);
                    let height_center =
                        opposite + width_axis * width + height_axis * (height * 0.5);
                    let rectangle_center =
                        opposite + width_axis * (width * 0.5) + height_axis * (height * 0.5);
                    let (vw, vh) = tab.scene.selection.borrow().view.vp_size;
                    let (camera, bounds) =
                        tab.scene.viewport_edit_frame((vw, vh)).unwrap_or_else(|| {
                            (
                                tab.scene.camera.borrow().clone(),
                                tab.scene.active_model_tile_bounds(vw, vh),
                            )
                        });
                    let project = |point| {
                        camera.project(point, bounds).map(|screen| {
                            iced::Point::new(bounds.x + screen.x, bounds.y + screen.y)
                        })
                    };
                    let center = project(rectangle_center)?;
                    let offset_from_center = |side: iced::Point, pixels: f32| {
                        let dx = side.x - center.x;
                        let dy = side.y - center.y;
                        let length = dx.hypot(dy);
                        if length > 1.0e-3 {
                            iced::Point::new(
                                side.x + dx / length * pixels,
                                side.y + dy / length * pixels,
                            )
                        } else {
                            side
                        }
                    };
                    Some((
                        offset_from_center(project(width_center)?, 14.0),
                        offset_from_center(project(height_center)?, 18.0),
                    ))
                });
            let boxes: Vec<crate::ui::overlay::DynBox> = tab
                .dyn_fields
                .iter()
                .enumerate()
                .map(|(idx, f)| {
                    let value = match (&f.buffer, live) {
                        (Some(b), _) => b.clone(),
                        (None, _) if rectangle_values.is_some() => {
                            let (width, height) = rectangle_values.unwrap();
                            match f.role {
                                crate::command::DynRole::Width => {
                                    crate::entities::common::format_length(width)
                                }
                                crate::command::DynRole::Height => {
                                    crate::entities::common::format_length(height)
                                }
                                _ => String::new(),
                            }
                        }
                        // An angle step with a command-supplied live value
                        // (ARC span / direction) shows it in degrees.
                        (None, Some(lv)) if f.component == DynComponent::Angle => {
                            crate::entities::common::format_angle(lv.to_radians())
                        }
                        (None, Some(lv))
                            if matches!(
                                f.component,
                                DynComponent::Scalar | DynComponent::Distance
                            ) =>
                        {
                            if f.component == DynComponent::Distance {
                                crate::entities::common::format_length(lv)
                            } else {
                                format!("{lv:.4}")
                            }
                        }
                        _ => dyn_component_value(
                            f,
                            w,
                            base,
                            &tab.ucs_xform(),
                            self.dyn_user_reshaped,
                            self.dyn_coord_absolute,
                        ),
                    };
                    crate::ui::overlay::DynBox {
                        label: f.role.label().to_string(),
                        value,
                        active: idx == tab.dyn_active,
                        locked: f.locked(),
                        role: f.role,
                        center: rectangle_label_screens.and_then(|(width, height)| match f.role {
                            crate::command::DynRole::Width => Some(width),
                            crate::command::DynRole::Height => Some(height),
                            _ => None,
                        }),
                    }
                })
                .collect();
            let prompt = tab
                .active_cmd
                .as_ref()
                .map(|c| c.prompt())
                .unwrap_or_default();

            let tracking_hint = match self.otrack_kind {
                Some(crate::snap::TrackingKind::Perpendicular) => {
                    Some(crate::tr!("common", "perpendicular"))
                }
                Some(crate::snap::TrackingKind::Extension) => {
                    Some(crate::tr!("common", "extension"))
                }
                _ => None,
            };

            Some(crate::ui::overlay::dynamic_input_overlay(
                tab.last_cursor_screen,
                tab.last_point_screen,
                tab.dyn_ref_screen,
                label_screen,
                tab.dyn_guide,
                boxes,
                prompt,
                tracking_hint,
            ))
        } else {
            None
        };

        mark("viewport_stack");
        // Every one of these is a *document* plane: the fill is the drawing's own
        // background or paper desk, not a rung of the chrome ladder, because a
        // viewport painted in the UI's grey would misreport what is on screen. The
        // screen hands the colour down; the kit owns the plane it lands on.
        let mut viewport_stack = if tab.is_start || self.layout_settling {
            // Start tab: only the welcome widget over a flat background.
            // Skip every drawing-only overlay (selection markers, snap info,
            // mouse-area capturing draw clicks, viewcube, nav toolbar, …).
            stack![container(viewport_3d)
                .style(move |_| kit::document_style(bg_color, ""))
                .width(Fill)
                .height(Fill)]
            .width(Fill)
            .height(Fill)
        } else if thumbnail_capture_clean {
            let capture_bg = if is_paper { desk_color } else { bg_color };
            stack![
                container(Space::new())
                    .style(move |_| kit::document_style(capture_bg, ""))
                    .width(Fill)
                    .height(Fill),
                viewport_3d,
            ]
            .width(Fill)
            .height(Fill)
        } else if is_paper {
            // Keep the grid above the opaque GPU sheet and below interaction UI.
            stack![
                container(Space::new())
                    .style(move |_| kit::document_style(desk_color, ""))
                    .width(Fill)
                    .height(Fill),
                viewport_3d,
                grid_overlay,
                selection_overlay,
                viewport_mouse,
            ]
            .width(Fill)
            .height(Fill)
        } else {
            stack![
                container(Space::new())
                    .style(move |_| kit::document_style(bg_color, ""))
                    .width(Fill)
                    .height(Fill),
                viewport_3d,
                grid_overlay,
                selection_overlay,
            ]
            .width(Fill)
            .height(Fill)
        };

        if !thumbnail_capture_clean && !self.layout_settling {
            if let Some(pivot) = self.spacemouse_pivot_overlay() {
                viewport_stack = viewport_stack.push(pivot);
            }
            // Per-pane input pane_grid goes ABOVE the crosshair overlay so it
            // receives mouse events (the overlay's `Hidden` cursor would otherwise
            // starve any layer beneath it). The controls bar is pushed on top of it.
            if let Some(input) = model_input_layer {
                viewport_stack = viewport_stack.push(input);
            }

            // Model-space render-mode picker, top-left. Sits ABOVE the
            // viewport mouse_area so clicks inside its bounds reach it
            // instead of the shader behind it; `opaque` stops them bubbling
            // further. Outside the chip the Fill container is transparent so
            // viewport drawing / selection is unaffected. In a paper layout
            // the active viewport gets its own picker (below) instead.
            if !is_paper && !tab.is_start {
                let (vw, vh) = tab.scene.selection.borrow().view.vp_size;
                let rect = tab.scene.active_model_tile_bounds(vw, vh);
                // Unified control chip: split buttons + render-mode picker +
                // grid / grid-snap toggles, for the active Model tile.
                let bar = viewport_controls(
                    tab.render_mode,
                    self.show_grid,
                    self.snapper.grid_snap(),
                    true,
                    tab.scene.model_tiles.borrow().len(),
                    self.render_mode_menu_open,
                    self.render_mode_preview,
                    &self.active_theme,
                );
                // Adaptive: DensitySwap measures the bar's real width every frame
                // (reported into `render_bar_w`, which the ViewCube reads to decide
                // overlap) and swaps it for an empty spacer only when it no longer
                // fits the tile. The fixed-width container bounds that fit decision
                // to the tile, not the whole canvas.
                let adaptive: Element<'_, Message> = DensitySwap::new(vec![
                    iced::widget::opaque(bar),
                    Space::new()
                        .width(iced::Length::Fixed(0.0))
                        .height(iced::Length::Fixed(0.0))
                        .into(),
                ])
                .report_width0(self.render_bar_w.clone())
                .report_width0(tab.scene.model_pane_min_reporter())
                .into();
                // Pin the bar to the active model tile's top-left corner so it
                // follows the active panel in a tiled layout.
                let bar_layer = iced::widget::pin(
                    container(adaptive).width(iced::Length::Fixed(rect.width.max(1.0))),
                )
                .position(iced::Point::new(rect.x.max(0.0), rect.y.max(0.0)));
                viewport_stack = viewport_stack.push(bar_layer);
            }

            // Active paper-space viewport overlays: a render-mode picker in
            // its top-left corner and a ViewCube hit area in its top-right,
            // both layered ABOVE the viewport mouse_area so they receive
            // clicks (the shader viewport sits below it). Positioned with
            // leading Spaces sized to the viewport's screen rectangle.
            mark("active_vp_rect");
            let active_vp_rect: Option<(codec::Handle, iced::Rectangle)> =
                if is_paper && !tab.is_start {
                    tab.scene.active_viewport.and_then(|h| {
                        let (cw, ch) = tab.scene.selection.borrow().view.vp_size;
                        tab.scene
                            .viewport_screen_rect(h, (cw, ch))
                            .map(|rect| (h, rect))
                    })
                } else {
                    None
                };
            if let Some((active_vp, rect)) = active_vp_rect {
                // Clip the outline to the visible canvas. Clamping only the origin
                // (max(0.0)) while keeping the full width/height shifted the whole
                // outline inward when the viewport ran off the top/left edge, so
                // its drawn border no longer matched the real viewport — clicks
                // that looked outside landed in (and activated) another viewport.
                let (cw, ch) = tab.scene.selection.borrow().view.vp_size;
                let x = rect.x.max(0.0);
                let y = rect.y.max(0.0);
                let vw = ((rect.x + rect.width).min(cw) - x).max(1.0);
                let vh = ((rect.y + rect.height).min(ch) - y).max(1.0);
                // Ring the active viewport so its boundary stays visible over the
                // GPU shader at any zoom. The ring itself is the kit's frame rung —
                // its width, its square corner and its fade are L1's; what this
                // screen decides is only *which* contrast colour rings a live
                // viewport, and that is the palette's own accent, so the light and
                // dark schemes both keep it legible instead of one fixed blue.
                let border_frame = container(
                    Space::new()
                        .width(iced::Length::Fixed(vw))
                        .height(iced::Length::Fixed(vh)),
                )
                .style(move |theme: &Theme| {
                    kit::frame_style(theme.palette().primary.base.color, "")
                });
                let border_layer = iced::widget::pin(border_frame).position(iced::Point::new(x, y));
                viewport_stack = viewport_stack.push(border_layer);

                let vp_mode = tab
                    .scene
                    .active_viewport_render_mode()
                    .unwrap_or(codec::entities::ViewportRenderMode::Wireframe2D);
                // Adaptive (same as model): the picker measures its real width into
                // `render_bar_w` and swaps to an empty spacer only when the viewport
                // can't hold it; the ViewCube reads that width to decide overlap.
                let bar = viewport_controls(
                    vp_mode,
                    self.show_grid,
                    self.snapper.grid_snap(),
                    false,
                    0,
                    self.render_mode_menu_open,
                    self.render_mode_preview,
                    &self.active_theme,
                );
                let adaptive: Element<'_, Message> = DensitySwap::new(vec![
                    iced::widget::opaque(bar),
                    Space::new()
                        .width(iced::Length::Fixed(0.0))
                        .height(iced::Length::Fixed(0.0))
                        .into(),
                ])
                .report_width0(self.render_bar_w.clone())
                .into();
                let picker_layer = iced::widget::pin(
                    container(adaptive).width(iced::Length::Fixed(rect.width.max(1.0))),
                )
                .position(iced::Point::new(x + 4.0, y + 4.0));
                viewport_stack = viewport_stack.push(picker_layer);

                // Hide the ViewCube first — before the render bar — when they collide.
                if viewcube_visible {
                    let cube_x = (rect.x + rect.width - VIEWCUBE_HIT_SIZE - VIEWCUBE_PAD).max(0.0);
                    let cube_y = (rect.y + VIEWCUBE_PAD).max(0.0);

                    let controls = iced::widget::pin(viewcube_nav_controls(Some(active_vp)))
                        .position(iced::Point::new(cube_x, cube_y));
                    viewport_stack = viewport_stack.push(controls);

                    let ucs_current = tab
                        .active_ucs
                        .as_ref()
                        .map(|u| u.name.clone())
                        .unwrap_or_default();
                    // Epoch-memoised (#34): Arc clone per frame, rebuilt only
                    // after a UCS-table bump.
                    let ucs_names = tab.scene.cached_ucs_names();
                    let picker = iced::widget::pin(iced::widget::opaque(viewcube_ucs_picker(
                        ucs_current,
                        ucs_names,
                    )))
                    .position(iced::Point::new(
                        cube_x + VIEWCUBE_HIT_SIZE * 0.5 - UCS_PICKER_W * 0.5,
                        cube_y + VIEWCUBE_HIT_SIZE + 10.0,
                    ));
                    viewport_stack = viewport_stack.push(picker);
                }
            }

            if viewcube_visible && !is_paper {
                // Place the ViewCube hit area in the active model tile's top-right
                // corner so it tracks the active panel in a tiled layout. The hit
                // test in update.rs already maps clicks through the active tile.
                let (vw, vh) = tab.scene.selection.borrow().view.vp_size;
                let rect = tab.scene.active_model_tile_bounds(vw, vh);
                let cube_x = (rect.x + rect.width - VIEWCUBE_HIT_SIZE - VIEWCUBE_PAD).max(0.0);
                let cube_y = (rect.y + VIEWCUBE_PAD).max(0.0);

                // Cube hit area + nav controls (home / roll / nudge) as one layer.
                let controls = iced::widget::pin(viewcube_nav_controls(None))
                    .position(iced::Point::new(cube_x, cube_y));
                viewport_stack = viewport_stack.push(controls);

                // WCS / named-UCS selector under the cube.
                let ucs_current = tab
                    .active_ucs
                    .as_ref()
                    .map(|u| u.name.clone())
                    .unwrap_or_default();
                // Epoch-memoised (#34): Arc clone per frame, rebuilt only
                // after a UCS-table bump.
                let ucs_names = tab.scene.cached_ucs_names();
                let picker = iced::widget::pin(iced::widget::opaque(viewcube_ucs_picker(
                    ucs_current,
                    ucs_names,
                )))
                .position(iced::Point::new(
                    cube_x + VIEWCUBE_HIT_SIZE * 0.5 - UCS_PICKER_W * 0.5,
                    cube_y + VIEWCUBE_HIT_SIZE + 10.0,
                ));
                viewport_stack = viewport_stack.push(picker);
            }

            if let Some(dyn_ol) = dyn_input_overlay {
                if !tab.is_start {
                    viewport_stack = viewport_stack.push(dyn_ol);
                }
            }

            if let Some(popup) = self.grip_popup.as_ref() {
                if !tab.is_start {
                    let labels: Vec<String> = popup
                        .items
                        .iter()
                        .map(|item| {
                            let (mark, raw) = item
                                .label
                                .strip_prefix("✓ ")
                                .map_or(("", item.label), |raw| ("✓ ", raw));
                            format!("{mark}{}", crate::i18n::translate(raw))
                        })
                        .collect();
                    let max_len = labels
                        .iter()
                        .map(|label| label.chars().count())
                        .max()
                        .unwrap_or(8) as f32;
                    let row_w = max_len * 7.0 + 24.0;
                    let mut col = column![].spacing(tokens::GAP_NONE).width(iced::Length::Fixed(row_w));
                    for (idx, label) in labels.into_iter().enumerate() {
                        let is_sel = idx == popup.selected;
                        let btn = kit::list_row(
                            kit::body(label, ""),
                            is_sel,
                            kit::Press::On(Message::GripMenuPick(idx)),
                            "",
                        );
                        col = col.push(btn);
                    }
                    let menu_panel = kit::glass_pane(
                        container(col)
                            .padding(tokens::GAP_HAIRLINE)
                            .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
                            .into(),
                        &self.active_theme,
                        false,
                        "",
                    );
                    // Offset the menu by 12 px so the cursor doesn't land on
                    // the first item immediately, matching the right-click
                    // context menu's "panel below the click point" feel.
                    let anchor = iced::Point::new(popup.anchor.x + 12.0, popup.anchor.y + 12.0);
                    viewport_stack =
                        viewport_stack.push(position_canvas_overlay(anchor, menu_panel.into()));
                }
            }

            // Dynamic-block visibility-state dropdown.
            if let Some(popup) = self.visibility_popup.as_ref() {
                if !tab.is_start {
                    let max_len = popup
                        .items
                        .iter()
                        .map(|s| s.chars().count())
                        .max()
                        .unwrap_or(4) as f32;
                    // +2 chars for the leading "✓ " / "  " marker column.
                    let row_w = (max_len + 2.0) * 7.0 + 24.0;
                    let mut col = column![].spacing(tokens::GAP_NONE).width(iced::Length::Fixed(row_w));
                    for (idx, name) in popup.items.iter().enumerate() {
                        let is_cur = popup.current == Some(idx);
                        let mark: Element<'_, Message> = if is_cur {
                            crate::ui::icons::themed_check_cell(true)
                        } else {
                            Space::new().width(11).into()
                        };
                        let btn = kit::list_row(
                            row![container(mark).width(16), text(name).size(tokens::TEXT_BODY),]
                                .spacing(tokens::GAP_HAIRLINE)
                                .align_y(iced::Center),
                            is_cur,
                            kit::Press::On(Message::VisibilityPick(idx)),
                            "",
                        );
                        col = col.push(btn);
                    }
                    let panel = kit::glass_pane(
                        container(kit::themed_scrollable(col).height(iced::Length::Shrink))
                            .height(iced::Length::Fit.max(360.0))
                            .padding(tokens::GAP_HAIRLINE)
                            .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
                            .into(),
                        &self.active_theme,
                        false,
                        "",
                    );
                    let anchor = iced::Point::new(popup.anchor.x + 12.0, popup.anchor.y + 12.0);
                    viewport_stack =
                        viewport_stack.push(position_canvas_overlay(anchor, panel.into()));
                }
            }

            // Selection actions: with only PDF underlays or only xrefs
            // selected, their tools take the right edge (over the paper-space
            // tools, which come back when the selection changes).
            // ponytail: one right-edge toolbar at a time; stack them if both
            // are ever needed together.
            let selection_tools = if tab.is_start {
                None
            } else if self.ribbon.xref_context() {
                crate::ui::side_toolbar::view(&crate::ui::ribbon::xref_tools(), &self.active_theme)
            } else if let Some(shown) = self.ribbon.point_cloud_context() {
                crate::ui::side_toolbar::view_with_active(
                    &crate::ui::ribbon::point_cloud_tools(),
                    &move |id| id == "_PCCROPSHOW" && shown, &self.active_theme
                )
            } else if let Some(ctx) = self.ribbon.underlay_context() {
                let ctx = ctx.clone();
                crate::ui::side_toolbar::view_with_active(
                    &crate::ui::ribbon::pdf_underlay_tools(ctx.kind),
                    &move |id| match id {
                        "_PDFULMONO" => ctx.monochrome,
                        "_PDFULSHOW" => ctx.shown,
                        "_PDFULSNAP" => ctx.snap,
                        _ => false,
                    }, &self.active_theme
                )
            } else {
                None
            };
            if let Some(tb) = selection_tools {
                viewport_stack = viewport_stack.push(tb);
            } else if is_paper && !tab.is_start {
                if let Some(tb) =
                    crate::ui::side_toolbar::view(&crate::modules::layout::paper_space_tools(), &self.active_theme)
                {
                    viewport_stack = viewport_stack.push(tb);
                }
            }

            // In-place block edit (REFEDIT): right-edge toolbar with Save / Discard
            // so the edit can be finished by clicking. (#136)
            if tab.refedit_session.is_some() && !tab.is_start {
                if let Some(tb) = crate::ui::side_toolbar::view(
                    &crate::modules::draw::modify::refedit::refedit_tools(), &self.active_theme
                ) {
                    viewport_stack = viewport_stack.push(tb);
                }
            }

            // BEDIT block editor: right-edge Save Block / Discard toolbar (#261).
            if tab.active_block_edit.is_some() && !tab.is_start {
                if let Some(tb) = crate::ui::side_toolbar::view(
                    &crate::modules::draw::modify::block_edit::block_edit_tools(), &self.active_theme
                ) {
                    viewport_stack = viewport_stack.push(tb);
                }
            }

            // Count mode: a blue frame round the drawing and the count toolbar.
            if let (Some(mode), false) = (tab.count.as_ref(), tab.is_start) {
                viewport_stack = viewport_stack.push(crate::ui::window::count_palette::border());
                viewport_stack = viewport_stack.push(crate::ui::window::count_palette::toolbar(
                    mode,
                    &tab.scene.document,
                    &self.active_theme,
                ));
            }

            // Reserve the overlaid command line when placing cursor-anchored panels.
            mark("chrome");
            let command_line_inset = if self.command_line.history_open {
                self.command_line.history_height.clamp(
                    crate::ui::command_line::HISTORY_HEIGHT_MIN,
                    crate::ui::command_line::history_max_height(self.win_size.1),
                ) + 72.0
            } else {
                34.0 + self.command_line.overlay_lines_height()
            };
            // Quick Properties: stay near the selection cursor, flipping around
            // it as needed to remain inside the visible drawing area.
            if self.quick_properties && !tab.is_start {
                if let Some(panel) = tab.properties.quick_view(&self.active_theme) {
                    viewport_stack = viewport_stack.push(position_canvas_overlay_near_cursor(
                        self.quick_properties_anchor,
                        command_line_inset,
                        panel,
                    ));
                }
            }

            // Shared performance panel: terminal PERF lines plus the current
            // tessellation summary. Copy / Clear mirror the command-history panel.
            if self.perf_hud {
                let s = &tab.scene;
                let perf_w = if render_bar_w.is_finite() && render_bar_w > 1.0 {
                    render_bar_w
                } else {
                    320.0
                };
                let summary = format!(
                    "tess {:.1} ms · {} wires · epoch {}",
                    s.last_tess_ms.get(),
                    s.last_tess_wires.get(),
                    s.geometry_epoch,
                );
                let trace = crate::perf::snapshot_tail_text(80);
                let trace = if trace.is_empty() {
                    t!("No samples yet").into_owned()
                } else {
                    trace
                };
                let copy_btn = plain_button(
                    row![
                        crate::ui::icons::themed_primary(crate::ui::icons::COPY, 11.0),
                        text(t!("Copy")).size(tokens::TEXT_LABEL),
                    ]
                    .spacing(tokens::GAP_TIGHT)
                    .align_y(iced::Center),
                )
                .on_press(Message::PerfCopy)
                .style(kit::secondary_style)
                .padding(kit::PAD_GLYPH);
                let clear_btn = plain_button(
                    row![
                        crate::ui::icons::themed_danger(crate::ui::icons::TRASH, 11.0),
                        text(t!("Clear")).size(tokens::TEXT_LABEL),
                    ]
                    .spacing(tokens::GAP_TIGHT)
                    .align_y(iced::Center),
                )
                .on_press(Message::PerfClear)
                .style(kit::secondary_style)
                .padding(kit::PAD_GLYPH);
                let header = row![
                    text(t!("PERF"))
                        .size(tokens::TEXT_BODY)
                        // The readout is flush over the drawing, so its plane is the
                        // scene's own background — a colour no theme tier owns and the
                        // ladder solve cannot see.
                        .style(move |theme: &Theme| {
                            kit::tone_style_on(theme, kit::Tone::Success, bg_color)
                        }),
                    Space::new().width(iced::Length::Fill),
                    copy_btn,
                    clear_btn,
                ]
                .spacing(tokens::GAP_CONTROL)
                .align_y(iced::Center);
                let log = kit::themed_scrollable(text(trace).size(tokens::TEXT_LABEL))
                    .height(iced::Length::Fixed(220.0))
                    .width(iced::Length::Fill);
                let panel = container(
                    column![
                        header,
                        text(summary)
                            .size(tokens::TEXT_LABEL)
                            .style(move |theme: &Theme| {
                                kit::tone_style_on(theme, kit::Tone::Success, bg_color)
                            }),
                        log,
                    ]
                    .spacing(tokens::GAP_CONTROL),
                )
                .width(iced::Length::Fixed(perf_w))
                .padding(tokens::GAP_CONTROL)
                // Flush: the profiling readout paints no plane of its own, so the
                // drawing stays legible under it while the tier vocabulary still
                // owns what a corner and an edge mean.
                .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""));
                viewport_stack = viewport_stack.push(position_canvas_overlay(
                    iced::Point::new(12.0, 40.0),
                    panel.into(),
                ));
            }

            // Selection-cycling list box: pick among overlapping objects.
            if let Some((pt, cands)) = &self.cycle_candidates {
                if !tab.is_start {
                    let items: Vec<crate::ui::popup::cycle_popup::CycleCandidate> = cands
                        .iter()
                        .filter_map(|&h| {
                            tab.scene.document.get_entity(h).map(|e| {
                                let style = crate::scene::view::render::render_style_for_viewport(
                                    &tab.scene.document,
                                    e,
                                    None,
                                );
                                crate::ui::popup::cycle_popup::CycleCandidate {
                                    handle: h,
                                    type_name: crate::t!(crate::entities::traits::entity_type_name(e))
                                        .into_owned(),
                                    layer: e.common().layer.clone(),
                                    color: style.0,
                                }
                            })
                        })
                        .collect();
                    if !items.is_empty() {
                        viewport_stack = viewport_stack.push(
                            crate::ui::popup::cycle_popup::cycle_popup_overlay(
                                *pt,
                                items,
                                &self.active_theme,
                            ),
                        );
                    }
                }
            }

            // Right-click context menu. Lives inside the viewport stack so
            // the cursor position (canvas-relative) anchors the menu under
            // the cursor instead of drifting into window-relative space.
            if !tab.is_start {
                let (ctx_pos, highlighted) = {
                    let sel = tab.scene.selection.borrow();
                    (sel.menu.open_at, sel.menu.ui.highlighted)
                };
                if let Some(p) = ctx_pos {
                    let menu = self.current_context_menu();
                    viewport_stack = viewport_stack.push(viewport_context_menu_overlay(
                        p,
                        command_line_inset,
                        &menu,
                        highlighted,
                    ));
                }
            }

            // In-place MText editor (toolbar + text area), anchored at the
            // insertion-point click.
            if !tab.is_start {
                let canvas = tab.scene.selection.borrow().view.vp_size;
                if let Some(ed) = &self.mtext_editor {
                    let styles: Vec<String> = tab
                        .scene
                        .document
                        .text_styles
                        .iter()
                        .map(|s| s.name.clone())
                        .collect();
                    viewport_stack = viewport_stack.push(mtext_editor_overlay(
                        ed,
                        styles,
                        self.modal_offset,
                        self.modal_resize,
                        self.modal_content_size,
                        &self.active_theme,
                    ));
                }
                if let Some(ed) = &self.text_inline {
                    viewport_stack = viewport_stack.push(text_inline_overlay(
                        ed,
                        canvas,
                        &self.active_theme,
                    ));
                }
            }
        }

        if self.show_node_graph && !tab.is_start {
            let sections = self.graph_all_sections(self.active_tab);
            viewport_stack = viewport_stack.push(tab.graph.view(sections, &self.active_theme));
        }

        // Docked pallets: each edge's icon strip and shown group (see
        // `app::dock`).
        let left_edge = self.build_edge_stack(crate::app::config::DockSide::Left, tab);
        let right_edge = self.build_edge_stack(crate::app::config::DockSide::Right, tab);

        // Command-line sits as a bottom-centre overlay on top of the
        // viewport stack rather than as a separate row in the main
        // column — frees up vertical space when no command is active
        // and keeps the input close to where the cursor is drawing.
        // Autocomplete shows only when no command or grip-menu action is
        // collecting its own input. A pending Lengthen / Radius / Arc Length
        // value is numeric input, not the prefix of a new command (for example,
        // `3` must not open the 3DALIGN / 3DARRAY suggestions). The predicate
        // lives beside the motion sync that starts the popup's fade — the
        // update side and the paint must decide "showing" the same way.
        let allow_autocomplete = self.autocomplete_allowed();
        // Dynamic input captures keystrokes when its fields are showing,
        // so the command-line field must release focus / its on_input.
        // The MText preview also captures keystrokes (typing edits it), so the
        // command line must likewise release its on_input there.
        let interactive_value_grip = tab.active_grip.as_ref().is_some_and(|grip| {
            matches!(
                grip.mode,
                crate::scene::pick::grip::GripEditMode::Lengthen
                    | crate::scene::pick::grip::GripEditMode::Radius
                    | crate::scene::pick::grip::GripEditMode::ArcLength
                    | crate::scene::pick::grip::GripEditMode::RectangleWidth
                    | crate::scene::pick::grip::GripEditMode::RectangleHeight
                    | crate::scene::pick::grip::GripEditMode::MoveParallel
            )
        });
        let dyn_capturing = (self.dyn_input
            && (tab.active_cmd.is_some() || tab.active_grip.is_some())
            && !tab.dyn_fields.is_empty()
            && !interactive_value_grip)
            || self.mtext_editor.as_ref().is_some_and(|e| e.show_preview)
            || self.text_inline.is_some();
        // The workspace row is: left edge stack, viewport, right edge stack.
        // On the sheet the edges are cards floating on the page, so the host
        // keeps the air between a card and the window frame — the panel never
        // touches the edge on three sides. Vendor themes keep their flush
        // docks, where the strip band is meant to meet the frame.
        let sheet = crate::ui::style::draftflow_theme::draftflow_colors(&self.active_theme).is_some();
        let mut parts: Vec<Element<'_, Message>> = Vec::new();
        if let Some(e) = left_edge {
            let pad = if sheet {
                iced::Padding {
                    top: tokens::GAP_CONTROL,
                    bottom: tokens::GAP_CONTROL,
                    left: tokens::GAP_SECTION,
                    right: tokens::GAP_CONTROL,
                }
            } else {
                iced::Padding::ZERO
            };
            parts.push(iced::widget::container(e).padding(pad).into());
        }
        parts.push(
            crate::ui::wrap_bar::PosReport::new(VIEWPORT_CAPTURE_BOUNDS_ID, viewport_stack).into(),
        );
        if let Some(e) = right_edge {
            let pad = if sheet {
                iced::Padding {
                    top: tokens::GAP_CONTROL,
                    bottom: tokens::GAP_CONTROL,
                    left: tokens::GAP_CONTROL,
                    right: tokens::GAP_SECTION,
                }
            } else {
                iced::Padding::ZERO
            };
            parts.push(iced::widget::container(e).padding(pad).into());
        }
        let workspace: Element<'_, Message> = row(parts).width(Fill).height(Fill).into();

        let workspace = self.dock_overlay(workspace, tab);
        let command_line = self.command_line.view(
            allow_autocomplete,
            dyn_capturing,
            &self.history_content,
            self.win_size.0,
            self.win_size.1,
            self.control.enabled,
            self.control_busy(),
            self.pending_pick_label().is_some(),
            self.show_node_graph,
            &self.active_theme,
        );
        let center_stack: Element<'_, Message> = if thumbnail_capture_clean {
            workspace
        } else if tab.is_start {
            column![
                workspace,
                iced::widget::container(command_line)
                    .width(Fill)
                    .align_x(iced::alignment::Horizontal::Center)
                    .padding(iced::Padding {
                        top: 0.0,
                        right: 0.0,
                        bottom: tokens::GAP_SECTION,
                        left: 0.0,
                    }),
            ]
            .width(Fill)
            .height(Fill)
            .into()
        } else {
            let command_line_overlay = iced::widget::container(command_line)
                .width(Fill)
                .height(Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Bottom)
                .padding(iced::Padding {
                    top: 0.0,
                    right: 0.0,
                    bottom: tokens::GAP_SECTION,
                    left: 0.0,
                });
            // With a live selection, the bottom-right corner carries the
            // selection's own capsule group: what is selected, and the three
            // ways to change it. It floats over the drawing, clear of the
            // command card, and disappears the moment nothing is selected.
            let selected = tab.scene.selected.len();
            if selected == 0 {
                iced::widget::stack![workspace, command_line_overlay]
                    .width(Fill)
                    .height(Fill)
                    .into()
            } else {
                let pills = row![
                    kit::state_pill(
                        &self.active_theme,
                        format!("{} {} {}", t!("Selected"), selected, t!("Objects")),
                        false,
                    ),
                    kit::chip(
                        text(t!("Quick Select")).size(tokens::TEXT_LABEL),
                        false,
                        kit::Press::On(Message::Command("QSELECT".into())),
                        "",
                    ),
                    kit::chip(
                        text(t!("Select All")).size(tokens::TEXT_LABEL),
                        false,
                        kit::Press::On(Message::SelectAllShortcut),
                        "",
                    ),
                    kit::chip(
                        text(t!("Filter")).size(tokens::TEXT_LABEL),
                        false,
                        kit::Press::On(Message::ToggleSelectionFilterPopup),
                        "",
                    ),
                ]
                .spacing(tokens::GAP_CONTROL)
                .align_y(iced::Center);
                let selection_overlay = iced::widget::container(
                    kit::surface(
                        pills,
                        kit::Tier::Card,
                        iced::Padding::from([tokens::GAP_CONTROL, tokens::GAP_FIELD]),
                        "",
                    ),
                )
                .width(Fill)
                .height(Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Bottom)
                .padding(iced::Padding {
                    top: 0.0,
                    right: tokens::GAP_SECTION,
                    bottom: tokens::GAP_SECTION,
                    left: 0.0,
                });
                iced::widget::stack![workspace, command_line_overlay, selection_overlay]
                    .width(Fill)
                    .height(Fill)
                    .into()
            }
        };

        let center_stack: Element<'_, Message> = if self.command_history_resizing {
            mouse_area(center_stack)
                .on_move(Message::CommandHistoryResizeMove)
                .on_release(Message::CommandHistoryResizeRelease)
                .interaction(iced::mouse::Interaction::ResizingVertically)
                .into()
        } else {
            center_stack
        };

        let main_ui = container({
            // Clean-screen mode drops the ribbon for a full-canvas view; the
            // status bar stays so the mode can be toggled back off.
            let mut col = column![];
            if !self.clean_screen {
                // Three stacked fields: the application bar (a card floating on
                // the page), the document tab strip, and the bare tool row. The
                // host keeps the air between the card's corner and the frame so
                // the sheet never touches the edge on three sides.
                //
                // The Start page has no drawing, so it shows neither the tool
                // field nor a document strip: the disabled tools read as broken
                // chrome, and the sheet's own page is the content there. The
                // strip still appears when the home page sits beside open
                // documents, because it is the only way back to them.
                let on_start = self.tabs[self.active_tab].is_start;
                let running = self.tabs[self.active_tab]
                    .active_cmd
                    .as_ref()
                    .map(|c| c.name());
                let (app_bar, tools) = self.ribbon.chrome(
                    is_paper,
                    self.tabs[self.active_tab].is_start,
                    self.tabs[self.active_tab].history.undo_stack.len(),
                    self.tabs[self.active_tab].history.redo_stack.len(),
                    self.show_block_palette,
                    running,
                    tab.scene.document.entity_count(),
                    self.tabs.iter().position(|t| t.is_start),
                    &self.active_theme,
                );
                let bar: Element<'_, Message> = {
                    let bar = iced::widget::container(app_bar).padding(iced::Padding {
                        top: tokens::GAP_CONTROL,
                        right: tokens::GAP_SECTION,
                        bottom: 0.0,
                        left: tokens::GAP_SECTION,
                    });
                    if sheet {
                        bar.style(|theme: &Theme| kit::sheet_style(theme, "")).into()
                    } else {
                        bar.into()
                    }
                };
                col = col.push(bar);
                if self.show_file_tabs && !(on_start && self.tabs.len() == 1) {
                    col = col.push(doc_tab_bar(
                        &self.tabs,
                        self.active_tab,
                        self.hovered_doc_tab,
                        &self.active_theme,
                    ));
                }
                if !on_start {
                    col = col.push(
                        iced::widget::container(tools).width(Fill).padding(iced::Padding {
                            top: tokens::GAP_CONTROL,
                            right: tokens::GAP_SECTION,
                            bottom: 0.0,
                            left: tokens::GAP_SECTION,
                        }),
                    );
                    if !sheet {
                        col = col.push(crate::ui::ribbon::ribbon_hairline());
                    }
                }
            } else if self.show_file_tabs {
                col = col.push(doc_tab_bar(
                    &self.tabs,
                    self.active_tab,
                    self.hovered_doc_tab,
                    &self.active_theme,
                ));
            }
            // Split the `chrome` bucket so live `view-detail` traces show
            // ribbon vs status-bar construction separately (both gated on
            // PERF; zero cost otherwise).
            mark("ribbon");
            col.push(center_stack)
                .push({
                    let is_model = tab.scene.current_layout == "Model";
                    let scale_pill_enabled = is_model
                        || tab.scene.active_viewport.is_some()
                        || tab.scene.has_selected_viewport();
                    // The cursor is tracked in local render space; re-add the
                    // model-space world offset so the readout shows true
                    // drawing coordinates (paper space carries no offset), then
                    // report it in the active UCS — the readout follows the
                    // user's coordinate system, not raw WCS (no-op without UCS).
                    let to_readout = |p: glam::DVec3| {
                        // The readout follows the active pane's UCS — model space
                        // or inside a floating viewport (no-op without a UCS).
                        if tab.editing_model_space() {
                            tab.ucs_xform().to_ucs(p)
                        } else {
                            p
                        }
                    };
                    let cursor_coord = to_readout(tab.last_cursor_world);
                    // The last picked point (same UCS as the cursor) drives the
                    // static ($COORDS 0) and polar ($COORDS 2) readouts.
                    let last_coord = self.last_point.map(to_readout);
                    let coords_mode = tab.scene.document.header.coords_mode;
                    let picking = tab.active_cmd.is_some();
                    // Cached Arcs are moved (refcount bump only), never deep-cloned:
                    // `StatusBar::view` / `StatusMenuData` take ownership of the
                    // `Arc`s and borrow them internally. `current_layout` is the
                    // one true borrow — it anchors to `tab`, which outlives the
                    // call — because a call-site slice over an `Arc` temporary
                    // cannot outlive this block (E0515).
                    let layout_names = tab.scene.cached_layout_names();
                    let scale_list = tab.scene.cached_scale_picker_list();
                    let selection_types = tab.scene.entity_type_names_in_layout();
                    let current_scale_name = tab.scene.displayed_annotation_scale_name();
                    let block_tabs: Vec<String> = tab
                        .block_edits
                        .iter()
                        .map(|session| session.block_name.clone())
                        .collect();
                    let active_block = tab
                        .active_block_edit_session()
                        .map(|session| session.block_name.clone());
                    // The coordinate readout formats through the same helper
                    // the properties panel uses, so it has to see the drawing's
                    // linear-unit settings. Properties happens to set this on
                    // every refresh; the status bar redraws on its own schedule
                    // and cannot rely on that having happened.
                    crate::entities::common::set_unit_context(
                        crate::entities::common::UnitContext::from_header(
                            &tab.scene.document.header,
                        ),
                    );
                    let status_menu_data = crate::ui::statusbar::StatusMenuData {
                        layout_names: Arc::clone(&layout_names),
                        polar_custom_input: &self.polar_custom_input,
                        scale_is_model: is_model,
                        current_scale_name,
                        scale_list,
                        has_selection: !tab.scene.selected.is_empty(),
                        selection_types,
                        selection_filter: &tab.scene.selection_filter,
                        tooltip_hidden: self.status_menu_tooltip_hidden,
                    };
                    let lights = self.status_lights();
                    self.status_bar.view(
                        &self.snapper,
                        &lights,
                        self.polar_increment_deg,
                        self.isometric_drafting,
                        self.iso_plane,
                        layout_names,
                        block_tabs,
                        &tab.scene.current_layout,
                        active_block,
                        tab.is_start,
                        self.layout_rename_state.as_ref(),
                        tab.scene.first_viewport_scale(),
                        tab.scene.viewport_count(),
                        self.show_layout_tabs,
                        tab.scene.annotation_scale,
                        scale_pill_enabled,
                        cursor_coord,
                        coords_mode,
                        last_coord,
                        picking,
                        tab.scene.document.header.linear_unit_format,
                        &self.statusbar_config,
                        status_menu_data,
                        tab.scene
                            .parametric_constraint_set(tab.current_parametric_scope())
                            .and_then(|s| s.dof),
                        tab.scene
                            .parametric_constraint_set(tab.current_parametric_scope())
                            .map(|s| s.conflicts.len())
                            .unwrap_or(0),
                        &self.gpu_status,
                        (self.spacemouse.visible()
                            || self.spacemouse_preferences.mode
                                != crate::input::spacemouse::NavigationMode::Auto)
                            .then(|| {
                                crate::ui::statusbar::spacemouse::view(
                                    self.spacemouse_preferences,
                                    self.spacemouse.status(),
                                    self.spacemouse_paused,
                                    self.spacemouse_label(),
                                    self.spacemouse_sheet(),
                                )
                            }),
                        &self.active_theme,
                    )
                })
                .width(Fill)
                .height(Fill)
        })
        // Root surface cross-fades on theme switch: SetTheme captured the
        // outgoing background and the track eases the blend over NORMAL ms. The
        // outgoing colour is *data* (it is whatever the previous theme was), so
        // the screen hands it to the kit and the kit still owns the paint.
        .style({
            let fade_from = self.theme_fade_from;
            move |theme: &Theme| kit::page_style(fade_from, theme, "theme:fade")
        })
        .width(Fill)
        .height(Fill);
        mark("statusbar");

        // History labels are built only when their dropdown is actually open;
        // otherwise an empty slice short-circuits the overlay gate
        // (`dropdown_overlay` returns `None` for empty labels anyway).
        let open_dropdown = self.ribbon.open_dropdown.as_deref();
        let undo_labels: Vec<String> = if open_dropdown == Some(crate::ui::ribbon::UNDO_HISTORY_ID)
        {
            history_dropdown_labels(&self.tabs[self.active_tab].history.undo_stack)
        } else {
            Vec::new()
        };
        let redo_labels: Vec<String> = if open_dropdown == Some(crate::ui::ribbon::REDO_HISTORY_ID)
        {
            history_dropdown_labels(&self.tabs[self.active_tab].history.redo_stack)
        } else {
            Vec::new()
        };
        // From here up the composition is layers over the workspace. Tab
        // cycles the highest populated layer, so an open dropdown takes the
        // key before the chrome behind it can.
        crate::ui::focus::set_scope(crate::ui::focus::Scope::Overlay);
        let gallery_open = open_dropdown == Some(crate::modules::insert::insert_block::GALLERY_ID)
            && !self.tabs[self.active_tab].is_start;
        let dropdown_layer: Element<'_, Message> = if gallery_open {
            self.ribbon.place_dropdown(
                crate::modules::insert::insert_block::GALLERY_ID,
                crate::ui::window::block_palette::gallery(&self.block_palette, &self.active_theme),
                crate::ui::window::block_palette::GALLERY_W,
                self.win_size.0,
            )
        } else {
            self.ribbon
                .dropdown_overlay(
                    &undo_labels,
                    &redo_labels,
                    self.win_size,
                    self.tabs[self.active_tab].is_start,
                    &self.recent_colors,
                    &self.active_theme,
                )
                .unwrap_or_else(|| iced::widget::Space::new().width(0).height(0).into())
        };

        let snap_override_layer: Element<'_, Message> = if let Some(pos) = self.snap_override_popup
        {
            overlay::snap_override_overlay(pos, &self.active_theme)
        } else {
            iced::widget::Space::new().width(0).height(0).into()
        };

        let qselect_layer: Element<'_, Message> = if let Some(state) = &self.qselect {
            qselect_overlay(
                state,
                &state.available_types,
                &state.available_properties,
                state.candidate_count,
                self.modal_offset,
                self.modal_resize,
                &self.active_theme,
            )
        } else {
            iced::widget::Space::new().width(0).height(0).into()
        };

        let open_progress_layer: Element<'_, Message> = if let Some(p) = self
            .opening
            .as_ref()
            .filter(|progress| progress.recovery_error.is_none())
        {
            crate::ui::window::open_progress::view(p, iced::time::Instant::now())
        } else {
            iced::widget::Space::new().width(0).height(0).into()
        };

        let composed = stack![
            main_ui,
            dropdown_layer,
            qselect_layer,
            snap_override_layer,
            open_progress_layer,
        ];

        // If the Plot Style editor was launched from PLOT, keep the Plot dialog
        // rendered underneath as its blocked parent.
        let modal_underlay: Element<'_, Message> = if self.active_modal
            == Some(super::ModalKind::Plotstyle)
        {
            if let Some((plot_offset, plot_resize)) = self.plotstyle_parent_plot_geometry.as_ref() {
                let plot_content = self.plot_modal_content(*plot_resize);

                crate::ui::shell::modal(
                    composed,
                    crate::tr!("modal", "plot"),
                    plot_content,
                    Message::CloseModal,
                    *plot_offset,
                    crate::ui::shell::ModalOptions::STANDARD,
                    None,
                    &self.active_theme,
                )
            } else {
                composed.into()
            }
        } else {
            composed.into()
        };

        // ── In-canvas modal dialogs (Plan B) ───────────────────────────────
        // Former pop-up windows render as overlays here, so they work on both
        // the native (single main window) and web builds.
        // A modal traps the cycle: while one is up, Tab reaches only its own
        // controls. With none open this scope stays empty and the key falls
        // back to the overlay, then the chrome.
        crate::ui::focus::set_scope(crate::ui::focus::Scope::Modal);
        let base: Element<'_, Message> = match self.modal_content() {
            Some(content) => {
                let modal_options = match self.active_modal {
                    Some(super::ModalKind::AttributeEditor)
                    | Some(super::ModalKind::Unsaved)
                    | Some(super::ModalKind::LayerDeleteWarning) => {
                        crate::ui::shell::ModalOptions::BARE
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    Some(super::ModalKind::FileInUse) => crate::ui::shell::ModalOptions::BARE,
                    _ => crate::ui::shell::ModalOptions::STANDARD,
                };
                crate::ui::shell::modal(
                    modal_underlay,
                    self.modal_title(),
                    content,
                    Message::CloseModal,
                    self.modal_offset + self.modal_entrance_offset(),
                    modal_options,
                    self.modal_footer(),
                    &self.active_theme,
                )
            }
            None => modal_underlay,
        };
        // Shared CAD colour picker. Indexed ACI colours use DraftFlowCAD's own
        // dialog; True Color keeps the existing iced_aw gradient picker.
        // Which part of the widget tree the frame went into. Reported as a
        // breakdown rather than one number, because a first switch to a paper
        // layout spends seconds here and `view` alone cannot say where.
        if let Some(t0) = view_t0 {
            mark("end");
            let total = t0.elapsed().as_secs_f64() * 1000.0;
            if total >= 1.0 {
                let marks = view_marks.borrow();
                // A mark opens a section; the following mark closes it, so a
                // section's cost is the gap to its successor. The first mark
                // is preceded by the preamble, reported as `head`.
                let mut detail = String::new();
                if let Some((_, first)) = marks.first() {
                    let _ =
                        std::fmt::Write::write_fmt(&mut detail, format_args!(" head={first:.1}"));
                }
                for pair in marks.windows(2) {
                    let _ = std::fmt::Write::write_fmt(
                        &mut detail,
                        format_args!(" {}={:.1}", pair[0].0, pair[1].1 - pair[0].1),
                    );
                }
                crate::perf_record!("[perf] view-detail total={total:.1}ms{detail}");
            }
        }

        if let Some((_, current)) = self.color_pick_target.as_ref() {
            match self.color_picker_tab {
                super::ColorPickerTab::Index => {
                    let content =
                        crate::ui::color_select::index_color_page(*current, &self.recent_colors);

                    crate::ui::shell::modal(
                        base,
                        t!("Select Color"),
                        content,
                        Message::CloseColorPicker,
                        self.modal_offset,
                        crate::ui::shell::ModalOptions::NOTICE,
                        None,
                        &self.active_theme,
                    )
                }

                super::ColorPickerTab::TrueColor => {
                    let initial = crate::ui::properties::acad_color_display(*current).0;
                    let modal_base = crate::ui::shell::backdrop(base, Message::CloseColorPicker);

                    iced_aw::ColorPicker::new(
                        true,
                        initial,
                        modal_base,
                        // Cancel inside True Color returns to the indexed page instead
                        // of closing the whole CAD colour picker.
                        Message::ColorPickerTabChanged(super::ColorPickerTab::Index),
                        |color| {
                            Message::ColorWindowPick(crate::ui::color_select::iced_to_acad_color(
                                color,
                            ))
                        },
                    )
                    .into()
                }
            }
        } else {
            base
        }
    }

    /// How far the open dialog still is from its resting place.
    ///
    /// The entrance stores the direction it came from; the fade track's
    /// progress carries it home, so the displacement and the fade always
    /// finish on the same frame and a swapped dialog (track pinned at 1.0)
    /// contributes nothing.
    fn modal_entrance_offset(&self) -> iced::Vector {
        if self.modal_entrance == iced::Vector::ZERO {
            return iced::Vector::ZERO;
        }
        let left = 1.0
            - crate::ui::motion::value(crate::ui::motion::MODAL_FADE).clamp(0.0, 1.0);
        self.modal_entrance * left
    }

    /// Measured outer pixel bounds for whichever shared modal is visible, used
    /// to keep its frame on-screen while dragging.
    pub(crate) fn modal_outer_size(&self) -> Option<(f32, f32)> {
        let size = self.modal_content_size?;
        // Frame padding is 10 px per side; title + body spacing is 30 px.
        Some((size.width + 20.0, size.height + 50.0))
    }
}

impl DraftFlowCAD {
    pub fn subscription(&self) -> Subscription<Message> {
        use iced::event;
        // Only request per-frame ticks while something on screen is animating
        // (currently just the open-progress indicator). Without this gate the
        // app burned 2-3% CPU continuously redrawing an unchanged view.
        // See #18.
        let needs_frames = self
            .opening
            .as_ref()
            .is_some_and(|progress| progress.recovery_error.is_none());
        let frames = if needs_frames {
            window::frames().map(Message::Tick)
        } else {
            Subscription::none()
        };
        // Chrome micro-motion: only request frames while a motion track is
        // running; the registry itself holds wall-clock progress, so this
        // subscription's sole job is redrawing (see `ui::motion`).
        let motion_frames = if crate::ui::motion::any_active() {
            window::frames().map(Message::MotionTick)
        } else {
            Subscription::none()
        };
        // Boot splash: one frame per vsync for the length of the entrance and
        // fade, then the subscription disappears with `splash`.
        #[cfg(not(target_arch = "wasm32"))]
        let splash_frames = if self.splash.is_some() {
            window::frames().map(Message::SplashTick)
        } else {
            Subscription::none()
        };
        // While the command-line overlay is still displaying any
        // recently-pushed history entry that is expiring, re-render every frame so the
        // entry disappears at the moment its visible window expires.
        // The subscription auto-stops once no entry is expiring.
        let history_tick = if self.command_line.has_expiring_history() {
            window::frames().map(Message::Tick)
        } else {
            Subscription::none()
        };
        // While the cursor sits over a grip, request animation frames
        // so the multi-functional popup opens even when the user keeps
        // the mouse perfectly still — `ViewportMove` alone would never
        // fire again. Auto-stops once the hover clears or the popup is
        // already open.
        // Resume scene construction after the notice redraw.
        let layout_settle = if self.layout_settling {
            window::frames().map(|_| Message::LayoutSettled)
        } else {
            Subscription::none()
        };
        let grip_dwell = if self.grip_hover.is_some() && self.grip_popup.is_none() {
            window::frames().map(|_| Message::GripDwellTick)
        } else {
            Subscription::none()
        };
        // While a rollover pick is queued, drive ticks so it fires the
        // moment the cursor has been still for the dwell window — without
        // this `ViewportMove` alone never re-fires once the user stops.
        let hover_dwell = if self.hover_dwell.is_some() {
            window::frames().map(|_| Message::HoverDwellTick)
        } else {
            Subscription::none()
        };
        // Interaction LOD: while the view is (or just was) navigating, keep
        // requesting frames a little past the settle point. Panning itself is
        // driven by input events, but once the cursor stops no event would fire
        // the one full-quality frame that re-renders hatches — this tick does,
        // then the scene-render cache holds it and the subscription auto-stops.
        // Underlay rasters follow the zoom once it settles; tick until then.
        let nav_settle = if self.tabs[self.active_tab].scene.is_settling()
            || self.tabs[self.active_tab].scene.underlay_resolution_stale()
        {
            window::frames().map(Message::Tick)
        } else {
            Subscription::none()
        };
        // Graphics verdict: a device appears inside the first frame and a
        // dropped scene shows up as draws without one, but neither arrives
        // as a message. Tick until the verdict is in — one frame on a
        // working GPU, three on none — then stop. The Start page draws no
        // viewport, so there is nothing to learn there.
        let gpu_probe = if matches!(self.gpu_status, crate::scene::pipeline::GpuStatus::Unknown)
            && !self.tabs[self.active_tab].is_start
        {
            window::frames().map(Message::Tick)
        } else {
            Subscription::none()
        };
        let thumbnail_capture = if self.thumbnail_capture_clean {
            window::frames().map(|_| Message::ThumbnailCaptureFrame)
        } else {
            Subscription::none()
        };
        // C11: park a `capture` request until the window draws another frame;
        // a timer fallback prevents a request from hanging when no redraw is
        // pending.
        let capture_frame = if self.control.capture_frame_pending {
            Subscription::batch([
                window::frames().map(|_| Message::ControlCaptureFrame { redrew: true }),
                iced::time::every(std::time::Duration::from_millis(50))
                    .map(|_| Message::ControlCaptureFrame { redrew: false }),
            ])
        } else {
            Subscription::none()
        };
        // Blink the MText preview caret while the editor is open.
        let caret_blink = if self.mtext_editor.is_some() {
            iced::time::every(std::time::Duration::from_millis(530))
                .map(|_| Message::MTextCaretBlink)
        } else {
            Subscription::none()
        };
        // Web: poll for per-script fonts that a drawing's text needs but hasn't
        // fetched yet. Cheap — `PollWebFonts` is a no-op when nothing is
        // pending. Native has system fonts, so no polling. (#141)
        #[cfg(target_arch = "wasm32")]
        let web_fonts =
            iced::time::every(std::time::Duration::from_millis(300)).map(|_| Message::PollWebFonts);
        #[cfg(not(target_arch = "wasm32"))]
        let web_fonts = Subscription::none();
        // Periodic autosave (SAVETIME): a `.sv$` recovery copy beside the file
        // on native, an OPFS snapshot on web. SAVETIME is the interval in
        // minutes; 0 disables it.
        let autosave = if self.savetime_min > 0 {
            iced::time::every(std::time::Duration::from_secs(
                self.savetime_min as u64 * 60,
            ))
            .map(|_| Message::AutoSave)
        } else {
            Subscription::none()
        };
        // Drawings handed over by a second launch (single instance). Inert in a
        // process that lost the port election, so it costs nothing there.
        #[cfg(not(target_arch = "wasm32"))]
        let single_instance = crate::io::single_instance::subscribe().map(Message::OpenExternal);
        #[cfg(target_arch = "wasm32")]
        let single_instance = Subscription::none();
        // While a sheet set is open: it reloads when its .dst changes on disk,
        // and its sheets' .dwl lock files follow the open tabs.
        #[cfg(not(target_arch = "wasm32"))]
        let sheet_set_poll = if self.sheet_set.sets.is_empty() {
            Subscription::none()
        } else {
            iced::time::every(std::time::Duration::from_secs(1))
                .map(|_| Message::SheetSet(crate::ui::window::sheet_set::SheetSetMsg::Poll))
        };
        #[cfg(target_arch = "wasm32")]
        let sheet_set_poll = Subscription::none();
        let hatch_pattern_keys = if self.tabs[self.active_tab]
            .properties
            .hatch_pattern_picker_open
        {
            event::listen_with(hatch_pattern_key_event)
        } else {
            Subscription::none()
        };
        // While a shortcut-editor Key cell is armed for capture, a dedicated
        // listener records the pressed combination and swallows every other
        // key so captured shortcuts do not run in the drawing.
        let keyboard_events = if self.shortcut_capture_row.is_some() {
            event::listen_with(shortcut_capture_key_event)
        } else {
            event::listen_with(|ev, status, win_id| {
                use iced::event::Status;
                match ev {
                    iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                        iced::mouse::Button::Left,
                    )) => Some(Message::PropPointerPressed),
                    iced::Event::Window(window::Event::CloseRequested) => {
                        Some(Message::WindowCloseRequested(win_id))
                    }
                    iced::Event::Window(window::Event::Closed) => {
                        Some(Message::OsWindowClosed(win_id))
                    }
                    iced::Event::Window(window::Event::Resized(sz)) => {
                        Some(Message::WindowResized(sz.width as f32, sz.height as f32))
                    }
                    iced::Event::Window(window::Event::FileDropped(path)) => {
                        Some(Message::FileDropped(path))
                    }
                    iced::Event::Keyboard(keyboard::Event::ModifiersChanged(m)) => {
                        // `command()` is Cmd on macOS, Ctrl elsewhere — the
                        // platform multi-select modifier for the layer list.
                        Some(Message::SetModifiers {
                            shift: m.shift(),
                            ctrl: m.command() || m.control(),
                        })
                    }
                    iced::Event::Keyboard(keyboard::Event::KeyPressed {
                        key,
                        physical_key,
                        modifiers,
                        text,
                        ..
                    }) => {
                        #[cfg(target_arch = "wasm32")]
                        let accel = modifiers.command();
                        // AltGr reaches Windows apps as Ctrl+Alt, and a macOS
                        // layout types some characters with Option: when such a
                        // chord produces a printable glyph (`@`, `<`, `{` on many
                        // European layouts) it is typing, not a shortcut. (#1236)
                        let types_glyph = text.as_deref().is_some_and(|value| {
                            !value.is_empty()
                                && value
                                    .chars()
                                    .all(|ch| !ch.is_control() && !ch.is_whitespace())
                        });
                        let altgr = modifiers.control() && modifiers.alt();
                        let mac_option = cfg!(target_os = "macos")
                            && modifiers.alt()
                            && !modifiers.control()
                            && !modifiers.logo();
                        let shortcut_modifier =
                            (modifiers.control() || modifiers.alt() || modifiers.logo())
                                && !(types_glyph && (altgr || mac_option));
                        // Any key that produces a printable glyph types it,
                        // even when its logical key resolves to navigation
                        // (NumLock-on Numpad8 / Numpad2 arrive as
                        // ArrowUp / ArrowDown but still carry text "8" /
                        // "2"). Checked before the Arrow / history arms so
                        // those numpad digits aren't swallowed as history
                        // navigation. Whitespace / control text (Space,
                        // Enter, Tab) falls through to the named handlers.
                        // The numpad decimal key emits the OS-layout separator
                        // (a comma on German/European layouts), which the
                        // coordinate parser rejects. Force it to a decimal point
                        // from the physical key, independent of layout.
                        if !shortcut_modifier
                            && status == Status::Ignored
                            && matches!(
                                physical_key,
                                keyboard::key::Physical::Code(
                                    keyboard::key::Code::NumpadDecimal
                                        | keyboard::key::Code::NumpadComma
                                )
                            )
                        {
                            return Some(Message::CommandAppendChar(".".to_string()));
                        }
                        if !shortcut_modifier && status == Status::Ignored {
                            if let Some(t) = text.as_deref() {
                                if !t.is_empty()
                                    && t.chars().all(|c| !c.is_control() && !c.is_whitespace())
                                {
                                    return Some(Message::CommandAppendChar(t.to_string()));
                                }
                            }
                        }
                        let has_printable_text = text.as_deref().is_some_and(|value| {
                            !value.is_empty()
                                && value
                                    .chars()
                                    .all(|ch| !ch.is_control() && !ch.is_whitespace())
                        });
                        let arrow = match &key {
                            keyboard::Key::Named(keyboard::key::Named::ArrowUp) => {
                                Some(ArrowKey::Up)
                            }
                            keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
                                Some(ArrowKey::Down)
                            }
                            keyboard::Key::Named(keyboard::key::Named::ArrowLeft) => {
                                Some(ArrowKey::Left)
                            }
                            keyboard::Key::Named(keyboard::key::Named::ArrowRight) => {
                                Some(ArrowKey::Right)
                            }
                            _ => None,
                        };
                        if !has_printable_text && status == Status::Ignored {
                            if let Some(direction) = arrow {
                                return Some(Message::ArrowKeyPressed {
                                    direction,
                                    shortcut: shortcut_key_name(&key, modifiers)?,
                                    extend_selection: modifiers.shift(),
                                });
                            }
                        }
                        if !has_printable_text
                            && status == Status::Captured
                            && matches!(arrow, Some(ArrowKey::Up | ArrowKey::Down))
                        {
                            return Some(Message::CommandLineArrowProbe {
                                direction: arrow?,
                                extend_selection: modifiers.shift(),
                            });
                        }
                        // A focused web text field needs the browser clipboard;
                        // drawing shortcuts only run for ignored C/V events.
                        #[cfg(target_arch = "wasm32")]
                        if accel && status == Status::Captured {
                            if let keyboard::Key::Character(value) = &key {
                                if value.eq_ignore_ascii_case("v") {
                                    return Some(Message::WebFieldPaste);
                                }
                                if value.eq_ignore_ascii_case("c") {
                                    return Some(Message::WebFieldCopy);
                                }
                            }
                        }
                        let shortcut = shortcut_key_name(&key, modifiers)?;
                        if status == Status::Ignored
                            || crate::app::shortcuts::is_global_key(&shortcut)
                        {
                            return Some(Message::ShortcutPressed(shortcut));
                        }
                        // A text field captured the key. SPACE rebound away from
                        // COMMANDSPACE still fires from the command line, but
                        // only there: other fields keep typing spaces. This
                        // closure is a plain fn, so the binding is read from a
                        // flag and the focus is asked for in update. (#1710)
                        (shortcut == "SPACE"
                            && crate::app::shortcuts::SPACE_REBOUND_OFF_COMMANDSPACE
                                .load(std::sync::atomic::Ordering::Relaxed))
                        .then_some(Message::CommandLineSpaceProbe)
                    }
                    _ => None,
                }
            })
        };
        #[cfg(not(target_arch = "wasm32"))]
        let control = iced::Subscription::batch([
            super::control::subscribe().map(Message::ControlRequest),
            // Loopback REST channel when launched with files + --http.
            super::control::http_bridge::subscribe().map(Message::ControlRequest),
        ]);
        #[cfg(target_arch = "wasm32")]
        let control = iced::time::every(std::time::Duration::from_millis(50))
            .map(|_| Message::PollWebControl);
        iced::Subscription::batch([
            if self.spacemouse.moving() && self.spacemouse_focused && !self.spacemouse_paused {
                window::frames().map(Message::SpaceMouseFrame)
            } else {
                Subscription::none()
            },
            self.spacemouse
                .subscription()
                .map(|_| Message::SpaceMouseWake),
            crate::input::trackpad::subscription().map(Message::TrackpadPinch),
            event::listen_with(|event, _, id| match event {
                iced::Event::Window(window::Event::Focused) => {
                    Some(Message::SpaceMouseFocus(id, true))
                }
                iced::Event::Window(window::Event::Unfocused) => {
                    Some(Message::SpaceMouseFocus(id, false))
                }
                _ => None,
            }),
            control,
            // Passive: records where the pointer is so a dialog opened from a
            // keyboard shortcut can still enter from the last thing used.
            event::listen_with(|event, _, _| {
                crate::ui::pointer::note_event(&event);
                None
            }),
            frames,
            motion_frames,
            #[cfg(not(target_arch = "wasm32"))]
            splash_frames,
            history_tick,
            layout_settle,
            grip_dwell,
            hover_dwell,
            nav_settle,
            gpu_probe,
            thumbnail_capture,
            capture_frame,
            caret_blink,
            web_fonts,
            autosave,
            sheet_set_poll,
            single_instance,
            hatch_pattern_keys,
            keyboard_events,
        ])
    }

    pub(super) fn focus_cmd_input(&self) -> Task<Message> {
        iced::widget::operation::focus(iced::widget::Id::new(crate::ui::command_line::CMD_INPUT_ID))
    }

    pub(super) fn unfocus_widgets(&self) -> Task<Message> {
        iced::advanced::widget::operate(iced::advanced::widget::operation::focusable::unfocus())
    }
}

// ── Document tab bar ───────────────────────────────────────────────────────

/// Right-click actions for a drawing tab. `ContextMenu` owns opening,
/// cursor-relative placement, boundary clamping, and dismissal.
fn doc_tab_context_menu(
    tab_idx: usize,
    has_current_path: bool,
    has_other_drawings: bool,
    theme: &Theme,
) -> Element<'static, Message> {
    const MENU_W: f32 = 210.0;

    let item = |label: &'static str, msg: Option<Message>| -> Element<'static, Message> {
        // A menu row is an unframed pressable that reports its own availability.
        // `Press::Off` *is* the disabled state, so the kit dims the ink and clears
        // the lift instead of this row hand-picking a disabled alpha — and the
        // enabled row gains a hover face that a text colour alone could not give
        // it, which is what tells the pointer the row is reachable.
        kit::btn(
            text(t!(label)).size(tokens::TEXT_BODY),
            kit::Role::Ghost,
            kit::Size::Compact,
            match msg {
                Some(msg) => kit::Press::On(msg),
                None => kit::Press::Off,
            },
            "",
        )
        .width(Fill)
        .into()
    };

    let mut menu = column![
        item("Save All", Some(Message::DocTabSaveAll)),
        item("Close All", Some(Message::DocTabCloseAll)),
        item(
            "Close All Other Drawings",
            has_other_drawings.then_some(Message::DocTabCloseOthers(tab_idx)),
        ),
    ];
    if cfg!(not(target_arch = "wasm32")) {
        menu = menu
            .push(item(
                "Copy Full File Path",
                has_current_path.then_some(Message::DocTabCopyFullPath(tab_idx)),
            ))
            .push(item(
                "Open File Location",
                has_current_path.then_some(Message::DocTabOpenFileLocation(tab_idx)),
            ));
    }

    // A menu floats above the document, so it reads the frame behind it: the
    // same glass the grip and visibility popups in this very screen wear. The
    // host keeps its inset and width and paints nothing itself.
    kit::glass_pane(
        container(menu.spacing(tokens::GAP_NONE).width(MENU_W))
            .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
            .padding([tokens::GAP_TIGHT, tokens::GAP_NONE])
            .width(iced::Length::Fixed(MENU_W))
            .into(),
        theme,
        false,
        "",
    )
}

pub(super) fn doc_tab_bar<'a>(
    tabs: &'a [DocumentTab],
    active_tab: usize,
    hovered_tab: Option<usize>,
    theme: &'a Theme,
) -> Element<'a, Message> {
    // Document tabs live in a flex-wrap flow so they spill onto lower rows when
    // there are more tabs than the width can hold on one line.
    let sheet = crate::ui::style::draftflow_theme::draftflow_colors(theme).is_some();
    let mut items: Vec<Element<'_, Message>> = Vec::new();
    let drag_targets: std::sync::Arc<[usize]> = tabs
        .iter()
        .enumerate()
        .filter_map(|(idx, tab)| (!tab.is_start).then_some(idx))
        .collect::<Vec<_>>()
        .into();

    for (idx, tab) in tabs.iter().enumerate() {
        // On the sheet the start page lives in the ribbon's tab row (see
        // `ribbon::chrome`), so it does not repeat here among the documents.
        if sheet && tab.is_start {
            continue;
        }
        let is_active = idx == active_tab;
        let is_hovered = hovered_tab == Some(idx);
        let full_name = tab.tab_display_name();
        // The tab's ink is its state and nothing else: the active document speaks
        // in the body voice, every other tab in the muted one. A tab wears no
        // plate of its own — the depth comes from the cell tier underneath it —
        // so the pressable here is a `Ghost`: a label that happens to work.
        // A shortened name keeps its whole self one hover away.
        let tab_name: Element<'_, Message> =
            crate::ui::text_util::elided(&full_name, 24, theme, |shown| {
                if is_active {
                    kit::body(shown, "")
                } else {
                    kit::muted(shown, "")
                }
            });
        let title_inner: Element<'_, Message> = if tab.dirty {
            row![
                crate::ui::icons::themed_warning(crate::ui::icons::DIRTY_DOT, 14.0),
                tab_name,
            ]
            .spacing(tokens::GAP_CONTROL)
            .align_y(iced::Center)
            .into()
        } else {
            tab_name
        };

        let title_btn = kit::btn(
            title_inner,
            kit::Role::Ghost,
            kit::Size::Control,
            kit::Press::On(Message::TabSwitch(idx)),
            "",
        )
        .height(Fill);
        let title_btn: Element<'_, Message> = if tab.is_start {
            title_btn.into()
        } else {
            crate::ui::wrap_bar::ReorderTab::document(idx, drag_targets.clone(), title_btn).into()
        };

        // Start tab is fixed — no close button. Every other tab gets a close.
        let row_inner: Row<'_, Message> = if tab.is_start {
            row![title_btn].spacing(tokens::GAP_NONE).align_y(iced::Center)
        } else {
            // Closing a drawing destroys work, but a framed red plate beside
            // every open tab would make the strip read as an alarm: `QuietDanger`
            // rests as an unframed glyph and arms into the danger ramp only when
            // the pointer is already on it.
            let close_btn = kit::btn(
                text("×").size(tokens::TEXT_BODY),
                kit::Role::QuietDanger,
                kit::Size::Control,
                kit::Press::On(Message::TabClose(tab.id)),
                "",
            )
            .height(Fill);
            row![title_btn, close_btn]
                .spacing(tokens::GAP_NONE)
                .height(Fill)
                .align_y(iced::Center)
        };

        // Depth is the whole tab story, so the cell states name a rung of the
        // surface ladder instead of a colour triple: the active document is a
        // card lifted off the strip, a tab under the pointer is a plate, and a
        // resting tab is the strip itself — it paints no plane, so the band
        // shows through it and a full row of tabs never shouts.
        let tab_tier = if is_active {
            kit::Tier::Card
        } else if is_hovered {
            kit::Tier::Plate
        } else {
            kit::Tier::Flush
        };
        let tab_container = container(row_inner)
            .height(iced::Length::Fixed(if sheet { 30.0 } else { 28.0 }))
            // The sheet's tab is a capsule: the label and its close glyph sit
            // inside the plate's own round rather than touching its corner.
            .padding(if sheet {
                iced::Padding {
                    left: tokens::GAP_CONTROL,
                    right: tokens::GAP_HAIRLINE,
                    ..iced::Padding::ZERO
                }
            } else {
                iced::Padding::ZERO
            })
            .style(move |theme: &Theme| tab_tier.style(theme, ""));

        let tab_element: Element<'_, Message> = if tab.is_start {
            tab_container.into()
        } else {
            let has_current_path = tab.current_path.is_some();
            let has_other_drawings = drag_targets.len() > 1;
            let tab_target: Element<'_, Message> = if let Some(path) = &tab.current_path {
                iced::widget::tooltip(
                    tab_container,
                    // A tooltip is detached from the page, so its plate is glass
                    // reading the strip behind it; the host paints nothing.
                    kit::glass_pane(
                        container(
                            text(path.to_string_lossy().into_owned()).size(tokens::TEXT_LABEL),
                        )
                        .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
                        .padding(kit::Size::Compact.padding())
                        .into(),
                        theme,
                        false,
                        "",
                    ),
                    iced::widget::tooltip::Position::Bottom,
                )
                .gap(4)
                .into()
            } else {
                tab_container.into()
            };
            crate::ui::wrap_bar::PosReport::owned(
                format!("DOC_TAB:{idx}"),
                ContextMenu::new(tab_target, move || {
                    doc_tab_context_menu(idx, has_current_path, has_other_drawings, theme)
                }),
            )
            .into()
        };
        items.push(
            mouse_area(tab_element)
                .on_enter(Message::DocTabHover(Some(idx)))
                .on_exit(Message::DocTabHover(None))
                .into(),
        );
    }

    // New-drawing affordance of the strip: a plain pressable plate, so it takes
    // the control role and lets L1 walk the background ladder under the pointer.
    let new_btn = kit::btn(
        text("+").size(tokens::TEXT_HEADING),
        kit::Role::Standard,
        kit::Size::Compact,
        kit::Press::On(Message::TabNew),
        "",
    )
    .height(iced::Length::Fixed(28.0));

    items.push(
        container(new_btn)
            .padding(iced::Padding {
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
                left: 2.0,
            })
            .into(),
    );

    // On the sheet the strip is the page: the ribbon card above already carries
    // the edge, so a band under it would stack two fields of chrome with
    // nothing between them. The tab capsules are the only marks on it.
    let strip = container(
        Row::with_children(items)
            .spacing(if sheet {
                tokens::GAP_CONTROL
            } else {
                tokens::GAP_NONE
            })
            .align_y(iced::Center)
            .wrap()
            .vertical_spacing(2.0),
    )
    .width(Fill);
    let strip = if sheet {
        // The document strip is its own full-width band between the
        // application bar and the tool row: the open drawings sit on it like
        // file tabs, the active one lifted to a card.
        strip
            .style(|theme: &Theme| kit::sheet_strip_style(kit::Tier::Band, theme, ""))
            .padding(iced::Padding {
                top: tokens::GAP_SEAM,
                right: tokens::GAP_SECTION,
                bottom: 0.0,
                left: tokens::GAP_SECTION,
            })
    } else {
        strip
            .style(|theme: &Theme| kit::strip_style(theme, ""))
            .padding([tokens::GAP_HAIRLINE, tokens::GAP_HAIRLINE])
    };
    strip.into()
}

// ── Layout context-menu overlay ────────────────────────────────────────────

// ── Canvas-relative overlay positioning ────────────────────────────────────

/// Wraps `panel` in a column+row of `Space` widgets so it sits at
/// canvas-relative coordinates `(anchor.x, anchor.y)`. `panel` is wrapped
/// in `iced::widget::opaque` so mouse events on the panel itself do not
/// fall through to the viewport mouse area underneath; outside-click
/// dismissal is the caller's responsibility (handled via `ViewportLeftPress`
/// in `update.rs`, identical to how the multi-functional grip popup is
/// dismissed). Pushed into `viewport_stack` so the anchor is interpreted
/// in canvas-relative space, not window-relative.
// ── Start / Welcome page ──────────────────────────────────────────────────
//
// Renders in place of the model-space viewport when the active tab is the
// fixed Start tab (`DocumentTab::is_start`). English-only by design — this
// is the public welcome screen and stays consistent across locales.

/// Transparent input layer for one Model pane: a `mouse_area` filling the pane
/// that emits pane-tagged viewport events (`idx` = the pane's tile index). The
/// handlers offset the pane-local point to canvas coords and focus the pane.
fn pane_mouse_area<'a>(idx: usize) -> Element<'a, Message> {
    mouse_area(container(Space::new().width(Fill).height(Fill)))
        .on_move(move |p| Message::PaneMove(idx, p))
        .on_press(Message::PanePress(idx))
        .on_release(Message::PaneRelease(idx))
        .on_right_press(Message::PaneRightPress(idx))
        .on_right_release(Message::PaneRightRelease(idx))
        .on_middle_press(Message::PaneMiddlePress(idx))
        .on_middle_release(Message::PaneMiddleRelease(idx))
        .on_scroll(move |d| Message::PaneScroll(idx, d))
        .on_exit(Message::ViewportExit)
        .into()
}

pub(super) fn start_page_view<'a>(
    recents: &'a [std::path::PathBuf],
    thumbs: &'a rustc_hash::FxHashMap<std::path::PathBuf, Option<iced::widget::image::Handle>>,
    recent_limit: usize,
    recent_limit_input: &'a str,
    active: super::StartSection,
    query: &'a str,
    filter: super::StartFilter,
    tabs: &'a [DocumentTab],
    savetime_min: i32,
    theme: &'a Theme,
) -> Element<'a, Message> {
    responsive(move |size| {
        start_page_content(
            recents,
            thumbs,
            recent_limit,
            recent_limit_input,
            size.width,
            active,
            query,
            filter,
            tabs,
            savetime_min,
            theme,
        )
    })
    .into()
}

fn start_page_content<'a>(
    recents: &'a [std::path::PathBuf],
    thumbs: &'a rustc_hash::FxHashMap<std::path::PathBuf, Option<iced::widget::image::Handle>>,
    recent_limit: usize,
    recent_limit_input: &'a str,
    avail_w: f32,
    active: super::StartSection,
    query: &'a str,
    filter: super::StartFilter,
    tabs: &'a [DocumentTab],
    savetime_min: i32,
    theme: &'a Theme,
) -> Element<'a, Message> {
    const LEFT_COL: f32 = 280.0;
    const RIGHT_COL: f32 = 288.0;
    const GUTTER: f32 = 24.0;
    const CENTER_MIN: f32 = 540.0;

    let page_pad = iced::Padding {
        top: 22.0,
        right: 28.0,
        bottom: 20.0,
        left: 28.0,
    };
    let left = start_left(recents, theme);
    let right = start_right(savetime_min, recents);
    let recent = recent_files_panel(
        recents,
        thumbs,
        recent_limit,
        recent_limit_input,
        query,
        filter,
        tabs,
        theme,
        iced::Length::Fill,
    );

    // Three columns while the window can hold two action columns around a
    // readable table. Below that the same content becomes a tab strip: the
    // table first, the two narrow columns stacked in the second section.
    let wide = avail_w - page_pad.left - page_pad.right
        >= LEFT_COL + GUTTER + CENTER_MIN + GUTTER + RIGHT_COL;
    let body: Element<'a, Message> = if wide {
        row![
            container(left).width(LEFT_COL).height(Fill),
            container(recent).width(Fill).height(Fill),
            container(right).width(RIGHT_COL).height(Fill),
        ]
        .spacing(GUTTER)
        .height(Fill)
        .into()
    } else {
        let tab_btn = |label: String, section: super::StartSection| {
            let is_active = active == section;
            kit::btn(
                text(label).size(tokens::TEXT_HEADING),
                if is_active {
                    kit::Role::Engaged
                } else {
                    kit::Role::Ghost
                },
                kit::Size::Action,
                kit::Press::On(Message::StartSectionSelect(section)),
                "",
            )
        };
        let tab_bar = Row::with_children(vec![
            tab_btn(
                crate::tr!("start", "recent-files"),
                super::StartSection::Recent,
            )
            .into(),
            tab_btn(crate::tr!("start", "welcome"), super::StartSection::Welcome).into(),
        ])
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center)
        .wrap()
        .vertical_spacing(0.0);
        let section_body: Element<'a, Message> = match active {
            super::StartSection::Recent => container(recent)
                .width(Fill)
                .height(Fill)
                .center_x(Fill)
                .into(),
            super::StartSection::Welcome => kit::themed_scrollable(
                column![left, Space::new().height(tokens::GAP_SECTION), right].width(Fill),
            )
            .height(Fill)
            .into(),
        };
        column![
            Space::new().height(tokens::GAP_SECTION),
            container(tab_bar).width(Fill),
            Space::new().height(tokens::GAP_SECTION),
            section_body,
        ]
        .width(Fill)
        .height(Fill)
        .into()
    };

    // No background of its own: the root surface carries the theme colour and
    // its cross-fade, so the start page blends with theme switches too.
    container(body)
        .padding(page_pad)
        .width(Fill)
        .height(Fill)
        .into()
}

/// Function-key bindings, identical to the entries `default_bindings` ships.
/// The test below reads the live table, so this list cannot drift.
const START_SHORTCUTS: [(&str, &str); 6] = [
    ("F1", "HELP"),
    ("F2", "COMMANDHISTORY"),
    ("F3", "TOGGLEOSNAP"),
    ("F7", "GRID"),
    ("F8", "ORTHO"),
    ("F9", "SNAP"),
];

/// The action column: the four document commands at full width, the two
/// configuration panels beneath them, the most recent entries as chips, the
/// function keys the build actually binds, and the licence line.
fn start_left<'a>(recents: &'a [std::path::PathBuf], theme: &Theme) -> Element<'a, Message> {
    let mut col = column![
        text(crate::tr!("start", "begin"))
            .size(tokens::TEXT_TITLE)
            .font(tokens::UI_FONT_MEDIUM),
        Space::new().height(tokens::GAP_HAIRLINE),
        kit::caption(crate::tr!("start", "subtitle"), "start:card1"),
        Space::new().height(tokens::GAP_SECTION),
        start_action(
            crate::ui::icons::DOC_NEW,
            crate::tr!("start", "new-drawing"),
            super::shortcuts::chord("N"),
            true,
            Message::TabNew,
        ),
        Space::new().height(tokens::GAP_CONTROL),
        start_action(
            crate::ui::icons::FOLDER_OPEN,
            crate::tr!("start", "open-file"),
            super::shortcuts::chord("O"),
            false,
            Message::OpenFile,
        ),
        Space::new().height(tokens::GAP_CONTROL),
        start_action(
            crate::ui::icons::DOC_NEW,
            crate::tr!("start", "new-from-template"),
            ".dwt".to_string(),
            false,
            Message::StartNewFromTemplate,
        ),
        Space::new().height(tokens::GAP_CONTROL),
        start_action(
            crate::ui::icons::FILE_EXPORT,
            crate::tr!("start", "import-pdf"),
            "PDFIMPORT".to_string(),
            false,
            Message::StartPdfImport,
        ),
        Space::new().height(tokens::GAP_GUTTER),
        kit::rule_h("start:card1"),
        Space::new().height(tokens::GAP_CONTROL),
        start_nav_row(
            crate::ui::icons::GEAR,
            crate::tr!("action", "options"),
            Message::OptionsOpen,
        ),
    ]
    .spacing(tokens::GAP_NONE)
    .width(Fill);

    col = col
        .push(Space::new().height(tokens::GAP_GUTTER))
        .push(kit::rule_h("start:card1"))
        .push(Space::new().height(tokens::GAP_GUTTER))
        .push(meta(crate::tr!("start", "recent-used")))
        .push(Space::new().height(tokens::GAP_CONTROL));

    let mut chips: Vec<Element<'a, Message>> = Vec::new();
    for path in recents.iter().take(3) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        chips.push(
            kit::chip(
                crate::ui::text_util::elided(&name, 18, theme, |shown| {
                    text(shown).size(tokens::TEXT_META).into()
                }),
                false,
                kit::Press::On(Message::OpenRecent(path.clone())),
                "start:card1",
            )
            .into(),
        );
    }
    if recents.is_empty() {
        chips.push(meta(crate::tr!("start", "no-recent-files")));
    }
    col = col.push(
        Row::with_children(chips)
            .spacing(tokens::GAP_CONTROL)
            .wrap()
            .vertical_spacing(tokens::GAP_CONTROL),
    );

    col = col
        .push(Space::new().height(tokens::GAP_GUTTER))
        .push(meta(crate::tr!("common", "shortcuts")))
        .push(Space::new().height(tokens::GAP_CONTROL));
    let mut keys: Vec<Element<'a, Message>> = Vec::new();
    for (key, command) in START_SHORTCUTS {
        keys.push(
            row![
                meta_ink(command),
                Space::new().width(tokens::GAP_TIGHT),
                meta(key),
            ]
            .align_y(iced::Center)
            .into(),
        );
    }
    col = col
        .push(
            Row::with_children(keys)
                .spacing(tokens::GAP_SECTION)
                .wrap()
                .vertical_spacing(tokens::GAP_CONTROL),
        )
        .push(Space::new().height(Fill))
        .push(
        row![
            positive_glyph(crate::ui::icons::CHECK, 13.0),
            meta("GNU GPL v3"),
            Space::new().width(Fill),
            meta(format!("DraftFlow CAD {}", env!("CARGO_PKG_VERSION"))),
        ]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center),
    );
    col.into()
}

/// The right rail: autosave, the file commands' chords, the tips the drawing
/// pipeline actually honours, and a shortcut back into the newest drawing.
fn start_right<'a>(
    savetime_min: i32,
    recents: &'a [std::path::PathBuf],
) -> Element<'a, Message> {
    let card = move |title: String, body: Element<'a, Message>| -> Element<'a, Message> {
        kit::surface(
            column![
                kit::group(title, "start:card3"),
                Space::new().height(tokens::GAP_FIELD),
                body,
            ]
            .width(Fill),
            kit::Tier::Card,
            iced::Padding {
                top: tokens::GAP_SECTION,
                right: tokens::GAP_SECTION,
                bottom: tokens::GAP_SECTION,
                left: tokens::GAP_SECTION,
            },
            "start:card3",
        )
        .width(Fill)
        .into()
    };

    let autosave_rows = column![
        row![
            glyph(crate::ui::icons::SAVE, 13.0),
            text(if savetime_min > 0 {
                crate::tr!("start", "autosave-every", minutes = savetime_min)
            } else {
                crate::t!("Off").to_string()
            })
            .size(tokens::TEXT_META),
        ]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center),
        Space::new().height(tokens::GAP_TIGHT),
        tip(crate::tr!("start", "autosave-hint")),
    ]
    .width(Fill);

    let mut shortcuts = column![].spacing(tokens::GAP_TIGHT).width(Fill);
    for (label, chord) in [
        (crate::tr!("start", "new-drawing").to_string(), super::shortcuts::chord("N")),
        (crate::tr!("start", "open-file").to_string(), super::shortcuts::chord("O")),
        (crate::t!("Save").to_string(), super::shortcuts::chord("S")),
        (crate::t!("Print").to_string(), super::shortcuts::chord("P")),
        (crate::tr!("start", "command-list").to_string(), "F1".to_string()),
    ] {
        shortcuts = shortcuts.push(start_shortcut_row(label, chord));
    }

    let tips = column![
        tip(crate::tr!("start", "tip-fonts")),
        Space::new().height(tokens::GAP_CONTROL),
        tip(crate::tr!("start", "tip-commands")),
    ]
    .width(Fill);

    let mut rail = column![
        card(crate::t!("Autosave").to_string(), autosave_rows.into()),
        Space::new().height(tokens::GAP_SECTION),
        card(crate::t!("Shortcuts").to_string(), shortcuts.into()),
        Space::new().height(tokens::GAP_SECTION),
        card(crate::tr!("start", "tips").to_string(), tips.into()),
        Space::new().height(Fill),
    ]
    .width(Fill);

    if let Some(path) = recents.first() {
        rail = rail.push(
            kit::btn(
                text(crate::tr!("start", "continue-editing")).size(tokens::TEXT_META),
                kit::Role::Primary,
                kit::Size::Action,
                kit::Press::On(Message::OpenRecent(path.clone())),
                "start:card3",
            )
            .width(Fill),
        );
    }
    kit::themed_scrollable(rail).height(Fill).into()
}

/// One "command — chord" line in the right rail's shortcut card.
fn start_shortcut_row<'a>(label: String, chord: String) -> Element<'a, Message> {
    row![
        text(label).size(tokens::TEXT_META),
        Space::new().width(Fill),
        meta(chord),
    ]
    .height(24.0)
    .align_y(iced::Center)
    .into()
}

/// A wrapped tip line in the right rail.
fn tip<'a>(label: impl Into<String>) -> Element<'a, Message> {
    let label: String = label.into();
    text(label)
        .size(tokens::TEXT_META)
        .style(kit::muted_style)
        .width(Fill)
        .into()
}

/// A start-page action: glyph, label, chord. `primary` picks the filled plate
/// and the ink that reads on it; everything else is a plain outlined plate.
fn start_action<'a>(
    bytes: &'static [u8],
    label: String,
    chord: String,
    primary: bool,
    message: Message,
) -> iced::widget::Button<'a, Message> {
    const ACTION_H: f32 = 40.0;
    let mark = if primary {
        primary_glyph(bytes, 15.0)
    } else {
        glyph(bytes, 15.0)
    };
    let chord_style = move |theme: &Theme| kit::chord_ink_style(theme, primary);
    let content = container(
        row![
            mark,
            text(label).size(tokens::TEXT_META),
            Space::new().width(Fill),
            text(chord).size(tokens::TEXT_LABEL).style(chord_style),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
    )
    .width(Fill)
    .padding(iced::Padding {
        top: 0.0,
        right: tokens::GAP_SECTION,
        bottom: 0.0,
        left: tokens::GAP_SECTION,
    })
    .center_y(Fill);

    kit::btn(
        content,
        if primary {
            kit::Role::Primary
        } else {
            kit::Role::Standard
        },
        kit::Size::Flush,
        kit::Press::On(message),
        "start:card1",
    )
    .width(Fill)
    .height(ACTION_H)
}

/// An unframed row that opens a configuration panel.
fn start_nav_row<'a>(
    bytes: &'static [u8],
    label: String,
    message: Message,
) -> Element<'a, Message> {
    const NAV_H: f32 = 32.0;
    let content = container(
        row![glyph(bytes, 14.0), text(label).size(tokens::TEXT_META)]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center),
    )
    .width(Fill)
    .padding(iced::Padding {
        top: 0.0,
        right: tokens::GAP_SECTION,
        bottom: 0.0,
        left: tokens::GAP_SECTION,
    })
    .center_y(Fill);
    kit::btn(
        content,
        kit::Role::Ghost,
        kit::Size::Flush,
        kit::Press::On(message),
        "start:card1",
    )
    .width(Fill)
    .height(NAV_H)
    .into()
}

/// A chrome glyph at the kit's muted ink, for rows that carry no signal.
fn glyph<'a>(bytes: &'static [u8], size: f32) -> Element<'a, Message> {
    crate::ui::icons::themed_by(bytes, size, |theme| {
        kit::muted_style(theme)
            .color
            .unwrap_or_else(|| theme.palette().background.base.text)
    })
}

/// A glyph carrying the positive verdict — the licence line's check.
fn positive_glyph<'a>(bytes: &'static [u8], size: f32) -> Element<'a, Message> {
    crate::ui::icons::themed_by(bytes, size, |theme| {
        crate::ui::style::draftflow_theme::draftflow_colors(theme)
            .map(|colors| colors.positive)
            .unwrap_or_else(|| theme.palette().success.base.color)
    })
}

/// A glyph that sits on the filled primary plate, so it takes the plate's ink
/// rather than the page's.
fn primary_glyph<'a>(bytes: &'static [u8], size: f32) -> Element<'a, Message> {
    crate::ui::icons::themed_by(bytes, size, |theme| {
        crate::ui::style::draftflow_theme::draftflow_colors(theme)
            .map(|colors| colors.on_ink)
            .unwrap_or_else(|| theme.palette().primary.base.text)
    })
}

/// The 11 pt meta voice: table headers, paths, timestamps and licences. The
/// kit's `faint` and `body` are body rungs (12 pt); the start page's secondary
/// columns sit one rung below its 13 pt actions.
fn meta<'a>(label: impl Into<String>) -> Element<'a, Message> {
    text(label.into())
        .size(tokens::TEXT_LABEL)
        .style(kit::faint_style)
        .into()
}

fn meta_ink<'a>(label: impl Into<String>) -> Element<'a, Message> {
    text(label.into()).size(tokens::TEXT_LABEL).into()
}

/// A right-aligned numeric column in the recent-documents table.
fn right_cell<'a>(label: String, width: f32) -> Element<'a, Message> {
    container(
        text(label)
            .size(tokens::TEXT_LABEL)
            .style(kit::faint_style),
    )
    .width(width)
    .align_x(iced::alignment::Horizontal::Right)
    .into()
}

// ── Recent Documents panel (Start tab) ────────────────────────────────────
//
// The list is restored from disk at boot and re-saved on every open — entries
// persist across sessions. It is a table, not a card: the page is already the
// surface, and a second plate under the rows would put a frame around a frame.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RecentState {
    Opened,
    Saved,
    Missing,
}

/// A recent entry's state: a path another tab already holds is "opened", one
/// that still resolves on disk is "saved", and one whose metadata no longer
/// reads is "missing". The web build has no filesystem, so it never claims a
/// file is missing.
fn recent_state(path: &std::path::Path, tabs: &[DocumentTab]) -> RecentState {
    if tabs
        .iter()
        .filter(|tab| !tab.is_start)
        .any(|tab| tab.current_path.as_deref() == Some(path))
    {
        RecentState::Opened
    } else if recent_is_missing(path) {
        RecentState::Missing
    } else {
        RecentState::Saved
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn recent_is_missing(path: &std::path::Path) -> bool {
    !path.exists()
}

#[cfg(target_arch = "wasm32")]
fn recent_is_missing(_path: &std::path::Path) -> bool {
    false
}

/// The age band filter reads the file's last write time; an entry whose
/// metadata cannot be read is kept in every band rather than silently hidden.
fn recent_within_band(path: &std::path::Path, filter: super::StartFilter) -> bool {
    match filter {
        super::StartFilter::All => true,
        super::StartFilter::Today => recent_age_secs(path).map_or(true, |secs| secs < 86_400),
        super::StartFilter::Week => recent_age_secs(path).map_or(true, |secs| secs < 604_800),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn recent_age_secs(path: &std::path::Path) -> Option<u64> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .elapsed()
        .ok()
        .map(|elapsed| elapsed.as_secs())
}

#[cfg(target_arch = "wasm32")]
fn recent_age_secs(_path: &std::path::Path) -> Option<u64> {
    None
}

pub(super) fn recent_files_panel<'a>(
    recents: &'a [std::path::PathBuf],
    thumbs: &'a rustc_hash::FxHashMap<std::path::PathBuf, Option<iced::widget::image::Handle>>,
    limit: usize,
    limit_input: &'a str,
    query: &'a str,
    filter: super::StartFilter,
    tabs: &'a [DocumentTab],
    theme: &'a Theme,
    width: iced::Length,
) -> Element<'a, Message> {
    const ROW_H: f32 = 58.0;
    const THUMB_W: f32 = 44.0;
    const THUMB_H: f32 = 30.0;
    const POS_W: f32 = 150.0;
    const TIME_W: f32 = 92.0;
    const SIZE_W: f32 = 68.0;
    const STATUS_W: f32 = 68.0;
    const COL_GAP: f32 = 10.0;

    let needle = query.trim().to_lowercase();
    let matching: Vec<&'a std::path::PathBuf> = recents
        .iter()
        .filter(|path| recent_within_band(path, filter))
        .filter(|path| {
            if needle.is_empty() {
                return true;
            }
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let dir = path
                .parent()
                .map(|p| p.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            name.contains(needle.as_str()) || dir.contains(needle.as_str())
        })
        .collect();

    let count_pill = container(
        text(crate::tr!("start", "count-files", count = matching.len())).size(tokens::TEXT_LABEL),
    )
    .padding(iced::Padding {
        top: 1.0,
        right: tokens::GAP_ROW,
        bottom: 1.0,
        left: tokens::GAP_ROW,
    })
    .style(move |theme: &Theme| kit::Tier::Well.style(theme, "start:card2"));

    let filter_chip = |label: String, band: super::StartFilter| {
        kit::chip(
            text(label).size(tokens::TEXT_META),
            filter == band,
            kit::Press::On(Message::StartFilterSelect(band)),
            "start:card2",
        )
    };
    let filters = Row::with_children(vec![
        filter_chip(crate::t!("All").to_string(), super::StartFilter::All).into(),
        filter_chip(
            crate::tr!("start", "today").to_string(),
            super::StartFilter::Today,
        )
        .into(),
        filter_chip(
            crate::tr!("start", "this-week").to_string(),
            super::StartFilter::Week,
        )
        .into(),
    ])
    .spacing(tokens::GAP_TIGHT)
    .align_y(iced::Center);

    let search = iced::widget::text_input(crate::t!("Search…").as_ref(), query)
        .on_input(Message::StartQueryChanged)
        .size(tokens::TEXT_META)
        .padding(kit::PAD_GLYPH)
        .width(iced::Length::Fixed(180.0))
        .style(kit::field_style);

    let header = Row::with_children(vec![
        kit::heading(crate::tr!("start", "recent-documents"), "start:card2"),
        count_pill.into(),
        Space::new().width(Fill).into(),
        filters.into(),
        search.into(),
        meta(format!("/ {}", super::recent::RECENT_MAX)).into(),
    ])
    .spacing(tokens::GAP_CONTROL)
    .align_y(iced::Center)
    .wrap()
    .vertical_spacing(tokens::GAP_CONTROL);

    let columns = row![
        container(meta(crate::tr!("common", "name-title-case"))).width(Fill),
        container(meta(crate::t!("Location"))).width(POS_W),
        right_cell(crate::tr!("start", "modified"), TIME_W),
        right_cell(crate::tr!("common", "size"), SIZE_W),
        container(meta(crate::t!("Status"))).width(STATUS_W),
    ]
    .spacing(COL_GAP)
    .padding(iced::Padding {
        top: 0.0,
        right: tokens::GAP_ROW,
        bottom: 5.0,
        left: tokens::GAP_ROW,
    })
    .height(24.0)
    .align_y(iced::alignment::Vertical::Bottom);

    let empty_state = |label: String| -> Element<'a, Message> {
        container(
            column![
                glyph(crate::ui::icons::DOC, 22.0),
                meta(label),
            ]
            .spacing(tokens::GAP_ROW)
            .align_x(iced::alignment::Horizontal::Center),
        )
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill)
        .into()
    };

    let body: Element<'a, Message> = if recents.is_empty() {
        empty_state(crate::tr!("start", "no-recent-files").to_string())
    } else if matching.is_empty() {
        empty_state(crate::t!("No matches").to_string())
    } else {
        let mut list = column![].spacing(tokens::GAP_NONE).width(Fill);
        for path in matching {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned());
            let dir = path
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            // Browsers intentionally do not reveal the source folder selected
            // by the user. The reusable copy lives in origin-private browser
            // storage, which is the truthful web equivalent of native's parent
            // directory line.
            #[cfg(target_arch = "wasm32")]
            let dir = if dir.is_empty() {
                crate::tr!("start", "browser-storage")
            } else {
                dir
            };
            let (modified, size) = recent_facts(path);

            let thumb: Element<'a, Message> =
                match thumbs.get(path).and_then(|entry| entry.as_ref()) {
                    Some(handle) => container(
                        iced::widget::image(handle.clone())
                            .content_fit(iced::ContentFit::Contain)
                            .width(Fill)
                            .height(Fill),
                    )
                    .width(THUMB_W)
                    .height(THUMB_H)
                    .style(move |theme: &Theme| kit::Tier::Well.style(theme, "start:card2"))
                    .into(),
                    None => container(glyph(crate::ui::icons::DOC, 18.0))
                        .width(THUMB_W)
                        .height(THUMB_H)
                        .center_x(Fill)
                        .center_y(Fill)
                        .style(move |theme: &Theme| kit::Tier::Well.style(theme, "start:card2"))
                        .into(),
                };

            let state = recent_state(path, tabs);
            let (state_label, state_ink) = match state {
                RecentState::Opened => (
                    crate::tr!("start", "opened").to_string(),
                    kit::accent_style(theme),
                ),
                RecentState::Saved => (
                    crate::tr!("start", "saved").to_string(),
                    kit::faint_style(theme),
                ),
                RecentState::Missing => (
                    crate::tr!("start", "missing").to_string(),
                    kit::tone_style(theme, kit::Tone::Danger),
                ),
            };
            let badge = container(
                container(
                    text(state_label)
                        .size(tokens::TEXT_LABEL)
                        .style(move |_: &Theme| state_ink),
                )
                    .padding(iced::Padding {
                        top: 1.0,
                        right: tokens::GAP_CONTROL,
                        bottom: 1.0,
                        left: tokens::GAP_CONTROL,
                    })
                    .style(move |theme: &Theme| kit::Tier::Well.style(theme, "start:card2")),
            )
            .width(STATUS_W)
            .align_x(iced::alignment::Horizontal::Left);

            let content = row![
                thumb,
                container(crate::ui::text_util::elided(&name, 28, theme, |shown| {
                    text(shown).size(tokens::TEXT_META).into()
                }))
                .width(Fill),
                container(crate::ui::text_util::elided(&dir, 30, theme, meta)).width(POS_W),
                right_cell(modified, TIME_W),
                right_cell(size, SIZE_W),
                badge,
            ]
            .spacing(COL_GAP)
            .height(Fill)
            .align_y(iced::Center);

            let open = kit::list_row(
                content,
                false,
                kit::Press::On(Message::OpenRecent(path.clone())),
                "start:card2",
            )
            .height(ROW_H);

            // A `✕` on a row: unframed until the pointer arrives, then the
            // danger ramp. `QuietDanger` is that whole rule. It sits in a
            // `stack` above the open button so a click on it cannot open the
            // document underneath.
            let remove = kit::btn(
                crate::ui::icons::themed_danger(crate::ui::icons::CLOSE, 13.0),
                kit::Role::QuietDanger,
                kit::Size::Compact,
                kit::Press::On(Message::RecentRemove(path.clone())),
                "start:card2",
            );
            let overlay = container(remove)
                .width(Fill)
                .center_y(iced::Length::Fixed(ROW_H))
                .align_x(iced::alignment::Horizontal::Right)
                .padding(iced::Padding {
                    top: 0.0,
                    right: tokens::GAP_ROW,
                    bottom: 0.0,
                    left: 0.0,
                });

            list = list.push(stack![open, overlay].width(Fill));
            list = list.push(kit::rule_h("start:card2"));
        }
        kit::themed_scrollable(list).height(Fill).into()
    };

    // Footer: how many recent files to keep — [-] [editable count] [+] with the
    // max shown. The count box takes keyboard input (applied on Enter); the
    // update handler clamps to [MIN, MAX] and persists (see `set_recent_limit`),
    // so an over-max entry snaps to the max.
    const STEP: usize = 5;
    // +/- step from whatever is currently shown in the box (mid-edit included).
    let shown = limit_input.parse::<usize>().unwrap_or(limit);
    let count_box = iced::widget::text_input("", limit_input)
        .on_input(Message::RecentLimitInput)
        .on_submit(Message::SetRecentLimit(shown))
        .size(tokens::TEXT_META)
        .padding(kit::PAD_GLYPH)
        .width(iced::Length::Fixed(46.0));
    let limit_row = row![
        meta(crate::tr!("start", "keep-recent-files")),
        Space::new().width(Fill),
        kit::btn(
            crate::ui::icons::themed(crate::ui::icons::MINUS, 11.0),
            kit::Role::Standard,
            kit::Size::Compact,
            kit::Press::On(Message::SetRecentLimit(shown.saturating_sub(STEP))),
            "start:card2",
        ),
        count_box,
        kit::btn(
            crate::ui::icons::themed(crate::ui::icons::PLUS, 11.0),
            kit::Role::Standard,
            kit::Size::Compact,
            kit::Press::On(Message::SetRecentLimit(shown.saturating_add(STEP))),
            "start:card2",
        ),
        meta(format!("/ {}", super::recent::RECENT_MAX)),
    ]
    .spacing(tokens::GAP_CONTROL)
    .align_y(iced::Center);

    container(
        column![
            header,
            Space::new().height(tokens::GAP_FIELD),
            columns,
            kit::rule_h("start:card2"),
            body,
            kit::rule_h("start:card2"),
            container(limit_row)
                .padding(iced::Padding {
                    top: 0.0,
                    right: tokens::GAP_ROW,
                    bottom: 0.0,
                    left: tokens::GAP_ROW,
                })
                .height(34.0),
        ]
        .spacing(tokens::GAP_NONE)
        .width(Fill)
        .height(Fill),
    )
    .width(width)
    .height(Fill)
    .into()
}

/// The two columns the table shows beside a document name. Native builds read
/// the file's real metadata; the web build has no filesystem, so both cells
/// stay empty rather than inventing a value.
#[cfg(not(target_arch = "wasm32"))]
fn recent_facts(path: &std::path::Path) -> (String, String) {
    let Ok(meta) = std::fs::metadata(path) else {
        return (String::new(), String::new());
    };
    let modified = meta
        .modified()
        .ok()
        .and_then(|time| time.elapsed().ok())
        .map(|elapsed| {
            let secs = elapsed.as_secs();
            if secs < 60 {
                "<1m".to_string()
            } else if secs < 3_600 {
                format!("{}m", secs / 60)
            } else if secs < 86_400 {
                format!("{}h", secs / 3_600)
            } else {
                format!("{}d", secs / 86_400)
            }
        })
        .unwrap_or_default();
    let bytes = meta.len();
    let size = if bytes < 1_024 {
        format!("{bytes} B")
    } else if bytes < 1_048_576 {
        format!("{:.1} KB", bytes as f64 / 1_024.0)
    } else if bytes < 1_073_741_824 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    };
    (modified, size)
}

#[cfg(target_arch = "wasm32")]
fn recent_facts(_path: &std::path::Path) -> (String, String) {
    (String::new(), String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crosshair_background_paper_space() {
        let tab = crate::app::document::DocumentTab::new_drawing(1);
        let bg = crosshair_background(&tab, true);
        assert_eq!(bg, tab.scene.paper_bg_color);
        assert!(crate::ui::style::common::canvas_is_light(bg));
    }

    /// The [+] step on the keep-recent-files box is built eagerly from an
    /// uncapped user string (`RecentLimitInput` keeps every ASCII digit):
    /// `18446744073709551615` parses to `usize::MAX`, and the plain
    /// `shown + STEP` overflowed while the *widget was constructed* —
    /// i.e. during `view()`, before any click. The [−] button next to it
    /// already used `saturating_sub`.
    #[test]
    fn recent_limit_plus_step_survives_an_overflowing_input() {
        let theme = Theme::Light;
        let _ = start_page_content(
            &[],
            &rustc_hash::FxHashMap::default(),
            50,
            "18446744073709551615",
            800.0,
            super::super::StartSection::default(),
            "",
            super::super::StartFilter::default(),
            &[],
            10,
            &theme,
        );
    }

    /// The start page prints function keys as plain text, so the column can
    /// drift from the table the build actually binds. Read the live table and
    /// fail on any mismatch.
    #[test]
    fn start_page_function_keys_match_the_default_bindings() {
        let bindings = crate::app::shortcuts::default_bindings();
        for (key, command) in START_SHORTCUTS {
            assert_eq!(
                bindings.get(key).map(String::as_str),
                Some(command),
                "{key} is not bound to {command}"
            );
        }
    }
}
