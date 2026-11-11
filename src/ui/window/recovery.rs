use crate::ui::style::tokens;
use crate::app::Message;
use crate::io::recovery::{RecoveryReport, RecoveryStatus};
use crate::ui::kit::muted_style;
use iced::widget::{column, container, row, text, Space};
use crate::ui::kit::plain_button;
use iced::{Element, Fill, Length, Shrink, Theme};
use crate::ui::kit;

fn status_style(status: RecoveryStatus) -> impl Fn(&Theme) -> iced::widget::text::Style + Copy {
    move |theme: &Theme| {
        kit::color_style(match status {
            RecoveryStatus::Recovered => theme.palette().warning.base.color,
            RecoveryStatus::Failed => theme.palette().danger.base.color,
        })
    }
}

fn metric<'a>(label: String, value: String) -> Element<'a, Message> {
    container(
        column![
            text(label).size(tokens::TEXT_MICRO).style(muted_style),
            text(value).size(tokens::TEXT_FOCAL),
        ]
        .spacing(tokens::GAP_TIGHT),
    )
    .padding(kit::PAD_CARD)
    .width(Fill)
    .style(kit::pane_plate_style)
    .into()
}

pub fn view_window<'a>(
    report: &'a RecoveryReport,
    allow_save_copy: bool,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let recovered = report.status == RecoveryStatus::Recovered;
    let heading = if recovered {
        crate::tr!("recovery", "opened-with-repairs")
    } else {
        crate::tr!("recovery", "open-failed")
    };
    let description = if recovered {
        crate::tr!("recovery", "repaired-description")
    } else {
        crate::tr!("recovery", "failed-description")
    };
    let status = report.status;

    let metrics = row![
        metric(
            crate::tr!("recovery", "entities-checked"),
            report.entities_scanned.to_string(),
        ),
        metric(
            crate::tr!("recovery", "issues-found"),
            report.issues_found().to_string(),
        ),
        metric(
            crate::tr!("recovery", "entities-removed"),
            report.removed_total().to_string(),
        ),
        metric(
            crate::tr!("recovery", "references-checked"),
            report.references_checked.to_string(),
        ),
    ]
    .spacing(tokens::GAP_ROW)
    .width(Fill);

    let mut details = column![].spacing(tokens::GAP_CONTROL);
    if report.referenced_entities_removed > 0 {
        details = details.push(
            text(format!(
                "{}: {}",
                crate::tr!("recovery", "referenced-entities-removed"),
                report.referenced_entities_removed
            ))
            .size(tokens::TEXT_LABEL),
        );
    }
    if report.references_missing > 0 || report.references_failed > 0 {
        details = details.push(
            text(format!(
                "{}: {}",
                crate::tr!("recovery", "references-unavailable"),
                report
                    .references_missing
                    .saturating_add(report.references_failed)
            ))
            .size(tokens::TEXT_LABEL),
        );
    }
    if let Some(error) = &report.error {
        details = details.push(text(error).size(tokens::TEXT_LABEL));
    }
    for (kind, message) in report.diagnostics.iter().take(100) {
        details = details.push(text(format!("[{kind}] {message}")).size(tokens::TEXT_MICRO));
    }
    if let Some(path) = &report.log_path {
        details = details.push(
            text(format!(
                "{}: {}",
                crate::tr!("recovery", "log-path"),
                path.display()
            ))
            .size(tokens::TEXT_MICRO)
            .style(muted_style),
        );
    } else if let Some(error) = &report.log_error {
        details = details.push(
            text(format!(
                "{}: {error}",
                crate::tr!("recovery", "log-write-failed")
            ))
            .size(tokens::TEXT_MICRO)
            .style(status_style(RecoveryStatus::Failed)),
        );
    } else if cfg!(target_arch = "wasm32") {
        details = details.push(
            text(crate::tr!("recovery", "log-download-ready"))
                .size(tokens::TEXT_MICRO)
                .style(muted_style),
        );
    }

    let detail_panel = container(crate::ui::kit::themed_scrollable(details).height(Fill))
        .padding(kit::PAD_CARD)
        .width(Fill)
        .height(Fill)
        .style(|theme: &Theme| kit::Tier::Well.style(theme, ""));

    let mut actions = row![Space::new().width(Fill)].spacing(tokens::GAP_ROW);
    if recovered && report.save_as_required && allow_save_copy {
        actions = actions.push(
            plain_button(text(crate::tr!("recovery", "save-copy")).size(tokens::TEXT_BODY))
                .on_press(Message::RecoverySaveAs)
                .style(kit::primary_style)
                .padding([tokens::GAP_CONTROL, tokens::GAP_SECTION]),
        );
    }
    if report.log_path.is_some() || cfg!(target_arch = "wasm32") {
        actions = actions.push(
            plain_button(text(crate::tr!("recovery", "show-log")).size(tokens::TEXT_BODY))
                .on_press(Message::RecoveryShowLog)
                .style(kit::secondary_style)
                .padding([tokens::GAP_CONTROL, tokens::GAP_SECTION]),
        );
    }
    actions = actions.push(
        plain_button(text(crate::tr!("action", "close")).size(tokens::TEXT_BODY))
            .on_press(Message::RecoveryClose)
            .style(kit::secondary_style)
            .padding([tokens::GAP_CONTROL, tokens::GAP_SECTION]),
    );

    container(
        column![
            text(heading).size(tokens::TEXT_TITLE).style(status_style(status)),
            text(&report.file_name).size(tokens::TEXT_META),
            text(description).size(tokens::TEXT_LABEL).style(muted_style),
            Space::new().height(4),
            metrics,
            detail_panel,
            actions,
        ]
        .spacing(tokens::GAP_FIELD)
        .width(sizing.width)
        .height(sizing.height),
    )
    .padding([tokens::GAP_SECTION, tokens::GAP_GUTTER])
    .width(sizing.width)
    .height(sizing.height)
    .into()
}

pub fn view_prompt<'a>(
    file_name: &'a str,
    error: &'a str,
    sizing: crate::ui::shell::Sizing,
) -> Element<'a, Message> {
    let content_width = if matches!(sizing.width, Length::Fill) {
        Fill
    } else {
        Shrink
    };
    let content_height = if matches!(sizing.height, Length::Fill) {
        Fill
    } else {
        Shrink
    };
    let actions = row![
        Space::new().width(content_width),
        plain_button(text(crate::tr!("recovery", "decline")).size(tokens::TEXT_BODY))
            .on_press(Message::RecoveryDecline)
            .style(kit::secondary_style)
            .padding([tokens::GAP_CONTROL, tokens::GAP_SECTION]),
        plain_button(text(crate::tr!("recovery", "attempt")).size(tokens::TEXT_BODY))
            .on_press(Message::RecoveryAttempt)
            .style(kit::primary_style)
            .padding([tokens::GAP_CONTROL, tokens::GAP_SECTION]),
    ]
    .spacing(tokens::GAP_ROW);

    container(
        column![
            text(crate::tr!("recovery", "prompt-heading")).size(tokens::TEXT_TITLE),
            text(file_name).size(tokens::TEXT_META),
            text(crate::tr!("recovery", "prompt-description"))
                .size(tokens::TEXT_LABEL)
                .style(muted_style),
            container(
                crate::ui::kit::themed_scrollable(crate::ui::kit::feedback::line(
                    crate::ui::kit::Tone::Danger,
                    error,
                ))
                .height(content_height),
            )
                .padding(kit::PAD_CARD)
                .width(content_width)
                .height(content_height)
                .style(|theme: &Theme| kit::Tier::Well.style(theme, "")),
            actions,
        ]
        .spacing(tokens::GAP_FIELD)
        .width(sizing.width)
        .height(sizing.height),
    )
    .padding([tokens::GAP_SECTION, tokens::GAP_GUTTER])
    .into()
}
