use super::super::Message;
use crate::scene::{VIEWCUBE_PX, VIEWCUBE_REGION_PX, VIEWCUBE_RING_R, VIEWCUBE_SCALE};
use crate::ui::kit::{self, Press, Role};
use crate::ui::style::tokens;
use iced::widget::{container, mouse_area, stack, Space};
use iced::Element;

// ── Render-mode picker ──────────────────────────────────────────────────────

/// Top-left viewport control bar: a single dark chip holding (optionally) the
/// horizontal/vertical split buttons, the render-mode picker, and the grid /
/// grid-snap toggles. `include_split` is off for paper-space viewports, which
/// have no model-tile splitting. Grid / snap reflect the active viewport's
/// state and emit `ToggleGrid` / `ToggleGridSnap`.
// ── ViewCube navigation controls (home / roll / nudge / UCS) ───────────────

/// Place `el` at pixel offset (x, y) inside a fill layer (top-left origin).
fn vc_place<'a>(x: f32, y: f32, el: Element<'a, Message>) -> Element<'a, Message> {
    iced::widget::pin(el)
        .position(iced::Point::new(x.max(0.0), y.max(0.0)))
        .into()
}

/// Borderless square icon button used by the ViewCube nav controls.
fn vc_btn<'a>(content: Element<'a, Message>, size: f32, msg: Message) -> Element<'a, Message> {
    kit::icon_btn(content, Role::Ghost, size, Press::On(msg), "").into()
}

/// Overlay of home / roll / nudge controls sized to the whole nav region, so
/// the caller can position it exactly like the cube hit area.
pub(super) fn viewcube_nav_controls<'a>(viewport: Option<codec::Handle>) -> Element<'a, Message> {
    use crate::scene::NudgeDir;
    use crate::ui::icons;
    let r = VIEWCUBE_REGION_PX;
    let c = r * 0.5;
    let cube_half = VIEWCUBE_PX as f32 * VIEWCUBE_SCALE;
    // Nudge triangles ride on the compass ring itself: between the cube body
    // and the ring the gaps are too small at a three-quarter view, while the
    // ring's four nearest points sit clear of the projected silhouette.
    let nr = cube_half * VIEWCUBE_RING_R;
    const BTN: f32 = 14.0;
    const TRI: f32 = 7.5;
    let ctr = |cx: f32, cy: f32, s: f32| (cx - s * 0.5, cy - s * 0.5);

    // Home top-left, roll arrows top-right.
    let (rax, ray) = (r - 2.0 * BTN - 4.0, 2.0);
    let (rbx, rby) = (r - BTN - 2.0, 2.0);
    // Nudge triangles pointing inward at the four cube faces.
    let (tux, tuy) = ctr(c, c - nr, TRI);
    let (tdx, tdy) = ctr(c, c + nr, TRI);
    let (tlx, tly) = ctr(c - nr, c, TRI);
    let (trx, try_) = ctr(c + nr, c, TRI);

    // Bottom layer: the cube/cardinal hit area covering the whole region. The
    // control buttons sit ABOVE it in the same stack, so a click on a button is
    // caught by the button while a click on the cube (or empty space) falls
    // through to this mouse_area → ViewportClick. Moves keep cursor_pos current.
    let cube_hit = mouse_area(
        Space::new()
            .width(iced::Length::Fixed(r))
            .height(iced::Length::Fixed(r)),
    )
    .on_move(move |point| Message::CursorMoved(point, viewport))
    .on_press(Message::ViewportClick(viewport));

    let controls = stack![
        cube_hit,
        vc_place(
            3.0,
            3.0,
            vc_btn(icons::themed_home(11.5), BTN, Message::ViewCubeHome)
        ),
        vc_place(
            rax,
            ray,
            vc_btn(
                icons::themed_undo(10.5, true),
                BTN,
                Message::ViewCubeRoll(false)
            )
        ),
        vc_place(
            rbx,
            rby,
            vc_btn(
                icons::themed_redo(10.5, true),
                BTN,
                Message::ViewCubeRoll(true)
            )
        ),
        vc_place(
            tux,
            tuy,
            vc_btn(
                icons::themed_arrow_down(6.5),
                TRI,
                Message::ViewCubeNudge(NudgeDir::Up)
            )
        ),
        vc_place(
            tdx,
            tdy,
            vc_btn(
                icons::themed_arrow_up(6.5),
                TRI,
                Message::ViewCubeNudge(NudgeDir::Down)
            )
        ),
        vc_place(
            tlx,
            tly,
            vc_btn(
                icons::themed_arrow_right(6.5),
                TRI,
                Message::ViewCubeNudge(NudgeDir::Left)
            )
        ),
        vc_place(
            trx,
            try_,
            vc_btn(
                icons::themed_arrow_left(6.5),
                TRI,
                Message::ViewCubeNudge(NudgeDir::Right)
            )
        ),
    ];

    container(controls)
        .width(iced::Length::Fixed(r))
        .height(iced::Length::Fixed(r))
        .into()
}

/// Fixed width of the UCS picker, so it can be centred under the cube.
pub(super) const UCS_PICKER_W: f32 = 84.0;

/// The WCS / named-UCS selector shown under the cube. `names` is the
/// epoch-memoised `Scene::cached_ucs_names` Arc (#34) — a refcount bump per
/// frame instead of N String clones.
pub(super) fn viewcube_ucs_picker<'a>(
    current: String,
    names: std::sync::Arc<[String]>,
) -> Element<'a, Message> {
    let mut options = vec!["WCS".to_string()];
    options.extend(names.iter().cloned());
    let selected = if current.is_empty() {
        "WCS".to_string()
    } else {
        current
    };
    kit::themed_pick_list(Some(selected), options, |value| crate::i18n::app_name_display(value))
        .on_select(Message::SetViewcubeUcs)
        .text_size(tokens::TEXT_LABEL)
        .padding(kit::PAD_GLYPH)
        // Fixed width so the caller can centre it under the cube centre with a
        // simple half-width offset (content-sized width would drift off-centre).
        .width(iced::Length::Fixed(UCS_PICKER_W))
        .into()
}
