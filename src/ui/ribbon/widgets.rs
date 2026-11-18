// Shared rendering helpers, button styles, colours, layout constants, and
// free functions used by the Ribbon view/overlay methods.

use crate::ui::kit;
use rustc_hash::FxHashMap as HashMap;
use std::cell::RefCell;
use std::time::Duration;

use codec::types::{Color as AcadColor, LineWeight};
use iced::advanced::{
    layout, mouse, overlay, renderer, text as advanced_text, widget, Layout, Shell, Widget,
};
// Ribbon tooltips anchor to the right of their button so the cursor — which
// rests on the button itself — never covers the tip text. (#143)
use iced::widget::tooltip::Position as TipPos;
use iced::widget::{button, column, container, mouse_area, row, text, tooltip};
use crate::ui::kit::plain_button;
use iced::{
    Color, Element, Event, Fill, Length, Padding, Pixels, Rectangle, Size,
    Theme, Vector,
};

use crate::app::Message;
use crate::modules::{IconKind, ModuleEvent, RibbonItem, StyleKey, ToolDef};
use crate::t;
use crate::ui::icons;
use crate::ui::kit::{style_for, Role};
use crate::ui::properties::{acad_color_display, linetype_display_name, LwItem};
use crate::ui::style::tokens;
use crate::ui::wrap_bar::PosReport;

use super::LayerInfo;

/// Live on/off state of every ribbon toggle button. Passed as a single value
/// through the render path so adding a new toggle only touches `is_active_tool`
/// plus the `Ribbon::toggle_state` builder — not every render-function signature
/// and call site.
#[derive(Clone, Copy)]
pub(super) struct ToggleState {
    pub ortho_mode: bool,
    pub show_viewcube: bool,
    pub show_ucs_icon: bool,
    pub show_properties: bool,
    pub show_count_palette: bool,
    pub show_graphic_attributes: bool,
    pub show_block_palette: bool,
    pub show_sheet_set: bool,
    pub show_file_tabs: bool,
    pub show_layout_tabs: bool,
}

// ── Layout constants (single source of truth: ROW_H from ui::mod) ─────────

use crate::ui::ROW_H;

/// Icon size inside a 3-row (large) button (sheet: `.tbtn .ic` 20 px).
pub(super) const LARGE_ICON: f32 = 20.0;
/// Icon size inside a 1-row (small) button (sheet: `.tbtn.sm .ic` 16 px).
pub(super) const SMALL_ICON: f32 = 16.0;
/// Width of a 3-row (large) button (sheet: `.tbtn` 68 px).
pub(super) const LARGE_W: f32 = 68.0;
/// Height of a 3-row (large) button (sheet: `.tbtn` 62 px).
pub(super) const LARGE_H: f32 = 62.0;
/// Horizontal button padding surrounding a large tool's label.
const LARGE_LABEL_HPAD: f32 = 8.0;
/// Large ribbon labels use at most two lines before their button grows.
const LARGE_LABEL_LINES: f32 = 2.0;
const LARGE_LABEL_SIZE: f32 = tokens::TEXT_BODY;
/// Width of a 1-row icon-only button. It was square with `SMALL_H` at 21; the
/// 2026-11-02 ladder lift made the row 25 to clear 14 px text, and the icon
/// button kept its width because a 16 px glyph needs no more than this.
pub(super) const SMALL_W: f32 = 21.0;
/// Height of a 1-row small button (sheet: `.tbtn.sm` and the grid rows). 25
/// since 2026-11-02: 14 px of label plus `PAD_CELL`'s 3 px above and below.
pub(super) const SMALL_H: f32 = 25.0;
const LABELED_SMALL_W: f32 = 100.0;
/// Width of the ▾ strip on a small dropdown.
pub(super) const ARROW_W: f32 = ROW_H * 0.4;
/// Arrow size used under large dropdown tools such as Circle.
pub(super) const LARGE_DROPDOWN_ARROW_SIZE: f32 = 9.0;
/// Glyph size of the one mark a control wears when it has more choices behind
/// it. `caret_option.svg` is a tight 16×10 viewBox, so at 11 px the ink is a
/// full 11×7 px — a caret you can see at a glance, still well under the 20 px
/// tool icon above it.
pub(super) const OPTION_MARK_GLYPH: f32 = 11.0;
/// Height of the mark's band. The caret sits dead centre in it, directly under
/// the tool's name, and the band itself is the click target: the full cell
/// width × 14 px rather than a glyph-sized sliver.
pub(super) const OPTION_MARK_H: f32 = 14.0;
/// Gap between the caret's ink and the bottom of its band.
const OPTION_MARK_INSET_BOTTOM: f32 = 1.0;

/// Ink height of the caret: the tight 16×10 viewBox scales 16 px to
/// [`OPTION_MARK_GLYPH`], so the ink is 10/16 of the element tall.
fn option_mark_ink_h() -> f32 {
    OPTION_MARK_GLYPH * (10.0 / 16.0)
}

/// One rendered line of a label at `size`, in px.
fn label_line_h(size: f32) -> f32 {
    iced::advanced::text::LineHeight::default()
        .to_absolute(Pixels(size))
        .0
}

/// How far the mark band is lifted off the cell's bottom edge.
///
/// Every host has a different height (76 px strip cell, 62 px large button,
/// 64 px panel cell), but the caret must keep one relationship with its label:
/// one hairline under the last line. Without the lift the band is pinned to the
/// cell's bottom edge, so on the taller hosts the caret drifts away from the
/// name and reads as a sticker rather than part of the button.
pub(super) fn option_mark_lift(cell_h: f32, pad_top: f32, icon_h: f32, label_size: f32) -> f32 {
    let label_bottom = pad_top + icon_h + tokens::GAP_HAIRLINE + label_line_h(label_size);
    option_mark_lift_below(cell_h, label_bottom, tokens::GAP_HAIRLINE)
}

/// The lift for a cell whose label block (pad + icon + every line) ends at
/// `label_block_h`. A host that reserves two lines passes the taller block, so
/// the caret still lands under the name instead of on top of its second line.
fn option_mark_lift_below(cell_h: f32, label_block_h: f32, gap: f32) -> f32 {
    (cell_h - label_block_h - gap - option_mark_ink_h() - OPTION_MARK_INSET_BOTTOM).max(0.0)
}
/// Tab-strip height: the sheet's 32 px row, shared by the layout and the
/// dropdown anchors that hang off it.
pub(super) const TAB_BAR_H: f32 = 32.0;
/// Total ribbon tool-area height (sheet: `.ribbon` 90 px).
pub(super) const TOOL_BAR_H: f32 = 90.0;
/// Height of a collapsed panel button's large representative face (big icon +
/// its label). A collapsed button is this face plus the title opener, so it is
/// shorter than a full 3-row panel — the ribbon height follows it down.
pub(super) const COLLAPSED_FACE_H: f32 =
    LARGE_ICON + LARGE_LABEL_SIZE * 2.4 + 2.0 * tokens::GAP_TIGHT;

/// Gap between the combo dropdown and the tool rows beneath it (sheet: 2 px).
const COMBO_ROW_SPACING: f32 = tokens::GAP_HAIRLINE;
/// Shared padding of combo-style panels (Layers, Properties, style selectors).
/// The top inset pins every primary dropdown to the same baseline regardless
/// of how many icon rows sit underneath it.
const COMBO_PANEL_PAD: Padding = Padding {
    top: tokens::GAP_TIGHT,
    bottom: tokens::GAP_NONE,
    left: tokens::GAP_TIGHT,
    right: tokens::GAP_TIGHT,
};

// ── Automatic large-button sizing ────────────────────────────────────────

thread_local! {
    /// Ribbon layout runs on every pointer-driven view update. Cache the font-
    /// measured width per translated label so the automatic sizing stays cheap.
    static LARGE_WIDTH_CACHE: RefCell<HashMap<String, f32>> =
        RefCell::new(HashMap::default());
    /// Strip cells repeat their labels even more often than large buttons do,
    /// and they measure against a narrower profile, so they keep their own map.
    static STRIP_WIDTH_CACHE: RefCell<HashMap<String, f32>> =
        RefCell::new(HashMap::default());
}

/// Dropdown items that report a state and cannot be picked.
pub(super) fn is_disabled_item(cmd: &str) -> bool {
    cmd == "FRAMES3"
}

fn ribbon_label_bounds(
    renderer: &iced::Renderer,
    label: &str,
    width: f32,
    size: f32,
    wrapping: advanced_text::Wrapping,
) -> Size {
    use advanced_text::{Paragraph as _, Renderer as _};

    let paragraph =
        <iced::Renderer as advanced_text::Renderer>::Paragraph::with_text(advanced_text::Text {
            content: label,
            bounds: Size::new(width, f32::INFINITY),
            size: Pixels(size),
            line_height: advanced_text::LineHeight::default(),
            font: renderer.default_font(),
            align_x: advanced_text::Alignment::Center,
            align_y: iced::alignment::Vertical::Center,
            shaping: advanced_text::Shaping::default(),
            wrapping,
            ellipsis: advanced_text::Ellipsis::None,
            hint_factor: None,
        });
    paragraph.min_bounds()
}

/// Measure a translated label against one host's box. It wraps first; only
/// labels that would need more than two lines widen their button. The binary
/// search uses the renderer's real font metrics, so locale and UI scale
/// changes do not rely on character-count estimates.
///
/// `inner` is the label's width inside the host's padding, `hpad` is that
/// padding in total, and the result is the outer width the host should take.
fn measure_label_width(
    renderer: &iced::Renderer,
    label: &str,
    size: f32,
    base_inner: f32,
    hpad: f32,
    base_outer: f32,
) -> f32 {
    let max_label_height = label_line_h(size) * LARGE_LABEL_LINES + 0.5;
    let fits = |width: f32| {
        let bounds = ribbon_label_bounds(
            renderer,
            label,
            width,
            size,
            advanced_text::Wrapping::Word,
        );
        bounds.width <= width + 0.5 && bounds.height <= max_label_height
    };

    if fits(base_inner) {
        return base_outer;
    }

    let natural = ribbon_label_bounds(
        renderer,
        label,
        f32::INFINITY,
        size,
        advanced_text::Wrapping::None,
    )
    .width
    .max(base_inner);
    if !fits(natural) {
        return (natural + hpad).ceil();
    }

    let mut low = base_inner;
    let mut high = natural;
    for _ in 0..10 {
        let mid = (low + high) * 0.5;
        if fits(mid) {
            high = mid;
        } else {
            low = mid;
        }
    }
    (high + hpad).ceil().max(base_outer)
}

fn automatic_large_width(renderer: &iced::Renderer, label: &str) -> f32 {
    if let Some(width) = LARGE_WIDTH_CACHE.with(|cache| cache.borrow().get(label).copied()) {
        return width;
    }

    let width = measure_label_width(
        renderer,
        label,
        LARGE_LABEL_SIZE,
        (LARGE_W - LARGE_LABEL_HPAD).max(1.0),
        LARGE_LABEL_HPAD,
        LARGE_W,
    );
    LARGE_WIDTH_CACHE.with(|cache| {
        cache.borrow_mut().insert(label.to_string(), width);
    });
    width
}

fn automatic_strip_width(renderer: &iced::Renderer, label: &str) -> f32 {
    if let Some(width) = STRIP_WIDTH_CACHE.with(|cache| cache.borrow().get(label).copied()) {
        return width;
    }

    let width = measure_label_width(
        renderer,
        label,
        tokens::TEXT_LABEL,
        (STRIP_W - STRIP_LABEL_HPAD).max(1.0),
        STRIP_LABEL_HPAD,
        STRIP_W,
    );
    STRIP_WIDTH_CACHE.with(|cache| {
        cache.borrow_mut().insert(label.to_string(), width);
    });
    width
}

struct AutomaticWidth<'a> {
    profile: MeasureProfile,
    label: String,
    content: Element<'a, Message>,
}

/// Which host a label is being measured for. The two hosts share the rule
/// (widen only when the name would need a third line) but not the numbers.
#[derive(Clone, Copy)]
enum MeasureProfile {
    Large,
    Strip,
}

impl MeasureProfile {
    fn width(self, renderer: &iced::Renderer, label: &str) -> f32 {
        match self {
            Self::Large => automatic_large_width(renderer, label),
            Self::Strip => automatic_strip_width(renderer, label),
        }
    }
}

impl Widget<Message, Theme, iced::Renderer> for AutomaticWidth<'_> {
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
        Size::new(Length::Shrink, self.content.as_widget().size().height)
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let width = self.profile.width(renderer, &self.label);
        self.content
            .as_widget_mut()
            .layout(tree, renderer, &limits.width(Length::Fixed(width)))
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
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
        self.content
            .as_widget_mut()
            .update(tree, event, layout, cursor, renderer, shell, viewport);
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
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
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

pub(super) fn automatic_large_button<'a>(
    label: String,
    content: Element<'a, Message>,
) -> Element<'a, Message> {
    Element::new(AutomaticWidth {
        profile: MeasureProfile::Large,
        label,
        content,
    })
}

/// A strip cell whose width follows its name. The row itself never wraps, so a
/// name that needed a third line used to spill over its neighbours; the cell
/// widens just enough for two lines instead.
fn automatic_strip_cell<'a>(label: String, content: Element<'a, Message>) -> Element<'a, Message> {
    Element::new(AutomaticWidth {
        profile: MeasureProfile::Strip,
        label,
        content,
    })
}

// ── Tab-bar constants ──────────────────────────────────────────────────────

pub(super) const TOP_ARR_W: f32 = 12.0;
pub(super) const TOP_HIST_W: f32 = 28.0;
pub(super) const TOP_HIST_GAP: f32 = 4.0;

/// The selected tab's ring is heavier than a control's 1px edge: it is the only
/// thing that says which tab is current once the panel below it has the same
/// aurora wash.
pub(super) const TAB_RING_W: f32 = 2.0;
/// Peak alpha of the brand halo behind the selected tab, and the wash a tab
/// holds while a command from it is running.
pub(super) const TAB_HALO_A: f32 = 0.30;
pub(super) const HOLDS_TOOL_WASH: f32 = 0.35;

// ── Dropdown / combo ID constants ─────────────────────────────────────────

pub(crate) const UNDO_HISTORY_ID: &str = "UNDO_HISTORY";
pub(crate) const REDO_HISTORY_ID: &str = "REDO_HISTORY";
pub(crate) const LAYER_COMBO_ID: &str = "LAYER_COMBO";
/// Dropdown id for the tab-bar panel-density selector.
pub(super) const COLLAPSE_MODE_ID: &str = "COLLAPSE_MODE";
pub(super) const PROP_COLOR_ID: &str = "PROP_COLOR";
pub(super) const PROP_LINETYPE_ID: &str = "PROP_LINETYPE";
pub(super) const PROP_LW_ID: &str = "PROP_LW";

// ── Style context (passed from Ribbon to render_large) ────────────────────

pub(super) struct StyleContext<'a> {
    pub text_style_names: &'a [String],
    pub active_text_style: &'a str,
    pub dim_style_names: &'a [String],
    pub active_dim_style: &'a str,
    pub mleader_style_names: &'a [String],
    pub active_mleader_style: &'a str,
    pub table_style_names: &'a [String],
    pub active_table_style: &'a str,
}

impl<'a> StyleContext<'a> {
    pub(super) fn names_for(&self, key: StyleKey) -> &[String] {
        match key {
            StyleKey::TextStyle => self.text_style_names,
            StyleKey::DimStyle => self.dim_style_names,
            StyleKey::MLeaderStyle => self.mleader_style_names,
            StyleKey::TableStyle => self.table_style_names,
        }
    }
    pub(super) fn active_for(&self, key: StyleKey) -> &str {
        match key {
            StyleKey::TextStyle => self.active_text_style,
            StyleKey::DimStyle => self.active_dim_style,
            StyleKey::MLeaderStyle => self.active_mleader_style,
            StyleKey::TableStyle => self.active_table_style,
        }
    }
}

// ── Layout helpers ─────────────────────────────────────────────────────────

/// Flush up-to-3 small items as a vertical column into the group row.
pub(super) fn flush_small_col<'a>(
    buf: &mut Vec<Element<'a, Message>>,
    out: &mut Vec<Element<'a, Message>>,
) {
    if buf.is_empty() {
        return;
    }
    let col = column(std::mem::take(buf)).spacing(tokens::GAP_NONE);
    out.push(col.into());
}

pub(super) fn make_icon(icon: IconKind, size: f32) -> Element<'static, Message> {
    match icon {
        IconKind::Glyph(s) => text(s).size(size * 0.7).into(),
        IconKind::Svg(bytes) => icons::semantic(bytes, size),
    }
}

pub(super) fn is_active_tool(id: &str, active_tool: &Option<String>, state: &ToggleState) -> bool {
    match id {
        "ORTHO" => state.ortho_mode,
        "PERSP" => !state.ortho_mode,
        "NAVVCUBE" => state.show_viewcube,
        "UCSICON" => state.show_ucs_icon,
        "PROPERTIES" => state.show_properties,
        "COUNTLIST" => state.show_count_palette,
        "GRAPHICATTRIBUTES" => state.show_graphic_attributes,
        "BLOCKPALETTE" => state.show_block_palette,
        "SHEETSET" => state.show_sheet_set,
        "FILETAB" => state.show_file_tabs,
        "LAYOUTTAB" => state.show_layout_tabs,
        id => active_tool.as_deref() == Some(id),
    }
}

// ── Button style ───────────────────────────────────────────────────────────

/// Tool-button chrome with motion. `glow` is the activation track (pulses to
/// 1 and settles at `motion::REST_GLOW` while engaged, fades to 0 on
/// deactivation); `hover_p` is the pointer track (0..1, eased both ways).
pub(super) fn tool_btn_style(
    theme: &Theme,
    is_active: bool,
    status: button::Status,
    glow: f32,
    hover_p: f32,
) -> button::Style {
    let glow = glow.clamp(0.0, 1.0);
    let hover_p = hover_p.clamp(0.0, 1.0);

    if let Some(style) = kit::tool_plate_style(theme, is_active, status, glow, hover_p) {
        return style;
    }

    // While engaged the aurora never drops below the resting micro-glow;
    // resting a pointer on it lights it fully, without a hard step.
    let aurora_a = if is_active {
        glow.max(hover_p).max(crate::ui::motion::REST_GLOW)
    } else {
        glow
    };
    // The kit owns the chrome: a tool that is engaged (or still fading out of
    // engagement) is the kit's `Engaged` role, a free one is `Ghost`. What the
    // kit cannot express here is that the ribbon drives a second motion track —
    // the activation glow has to keep fading after the pointer has left — so
    // that track washes the kit result rather than replacing it.
    let engaged = is_active || aurora_a > 0.02;
    let mut style = style_for(
        theme,
        status,
        if engaged { Role::Engaged } else { Role::Ghost },
        "",
    );
    if engaged {
        style.background = Some(tokens::aurora_gradient(theme, aurora_a));
        // The ring is one pixel whether or not a tool is engaged: the stroke is
        // painted inside the button bounds, so only its colour changes between
        // states and the icon never moves. It fades with the aurora on
        // deactivation instead of snapping off.
        style.border.color = tokens::aurora_edge(theme).scale_alpha(aurora_a);
        // Soft brand glow behind engaged controls — the light leaks past the
        // edge. Inactive hovers get a faint version so the pointer always has a
        // response, without shouting.
        let glow_shadow_a = if is_active {
            aurora_a
        } else {
            glow.max(0.45 * hover_p)
        };
        style.shadow = iced::Shadow {
            color: tokens::aurora_lo(theme).scale_alpha(glow_shadow_a),
            ..tokens::elevation(theme, tokens::LIFT_HOVER)
        };
    }
    style
}

/// Wraps a button so its hover state drives the motion registry; the style
/// closure then reads interpolated progress for `key`.
pub(super) fn hover_tracked<'a>(
    key: &'static str,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    mouse_area(content)
        .on_enter(Message::UiHover {
            key,
            entered: true,
        })
        .on_exit(Message::UiHover {
            key,
            entered: false,
        })
        .into()
}

/// The one mark a ribbon control wears when it has more choices behind it:
/// a bare caret one hairline under the tool's name, centred, with no plate,
/// no pill and no fill of its own.
///
/// The mark speaks the cell's own language. Its ink is the label ink — muted
/// at rest, mixed toward the primary ink on hover, accent while its own list
/// is open — so it never reads as a second, brighter control parked inside the
/// cell. A running tool is the cell's state, not the mark's: it washes the
/// tile and inks the name, and deliberately leaves the caret alone, so the
/// caret only ever answers its own pointer and its own list. The band it lives
/// in is lifted to sit just below the name (see [`option_mark_lift`]) and is
/// the hit target: the full cell width × 14 px. The pointer feedback comes
/// from the cell's own hover wash, not from a second rectangle: the caller
/// wraps the face and this mark in one hover area, so the cell lights as a
/// unit wherever the pointer lands.
pub(super) fn option_mark<'a>(
    id: &str,
    tip: String,
    hover_key: &'static str,
    open: bool,
    lift: f32,
    theme: &Theme,
) -> Element<'a, Message> {
    let ink = move |theme: &Theme| {
        let Some(colors) = crate::ui::style::draftflow_theme::draftflow_colors(theme) else {
            return theme.palette().background.base.text;
        };
        if open {
            colors.accent
        } else {
            colors
                .muted
                .mix(colors.ink, crate::ui::motion::value(hover_key))
        }
    };
    let btn = plain_button(
        container(icons::themed_option_caret_by(OPTION_MARK_GLYPH, ink))
            .width(Fill)
            .height(Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Bottom)
            .padding(Padding {
                top: 0.0,
                right: 0.0,
                bottom: OPTION_MARK_INSET_BOTTOM,
                left: 0.0,
            }),
    )
    .on_press(Message::ToggleRibbonDropdown(id.to_string()))
    .style(|_theme: &Theme, _status| button::Style::default())
    .width(Length::Fill)
    .height(Length::Fixed(OPTION_MARK_H))
    .padding(tokens::GAP_NONE);
    let band = container(btn)
        .width(Fill)
        .height(Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Bottom)
        .padding(Padding {
            top: 0.0,
            right: 0.0,
            bottom: lift,
            left: 0.0,
        });
    ribbon_tip(band, tip, TipPos::Bottom, tokens::GAP_CONTROL, theme)
}

/// A ribbon combo — the layer selector, the property pickers, the undo history
/// split button — is an ordinary `Standard` plate that reads as `Engaged` for
/// exactly as long as its dropdown is open.
pub(super) fn combo_role(is_open: bool) -> Role {
    if is_open {
        Role::Engaged
    } else {
        Role::Standard
    }
}

// ── Tooltip helpers ────────────────────────────────────────────────────────

pub(super) fn make_tip(tip: String) -> Element<'static, Message> {
    text(tip).size(tokens::TEXT_LABEL).into()
}

/// One ribbon tip: the ink on a glass pane, held back 400 ms so a pointer
/// crossing a strip never lights a hundred of them, and offset by one of L0's
/// gap rungs.
///
/// The pane *reads* whatever the ribbon is standing over — the band, the
/// drawing, the panel below — so its four colours have to be resolved when the
/// tip is built: iced never hands a shader program the theme. That is what
/// `theme` is for, and why every renderer that can emit a tip carries it.
pub(super) fn ribbon_tip<'a>(
    target: impl Into<Element<'a, Message>>,
    tip: String,
    position: TipPos,
    gap: f32,
    theme: &Theme,
) -> Element<'a, Message> {
    tooltip(
        target,
        kit::glass_pane(
            kit::surface(make_tip(tip), kit::Tier::Flush, kit::PAD_PLATE, "").into(),
            theme,
            false,
            "",
        ),
        position,
    )
    .gap(gap)
    .padding(tokens::GAP_NONE)
    .delay(Duration::from_millis(400))
    .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""))
    .into()
}

/// The sentence the reference shows under a tool's name, where known.
fn tool_description(id: &str) -> Option<&'static str> {
    Some(match id {
        "DCLINEAR" => "Constrains the horizontal or vertical distance between points",
        "DCHORIZONTAL" => "Constrains the X-distance between points on an object, or between two points on different objects",
        "DCVERTICAL" => "Constrains the Y-distance between points on an object, or between two points on different objects",
        "DCALIGNED" => "Constrains the distance between two points on an object or between two points on different objects",
        "DCRADIUS" => "Constrains the radius of a circle or arc",
        "DCDIAMETER" => "Constrains the diameter of a circle or arc",
        "DCANGULAR" => "Constrains the angle between line or polyline segments, the angle swept out by an arc or a polyline arc segment, or the angle between three points on objects",
        "DCCONVERT" => "Convert dimensions to dimensional constraints",
        _ => return None,
    })
}

/// A tool's tooltip: its name, its description when one is known, and the
/// command it runs.
fn tool_tip_text(label: &str, id: &str) -> String {
    match tool_description(id) {
        Some(description) => format!("{label}\n{}\n{} {id}", t!(description), t!("Command:")),
        None => format!("{label}\n{} {id}", t!("Command:")),
    }
}

// ── Small item renderer ────────────────────────────────────────────────────

/// Render a 1-row small button (Tool or Dropdown).
pub(super) fn render_small<'a>(
    item: &RibbonItem,
    active_tool: &Option<String>,
    open_dd: &Option<String>,
    last_cmd: &HashMap<&'static str, &'static str>,
    state: ToggleState,
    theme: &Theme,
) -> Element<'a, Message> {
    match item {
        // Large variants render small too, so the ribbon can shrink a panel of
        // large buttons to icon-only columns when the width is tight.
        RibbonItem::Tool(t) | RibbonItem::LargeTool(t) => {
            let active = is_active_tool(t.id, active_tool, &state);
            let event = t.event.clone();
            let tool_id = t.id.to_string();
            let tip_text = tool_tip_text(&t!(t.label), t.id);
            let hover_key = crate::ui::motion::hover_key(t.id);
            let glow_key = crate::ui::motion::glow_key(t.id);
            let btn = plain_button(make_icon(t.icon, SMALL_ICON))
                .on_press(Message::RibbonToolClick { tool_id, event })
                .style(move |theme: &Theme, status| {
                    tool_btn_style(
                        theme,
                        active,
                        status,
                        crate::ui::motion::value(glow_key),
                        crate::ui::motion::value(hover_key),
                    )
                })
                .width(Length::Fixed(SMALL_W))
                .height(SMALL_H)
                .padding(tokens::GAP_HAIRLINE);
            ribbon_tip(
                hover_tracked(hover_key, btn),
                tip_text,
                TipPos::Right,
                tokens::GAP_CONTROL,
                theme,
            )
            .into()
        }

        RibbonItem::LabeledTool(t) => {
            let active = is_active_tool(t.id, active_tool, &state);
            let event = t.event.clone();
            let tool_id = t.id.to_string();
            let label = t!(t.label).into_owned();
            let tip_text = tool_tip_text(&label, t.id);
            let content = row![
                container(make_icon(t.icon, SMALL_ICON)).width(Length::Fixed(SMALL_W)),
                text(label).size(tokens::TEXT_BODY).wrapping(advanced_text::Wrapping::None),
            ]
            .spacing(tokens::GAP_TIGHT)
            .align_y(iced::Center);
            let hover_key = crate::ui::motion::hover_key(t.id);
            let glow_key = crate::ui::motion::glow_key(t.id);
            let btn = plain_button(content)
                .on_press(Message::RibbonToolClick { tool_id, event })
                .style(move |theme: &Theme, status| {
                    tool_btn_style(
                        theme,
                        active,
                        status,
                        crate::ui::motion::value(glow_key),
                        crate::ui::motion::value(hover_key),
                    )
                })
                .width(Length::Fixed(LABELED_SMALL_W + ARROW_W))
                .height(SMALL_H)
                .padding(kit::PAD_CELL);
            ribbon_tip(
                hover_tracked(hover_key, btn),
                tip_text,
                TipPos::Right,
                tokens::GAP_CONTROL,
                theme,
            )
            .into()
        }

        RibbonItem::Dropdown {
            id,
            icon,
            items,
            default,
            ..
        }
        | RibbonItem::LargeDropdown {
            id,
            icon,
            items,
            default,
            ..
        } => {
            let active = active_tool.as_deref() == Some(*id)
                || items
                    .iter()
                    .any(|(cmd, _, _)| active_tool.as_deref() == Some(*cmd));
            let dd_open = open_dd.as_deref() == Some(*id);
            let last = last_cmd.get(id).copied().unwrap_or(*default);
            let cur_icon = last_cmd
                .get(id)
                .copied()
                .and_then(|cmd| {
                    items
                        .iter()
                        .find(|(c, _, _)| *c == cmd)
                        .map(|(_, _, ik)| *ik)
                })
                .or_else(|| items.first().map(|(_, _, ik)| *ik))
                .unwrap_or(*icon);

            let cur_label = last_cmd
                .get(id)
                .copied()
                .and_then(|cmd| {
                    items
                        .iter()
                        .find(|(c, _, _)| *c == cmd)
                        .map(|(_, lbl, _)| *lbl)
                })
                .or_else(|| items.first().map(|(_, lbl, _)| *lbl))
                .unwrap_or(*id);
            let tip_text = tool_tip_text(&t!(cur_label), last);

            // Split control: face and arrow share one hover track, so the
            // pointer lights the whole control; the face's glow reads both
            // the dropdown id and the item that actually activated.
            let hover_key = crate::ui::motion::hover_key(*id);
            let dd_glow = crate::ui::motion::glow_key(*id);
            let last_glow = crate::ui::motion::glow_key(last);
            let icon_btn = plain_button(make_icon(cur_icon, SMALL_ICON))
                .on_press(Message::RibbonToolClick {
                    tool_id: last.to_string(),
                    event: ModuleEvent::Command(last.to_string()),
                })
                .style(move |theme: &Theme, status| {
                    tool_btn_style(
                        theme,
                        active,
                        status,
                        crate::ui::motion::value(dd_glow).max(crate::ui::motion::value(last_glow)),
                        crate::ui::motion::value(hover_key),
                    )
                })
                .width(Length::Fixed(SMALL_W))
                .height(SMALL_H)
                .padding(tokens::GAP_HAIRLINE);

            let arr_tip = format!("{} {}", t!(cur_label), t!("options"));
            let arr_btn = plain_button(
                container(icons::themed_arrow_down(8.0))
                    .width(Fill)
                    .height(Fill)
                    .align_x(iced::Center)
                    .align_y(iced::Center),
            )
            .on_press(Message::ToggleRibbonDropdown(id.to_string()))
            .style(move |theme: &Theme, status| {
                tool_btn_style(
                    theme,
                    dd_open,
                    status,
                    crate::ui::motion::value(dd_glow),
                    crate::ui::motion::value(hover_key),
                )
            })
            .width(Length::Fixed(ARROW_W))
            .height(SMALL_H)
            .padding(tokens::GAP_NONE);

            let icon_with_tip = ribbon_tip(
                hover_tracked(hover_key, icon_btn),
                tip_text,
                TipPos::Right,
                tokens::GAP_CONTROL,
                theme,
            );
            let arr_with_tip = ribbon_tip(
                hover_tracked(hover_key, arr_btn),
                arr_tip,
                TipPos::Right,
                tokens::GAP_CONTROL,
                theme,
            );

            PosReport::new(
                *id,
                row![icon_with_tip, arr_with_tip].spacing(tokens::GAP_NONE).height(SMALL_H),
            )
            .into()
        }

        RibbonItem::LabeledDropdown {
            id,
            label,
            icon,
            items,
            default,
        } => {
            let active = active_tool.as_deref() == Some(*id)
                || items
                    .iter()
                    .any(|(cmd, _, _)| active_tool.as_deref() == Some(*cmd));
            let dd_open = open_dd.as_deref() == Some(*id);
            let last = last_cmd.get(id).copied().unwrap_or(*default);
            let cur_icon = last_cmd
                .get(id)
                .copied()
                .and_then(|cmd| {
                    items
                        .iter()
                        .find(|(candidate, _, _)| *candidate == cmd)
                        .map(|(_, _, item_icon)| *item_icon)
                })
                .or_else(|| items.first().map(|(_, _, item_icon)| *item_icon))
                .unwrap_or(*icon);
            let localized_label = if label.is_empty() {
                items
                    .iter()
                    .find(|(cmd, _, _)| *cmd == last)
                    .map(|(_, item_label, _)| t!(*item_label).into_owned())
                    .unwrap_or_default()
            } else {
                t!(*label).into_owned()
            };
            let face = row![
                container(make_icon(cur_icon, SMALL_ICON)).width(Length::Fixed(SMALL_W)),
                text(localized_label.clone())
                    .size(tokens::TEXT_BODY)
                    .wrapping(advanced_text::Wrapping::None),
            ]
            .spacing(tokens::GAP_TIGHT)
            .align_y(iced::Center);
            let hover_key = crate::ui::motion::hover_key(*id);
            let dd_glow = crate::ui::motion::glow_key(*id);
            let last_glow = crate::ui::motion::glow_key(last);
            // A state that is only shown (not chosen) opens the list instead.
            let face_btn = plain_button(face)
                .on_press(if is_disabled_item(last) {
                    Message::ToggleRibbonDropdown(id.to_string())
                } else {
                    Message::RibbonToolClick {
                        tool_id: last.to_string(),
                        event: ModuleEvent::Command(last.to_string()),
                    }
                })
                .style(move |theme: &Theme, status| {
                    tool_btn_style(
                        theme,
                        active,
                        status,
                        crate::ui::motion::value(dd_glow).max(crate::ui::motion::value(last_glow)),
                        crate::ui::motion::value(hover_key),
                    )
                })
                .width(Length::Shrink)
                .height(SMALL_H)
                .padding(kit::PAD_CELL);
            let arrow = plain_button(
                container(icons::themed_arrow_down(8.0))
                    .width(Fill)
                    .height(Fill)
                    .align_x(iced::Center)
                    .align_y(iced::Center),
            )
            .on_press(Message::ToggleRibbonDropdown(id.to_string()))
            .style(move |theme: &Theme, status| {
                tool_btn_style(
                    theme,
                    dd_open,
                    status,
                    crate::ui::motion::value(dd_glow),
                    crate::ui::motion::value(hover_key),
                )
            })
            .width(Length::Fixed(ARROW_W))
            .height(SMALL_H)
            .padding(tokens::GAP_NONE);
            let face_tip = tool_tip_text(&localized_label, last);
            let arrow_tip = format!("{} {}", localized_label, t!("options"));
            PosReport::new(
                *id,
                row![
                    ribbon_tip(
                        hover_tracked(hover_key, face_btn),
                        face_tip,
                        TipPos::Right,
                        tokens::GAP_CONTROL,
                        theme,
                    ),
                    ribbon_tip(
                        hover_tracked(hover_key, arrow),
                        arrow_tip,
                        TipPos::Right,
                        tokens::GAP_CONTROL,
                        theme,
                    ),
                ]
                .spacing(tokens::GAP_NONE)
                .height(SMALL_H),
            )
            .into()
        }

        _ => text("").into(),
    }
}

// ── Sheet tool strip ──────────────────────────────────────────────────────

/// Icon size on the sheet's single-row tool strip.
pub(super) const STRIP_ICON: f32 = 20.0;
/// Width of one strip button (the sheet's `.tbtn` in its one-row dress).
pub(super) const STRIP_W: f32 = 62.0;
/// Height of one strip button: the icon, two lines of name, and the caret band
/// under them. Two lines rather than one because a translated name ("添加当前
/// 比例", "重新关联中心对象") is wider than a 62 px cell; a single unwrapped
/// line used to run under its neighbours. The extra line still fits the sheet's
/// 90 px band, so the row costs the drawing no height.
pub(super) const STRIP_H: f32 = 76.0;
/// Left and right insets of a strip label, i.e. what the plate takes from the
/// cell before the name gets a pixel.
pub(super) const STRIP_LABEL_HPAD: f32 = 2.0 * tokens::GAP_HAIRLINE;
/// Top inset of a strip plate. The name may take two lines, so the cluster sits
/// one rung higher than a one-line cell's did and the slack lands below it.
const STRIP_PAD_TOP: f32 = tokens::GAP_HAIRLINE;
/// Where the label block ends: top inset, icon, the gap to the name, and the
/// two lines the cell reserves for it.
fn strip_label_block_h() -> f32 {
    STRIP_PAD_TOP + STRIP_ICON + tokens::GAP_HAIRLINE + 2.0 * label_line_h(tokens::TEXT_LABEL)
}

/// The name a strip cell shows. A dropdown's face names its current choice, and
/// the variant suffix ("Rectangle - Two Corners") belongs to the full ribbon
/// and to the tooltip; on a 62 px cell it would only force a second line.
fn strip_face_label(full: &str) -> &str {
    full.split(" - ").next().unwrap_or(full)
}

/// The strip's own line-breaking hint. A CJK label wraps at any character, so
/// "圆心（半径）" split mid-word into "圆心（半" / "径）": an inserted space is
/// only one more break opportunity and the filler still prefers the fullest
/// line. A hard break before the opening bracket puts the register change on
/// its own line; Latin labels keep their own wrapping.
fn strip_break_label(full: &str) -> String {
    strip_face_label(full).replace('（', "\n（")
}

/// Whether [`render_strip`] can draw this item as a one-row button. A group's
/// overflow entry is offered only when the row still hides one of these; a
/// "More…" beside a group that is already fully shown is noise.
pub(super) fn strip_supported(item: &RibbonItem) -> bool {
    matches!(
        item,
        RibbonItem::Tool(_)
            | RibbonItem::LabeledTool(_)
            | RibbonItem::LargeTool(_)
            | RibbonItem::Dropdown { .. }
            | RibbonItem::LabeledDropdown { .. }
            | RibbonItem::LargeDropdown { .. }
    )
}

/// One button on the sheet's single-row tool strip: the icon above its name
/// and nothing else — no group plate, no rule, no group title. Compound items
/// (tool grids, combo groups) are the caller's to place and return `None` here.
///
/// A dropdown keeps both of its entries: pressing the face runs the command the
/// dropdown currently stands on, and a small ▾ plate hanging under the name
/// opens the list. That keeps every option reachable without the full ribbon.
pub(super) fn render_strip<'a>(
    item: &RibbonItem,
    ctx: &RenderCtx<'_>,
) -> Option<Element<'a, Message>> {
    let (icon, label, run_id, event, dd_id) = match item {
        RibbonItem::Tool(t) | RibbonItem::LabeledTool(t) | RibbonItem::LargeTool(t) => {
            (t.icon, t.label, t.id, t.event.clone(), None)
        }
        RibbonItem::Dropdown {
            id,
            icon,
            items,
            default,
            ..
        }
        | RibbonItem::LargeDropdown {
            id,
            icon,
            items,
            default,
            ..
        }
        | RibbonItem::LabeledDropdown {
            id,
            icon,
            items,
            default,
            ..
        } => {
            let last = ctx.last_cmd.get(id).copied().unwrap_or(*default);
            let (cur_icon, cur_label) = items
                .iter()
                .find(|(cmd, _, _)| *cmd == last)
                .map(|(_, lbl, ik)| (*ik, *lbl))
                .unwrap_or((*icon, *id));
            (
                cur_icon,
                cur_label,
                last,
                ModuleEvent::Command(last.to_string()),
                Some(*id),
            )
        }
        _ => return None,
    };

    let active = ctx.active_tool.as_deref() == Some(run_id)
        || dd_id.is_some_and(|id| ctx.active_tool.as_deref() == Some(id));
    // A dropdown's face and its option mark are one control: they share the
    // dropdown's hover track so the whole cell lights as a unit. A plain tool
    // tracks its own command.
    let track_id = dd_id.unwrap_or(run_id);
    let hover_key = crate::ui::motion::hover_key(track_id);
    let glow_key = crate::ui::motion::glow_key(run_id);
    // The strip's buttons are narrow: a dropdown's "Rectangle - Two Corners"
    // names its variant for the full ribbon, where there is room for it. On
    // the strip the variant is the tooltip's job; the face says "Rectangle".
    let full_label = t!(label).into_owned();
    let label_text = strip_break_label(&full_label);
    let tip_text = tool_tip_text(&full_label, run_id);

    let face = plain_button(
        column![
            container(make_icon(icon, STRIP_ICON))
                .width(Fill)
                .height(Length::Fixed(STRIP_ICON))
                .align_x(iced::Center)
                .align_y(iced::Center),
            text(label_text.clone())
                .size(tokens::TEXT_LABEL)
                .width(Fill)
                .align_x(iced::Center)
                .wrapping(advanced_text::Wrapping::Word),
        ]
        .align_x(iced::Center)
        .spacing(tokens::GAP_HAIRLINE)
        .width(Fill)
        .height(Fill),
    )
    .on_press(Message::RibbonToolClick {
        tool_id: run_id.to_string(),
        event,
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
    .width(Fill)
    .height(Fill)
    .padding(Padding {
        top: STRIP_PAD_TOP,
        right: tokens::GAP_HAIRLINE,
        bottom: tokens::GAP_HAIRLINE,
        left: tokens::GAP_HAIRLINE,
    });

    let face: Element<'a, Message> = ribbon_tip(
        face,
        tip_text,
        TipPos::Right,
        tokens::GAP_CONTROL,
        ctx.theme,
    );

    let cell: Element<'a, Message> = match dd_id {
        Some(id) => {
            let dd_open = ctx.open_dd.as_deref() == Some(id);
            let mark = option_mark(
                id,
                format!("{} {}", t!(label), t!("options")),
                hover_key,
                dd_open,
                option_mark_lift_below(
                    STRIP_H,
                    strip_label_block_h(),
                    tokens::GAP_TIGHT,
                ),
                ctx.theme,
            );
            // The face and its mark are one control, so they share one hover
            // area: two nested areas on the same key let the face's exit cancel
            // the mark's enter, leaving the cell unlit exactly when the pointer
            // is on the caret.
            hover_tracked(hover_key, iced::widget::stack![face, mark])
        }
        None => hover_tracked(hover_key, face),
    };

    let cell = container(cell)
        .width(Length::Fill)
        .height(Length::Fixed(STRIP_H));
    // Record where this button landed so its dropdown opens under *this*
    // cell. Without the report `dd_anchor` falls back to the ribbon's left
    // edge, which is why every strip dropdown used to fly out at x = 0.
    let cell: Element<'a, Message> = match dd_id {
        Some(id) => PosReport::new(id, cell).into(),
        None => cell.into(),
    };
    Some(automatic_strip_cell(label_text, cell))
}

// ── Large item renderer ────────────────────────────────────────────────────

/// The group's overflow entry at the end of the sheet's one-row strip: the row
/// shows a group's everyday commands and this opens the rest of that group.
///
/// It is a command of its own, not the ▾ of the tool beside it. It therefore
/// wears the same cell as every strip tool — icon over name, hover fill, no
/// resting plate — with the three-dot "more" glyph, so it reads as its own
/// button instead of another tool's caret. A caret here also collided with the
/// ▾ plates under real dropdowns and looked like one more of their triggers.
pub(super) fn render_strip_more<'a>(
    panel_id: &'static str,
    ctx: &RenderCtx<'_>,
) -> Element<'a, Message> {
    let hover_key = crate::ui::motion::hover_key(panel_id);
    let glow_key = crate::ui::motion::glow_key(panel_id);
    let open = ctx.open_dd.as_deref() == Some(panel_id);
    // The catalog's entry is "More…"; the trailing ellipsis marks a menu in
    // dialog copy. On a strip cell it reads as a truncation of the tool's own
    // name, so the label is the bare word here.
    let more_label = t!("More…")
        .trim_end_matches(|c: char| c == '…' || c == '.')
        .to_owned();
    let face = plain_button(
        column![
            container(icons::themed(icons::MORE, STRIP_ICON))
                .width(Fill)
                .height(Length::Fixed(STRIP_ICON))
                .align_x(iced::Center)
                .align_y(iced::Center),
            text(more_label.clone())
                .size(tokens::TEXT_LABEL)
                .width(Fill)
                .align_x(iced::Center)
                .wrapping(advanced_text::Wrapping::Word),
        ]
        .align_x(iced::Center)
        .spacing(tokens::GAP_HAIRLINE)
        .width(Fill)
        .height(Fill),
    )
    .on_press(Message::ToggleRibbonDropdown(panel_id.to_string()))
    .style(move |theme: &Theme, status| {
        tool_btn_style(
            theme,
            open,
            status,
            crate::ui::motion::value(glow_key),
            crate::ui::motion::value(hover_key),
        )
    })
    .width(Fill)
    .height(Fill)
    .padding(Padding {
        top: STRIP_PAD_TOP,
        right: tokens::GAP_HAIRLINE,
        bottom: tokens::GAP_HAIRLINE,
        left: tokens::GAP_HAIRLINE,
    });
    let face = ribbon_tip(
        hover_tracked(hover_key, face),
        more_label.clone(),
        TipPos::Right,
        tokens::GAP_CONTROL,
        ctx.theme,
    );
    let cell = container(face)
        .width(Length::Fill)
        .height(Length::Fixed(STRIP_H));
    // Report where the entry landed, the same way a strip dropdown does:
    // without it the group flyout falls back to the window's centre instead of
    // hanging under the edge it was opened from.
    automatic_strip_cell(more_label, PosReport::new(panel_id, cell).into())
}

/// Shared render context threaded through the large-item render path. Bundling
/// the per-view state keeps new fields from rippling through every render
/// signature and call site (same idea as `ToggleState`).
#[derive(Clone, Copy)]
pub(super) struct RenderCtx<'a> {
    pub active_tool: &'a Option<String>,
    /// The palette the ribbon is being drawn with. Tips are glass, and a glass
    /// pane resolves its colours when it is built, so every renderer that can
    /// emit one has to be handed this.
    pub theme: &'a Theme,
    pub open_dd: &'a Option<String>,
    pub last_cmd: &'a HashMap<&'static str, &'static str>,
    pub state: ToggleState,
    pub layer_infos: &'a [LayerInfo],
    pub active_layer: &'a str,
    pub active_color: AcadColor,
    pub active_linetype: &'a str,
    pub active_lineweight: LineWeight,
    pub style_ctx: &'a StyleContext<'a>,
    /// When compact, the Properties panel's Match button shrinks to a small icon.
    pub compact: bool,
}

/// A large dropdown button: the current icon on top, its ▾ directly beneath the
/// icon, then the label at the bottom. Shared by `LargeDropdown` / `Dropdown` in
/// the full ribbon and by a collapsed panel whose representative is a dropdown.
/// `explicit_label` overrides the derived (current-item) label when given.
pub(super) fn render_large_dropdown<'a>(
    id: &'static str,
    icon: IconKind,
    explicit_label: Option<&'a str>,
    items: &[(&'static str, &'static str, IconKind)],
    default: &'static str,
    ctx: &RenderCtx<'_>,
) -> Element<'a, Message> {
    let active_tool = ctx.active_tool;
    let open_dd = ctx.open_dd;
    let last_cmd = ctx.last_cmd;
    let theme = ctx.theme;
    let active = active_tool.as_deref() == Some(id)
        || items
            .iter()
            .any(|(cmd, _, _)| active_tool.as_deref() == Some(*cmd));
    let dd_open = open_dd.as_deref() == Some(id);
    let last = last_cmd.get(id).copied().unwrap_or(default);
    let cur_icon = last_cmd
        .get(id)
        .copied()
        .and_then(|cmd| {
            items
                .iter()
                .find(|(c, _, _)| *c == cmd)
                .map(|(_, _, ik)| *ik)
        })
        .or_else(|| items.first().map(|(_, _, ik)| *ik))
        .unwrap_or(icon);
    let cur_label = last_cmd
        .get(id)
        .copied()
        .and_then(|cmd| {
            items
                .iter()
                .find(|(c, _, _)| *c == cmd)
                .map(|(_, lbl, _)| *lbl)
        })
        .or_else(|| items.first().map(|(_, lbl, _)| *lbl))
        .unwrap_or(id);
    let label = t!(explicit_label.unwrap_or(cur_label)).into_owned();
    let tip_text = tool_tip_text(&t!(cur_label), last);
    let arr_tip = format!("{} {}", label, t!("options"));
    let hover_key = crate::ui::motion::hover_key(id);
    let dd_glow = crate::ui::motion::glow_key(id);
    let last_glow = crate::ui::motion::glow_key(last);

    // The label owns the bottom of the face. The icon's Fill container centers
    // it in all remaining space between the button top and the label.
    let top_btn = plain_button(
        column![
            container(make_icon(cur_icon, LARGE_ICON))
                .width(Fill)
                .height(Length::Fixed(LARGE_ICON))
                .align_x(iced::Center)
                .align_y(iced::Center),
            text(label.clone())
                .size(tokens::TEXT_BODY)
                .width(Fill)
                .align_x(iced::Center)
                .wrapping(advanced_text::Wrapping::Word),
        ]
        .align_x(iced::Center)
        .spacing(tokens::GAP_HAIRLINE)
        .width(Fill)
        .height(Fill),
    )
    .on_press(Message::RibbonToolClick {
        tool_id: last.to_string(),
        event: ModuleEvent::Command(last.to_string()),
    })
    .style(move |theme: &Theme, status| {
        tool_btn_style(
            theme,
            active,
            status,
            crate::ui::motion::value(dd_glow).max(crate::ui::motion::value(last_glow)),
            crate::ui::motion::value(hover_key),
        )
    })
    .width(Fill)
    .height(Fill)
    .padding(Padding {
        top: tokens::GAP_TIGHT,
        right: tokens::GAP_HAIRLINE,
        bottom: tokens::GAP_HAIRLINE,
        left: tokens::GAP_HAIRLINE,
    });

    let face = ribbon_tip(top_btn, tip_text, TipPos::Right, tokens::GAP_CONTROL, theme);
    // A large dropdown wears the same bottom-edge mark as a strip cell. It
    // used to carry a second full-width row under the face holding its own ▾,
    // which made every large dropdown twice as tall as a plain large tool and
    // put the caret furthest from the icon it belonged to; the mark now sits
    // in the dead band under the name, on the cell's own footprint.
    let mark = option_mark(
        id,
        arr_tip,
        hover_key,
        dd_open,
        option_mark_lift(LARGE_H, tokens::GAP_TIGHT, LARGE_ICON, LARGE_LABEL_SIZE),
        theme,
    );
    // One hover area for the face and its mark: the whole cell answers the
    // pointer, including the caret.
    let content: Element<'a, Message> = hover_tracked(
        hover_key,
        iced::widget::stack![face, mark]
            .width(Fill)
            .height(Length::Fixed(LARGE_H)),
    )
    .into();

    PosReport::new(id, automatic_large_button(label, content)).into()
}

/// A row of small tool buttons beneath a combo dropdown. Shared by the
/// Layers, Properties and style-combo panels so their icon rows render
/// identically.
fn tool_row<'a>(
    tools: &[ToolDef],
    active_tool: &Option<String>,
    theme: &Theme,
) -> Element<'a, Message> {
    let btns: Vec<Element<Message>> = tools
        .iter()
        .map(|t| {
            let is_active = active_tool.as_deref() == Some(t.id);
            let tip = t!(t.label);
            let event = t.event.clone();
            let icon_el: Element<Message> = make_icon(t.icon, 16.0);
            let msg = module_event_to_message(event);
            let hover_key = crate::ui::motion::hover_key(t.id);
            let glow_key = crate::ui::motion::glow_key(t.id);
            ribbon_tip(
                hover_tracked(
                    hover_key,
                    plain_button(icon_el)
                        .on_press(msg)
                        .style(move |theme: &Theme, status| {
                            tool_btn_style(
                                theme,
                                is_active,
                                status,
                                crate::ui::motion::value(glow_key),
                                crate::ui::motion::value(hover_key),
                            )
                        })
                        .padding(kit::PAD_GLYPH),
                ),
                tip.to_string(),
                TipPos::Right,
                tokens::GAP_TIGHT,
                theme,
            )
        })
        .collect();
    row(btns).spacing(tokens::GAP_HAIRLINE).align_y(iced::Center).into()
}

/// Fixed-height wrapper for a combo-style panel: dropdown on top, icon rows
/// below, all pinned to the panel's top edge so every panel's primary
/// dropdown shares one baseline.
fn combo_panel_col(width: f32, items: Vec<Element<'_, Message>>) -> Element<'_, Message> {
    container(column(items).spacing(COMBO_ROW_SPACING).align_x(iced::Left))
        .width(Length::Fixed(width))
        .height(Fill)
        .align_y(iced::Top)
        .padding(COMBO_PANEL_PAD)
        .into()
}

/// Render a full-height large button (LargeTool, LargeDropdown, LayerCombo, StyleCombo).
pub(super) fn render_large<'a>(item: &RibbonItem, ctx: &RenderCtx<'_>) -> Element<'a, Message> {
    let active_tool = ctx.active_tool;
    let open_dd = ctx.open_dd;
    let last_cmd = ctx.last_cmd;
    let theme = ctx.theme;
    let state = ctx.state;
    let layer_infos = ctx.layer_infos;
    let active_layer = ctx.active_layer;
    let active_color = ctx.active_color;
    let active_linetype = ctx.active_linetype;
    let active_lineweight = ctx.active_lineweight;
    let style_ctx = ctx.style_ctx;
    let compact = ctx.compact;
    match item {
        // A plain Tool renders large too, so a collapsed panel can show its
        // representative tool as a big icon.
        RibbonItem::LargeTool(t) | RibbonItem::Tool(t) | RibbonItem::LabeledTool(t) => {
            let active = is_active_tool(t.id, active_tool, &state);
            let event = t.event.clone();
            let tool_id = t.id.to_string();
            let label = t!(t.label).into_owned();
            let tip_text = tool_tip_text(&label, t.id);
            let hover_key = crate::ui::motion::hover_key(t.id);
            let glow_key = crate::ui::motion::glow_key(t.id);
            let btn = plain_button(
                column![
                    container(make_icon(t.icon, LARGE_ICON))
                        .width(Fill)
                        .height(Length::Fixed(LARGE_ICON))
                        .align_x(iced::Center)
                        .align_y(iced::Center),
                    text(label.clone())
                        .size(tokens::TEXT_BODY)
                        .width(Fill)
                        .align_x(iced::Center)
                        .wrapping(advanced_text::Wrapping::Word),
                ]
                .align_x(iced::Center)
                .spacing(tokens::GAP_HAIRLINE)
                .width(Fill)
                .height(Fill),
            )
            .on_press(Message::RibbonToolClick { tool_id, event })
            .style(move |theme: &Theme, status| {
                tool_btn_style(
                    theme,
                    active,
                    status,
                    crate::ui::motion::value(glow_key),
                    crate::ui::motion::value(hover_key),
                )
            })
            .width(Fill)
            .height(Length::Fixed(LARGE_H))
            .padding(Padding {
                top: tokens::GAP_TIGHT,
                right: tokens::GAP_HAIRLINE,
                bottom: tokens::GAP_NONE,
                left: tokens::GAP_HAIRLINE,
            });
            ribbon_tip(
                automatic_large_button(label, hover_tracked(hover_key, btn).into()),
                tip_text,
                TipPos::Right,
                tokens::GAP_CONTROL,
                theme,
            )
            .into()
        }

        RibbonItem::LargeDropdown {
            id,
            label,
            icon,
            items,
            default,
        } => render_large_dropdown(*id, *icon, Some(*label), items, *default, ctx),

        RibbonItem::LabeledDropdown {
            id,
            label,
            icon,
            items,
            default,
        } => render_large_dropdown(*id, *icon, Some(*label), items, *default, ctx),

        RibbonItem::ToolGrid { columns } => columns
            .iter()
            .fold(
                row![].spacing(tokens::GAP_HAIRLINE).height(Fill).align_y(iced::Top),
                |row, tools| {
                    let column = tools.iter().fold(
                        column![].spacing(tokens::GAP_NONE).width(Length::Fixed(SMALL_W)),
                        |column, tool| {
                            column.push(render_small(
                                &RibbonItem::Tool(tool.clone()),
                                active_tool,
                                open_dd,
                                last_cmd,
                                state,
                                theme,
                            ))
                        },
                    );
                    row.push(column)
                },
            )
            .into(),

        // A plain Dropdown renders large too (used by a collapsed panel whose
        // representative tool is a dropdown).
        RibbonItem::Dropdown {
            id,
            icon,
            items,
            default,
        } => render_large_dropdown(*id, *icon, None, items, *default, ctx),

        RibbonItem::LayerComboGroup { row2, row3 } => {
            const TOOL_BUTTON_W: f32 = 26.0;
            const TOOL_SPACING: f32 = 2.0;
            const GROUP_PADDING: f32 = 8.0;
            let tool_count = row2.len().max(row3.len()) as f32;
            let tools_w = tool_count * TOOL_BUTTON_W
                + (tool_count - 1.0).max(0.0) * TOOL_SPACING
                + GROUP_PADDING;
            // Wide enough to read a typical layer name (about 25 characters)
            // in the closed combo.
            let combo_w = (LARGE_W * 2.5).max(tools_w).max(250.0);

            let info = layer_infos.iter().find(|l| l.name == active_layer);
            let lc = info.map(|l| l.color).unwrap_or(Color::WHITE);
            let lv = info.map(|l| l.visible).unwrap_or(true);
            let lf = info.map(|l| l.frozen).unwrap_or(false);
            let ll = info.map(|l| l.locked).unwrap_or(false);
            let is_open = open_dd.as_deref() == Some(LAYER_COMBO_ID);

            let vis_icon = icons::semantic(icons::layer_visible(lv), 14.0);
            let freeze_icon = icons::semantic(icons::layer_freeze(lf), 14.0);
            let lock_icon = icons::semantic(icons::layer_lock(ll), 14.0);
            let swatch = container(text(""))
                .style(move |theme: &Theme| kit::swatch_style(theme, lc, ""))
                .width(12)
                .height(12);

            const FIXED_COMBO_W: f32 = 14.0 * 3.0 + 12.0 + 9.0 + 4.0 * 4.0 + 8.0 * 2.0;
            let name_w = (combo_w - FIXED_COMBO_W).max(24.0);
            // About 6 px per glyph at 11 px. The dropdown itself keeps the
            // complete layer name available; only its closed ribbon label is
            // shortened to preserve the fixed row height.
            let name_budget = ((name_w / 6.0) as usize).max(4);
            let active_layer_label = crate::ui::text_util::elided(
                active_layer,
                name_budget, ctx.theme, |shown| text(shown).size(tokens::TEXT_LABEL).into(),
            );

            let combo_btn = plain_button(
                row![
                    vis_icon,
                    freeze_icon,
                    lock_icon,
                    swatch,
                    container(active_layer_label)
                        .width(name_w)
                        .clip(true),
                    icons::themed_arrow_down(9.0),
                ]
                .spacing(tokens::GAP_TIGHT)
                .align_y(iced::Center),
            )
            .on_press(Message::ToggleRibbonDropdown(LAYER_COMBO_ID.to_string()))
            .style(move |theme: &Theme, status| style_for(theme, status, combo_role(is_open), ""))
            .padding(kit::PAD_TRIGGER)
            .width(Fill);

            combo_panel_col(
                combo_w,
                vec![
                    container(PosReport::new(LAYER_COMBO_ID, combo_btn))
                        .width(Fill)
                        .into(),
                    tool_row(row2, active_tool, theme),
                    tool_row(row3, active_tool, theme),
                ],
            )
        }

        RibbonItem::PropertiesGroup { match_prop } => {
            // The Match Properties button is a large icon normally, but shrinks
            // to a small icon when the panel is compacted.
            let mp_el: Element<'a, Message> = if compact {
                render_small(
                    &RibbonItem::Tool(match_prop.clone()),
                    active_tool,
                    open_dd,
                    last_cmd,
                    state,
                    theme,
                )
            } else {
                render_large(
                    &RibbonItem::LargeTool(match_prop.clone()),
                    &RenderCtx {
                        compact: false,
                        ..*ctx
                    },
                )
            };

            const PROP_W: f32 = 130.0;

            let prop_row = |label: String, dd_id: &'static str, swatch: Option<Color>| {
                let is_open = open_dd.as_deref() == Some(dd_id);
                let swatch_el: Element<'a, Message> = if let Some(c) = swatch {
                    container(text(""))
                        .style(move |theme: &Theme| kit::swatch_style(theme, c, ""))
                        .width(12)
                        .height(12)
                        .into()
                } else {
                    iced::widget::Space::new().width(tokens::GAP_NONE).into()
                };
                plain_button(
                    row![
                        swatch_el,
                        container(text(label).size(tokens::TEXT_MICRO)).width(Fill).clip(true),
                        if is_open {
                            icons::themed_arrow_up(8.0)
                        } else {
                            icons::themed_arrow_down(8.0)
                        },
                    ]
                    .spacing(tokens::GAP_TIGHT)
                    .align_y(iced::Center),
                )
                .on_press(Message::ToggleRibbonDropdown(dd_id.to_string()))
                .style(move |theme: &Theme, status| style_for(theme, status, combo_role(is_open), ""))
                .padding(kit::PAD_TRIGGER)
                .width(Length::Fixed(PROP_W))
            };

            let (color_swatch, _) = acad_color_display(active_color);
            let color_row = prop_row(
                crate::ui::color_select::color_display_name(active_color),
                PROP_COLOR_ID,
                Some(color_swatch),
            );
            let lt_row = prop_row(
                linetype_display_name(active_linetype),
                PROP_LINETYPE_ID,
                None,
            );
            let lw_row = prop_row(LwItem(active_lineweight).to_string(), PROP_LW_ID, None);

            // Top-pinned like every combo panel, so the Color dropdown shares
            // the same baseline as the Layers / style selectors.
            let combos = container(
                column![
                    PosReport::new(PROP_COLOR_ID, color_row),
                    PosReport::new(PROP_LINETYPE_ID, lt_row),
                    PosReport::new(PROP_LW_ID, lw_row),
                ]
                .spacing(tokens::GAP_HAIRLINE)
                .align_x(iced::Left),
            )
            .height(Fill)
            .align_y(iced::Top)
            .padding(Padding {
                left: tokens::GAP_NONE,
                ..COMBO_PANEL_PAD
            });

            row![mp_el, combos]
                .spacing(tokens::GAP_TIGHT)
                .align_y(iced::Center)
                .height(Fill)
                .into()
        }

        RibbonItem::StyleComboGroup {
            style_key,
            combo_id,
            rows,
            ..
        } => {
            const STYLE_COMBO_W: f32 = LARGE_W * 2.3;
            let active: String = style_ctx.active_for(*style_key).to_string();
            let is_open = open_dd.as_deref() == Some(*combo_id);

            // ── combo button ──
            let combo_btn = plain_button(
                row![
                    container(text(active.clone()).size(tokens::TEXT_LABEL))
                        .width(Fill)
                        .clip(true),
                    if is_open {
                        icons::themed_arrow_up(9.0)
                    } else {
                        icons::themed_arrow_down(9.0)
                    },
                ]
                .spacing(tokens::GAP_TIGHT)
                .align_y(iced::Center),
            )
            .on_press(Message::ToggleRibbonDropdown(combo_id.to_string()))
            .style(move |theme: &Theme, status| style_for(theme, status, combo_role(is_open), ""))
            .padding(kit::PAD_TRIGGER)
            .width(Fill);

            // The open style list renders as a floating overlay
            // (`Ribbon::style_combo_overlay`) so it isn't clipped by the fixed
            // ribbon-row height — matching the Draw-tab dropdowns. (#153) It is
            // not a child of this row, so the row carries only its button.
            let mut col_items: Vec<Element<Message>> =
                vec![
                    container(row![PosReport::new(*combo_id, combo_btn)].spacing(tokens::GAP_NONE))
                        .width(Fill)
                        .into(),
                ];
            for row_tools in rows {
                col_items.push(tool_row(row_tools, active_tool, theme));
            }

            combo_panel_col(STYLE_COMBO_W, col_items)
        }
    }
}

// ── Message helpers ────────────────────────────────────────────────────────

#[allow(dead_code)]
pub fn module_event_to_message(event: ModuleEvent) -> Message {
    match event {
        ModuleEvent::Command(cmd) => Message::Command(cmd),
        ModuleEvent::OpenFileDialog => Message::OpenFile,
        ModuleEvent::ClearModels => Message::ClearScene,
        ModuleEvent::SetVisualStyle(name) => {
            match crate::modules::view::visual_style::mode_for_keyword(&name) {
                Some(mode) => Message::SetRenderMode(mode),
                // Report through the command line rather than silently doing
                // nothing, so a typo surfaces there.
                None => Message::Command(format!("VISUALSTYLES {name}")),
            }
        }
        ModuleEvent::ToggleLayers => Message::ToggleLayers,
    }
}

// ── History control ────────────────────────────────────────────────────────

/// Quick-access chrome button (New / Open / Save / Save As / Print) in the top
/// strip: an SVG icon that dispatches a command string, with a hover tooltip.
pub(super) fn quick_access_btn<'a>(
    icon_bytes: &'static [u8],
    label: &'static str,
    cmd: &'static str,
    theme: &Theme,
) -> Element<'a, Message> {
    // The glyph sits unframed in the strip: muted at rest, ink under the
    // pointer, the way the sheet draws every chrome icon button.
    let hover_key = crate::ui::motion::hover_key(cmd);
    let icon = icons::themed_by(icon_bytes, 16.0, move |theme| {
        let p = crate::ui::motion::value(hover_key);
        match crate::ui::style::draftflow_theme::draftflow_colors(theme) {
            Some(colors) => colors.muted.mix(colors.ink, p),
            None => theme.palette().background.base.text,
        }
    });
    let btn = plain_button(
        container(icon)
            .width(Fill)
            .height(Fill)
            .align_x(iced::Center)
            .align_y(iced::Center),
    )
    .on_press(Message::Command(cmd.to_string()))
    .style(move |theme: &Theme, status| {
        if crate::ui::style::draftflow_theme::draftflow_colors(theme).is_some() {
            style_for(theme, status, Role::Ghost, "")
        } else {
            kit::subtle_style(theme, status)
        }
    })
    .width(Length::Fixed(TOP_HIST_W))
    .height(24)
    .padding([tokens::GAP_HAIRLINE, tokens::GAP_NONE]);
    ribbon_tip(
        hover_tracked(hover_key, btn),
        t!(label).into_owned(),
        TipPos::Bottom,
        tokens::GAP_CONTROL,
        theme,
    )
    .into()
}

pub(super) fn render_history_control<'a>(
    label: &'static str,
    dropdown_id: &'static str,
    count: usize,
    open_dropdown: &Option<String>,
    theme: &Theme,
) -> Element<'a, Message> {
    let dd_open = open_dropdown.as_deref() == Some(dropdown_id);
    let active = count > 0;

    let main_btn = {
        let glyph = if dropdown_id == UNDO_HISTORY_ID {
            icons::themed_undo(15.0, active)
        } else {
            icons::themed_redo(15.0, active)
        };
        let btn = plain_button(
            container(glyph)
                .width(Fill)
                .height(Fill)
                .align_x(iced::Center)
                .align_y(iced::Center),
        )
        .style(move |theme: &Theme, status| style_for(theme, status, combo_role(dd_open), ""))
        .width(Length::Fixed(TOP_HIST_W))
        .height(24)
        .padding([tokens::GAP_HAIRLINE, tokens::GAP_NONE]);
        let btn = if active {
            if dropdown_id == UNDO_HISTORY_ID {
                btn.on_press(Message::Undo)
            } else {
                btn.on_press(Message::Redo)
            }
        } else {
            btn
        };
        ribbon_tip(
            btn,
            format!(
                "{}\n{}",
                t!(label),
                t!("%{count} steps available", count = count)
            ),
            TipPos::Right,
            tokens::GAP_CONTROL,
            theme,
        )
    };

    let arrow_btn = {
        let btn = plain_button(
            container(if active {
                icons::themed_arrow_down(8.0)
            } else {
                icons::themed_disabled_arrow_down(8.0)
            })
            .width(Fill)
            .height(Fill)
            .align_x(iced::Center)
            .align_y(iced::Center),
        )
        .style(move |theme: &Theme, status| style_for(theme, status, combo_role(dd_open), ""))
        .width(Length::Fixed(TOP_ARR_W))
        .height(24)
        .padding(tokens::GAP_NONE);
        let btn = if active {
            btn.on_press(Message::ToggleRibbonDropdown(dropdown_id.to_string()))
        } else {
            btn
        };
        ribbon_tip(
            btn,
            format!("{}", t!("%{label} history", label = t!(label))),
            TipPos::Right,
            tokens::GAP_CONTROL,
            theme,
        )
    };

    PosReport::new(dropdown_id, row![main_btn, arrow_btn].spacing(tokens::GAP_NONE)).into()
}

#[cfg(test)]
mod style_context_tests {
    use super::StyleContext;
    use crate::modules::StyleKey;

    #[test]
    fn style_context_borrowed_without_cloning() {
        // RED: should construct from borrowed slices/strs without cloning.
        let text_names = vec!["Standard".to_string(), "MyStyle".to_string()];
        let active_text = "Standard".to_string();
        let dim_names = vec!["DimStd".to_string()];
        let active_dim = "DimStd".to_string();
        let mleader_names: Vec<String> = vec![];
        let active_mleader = String::new();
        let table_names: Vec<String> = vec![];
        let active_table = String::new();

        let ctx = StyleContext {
            text_style_names: &text_names,
            active_text_style: &active_text,
            dim_style_names: &dim_names,
            active_dim_style: &active_dim,
            mleader_style_names: &mleader_names,
            active_mleader_style: &active_mleader,
            table_style_names: &table_names,
            active_table_style: &active_table,
        };
        assert_eq!(ctx.names_for(StyleKey::TextStyle), &text_names);
        assert_eq!(ctx.active_for(StyleKey::TextStyle), "Standard");
        assert_eq!(ctx.names_for(StyleKey::DimStyle), &dim_names);
    }

    #[test]
    fn style_context_no_clone_needed() {
        let names = vec!["A".to_string(), "B".to_string()];
        let active = "A".to_string();
        let empty: Vec<String> = vec![];
        let empty_s = String::new();
        let ctx = StyleContext {
            text_style_names: &names,
            active_text_style: &active,
            dim_style_names: &empty,
            active_dim_style: &empty_s,
            mleader_style_names: &empty,
            active_mleader_style: &empty_s,
            table_style_names: &empty,
            active_table_style: &empty_s,
        };
        assert_eq!(ctx.active_for(StyleKey::TableStyle), "");
        assert!(ctx.names_for(StyleKey::MLeaderStyle).is_empty());
    }
}

#[cfg(test)]
mod strip_layout_tests {
    use super::*;

    fn headless_renderer() -> iced::Renderer {
        iced_test::futures::futures::executor::block_on(
            <iced::Renderer as iced_test::core::renderer::Headless>::new(
                iced_test::core::renderer::Settings::default(),
                None,
            ),
        )
        .expect("headless renderer")
    }

    /// Every name a strip cell can put on its face. Tool and dropdown faces use
    /// the same `strip_face_label` the renderer does, so this gate measures the
    /// string the user actually sees.
    fn strip_faces(item: &RibbonItem) -> Vec<&'static str> {
        match item {
            RibbonItem::Tool(t) | RibbonItem::LabeledTool(t) | RibbonItem::LargeTool(t) => {
                vec![t.label]
            }
            RibbonItem::Dropdown { items, .. } => items.iter().map(|(_, l, _)| *l).collect(),
            RibbonItem::LabeledDropdown { label, items, .. }
            | RibbonItem::LargeDropdown { label, items, .. } => {
                let mut faces = vec![*label];
                faces.extend(items.iter().map(|(_, l, _)| *l));
                faces
            }
            _ => Vec::new(),
        }
    }

    /// The cell is two lines tall; a third line would run under the caret and
    /// past the plate. Walk every built-in module so a label added later fails
    /// here instead of on screen — `render_strip` narrows the same way.
    #[test]
    fn every_ribbon_label_fits_the_strip_cell() {
        let renderer = headless_renderer();
        let mut checked = 0_usize;
        let mut bad = Vec::new();

        for module in crate::modules::registry::all_modules() {
            for group in module.ribbon_groups() {
                for item in &group.tools {
                    if !strip_supported(item) {
                        continue;
                    }
                    for full in strip_faces(item) {
                        checked += 1;
                        let face = strip_break_label(full);
                        let width = automatic_strip_width(&renderer, &face);
                        let inner = width - STRIP_LABEL_HPAD;
                        let bounds = ribbon_label_bounds(
                            &renderer,
                            &face,
                            inner,
                            tokens::TEXT_LABEL,
                            advanced_text::Wrapping::Word,
                        );
                        let max_h = 2.0 * label_line_h(tokens::TEXT_LABEL) + 0.5;
                        if bounds.width > inner + 0.5
                            || bounds.height > max_h
                            || width < STRIP_W
                        {
                            bad.push(format!(
                                "{face:?}: cell {width} (inner {inner}), ink {:?}",
                                bounds
                            ));
                        }
                    }
                }
            }
        }

        assert!(
            checked >= 30,
            "the walk saw only {checked} strip faces, so it is broken"
        );
        assert!(
            bad.is_empty(),
            "{} strip labels need a third line or spill sideways:\n{}",
            bad.len(),
            bad.join("\n")
        );
    }

    /// The two-line label block and the caret band must both fit the cell, and
    /// the cell must fit the sheet's tool band. A caret that clamps to the
    /// bottom edge sits on the second line; that is what this catches.
    #[test]
    fn strip_cell_reserves_room_for_two_lines_and_the_caret() {
        let label_h = strip_label_block_h();
        let lift = option_mark_lift_below(STRIP_H, label_h, tokens::GAP_TIGHT);
        let band_top = STRIP_H - lift - OPTION_MARK_H;
        let ink_top = STRIP_H - lift - OPTION_MARK_INSET_BOTTOM - option_mark_ink_h();

        assert!(
            lift > 0.0,
            "the caret is pinned to the cell's bottom edge (lift {lift})"
        );
        assert!(
            ink_top >= label_h + tokens::GAP_HAIRLINE,
            "caret ink (top {ink_top}) collides with the two-line label block ({label_h})"
        );
        assert!(
            band_top >= label_h - 3.0,
            "caret hit band (top {band_top}) covers the label's last line ({label_h})"
        );
        assert!(
            STRIP_H + 2.0 * tokens::GAP_CONTROL <= TOOL_BAR_H,
            "a {STRIP_H} px strip cell no longer fits the sheet's {TOOL_BAR_H} px band"
        );
    }
}
