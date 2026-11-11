//! Dialogs used by the Annotate tab's table and drawing-data tools.
//!
//! The working state lives outside the drawing until the user presses OK or
//! Finish.  That keeps Cancel lossless and lets the wizard validate one page
//! before advancing to the next.

use crate::ui::style::tokens;
use std::collections::BTreeSet;
use std::fmt;

use codec::types::Handle;
use iced::widget::{checkbox, column, container, row, text, text_input, Space};
use iced::{Element, Fill, Length};

use crate::app::Message;
use crate::t;
use crate::ui::kit::{dialog_button, dialog_button_styled_opt, form_radio};
use crate::ui::kit;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TableSource {
    #[default]
    Empty,
    DataLink,
    DataExtraction,
}

impl fmt::Display for TableSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Empty => t!("Empty table").into_owned(),
            Self::DataLink => t!("Data link").into_owned(),
            Self::DataExtraction => t!("Data extraction").into_owned(),
        };
        f.write_str(&label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TableInsertion {
    #[default]
    Point,
    Window,
}

impl fmt::Display for TableInsertion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Point => t!("Specify insertion point").into_owned(),
            Self::Window => t!("Specify window").into_owned(),
        };
        f.write_str(&label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataLinkChoice {
    pub handle: Handle,
    pub name: String,
    pub path: String,
    pub valid: bool,
}

impl fmt::Display for DataLinkChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.valid {
            write!(f, "🔗 {}", self.name)
        } else {
            write!(f, "⚠ {}", self.name)
        }
    }
}

#[derive(Debug, Clone)]
pub struct TableInsertState {
    pub styles: Vec<String>,
    pub style: String,
    pub source: TableSource,
    pub links: Vec<DataLinkChoice>,
    pub link: Option<DataLinkChoice>,
    pub insertion: TableInsertion,
    pub columns: String,
    pub data_rows: String,
    pub column_width: String,
    pub row_height: String,
    pub preview: bool,
    pub first_row_style: String,
    pub second_row_style: String,
    pub other_row_style: String,
    pub error: String,
}

impl Default for TableInsertState {
    fn default() -> Self {
        Self {
            styles: vec!["Standard".into()],
            style: "Standard".into(),
            source: TableSource::Empty,
            links: Vec::new(),
            link: None,
            insertion: TableInsertion::Point,
            columns: "3".into(),
            data_rows: "4".into(),
            column_width: "2.0".into(),
            row_height: "0.5".into(),
            preview: true,
            first_row_style: "Title".into(),
            second_row_style: "Header".into(),
            other_row_style: "Data".into(),
            error: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TableInsertField {
    Style(String),
    Source(TableSource),
    Link(DataLinkChoice),
    Insertion(TableInsertion),
    Columns(String),
    DataRows(String),
    ColumnWidth(String),
    RowHeight(String),
    Preview(bool),
    FirstRowStyle(String),
    SecondRowStyle(String),
    OtherRowStyle(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinkPathType {
    #[default]
    Full,
    Relative,
    FileName,
}

impl fmt::Display for LinkPathType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Full => t!("Full path").into_owned(),
            Self::Relative => t!("Relative path").into_owned(),
            Self::FileName => t!("No path").into_owned(),
        };
        f.write_str(&label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinkRange {
    #[default]
    EntireSheet,
    NamedRange,
    CellRange,
}

impl fmt::Display for LinkRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::EntireSheet => t!("Entire sheet").into_owned(),
            Self::NamedRange => t!("Named range").into_owned(),
            Self::CellRange => t!("Cell range").into_owned(),
        };
        f.write_str(&label)
    }
}

#[derive(Debug, Clone)]
pub struct DataLinkManagerState {
    pub links: Vec<DataLinkChoice>,
    pub selected: Option<DataLinkChoice>,
    pub editing: bool,
    pub editing_handle: Option<Handle>,
    pub name: String,
    pub path: String,
    pub path_type: LinkPathType,
    pub range_kind: LinkRange,
    pub range: String,
    pub sheets: Vec<String>,
    pub sheet: String,
    pub named_ranges: Vec<String>,
    pub allow_write: bool,
    pub use_source_formatting: bool,
    pub update_source_formatting: bool,
    pub insert_table: bool,
    pub more_options: bool,
    pub preview: Vec<Vec<String>>,
    pub status: String,
}

impl Default for DataLinkManagerState {
    fn default() -> Self {
        Self {
            links: Vec::new(),
            selected: None,
            editing: false,
            editing_handle: None,
            name: String::new(),
            path: String::new(),
            path_type: LinkPathType::Full,
            range_kind: LinkRange::EntireSheet,
            range: String::new(),
            sheets: Vec::new(),
            sheet: String::new(),
            named_ranges: Vec::new(),
            allow_write: false,
            use_source_formatting: true,
            update_source_formatting: true,
            insert_table: false,
            more_options: false,
            preview: Vec::new(),
            status: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DataLinkField {
    Name(String),
    Path(String),
    PathType(LinkPathType),
    RangeKind(LinkRange),
    Range(String),
    Sheet(String),
    AllowWrite(bool),
    UseSourceFormatting(bool),
    UpdateSourceFormatting(bool),
    InsertTable(bool),
    MoreOptions(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtractionPage {
    #[default]
    Begin,
    Source,
    Objects,
    Properties,
    Refine,
    Output,
    TableStyle,
    Finish,
}

impl ExtractionPage {
    pub const fn number(self) -> usize {
        match self {
            Self::Begin => 1,
            Self::Source => 2,
            Self::Objects => 3,
            Self::Properties => 4,
            Self::Refine => 5,
            Self::Output => 6,
            Self::TableStyle => 7,
            Self::Finish => 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtractionBegin {
    #[default]
    New,
    Template,
    Edit,
}

impl fmt::Display for ExtractionBegin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::New => t!("Create a new data extraction").into_owned(),
            Self::Template => t!("Use a previous extraction as a template").into_owned(),
            Self::Edit => t!("Edit an existing data extraction").into_owned(),
        };
        f.write_str(&label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtractionSource {
    #[default]
    CurrentDrawing,
    CurrentSelection,
    DrawingsAndFolders,
}

impl fmt::Display for ExtractionSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::CurrentDrawing => t!("Current drawing").into_owned(),
            Self::CurrentSelection => t!("Selected objects in current drawing").into_owned(),
            Self::DrawingsAndFolders => t!("Drawings / folders").into_owned(),
        };
        f.write_str(&label)
    }
}

#[derive(Debug, Clone)]
pub struct ExtractionObject {
    pub name: String,
    pub checked: bool,
    pub count: usize,
}

#[derive(Debug, Clone)]
pub struct ExtractionProperty {
    pub key: String,
    pub name: String,
    pub category: String,
    pub checked: bool,
}

#[derive(Debug, Clone)]
pub struct DataExtractionState {
    pub page: ExtractionPage,
    pub begin: ExtractionBegin,
    pub settings_path: String,
    pub source: ExtractionSource,
    pub include_current: bool,
    pub include_subfolders: bool,
    pub source_files: Vec<String>,
    pub selection_handles: Vec<Handle>,
    pub objects: Vec<ExtractionObject>,
    pub properties: Vec<ExtractionProperty>,
    pub categories: BTreeSet<String>,
    pub combine_identical: bool,
    pub show_count: bool,
    pub show_name: bool,
    pub preview: Vec<Vec<String>>,
    pub output_table: bool,
    pub output_file: bool,
    pub output_path: String,
    pub table_styles: Vec<String>,
    pub table_style: String,
    pub table_title: String,
    pub error: String,
}

impl Default for DataExtractionState {
    fn default() -> Self {
        Self {
            page: ExtractionPage::Begin,
            begin: ExtractionBegin::New,
            settings_path: String::new(),
            source: ExtractionSource::CurrentDrawing,
            include_current: true,
            include_subfolders: true,
            source_files: Vec::new(),
            selection_handles: Vec::new(),
            objects: Vec::new(),
            properties: vec![
                ExtractionProperty {
                    key: "type".into(),
                    name: "Type".into(),
                    category: "General".into(),
                    checked: true,
                },
                ExtractionProperty {
                    key: "handle".into(),
                    name: "Handle".into(),
                    category: "General".into(),
                    checked: true,
                },
                ExtractionProperty {
                    key: "layer".into(),
                    name: "Layer".into(),
                    category: "General".into(),
                    checked: true,
                },
                ExtractionProperty {
                    key: "color".into(),
                    name: "Color".into(),
                    category: "General".into(),
                    checked: true,
                },
                ExtractionProperty {
                    key: "linetype".into(),
                    name: "Linetype".into(),
                    category: "General".into(),
                    checked: true,
                },
                ExtractionProperty {
                    key: "details".into(),
                    name: "Details".into(),
                    category: "Geometry".into(),
                    checked: true,
                },
            ],
            categories: ["General".to_string(), "Geometry".to_string()]
                .into_iter()
                .collect(),
            combine_identical: false,
            show_count: true,
            show_name: true,
            preview: Vec::new(),
            output_table: true,
            output_file: false,
            output_path: String::new(),
            table_styles: vec!["Standard".into()],
            table_style: "Standard".into(),
            table_title: "Data Extraction".into(),
            error: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DataExtractionField {
    Begin(ExtractionBegin),
    SettingsPath(String),
    Source(ExtractionSource),
    IncludeCurrent(bool),
    IncludeSubfolders(bool),
    Object(usize, bool),
    Property(usize, bool),
    Category(String, bool),
    CombineIdentical(bool),
    ShowCount(bool),
    ShowName(bool),
    OutputTable(bool),
    OutputFile(bool),
    OutputPath(String),
    TableStyle(String),
    TableTitle(String),
}

/// The label column of this dialog family's `label : control` rows.
///
/// [`kit::LABEL_WIDTH`] (92) is the column the options dialogs pin, and it is
/// too narrow for what this family labels: a spreadsheet path, an
/// extraction-settings file, "Include subfolders". The width is therefore the
/// screen's own — but eleven rows were each re-deciding it by hand, which is
/// how two dialogs end up a pixel apart (the same reason `plot.rs` names its
/// column `SIDE_LABEL`). One number, named, and the board's blind spot for
/// `.width()` is recorded in the ledger rather than hidden.
const FIELD_LABEL: f32 = 118.0;
/// The link list's column in the Data Link Manager, owned here for the same
/// reason as [`FIELD_LABEL`]: it shows file names, not option captions.
const LINK_LIST_WIDTH: f32 = 220.0;

/// A `label : text field` row of this family.
///
/// It is not folded into `kit::stack::labeled_field` because that primitive
/// pins its label column to [`kit::LABEL_WIDTH`] (92 — see [`FIELD_LABEL`]) and
/// paints the kit's recessed field well, while every field on this screen still
/// wears iced's own field plate. The plate is a real disagreement and it is a
/// separate batch's decision (the same deferral is recorded for this screen's
/// checkboxes at ledger line 621); what this batch takes from the kit is the
/// *inset*: a field of a dialog is a control, so it wears
/// [`kit::Size::Control`] rather than the dense `[3, 6]` a table cell uses.
fn labeled_input<'a>(
    label: impl Into<String>,
    value: &'a str,
    msg: fn(String) -> Message,
) -> Element<'a, Message> {
    row![
        text(label.into()).size(tokens::TEXT_LABEL).style(kit::muted_style).width(FIELD_LABEL),
        text_input("", value)
            .on_input(msg)
            .padding(kit::Size::Control.padding())
            .size(tokens::TEXT_BODY)
            .width(Fill)
    ]
    .spacing(tokens::GAP_ROW)
    .align_y(iced::Center)
    .into()
}

fn cell_style_picker<'a>(
    label: impl Into<String>,
    selected: &'a str,
    msg: fn(String) -> Message,
) -> Element<'a, Message> {
    row![
        text(label.into()).size(tokens::TEXT_LABEL).style(kit::muted_style).width(FIELD_LABEL),
        crate::ui::kit::themed_pick_list(
            Some(selected.to_string()),
            vec![
                "Title".to_string(),
                "Header".to_string(),
                "Data".to_string()
            ],
            |value| value.clone(),
        )
        .on_select(msg)
        .padding(kit::PAD_FIELD)
        .text_size(tokens::TEXT_BODY)
        .width(Fill),
    ]
    .spacing(tokens::GAP_ROW)
    .align_y(iced::Center)
    .into()
}

fn preview_grid<'a>(rows: &'a [Vec<String>]) -> Element<'a, Message> {
    if rows.is_empty() {
        return text(t!("No preview available."))
            .size(tokens::TEXT_LABEL)
            .style(kit::muted_style)
            .into();
    }
    let lines = rows
        .iter()
        .take(8)
        .map(|row| row.join("  |  "))
        .collect::<Vec<_>>()
        .join("\n");
    container(text(lines).size(tokens::TEXT_LABEL))
        .padding(tokens::GAP_CONTROL)
        .width(Fill)
        .into()
}

pub fn table_insert_view(
    state: &TableInsertState,
    sizing: crate::ui::shell::Sizing,
) -> Element<'_, Message> {
    let style = crate::ui::kit::themed_pick_list(Some(state.style.clone()), state.styles.clone(), |value| {
        value.clone()
    })
    .on_select(Message::TableInsertStyle)
    .padding(kit::PAD_FIELD)
    .text_size(tokens::TEXT_BODY)
    .width(Fill);
    let source = column![
        form_radio(
            t!("Start from empty table"),
            TableSource::Empty,
            Some(state.source),
            |v| Message::TableInsertField(TableInsertField::Source(v)),
        ),
        form_radio(
            t!("Start from data link"),
            TableSource::DataLink,
            Some(state.source),
            |v| Message::TableInsertField(TableInsertField::Source(v)),
        ),
        form_radio(
            t!("Start from data extraction"),
            TableSource::DataExtraction,
            Some(state.source),
            |v| Message::TableInsertField(TableInsertField::Source(v)),
        ),
    ]
    .spacing(tokens::GAP_HAIRLINE);
    let link_picker: Element<'_, Message> = if state.source == TableSource::DataLink {
        row![
            text(t!("Data link")).size(tokens::TEXT_LABEL).style(kit::muted_style).width(FIELD_LABEL),
            crate::ui::kit::themed_pick_list(state.link.clone(), state.links.clone(), |value| value
                .to_string())
            .on_select(|value| Message::TableInsertField(TableInsertField::Link(value)))
            .padding(kit::PAD_FIELD)
            .text_size(tokens::TEXT_BODY)
            .width(Fill),
            kit::btn(
                text(t!("Manage…")).size(tokens::TEXT_LABEL),
                kit::Role::Standard,
                kit::Size::Compact,
                kit::Press::On(Message::DataLinkManagerOpen),
                "",
            ),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center)
        .into()
    } else {
        Space::new().height(tokens::GAP_NONE).into()
    };
    let insertion = column![
        form_radio(
            t!("Specify insertion point"),
            TableInsertion::Point,
            Some(state.insertion),
            |v| Message::TableInsertField(TableInsertField::Insertion(v)),
        ),
        form_radio(
            t!("Specify window"),
            TableInsertion::Window,
            Some(state.insertion),
            |v| Message::TableInsertField(TableInsertField::Insertion(v)),
        ),
    ];
    let settings = column![
        labeled_input(
            t!("Columns"),
            &state.columns,
            |v| Message::TableInsertField(TableInsertField::Columns(v))
        ),
        labeled_input(t!("Column width"), &state.column_width, |v| {
            Message::TableInsertField(TableInsertField::ColumnWidth(v))
        }),
        labeled_input(t!("Data rows"), &state.data_rows, |v| {
            Message::TableInsertField(TableInsertField::DataRows(v))
        }),
        labeled_input(t!("Row height"), &state.row_height, |v| {
            Message::TableInsertField(TableInsertField::RowHeight(v))
        }),
    ]
    .spacing(tokens::GAP_TIGHT);
    let cell_styles = column![
        cell_style_picker(t!("First row"), &state.first_row_style, |v| {
            Message::TableInsertField(TableInsertField::FirstRowStyle(v))
        }),
        cell_style_picker(t!("Second row"), &state.second_row_style, |v| {
            Message::TableInsertField(TableInsertField::SecondRowStyle(v))
        }),
        cell_style_picker(t!("All other rows"), &state.other_row_style, |v| {
            Message::TableInsertField(TableInsertField::OtherRowStyle(v))
        }),
    ]
    .spacing(tokens::GAP_TIGHT);
    column![
        kit::group_box(t!("Table style"), row![style]),
        kit::group_box(
            t!("Insert options"),
            column![source, link_picker].spacing(tokens::GAP_TIGHT)
        ),
        row![
            kit::group_box(t!("Insertion behavior"), insertion),
            kit::group_box(t!("Column & row settings"), settings)
        ]
        .spacing(tokens::GAP_ROW),
        kit::group_box(t!("Set cell styles"), cell_styles),
        checkbox(state.preview)
            .label(t!("Preview"))
            .on_toggle(|v| Message::TableInsertField(TableInsertField::Preview(v)))
            .size(tokens::MARK_SQUARE),
        text(&state.error).size(tokens::TEXT_LABEL),
        row![
            Space::new().width(Fill),
            dialog_button(t!("Cancel"), Message::CloseModal, false),
            dialog_button(t!("OK"), Message::TableInsertApply, true),
        ]
        .spacing(tokens::GAP_ROW),
    ]
    .spacing(tokens::GAP_ROW)
    .padding(tokens::GAP_FIELD)
    .width(sizing.width)
    .height(sizing.height)
    .into()
}

pub fn data_link_view(
    state: &DataLinkManagerState,
    sizing: crate::ui::shell::Sizing,
) -> Element<'_, Message> {
    let mut links = column![text(t!("Spreadsheet Links")).size(tokens::TEXT_LABEL).style(kit::muted_style)].spacing(tokens::GAP_HAIRLINE);
    links = links.push(
        kit::btn(
            text(t!("Create a New Data Link")).size(tokens::TEXT_BODY),
            kit::Role::Ghost,
            kit::Size::Compact,
            kit::Press::On(Message::DataLinkNew),
            "",
        )
        .width(Fill),
    );
    for link in &state.links {
        let selected = state
            .selected
            .as_ref()
            .is_some_and(|v| v.handle == link.handle);
        links = links.push(kit::list_row(
            text(link.to_string()).size(tokens::TEXT_BODY),
            selected,
            kit::Press::On(Message::DataLinkSelect(link.handle)),
            "",
        ));
    }
    let tree = container(crate::ui::kit::themed_scrollable(links).height(Fill))
        .padding(tokens::GAP_CONTROL)
        .width(Length::Fixed(LINK_LIST_WIDTH))
        .height(Fill);
    let right: Element<'_, Message> = if state.editing {
        let mut editor = column![
            labeled_input(t!("Data link name"), &state.name, |v| {
                Message::DataLinkField(DataLinkField::Name(v))
            }),
            row![
                text(t!("File")).size(tokens::TEXT_LABEL).style(kit::muted_style).width(FIELD_LABEL),
                text_input("", &state.path)
                    .on_input(|v| Message::DataLinkField(DataLinkField::Path(v)))
                    .padding(kit::Size::Control.padding())
                    .size(tokens::TEXT_BODY)
                    .width(Fill),
                kit::btn(
                    text("…"),
                    kit::Role::Standard,
                    kit::Size::Compact,
                    kit::Press::On(Message::DataLinkBrowse),
                    "",
                )
            ]
            .spacing(tokens::GAP_CONTROL)
            .align_y(iced::Center),
            row![
                text(t!("Path type")).size(tokens::TEXT_LABEL).style(kit::muted_style).width(FIELD_LABEL),
                crate::ui::kit::themed_pick_list(
                    Some(state.path_type),
                    vec![
                        LinkPathType::Full,
                        LinkPathType::Relative,
                        LinkPathType::FileName
                    ],
                    |value| value.to_string()
                )
                .on_select(|value| Message::DataLinkField(DataLinkField::PathType(value)))
                .width(Fill)
                .padding(kit::PAD_FIELD)
            ]
            .spacing(tokens::GAP_ROW),
            row![
                text(t!("Link option")).size(tokens::TEXT_LABEL).style(kit::muted_style).width(FIELD_LABEL),
                crate::ui::kit::themed_pick_list(
                    Some(state.range_kind),
                    vec![
                        LinkRange::EntireSheet,
                        LinkRange::NamedRange,
                        LinkRange::CellRange
                    ],
                    |value| value.to_string()
                )
                .on_select(|value| Message::DataLinkField(DataLinkField::RangeKind(value)))
                .width(Fill)
                .padding(kit::PAD_FIELD)
            ]
            .spacing(tokens::GAP_ROW),
        ]
        .spacing(tokens::GAP_CONTROL);
        if !state.sheets.is_empty() && state.range_kind != LinkRange::NamedRange {
            editor = editor.push(
                row![
                    text(t!("Sheet")).size(tokens::TEXT_LABEL).style(kit::muted_style).width(FIELD_LABEL),
                    crate::ui::kit::themed_pick_list(Some(state.sheet.clone()), state.sheets.clone(), |value| {
                        value.clone()
                    })
                    .on_select(|value| Message::DataLinkField(DataLinkField::Sheet(value)))
                    .width(Fill)
                    .padding(kit::PAD_FIELD),
                ]
                .spacing(tokens::GAP_ROW),
            );
        }
        if state.range_kind == LinkRange::NamedRange && !state.named_ranges.is_empty() {
            editor = editor.push(
                row![
                    text(t!("Named range")).size(tokens::TEXT_LABEL).style(kit::muted_style).width(FIELD_LABEL),
                    crate::ui::kit::themed_pick_list(
                        Some(state.range.clone()),
                        state.named_ranges.clone(),
                        |value| value.clone()
                    )
                    .on_select(|value| Message::DataLinkField(DataLinkField::Range(value)))
                    .width(Fill)
                    .padding(kit::PAD_FIELD),
                ]
                .spacing(tokens::GAP_ROW),
            );
        } else if state.range_kind != LinkRange::EntireSheet {
            editor = editor.push(labeled_input(t!("Range"), &state.range, |v| {
                Message::DataLinkField(DataLinkField::Range(v))
            }));
        }
        editor = editor.push(
            checkbox(state.more_options)
                .label(t!("More options"))
                .on_toggle(|v| Message::DataLinkField(DataLinkField::MoreOptions(v)))
                .size(tokens::MARK_SQUARE),
        );
        if state.more_options {
            editor = editor
                .push(
                    checkbox(state.allow_write)
                        .label(t!("Allow writing to source file"))
                        .on_toggle(|v| Message::DataLinkField(DataLinkField::AllowWrite(v)))
                        .size(tokens::MARK_SQUARE),
                )
                .push(
                    checkbox(state.use_source_formatting)
                        .label(t!("Use source formatting"))
                        .on_toggle(|v| {
                            Message::DataLinkField(DataLinkField::UseSourceFormatting(v))
                        })
                        .size(tokens::MARK_SQUARE),
                )
                .push(
                    checkbox(state.update_source_formatting)
                        .label(t!("Keep table updated to source formatting"))
                        .on_toggle(|v| {
                            Message::DataLinkField(DataLinkField::UpdateSourceFormatting(v))
                        })
                        .size(tokens::MARK_SQUARE),
                );
        }
        editor = editor.push(
            checkbox(state.insert_table)
                .label(t!("Insert table after saving link"))
                .on_toggle(|v| Message::DataLinkField(DataLinkField::InsertTable(v)))
                .size(tokens::MARK_SQUARE),
        );
        kit::group_box(t!("Link settings"), editor).into()
    } else if let Some(link) = &state.selected {
        column![
            kit::group_box(
                t!("Details"),
                column![
                    text(format!("{}: {}", t!("Name"), link.name)).size(tokens::TEXT_LABEL),
                    text(format!("{}: {}", t!("File"), link.path)).size(tokens::TEXT_LABEL),
                    text(if link.valid {
                        t!("Status: Linked")
                    } else {
                        t!("Status: Source not found")
                    })
                    .size(tokens::TEXT_LABEL)
                ]
                .spacing(tokens::GAP_TIGHT)
            ),
            kit::group_box(t!("Preview"), preview_grid(&state.preview)),
        ]
        .spacing(tokens::GAP_ROW)
        .into()
    } else {
        kit::group_box(
            t!("Details"),
            text(t!("Select a data link or create a new one."))
                .size(tokens::TEXT_LABEL)
                .style(kit::muted_style),
        ).into()
    };
    let actions = if state.editing {
        row![
            Space::new().width(Fill),
            dialog_button(t!("Cancel"), Message::DataLinkEditCancel, false),
            dialog_button(t!("OK"), Message::DataLinkSave, true),
        ]
        .spacing(tokens::GAP_ROW)
    } else {
        let enabled = state.selected.is_some();
        let edit = dialog_button_styled_opt(
            t!("Edit"),
            enabled.then_some(Message::DataLinkEdit),
            kit::secondary_style,
        );
        let delete = dialog_button_styled_opt(
            t!("Delete"),
            enabled.then_some(Message::DataLinkDelete),
            kit::secondary_style,
        );
        let insert = dialog_button_styled_opt(
            t!("Insert Table"),
            enabled.then_some(Message::DataLinkInsert),
            kit::primary_style,
        );
        let close = dialog_button(t!("Close"), Message::DataLinkClose, false);
        row![edit, delete, Space::new().width(Fill), insert, close].spacing(tokens::GAP_ROW)
    };
    column![
        row![tree, container(right).width(Fill).height(Fill)]
            .spacing(tokens::GAP_ROW)
            .height(Fill),
        text(&state.status).size(tokens::TEXT_LABEL),
        actions
    ]
    .spacing(tokens::GAP_ROW)
    .padding(tokens::GAP_FIELD)
    .width(sizing.width)
    .height(sizing.height)
    .into()
}

fn wizard_nav(state: &DataExtractionState) -> iced::widget::Row<'_, Message> {
    let back = dialog_button_styled_opt(
        t!("Back"),
        (state.page != ExtractionPage::Begin).then_some(Message::DataExtractionBack),
        kit::secondary_style,
    );
    let cancel = dialog_button(t!("Cancel"), Message::CloseModal, false);
    let next = dialog_button_styled_opt(
        if state.page == ExtractionPage::Finish {
            t!("Finish")
        } else {
            t!("Next")
        },
        state
            .error
            .is_empty()
            .then_some(if state.page == ExtractionPage::Finish {
                Message::DataExtractionFinish
            } else {
                Message::DataExtractionNext
            }),
        kit::primary_style,
    );
    row![Space::new().width(Fill), back, cancel, next].spacing(tokens::GAP_ROW)
}

pub fn data_extraction_view(
    state: &DataExtractionState,
    sizing: crate::ui::shell::Sizing,
) -> Element<'_, Message> {
    // One button, two platforms. The web has no folder picker and no directory
    // to walk, so there it is not pressable and says why in a tooltip; that is
    // `Press::Off`, the kit's spelling of "cannot act here", which replaces the
    // cfg'd re-binding that used to build the same button twice.
    #[cfg(not(target_arch = "wasm32"))]
    let add_folder_press = kit::Press::On(Message::DataExtractionAddFolder);
    #[cfg(target_arch = "wasm32")]
    let add_folder_press = kit::Press::Off;
    let add_folder = kit::btn(
        text(t!("Add folder…")).size(tokens::TEXT_BODY),
        kit::Role::Standard,
        kit::Size::Control,
        add_folder_press,
        "",
    );
    #[cfg(target_arch = "wasm32")]
    let add_folder = iced::widget::tooltip(
        add_folder,
        container(text(t!("Adding folders is not available in the browser. Add drawing files instead.")).size(tokens::TEXT_LABEL))
            // A tooltip body is a floating plate, not a field: it takes the
            // plate inset rather than the dense `[3, 6]` a table cell wears.
            // 1px of vertical.
            .padding(kit::PAD_PLATE)
            // The one tip in the app that stays on the kit's opaque floating
            // rung instead of a glass pane, and the reason is the button above
            // it: this tip exists only in the browser build, and the function
            // that builds it has no theme to hand a pane, because no other
            // surface in this wizard floats over anything. A `&Theme` argument
            // this view takes would be read only when `wasm32` and dead in the
            // desktop build that ships — so the tip names the kit's fallback
            // (`kit::float_style`, the single remaining user of the `Float`
            // rung outside the kit) rather than inventing a second recipe. The
            // closure's argument type comes from `Container::style`, which is
            // also what keeps this line free of a `Theme` import this file does
            // not otherwise need.
            .style(|theme| kit::float_style(theme)),
        iced::widget::tooltip::Position::Top,
    )
    // 4 was already `GAP_TIGHT`; named so the ratchet cannot re-grow it.
    .gap(tokens::GAP_TIGHT);

    let body: Element<'_, Message> = match state.page {
        ExtractionPage::Begin => kit::group_box(
            t!("Begin"),
            column![
                form_radio(
                    t!("Create a new data extraction"),
                    ExtractionBegin::New,
                    Some(state.begin),
                    |v| Message::DataExtractionField(DataExtractionField::Begin(v)),
                ),
                form_radio(
                    t!("Use a previous extraction as a template"),
                    ExtractionBegin::Template,
                    Some(state.begin),
                    |v| Message::DataExtractionField(DataExtractionField::Begin(v)),
                ),
                form_radio(
                    t!("Edit an existing data extraction"),
                    ExtractionBegin::Edit,
                    Some(state.begin),
                    |v| Message::DataExtractionField(DataExtractionField::Begin(v)),
                ),
                row![
                    text(t!("Extraction settings"))
                        .size(tokens::TEXT_LABEL)
                        .style(kit::muted_style)
                        .width(FIELD_LABEL),
                    text_input("", &state.settings_path)
                        .on_input(|v| Message::DataExtractionField(
                            DataExtractionField::SettingsPath(v)
                        ))
                        .padding(kit::Size::Control.padding())
                        .size(tokens::TEXT_BODY)
                        .width(Fill),
                    kit::btn(
                        text("…"),
                        kit::Role::Standard,
                        kit::Size::Compact,
                        kit::Press::On(Message::DataExtractionBrowseSettings),
                        "",
                    )
                ]
                .spacing(tokens::GAP_CONTROL)
                .align_y(iced::Center),
            ]
            .spacing(tokens::GAP_CONTROL),
        ).into(),
        ExtractionPage::Source => kit::group_box(
            t!("Define data source"),
            column![
                form_radio(
                    t!("Current drawing"),
                    ExtractionSource::CurrentDrawing,
                    Some(state.source),
                    |v| Message::DataExtractionField(DataExtractionField::Source(v)),
                ),
                form_radio(
                    t!("Select objects in the current drawing"),
                    ExtractionSource::CurrentSelection,
                    Some(state.source),
                    |v| Message::DataExtractionField(DataExtractionField::Source(v)),
                ),
                form_radio(
                    t!("Drawings / folders"),
                    ExtractionSource::DrawingsAndFolders,
                    Some(state.source),
                    |v| Message::DataExtractionField(DataExtractionField::Source(v)),
                ),
                checkbox(state.include_current)
                    .label(t!("Include current drawing"))
                    .on_toggle(|v| Message::DataExtractionField(
                        DataExtractionField::IncludeCurrent(v)
                    ))
                    .size(tokens::MARK_SQUARE),
                checkbox(state.include_subfolders)
                    .label(t!("Include subfolders"))
                    .on_toggle(|v| Message::DataExtractionField(
                        DataExtractionField::IncludeSubfolders(v)
                    ))
                    .size(tokens::MARK_SQUARE),
                row![
                    kit::btn(
                        text(t!("Add drawings…")).size(tokens::TEXT_BODY),
                        kit::Role::Standard,
                        kit::Size::Control,
                        kit::Press::On(Message::DataExtractionAddDrawings),
                        "",
                    ),
                    add_folder,
                    kit::btn(
                        text(t!("Clear")).size(tokens::TEXT_BODY),
                        kit::Role::Standard,
                        kit::Size::Control,
                        kit::Press::On(Message::DataExtractionClearSources),
                        "",
                    ),
                ]
                .spacing(tokens::GAP_CONTROL),
                text(state.source_files.join("\n")).size(tokens::TEXT_MICRO),
            ]
            .spacing(tokens::GAP_CONTROL),
        ).into(),
        ExtractionPage::Objects => {
            let mut list = column![].spacing(tokens::GAP_TIGHT);
            for (i, item) in state.objects.iter().enumerate() {
                list = list.push(
                    checkbox(item.checked)
                        .label(format!("{}  ({})", item.name, item.count))
                        .on_toggle(move |v| {
                            Message::DataExtractionField(DataExtractionField::Object(i, v))
                        })
                        .size(tokens::MARK_SQUARE),
                );
            }
            kit::group_box(t!("Select objects"), crate::ui::kit::themed_scrollable(list).height(Fill)).into()
        }
        ExtractionPage::Properties => {
            let mut categories = row![].spacing(tokens::GAP_FIELD);
            for category in ["General", "Geometry"] {
                let checked = state.categories.contains(category);
                let key = category.to_string();
                categories = categories.push(
                    checkbox(checked)
                        .label(category)
                        .on_toggle(move |v| {
                            Message::DataExtractionField(DataExtractionField::Category(
                                key.clone(),
                                v,
                            ))
                        })
                        .size(tokens::MARK_SQUARE),
                );
            }
            let mut list = column![categories].spacing(tokens::GAP_TIGHT);
            for (i, item) in state
                .properties
                .iter()
                .enumerate()
                .filter(|(_, p)| state.categories.contains(&p.category))
            {
                list = list.push(
                    checkbox(item.checked)
                        .label(format!("{}  ·  {}", item.name, item.category))
                        .on_toggle(move |v| {
                            Message::DataExtractionField(DataExtractionField::Property(i, v))
                        })
                        .size(tokens::MARK_SQUARE),
                );
            }
            kit::group_box(
                t!("Select properties"),
                crate::ui::kit::themed_scrollable(list).height(Fill),
            ).into()
        }
        ExtractionPage::Refine => kit::group_box(
            t!("Refine data"),
            column![
                row![
                    checkbox(state.combine_identical)
                        .label(t!("Combine identical rows"))
                        .on_toggle(|v| Message::DataExtractionField(
                            DataExtractionField::CombineIdentical(v)
                        ))
                        .size(tokens::MARK_SQUARE),
                    checkbox(state.show_count)
                        .label(t!("Show count column"))
                        .on_toggle(|v| Message::DataExtractionField(
                            DataExtractionField::ShowCount(v)
                        ))
                        .size(tokens::MARK_SQUARE),
                    checkbox(state.show_name)
                        .label(t!("Show name column"))
                        .on_toggle(
                            |v| Message::DataExtractionField(DataExtractionField::ShowName(v))
                        )
                        .size(tokens::MARK_SQUARE)
                ]
                .spacing(tokens::GAP_SECTION),
                crate::ui::kit::themed_scrollable(preview_grid(&state.preview)).height(Fill),
            ]
            .spacing(tokens::GAP_ROW),
        ).into(),
        ExtractionPage::Output => kit::group_box(
            t!("Choose output"),
            column![
                checkbox(state.output_table)
                    .label(t!("Insert data extraction table into drawing"))
                    .on_toggle(
                        |v| Message::DataExtractionField(DataExtractionField::OutputTable(v))
                    )
                    .size(tokens::MARK_SQUARE),
                checkbox(state.output_file)
                    .label(t!("Output data to external file"))
                    .on_toggle(|v| Message::DataExtractionField(DataExtractionField::OutputFile(v)))
                    .size(tokens::MARK_SQUARE),
                row![
                    text(t!("Output file")).size(tokens::TEXT_LABEL).style(kit::muted_style).width(FIELD_LABEL),
                    text_input("", &state.output_path)
                        .on_input(|v| Message::DataExtractionField(
                            DataExtractionField::OutputPath(v)
                        ))
                        .padding(kit::Size::Control.padding())
                        .size(tokens::TEXT_BODY)
                        .width(Fill),
                    kit::btn(
                        text("…"),
                        kit::Role::Standard,
                        kit::Size::Compact,
                        kit::Press::On(Message::DataExtractionBrowseOutput),
                        "",
                    )
                ]
                .spacing(tokens::GAP_CONTROL)
                .align_y(iced::Center),
            ]
            .spacing(tokens::GAP_CONTROL),
        ).into(),
        ExtractionPage::TableStyle => kit::group_box(
            t!("Table style"),
            column![
                row![
                    text(t!("Style")).size(tokens::TEXT_LABEL).style(kit::muted_style).width(FIELD_LABEL),
                    crate::ui::kit::themed_pick_list(
                        Some(state.table_style.clone()),
                        state.table_styles.clone(),
                        |value| value.clone()
                    )
                    .on_select(|value| Message::DataExtractionField(
                        DataExtractionField::TableStyle(value)
                    ))
                    .padding(kit::PAD_FIELD)
                    .width(Fill)
                ]
                .spacing(tokens::GAP_ROW),
                labeled_input(t!("Title"), &state.table_title, |v| {
                    Message::DataExtractionField(DataExtractionField::TableTitle(v))
                }),
                preview_grid(&state.preview),
            ]
            .spacing(tokens::GAP_CONTROL),
        ).into(),
        ExtractionPage::Finish => kit::group_box(
            t!("Finish"),
            column![
                text(t!("The selected drawing data is ready to be extracted.")).size(tokens::TEXT_BODY),
                text(format!(
                    "{}: {}",
                    t!("Rows"),
                    state.preview.len().saturating_sub(1)
                ))
                .size(tokens::TEXT_LABEL),
                text(format!(
                    "{}: {}{}",
                    t!("Output"),
                    if state.output_table {
                        t!("Drawing table")
                    } else {
                        "".into()
                    },
                    if state.output_file {
                        format!("  {}", t!("External file"))
                    } else {
                        String::new()
                    }
                ))
                .size(tokens::TEXT_LABEL)
            ]
            .spacing(tokens::GAP_CONTROL),
        ).into(),
    };
    column![
        text(format!(
            "{}  {}/8",
            t!("Data Extraction"),
            state.page.number()
        ))
        .size(tokens::TEXT_HEADING),
        container(body).height(Fill).width(Fill),
        text(&state.error).size(tokens::TEXT_LABEL),
        wizard_nav(state),
    ]
    .spacing(tokens::GAP_ROW)
    .padding(tokens::GAP_FIELD)
    .width(sizing.width)
    .height(sizing.height)
    .into()
}
