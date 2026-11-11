//! The in-canvas dialog frame: scrim, title bar, move and resize grips.
//!
//! Former pop-up *windows* (layer manager, style editors, About, …) render as
//! centered overlays on top of the main view instead of separate OS windows.
//! The native build has one main window and the web build has only the canvas,
//! so both stack dialogs here — one code path for every platform.
//!
//! This is shell, not kit: it decides *no* colour. Every plate, scrim and rule
//! it wears comes from `kit` (`scrim_style`, `brand_rule`, `glass_pane`), and
//! every gap from `tokens`. What lives here is the frame machinery a screen
//! would otherwise re-implement once per dialog.

use crate::app::Message;
use crate::ui::kit;
use crate::ui::style::tokens;
use iced::widget::{column, container, mouse_area, opaque, row, sensor, stack, Space};
use iced::{Element, Length, Padding, Theme, Vector};

#[derive(Debug, Clone, Copy)]
pub struct ModalOptions {
    movable: bool,
    resizable: bool,
    neutral_close: bool,
    /// No title bar and no ✕: the body draws its own header, as the sample
    /// sheets' attribute editor and small confirmations do.
    bare: bool,
}

/// The sample sheets' dialog footer: a full-bleed information band under a
/// hairline, then an action row with a leading group (Help) and a trailing
/// group (Cancel / OK / Close).
///
/// The footer lives in the shell rather than in each body because the band
/// runs edge to edge across the sheet — a body's own padding would inset it,
/// and every dialog would have to re-derive the same seam and insets. A
/// dialog with nothing to say simply passes `None`.
pub struct ModalFooter<'a> {
    /// Fields along the band, left to right. The band carries the sheet's band
    /// fill; the fields are the caller's, because only the caller knows which
    /// numbers they are showing.
    pub meta: Option<Element<'a, Message>>,
    /// Actions at the leading edge. `None` leaves the corner empty.
    pub leading: Option<Element<'a, Message>>,
    /// Actions at the trailing edge. `None` leaves the corner empty.
    pub trailing: Option<Element<'a, Message>>,
}

impl ModalOptions {
    pub const STANDARD: Self = Self {
        movable: true,
        resizable: true,
        neutral_close: false,
        bare: false,
    };

    pub const NOTICE: Self = Self {
        movable: true,
        resizable: false,
        neutral_close: true,
        bare: false,
    };

    pub const BARE: Self = Self {
        movable: true,
        resizable: true,
        neutral_close: true,
        bare: true,
    };
}

/// Dim and block the application beneath an overlay-owned dialog.
///
/// Some third-party overlays ignore pointer movement outside their interactive
/// controls. The shield owns the cursor and pointer presses across the window,
/// making such an overlay behave modally.
pub fn backdrop<'a>(
    base: impl Into<Element<'a, Message>>,
    on_close: Message,
) -> Element<'a, Message> {
    // The scrim: the scheme's own darkest plane, eased in with the modal
    // fade. No `Tier` paints this — a tier is a plane content rests *on*,
    // and the scrim is what pushes the page *under*.
    let shield = mouse_area(
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|theme: &Theme| kit::scrim_style(theme, "modal:fade")),
    )
    .on_press(on_close)
    .interaction(iced::mouse::Interaction::Idle);

    stack![base.into(), opaque(shield)].into()
}

/// A dialog's "unsaved changes" guard: `main` dimmed behind a small panel
/// that offers to discard the changes and close, or to keep editing. Clicking
/// the dim area keeps editing. Callers show it in place of the plain dialog
/// once the user asked to close with something to lose.
pub fn discard_guard<'a>(
    main: impl Into<Element<'a, Message>>,
    on_discard: Message,
    on_keep: Message,
) -> Element<'a, Message> {
    let shield = mouse_area(
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|theme: &Theme| kit::scrim_style(theme, "modal:fade")),
    )
    .on_press(on_keep.clone());
    let panel = kit::surface(
        column![
            iced::widget::text(crate::t!("Unsaved changes will be discarded."))
                .size(tokens::TEXT_HEADING),
            row![
                kit::btn(
                    iced::widget::text(crate::t!("Discard & close")).size(tokens::TEXT_BODY),
                    kit::Role::Danger,
                    kit::Size::Action,
                    kit::Press::On(on_discard),
                    "",
                ),
                kit::dialog_button(crate::t!("Keep editing"), on_keep, false),
            ]
            .spacing(tokens::GAP_FIELD),
        ]
        .spacing(tokens::GAP_SECTION),
        kit::Tier::Card,
        kit::PAD_CARD,
        "",
    );
    stack![
        main.into(),
        shield,
        container(panel)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
    ]
    .into()
}

/// Stack `content` over `base` behind a dimmed backdrop, framed with a title bar
/// (the ✕ close button at its right end). The backdrop only dims and blocks
/// clicks from reaching the view beneath — it does **not** dismiss the dialog;
/// closing is the ✕ button alone (`on_close`).
///
/// `offset` shifts the dialog from screen-centre so it can be dragged by its
/// title bar; pass `Vector::ZERO` to keep it centred.
///
/// `theme` resolves the pane's colours. A glass plate samples the frame under
/// it, and the shader is handed no theme of its own, so the caller's active
/// palette is the only place those four colours can come from.
pub fn modal<'a>(
    base: impl Into<Element<'a, Message>>,
    title: impl iced::widget::text::IntoFragment<'a>,
    content: impl Into<Element<'a, Message>>,
    on_close: Message,
    offset: Vector,
    options: ModalOptions,
    footer: Option<ModalFooter<'a>>,
    theme: &Theme,
) -> Element<'a, Message> {
    // The ✕ is the kit's quiet destructive control: it rests unframed so a
    // title bar never opens on a red plate, and arms into the danger ramp under
    // the pointer. A neutral notice dismisses rather than destroys, so its
    // close takes `Role::Ghost` — the same quiet shape without the danger hue.
    // The glyph keeps the ink it had: the icon carries its own colour, so the
    // role only paints the plate behind it.
    let (glyph, role) = if options.neutral_close {
        (
            crate::ui::icons::themed_secondary(crate::ui::icons::CLOSE, 13.0),
            kit::Role::Ghost,
        )
    } else {
        (
            crate::ui::icons::themed_secondary(crate::ui::icons::CLOSE, 13.0),
            kit::Role::QuietDanger,
        )
    };
    let close = kit::btn(glyph, role, kit::Size::Compact, kit::Press::On(on_close), "");

    // The dialog name is centred across the content width with the ✕ overlaid
    // at the right edge. The title bar is itself an overlay: the content layer
    // below determines the intrinsic modal width before `Fill` is resolved.
    // The dialog name opens at the leading edge, the way the batch-047 sheets
    // set it: the title names the panel and the eye starts where the reading
    // starts, with the ✕ alone at the right end of the bar.
    // The sheets set a dialog's name on a header row of its own, one rung above
    // the body ink and inset from the plate's edge by the same measure the body
    // uses, so the title and the first field share a left margin. 36px is that
    // row's height: a 16px name needs 10px of air above and below to read as a
    // header rather than as a stray line above the content.
    const TITLE_ROW_H: f32 = 36.0;
    let title_text = iced::widget::text(title)
        .size(tokens::TEXT_PANE_HEAD)
        .font(tokens::UI_FONT_SEMIBOLD);
    let title_surface: Element<'a, Message> = container(title_text)
        .width(Length::Fill)
        .height(Length::Fixed(TITLE_ROW_H))
        .padding(iced::Padding {
            left: kit::PAD_CARD.left,
            ..iced::Padding::ZERO
        })
        .align_x(iced::alignment::Horizontal::Left)
        .align_y(iced::alignment::Vertical::Center)
        .into();
    let title_surface: Element<'a, Message> = if options.movable {
        mouse_area(title_surface)
            .on_press(Message::ModalGrab)
            .interaction(iced::mouse::Interaction::Grab)
            .into()
    } else {
        title_surface
    };
    let controls: Element<'a, Message> = row![close].align_y(iced::Center).into();
    let title_bar = stack![
        title_surface,
        container(controls)
            .width(Length::Fill)
            .height(Length::Fixed(TITLE_ROW_H))
            .padding(iced::Padding {
                right: kit::PAD_CARD.right,
                ..iced::Padding::ZERO
            })
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Center),
    ]
    .width(Length::Fill)
    .height(Length::Fixed(TITLE_ROW_H));

    // The first stack layer dictates its intrinsic size. Its top spacer reserves
    // room for the title, while the actual title bar and resize grip overlay it
    // without influencing the modal dimensions.
    let measured_content = sensor(content).on_resize(Message::ModalContentResized);
    let body_base = if options.bare {
        column![measured_content]
    } else {
        column![
            Space::new().height(Length::Fixed(TITLE_ROW_H)),
            measured_content,
        ]
        .spacing(tokens::GAP_CONTROL)
    };
    let mut body = if options.bare {
        stack![body_base]
    } else {
        stack![body_base, title_bar]
    };
    // Aurora brand rule along the panel's top edge: a dialog is the highest
    // surface on screen, so it is the one place the brand mark always shows.
    body = body.push(kit::brand_rule("modal:fade"));
    if options.resizable {
        let resize = mouse_area(
            container(crate::ui::icons::themed_primary(
                crate::ui::icons::RESIZE,
                15.0,
            ))
            .padding([tokens::GAP_NONE, tokens::GAP_HAIRLINE]),
        )
        .on_press(Message::ModalResizeGrab)
        .interaction(iced::mouse::Interaction::Grab);
        body = body.push(
            container(resize)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Bottom),
        );
    }
    // A dialog is paper on the page: an opaque sheet, not a pane that reads the
    // drawing through it. The batch-047 screens put a solid card over the
    // dimmed frame, and a glass tint alone left the canvas grid showing between
    // the dialog's own cards — the sheet has to be a fill of its own. `Float`
    // is the raised ladder's detached rung (opaque surface, 1px edge, the
    // panel corner, the deepest lift), so the body keeps the inset from the
    // plate's edge (`kit::PAD_CARD`) and the host types no colour.
    // ── Footer: information band, then the action row ────────────────────
    // The band is the sample sheet's own margin note. It runs the full width
    // of the sheet — outside the body's padding — so it reads as part of the
    // paper rather than as the last row of the content. The action row is the
    // same band, one step taller, with the leading group (Help) at the start
    // and the confirming actions at the end.
    let footer_section: Option<Element<'a, Message>> = footer.map(|footer| {
        let mut section = column![kit::hdivider(Length::Fill)];
        if let Some(meta) = footer.meta {
            section = section.push(
                container(meta)
                    .width(Length::Fill)
                    .padding(Padding {
                        top: tokens::GAP_TIGHT,
                        right: kit::PAD_CARD.right,
                        bottom: tokens::GAP_TIGHT,
                        left: kit::PAD_CARD.left,
                    })
                    .style(|theme: &Theme| {
                        kit::sheet_strip_style(kit::Tier::Band, theme, "modal:fade")
                    }),
            );
        }
        if footer.leading.is_some() || footer.trailing.is_some() {
            let mut actions = row![].align_y(iced::Center);
            if let Some(leading) = footer.leading {
                actions = actions.push(leading);
            }
            actions = actions.push(Space::new().width(Length::Fill));
            if let Some(trailing) = footer.trailing {
                actions = actions.push(trailing);
            }
            section = section.push(
                container(actions)
                    .width(Length::Fill)
                    .padding(Padding {
                        top: tokens::GAP_FIELD,
                        right: kit::PAD_CARD.right,
                        bottom: tokens::GAP_FIELD,
                        left: kit::PAD_CARD.left,
                    }),
            );
        }
        section.into()
    });

    let mut sheet = column![container(body).padding(kit::PAD_CARD)];
    if let Some(section) = footer_section {
        sheet = sheet.push(section);
    }
    let framed: Element<'a, Message> = kit::glass_pane(
        container(sheet)
            .style(|theme: &Theme| kit::sheet_style(theme, "modal:fade"))
            .into(),
        theme,
        true,
        "modal:fade",
    );

    // Position via asymmetric padding (padding is non-negative): shifting a
    // centred box by `d` on an axis needs (near − far) padding = 2·d there.
    let pad = Padding {
        top: offset.y.max(0.0) * 2.0,
        right: (-offset.x).max(0.0) * 2.0,
        bottom: (-offset.y).max(0.0) * 2.0,
        left: offset.x.max(0.0) * 2.0,
    };

    // The backdrop fills the screen so dragging keeps tracking the cursor even
    // when it leaves the title bar; release anywhere ends the drag. The fill is
    // the kit's scrim, the same lid as [`backdrop`]'s.
    let backdrop = mouse_area(
        container(framed)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding(pad)
            .style(|theme: &Theme| kit::scrim_style(theme, "modal:fade")),
    )
    .on_move(Message::ModalDragMove)
    .on_release(Message::ModalDragRelease);

    stack![
        base.into(),
        // `opaque` blocks pointer events from passing through, so the dimmed
        // backdrop swallows clicks instead of closing or hitting the view.
        opaque(backdrop),
    ]
    .into()
}
