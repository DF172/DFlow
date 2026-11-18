//! Shared colour selector: a dropdown-style button that opens a list of named
//! colours (each shown with its swatch) plus the full ACI palette. Used by the
//! properties panel and every style editor so colour selection looks and
//! behaves the same everywhere.

use crate::ui::style::tokens;
use crate::app::Message;
use crate::t;
use crate::ui::properties::acad_color_display;
use crate::ui::ROW_H;
use codec::types::Color as AcadColor;
use iced::widget::{column, container, row, text};
use crate::ui::kit::plain_button;
use iced::{Color, Element, Length, Theme};
use crate::ui::kit;

/// Which "logical" entries the colour list offers besides the standard ACI
/// colours.
#[derive(Clone, Copy, Default)]
pub struct ColorExtras {
    pub by_layer: bool,
    pub by_block: bool,
    pub none: bool,
    pub background: bool,
}

/// Encode a colour as the ACI integer string the style editors store
/// (ByBlock=0, ByLayer=256, indexed 1-255). True colours are mapped to the
/// closest ACI entry because these fields cannot store RGB values.
pub fn color_to_aci_string(c: AcadColor) -> String {
    match c {
        AcadColor::ByBlock => "0".to_string(),
        AcadColor::ByLayer => "256".to_string(),
        AcadColor::None => "257".to_string(),
        AcadColor::Index(i) => i.to_string(),
        AcadColor::Rgb { r, g, b } => nearest_aci(r, g, b).to_string(),
    }
}

/// Convert an Iced colour chosen by `iced_aw::ColorPicker` into a DWG true
/// colour. ACI-only destinations map it to their closest indexed colour later.
pub fn iced_to_acad_color(color: Color) -> AcadColor {
    // Truncate the way the picker labels its channels, so the stored colour
    // is the one the dialog showed; rounding drifted dragged values by one
    // (#1621).
    let byte = |c: f32| (c * 255.0) as u8;
    AcadColor::Rgb {
        r: byte(color.r),
        g: byte(color.g),
        b: byte(color.b),
    }
}

/// Return the closest CAD Color Index for an RGB colour.
pub fn nearest_aci(r: u8, g: u8, b: u8) -> u8 {
    let mut best = 7;
    let mut best_distance = u32::MAX;

    for index in 1..=255 {
        let Some((ar, ag, ab)) = codec::types::aci_table::aci_to_rgb(index) else {
            continue;
        };
        let dr = i32::from(r) - i32::from(ar);
        let dg = i32::from(g) - i32::from(ag);
        let db = i32::from(b) - i32::from(ab);
        let distance = (dr * dr + dg * dg + db * db) as u32;
        if distance < best_distance {
            best = index;
            best_distance = distance;
        }
    }

    best
}

/// Decode an ACI integer string back into an `AcadColor`.
pub fn aci_string_to_color(s: &str) -> AcadColor {
    match s.trim().parse::<i16>().unwrap_or(256) {
        0 => AcadColor::ByBlock,
        256 => AcadColor::ByLayer,
        257 => AcadColor::None,
        n if (1..=255).contains(&n) => AcadColor::Index(n as u8),
        _ => AcadColor::ByLayer,
    }
}

/// Display name for a colour: the standard name for ACI 1-9 / ByLayer /
/// ByBlock, otherwise the "R,G,B" values (0-255) so unnamed palette colours
/// read meaningfully.
pub fn color_display_name(c: AcadColor) -> String {
    let (_, label) = acad_color_display(c);
    if label == "Index" || label == "Custom" {
        match c {
            AcadColor::Index(i) => {
                let (r, g, b) = codec::types::aci_table::aci_to_rgb(i).unwrap_or((128, 128, 128));
                format!("{r},{g},{b}")
            }
            AcadColor::Rgb { r, g, b } => format!("{r},{g},{b}"),
            _ => label.to_string(),
        }
    } else {
        t!(label).into_owned()
    }
}

/// A small colour square.
///
/// The fill is the sampled colour — data, not chrome — so this plate cannot go
/// through a `Tier`. It goes through [`kit::swatch_style`], the kit primitive
/// that takes the caller's colour as data and supplies the theme hairline at
/// `tokens::RADIUS_MARK` itself: a white swatch on a light popup and a black
/// one on a dark popup are both invisible without that edge.
pub fn swatch<'a>(bg: Color) -> Element<'a, Message> {
    container(text("").width(tokens::MARK_SQUARE).height(tokens::MARK_SQUARE))
        .style(move |theme: &Theme| kit::swatch_style(theme, bg, ""))
        .width(tokens::MARK_SQUARE)
        .height(tokens::MARK_SQUARE)
        .into()
}

/// Build a colour selector.
///
/// * `current` — the currently selected colour (shown on the button).
/// * `open` — whether the colour list / palette is expanded.
/// * `extras` — whether ByLayer / ByBlock appear in the list.
/// * `on_select` — called with the chosen colour.
/// * `on_toggle` — opens / closes the list.
pub fn color_selector<'a>(
    current: AcadColor,
    open: bool,
    extras: ColorExtras,
    on_select: impl Fn(AcadColor) -> Message + 'a,
    on_toggle: Message,
    on_more: Message,
    theme: &Theme,
) -> Element<'a, Message> {
    color_selector_with_name(
        current,
        None,
        open,
        extras,
        on_select,
        on_toggle,
        on_more,
        theme,
    )
}

pub fn color_selector_with_name<'a>(
    current: AcadColor,
    display_name: Option<&'a str>,
    open: bool,
    extras: ColorExtras,
    on_select: impl Fn(AcadColor) -> Message + 'a,
    on_toggle: Message,
    on_more: Message,
    theme: &Theme,
) -> Element<'a, Message> {
    let (cur_bg, _) = acad_color_display(current);
    let cur_name = display_name
        .map(str::to_string)
        .unwrap_or_else(|| color_display_name(current));

    color_selector_with_indicator(
        swatch(cur_bg),
        cur_name,
        open,
        extras,
        on_select,
        on_toggle,
        on_more,
        theme,
    )
}

/// Build the shared colour selector for a mixed selection.
pub fn color_selector_varies<'a>(
    open: bool,
    extras: ColorExtras,
    on_select: impl Fn(AcadColor) -> Message + 'a,
    on_toggle: Message,
    on_more: Message,
    theme: &Theme,
) -> Element<'a, Message> {
    let indicator = container(text(crate::t!("?")).size(tokens::TEXT_MICRO))
        .style(|theme: &Theme| kit::mark_plate_style(theme, ""))
        .width(tokens::MARK_SQUARE)
        .height(tokens::MARK_SQUARE)
        .align_x(iced::Center)
        .align_y(iced::Center);

    color_selector_with_indicator(
        indicator.into(),
        "*VARIES*".to_string(),
        open,
        extras,
        on_select,
        on_toggle,
        on_more,
        theme,
    )
}

fn color_selector_with_indicator<'a>(
    indicator: Element<'a, Message>,
    name: String,
    open: bool,
    extras: ColorExtras,
    on_select: impl Fn(AcadColor) -> Message + 'a,
    on_toggle: Message,
    on_more: Message,
    theme: &Theme,
) -> Element<'a, Message> {
    let on_dismiss = on_toggle.clone();

    // Closed button: current swatch + name + caret.
    let head = plain_button(
        row![
            indicator,
            text(name).size(tokens::TEXT_LABEL),
            iced::widget::Space::new().width(Length::Fill),
            crate::ui::icons::themed_arrow_toggle(open, 9.0),
        ]
        .spacing(tokens::GAP_TIGHT)
        .align_y(iced::Center),
    )
    .on_press(on_toggle)
    .padding(kit::PAD_FIELD)
    .height(ROW_H)
    .width(Length::Fill)
    // Without a style iced paints this head with the palette primary plate,
    // which in the DraftFlow sheet is the signal orange: a value field would
    // read as the dialog's one action. It is a field, so it wears the field
    // plate the rows beside it use.
    .style(kit::secondary_style);

    if !open {
        return head.into();
    }

    // The list drops from the field and floats over the rows under it, so it
    // reads them through a glass pane instead of wearing the old opaque popup
    // plate. Its `GAP_HAIRLINE` inset is unchanged, and the pane adds no size:
    // it is the under-layer of a `Stack` whose content fixes the bounds.
    let plate = container(color_list(extras, on_select, on_more))
        .padding(tokens::GAP_HAIRLINE)
        .style(|theme: &Theme| kit::Tier::Flush.style(theme, ""));
    let popup = kit::glass_pane(plate.into(), theme, false, "");

    // `DropDown` keeps the popup outside the surrounding form layout and
    // handles viewport placement, Escape, and outside-click dismissal.
    // By omitting `.width(...)`, DropDown defaults to the exact pixel width
    // of the underlay (`head`), matching the value column width and aligning flush.
    iced_aw::DropDown::new(head, popup, true)
        .alignment(iced_aw::drop_down::Alignment::Bottom)
        .offset(2.0)
        .on_dismiss(on_dismiss)
        .into()
}

/// The colour list shown inside a picker popup: named ACI colours (with
/// swatches) plus a "Select Color..." entry that opens the full palette window.
/// Shared by `color_selector` and the ribbon's colour overlay.
pub fn color_list<'a>(
    extras: ColorExtras,
    on_select: impl Fn(AcadColor) -> Message + 'a,
    on_more: Message,
) -> Element<'a, Message> {
    let named_row = |color: AcadColor| -> Element<'a, Message> {
        let (bg, name) = acad_color_display(color);
        kit::list_row(
            row![swatch(bg), text(t!(name)).size(tokens::TEXT_LABEL)]
                .spacing(tokens::GAP_TIGHT)
                .align_y(iced::Center),
            false,
            kit::Press::On(on_select(color)),
            "",
        )
        .into()
    };

    let logical_row = |color: AcadColor, label: &'static str| -> Element<'a, Message> {
        let (bg, _) = acad_color_display(color);
        kit::list_row(
            row![swatch(bg), text(t!(label)).size(tokens::TEXT_LABEL)]
                .spacing(tokens::GAP_TIGHT)
                .align_y(iced::Center),
            false,
            kit::Press::On(on_select(color)),
            "",
        )
        .into()
    };

    let mut list = column![].spacing(tokens::GAP_HAIRLINE);
    if extras.none {
        list = list.push(named_row(AcadColor::None));
    }
    if extras.background {
        list = list.push(logical_row(AcadColor::ByBlock, "Background"));
    }
    if extras.by_layer {
        list = list.push(named_row(AcadColor::ByLayer));
    }
    if extras.by_block {
        list = list.push(named_row(AcadColor::ByBlock));
    }
    for i in 1u8..=9 {
        list = list.push(named_row(AcadColor::Index(i)));
    }
    list = list.push(
        kit::list_row(
            text(t!("Select Color...")).size(tokens::TEXT_LABEL),
            false,
            kit::Press::On(on_more),
            "",
        ),
    );
    list.into()
}

/// Render `base` inline with `popup` in an `iced_aw` dropdown.
pub fn drop_down_below<'a>(
    base: Element<'a, Message>,
    popup: Element<'a, Message>,
    popup_width: Option<Length>,
    popup_height: Length,
    on_dismiss: Message,
) -> Element<'a, Message> {
    let mut dd = iced_aw::DropDown::new(base, popup, true)
        .height(popup_height)
        .alignment(iced_aw::drop_down::Alignment::Bottom)
        .offset(2.0)
        .on_dismiss(on_dismiss);
    if let Some(w) = popup_width {
        dd = dd.width(w);
    }
    dd.into()
}

/// Full CAD indexed-colour page used by the standalone Select Color dialog.
///
/// This only edits the pending colour. The caller commits it with
/// `Message::ColorWindowPick`, so choosing a swatch does not immediately close
/// the dialog.
pub fn index_color_page<'a>(
    current: AcadColor,
    recent_colors: &'a [AcadColor],
) -> Element<'a, Message> {
    let swatch_button = |color: AcadColor, size: f32| -> Element<'a, Message> {
        let (bg, _) = acad_color_display(color);
        let selected = current == color;

        // The cell's fill *is* the sampled colour, so no `Role` can paint it:
        // `chip_button_style` and `list_row` both own the background, which would
        // erase the swatch. `kit::swatch_button_style` is the style that takes a
        // caller's colour and supplies only the frame around it — edge, corner and
        // selected ring, all read off L0. Two numbers here used to be hand-spun:
        // a 1px corner (now `RADIUS_MARK`) and a 2.5px selected ring (now
        // `EDGE_FRAME`, which also makes this cell agree with the ribbon flyout
        // that always used 2.0).
        plain_button(
            text("")
                .width(Length::Fixed(size))
                .height(Length::Fixed(size)),
        )
        .on_press(Message::ColorPickerColorChanged(color))
        .style(kit::swatch_button_style(bg, selected))
        .padding(tokens::GAP_NONE)
        .into()
    };

    // The first standard indexed CAD colours.
    let mut standard = row![].spacing(tokens::GAP_TIGHT);
    for idx in 1u8..=9 {
        standard = standard.push(swatch_button(AcadColor::Index(idx), 22.0));
    }

    // ACI 10-249.
    //
    // Each group of ten indices belongs together. Arrange the 24 colour
    // families horizontally and their ten shade/intensity variants vertically.
    // This makes the palette read as a colour spectrum instead of a raw numeric
    // sequence.
    let mut palette_rows = column![].spacing(tokens::GAP_HAIRLINE);

    for variant in 0u8..10 {
        let mut palette_row = row![].spacing(tokens::GAP_HAIRLINE);

        for family in 1u8..=24 {
            let idx = family * 10 + variant;
            palette_row = palette_row.push(swatch_button(AcadColor::Index(idx), 16.0));
        }

        palette_rows = palette_rows.push(palette_row);
    }

    // ACI 250-255 are the grayscale entries and are shown separately.
    let mut grayscale = row![].spacing(tokens::GAP_TIGHT);

    for idx in 250u8..=255 {
        grayscale = grayscale.push(swatch_button(AcadColor::Index(idx), 22.0));
    }

    let recent: Element<'a, Message> = if recent_colors.is_empty() {
        text(crate::t!("No recent colors yet"))
            .size(tokens::TEXT_MICRO)
            .style(kit::muted_style)
            .into()
    } else {
        let mut recent_row = row![].spacing(tokens::GAP_TIGHT);

        for &color in recent_colors.iter().take(12) {
            recent_row = recent_row.push(swatch_button(color, 22.0));
        }

        recent_row.into()
    };

    let selected_text = match current {
        AcadColor::Index(i) => {
            let (r, g, b) = codec::types::aci_table::aci_to_rgb(i).unwrap_or((128, 128, 128));
            format!("ACI {i}    RGB {r}, {g}, {b}")
        }
        AcadColor::Rgb { r, g, b } => crate::tf!("True Color    RGB {r}, {g}, {b}").into_owned(),
        AcadColor::ByLayer => t!("ByLayer").into_owned(),
        AcadColor::ByBlock => t!("ByBlock").into_owned(),
        AcadColor::None => t!("None").into_owned(),
    };

    let tabs = row![
        kit::dialog_button_styled_opt(
            crate::t!("Index Color"),
            None::<Message>,
            kit::primary_style,
        ),
        kit::dialog_button(
            crate::t!("True Color"),
            Message::ColorPickerTabChanged(crate::app::ColorPickerTab::TrueColor),
            false,
        ),
    ]
    .spacing(tokens::GAP_TIGHT);

    let actions = container(
        row![
            kit::dialog_button(crate::t!("Cancel"), Message::CloseColorPicker, false),
            kit::dialog_button(crate::t!("OK"), Message::ColorWindowPick(current), true),
        ]
        .spacing(tokens::GAP_ROW),
    )
    .width(Length::Fill)
    .align_x(iced::alignment::Horizontal::Right);

    container(
        column![
            tabs,
            text(crate::t!("Standard colors")).size(tokens::TEXT_LABEL),
            standard,
            text(crate::t!("CAD Color Index (ACI) 10–249")).size(tokens::TEXT_LABEL),
            text(crate::t!("Color family  →    ·    Shade / intensity  ↓"))
                .size(tokens::TEXT_MICRO)
                .style(kit::muted_style),
            container(palette_rows)
                .padding(tokens::GAP_CONTROL)
                .style(|theme: &Theme| kit::Tier::Well.style(theme, "")),
            text(crate::t!("Grayscale 250–255")).size(tokens::TEXT_LABEL),
            grayscale,
            text(crate::t!("Recently used")).size(tokens::TEXT_LABEL),
            recent,
            text(selected_text).size(tokens::TEXT_LABEL),
            actions,
        ]
        .spacing(tokens::GAP_FIELD),
    )
    .padding(tokens::GAP_FIELD)
    .width(Length::Fixed(470.0))
    .into()
}
