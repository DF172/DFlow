// Small platform shims: URL launching, file dialogs, crash logs.

/// Drawing hyperlinks may open web pages, never local files or custom handlers.
pub(crate) fn web_hyperlink(value: &str) -> Option<String> {
    if value.chars().any(char::is_control) {
        return None;
    }
    let url = url::Url::parse(value.trim()).ok()?;
    (matches!(url.scheme(), "http" | "https") && url.host_str().is_some()).then(|| url.into())
}

#[test]
fn drawing_hyperlinks_only_open_web_pages() {
    for value in ["https://example.com/path", " HTTP://example.com "] {
        assert!(web_hyperlink(value).is_some(), "{value}");
    }
    for value in [
        "",
        "https://",
        "javascript:alert(1)",
        "data:text/html,example",
        "file:///tmp/program.desktop",
        "/tmp/program.desktop",
        "custom:run",
        "mailto:user@example.com",
        "https://example.com/\npath",
    ] {
        assert!(web_hyperlink(value).is_none(), "{value}");
    }
}

/// Open a URL in the user's browser.
///
/// Wayland requires an xdg-activation token from the source window before it
/// will let an existing browser window take focus. Linux obtains that token
/// from the DraftFlow surface and passes it to xdg-open.
#[cfg(target_os = "linux")]
pub fn open_url<Message: Send + 'static>(
    url: &str,
    parent: Option<iced::window::Id>,
) -> iced::Task<Message> {
    let url = url.to_string();
    match parent {
        Some(parent) => iced::window::run(parent, move |window| {
            open_url_linux(&url, linux_activation_token(window));
        })
        .discard(),
        None => {
            let _ = open::that_detached(url);
            iced::Task::none()
        }
    }
}

#[cfg(target_os = "linux")]
fn linux_activation_token(window: &dyn iced::window::Window) -> Option<String> {
    use iced::window::raw_window_handle::{RawDisplayHandle, RawWindowHandle};

    let RawWindowHandle::Wayland(window_handle) = window.window_handle().ok()?.as_raw() else {
        return None;
    };
    let RawDisplayHandle::Wayland(display_handle) = window.display_handle().ok()?.as_raw() else {
        return None;
    };

    // Keep the iced window borrowed while ashpd uses its raw Wayland surface.
    // The returned token is an owned string and is immediately handed to the
    // child process.
    unsafe {
        iced::futures::executor::block_on(ashpd::ActivationToken::from_wayland_raw(
            None,
            window_handle.surface.as_ptr(),
            display_handle.display.as_ptr(),
        ))
    }
    .map(String::from)
}

#[cfg(target_os = "linux")]
fn open_url_linux(url: &str, activation_token: Option<String>) {
    let Some(token) = activation_token else {
        let _ = open::that_detached(url);
        return;
    };

    let mut command = std::process::Command::new("xdg-open");
    command
        .arg(url)
        .env("XDG_ACTIVATION_TOKEN", &token)
        .env("DESKTOP_STARTUP_ID", token);

    if let Ok(mut child) = command.spawn() {
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    } else {
        let _ = open::that_detached(url);
    }
}

#[cfg(not(target_os = "linux"))]
pub fn open_url<Message>(url: &str, _parent: Option<iced::window::Id>) -> iced::Task<Message> {
    let _ = open::that_detached(url);
    iced::Task::none()
}

/// Reveal a saved drawing in the native platform's file manager.
pub fn reveal_in_file_manager(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let status = std::process::Command::new("explorer.exe")
        .arg("/select,")
        .arg(path)
        .status();

    #[cfg(target_os = "macos")]
    let status = std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .status();

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let folder = path
            .parent()
            .ok_or_else(|| format!("Path has no parent folder: {}", path.display()))?;
        return open::that(folder).map_err(|e| e.to_string());
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!("File manager exited with {status}")),
        Err(error) => Err(error.to_string()),
    }
}

/// If the main application window is minimized, restore it quietly in the background
/// without activating it or stealing user focus, so viewport captures succeed.
#[cfg(target_os = "windows")]
pub fn restore_window_if_minimized() -> bool {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetForegroundWindow, GetWindowThreadProcessId, IsIconic,
        SetWindowPos, ShowWindow, HWND_BOTTOM, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        SW_SHOWNOACTIVATE,
    };
    #[allow(clippy::upper_case_acronyms)] // Win32 ABI type name
    type BOOL = i32;

    struct Context {
        pid: u32,
        restored: bool,
    }

    unsafe {
        let current_pid = GetCurrentProcessId();
        let mut ctx = Context {
            pid: current_pid,
            restored: false,
        };

        unsafe extern "system" fn enum_wnd(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let ctx = &mut *(lparam as *mut Context);
            let mut wnd_pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, &mut wnd_pid);
            if wnd_pid == ctx.pid {
                let mut class_buf = [0u16; 64];
                let len = GetClassNameW(hwnd, class_buf.as_mut_ptr(), 64);
                let class_str = String::from_utf16_lossy(&class_buf[..len as usize]);

                // CRITICAL FIX: Only touch the main application window ("Window Class").
                // NEVER touch "Winit Thread Event Target" or hidden helper windows!
                if class_str.starts_with("Window Class") && IsIconic(hwnd) != 0 {
                    let fg = GetForegroundWindow();
                    // Restore in background without activation (never steal user focus)
                    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                    // Ensure it stays behind active user applications
                    if fg != hwnd && !fg.is_null() {
                        SetWindowPos(
                            hwnd,
                            HWND_BOTTOM,
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                        );
                    }
                    ctx.restored = true;
                }
            }
            1
        }
        EnumWindows(Some(enum_wnd), (&mut ctx as *mut Context) as LPARAM);
        ctx.restored
    }
}

#[cfg(not(target_os = "windows"))]
pub fn restore_window_if_minimized() -> bool {
    false
}

/// Turn an `rfd` file handle into a `PathBuf` the rest of the app keys on.
pub fn handle_path(h: &rfd::FileHandle) -> std::path::PathBuf {
    let p = h.path().to_path_buf();
    // Every dialog result funnels through here — remember its folder so the
    // next dialog opens where the user left off.
    crate::config::remember_dialog_dir(&p);
    p
}

/// New async file dialog seeded with the last directory a dialog was used in.
/// All pickers should start from this instead of `rfd::AsyncFileDialog::new()`.
pub fn file_dialog() -> rfd::AsyncFileDialog {
    let dlg = rfd::AsyncFileDialog::new();
    match crate::config::last_dialog_dir() {
        Some(dir) => dlg.set_directory(dir),
        None => dlg.set_directory("D:/"),
    }
}

/// Blocking native file dialog seeded with the last-used directory.
///
/// Use this through `iced::window::run` when the picker needs the main
/// window's raw handle. Portal-based desktops can otherwise reject or lose a
/// parentless save request without ever showing a dialog (#537).
pub fn blocking_file_dialog() -> rfd::FileDialog {
    let dlg = rfd::FileDialog::new();
    match crate::config::last_dialog_dir() {
        Some(dir) => dlg.set_directory(dir),
        None => dlg,
    }
}

// The live preview overlay's Download-button closure. The closure captures
// a full copy of the PDF bytes, so `forget()`-ing it would leak one PDF per
// preview; holding it here lets the next `show_pdf_preview` drop and free
// the previous one (the overlay element's removal already released the JS
// handler reference). A thread-local `RefCell` rather than a static
// `Mutex` like `preview_urls`: `Closure` is not `Send`, and the overlay
// only ever lives on the one UI thread.

/// Short platform string for bug reports: OS + architecture on the desktop,
/// the browser user-agent on the web.
pub fn platform_info() -> String {
    format!("{} {}", std::env::consts::OS, std::env::consts::ARCH)
}

/// Desktop crash log.
///
/// A release build hides the console (`windows_subsystem = "windows"`) and is
/// stripped, so a panic used to end the process with nothing on screen, nothing
/// on stderr and nothing on disk: the application simply vanished. That is what
/// every "it just closes" report has had to work from (#635, #845), and why a
/// crash like the TRIM one (#830) could be reported for months with no detail
/// beyond the steps.
///
/// Every panic now leaves one small file behind naming what failed and where.
/// The hook chains onto whatever was installed before it, so a debug build
/// still prints to stderr as well.
pub mod crash_log {
    use std::path::PathBuf;

    /// Newest reports to keep. A crash loop must not fill the user's profile,
    /// and the first report of a loop is the interesting one, so keep both
    /// ends by pruning only what is older than this.
    const KEEP: usize = 20;

    /// Where reports go, beside the recovery logs.
    pub fn directory() -> Option<PathBuf> {
        crate::config::config_dir().map(|path| path.join("crash_logs"))
    }

    /// The report a given process left behind, if it left one.
    ///
    /// Reports are named for the process that wrote them, so a caller holding
    /// a process id from somewhere else — the GPU crash sentinel does — can
    /// ask what killed that exact run instead of guessing from the newest
    /// file, which on a busy machine may belong to a different one.
    ///
    /// Process ids are reused, so only a report written at or after
    /// `not_before` (Unix seconds — when that run started) counts: an old
    /// report from an unrelated run that once had the same id does not.
    pub fn report_for_pid(pid: u32, not_before: u64) -> Option<String> {
        let suffix = format!("-{pid}.log");
        for directory in read_directories() {
            let entry = match std::fs::read_dir(&directory) {
                Ok(entries) => entries.flatten().find(|entry| {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    name.strip_prefix("crash-")
                        .and_then(|rest| rest.strip_suffix(suffix.as_str()))
                        .and_then(|when| when.parse::<u64>().ok())
                        .is_some_and(|when| when >= not_before)
                }),
                Err(_) => None,
            };
            let Some(entry) = entry else {
                continue;
            };
            if let Ok(report) = std::fs::read_to_string(entry.path()) {
                return Some(report);
            }
        }
        None
    }

    /// Whether a report describes the graphics device giving up.
    ///
    /// Deliberately narrow. Anything that reaches wgpu — an exhausted device,
    /// a lost one, a buffer that was never created — is the backend's
    /// business; a panic in the drawing code that happens to run on a frame
    /// is not, and must not cost the user a working backend.
    pub fn is_device_failure(report: &str) -> bool {
        let mut panic_line = "";
        let mut at_line = "";
        for line in report.lines() {
            if let Some(rest) = line.strip_prefix("panic: ") {
                panic_line = rest;
            } else if let Some(rest) = line.strip_prefix("at: ") {
                at_line = rest;
            }
        }
        // The message wgpu panics with names the call, and the location names
        // wgpu itself; either alone is enough, since a stripped build can
        // leave the message terse. Plain words like "buffer" or "surface"
        // are not: drawing code and the modelling kernel panic with those
        // too, and that is no reason to give up a working backend.
        let haystack = format!("{panic_line} {at_line}").to_ascii_lowercase();
        ["wgpu", "out of memory", "device lost", "validation error"]
            .iter()
            .any(|needle| haystack.contains(needle))
    }

    /// The backend a report says was live, as written by [`report_from`].
    pub fn backend_in_report(report: &str) -> Option<String> {
        report
            .lines()
            .find_map(|line| line.strip_prefix("gpu backend: "))
            .map(str::trim)
            .filter(|backend| !backend.is_empty() && *backend != "(default)")
            .map(str::to_string)
    }

    /// Chain a report writer onto the current panic hook.
    ///
    /// Call once, early: a panic before this runs is still silent.
    pub fn install() {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            // A panic while reporting a panic must not replace the original
            // with a recursion, so failures here are dropped on purpose.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| write(info)));
            previous(info);
        }));
    }

    /// The report body. Separated from the IO so a test can read it.
    pub fn report(info: &std::panic::PanicHookInfo<'_>, when: u64) -> String {
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "(no message)".to_string());
        let where_ = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "(unknown location)".to_string());
        report_from(&payload, &where_, when)
    }

    /// The report body, with the two pieces a panic carries already pulled
    /// out. Separate so a test can exercise the layout without installing a
    /// panic hook: the hook is process-global, and a test that asserts inside
    /// one fires on every other test's panic and aborts the whole binary.
    pub fn report_from(payload: &str, where_: &str, when: u64) -> String {
        // `WGPU_BACKEND` is what the backend resolver settled on, so the
        // report says which graphics path was live without reaching into the
        // renderer from a panic handler.
        let backend = std::env::var("WGPU_BACKEND").unwrap_or_else(|_| "(default)".to_string());
        // `force_capture`, not `capture`: a report whose backtrace says
        // "disabled backtrace" because the user never set RUST_BACKTRACE is
        // exactly the report that helps nobody. Release builds are stripped,
        // so some frames are addresses, but the frame count and the module
        // boundaries still place the fault.
        let backtrace = std::backtrace::Backtrace::force_capture();
        [
            format!(
                "DraftFlow CAD {} crash report",
                env!("DRAFTFLOW_APP_VERSION")
            ),
            format!("when: {when} (unix seconds)"),
            format!("os: {} {}", std::env::consts::OS, std::env::consts::ARCH),
            format!("gpu backend: {backend}"),
            format!(
                "thread: {}",
                std::thread::current().name().unwrap_or("unnamed")
            ),
            format!("panic: {payload}"),
            format!("at: {where_}"),
            String::new(),
            "backtrace:".to_string(),
            backtrace.to_string(),
            String::new(),
        ]
        .join("\n")
    }

    fn write(info: &std::panic::PanicHookInfo<'_>) {
        let when = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let name = format!("crash-{when}-{}.log", std::process::id());
        let text = report(info, when);
        let landed = write_to_first(&report_directories(), &name, &text);
        // A report that reached no disk is the only case where the panic leaves
        // this process with nothing in it, so the body goes to stderr instead.
        // Without this the user loses the crash evidence and is never told that
        // it was lost — which is what the whole file exists to prevent.
        match landed {
            Some(path) => {
                eprintln!(
                    "DraftFlow CAD crashed. Report written to {}",
                    path.display()
                );
            }
            None => {
                eprintln!("DraftFlow CAD crashed and could not write its crash report.\n{text}");
            }
        }
    }

    /// Try each folder in order and keep the first report that lands, so the
    /// caller knows whether the evidence exists on disk or only on stderr.
    fn write_to_first(directories: &[PathBuf], name: &str, text: &str) -> Option<PathBuf> {
        for directory in directories {
            let path = directory.join(name);
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create(true).truncate(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            if let Ok(mut file) = options.open(&path) {
                use std::io::Write;
                if file.write_all(text.as_bytes()).is_ok() {
                    prune(directory);
                    return Some(path);
                }
            }
        }
        None
    }

    /// Where a report may be read from: the profile's folder, then the OS
    /// scratch folder the writer falls back to when the profile refused.
    fn read_directories() -> Vec<PathBuf> {
        let mut directories: Vec<PathBuf> = Vec::new();
        if let Some(profile) = directory() {
            directories.push(profile);
        }
        let scratch = std::env::temp_dir();
        if !directories.contains(&scratch) {
            directories.push(scratch);
        }
        directories
    }

    /// Where a report may be written to — the readable folders, minus any that
    /// cannot be created.
    fn report_directories() -> Vec<PathBuf> {
        read_directories()
            .into_iter()
            .filter(|directory| std::fs::create_dir_all(directory).is_ok())
            .collect()
    }

    /// Keep the newest [`KEEP`] reports.
    ///
    /// Only files this module names are candidates: the fallback folder is the
    /// shared OS scratch dir, and pruning every older `.log` there would delete
    /// other applications' files.
    fn prune(directory: &std::path::Path) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        let mut files: Vec<(std::time::SystemTime, PathBuf)> = entries
            .flatten()
            .filter(|e| {
                let name = e
                    .path()
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned());
                name.is_some_and(|n| n.starts_with("crash-") && n.ends_with(".log"))
            })
            .filter_map(|e| {
                let modified = e.metadata().ok()?.modified().ok()?;
                Some((modified, e.path()))
            })
            .collect();
        if files.len() <= KEEP {
            return;
        }
        files.sort_by_key(|(when, _)| *when);
        for (_, path) in &files[..files.len() - KEEP] {
            let _ = std::fs::remove_file(path);
        }
    }

    #[cfg(test)]
    mod tests {
        use std::path::PathBuf;

        /// A profile folder that refuses the report (no `%APPDATA%`, a read-only
        /// roaming folder, a full disk) used to swallow the crash evidence and
        /// say nothing at all. The scratch folder is the second door.
        #[test]
        fn a_refused_profile_falls_back_to_the_scratch_folder() {
            let scratch = std::env::temp_dir();
            let blocked = scratch.join("draftflow-crash-blocked-file");
            std::fs::write(&blocked, b"not a folder").unwrap();

            let landed = super::write_to_first(
                &[blocked.clone(), scratch.clone()],
                "crash-guard-test.log",
                "panic: from a guard",
            )
            .expect("the second folder takes the report");

            assert_eq!(landed.parent().unwrap(), scratch);
            assert_eq!(
                std::fs::read_to_string(&landed).unwrap(),
                "panic: from a guard"
            );
            let _ = std::fs::remove_file(&landed);
            let _ = std::fs::remove_file(&blocked);
        }

        /// And when every folder refuses, the answer is `None` — which is what
        /// makes the hook put the body on stderr instead of pretending the
        /// evidence exists.
        #[test]
        fn a_report_with_nowhere_to_land_says_so() {
            let scratch = std::env::temp_dir();
            let blocked = scratch.join("draftflow-crash-blocked-file-2");
            std::fs::write(&blocked, b"not a folder").unwrap();

            let nowhere = [
                blocked.clone(),
                PathBuf::from(blocked.display().to_string()).join("deeper"),
            ];

            assert!(
                super::write_to_first(&nowhere, "crash-guard-test.log", "body").is_none(),
                "nothing may be claimed as written when no folder took it"
            );
            let _ = std::fs::remove_file(&blocked);
        }

        /// The report has to carry the two things a maintainer cannot guess:
        /// what the message was and where it came from. It also has to be
        /// readable — an earlier version built it with `\` continuations and
        /// pasted the source indentation into every line.
        #[test]
        fn a_report_names_the_message_and_the_line() {
            let text = super::report_from("a deliberate test panic", "src/sys.rs:1:2", 42);
            assert!(text.starts_with("DraftFlow CAD "), "{text}");
            assert!(text.contains("panic: a deliberate test panic"), "{text}");
            assert!(text.contains("at: src/sys.rs:1:2"), "{text}");
            assert!(text.contains("when: 42 (unix seconds)"), "{text}");
            let (header, trace) = text
                .split_once("\nbacktrace:\n")
                .expect("the report carries a backtrace section");
            assert!(!trace.trim().is_empty(), "the backtrace must not be empty");
            // Header only: a backtrace indents its own continuation lines,
            // and that is how a backtrace is meant to read.
            assert!(
                header.lines().all(|line| !line.starts_with(' ')),
                "header lines must not be indented:\n{header}"
            );
        }

        /// Only the device's own failures may cost a backend. A panic in
        /// drawing code that merely happened to run on a frame must not.
        #[test]
        fn a_device_failure_is_told_apart_from_an_ordinary_panic() {
            let device = super::report_from(
                "Error in Buffer::get_mapped_range: Validation Error",
                "wgpu-29.0.4/src/backend/wgpu_core.rs:2253:18",
                1,
            );
            assert!(super::is_device_failure(&device), "{device}");

            let oom = super::report_from("Out of Memory", "src/scene/pipeline/mod.rs:10:1", 1);
            assert!(super::is_device_failure(&oom), "{oom}");

            let ordinary = super::report_from(
                "index out of bounds: the len is 3 but the index is 7",
                "src/app/commands/draw.rs:120:5",
                1,
            );
            assert!(
                !super::is_device_failure(&ordinary),
                "an indexing bug is not the graphics device's fault:
{ordinary}"
            );
        }

        /// The backend line is what the sentinel joins on, so it has to come
        /// back out — and "(default)" names nothing to blame.
        #[test]
        fn the_backend_is_read_back_out_of_a_report() {
            // SAFETY: single-threaded test mutating a process-local variable.
            unsafe { std::env::set_var("WGPU_BACKEND", "dx12") };
            let named = super::report_from("boom", "x.rs:1:1", 0);
            assert_eq!(super::backend_in_report(&named).as_deref(), Some("dx12"));

            unsafe { std::env::remove_var("WGPU_BACKEND") };
            let unnamed = super::report_from("boom", "x.rs:1:1", 0);
            assert_eq!(super::backend_in_report(&unnamed), None);
        }

        /// A panic with no message still has to produce a filed report rather
        /// than nothing at all.
        #[test]
        fn a_report_survives_a_panic_with_nothing_to_say() {
            let text = super::report_from("(no message)", "(unknown location)", 0);
            assert!(text.contains("panic: (no message)"), "{text}");
            assert!(text.contains("at: (unknown location)"), "{text}");
        }
    }
}
