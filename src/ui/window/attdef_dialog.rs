//! Attribute Definition (ATTDEF) and Edit Attribute Definition dialogs:
//! cards with accent headings, a help button on the left and the named
//! action on the right, like the other drawing dialogs.

use iced::widget::{checkbox, column, container, row, text, text_input, Space};
use iced::{Element, Fill, Length, Theme};

use crate::app::Message;
use crate::modules::draw::draw::attdef::{
    Justify, AFLAG_CONSTANT, AFLAG_INVISIBLE, AFLAG_LOCK, AFLAG_MULTILINE, AFLAG_PRESET,
    AFLAG_VERIFY,
};
use crate::t;
use crate::ui::kit::muted_style;
use crate::ui::kit;
use crate::ui::style::tokens;

/// One edit in either attribute definition dialog.
#[derive(Debug, Clone)]
pub enum AttdefDialogMsg {
    Mode(u8, bool),
    Annotative(bool),
    Tag(String),
    Prompt(String),
    Default(String),
    Justify(JustifyChoice),
    Style(String),
    Height(String),
    Rotation(String),
    Width(String),
    OnScreen(bool),
    Coord(usize, String),
    AlignBelow(bool),
    PickHeight,
    PickRotation,
    PickWidth,
    /// The ƒ button: pick a field for the Default value.
    InsertField,
    EditInsertField,
    /// Edit the default value in the multi-line text editor.
    EditValue,
    DismissError,
    Ok,
    Help,
    EditTag(String),
    EditPrompt(String),
    EditDefault(String),
    EditOk,
}

/// A justification as the dialog list shows it (translated label).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JustifyChoice(pub Justify);

impl std::fmt::Display for JustifyChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&t!(self.0.label()))
    }
}

/// The ATTDEF dialog's fields.
#[derive(Debug, Clone)]
pub struct AttdefDialogState {
    pub aflags: u8,
    pub annotative: bool,
    pub tag: String,
    pub prompt: String,
    pub default: String,
    pub justify: Justify,
    pub style: String,
    pub styles: Vec<String>,
    pub height: String,
    pub rotation: String,
    pub width: String,
    pub on_screen: bool,
    pub coords: [String; 3],
    pub align_below: bool,
    /// A definition was placed before in this session.
    pub can_align_below: bool,
    /// Validation message shown above the buttons.
    pub error: Option<String>,
    /// The field the Default value shows, when one was inserted.
    pub field: Option<(String, Vec<codec::Handle>)>,
}

/// The Edit Attribute Definition dialog's fields.
#[derive(Debug, Clone)]
pub struct AttdefEditState {
    pub handle: codec::Handle,
    pub tag: String,
    pub prompt: String,
    pub default: String,
    pub constant: bool,
    pub error: Option<String>,
    pub field: Option<(String, Vec<codec::Handle>)>,
    /// The Default was retyped or a field inserted: the stored field is
    /// replaced on Apply.
    pub field_changed: bool,
}

fn msg(m: AttdefDialogMsg) -> Message {
    Message::AttdefDialog(m)
}

fn card<'a>(title: String, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(
        column![
            text(title)
                .size(tokens::TEXT_MICRO)
                .style(muted_style),
            content.into()
        ]
        .spacing(tokens::GAP_ROW),
    )
    // The card is a resting plate inside a dialog body: `PAD_CARD` is the tier
    // the kit names for it, and the heading above the content already borrows
    // the kit's accent.
    .padding(kit::PAD_CARD)
    .width(Fill)
    .style(|theme: &Theme| kit::wash_style(kit::Tier::Card, theme, ""))
    .into()
}

fn labelled<'a>(label: String, field: Element<'a, Message>, enabled: bool) -> Element<'a, Message> {
    let label = text(label).size(tokens::TEXT_BODY).width(Length::Fixed(118.0));
    let label = if enabled {
        label
    } else {
        label.style(muted_style)
    };
    row![label, field].spacing(tokens::GAP_ROW).align_y(iced::Center).into()
}

fn input<'a>(
    value: &str,
    enabled: bool,
    on: fn(String) -> AttdefDialogMsg,
) -> Element<'a, Message> {
    let mut field = text_input("", value)
        .size(tokens::TEXT_BODY)
        // Every field in this dialog is a labelled row, not a primary field with
        // its caption above it: `PAD_FIELD` is the inset for that role, and it is
        // what the two pick lists and their disabled mirrors below wear too, so a
        // state change cannot resize the plate.
        .padding(kit::PAD_FIELD)
        .width(Fill)
        .style(kit::field_style);
    if enabled {
        field = field.on_input(move |v| msg(on(v)));
    }
    field.into()
}

/// The small button beside a field that takes the value from the drawing.
fn pick<'a>(
    face: Element<'a, Message>,
    tip: String,
    message: Option<AttdefDialogMsg>,
) -> Element<'a, Message> {
    let b = kit::btn(
        face,
        kit::Role::Standard,
        kit::Size::Compact,
        match message {
            Some(m) => kit::Press::On(msg(m)),
            None => kit::Press::Off,
        },
        "",
    );
    iced::widget::tooltip(
        b,
        text(tip).size(tokens::TEXT_LABEL),
        iced::widget::tooltip::Position::Bottom,
    )
    .into()
}

fn mode<'a>(label: String, on: bool, bit: u8) -> Element<'a, Message> {
    checkbox(on)
        .label(label)
        .text_size(tokens::TEXT_BODY)
        .size(tokens::MARK_SQUARE)
        .on_toggle(move |v| msg(AttdefDialogMsg::Mode(bit, v)))
        .into()
}

/// The validation message band: the kit's verdict plate, in `Danger`.
fn error_band<'a>(error: &Option<String>) -> Option<Element<'a, Message>> {
    let error = error.clone()?;
    Some(
        container(
            row![
                crate::ui::icons::semantic(ICON_WARN, 14.0),
                text(error).size(tokens::TEXT_LABEL),
                Space::new().width(Fill),
                kit::btn(
                    crate::ui::icons::themed(crate::ui::icons::CLOSE, 10.0),
                    kit::Role::Standard,
                    kit::Size::Compact,
                    kit::Press::On(msg(AttdefDialogMsg::DismissError)),
                    "",
                ),
            ]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center),
        )
        // A verdict band across a dialog body: `PAD_BAND` is what the two
        // rebuilt dialogs that draw this same plate already name it, and it is
        // one notch tighter than the `PAD_CARD` body it sits inside. Its ground
        // is the body itself — the band is dismissed in place and never floats,
        // so the `Float` rung it borrowed put a detached popup's elevation on
        // a line of text that lives inside the form.
        .padding(kit::PAD_BAND)
        .width(Fill)
        .style(|theme: &iced::Theme| {
            crate::ui::kit::tone_plate_style_on(
                theme,
                crate::ui::kit::Tone::Danger,
                crate::ui::kit::dialog_body_plate(theme),
                "",
            )
        })
        .into(),
    )
}

/// The face of the buttons that take a value from the drawing.
fn screen_pick<'a>() -> Element<'a, Message> {
    crate::ui::icons::themed(crate::ui::icons::SNAP, 14.0)
}

static ICON_WARN: &[u8] = include_bytes!("../../../assets/icons/ui/warning_triangle.svg");

pub fn view<'a>(
    state: &'a AttdefDialogState,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let f = state.aflags;
    let constant = f & AFLAG_CONSTANT != 0;
    let multiline = f & AFLAG_MULTILINE != 0;
    let modes = card(
        t!("Mode").into_owned(),
        column![
            mode(
                t!("Invisible").into_owned(),
                f & AFLAG_INVISIBLE != 0,
                AFLAG_INVISIBLE
            ),
            mode(t!("Constant").into_owned(), constant, AFLAG_CONSTANT),
            mode(
                t!("Verify").into_owned(),
                f & AFLAG_VERIFY != 0,
                AFLAG_VERIFY
            ),
            mode(
                t!("Preset").into_owned(),
                f & AFLAG_PRESET != 0,
                AFLAG_PRESET
            ),
            mode(
                t!("Lock position").into_owned(),
                f & AFLAG_LOCK != 0,
                AFLAG_LOCK
            ),
            mode(
                t!("Multiple lines").into_owned(),
                multiline,
                AFLAG_MULTILINE
            ),
        ]
        .spacing(tokens::GAP_ROW),
    );
    let coords_on = !state.on_screen && !state.align_below;
    let coord = |i: usize, axis: &'static str| -> Element<'a, Message> {
        let mut field = text_input("", &state.coords[i])
            .size(tokens::TEXT_BODY)
            .padding(kit::PAD_FIELD)
            .width(Fill)
            .style(kit::field_style);
        if coords_on {
            field = field.on_input(move |v| msg(AttdefDialogMsg::Coord(i, v)));
        }
        let label = text(axis).size(tokens::TEXT_BODY).width(Length::Fixed(14.0));
        let label = if coords_on {
            label
        } else {
            label.style(muted_style)
        };
        row![label, field].spacing(tokens::GAP_ROW).align_y(iced::Center).into()
    };
    let mut on_screen = checkbox(state.on_screen)
        .label(t!("Specify on-screen").into_owned())
        .text_size(tokens::TEXT_BODY)
        .size(tokens::MARK_SQUARE);
    if !state.align_below {
        on_screen = on_screen.on_toggle(|v| msg(AttdefDialogMsg::OnScreen(v)));
    }
    let insertion = card(
        t!("Insertion Point").into_owned(),
        column![on_screen, coord(0, "X"), coord(1, "Y"), coord(2, "Z")].spacing(tokens::GAP_ROW),
    );
    let attribute = card(
        t!("Attribute").into_owned(),
        column![
            labelled(
                t!("Tag").into_owned(),
                input(&state.tag, true, AttdefDialogMsg::Tag),
                true
            ),
            labelled(
                t!("Prompt").into_owned(),
                input(&state.prompt, !constant, AttdefDialogMsg::Prompt),
                !constant
            ),
            labelled(
                if constant { t!("Value") } else { t!("Default") }.into_owned(),
                {
                    let mut value =
                        row![input(&state.default, true, AttdefDialogMsg::Default)].spacing(tokens::GAP_CONTROL);
                    if multiline {
                        value = value.push(pick(
                            text("...").size(tokens::TEXT_BODY).into(),
                            t!("Multiline editor").into_owned(),
                            Some(AttdefDialogMsg::EditValue),
                        ));
                    }
                    value.push(pick(
                        text("fx").size(tokens::TEXT_BODY).into(),
                        t!("Insert field").into_owned(),
                        Some(AttdefDialogMsg::InsertField),
                    ))
                }
                .spacing(tokens::GAP_CONTROL)
                .into(),
                true
            ),
        ]
        .spacing(tokens::GAP_ROW),
    );
    let text_on = !state.align_below;
    let justify: Element<'a, Message> = if text_on && !multiline {
        crate::ui::kit::themed_pick_list(
            Some(JustifyChoice(state.justify)),
            Justify::ALL.map(JustifyChoice).to_vec(),
            |c: &JustifyChoice| c.to_string(),
        )
        .on_select(|c| msg(AttdefDialogMsg::Justify(c)))
        .text_size(tokens::TEXT_BODY)
        .padding(kit::PAD_FIELD)
        .width(Fill)
        .into()
    } else {
        container(
            text(JustifyChoice(state.justify).to_string())
                .size(tokens::TEXT_BODY)
                .style(muted_style),
        )
        .padding(kit::PAD_FIELD)
        .width(Fill)
        .into()
    };
    let style: Element<'a, Message> = if text_on {
        crate::ui::kit::themed_pick_list(
            Some(state.style.clone()),
            state.styles.clone(),
            |s: &String| s.clone(),
        )
        .on_select(|s| msg(AttdefDialogMsg::Style(s)))
        .text_size(tokens::TEXT_BODY)
        .padding(kit::PAD_FIELD)
        .width(Fill)
        .into()
    } else {
        container(text(state.style.clone()).size(tokens::TEXT_BODY).style(muted_style))
            .padding(kit::PAD_FIELD)
            .width(Fill)
            .into()
    };
    let mut annotative = checkbox(state.annotative)
        .label(t!("Annotative").into_owned())
        .text_size(tokens::TEXT_BODY)
        .size(tokens::MARK_SQUARE);
    if text_on {
        annotative = annotative.on_toggle(|v| msg(AttdefDialogMsg::Annotative(v)));
    }
    let size_on = text_on && state.justify != Justify::Align;
    let rot_on = text_on && !state.justify.two_point();
    let settings = card(
        t!("Text Settings").into_owned(),
        column![
            labelled(
                t!("Justification").into_owned(),
                justify,
                text_on && !multiline
            ),
            labelled(t!("Text style").into_owned(), style, text_on),
            row![Space::new().width(Length::Fixed(126.0)), annotative],
            labelled(
                t!("Text height").into_owned(),
                row![
                    input(&state.height, size_on, AttdefDialogMsg::Height),
                    pick(
                        screen_pick(),
                        t!("Specify on-screen").into_owned(),
                        size_on.then_some(AttdefDialogMsg::PickHeight)
                    ),
                ]
                .spacing(tokens::GAP_CONTROL)
                .into(),
                size_on
            ),
            labelled(
                t!("Rotation").into_owned(),
                row![
                    input(&state.rotation, rot_on, AttdefDialogMsg::Rotation),
                    pick(
                        screen_pick(),
                        t!("Specify on-screen").into_owned(),
                        rot_on.then_some(AttdefDialogMsg::PickRotation)
                    ),
                ]
                .spacing(tokens::GAP_CONTROL)
                .into(),
                rot_on
            ),
            labelled(
                t!("Boundary width").into_owned(),
                row![
                    input(&state.width, text_on && multiline, AttdefDialogMsg::Width),
                    pick(
                        screen_pick(),
                        t!("Specify on-screen").into_owned(),
                        (text_on && multiline).then_some(AttdefDialogMsg::PickWidth)
                    ),
                ]
                .spacing(tokens::GAP_CONTROL)
                .into(),
                text_on && multiline
            ),
        ]
        .spacing(tokens::GAP_ROW),
    );
    let mut align = checkbox(state.align_below)
        .label(t!("Align below previous attribute definition").into_owned())
        .text_size(tokens::TEXT_BODY)
        .size(tokens::MARK_SQUARE);
    if state.can_align_below {
        align = align.on_toggle(|v| msg(AttdefDialogMsg::AlignBelow(v)));
    }
    let body = row![
        column![modes, insertion]
            .spacing(tokens::GAP_FIELD)
            .width(Length::Fixed(200.0)),
        column![attribute, settings].spacing(tokens::GAP_FIELD).width(Fill),
    ]
    .spacing(tokens::GAP_FIELD);
    let mut content = column![body, align];
    if let Some(band) = error_band(&state.error) {
        content = content.push(band);
    }
    // The action row is on the shell's full-bleed footer
    // (`DraftFlowCAD::modal_footer`); this body ends at the alignment row.
    content
        .spacing(tokens::GAP_FIELD)
        // The dialog's own body: the same `PAD_CARD` tier the cards inside it
        // wear and the two rebuilt attribute dialogs name for a body inset.
        .padding(kit::PAD_CARD)
        .width(sizing.width)
        .into()
}

pub fn view_edit<'a>(
    state: &'a AttdefEditState,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let field =
        |label: String, value: &'a str, enabled: bool, on: fn(String) -> AttdefDialogMsg| {
            let mut input = text_input("", value)
                .size(tokens::TEXT_BODY)
                .padding(kit::PAD_FIELD)
                .width(Fill)
                .style(kit::field_style);
            if enabled {
                input = input.on_input(move |v| msg(on(v)));
            }
            let label = text(label).size(tokens::TEXT_BODY).width(Length::Fixed(80.0));
            let label = if enabled {
                label
            } else {
                label.style(muted_style)
            };
            row![label, input].spacing(tokens::GAP_ROW).align_y(iced::Center)
        };
    let content = card(
        t!("Attribute Definition").into_owned(),
        column![
            field(
                t!("Tag").into_owned(),
                &state.tag,
                true,
                AttdefDialogMsg::EditTag
            ),
            field(
                t!("Prompt").into_owned(),
                &state.prompt,
                !state.constant,
                AttdefDialogMsg::EditPrompt
            ),
            row![
                field(
                    if state.constant {
                        t!("Value")
                    } else {
                        t!("Default")
                    }
                    .into_owned(),
                    &state.default,
                    true,
                    AttdefDialogMsg::EditDefault
                ),
                pick(
                    text("fx").size(tokens::TEXT_BODY).into(),
                    t!("Insert field").into_owned(),
                    Some(AttdefDialogMsg::EditInsertField)
                ),
            ]
            .spacing(tokens::GAP_CONTROL)
            .align_y(iced::Center),
        ]
        .spacing(tokens::GAP_ROW),
    );
    let mut content = column![content];
    if let Some(band) = error_band(&state.error) {
        content = content.push(band);
    }
    content
        .push(
            row![
                Space::new().width(Fill),
                kit::dialog_button(t!("Apply"), msg(AttdefDialogMsg::EditOk), true),
            ]
            .align_y(iced::Center),
        )
        .spacing(tokens::GAP_FIELD)
        .padding(kit::PAD_CARD)
        .width(if matches!(sizing.width, Length::Fill) {
            Fill
        } else {
            Length::Fixed(582.0)
        })
        .into()
}

/// The tag messages the reference shows for the dialogs.
pub fn tag_error(tag: &str) -> Option<&'static str> {
    if tag.trim().is_empty() {
        Some("The tag cannot be empty.")
    } else if tag.trim().contains(char::is_whitespace) {
        Some("The tag may not contain spaces.")
    } else {
        None
    }
}
