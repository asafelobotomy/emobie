//! Unix-domain socket client for emobie-inputd.

use serde::Deserialize;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_RESPONSE_BYTES: u64 = 64 * 1024;

#[derive(Deserialize)]
pub struct DaemonResponse {
    pub ok: bool,
    pub can_inject: bool,
    pub can_listen: bool,
    pub detail: String,
    pub error: Option<String>,
    #[serde(default)]
    pub suppress_jobs: Option<usize>,
    #[serde(default)]
    pub restore_clipboard: Option<bool>,
    #[serde(default)]
    pub last_inject_backend: Option<String>,
    #[serde(default)]
    pub paste_chord: Option<String>,
    /// Absent on helpers older than 0.6.28.
    #[serde(default)]
    pub version: Option<String>,
}

pub(super) fn current_uid() -> u32 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("Uid:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|u| u.parse().ok())
        })
        .unwrap_or(0)
}

fn tmp_emobie_socket() -> PathBuf {
    PathBuf::from(format!("/tmp/emobie-{}/emobie-inputd.sock", current_uid()))
}

/// Candidate socket paths, in priority order. Every one — not just the env
/// override — must pass `trusted_socket_path`, so a directory another local
/// user pre-created (e.g. `/tmp/emobie-<uid>`) is never connected to.
fn candidate_sockets() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(custom) = std::env::var("EMOBIE_INPUTD_SOCKET") {
        paths.push(PathBuf::from(&custom));
    }
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        paths.push(PathBuf::from(runtime).join("emobie/emobie-inputd.sock"));
    }
    paths.push(PathBuf::from("/run/emobie/emobie-inputd.sock"));
    paths.push(tmp_emobie_socket());
    paths.retain(|p| trusted_socket_path(p) && socket_file_ours(p));
    paths
}

/// The socket node itself must be owned by us (the daemon runs as our uid).
fn socket_file_ours(path: &Path) -> bool {
    use std::os::unix::fs::FileTypeExt;
    std::fs::symlink_metadata(path)
        .map(|m| m.file_type().is_socket() && m.uid() == current_uid())
        .unwrap_or(false)
}

pub(super) fn trusted_socket_path(path: &Path) -> bool {
    // Keep in sync with crates/emobie-inputd/src/socket_path.rs::is_trusted.
    if path.file_name().and_then(|n| n.to_str()) != Some("emobie-inputd.sock") {
        return false;
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    if !socket_parent_dir_safe(parent) {
        return false;
    }
    if parent == Path::new("/run/emobie") {
        return true;
    }
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        if parent == Path::new(&runtime).join("emobie") {
            return true;
        }
    }
    let tmp_fallback = PathBuf::from(format!("/tmp/emobie-{}", current_uid()));
    if parent == tmp_fallback.as_path() {
        return true;
    }
    false
}

fn socket_parent_dir_safe(path: &Path) -> bool {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return false;
    };
    if !meta.is_dir() {
        return false;
    }
    dir_owner_acceptable(meta.uid(), current_uid(), meta.mode())
}

/// Ours at any mode, or root's and not group/other-writable. A directory owned
/// by any other user is never trusted, even at 0755 — its owner could plant a
/// socket there. Keep in sync with the daemon's `socket_path::dir_owner_acceptable`.
pub(super) fn dir_owner_acceptable(owner: u32, uid: u32, mode: u32) -> bool {
    owner == uid || (owner == 0 && (mode & 0o022) == 0)
}

fn connect_with_timeout(timeout: Duration) -> Option<UnixStream> {
    for path in candidate_sockets() {
        if let Ok(stream) = UnixStream::connect(&path) {
            let _ = stream.set_read_timeout(Some(timeout));
            let _ = stream.set_write_timeout(Some(timeout));
            return Some(stream);
        }
    }
    None
}

pub fn request(cmd: serde_json::Value) -> Result<DaemonResponse, String> {
    request_with_timeout(cmd, Duration::from_millis(800))
}

pub(super) fn request_with_timeout(
    cmd: serde_json::Value,
    timeout: Duration,
) -> Result<DaemonResponse, String> {
    let mut stream =
        connect_with_timeout(timeout).ok_or_else(|| "emobie-inputd not running".to_string())?;
    let payload = serde_json::to_string(&cmd).map_err(|e| e.to_string())?;
    writeln!(stream, "{payload}").map_err(|e| e.to_string())?;
    // Bound the reply: the daemon's status lines are tiny, and a socket that
    // streams without a newline must not be able to grow this buffer forever.
    let mut reader = BufReader::new(stream.take(MAX_RESPONSE_BYTES));
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| e.to_string())?;
    serde_json::from_str(line.trim()).map_err(|e| e.to_string())
}
