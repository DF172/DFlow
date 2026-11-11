//! PDF dialogs: Attach PDF Underlay (pages, path type, insertion, scale,
//! rotation), Underlay Layers, PDF Import Settings and Import PDF, built
//! from cards, segmented choices and toggle chips; the point cloud Attach
//! and Color Map dialogs share them.

use crate::ui::style::tokens;
use std::fmt;

use iced::widget::{column, combo_box, container, image, row, text, text_input, Space};
use iced::{Element, Fill, Length, Theme};

use crate::app::Message;
use crate::io::xref_model::Pathtype;
use crate::modules::insert::pdf_import::{ImportLayers, PdfImportSettings};
use crate::t;
use crate::ui::kit::muted_style;
use crate::ui::kit::{
    self, dialog_button, document_style, field_style, paper_style, swatch_style,
    tone_style, wash_style, Tier, Tone, PAD_CARD,
};

/// One edit in any of the PDF dialogs.
#[derive(Debug, Clone)]
pub enum PdfDialogMsg {
    // Attach
    AttachName(String),
    AttachBrowse,
    AttachPage(usize),
    /// DGN conversion units: sub (true) or master.
    AttachSubUnits(bool),
    AttachPathType(PathTypeChoice),
    AttachInsertOnScreen(bool),
    AttachInsert(usize, String),
    AttachScaleOnScreen(bool),
    AttachScale(String),
    AttachRotationOnScreen(bool),
    AttachRotation(String),
    AttachDetails(bool),
    AttachOk,
    // Attach Point Cloud
    CloudBrowse,
    CloudPathType(PathTypeChoice),
    CloudInsertOnScreen(bool),
    CloudInsert(usize, String),
    CloudScaleOnScreen(bool),
    CloudScale(String),
    CloudRotationOnScreen(bool),
    CloudRotation(String),
    CloudLock(bool),
    CloudZoom(bool),
    CloudOk,
    // Point Cloud Color Map
    /// Elevation tab (else intensity).
    MapTab(bool),
    MapScheme(String),
    MapCount(usize),
    MapEven,
    MapReverse,
    MapGradient(bool),
    MapNew,
    MapDelete,
    MapRename,
    MapNameInput(String),
    MapNameOk,
    MapNameCancel,
    MapMax(String),
    MapMin(String),
    MapInterval(String),
    MapExtents(bool),
    MapOutOfRange(OutOfRange),
    MapCurrent(bool),
    MapApply,
    MapOk,
    // Extract Section Lines from Point Cloud
    SecPerimeter(bool),
    SecMaxPoints(String),
    SecLayer(String),
    SecColor(LineColor),
    SecPolylines(bool),
    SecWidth(String),
    SecMinLength(String),
    SecConnect(String),
    SecAngle(String),
    /// Measure the connect tolerance (else the minimum length) on screen.
    SecPick(bool),
    SecPreview(bool),
    SecCreate,
    // Underlay Layers
    LayersUnderlay(String),
    LayersSearch(String),
    LayersToggle(String),
    LayersOk,
    // Import settings (the settings dialog and the Import PDF dialog)
    Vector(bool),
    Fills(bool),
    Text(bool),
    Raster(bool),
    Layers(ImportLayers),
    AsBlock(bool),
    Join(bool),
    Hatches(bool),
    Lineweights(bool),
    Linetypes(bool),
    SettingsOk,
    // Import PDF
    ImportBrowse,
    ImportPage(usize),
    ImportPageText(String),
    ImportInsertOnScreen(bool),
    ImportScale(String),
    ImportRotation(RotationChoice),
    ImportOk,
    Options,
    Help(&'static str),
}

fn msg(m: PdfDialogMsg) -> Message {
    Message::PdfDialog(m)
}

/// Path type as the Attach dialog lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathTypeChoice(pub Pathtype);

impl PathTypeChoice {
    pub const ALL: [PathTypeChoice; 3] = [
        PathTypeChoice(Pathtype::Full),
        PathTypeChoice(Pathtype::Relative),
        PathTypeChoice(Pathtype::None),
    ];
}

impl fmt::Display for PathTypeChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self.0 {
            Pathtype::Full => t!("Full path"),
            Pathtype::Relative => t!("Relative path"),
            Pathtype::None => t!("No path"),
        };
        f.write_str(label.as_ref())
    }
}

/// Rotation of an imported page: the four right angles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RotationChoice(pub u16);

impl RotationChoice {
    pub const ALL: [RotationChoice; 4] = [
        RotationChoice(0),
        RotationChoice(90),
        RotationChoice(180),
        RotationChoice(270),
    ];
}

impl fmt::Display for RotationChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A page of the chosen file and its thumbnail.
#[derive(Clone)]
pub struct PageThumb {
    pub label: String,
    pub image: Option<image::Handle>,
}

/// Thumbnails of every page, about 220 px wide (sharp at preview size).
// ponytail: all pages are rasterised when the dialog opens; lazy per-row
// thumbnails if files with hundreds of pages matter.
pub fn page_thumbs(path: &str) -> Vec<PageThumb> {
    use crate::scene::model::pdf_raster::{page_count, page_size_inches, rasterize_page_at_dpi};
    let count = page_count(path).unwrap_or(0);
    (1..=count)
        .map(|n| {
            let label = n.to_string();
            let image = page_size_inches(path, &label).and_then(|(w, h)| {
                let dpi = (220.0 / w.max(h).max(0.1)) as f32;
                let page = rasterize_page_at_dpi(path, &label, dpi)?;
                Some(image::Handle::from_rgba(
                    page.width,
                    page.height,
                    page.pixels.to_vec(),
                ))
            });
            PageThumb { label, image }
        })
        .collect()
}

/// Thumbnails of a file's pages (PDF), sheets (DWF) or models (DGN).
pub fn item_thumbs(kind: codec::entities::UnderlayType, path: &str) -> Vec<PageThumb> {
    use crate::scene::model::underlay_vector;
    if kind == codec::entities::UnderlayType::Pdf {
        return page_thumbs(path);
    }
    underlay_vector::item_names(kind, path)
        .unwrap_or_default()
        .into_iter()
        .map(|label| {
            let backdrop = underlay_vector::Backdrop {
                max_channel: 0,
                light: false,
            };
            let image = underlay_vector::display_raster(kind, path, &label, &[], 220.0, backdrop)
                .map(|page| {
                    image::Handle::from_rgba(page.width, page.height, page.pixels.to_vec())
                });
            PageThumb { label, image }
        })
        .collect()
}

/// Ctrl adds or removes a page, Shift takes the range from the last click,
/// a plain click selects that page alone.
pub fn click_page(
    selected: &mut Vec<usize>,
    anchor: &mut usize,
    page: usize,
    ctrl: bool,
    shift: bool,
) {
    if shift {
        let (a, b) = if *anchor <= page {
            (*anchor, page)
        } else {
            (page, *anchor)
        };
        *selected = (a..=b).collect();
    } else if ctrl {
        match selected.iter().position(|p| *p == page) {
            Some(at) if selected.len() > 1 => {
                selected.remove(at);
            }
            Some(_) => {}
            None => {
                selected.push(page);
                selected.sort_unstable();
            }
        }
        *anchor = page;
    } else {
        *selected = vec![page];
        *anchor = page;
    }
}

// ── Shared look ────────────────────────────────────────────────────────────
//
// The PDF dialogs are built from rounded cards on a slightly raised surface,
// accent-coloured card titles, segmented choices and toggle chips, with the
// help button on the left of the footer and the named action on the right.
// Every one of those plates and inks is painted by `kit`; this file owns no
// style authority of its own.

/// A titled card.
fn card<'a>(title: String, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    kit::surface(
        column![
            text(title).size(tokens::TEXT_MICRO).style(muted_style),
            content.into()
        ]
        .spacing(tokens::GAP_ROW),
        Tier::Card,
        PAD_CARD,
        "",
    )
    .width(Fill)
    .into()
}

/// One choice out of a few, as joined buttons.
fn segmented<'a, T: Copy + PartialEq + 'a>(
    options: Vec<(T, String)>,
    current: T,
    on: fn(T) -> PdfDialogMsg,
) -> Element<'a, Message> {
    let buttons = options.into_iter().map(|(value, label)| {
        kit::btn(
            text(label).size(tokens::TEXT_LABEL).width(Fill).align_x(iced::Center),
            if value == current {
                kit::Role::Engaged
            } else {
                kit::Role::Standard
            },
            kit::Size::Compact,
            kit::Press::On(msg(on(value))),
            "",
        )
        .width(Fill)
        .into()
    });
    container(iced::widget::Row::with_children(buttons.collect::<Vec<_>>()).spacing(tokens::GAP_HAIRLINE))
        .padding(tokens::GAP_HAIRLINE)
        .width(Fill)
        .style(|theme: &Theme| wash_style(Tier::Well, theme, ""))
        .into()
}

/// A switch shown as a chip: filled with a tick while on.
fn chip<'a>(label: String, on: bool, message: Message) -> Element<'a, Message> {
    // The tick is an SVG: the web build's font has no check-mark glyph.
    let mut content = row![].spacing(tokens::GAP_TIGHT).align_y(iced::Center);
    if on {
        content = content.push(
            iced::widget::svg(crate::ui::icons::themed_handle(
                crate::ui::icons::CHECK,
                11.0,
            ))
            .width(11.0)
            .height(11.0)
                .style(|theme: &Theme, _| iced::widget::svg::Style {
                    color: Some(theme.palette().primary.base.text),
                }),
        );
    }
    kit::btn(
        content.push(text(label).size(tokens::TEXT_LABEL)),
        if on {
            // The sheets mark a lit switch with the pale accent wash and a
            // ticked label, not a solid primary plate: only a dialog's
            // confirming action carries the full accent.
            kit::Role::Engaged
        } else {
            kit::Role::Standard
        },
        kit::Size::Control,
        kit::Press::On(message),
        "",
    )
    .into()
}

/// A labelled field on one row; read-only while `enabled` is false.
fn field<'a>(
    label: String,
    value: &'a str,
    enabled: bool,
    label_width: f32,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let mut input = text_input("", value)
        .size(tokens::TEXT_LABEL)
        .padding(kit::Size::Compact.padding())
        .width(Fill)
        .style(field_style);
    if enabled {
        input = input.on_input(on_input);
    }
    row![
        text(label)
            .size(tokens::TEXT_LABEL)
            .style(muted_style)
            .width(Length::Fixed(label_width)),
        input
    ]
    .spacing(tokens::GAP_CONTROL)
    .align_y(iced::Center)
    .into()
}

/// A label and a value on one line; a long value keeps its end.
fn info_line<'a>(label: String, value: &'a str) -> Element<'a, Message> {
    const MAX: usize = 70;
    let count = value.chars().count();
    let shown = if count > MAX {
        format!(
            "…{}",
            value.chars().skip(count - (MAX - 1)).collect::<String>()
        )
    } else {
        value.to_string()
    };
    row![
        text(label)
            .size(tokens::TEXT_LABEL)
            .style(muted_style)
            .width(Length::Fixed(96.0)),
        text(shown)
            .size(tokens::TEXT_LABEL)
            .width(Fill)
            .wrapping(iced::advanced::text::Wrapping::None),
    ]
    .spacing(tokens::GAP_CONTROL)
    .into()
}

/// Help on the left, extra buttons, then Cancel and the named action.
fn footer<'a>(
    help: &'static str,
    extra: Option<Element<'a, Message>>,
    action: std::borrow::Cow<'static, str>,
    ok: PdfDialogMsg,
    cancel: bool,
) -> Element<'a, Message> {
    let mut left = row![dialog_button("?", msg(PdfDialogMsg::Help(help)), false)]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center);
    if let Some(extra) = extra {
        left = left.push(extra);
    }
    let mut actions = row![
        left,
        Space::new().width(Fill),
    ]
    .spacing(tokens::GAP_CONTROL)
    .align_y(iced::Center);
    if cancel {
        actions = actions.push(dialog_button(t!("Cancel"), Message::CloseModal, false));
    }
    actions
        .push(dialog_button(action, msg(ok), true))
        .into()
}

/// Page thumbnails as cards; the chosen ones framed in the accent colour
/// with a ticked page number.
fn page_tiles<'a>(
    pages: &'a [PageThumb],
    selected: &[usize],
    on_click: fn(usize) -> PdfDialogMsg,
) -> Element<'a, Message> {
    let tiles = pages.iter().enumerate().map(|(index, page)| {
        let chosen = selected.contains(&index);
        let picture: Element<'a, Message> = match &page.image {
            Some(handle) => image(handle.clone()).width(Fill).height(Fill).into(),
            None => Space::new().width(Fill).height(Fill).into(),
        };
        let tick: Element<'a, Message> = if chosen {
            crate::ui::icons::themed_primary(crate::ui::icons::CHECK, 11.0)
        } else {
            Space::new().width(tokens::GAP_NONE).into()
        };
        let tile = column![
            container(picture)
                .width(Length::Fixed(132.0))
                .height(Length::Fixed(86.0))
                .padding(tokens::GAP_TIGHT)
                .style(move |theme: &Theme| {
                    paper_style(theme, iced::Color::WHITE, chosen, "")
                }),
            container(
                row![
                    tick,
                    text(crate::t!(&page.label))
                        .size(tokens::TEXT_LABEL)
                        .style(move |theme: &Theme| {
                            if chosen {
                                tone_style(theme, Tone::Accent)
                            } else {
                                muted_style(theme)
                            }
                        })
                ]
                .spacing(tokens::GAP_TIGHT)
                .align_y(iced::Center),
            )
            .width(Length::Fixed(132.0))
            .align_x(iced::Center),
        ]
        .spacing(tokens::GAP_TIGHT);
        kit::btn(
            tile,
            kit::Role::Ghost,
            kit::Size::Compact,
            kit::Press::On(msg(on_click(index))),
            "",
        )
        .padding(tokens::GAP_TIGHT)
        .into()
    });
    crate::ui::kit::themed_scrollable(
        iced::widget::Row::with_children(tiles.collect::<Vec<_>>())
            .spacing(tokens::GAP_CONTROL)
            .wrap()
            .vertical_spacing(6),
    )
    .height(Fill)
    .into()
}

// ── Attach PDF Underlay ────────────────────────────────────────────────────

/// Choices the Attach dialog keeps for its next opening.
#[derive(Clone)]
pub struct AttachMemory {
    pub path_type: PathTypeChoice,
    pub insert_on_screen: bool,
    pub scale_on_screen: bool,
    pub scale: String,
    pub rotation_on_screen: bool,
    pub rotation: String,
    pub details: bool,
}

impl Default for AttachMemory {
    fn default() -> Self {
        Self {
            path_type: PathTypeChoice(Pathtype::Relative),
            insert_on_screen: true,
            scale_on_screen: false,
            scale: "1.0000".into(),
            rotation_on_screen: false,
            rotation: "0".into(),
            details: false,
        }
    }
}

pub static ATTACH_MEMORY: std::sync::Mutex<Option<AttachMemory>> = std::sync::Mutex::new(None);

pub struct PdfAttachState {
    /// PDF pages, DWF sheets or DGN models.
    pub kind: codec::entities::UnderlayType,
    /// DGN: sub units chosen (the scale then turns them into master units).
    pub sub_units: bool,
    /// DGN: sub units per master unit of the chosen model.
    pub sub_per_master: f64,
    /// The file read (absolute).
    pub path: String,
    pub name: String,
    /// PDF files already attached: name and read path.
    pub existing: Vec<(String, String)>,
    pub name_combo: combo_box::State<String>,
    pub pages: Vec<PageThumb>,
    pub selected: Vec<usize>,
    pub anchor: usize,
    pub path_type: PathTypeChoice,
    pub insert_on_screen: bool,
    pub insert: [String; 3],
    pub scale_on_screen: bool,
    pub scale: String,
    pub rotation_on_screen: bool,
    pub rotation: String,
    pub found_in: String,
    pub saved_path: String,
    pub page_size: String,
    pub details: bool,
}

impl PdfAttachState {
    pub fn new(existing: Vec<(String, String)>) -> Self {
        let memory = ATTACH_MEMORY
            .lock()
            .ok()
            .and_then(|m| m.clone())
            .unwrap_or_default();
        let names = existing.iter().map(|(name, _)| name.clone()).collect();
        Self {
            kind: codec::entities::UnderlayType::Pdf,
            sub_units: false,
            sub_per_master: 1.0,
            path: String::new(),
            name: String::new(),
            existing,
            name_combo: combo_box::State::new(names),
            pages: Vec::new(),
            selected: vec![0],
            anchor: 0,
            path_type: memory.path_type,
            insert_on_screen: memory.insert_on_screen,
            insert: ["0.0000".into(), "0.0000".into(), "0.0000".into()],
            scale_on_screen: memory.scale_on_screen,
            scale: memory.scale,
            rotation_on_screen: memory.rotation_on_screen,
            rotation: memory.rotation,
            found_in: String::new(),
            saved_path: String::new(),
            page_size: String::new(),
            details: memory.details,
        }
    }

    pub fn remember(&self) {
        if let Ok(mut m) = ATTACH_MEMORY.lock() {
            *m = Some(AttachMemory {
                path_type: self.path_type,
                insert_on_screen: self.insert_on_screen,
                scale_on_screen: self.scale_on_screen,
                scale: self.scale.clone(),
                rotation_on_screen: self.rotation_on_screen,
                rotation: self.rotation.clone(),
                details: self.details,
            });
        }
    }

    pub fn page_labels(&self) -> Vec<String> {
        self.selected
            .iter()
            .filter_map(|i| self.pages.get(*i).map(|p| p.label.clone()))
            .collect()
    }
}

pub fn view_attach<'a>(
    state: &'a PdfAttachState,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    // File: the attached PDFs to pick from, Browse, and the page count.
    let name_combo = combo_box(
        &state.name_combo,
        "",
        (!state.name.is_empty()).then_some(&state.name),
        |chosen: String| msg(PdfDialogMsg::AttachName(chosen)),
    )
    .size(tokens::TEXT_BODY)
    .padding(kit::Size::Control.padding())
    .width(Fill);
    use codec::entities::UnderlayType;
    let count_text = match state.kind {
        UnderlayType::Pdf => crate::tf!("{count} pages", count = state.pages.len()),
        UnderlayType::Dwf => crate::tf!("{count} sheets", count = state.pages.len()),
        UnderlayType::Dgn => crate::tf!("{count} models", count = state.pages.len()),
    };
    let count = container(text(count_text).size(tokens::TEXT_LABEL).style(|theme: &Theme| kit::tone_style(theme, kit::Tone::Accent)))
        .padding(kit::Size::Control.padding())
        .style(|theme: &Theme| wash_style(Tier::Well, theme, ""));
    let file = row![
        name_combo,
        count,
        kit::btn(
            text(t!("Browse...")).size(tokens::TEXT_LABEL),
            kit::Role::Standard,
            kit::Size::Control,
            kit::Press::On(msg(PdfDialogMsg::AttachBrowse)),
            "",
        ),
    ]
    .spacing(tokens::GAP_ROW)
    .align_y(iced::Center);

    // Pages: the thumbnails, then which ones are chosen.
    let chosen = state.page_labels().join(", ");
    let (items_title, chosen_text, size_label) = match state.kind {
        UnderlayType::Pdf => (
            t!("Pages"),
            crate::tf!("Selected pages: {pages}", pages = chosen),
            t!("Page size:"),
        ),
        UnderlayType::Dwf => (
            t!("Sheets"),
            crate::tf!("Selected sheet: {sheet}", sheet = chosen),
            t!("Sheet size:"),
        ),
        UnderlayType::Dgn => (
            t!("Models"),
            crate::tf!("Selected model: {model}", model = chosen),
            t!("Model size:"),
        ),
    };
    let pages = card(
        items_title.into_owned(),
        column![
            container(page_tiles(
                &state.pages,
                &state.selected,
                PdfDialogMsg::AttachPage
            ))
            .padding(tokens::GAP_CONTROL)
            .height(Length::Fixed(226.0))
            .width(Fill)
            .style(|theme: &Theme| wash_style(Tier::Well, theme, "")),
            text(chosen_text).size(tokens::TEXT_LABEL).style(muted_style),
        ]
        .spacing(tokens::GAP_CONTROL),
    );

    // Placement: insertion point, scale and rotation, each with an
    // "On screen" chip that leaves the value to the command line.
    let on_screen = t!("On screen").into_owned();
    let insert_enabled = !state.insert_on_screen;
    let xyz = row![
        field("X".into(), &state.insert[0], insert_enabled, 12.0, |v| msg(
            PdfDialogMsg::AttachInsert(0, v)
        )),
        field("Y".into(), &state.insert[1], insert_enabled, 12.0, |v| msg(
            PdfDialogMsg::AttachInsert(1, v)
        )),
        field("Z".into(), &state.insert[2], insert_enabled, 12.0, |v| msg(
            PdfDialogMsg::AttachInsert(2, v)
        )),
    ]
    .spacing(tokens::GAP_ROW);
    let heading = |label: std::borrow::Cow<'static, str>, on: bool, message: Message| {
        row![
            text(label).size(tokens::TEXT_LABEL).width(Fill),
            chip(on_screen.clone(), on, message)
        ]
        .align_y(iced::Center)
    };
    let placement = card(
        t!("Placement").into_owned(),
        column![
            heading(
                t!("Insertion point"),
                state.insert_on_screen,
                msg(PdfDialogMsg::AttachInsertOnScreen(!state.insert_on_screen))
            ),
            xyz,
            heading(
                t!("Scale"),
                state.scale_on_screen,
                msg(PdfDialogMsg::AttachScaleOnScreen(!state.scale_on_screen))
            ),
            field(
                String::new(),
                &state.scale,
                !state.scale_on_screen,
                0.0,
                |v| msg(PdfDialogMsg::AttachScale(v))
            ),
            heading(
                t!("Rotation"),
                state.rotation_on_screen,
                msg(PdfDialogMsg::AttachRotationOnScreen(
                    !state.rotation_on_screen
                ))
            ),
            field(
                String::new(),
                &state.rotation,
                !state.rotation_on_screen,
                0.0,
                |v| msg(PdfDialogMsg::AttachRotation(v))
            ),
        ]
        .spacing(tokens::GAP_CONTROL),
    );

    let path = card(
        t!("Path type").into_owned(),
        segmented(
            PathTypeChoice::ALL
                .iter()
                .map(|c| (*c, c.to_string()))
                .collect(),
            state.path_type,
            PdfDialogMsg::AttachPathType,
        ),
    );

    let details = card(
        t!("File details").into_owned(),
        column![
            info_line(t!("Found in:").into_owned(), &state.found_in),
            info_line(t!("Saved path:").into_owned(), &state.saved_path),
            info_line(size_label.into_owned(), &state.page_size),
        ]
        .spacing(tokens::GAP_TIGHT),
    );

    let mut right = column![placement, path]
        .spacing(tokens::GAP_ROW)
        .width(Length::FillPortion(2));
    // A DGN model is attached in its master or its sub units.
    if state.kind == UnderlayType::Dgn {
        right = right.push(card(
            t!("Conversion units").into_owned(),
            column![
                segmented(
                    vec![
                        (false, t!("Master units").into_owned()),
                        (true, t!("Sub units").into_owned())
                    ],
                    state.sub_units,
                    PdfDialogMsg::AttachSubUnits,
                ),
                text(crate::tf!(
                    "Sub units make the default scale {scale}",
                    scale = 1.0 / state.sub_per_master.max(1e-12)
                ))
                .size(tokens::TEXT_LABEL)
                .style(muted_style),
            ]
            .spacing(tokens::GAP_CONTROL),
        ));
    }
    let body = row![
        column![pages, details]
            .spacing(tokens::GAP_ROW)
            .width(Length::FillPortion(3)),
        right,
    ]
    .spacing(tokens::GAP_FIELD);

    column![
        file,
        body,
        footer("attach", None, t!("Attach"), PdfDialogMsg::AttachOk, true)
    ]
    .spacing(tokens::GAP_FIELD)
    .padding(PAD_CARD)
    .width(sizing.width)
    .into()
}

// ── Attach Point Cloud ─────────────────────────────────────────────────────

/// The Attach Point Cloud dialog: the scan or project chosen, what it
/// holds, and how it is placed.
pub struct PointCloudAttachState {
    /// The file read (absolute).
    pub path: String,
    pub name: String,
    pub preview: Option<image::Handle>,
    /// "1 scan · 365281 points".
    pub summary: String,
    /// Which data the scans carry (colour, intensity, normals; then
    /// classification and segmentation).
    pub data: String,
    pub data_more: String,
    /// Width × length × height.
    pub size: String,
    pub unit: String,
    pub found_in: String,
    pub saved_path: String,
    pub path_type: PathTypeChoice,
    pub insert_on_screen: bool,
    pub insert: [String; 3],
    pub scale_on_screen: bool,
    pub scale: String,
    pub rotation_on_screen: bool,
    pub rotation: String,
    pub lock: bool,
    pub zoom: bool,
}

/// A switch that cannot be used here: dimmed, not pressable.
fn off_chip<'a>(label: String) -> Element<'a, Message> {
    container(text(label).size(tokens::TEXT_LABEL).style(muted_style))
        .padding(kit::Size::Control.padding())
        .into()
}

pub fn view_point_cloud_attach<'a>(
    state: &'a PointCloudAttachState,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let file = row![
        text_input("", &state.name)
            .size(tokens::TEXT_BODY)
            .padding(kit::Size::Control.padding())
            .width(Fill)
            .style(field_style),
        container(text(state.summary.clone()).size(tokens::TEXT_LABEL).style(|theme: &Theme| kit::tone_style(theme, kit::Tone::Accent)))
            .padding(kit::Size::Control.padding())
            .style(|theme: &Theme| wash_style(Tier::Well, theme, "")),
        kit::btn(
            text(t!("Browse...")).size(tokens::TEXT_LABEL),
            kit::Role::Standard,
            kit::Size::Control,
            kit::Press::On(msg(PdfDialogMsg::CloudBrowse)),
            "",
        ),
    ]
    .spacing(tokens::GAP_ROW)
    .align_y(iced::Center);

    let picture: Element<'a, Message> = match &state.preview {
        Some(handle) => image(handle.clone()).width(Fill).height(Fill).into(),
        None => Space::new().width(Fill).height(Fill).into(),
    };
    let preview = card(
        t!("Preview").into_owned(),
        container(picture)
            .padding(tokens::GAP_CONTROL)
            .height(Length::Fixed(300.0))
            .width(Fill)
            .style(|theme: &Theme| wash_style(Tier::Well, theme, "")),
    );

    let details = card(
        t!("File details").into_owned(),
        column![
            info_line(t!("Found in:").into_owned(), &state.found_in),
            info_line(t!("Saved path:").into_owned(), &state.saved_path),
            info_line(t!("Data:").into_owned(), &state.data),
            info_line(String::new(), &state.data_more),
            info_line(t!("Size:").into_owned(), &state.size),
            info_line(t!("Unit:").into_owned(), &state.unit),
        ]
        .spacing(tokens::GAP_TIGHT),
    );

    let on_screen = t!("On screen").into_owned();
    let heading = |label: std::borrow::Cow<'static, str>, on: bool, message: Message| {
        row![
            text(label).size(tokens::TEXT_LABEL).width(Fill),
            chip(on_screen.clone(), on, message)
        ]
        .align_y(iced::Center)
    };
    let insert_enabled = !state.insert_on_screen;
    let xyz = row![
        field("X".into(), &state.insert[0], insert_enabled, 12.0, |v| msg(
            PdfDialogMsg::CloudInsert(0, v)
        )),
        field("Y".into(), &state.insert[1], insert_enabled, 12.0, |v| msg(
            PdfDialogMsg::CloudInsert(1, v)
        )),
        field("Z".into(), &state.insert[2], insert_enabled, 12.0, |v| msg(
            PdfDialogMsg::CloudInsert(2, v)
        )),
    ]
    .spacing(tokens::GAP_ROW);
    let placement = card(
        t!("Placement").into_owned(),
        column![
            heading(
                t!("Insertion point"),
                state.insert_on_screen,
                msg(PdfDialogMsg::CloudInsertOnScreen(!state.insert_on_screen))
            ),
            xyz,
            heading(
                t!("Scale"),
                state.scale_on_screen,
                msg(PdfDialogMsg::CloudScaleOnScreen(!state.scale_on_screen))
            ),
            field(
                String::new(),
                &state.scale,
                !state.scale_on_screen,
                0.0,
                |v| msg(PdfDialogMsg::CloudScale(v))
            ),
            heading(
                t!("Rotation"),
                state.rotation_on_screen,
                msg(PdfDialogMsg::CloudRotationOnScreen(
                    !state.rotation_on_screen
                ))
            ),
            field(
                String::new(),
                &state.rotation,
                !state.rotation_on_screen,
                0.0,
                |v| msg(PdfDialogMsg::CloudRotation(v))
            ),
        ]
        .spacing(tokens::GAP_CONTROL),
    );
    let path = card(
        t!("Path type").into_owned(),
        segmented(
            PathTypeChoice::ALL
                .iter()
                .map(|c| (*c, c.to_string()))
                .collect(),
            state.path_type,
            PdfDialogMsg::CloudPathType,
        ),
    );
    let options = card(
        t!("Options").into_owned(),
        column![
            off_chip(t!("Use geographic location").into_owned()),
            chip(
                t!("Lock point cloud").into_owned(),
                state.lock,
                msg(PdfDialogMsg::CloudLock(!state.lock))
            ),
            chip(
                t!("Zoom to point cloud").into_owned(),
                state.zoom,
                msg(PdfDialogMsg::CloudZoom(!state.zoom))
            ),
        ]
        .spacing(tokens::GAP_CONTROL),
    );

    let body = row![
        column![preview, details]
            .spacing(tokens::GAP_ROW)
            .width(Length::FillPortion(3)),
        column![placement, path, options]
            .spacing(tokens::GAP_ROW)
            .width(Length::FillPortion(2)),
    ]
    .spacing(tokens::GAP_FIELD);
    column![
        file,
        body,
        footer("pointcloud", None, t!("Attach"), PdfDialogMsg::CloudOk, true)
    ]
    .spacing(tokens::GAP_FIELD)
    .padding(PAD_CARD)
    .width(sizing.width)
    .into()
}

// ── Underlay Layers ────────────────────────────────────────────────────────

/// One PDF underlay the dialog can show: its name, its file's layers, and
/// which of them it turns off.
#[derive(Clone)]
pub struct LayerTarget {
    pub name: String,
    pub handle: codec::Handle,
    pub layers: Vec<String>,
    pub hidden: Vec<String>,
}

pub struct UnderlayLayersState {
    pub targets: Vec<LayerTarget>,
    pub current: usize,
    pub search: String,
}

pub fn view_layers<'a>(
    state: &'a UnderlayLayersState,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let names: Vec<String> = state.targets.iter().map(|t| t.name.clone()).collect();
    let target = state.targets.get(state.current);
    let underlay = crate::ui::kit::themed_pick_list(target.map(|t| t.name.clone()), names, |n: &String| {
        n.clone()
    })
    .on_select(|n| msg(PdfDialogMsg::LayersUnderlay(n)))
    .text_size(tokens::TEXT_BODY)
    .padding(kit::Size::Control.padding())
    .width(Fill);
    let search = text_input(t!("Search for layer").as_ref(), &state.search)
        .on_input(|s| msg(PdfDialogMsg::LayersSearch(s)))
        .size(tokens::TEXT_LABEL)
        .padding(kit::Size::Control.padding())
        .width(Fill)
        .style(field_style);

    let (total, hidden) = target.map_or((0, 0), |t| (t.layers.len(), t.hidden.len()));
    let summary = text(crate::tf!(
        "{count} layers, {hidden} hidden",
        count = total,
        hidden = hidden
    ))
    .size(tokens::TEXT_LABEL)
    .style(muted_style);

    // Each layer: its name and an On / Off switch.
    let body: Element<'a, Message> = match target {
        Some(t) if !t.layers.is_empty() => {
            let needle = state.search.to_lowercase();
            let rows = t
                .layers
                .iter()
                .filter(|name| needle.is_empty() || name.to_lowercase().contains(&needle))
                .map(|name| {
                    let on = !t.hidden.contains(name);
                    let switch = kit::btn(
                        text(if on { t!("On") } else { t!("Off") })
                            .size(tokens::TEXT_LABEL)
                            .width(Fill)
                            .align_x(iced::Center),
                        if on {
                            kit::Role::Engaged
                        } else {
                            kit::Role::Standard
                        },
                        kit::Size::Compact,
                        kit::Press::On(msg(PdfDialogMsg::LayersToggle(name.clone()))),
                        "",
                    )
                    .width(Length::Fixed(64.0));
                    container(
                        row![text(name.clone()).size(tokens::TEXT_BODY).width(Fill), switch].align_y(iced::Center),
                    )
                    .padding(kit::PAD_WELL)
                    .style(|theme: &Theme| wash_style(Tier::Well, theme, ""))
                    .into()
                });
            crate::ui::kit::themed_scrollable(iced::widget::Column::with_children(rows.collect::<Vec<_>>()).spacing(tokens::GAP_TIGHT))
                .height(Fill)
                .into()
        }
        _ => container(
            text(t!("This file does not contain any layers"))
                .size(tokens::TEXT_LABEL)
                .style(muted_style),
        )
        .width(Fill)
        .height(Fill)
        .align_x(iced::Center)
        .align_y(iced::Center)
        .into(),
    };

    column![
        card(t!("Underlay").into_owned(), underlay),
        card(
            t!("Layers").into_owned(),
            column![
                search,
                summary,
                container(body).height(Length::Fixed(350.0))
            ]
            .spacing(tokens::GAP_CONTROL)
        ),
        footer("layers", None, t!("Apply"), PdfDialogMsg::LayersOk, true),
    ]
    .spacing(tokens::GAP_FIELD)
    .padding(PAD_CARD)
    .width(sizing.width)
    .into()
}

// ── PDF Import Settings ────────────────────────────────────────────────────

/// Content as chips, layers as a segmented choice, options as chips.
fn settings_cards<'a>(
    s: &'a PdfImportSettings,
) -> (
    Element<'a, Message>,
    Element<'a, Message>,
    Element<'a, Message>,
) {
    let fills = if s.vector {
        chip(
            t!("Solid fills").into_owned(),
            s.fills,
            msg(PdfDialogMsg::Fills(!s.fills)),
        )
    } else {
        chip(t!("Solid fills").into_owned(), s.fills, Message::Noop)
    };
    let content = card(
        t!("PDF data to import").into_owned(),
        row![
            chip(
                t!("Vector geometry").into_owned(),
                s.vector,
                msg(PdfDialogMsg::Vector(!s.vector))
            ),
            fills,
            chip(
                t!("TrueType text").into_owned(),
                s.text,
                msg(PdfDialogMsg::Text(!s.text))
            ),
            chip(
                t!("Raster images").into_owned(),
                s.raster,
                msg(PdfDialogMsg::Raster(!s.raster))
            ),
        ]
        .spacing(tokens::GAP_CONTROL)
        .wrap()
        .vertical_spacing(6),
    );
    let layers = card(
        t!("Layers").into_owned(),
        segmented(
            vec![
                (ImportLayers::Pdf, t!("Use PDF layers").into_owned()),
                (
                    ImportLayers::Object,
                    t!("Create object layers").into_owned(),
                ),
                (ImportLayers::Current, t!("Current layer").into_owned()),
            ],
            s.layers,
            PdfDialogMsg::Layers,
        ),
    );
    let options = card(
        t!("Import options").into_owned(),
        row![
            chip(
                t!("Import as block").into_owned(),
                s.as_block,
                msg(PdfDialogMsg::AsBlock(!s.as_block))
            ),
            chip(
                t!("Join line and arc segments").into_owned(),
                s.join,
                msg(PdfDialogMsg::Join(!s.join))
            ),
            chip(
                t!("Convert solid fills to hatches").into_owned(),
                s.hatches,
                msg(PdfDialogMsg::Hatches(!s.hatches))
            ),
            chip(
                t!("Apply lineweight properties").into_owned(),
                s.lineweights,
                msg(PdfDialogMsg::Lineweights(!s.lineweights))
            ),
            chip(
                t!("Infer linetypes from collinear dashes").into_owned(),
                s.linetypes,
                msg(PdfDialogMsg::Linetypes(!s.linetypes))
            ),
        ]
        .spacing(tokens::GAP_CONTROL)
        .wrap()
        .vertical_spacing(6),
    );
    (content, layers, options)
}

pub fn view_import_settings<'a>(
    settings: &'a PdfImportSettings,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let (content, layers, options) = settings_cards(settings);
    let more = kit::btn(
        text(t!("Options...")).size(tokens::TEXT_LABEL),
        kit::Role::Standard,
        kit::Size::Control,
        kit::Press::On(msg(PdfDialogMsg::Options)),
        "",
    );
    column![
        content,
        layers,
        options,
        footer(
            "settings",
            Some(more.into()),
            t!("Save"),
            PdfDialogMsg::SettingsOk,
            false,
        ),
    ]
    .spacing(tokens::GAP_FIELD)
    .padding(PAD_CARD)
    .width(sizing.width)
    .into()
}

// ── Import PDF (File) ──────────────────────────────────────────────────────

pub struct PdfImportFileState {
    pub path: String,
    pub pages: Vec<PageThumb>,
    pub selected: usize,
    pub page_text: String,
    pub insert_on_screen: bool,
    pub scale: String,
    pub rotation: RotationChoice,
    pub page_size: String,
    pub settings: PdfImportSettings,
}

pub fn view_import_file<'a>(
    state: &'a PdfImportFileState,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let file_name = std::path::Path::new(&state.path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let file = row![
        container(text(file_name).size(tokens::TEXT_BODY))
            .padding(kit::Size::Control.padding())
            .width(Fill)
            .style(|theme: &Theme| wash_style(Tier::Well, theme, "")),
        kit::btn(
            text(t!("Browse...")).size(tokens::TEXT_LABEL),
            kit::Role::Standard,
            kit::Size::Control,
            kit::Press::On(msg(PdfDialogMsg::ImportBrowse)),
            "",
        ),
    ]
    .spacing(tokens::GAP_ROW)
    .align_y(iced::Center);

    // One page at a time: a large preview with previous / next.
    let count = state.pages.len();
    let preview: Element<'a, Message> = match state
        .pages
        .get(state.selected)
        .and_then(|p| p.image.clone())
    {
        Some(handle) => image(handle).width(Fill).height(Fill).into(),
        None => Space::new().width(Fill).height(Fill).into(),
    };
    let step = |icon: Element<'a, Message>, to: Option<usize>| {
        kit::icon_btn(
            icon,
            kit::Role::Standard,
            22.0,
            // Spelled as a match: passing the Press::On function item to
            // map_or lets M be inferred from the Option's item type, which is
            // the page index, not Message.
            match to {
                Some(p) => kit::Press::On(msg(PdfDialogMsg::ImportPage(p))),
                None => kit::Press::Off,
            },
            "",
        )
    };
    let navigator = row![
        step(
            crate::ui::icons::themed_arrow_left(12.0),
            state.selected.checked_sub(1)
        ),
        Space::new().width(Fill),
        text_input("", &state.page_text)
            .on_input(|s| msg(PdfDialogMsg::ImportPageText(s)))
            .size(tokens::TEXT_LABEL)
            .padding(kit::PAD_FIELD)
            .width(Length::Fixed(48.0))
            .style(field_style),
        text(format!("/ {count}")).size(tokens::TEXT_LABEL).style(muted_style),
        Space::new().width(Fill),
        step(
            crate::ui::icons::themed_arrow_right(12.0),
            (state.selected + 1 < count).then_some(state.selected + 1),
        ),
    ]
    .spacing(tokens::GAP_CONTROL)
    .align_y(iced::Center);
    let page = card(
        t!("Pages").into_owned(),
        column![
            container(preview)
                .padding(tokens::GAP_ROW)
                .height(Length::Fixed(300.0))
                .width(Fill)
                .style(|_: &Theme| document_style(iced::Color::WHITE, "")),
            navigator,
            row![
                text(crate::tf!(
                    "Page size: {size}",
                    size = state.page_size.clone()
                ))
                .size(tokens::TEXT_LABEL)
                .style(muted_style),
                Space::new().width(Fill),
                text(crate::tf!(
                    "PDF scale: {scale}:1",
                    scale = state.scale.trim()
                ))
                .size(tokens::TEXT_LABEL)
                .style(muted_style),
            ],
        ]
        .spacing(tokens::GAP_ROW),
    );

    let placement = card(
        t!("Placement").into_owned(),
        column![
            row![
                text(t!("Insertion point")).size(tokens::TEXT_LABEL).width(Fill),
                chip(
                    t!("On screen").into_owned(),
                    state.insert_on_screen,
                    msg(PdfDialogMsg::ImportInsertOnScreen(!state.insert_on_screen))
                ),
            ]
            .align_y(iced::Center),
            field(t!("Scale").into_owned(), &state.scale, true, 60.0, |s| msg(
                PdfDialogMsg::ImportScale(s)
            )),
            row![
                text(t!("Rotation"))
                    .size(tokens::TEXT_LABEL)
                    .style(muted_style)
                    .width(Length::Fixed(60.0)),
                segmented(
                    RotationChoice::ALL
                        .iter()
                        .map(|r| (*r, format!("{}\u{b0}", r.0)))
                        .collect(),
                    state.rotation,
                    PdfDialogMsg::ImportRotation,
                ),
            ]
            .spacing(tokens::GAP_CONTROL)
            .align_y(iced::Center),
        ]
        .spacing(tokens::GAP_ROW),
    );
    let (content, layers, options) = settings_cards(&state.settings);
    let right = crate::ui::kit::themed_scrollable(column![placement, content, layers, options].spacing(tokens::GAP_ROW)).height(Fill);
    let body = row![
        container(page).width(Length::FillPortion(1)),
        container(right).width(Length::FillPortion(1)),
    ]
    .spacing(tokens::GAP_FIELD)
    .height(Length::Fixed(420.0));

    let more = kit::btn(
        text(t!("Options...")).size(tokens::TEXT_LABEL),
        kit::Role::Standard,
        kit::Size::Control,
        kit::Press::On(msg(PdfDialogMsg::Options)),
        "",
    );
    column![
        file,
        body,
        footer(
            "import",
            Some(more.into()),
            t!("Import"),
            PdfDialogMsg::ImportOk,
            true,
        )
    ]
    .spacing(tokens::GAP_FIELD)
    .padding(PAD_CARD)
    .width(sizing.width)
    .into()
}

// ── Point Cloud Color Map ──────────────────────────────────────────────────

/// A colour scheme as the dialog edits it: identifier, name and colours
/// (high end first).
#[derive(Debug, Clone, PartialEq)]
pub struct MapRamp {
    pub id: String,
    pub name: String,
    pub colors: Vec<[u8; 3]>,
}

/// What a scheme does with values outside its range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutOfRange(pub i16);

impl OutOfRange {
    pub const ALL: [OutOfRange; 3] = [OutOfRange(0), OutOfRange(1), OutOfRange(2)];
}

impl fmt::Display for OutOfRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self.0 {
            0 => t!("Use min/max colors"),
            2 => t!("Hide points"),
            _ => t!("Use RGB scan colors"),
        };
        f.write_str(label.as_ref())
    }
}

/// One tab's settings: its scheme and range.
#[derive(Debug, Clone)]
pub struct MapSlot {
    pub scheme: String,
    pub gradient: bool,
    pub max: String,
    pub min: String,
    pub out_of_range: OutOfRange,
}

/// The Point Cloud Color Map dialog: the drawing's colour schemes and, for
/// a chosen cloud, its intensity and elevation colouring.
pub struct PointCloudColorMapState {
    /// The cloud edited; `None` edits the drawing's default schemes.
    pub cloud: Option<codec::Handle>,
    /// Elevation tab (else intensity).
    pub elevation_tab: bool,
    pub ramps: Vec<MapRamp>,
    pub intensity: MapSlot,
    pub elevation: MapSlot,
    /// Elevation over the cloud's own height (else the fixed range).
    pub extents: bool,
    /// The cloud's height range.
    pub height: (f64, f64),
    pub make_current: bool,
    /// The height of one band (elevation), as shown and typed.
    pub interval: String,
    /// A name being typed for a new scheme (`true`) or a rename.
    pub naming: Option<(bool, String)>,
}

impl PointCloudColorMapState {
    pub fn slot(&self) -> &MapSlot {
        if self.elevation_tab {
            &self.elevation
        } else {
            &self.intensity
        }
    }

    pub fn slot_mut(&mut self) -> &mut MapSlot {
        if self.elevation_tab {
            &mut self.elevation
        } else {
            &mut self.intensity
        }
    }

    pub fn ramp(&self) -> Option<&MapRamp> {
        let id = &self.slot().scheme;
        self.ramps
            .iter()
            .find(|r| r.id.eq_ignore_ascii_case(id))
            .or(self.ramps.first())
    }

    pub fn ramp_mut(&mut self) -> Option<&mut MapRamp> {
        let id = self.slot().scheme.clone();
        let at = self
            .ramps
            .iter()
            .position(|r| r.id.eq_ignore_ascii_case(&id))
            .unwrap_or(0);
        self.ramps.get_mut(at)
    }

    /// The range the ramp spans on the current tab.
    pub fn range(&self) -> Option<(f64, f64)> {
        if self.elevation_tab && self.extents {
            return Some(self.height);
        }
        let slot = self.slot();
        Some((slot.min.trim().parse().ok()?, slot.max.trim().parse().ok()?))
    }
}

/// `t` (0 at the high end) along colours, blended; channels are cut to
/// whole values, as the reference stores resampled schemes.
pub fn blend(colors: &[[u8; 3]], t: f64) -> [u8; 3] {
    if colors.len() < 2 {
        return colors.first().copied().unwrap_or([255; 3]);
    }
    let x = t.clamp(0.0, 1.0) * (colors.len() - 1) as f64;
    let k = (x as usize).min(colors.len() - 2);
    let f = x - k as f64;
    let (a, b) = (colors[k], colors[k + 1]);
    [0, 1, 2].map(|c| (f64::from(a[c]) + (f64::from(b[c]) - f64::from(a[c])) * f) as u8)
}

/// A number as the dialog labels it: at most two decimals.
pub fn short(value: f64) -> String {
    let value = if value.abs() < 0.005 { 0.0 } else { value };
    let text = format!("{:.2}", value);
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn swatch<'a>(color: [u8; 3], width: f32, height: f32) -> Element<'a, Message> {
    let color = iced::Color::from_rgb8(color[0], color[1], color[2]);
    container(Space::new().width(width).height(height))
        .style(move |theme: &Theme| swatch_style(theme, color, ""))
        .into()
}

/// The ramp as a bar, high end on top, blended or in bands, with the values
/// where the bands meet (blended: the ends and the inner band centres).
fn ramp_bar<'a>(
    colors: &[[u8; 3]],
    gradient: bool,
    range: Option<(f64, f64)>,
    percent: bool,
) -> Element<'a, Message> {
    const HEIGHT: f32 = 360.0;
    let n = colors.len().max(1);
    let strips: Vec<Element<'a, Message>> = if gradient {
        (0..72)
            .map(|k| {
                let c = blend(colors, (k as f64 + 0.5) / 72.0);
                let color = iced::Color::from_rgb8(c[0], c[1], c[2]);
                container(Space::new().width(Fill).height(Fill))
                    .height(Length::FillPortion(1))
                    .style(move |_: &Theme| document_style(color, ""))
                    .into()
            })
            .collect()
    } else {
        colors
            .iter()
            .map(|c| {
                let color = iced::Color::from_rgb8(c[0], c[1], c[2]);
                container(Space::new().width(Fill).height(Fill))
                    .height(Length::FillPortion(1))
                    .style(move |_: &Theme| document_style(color, ""))
                    .into()
            })
            .collect()
    };
    let bar = container(iced::widget::Column::with_children(strips))
        .width(Length::Fixed(56.0))
        .height(Length::Fixed(HEIGHT));
    // Label positions from the top (0) to the bottom (1).
    let marks: Vec<f64> = if gradient {
        let mut marks = vec![0.0];
        marks.extend((1..n.saturating_sub(1)).map(|k| (k as f64 + 0.5) / n as f64));
        marks.push(1.0);
        marks
    } else {
        (0..=n).map(|k| k as f64 / n as f64).collect()
    };
    let mut labels = column![];
    if let Some((low, high)) = range {
        // Each label is centred on its mark, kept inside the bar.
        let mut used = 0.0;
        for mark in marks {
            let value = high - (high - low) * mark;
            let label = if percent {
                format!("{}%", value.round())
            } else {
                short(value)
            };
            let top = (mark * f64::from(HEIGHT) - 7.0).clamp(0.0, f64::from(HEIGHT) - 14.0);
            labels = labels
                .push(Space::new().height((top - used).max(0.0) as f32))
                .push(text(label).size(tokens::TEXT_MICRO).height(14.0));
            used = top.max(used) + 14.0;
        }
    }
    row![bar, labels.width(Length::Fixed(44.0))]
        .spacing(tokens::GAP_TIGHT)
        .into()
}

pub fn view_point_cloud_color_map<'a>(
    state: &'a PointCloudColorMapState,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let tab = |label: std::borrow::Cow<'static, str>,
               on: bool,
               message: Option<Message>|
     -> Element<'a, Message> {
        // The press stays On/Off, not Locked: the call sites hand a message even
        // for the open tab, so clicking it re-selects it — that behaviour is kept.
        kit::btn(
            text(label).size(tokens::TEXT_LABEL).width(Fill).align_x(iced::Center),
            if on {
                kit::Role::Engaged
            } else {
                kit::Role::Standard
            },
            kit::Size::Compact,
            match message {
                Some(message) => kit::Press::On(message),
                None => kit::Press::Off,
            },
            "",
        )
        .width(Length::Fill)
        .into()
    };
    let tabs = container(
        row![
            tab(
                t!("Intensity"),
                !state.elevation_tab,
                Some(msg(PdfDialogMsg::MapTab(false)))
            ),
            tab(
                t!("Elevation"),
                state.elevation_tab,
                Some(msg(PdfDialogMsg::MapTab(true)))
            ),
            tab(t!("Classification"), false, None),
        ]
        .spacing(tokens::GAP_HAIRLINE),
    )
    .width(Fill);

    let slot = state.slot();
    let ramp = state.ramp();
    let colors: Vec<[u8; 3]> = ramp.map(|r| r.colors.clone()).unwrap_or_default();
    let ramp_card = card(
        t!("Color ramp").into_owned(),
        ramp_bar(
            &colors,
            slot.gradient,
            state.cloud.and(state.range()),
            !state.elevation_tab,
        ),
    );

    // Listed by name, as the reference lists them.
    let mut names: Vec<String> = state.ramps.iter().map(|r| r.name.clone()).collect();
    names.sort_by_key(|n| n.to_lowercase());
    let scheme = crate::ui::kit::themed_pick_list(ramp.map(|r| r.name.clone()), names, |n: &String| n.clone())
        .on_select(|n| msg(PdfDialogMsg::MapScheme(n)))
        .text_size(tokens::TEXT_BODY)
        .padding(kit::Size::Control.padding())
        .width(Fill);
    let small = |label: std::borrow::Cow<'static, str>, message: PdfDialogMsg| {
        kit::btn(
            text(label).size(tokens::TEXT_LABEL).width(Fill).align_x(iced::Center),
            kit::Role::Standard,
            kit::Size::Control,
            kit::Press::On(msg(message)),
            "",
        )
        .width(Length::Fixed(118.0))
    };
    let counts: Vec<usize> = (2..=25).collect();
    let count = crate::ui::kit::themed_pick_list(Some(colors.len().max(2)), counts, |n: &usize| n.to_string())
        .on_select(|n| msg(PdfDialogMsg::MapCount(n)))
        .text_size(tokens::TEXT_BODY)
        .padding(kit::Size::Compact.padding())
        .width(Length::Fixed(70.0));
    // Blend: a strip from the first colour to the last.
    let ends = [
        colors.first().copied().unwrap_or([255; 3]),
        colors.last().copied().unwrap_or([255; 3]),
    ];
    let blend_strip = row((0..6).map(|k| swatch(blend(&ends, k as f64 / 5.0), 3.0, 14.0)));
    let even = kit::btn(
        blend_strip,
        kit::Role::Standard,
        kit::Size::Control,
        kit::Press::On(msg(PdfDialogMsg::MapEven)),
        "",
    );
    let reverse = kit::icon_btn(
        iced::widget::svg(crate::ui::icons::themed_handle(
            include_bytes!("../../../assets/icons/modify_reverse.svg"),
            16.0,
        ))
        .width(16.0)
        .height(16.0),
        kit::Role::Standard,
        22.0,
        kit::Press::On(msg(PdfDialogMsg::MapReverse)),
        "",
    );
    let mut scheme_rows = column![
        row![scheme, small(t!("New"), PdfDialogMsg::MapNew)]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center),
        row![
            text(t!("Number of colors"))
                .size(tokens::TEXT_LABEL)
                .width(Length::Fixed(110.0)),
            count,
            even,
            reverse,
            Space::new().width(Fill),
            small(t!("Delete"), PdfDialogMsg::MapDelete)
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
        row![
            chip(
                t!("Display as gradient").into_owned(),
                slot.gradient,
                msg(PdfDialogMsg::MapGradient(!slot.gradient))
            ),
            Space::new().width(Fill),
            small(t!("Rename"), PdfDialogMsg::MapRename)
        ]
        .align_y(iced::Center),
    ]
    .spacing(tokens::GAP_ROW);
    if let Some((_, name)) = &state.naming {
        scheme_rows = scheme_rows.push(
            row![
                text(t!("Scheme name"))
                    .size(tokens::TEXT_LABEL)
                    .style(muted_style)
                    .width(Length::Fixed(110.0)),
                text_input("", name)
                    .on_input(|v| msg(PdfDialogMsg::MapNameInput(v)))
                    .on_submit(msg(PdfDialogMsg::MapNameOk))
                    .size(tokens::TEXT_LABEL)
                    .padding(kit::Size::Compact.padding())
                    .width(Fill)
                    .style(field_style),
                dialog_button(t!("Cancel"), msg(PdfDialogMsg::MapNameCancel), false),
                dialog_button(t!("OK"), msg(PdfDialogMsg::MapNameOk), true),
            ]
            .spacing(tokens::GAP_CONTROL)
            .align_y(iced::Center),
        );
    }
    let scheme_card = card(t!("Color scheme").into_owned(), scheme_rows);

    let has_cloud = state.cloud.is_some();
    let top = colors.first().copied().unwrap_or([255; 3]);
    let bottom = colors.last().copied().unwrap_or([255; 3]);
    let editable = has_cloud && !(state.elevation_tab && state.extents);
    let with_swatch = |field: Element<'a, Message>, color: [u8; 3]| -> Element<'a, Message> {
        row![field, swatch(color, 22.0, 22.0)]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center)
            .into()
    };
    let out_of_range = {
        let list = crate::ui::kit::themed_pick_list(
            Some(slot.out_of_range),
            OutOfRange::ALL.to_vec(),
            |o: &OutOfRange| o.to_string(),
        )
        .text_size(tokens::TEXT_BODY)
        .padding(kit::Size::Compact.padding())
        .width(Fill);
        let list = if editable {
            list.on_select(|o| msg(PdfDialogMsg::MapOutOfRange(o)))
        } else {
            list
        };
        row![
            text(t!("Out of range points"))
                .size(tokens::TEXT_LABEL)
                .style(muted_style)
                .width(Length::Fixed(150.0)),
            list
        ]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center)
    };
    let range_rows = if state.elevation_tab {
        let (low, high) = state.height;
        let interval = state.interval.as_str();
        column![
            chip(
                format!(
                    "{} ({} ~ {})",
                    t!("Apply to extents of point cloud"),
                    short(low),
                    short(high)
                ),
                state.extents,
                msg(PdfDialogMsg::MapExtents(!state.extents))
            ),
            field(
                t!("Interval height").into_owned(),
                interval,
                editable,
                150.0,
                |v| msg(PdfDialogMsg::MapInterval(v))
            ),
            with_swatch(
                field(
                    t!("Maximum elevation").into_owned(),
                    &slot.max,
                    editable,
                    150.0,
                    |v| msg(PdfDialogMsg::MapMax(v))
                ),
                top
            ),
            with_swatch(
                field(
                    t!("Minimum elevation").into_owned(),
                    &slot.min,
                    editable,
                    150.0,
                    |v| msg(PdfDialogMsg::MapMin(v))
                ),
                bottom
            ),
            out_of_range,
        ]
        .spacing(tokens::GAP_ROW)
    } else {
        column![
            with_swatch(
                field(
                    t!("Maximum intensity").into_owned(),
                    &slot.max,
                    editable,
                    150.0,
                    |v| msg(PdfDialogMsg::MapMax(v))
                ),
                top
            ),
            with_swatch(
                field(
                    t!("Minimum intensity").into_owned(),
                    &slot.min,
                    editable,
                    150.0,
                    |v| msg(PdfDialogMsg::MapMin(v))
                ),
                bottom
            ),
            out_of_range,
        ]
        .spacing(tokens::GAP_ROW)
    };
    let range_card = card(t!("Range of colorized points").into_owned(), range_rows);

    let body = row![
        container(ramp_card).width(Length::Fixed(200.0)),
        column![scheme_card, range_card].spacing(tokens::GAP_ROW).width(Fill),
    ]
    .spacing(tokens::GAP_FIELD);

    let mut left = row![dialog_button("?", msg(PdfDialogMsg::Help("pointcloudcolormap")), false)]
    .spacing(tokens::GAP_ROW)
    .align_y(iced::Center);
    if has_cloud {
        left = left.push(chip(
            t!("Make stylization current").into_owned(),
            state.make_current,
            msg(PdfDialogMsg::MapCurrent(!state.make_current)),
        ));
    }
    let footer = row![
        left,
        Space::new().width(Fill),
        dialog_button(t!("Cancel"), Message::CloseModal, false),
        dialog_button(t!("Apply"), msg(PdfDialogMsg::MapApply), false),
        dialog_button(t!("OK"), msg(PdfDialogMsg::MapOk), true),
    ]
    .spacing(tokens::GAP_CONTROL)
    .align_y(iced::Center);
    column![tabs, body, footer]
        .spacing(tokens::GAP_FIELD)
        .padding(PAD_CARD)
        .width(sizing.width)
        .into()
}

// ── Extract Section Lines from Point Cloud ─────────────────────────────────

/// A colour the extracted lines can take (ACI; 256 ByLayer, 0 ByBlock).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineColor(pub i16);

impl LineColor {
    pub const ALL: [LineColor; 9] = [
        LineColor(256),
        LineColor(0),
        LineColor(1),
        LineColor(2),
        LineColor(3),
        LineColor(4),
        LineColor(5),
        LineColor(6),
        LineColor(7),
    ];
}

impl fmt::Display for LineColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self.0 {
            256 => t!("ByLayer"),
            0 => t!("ByBlock"),
            1 => t!("Red"),
            2 => t!("Yellow"),
            3 => t!("Green"),
            4 => t!("Cyan"),
            5 => t!("Blue"),
            6 => t!("Magenta"),
            _ => t!("White"),
        };
        f.write_str(label.as_ref())
    }
}

/// The dialog's fields; numbers as typed.
pub struct PcSectionState {
    pub perimeter: bool,
    pub max_points: String,
    /// "Use Current" first, then the drawing's layers.
    pub layers: Vec<String>,
    pub layer: String,
    pub color: LineColor,
    pub polylines: bool,
    pub width: String,
    pub min_length: String,
    pub connect: String,
    pub angle: String,
    pub preview: bool,
}

pub fn view_pc_section<'a>(
    state: &'a PcSectionState,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let extract = card(
        t!("Extract").into_owned(),
        segmented(
            vec![
                (false, t!("Entire cross section").into_owned()),
                (true, t!("Perimeter only").into_owned()),
            ],
            state.perimeter,
            PdfDialogMsg::SecPerimeter,
        ),
    );
    let points = state
        .max_points
        .trim()
        .parse::<f32>()
        .unwrap_or(18_000.0)
        .clamp(1_000.0, 200_000.0);
    // ponytail: the time shown scales with the points (18000 ≈ 1 minute, as
    // the reference shows it); the real rate depends on the machine.
    let minutes = (points / 18_000.0).ceil().max(1.0) as u32;
    let max_points = card(
        t!("Maximum points to process").into_owned(),
        column![
            row![
                iced::widget::slider(1_000.0..=200_000.0, points, |v| msg(
                    PdfDialogMsg::SecMaxPoints(((v / 1_000.0).round() * 1_000.0).to_string())
                ))
                .step(1_000.0)
                .width(Fill),
                text_input("", &state.max_points)
                    .on_input(|v| msg(PdfDialogMsg::SecMaxPoints(v)))
                    .size(tokens::TEXT_LABEL)
                    .padding(kit::Size::Compact.padding())
                    .width(Length::Fixed(90.0))
                    .style(field_style),
            ]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center),
            row![
                text(t!("Faster")).size(tokens::TEXT_MICRO).style(muted_style),
                Space::new().width(Fill),
                text(t!("More accurate")).size(tokens::TEXT_MICRO).style(muted_style),
                Space::new().width(Fill),
                text(crate::tf!(
                    "Estimated time: {count} minutes",
                    count = minutes
                ))
                .size(tokens::TEXT_MICRO)
                .style(muted_style),
            ],
        ]
        .spacing(tokens::GAP_CONTROL),
    );
    let layer = crate::ui::kit::themed_pick_list(
        Some(state.layer.clone()),
        state.layers.clone(),
        |n: &String| n.clone(),
    )
    .on_select(|n| msg(PdfDialogMsg::SecLayer(n)))
    .text_size(tokens::TEXT_BODY)
    .padding(kit::Size::Compact.padding())
    .width(Fill);
    let color = crate::ui::kit::themed_pick_list(
        Some(state.color),
        LineColor::ALL.to_vec(),
        |c: &LineColor| c.to_string(),
    )
    .on_select(|c| msg(PdfDialogMsg::SecColor(c)))
    .text_size(tokens::TEXT_BODY)
    .padding(kit::Size::Compact.padding())
    .width(Fill);
    let labelled = |label: std::borrow::Cow<'static, str>,
                    control: Element<'a, Message>|
     -> Element<'a, Message> {
        row![
            text(label)
                .size(tokens::TEXT_LABEL)
                .style(muted_style)
                .width(Length::Fixed(70.0)),
            control
        ]
        .spacing(tokens::GAP_CONTROL)
        .align_y(iced::Center)
        .into()
    };
    let output = card(
        t!("Output geometry").into_owned(),
        column![
            labelled(t!("Layer"), layer.into()),
            labelled(t!("Color"), color.into()),
            segmented(
                vec![
                    (false, t!("Lines").into_owned()),
                    (true, t!("2D Polylines").into_owned())
                ],
                state.polylines,
                PdfDialogMsg::SecPolylines,
            ),
            field(
                t!("Polyline width").into_owned(),
                &state.width,
                state.polylines,
                150.0,
                |v| msg(PdfDialogMsg::SecWidth(v))
            ),
        ]
        .spacing(tokens::GAP_ROW),
    );
    let pick = |connect: bool| {
        kit::btn(
            text("⌖").size(tokens::TEXT_BODY),
            kit::Role::Standard,
            kit::Size::Compact,
            kit::Press::On(msg(PdfDialogMsg::SecPick(connect))),
            "",
        )
    };
    let tolerances = card(
        t!("Extraction tolerances").into_owned(),
        column![
            row![
                field(
                    t!("Minimum line length").into_owned(),
                    &state.min_length,
                    true,
                    170.0,
                    |v| msg(PdfDialogMsg::SecMinLength(v))
                ),
                pick(false)
            ]
            .spacing(tokens::GAP_CONTROL)
            .align_y(iced::Center),
            row![
                field(
                    t!("Connect lines tolerance").into_owned(),
                    &state.connect,
                    true,
                    170.0,
                    |v| msg(PdfDialogMsg::SecConnect(v))
                ),
                pick(true)
            ]
            .spacing(tokens::GAP_CONTROL)
            .align_y(iced::Center),
            row![
                field(
                    t!("Collinear angle tolerance").into_owned(),
                    &state.angle,
                    true,
                    170.0,
                    |v| msg(PdfDialogMsg::SecAngle(v))
                ),
                text("0–10°").size(tokens::TEXT_MICRO).style(muted_style)
            ]
            .spacing(tokens::GAP_CONTROL)
            .align_y(iced::Center),
        ]
        .spacing(tokens::GAP_ROW),
    );
    let body = column![
        row![extract, max_points].spacing(tokens::GAP_FIELD),
        row![output, tolerances].spacing(tokens::GAP_FIELD),
    ]
    .spacing(tokens::GAP_FIELD);
    let footer = row![
        dialog_button("?", msg(PdfDialogMsg::Help("pcsection")), false),
        chip(
            t!("Preview result").into_owned(),
            state.preview,
            msg(PdfDialogMsg::SecPreview(!state.preview))
        ),
        Space::new().width(Fill),
        dialog_button(t!("Cancel"), Message::CloseModal, false),
        dialog_button(t!("Create"), msg(PdfDialogMsg::SecCreate), true),
    ]
    .spacing(tokens::GAP_ROW)
    .align_y(iced::Center);
    column![body, footer]
        .spacing(tokens::GAP_FIELD)
        .padding(PAD_CARD)
        .width(sizing.width)
        .into()
}
