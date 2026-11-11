use crate::app::Message;
use crate::t;
use crate::ui::kit;
use crate::ui::kit::muted_style;
use crate::ui::style::tokens;
use iced::widget::{column, row, svg, text, Space};
use iced::{Element, Fill, Length, Theme};

const LICENSE: &str = "GPL-3.0-only";

/// A label/value line inside the About page.
fn info_row<'a>(
    label: std::borrow::Cow<'static, str>,
    value: impl iced::widget::text::IntoFragment<'a>,
) -> Element<'a, Message> {
    iced::widget::container(
        row![
            text(label)
                .size(tokens::TEXT_LABEL)
                .style(muted_style)
                .width(Length::Fixed(110.0)),
            text(value).size(tokens::TEXT_META).width(Fill),
        ]
        .spacing(tokens::GAP_ROW)
        .align_y(iced::Center),
    )
    .padding([tokens::GAP_ROW, tokens::GAP_SECTION])
    .width(Fill)
    .style(kit::inset_card_style)
    .into()
}

pub(crate) fn platform_name() -> &'static str {
    #[cfg(target_arch = "wasm32")]
    {
        "Web"
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        match std::env::consts::OS {
            "linux" => "Linux",
            "windows" => "Windows",
            "macos" => "macOS",
            other => other,
        }
    }
}

pub(crate) fn architecture_name() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x86-64",
        "aarch64" => "ARM64",
        "wasm32" => "WebAssembly 32-bit",
        other => other,
    }
}

/// The text `Copy Info` puts on the clipboard: everything a bug report needs
/// about the build and the machine.
pub(crate) fn report() -> String {
    use crate::sysinfo::{format_bytes, system_info};
    let sys = system_info();
    #[allow(unused_mut)]
    let mut out = format!(
        "DraftFlow CAD v{}\nRevision: {}\nCommit date: {}\nBuild date: {}\nProfile: {}\nFeatures: {}\n\n\
         [System]\nOS: {} ({} {})\nCPU: {}\nLogical processors: {}\nRAM: {} total, {} available\n",
        env!("DRAFTFLOW_APP_VERSION"),
        env!("DRAFTFLOW_GIT_REV"),
        env!("DRAFTFLOW_COMMIT_DATE"),
        env!("DRAFTFLOW_BUILD_DATE"),
        env!("DRAFTFLOW_BUILD_PROFILE"),
        env!("DRAFTFLOW_BUILD_FEATURES"),
        sys.os,
        platform_name(),
        architecture_name(),
        sys.cpu,
        sys.logical_cores,
        format_bytes(sys.ram_total),
        format_bytes(sys.ram_available),
    );
    #[cfg(not(target_arch = "wasm32"))]
    {
        use crate::gpu_backend::{active_gpu, load_prefs};
        let gpu = active_gpu();
        let prefs = load_prefs();
        let or_unknown = |s: &str| {
            if s.is_empty() {
                "Unknown".to_string()
            } else {
                s.to_string()
            }
        };
        out.push_str(&format!(
            "\n[Graphics]\nGPU: {}\nDriver: {}\nBackend: {}\nSelected by: {}\n\
             Below WebGPU baseline: {}\nWGPU_BACKEND: {}\n\
             Saved options: backend={}, compat renderer={}, OpenGL on older GPUs={}\n",
            or_unknown(gpu.adapter.as_deref().unwrap_or("")),
            or_unknown(&gpu.driver),
            or_unknown(gpu.backend.as_deref().unwrap_or("")),
            or_unknown(&gpu.origin),
            if gpu.legacy { "yes" } else { "no" },
            std::env::var("WGPU_BACKEND").unwrap_or_else(|_| "(unset)".into()),
            prefs.backend.as_str().unwrap_or("auto"),
            prefs.compat_renderer,
            prefs.legacy_gl,
        ));
    }
    #[cfg(target_arch = "wasm32")]
    {
        out.push_str("\n[Graphics]\nBackend: WebGPU / WebGL\n");
    }
    out.push_str(&format!("\n[Legal]\nLicense: {}\n", LICENSE));
    out
}

/// The About page inside Options: identity, build and licence.
pub fn view_page() -> Element<'static, Message> {
    let icon = svg(svg::Handle::from_memory(include_bytes!(
        "../../../assets/appicon.svg"
    )))
    .width(Length::Fixed(96.0))
    .height(Length::Fixed(96.0));

    let identity = column![
        icon,
        text("DraftFlow CAD")
            .size(tokens::TEXT_HERO)
            .style(|theme: &Theme| kit::color_style(theme.palette().primary.base.color)),
        text(t!("CAD application for Architecture & Engineering"))
            .size(tokens::TEXT_LABEL)
            .style(muted_style),
    ]
    .spacing(tokens::GAP_CONTROL)
    .align_x(iced::Center)
    .width(Fill);

    let details = column![
        info_row(t!("Version"), env!("DRAFTFLOW_APP_VERSION")),
        info_row(t!("Build"), env!("DRAFTFLOW_BUILD_DATE")),
        info_row(t!("License"), LICENSE),
    ]
    .spacing(tokens::GAP_CONTROL)
    .width(Length::Fixed(420.0));

    column![
        Space::new().height(Length::Fixed(40.0)),
        identity,
        Space::new().height(Length::Fixed(28.0)),
        details,
        Space::new().height(Length::Fixed(24.0)),
        kit::dialog_button(t!("Copy Info"), Message::AboutCopyInfo, true),
    ]
    .align_x(iced::Center)
    .width(Fill)
    .into()
}
