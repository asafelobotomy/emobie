//! Unix-socket RPC: request read, dispatch, and response write.

use crate::inject;
use crate::listen;
use crate::matcher::TriggerTrie;
use crate::protocol::{MatchRule, Request, Response};
use crate::state;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

const MAX_REQUEST_BYTES: u64 = 512 * 1024;
pub(crate) const MAX_CLIENT_THREADS: usize = 32;

fn persist_locked(enabled: &AtomicBool, matches: &[MatchRule]) {
    state::save_reloading_enabled(enabled, matches);
}

pub(crate) struct ClientSlot<'a>(pub &'a AtomicUsize);

impl Drop for ClientSlot<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

pub(crate) fn configure_client_stream(stream: &UnixStream) {
    // Free client slots if a peer stalls mid-request (same-UID DoS / hung app).
    let timeout = Duration::from_secs(5);
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
}

fn read_request_line(
    reader: &mut BufReader<UnixStream>,
    line: &mut String,
) -> std::io::Result<bool> {
    line.clear();
    // Bound the read so a huge unterminated payload cannot OOM the daemon.
    let mut buf = Vec::new();
    let n = {
        let mut limited = reader.by_ref().take(MAX_REQUEST_BYTES + 1);
        limited.read_until(b'\n', &mut buf)?
    };
    if n == 0 {
        return Ok(false);
    }
    if buf.len() > MAX_REQUEST_BYTES as usize || !buf.ends_with(b"\n") {
        return Err(std::io::Error::new(
            ErrorKind::InvalidData,
            "request too large or missing newline",
        ));
    }
    *line = String::from_utf8_lossy(&buf).into_owned();
    Ok(true)
}

fn write_response(writer: &mut UnixStream, response: &Response) {
    if let Ok(payload) = serde_json::to_string(response) {
        let _ = writeln!(writer, "{payload}");
    }
}

pub(crate) fn handle_client(
    stream: UnixStream,
    enabled: &AtomicBool,
    trie: &Mutex<TriggerTrie>,
    stored: &Mutex<Vec<MatchRule>>,
    _clients: ClientSlot<'_>,
) {
    let writer_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    };
    let mut reader = BufReader::new(stream);
    let mut writer = writer_stream;
    let mut line = String::new();
    loop {
        match read_request_line(&mut reader, &mut line) {
            Ok(true) => {}
            Ok(false) => break,
            Err(err) if err.kind() == ErrorKind::InvalidData => {
                let can_inject = inject::can_inject();
                let can_listen = listen::can_listen();
                write_response(
                    &mut writer,
                    &Response::err(
                        can_inject,
                        can_listen,
                        enabled.load(Ordering::Relaxed),
                        "request too large",
                    ),
                );
                break;
            }
            Err(_) => break,
        }
        let trimmed = line.trim().to_string();
        line.clear();
        if trimmed.is_empty() {
            continue;
        }
        let can_inject = inject::can_inject();
        let can_listen = listen::can_listen();
        let enabled_now = enabled.load(Ordering::Relaxed);
        let response = match serde_json::from_str::<Request>(&trimmed) {
            Ok(Request::Status) => Response::status(
                can_inject,
                can_listen,
                enabled_now,
                if can_listen {
                    "emobie-inputd running"
                } else {
                    "running, but keyboard access missing — run setup-input-access.sh \
(ACLs usually avoid logout; otherwise log out/in once)"
                },
            ),
            Ok(Request::SetEnabled { enabled: value }) => {
                enabled.store(value, Ordering::Relaxed);
                inject::set_expand_enabled(value);
                if !value {
                    listen::clear_pending();
                }
                // Hold `stored` while saving so we cannot overwrite a concurrent
                // SyncMatches with a stale matches snapshot.
                match stored.lock() {
                    Ok(guard) => persist_locked(enabled, &guard),
                    Err(poisoned) => persist_locked(enabled, &poisoned.into_inner()),
                }
                Response::status(
                    can_inject,
                    can_listen,
                    value,
                    if value {
                        "expansion enabled"
                    } else {
                        "expansion disabled"
                    },
                )
            }
            Ok(Request::SyncMatches { matches }) => match state::validate_matches(&matches) {
                Ok(()) => {
                    let matches = state::dedupe_matches(matches);
                    let pairs = state::pairs_from_matches(&matches);
                    let count = pairs.len();
                    // Update trie under lock; persist outside so typing is not blocked on disk.
                    let sync_result = (|| -> Result<(bool, Vec<MatchRule>), String> {
                        let mut stored_guard = stored
                            .lock()
                            .map_err(|_| "internal lock poisoned — try again".to_string())?;
                        let mut trie_guard = trie
                            .lock()
                            .map_err(|_| "internal lock poisoned — try again".to_string())?;
                        let unchanged = *stored_guard == matches;
                        trie_guard.load(&pairs);
                        *stored_guard = matches;
                        let snapshot = stored_guard.clone();
                        Ok((unchanged, snapshot))
                    })();
                    match sync_result {
                        Ok((unchanged, snapshot)) => {
                            if !unchanged {
                                state::save_reloading_enabled(enabled, &snapshot);
                            }
                            Response::status(
                                can_inject,
                                can_listen,
                                enabled.load(Ordering::Relaxed),
                                &if unchanged {
                                    format!("synced {count} matches (unchanged)")
                                } else {
                                    format!("synced {count} matches")
                                },
                            )
                        }
                        Err(err) => Response::err(can_inject, can_listen, enabled_now, &err),
                    }
                }
                Err(err) => Response::err(can_inject, can_listen, enabled_now, &err),
            },
            Ok(Request::SetOptions { restore_clipboard }) => {
                if let Some(value) = restore_clipboard {
                    inject::set_restore_clipboard(value);
                }
                Response::status(
                    can_inject,
                    can_listen,
                    enabled_now,
                    &format!(
                        "options updated (restore_clipboard={})",
                        inject::restore_clipboard_enabled()
                    ),
                )
            }
            Ok(Request::InjectPaste) => match inject::inject_ctrl_v() {
                Ok(()) => Response::status(
                    can_inject,
                    can_listen,
                    enabled.load(Ordering::Relaxed),
                    "paste injected",
                ),
                Err(err) => Response::err(can_inject, can_listen, enabled_now, &err),
            },
            Err(err) => Response::err(
                can_inject,
                can_listen,
                enabled_now,
                &format!("bad request: {err}"),
            ),
        };
        write_response(&mut writer, &response);
    }
}

#[cfg(test)]
mod tests {
    use super::MAX_REQUEST_BYTES;
    use std::io::{BufRead, BufReader, Cursor, Read};

    fn read_bounded(data: &[u8]) -> (usize, Vec<u8>) {
        let mut reader = BufReader::new(Cursor::new(data));
        let mut buf = Vec::new();
        let n = {
            let mut limited = reader.by_ref().take(MAX_REQUEST_BYTES + 1);
            limited.read_until(b'\n', &mut buf).unwrap()
        };
        (n, buf)
    }

    #[test]
    fn bounded_read_rejects_oversize_line() {
        let huge = "x".repeat(MAX_REQUEST_BYTES as usize + 2) + "\n";
        let (n, buf) = read_bounded(huge.as_bytes());
        assert!(n > 0);
        assert!(buf.len() > MAX_REQUEST_BYTES as usize || !buf.ends_with(b"\n"));
    }

    #[test]
    fn bounded_read_accepts_normal_line() {
        let (n, buf) = read_bounded(b"{\"cmd\":\"status\"}\n");
        assert!(n > 0);
        assert!(buf.ends_with(b"\n"));
        assert!(buf.len() <= MAX_REQUEST_BYTES as usize);
    }
}
