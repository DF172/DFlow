//! Surfaces: the planes every screen is built out of.
//!
//! Before the kit, "a card" meant 60 slightly different things: 361 hand-written
//! `container::Style` / `button::Style` / `text_input::Style` literals across the
//! UI tree (audited 2026-10-15, `D:\tmp\kit-baseline.csv`), radii of 0/1/2/5/7/9/
//! 10/14 in use for the same concept, five separate implementations of a
//! divider. Here a plane is named by what it *does* — you dock against it, you
//! rest on it, you type into it, it floats above the document — and its fill,
//! edge, radius and shadow follow from the name in exactly one place:
//! [`surface`]. Nothing else in this module paints.
//!
//! Every function takes a `track`: the entrance-fade key the owning screen
//! animates (see [`crate::ui::motion`]). Multiplying the paint by that value is
//! what lets a panel dissolve in on boot without each call site re-inventing
//! the alpha math.

use crate::ui::motion;
use crate::ui::style::draftflow_theme::draftflow_colors;
use crate::ui::style::tokens;
use super::label::Tone;
use iced::widget::{button, column, container, text, Column, Container, Row, Space};
use iced::{Background, Border, Color, Element, Fill, Length, Theme};

/// Entrance fade for `track`; 1.0 while nothing is animating.
pub fn fade(track: &'static str) -> f32 {
    crate::ui::motion::value_or(track, 1.0)
}

/// The whole depth vocabulary. Two adjacent tiers must be tellable apart at a
/// glance — that is what [`crate::ui::style::tokens::SURFACE_LADDER`] buys, and
/// `kit_contract_tests::surface_tiers_separate_in_every_theme` is the guard.
/// It is also why the old five-step ladder lost its `Raised` rung: on the light
/// scheme `Card 0.45 → Raised 0.7` moved the fill by 6% and read as the same
/// plane, so the step bought nothing and cost a rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// The window itself. No fill, no edge, no shadow.
    Flush,
    /// A full-width chrome strip: the quick bar, the tab strip, the status bar.
    /// Squared corners because it meets the window edge-to-edge.
    Band,
    /// A group plate inside chrome — a ribbon tool group. Deliberately *not*
    /// shadowed: it pairs with [`Tier::Sink`] and separates by fill contrast
    /// alone, so the tool band stays quiet while the plate reads as a unit.
    Plate,
    /// A resting card: a dialog body, a dock panel, a recent-file tile.
    Card,
    /// Detached from the document — a popup, a tooltip, a modal. The only tier
    /// whose depth comes mostly from its shadow, because it has to read as
    /// above a page that already contains cards.
    Float,
    /// A shallow recess an edit sits in: a text field, a list well.
    Well,
    /// A deep recess: the ribbon's tool field, a canvas gutter. One notch below
    /// [`Tier::Well`], used where the band is chrome rather than a target.
    Sink,
}

impl Tier {
    /// Which rung of the raised ladder this tier owns. `None` for the recesses
    /// and the page: they are not on that ladder, and routing them through
    /// `surface()` would let a call site quietly brighten what must darken.
    fn rank(self) -> Option<tokens::Rank> {
        match self {
            Tier::Band => Some(tokens::Rank::Band),
            Tier::Plate => Some(tokens::Rank::Plate),
            Tier::Card => Some(tokens::Rank::Card),
            Tier::Float => Some(tokens::Rank::Float),
            Tier::Flush | Tier::Well | Tier::Sink => None,
        }
    }

    fn fill(self, theme: &Theme) -> Option<Background> {
        match self {
            Tier::Flush => None,
            Tier::Well => Some(Background::Color(tokens::inset(theme))),
            Tier::Sink => Some(Background::Color(tokens::sunk(theme))),
            other => Some(Background::Color(tokens::surface(
                theme,
                other.rank().expect("raised tier without a rank"),
            ))),
        }
    }

    fn radius(self, theme: &Theme) -> f32 {
        // The sheet's ladder is one step larger than the vendor themes' — a
        // card is a sheet floating on the page, a tile nests inside it, and a
        // control is a capsule. The vendor themes keep their own corners
        // bit-for-bit (`the_vendor_equivalents_are_exact`), so the two ladders
        // branch here rather than in every call site.
        if crate::ui::style::draftflow_theme::draftflow_colors(theme).is_some() {
            match self {
                Tier::Band => 0.0,
                Tier::Plate => tokens::RADIUS_TILE,
                Tier::Card | Tier::Float => tokens::RADIUS_CARD,
                Tier::Well | Tier::Sink => tokens::RADIUS_TILE,
                Tier::Flush => 0.0,
            }
        } else {
            match self {
                Tier::Band => 0.0,
                Tier::Plate => tokens::RADIUS_CONTROL,
                Tier::Card | Tier::Float => tokens::RADIUS_PANEL,
                Tier::Well | Tier::Sink => tokens::RADIUS_FIELD,
                Tier::Flush => 0.0,
            }
        }
    }

    /// The pixel this tier paints at full strength, or `None` for the page.
    ///
    /// `style` multiplies this by the entrance fade and hands it to iced; this
    /// is the same colour without the motion term, because anything that has to
    /// *choose an ink* cannot work with a moving plate — it needs the one the
    /// text will sit on once the panel has arrived. `fade` is 1.0 at rest, so
    /// this is the resting pixel, and a fading panel fades its ink with it.
    ///
    /// It exists so L1 can ask "what am I writing on?" without each call site
    /// re-deriving the fill from the ladder: [`crate::ui::kit::label::tone_style`]
    /// solves a verdict word against all seven of these, because the colour a
    /// chip is *filled* with is not automatically a legible word.
    #[must_use]
    pub fn paint(self, theme: &Theme) -> Option<Color> {
        match self.fill(theme) {
            Some(Background::Color(color)) => Some(color),
            Some(_) => None,
            None => None,
        }
    }

    /// Which tiers draw an edge. `Band` does not: it is chrome, and a hairline
    /// around a full-width strip reads as a box that has escaped the window.
    fn bordered(self) -> bool {
        !matches!(self, Tier::Flush | Tier::Band)
    }

    fn lift(self) -> f32 {
        match self {
            Tier::Card => 0.8,
            Tier::Float => 3.0,
            _ => 0.0,
        }
    }

    /// The tier's stylesheet, for call sites that already hold a configured
    /// `Container` and need only its paint. [`surface`] is this wrapped in the
    /// closure iced wants and applied to new content; both go through here, so
    /// a tier cannot end up looking different depending on which entry point a
    /// screen happened to use.
    pub fn style(self, theme: &Theme, track: &'static str) -> container::Style {
        let p = fade(track);
        let mut shadow = tokens::elevation(theme, self.lift());
        shadow.color = shadow.color.scale_alpha(p);
        let paint = super::alpha::Paint::start();
        paint.finish(container::Style {
            background: self.fill(theme).map(|bg| bg.scale_alpha(p)),
            border: Border {
                color: if self.bordered() {
                    let edge = if self == Tier::Well {
                        tokens::field_edge(theme)
                    } else {
                        tokens::edge(theme)
                    };
                    edge.scale_alpha(p)
                } else {
                    Color::TRANSPARENT
                },
                width: if self.bordered() { 1.0 } else { 0.0 },
                radius: self.radius(theme).into(),
            },
            shadow,
            text_color: None,
            snap: true,
        })
    }
}

/// The single plane a part is dropped onto, answered in one pixel.
///
/// [`Tier::paint`] returns an `Option` because `Flush` genuinely paints nothing,
/// and a caller that has to write `.unwrap_or(page)` in order to name its own
/// background is a caller that will guess instead. This resolves the page the
/// same way [`crate::ui::kit::label::kit_planes`] does — the window's base colour,
/// which is what a flush container actually shows — so a band or a chip can say
/// `kit::plane(theme, kit::Tier::Card)` and hand the solve the ground it will
/// really sit on rather than a set of grounds it might.
#[must_use]
pub fn plane(theme: &Theme, tier: Tier) -> Color {
    tier.paint(theme)
        .unwrap_or_else(|| theme.palette().background.base.color)
}

/// The only function in the app that decides what a plane looks like.
pub fn surface<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    tier: Tier,
    padding: iced::Padding,
    track: &'static str,
) -> Container<'a, M> {
    container(content)
        .padding(padding)
        .style(move |theme: &Theme| tier.style(theme, track))
}

/// The window's own ground while the theme is changing.
///
/// [`Tier::Flush`] paints nothing on purpose, because the page fill belongs to
/// the window rather than to any container inside it. A theme switch still must
/// not snap: `SetTheme` captures the *outgoing* base colour into `from`, plants
/// the fade track, and the ground walks between the two schemes. The walk lives
/// here because it is the same channel every other plane already reads, and
/// because `motion::mix` is exact at both ends — at 1.0 it returns the theme's
/// own base, so a window that has never switched themes is just the theme.
#[must_use]
pub fn page_style(from: Color, theme: &Theme, track: &'static str) -> container::Style {
    container::Style {
        background: Some(Background::Color(crate::ui::motion::mix(
            from,
            theme.palette().background.base.color,
            fade(track),
        ))),
        ..Default::default()
    }
}

/// [`page_style`] onto content: the root container of a window.
#[must_use]
pub fn page<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    from: Color,
    track: &'static str,
) -> Container<'a, M> {
    container(content).style(move |theme: &Theme| page_style(from, theme, track))
}

/// A tier's fill at its own corner radius and nothing else: no edge, no shadow.
///
/// For the washes *inside* a plane — the row behind a selected property, the
/// background of a grouped column in the dock, a highlighted option in a colour
/// picker. [`Tier::style`] must stay the single answer to "what does a Card look
/// like"; a row plate painted with its border and shadow would stack a second
/// hairline and a second shadow onto the panel it already sits on, and the
/// stack is what makes a dense list read as a grid of boxes. So this reuses
/// `fill()` and `radius()` and paints only those.
#[must_use]
pub fn wash_style(tier: Tier, theme: &Theme, track: &'static str) -> container::Style {
    let paint = super::alpha::Paint::start();
    paint.finish(wash_style_solve(tier, theme, track))
}

fn wash_style_solve(tier: Tier, theme: &Theme, track: &'static str) -> container::Style {
    let p = fade(track);
    container::Style {
        background: tier.fill(theme).map(|bg| bg.scale_alpha(p)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: tier.radius(theme).into(),
        },
        shadow: tokens::elevation(theme, 0.0),
        text_color: None,
        snap: true,
    }
}

/// [`wash_style`] in the shape iced wants, for `.style(...)` on an existing
/// container.
#[must_use]
pub fn wash<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    tier: Tier,
    padding: iced::Padding,
    track: &'static str,
) -> Container<'a, M> {
    container(content)
        .padding(padding)
        .style(move |theme: &Theme| wash_style(tier, theme, track))
}

/// The paint for a *mark*: a 2-3 px bar that points at where something will
/// land — a dock drop line, an insertion caret between strip icons, the slat
/// under the active tool.
///
/// Deliberately not a [`Tier`]: a mark is not a plane, and running it through
/// `surface()` would give it the panel hairline and an elevation shadow, which
/// is both invisible on a 2 px bar and the wrong hue. A mark is a solid accent
/// fill — the pencil coming down where the thing will land. No glow: the sheet
/// draws with one pen and adds no light.
#[must_use]
pub fn indicator_style(theme: &Theme, track: &'static str) -> container::Style {
    let p = fade(track);
    let accent = theme.palette().primary.base.color;
    container::Style {
        background: Some(Background::Color(accent.scale_alpha(p))),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: tokens::RADIUS_MARK.into(),
        },
        shadow: iced::Shadow::default(),
        text_color: None,
        snap: true,
    }
}

/// [`indicator_style`] wrapped onto content, the common case for a strip cell.
#[must_use]
pub fn indicator<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    padding: iced::Padding,
    track: &'static str,
) -> Container<'a, M> {
    container(content)
        .padding(padding)
        .style(move |theme: &Theme| indicator_style(theme, track))
}

/// An accent **wash**: the page's own ground tinted with the brand to point at
/// one thing inside it.
///
/// This is the container-side answer to [`crate::ui::kit::control::Role`] on a
/// button. A row in the properties palette, a cell in a dock strip or a drop
/// target is not a `Button` — it is a `container` (sometimes a `mouse_area`,
/// because a button fires on release and a drag has to start on press) — so
/// before this existed every one of them hand-wrote `primary.weak.color` at an
/// alpha someone had eyeballed, and the app ended up with four different
/// strengths for "this row". The rungs now live in
/// [`crate::ui::style::tokens::TINT_SELECTED`] and friends, one place, one
/// ladder, and the gradient stays where it belongs: on engaged controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tint {
    /// The row or cell the pointer is on, or the field that holds the value.
    Selected,
    /// A drop target open under the pointer. The loudest wash in the app.
    Dropped,
    /// A thing being dragged: still itself, no longer part of the page.
    Ghost,
    /// A small count or status plate pinned to a corner. Carries its own ink,
    /// because white-on-accent and ink-on-wash are not interchangeable.
    Badge,
    /// Brand colour read as warmth rather than as a target: the hero card.
    Hero,
}

#[must_use]
pub fn tint_style(tint: Tint, theme: &Theme, track: &'static str) -> container::Style {
    let paint = super::alpha::Paint::start();
    paint.finish(tint_style_solve(tint, theme, track))
}

fn tint_style_solve(tint: Tint, theme: &Theme, track: &'static str) -> container::Style {
    use crate::ui::style::tokens::{EDGE_RING, TINT_DROPPED, TINT_GHOST, TINT_HERO};
    let palette = theme.palette();
    let p = fade(track);
    let flush = Border {
        color: Color::TRANSPARENT,
        width: 0.0,
        radius: 0.0.into(),
    };
    match tint {
        // Rows and cells are flush rectangles: a corner on a 24px row that sits
        // inside a square panel just reads as a gap.
        Tint::Selected => container::Style {
            background: Some(Background::Color(
                tokens::selection_fill(theme).scale_alpha(p),
            )),
            border: flush,
            shadow: iced::Shadow::default(),
            text_color: None,
            snap: true,
        },
        // The loudest neutral the pen has: the pressed row / open drop target.
        // DraftFlow spends `active` here; the strength constants stay the
        // vendor ladder's answer so other themes do not move.
        Tint::Dropped => container::Style {
            background: Some(Background::Color({
                let fill = tokens::selection_press_fill(theme);
                if crate::ui::style::draftflow_theme::draftflow_colors(theme).is_some() {
                    fill
                } else {
                    palette.primary.weak.color.scale_alpha(TINT_DROPPED)
                }
            }
            .scale_alpha(p))),
            border: flush,
            shadow: iced::Shadow::default(),
            text_color: None,
            snap: true,
        },
        // A ghost is an object lifted out of the page, so it keeps the control
        // corner and gains the ring that separates it from what it covers. The
        // sheet lifts it as paper on paper: a surface fill and a strong line,
        // never a coloured glow.
        Tint::Ghost => {
            let (fill, ring) =
                match crate::ui::style::draftflow_theme::draftflow_colors(theme) {
                    Some(colors) => (colors.surface, colors.line_strong),
                    None => (
                        palette.primary.base.color.scale_alpha(TINT_GHOST),
                        palette.primary.base.color,
                    ),
                };
            container::Style {
                background: Some(Background::Color(fill.scale_alpha(p))),
                border: Border {
                    color: ring.scale_alpha(p),
                    width: EDGE_RING,
                    radius: tokens::RADIUS_CONTROL.into(),
                },
                shadow: iced::Shadow::default(),
                text_color: None,
                snap: true,
            }
        }
        Tint::Badge => container::Style {
            background: Some(Background::Color(palette.primary.base.color.scale_alpha(p))),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: tokens::RADIUS_CONTROL.into(),
            },
            shadow: iced::Shadow::default(),
            text_color: Some(palette.primary.base.text.scale_alpha(p)),
            snap: true,
        },
        Tint::Hero => container::Style {
            background: Some(match crate::ui::style::draftflow_theme::draftflow_colors(theme) {
                Some(colors) => Background::Color(colors.panel2.scale_alpha(p)),
                None => tokens::aurora_gradient(theme, TINT_HERO * p),
            }),
            border: Border {
                color: tokens::edge(theme).scale_alpha(p),
                width: 1.0,
                radius: tokens::RADIUS_PANEL.into(),
            },
            shadow: tokens::elevation(theme, 0.8),
            text_color: None,
            snap: true,
        },
    }
}

#[must_use]
pub fn tint<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    tint: Tint,
    padding: iced::Padding,
    track: &'static str,
) -> Container<'a, M> {
    container(content)
        .padding(padding)
        .style(move |theme: &Theme| tint_style(tint, theme, track))
}

/// A plane whose fill comes from the **drawing**, not from the theme: a
/// model-space viewport, a paper-space desk, a clean export thumbnail.
///
/// This is the one place a screen is allowed to hand a colour to the kit, and
/// it is not a style decision — the user's DWG background *is* the content, and
/// a viewport painted in the chrome's grey would lie about what is on screen.
/// Everything else about the plane stays in L1: no edge, no shadow, and the
/// entrance fade applied like anywhere else.
#[must_use]
pub fn document_style(fill: Color, track: &'static str) -> container::Style {
    container::Style {
        background: Some(Background::Color(fill.scale_alpha(fade(track)))),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        shadow: iced::Shadow::default(),
        text_color: None,
        snap: true,
    }
}

/// [`document_style`] onto content.
#[must_use]
pub fn document_plane<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    fill: Color,
    padding: iced::Padding,
    track: &'static str,
) -> Container<'a, M> {
    container(content)
        .padding(padding)
        .style(move |_| document_style(fill, track))
}

/// A page of paper that can be picked: the caller's fill as **data**, the
/// accent as the ring that says *this one is in the selection*.
///
/// Not [`document_style`] plus [`ring_style`] stacked at the call site. Two
/// containers would each eat their own border thickness out of a fixed tile,
/// and `ring_style` is a *focus* ring: aurora, `EDGE_RING`, `RADIUS_FIELD` —
/// the wrong colour, weight and corner for a selection that has to outlive the
/// pointer. And it is not [`swatch_style`] either: a swatch's edge is the
/// theme's hairline and exists so an invisible-on-the-plate colour still shows
/// its bounds, whereas this ring is the whole point of the plate and is off
/// until the row is chosen.
///
/// `picked` changes the ring's *colour*, never its width. The page-tile grid
/// this was written for is `132 × 86` fixed boxes, and the literal it replaced
/// toggled `width: 0.0 ↔ 2.5`, so clicking a page resized its own picture by
/// 2.5 px on every side — selection that moved the thing being selected.
/// The frame is therefore always [`tokens::EDGE_FRAME`] wide and unselected
/// just draws it transparent.
#[must_use]
pub fn paper_style(
    theme: &Theme,
    fill: Color,
    picked: bool,
    track: &'static str,
) -> container::Style {
    container::Style {
        background: Some(Background::Color(fill.scale_alpha(fade(track)))),
        border: Border {
            color: if picked {
                theme.palette().primary.base.color.scale_alpha(fade(track))
            } else {
                Color::TRANSPARENT
            },
            width: tokens::EDGE_FRAME,
            radius: tokens::RADIUS_CONTROL.into(),
        },
        shadow: iced::Shadow::default(),
        text_color: None,
        snap: true,
    }
}

/// The focus ring: the focus colour on an otherwise transparent container, for
/// the composite controls (a field wrapped in its own well) where the ring has
/// to sit on the wrapper rather than on the input.
#[must_use]
pub fn ring_style(theme: &Theme, track: &'static str) -> container::Style {
    container::Style {
        background: None,
        border: Border {
            color: tokens::focus_ring(theme).scale_alpha(fade(track)),
            width: tokens::EDGE_RING,
            radius: tokens::RADIUS_FIELD.into(),
        },
        shadow: iced::Shadow::default(),
        text_color: None,
        snap: true,
    }
}

/// The frame around the live viewport. `edge` is the caller's because that ring
/// is drawn in the drawing's own contrast colour, not the theme's; its width,
/// its square corner and its fade are decided here.
#[must_use]
pub fn frame_style(edge: Color, track: &'static str) -> container::Style {
    container::Style {
        background: None,
        border: Border {
            color: edge.scale_alpha(fade(track)),
            width: tokens::EDGE_FRAME,
            radius: 0.0.into(),
        },
        shadow: iced::Shadow::default(),
        text_color: None,
        snap: true,
    }
}

/// A colour swatch: the caller's colour as **data**, the theme's hairline as
/// chrome.
///
/// This is not [`document_style`] with an edge bolted on. `document_style` is
/// deliberately edgeless because it paints the drawing, and a drawing's bounds
/// are the viewport's business; a swatch has to keep its edge, because a white
/// swatch on a light popup and a black swatch on a dark one are both invisible
/// without it — which is how the ACI grid's colour 7 (white/black) used to read
/// as a hole in the panel. The fill stays exactly as the caller supplied it: no
/// tier, no wash, no theme mixing, because the colour *is* the information.
///
/// Two measured callers owned these literals first (`color_select.rs:114` and
/// `:182`), both at `RADIUS_MARK` on a 13px plate, both spelling the hairline by
/// hand — plus twelve more swatches in `window/options.rs` that had lost their
/// edge entirely.
#[must_use]
pub fn swatch_style(theme: &Theme, fill: Color, track: &'static str) -> container::Style {
    let paint = super::alpha::Paint::start();
    paint.finish(container::Style {
        background: Some(Background::Color(fill)),
        border: Border {
            color: tokens::edge(theme).scale_alpha(fade(track)),
            width: tokens::EDGE_HAIRLINE,
            radius: tokens::RADIUS_MARK.into(),
        },
        shadow: iced::Shadow::default(),
        text_color: None,
        snap: true,
    })
}

/// The non-data twin of [`swatch_style`]: a mark plate whose fill *is* a theme
/// role, for the glyph sitting inside it (the "?" on a mixed-selection colour, a
/// "+N" overflow). Same square, same corner, same hairline; the fill and ink come
/// from [`tokens::Control::Strong`] so the plate stays one step above the panel it
/// sits in in every theme. Without this the two plates that share a row would
/// disagree: the swatch follows the drawing, the marker follows the theme.
#[must_use]
pub fn mark_plate_style(theme: &Theme, track: &'static str) -> container::Style {
    let step = tokens::Control::Strong;
    let paint = super::alpha::Paint::start();
    paint.finish(container::Style {
        background: Some(Background::Color(tokens::control_plate(theme, step))),
        border: Border {
            color: tokens::edge(theme).scale_alpha(fade(track)),
            width: tokens::EDGE_HAIRLINE,
            radius: tokens::RADIUS_MARK.into(),
        },
        shadow: iced::Shadow::default(),
        text_color: Some(tokens::control_ink(theme, step)),
        snap: true,
    })
}

/// The edge a swatch wears, in the three states every colour cell has.
///
/// One rule, read off the L0 edge ladder: at rest the theme's own hairline at
/// [`tokens::EDGE_HAIRLINE`]; under the pointer the theme's ink at
/// [`tokens::EDGE_RING`]; when it *is* the current colour, the accent at
/// [`tokens::EDGE_FRAME`]. Hover deliberately does **not** use the accent —
/// `color_select.rs` used to paint hover and selection with the same primary, so
/// the pointer looked like a choice had already been made.
fn swatch_border(theme: &Theme, selected: bool, hovered: bool) -> Border {
    let palette = theme.palette();
    Border {
        color: if selected {
            palette.primary.base.color
        } else if hovered {
            palette.background.base.text
        } else {
            tokens::edge(theme)
        },
        width: if selected {
            tokens::EDGE_FRAME
        } else if hovered {
            tokens::EDGE_RING
        } else {
            tokens::EDGE_HAIRLINE
        },
        radius: tokens::RADIUS_MARK.into(),
    }
}

/// A clickable colour cell: the caller's colour as **data**, the kit's edge,
/// corner and selected ring as chrome.
///
/// This is the primitive [`crate::ui::color_select`]'s own comment asked for:
/// `chip_button_style` and `list_row` both own the background, which would erase
/// the swatch, so what is missing was never a role — it was a style that takes a
/// fill and supplies only the frame around it. [`swatch_style`] is that shape for
/// a cell that cannot be selected; this is its stateful twin.
///
/// The fill passes through untouched. No tier, no wash, no theme mixing: the
/// colour *is* the information, and a palette cell that greys out on hover stops
/// being a colour sample.
#[must_use]
pub fn swatch_button_style(
    fill: Color,
    selected: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme: &Theme, status| {
        let paint = super::alpha::Paint::start();
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        paint.finish(button::Style {
            background: Some(Background::Color(fill)),
            border: swatch_border(theme, selected, hovered),
            ..Default::default()
        })
    }
}

/// The non-clickable twin of [`swatch_button_style`]: same fill-as-data rule and
/// same selected ring, but a `container` because the press belongs to a row that
/// wraps it (a named-colour list line, a property label).
///
/// It cannot hover, so it never claims a hover edge — that absence is the whole
/// difference from the button form, not a second design.
#[must_use]
pub fn swatch_plate_style(fill: Color, selected: bool) -> impl Fn(&Theme) -> container::Style {
    move |theme: &Theme| container::Style {
        background: Some(Background::Color(fill)),
        border: swatch_border(theme, selected, false),
        shadow: iced::Shadow::default(),
        text_color: None,
        snap: true,
    }
}

/// The ink that stays readable on top of a colour cell the caller supplied.
///
/// This is the Rec.709 luma against the 0.55 threshold that
/// `window/graphic_attributes.rs` has always used, moved into L1 rather than
/// invented: three screens type the same weighted sum by hand, and a threshold
/// that lives in a screen is a threshold a fourth screen will re-guess slightly
/// differently.
///
/// **It is deliberately not [`tokens::legible_on`].** `legible_on` iterates until
/// it clears a WCAG ratio and will therefore return a mid-grey rather than one of
/// two extremes; that is a better answer to "is this readable", and a different
/// answer to "what does a colour sample's label look like on every theme". Making
/// this cell call `legible_on` changes ink on shipped swatch labels, which is a
/// sample-on-screen decision for #48, not a fold. Recorded here so the next reader
/// does not "simplify" the two into one and move pixels silently.
#[must_use]
pub fn swatch_edit_ink(fill: Color) -> Color {
    if fill.r * 0.299 + fill.g * 0.587 + fill.b * 0.114 > 0.55 {
        Color::BLACK
    } else {
        Color::WHITE
    }
}

/// A colour cell you press **to edit** the colour, as opposed to
/// [`swatch_button_style`]'s cell you press to select it.
///
/// The difference is one word in the border and one line of ink, and both are
/// load-bearing. There is no selected state, so nothing may claim a selected
/// ring; the pointer instead gets the brand's own hairline (not `base.text` at
/// [`tokens::EDGE_RING`]), because this plate is not a member of a grid where one
/// cell must outshine the others — it is a field that opens a colour window. And
/// it supplies ink, because the label drawn over a user's colour is the one place
/// where the kit must decide readability *for* data it does not own.
#[must_use]
pub fn swatch_edit_style(fill: Color) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: Some(Background::Color(fill)),
            text_color: swatch_edit_ink(fill),
            border: Border {
                color: if hovered {
                    theme.palette().primary.base.color
                } else {
                    tokens::edge(theme)
                },
                width: tokens::EDGE_HAIRLINE,
                radius: tokens::RADIUS_MARK.into(),
            },
            ..Default::default()
        }
    }
}

/// The plate of a control that shows the current value and opens a list: a fill
/// picker, a linetype chooser, a hatch selector.
///
/// It is not [`crate::ui::kit::control::style_for`] with a border bolted on, and
/// it is not `Role::Engaged`: the engaged role carries the aurora wash because it
/// says "this one of twelve is the active one", while this control says "this is
/// the drawing's current value, and a list is behind it". Its ground is therefore
/// the page's own [`crate::ui::kit::ground_style`] and its ink the page's ink, so
/// opening it never makes the panel change brightness. Only the edge answers the
/// open state, and it answers with the brand hairline at the width the page edge
/// already uses — the same fact a focused panel border carries below.
#[must_use]
pub fn picker_trigger_style(
    engaged: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme: &Theme, status| {
        let lifted = engaged
            || matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: Some(Background::Color(theme.palette().background.base.color)),
            text_color: theme.palette().background.base.text,
            border: Border {
                color: if lifted {
                    theme.palette().primary.base.color
                } else {
                    tokens::edge(theme)
                },
                width: tokens::EDGE_HAIRLINE,
                radius: tokens::RADIUS_MARK.into(),
            },
            ..Default::default()
        }
    }
}

/// The plate and the ink of a **pickable data tile**: a cell in a grid whose fill
/// is the item it stands for, and one of which can be armed.
///
/// Moved out of `ui/window/block_palette.rs` by the twenty-second screen, the leg
/// that drained that file's last hand-written `container::Style`. The state
/// machine is verbatim; only the parameter is renamed to the kit's word for "the
/// plate *is* the readout" — `engaged`, which [`picker_trigger_style`] already
/// spends — instead of the screen's `is_placing`.
///
/// It is deliberately not a [`Tier`] and not [`swatch_style`]: a tier answers
/// "what is content resting on", and `swatch_style` answers "frame a colour the
/// drawing owns". This answers the third thing — a tile that is *about* an item,
/// so its fill can be replaced by the brand when the item is the pending action,
/// and its ink has to be solved together with that fill or the label goes
/// unreadable at the exact moment the tile means "click me next".
///
/// The `Hovered`/`Pressed` arm has no caller inside the palette, because a
/// container style closure is never handed a status; it stays because the
/// composite form below takes the status anyway, so a button-tile version of this
/// plate gets the same answer, and because
/// [`crate::ui::theme_accessibility_tests`] holds all four states to AA on every
/// theme. Deleting the arm would be deleting tested behaviour to make a fold look
/// cheaper.
#[must_use]
pub fn picker_tile_colors(
    theme: &Theme,
    engaged: bool,
    status: button::Status,
) -> (Color, Color) {
    let palette = theme.palette();
    if engaged {
        (palette.primary.base.color, palette.primary.base.text)
    } else {
        match status {
            button::Status::Hovered | button::Status::Pressed => (
                palette.background.strong.color,
                palette.background.strong.text,
            ),
            _ => (palette.background.weak.color, palette.background.weak.text),
        }
    }
}

/// The line around a [tile](picker_tile_colors): the brand while it is armed, the
/// theme's own hairline otherwise.
#[must_use]
pub fn picker_tile_border(theme: &Theme, engaged: bool) -> Color {
    if engaged {
        theme.palette().primary.base.color
    } else {
        tokens::edge(theme)
    }
}

/// The two answers above assembled into the plate the block palette used to spell
/// by hand — fill, ink, [`tokens::EDGE_HAIRLINE`] under [`tokens::RADIUS_FIELD`],
/// and nothing else. `shadow`, `snap` and the rest stay iced's defaults because
/// that is what the screen was doing; a tile that grew an elevation would be a
/// plate decision, and plate decisions are [#48]'s, not a refactor leg's.
#[must_use]
pub fn picker_tile_style(
    theme: &Theme,
    engaged: bool,
    status: button::Status,
) -> container::Style {
    let (fill, ink) = picker_tile_colors(theme, engaged, status);
    container::Style {
        background: Some(Background::Color(fill)),
        text_color: Some(ink),
        border: Border {
            color: picker_tile_border(theme, engaged),
            width: tokens::EDGE_HAIRLINE,
            radius: tokens::RADIUS_FIELD.into(),
        },
        ..Default::default()
    }
}

/// The plate of a detached surface — a tooltip, a popup, a floating panel — as a
/// plain `fn` item, so a call site can hand it to `.style` instead of writing the
/// closure that only exists to drop `iced`'s extra arguments.
///
/// Measured, not composed. The fact "a detached surface wears the Float plate"
/// had **sixty** owners before this name existed: **thirty-two** of them spell the
/// call — **twenty-nine** inline closures of the shape
/// `|theme| Tier::Float.style(theme, "")` across **fourteen** files
/// (`ui/color_select.rs`, `ui/command_line.rs` ×4, `ui/node_graph.rs`,
/// `ui/popup/cycle_popup.rs`, `ui/properties.rs` ×4, `ui/statusbar/mod.rs` ×3,
/// `ui/window/annotation_data.rs`, `ui/window/count_palette.rs`,
/// `ui/window/graphic_attributes.rs` ×3, `ui/window/named_parameters.rs`,
/// `ui/window/xref_manager.rs` ×3, `ui/app/dock/view.rs` ×2, `ui/app/view/mod.rs` ×5,
/// `ui/app/view/overlay.rs`), one of them inside `ui/app/view/mod.rs:1499` written
/// outside a closure, and the last two inside the ribbon's local aliases — plus
/// **twenty-eight** further sites that reach the plate *through* those two aliases
/// (`tip_style` 17 times, `popup_panel_style` 11). The aliases were deleted by the
/// eighteenth screen, which is where the twenty-eight came from.
///
/// The closure is not a choice about paint: it exists only to drop the motion-track
/// argument that [`Tier::style`] takes and iced's container `StyleFn` does not.
///
/// It is [`Tier::Float`] and nothing local, which is why there is no colour or
/// radius spelled here. A detached surface that animates its own fade has to pass a
/// track and does *not* take this — `ui/shell/dialog.rs:214` is that case
/// (`"modal:fade"`), and pinning the empty track here is what keeps the modal's
/// motion label from being copied into every tooltip by a tree-wide substitution.
///
/// ## Where it is left standing
///
/// Re-measured after the glass rollout: **one** caller,
/// `ui/window/annotation_data.rs`'s browser-only "add a folder" tip, inside a
/// `#[cfg(target_arch = "wasm32")]` block. Everything else this name used to
/// paint — the colour lists, the right-click menus, the ribbon's dropdowns, the
/// drag ghost, the block gallery, the hover lists — reads the frame under it
/// through [`crate::ui::kit::glass::glass_pane`] instead, and a pane cannot be
/// painted by a `StyleFn` because its material is a shader, not a fill. The
/// guard in `ui/style/kit_contract_tests.rs` asserts both halves: that no call
/// site names `Tier::Float` outside this module, and that this function's only
/// user is that wasm block. A second user appearing here is the signal that a
/// floating surface was painted opaque again.
#[must_use]
pub fn float_style(theme: &Theme) -> container::Style {
    Tier::Float.style(theme, "")
}

/// The paper a dropdown or a shortcut menu is written on: the `Float` rung,
/// with the edge of a field rather than the edge of a chrome seam.
///
/// The rung's own edge is [`tokens::edge`] — the hairline *between two chrome
/// planes* (`line` on the sheet). On the dark sheet `line` (#2A2F35) is darker
/// than the `Float` fill it outlines (#282C32), so a menu floating over the
/// canvas has no contour at all: the fill and the canvas differ by five steps
/// and the border pulls the silhouette back down. That is the "transparent"
/// shortcut menu the 2026-11-02 review caught. A menu is an object the size
/// of a field floating over the drawing, so it takes the field's edge
/// (`line_strong`, #3A4048), the same 1px frame the layer palette and the
/// properties panel already wear. Fill, corner (16), padding and the sheet's
/// deliberate lack of a cast shadow stay the ladder's own: this name changes
/// the contour and nothing else, which is what keeps a menu beside a card
/// reading as the same family rather than as a second material.
///
/// Vendor themes are unaffected in practice — their `field_edge` is `edge`.
#[must_use]
pub fn popup_plate<'a, Message>(
    content: Element<'a, Message>,
    track: &'static str,
) -> Element<'a, Message>
where
    Message: 'a,
{
    container(content)
        // The rows of a menu are flush rectangles, and a container's corner
        // does not clip its children — so a hovered row would paint a square
        // over the rounded corner unless the content is held inside it. A
        // corner of radius R is cleared at R·(1 − 1/√2) ≈ 0.29R; the sheet's
        // 16 px card corner is therefore fully inside a 5 px inset. One gap
        // rung does it in both schemes and both ladders.
        .padding(tokens::GAP_CONTROL)
        .style(move |theme: &Theme| {
            // This plate is the outermost painter of a popup: `Tier::style`
            // runs inside it and therefore stays whole, and the assembled
            // plate takes the entrance alpha once, so a menu arriving out of
            // its anchor fades its paper and its rows together.
            let paint = super::alpha::Paint::start();
            let p = fade(track);
            let mut style = Tier::Float.style(theme, track);
            style.border.color = tokens::field_edge(theme).scale_alpha(p);
            paint.finish(style)
        })
        .into()
}

/// The paper a dialog is written on: the `Float` rung, carrying its own motion
/// track.
///
/// A dialog is the one detached surface that is a *sheet* rather than a pane —
/// the batch-047 screens draw it as an opaque card over the dimmed frame, and a
/// glass tint alone let the canvas grid show between the dialog's own groups.
/// [`float_style`] pins the empty track for tooltips; a modal fades in with
/// `"modal:fade"` and therefore cannot take it, which is why this second name
/// exists inside the kit rather than at the call site: the census in
/// `ui/style/kit_contract_tests.rs` still has no surface outside `ui::kit`
/// naming the rung, and no second caller of `float_style`.
#[must_use]
pub fn sheet_style(theme: &Theme, track: &'static str) -> container::Style {
    Tier::Float.style(theme, track)
}

/// The colour [`sheet_style`] paints, for a surface that has to fade *into* a
/// sheet rather than sit on one.
///
/// A strip that scrolls sideways ends in a scrim, and the scrim's opaque end has
/// to be the card's own ground or the fade reads as a grey bar laid over the
/// card. Asking the ladder for that colour at the call site would name the
/// floating rung outside the kit, which the census forbids — so the kit hands
/// the colour out by name and keeps the rung to itself.
#[must_use]
pub fn sheet_ground(theme: &Theme) -> Color {
    Tier::Float
        .paint(theme)
        .unwrap_or_else(|| theme.palette().background.base.color)
}

/// The ground a window paints itself with: the theme's own base, untouched.
///
/// Distinct from [`crate::ui::kit::Tier`]'s plates, which are *above* the ground
/// by one rank, and distinct from [`page_style`], which exists to walk between two
/// schemes while a theme switch animates. A screen that only wants "the page colour
/// here and now" was previously rebuilding `page_style` with a dummy `from`, which
/// reads as a decision about motion when it is a decision about nothing at all.
#[must_use]
pub fn ground_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme.palette().background.base.color)),
        ..Default::default()
    }
}

/// The plate a confirmation guard puts over a dialog's own ground: the page
/// colour, separated from the page it covers by the page's own edge, on the
/// corner a clickable thing wears.
///
/// Measured, not composed: exactly two screens type this triple, and they type
/// it identically — `ui/window/shortcuts.rs` and `ui/window/alias_editor.rs`
/// both build the unsaved-changes guard out of the same ground, the same
/// `tokens::edge` hairline, the same `RADIUS_CONTROL`, the same `GAP_GUTTER`
/// inset and the same `Length::Fixed(320.0)`. The duplicate is the reason for
/// the name; each screen is landed on its own leg, because substituting across
/// screens is the batch edit this kit forbids.
///
/// It is not [`ground_style`], which carries no line because it *is* the page —
/// a guard has to separate itself from what it covers. And it is not
/// [`crate::ui::kit::glass::glass_pane`], which is what a panel that *is* the
/// object of attention wears now: a pane says "look here, and here is what lies
/// under me" by reading the frame beneath it. A warning is not a selection, and
/// a guard that dimmed the document behind it would make an alert look like a
/// tool. Outlining a discard prompt in brand ink — the plate this kit used to
/// offer for "chosen" — had the same defect: it read an alert as a state of the
/// document.
///
/// The corner is `RADIUS_CONTROL` because everything the panel holds is a
/// button, and that rung is defined as "buttons, list rows, chips: anything you
/// click" — the panel takes the corner of its contents, not its own.
#[must_use]
pub fn guard_panel_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme.palette().background.base.color)),
        border: iced::Border {
            color: tokens::edge(theme),
            width: tokens::EDGE_HAIRLINE,
            radius: tokens::RADIUS_CONTROL.into(),
        },
        ..Default::default()
    }
}

/// A flat band laid on the page: the resting control plate, square, no line.
///
/// Measured before it was named — **eight** screens hand-type this exact pair
/// (`background.weak` + `..Default::default()`): the five style managers' toolbars
/// (`ui/style/style_manager.rs`, landed by the twenty-first leg, and the copy
/// `ui/style/dimstyle.rs` carried until the twentieth leg deleted it; still hand-typed
/// in `ui/style/anno_object_scale.rs`, `ui/style/plotstyle.rs`,
/// `ui/style/scale_manager.rs`), the command browser's section head
/// (`ui/window/browser.rs`) and three popup headers
/// (`ui/popup/{snap_popup,units_popup,selection_filter_popup}.rs`). One decision,
/// eight owners, which is the standing reason a pair belongs at L1. Each screen is
/// landed on its own leg; this is the tier they land on. Addresses are not quoted:
/// every leg moves the lines below it, and a doc that pins a number goes stale
/// before the screen it describes.
///
/// The whole field, for the leg that comes next: the iced read spelled
/// `palette().` + `background.weak` + `.color` stands at **15 lines over 14 files**
/// on 2026-10-19, once this page's own spelling of it is excluded — a doc that
/// quotes a search pattern is a doc that changes the pattern's count, which is
/// worth saying out loud because the next reader will grep it
/// (`snap_popup` 2, then one each in `recovery`, `print_all`, `pc_manager`,
/// `browser`, `block_palette`, `tablestyle`, `scale_manager`, `plotstyle`,
/// `mlstyle`, `anno_object_scale`, `units_popup`, `selection_filter_popup`,
/// `node_graph`). That is the queue for #48, not a claim that fourteen more
/// belong here — the field is also spent on shapes that are neither a band nor a
/// pane, and which is which is decided from the shape, per leg.
///
/// The fill reads the control-plate ladder, not iced's field: decision A made
/// [`tokens::Control::Weak`] *equal by definition* to `palette.background.weak`,
/// and `the_control_plate_ladder_is_exact` pins that against iced's own palettes on
/// all 26 themes — so this reproduces the hand-typed pixel for pixel and costs
/// nothing to fold.
///
/// It is not [`crate::ui::kit::Tier::Band`], whose fill comes off the *surface*
/// ladder. [`tokens`] records why the two are not interchangeable (up to 21.57 CIE
/// L\* apart, on three themes on opposite sides of their own page) and leaves the
/// reconciliation to the sample-shot gate at #48; a screen that needs the band rung
/// takes `Tier::Band` on that leg, with the measured plate move printed. And it is
/// not [`strip_style`], which is the same band *with a seam under it* — a manager
/// toolbar already has [`hdivider`] below it and drawing a hairline through that
/// divider is the double line the ribbon had to be corrected for.
#[must_use]
pub fn chrome_plate_style(theme: &Theme) -> container::Style {
    let paint = super::alpha::Paint::start();
    paint.finish(container::Style {
        background: Some(Background::Color(tokens::control_plate(
            theme,
            tokens::Control::Weak,
        ))),
        ..Default::default()
    })
}

/// The plate a bounded pane wears: the resting control fill, the theme's own
/// hairline, the field corner.
///
/// Measured before it was named — **eight** screens hand-type this exact triple
/// (`background.weak` + `tokens::edge` + [`tokens::EDGE_HAIRLINE`] +
/// [`tokens::RADIUS_FIELD`]): the two panes of the shared style-manager frame
/// (the style list and the preview plate in `ui/style/style_manager.rs`, both landed
/// by the twenty-first leg), the same two roles in three sibling managers
/// (`ui/style/{anno_object_scale,plotstyle,scale_manager}.rs`), the recovery list
/// (`ui/window/recovery.rs`), the block palette's browse pane
/// (`ui/window/block_palette.rs`) and the print-all preview (`ui/window/print_all.rs`).
/// Addresses are not quoted, for the reason given on [`chrome_plate_style`].
///
/// Geometry is what makes this a primitive rather than a preference: the fill, the
/// line and the corner are the *same three tokens* at all eight sites, and the only
/// thing that varies between them is which content they bound. The inset stays with
/// the screen — a list well and a preview plate are the same plane wearing different
/// padding, and folding padding in here would put a ninth number on a surface that
/// is supposed to answer one question.
///
/// Deliberately not [`crate::ui::kit::Tier::Well` even though the corner and the line
/// already match: `Well` fills from the *recess* ladder (`tokens::inset`), which is a
/// plane below the page, while these eight panes are filled one step *above* it. The
/// two are not a naming disagreement, they are two different answers to "is this
/// content pressed into the window or laid on top of it", and switching eight panes
/// from one answer to the other is a plate move of |ΔL\*| 0.98–6.29 on the 26 themes
/// the fold ledger measures. That decision belongs to #48's sample shots, not to a
/// refactor leg. What this function takes over from the screens is the *call*: L3
/// stops spelling iced's palette fields, and when #48 decides the plane, it decides
/// it here once.
#[must_use]
pub fn pane_plate_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(tokens::control_plate(
            theme,
            tokens::Control::Weak,
        ))),
        border: iced::Border {
            color: tokens::edge(theme),
            width: tokens::EDGE_HAIRLINE,
            radius: tokens::RADIUS_FIELD.into(),
        },
        ..Default::default()
    }
}

/// A plate that is **nothing but a fill**: no line, no corner, no shadow, no ink.
///
/// Measured before it was named — two screens type this exact one-field block and
/// nothing else: a docked panel's footer band and a modal's click-blocking
/// backdrop. The shape is shared, the colour is not, which is why the fill is a
/// parameter rather than a rung.
///
/// Not [`ground_style`], which spells the page's own base colour instead of taking
/// one; not [`crate::ui::kit::document_style`], which applies the entrance fade and
/// `snap: true` because it paints a viewport that arrives with the window; and not
/// [`scrim_style`], which owns the modal dim at L1's own strength. The backdrop
/// that calls this one dims at its own number, and reconciling the two strengths is
/// #48's sample-shot decision, not a refactor leg's.
#[must_use]
pub fn plain_plate_style(fill: Background) -> container::Style {
    container::Style {
        background: Some(fill),
        ..Default::default()
    }
}

/// A fill plus the field corner and no line: the plate a bounded thing wears when
/// its bounds are the corner, not an outline.
///
/// Measured before it was named — three screens type this exact two-field block:
/// the progress bar's filled portion (which hands the kit a full-strength aurora),
/// the same bar's unfilled track (the strongest plate on the page) and a tree
/// panel's card body. The fill is a parameter because those three are three
/// different answers to "what is this plate made of"; the corner is not, because
/// all three already read [`tokens::RADIUS_FIELD`].
///
/// Not [`pane_plate_style`], which adds the theme hairline: drawing a line around
/// an 8 px progress bar is new ink, not a renaming. Not [`swatch_plate_style`],
/// whose whole point is the ring that says *this one is picked*. And not
/// [`fieldset_well_style`], which is the line without the fill — the recess, not
/// the plate laid into it.
#[must_use]
pub fn field_radius_plate_style(fill: Background) -> container::Style {
    container::Style {
        background: Some(fill),
        border: Border {
            radius: tokens::RADIUS_FIELD.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// The plate a floating card wears: the resting control fill, the page's own
/// hairline, and the corner of the things inside it.
///
/// One owner today — the open-progress card. It earns the name anyway because the
/// geometry is none of its neighbours', and the two closest ones each differ by
/// exactly one field, which is the shape of a decision somebody would otherwise
/// re-type:
///   * [`guard_panel_style`] is the same line and the same
///     [`tokens::RADIUS_CONTROL`] corner but filled with the *page ground*, one rung
///     lower — a guard separates itself from what it covers, while this card is the
///     surface the dialog put on the page.
///   * [`pane_plate_style`] is the same fill and the same line on
///     [`tokens::RADIUS_FIELD`], which is 5.0 where this corner is 7.0 — folding
///     this site onto it would take 2 px off every corner of the card.
///
/// The fill reads the plate ladder (`tokens::Control::Weak`) rather than iced's
/// palette field, exactly as [`pane_plate_style`] does: decision A made the two
/// *equal by definition* and `the_control_plate_ladder_is_exact` pins that against
/// all 26 selectable themes, so this reproduces the hand-typed pixel for pixel.
#[must_use]
pub fn card_plate_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(tokens::control_plate(
            theme,
            tokens::Control::Weak,
        ))),
        border: Border {
            color: tokens::edge(theme),
            width: tokens::EDGE_HAIRLINE,
            radius: tokens::RADIUS_CONTROL.into(),
        },
        ..Default::default()
    }
}

/// The solid plate a list row wears while it is the picked row.
///
/// One owner today. It is a name and not a fold because the three L1 relatives that
/// also mean "selected" are measurably different plates, and a reader arriving after
/// this one will pattern-match on the word:
///   * [`selected_row_wash_style`] is the *wash* the style lists wear — an aurora
///     gradient at [`tokens::WASH_ROW`] over a flush border, see-through, owning no
///     fill colour of its own.
///   * `tint_style(Tint::Selected)` is `primary.weak` scaled to
///     [`tokens::TINT_SELECTED`] (0.12), so the page shows through the row.
///   * [`swatch_plate_style`] adds an accent ring when picked, which a row already
///     inside a panel implies.
/// This one is the accent's own colour at full alpha with the accent's own ink:
/// opaque, because the row it paints is a configuration row whose text has to stay
/// readable *over* the plate rather than through it.
///
/// Deliberately not what `ui/node_graph.rs` does with the same two fields: a node
/// header paints `primary.weak` **unconditionally**, so it is a heading wearing the
/// accent, not a selection state. Calling this with `true` would make the flag a lie.
#[must_use]
pub fn row_selection_plate_style(theme: &Theme, selected: bool) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: selected.then(|| Background::Color(palette.primary.weak.color)),
        text_color: selected.then(|| palette.primary.weak.text),
        ..Default::default()
    }
}

/// The well a picture sits in: the page ground, the strongest line on the page,
/// the mark corner.
///
/// One owner today — an attach-external-reference dialog's image preview. The corner
/// is what makes it its own plate rather than a member of [`fieldset_well_style`]:
/// that fn's doc already names this site as a deliberate non-member because it wears
/// [`tokens::RADIUS_MARK`] (2.0) where the well wears [`tokens::RADIUS_FIELD`] (5.0).
/// It is not [`swatch_style`] either, whose line is the theme's hairline (a light
/// preview would lose its bounds on a light panel) and which fades and snaps because
/// a swatch arrives with its popup.
///
/// The line reads `background.strong` off the palette, the same read
/// [`fieldset_well_style`] makes; retiring that spelling is #48's queue, and nothing
/// moves here.
#[must_use]
pub fn preview_well_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(Background::Color(palette.background.base.color)),
        border: Border {
            color: palette.background.strong.color,
            width: tokens::EDGE_HAIRLINE,
            radius: tokens::RADIUS_MARK.into(),
        },
        ..Default::default()
    }
}

/// The brand's own plate: the accent as a fill, the ink that reads on it, and
/// nothing else — no edge, no corner, no shadow.
///
/// Cut for the head bar of a docked sheet, which is square because the sheet it
/// caps is square. It is not `tint_style(Tint::Badge)`: that plate carries
/// `RADIUS_CONTROL` on its corner, so routing a square head through it rounds it.
/// It is not [`picker_tile_style`] either, whose `engaged` arm adds an
/// `EDGE_HAIRLINE` under `RADIUS_FIELD` — new ink on a bar that never had a line.
/// The `(fill, ink)` pair *is* the engaged arm of [`picker_tile_colors`], which is
/// the evidence that the pair is a real role; only the tile's border is not this.
#[must_use]
pub fn accent_plate_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(Background::Color(palette.primary.base.color)),
        text_color: Some(palette.primary.base.text),
        ..Default::default()
    }
}

/// [`frame_style`]'s ring with the page showing through under it.
///
/// `frame_style` answers the ring and nothing else — its own body sets
/// `background: None` — so a sheet that wants the drawing visible beneath its own
/// frame has to restate the fill, and that restatement is the row. The alpha stays
/// an argument rather than a rung: it is this one sheet's decision about how much
/// page shows through, no column measures it (finding F4), and naming it at L0
/// would hand #58 a collision for no measured gain.
#[must_use]
pub fn ghost_sheet_style(theme: &Theme, alpha: f32) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(Background::Color(
            palette.background.base.color.scale_alpha(alpha),
        )),
        ..frame_style(palette.primary.base.color, "")
    }
}

/// A ring with nothing inside it: the caller's colour as a border on
/// [`tokens::EDGE_FRAME`], the field corner, no fill.
///
/// Cut for the navigation marker, which is a `Space` — a thing with no content to
/// ink — so `text_color` stays iced's default here rather than being decided.
/// It is not [`guard_panel_style`], which fills with `background.base` and
/// outlines with one hairline; the marker has neither a page-coloured plate nor a
/// hairline to carry. It is not [`frame_style`], which fades its edge by
/// `fade(track)`, pins `radius: 0.0` and sets `snap: true`.
///
/// The site typed its border width as a bare `2.`; [`tokens::EDGE_FRAME`] is 2.0,
/// so this names the number that was already reaching iced.
#[must_use]
pub fn marker_ring_style(edge: Color) -> container::Style {
    container::Style {
        border: Border {
            color: edge,
            width: tokens::EDGE_FRAME,
            radius: tokens::RADIUS_FIELD.into(),
        },
        ..Default::default()
    }
}

/// A pickable row that previews its own hover before it is committed: the hover
/// plate, the ink for whichever state is hotter, the field corner with no line.
///
/// Both flags are arguments because they are two different questions — the row the
/// pointer is over is not the row the setting is on, and this row paints the
/// pointer's half but only *tints* its ink for either. It is not
/// [`row_selection_plate_style`], which fills with `primary.weak` and has no hover
/// half; not `tint_style(Tint::Selected)`, whose fill is a 0.12 wash of the accent
/// rather than the theme's own `background.strong`. The border is radius-only, the
/// shape no neighbour carries: a corner with no line.
#[must_use]
pub fn option_row_style(theme: &Theme, highlighted: bool, selected: bool) -> container::Style {
    let palette = theme.palette();
    let background = highlighted.then(|| Background::Color(palette.background.strong.color));
    let text_color = if highlighted || selected {
        palette.background.strong.text
    } else {
        palette.background.base.text
    };
    container::Style {
        background,
        text_color: Some(text_color),
        border: Border {
            radius: tokens::RADIUS_FIELD.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// A node card's title band: the accent's weak pair, fill and ink together,
/// unconditionally.
///
/// Cut for `ui/node_graph.rs`'s node header. It is not
/// [`row_selection_plate_style`], whose `selected` flag would be a lie here —
/// the band paints `primary.weak` whether or not the node is selected, because
/// it is the node's *name*, not its state. No edge, no corner: the card under
/// it ([`node_plate_style`]) carries both.
#[must_use]
pub fn heading_plate_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(Background::Color(palette.primary.weak.color)),
        text_color: Some(palette.primary.weak.text),
        ..Default::default()
    }
}

/// A node card's body: the page ground with the strong edge and the field
/// corner.
///
/// Cut for `ui/node_graph.rs`'s card shell. It is [`fieldset_well_style`] plus
/// the `background.base` fill — the well leaves its fill to the page, while a
/// node floats over the canvas and must occlude it. It is not
/// [`preview_well_style`], whose corner is [`tokens::RADIUS_MARK`].
#[must_use]
pub fn node_plate_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(Background::Color(palette.background.base.color)),
        border: Border {
            color: palette.background.strong.color,
            width: tokens::EDGE_HAIRLINE,
            radius: tokens::RADIUS_FIELD.into(),
        },
        ..Default::default()
    }
}

/// A node's link port: the signal ring, filled when a wire lands on it.
///
/// Cut for `ui/node_graph.rs`'s `port_dot`. The fill *is* the flag — an empty
/// ring is an open port — so `linked` is an argument, not two functions. The
/// site typed its ring width as a bare `1.5`; [`tokens::EDGE_RING`] is 1.5, so
/// this names the number that was already reaching iced. Under the sheet a
/// link is drawing state, so it wears the vermilion; vendor themes keep the
/// aurora.
#[must_use]
pub fn port_dot_style(theme: &Theme, linked: bool) -> container::Style {
    let ring = match draftflow_colors(theme) {
        Some(colors) => colors.accent,
        None => tokens::aurora_hi(theme),
    };
    container::Style {
        background: linked.then(|| Background::Color(ring)),
        border: Border {
            color: ring,
            width: tokens::EDGE_RING,
            radius: tokens::RADIUS_FIELD.into(),
        },
        ..Default::default()
    }
}

/// An about-panel fact card: the weakest fill, the deviated edge, the panel
/// corner.
///
/// Cut for `ui/window/about.rs`'s `surface_style` (three consumers: the info
/// cards, the info rows, the hero). It is not [`Tier::Card`], whose fill is a
/// computed ladder step with elevation and `snap: true`; this plate paints the
/// palette's own `weakest` and casts no shadow.
#[must_use]
pub fn inset_card_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(Background::Color(palette.background.weakest.color)),
        border: Border {
            color: tokens::edge(theme),
            width: tokens::EDGE_HAIRLINE,
            radius: tokens::RADIUS_PANEL.into(),
        },
        ..Default::default()
    }
}

/// The body plate of a dialog: a fill, a corner, no line.
///
/// This is iced's `container::rounded_box` moved into L1, not a new plane, and
/// that is why it is allowed to exist at all. Decision A made
/// `palette.background.weak` (the vendor's fill) *equal by definition* to
/// `tokens::Control::Weak`, and `the_control_plate_ladder_is_exact` pins all 182
/// plate/in-cell pairs against iced's own palettes — so reading the ladder here
/// reproduces the vendor pixel-for-pixel instead of approximating it. Same
/// argument as [`crate::ui::kit::control::subtle_style`], one sheet over.
///
/// Three details are load-bearing:
///   * The border is built with `iced::border::rounded`, the same call the vendor
///     makes, so "transparent, zero width, radius 2.0" cannot drift into
///     "hairline edge" the way a hand-written [`Border`] literal could.
///     `RADIUS_CONTROL` is the kit's name for 2.0, not a new number.
///   * `shadow` and `snap` come from `..container::Style::default()` rather than
///     being spelled here, because the vendor takes them the same way. `snap`'s
///     default is `renderer::CRISP` (`cfg!(feature = "crisp")`, off in this
///     manifest), so writing `snap: true` like the tiers above do would have made
///     this sheet *not* a replacement for `rounded_box` while still compiling —
///     a difference you only see in the pixels.
///   * There is no `track` parameter. Nothing on this plate is translucent, so a
///     fade argument would be API surface with no caller, which is the exact rule
///     this module's header sets against.
///
/// Four call sites wore the vendor sheet directly (`window/auto_constrain_settings.rs`,
/// `window/block_definition.rs`, `window/drafting_settings.rs`, `window/options.rs`),
/// all four as the body of a settings/dialog panel, all four with hand-set
/// `.padding([14, 16])`-family literals that stay where they are.
#[must_use]
pub fn dialog_body_style(theme: &Theme) -> container::Style {
    let step = tokens::Control::Weak;
    container::Style {
        background: Some(Background::Color(dialog_body_plate(theme))),
        text_color: Some(tokens::control_ink(theme, step)),
        border: iced::border::rounded(tokens::RADIUS_CONTROL),
        ..container::Style::default()
    }
}

/// The one pixel a dialog body paints, so that anything dropped *on* it can name
/// its ground instead of guessing one.
///
/// Public for the same reason [`tone_band_plate`] is: a caller that has to solve
/// against this plane and re-types `tokens::control_plate(theme, Control::Weak)`
/// is a second definition of the body, and the two drift the first time the rung
/// moves. A verdict band sitting in a dialog solves its hairline against *this*.
#[must_use]
pub fn dialog_body_plate(theme: &Theme) -> Color {
    tokens::control_plate(theme, tokens::Control::Weak)
}

/// The lid over a page while a dialog is open.
///
/// A [`crate::ui::kit::Tier`] is a plane content rests *on*; a scrim is what
/// pushes the page *under*, so no tier paints this and the strength is L0's
/// [`tokens::SCRIM_INK`], not a per-dialog preference. Three sites in
/// `ui/modal.rs` carried the same `0.55 * motion::value("modal:fade")` expression
/// inline; that is the same arithmetic written three times, which is exactly what
/// this module exists to stop.
#[must_use]
pub fn scrim_style(theme: &Theme, track: &'static str) -> container::Style {
    container::Style {
        background: Some(Background::Color(
            tokens::control_plate(theme, tokens::Control::Strongest)
                .scale_alpha(tokens::SCRIM_INK * fade(track)),
        )),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        shadow: iced::Shadow::default(),
        text_color: None,
        snap: true,
    }
}

/// The verdict plate: a tone's own ground, outlined in that tone, inked in a
/// colour solved for that ground.
///
/// [`crate::ui::kit::label::tone_style`] puts a tone on a *word* and
/// [`crate::ui::kit::control::signal_button_style`] puts it behind a *button*;
/// this is the third thing a verdict gets worn on — a band of explanatory text
/// inside a panel (the properties table's error line, a sheet-set status badge, a
/// dialog's validation message). Before it, three dialogs spelled `danger.base`
/// at 0.18 over whatever was behind them and two drew their own edge, while
/// `tone_plate_style` filled itself with `danger.weak` and inked its words with
/// `weak.text`. So there were four verdict bands and no measurement of any of
/// them — which is what `the_verdict_band_feeds_its_own_words` now is.
///
/// The ground is **opaque**, and that is the load-bearing decision, not a
/// shortcut. A band that lets its parent tier through inherits the whole ladder
/// into its own plate: measured 2026-10-16, `success.base` at 0.18 over Dracula's
/// tiers spans `Y` 0.142–0.242, and no single ink clears 4.5:1 at both ends of
/// that (a light ink would need `Y ≥ 1.26`, a dark one `Y ≤ −0.007`), so the
/// translucent version of this plate was *unsolvable by construction* — 3.27:1
/// was its best case, and the best case was the palette's own chip colour
/// untouched. An opaque ground takes the tier out of the equation: one plate per
/// tone per theme, one measurement, and no "where did the band sit?" clause for a
/// screen to get wrong.
///
/// The tone's `base` at [`tokens::TONE_WASH`] is blended *into* that ground rather
/// than over the page, which is how the band keeps the measured identity the three
/// hand-written dialogs agreed on without giving back the span problem.
///
/// The edge is the tone's `base` pulled until it separates from **the plane the
/// band is dropped on**, which is why that plane is an argument. A hairline sits
/// between two pixels and the temptation was to answer for both at once without
/// being told where the band lands; that is not a stricter standard, it is an
/// unsatisfiable one. [`crate::ui::kit::label::resolve_edge`] carries the
/// measurement: solved per ground this law has an answer in every theme, solved
/// against all of the kit's planes with the band's own ground added to the set,
/// **39 of the 312** band/chip cases have no pixel at all that clears
/// [`tokens::AA_COMPONENT_CONTRAST` — an exact supremum of 2.651:1 at Tokyo
/// Night's Danger chip and 2.998:1 at Nord's Accent chip, with the plate at
/// `Y 0.32` on one side of the 3:1 band and a `#404F56`-tier plane on the other
/// (`D:/tmp/s73-probe2.log`)]. Two wrong questions were measured first: solving
/// against the *plate only* left **158 of 2392** readings below the bar
/// (`D:/tmp/s73d-suite.log`) — iced Dark's Success band at 1.10:1 against
/// `#545556` with a 2.99:1 edge — and solving the *conjunction* over all 23
/// grounds left **151 of 2392** (`D:/tmp/s73e-suite.log`), which is the tell that
/// no choice of bar fixes a set whose branches do not intersect.
///
/// Where the fill already separates the band from its ground the hairline keeps
/// the palette's own colour: nothing asked for that hue to move.
/// [`crate::ui::theme_accessibility_tests::the_verdict_band_feeds_its_own_words`]
/// asserts the per-ground disjunction, sweeping the ground the way a screen drops
/// the band.
#[must_use]
pub fn tone_plate_style_on(
    theme: &Theme,
    tone: Tone,
    ground: Color,
    track: &'static str,
) -> container::Style {
    let swatch = crate::ui::kit::label::tone_swatch(theme, tone);
    let plate = tone_band_plate(theme, tone);
    let edge = crate::ui::kit::label::resolve_edge(
        swatch.base.color,
        plate,
        ground,
        tokens::AA_COMPONENT_CONTRAST,
    );
    let paint = super::alpha::Paint::start();
    paint.finish(container::Style {
        background: Some(Background::Color(plate)),
        border: Border {
            color: edge.scale_alpha(fade(track)),
            width: tokens::EDGE_HAIRLINE,
            radius: tokens::RADIUS_FIELD.into(),
        },
        shadow: iced::Shadow::default(),
        text_color: Some(crate::ui::kit::label::tone_ink_on(theme, tone, plate)),
        snap: true,
    })
}

/// The one pixel a verdict band of this tone paints, whatever tier it is dropped
/// into: the tone's `weak` rung carrying [`tokens::TONE_WASH`] of its own `base`.
///
/// Public because the guard measures the plate the ink is solved against; a guard
/// with its own recipe for the ground is a second definition of the band.
pub fn tone_band_plate(theme: &Theme, tone: Tone) -> Color {
    let swatch = crate::ui::kit::label::tone_swatch(theme, tone);
    tokens::over(
        swatch.base.color.scale_alpha(tokens::TONE_WASH),
        swatch.weak.color,
    )
}

// ── Paint on the document canvas ───────────────────────────────────────────

/// Everything the constraint glyphs on the drawing can need, solved once against
/// the canvas pixel they are about to be dropped on.
///
/// The document is not a tier. Its colour comes from the drawing — `Scene::bg_color`
/// and `paper_bg_color`, `scene/mod.rs:2850`, both user-editable — so nothing the
/// theme's ladder decides can have been measured for it, which is the same reason
/// [`crate::ui::kit::label::tone_style_on`] exists for words written on paper.
/// Until this struct the overlay spelled its own answers as eight
/// `Color::from_rgb8` literals, and the 2026-10-17 sweep over the two shipped
/// default canvases and 26 themes measured what that costs:
///
/// | mark | as shipped | on | worst reading |
/// |---|---|---|---|
/// | a dynamic dimension's lock, drawn with **no badge** | `#FFFFFF` | paper `#FFFFFF` | **1.00:1** — invisible, 26 of 26 themes |
/// | the keyhole of that same lock | `#676D76` | model `#212830` | 2.85:1, 26 of 26 |
/// | the selection ring | `primary.strong` | its own badge `#676D76` | **1.01:1**, 26 of 26 |
/// | the ring on a conflict badge | `primary.strong` | `danger.base` | 1.03:1, 26 of 26 |
/// | object-mode lock / vertical datum | `#D64C4C` | `#676D76` | 1.24:1, 26 of 26 |
/// | the point marker | `#E0823E` | `#676D76` | 1.85:1, 26 of 26 |
/// | the Equal label | `#48C774` | `#676D76` | 2.41:1, 26 of 26 |
///
/// Each field is the pixel that gets painted, and the two bars are L0's: a glyph
/// that reads as a word (`=`, `T`, the text labels) is held to
/// [`tokens::AA_INK_CONTRAST`]; a stroke that only has to be *seen* — the ring, a
/// hover cross laid on the drawing — to [`tokens::AA_COMPONENT_CONTRAST`].
///
/// One part, one ground. The first version of this solve asked a mark to clear
/// the ordinary badge *and* the conflict badge at the same time (and the ring to
/// clear three badges plus the drawing), and 23 of 104 `equal`/`datum`/`marker`
/// readings and 63 of 208 ring readings still failed, because the two plates are
/// solved only against the canvas and can land on opposite sides of it: no colour
/// is 4.5:1 against a plate at `Y` 0.02 and a plate at `Y` 0.9 at once. That is
/// the same impossible question the verdict-band guard was rewritten to refuse,
/// and the answer here is the same shape: the painter knows
/// which badge it is about to fill, so it asks for the bundle of that badge and
/// every mark is measured on the pixel it actually lands on.
#[derive(Clone, Copy, Debug)]
pub struct CanvasBadge {
    /// The plate the badge paints.
    pub plate: Color,
    /// The glyph, the text label, the lock body: the badge's own ink.
    pub ink: Color,
    /// The `Equal` label, in the success hue.
    pub equal: Color,
    /// The object-mode lock and the vertical datum, in the danger hue.
    pub datum: Color,
    /// The point marker, in the warning hue.
    pub marker: Color,
    /// The 2px selection ring drawn around this badge.
    pub ring: Color,
}

/// Everything the constraint glyphs on the drawing can need, solved once against
/// the canvas pixel they are about to be dropped on. See [`CanvasBadge`] for why
/// the marks come in one bundle per badge.
#[derive(Clone, Copy, Debug)]
pub struct CanvasGlyphs {
    /// The ordinary relation badge.
    pub ordinary: CanvasBadge,
    /// A coincident relation, which used to get its own blue no theme could reach.
    /// It is drawn compact — plate only, no glyph — so its marks exist but are
    /// never painted.
    pub coincident: CanvasBadge,
    /// A redundant or conflicting relation: the danger swatch, solved to an opaque
    /// plate that keeps its hue and hosts its own ink.
    pub conflict: CanvasBadge,
    /// A glyph drawn with **no** badge — a dynamic dimension's lock sits bare on
    /// the geometry — solved against the canvas itself.
    pub bare: Color,
    /// The keyhole of that bare lock: the badge colour would be a knock-out on a
    /// badge and is a real mark on the drawing, so it is solved like one.
    pub bare_keyhole: Color,
    /// The selection ring around a badge-less glyph, whose other side is the
    /// drawing rather than a plate.
    pub bare_ring: Color,
    /// The hover cross, on the drawing.
    pub hover: Color,
}

/// Solve [`CanvasGlyphs`] for one canvas pixel.
///
/// The hue of every part comes from a [`Tone`] or the scheme's own neutral — no
/// call-site colour survives. Order matters and is the order of the constraint:
/// [`tokens::painted`] first, because a plate that cannot host a legible glyph is
/// not a badge at all; then the plate's separation from the drawing, on a plate
/// that is already ink-capable; then the ink and every mark solved against *that*
/// final pixel, so no measure is taken against a plate that no longer exists.
/// Each mark's ground is the one plate it is painted on, never a set of them —
/// see [`CanvasBadge`] for why the set is an unsatisfiable question.
#[must_use]
pub fn canvas_glyphs(canvas: Color, theme: &Theme) -> CanvasGlyphs {
    use crate::ui::kit::label::{resolve_on, tone_pair};
    let accent = tone_pair(theme, Tone::Accent).color;
    let danger = tone_pair(theme, Tone::Danger).color;
    let success = tone_pair(theme, Tone::Success).color;
    let warning = tone_pair(theme, Tone::Warning).color;
    // Named for what it is now: the strongest control rung, not the palette's
    // `neutral` slot — `tokens::Control::Neutral` is a different step.
    let neutral_plate = tokens::control_plate(theme, tokens::Control::Strong);
    let ring_hue = theme.palette().primary.strong.color;

    let badge = |hue: Color| {
        let (raw_plate, raw_ink) = tokens::painted(hue, 0.0);
        let plate = resolve_on(raw_plate, &[canvas], tokens::AA_COMPONENT_CONTRAST);
        let ink = resolve_on(raw_ink, &[plate], tokens::AA_INK_CONTRAST);
        CanvasBadge {
            plate,
            ink,
            equal: resolve_on(success, &[plate], tokens::AA_INK_CONTRAST),
            datum: resolve_on(danger, &[plate], tokens::AA_INK_CONTRAST),
            marker: resolve_on(warning, &[plate], tokens::AA_INK_CONTRAST),
            ring: resolve_on(ring_hue, &[plate], tokens::AA_COMPONENT_CONTRAST),
        }
    };
    let ordinary = badge(neutral_plate);
    let bare = resolve_on(ordinary.ink, &[canvas], tokens::AA_INK_CONTRAST);
    CanvasGlyphs {
        coincident: badge(accent),
        conflict: badge(danger),
        bare_keyhole: resolve_on(ordinary.plate, &[bare], tokens::AA_INK_CONTRAST),
        bare_ring: resolve_on(ring_hue, &[canvas], tokens::AA_COMPONENT_CONTRAST),
        hover: resolve_on(danger, &[canvas], tokens::AA_COMPONENT_CONTRAST),
        bare,
        ordinary,
    }
}

/// A hairline box around a plane that must stay square: table rows, a column of
/// reported properties, anything whose neighbours have to line up edge to edge.
///
/// [`frame_style`] is the thick viewport ring (2px) and [`ring_style`] is the lit
/// aurora focus edge (1.5px); neither is a table seam, and a seam at either of
/// those weights reads as a frame around each row rather than a line between
/// rows. No fill: a seam draws the boundary and lets the tier underneath supply
/// the plane. `properties.rs` spelled this twice (`:1743`, `:1958`).
#[must_use]
pub fn seam_style(theme: &Theme, track: &'static str) -> container::Style {
    let paint = super::alpha::Paint::start();
    paint.finish(container::Style {
        background: None,
        border: Border {
            color: tokens::edge(theme).scale_alpha(fade(track)),
            width: tokens::EDGE_HAIRLINE,
            radius: 0.0.into(),
        },
        shadow: iced::Shadow::default(),
        text_color: None,
        snap: true,
    })
}

/// The brand mark: a full-width aurora bar along the top edge of a floating
/// panel.
///
/// A dialog is the highest thing on screen, so this is the one surface where the
/// gradient always shows — it is the app signing its own window. [`tick`] is a 2
/// ×10 vertical slat and [`rule_h`] a theme hairline; neither is a horizontal
/// gradient bar, and the strength is L0's [`tokens::AURORA_RULE`] because 0.9 is
/// what makes the violet end survive being 2px tall without washing the title
/// above it.
#[must_use]
pub fn brand_rule<'a, M: 'a>(track: &'static str) -> Container<'a, M> {
    container(Space::new().width(Length::Fill).height(tokens::RULE_BRAND))
        .style(move |theme: &Theme| container::Style {
            background: Some(match draftflow_colors(theme) {
                // The sheet signs a floating panel with its rule, not with a
                // brand hue: the highest surface on screen still gets one line.
                Some(colors) => Background::Color(colors.line_strong),
                None => tokens::aurora_gradient(theme, tokens::AURORA_RULE * fade(track)),
            }),
            ..Default::default()
        })
}

/// Stacked chrome: the tab bar, the status bar, the command line.
///
/// [`Tier::Band`] is the right *fill* for these and the wrong *edge* — a band
/// meets the window on three sides, so the hairline a band needs is the one
/// that separates it from the band above and below, not a box around it. Both
/// are square, both are unlit by elevation, and only this one draws an edge,
/// because a status bar with no seam under it reads as floating over the
/// command line. Two screens reached for this by hand before it was named
/// (`statusbar/mod.rs` and the document tab bar), which is what makes it a
/// primitive rather than a preference.
#[must_use]
pub fn strip_style(theme: &Theme, track: &'static str) -> container::Style {
    let p = fade(track);
    let paint = super::alpha::Paint::start();
    paint.finish(container::Style {
        background: Some(Background::Color(tokens::surface(
            theme,
            tokens::Rank::Band,
        ))),
        border: Border {
            color: tokens::edge(theme).scale_alpha(p),
            width: 1.0,
            radius: 0.0.into(),
        },
        shadow: iced::Shadow::default(),
        text_color: None,
        snap: true,
    })
}

/// [`strip_style`] onto content.
#[must_use]
pub fn strip<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    padding: iced::Padding,
    track: &'static str,
) -> Container<'a, M> {
    container(content)
        .padding(padding)
        .style(move |theme: &Theme| strip_style(theme, track))
}

/// A chrome strip on the sheet: `tier`'s own fill (`Tier::Band` is the panel
/// under the command line, `Tier::Sink` the shell behind the status bar), no
/// frame.
///
/// The sheet separates stacked strips with its own one-pixel line
/// ([`sheet_rule`]), not with a box around each band, so the four-sided
/// [`strip_style`] edge would draw three seams nobody asked for. Every other
/// theme keeps that strip.
#[must_use]
pub fn sheet_strip_style(tier: Tier, theme: &Theme, track: &'static str) -> container::Style {
    match draftflow_colors(theme) {
        Some(_) => container::Style {
            background: tier.fill(theme),
            ..Default::default()
        },
        None => strip_style(theme, track),
    }
}

/// A chrome chip on a band: the lit one is a `panel` plate, the resting one is
/// flat until the pointer washes it. Other themes keep the accent wash and the
/// flush ground they had.
#[must_use]
pub fn sheet_chip_style(
    theme: &Theme,
    active: bool,
    hover: f32,
    track: &'static str,
) -> container::Style {
    match draftflow_colors(theme) {
        Some(colors) => container::Style {
            background: if active {
                Some(Background::Color(colors.panel))
            } else {
                (hover > 0.0)
                    .then(|| Background::Color(colors.hover.scale_alpha(hover.clamp(0.0, 1.0))))
            },
            ..Default::default()
        },
        None => {
            if active {
                tint_style(Tint::Selected, theme, track)
            } else {
                wash_style(Tier::Flush, theme, track)
            }
        }
    }
}

/// A chrome row that sits flat on the sheet and keeps `fallback`'s recess in
/// every other theme.
#[must_use]
pub fn sheet_flat_style(fallback: Tier, theme: &Theme, track: &'static str) -> container::Style {
    if draftflow_colors(theme).is_some() {
        Tier::Flush.style(theme, track)
    } else {
        fallback.style(theme, track)
    }
}

/// The plate under one row of a striped table. `index` says which row; the
/// recess ladder says how loud the stripe is.
///
/// Striping used to be two palette roles — `background.base` on even rows,
/// `background.weak` on odd — hand-copied between the layers panel and the
/// reference manager. As colours they do not track the well the table sits in,
/// and on the schemes where `weak` and `base` land close together the stripe
/// simply vanished. As rungs it cannot: even rows stay in the table's own
/// [`Tier::Sink`] and odd rows lift one notch to [`Tier::Well`], so the step is
/// `RECESS_LADDER`'s own distance (3.0 L* at full scale) and it compresses with
/// the theme rather than collapsing. Law 3 of
/// `kit_contract_tests::surface_tiers_separate_in_every_theme` is the guard.
///
/// Selection is deliberately absent: a striped row that is also selected shows
/// [`Tint::Selected`], the app's only "this one" wash. Compose the two at the
/// call site — never blend them here, or a stripe would start competing with
/// the selection it is supposed to sit underneath.
#[must_use]
pub fn stripe_style(theme: &Theme, index: usize, track: &'static str) -> container::Style {
    let tier = if index % 2 == 0 {
        Tier::Sink
    } else {
        Tier::Well
    };
    wash_style(tier, theme, track)
}

/// The ink a pill's own content must wear, given how far the pointer is into
/// it. One rule, shared by [`pill_style`] (which hands it to iced as
/// `text_color`) and by L3 call sites that paint something iced cannot inherit
/// — an `svg` style closure only receives the theme, so a pill's icon has to
/// ask for this instead of reading a palette slot of its own.
///
/// Why the ink moves at all: a wash is a surface change, and the page's body
/// ink is tuned for the *unlifted* page — the same measurement that put
/// [`tokens::Control::Strong`]'s ink on [`crate::ui::kit::Role::Engaged`]'s aurora
/// (`control.rs`, "A wash lifts the page toward the brand"). A pill that lifts
/// its plate and keeps the page's ink is the defect this replaces: the lit
/// status-bar pill was inked with `primary.weak.text`, the foreground of a
/// `primary.weak` plate the kit stopped painting.
fn pill_ink_at(theme: &Theme, engage: f32, hover: f32) -> Color {
    let engage = engage.clamp(0.0, 1.0);
    let (rest, lit) = if let Some(colors) = draftflow_colors(theme) {
        // Under the sheet a pill carries no plate at all: it is a word on the
        // bar with a rule under it, so its ink is the page's own body ink.
        // `control_ink(Strong)` is the ink for the `strong` background rung,
        // which on the sheet is the same graphite as an ink plate: asking for
        // it painted the icon onto itself and the whole pill read as a solid
        // black rectangle. Resting is the faint rung; the pointer walks it up
        // to body ink.
        (motion::mix(colors.faint, colors.ink, hover), colors.ink)
    } else {
        // The resting plate is the weakest→weak step, so its ink is that step's
        // own text, walked on the same pointer channel as the plate. Taking
        // `base.text` here would be the page ink again.
        (
            motion::mix(
                tokens::control_ink(theme, tokens::Control::Weakest),
                tokens::control_ink(theme, tokens::Control::Weak),
                hover,
            ),
            tokens::control_ink(theme, tokens::Control::Strong),
        )
    };
    motion::mix(rest, lit, engage)
}

/// [`pill_ink_at`] for a caller that has an id but no hover value yet — the
/// pill's icon, which is built before any style closure runs.
#[must_use]
pub fn pill_ink(id: &'static str, active: bool, theme: &Theme, track: &'static str) -> Color {
    let engage = motion::engage(motion::state_key(id), active);
    pill_ink_at(theme, engage, pill_hover(id)).scale_alpha(fade(track))
}

/// The pointer channel of a pill: its own hover track, clamped the way every
/// plate walk clamps it.
fn pill_hover(id: &'static str) -> f32 {
    motion::value(motion::hover_key(id)).clamp(0.0, 1.0)
}

/// The plate under a mode pill: a container that toggles a setting and reports
/// it, split into halves that each carry their own press.
///
/// A pill is not a `Button`, so [`crate::ui::kit::control::Role`] cannot paint
/// it: the status bar's polar-tracking and snap pills are `mouse_area`s because
/// a left click toggles the mode while a right click cycles its increment, and
/// iced buttons fire on release, which a right-click cycle must not wait for.
/// Before this, each one hand-wrote the engaged gradient *and* the resting →
/// hover colour walk.
///
/// `id` is the pill's own motion id, used here to read its hover channel: the
/// `track` argument is the *entrance* fade, which is 1.0 before the first
/// pointer ever crosses the control, so hover cannot ride on it. Passing the id
/// keeps the interpolation out of the screen — L3 says which pill is lit, L1
/// decides what lit looks like and how the pointer walks into it.
#[must_use]
pub fn pill_style(
    id: &'static str,
    active: bool,
    theme: &Theme,
    track: &'static str,
) -> container::Style {
    let engage = motion::engage(motion::state_key(id), active);
    let (background, border, ink) = pill_parts(theme, engage, pill_hover(id), track);
    container::Style {
        background,
        border,
        shadow: iced::Shadow::default(),
        text_color: Some(ink),
        snap: true,
    }
}

/// [`pill_style`]'s rung on a real iced `button`. The status bar's simple
/// toggles are buttons — they sit in the tab order, and sharing the pill ladder
/// must not cost them that. A press is the one thing a `container` style closure
/// cannot know, so here it walks the pointer channel to its top: the same rung
/// [`crate::ui::kit::control::Role::Engaged`] reaches under the cursor.
#[must_use]
pub fn pill_button_style(
    id: &'static str,
    active: bool,
    theme: &Theme,
    status: button::Status,
    track: &'static str,
) -> button::Style {
    let pressed = matches!(status, button::Status::Pressed);
    let hover = if pressed { 1.0 } else { pill_hover(id) };
    let engage = motion::engage(motion::state_key(id), active);
    let (background, border, text_color) = pill_parts(theme, engage, hover, track);
    button::Style {
        background,
        border,
        // `button::Style` carries a plain `Color` where `container::Style`
        // carries an `Option`: the plate cannot be painted without an ink, so
        // the ladder hands one to both shapes rather than letting the button
        // fall back to iced's own palette pair.
        text_color,
        shadow: iced::Shadow::default(),
        ..Default::default()
    }
}

/// The pill ladder in one place: fill, edge and ink for a rung, given how far
/// the pointer is into it. [`pill_style`] and [`pill_button_style`] are adapters
/// onto this and neither may re-decide a number — that is what let the two
/// shapes of status-bar pill disagree about what "lit" means, and what let the
/// resting half be spelled a third way by hand at the call site.
fn pill_parts(
    theme: &Theme,
    engage: f32,
    hover: f32,
    track: &'static str,
) -> (Option<Background>, Border, Color) {
    let p = fade(track);
    let engage = engage.clamp(0.0, 1.0);
    let ink = pill_ink_at(theme, engage, hover).scale_alpha(p);
    // Under the sheet a pill is a word on the bar, not a plate: the lit rung is
    // carried by the ink and by the rule L3 draws over or under it
    // ([`crate::ui::kit::rule_under`]), and resting pills only gain a hover
    // wash. The plate ladder below belongs to the vendor themes. The wash
    // yields as the pill engages, so the plate a pointer put down hands over to
    // the ink instead of stacking two signals on top of each other.
    if let Some(colors) = draftflow_colors(theme) {
        let wash = hover * (1.0 - engage) * p;
        return (
            // The resting wash is handed back at alpha 0 rather than as `None`.
            // Both paint nothing, but the fold ledger measures this rung as a
            // transparent plate: `None` reads as a surface the fold deleted.
            Some(Background::Color(colors.hover.scale_alpha(wash))),
            Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: tokens::RADIUS_FIELD.into(),
            },
            ink,
        );
    }
    // `mix` is exact at t = 0, so the resting state needs no guard branch.
    let tint = tokens::TINT_ENGAGED + (tokens::TINT_ENGAGED_ACTIVE - tokens::TINT_ENGAGED) * hover;
    if engage <= 1e-3 {
        (
            Some(Background::Color(
                motion::mix(
                    tokens::control_plate(theme, tokens::Control::Weakest),
                    tokens::control_plate(theme, tokens::Control::Weak),
                    hover,
                )
                .scale_alpha(p),
            )),
            Border {
                color: motion::mix(
                    tokens::edge(theme),
                    tokens::control_plate(theme, tokens::Control::Strong),
                    hover,
                )
                .scale_alpha(p),
                width: 1.0,
                radius: tokens::RADIUS_FIELD.into(),
            },
            ink,
        )
    } else {
        // The aurora arrives by alpha rather than by blending into the plate:
        // a `Background` is one plate, and the weakest resting rung is within a
        // step of the bar it is painted on, so the handover has no visible seam
        // while the two rungs keep their own exact colours at the ends.
        (
            Some(tokens::aurora_gradient(theme, tint * engage * p)),
            Border {
                color: motion::mix(
                    motion::mix(
                        tokens::edge(theme),
                        tokens::control_plate(theme, tokens::Control::Strong),
                        hover,
                    ),
                    tokens::aurora_edge(theme),
                    engage,
                )
                .scale_alpha(p),
                width: 1.0,
                radius: tokens::RADIUS_FIELD.into(),
            },
            ink,
        )
    }
}

/// [`pill_style`] onto content.
#[must_use]
pub fn pill<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    id: &'static str,
    active: bool,
    padding: iced::Padding,
    track: &'static str,
) -> Container<'a, M> {
    container(content)
        .padding(padding)
        .style(move |theme: &Theme| pill_style(id, active, theme, track))
}

/// The brand mark: a 2×10 slat that opens a title. One implementation
/// shared by panel heads, section heads and dialog titles, so the mark is the
/// same object everywhere the app speaks. It replaced a copy that had grown in
/// `ribbon/draw_panel.rs` beside the title it decorates.
///
/// Under the sheet the slat is ink, not a hue: it reads as the nib touching
/// paper, which is the whole mark. Vendor themes keep the aurora.
pub fn tick<'a, M: 'a>(track: &'static str) -> Container<'a, M> {
    container(Space::new().width(2).height(10))
        .style(move |theme: &Theme| container::Style {
            background: Some(Background::Color(match draftflow_colors(theme) {
                Some(colors) => colors.ink.scale_alpha(fade(track)),
                None => tokens::aurora_edge(theme).scale_alpha(0.85 * fade(track)),
            })),
            border: Border {
                radius: 1.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
}

/// A 1px hairline across the available width.
pub fn rule_h<'a, M: 'a>(track: &'static str) -> Container<'a, M> {
    container(Space::new().height(1.0))
        .width(Fill)
        .style(move |theme: &Theme| container::Style {
            background: Some(Background::Color(
                tokens::edge(theme).scale_alpha(fade(track)),
            )),
            ..Default::default()
        })
}

/// The 1px seam the sheet draws between two chrome strips: its own line
/// colour, and nothing on the vendor themes, which separate bands by plane
/// contrast instead of by a rule.
pub fn sheet_rule<'a, M: 'a>() -> Element<'a, M> {
    container(Space::new().height(1.0))
        .width(Fill)
        .style(|theme: &Theme| container::Style {
            background: draftflow_colors(theme).map(|colors| Background::Color(colors.line)),
            ..Default::default()
        })
        .into()
}

/// The vertical twin of [`sheet_rule`]: the same 1px line in the sheet's own
/// ink, tall enough to separate two ribbon groups.
pub fn sheet_rule_v<'a, M: 'a>() -> Element<'a, M> {
    container(Space::new().width(1.0).height(Fill))
        .style(|theme: &Theme| container::Style {
            background: draftflow_colors(theme).map(|colors| Background::Color(colors.line)),
            ..Default::default()
        })
        .into()
}

/// A 1px hairline tall enough to separate two groups in a row.
pub fn rule_v<'a, M: 'a>(height: f32, track: &'static str) -> Container<'a, M> {
    container(Space::new().width(1.0).height(height))
        .style(move |theme: &Theme| container::Style {
            background: Some(Background::Color(
                tokens::edge(theme).scale_alpha(fade(track)),
            )),
            ..Default::default()
        })
}

// ── The two dividers `form` shipped ───────────────────────────────────────
//
// [`rule_h`] and [`rule_v`] already paint exactly these; they differ only in
// how the extent arrives. A `Container`'s width/height are plain fields whose
// last setter wins, so these take the rule and re-state the extent instead of
// growing a second hairline stylesheet in the same file. The extent is a
// `Length` because that is what the 20 dialog and panel call sites hand in.

/// One-pixel horizontal rule.
pub fn hdivider<'a, M: 'a>(width: Length) -> Element<'a, M> {
    rule_h("").width(width).into()
}

/// One-pixel vertical rule.
pub fn vseparator<'a, M: 'a>(height: Length) -> Element<'a, M> {
    rule_v(1.0, "").height(height).into()
}

/// Alias kept for call sites that historically used `vsep`.
/// Same implementation as [`vseparator`].
pub fn vsep<'a, M: 'a>(height: Length) -> Element<'a, M> {
    vseparator(height)
}

/// A titled group: the aurora slat, the name, a spacer, then anything trailing
/// on the right, closed by a rule. This is the only place a group gets a name,
/// so a screen that needs a heading composes one of these rather than
/// inventing a text style.
pub fn section_head<'a, M: 'a>(
    title: impl Into<Element<'a, M>>,
    trailing: Vec<Element<'a, M>>,
    track: &'static str,
) -> Column<'a, M> {
    let mut children: Vec<Element<'a, M>> =
        vec![tick(track).into(), title.into(), Space::new().width(Fill).into()];
    children.extend(trailing);
    let head: Row<'a, M> = Row::with_children(children)
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center);
    column![head, rule_h(track)]
        .spacing(tokens::GAP_TIGHT)
        .width(Fill)
}

/// The titled fieldset: a caption over a block of controls, inside a hairline
/// edge.
///
/// Five dialogs in the S5 family each wrote this box, and the paint was never
/// the disagreement — every one of them wrote the same
/// `EDGE_HAIRLINE / RADIUS_FIELD / background.strong.color` border with no
/// fill, and the same 11px caption. What drifted was the room inside it:
///
/// | screen | inset | caption→body | caption ink |
/// |---|---|---|---|
/// | `block_definition.rs` (5 boxes) | `[5, 8]` | 4 | `strongest.text × 0.85` |
/// | `wblock.rs` | `[5, 8]` | 4 | `strongest.text × 0.85` |
/// | `drafting_settings.rs` | `[8, 10]` | 6 | `strongest.text × 0.75` |
/// | `annotation_data.rs` | `9` | 7 | the muted forwarder |
/// | `drawing_units.rs` | `8` | 6 | the muted forwarder |
///
/// That is one primitive in five files, and the two ink columns are the same
/// bug twice: an alpha multiplied onto a *palette text* colour, which is solved
/// for nothing — `voice_ink` exists because the quiet voice had to be solved
/// against the planes it lands on. So the caption here is
/// [`super::label::muted_style`] and the pair is [`PAD_WELL`]: the well's "[6, 8]"
/// is the composition this box actually wants (a recess, wider than tall, one
/// notch tighter than the [`PAD_CARD`] body it sits in). Folding the other four
/// files onto this call is their own #42 batch; `block_definition.rs` moves by
/// 1px vertically and 2px of caption gap, and no other screen moves until its
/// turn.
///
/// There is no `track` argument for the same reason [`dialog_body_style`] has
/// none: the edge is opaque and there is no fill, so a fade multiplier would be
/// API surface with no caller. The content inside fades through its own widgets.
///
/// Returns [`Container`] rather than `Element` because the box's *height* is the
/// screen's business, not the kit's: these dialogs size their fields to a fixed
/// plate (`Length::Fixed(152.0)`) and a factory that ate `Element` would force
/// every call site to re-open it to chain `.height()`.
#[must_use]
pub fn group_box<'a, M: 'a>(
    title: impl Into<String>,
    body: impl Into<Element<'a, M>>,
) -> Container<'a, M> {
    container(
        column![
            text(title.into())
                .size(tokens::TEXT_LABEL)
                .style(super::label::muted_style),
            body.into(),
        ]
        .spacing(tokens::GAP_CONTROL),
    )
    .padding(PAD_WELL)
    .width(Fill)
    .style(fieldset_well_style)
}

/// The fieldset well: a hairline border at the field corner radius, drawn in
/// iced's own `background.strong`, with no fill.
///
/// Three screens spelled this byte-for-byte the same way —
/// `ui/window/drawing_units.rs:78`, `ui/window/geometric_tolerance.rs:395` and
/// `ui/window/layer_translator.rs:72` — and `group_box` above carried a private
/// copy of the same four fields. Two prior #42 screens had already drained their
/// fieldset closures onto `group_box` on exactly this evidence ("bit-for-bit what
/// `kit::group_box` paints"), so this is the same fold with the box's paint split
/// out, so that a screen which owns its own inset and width can take the border
/// decision from L1 without importing the caption, the [`PAD_WELL`] inset or the
/// `Fill` width along with it: `layer_translator.rs` sizes its well to
/// `sizing.width`/`sizing.height` and insets by `tokens::GAP_CONTROL`, while
/// `drawing_units.rs` and `geometric_tolerance.rs` are `Fill` wide and inset by
/// `tokens::GAP_ROW`. Nothing here moves a pixel.
///
/// Two relatives deliberately do NOT belong to this fn, and are named so the
/// next reader does not fold them by pattern-match: `ui/node_graph.rs:821` paints
/// the same edge over a `background.base` fill, and
/// `ui/window/xref_attach.rs:233` wears [`tokens::RADIUS_MARK`], not
/// [`tokens::RADIUS_FIELD`]. Each is its own decision.
#[must_use]
pub fn fieldset_well_style(theme: &Theme) -> container::Style {
    container::Style {
        border: Border {
            width: tokens::EDGE_HAIRLINE,
            radius: tokens::RADIUS_FIELD.into(),
            color: theme.palette().background.strong.color,
        },
        ..Default::default()
    }
}
/// The wash a list row wears while it is the row the user has picked.
///
/// Two screens spelled this block by hand and spelled it identically in all
/// five fields (`scale_manager.rs` for the scale list, `style_list.rs` for the
/// linetype / dimstyle / layer lists), so "what a selected row looks like" was
/// a decision living twice in L3. Folding them here moves no paint: same
/// [`tokens::WASH_ROW`] strength, same gradient fn, same ink, same flush border.
///
/// The three L1 neighbours it is NOT, named so the next reader does not merge
/// them by shape: [`wash_style`] scales the *tier fill* by `fade(track)` and
/// carries `snap: true` plus an elevation shadow, none of which a list row
/// wants; `tint_style(Tint::Selected)` is a solid `primary.weak` plate at
/// [`tokens::TINT_SELECTED`] (0.12), one rank hotter than the 0.10 aurora here;
/// and [`guard_panel_style`] is a bordered plate, not a wash at all. The row
/// sits exactly between the hover a pointer leaves behind and the plate a
/// picked swatch wears - which is the band `tokens.rs` pins WASH_ROW inside.
#[must_use]
/// The dot a style-list row opens with: the accent when the row is the one
/// currently in effect (or the font the preview is using), the sheet's own
/// edge otherwise.
///
/// The sample managers put a dot on every row, not only on the current one:
/// the column then reads as a list of styles rather than as a set of labels,
/// and the one live row is found by hue instead of by a check mark that
/// changes the row's leading width. Size is the list's own rung; this is not
/// the 7px status-bar dot, which sits on a band and borrows a different
/// rhythm.
pub fn status_dot_style(theme: &Theme, lit: bool) -> container::Style {
    const DOT: f32 = 8.0;
    container::Style {
        background: Some(Background::Color(if lit {
            theme.palette().primary.base.color
        } else {
            tokens::edge(theme)
        })),
        border: Border {
            radius: (DOT * 0.5).into(),
            ..Border::default()
        },
        ..Default::default()
    }
}

pub fn selected_row_wash_style(theme: &Theme, selected: bool) -> container::Style {
    let palette = theme.palette();
    let colors = draftflow_colors(theme);
    container::Style {
        background: selected.then(|| match colors {
            Some(colors) => Background::Color(colors.selected_row),
            None => tokens::aurora_gradient(theme, tokens::WASH_ROW),
        }),
        text_color: selected.then(|| match colors {
            Some(colors) => colors.ink,
            None => palette.primary.weak.text,
        }),
        border: Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: tokens::RADIUS_FIELD.into(),
        },
        ..Default::default()
    }
}

/// Insets, so a band's padding is not re-typed per screen.
///
/// What the kit decides is the *composition* — which rung wears which axis, and
/// that a band is wider than it is tall while a card is the other way round.
/// The numbers are L0's: every field below names a `tokens::GAP_*` rung instead
/// of repeating it. Counted 2026-10-16 across all sixteen values, they were
/// already on the ladder, so this costs no pixels anywhere.
pub const PAD_BAND: iced::Padding = iced::Padding {
    top: tokens::GAP_TIGHT,
    right: tokens::GAP_FIELD,
    bottom: tokens::GAP_TIGHT,
    left: tokens::GAP_FIELD,
};
pub const PAD_PLATE: iced::Padding = iced::Padding {
    top: tokens::GAP_TIGHT,
    right: tokens::GAP_CONTROL,
    bottom: tokens::GAP_TIGHT,
    left: tokens::GAP_CONTROL,
};
pub const PAD_CARD: iced::Padding = iced::Padding {
    top: tokens::GAP_FIELD,
    right: tokens::GAP_SECTION,
    bottom: tokens::GAP_FIELD,
    left: tokens::GAP_SECTION,
};
pub const PAD_WELL: iced::Padding = iced::Padding {
    top: tokens::GAP_CONTROL,
    right: tokens::GAP_ROW,
    bottom: tokens::GAP_CONTROL,
    left: tokens::GAP_ROW,
};
/// The inset of an inline badge — a two-word pill that sits *inside* a row of a
/// list or a tree, not on a band of its own.
///
/// The vertical axis is `GAP_NONE`, and that is the whole decision. Every other
/// rung here puts blank above the ink, because every other surface owns its own
/// line height. A badge does not: it borrows the row's, so any vertical inset
/// it takes is taken out of the row. `sheet_set.rs`'s status pill measured this
/// while it was wearing `[0, 5]` at `TEXT_MICRO` — 21.6px of row — and the two
/// rungs that were already on the ladder nearby (`Size::Compact` `[4, 8]`,
/// `PAD_FIELD` `[3, 6]`) would have grown that row to 29.6px and 25px
/// respectively, pushing the tree's density down 20-40% to re-decide a number
/// the role had already settled. So the kit composes zero vertical from
/// `GAP_NONE` and horizontal from `GAP_TIGHT`, and the row height is untouched.
///
/// A badge that sits on a band of its own is not this surface: take
/// [`PAD_PLATE`](crate::ui::kit::PAD_PLATE).
pub const PAD_BADGE: iced::Padding = iced::Padding {
    top: tokens::GAP_NONE,
    right: tokens::GAP_TIGHT,
    bottom: tokens::GAP_NONE,
    left: tokens::GAP_TIGHT,
};
/// The inset inside a field — the blank between the ink and the plate it is
/// painted on — for the fields a screen packs into a row or a table cell.
///
/// `stack::labeled_field_compact` wrote this pair inline, and the #42 calibration
/// screen found it wearing the same two numbers at three more sites. It is the
/// largest off-tier pair in the ratchet's (f) column: `D:\tmp\ladder-snap.csv`
/// counts `[3, 6]` at 69 call sites tree-wide, and it is the only one of the top
/// pairs whose two numbers were *already* decided at L0 — `tokens::PAD_FIELD_Y`
/// is 3 (and its doc records why 3 is a named inset and not a rung: 150 sites,
/// one pixel from two neighbours) and `tokens::GAP_CONTROL` is 6. Naming it here
/// therefore moves nothing at those sites; the snap planner, which only knows the
/// `GAP_*` ladder, was proposing to push all 69 up to `[4, 6]` and pay 69 pixels
/// to re-decide a number L0 had already settled. That is finding F1 of #42.
///
/// A *primary* field — the one a dialog asks you to type into, with its label
/// above it — takes [`Size::Control`](crate::ui::kit::control::Size::Control)
/// `[4, 10]` instead: it is a panel control with a text budget to protect, not a
/// dense cell. Which of the two applies is a role question, so it is decided
/// per call site in the screen and never by a tree-wide substitution.
pub const PAD_FIELD: iced::Padding = iced::Padding {
    top: tokens::PAD_FIELD_Y,
    right: tokens::GAP_CONTROL,
    bottom: tokens::PAD_FIELD_Y,
    left: tokens::GAP_CONTROL,
};
/// The inset of a glyph-only plate: an arrow, an eye, a lock, a ▾ that carries
/// no word beside it inside the same button.
///
/// Both numbers come off the ladder, so this is a composition and not a new
/// decision. It exists because the dense strips measured 15 different pairs for
/// the same job (`D:\tmp` count over numeric `.padding([a, b])` whose four
/// preceding lines carry an `icons::` call: 28 sites, of which `[2, 6]` is the
/// plurality at 8, spread over `app/dock/view.rs`, `app/view/mod.rs`,
/// `app/view/viewcube.rs`, `ui/command_line.rs`, `ui/popup/polar_popup.rs`,
/// `ui/window/{alias_editor,layer_translator,shortcuts,named_parameters}.rs`),
/// and neither neighbour fits: `PAD_PLATE` `[4, 6]` makes a 14px glyph plate
/// 24px tall, while `PAD_BADGE` `[0, 4]` removes the plate's own air and lets
/// the glyph touch its edge. A glyph row is therefore hairline-vertical and
/// one-control lateral — the same two rungs `GAP_*` already names.
pub const PAD_GLYPH: iced::Padding = iced::Padding {
    top: tokens::GAP_HAIRLINE,
    right: tokens::GAP_CONTROL,
    bottom: tokens::GAP_HAIRLINE,
    left: tokens::GAP_CONTROL,
};
/// The narrow edge a container leaves around a control that already owns its
/// height: a ribbon panel's card, a popup's footer row, a search box inside a
/// scrolled column.
///
/// The vertical axis reuses [`tokens::PAD_FIELD_Y`] (3) because the question is
/// the same one a field asks — ink to plate, with the row height fixed elsewhere
/// — and the lateral axis takes [`tokens::GAP_TIGHT`] (4), the tightest rung that
/// still reads as separation between two plates. Measured at #42's thirteenth
/// screen, where `[3, 4]` appeared three times in one file for exactly this job
/// and `[4, 4]`, `[3, 5]`, `[1, 4]` were the same role mis-typed three other
/// ways. Unlike [`PAD_BADGE`] this is not a pill inside someone else's line: the
/// container here is the plate, so it keeps air above the ink.
pub const PAD_CELL: iced::Padding = iced::Padding {
    top: tokens::PAD_FIELD_Y,
    right: tokens::GAP_TIGHT,
    bottom: tokens::PAD_FIELD_Y,
    left: tokens::GAP_TIGHT,
};
/// The inset of a ribbon tab's plate, whose lateral number is load-bearing:
/// the tab row may squeeze its gap into negative territory by exactly
/// `PAD_TAB_X - GAP_HAIRLINE`, so the two numbers have to travel together.
///
/// See [`tokens::PAD_TAB_X`] for the arithmetic and for why this is an inset
/// named at L0 rather than a ninth rung on the gap ladder.
pub const PAD_TAB: iced::Padding = iced::Padding {
    top: tokens::PAD_TAB_V,
    right: tokens::PAD_TAB_X,
    bottom: tokens::PAD_TAB_V,
    left: tokens::PAD_TAB_X,
};
/// The inset of one line inside a detached menu: a row the pointer can press,
/// or the small caption that labels the rows under it.
///
/// Named from a census, not composed, and the census is re-counted by a guard
/// test (`the_menu_row_family_keeps_its_measured_ceiling`) so the numbers below
/// are a measurement rather than prose. The family is **16** sites in **11** files and
/// every one of them names this tier. Four rounds of folds got them there: **8** menu lines that
/// fold batch 006 named on 2026-10-18 (`[4, 12]` in `ui/popup/isolate_popup.rs`,
/// `ui/statusbar/mod.rs` twice (the layout context menu's Rename and Delete),
/// `ui/statusbar/statusbar_menu.rs` and `ui/window/xref_manager.rs` three times (its
/// `menu_row`, its `submenu_row` and its flyout row), plus `ui/window/block_palette.rs`'s
/// context-menu entry as `[5, 12]`); **2** `ui/window/count_palette.rs` lines named at
/// the sixteenth screen; the **5** verbs the 24th freeze still found hand-typed
/// (`ui/style/mlstyle.rs`'s Add, `ui/window/xref_attach.rs`'s Browse, `print_all.rs`'s
/// Options and the discard/keep pair in `ui/window/alias_editor.rs`); and **1** that
/// joined from a different pair, `ui/window/open_progress.rs`'s Cancel.
///
/// The **13** hand-typed count is what the guard first measured, and both ceilings in
/// that test are written against it: the family held 15 sites when this tier was cut,
/// 2 of them already named, and grew to 16 when `open_progress.rs`'s Cancel joined it.
///
/// The odd `[5, 12]` vertical is the pixel the ladder sends back to `GAP_TIGHT`;
/// the lateral is the menu's own gutter, shared by its rows and by the caption
/// above them so the two line up. Neither number belongs to a screen.
///
/// Two of those folds moved ink and are disclosed rather than approved, both on #48:
/// `print_all.rs`'s Options came from `[5, 12]` and its line is 2 px shorter, and the
/// Cancel button came from `[4, 14]` and is 2 px narrower. The other four verbs wrote
/// `[4, 12]` byte-for-byte and moved 0 px.
///
/// `kit::Size::Control` is still a different role: `[4, 10]`, the dialog footer's verb,
/// which is where the fifteenth screen sent `ui/window/shortcuts.rs`'s own discard/keep
/// pair. The pin test fails the moment the two laterals converge, because the 2 px of
/// extra gutter is what makes a detached menu read as a menu instead of a form.
pub const PAD_MENU_ROW: iced::Padding = iced::Padding {
    top: tokens::GAP_TIGHT,
    right: tokens::GAP_SECTION,
    bottom: tokens::GAP_TIGHT,
    left: tokens::GAP_SECTION,
};
/// The inset of a **trigger that lives inside a strip**: a combo or dropdown the
/// ribbon itself shows (layer picker, properties picker, style picker), as opposed
/// to a line inside the detached menu that trigger opens.
///
/// Both numbers were already decided at L0 before this name existed — `3` is
/// [`tokens::PAD_FIELD_Y`], the inset that protects a word's own line, and `8` is
/// [`tokens::GAP_ROW`], one rung wider than a control because the surface carries a
/// label plus a caret and the label is what the user is reading. Naming it moves
/// nothing: the three ribbon triggers at `ui/ribbon/widgets.rs:1228/1292/1363` pay
/// 0 px, and the row's own budget (`Fixed(130)` / `Fill` with a clipped name) is
/// computed independently of the inset, so widening it later would silently re-elide
/// a layer name.
///
/// The census found **twelve** sites typing `[3, 8]` in **seven** files
/// (`ui/node_graph.rs`, `ui/popup/{cycle_popup,selection_filter_popup,snap_popup}.rs`,
/// `ui/ribbon/widgets.rs` ×3, `ui/style/textstyle.rs`, `ui/window/block_palette.rs`).
/// They are not all this role: the popup rows read as menu lines (their own tier is
/// [`PAD_MENU_ROW`]) and the two style-manager lists read as list rows. The name is
/// therefore priced at the trigger role only, and every other site is landed on its
/// own leg — a tree-wide substitution would re-decide eight screens' roles from one
/// file's reading, which is what this kit forbids.
pub const PAD_TRIGGER: iced::Padding = iced::Padding {
    top: tokens::PAD_FIELD_Y,
    right: tokens::GAP_ROW,
    bottom: tokens::PAD_FIELD_Y,
    left: tokens::GAP_ROW,
};
/// The inset of the start page's brand plate.
///
/// The composition is the decision here, not the numbers: a hero is a **band**
/// turned upright — it is wider than it is tall, so its air goes lateral — which
/// is the opposite of [`PAD_CARD`]. Both values are L0's, see
/// [`tokens::PAD_HERO_V`] for what each one holds.
pub const PAD_HERO: iced::Padding = iced::Padding {
    top: tokens::PAD_HERO_V,
    right: tokens::PAD_HERO_X,
    bottom: tokens::PAD_HERO_V,
    left: tokens::PAD_HERO_X,
};
/// The inset of the open-progress card, the only modal whose body is a fixed-width
/// bar rather than a stack of fields.
///
/// Kept apart from [`PAD_PLATE`] on purpose: the plate pair `[24, 28]` was measured
/// across six warning windows whose contents are prose, and pulling this card onto
/// it would shrink the bar by 12 px of track to borrow a number that was decided for
/// a different surface. See [`tokens::PAD_PROGRESS_CARD_V`].
pub const PAD_PROGRESS_CARD: iced::Padding = iced::Padding {
    top: tokens::PAD_PROGRESS_CARD_V,
    right: tokens::PAD_PROGRESS_CARD_X,
    bottom: tokens::PAD_PROGRESS_CARD_V,
    left: tokens::PAD_PROGRESS_CARD_X,
};
