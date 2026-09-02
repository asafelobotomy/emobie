//! Trusted Unix socket locations for emobie-inputd.
//!
//! Allowed parents:
//! - `$XDG_RUNTIME_DIR/emobie/` (normal sessions; mode 0700)
//! - `/run/emobie/` when safe (uid-owned, or not other-writable, or sticky)
//! - `/tmp/emobie-$UID/` (fallback when XDG_RUNTIME_DIR is unset)
//!
//! Keep `is_trusted` in sync with `src-tauri/src/input_helper/unix.rs::trusted_socket_path`.

use nix::fcntl::{Flock, FlockArg};
use nix::unistd::getuid;
use std::fs::{self, File};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

const SOCKET_NAME: &str = "emobie-inputd.sock";
const LOCK_NAME: &str = "emobie-inputd.lock";

/// Directory is safe for a same-UID socket (not a world-writable hijack point).
fn dir_is_safe(path: &Path) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    if !meta.is_dir() {
        return false;
    }
    let uid = getuid().as_raw();
    if meta.uid() == uid {
        return true;
    }
    let mode = meta.mode();
    // Not writable by others, or sticky (like /tmp).
    (mode & 0o002) == 0 || (mode & 0o1000) != 0
}

/// Whether `path` is an allowed socket location (under runtime dir or /run/emobie).
pub fn is_trusted(path: &Path) -> bool {
    if path.file_name().and_then(|n| n.to_str()) != Some(SOCKET_NAME) {
        return false;
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    if parent == Path::new("/run/emobie") {
        return dir_is_safe(parent);
    }
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        if parent == Path::new(&runtime).join("emobie") {
            return dir_is_safe(parent);
        }
    }
    // Fallback used when XDG_RUNTIME_DIR is unset (tests / broken sessions).
    if parent == Path::new(&format!("/tmp/emobie-{}", getuid())) {
        return dir_is_safe(parent);
    }
    false
}

pub fn default_runtime_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(dir).join("emobie");
    }
    PathBuf::from(format!("/tmp/emobie-{}", getuid()))
}

pub fn default_socket_path() -> PathBuf {
    default_runtime_dir().join(SOCKET_NAME)
}

/// Per-uid lock directory — always the default runtime dir, even when the socket
/// lives under `/run/emobie`, so XDG and `/run` cannot dual-listen.
pub fn instance_lock_dir() -> PathBuf {
    default_runtime_dir()
}

/// Resolve bind/connect path; ignores untrusted `EMOBIE_INPUTD_SOCKET` overrides.
pub fn resolve_socket_path() -> PathBuf {
    if let Ok(custom) = std::env::var("EMOBIE_INPUTD_SOCKET") {
        let path = PathBuf::from(&custom);
        if is_trusted(&path) {
            return path;
        }
        eprintln!(
            "ignoring untrusted EMOBIE_INPUTD_SOCKET (must be under \
             $XDG_RUNTIME_DIR/emobie/, /run/emobie/, or /tmp/emobie-$UID/): {}",
            custom
        );
    }
    default_socket_path()
}

/// Exclusive per-uid lock so a detached helper and systemd unit cannot both listen.
/// Keep the returned lock alive for the process lifetime.
pub fn acquire_instance_lock() -> Result<Flock<File>, String> {
    let runtime_dir = instance_lock_dir();
    let path = runtime_dir.join(LOCK_NAME);
    let file = File::options()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .open(&path)
        .map_err(|e| format!("open lock {}: {e}", path.display()))?;
    // Re-apply in case an older lock file was created with a looser mode.
    let _ = fs::set_permissions(
        &path,
        {
            let mut p = fs::metadata(&path)
                .map_err(|e| format!("stat lock {}: {e}", path.display()))?
                .permissions();
            p.set_mode(0o600);
            p
        },
    );
    Flock::lock(file, FlockArg::LockExclusiveNonblock).map_err(|_| {
        format!(
            "another emobie-inputd is already running (lock {})",
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_arbitrary_socket_override() {
        assert!(!is_trusted(Path::new("/tmp/evil/emobie-inputd.sock")));
    }

    #[test]
    fn accepts_runtime_socket() {
        if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
            let dir = PathBuf::from(&runtime).join("emobie");
            let _ = fs::create_dir_all(&dir);
            let path = dir.join("emobie-inputd.sock");
            assert!(is_trusted(&path));
        }
    }

    #[test]
    fn accepts_tmp_fallback_socket() {
        let dir = PathBuf::from(format!("/tmp/emobie-{}", getuid()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("emobie-inputd.sock");
        assert!(is_trusted(&path));
    }

    #[test]
    fn run_emobie_requires_safe_dir() {
        // Path shape alone is not enough when the directory is missing.
        if !Path::new("/run/emobie").exists() {
            assert!(!is_trusted(Path::new("/run/emobie/emobie-inputd.sock")));
        }
    }
}
