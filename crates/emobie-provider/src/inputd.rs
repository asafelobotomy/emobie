//! A minimal emobie-inputd client: one `inject_paste` request on its Unix socket. The socket
//! locations and trust rules match `src-tauri/src/input_helper/unix/socket.rs` and
//! `crates/emobie-inputd/src/socket_path.rs`: only directories that are ours (or root's and
//! not group/other-writable), and only a socket node we own.

use crate::activate::Paster;
use serde::Deserialize;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

const SOCKET_NAME: &str = "emobie-inputd.sock";
const MAX_RESPONSE_BYTES: u64 = 64 * 1024;

#[derive(Deserialize)]
struct Reply {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    detail: String,
}

fn current_uid() -> Option<u32> {
    std::fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find(|l| l.starts_with("Uid:"))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

fn dir_owner_acceptable(owner: u32, uid: u32, mode: u32) -> bool {
    owner == uid || (owner == 0 && (mode & 0o022) == 0)
}

fn parent_safe(path: &Path, uid: u32) -> bool {
    let Some(parent) = path.parent() else {
        return false;
    };
    std::fs::symlink_metadata(parent)
        .map(|m| m.is_dir() && dir_owner_acceptable(m.uid(), uid, m.mode()))
        .unwrap_or(false)
}

fn socket_ours(path: &Path, uid: u32) -> bool {
    std::fs::symlink_metadata(path)
        .map(|m| m.file_type().is_socket() && m.uid() == uid)
        .unwrap_or(false)
}

pub struct Inputd {
    candidates: Vec<PathBuf>,
    timeout: Duration,
}

impl Inputd {
    /// The helper's default socket locations (no `EMOBIE_INPUTD_SOCKET` override: the provider
    /// is started by the bus, not from a shell the user configured).
    pub fn system() -> Self {
        let mut candidates = Vec::new();
        if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from) {
            if runtime.is_absolute() {
                candidates.push(runtime.join("emobie").join(SOCKET_NAME));
            }
        }
        candidates.push(PathBuf::from("/run/emobie").join(SOCKET_NAME));
        if let Some(uid) = current_uid() {
            candidates.push(PathBuf::from(format!("/tmp/emobie-{uid}")).join(SOCKET_NAME));
        }
        Self::with_candidates(candidates)
    }

    pub fn with_candidates(candidates: Vec<PathBuf>) -> Self {
        Self {
            candidates,
            timeout: Duration::from_secs(3),
        }
    }

    fn sockets(&self) -> Vec<&PathBuf> {
        let Some(uid) = current_uid() else {
            return Vec::new();
        };
        self.candidates
            .iter()
            .filter(|p| p.file_name().and_then(|n| n.to_str()) == Some(SOCKET_NAME))
            .filter(|p| parent_safe(p, uid) && socket_ours(p, uid))
            .collect()
    }

    fn request(&self, line: &str) -> Result<Reply, String> {
        let mut stream = self
            .sockets()
            .into_iter()
            .find_map(|p| UnixStream::connect(p).ok())
            .ok_or("emobie-inputd is not running")?;
        let _ = stream.set_read_timeout(Some(self.timeout));
        let _ = stream.set_write_timeout(Some(self.timeout));
        writeln!(stream, "{line}").map_err(|e| e.to_string())?;
        let mut reply = String::new();
        BufReader::new(stream.take(MAX_RESPONSE_BYTES))
            .read_line(&mut reply)
            .map_err(|e| e.to_string())?;
        serde_json::from_str(reply.trim()).map_err(|e| format!("bad reply from emobie-inputd: {e}"))
    }
}

impl Paster for Inputd {
    fn available(&self) -> bool {
        !self.sockets().is_empty()
    }

    fn paste(&self) -> Result<(), String> {
        let reply = self.request(r#"{"cmd":"inject_paste"}"#)?;
        if reply.ok {
            Ok(())
        } else {
            Err(reply.error.unwrap_or(reply.detail))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    fn tempdir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "emobie-provider-inputd-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn owner_rules_match_the_helper() {
        assert!(dir_owner_acceptable(1000, 1000, 0o777));
        assert!(dir_owner_acceptable(0, 1000, 0o755));
        assert!(!dir_owner_acceptable(0, 1000, 0o775));
        assert!(!dir_owner_acceptable(1001, 1000, 0o700));
    }

    #[test]
    fn not_available_without_a_socket() {
        let dir = tempdir("none");
        let client = Inputd::with_candidates(vec![dir.join(SOCKET_NAME)]);
        assert!(!client.available());
        assert!(client.paste().unwrap_err().contains("not running"));
        // A regular file with the socket's name is not a socket.
        std::fs::write(dir.join(SOCKET_NAME), "").unwrap();
        assert!(!client.available());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sends_inject_paste_and_reads_the_reply() {
        let dir = tempdir("paste");
        let path = dir.join(SOCKET_NAME);
        let listener = UnixListener::bind(&path).unwrap();
        let server = std::thread::spawn(move || {
            let mut seen = Vec::new();
            for reply in [
                r#"{"ok":true,"detail":"paste injected"}"#,
                r#"{"ok":false,"detail":"no uinput","error":"no uinput"}"#,
            ] {
                let (stream, _) = listener.accept().unwrap();
                let mut line = String::new();
                BufReader::new(stream.try_clone().unwrap())
                    .read_line(&mut line)
                    .unwrap();
                seen.push(line);
                writeln!(&stream, "{reply}").unwrap();
            }
            seen
        });
        let client = Inputd::with_candidates(vec![dir.join("other.sock"), path.clone()]);
        assert!(client.available());
        client.paste().unwrap();
        assert_eq!(client.paste().unwrap_err(), "no uinput");
        let seen = server.join().unwrap();
        assert_eq!(seen, vec!["{\"cmd\":\"inject_paste\"}\n"; 2]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
