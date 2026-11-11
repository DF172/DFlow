//! Shared SVG rendering for monochrome UI chrome and multi-colour tool icons.
//!
//! Dropdown carets and the undo/redo controls used to be drawn as Unicode
//! glyphs (`▾`, `▲`, `↶`, `↷`). Those depend on the active text font carrying
//! the glyph: on desktop the system fallback fonts supply them, but the web
//! build bundles only Fira Sans, which lacks them, so they rendered as empty
//! boxes. Drawing them from SVG instead makes the chrome font-independent.

use std::cell::RefCell;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::svg::{self as core_svg, Renderer as _};
use iced::advanced::widget::{Tree, Widget};
use iced::widget::{container, svg, Space};
use iced::{Color, ContentFit, Element, Length, Point, Radians, Rectangle, Renderer, Size, Theme};
use rustc_hash::FxHashMap;

use crate::ui::style::common::{accessible_accent, wcag_contrast};

static TRI_DOWN: &[u8] = include_bytes!("../../assets/icons/ui/tri_down.svg");
/// Fuller caret for the option mark a ribbon cell wears in its top-right
/// corner. `TRI_DOWN`'s 12×7 triangle leaves a sliver at 12 px; this one is
/// 16×10 for the same element size, so the mark is a target you can see.
static OPTION_CARET: &[u8] = include_bytes!("../../assets/icons/ui/caret_option.svg");
static TRI_UP: &[u8] = include_bytes!("../../assets/icons/ui/tri_up.svg");
static TRI_RIGHT: &[u8] = include_bytes!("../../assets/icons/ui/tri_right.svg");
static TRI_LEFT: &[u8] = include_bytes!("../../assets/icons/ui/tri_left.svg");
static HOME: &[u8] = include_bytes!("../../assets/icons/ui/home.svg");
static UNDO: &[u8] = include_bytes!("../../assets/icons/ui/undo.svg");
static REDO: &[u8] = include_bytes!("../../assets/icons/ui/redo.svg");

// OSNAP marker symbols. Rendered as SVG (not Unicode glyphs) so the snap menu
// shows the right shapes on the web build, whose bundled Fira Sans lacks the
// geometric glyphs and rendered them as tofu boxes. (#138)
static OSNAP_ENDPOINT: &[u8] = include_bytes!("../../assets/icons/osnap/endpoint.svg");
static OSNAP_MIDPOINT: &[u8] = include_bytes!("../../assets/icons/osnap/midpoint.svg");
static OSNAP_CENTER: &[u8] = include_bytes!("../../assets/icons/osnap/center.svg");
static OSNAP_GEOMETRIC_CENTER: &[u8] =
    include_bytes!("../../assets/icons/osnap/geometric_center.svg");
static OSNAP_NODE: &[u8] = include_bytes!("../../assets/icons/osnap/node.svg");
static OSNAP_QUADRANT: &[u8] = include_bytes!("../../assets/icons/osnap/quadrant.svg");
static OSNAP_INTERSECTION: &[u8] = include_bytes!("../../assets/icons/osnap/intersection.svg");
static OSNAP_EXTENSION: &[u8] = include_bytes!("../../assets/icons/osnap/extension.svg");
static OSNAP_INSERTION: &[u8] = include_bytes!("../../assets/icons/osnap/insertion.svg");
static OSNAP_PERPENDICULAR: &[u8] = include_bytes!("../../assets/icons/osnap/perpendicular.svg");
static OSNAP_TANGENT: &[u8] = include_bytes!("../../assets/icons/osnap/tangent.svg");
static OSNAP_NEAREST: &[u8] = include_bytes!("../../assets/icons/osnap/nearest.svg");
static OSNAP_APPARENT: &[u8] = include_bytes!("../../assets/icons/osnap/apparent.svg");
static OSNAP_PARALLEL: &[u8] = include_bytes!("../../assets/icons/osnap/parallel.svg");
static OSNAP_GRID: &[u8] = include_bytes!("../../assets/icons/osnap/grid.svg");
static OSNAP_MTP: &[u8] = include_bytes!("../../assets/icons/osnap/mtp.svg");
static NAV_PAN: &[u8] = include_bytes!("../../assets/icons/pan.svg");
static NAV_ZOOM: &[u8] = include_bytes!("../../assets/icons/zoom_in.svg");

static LAY_ON: &[u8] = include_bytes!("../../assets/icons/layers/layon.svg");
static LAY_OFF: &[u8] = include_bytes!("../../assets/icons/layers/layoff.svg");
static LAY_FRZ: &[u8] = include_bytes!("../../assets/icons/layers/layfrz.svg");
static LAY_THW: &[u8] = include_bytes!("../../assets/icons/layers/laythw.svg");
static LAY_VP_FRZ: &[u8] = include_bytes!("../../assets/icons/layers/layvpfrz.svg");
static LAY_VP_THW: &[u8] = include_bytes!("../../assets/icons/layers/layvpthw.svg");
static LAY_LCK: &[u8] = include_bytes!("../../assets/icons/layers/laylck.svg");
static LAY_ULK: &[u8] = include_bytes!("../../assets/icons/layers/layulk.svg");

// Monochrome chrome glyphs (replace Unicode glyphs in buttons / menus / toolbars).
// All are black-on-transparent; recolour them at the call site with [`tinted`].
pub static CHECK: &[u8] = include_bytes!("../../assets/icons/ui/check.svg");
pub static CLOSE: &[u8] = include_bytes!("../../assets/icons/ui/close.svg");
pub static PLUS: &[u8] = include_bytes!("../../assets/icons/ui/plus.svg");
pub static REFRESH: &[u8] = include_bytes!("../../assets/icons/ui/refresh.svg");
pub static SWAP: &[u8] = include_bytes!("../../assets/icons/ui/swap.svg");
pub static MINUS: &[u8] = include_bytes!("../../assets/icons/ui/minus.svg");
pub static TRASH: &[u8] = include_bytes!("../../assets/icons/ui/trash.svg");
pub static NODE_GRAPH: &[u8] = include_bytes!("../../assets/icons/ui/node_graph.svg");
pub static COPY: &[u8] = include_bytes!("../../assets/icons/ui/copy.svg");
pub static MENU: &[u8] = include_bytes!("../../assets/icons/ui/menu.svg");
pub static MOVE: &[u8] = include_bytes!("../../assets/icons/ui/move.svg");
pub static RESIZE: &[u8] = include_bytes!("../../assets/icons/ui/resize.svg");
/// `RESIZE` mirrored for a bottom-left corner grip.
pub static RESIZE_LEFT: &[u8] = include_bytes!("../../assets/icons/ui/resize_left.svg");
pub static PIN: &[u8] = include_bytes!("../../assets/icons/ui/pin.svg");
/// `PIN` tilted 30°: the pin while auto-hide is on.
pub static PIN_ACTIVE: &[u8] = include_bytes!("../../assets/icons/ui/pin_active.svg");
pub static SPLIT_V: &[u8] = include_bytes!("../../assets/icons/ui/split_v.svg");
pub static SPLIT_H: &[u8] = include_bytes!("../../assets/icons/ui/split_h.svg");
pub static GRID: &[u8] = include_bytes!("../../assets/icons/ui/grid.svg");
pub static SNAP: &[u8] = include_bytes!("../../assets/icons/ui/snap.svg");
pub static DOC_NEW: &[u8] = include_bytes!("../../assets/icons/ui/doc_new.svg");
pub static DOC: &[u8] = include_bytes!("../../assets/icons/ui/doc.svg");
pub static FOLDER_OPEN: &[u8] = include_bytes!("../../assets/icons/ui/folder_open.svg");
pub static SAVE: &[u8] = include_bytes!("../../assets/icons/ui/save.svg");
pub static FILE_EXPORT: &[u8] = include_bytes!("../../assets/icons/ui/file_export.svg");
pub static PRINT: &[u8] = include_bytes!("../../assets/icons/ui/print.svg");
pub static GEAR: &[u8] = include_bytes!("../../assets/icons/ui/gear.svg");
pub static PLUGIN: &[u8] = include_bytes!("../../assets/icons/ui/bolt.svg");
pub static DOT: &[u8] = include_bytes!("../../assets/icons/ui/dot.svg");
pub static DIRTY_DOT: &[u8] = include_bytes!("../../assets/icons/ui/dirty_dot.svg");
pub static ARROW_LONG_RIGHT: &[u8] = include_bytes!("../../assets/icons/ui/arrow_long_right.svg");
pub static MORE: &[u8] = include_bytes!("../../assets/icons/ui/more.svg");
pub static INFO: &[u8] = include_bytes!("../../assets/icons/ui/info.svg");
/// Warning triangle: the non-colour half of a [`crate::ui::kit::Tone::Warning`]
/// verdict (see `kit::feedback`).
pub static WARNING: &[u8] = include_bytes!("../../assets/icons/ui/warning_triangle.svg");
/// Half-filled disc: the light/dark scheme switch.
pub static THEME_TOGGLE: &[u8] = include_bytes!("../../assets/icons/ui/theme_toggle.svg");

// ── Status-bar toggle icons (issue #216) ──────────────────────────────────
pub static ST_ORTHO: &[u8] = include_bytes!("../../assets/icons/status/ortho.svg");
pub static ST_POLAR: &[u8] = include_bytes!("../../assets/icons/status/polar.svg");
pub static ST_OSNAP: &[u8] = include_bytes!("../../assets/icons/status/osnap.svg");
pub static ST_OTRACK: &[u8] = include_bytes!("../../assets/icons/status/otrack.svg");
pub static ST_DYN: &[u8] = include_bytes!("../../assets/icons/status/dyn.svg");
pub static ST_LWT: &[u8] = include_bytes!("../../assets/icons/status/lwt.svg");
pub static ST_TRANSPARENCY: &[u8] = include_bytes!("../../assets/icons/status/transparency.svg");
pub static ST_ISOLATE: &[u8] = include_bytes!("../../assets/icons/status/isolate.svg");
pub static ST_QUICKPROPS: &[u8] = include_bytes!("../../assets/icons/status/quickprops.svg");
pub static ST_FILTER: &[u8] = include_bytes!("../../assets/icons/status/filter.svg");
pub static ST_SELCYCLE: &[u8] = include_bytes!("../../assets/icons/status/selcycle.svg");
pub static ST_CLEANSCREEN: &[u8] = include_bytes!("../../assets/icons/status/cleanscreen.svg");

// Tool SVGs share a small source palette. These colours are semantic rather
// than literal: cyan is the accent, pale grey is foreground, yellow is warning,
// and so on. `SemanticIcon` resolves those roles from Iced's active extended
// palette at draw time, retaining the artwork's multiple colours across themes.
const SEMANTIC_CACHE_LIMIT: usize = 4096;

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
struct SemanticCacheKey {
    address: usize,
    length: usize,
    size: u32,
    palette: [[u8; 4]; 6],
}

thread_local! {
    static SEMANTIC_CACHE: RefCell<FxHashMap<SemanticCacheKey, svg::Handle>> =
        RefCell::new(FxHashMap::default());
}

// Per-thread cache of `svg::Handle`s keyed by the bytes'
// `(address, length)`. See the doc comment on [`themed_handle`]
// below for the cache key rationale, the sub-slice guard, the
// mirror of `SemanticCacheKey`, and the threading choice.
#[derive(Default)]
struct ThemedCache {
    mru: [Option<((usize, usize, u32), svg::Handle)>; 2],
    map: FxHashMap<(usize, usize, u32), svg::Handle>,
}

thread_local! {
    static THEMED_CACHE: RefCell<ThemedCache> =
        RefCell::new(ThemedCache::default());
}

/// Look up (or build and cache) the `svg::Handle` for a `&'static [u8]`
/// passed to a `themed*` function. This is the single source of
/// truth for the per-`bytes` handle in the `themed` icon family —
/// the 9 `themed*` functions all go through here, so the 22
/// `themed*` call sites (and the 113 total `icons::themed`
/// references) share one entry per artwork.
///
/// **Cache key: `(address, length)`.** Every `include_bytes!`
/// produces a unique `&'static` slice, and the address is stable
/// for the program's lifetime. Including the length guards against
/// any future call site that passes a sub-slice of a static
/// (e.g. `&BYTES[1..]`), which would otherwise alias the wrong
/// handle. Mirrors the proven `SemanticCacheKey` belt-and-braces
/// pattern above.
///
/// **`themed` vs `semantic` — why the simpler key is correct.**
/// The `semantic` family bakes the palette into the SVG
/// (recolours the artwork per theme role), so its cache key
/// includes the palette. The `themed` family reads its colour
/// from an Iced style closure at draw time, so the handle
/// depends only on the bytes — a single entry per artwork is
/// correct and shared across all themes.
///
/// **Threading.** The cache is `thread_local!` because `svg::Handle`
/// is not `Sync`. The set of `&'static [u8]` art is fixed at
/// compile time (~25-30 `include_bytes!` entries), so the cache
/// is bounded by construction and needs no eviction. Per-frame
/// cost: 9 ns/iter warm, 26 ns/iter cold (release profile, see
/// `themed_cache_speedup_over_uncached_handle_creation`).
///
/// **No borrow held during handle construction.** The cache is
/// read (immutable `borrow`), the `Ref` is dropped, and only then
/// is `svg::Handle::from_memory` invoked. The result is then
/// inserted under a fresh `borrow_mut`. This is defensive: a
/// re-entrant call into `themed*` during SVG parsing cannot
/// trigger a `RefCell` panic on the second `borrow_mut`, because
/// no `borrow_mut` is held while parsing runs. (The same
/// `thread_local!` + `RefCell` shape is used by `SEMANTIC_CACHE`.)
#[inline]
#[doc(hidden)]
pub fn themed_handle(bytes: &'static [u8], size: f32) -> svg::Handle {
    let key = (
        bytes.as_ptr() as usize,
        bytes.len(),
        (size.max(0.0) * 100.0).round() as u32,
    );
    let hit = THEMED_CACHE.with(|cache| {
        let mut c = cache.borrow_mut();
        if let Some((k, ref handle)) = c.mru[0] {
            if k == key {
                return Some(handle.clone());
            }
        }
        if let Some((k, _)) = c.mru[1] {
            if k == key {
                c.mru.swap(0, 1);
                return Some(c.mru[0].as_ref().unwrap().1.clone());
            }
        }
        if let Some(handle) = c.map.get(&key) {
            let handle = handle.clone();
            c.mru[1] = c.mru[0].take();
            c.mru[0] = Some((key, handle.clone()));
            return Some(handle);
        }
        None
    });
    if let Some(handle) = hit {
        return handle;
    }

    // Miss: build the handle with no cache borrow held, then insert.
    let handle = svg::Handle::from_memory(scale_strokes(bytes, size));
    THEMED_CACHE.with(|cache| {
        let mut c = cache.borrow_mut();
        c.mru[1] = c.mru[0].take();
        c.mru[0] = Some((key, handle.clone()));
        c.map.insert(key, handle.clone());
    });
    handle
}

#[derive(Clone, Copy)]
struct SemanticColors {
    background: [u8; 7],
    text: [u8; 7],
    primary_weak: [u8; 7],
    primary: [u8; 7],
    primary_strong: [u8; 7],
    secondary_weak: [u8; 7],
    secondary: [u8; 7],
    secondary_strong: [u8; 7],
    success_weak: [u8; 7],
    success: [u8; 7],
    warning: [u8; 7],
    warning_strong: [u8; 7],
    danger_weak: [u8; 7],
    danger: [u8; 7],
    /// Per-tool-category hue ramp for the multi-colour DraftFlow tool icons.
    /// Index order is the hue code: cyan, blue, purple, green, amber,
    /// orange, magenta, teal. Each entry carries the four role slots used by
    /// the artwork (gradient top, gradient bottom, extrusion wall, outline).
    hues: [HueSet; 8],
}

/// One tool-icon colour ramp: `hi`/`lo` feed the vertical top-light
/// gradient, `wall` paints the 1.25px extrusion layer, `edge` outlines.
#[derive(Clone, Copy)]
struct HueSet {
    hi: [u8; 7],
    lo: [u8; 7],
    wall: [u8; 7],
    edge: [u8; 7],
}

impl HueSet {
    const fn of(hi: &str, lo: &str, wall: &str, edge: &str) -> Self {
        let h = hi.as_bytes();
        let l = lo.as_bytes();
        let w = wall.as_bytes();
        let e = edge.as_bytes();
        Self {
            hi: [h[0], h[1], h[2], h[3], h[4], h[5], h[6]],
            lo: [l[0], l[1], l[2], l[3], l[4], l[5], l[6]],
            wall: [w[0], w[1], w[2], w[3], w[4], w[5], w[6]],
            edge: [e[0], e[1], e[2], e[3], e[4], e[5], e[6]],
        }
    }
}

impl SemanticColors {
    fn from_theme(theme: &Theme) -> Self {
        let palette = theme.palette();
        let hues = if palette.is_dark {
            [
                HueSet::of("#7FE7FF", "#00A8E8", "#0B4A66", "#0E7FB0"),
                HueSet::of("#8FD0FF", "#3D7FE8", "#132C55", "#2B5FB8"),
                HueSet::of("#B49BFF", "#7A5CFF", "#2E2366", "#5A46C8"),
                HueSet::of("#6FFFD2", "#00B894", "#0A4F40", "#0E8F72"),
                HueSet::of("#FFD27F", "#E8862E", "#5C3510", "#B86A1E"),
                HueSet::of("#FF9F7F", "#F0553D", "#5E2418", "#C0422E"),
                HueSet::of("#FF8FD8", "#D84FA8", "#56224A", "#B03E86"),
                HueSet::of("#9FFFA0", "#2EBB5E", "#14502C", "#1E9450"),
            ]
        } else {
            [
                HueSet::of("#5FD8FF", "#0086C9", "#0B6CA6", "#0E7FB0"),
                HueSet::of("#6FB2FF", "#2B6BD8", "#1D4E9E", "#1B4A8C"),
                HueSet::of("#9F82FF", "#6A4BE0", "#4A38A8", "#5A46C8"),
                HueSet::of("#4FE8B8", "#00997A", "#0E7A62", "#0E8F72"),
                HueSet::of("#FFC060", "#D06A1E", "#B25A16", "#B86A1E"),
                HueSet::of("#FF8A66", "#D8432B", "#B03422", "#C0422E"),
                HueSet::of("#F57AC0", "#C23A8E", "#9C2E72", "#B03E86"),
                HueSet::of("#7FE88A", "#1FA04A", "#167C38", "#1E9450"),
            ]
        };
        Self {
            background: color_hex(palette.background.strong.color),
            text: color_hex(palette.background.base.text),
            primary_weak: color_hex(palette.primary.weak.color),
            primary: color_hex(palette.primary.base.color),
            primary_strong: color_hex(palette.primary.strong.color),
            secondary_weak: color_hex(palette.secondary.weak.color),
            secondary: color_hex(palette.secondary.base.color),
            secondary_strong: color_hex(palette.secondary.strong.color),
            success_weak: color_hex(palette.success.weak.color),
            success: color_hex(palette.success.base.color),
            warning: color_hex(palette.warning.base.color),
            warning_strong: color_hex(palette.warning.strong.color),
            danger_weak: color_hex(palette.danger.weak.color),
            danger: color_hex(palette.danger.base.color),
            hues,
        }
    }
}

struct SemanticIcon {
    bytes: &'static [u8],
    size: f32,
    opacity: f32,
}

impl<M> Widget<M, Theme, Renderer> for SemanticIcon {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.size), Length::Fixed(self.size))
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, Length::Fixed(self.size), Length::Fixed(self.size))
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: iced::advanced::mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let handle = semantic_handle(self.bytes, self.size, theme);
        let measured = renderer.measure_svg(&handle);
        if measured.width == 0 || measured.height == 0 {
            return;
        }

        let image_size = Size::new(measured.width as f32, measured.height as f32);
        let bounds = layout.bounds();
        let fitted = ContentFit::Contain.fit(image_size, bounds.size());
        let position = Point::new(
            bounds.center_x() - fitted.width / 2.0,
            bounds.center_y() - fitted.height / 2.0,
        );

        renderer.draw_svg(
            core_svg::Svg {
                handle,
                color: None,
                rotation: Radians(0.0),
                opacity: self.opacity,
            },
            Rectangle::new(position, fitted),
            bounds,
        );
    }
}

/// Render a multi-colour tool icon using semantic colours from the active theme.
pub fn semantic<'a, M: 'a>(bytes: &'static [u8], size: f32) -> Element<'a, M> {
    Element::new(SemanticIcon {
        bytes,
        size,
        opacity: crate::ui::kit::alpha::current(),
    })
}

/// [`semantic`] faded like disabled menu text.
pub fn semantic_disabled<'a, M: 'a>(bytes: &'static [u8], size: f32) -> Element<'a, M> {
    Element::new(SemanticIcon {
        bytes,
        size,
        opacity: 0.42 * crate::ui::kit::alpha::current(),
    })
}

fn semantic_handle(bytes: &'static [u8], size: f32, theme: &Theme) -> svg::Handle {
    let key = SemanticCacheKey {
        address: bytes.as_ptr() as usize,
        length: bytes.len(),
        size: (size.max(0.0) * 100.0).round() as u32,
        palette: palette_key(theme),
    };

    SEMANTIC_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(handle) = cache.get(&key) {
            return handle.clone();
        }

        let recolored = recolor_semantic_svg(bytes, theme);
        let scaled = scale_strokes(&recolored, size);
        let handle = svg::Handle::from_memory(scaled);
        if cache.len() >= SEMANTIC_CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(key, handle.clone());
        handle
    })
}

fn palette_key(theme: &Theme) -> [[u8; 4]; 6] {
    let palette = theme.palette();
    [
        palette.background.base.color.into_rgba8(),
        palette.background.base.text.into_rgba8(),
        palette.primary.base.color.into_rgba8(),
        palette.success.base.color.into_rgba8(),
        palette.warning.base.color.into_rgba8(),
        palette.danger.base.color.into_rgba8(),
    ]
}

fn recolor_semantic_svg(source: &[u8], theme: &Theme) -> Vec<u8> {
    let colors = SemanticColors::from_theme(theme);
    let mut output = Vec::with_capacity(source.len());
    let mut index = 0;

    while index < source.len() {
        if source[index] == b'#' {
            let mut end = index + 1;
            while end < source.len() && source[end].is_ascii_hexdigit() {
                end += 1;
            }
            let digit_count = end - index - 1;
            if matches!(digit_count, 3 | 4 | 6 | 8) {
                if let Some(replacement) = semantic_color(&source[index..end], &colors) {
                    output.extend_from_slice(replacement);
                    index = end;
                    continue;
                }
            }
        } else if starts_with_word_ignore_ascii_case(source, index, b"white") {
            output.extend_from_slice(&colors.text);
            index += 5;
            continue;
        } else if starts_with_word_ignore_ascii_case(source, index, b"currentColor") {
            output.extend_from_slice(&colors.text);
            index += 12;
            continue;
        }

        output.push(source[index]);
        index += 1;
    }

    output
}

/// Keep the drawn stroke at 1.5 device pixels regardless of icon size.
///
/// The artwork lives on a 24-unit grid, so usvg would scale a 1.5-unit stroke
/// down with the icon. Multiplying every `stroke-width` (and dash length) by
/// `24 / size` cancels that scale and preserves the optical weight.
fn scale_strokes(source: &[u8], size: f32) -> Vec<u8> {
    const WIDTH: &str = "stroke-width=\"";
    const DASH: &str = "stroke-dasharray=\"";

    let factor = if size > 0.0 { 24.0 / size } else { 1.0 };
    if (factor - 1.0).abs() < f32::EPSILON {
        return source.to_vec();
    }

    let text = String::from_utf8_lossy(source);
    let mut output = String::with_capacity(text.len() + 96);
    let mut rest = text.as_ref();
    while let Some((position, prefix)) = [WIDTH, DASH]
        .into_iter()
        .filter_map(|prefix| rest.find(prefix).map(|position| (position, prefix)))
        .min_by_key(|(position, _)| *position)
    {
        let (before, from_attribute) = rest.split_at(position);
        output.push_str(before);
        let Some(after_prefix) = from_attribute.strip_prefix(prefix) else {
            unreachable!("the prefix was just located");
        };
        output.push_str(prefix);
        let Some(end) = after_prefix.find('"') else {
            output.push_str(after_prefix);
            rest = "";
            break;
        };
        let (value, tail) = after_prefix.split_at(end);
        output.push_str(&scale_numbers(value, factor));
        rest = tail;
    }
    output.push_str(rest);
    output.into_bytes()
}

fn scale_numbers(value: &str, factor: f32) -> String {
    value
        .split(' ')
        .map(|token| match token.parse::<f32>() {
            Ok(number) => trim_number(number * factor),
            Err(_) => token.to_owned(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn trim_number(value: f32) -> String {
    let formatted = format!("{value:.3}");
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn semantic_color<'a>(token: &[u8], colors: &'a SemanticColors) -> Option<&'a [u8; 7]> {
    if let Some(hue) = hue_semantic_color(token, colors) {
        return Some(hue);
    }
    if is_one_of(
        token,
        &["#b4b6b9", "#e0e0e0", "#eeeeee", "#ffffff", "#e1e1e1"],
    ) {
        Some(&colors.text)
    } else if is_one_of(token, &["#cccccc", "#bdbdbd", "#aaaaaa"]) {
        Some(&colors.secondary_strong)
    } else if is_one_of(
        token,
        &[
            "#888888", "#888", "#9e9e9e", "#90a4ae", "#78909c", "#7a7a7a", "#777777",
        ],
    ) {
        Some(&colors.secondary)
    } else if is_one_of(
        token,
        &[
            "#505050", "#555", "#606060", "#666", "#616161", "#546e7a", "#455a64", "#37474f",
        ],
    ) {
        Some(&colors.secondary_weak)
    } else if is_one_of(token, &["#1a1a1a"]) {
        Some(&colors.background)
    } else if is_one_of(
        token,
        &["#6db7ed", "#4cc9f0", "#4bc8f0", "#4a9eff", "#0099e5"],
    ) {
        Some(&colors.primary)
    } else if is_one_of(token, &["#9cc2f5"]) {
        Some(&colors.hues[1].hi)
    } else if is_one_of(token, &["#4f6fe0"]) {
        Some(&colors.hues[1].lo)
    } else if is_one_of(token, &["#5e86e8"]) {
        Some(&colors.hues[1].wall)
    } else if is_one_of(token, &["#1565c0"]) {
        Some(&colors.primary_strong)
    } else if is_one_of(token, &["#0d47a1"]) {
        Some(&colors.primary_weak)
    } else if is_one_of(token, &["#4ccf6f"]) {
        Some(&colors.success)
    } else if is_one_of(token, &["#00695c", "#004d40"]) {
        Some(&colors.success_weak)
    } else if is_one_of(token, &["#f0c040", "#ffd740", "#fdd835"]) {
        Some(&colors.warning)
    } else if is_one_of(token, &["#f9a825"]) {
        Some(&colors.warning_strong)
    } else if is_one_of(
        token,
        &[
            "#e05050", "#ef5350", "#e06c6c", "#e53935", "#ff0000", "#e10000",
        ],
    ) {
        Some(&colors.danger)
    } else if is_one_of(token, &["#b71c1c"]) {
        Some(&colors.danger_weak)
    } else {
        None
    }
}

/// Resolve a DraftFlow hue code (`#f0a1XY`, `X` = hue 0..7, `Y` = slot
/// 0..3) against the active theme's fixed per-category colour ramp.
fn hue_semantic_color<'a>(token: &[u8], colors: &'a SemanticColors) -> Option<&'a [u8; 7]> {
    if token.len() != 7 || !token[..5].eq_ignore_ascii_case(b"#f0a1") {
        return None;
    }
    let hex = |b: u8| -> Option<usize> { (b as char).to_digit(16).map(|d| d as usize) };
    let hue = hex(*token.get(5)?)?;
    let slot = hex(*token.get(6)?)?;
    if hue >= colors.hues.len() || slot >= 4 {
        return None;
    }
    let set = &colors.hues[hue];
    Some(match slot {
        0 => &set.hi,
        1 => &set.lo,
        2 => &set.wall,
        _ => &set.edge,
    })
}

fn is_one_of(token: &[u8], candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| token.eq_ignore_ascii_case(candidate.as_bytes()))
}

fn starts_with_word_ignore_ascii_case(source: &[u8], index: usize, word: &[u8]) -> bool {
    let Some(end) = index.checked_add(word.len()) else {
        return false;
    };
    if end > source.len()
        || !source[index..end].eq_ignore_ascii_case(word)
        || index > 0 && source[index - 1].is_ascii_alphabetic()
        || end < source.len() && source[end].is_ascii_alphabetic()
    {
        return false;
    }
    true
}

fn color_hex(color: Color) -> [u8; 7] {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let [red, green, blue, _] = color.into_rgba8();
    [
        b'#',
        HEX[(red >> 4) as usize],
        HEX[(red & 0x0f) as usize],
        HEX[(green >> 4) as usize],
        HEX[(green & 0x0f) as usize],
        HEX[(blue >> 4) as usize],
        HEX[(blue & 0x0f) as usize],
    ]
}

/// The one place a chrome glyph is built, so the entrance scope reaches it.
///
/// The fade must go through the widget's own opacity: an SVG tint replaces its
/// RGB and ignores alpha on the style colour (see [`themed_disabled`]).
fn glyph<'a>(bytes: &'static [u8], size: f32) -> svg::Svg<'a> {
    svg(themed_handle(bytes, size))
        .width(size)
        .height(size)
        .opacity(crate::ui::kit::alpha::current())
}

/// Render a chrome icon with the active Iced theme's normal text color.
pub fn themed<'a, M: 'a>(bytes: &'static [u8], size: f32) -> Element<'a, M> {
    glyph(bytes, size)
        .style(|theme: &Theme, _| svg::Style {
            color: Some(theme.palette().background.base.text),
        })
        .into()
}

/// Render secondary chrome with the active Iced theme's text color.
pub fn themed_secondary<'a, M: 'a>(bytes: &'static [u8], size: f32) -> Element<'a, M> {
    glyph(bytes, size)
        .style(|theme: &Theme, _| svg::Style {
            color: Some(theme.palette().background.base.text.scale_alpha(0.72)),
        })
        .into()
}

/// Render chrome with ink the caller solved for the surface it lands on.
///
/// An `svg` style closure receives only the theme, so an icon cannot inherit
/// the `text_color` the kit hands the container it sits in. That is why the
/// status-bar pills used to pick a palette slot of their own and drift off the
/// plate underneath them: the lit pill was tinted `primary.weak.text`, the
/// foreground of a surface [`crate::ui::kit::surface::pill_style`] no longer
/// paints. A pill's icon asks the kit instead —
/// `themed_by(bytes, size, |t| kit::surface::pill_ink(id, active, t, ""))` —
/// so plate and ink stay one decision.
pub fn themed_by<'a, M: 'a>(
    bytes: &'static [u8],
    size: f32,
    ink: impl Fn(&Theme) -> Color + 'a,
) -> Element<'a, M> {
    glyph(bytes, size)
        .style(move |theme: &Theme, _| svg::Style {
            color: Some(ink(theme)),
        })
        .into()
}

/// Render disabled chrome with the active Iced theme's text color, faded
/// like disabled text. The fade goes through the widget's opacity: an SVG
/// tint only replaces RGB, so alpha on the style colour would be ignored.
pub fn themed_disabled<'a, M: 'a>(bytes: &'static [u8], size: f32) -> Element<'a, M> {
    glyph(bytes, size)
        .opacity(0.42 * crate::ui::kit::alpha::current())
        .style(|theme: &Theme, _| svg::Style {
            color: Some(theme.palette().background.base.text),
        })
        .into()
}

/// Render an emphasized chrome icon with the active Iced theme's primary color.
pub fn themed_primary<'a, M: 'a>(bytes: &'static [u8], size: f32) -> Element<'a, M> {
    glyph(bytes, size)
        .style(|theme: &Theme, _| svg::Style {
            color: Some(theme.palette().primary.base.color),
        })
        .into()
}

/// Render an icon with the foreground chosen for a weak primary surface.
pub fn themed_primary_weak_text<'a, M: 'a>(bytes: &'static [u8], size: f32) -> Element<'a, M> {
    glyph(bytes, size)
        .style(|theme: &Theme, _| svg::Style {
            color: Some(theme.palette().primary.weak.text),
        })
        .into()
}

/// Render a positive-state chrome icon with the active Iced theme's success color.
pub fn themed_success<'a, M: 'a>(bytes: &'static [u8], size: f32) -> Element<'a, M> {
    glyph(bytes, size)
        .style(|theme: &Theme, _| svg::Style {
            color: Some(theme.palette().success.base.color),
        })
        .into()
}

/// Render an icon with the foreground chosen for a success-coloured surface.
pub fn themed_success_text<'a, M: 'a>(bytes: &'static [u8], size: f32) -> Element<'a, M> {
    glyph(bytes, size)
        .style(|theme: &Theme, _| svg::Style {
            color: Some(theme.palette().success.base.text),
        })
        .into()
}

/// Render a warning-state chrome icon from the active Iced theme.
pub fn themed_warning<'a, M: 'a>(bytes: &'static [u8], size: f32) -> Element<'a, M> {
    glyph(bytes, size)
        .style(|theme: &Theme, _| svg::Style {
            color: Some(theme.palette().warning.base.color),
        })
        .into()
}

/// Render a destructive-state chrome icon from the active Iced theme.
pub fn themed_danger<'a, M: 'a>(bytes: &'static [u8], size: f32) -> Element<'a, M> {
    glyph(bytes, size)
        .style(|theme: &Theme, _| svg::Style {
            color: Some(theme.palette().danger.base.color),
        })
        .into()
}

/// Render an icon with the foreground chosen for a danger-coloured surface.
pub fn themed_danger_text<'a, M: 'a>(bytes: &'static [u8], size: f32) -> Element<'a, M> {
    glyph(bytes, size)
        .style(|theme: &Theme, _| svg::Style {
            color: Some(theme.palette().danger.base.text),
        })
        .into()
}

/// Fixed-width check column colored from the active Iced theme.
pub fn themed_check_cell<'a, M: 'a>(active: bool) -> Element<'a, M> {
    let inner: Element<'a, M> = if active {
        glyph(CHECK, 11.0)
            .width(11.0)
            .height(11.0)
            .style(|theme: &Theme, _| {
                let p = theme.palette();
                let pri = p.primary.base.color;
                let bg_weak = p.background.weak.color;
                let bg_strong = p.background.strong.color;
                let fallback = p.background.weak.text;
                let candidate = accessible_accent(pri, bg_weak, fallback);
                let color = if wcag_contrast(candidate, bg_strong) >= 3.0 {
                    candidate
                } else {
                    fallback
                };
                svg::Style { color: Some(color) }
            })
            .into()
    } else {
        Space::new().width(0).into()
    };
    container(inner)
        .width(Length::Fixed(14.0))
        .align_x(iced::alignment::Horizontal::Center)
        .into()
}

/// SVG bytes for an OSNAP mode's marker symbol, for the snap menu. (#138)
pub fn osnap(snap: crate::snap::SnapType) -> &'static [u8] {
    use crate::snap::SnapType as S;
    match snap {
        S::Endpoint => OSNAP_ENDPOINT,
        S::Midpoint => OSNAP_MIDPOINT,
        S::Center => OSNAP_CENTER,
        S::GeometricCenter => OSNAP_GEOMETRIC_CENTER,
        S::Node => OSNAP_NODE,
        S::Quadrant => OSNAP_QUADRANT,
        S::Intersection => OSNAP_INTERSECTION,
        S::Extension => OSNAP_EXTENSION,
        S::Insertion => OSNAP_INSERTION,
        S::Perpendicular => OSNAP_PERPENDICULAR,
        S::Tangent => OSNAP_TANGENT,
        S::Nearest => OSNAP_NEAREST,
        S::ApparentIntersection => OSNAP_APPARENT,
        S::Parallel => OSNAP_PARALLEL,
        S::Grid => OSNAP_GRID,
        // Not shown in the snap menu; fall back to a neutral marker.
        S::ObjectPick => OSNAP_NEAREST,
        // 3D modes have no dedicated menu glyphs yet; reuse the closest 2D
        // shapes (hollow diamond ≈ vertex, hollow triangle ≈ edge midpoint).
        S::Vertex => OSNAP_NODE,
        S::EdgeMidpoint => OSNAP_MIDPOINT,
        S::FaceCenter => OSNAP_CENTER,
        S::Knot => OSNAP_NODE,
        S::FacePerpendicular => OSNAP_PERPENDICULAR,
        S::NearestFace => OSNAP_NEAREST,
    }
}

/// MTP menu icon: modal 2-pick modifier, not a persistent `SnapType` mode.
pub fn mtp_icon() -> &'static [u8] {
    OSNAP_MTP
}

/// Context-menu glyphs for the transparent navigation rows.
pub fn pan_icon() -> &'static [u8] {
    NAV_PAN
}

pub fn zoom_icon() -> &'static [u8] {
    NAV_ZOOM
}

pub fn undo_icon() -> &'static [u8] {
    UNDO
}

pub fn redo_icon() -> &'static [u8] {
    REDO
}

/// Layer visibility icon bytes (on / off).
pub fn layer_visible(visible: bool) -> &'static [u8] {
    if visible {
        LAY_ON
    } else {
        LAY_OFF
    }
}

/// Layer freeze icon bytes (frozen / thawed).
pub fn layer_freeze(frozen: bool) -> &'static [u8] {
    if frozen {
        LAY_FRZ
    } else {
        LAY_THW
    }
}

/// Freeze-in-current-viewport icon bytes (frozen / thawed).
pub fn layer_vp_freeze(frozen: bool) -> &'static [u8] {
    if frozen {
        LAY_VP_FRZ
    } else {
        LAY_VP_THW
    }
}

/// Layer lock icon bytes (locked / unlocked).
pub fn layer_lock(locked: bool) -> &'static [u8] {
    if locked {
        LAY_LCK
    } else {
        LAY_ULK
    }
}

pub fn themed_arrow_down<'a, M: 'a>(size: f32) -> Element<'a, M> {
    themed(TRI_DOWN, size)
}

/// The option mark a ribbon control wears when it has more choices behind it.
/// Fuller than [`themed_arrow_down`] at the same element size so the mark is
/// legible on a 62 px strip cell without growing into the tool's icon.
/// [`themed_option_caret_by`] inks it with the label ink of the owning cell.
/// of the cell that owns it, so a resting caret is the same muted grey as the
/// name above it and lifts with that name on hover.
pub fn themed_option_caret_by<'a, M: 'a>(
    size: f32,
    ink: impl Fn(&Theme) -> Color + 'a,
) -> Element<'a, M> {
    themed_by(OPTION_CARET, size, ink)
}

pub fn themed_arrow_up<'a, M: 'a>(size: f32) -> Element<'a, M> {
    themed(TRI_UP, size)
}

pub fn themed_arrow_right<'a, M: 'a>(size: f32) -> Element<'a, M> {
    themed(TRI_RIGHT, size)
}

pub fn themed_arrow_left<'a, M: 'a>(size: f32) -> Element<'a, M> {
    themed(TRI_LEFT, size)
}

pub fn themed_primary_weak_arrow_down<'a, M: 'a>(size: f32) -> Element<'a, M> {
    themed_primary_weak_text(TRI_DOWN, size)
}

pub fn themed_secondary_arrow_down<'a, M: 'a>(size: f32) -> Element<'a, M> {
    themed_secondary(TRI_DOWN, size)
}

/// [`themed_by`] for the dropdown caret. The caret bytes are private to this
/// module, so a pill that inks its caret through the kit needs this door rather
/// than a palette slot of its own.
pub fn themed_down_by<'a, M: 'a>(size: f32, ink: impl Fn(&Theme) -> Color + 'a) -> Element<'a, M> {
    themed_by(TRI_DOWN, size, ink)
}

pub fn themed_disabled_arrow_down<'a, M: 'a>(size: f32) -> Element<'a, M> {
    themed_disabled(TRI_DOWN, size)
}

pub fn themed_disabled_arrow_right<'a, M: 'a>(size: f32) -> Element<'a, M> {
    themed_disabled(TRI_RIGHT, size)
}

pub fn themed_home<'a, M: 'a>(size: f32) -> Element<'a, M> {
    themed(HOME, size)
}

/// Caret that flips up/down with `open`.
pub fn themed_arrow_toggle<'a, M: 'a>(open: bool, size: f32) -> Element<'a, M> {
    if open {
        themed_arrow_up(size)
    } else {
        themed_arrow_down(size)
    }
}

pub fn themed_undo<'a, M: 'a>(size: f32, enabled: bool) -> Element<'a, M> {
    if enabled {
        themed(UNDO, size)
    } else {
        themed_disabled(UNDO, size)
    }
}

pub fn themed_redo<'a, M: 'a>(size: f32, enabled: bool) -> Element<'a, M> {
    if enabled {
        themed(REDO, size)
    } else {
        themed_disabled(REDO, size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_svg_uses_multiple_theme_roles() {
        let source = br##"<svg>
            <path stroke="#e0e0e0"/>
            <path fill="#4cc9f0"/>
            <path fill="#f0c040"/>
            <path fill="#e05050"/>
            <path fill="#4ccf6f"/>
            <path fill="#123456"/>
        </svg>"##;
        let theme = Theme::Dark;
        let colors = SemanticColors::from_theme(&theme);
        let themed = recolor_semantic_svg(source, &theme);

        assert!(contains(&themed, &colors.text));
        assert!(contains(&themed, &colors.primary));
        assert!(contains(&themed, &colors.warning));
        assert!(contains(&themed, &colors.danger));
        assert!(contains(&themed, &colors.success));
        assert!(contains(&themed, b"#123456"));
    }

    #[test]
    fn semantic_svg_maps_named_white() {
        let theme = Theme::Dark;
        let colors = SemanticColors::from_theme(&theme);
        let themed = recolor_semantic_svg(br##"<path stroke="white"/>"##, &theme);

        assert!(contains(&themed, &colors.text));
    }

    #[test]
    fn semantic_svg_maps_current_color() {
        let theme = Theme::Dark;
        let colors = SemanticColors::from_theme(&theme);
        let themed = recolor_semantic_svg(
            br##"<path stroke="currentColor" fill="currentColor"/>"##,
            &theme,
        );
        let text = String::from_utf8_lossy(&themed);

        assert!(!text.contains("currentColor"));
        assert_eq!(text.matches(std::str::from_utf8(&colors.text).unwrap()).count(), 2);
    }

    #[test]
    fn semantic_strokes_keep_optical_weight_at_small_sizes() {
        let source = br##"<path stroke-width="1.5" stroke-dasharray="2 2"/>"##;
        let scaled = scale_strokes(source, 14.0);
        let text = String::from_utf8_lossy(&scaled);

        assert!(text.contains(r#"stroke-width="2.571""#), "{text}");
        assert!(text.contains(r#"stroke-dasharray="3.429 3.429""#), "{text}");
    }

    #[test]
    fn semantic_strokes_stay_untouched_on_the_grid() {
        let source = br##"<path stroke-width="1.5" stroke-dasharray="2 2"/>"##;
        let scaled = scale_strokes(source, 24.0);

        assert_eq!(scaled, source.to_vec());
    }

    fn contains(source: &[u8], needle: &[u8]) -> bool {
        source.windows(needle.len()).any(|window| window == needle)
    }
}

#[cfg(test)]
mod themed_cache_tests {
    use super::*;

    /// Number of `svg::Handle` entries currently in the `THEMED_CACHE`.
    /// Test-only inspection helper; the cache is private production state.
    fn themed_cache_size_for_test() -> usize {
        THEMED_CACHE.with(|c| c.borrow().map.len())
    }

    /// True when any entry for the address and length of a
    /// `&'static [u8]` sits in the `THEMED_CACHE`, at any size.
    /// The size is part of the production key; these helpers stay
    /// size-agnostic so the pre-existing invariants (bytes identity,
    /// length discrimination) keep reading as written.
    /// Test-only inspection helper.
    fn themed_cache_contains_for_test(ptr: *const u8, len: usize) -> bool {
        THEMED_CACHE.with(|c| {
            c.borrow()
                .map
                .keys()
                .any(|(address, length, _)| *address == ptr as usize && *length == len)
        })
    }

    /// Returns a clone of the cached `svg::Handle` for `(ptr, len)` at
    /// any size, or `None` if no entry is cached. Test-only inspection
    /// helper.
    fn themed_cache_get_handle_for_test(ptr: *const u8, len: usize) -> Option<svg::Handle> {
        THEMED_CACHE.with(|c| {
            c.borrow()
                .map
                .iter()
                .find(|((address, length, _), _)| *address == ptr as usize && *length == len)
                .map(|(_, handle)| handle.clone())
        })
    }

    /// Removes every cache entry for `(ptr, len)` at any size. Test-only
    /// helper so individual tests can start from a known-empty cache
    /// regardless of test execution order (tests share the
    /// `thread_local!` when run on the same thread).
    fn themed_cache_remove_for_test(ptr: *const u8, len: usize) {
        THEMED_CACHE.with(|c| {
            let mut c = c.borrow_mut();
            c.map.retain(|(address, length, _), _| {
                !(*address == ptr as usize && *length == len)
            });
            for slot in &mut c.mru {
                if let Some(((address, length, _), _)) = slot {
                    if *address == ptr as usize && *length == len {
                        *slot = None;
                    }
                }
            }
        })
    }

    /// Two calls to the public `themed` function with the same
    /// `&'static [u8]` must share a single cache entry: the second
    /// call is a cache hit, not a fresh `Handle::from_memory`.
    #[test]
    fn themed_cache_returns_same_handle_for_same_bytes() {
        themed_cache_remove_for_test(TRI_DOWN.as_ptr(), TRI_DOWN.len());
        assert!(
            !themed_cache_contains_for_test(TRI_DOWN.as_ptr(), TRI_DOWN.len()),
            "precondition: TRI_DOWN must not be cached before the test"
        );
        let size_before = themed_cache_size_for_test();
        let _: Element<'static, ()> = themed(TRI_DOWN, 16.0);
        let _: Element<'static, ()> = themed(TRI_DOWN, 16.0);
        assert_eq!(
            themed_cache_size_for_test(),
            size_before + 1,
            "two calls with the same bytes must share a single cache entry (delta +1)"
        );
        assert!(
            themed_cache_contains_for_test(TRI_DOWN.as_ptr(), TRI_DOWN.len()),
            "cache must hold an entry for the requested bytes"
        );
    }

    /// Distinct `&'static [u8]` slices must produce distinct cache
    /// entries (no false aliasing across different artwork).
    #[test]
    fn themed_cache_caches_distinct_bytes_separately() {
        themed_cache_remove_for_test(TRI_DOWN.as_ptr(), TRI_DOWN.len());
        themed_cache_remove_for_test(TRI_UP.as_ptr(), TRI_UP.len());
        assert!(!themed_cache_contains_for_test(
            TRI_DOWN.as_ptr(),
            TRI_DOWN.len()
        ));
        assert!(!themed_cache_contains_for_test(
            TRI_UP.as_ptr(),
            TRI_UP.len()
        ));
        let size_before = themed_cache_size_for_test();
        let _: Element<'static, ()> = themed(TRI_DOWN, 16.0);
        let _: Element<'static, ()> = themed(TRI_UP, 16.0);
        assert_eq!(
            themed_cache_size_for_test(),
            size_before + 2,
            "distinct bytes must produce distinct cache entries (delta +2)"
        );
        assert!(themed_cache_contains_for_test(
            TRI_DOWN.as_ptr(),
            TRI_DOWN.len()
        ));
        assert!(themed_cache_contains_for_test(
            TRI_UP.as_ptr(),
            TRI_UP.len()
        ));
    }

    /// The cached `svg::Handle` is a single source of truth: two
    /// calls with the same bytes must return a handle derived from
    /// the same cache entry, not a freshly-built handle.
    ///
    /// **`svg::Handle` is opaque and has no `PartialEq` impl**, so
    /// we cannot compare handles directly for pointer-identity. The
    /// best verifiable proxy is: (a) the second call does not grow
    /// the cache (a buggy `entry().or_insert_with` that always
    /// inserted would still show size 1 for a single key, so this
    /// rules out a double-insert on a single key), and (b) the
    /// getter returns the cached handle across the two calls (the
    /// cache survived and still resolves the key). Together these
    /// confirm the cache is a single source — the test's identity
    /// guarantee is the weaker "the cache is populated and reused"
    /// rather than the stronger "the handle is pointer-identical",
    /// which `svg::Handle` does not expose.
    #[test]
    fn themed_cache_second_call_is_a_cache_hit() {
        themed_cache_remove_for_test(TRI_DOWN.as_ptr(), TRI_DOWN.len());
        assert!(
            !themed_cache_contains_for_test(TRI_DOWN.as_ptr(), TRI_DOWN.len()),
            "precondition: TRI_DOWN must not be cached"
        );
        let size_before = themed_cache_size_for_test();
        // Warm the cache with the first call (miss → build → insert).
        let _: Element<'static, ()> = themed(TRI_DOWN, 16.0);
        let cached_after_first =
            themed_cache_get_handle_for_test(TRI_DOWN.as_ptr(), TRI_DOWN.len())
                .expect("cache must hold the handle after the first call");
        let size_after_first = themed_cache_size_for_test();
        assert_eq!(
            size_after_first,
            size_before + 1,
            "first call must insert exactly one entry"
        );
        // Second call: must be a cache hit, not a fresh build.
        let _: Element<'static, ()> = themed(TRI_DOWN, 16.0);
        let cached_after_second =
            themed_cache_get_handle_for_test(TRI_DOWN.as_ptr(), TRI_DOWN.len())
                .expect("cache must still hold the handle after the second call");
        assert_eq!(
            themed_cache_size_for_test(),
            size_after_first,
            "second call must not grow the cache (hit, not rebuild)"
        );
        let _ = (cached_after_first, cached_after_second);
    }

    /// Wrapper functions (e.g. `themed_arrow_down`) delegate to
    /// the 9 `themed*` byte-taking functions and therefore inherit
    /// the cache transparently. This end-to-end test confirms the
    /// delegation: a wrapper call populates the cache for its
    /// underlying static bytes. The cache entry is cleared first so
    /// the assertion is independent of test execution order (tests
    /// share the `thread_local!` when run on the same thread).
    #[test]
    fn themed_wrapper_populates_cache_for_underlying_bytes() {
        themed_cache_remove_for_test(TRI_DOWN.as_ptr(), TRI_DOWN.len());
        assert!(
            !themed_cache_contains_for_test(TRI_DOWN.as_ptr(), TRI_DOWN.len()),
            "precondition: TRI_DOWN must not be cached before the call"
        );
        let size_before = themed_cache_size_for_test();
        // `themed_arrow_down` delegates to `themed(TRI_DOWN, size)`.
        let _: Element<'static, ()> = themed_arrow_down(16.0);
        assert_eq!(
            themed_cache_size_for_test(),
            size_before + 1,
            "one wrapper call must populate exactly one cache entry (delta +1)"
        );
        assert!(
            themed_cache_contains_for_test(TRI_DOWN.as_ptr(), TRI_DOWN.len()),
            "the cache must hold an entry for the wrapper's underlying bytes (TRI_DOWN)"
        );
    }

    /// Cache key includes length: a sub-slice of a static must not
    /// alias the full static's entry. This guards the
    /// `(address, length)` belt-and-braces key `src/ui/icons.rs:164`
    /// — a pointer-only key would incorrectly hit on `&BYTES[1..]`.
    #[test]
    fn themed_cache_key_includes_length() {
        // Ensure clean state for both keys.
        themed_cache_remove_for_test(TRI_DOWN.as_ptr(), TRI_DOWN.len());
        // Sub-slice of TRI_DOWN: same address region but different length.
        // SAFETY: TRI_DOWN is at least 2 bytes (real SVG), so [1..] is valid.
        let sub = &TRI_DOWN[1..];
        themed_cache_remove_for_test(sub.as_ptr(), sub.len());
        assert!(!themed_cache_contains_for_test(
            TRI_DOWN.as_ptr(),
            TRI_DOWN.len()
        ));
        assert!(!themed_cache_contains_for_test(sub.as_ptr(), sub.len()));
        let size_before = themed_cache_size_for_test();
        // Populate full static.
        let _: Element<'static, ()> = themed(TRI_DOWN, 16.0);
        assert!(themed_cache_contains_for_test(
            TRI_DOWN.as_ptr(),
            TRI_DOWN.len()
        ));
        assert!(
            !themed_cache_contains_for_test(sub.as_ptr(), sub.len()),
            "sub-slice must not hit the full static's entry"
        );
        assert_eq!(themed_cache_size_for_test(), size_before + 1);
        // Populate sub-slice directly via the internal helper (bypasses
        // the `themed` wrappers which only use the full statics).
        let _ = super::themed_handle(sub, 16.0);
        assert!(
            themed_cache_contains_for_test(sub.as_ptr(), sub.len()),
            "sub-slice must be cached at the size it was rendered at"
        );
        assert_eq!(
            themed_cache_size_for_test(),
            size_before + 2,
            "sub-slice must produce a distinct entry (length part of key)"
        );
    }

    /// Cache key includes size: the same static rendered at two sizes
    /// must produce two entries, because the handle carries stroke
    /// widths scaled for that size. Sharing one entry across sizes
    /// would draw 16 px icons with 24 px strokes (or the reverse).
    #[test]
    fn themed_cache_key_includes_size() {
        themed_cache_remove_for_test(TRI_DOWN.as_ptr(), TRI_DOWN.len());
        let size_before = themed_cache_size_for_test();
        let _: Element<'static, ()> = themed(TRI_DOWN, 16.0);
        let _: Element<'static, ()> = themed(TRI_DOWN, 20.0);
        assert_eq!(
            themed_cache_size_for_test(),
            size_before + 2,
            "one static at two sizes must hold two entries (size part of key)"
        );
        themed_cache_remove_for_test(TRI_DOWN.as_ptr(), TRI_DOWN.len());
    }

    /// Before/after benchmark: the cached `themed_handle` path
    /// (hashmap get + a `svg::Handle` clone) vs. the original
    /// un-cached path (`svg::Handle::from_memory` per call, which
    /// parses the SVG and allocates).
    ///
    /// Integrated as a `#[test]` so it runs in the existing
    /// `cargo test --lib` flow with no extra infrastructure — no
    /// `criterion`, no `benches/` directory, no nightly
    /// `#![feature(test)]`. The "before" is faithfully simulated by
    /// calling `svg::Handle::from_memory` directly on the same
    /// `&'static [u8]`, which is exactly what the pre-cache code did
    /// per call. The "after" warms the cache first so the loop
    /// measures pure cache hits. A cold cache miss is also measured
    /// on a different static.
    ///
    /// Run with `--nocapture` to see the numbers:
    /// `cargo test --lib themed_cache_speedup_over_uncached_handle_creation -- --ignored --nocapture`.
    ///
    /// Ignored by default: it compares wall-clock times, and under the
    /// parallel suite's load the uncached loop can finish first. The cache's
    /// behaviour is pinned by the entry-count tests above.
    #[test]
    #[ignore = "wall-clock benchmark; run with --ignored"]
    fn themed_cache_speedup_over_uncached_handle_creation() {
        use std::time::Instant;

        const ITER: u64 = 20_000;
        let bytes = TRI_DOWN; // &'static [u8]

        // Warm the cache so the "after" loop measures pure cache hits.
        let _ = super::themed_handle(bytes, 16.0);

        // After: cached path — one hashmap get + a `Handle` clone per call.
        let t_after = Instant::now();
        for _ in 0..ITER {
            let _h = super::themed_handle(bytes, 16.0);
        }
        let after = t_after.elapsed();

        // Before: original (un-cached) path — SVG parse + allocation per call.
        let t_before = Instant::now();
        for _ in 0..ITER {
            let _h = svg::Handle::from_memory(bytes);
        }
        let before = t_before.elapsed();

        // Cold first call (cache miss): one parse + one insert. Measured
        // on a different static so the entry is guaranteed to be fresh.
        let cold_bytes = TRI_UP;
        let t_cold = Instant::now();
        let _h = super::themed_handle(cold_bytes, 16.0);
        let cold = t_cold.elapsed();

        let after_ns = after.as_nanos() as u64;
        let before_ns = before.as_nanos() as u64;
        let cold_ns = cold.as_nanos() as u64;
        let per_after = after_ns / ITER;
        let per_before = before_ns / ITER;
        let speedup = if after_ns > 0 {
            before_ns as f64 / after_ns as f64
        } else {
            f64::INFINITY
        };

        println!(
            "themed_handle: before={}ns/iter, after={}ns/iter, speedup={:.1}x, cold_miss={}ns (ITER={})",
            per_before, per_after, speedup, cold_ns, ITER,
        );

        assert!(
            after_ns < before_ns,
            "cached path must be faster than from_memory: after={}ns before={}ns",
            after_ns,
            before_ns,
        );
    }
}

#[cfg(test)]
mod check_cell_tests {
    use super::*;
    use crate::ui::style::common::{accessible_accent, wcag_contrast};

    #[test]
    fn themed_check_cell_contrast_across_all_themes() {
        for theme in iced::Theme::ALL {
            let p = theme.palette();
            let bg_weak = p.background.weak.color;
            let bg_strong = p.background.strong.color;
            let pri = p.primary.base.color;
            let fallback = p.background.weak.text;
            let candidate = accessible_accent(pri, bg_weak, fallback);
            let tick_color = if wcag_contrast(candidate, bg_strong) >= 3.0 {
                candidate
            } else {
                fallback
            };
            let contrast_weak = wcag_contrast(tick_color, bg_weak);
            let contrast_strong = wcag_contrast(tick_color, bg_strong);
            assert!(
                contrast_weak >= 3.0,
                "Theme {:?} checkmark contrast {:.2}:1 against weak background is below 3.0:1",
                theme,
                contrast_weak
            );
            assert!(
                contrast_strong >= 3.0,
                "Theme {:?} checkmark contrast {:.2}:1 against strong background is below 3.0:1",
                theme,
                contrast_strong
            );
        }
    }

    #[test]
    fn themed_check_cell_dimensions() {
        let active: Element<'static, ()> = themed_check_cell(true);
        let inactive: Element<'static, ()> = themed_check_cell(false);
        assert_eq!(active.as_widget().size().width, Length::Fixed(14.0));
        assert_eq!(inactive.as_widget().size().width, Length::Fixed(14.0));
    }

    /// Chrome glyphs may only be built in [`glyph`], the one constructor that
    /// carries the entrance opacity. A builder anywhere else paints at full
    /// strength inside a fading row, so it is a hole in the wave.
    fn glyph_sites_outside_glyph(source: &str) -> usize {
        // Assembled instead of written out: this test scans its own file, and
        // a literal would count itself as a construction site.
        let needle = ["svg(", "themed_handle("].concat();
        let glyph = source.find("fn glyph").expect("glyph constructor exists");
        let glyph_end = source[glyph..]
            .find("\n}\n")
            .map(|end| glyph + end)
            .unwrap_or(source.len());
        source
            .match_indices(needle.as_str())
            .filter(|(at, _)| *at < glyph || *at > glyph_end)
            .count()
    }

    #[test]
    fn every_chrome_glyph_is_built_in_the_alpha_constructor() {
        let source = include_str!("icons.rs");
        assert_eq!(
            glyph_sites_outside_glyph(source),
            0,
            "a glyph is built outside `glyph`, so it ignores the entrance alpha"
        );
        assert_eq!(
            source.matches(["svg(", "themed_handle("].concat().as_str()).count(),
            1,
            "more than one glyph construction path"
        );
        assert!(
            source.contains(".opacity(crate::ui::kit::alpha::current())"),
            "`glyph` no longer reads the entrance alpha"
        );
    }

    /// The gate above must actually see a bypass; this is the shape it catches.
    #[test]
    fn the_glyph_gate_reports_a_bypass() {
        let (svg, handle) = ("svg(", "themed_handle(");
        let source = format!(
            "fn glyph<'a>(bytes: &'static [u8], size: f32) -> svg::Svg<'a> {{\n    {svg}{handle}bytes, size))\n}}\n\npub fn leaked<'a, M: 'a>(bytes: &'static [u8], size: f32) -> Element<'a, M> {{\n    {svg}{handle}bytes, size)).into()\n}}\n"
        );
        assert_eq!(glyph_sites_outside_glyph(&source), 1);
    }
}
