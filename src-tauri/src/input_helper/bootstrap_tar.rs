//! Host-bundle tar handling for the emobie-inputd bootstrap (AppImage / Flatpak).

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Members allowed in the host bootstrap tarball (must match scripts/stage-inputd.sh).
pub(super) const TAR_MEMBERS: &[&str] = &[
    "emobie-inputd",
    "bootstrap-inputd-host.sh",
    "setup-input-access.sh",
    "99-emobie-input.rules",
    "io.github.asafelobotomy.emobie.inputd.policy",
    "selinux/emobie-inputd.te",
];

/// Arguments for extracting the host bundle into `dest` from stdin.
///
/// GNU tar already strips leading `/` and rejects `..` members, so no flag is
/// needed for that — `--no-absolute-names` is not a GNU tar option and made
/// every extraction fail. Members are matched by exact name, so the bundle must
/// be built with explicit members (no `./` prefix): see scripts/stage-inputd.sh.
pub(super) fn tar_extract_args(dest: &Path) -> Vec<std::ffi::OsString> {
    let mut args: Vec<std::ffi::OsString> = vec!["xzf".into(), "-".into(), "-C".into(), dest.into()];
    args.push("--no-overwrite-dir".into());
    args.extend(TAR_MEMBERS.iter().map(|m| std::ffi::OsString::from(*m)));
    args
}

pub(super) fn extract_tarball_to(data: &Path, bytes: &[u8]) -> bool {
    if fs_create_dir_all(data).is_err() {
        return false;
    }
    let mut cmd = Command::new("tar");
    cmd.args(tar_extract_args(data))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    cmd.spawn()
        .and_then(|mut child| {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(bytes)?;
            }
            child.wait()
        })
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Shell script run on the host (via `flatpak-spawn --host bash -c`) that
/// extracts the bundle from stdin and runs the bootstrap script. `data_str`
/// must already have passed `path_safe_for_shell`.
pub(super) fn host_extract_script(data_str: &str) -> String {
    let members = TAR_MEMBERS
        .iter()
        .map(|m| format!("'{m}'"))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "set -euo pipefail; \
         mkdir -p '{data_str}'; \
         tar xzf - -C '{data_str}' --no-overwrite-dir {members}; \
         exec bash '{data_str}/bootstrap-inputd-host.sh' '{data_str}/emobie-inputd'"
    )
}

fn fs_create_dir_all(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)
}

#[cfg(test)]
mod tests {
    use super::{host_extract_script, tar_extract_args, TAR_MEMBERS};
    use std::process::{Command, Stdio};

    /// Builds a bundle exactly like scripts/stage-inputd.sh (explicit members)
    /// and extracts it with the argument list the app really uses.
    #[test]
    fn bundle_extracts_with_real_tar() {
        let root = std::env::temp_dir().join(format!("emobie-bootstrap-tar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let stage = root.join("stage");
        let out = root.join("out");
        std::fs::create_dir_all(stage.join("selinux")).unwrap();
        std::fs::create_dir_all(&out).unwrap();
        for m in TAR_MEMBERS {
            std::fs::write(stage.join(m), b"x").unwrap();
        }
        let tgz = root.join("bundle.tgz");
        let Ok(made) = Command::new("tar")
            .arg("czf")
            .arg(&tgz)
            .arg("-C")
            .arg(&stage)
            .args(TAR_MEMBERS)
            .status()
        else {
            return; // no tar on this host
        };
        assert!(made.success());
        let status = Command::new("tar")
            .args(tar_extract_args(&out))
            .stdin(std::fs::File::open(&tgz).unwrap())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "tar rejected the bootstrap arguments");
        for m in TAR_MEMBERS {
            assert!(out.join(m).is_file(), "{m} missing after extraction");
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn host_script_has_no_unsupported_flags() {
        let script = host_extract_script("/home/u/.local/share/emobie");
        assert!(!script.contains("--no-absolute-names"));
        assert!(script.contains("'selinux/emobie-inputd.te'"));
    }
}
