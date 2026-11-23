//! "Check for Updates" against the GitHub Releases API, plus the in-place
//! self-update the startup check offers.
//!
//! Nothing here runs on its own — there is no background polling and no
//! telemetry. The two callers both start from a user action: the About page's
//! button, and the boot self-check that only asks once per launch. The only
//! network traffic this module produces is the request those ask for.
//!
//! Everything above [`Fetcher`] is pure: tag parsing, numeric comparison, the
//! release JSON and the executable swap are tested without a socket, and the
//! shipping HTTP client ([`HttpFetcher`]) is a thin shell over `ureq`.
//!
//! ## How the in-place update works
//!
//! A release carries the Inno Setup package
//! (`DraftFlowCAD-<version>-x64-setup.exe`) and that is the published channel:
//!
//! * **Setup package** — the download is run silently with
//!   `/DIR="<folder of the running exe>"`, so it lands on the copy the user
//!   already has instead of a default location. The installer keeps the same
//!   `AppId`, which is what makes it an upgrade of the existing entry rather
//!   than a second installation: same folder, same shortcuts, one uninstall
//!   record. A detached shell waits for this process to exit, runs the
//!   installer, starts the new executable, then deletes the download.
//! * **Bare executable** — kept for a release that carries no setup package. A
//!   running Windows executable cannot be deleted or overwritten, but it can be
//!   *renamed*, so the swap is: rename `DraftFlowCAD.exe` to
//!   `DraftFlowCAD.exe.old-<stamp>`, put the verified download at the original
//!   name, then close the app and let a detached shell start the new file a few
//!   seconds later. The old name stays on disk as the rollback, and a failed
//!   copy restores it before the error is reported.
//!
//! The setup package wins when a release publishes both.

use std::cmp::Ordering;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

/// `releases/latest` of the project repository. That endpoint excludes drafts
/// and pre-releases, which is exactly the version a user should be offered.
pub const RELEASES_API: &str = "https://api.github.com/repos/DF172/DFlow/releases/latest";

/// Opened by "Download" when the reply carries no usable `html_url`.
pub const RELEASES_PAGE: &str = "https://github.com/DF172/DFlow/releases/latest";

/// Why a check or an install ended without an answer. The kinds get different
/// copy: a connection problem is worth retrying, an unusable reply is not, a
/// rejected file means the bytes on disk are wrong, and an install failure
/// means the program folder refused the write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// The request itself failed: offline, DNS, TLS, timeout, HTTP status.
    Network,
    /// A reply arrived but carried no release metadata we could read.
    Unreadable,
    /// The executable could not be fetched.
    Download,
    /// The bytes did not match the published size or digest.
    Integrity,
    /// The program folder refused the swap (no permission, missing file).
    Install,
}

/// What kind of file an [`Asset`] is. The two take different install routes: a
/// setup package runs the Inno installer against the current folder, a bare
/// executable is swapped in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    Exe,
    Setup,
}

/// One downloadable file from the release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    /// Zero when the reply carried no size.
    pub size: u64,
    pub url: String,
    /// Lowercase hex from the reply's `sha256:` digest, when it published one.
    pub sha256: Option<String>,
    pub kind: AssetKind,
}

/// The latest release as read off the API reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// Normalized numeric version, e.g. `1.2.0` from tag `v1.2.0`.
    pub version: String,
    /// Browser target for "Download".
    pub url: String,
    /// The bare executable, when the release publishes one.
    pub exe: Option<Asset>,
    /// The Inno Setup package, when the release publishes one.
    pub setup: Option<Asset>,
}

impl Release {
    /// The file an upgrade runs. The setup package wins when a release carries
    /// both: it is the published channel, and it also refreshes shortcuts and
    /// file associations that a bare executable swap leaves alone.
    pub fn installer(&self) -> Option<Asset> {
        self.setup.clone().or_else(|| self.exe.clone())
    }
}

/// What one check produced. Carried by the finishing message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    UpToDate {
        latest: String,
    },
    Available {
        version: String,
        url: String,
        /// `None` when the release has nothing the app can install itself.
        installer: Option<Asset>,
    },
    Failed(Failure),
}

/// Which part of the install the worker is in. The dialog names the stage, so
/// a slow download and a stuck copy never share one indistinguishable spinner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusyStage {
    Downloading,
    Verifying,
    Installing,
}

/// What one install pass produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallOutcome {
    /// Swapped in. `relaunch` is false when the replacement could not be
    /// scheduled — the update is on disk, but the user has to start it.
    Installed {
        relaunch: bool,
    },
    Failed(Failure),
    /// The user asked to stop before the swap; nothing on disk changed.
    Cancelled,
}

/// The update dialog's whole state machine: idle, in flight, and the outcomes.
/// Kept separate from the dialog so the About page can also read it without
/// the view being open.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum CheckState {
    #[default]
    Idle,
    Checking,
    UpToDate {
        latest: String,
    },
    Available {
        version: String,
        url: String,
        installer: Option<Asset>,
    },
    /// The upgrade was refused because a drawing has unsaved changes.
    Blocked {
        version: String,
        url: String,
        installer: Option<Asset>,
    },
    /// Download, verification or the swap itself is running.
    Busy {
        version: String,
        url: String,
        asset: Asset,
        stage: BusyStage,
        done: u64,
        total: Option<u64>,
    },
    /// The new executable is in place.
    Installed {
        version: String,
        relaunch: bool,
    },
    Failed(Failure),
}

impl CheckState {
    /// Folds a finished check into the dialog state.
    pub fn apply(&mut self, outcome: Outcome) {
        *self = match outcome {
            Outcome::UpToDate { latest } => CheckState::UpToDate { latest },
            Outcome::Available {
                version,
                url,
                installer,
            } => CheckState::Available {
                version,
                url,
                installer,
            },
            Outcome::Failed(failure) => CheckState::Failed(failure),
        };
    }

    /// True when the state names a newer release the user has not yet acted
    /// on. The startup check uses this to decide whether to raise the dialog.
    pub fn offers_newer_version(&self) -> bool {
        matches!(self, CheckState::Available { .. })
    }
}

/// Fetches the release metadata body. A trait so the tests inject canned
/// replies; the shipping implementation is [`HttpFetcher`].
pub trait Fetcher {
    fn fetch(&self, url: &str) -> Result<String, String>;
}

/// Runs one check against `fetcher`, comparing with the running `current`
/// version.
pub fn check_with(fetcher: &dyn Fetcher, current: &str) -> Outcome {
    let body = match fetcher.fetch(RELEASES_API) {
        Ok(body) => body,
        Err(error) => {
            log::warn!("update check request failed: {error}");
            return Outcome::Failed(Failure::Network);
        }
    };
    let release = match parse_release_json(&body) {
        Ok(release) => release,
        Err(reason) => {
            log::warn!("update check reply unusable: {reason}");
            return Outcome::Failed(Failure::Unreadable);
        }
    };
    if is_newer(&release.version, current) {
        let installer = release.installer();
        Outcome::Available {
            version: release.version,
            url: release.url,
            installer,
        }
    } else {
        Outcome::UpToDate {
            latest: release.version,
        }
    }
}

/// Parses `{"tag_name": "v1.2.0", "html_url": "…", "assets": [...]}`.
///
/// Only four fields are read; the rest of what GitHub sends (notes, uploader
/// …) is ignored, so a richer reply never breaks the check.
pub fn parse_release_json(body: &str) -> Result<Release, &'static str> {
    #[derive(serde::Deserialize)]
    struct Reply {
        tag_name: Option<String>,
        html_url: Option<String>,
        #[serde(default)]
        assets: Vec<AssetReply>,
    }

    #[derive(serde::Deserialize)]
    struct AssetReply {
        name: Option<String>,
        size: Option<u64>,
        browser_download_url: Option<String>,
        digest: Option<String>,
    }

    let reply: Reply = serde_json::from_str(body).map_err(|_| "not a JSON object")?;
    let tag = reply.tag_name.ok_or("no tag_name")?;
    let version = normalize(&tag).ok_or("tag_name is not a version")?;
    let url = match reply.html_url {
        Some(url) if url.starts_with("http") => url,
        _ => RELEASES_PAGE.to_string(),
    };
    let mut exe = None;
    let mut setup = None;
    for asset in reply.assets {
        let (Some(name), Some(url)) = (asset.name, asset.browser_download_url) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        // The name decides both the verification and the install route, so it
        // is read before the asset is built rather than inferred later. A
        // published setup package is named `…-setup.exe`.
        let kind = if !lower.ends_with(".exe") {
            continue;
        } else if lower.contains("setup") {
            AssetKind::Setup
        } else {
            AssetKind::Exe
        };
        let candidate = Asset {
            name,
            size: asset.size.unwrap_or(0),
            url,
            sha256: asset.digest.as_deref().and_then(sha256_from_digest),
            kind,
        };
        match kind {
            AssetKind::Exe if exe.is_none() => exe = Some(candidate),
            AssetKind::Setup if setup.is_none() => setup = Some(candidate),
            _ => {}
        }
    }
    Ok(Release {
        version,
        url,
        exe,
        setup,
    })
}

/// The hex half of a GitHub `sha256:<hex>` digest, lowercased. Anything else —
/// a future algorithm, a truncated value — is treated as "no digest".
fn sha256_from_digest(digest: &str) -> Option<String> {
    let hex = digest.strip_prefix("sha256:")?;
    if hex.len() == 64 && hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Some(hex.to_ascii_lowercase())
    } else {
        None
    }
}

/// Numeric form of a tag: `v1.2.3` → `1.2.3`. A pre-release suffix
/// (`1.2.0-beta.1`) keeps only the numeric lead of each segment; that is safe
/// here because `releases/latest` never returns pre-releases.
pub fn normalize(tag: &str) -> Option<String> {
    let parts = parse_version(tag)?;
    Some(
        parts
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join("."),
    )
}

/// True when `candidate` is a later release than `current`.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(candidate), Some(current)) => compare(&candidate, &current) == Ordering::Greater,
        _ => false,
    }
}

/// Numeric segments of a version or tag: `v1.2.3` → `[1, 2, 3]`,
/// `1.2.3-beta.1` → `[1, 2, 3]`, `v2` → `[2]`. `None` when no segment carries
/// a digit at all.
fn parse_version(text: &str) -> Option<Vec<u64>> {
    let text = text.trim();
    let text = text
        .strip_prefix('v')
        .or_else(|| text.strip_prefix('V'))
        .unwrap_or(text);
    let mut parts = Vec::new();
    for segment in text.split('.') {
        let digits: String = segment
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect();
        if digits.is_empty() {
            break;
        }
        let pre_release = digits.len() != segment.len();
        // A segment too long for u64 clamps rather than failing: the release
        // still sorts last, which is the honest verdict for "bigger number".
        parts.push(digits.parse::<u64>().unwrap_or(u64::MAX));
        if pre_release {
            // `1.2.3-beta.1`: the suffix ends the release version; the `.1`
            // after it belongs to the pre-release, not to a fourth segment.
            break;
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts)
    }
}

/// Segment-wise numeric comparison with missing segments read as zero, so
/// `1.0.0` equals `1.0` and `1.0.10` outranks `1.0.9` (plain string order
/// would get that backwards).
fn compare(a: &[u64], b: &[u64]) -> Ordering {
    for index in 0..a.len().max(b.len()) {
        let left = a.get(index).copied().unwrap_or(0);
        let right = b.get(index).copied().unwrap_or(0);
        match left.cmp(&right) {
            Ordering::Equal => {}
            other => return other,
        }
    }
    Ordering::Equal
}

/// The shipping fetcher: one request, one 8-second deadline, no retries.
pub struct HttpFetcher;

impl Fetcher for HttpFetcher {
    fn fetch(&self, url: &str) -> Result<String, String> {
        use std::time::Duration;

        let agent = metadata_agent(Duration::from_secs(8));
        let mut response = agent
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", user_agent())
            .call()
            .map_err(|error| error.to_string())?;
        response
            .body_mut()
            .read_to_string()
            .map_err(|error| error.to_string())
    }
}

/// One agent for every reply in this module. Windows Schannel (native-tls),
/// not rustls: no C toolchain and no extra crypto backend in the build. Both
/// the provider and the system root store have to be named explicitly — the
/// crate defaults pick rustls and then fail at handshake time.
fn metadata_agent(timeout: std::time::Duration) -> ureq::Agent {
    use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .tls_config(
            TlsConfig::builder()
                .provider(TlsProvider::NativeTls)
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into()
}

/// The downloader's agent. No global deadline: 90 MB over a slow link is not a
/// failure, but a stalled socket is, so the read timeout is the one that bites.
fn download_agent() -> ureq::Agent {
    use std::time::Duration;
    use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

    ureq::Agent::config_builder()
        .timeout_global(None)
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .timeout_per_read(Some(Duration::from_secs(45)))
        .tls_config(
            TlsConfig::builder()
                .provider(TlsProvider::NativeTls)
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into()
}

fn user_agent() -> String {
    format!("DraftFlowCAD/{}", env!("CARGO_PKG_VERSION"))
}

/// Download the release file, verify it and hand it to the route its kind
/// names. Progress and stage changes go through `report`; the install runs on a
/// worker thread, never on the UI thread.
///
/// The order is deliberate: nothing touches the current installation until the
/// download has passed its size and digest checks.
pub fn install(
    target: &Path,
    asset: &Asset,
    cancel: &AtomicBool,
    mut report: impl FnMut(BusyStage, u64, Option<u64>),
) -> InstallOutcome {
    let staging = staging_path(target, asset.kind);
    if let Err(error) = download_to(asset, &staging, cancel, &mut report) {
        let _ = std::fs::remove_file(&staging);
        return match error {
            DownloadError::Cancelled => InstallOutcome::Cancelled,
            DownloadError::Failed(failure) => InstallOutcome::Failed(failure),
        };
    }
    report(BusyStage::Verifying, 0, None);
    if let Err(failure) = verify_download(&staging, asset) {
        log::warn!("update download rejected: {failure:?}");
        let _ = std::fs::remove_file(&staging);
        return InstallOutcome::Failed(failure);
    }
    report(BusyStage::Installing, 0, None);
    match asset.kind {
        AssetKind::Exe => install_executable(target, &staging),
        // The setup package's copy stays on disk until the helper has run it;
        // that helper removes it afterwards.
        AssetKind::Setup => install_setup(target, &staging),
    }
}

/// The portable route: rename the running file aside, put the download in its
/// place, and start it again once this process is gone.
fn install_executable(target: &Path, staging: &Path) -> InstallOutcome {
    if let Err(failure) = replace_executable(target, staging) {
        log::warn!("update swap failed: {failure:?}");
        return InstallOutcome::Failed(failure);
    }
    match schedule_relaunch(target) {
        Ok(()) => InstallOutcome::Installed { relaunch: true },
        Err(failure) => {
            log::warn!("update installed but the restart could not be scheduled: {failure:?}");
            InstallOutcome::Installed { relaunch: false }
        }
    }
}

/// The published route: hand the work to the Inno Setup package, pointed at the
/// folder the running executable lives in. The helper it starts is also what
/// reopens the application, so `relaunch` is honest here — the restart is
/// already in the command line.
fn install_setup(target: &Path, setup: &Path) -> InstallOutcome {
    match run_setup(target, setup) {
        Ok(()) => InstallOutcome::Installed { relaunch: true },
        Err(failure) => {
            log::warn!("update setup could not be started: {failure:?}");
            InstallOutcome::Failed(failure)
        }
    }
}

/// The `cmd.exe` arguments that run the setup package and then reopen the app.
///
/// The leading `ping` keeps `cmd` alive while this process exits, because the
/// installer cannot replace an executable that is still running. `/DIR` is the
/// folder of the running executable, so the installer upgrades the copy the
/// user actually launched instead of its own default location; the shared
/// `AppId` makes that an upgrade of the same installation, not a second one.
/// The `start` runs only once the install has finished, and the staging file is
/// removed then too. Pure so the shape is testable without running anything.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn setup_command(target: &Path, setup: &Path) -> String {
    let folder = target.parent().unwrap_or_else(|| Path::new("."));
    format!(
        "/C ping -n 6 127.0.0.1 >NUL & \"{}\" /SILENT /NORESTART /SUPPRESSMSGBOXES /CLOSEAPPLICATIONS /DIR=\"{}\" & start \"\" \"{}\" & del /f /q \"{}\" >NUL 2>NUL",
        setup.display(),
        folder.display(),
        target.display(),
        setup.display(),
    )
}

/// Runs [`setup_command`] in a hidden helper console. No `runas`: the setup
/// package installs per user (`PrivilegesRequired=lowest`), so an elevation
/// prompt on every update would be noise, and it also keeps the install mode —
/// and therefore the one uninstall record — identical between the first
/// install and every upgrade after it.
#[cfg(target_os = "windows")]
pub fn run_setup(target: &Path, setup: &Path) -> Result<(), Failure> {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let wide = |text: &str| -> Vec<u16> { text.encode_utf16().chain(Some(0)).collect() };
    let verb = wide("open");
    let file = wide("cmd.exe");
    let params = wide(&setup_command(target, setup));
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_FLAG_NO_UI | SEE_MASK_NOASYNC,
        lpVerb: verb.as_ptr(),
        lpFile: file.as_ptr(),
        lpParameters: params.as_ptr(),
        nShow: SW_HIDE,
        ..Default::default()
    };
    // SAFETY: every pointer in `info` outlives the call.
    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        let shell_code = info.hInstApp as usize;
        let code = if (1..=32).contains(&shell_code) {
            shell_code as u32
        } else {
            unsafe { GetLastError() }
        };
        log::warn!("update setup dispatch failed (code {code})");
        return Err(Failure::Install);
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn run_setup(_target: &Path, _setup: &Path) -> Result<(), Failure> {
    Err(Failure::Install)
}

/// Why a download stopped short.
enum DownloadError {
    Cancelled,
    Failed(Failure),
}

/// Streams `asset.url` into `destination`, reporting byte counts as they land.
/// `Ok(())` when the body ended cleanly; the caller verifies the bytes.
fn download_to(
    asset: &Asset,
    destination: &Path,
    cancel: &AtomicBool,
    report: &mut impl FnMut(BusyStage, u64, Option<u64>),
) -> Result<(), DownloadError> {
    let mut response = download_agent()
        .get(&asset.url)
        .header("Accept", "application/octet-stream")
        .header("User-Agent", user_agent())
        .call()
        .map_err(|error| {
            log::warn!("update download request failed: {error}");
            DownloadError::Failed(Failure::Download)
        })?;
    let total = response
        .body()
        .content_length()
        .or_else(|| (asset.size > 0).then_some(asset.size));
    let mut reader = response.body_mut().as_reader();
    let mut file = std::fs::File::create(destination).map_err(|error| {
        log::warn!("update download cannot be written: {error}");
        DownloadError::Failed(Failure::Install)
    })?;
    let mut buffer = vec![0u8; 64 * 1024];
    let mut done = 0u64;
    loop {
        if cancel.load(AtomicOrdering::Relaxed) {
            return Err(DownloadError::Cancelled);
        }
        let read = reader.read(&mut buffer).map_err(|error| {
            log::warn!("update download body failed: {error}");
            DownloadError::Failed(Failure::Download)
        })?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read]).map_err(|error| {
            log::warn!("update download cannot be written: {error}");
            DownloadError::Failed(Failure::Install)
        })?;
        done += read as u64;
        report(BusyStage::Downloading, done, total);
    }
    file.flush().map_err(|error| {
        log::warn!("update download cannot be flushed: {error}");
        DownloadError::Failed(Failure::Install)
    })?;
    Ok(())
}

/// The size and digest checks, plus the executable header. A release asset that
/// is not the file its name claims would otherwise be swapped in as junk or run
/// as a setup package.
///
/// The header bytes are fed into the digest rather than read and thrown away:
/// `read_exact` leaves the cursor after them, so hashing from there would cover
/// the file minus its lead bytes and never match the published digest. That is
/// the regression `verification_accepts_matching_size_and_digest` exists for.
pub fn verify_download(path: &Path, asset: &Asset) -> Result<(), Failure> {
    let metadata = std::fs::metadata(path).map_err(|_| Failure::Integrity)?;
    if asset.size > 0 && metadata.len() != asset.size {
        log::warn!(
            "update download has {} bytes, the release says {}",
            metadata.len(),
            asset.size
        );
        return Err(Failure::Integrity);
    }
    // Both kinds are Windows executables, so both start with `MZ`.
    let magic: &[u8] = b"MZ";
    let mut head = [0u8; 8];
    let mut file = std::fs::File::open(path).map_err(|_| Failure::Integrity)?;
    file.read_exact(&mut head[..magic.len()])
        .map_err(|_| Failure::Integrity)?;
    if &head[..magic.len()] != magic {
        log::warn!("update download is not a {}", asset.name);
        return Err(Failure::Integrity);
    }
    let Some(expected) = &asset.sha256 else {
        // No published digest: size and header are all this release offers.
        // Say so, because the report has to be able to.
        log::info!("update download verified by size only; the release published no digest");
        return Ok(());
    };
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    // The header the check above already consumed is part of the file.
    hasher.update(&head[..magic.len()]);
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|_| Failure::Integrity)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let actual = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if actual != *expected {
        log::warn!("update download digest {actual} from the release {expected}");
        return Err(Failure::Integrity);
    }
    Ok(())
}

/// Where the download lands. Next to the executable first so the swap is a
/// same-volume rename; a read-only program folder falls back to the user's temp
/// directory, where the copy into place is what fails.
///
/// Both names end in `.exe` because both kinds are launched as programs; the
/// setup one says so, since it is what the helper runs.
pub fn staging_path(target: &Path, kind: AssetKind) -> PathBuf {
    let name = match kind {
        AssetKind::Exe => "DraftFlowCAD-update-download.exe",
        AssetKind::Setup => "DraftFlowCAD-update-setup.exe",
    };
    if let Some(dir) = target.parent() {
        let candidate = dir.join(name);
        if std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&candidate)
            .is_ok()
        {
            return candidate;
        }
    }
    std::env::temp_dir().join(name)
}

/// Puts `new_file` at `target`, keeping the previous bytes as
/// `<target>.old-<stamp>`. A failed install puts the old file back, so a
/// refused write leaves the application exactly as it was.
pub fn replace_executable(target: &Path, new_file: &Path) -> Result<(), Failure> {
    let backup = backup_path(target);
    std::fs::rename(target, &backup).map_err(|error| {
        log::warn!("update cannot move the running executable: {error}");
        Failure::Install
    })?;
    if let Err(error) = move_into_place(new_file, target) {
        log::warn!("update cannot install the new executable: {error}");
        let _ = std::fs::rename(&backup, target);
        return Err(Failure::Install);
    }
    cleanup_old_backups(target, &backup);
    Ok(())
}

fn move_into_place(new_file: &Path, target: &Path) -> std::io::Result<()> {
    match std::fs::rename(new_file, target) {
        Ok(()) => Ok(()),
        // A temp-directory staging file lives on another volume.
        Err(_) => {
            std::fs::copy(new_file, target)?;
            let _ = std::fs::remove_file(new_file);
            Ok(())
        }
    }
}

/// The rollback name. Named after the executable so a user looking at the
/// folder reads what it is, and stamped so repeated updates never collide.
fn backup_path(target: &Path) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    let name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "DraftFlowCAD.exe".to_string());
    let dir = target.parent().unwrap_or(Path::new("."));
    let mut candidate = dir.join(format!("{name}.old-{stamp}"));
    let mut copy = 1;
    while candidate.exists() {
        candidate = dir.join(format!("{name}.old-{stamp}-{copy}"));
        copy += 1;
    }
    candidate
}

/// Drop older rollbacks; the newest one is the one worth keeping.
fn cleanup_old_backups(target: &Path, keep: &Path) {
    let (Some(dir), Some(name)) = (target.parent(), target.file_name()) else {
        return;
    };
    let prefix = format!("{}.old-", name.to_string_lossy());
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path == keep {
            continue;
        }
        if path
            .file_name()
            .is_some_and(|file| file.to_string_lossy().starts_with(&prefix))
        {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// Starts the freshly installed executable a few seconds after this process
/// exits, through a detached shell — the update is only complete once the new
/// build is on screen.
#[cfg(target_os = "windows")]
pub fn schedule_relaunch(exe: &Path) -> Result<(), Failure> {
    use std::os::windows::process::CommandExt;
    /// Keeps the `cmd` window from flashing over the app as it closes.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let script = format!(
        "ping -n 4 127.0.0.1 >NUL & start \"\" \"{}\"",
        exe.display()
    );
    std::process::Command::new("cmd")
        .args(["/C", &script])
        .current_dir(exe.parent().unwrap_or(Path::new(".")))
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            log::warn!("update relaunch could not be started: {error}");
            Failure::Install
        })
}

#[cfg(not(target_os = "windows"))]
pub fn schedule_relaunch(exe: &Path) -> Result<(), Failure> {
    let script = format!("sleep 3; exec \"{}\"", exe.display());
    std::process::Command::new("sh")
        .args(["-c", &script])
        .current_dir(exe.parent().unwrap_or(Path::new(".")))
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            log::warn!("update relaunch could not be started: {error}");
            Failure::Install
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_normalize_to_numeric_versions() {
        assert_eq!(normalize("v1.2.3").as_deref(), Some("1.2.3"));
        assert_eq!(normalize("1.2.3").as_deref(), Some("1.2.3"));
        assert_eq!(normalize("V2.0").as_deref(), Some("2.0"));
        assert_eq!(normalize("  v1.0.0  ").as_deref(), Some("1.0.0"));
        assert_eq!(normalize("v1.2.3-beta.1").as_deref(), Some("1.2.3"));
        assert_eq!(normalize("v2").as_deref(), Some("2"));
        // Nothing numeric at all: not a version this dialog can offer.
        assert_eq!(normalize("release"), None);
        assert_eq!(normalize("v"), None);
        assert_eq!(normalize(""), None);
    }

    #[test]
    fn newer_versions_compare_numerically_not_lexically() {
        assert!(is_newer("1.0.1", "1.0.0"));
        // The whole point of the numeric compare: 10 > 9.
        assert!(is_newer("1.0.10", "1.0.9"));
        assert!(is_newer("v1.1", "1.0.9"));
        assert!(is_newer("1.2.0", "1.1.99"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("1.0", "1.0.0"), "trailing zeros do not count");
        assert!(!is_newer("0.9.9", "1.0.0"));
        assert!(!is_newer("not-a-version", "1.0.0"));
    }

    #[test]
    fn release_json_yields_tag_and_page() {
        let body = r#"{"tag_name":"v1.2.0","html_url":"https://github.com/DF172/DFlow/releases/tag/v1.2.0","name":"1.2.0","assets":[]}"#;
        let release = parse_release_json(body).unwrap();
        assert_eq!(release.version, "1.2.0");
        assert_eq!(
            release.url,
            "https://github.com/DF172/DFlow/releases/tag/v1.2.0"
        );
        assert_eq!(release.exe, None);
        assert_eq!(release.setup, None);
    }

    #[test]
    fn release_json_falls_back_to_the_releases_page() {
        let release = parse_release_json(r#"{"tag_name":"v1.2.0"}"#).unwrap();
        assert_eq!(release.url, RELEASES_PAGE);
        // A non-URL html_url is no better than a missing one.
        let release = parse_release_json(r#"{"tag_name":"v1.2.0","html_url":"df"}"#).unwrap();
        assert_eq!(release.url, RELEASES_PAGE);
    }

    #[test]
    fn release_json_picks_the_setup_package_over_a_bare_executable() {
        let body = r#"{
            "tag_name":"v1.1.0",
            "html_url":"https://example.com/tag",
            "assets":[
                {"name":"DraftFlowCAD-1.1.0-x64-setup.exe","size":31511862,
                 "browser_download_url":"https://example.com/DraftFlowCAD-1.1.0-x64-setup.exe",
                 "digest":"sha256:BEB7108EBAEFF8B8DEF110362E42C263789D6306731A2849D148C412DDEA6867"},
                {"name":"DraftFlowCAD.exe","size":94014464,
                 "browser_download_url":"https://example.com/DraftFlowCAD.exe",
                 "digest":"sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"}
            ]
        }"#;
        let release = parse_release_json(body).unwrap();
        let setup = release
            .setup
            .as_ref()
            .expect("the setup asset must be recognised");
        assert_eq!(setup.name, "DraftFlowCAD-1.1.0-x64-setup.exe");
        assert_eq!(setup.kind, AssetKind::Setup);
        assert_eq!(setup.size, 31_511_862);
        assert_eq!(
            setup.url,
            "https://example.com/DraftFlowCAD-1.1.0-x64-setup.exe"
        );
        // Digests are normalised to lowercase hex.
        assert_eq!(
            setup.sha256.as_deref(),
            Some("beb7108ebaeff8b8def110362e42c263789d6306731a2849d148c412ddea6867")
        );
        let exe = release
            .exe
            .as_ref()
            .expect("the bare executable must be recognised");
        assert_eq!(exe.kind, AssetKind::Exe);
        // The setup package is the published channel, so it is what an upgrade
        // runs when both files are there.
        assert_eq!(
            release.installer().map(|asset| asset.kind),
            Some(AssetKind::Setup)
        );
    }

    #[test]
    fn a_release_that_only_ships_an_msi_installs_nothing_by_itself() {
        let body = r#"{
            "tag_name":"v1.1.0",
            "assets":[
                {"name":"DraftFlowCAD-1.1.0-x64.msi","size":43208704,
                 "browser_download_url":"https://example.com/a.msi"}
            ]
        }"#;
        let release = parse_release_json(body).unwrap();
        assert_eq!(release.exe, None);
        assert_eq!(release.setup, None);
        assert_eq!(release.installer(), None);
    }

    #[test]
    fn a_release_without_an_installer_only_offers_the_page() {
        let body = r#"{
            "tag_name":"v2.0.0",
            "assets":[
                {"name":"notes.txt","size":12,"browser_download_url":"https://example.com/n.txt"}
            ]
        }"#;
        let release = parse_release_json(body).unwrap();
        assert_eq!(release.installer(), None);
    }

    #[test]
    fn assets_without_a_digest_or_url_are_tolerated() {
        let body = r#"{
            "tag_name":"v2.0.0",
            "assets":[
                {"name":"DraftFlowCAD.exe","digest":null},
                {"name":"portable.EXE","size":10,"browser_download_url":"https://example.com/p.exe",
                 "digest":"sha512:deadbeef"}
            ]
        }"#;
        let release = parse_release_json(body).unwrap();
        let exe = release
            .exe
            .expect("a URL-less asset must be skipped, not fatal");
        assert_eq!(exe.name, "portable.EXE", "matching is case-insensitive");
        assert_eq!(exe.sha256, None, "an unknown digest algorithm is no digest");
    }

    #[test]
    fn unusable_release_replies_are_rejected() {
        assert!(parse_release_json("not json").is_err());
        assert!(parse_release_json(r#"{"html_url":"https://example.com"}"#).is_err());
        assert!(parse_release_json(r#"{"tag_name":"release"}"#).is_err());
        assert!(parse_release_json(r#"{"tag_name":5}"#).is_err());
    }

    struct Mock {
        reply: Result<String, String>,
    }

    impl Fetcher for Mock {
        fn fetch(&self, url: &str) -> Result<String, String> {
            assert_eq!(url, RELEASES_API, "the check must ask releases/latest");
            self.reply.clone()
        }
    }

    fn outcome_with(reply: Result<String, String>) -> Outcome {
        check_with(&Mock { reply }, "1.0.0")
    }

    #[test]
    fn check_reports_a_newer_release_with_its_page() {
        let outcome = outcome_with(Ok(
            r#"{"tag_name":"v1.1.0","html_url":"https://github.com/DF172/DFlow/releases/tag/v1.1.0"}"#
                .to_string(),
        ));
        assert_eq!(
            outcome,
            Outcome::Available {
                version: "1.1.0".to_string(),
                url: "https://github.com/DF172/DFlow/releases/tag/v1.1.0".to_string(),
                installer: None,
            }
        );
    }

    #[test]
    fn check_reports_up_to_date_on_the_same_version() {
        let outcome = outcome_with(Ok(r#"{"tag_name":"v1.0.0"}"#.to_string()));
        assert_eq!(
            outcome,
            Outcome::UpToDate {
                latest: "1.0.0".to_string()
            }
        );
    }

    #[test]
    fn check_reports_a_network_failure() {
        assert_eq!(
            outcome_with(Err("offline".to_string())),
            Outcome::Failed(Failure::Network)
        );
    }

    #[test]
    fn check_reports_an_unreadable_reply() {
        assert_eq!(
            outcome_with(Ok("<html>rate limited</html>".to_string())),
            Outcome::Failed(Failure::Unreadable)
        );
    }

    #[test]
    fn finished_checks_land_in_the_dialog_state() {
        let mut state = CheckState::Checking;
        state.apply(Outcome::Available {
            version: "2.0.0".to_string(),
            url: "https://example.com".to_string(),
            installer: None,
        });
        assert!(state.offers_newer_version());
        assert!(matches!(state, CheckState::Available { .. }));
        state.apply(Outcome::Failed(Failure::Network));
        assert_eq!(state, CheckState::Failed(Failure::Network));
        assert!(!state.offers_newer_version());
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "draftflow-update-test-{}-{}-{name}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("scratch directory");
        dir
    }

    #[test]
    fn replacing_the_executable_keeps_the_previous_bytes() {
        let dir = scratch("swap");
        let target = dir.join("DraftFlowCAD.exe");
        let new_file = dir.join("DraftFlowCAD-update-download.exe");
        std::fs::write(&target, b"old build").unwrap();
        std::fs::write(&new_file, b"new build").unwrap();

        replace_executable(&target, &new_file).expect("swap must succeed");
        assert_eq!(std::fs::read(&target).unwrap(), b"new build");
        let backups: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|entry| entry.file_name().to_string_lossy().contains(".old-"))
            .collect();
        assert_eq!(backups.len(), 1, "exactly one rollback copy");
        assert_eq!(
            std::fs::read(backups[0].path()).unwrap(),
            b"old build",
            "the rollback copy is the previous build"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_current_executable_is_an_install_failure() {
        let dir = scratch("missing");
        let target = dir.join("DraftFlowCAD.exe");
        let new_file = dir.join("DraftFlowCAD-update-download.exe");
        std::fs::write(&new_file, b"new build").unwrap();
        assert_eq!(
            replace_executable(&target, &new_file),
            Err(Failure::Install)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An asset of a given kind with the given size and digest, so the
    /// verification tests name only what they vary.
    fn asset_of(kind: AssetKind, name: &str, size: u64, sha256: Option<String>) -> Asset {
        Asset {
            name: name.to_string(),
            size,
            url: "https://example.com".to_string(),
            sha256,
            kind,
        }
    }

    #[test]
    fn verification_accepts_matching_size_and_digest() {
        let dir = scratch("verify-ok");
        let path = dir.join("candidate.exe");
        // `MZ` plus a short body: the header check only reads the first two
        // bytes, and the digest covers exactly what was written.
        let bytes = b"MZfake executable body";
        std::fs::write(&path, bytes).unwrap();
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let asset = asset_of(
            AssetKind::Exe,
            "DraftFlowCAD.exe",
            bytes.len() as u64,
            Some(digest),
        );
        assert_eq!(verify_download(&path, &asset), Ok(()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn verification_accepts_a_setup_package() {
        let dir = scratch("verify-setup");
        let path = dir.join("candidate.exe");
        let bytes = b"MZsetup package body";
        std::fs::write(&path, bytes).unwrap();
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let asset = asset_of(
            AssetKind::Setup,
            "DraftFlowCAD-1.1.0-x64-setup.exe",
            bytes.len() as u64,
            Some(digest),
        );
        assert_eq!(verify_download(&path, &asset), Ok(()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn verification_rejects_a_wrong_size_digest_or_header() {
        let dir = scratch("verify-bad");
        let path = dir.join("candidate.exe");
        std::fs::write(&path, b"MZpayload").unwrap();

        let short = asset_of(AssetKind::Exe, "DraftFlowCAD.exe", 9_999, None);
        assert_eq!(verify_download(&path, &short), Err(Failure::Integrity));

        let wrong_digest = asset_of(AssetKind::Exe, "DraftFlowCAD.exe", 0, Some("0".repeat(64)));
        assert_eq!(
            verify_download(&path, &wrong_digest),
            Err(Failure::Integrity)
        );

        std::fs::write(&path, b"not a PE file").unwrap();
        let header = asset_of(AssetKind::Exe, "DraftFlowCAD.exe", 0, None);
        assert_eq!(verify_download(&path, &header), Err(Failure::Integrity));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_setup_command_waits_then_installs_in_place_then_reopens() {
        let target = Path::new(r"D:\app\DraftFlowCAD\DraftFlowCAD.exe");
        let setup = Path::new(r"D:\app\DraftFlowCAD\DraftFlowCAD-update-setup.exe");
        let command = setup_command(target, setup);
        assert!(
            command.contains(&format!("\"{}\"", setup.display())),
            "the setup path must be quoted: {command}"
        );
        assert!(command.contains("/SILENT"), "{command}");
        assert!(command.contains("/NORESTART"), "{command}");
        assert!(command.contains("/SUPPRESSMSGBOXES"), "{command}");
        assert!(command.contains("/CLOSEAPPLICATIONS"), "{command}");
        // The install folder is the folder the running executable lives in, so
        // the package upgrades the user's copy instead of its own default.
        assert!(
            command.contains(r#"/DIR="D:\app\DraftFlowCAD""#),
            "the install must stay in the launched folder: {command}"
        );
        assert!(
            command.contains(&format!("\"{}\"", target.display())),
            "the reopened copy must be the same path: {command}"
        );
        // The install and the restart stay in that order: a `start` before the
        // setup would reopen the version about to be replaced.
        let wait_at = command.find("ping").unwrap();
        let install_at = command.find("setup.exe").unwrap();
        let reopen_at = command.find("start \"\"").unwrap();
        assert!(wait_at < install_at && install_at < reopen_at, "{command}");
    }

    #[test]
    fn downloads_stage_under_distinct_executable_names() {
        // A directory that does not exist, so the probe falls through to the
        // temp directory on every machine instead of writing into a real
        // program folder on an elevated test run.
        let target = Path::new(r"C:\draftflow-no-such-dir\DraftFlowCAD.exe");
        assert_eq!(
            staging_path(target, AssetKind::Setup)
                .file_name()
                .and_then(|name| name.to_str()),
            Some("DraftFlowCAD-update-setup.exe")
        );
        assert_eq!(
            staging_path(target, AssetKind::Exe)
                .file_name()
                .and_then(|name| name.to_str()),
            Some("DraftFlowCAD-update-download.exe")
        );
    }
}
