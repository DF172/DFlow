//! Native drawing edit leases and external-change fingerprints.
//!
//! The sidecar lock survives the editor's atomic temp-file replacement, while
//! the drawing lock interoperates with platform `flock` / `LockFileEx` users.
//! Neither mechanism can force every third-party Unix application to cooperate,
//! so saves also compare [`FileFingerprint`] immediately before replacement.

use std::fs::{File, OpenOptions, TryLockError};
use std::hash::{Hash, Hasher};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const SAMPLE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFingerprint {
    len: u64,
    modified_ns: Option<u128>,
    sample_hash: u64,
}

impl FileFingerprint {
    pub fn capture(path: &Path) -> std::io::Result<Self> {
        Self::capture_from(&mut File::open(path)?)
    }

    /// Fingerprint an already-open drawing handle.
    pub fn capture_from(file: &mut File) -> std::io::Result<Self> {
        let metadata = file.metadata()?;
        let len = metadata.len();
        let modified_ns = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_nanos());

        let mut offsets = vec![0];
        if len > SAMPLE_BYTES as u64 {
            offsets.push((len / 2).saturating_sub((SAMPLE_BYTES / 2) as u64));
            offsets.push(len.saturating_sub(SAMPLE_BYTES as u64));
        }
        offsets.sort_unstable();
        offsets.dedup();

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        len.hash(&mut hasher);
        for offset in offsets {
            file.seek(SeekFrom::Start(offset))?;
            let mut remaining = (len.saturating_sub(offset)).min(SAMPLE_BYTES as u64) as usize;
            let mut buffer = vec![0_u8; remaining];
            let mut read = 0;
            while remaining > 0 {
                let count = file.read(&mut buffer[read..])?;
                if count == 0 {
                    break;
                }
                read += count;
                remaining -= count;
            }
            offset.hash(&mut hasher);
            buffer[..read].hash(&mut hasher);
        }

        Ok(Self {
            len,
            modified_ns,
            sample_hash: hasher.finish(),
        })
    }
}

/// What a loaded/current fingerprint pair says about the file on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskState {
    /// Nothing was known before the open, so nothing can be claimed about it.
    NoBaseline,
    Unchanged,
    Changed,
    /// The file could not be re-read while a baseline was held. Not the same
    /// as [`DiskState::Unchanged`]: absence of evidence is not evidence that
    /// the drawing is alone on disk.
    Unverifiable,
}

/// Three-way read of a fingerprint pair. Callers that compared with
/// `loaded.zip(current).is_some_and(|(a, b)| a != b)` collapsed
/// [`DiskState::Unverifiable`] into "unchanged" — the one branch that must
/// stay distinguishable, because [`FileFingerprint::capture`] fails precisely
/// when another process has the drawing to itself.
pub fn compare_fingerprints(
    loaded: Option<&FileFingerprint>,
    current: Option<&FileFingerprint>,
) -> DiskState {
    match (loaded, current) {
        (Some(loaded), Some(current)) if loaded != current => DiskState::Changed,
        (Some(_), Some(_)) => DiskState::Unchanged,
        (Some(_), None) => DiskState::Unverifiable,
        (None, _) => DiskState::NoBaseline,
    }
}

#[cfg(unix)]
pub(crate) fn path_matches_file(path: &Path, file: &File) -> std::io::Result<bool> {
    use std::os::unix::fs::MetadataExt;

    let path_metadata = std::fs::metadata(path)?;
    let file_metadata = file.metadata()?;
    Ok(path_metadata.dev() == file_metadata.dev() && path_metadata.ino() == file_metadata.ino())
}

#[cfg(target_os = "windows")]
pub(crate) fn path_matches_file(path: &Path, file: &File) -> std::io::Result<bool> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };

    let path_file = OpenOptions::new()
        .access_mode(0)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .open(path)?;
    Ok(windows_file_identity(&path_file)? == windows_file_identity(file)?)
}

#[cfg(target_os = "windows")]
fn windows_file_identity(file: &File) -> std::io::Result<(u32, u64)> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    if unsafe { GetFileInformationByHandle(file.as_raw_handle() as _, &mut info) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    let index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
    Ok((info.dwVolumeSerialNumber, index))
}

#[cfg(not(any(unix, target_os = "windows")))]
pub(crate) fn path_matches_file(path: &Path, file: &File) -> std::io::Result<bool> {
    let mut path_file = File::open(path)?;
    let mut leased_file = file.try_clone()?;
    Ok(FileFingerprint::capture_from(&mut path_file)?
        == FileFingerprint::capture_from(&mut leased_file)?)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditLeaseError {
    Locked(String),
    Unavailable(String),
}

impl std::fmt::Display for EditLeaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Locked(message) | Self::Unavailable(message) => f.write_str(message),
        }
    }
}

#[derive(Debug)]
pub struct EditLease {
    _sidecar: File,
    drawing: Option<File>,
    lock_path: PathBuf,
    platform_warning: Option<String>,
}

impl EditLease {
    pub fn acquire(path: &Path) -> Result<Self, EditLeaseError> {
        let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let lock_path = sidecar_path(&canonical);
        let mut sidecar = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&lock_path)
            .map_err(|error| {
                EditLeaseError::Unavailable(format!(
                    "Could not create edit lock {}: {error}",
                    lock_path.display()
                ))
            })?;

        match sidecar.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                let owner = std::fs::read_to_string(&lock_path)
                    .ok()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| "another DraftFlow CAD session".to_string());
                return Err(EditLeaseError::Locked(format!(
                    "Drawing edit lock is held by {owner}."
                )));
            }
            Err(TryLockError::Error(error)) => {
                return Err(EditLeaseError::Unavailable(format!(
                    "Could not acquire edit lock {}: {error}",
                    lock_path.display()
                )));
            }
        }

        let identity = format!(
            "DraftFlow CAD pid={} path={}",
            std::process::id(),
            canonical.display()
        );
        let identity_warning = publish_identity(&mut sidecar, &lock_path, &identity);

        let (drawing, platform_warning) = match acquire_drawing_lock(&canonical) {
            Ok(lock) => lock,
            Err(error) => {
                let _ = std::fs::remove_file(&lock_path);
                return Err(error);
            }
        };
        // A lease whose owner line did not land is still a lease — `try_lock`
        // succeeded — but the "held by …" message another session gets from
        // this file is now known to be unreliable, so say so on the same
        // channel that already reports platform lock weakness.
        let platform_warning = match (platform_warning, identity_warning) {
            (Some(warning), Some(note)) => Some(format!("{warning} {note}")),
            (Some(warning), None) => Some(warning),
            (None, Some(note)) => Some(note),
            (None, None) => None,
        };
        Ok(Self {
            _sidecar: sidecar,
            drawing,
            lock_path,
            platform_warning,
        })
    }

    pub fn platform_warning(&self) -> Option<&str> {
        self.platform_warning.as_deref()
    }

    /// Fingerprint through the lease's drawing handle.
    pub fn fingerprint(&mut self) -> std::io::Result<Option<FileFingerprint>> {
        self.drawing
            .as_mut()
            .map(FileFingerprint::capture_from)
            .transpose()
    }

    /// Clone the leased drawing handle for a worker.
    pub fn reader(&self) -> std::io::Result<Option<File>> {
        self.drawing.as_ref().map(File::try_clone).transpose()
    }

    /// Atomic save replaces the path with a new file object. Move the platform
    /// lock to that new object; the sidecar remains locked throughout.
    pub fn refresh_drawing_lock(&mut self, path: &Path) -> Result<(), EditLeaseError> {
        let (drawing, warning) = acquire_drawing_lock(path)?;
        self.drawing = drawing;
        self.platform_warning = warning;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn lock_path(&self) -> &Path {
        &self.lock_path
    }
}

impl Drop for EditLease {
    fn drop(&mut self) {
        // Remove the path while the exclusive sidecar lock is still held so
        // another editor cannot acquire it during cleanup.
        let _ = std::fs::remove_file(&self.lock_path);
    }
}

/// Open the drawing so other applications may still read it but not write it.
///
/// Windows: our handle only reads, and its share mode admits other readers
/// (and the atomic save's replace) while refusing any write open; the shared
/// byte lock below refuses writes through handles opened before ours. An
/// exclusive lock would also refuse reads, so another CAD program could not
/// even open the drawing read-only while it is being edited here.
#[cfg(target_os = "windows")]
fn open_drawing_for_lease(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_DELETE, FILE_SHARE_READ};
    OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE)
        .open(path)
}

#[cfg(not(target_os = "windows"))]
fn open_drawing_for_lease(path: &Path) -> std::io::Result<File> {
    OpenOptions::new().read(true).write(true).open(path)
}

/// Windows takes the shared lock (writes refused, reads allowed); elsewhere the
/// lock is advisory and never stops a reader, so it stays exclusive.
#[cfg(target_os = "windows")]
fn lock_drawing(drawing: &File) -> Result<(), TryLockError> {
    drawing.try_lock_shared()
}

#[cfg(not(target_os = "windows"))]
fn lock_drawing(drawing: &File) -> Result<(), TryLockError> {
    drawing.try_lock()
}

/// Publish this session as the lock's owner and confirm the bytes a colliding
/// session would read really say so.
///
/// Each of the four steps used to be `let _ =`, so a refused `set_len` (an SMB
/// share with an oplock conflict, a full quota, an antivirus holding the
/// sidecar) left the *previous* holder's `pid=` line — or a longer line with
/// our short one written over its prefix — in the file that answers
/// "held by whom". `Some(reason)` does not cost us the lock: `try_lock`
/// already succeeded, the drawing is still ours to edit.
fn publish_identity(sidecar: &mut File, lock_path: &Path, identity: &str) -> Option<String> {
    let mut notes = Vec::new();
    if let Err(error) = sidecar
        .set_len(0)
        .and_then(|_| sidecar.seek(SeekFrom::Start(0)))
        .and_then(|_| writeln!(sidecar, "{identity}"))
        .and_then(|_| sidecar.flush())
    {
        notes.push(format!("writing the owner line failed: {error}"));
    }
    // Read the path another session reads, not this handle: the two disagree
    // exactly when our write went somewhere else.
    match std::fs::read_to_string(lock_path) {
        Ok(value) if value.trim() == identity => {}
        Ok(value) => notes.push(format!(
            "the lock file holds {:?}, not this session's owner line",
            value.trim()
        )),
        Err(error) => notes.push(format!("reading the lock file back failed: {error}")),
    }
    (!notes.is_empty()).then(|| {
        format!(
            "Edit-lock owner could not be published ({}); another session opening this drawing would be told the wrong holder.",
            notes.join("; ")
        )
    })
}

fn acquire_drawing_lock(path: &Path) -> Result<(Option<File>, Option<String>), EditLeaseError> {
    let drawing = match open_drawing_for_lease(path) {
        Ok(file) => file,
        Err(error) if error_is_lock_conflict(&error) => {
            return Err(EditLeaseError::Locked(format!(
                "Drawing is open for writing in another application: {error}"
            )));
        }
        Err(error) => {
            return Ok((
                None,
                Some(format!("Platform drawing lock unavailable: {error}")),
            ));
        }
    };

    match lock_drawing(&drawing) {
        Ok(()) => Ok((Some(drawing), None)),
        Err(TryLockError::WouldBlock) => Err(EditLeaseError::Locked(
            "Drawing is locked by another application.".to_string(),
        )),
        Err(TryLockError::Error(error)) => Ok((
            None,
            Some(format!("Platform drawing lock unavailable: {error}")),
        )),
    }
}

#[cfg(target_os = "windows")]
fn error_is_lock_conflict(error: &std::io::Error) -> bool {
    matches!(error.raw_os_error(), Some(32 | 33))
}

#[cfg(not(target_os = "windows"))]
fn error_is_lock_conflict(_error: &std::io::Error) -> bool {
    false
}

fn sidecar_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "drawing".to_string());
    path.parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(".{name}.draftflow.lock"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "draftflow_edit_lock_{}_{}_{}",
            std::process::id(),
            name,
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    /// The sidecar is what a colliding session reads to answer "held by whom".
    /// All four identity writes were `let _ =`, so a refused `set_len` left the
    /// previous holder's line — or our short line over its prefix — in place.
    #[test]
    fn the_lock_file_holds_only_this_sessions_owner_line() {
        let lock_path = unique_path("identity.lock");
        std::fs::write(
            &lock_path,
            "DraftFlow CAD pid=999 path=Z:\\other\\drawing.dxf\n",
        )
        .expect("seed previous holder");
        let mut sidecar = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock_path)
            .expect("open sidecar");

        let identity = format!("DraftFlow CAD pid={} path=Y:\\mine.dxf", std::process::id());
        assert_eq!(
            publish_identity(&mut sidecar, &lock_path, &identity),
            None,
            "a published identity is not a warning"
        );
        assert_eq!(
            std::fs::read_to_string(&lock_path).expect("read back"),
            format!("{identity}\n"),
            "the previous holder's bytes must not survive a shorter owner line"
        );
        std::fs::remove_file(&lock_path).expect("remove sidecar");
    }

    /// Verification reads the path another session reads, not this handle: a
    /// write that lands somewhere else, and a file we cannot read at all, are
    /// both reported instead of trusted.
    #[test]
    fn an_owner_line_that_does_not_reach_the_lock_file_is_reported() {
        let sidecar_path = unique_path("wrong-sidecar");
        let read_path = unique_path("wrong-readback");
        std::fs::write(&read_path, "DraftFlow CAD pid=1 path=Z:\\stale.dxf").expect("seed");
        let mut sidecar = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&sidecar_path)
            .expect("create sidecar");

        let warning = publish_identity(&mut sidecar, &read_path, "DraftFlow CAD pid=2 path=Y.dxf")
            .expect("a lock file that does not hold our owner line must be reported");
        assert!(
            warning.contains("would be told the wrong holder"),
            "the warning has to name the consequence: {warning}"
        );

        std::fs::remove_file(&read_path).expect("remove seed");
        match publish_identity(&mut sidecar, &read_path, "DraftFlow CAD pid=3 path=Y.dxf") {
            Some(warning) => assert!(
                warning.contains("reading the lock file back failed"),
                "an unreadable lock file is not a published one: {warning}"
            ),
            None => panic!("a lock file that cannot be read back must not pass as published"),
        }
        std::fs::remove_file(&sidecar_path).expect("remove sidecar");
    }

    #[test]
    fn a_baseline_that_cannot_be_re_read_is_not_reported_as_unchanged() {
        let loaded = FileFingerprint {
            len: 8,
            modified_ns: Some(1),
            sample_hash: 7,
        };
        let same = FileFingerprint {
            len: 8,
            modified_ns: Some(1),
            sample_hash: 7,
        };
        let moved = FileFingerprint {
            len: 9,
            modified_ns: Some(1),
            sample_hash: 7,
        };

        assert_eq!(
            compare_fingerprints(Some(&loaded), Some(&same)),
            DiskState::Unchanged
        );
        assert_eq!(
            compare_fingerprints(Some(&loaded), Some(&moved)),
            DiskState::Changed
        );
        assert_eq!(
            compare_fingerprints(Some(&loaded), None),
            DiskState::Unverifiable
        );
        assert_eq!(
            compare_fingerprints(None, Some(&same)),
            DiskState::NoBaseline
        );
        assert_eq!(compare_fingerprints(None, None), DiskState::NoBaseline);

        // Why the enum exists: the `zip(..).is_some_and(..)` it replaces hands
        // back the same answer for "nothing moved" and "the file is locked by
        // someone else", so a caller cannot tell a clean open from no evidence.
        let old_shape_unchanged = Some(&loaded).zip(Some(&same)).is_some_and(|(a, b)| a != b);
        let old_shape_unverifiable = Some(&loaded)
            .zip(None::<&FileFingerprint>)
            .is_some_and(|(a, b)| a != b);
        assert_eq!(
            old_shape_unverifiable, old_shape_unchanged,
            "zip collapses the two"
        );
        assert_ne!(
            compare_fingerprints(Some(&loaded), Some(&same)),
            compare_fingerprints(Some(&loaded), None),
            "the enum keeps them apart"
        );
    }

    #[test]
    fn edit_lease_blocks_second_ocs_session() {
        let path = unique_path("lease.dwg");
        std::fs::write(&path, b"dwg").unwrap();
        let first = EditLease::acquire(&path).unwrap();
        assert!(matches!(
            EditLease::acquire(&path),
            Err(EditLeaseError::Locked(_))
        ));
        drop(first);
        assert!(EditLease::acquire(&path).is_ok());
        let _ = std::fs::remove_file(sidecar_path(&path));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_lease_fingerprints_the_drawing_it_has_locked() {
        let path = unique_path("leased.dwg");
        std::fs::write(&path, b"drawing bytes").unwrap();
        let unlocked = FileFingerprint::capture(&path).unwrap();

        let mut lease = EditLease::acquire(&path).unwrap();
        let leased = lease
            .fingerprint()
            .expect("the lease handle is readable")
            .expect("the lease holds a drawing handle");
        assert_eq!(unlocked, leased);

        drop(lease);
        let _ = std::fs::remove_file(sidecar_path(&path));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_lease_lends_a_reader_for_the_drawing_it_has_locked() {
        let path = unique_path("leased_reader.dwg");
        std::fs::write(&path, b"drawing bytes").unwrap();

        let lease = EditLease::acquire(&path).unwrap();
        let mut reader = lease
            .reader()
            .expect("the lease handle is cloneable")
            .expect("the lease holds a drawing handle");
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).unwrap();
        assert_eq!(b"drawing bytes".as_slice(), bytes.as_slice());

        drop(reader);
        drop(lease);
        let _ = std::fs::remove_file(sidecar_path(&path));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_replaced_path_no_longer_matches_the_lease() {
        let path = unique_path("identity.dwg");
        let replacement = unique_path("replacement.dwg");
        std::fs::write(&path, b"old").unwrap();
        std::fs::write(&replacement, b"new").unwrap();

        let lease = EditLease::acquire(&path).unwrap();
        let reader = lease
            .reader()
            .unwrap()
            .expect("the lease holds a drawing handle");
        std::fs::remove_file(&path).unwrap();
        std::fs::rename(&replacement, &path).unwrap();
        assert!(!path_matches_file(&path, &reader).unwrap());

        drop(reader);
        drop(lease);
        let _ = std::fs::remove_file(sidecar_path(&path));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn fingerprint_detects_same_size_content_change() {
        let path = unique_path("fingerprint.dwg");
        std::fs::write(&path, b"abcd").unwrap();
        let before = FileFingerprint::capture(&path).unwrap();
        std::fs::write(&path, b"abce").unwrap();
        let after = FileFingerprint::capture(&path).unwrap();
        assert_ne!(before, after);
        let _ = std::fs::remove_file(path);
    }
}
