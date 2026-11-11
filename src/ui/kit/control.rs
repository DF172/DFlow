//! Controls: the things a user presses, and the single authority for how a
//! pressable thing looks.
//!
//! This module *replaces* `ui::style::form`, which had been the de-facto kit
//! (its `primary_style` / `field_style` / `dialog_button` were called from ~270
//! places while another ~78 call sites hand-rolled the same shapes next to
//! them). Keeping both would be a second coat of paint, so the numbers below
//! are form's — deliberately, because they are the ones locked by
//! `theme_accessibility_tests` — and form is now a forwarding shim on its way
//! out. New roles are only added here when a real screen consumes them.
//!
//! Two orthogonal axes, because conflating them is what made the old API
//! ambiguous:
//!   * [`Role`] — what the control *means* (commit / neutral / currently-on /
//!     dismiss / destructive).
//!   * [`Press`] — whether it can be pressed right now. Disabled is not an
//!     intent, so it is not a role; iced reports it as `Status::Disabled`,
//!     which is why `Press::Off` and the disabled look are the same fact here.

use super::label::{self, Tone};
use super::surface::{fade, plane, Tier};
use crate::ui::style::draftflow_theme::draftflow_colors;
use crate::ui::style::tokens;
use crate::ui::{focus, motion};
use iced::widget::{
    button, checkbox, container, pick_list, radio, row, scrollable, text, text_editor, text_input,
    Button,
};
use iced::{Background, Border, Color, Element, Fill, Theme, Vector};

/// Padding ladder. A control's size is chosen, never typed as numbers.
/// `PartialEq` only: [`Size::Icon`] carries an `f32`, which is not `Eq`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Size {
    /// A whole plate: the body of a modal dialog, inside its own border.
    ///
    /// This is the tier a screen reaches for when it is laying out a *surface*,
    /// not a control. It exists because six warning and confirmation windows
    /// independently wrote the same inset — see [`tokens::PAD_PLATE_V`] for the
    /// measurement and for the four sibling rows still waiting on it in `#42`.
    Plate,
    /// A dialog's footer action: text + breathing room.
    Action,
    /// A toolbar or panel control.
    Control,
    /// A dense row item or status pill.
    Compact,
    /// A square glyph button; `Size` doubles as its edge length.
    Icon(f32),
    /// A control whose plate is the box the caller has already sized: a table
    /// cell, a column header. Zero inset, because the width handed to it *is*
    /// its text budget — see the arm in [`Size::padding`].
    Flush,
}

impl Size {
    /// Public so a screen sizing its own box can take the inset pair from the
    /// ladder (`kit::Size::Compact.padding()`) instead of typing `[4, 8]`.
    pub fn padding(self) -> iced::Padding {
        match self {
            // Neither number is a `GAP_*` rung: a plate inset is not a gap
            // between siblings. Both are decided at L0 from the six measured
            // windows that already use them, so taking this tier moves nothing.
            Size::Plate => iced::Padding {
                top: tokens::PAD_PLATE_V,
                right: tokens::PAD_PLATE_X,
                bottom: tokens::PAD_PLATE_V,
                left: tokens::PAD_PLATE_X,
            },
            // The action button's horizontal inset is one of the two numbers in
            // this ladder that is not a `GAP_*` rung: it is an *inset* (text to
            // plate), and it is named at L0 as `PAD_ACTION_X` rather than
            // rounded to 16 or 20 so no shipped dialog footer moves.
            Size::Action => iced::Padding {
                top: tokens::GAP_CONTROL,
                right: tokens::PAD_ACTION_X,
                bottom: tokens::GAP_CONTROL,
                left: tokens::PAD_ACTION_X,
            },
            Size::Control => iced::Padding {
                top: tokens::GAP_TIGHT,
                right: tokens::GAP_FIELD,
                bottom: tokens::GAP_TIGHT,
                left: tokens::GAP_FIELD,
            },
            Size::Compact => iced::Padding {
                top: tokens::GAP_TIGHT,
                right: tokens::GAP_ROW,
                bottom: tokens::GAP_TIGHT,
                left: tokens::GAP_ROW,
            },
            Size::Icon(_) => iced::Padding::ZERO,
            // Measured, not aesthetic. The layers panel's widest glyph header is
            // "Freeze", ~36px at the 10px row font (~6px per glyph, the same
            // estimate `layers.rs` uses to elide row names), in a 44px column.
            // [`Size::Compact`]'s 8px lateral inset would leave 28px and clip
            // two glyphs, so a control that is given its column must take
            // nothing out of it. The plate still grows on the pointer; only the
            // text box is the caller's.
            Size::Flush => iced::Padding::ZERO,
        }
    }
}

/// Whether the control currently accepts a press.
pub enum Press<M> {
    On(M),
    /// Cannot be pressed *because it is unavailable*: the role's rest paint,
    /// then [`dim`], and iced's own "no pointer" behaviour.
    Off,
    /// Cannot be pressed *because it already says what it is*: the current
    /// segment of a view-mode switch, a row whose toggle is already on.
    ///
    /// This is not [`Press::Off`]. A locked control keeps its rest paint at
    /// full strength and stops answering the pointer, because the plate *is*
    /// the readout — dimming it would say "nothing is selected" at the exact
    /// moment the screen means "this one is". The rung was written for the
    /// XREF manager's List/Tree switch, whose active half had to hand-roll an
    /// engaged gradient that ignored `Status::Disabled`, because iced reports
    /// every press-less button as disabled and [`style_for`] dims on that one
    /// signal.
    Locked,
}

impl<M> Press<M> {
    fn into_option(self) -> Option<M> {
        match self {
            Press::On(m) => Some(m),
            Press::Off | Press::Locked => None,
        }
    }

    fn locked(&self) -> bool {
        matches!(self, Press::Locked)
    }
}

/// What a control commits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// The one action this screen is asking for. The only role that carries
    /// the brand gradient at full strength, and the only one that glows.
    Primary,
    /// A normal, clickable plate: dialog footers, toolbar entries.
    Standard,
    /// The control that is currently the active one — a selected tab, an
    /// engaged snap mode. The same gradient at *wash* strength plus a 1px
    /// aurora edge, so a panel with a dozen engaged states does not read as a
    /// runway.
    Engaged,
    /// Clickable but unframed: an inline reset, a label that happens to work.
    Ghost,
    /// Destructive. Neutral until the pointer arrives, then the danger ramp.
    Danger,
    /// Destructive on a control that must not advertise itself: a `✕` on a tab,
    /// a remove glyph in a row. It rests as [`Role::Ghost`] and arms into the
    /// danger ramp on the pointer, because a framed red plate beside every open
    /// document tab would make the tab strip look like an alarm.
    QuietDanger,
}

/// Which side of a control is glued to a neighbour, so that side loses its
/// corners and the pair reads as one object.
///
/// Geometry, not meaning, so it is deliberately not a [`Role`]: the two halves
/// of a split button are the *same* role wearing different corners, and a tab is
/// an ordinary or engaged plate with its bottom squared off so it can sit on the
/// surface it introduces. Before this existed both shapes were spelled out in
/// the screen as `Radius { top_left: 3.0, .. }`, which put two decisions — the
/// corner value and *which* corners — in the layer forbidden from making either.
///
/// `Copy` is load-bearing, not decoration: the style callback handed to iced is
/// built twice (once for the square-sized arm, once for the flowing one), and a
/// captured `Join` that had to be moved would make the second construction fail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Join {
    /// The leading half: keeps its left corners, squares the right.
    Leading,
    /// The trailing half: squares the left, keeps its right corners.
    Trailing,
    /// A tab: keeps its top corners, squares the bottom so it lands flush on the
    /// panel beneath it.
    Top,
}

/// Apply a [`Join`] on top of the radius a [`Role`] already chose.
fn join_radius(join: Join, base: iced::border::Radius) -> iced::border::Radius {
    use iced::border::Radius;
    match join {
        // The two halves inherit their role's rung, because the point of joining
        // them is to sit beside controls of that same role in a toolbar: a split
        // button whose outer corners were rounder or sharper than the chips next
        // to it reads as a different class of object.
        Join::Leading => Radius {
            top_right: 0.0,
            bottom_right: 0.0,
            ..base
        },
        Join::Trailing => Radius {
            top_left: 0.0,
            bottom_left: 0.0,
            ..base
        },
        // Pinned, and deliberately not to the role. `Role::Engaged` and
        // `Role::Standard` sit on different rungs, so a tab that inherited its
        // corner from its role would change shape the moment it was selected and
        // the strip would flinch under the click. A tab is a tab at both ends of
        // that row, so its round corner is the tab's own and constant.
        Join::Top => Radius {
            top_left: tokens::RADIUS_FIELD,
            top_right: tokens::RADIUS_FIELD,
            bottom_right: 0.0,
            bottom_left: 0.0,
        },
    }
}

/// A pressable control. `content` is text or a `row!` of glyph and text; the
/// role and size decide everything about its appearance.
pub fn btn<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    role: Role,
    size: Size,
    press: Press<M>,
    track: &'static str,
) -> Button<'a, M> {
    paint_button(content, role, size, press, None, track)
}

/// A [`btn`] that shares an edge with another control: the two halves of a split
/// button, or a tab in a strip. The facing side is passed as [`Join`]; the pair
/// then meets at [`tokens::JOIN_OVERLAP`] so their shared border draws once.
pub fn joined_btn<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    role: Role,
    size: Size,
    press: Press<M>,
    join: Join,
    track: &'static str,
) -> Button<'a, M> {
    paint_button(content, role, size, press, Some(join), track)
}

/// The two constructors above, sharing everything about how a plate is built so
/// a joined control cannot drift from a free-standing one.
fn paint_button<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    role: Role,
    size: Size,
    press: Press<M>,
    join: Option<Join>,
    track: &'static str,
) -> Button<'a, M> {
    let locked = press.locked();
    let content = content.into();
    // Every kit control gets the sink and a tab stop from here, so no screen
    // has to ask for them. An empty track means the caller predates the
    // registry; it is left exactly as it was rather than given a shared key.
    let content = if track.is_empty() {
        content
    } else {
        if !locked {
            focus::enroll(track);
        }
        super::press::sink(content, motion::press_key(track)).into()
    };
    let button = button(content)
        .padding(size.padding())
        .on_press_maybe(press.into_option());
    // The entrance alpha is not captured here: `style_for` owns the paint and
    // reads the scope when it solves, which is this same draw.
    let paint = move |theme: &Theme, status| {
        let mut style = if locked {
            locked_style(theme, role, track)
        } else {
            style_for(theme, status, role, track)
        };
        if let Some(j) = join {
            style.border.radius = join_radius(j, style.border.radius);
        }
        style
    };
    match size {
        Size::Icon(side) => button
            .width(side)
            .height(side)
            .style(move |theme: &Theme, status| paint(theme, status)),
        _ => button.style(move |theme: &Theme, status| paint(theme, status)),
    }
}

/// A square control holding only a glyph, centred in both axes.
pub fn icon_btn<'a, M: 'a>(
    glyph: impl Into<Element<'a, M>>,
    role: Role,
    side: f32,
    press: Press<M>,
    track: &'static str,
) -> Button<'a, M> {
    btn(
        container(glyph)
            .width(Fill)
            .height(Fill)
            .center_x(Fill)
            .center_y(Fill),
        role,
        Size::Icon(side),
        press,
        track,
    )
}

/// A status pill in the band along the bottom of the window: it reports a mode
/// and toggles it.
pub fn chip<'a, M: 'a>(
    label: impl Into<Element<'a, M>>,
    engaged: bool,
    press: Press<M>,
    track: &'static str,
) -> Button<'a, M> {
    btn(
        label,
        if engaged { Role::Engaged } else { Role::Standard },
        Size::Compact,
        press,
        track,
    )
}

/// A status pill for the application bar: one dot and a short label on a low
/// plate. `live` lights the dot with the accent — the bar uses it while a
/// command is in flight — and rests on the muted ink otherwise. The label is
/// data (a command name, a count), so it is passed in rather than looked up.
#[must_use]
pub fn state_pill<'a, M: 'a>(
    theme: &Theme,
    label: impl iced::advanced::text::IntoFragment<'a>,
    live: bool,
) -> Element<'a, M> {
    const DOT: f32 = 7.0;
    let ink = match draftflow_colors(theme) {
        Some(colors) if live => colors.accent,
        Some(colors) => colors.muted,
        None if live => theme.palette().primary.base.color,
        None => theme.palette().background.weak.color,
    };
    let dot = container(iced::widget::Space::new().width(DOT).height(DOT)).style(
        move |_theme: &Theme| container::Style {
            background: Some(Background::Color(ink)),
            border: Border {
                radius: (DOT * 0.5).into(),
                ..Border::default()
            },
            ..container::Style::default()
        },
    );
    let body = row![
        dot,
        text(label).size(tokens::TEXT_LABEL).style({
            let ink = ink;
            move |theme: &Theme| match draftflow_colors(theme) {
                Some(colors) => label::color_style(if live { colors.ink } else { ink }),
                None => label::muted_style(theme),
            }
        })
    ]
    .spacing(tokens::GAP_CONTROL)
    .align_y(iced::Center);
    container(body)
        .padding(iced::Padding::from([
            tokens::GAP_SEAM,
            tokens::GAP_CONTROL,
        ]))
        .style(move |theme: &Theme| Tier::Plate.style(theme, ""))
        .into()
}

/// A full-width row in a list: a layer, a block, a recent file. Framed by
/// nothing — a border on a 26px row glues adjacent rows together and kills the
/// zebra striping, so selection is carried by the wash alone.
///
/// A row is an engaged thing, so it walks the same wash ladder as a pill: it
/// rests at [`crate::ui::style::tokens::TINT_SELECTED`] and lifts to
/// [`crate::ui::style::tokens::TINT_ENGAGED`] under the pointer. Before that it
/// carried its own pair of numbers (0.10 / 0.14), which is how a selected layer
/// row and the snap pill next to it ended up two different strengths of "this
/// one". Disabled is the kit's own [`dim`] step: a locked layer still shows
/// which row is selected, it just stops advertising that the row answers a
/// press.
pub fn list_row<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    engaged: bool,
    press: Press<M>,
    track: &'static str,
) -> Button<'a, M> {
    list_row_at(content, if engaged { 1.0 } else { 0.0 }, press, track)
}

/// [`list_row`] with a continuous engagement.
///
/// `engage` is how much of the selection plate this row owns right now. A
/// command-line highlight that slides from one suggestion to the next paints
/// two rows at fractional values instead of restarting a per-row fade, which
/// is the only way to move a highlight without it flickering through empty
/// space between rows.
pub fn list_row_at<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    engage: f32,
    press: Press<M>,
    track: &'static str,
) -> Button<'a, M> {
    button(content)
        .width(Fill)
        .padding(Size::Compact.padding())
        .on_press_maybe(press.into_option())
        .style(move |theme: &Theme, status| {
            let palette = theme.palette();
            let p = fade(track);
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let disabled = matches!(status, button::Status::Disabled);
            let engage = engage.clamp(0.0, 1.0);
            let flush = Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: tokens::RADIUS_FIELD.into(),
            };
            let rest_ink = palette.background.base.text;
            let style = match draftflow_colors(theme) {
                // Selection in a One Pen list is a neutral plate, not a
                // brand wash: the list is chrome, and the single signal
                // colour belongs to the drawing, the caret and the slider.
                Some(colors) => {
                    let plate = if hovered { colors.active } else { colors.selected_row };
                    button::Style {
                        background: (engage > 1e-3)
                            .then(|| Background::Color(plate.scale_alpha(engage * p))),
                        text_color: motion::mix(rest_ink, colors.ink, engage).scale_alpha(p),
                        border: flush,
                        ..Default::default()
                    }
                }
                None => button::Style {
                    background: if engage > 1e-3 {
                        Some(tokens::aurora_gradient(
                            theme,
                            if hovered {
                                tokens::TINT_ENGAGED
                            } else {
                                tokens::TINT_SELECTED
                            } * engage
                                * p,
                        ))
                    } else {
                        hovered.then(|| {
                            Background::Color(
                                rest_ink.scale_alpha(tokens::WASH_POINTER * p),
                            )
                        })
                    },
                    text_color: rest_ink.scale_alpha(p),
                    border: flush,
                    ..Default::default()
                },
            };
            if disabled {
                dim(style)
            } else {
                style
            }
        })
}

/// The kit's single disabled treatment: the control keeps its shape and loses
/// its strength. Half alpha on both fills, matching iced's own disabled buttons,
/// because without it a control that cannot answer a press still advertises its
/// shadow and reads as clickable.
fn dim(mut style: button::Style) -> button::Style {
    style.background = style
        .background
        .map(|bg| bg.scale_alpha(tokens::INK_DISABLED));
    style.text_color = style.text_color.scale_alpha(tokens::INK_DISABLED);
    style.shadow = iced::Shadow::default();
    style
}

/// Decision B, frozen 2026-10-17: the kit's own spelling of the three iced
/// button stylesheets that still had call sites here (`subtle` 30, `text` 25,
/// `danger` 22).
///
/// These are **not** the kit's `Role` ladder and are not pretending to be.
/// Folding `subtle` onto a tier moves its plate by |ΔL*| 7.18–8.81 and its ink
/// and frame on every one of the 26 selectable themes; `danger` moves by
/// 16.77–48.91 (measured by
/// `kit_contract_tests::folding_the_builtin_styles_into_the_kit_is_measured_not_assumed`).
/// That is a redesign, and a redesign is accepted against sample shots, not in a
/// refactor commit. What a refactor *can* take is ownership of the shape: each
/// function below reproduces its vendor original field for field, so the 77 call
/// sites answer to L1 instead of to `vendor/iced`, and when the ladder decision
/// lands there is exactly one place left to change. `the_vendor_equivalents_are_exact`
/// asserts the bit equality for all four statuses across all 26 themes — the
/// claim in this paragraph is a measured one, not a hope.
///
/// The disabled arm is the same fact twice: [`tokens::INK_DISABLED`] is 0.5,
/// which is the half-alpha iced applies internally.
pub fn subtle_style(theme: &Theme, status: button::Status) -> button::Style {
    let paint = super::alpha::Paint::start();
    paint.finish(subtle_style_solve(theme, status))
}

fn subtle_style_solve(theme: &Theme, status: button::Status) -> button::Style {
    use tokens::Control;
    if let Some(colors) = draftflow_colors(theme) {
        // The vendor ladder maps `Weakest` and `Weaker` onto the same rung, so
        // a subtle control would have had no hover response at all. The sheet
        // walks its own plate ladder instead: panel2 at rest, one step down
        // under the pointer, another on the press.
        let (plate, ink) = match status {
            button::Status::Active => (Some(colors.panel2), colors.ink),
            button::Status::Hovered => (Some(colors.hover), colors.ink),
            button::Status::Pressed => (Some(colors.active), colors.ink),
            button::Status::Disabled => (Some(colors.panel2), colors.disabled),
        };
        return button::Style {
            background: plate.map(Background::Color),
            text_color: ink,
            border: iced::border::rounded(tokens::RADIUS_CONTROL),
            ..button::Style::default()
        };
    }
    let base = button::Style {
        background: Some(Background::Color(tokens::control_plate(
            theme,
            Control::Weakest,
        ))),
        text_color: tokens::control_ink(theme, Control::Weakest),
        border: iced::border::rounded(tokens::RADIUS_CONTROL),
        ..button::Style::default()
    };

    match status {
        button::Status::Active => base,
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(tokens::control_plate(
                theme,
                Control::Strong,
            ))),
            ..base
        },
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(tokens::control_plate(
                theme,
                Control::Weaker,
            ))),
            ..base
        },
        button::Status::Disabled => dim(base),
    }
}

/// Decision B: the kit's spelling of `button::text`, the plate-less button used
/// for links. It has no plate at all on either side of the fold, which is why the
/// ledger reports it as the only bucket already zero-pixel on 26/26 themes.
pub fn text_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let base = button::Style {
        text_color: palette.background.base.text,
        ..button::Style::default()
    };

    match status {
        button::Status::Active | button::Status::Pressed => base,
        button::Status::Hovered => button::Style {
            text_color: palette.background.base.text.scale_alpha(0.8),
            ..base
        },
        button::Status::Disabled => dim(base),
    }
}

/// Decision B: the kit's spelling of `button::danger`, kept at the vendor plate
/// because replacing it with `Role::Danger` is the visible half of decision C.
pub fn danger_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let base = button::Style {
        background: Some(Background::Color(palette.danger.base.color)),
        text_color: palette.danger.base.text,
        border: iced::border::rounded(tokens::RADIUS_CONTROL),
        ..button::Style::default()
    };

    match status {
        button::Status::Active | button::Status::Pressed => base,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(palette.danger.strong.color)),
            ..base
        },
        button::Status::Disabled => dim(base),
    }
}

/// A readout on a chrome strip: a word on the bar rather than a capsule.
///
/// The sheet gives the status bar no plate at rest; the pointer brings the
/// hover wash and lifts the ink from `muted` to `ink`. On any theme that is
/// not one of the two DraftFlow schemes it answers with [`subtle_style`], the
/// kit's own resting control plate, because those palettes have no readout
/// role to rest on.
pub fn readout_style(theme: &Theme, status: button::Status) -> button::Style {
    let Some(colors) = draftflow_colors(theme) else {
        return subtle_style(theme, status);
    };
    let hovered = matches!(
        status,
        button::Status::Hovered | button::Status::Pressed
    );
    button::Style {
        background: hovered.then(|| Background::Color(colors.hover)),
        text_color: if hovered { colors.ink } else { colors.muted },
        border: iced::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: tokens::RADIUS_FLAT.into(),
        },
        ..button::Style::default()
    }
}

/// The ribbon's tab plate on the two DraftFlow schemes: the strip is chrome, so
/// the current tab is written in ink and the rule under its label carries the
/// mark; a tab holding a running command takes the selected-row wash so the
/// strip shows where the tool went.
///
/// `None` means the theme is not a DraftFlow scheme and the caller keeps its
/// vendor branch (engaged plate, ring and halo). The split exists because the
/// vendor branch needs motion tracks the kit does not own, while this branch is
/// pure palette and therefore belongs here.
#[must_use]
pub fn tab_plate_style(
    theme: &Theme,
    status: button::Status,
    is_active: bool,
    holds_tool: bool,
) -> Option<button::Style> {
    let colors = draftflow_colors(theme)?;
    let hovered = matches!(
        status,
        button::Status::Hovered | button::Status::Pressed
    );
    let background = if is_active {
        Some(Background::Color(colors.surface))
    } else if hovered {
        Some(Background::Color(colors.hover))
    } else if holds_tool {
        Some(Background::Color(colors.selected_row))
    } else {
        None
    };
    // The current tab is a resting card sitting on the strip, the shape the
    // batch-047 sheets give it; an idle tab is bare ink on the strip.
    Some(button::Style {
        background,
        text_color: if is_active || hovered || holds_tool {
            colors.ink
        } else {
            colors.muted
        },
        border: Border {
            color: if is_active {
                colors.line
            } else {
                Color::TRANSPARENT
            },
            width: if is_active { 1.0 } else { 0.0 },
            radius: tokens::RADIUS_PILL.into(),
        },
        shadow: if is_active {
            tokens::elevation(theme, tokens::LIFT_REST)
        } else {
            iced::Shadow::default()
        },
        ..button::Style::default()
    })
}

/// The ribbon tool button on the two DraftFlow schemes.
///
/// One Pen: the pointer adds a grey wash; a running tool fills with ink and
/// writes in panel, which is the same plate the sheet gives a selected tab.
/// The activation track survives as the plate's alpha so a tool still fades out
/// instead of cutting, and the label lifts from muted to ink under the pointer.
///
/// `None` means the theme is not a DraftFlow scheme; the caller keeps its
/// vendor branch, where the aurora is the engagement signal.
#[must_use]
pub fn tool_plate_style(
    theme: &Theme,
    is_active: bool,
    status: button::Status,
    glow: f32,
    hover_p: f32,
) -> Option<button::Style> {
    let colors = draftflow_colors(theme)?;
    let pressed = matches!(status, button::Status::Pressed);
    let engaged = if is_active { 1.0 } else { glow };
    let on = pressed || engaged > 0.001;
    let a = engaged.max(if pressed { 1.0 } else { 0.0 });
    let background = if on {
        Some(Background::Color(colors.selected_row.scale_alpha(a)))
    } else if hover_p > 0.001 {
        Some(Background::Color(colors.hover.scale_alpha(hover_p)))
    } else {
        None
    };
    // A running tool wears the accent ink on the accent's own row wash (the
    // sheet's selected tile), not a filled black plate: the tile is one of
    // many and an inverted slab reads as a button that has left the row.
    let ink = if on {
        colors.accent
    } else {
        colors.muted.mix(colors.ink, hover_p)
    };
    Some(button::Style {
        background,
        text_color: ink,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: tokens::RADIUS_TILE.into(),
        },
        ..button::Style::default()
    })
}

/// The single button stylesheet. Every role's numbers live in this one
/// function so "what does a hovered danger button look like" has exactly one
/// answer in the codebase.
/// `color` moved `t` of the way toward `other`, alpha kept. The sheet gives a
/// control one signal colour; its hover and pressed rungs are shades of that
/// colour rather than a second hue, so the whole interface reads as one pen.
fn toward(color: Color, other: Color, t: f32) -> Color {
    Color {
        r: color.r + (other.r - color.r) * t,
        g: color.g + (other.g - color.g) * t,
        b: color.b + (other.b - color.b) * t,
        a: color.a,
    }
}

pub fn style_for(
    theme: &Theme,
    status: button::Status,
    role: Role,
    track: &'static str,
) -> button::Style {
    let paint = super::alpha::Paint::start();
    paint.finish(style_for_solve(theme, status, role, track))
}

fn style_for_solve(
    theme: &Theme,
    status: button::Status,
    role: Role,
    track: &'static str,
) -> button::Style {
    let palette = theme.palette();
    let p = fade(track);
    let hovered = matches!(status, button::Status::Hovered);
    let pressed = matches!(status, button::Status::Pressed);
    let disabled = matches!(status, button::Status::Disabled);

    let style = match role {
        Role::Primary => {
            if let Some(colors) = draftflow_colors(theme) {
                // The one action carries the sheet's signal colour, the way the
                // batch-047 sheets draw a footer's verb: an accent plate with
                // the accent's own ink on it. Hover and press walk the plate
                // toward the page's ink, which is the direction a filled plate
                // can deepen without losing its label.
                let plate = if pressed {
                    toward(colors.accent, colors.ink, 0.18)
                } else if hovered {
                    toward(colors.accent, colors.ink, 0.10)
                } else {
                    colors.accent
                };
                button::Style {
                    background: Some(Background::Color(plate.scale_alpha(p))),
                    text_color: colors.on_accent.scale_alpha(p),
                    border: Border {
                        color: plate.scale_alpha(p),
                        width: 1.0,
                        radius: tokens::RADIUS_PILL.into(),
                    },
                    shadow: iced::Shadow {
                        color: colors.accent.scale_alpha(0.26 * p),
                        offset: Vector::new(0.0, 1.0),
                        blur_radius: 8.0,
                    },
                    snap: false,
                }
            } else {
                let dip = if pressed {
                    tokens::HERO_DIP_PRESS
                } else if hovered {
                    tokens::HERO_DIP_HOVER
                } else {
                    tokens::HERO_DIP_REST
                };
                let glow = if disabled || pressed {
                    0.0
                } else if hovered {
                    0.45
                } else {
                    0.28
                };
                // Plate and ink come back as a pair from L0 because which ink is
                // legible is a property of the plate, not of the role: across the
                // 26 selectable palettes the aurora's bright end swings from
                // white-carrying to near-black-carrying.
                let (plate, ink) = tokens::hero(theme, dip);
                button::Style {
                    background: Some(plate),
                    text_color: ink.scale_alpha(p),
                    border: Border {
                        color: Color::TRANSPARENT,
                        width: 1.0,
                        radius: tokens::RADIUS_CONTROL.into(),
                    },
                    shadow: iced::Shadow {
                        color: tokens::aurora_lo(theme).scale_alpha(glow * p),
                        offset: Vector::new(0.0, 1.0),
                        blur_radius: if hovered { 12.0 } else { 8.0 },
                    },
                    snap: false,
                }
            }
        }
        // A 1px border cannot carry depth on its own and iced paints no
        // gradient on a button, so the neutral control walks the background
        // ladder and lifts on the pointer instead.
        Role::Engaged => {
            if let Some(colors) = draftflow_colors(theme) {
                // A segmented control's chosen half, a snap mode that is on.
                // The sheets mark a chosen control with the signal colour at
                // wash strength and a hairline of the same colour, not with a
                // filled plate: a row of engaged chips then reads as one
                // selection, not a runway of buttons. The label walks one step
                // past the signal colour toward the page's ink (away from it
                // on the dark scheme), because readable ink is a property of
                // the wash it sits on: signal-on-signal measured 3.70:1.
                let wash = if pressed {
                    0.18
                } else if hovered {
                    0.15
                } else {
                    0.12
                };
                let dark = theme.palette().is_dark;
                let label = if dark {
                    toward(colors.accent, colors.ink_hover, 0.35)
                } else {
                    toward(colors.accent, colors.ink, 0.35)
                };
                button::Style {
                    background: Some(Background::Color(colors.accent.scale_alpha(wash * p))),
                    text_color: label.scale_alpha(p),
                    border: Border {
                        color: colors.accent.scale_alpha(0.55 * p),
                        width: 1.0,
                        radius: tokens::RADIUS_PILL.into(),
                    },
                    ..Default::default()
                }
            } else {
                button::Style {
                    background: Some(tokens::aurora_gradient(
                        theme,
                        if hovered || pressed {
                            tokens::TINT_ENGAGED_ACTIVE
                        } else {
                            tokens::TINT_ENGAGED
                        } * p,
                    )),
                    // The wash lifts the page toward the brand, and the page's own body
                    // ink is tuned for the *unlifted* page — measured 4.20:1 at the
                    // engaged rung against 7.60:1 from the step the wash actually
                    // corresponds to. A wash is a surface change, so the ink walks
                    // surfaces with it.
                    text_color: tokens::control_ink(theme, tokens::Control::Strong)
                        .scale_alpha(p),
                    border: Border {
                        color: tokens::aurora_edge(theme).scale_alpha(p),
                        width: 1.0,
                        radius: tokens::RADIUS_FIELD.into(),
                    },
                    ..Default::default()
                }
            }
        }
        Role::Standard => {
            if let Some(colors) = draftflow_colors(theme) {
                // One Pen: a resting control is a sheet-white plate inside a
                // hairline, and the pointer walks the grey ladder under it.
                // The palette's `weak` slot is the hover grey, so deriving the
                // plate from it painted every idle control already hovered.
                let plate = if pressed {
                    colors.active
                } else if hovered {
                    colors.hover
                } else {
                    colors.surface
                };
                button::Style {
                    background: Some(Background::Color(plate.scale_alpha(p))),
                    text_color: colors.ink.scale_alpha(p),
                    border: Border {
                        color: colors.line.scale_alpha(p),
                        width: 1.0,
                        radius: tokens::RADIUS_PILL.into(),
                    },
                    ..button::Style::default()
                }
            } else {
                let step = if hovered || pressed {
                    tokens::Control::Strong
                } else {
                    tokens::Control::Weak
                };
                button::Style {
                    background: Some(Background::Color(
                        tokens::control_plate(theme, step).scale_alpha(p),
                    )),
                    text_color: tokens::control_ink(theme, step).scale_alpha(p),
                    border: Border {
                        color: tokens::edge(theme).scale_alpha(p),
                        width: 1.0,
                        radius: tokens::RADIUS_CONTROL.into(),
                    },
                    shadow: tokens::elevation(
                        theme,
                        if pressed { 0.0 } else if hovered { 2.0 } else { 1.0 },
                    ),
                    snap: false,
                }
            }
        }
        // A ghost is quiet because it has no plate and no frame, not because
        // its ink is faded: at `INK_MUTED` the resting label measured 3.09:1 on
        // the worst palette, which is a control you cannot read, not a control
        // that whispers. The pointer adds the wash; the press keeps it, so the
        // control does not blink when it is clicked.
        Role::Ghost => {
            if let Some(colors) = draftflow_colors(theme) {
                // A ghost on the sheet is invisible until reached for, and then
                // it takes the pointer grey rather than a wash of its own ink:
                // an ink wash darkens with the ink, so a dark scheme's hover
                // read as a deeper hole than the light scheme's.
                button::Style {
                    background: (hovered || pressed).then(|| {
                        Background::Color(
                            (if pressed { colors.active } else { colors.hover })
                                .scale_alpha(p),
                        )
                    }),
                    text_color: colors.ink.scale_alpha(p),
                    border: Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: tokens::RADIUS_CONTROL.into(),
                    },
                    ..button::Style::default()
                }
            } else {
                button::Style {
                    background: (hovered || pressed).then(|| {
                        Background::Color(
                            palette.background.base.text.scale_alpha(tokens::WASH_POINTER * p),
                        )
                    }),
                    text_color: palette.background.base.text.scale_alpha(p),
                    border: Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: tokens::RADIUS_CONTROL.into(),
                    },
                    ..Default::default()
                }
            }
        }
        Role::Danger | Role::QuietDanger => {
            if let Some(colors) = draftflow_colors(theme) {
                let quiet = role == Role::QuietDanger;
                let lit = hovered || pressed;
                button::Style {
                    background: if quiet && !lit {
                        None
                    } else if lit {
                        // The alarm stays in the word and the 1px edge. A wash of
                        // `danger` over `surface` lifted the dark plate until
                        // `danger` itself read only 3.63:1, so the lit plate walks
                        // the sheet's named neutral ladder instead: `hover` under
                        // the pointer, `panel` under the press. Both are tokens,
                        // not a mix, and both keep the danger ink above AA
                        // (5.07:1 / 6.01:1 measured on DraftFlow Dark).
                        Some(Background::Color(
                            (if pressed { colors.panel } else { colors.hover }).scale_alpha(p),
                        ))
                    } else {
                        Some(Background::Color(colors.surface.scale_alpha(p)))
                    },
                    text_color: colors.danger.scale_alpha(p),
                    border: Border {
                        color: if quiet && !lit {
                            Color::TRANSPARENT
                        } else if lit {
                            colors.danger.scale_alpha(p)
                        } else {
                            colors.line.scale_alpha(p)
                        },
                        width: if quiet && !lit { 0.0 } else { 1.0 },
                        radius: tokens::RADIUS_CONTROL.into(),
                    },
                    ..button::Style::default()
                }
            } else if hovered || pressed {
                // `danger.weak.color` is a tint, and a tint is drawn to be read
                // *through* — pairing it with the ramp's own danger text
                // measured 1.85:1 worst case. L0 picks the plate's ink instead,
                // so a light scheme gets the pale red with dark ink and a dark
                // scheme gets the deep red with light ink.
                let (plate, ink) = tokens::painted(palette.danger.weak.color, 0.0);
                button::Style {
                    background: Some(Background::Color(plate.scale_alpha(p))),
                    text_color: ink.scale_alpha(p),
                    border: Border {
                        color: palette.danger.base.color.scale_alpha(p),
                        width: 1.0,
                        radius: tokens::RADIUS_CONTROL.into(),
                    },
                    shadow: tokens::elevation(theme, if pressed { 0.0 } else { 1.0 }),
                    snap: false,
                }
            } else if role == Role::QuietDanger {
                button::Style {
                    background: None,
                    text_color: palette.background.base.text.scale_alpha(p),
                    border: Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: tokens::RADIUS_CONTROL.into(),
                    },
                    ..Default::default()
                }
            } else {
                button::Style {
                    background: Some(Background::Color(
                        tokens::control_plate(theme, tokens::Control::Weak).scale_alpha(p),
                    )),
                    text_color: palette.background.base.text.scale_alpha(p),
                    border: Border {
                        color: tokens::edge(theme).scale_alpha(p),
                        width: 1.0,
                        radius: tokens::RADIUS_CONTROL.into(),
                    },
                    shadow: tokens::elevation(theme, 1.0),
                    snap: false,
                }
            }
        }
    };

    let mut style = if disabled { dim(style) } else { style };

    if !track.is_empty() {
        // The style closure is the only place that sees `Status::Pressed`, so
        // the sink is latched here rather than in the widget that draws it.
        motion::press(motion::press_key(track), pressed && !disabled);

        let ring = focus::ring(track);
        if ring > 0.0 && !disabled {
            if let Some(colors) = draftflow_colors(theme) {
                style.border.color = motion::mix(style.border.color, colors.accent, ring);
                let weight = style.border.width.max(tokens::FOCUS_RING);
                style.border.width += (weight - style.border.width) * ring;
            }
        }
    }

    style
}

/// Kept as a plain function pointer so a call site that only needs the style
/// (a custom widget, an iced_aw control) can pass it where iced wants
/// `fn(&Theme, Status) -> Style`.
pub fn role_style(role: Role) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme: &Theme, status| {
        let paint = super::alpha::Paint::start();
        paint.finish(style_for(theme, status, role, ""))
    }
}

/// The paint of a [`Press::Locked`] control: the role's rest rung, at full
/// strength, for every status.
///
/// iced resolves a button with no message to `Status::Disabled`, so a control
/// that is unpressable *because it already reports its own state* would be
/// dimmed by [`dim`] and say "nothing is selected" at the exact moment the
/// screen means "this one". Pinning the status to `Active` is what separates
/// the two kinds of unpressable: `Press::Off` still goes through `style_for`
/// and loses its strength; `Press::Locked` comes here and keeps it.
pub fn locked_style(theme: &Theme, role: Role, track: &'static str) -> button::Style {
    style_for(theme, button::Status::Active, role, track)
}

// ── Fields and the controls that sit beside them ──────────────────────────

/// An editable field: a recessed well whose border turns into the aurora on
/// focus. The recess is what says "type here"; the ring is what says "this one".
///
/// The highlight is solved from the ink, not from the accent: iced paints the
/// selection *under* the glyphs and re-inks nothing, so the band is the only
/// part of this style free to move. [`tokens::backdrop_for`] takes the theme's
/// primary hue and pulls it along the luminance axis until `value` clears AA on
/// top of it, which keeps the selection recognisably the theme's own and makes
/// it lighter or darker than the recess rather than merely louder. The old
/// answer was the accent at a 0.5 wash (`SELECTION_TINT`, since deleted for
/// having no caller), and it was never measured: on 2026-10-16 the guard
/// `ui::theme_accessibility_tests::the_marked_text_survives_its_own_highlight`
/// found 112 of 156 marked-text checks under AA (Tokyo Night 2.45:1, DraftFlow
/// Light and Kanagawa Wave in the 3–4 range, every theme's read-only variant
/// worse). A translucent wash over a translucent plate leaves the plate in the
/// arithmetic, so the guarantee could not hold for both ink and band.
///
/// #69 stopped at the value, and the two quiet inks were the same mistake one
/// rung down: `icon` and `placeholder` were the palette's text times a private
/// alpha, measured by nobody. `the_quiet_ink_of_a_field_still_reads` measured
/// them on 2026-10-16 across the 26 selectable themes and found this field's
/// placeholder at 2.66:1 at worst and its icon at 2.98:1 — a hint that stands in
/// for an empty value is read, so it owes the 4.5:1 text bar, and a trailing
/// glyph is an affordance, so it owes the 3:1 component bar of WCAG 1.4.11. Both
/// are now solved against the recess they paint on rather than faded over it:
/// [`tokens::legible_on`] keeps the rung wherever the faded pixel actually
/// reaches the bar (near-black text at 0.62 over a light recess does), and pulls
/// it opaque only where it does not.
#[must_use]
pub fn field_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let paint = super::alpha::Paint::start();
    paint.finish(field_style_solve(theme, status))
}

fn field_style_solve(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = theme.palette();
    let focused = matches!(status, text_input::Status::Focused { .. });
    let ink = palette.background.base.text;
    let plate = tokens::inset(theme);
    text_input::Style {
        background: Background::Color(plate),
        border: Border {
            color: if focused {
                tokens::focus_ring(theme)
            } else {
                tokens::field_edge(theme)
            },
            width: if focused { tokens::EDGE_RING } else { 1.0 },
            radius: if draftflow_colors(theme).is_some() {
                tokens::RADIUS_TILE
            } else {
                tokens::RADIUS_FIELD
            }
            .into(),
        },
        icon: tokens::legible_on(
            ink.scale_alpha(tokens::INK_MUTED),
            plate,
            tokens::AA_COMPONENT_CONTRAST,
        ),
        placeholder: tokens::legible_on(
            ink.scale_alpha(tokens::INK_PLACEHOLDER),
            plate,
            tokens::AA_INK_CONTRAST,
        ),
        value: ink,
        selection: tokens::backdrop_for(ink, palette.primary.base.color, tokens::AA_INK_CONTRAST),
    }
}

/// The same recessed field plate as [`field_style`], drawn on a **container**.
///
/// Two shapes need it and cannot get it from a `Tier`: a selectable list whose
/// frame must read as one recessed field rather than a stack of rows, and a
/// read-only code plate that is a field nobody can type into. Both are the
/// field's own `inset(theme)` ground and `edge(theme)` border, which is a plate
/// the kit already owns — so the screens used to carry the conversion locally,
/// as a one-method trait plus an `impl` whose `container::Style { … }` literal
/// the ratchet counts as screen-painted style. Three board rows in
/// `field_dialog.rs` were that impl, not a colour decision.
///
/// Nothing here is a re-decision: background and border move over whole from
/// [`field_style`], and `text_input::Style`'s inks and icon have no container
/// counterpart, so they are dropped rather than faked. The result is
/// bit-identical to the conversion it replaces.
#[must_use]
pub fn field_container_style(theme: &Theme, status: text_input::Status) -> container::Style {
    let plate = field_style(theme, status);
    container::Style {
        background: Some(plate.background),
        border: plate.border,
        ..Default::default()
    }
}

/// [`field_style`] with the interactivity removed: the same recess and corner, so
/// a read-only value sits on the plane the editable fields beside it use, but
/// with no focus ring, no icon and the value inked at 0.72 so a column of
/// reported properties does not read as a form waiting for input. Selection
/// stays, so a user can still copy what they cannot edit.
///
/// That 0.72 is paid in colour, not in alpha. A faded ink held as an alpha is
/// washed out by whatever is painted under it, and a selection is exactly that:
/// measured over the 26 selectable themes, the faded value on the accent wash
/// landed at 1.92:1 (Tokyo Night) to 4.44:1 (Fusion White), failing 22 of 26 —
/// the worst of the three field styles. [`tokens::over`] resolves the rung
/// against the recess it is drawn on, which is the same pixel the alpha used to
/// produce and is opaque, so the highlight can then be solved for it.
///
/// The placeholder was the same unfixed fade one rung further down, and it is the
/// quietest ink in the app: `INK_FAINT` 0.48 measured 2.07:1 at worst, failing 25
/// of 26 themes (#74). It is now solved the way the value is, because a hint
/// inside a read-only cell is still a hint somebody has to read.
#[must_use]
pub fn read_only_style(theme: &Theme, _status: text_input::Status) -> text_input::Style {
    let palette = theme.palette();
    let plate = tokens::inset(theme);
    let ink = tokens::over(palette.background.base.text.scale_alpha(0.72), plate);
    text_input::Style {
        background: Background::Color(plate),
        border: Border {
            color: tokens::edge(theme),
            width: 1.0,
            radius: tokens::RADIUS_FIELD.into(),
        },
        icon: Color::TRANSPARENT,
        placeholder: tokens::legible_on(
            palette.background.base.text.scale_alpha(tokens::INK_FAINT),
            plate,
            tokens::AA_INK_CONTRAST,
        ),
        value: ink,
        selection: tokens::backdrop_for(ink, palette.primary.base.color, tokens::AA_INK_CONTRAST),
    }
}

/// A field whose plate is drawn by the container wrapped around it.
///
/// The composite controls — a property row's inline editor sitting inside a
/// bordered cell, a combo whose arrow button shares the well — need the field's
/// ink, caret and selection but none of its chrome. If the input also paints a
/// background and a border, the cell gets a box inside a box, and on focus the
/// two rings stack into one 2 px double line that reads as a rendering glitch.
/// So this is [`field_style`] with the plate removed, and the caller keeps its
/// own edge.
///
/// One measured caller (`ui/properties.rs`, the geometry row's editor) spelled
/// `Color::TRANSPARENT` three times plus `width: 0.0` inline; that is L1's
/// arithmetic at L3, and the reason it exists as a primitive rather than as a
/// one-off.
///
/// Removing the plate also removes the guarantee [`field_style`] makes, which is
/// the part #74 found: a field that paints its own recess can solve its inks
/// against that one pixel, but a bare field's ground is whatever the wrapped
/// container paints — any of the 22 planes in [`label::kit_planes`]. So each ink
/// that comes out of `field_style` solved for the recess is solved *again* here
/// against the plane set, at the same two bars: 4.5:1 for the placeholder and the
/// value, 3:1 for the icon. On 2026-10-16 the red run measured this style's
/// placeholder at 2.04:1 failing 25 of 26 themes and its icon at 2.18:1 failing
/// 16 of 26 (Tokyo Night for both); the value had never been measured here at
/// all, so the guard `the_quiet_ink_of_a_field_still_reads` now covers it too.
#[must_use]
pub fn bare_field_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let paint = super::alpha::Paint::start();
    paint.finish(bare_field_style_solve(theme, status))
}

#[must_use]
fn bare_field_style_solve(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = theme.palette();
    let planes = label::kit_planes(theme);
    let base = field_style(theme, status);
    let value = label::resolve_on(base.value, &planes, tokens::AA_INK_CONTRAST);
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        icon: label::resolve_on(base.icon, &planes, tokens::AA_COMPONENT_CONTRAST),
        placeholder: label::resolve_on(base.placeholder, &planes, tokens::AA_INK_CONTRAST),
        value,
        // The band is solved for the ink that actually lands here, not for the
        // palette's answer: `value` may have moved above.
        selection: tokens::backdrop_for(value, palette.primary.base.color, tokens::AA_INK_CONTRAST),
    }
}

/// A field whose current value is refused: the same recess and focus behaviour
/// as [`field_style`], with the palette's danger verdict carrying the plate.
///
/// It exists because two windows each carried a byte-identical 21-line copy of
/// it (`ui/window/alias_editor.rs:35`, `ui/window/shortcuts.rs:27`; the bodies
/// proved identical with `diff` on 2026-10-16), with a third window already
/// growing the same shape for its error text. A refused value is one fact with
/// many call sites, so it is painted in one place.
///
/// Every ink is chosen by the plate it lands on, the same law
/// [`crate::ui::kit::surface::tone_plate_style_on`] applies to a verdict band. The
/// duplicates inked the value at `danger.base.text` — readable on `danger.base`,
/// which is where the palette put it, and painted on `danger.weak`. Measured over
/// the 26 selectable themes on 2026-10-16 that pairing failed 80 of 104 checks
/// (DraftFlow Light's value at 2.38:1, Moonfly's placeholder at 1.30:1, DraftFlow
/// Dark's edge at 1.46:1), so the guard
/// `ui::theme_accessibility_tests::the_error_field_reads_on_its_own_danger_recess`
/// measures this function's output rather than its intent, and the inks come from
/// [`tokens::legible_on`]: the plate's own text role for the value, `danger.base`
/// pulled along the luminance axis for the edge and the icon.
///
/// Five numbers moved on the way in, all reported rather than quietly kept. The
/// corner was `border::rounded(4)`, 1 px tighter than every other field in the
/// app, so an invalid field sat beside a valid one looking like a different
/// control; it now takes `RADIUS_FIELD` like its neighbours. The placeholder was
/// `0.7` where `INK_PLACEHOLDER` is 0.62, and whether that rung survives the
/// recess is measured, not assumed: iced blends in sRGB, so near-black at 0.62
/// over a near-white danger recess paints 6.19:1 and keeps its rung, while the
/// same rung of mid red over a mid pink recess paints 1.39:1 and is pulled
/// opaque. [`tokens::legible_on`] resolves that per theme; an earlier draft of
/// this note claimed no faded ink could clear AA over a light plate, which was
/// a linear-light arithmetic error and is corrected in L0. The edge was the
/// untouched `danger.base`, a 1.4–2.8:1 hairline on its own recess; it is now
/// pulled to the 3:1 component bar, which means it reads stronger than before.
///
/// The selection is the fourth and it cannot be fixed the way the others can.
/// iced paints selected text on the style's `selection` colour and does not
/// re-ink the glyphs, so the highlight has to carry whatever `value` resolved
/// to — the duplicates' opaque `danger.color` block put the value at 1.6–3.3:1
/// on itself. `selection` is the only part of the field free to move, so it
/// takes the inverse constraint: [`tokens::backdrop_for`] pulls the primary hue
/// along the luminance axis until the value ink clears AA on top of it, which
/// keeps the selection recognisably the theme's own and makes it lighter or
/// darker than the recess rather than merely louder.
#[must_use]
pub fn error_field_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = theme.palette();
    let danger = palette.danger;
    let focused = matches!(status, text_input::Status::Focused { .. });
    let plate = danger.weak.color;
    let ink = tokens::legible_on(danger.weak.text, plate, tokens::AA_INK_CONTRAST);
    let edge = tokens::legible_on(
        danger.base.color,
        plate,
        tokens::AA_COMPONENT_CONTRAST,
    );
    text_input::Style {
        background: Background::Color(plate),
        border: Border {
            color: edge,
            width: if focused {
                tokens::EDGE_RING
            } else {
                tokens::EDGE_HAIRLINE
            },
            radius: tokens::RADIUS_FIELD.into(),
        },
        icon: edge,
        placeholder: tokens::legible_on(
            ink.scale_alpha(tokens::INK_PLACEHOLDER),
            plate,
            tokens::AA_INK_CONTRAST,
        ),
        value: ink,
        selection: tokens::backdrop_for(ink, palette.primary.base.color, tokens::AA_INK_CONTRAST),
    }
}

/// A multi-line field: the same recess, corner and focus ring as [`field_style`],
/// so the command log and the one-line prompt above it read as one control
/// family instead of a panel pasted under a field.
///
/// `text_editor::Style` implements no `Default`, so every call site that wanted
/// one had to spell out all five fields — which is how the command history ended
/// up with its own placeholder alpha and its own selection tint, a step away
/// from the input directly above it, and no border at all where the prompt has
/// one. The disabled step belongs here too: a log can be read-only while a
/// command holds the drawing.
///
/// The disabled step is why this style resolves its ink instead of scaling it.
/// Multiplying one `off` through every field looks like one fact — the whole
/// control is inert — but it stacks two translucencies, and then the selection
/// washes the value through from underneath: measured 2026-10-16 across the 26
/// selectable themes, every one of the 26 disabled-editor checks was under AA
/// (1.91:1 on Kanagawa Lotus to 4.08:1 on Fusion White), and the enabled ones
/// failed with the same 16 themes as [`field_style`]. So the chrome keeps the
/// alpha (a plate that lets its container through is what reads as inert) while
/// the ink is resolved to the pixel it actually paints, and the highlight is
/// then solved for that ink by [`tokens::backdrop_for`]. The resolution uses the
/// theme's own page as what sits under a translucent recess, which is exact for
/// the log and a small neutral shift for a disabled editor placed on a raised
/// panel — visible only while the control cannot be used.
///
/// The same stacking survived in the placeholder, one rung further down: it was
/// `text × (INK_PLACEHOLDER × off)`, so a disabled editor faded its hint twice
/// and painted a pixel nobody had measured. #74 found it failing **all 26**
/// selectable themes, 1.53:1 at worst (Solarized Light), and the enabled hint at
/// 2.66:1 on 10 of 26, which is why both go through the solver like the value
/// does. The bar changes with the state and is not a secret: an enabled hint is
/// text and owes [`tokens::AA_INK_CONTRAST`]; a disabled one keeps WCAG 1.4.3's
/// exemption for inactive controls but is held to the 3:1 floor of 1.4.11, so
/// "this log is read-only" never becomes "this log is blank".
///
/// Solving the disabled hint against `recess` was the first attempt, and the
/// guard caught it: `recess` is that sunk inset composited onto the *page*, but a
/// disabled editor gets dropped on a card, a well or a float too, and a
/// translucent plate folded onto those grounds lands somewhere the solver had
/// never seen — 25 of 26 themes came out under 3:1 on a plane outside its one
/// entry. So the ink is solved against [`label::grounds`]: the recess folded
/// through every plane. The enabled branch goes through the same call and comes
/// out identical, because its plate is opaque and [`tokens::over`] with an opaque
/// foreground is a no-op on each of the 22 grounds.
#[must_use]
pub fn editor_style(theme: &Theme, status: text_editor::Status) -> text_editor::Style {
    let palette = theme.palette();
    let focused = matches!(status, text_editor::Status::Focused { .. });
    let disabled = matches!(status, text_editor::Status::Disabled);
    let off = if disabled {
        tokens::INK_DISABLED
    } else {
        1.0
    };
    let sunk = tokens::inset(theme).scale_alpha(off);
    let recess = tokens::over(sunk, palette.background.base.color);
    let ink = tokens::over(palette.background.base.text.scale_alpha(off), recess);
    text_editor::Style {
        background: Background::Color(sunk),
        border: Border {
            color: if focused {
                tokens::focus_ring(theme)
            } else {
                tokens::field_edge(theme)
            }
            .scale_alpha(off),
            width: if focused { tokens::EDGE_RING } else { 1.0 },
            radius: tokens::RADIUS_FIELD.into(),
        },
        placeholder: label::resolve_on(
            palette.background.base.text.scale_alpha(tokens::INK_PLACEHOLDER * off),
            &label::grounds(theme, sunk),
            if disabled {
                tokens::AA_COMPONENT_CONTRAST
            } else {
                tokens::AA_INK_CONTRAST
            },
        ),
        value: ink,
        selection: tokens::backdrop_for(ink, palette.primary.base.color, tokens::AA_INK_CONTRAST),
    }
}

/// A dropdown with the same inset, radius and focus ring as [`field_style`],
/// so a `label : dropdown` row and a `label : field` row read as one form.
///
/// Every field of `pick_list::Style` is spelled out on purpose: the pinned iced
/// fork does not implement `Default` for it, so a `..Default::default()` here
/// would not compile.
///
/// Three of those fields used to be `background.base.text` times an alpha, and
/// none of them had ever been measured (#74): the hint reached 2.66:1 on 10 of the
/// 26 selectable themes, the handle sat at 2.98:1, and a disabled dropdown's
/// *value* fell to 2.14:1 on 8 of them. The inks are solved against the recess
/// now, at the bar each kind of thing owes: 4.5:1 for the placeholder, which is
/// read, 3:1 for the handle, which is WCAG 1.4.11's graphic, and 3:1 for a
/// disabled value, which keeps 1.4.3's exemption for inactive controls but is
/// still not allowed to vanish. The enabled value stays the palette's own ink,
/// exactly as [`field_style`] leaves its `value` alone — it is measured here, and
/// it clears.
#[must_use]
pub fn pick_list_style(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let palette = theme.palette();
    let ink = palette.background.base.text;
    let plate = tokens::inset(theme);
    let open = matches!(
        status,
        pick_list::Status::Opened { .. } | pick_list::Status::Hovered
    );
    let disabled = matches!(status, pick_list::Status::Disabled);
    pick_list::Style {
        text_color: if disabled {
            tokens::legible_on(
                ink.scale_alpha(tokens::INK_DISABLED),
                plate,
                tokens::AA_COMPONENT_CONTRAST,
            )
        } else {
            ink
        },
        placeholder_color: tokens::legible_on(
            ink.scale_alpha(tokens::INK_PLACEHOLDER),
            plate,
            tokens::AA_INK_CONTRAST,
        ),
        handle_color: tokens::legible_on(
            ink.scale_alpha(tokens::INK_MUTED),
            plate,
            tokens::AA_COMPONENT_CONTRAST,
        ),
        background: Background::Color(plate),
        border: Border {
            color: if open {
                ink
            } else {
                tokens::field_edge(theme)
            },
            width: if open { 1.5 } else { 1.0 },
            radius: if draftflow_colors(theme).is_some() {
                tokens::RADIUS_TILE
            } else {
                tokens::RADIUS_FIELD
            }
            .into(),
        },
    }
}

/// Scrollbars as an overlay rail rather than a chunk of chrome: the track
/// stays nearly invisible until the pointer is on it, which is what keeps a
/// long property list from looking boxed in.
pub fn scrollable_style(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let palette = theme.palette();
    let colors = draftflow_colors(theme);
    let (hovered, dragged) = match status {
        scrollable::Status::Active { .. } => (false, false),
        scrollable::Status::Hovered { .. } => (true, false),
        scrollable::Status::Dragged { .. } => (true, true),
    };
    let alpha = if dragged {
        0.72
    } else if hovered {
        0.55
    } else {
        0.34
    };
    let rail = |scroller_alpha: f32| iced::widget::scrollable::Rail {
        background: Some(Background::Color(
            palette.background.base.text.scale_alpha(tokens::WASH_POINTER),
        )),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: tokens::RADIUS_FIELD.into(),
        },
        scroller: iced::widget::scrollable::Scroller {
            background: Background::Color(palette.background.base.text.scale_alpha(scroller_alpha)),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: tokens::RADIUS_FIELD.into(),
            },
        },
    };
    scrollable::Style {
        // `scrollable::Style` has no `Default` in the pinned fork, so the two
        // fields the rail helper does not produce are spelled out: the
        // container stays transparent (the scrollable paints no plane of its
        // own) and the auto-scroll overlay carries the aurora, because it is
        // the one transient overlay that has to read as the brand.
        container: container::Style::default(),
        vertical_rail: rail(alpha),
        horizontal_rail: rail(alpha),
        gap: Some(Background::Color(palette.background.base.color)),
        auto_scroll: iced::widget::scrollable::AutoScroll {
            background: match colors {
                Some(colors) => Background::Color(colors.panel2),
                None => tokens::aurora_gradient(theme, 0.18),
            },
            border: Border {
                color: match colors {
                    Some(colors) => colors.line_strong,
                    None => tokens::aurora_edge(theme),
                },
                width: 1.0,
                radius: tokens::RADIUS_FIELD.into(),
            },
            shadow: tokens::elevation(theme, 3.0),
            icon: palette.background.base.text,
        },
    }
}

// ── The controls a dialog is made of ──────────────────────────────────────
//
// These builders moved here from `style::form` with their numbers untouched:
// `dialog_button` alone has 85 call sites and `themed_pick_list` 91, so the
// move is mechanical and `form` is a deprecated forwarder until they drain.
// They are generic over the message type `M` instead of naming
// `crate::app::Message`, which is what lets a dock panel, a dialog and a test
// harness reach one primitive.
//
// `themed_*` live here rather than in [`super::stack`] because what they add is
// a stylesheet, not a layout: a caller that already has its own row gets the
// chrome, and a caller that sets `.style(...)` after them still wins.

/// Accent button: the single action a dialog is asking for.
///
/// A `fn` item and not a closure because the dialog footers hand it to iced as
/// a function pointer (`fn(&Theme, Status) -> Style`); [`role_style`] returns
/// `impl Fn` and cannot be passed there.
pub fn primary_style(theme: &Theme, status: button::Status) -> button::Style {
    style_for(theme, status, Role::Primary, "")
}

/// Neutral button — Cancel, Close, Apply, and every list or toolbar control.
///
/// iced's own `button::secondary` fills from `palette.secondary`, a mid-tone
/// *derived from the ink colour* rather than from the chrome: under DraftFlow
/// Light it is a flat (132,147,170) slab with no border and no shadow, so it
/// sits in neither the background ladder nor the raised tier and looks printed
/// on rather than mounted. This walks the background ladder instead, so a
/// neutral control reads the same way in every theme.
pub fn secondary_style(theme: &Theme, status: button::Status) -> button::Style {
    style_for(theme, status, Role::Standard, "")
}

/// Unframed control — a line inside a detached menu, a glyph that carries no word
/// beside it. The `fn`-item twin of `role_style(Role::Ghost)`, named for the same
/// reason `primary_style` and `secondary_style` are: iced's `.style` and the custom
/// widgets want `fn(&Theme, Status) -> Style`, and `role_style` returns `impl Fn`.
///
/// Measured, not composed: the ribbon spelled this body as a local alias
/// (`ui/ribbon/widgets.rs::popup_row_style`) and **eleven** call sites in **four**
/// files handed that alias to `.style` — `ribbon/mod.rs` five, `ribbon/draw_panel.rs`
/// two, `ribbon/color_dropdown.rs` two, `window/block_palette.rs` two, plus the
/// alias itself. The alias was a second owner of an L1 decision, nothing more.
#[must_use]
pub fn ghost_style(theme: &Theme, status: button::Status) -> button::Style {
    style_for(theme, status, Role::Ghost, "")
}

/// A detached menu's own row: the accent while the keyboard is on it, the theme's
/// quiet plate while the pointer is, faded ink when it cannot be picked at all —
/// and no line, at any strength.
///
/// This body was a private `fn` in the start-page overlay (the eighth rebuilt
/// screen) and moved up rather than being re-decided: every field is that
/// function's, byte for byte, including the transparent zero-width border, which
/// is *spelled* rather than left to `button::Style::default()` because the overlay
/// spelled it and the plate must not change on the way up.
///
/// It is not [`ghost_style`]: that role has no keyboard half, because a button
/// style is never handed the highlighted flag — the popup owns it, and this is the
/// plate that consumes it. It is not [`chip_button_style`](chip_button_style)
/// either, whose engaged half is the aurora and whose quiet half carries a border.
/// The `0.42` on the disabled arm stays inside the body as a wash strength the
/// overlay chose, not a rung; `(h)` reads only `aurora_gradient`, so this number is
/// invisible to every column and is priced on the ledger, not here.
#[must_use]
pub fn menu_row_style(
    theme: &Theme,
    status: button::Status,
    highlighted: bool,
) -> button::Style {
    let palette = theme.palette();
    let colors = draftflow_colors(theme);
    let (background, text_color) = if highlighted {
        match colors {
            Some(colors) => (
                Some(Background::Color(colors.selected_row)),
                colors.ink,
            ),
            None => (
                Some(Background::Color(palette.primary.strong.color)),
                palette.primary.strong.text,
            ),
        }
    } else {
        match status {
            button::Status::Hovered | button::Status::Pressed => (
                Some(Background::Color(palette.background.weak.color)),
                palette.background.weak.text,
            ),
            button::Status::Disabled => (None, palette.background.base.text.scale_alpha(0.42)),
            button::Status::Active => (None, palette.background.base.text),
        }
    };
    // A menu is built inside the popup's entrance scope, so the row has to
    // take the scope's alpha: its face, its ink, and the hairline it does not
    // paint all leave together instead of the paper arriving empty.
    let paint = super::alpha::Paint::start();
    paint.finish(button::Style {
        background,
        text_color,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: tokens::RADIUS_FLAT.into(),
        },
        ..Default::default()
    })
}

/// A colour-picker trigger that reads as a field: the page ground, the
/// deviated edge, the mark corner, lifting one rung on hover.
///
/// Cut for `ui/window/gradient_editor.rs`'s colour rows. It is not
/// [`super::surface::picker_trigger_style`], which answers a different
/// question — whether the picker is *engaged* — with the primary edge; this
/// control's only state is the pointer's. It is not [`Role::Standard`], whose
/// plate walks the control ladder and carries elevation; a field stays on the
/// page ground.
#[must_use]
pub fn picker_field_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    button::Style {
        background: Some(Background::Color(
            if matches!(status, button::Status::Hovered) {
                palette.background.weak.color
            } else {
                palette.background.base.color
            },
        )),
        text_color: palette.background.base.text,
        border: Border {
            color: tokens::edge(theme),
            width: tokens::EDGE_HAIRLINE,
            radius: tokens::RADIUS_MARK.into(),
        },
        ..Default::default()
    }
}

/// One glyph cell in a pick grid: quiet on the weak plate, washed on hover,
/// wearing the caller's fill and the aurora edge when selected.
///
/// Cut for `ui/style/point_style.rs`'s point-mode grid. The selected fill
/// stays an argument: the grid chose `aurora_gradient(theme, 0.14)`, and 0.14
/// is a strength no rung owns — naming it would move ink, which is a signature
/// call, not a migration's. The 0.05 hover wash is the same kind of number but
/// invisible to every column (`(h)` reads only `aurora_gradient`), so it rides
/// inside the body and is priced on the batch ledger, exactly like
/// [`menu_row_style`]'s 0.42.
#[must_use]
pub fn glyph_cell_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
    selected_fill: Background,
) -> button::Style {
    let palette = theme.palette();
    let colors = draftflow_colors(theme);
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: if let Some(colors) = colors {
            // A chosen glyph cell is a selected *control*, so it wears the
            // neutral selection plate. The vermilion stays on the drawing.
            Some(Background::Color(if selected {
                colors.selected_row
            } else if hovered {
                colors.hover
            } else {
                colors.panel2
            }))
        } else if selected {
            Some(selected_fill)
        } else if hovered {
            Some(Background::Color(
                palette.background.base.text.scale_alpha(0.05),
            ))
        } else {
            Some(Background::Color(palette.background.weak.color))
        },
        border: Border {
            color: match colors {
                Some(colors) if selected => colors.line_strong,
                Some(_) => tokens::edge(theme),
                None if selected => tokens::aurora_edge(theme),
                None => tokens::edge(theme),
            },
            width: tokens::EDGE_HAIRLINE,
            radius: tokens::RADIUS_FIELD.into(),
        },
        text_color: match colors {
            Some(colors) => colors.ink,
            None if selected => palette.primary.weak.text,
            None => palette.background.base.text,
        },
        ..Default::default()
    }
}

/// Dialog button: accent (primary action) or neutral, from one bool.
pub fn button_style(accent: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    role_style(if accent {
        Role::Primary
    } else {
        Role::Standard
    })
}

/// A tab or list chip: engaged means the aurora, quietly.
///
/// The engaged half is [`Role::Engaged`] — the same gradient as the primary
/// button at wash strength plus the 1px aurora edge. The quiet half is *not*
/// [`Role::Standard`]: a chip sits in a strip of siblings, so it takes the
/// tighter `RADIUS_FIELD` corner and no resting elevation, which is what keeps
/// a dialog with a dozen engagement states from glowing like a runway.
pub fn chip_button_style(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme: &Theme, status| {
        let paint = super::alpha::Paint::start();
        paint.finish(chip_button_style_solve(active, theme, status))
    }
}

fn chip_button_style_solve(
    active: bool,
    theme: &Theme,
    status: button::Status,
) -> button::Style {
    {
        if active {
            style_for(theme, status, Role::Engaged, "")
        } else {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            // Same two rungs as [`Role::Standard`], without that role's fade or
            // elevation: a chip is a sibling in a strip, not a lone control.
            let step = if hovered {
                tokens::Control::Strong
            } else {
                tokens::Control::Weak
            };
            button::Style {
                background: Some(Background::Color(tokens::control_plate(theme, step))),
                text_color: tokens::control_ink(theme, step),
                border: Border {
                    color: tokens::edge(theme),
                    width: 1.0,
                    radius: tokens::RADIUS_FIELD.into(),
                },
                ..Default::default()
            }
        }
    }
}

/// A status chip whose hue **is** the message: off / waiting / busy / ready.
///
/// [`super::surface::Tint::Badge`] is a count pinned to a corner and is always the
/// brand accent, and a [`Role`] is an intent, not a verdict — the automation chip
/// does not *mean* danger, it *reports* it. So the fill walks the semantic
/// swatches through [`crate::ui::kit::label::tone_pair`] and [`tokens::painted`]
/// decides the plate and its ink together. That replaces four raw
/// `Color::from_rgb` values pasted under a hand-picked near-black, which no theme
/// could reach and whose contrast differed between the dark and light schemes.
///
/// It also replaces the version that took the palette's pair at face value —
/// `pair.color` at alpha 0.82, `pair.text` written on it, and the same translucent
/// colour drawn as its own border. Measured 2026-10-17 by calling this very
/// function over 26 themes × 4 tones × 7 tiers: **251 of 728** resting ink checks
/// under AA (worst 3.07:1, Fusion Black's Danger on the Sink tier), **84 of 728**
/// lit checks under AA (worst 4.14:1, Gruvbox Light's Accent on the page), and the
/// border failing the 1.4.11 bar on **226** resting checks — because a border that
/// *is* the plate draws no edge.
///
/// The plate is opaque now, [`tokens::SIGNAL_DIP_REST`] at rest and
/// [`tokens::SIGNAL_DIP_LIT`] under the pointer: the old 0.82 / 1.0 step
/// re-expressed as luminance, so the hover change stays small — the chip has to
/// say which state is true first and that it is clickable second, and it still
/// does not walk a background ladder like [`Role::Standard`] — without lending the
/// tier underneath to the contrast arithmetic. The ink is whatever
/// [`tokens::painted`] solves for that plate across both rungs.
///
/// The border is the tone pulled to clear [`tokens::AA_COMPONENT_CONTRAST`] from
/// **the plane the chip is dropped on**, through
/// [`crate::ui::kit::label::resolve_edge`] — the same ground-as-argument answer
/// the verdict band gives. Asking one hairline to answer for every plane at once
/// was measured, not assumed: 39 of the 312 band/chip placements have no pixel
/// that reaches the bar, with an exact supremum of 2.651:1 (`D:/tmp/s73-probe2.log`),
/// because a mid-tone chip plate and a near-black tier sit on opposite sides of
/// the 3:1 band and the intersection is empty. Two wrong questions were measured
/// before that one: solving against the plate alone shipped **62 of 1456** chip ×
/// tier readings with *neither* the fill nor the edge at 3:1
/// (`D:/tmp/s73d-suite.log`), and solving the conjunction over all 23 planes
/// shipped **397 of 4784** (`D:/tmp/s73e-suite.log`). The weakest fill was
/// Kanagawa Dragon's Accent under the pointer on the Float tier, 1.16:1 against
/// `#3C3B3B` carried only by a 2.84:1 hairline; the weakest edge was Gruvbox
/// Light's Danger on Float, 2.99:1 of plate with a 1.10:1 border — a chip that
/// reports a state nobody can see.
/// `the_signal_chip_carries_its_own_words` sweeps the ground and holds the
/// per-ground disjunction, because the hue-carrying limb is the fill and the limb
/// that must never fail is the edge.
///
/// Disabled is the one status that does not read the tone ladder at all: the
/// plate and the ink both go through [`dim`], so a chip whose action is
/// unavailable keeps its shape and loses its strength. This was missing until
/// 2026-10-21 and the screenshot that caught it was the drafts' own tick:
/// `alias_editor.rs` / `shortcuts.rs` hand `on_press_maybe` nothing while the
/// draft is incomplete, iced reports `Status::Disabled`, and the full-strength
/// plate claimed the row was ready to accept. The guard asserts the alpha on
/// both fills equals [`tokens::INK_DISABLED`] and the border does not move.
///
/// The ground is a [`Tier`], not a resolved [`Color`] as
/// [`crate::ui::kit::surface::tone_plate_style_on`] takes it. Not a preference:
/// iced hands the theme to the *producer* this returns, so a caller that has not
/// been given a theme yet — which is every `.style(...)` site in the app — could
/// not name a pixel at construction time. Naming the plane by its rung is both
/// what the caller actually knows and the shorter way to say it.
pub fn signal_button_style(
    tone: Tone,
    ground: Tier,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme: &Theme, status| {
        let pair = crate::ui::kit::label::tone_pair(theme, tone);
        let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let (plate, ink) = tokens::painted(
            pair.color,
            if lit {
                tokens::SIGNAL_DIP_LIT
            } else {
                tokens::SIGNAL_DIP_REST
            },
        );
        let edge = crate::ui::kit::label::resolve_edge(
            pair.color,
            plate,
            plane(theme, ground),
            tokens::AA_COMPONENT_CONTRAST,
        );
        let style = button::Style {
            background: Some(Background::Color(plate)),
            text_color: ink,
            border: Border {
                color: edge,
                width: tokens::EDGE_HAIRLINE,
                radius: tokens::RADIUS_CONTROL.into(),
            },
            ..Default::default()
        };
        // The two draft rows ship this chip disabled while their input is
        // incomplete (`on_press_maybe(done_ok.then_some(..))`). Without this arm
        // the chip that cannot answer a press is painted at full strength and
        // reads as available; `dim` is the kit's one disabled treatment and
        // scales by the same `INK_DISABLED` the rest of the kit uses.
        if status == button::Status::Disabled {
            dim(style)
        } else {
            style
        }
    }
}

/// Standard dialog action button with an explicit button style and optional
/// on_press action.
///
/// The style is a closure, not a fn pointer: [`signal_button_style`] carries its
/// tone and ground in the captured environment, so a site that has decided both
/// could not pass the result through a bare pointer.
pub fn dialog_button_styled_opt<'a, M: 'a>(
    label: impl text::IntoFragment<'a>,
    on_press: Option<M>,
    style: impl Fn(&Theme, button::Status) -> button::Style + 'a,
) -> Button<'a, M> {
    button(text(label).size(tokens::TEXT_BODY))
        .on_press_maybe(on_press)
        .padding(Size::Action.padding())
        .style(style)
}

/// Standard dialog action button with an explicit button style (primary,
/// secondary, danger, warning, etc.).
pub fn dialog_button_styled<'a, M: 'a>(
    label: impl text::IntoFragment<'a>,
    on_press: M,
    style: impl Fn(&Theme, button::Status) -> button::Style + 'a,
) -> Button<'a, M> {
    dialog_button_styled_opt(label, Some(on_press), style)
}

/// Standard dialog footer action button (OK, Cancel, Apply, Close, Help, etc.),
/// matching the Drafting Settings / Options appearance: 12px text, [6, 18] padding,
/// themed primary or secondary style.
pub fn dialog_button<'a, M: 'a>(
    label: impl text::IntoFragment<'a>,
    on_press: M,
    primary: bool,
) -> Button<'a, M> {
    dialog_button_styled(
        label,
        on_press,
        if primary {
            primary_style
        } else {
            secondary_style
        },
    )
}

/// Unified radio button helper ensuring consistent 16px size for exact
/// pixel-grid concentric alignment.
pub fn form_radio<'a, T, M>(
    label: impl Into<String>,
    value: T,
    selected: Option<T>,
    on_click: impl Fn(T) -> M + 'a,
) -> radio::Radio<'a, M>
where
    M: Clone + 'a,
    T: Copy + Eq + 'a,
{
    radio(label, value, selected, on_click)
        .size(16)
        .spacing(6)
        // The radio's own caption voice, one notch under `TEXT_BODY`: it is the
        // dense-table rung, and reading it from the ladder keeps the 2026-11-02
        // size lift from leaving this one call site behind at the old 11 px.
        .text_size(tokens::TEXT_LABEL)
}

/// A tick box with its caption beside it: the same job as [`form_radio`], for
/// the two-state case.
///
/// It lives here because two dialogs were each building it by hand and the
/// second one imported the first one's private helper across a window module
/// boundary: `window/block_definition.rs` defined it, `window/xref_attach.rs`
/// imported it (`use …::block_definition::{group, labeled_checkbox}`), which is
/// a screen owning a primitive another screen depends on — the exact shape of
/// the L1 gap this module exists to close. Nine call sites, both files.
///
/// The gap is [`form_radio`]'s 6, not a new number: a tick and its caption are
/// one control read as one word, and the radio in the same form already decided
/// how far apart those two are allowed to sit. The box edge is
/// [`tokens::MARK_SQUARE`], which every checkbox in the app already took, so the
/// only pixel this moves is xref_attach's four rows, which had 5.
#[must_use]
pub fn labeled_checkbox<'a, M: 'a>(
    label: impl Into<String>,
    is_checked: bool,
    on_toggle: impl Fn(bool) -> M + 'a,
) -> Element<'a, M> {
    row![
        checkbox(is_checked)
            .on_toggle(on_toggle)
            .size(tokens::MARK_SQUARE),
        text(label.into()).size(tokens::TEXT_LABEL),
    ]
    .spacing(tokens::GAP_CONTROL)
    .align_y(iced::Center)
    .into()
}

/// The same plate as [`labeled_checkbox`] with no callback: a tick that reports
/// a state this dialog is not the place to change.
///
/// The seven rows that asked for it are the Drafting Settings dialog's Pointer
/// Input, Dynamic Prompts, Palette Location and Selection Cycling lists, which
/// are drawn as checked or unchecked captions with nothing behind them. They
/// cannot fold into [`labeled_checkbox`] because its `on_toggle` argument *is*
/// the affordance: hand it a callback that ignores its input and the row lights
/// up as a control the click of which does nothing, which is a lie the widget
/// tells. Without a callback iced resolves the box to `Status::Disabled`, and
/// "disabled" is exactly what these rows mean.
///
/// Every other number is the sibling's, so the two kinds of row are the same
/// height, wear the same caption voice, and sit the same [`tokens::GAP_CONTROL`]
/// apart. Only the callback differs, and that is the whole difference between a
/// setting and a readout.
#[must_use]
pub fn static_checkbox<'a, M: 'a>(label: impl Into<String>, is_checked: bool) -> Element<'a, M> {
    row![
        checkbox(is_checked).size(tokens::MARK_SQUARE),
        text(label.into()).size(tokens::TEXT_LABEL),
    ]
    .spacing(tokens::GAP_CONTROL)
    .align_y(iced::Center)
    .into()
}

/// The tick a dialog draws when the option cannot be changed from here.
///
/// iced's `primary` already dims the plate for `Disabled`, but it leaves the
/// tick in `primary.base.text` and the caption at full ink, so an unpressable
/// row renders brighter than the pressable ones beside it. Only those two inks
/// move; plate and border stay the vendor's disabled arm, which is what keeps
/// every enabled row on exactly the pixels it had. The neutral control plate
/// that would let this primitive own the other three fields is #63's ladder.
#[must_use]
pub fn checkbox_style(theme: &Theme, status: checkbox::Status) -> checkbox::Style {
    let mut style = checkbox::primary(theme, status);
    if matches!(status, checkbox::Status::Disabled { .. }) {
        style.icon_color = style.icon_color.scale_alpha(tokens::INK_DISABLED);
        style.text_color = Some(
            theme
                .palette()
                .background
                .base
                .text
                .scale_alpha(tokens::INK_DISABLED),
        );
    }
    style
}

/// iced's `pick_list` with the shared [`pick_list_style`] already applied.
///
/// Dropdowns are built at ~91 call sites; wrapping the helper once means a
/// call site gets the themed chrome by importing this instead of
/// `iced::widget::pick_list`, and a call site that passes its own `.style(...)`
/// afterwards still wins (the builder just overwrites the class).
pub fn themed_pick_list<'a, T, L, V, M>(
    selected: Option<V>,
    options: L,
    to_string: impl Fn(&T) -> String + 'a,
) -> pick_list::PickList<'a, T, L, V, M>
where
    T: PartialEq + Clone + 'a,
    L: std::borrow::Borrow<[T]> + 'a,
    V: std::borrow::Borrow<T> + 'a,
    M: Clone,
{
    pick_list(selected, options, to_string).style(pick_list_style)
}

/// iced's `scrollable` with the shared [`scrollable_style`] already applied.
///
/// Same reasoning as [`themed_pick_list`]: the scrollbar chrome is one decision
/// for the whole app, and a call site that sets its own `.style(...)` after this
/// still overrides it.
pub fn themed_scrollable<'a, M>(content: impl Into<Element<'a, M>>) -> scrollable::Scrollable<'a, M>
where
    M: Clone,
{
    scrollable(content).style(scrollable_style)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::style::common::wcag_contrast;
    use crate::ui::style::draftflow_theme::{draftflow_dark, draftflow_light};

    fn fill(style: &button::Style) -> Color {
        match style.background {
            Some(Background::Color(color)) => color,
            _ => panic!("a dialog button needs a solid fill"),
        }
    }

    /// The neutral button has to stay on the chrome's background ladder. iced's
    /// `button::secondary` fills from `palette.secondary`, which is derived from
    /// the *ink* colour — under DraftFlow Light that is a flat mid blue-grey, so
    /// a dialog footer reads as printed on rather than mounted, and the two
    /// schemes stop agreeing about what the same control looks like.
    ///
    /// One Pen replaced the vendor's lifted plate with three named rungs of the
    /// sheet: `surface` at rest, `hover` under the pointer, `active` under the
    /// press (DESIGN.md §2.2, §4.3). None of them carries a shadow, so the old
    /// `hover.shadow > rest.shadow` assertion is the stale half of the contract,
    /// not a missing lift in the code.
    #[test]
    fn neutral_buttons_walk_the_background_ladder_in_both_schemes() {
        for theme in [draftflow_dark(), draftflow_light()] {
            let colors = crate::ui::style::draftflow_theme::draftflow_colors(&theme)
                .expect("the two shipped schemes carry an explicit sheet");
            let rest = style_for(&theme, button::Status::Active, Role::Standard, "");
            let hover = style_for(&theme, button::Status::Hovered, Role::Standard, "");
            let press = style_for(&theme, button::Status::Pressed, Role::Standard, "");
            assert_eq!(fill(&rest), colors.surface, "{theme}: idle sits on the sheet");
            assert_eq!(fill(&hover), colors.hover, "{theme}: hover walks one rung");
            assert_eq!(fill(&press), colors.active, "{theme}: the press walks two");
            assert_eq!(rest.border.width, 1.0, "{theme}");
            // `colors.line` is the hairline this sheet draws around controls.
            // DESIGN.md §4.3 calls the secondary edge `line-strong`; that drift
            // is reported to G1 rather than papered over by a test edit.
            assert_eq!(rest.border.color, colors.line, "{theme}");
            for (status, style) in [
                (button::Status::Active, &rest),
                (button::Status::Hovered, &hover),
                (button::Status::Pressed, &press),
            ] {
                assert_eq!(style.shadow.color.a, 0.0, "{theme} / {status:?}: no shadow");
            }

            // Disabled must not keep advertising the resting lift.
            let off = style_for(&theme, button::Status::Disabled, Role::Standard, "");
            assert_eq!(off.shadow.color.a, 0.0, "{theme}");
            assert!(fill(&off).a < fill(&rest).a, "{theme}");
        }
    }

    /// The Primary plate and its ink are one decision made at L0, so this test
    /// pins the *pairing* rather than a colour.
    ///
    /// The batch-047 sheets give the screen's one verb the signal colour:
    /// `accent` at rest, a shade of it under the pointer and the press, the
    /// paired `on_accent` as the label, and a 1px edge of the same plate. The
    /// bar stays at AA against the plate the button actually paints.
    #[test]
    fn the_accent_button_wears_the_accent_plate_and_its_paired_ink() {
        for theme in [draftflow_dark(), draftflow_light()] {
            let colors = crate::ui::style::draftflow_theme::draftflow_colors(&theme)
                .expect("the two shipped schemes carry an explicit sheet");
            for status in [
                button::Status::Active,
                button::Status::Hovered,
                button::Status::Pressed,
            ] {
                let style = style_for(&theme, status, Role::Primary, "");
                let plate = match status {
                    button::Status::Pressed => toward(colors.accent, colors.ink, 0.18),
                    button::Status::Hovered => toward(colors.accent, colors.ink, 0.10),
                    _ => colors.accent,
                };

                assert_eq!(fill(&style), plate, "{theme} / {status:?}: the plate");
                assert_eq!(
                    style.text_color, colors.on_accent,
                    "{theme} / {status:?}: the paired ink"
                );
                assert_eq!(
                    style.border.color, plate,
                    "{theme} / {status:?}: the edge is the plate"
                );
                assert_eq!(style.border.width, 1.0, "{theme} / {status:?}");

                let ratio = wcag_contrast(style.text_color, fill(&style));
                assert!(
                    ratio >= tokens::AA_INK_CONTRAST,
                    "{theme} / {status:?}: {ratio:.2}:1 on the accent plate, bar is {:.1}:1",
                    tokens::AA_INK_CONTRAST
                );
            }
        }
    }

    /// A chip carries the signal colour at wash strength, never at the primary
    /// button's full strength — the numbers form shipped with.
    ///
    /// The batch-047 sheets mark a chosen chip with a wash of the signal
    /// colour and a hairline of the same colour; the label walks one step past
    /// the signal colour so it stays readable on that wash. The quiet half
    /// keeps walking the control ladder.
    #[test]
    fn an_engaged_chip_wears_the_accent_wash() {
        for theme in [draftflow_dark(), draftflow_light()] {
            let colors = crate::ui::style::draftflow_theme::draftflow_colors(&theme)
                .expect("the two shipped schemes carry an explicit sheet");
            let dark = theme.palette().is_dark;
            let label = if dark {
                toward(colors.accent, colors.ink_hover, 0.35)
            } else {
                toward(colors.accent, colors.ink, 0.35)
            };
            let on = chip_button_style(true)(&theme, button::Status::Active);
            assert_eq!(
                fill(&on),
                colors.accent.scale_alpha(0.12),
                "{theme}: an engaged chip is a wash of the signal colour"
            );
            assert_eq!(on.text_color, label, "{theme}");
            assert_eq!(
                on.border,
                Border {
                    color: colors.accent.scale_alpha(0.55),
                    width: 1.0,
                    // The sheet draws a chosen chip as a capsule, the same
                    // corner the row's other quiet controls take.
                    radius: tokens::RADIUS_PILL.into(),
                },
                "{theme}: an engaged chip is edged in the signal colour"
            );
            for (status, wash) in [
                (button::Status::Hovered, 0.15),
                (button::Status::Pressed, 0.18),
            ] {
                let lit = chip_button_style(true)(&theme, status);
                assert_eq!(fill(&lit), colors.accent.scale_alpha(wash), "{theme} / {status:?}");
            }

            // The quiet chip is a framed plate with no lift of its own: the
            // palette edge at the field radius, and `Standard`'s elevation must
            // not leak in (a strip of lifted chips reads as a runway).
            let off = chip_button_style(false)(&theme, button::Status::Active);
            assert_eq!(
                fill(&off),
                tokens::control_plate(&theme, tokens::Control::Weak),
                "{theme}"
            );
            // Both rungs of the rule, so the pairing cannot drift on one side:
            // a chip wears the text of the surface it sits on — the weak rung's
            // ink on the weak rung's plate at rest, the strong rung's pair once
            // it is hovered. The hovered quiet chip was previously untested.
            assert_eq!(off.text_color, tokens::control_ink(&theme, tokens::Control::Weak), "{theme}");
            let off_hover = chip_button_style(false)(&theme, button::Status::Hovered);
            assert_eq!(
                fill(&off_hover),
                tokens::control_plate(&theme, tokens::Control::Strong),
                "{theme}"
            );
            assert_eq!(
                off_hover.text_color,
                tokens::control_ink(&theme, tokens::Control::Strong),
                "{theme}"
            );
            assert_eq!(
                off.border,
                Border {
                    color: tokens::edge(&theme),
                    width: 1.0,
                    radius: tokens::RADIUS_FIELD.into(),
                },
                "{theme}"
            );
            assert_eq!(off.shadow.color.a, 0.0, "{theme}");
        }
    }
}

/// The product's own mark: a lettered tile in the accent, with the sheet's
/// name and build beside it. `None` on a vendor palette — the tile is the
/// brand's signal plate, and painting one on another product's theme would put
/// two identities on one window.
///
/// It lives here rather than in the ribbon because the ribbon may not paint:
/// the tile is a `container::Style` literal and the two texts are `text::Style`
/// literals, which is exactly the pair the kit contract forbids a screen from
/// spelling. The geometry is the header row's, so it is kept beside the
/// control ladder rather than in `tokens`: 32 is the tile's own square, and the
/// three voices are the ones the sheet already uses for "which product", "which
/// build".
#[must_use]
pub fn brand_mark<'a, M: 'a>(
    theme: &Theme,
    letters: &'static str,
    name: impl text::IntoFragment<'a>,
    build: impl text::IntoFragment<'a>,
) -> Option<Element<'a, M>> {
    const TILE: f32 = 32.0;
    let colors = draftflow_colors(theme)?;
    let tile = container(
        text(letters)
            .size(tokens::TEXT_LABEL)
            .font(tokens::UI_FONT_SEMIBOLD)
            .style(move |_: &Theme| text::Style {
                color: Some(colors.on_accent),
            }),
    )
    .width(iced::Length::Fixed(TILE))
    .height(iced::Length::Fixed(TILE))
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .style(move |_: &Theme| container::Style {
        background: Some(Background::Color(colors.accent)),
        border: Border {
            radius: tokens::RADIUS_TILE.into(),
            ..Border::default()
        },
        ..container::Style::default()
    });
    let name = text(name)
        .size(tokens::TEXT_META)
        .font(tokens::UI_FONT_SEMIBOLD)
        .style(move |_: &Theme| text::Style {
            color: Some(colors.ink),
        });
    let build = text(build)
        .size(tokens::TEXT_MICRO)
        .style(move |_: &Theme| text::Style {
            color: Some(colors.muted),
        });
    let stack = iced::widget::Column::with_children(vec![name.into(), build.into()])
        .spacing(tokens::GAP_HAIRLINE);
    Some(
        row![tile, stack]
            .spacing(tokens::GAP_FIELD)
            .align_y(iced::Center)
            .padding(iced::Padding {
                left: tokens::GAP_SECTION,
                right: tokens::GAP_SECTION,
                ..iced::Padding::ZERO
            })
            .into(),
    )
}
