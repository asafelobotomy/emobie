//! Root-side package installation for deb/rpm updates.
//!
//! The package is copied into a fresh root-owned directory and its SHA-256 is
//! verified *there* before the installer runs, so a same-user process cannot
//! swap the file between verification and install.

use super::apply::{run_checked, which};
use std::path::Path;
use std::process::Command;

/// Runs as root via `pkexec sh -c`. Copies the package into a fresh root-owned
/// directory, verifies its SHA-256 *there* (so a same-user process cannot swap
/// the file between verification and install), then runs the installer with
/// `@PKG@` replaced by the verified copy. Arguments: src, sha256, installer argv.
const ROOT_INSTALL_SCRIPT: &str = r#"set -eu
src="$1"; want="$2"; shift 2
dir="$(mktemp -d)"
trap 'rm -rf "$dir"' EXIT
chmod 755 "$dir"
pkg="$dir/$(basename "$src")"
install -m 644 "$src" "$pkg"
got="$(sha256sum "$pkg" | cut -d' ' -f1)"
if [ "$got" != "$want" ]; then echo "checksum mismatch" >&2; exit 1; fi
n=$#
while [ "$n" -gt 0 ]; do
  a="$1"; shift
  if [ "$a" = "@PKG@" ]; then set -- "$@" "$pkg"; else set -- "$@" "$a"; fi
  n=$((n - 1))
done
"$@"
"#;

fn root_install_command(program: &str, pkg: &Path, sha256: &str, installer: &[&str]) -> Command {
    let mut cmd = Command::new(program);
    if program == "pkexec" {
        cmd.args(["sh", "-c", ROOT_INSTALL_SCRIPT, "emobie-update"]);
    } else {
        cmd.args(["-c", ROOT_INSTALL_SCRIPT, "emobie-update"]);
    }
    cmd.arg(pkg).arg(sha256).args(installer);
    cmd
}

fn install_as_root(pkg: &Path, sha256: &str, installer: &[&str]) -> Result<(), String> {
    run_checked(&mut root_install_command("pkexec", pkg, sha256, installer))
}

pub(super) fn install_deb(path: &Path, sha256: &str) -> Result<(), String> {
    if which("apt-get") {
        return install_as_root(
            path,
            sha256,
            &["env", "DEBIAN_FRONTEND=noninteractive", "apt-get", "install", "-y", "@PKG@"],
        );
    }
    install_as_root(path, sha256, &["dpkg", "-i", "@PKG@"])
}

pub(super) fn install_rpm(path: &Path, sha256: &str) -> Result<(), String> {
    if which("dnf") {
        return install_as_root(path, sha256, &["dnf", "install", "-y", "@PKG@"]);
    }
    if which("zypper") {
        // Our RPMs are unsigned; integrity comes from the SHA-256 check above.
        return install_as_root(
            path,
            sha256,
            &["zypper", "--non-interactive", "install", "--allow-unsigned-rpm", "@PKG@"],
        );
    }
    install_as_root(path, sha256, &["rpm", "-Uvh", "@PKG@"])
}

#[cfg(test)]
mod tests {
    use super::{root_install_command};
    use crate::updates::apply::lower_hex;
    use sha2::{Digest, Sha256};
    use std::path::PathBuf;

    fn temp_file(name: &str, body: &[u8]) -> (PathBuf, String) {
        let dir = std::env::temp_dir().join(format!("emobie-apply-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, body).unwrap();
        (path, lower_hex(&Sha256::digest(body)))
    }

    /// Runs the root script with plain `sh` (no pkexec) against a fake installer.
    fn run(path: &std::path::Path, sha: &str, installer: &[&str]) -> bool {
        root_install_command("sh", path, sha, installer)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[test]
    fn root_script_installs_only_matching_checksum() {
        let (path, sha) = temp_file("emobie_test.deb", b"package bytes");
        // Installer sees a non-empty copy that keeps the original file name.
        let check = ["sh", "-c", "test -s \"$1\" && test \"$(basename \"$1\")\" = emobie_test.deb", "t", "@PKG@"];
        assert!(run(&path, &sha, &check));
        assert!(!run(&path, &"0".repeat(64), &check), "wrong hash must fail");
        // A failing installer propagates its status.
        assert!(!run(&path, &sha, &["false"]));
    }

    #[test]
    fn lower_hex_formats() {
        assert_eq!(lower_hex(&[0x00, 0xab, 0x0f]), "00ab0f");
    }
}
