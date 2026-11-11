//! Editable keyboard shortcut table opened by CUI / SHORTCUTS.

use crate::ui::style::tokens;
use crate::app::Message;
use crate::t;
use crate::ui::kit::muted_style;
use iced::widget::{column, container, row, text, text_input, Space};
use crate::ui::kit::plain_button;
use iced::{Element, Length, Theme};
use crate::ui::kit;

#[derive(Clone, Copy, Debug)]
pub enum ShortcutField {
    Key,
    Command,
}

pub fn view_window<'a>(
    rows: &'a [(String, String)],
    capture_row: Option<usize>,
    pending_add: bool,
    reset_confirm: bool,
    duplicate_keys: &rustc_hash::FxHashSet<String>,
    duplicate_conflicts: &[(String, String)],
    unknown_commands: &rustc_hash::FxHashSet<String>,
    close_confirm: bool,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let hint = text(t!(
        "Click + Add, press a key combination, type the command, then Apply. Esc cancels the pending row."
    ))
    .size(tokens::TEXT_LABEL)
    .style(muted_style);
    // The sample sheet's own width: measuring at the design measure keeps the
    // dialog at its proportions instead of shrink-wrapping to the widest row.
    let frame_width = if matches!(sizing.width, Length::Fill) {
        Length::Fill
    } else {
        Length::Fixed(782.0)
    };
    let gutter = iced::Padding {
        top: tokens::GAP_NONE,
        right: tokens::GAP_GUTTER,
        bottom: tokens::GAP_NONE,
        left: tokens::GAP_NONE,
    };

    let head = container(
        row![
            container(text(t!("Key")).size(tokens::TEXT_LABEL).style(muted_style)).width(Length::Fixed(180.0)),
            container(text(t!("Command")).size(tokens::TEXT_LABEL).style(muted_style)).width(sizing.width),
            Space::new().width(Length::Fixed(30.0)),
        ]
        .spacing(tokens::GAP_ROW),
    )
    .padding(gutter);

    let mut list = column![].spacing(tokens::GAP_TIGHT);
    for (idx, (key, command)) in rows.iter().enumerate() {
        // Key cell: click to arm capture, then press the combination. The
        // armed cell shows what is expected and highlights until a key
        // arrives; clicking it again cancels. A key that already exists in
        // another row is shown red. Manual key entry is available via
        // SHORTCUTS SET.
        let armed = capture_row == Some(idx);
        let duplicate = !key.is_empty() && duplicate_keys.contains(key.as_str());
        let key_label = if armed {
            t!("Press a key combination...").into_owned()
        } else if key.is_empty() {
            t!("Click, then press keys...").into_owned()
        } else {
            key.clone()
        };
        let key_box = if armed {
            plain_button(text(key_label).size(tokens::TEXT_BODY))
                .on_press(Message::ShortcutCaptureClear)
                .padding(kit::PAD_FIELD)
                .width(Length::Fixed(180.0))
                .style(kit::primary_style)
        } else if duplicate {
            // The whole field turns red, not just the text.
            plain_button(text(key_label).size(tokens::TEXT_BODY))
                .on_press(Message::ShortcutCaptureStart(idx))
                .padding(kit::PAD_FIELD)
                .width(Length::Fixed(180.0))
                .style(kit::danger_style)
        } else {
            plain_button(text(key_label).size(tokens::TEXT_BODY).style(muted_style))
                .on_press(Message::ShortcutCaptureStart(idx))
                .padding(kit::PAD_FIELD)
                .width(Length::Fixed(180.0))
                .style(kit::secondary_style)
        };
        let unknown_command = !command.is_empty() && unknown_commands.contains(command.trim());
        let command_box = text_input(t!("command").as_ref(), command)
            .on_input(move |value| Message::ShortcutEditorInput {
                idx,
                field: ShortcutField::Command,
                value,
            })
            .size(tokens::TEXT_BODY)
            .padding(kit::PAD_FIELD)
            .width(sizing.width);
        // An unrunnable command turns the whole field red, mirroring the
        // duplicate key cell.
        let command_box = if unknown_command {
            command_box.style(kit::error_field_style)
        } else {
            command_box
        };
        // Draft rows get a check (finish the addition, without applying) and
        // a cancel ✕; committed rows get the trash bin.
        let remove = if pending_add && idx == 0 {
            // Done is only clickable for a valid row: key and command filled,
            // key not already used, command actually runnable.
            let done_ok = !key.is_empty()
                && !command.is_empty()
                && !duplicate
                && !unknown_commands.contains(command.trim());
            row![
                kit::btn(
                    crate::ui::icons::themed_success(crate::ui::icons::CHECK, 12.0),
                    kit::Role::Ghost,
                    kit::Size::Compact,
                    if done_ok {
                        kit::Press::On(Message::ShortcutEditorDraftAccept)
                    } else {
                        kit::Press::Off
                    },
                    "",
                ),
                kit::btn(
                    crate::ui::icons::themed_secondary(crate::ui::icons::CLOSE, 12.0),
                    kit::Role::QuietDanger,
                    kit::Size::Compact,
                    kit::Press::On(Message::ShortcutCaptureCancel),
                    "",
                ),
            ]
            .spacing(tokens::GAP_TIGHT)
            .align_y(iced::Center)
        } else {
            // The sample sheet's remove affordance is the row's quiet ✕, not a
            // red plate: it rests as a muted glyph and arms danger on hover.
            row![kit::btn(
                crate::ui::icons::themed_secondary(crate::ui::icons::CLOSE, 12.0),
                kit::Role::QuietDanger,
                kit::Size::Compact,
                kit::Press::On(Message::ShortcutEditorRemove(idx)),
                "",
            )]
            .align_y(iced::Center)
        };
        list = list.push(
            row![key_box, command_box, remove]
                .spacing(tokens::GAP_ROW)
                .align_y(iced::Center),
        );
    }

    // While a draft row is pending, the add button doubles as the visible
    // cancel affordance (Esc works too).
    let add = if pending_add {
        plain_button(text(t!("Cancel add (Esc)")).size(tokens::TEXT_BODY))
            .on_press(Message::ShortcutCaptureCancel)
            .padding(kit::Size::Control.padding())
            .style(kit::danger_style)
    } else {
        plain_button(text(format!("+ {}", t!("Add"))).size(tokens::TEXT_BODY))
            .on_press(Message::ShortcutEditorAdd)
            .padding(kit::Size::Control.padding())
            .style(kit::secondary_style)
    };
    // The reset confirmation replaces the shortcut count so the warning
    // gets the whole left side of the bar.
    let stats = if reset_confirm {
        row![Space::new().width(Length::Fixed(0.0))]
    } else {
        row![
            text(format!("{}: {}", t!("Number of shortcuts"), rows.len()))
                .size(tokens::TEXT_BODY)
                .style(muted_style)
        ]
    };
    // Reset asks for confirmation in place: the add/reset buttons are
    // replaced by the question with Yes / No.
    let (reset_area, add_area) = if reset_confirm {
        (
            row![
                text(t!(
                    "Are you sure you want to reset? You will lose all your current shortcuts!"
                ))
                .size(tokens::TEXT_BODY)
                .style(|theme: &Theme| kit::tone_style(theme, kit::Tone::Danger)),
                plain_button(text(t!("Yes, reset")).size(tokens::TEXT_BODY))
                    .on_press(Message::ShortcutEditorResetConfirm)
                    .padding(kit::Size::Control.padding())
                    .style(kit::danger_style),
                plain_button(text(t!("No")).size(tokens::TEXT_BODY))
                    .on_press(Message::ShortcutEditorResetDeny)
                    .padding(kit::Size::Control.padding())
                    .style(kit::secondary_style),
            ]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center),
            row![Space::new().width(Length::Fixed(0.0))],
        )
    } else {
        (
            row![plain_button(text(t!("Reset to default")).size(tokens::TEXT_BODY))
                .on_press(Message::ShortcutEditorResetAsk)
                .padding(kit::Size::Control.padding())
                .style(kit::secondary_style)]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center),
            row![add],
        )
    };
    let apply = plain_button(text(t!("Apply")).size(tokens::TEXT_BODY))
        .on_press(Message::ShortcutEditorApply)
        .padding(kit::Size::Action.padding())
        .style(kit::primary_style);
    let apply_exit = plain_button(text(t!("Apply & Exit")).size(tokens::TEXT_BODY))
        .on_press(Message::ShortcutEditorApplyExit)
        .padding(kit::Size::Action.padding())
        .style(kit::primary_style);
    // The reset confirmation takes over the whole action bar.
    let apply_area = if reset_confirm {
        row![Space::new().width(Length::Fixed(0.0))]
    } else {
        row![apply, apply_exit].spacing(tokens::GAP_ROW).align_y(iced::Center)
    };

    // Persistent duplicate warning: shown until every conflicting key is
    // resolved (re-captured, changed, or a row removed).
    let mut conflict_banner = column![].spacing(tokens::GAP_HAIRLINE);
    for (key, command) in duplicate_conflicts {
        conflict_banner = conflict_banner.push(
            text(crate::tf!(
                "Shortcut already used for command: {} → {}",
                key,
                command
            ))
            .size(tokens::TEXT_BODY)
            .style(|theme: &Theme| kit::tone_style(theme, kit::Tone::Danger)),
        );
    }
    for command in unknown_commands {
        conflict_banner = conflict_banner.push(
            text(crate::tf!("Unknown command: {}", command))
                .size(tokens::TEXT_BODY)
                .style(|theme: &Theme| kit::tone_style(theme, kit::Tone::Danger)),
        );
    }

    let content = container(
        column![
            hint,
            Space::new().height(tokens::GAP_CONTROL),
            head,
            crate::ui::kit::themed_scrollable(container(list).padding(gutter)).height(sizing.height),
            Space::new().height(tokens::GAP_CONTROL),
            conflict_banner,
            Space::new().height(tokens::GAP_TIGHT),
            row![
                stats,
                add_area,
                reset_area,
                Space::new().width(sizing.width),
                apply_area
            ]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center),
        ]
        .spacing(tokens::GAP_CONTROL)
        .width(frame_width)
        .height(sizing.height),
    )
    .padding(tokens::GAP_SECTION)
    .width(frame_width)
    .height(sizing.height);

    // Unsaved-changes guard: closing with un-applied rows stacks a dimmed
    // shield and a confirmation panel on top of the editor.
    if !close_confirm {
        return content.into();
    }
    let shield = iced::widget::mouse_area(
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|theme: &Theme| kit::scrim_style(theme, "")),
    )
    .on_press(Message::ShortcutEditorCloseKeep)
    .interaction(iced::mouse::Interaction::Idle);
    let panel = container(
        column![
            text(t!("Unsaved changes will be discarded.")).size(tokens::TEXT_HEADING),
            Space::new().height(tokens::GAP_FIELD),
            row![
                plain_button(text(t!("Discard & close")).size(tokens::TEXT_BODY))
                    .on_press(Message::ShortcutEditorCloseDiscard)
                    .padding(kit::Size::Control.padding())
                    .style(kit::danger_style),
                plain_button(text(t!("Keep editing")).size(tokens::TEXT_BODY))
                    .on_press(Message::ShortcutEditorCloseKeep)
                    .padding(kit::Size::Control.padding())
                    .style(kit::secondary_style),
            ]
            .spacing(tokens::GAP_ROW),
        ]
        .spacing(tokens::GAP_TIGHT),
    )
    .padding(tokens::GAP_GUTTER)
    .width(Length::Fixed(320.0))
    .style(kit::guard_panel_style);
    let centered = container(panel)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);
    iced::widget::stack![content, iced::widget::opaque(shield), centered].into()
}
