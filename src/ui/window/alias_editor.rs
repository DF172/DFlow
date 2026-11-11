//! Command-alias editor — an in-canvas modal (Plan B) for adding, remapping and
//! removing command-line aliases (the `ocad.pgp` table). Opened by ALIASEDIT.
//! Rows are `(alias, command)`; edits are buffered in `alias_editor_rows` and
//! committed to the alias table on Apply. Mirrors the shortcut editor:
//! transient draft row with explicit accept, duplicate/unknown validation,
//! reset-to-defaults with confirmation, apply-and-exit, alias count, and an
//! unsaved-changes guard on close.

use crate::ui::style::tokens;
use crate::app::Message;
use crate::t;
use crate::ui::kit::muted_style;
use iced::widget::{column, container, row, text, text_input, Space};
use crate::ui::kit::plain_button;
use iced::{Element, Length, Theme};
use crate::ui::kit;

/// Which column of an alias row a text edit targets.
#[derive(Clone, Copy, Debug)]
pub enum AliasField {
    Alias,
    Command,
}

/// Build the alias editor content. `rows` is the live working buffer.
pub fn view_window<'a>(
    rows: &'a [(String, String)],
    pending_add: bool,
    reset_confirm: bool,
    duplicate_aliases: &rustc_hash::FxHashSet<String>,
    duplicate_conflicts: &[(String, String)],
    unknown_commands: &rustc_hash::FxHashSet<String>,
    close_confirm: bool,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let hint = text(t!(
        "Click + Add alias, type the alias and command, accept the row, then Apply. Esc cancels the pending row."
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

    // Right gutter reserved so the scrollbar has its own lane and never sits on
    // top of the row delete (✕) buttons. Applied to both the header and the
    // scrollable rows so the columns stay aligned.
    let gutter = iced::Padding {
        top: 0.0,
        right: tokens::GAP_GUTTER,
        bottom: 0.0,
        left: 0.0,
    };

    let head = container(
        row![
            container(text(t!("Alias")).size(tokens::TEXT_LABEL).style(muted_style)).width(Length::Fixed(120.0)),
            container(text(t!("Command")).size(tokens::TEXT_LABEL).style(muted_style)).width(sizing.width),
            Space::new().width(Length::Fixed(62.0)),
        ]
        .spacing(tokens::GAP_ROW),
    )
    .padding(gutter);

    let mut list = column![].spacing(tokens::GAP_TIGHT);
    for (idx, (alias, cmd)) in rows.iter().enumerate() {
        let alias_key = alias.trim().to_uppercase();
        let duplicate = !alias_key.is_empty() && duplicate_aliases.contains(alias_key.as_str());
        let alias_box = text_input(t!("alias").as_ref(), alias)
            .on_input(move |v| Message::AliasEditorInput {
                idx,
                field: AliasField::Alias,
                value: v,
            })
            .size(tokens::TEXT_META)
            .padding(kit::PAD_FIELD)
            .width(Length::Fixed(120.0));
        let alias_box = if duplicate {
            alias_box.style(kit::error_field_style)
        } else {
            alias_box
        };
        let unknown_command = !cmd.trim().is_empty() && unknown_commands.contains(cmd.trim());
        let cmd_box = text_input(t!("command").as_ref(), cmd)
            .on_input(move |v| Message::AliasEditorInput {
                idx,
                field: AliasField::Command,
                value: v,
            })
            .size(tokens::TEXT_META)
            .padding(kit::PAD_FIELD)
            .width(sizing.width);
        let cmd_box = if unknown_command {
            cmd_box.style(kit::error_field_style)
        } else {
            cmd_box
        };
        // Draft rows get a check (finish the addition, without applying) and
        // a cancel ✕; committed rows get the trash bin.
        let remove = if pending_add && idx == 0 {
            let done_ok = !alias.trim().is_empty()
                && !cmd.trim().is_empty()
                && !duplicate
                && !unknown_commands.contains(cmd.trim());
            row![
                kit::btn(
                    crate::ui::icons::themed_success(crate::ui::icons::CHECK, 12.0),
                    kit::Role::Ghost,
                    kit::Size::Compact,
                    if done_ok {
                        kit::Press::On(Message::AliasEditorDraftAccept)
                    } else {
                        kit::Press::Off
                    },
                    "",
                ),
                kit::btn(
                    crate::ui::icons::themed_secondary(crate::ui::icons::CLOSE, 12.0),
                    kit::Role::QuietDanger,
                    kit::Size::Compact,
                    kit::Press::On(Message::AliasEditorDraftCancel),
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
                kit::Press::On(Message::AliasEditorRemove(idx)),
                "",
            )]
            .align_y(iced::Center)
        };
        list = list.push(
            row![alias_box, cmd_box, remove]
                .spacing(tokens::GAP_ROW)
                .align_y(iced::Center),
        );
    }

    // While a draft row is pending, the add button doubles as the visible
    // cancel affordance (Esc works too).
    let add = if pending_add {
        plain_button(text(t!("Cancel add (Esc)")).size(tokens::TEXT_BODY))
            .on_press(Message::AliasEditorDraftCancel)
            .padding(kit::Size::Control.padding())
            .style(kit::danger_style)
    } else {
        plain_button(text(t!("+ Add alias")).size(tokens::TEXT_BODY))
            .on_press(Message::AliasEditorAdd)
            .padding(kit::Size::Control.padding())
            .style(kit::secondary_style)
    };
    // The reset confirmation replaces the alias count so the warning
    // gets the whole left side of the bar.
    let stats = if reset_confirm {
        row![Space::new().width(Length::Fixed(0.0))]
    } else {
        row![text(format!("{}: {}", t!("Number of aliases"), rows.len()))
            .size(tokens::TEXT_BODY)
            .style(muted_style)]
    };
    // Reset asks for confirmation in place: the add/reset buttons are
    // replaced by the question with Yes / No.
    let (reset_area, add_area) = if reset_confirm {
        (
            row![
                text(t!(
                    "Are you sure you want to reset? You will lose all your current aliases!"
                ))
                .size(tokens::TEXT_BODY)
                .style(|theme: &Theme| kit::tone_style(theme, kit::Tone::Danger)),
                plain_button(text(t!("Yes, reset")).size(tokens::TEXT_BODY))
                    .on_press(Message::AliasEditorResetConfirm)
                    .padding(kit::Size::Control.padding())
                    .style(kit::danger_style),
                plain_button(text(t!("No")).size(tokens::TEXT_BODY))
                    .on_press(Message::AliasEditorResetDeny)
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
                .on_press(Message::AliasEditorResetAsk)
                .padding(kit::Size::Control.padding())
                .style(kit::secondary_style)]
            .spacing(tokens::GAP_ROW)
            .align_y(iced::Center),
            row![add],
        )
    };
    // Apply — primary action; commits the rows to ocad.pgp and stays open.
    let apply = plain_button(text(t!("Apply")).size(tokens::TEXT_BODY))
        .on_press(Message::AliasEditorApply)
        .padding([tokens::GAP_TIGHT, tokens::GAP_GUTTER])
        .style(kit::primary_style);
    let apply_exit = plain_button(text(t!("Apply & Exit")).size(tokens::TEXT_BODY))
        .on_press(Message::AliasEditorApplyExit)
        .padding([tokens::GAP_TIGHT, tokens::GAP_GUTTER])
        .style(kit::primary_style);
    // The reset confirmation takes over the whole action bar.
    let apply_area = if reset_confirm {
        row![Space::new().width(Length::Fixed(0.0))]
    } else {
        row![apply, apply_exit].spacing(tokens::GAP_ROW).align_y(iced::Center)
    };

    // Persistent validation warnings: shown until every conflicting alias is
    // resolved (edited or a row removed) and every command is runnable.
    let mut conflict_banner = column![].spacing(tokens::GAP_HAIRLINE);
    for (alias, command) in duplicate_conflicts {
        conflict_banner = conflict_banner.push(
            text(crate::tf!(
                "Alias already used for command: {} → {}",
                alias,
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
    .on_press(Message::AliasEditorCloseKeep)
    .interaction(iced::mouse::Interaction::Idle);
    let panel = container(
        column![
            text(t!("Unsaved changes will be discarded.")).size(tokens::TEXT_HEADING),
            Space::new().height(10),
            row![
                plain_button(text(t!("Discard & close")).size(tokens::TEXT_BODY))
                    .on_press(Message::AliasEditorCloseDiscard)
                    .padding(kit::PAD_MENU_ROW)
                    .style(kit::danger_style),
                plain_button(text(t!("Keep editing")).size(tokens::TEXT_BODY))
                    .on_press(Message::AliasEditorCloseKeep)
                    .padding(kit::PAD_MENU_ROW)
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
