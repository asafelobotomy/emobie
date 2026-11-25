//! How this copy of emobie was installed, which decides how (and whether) emobie updates it.
//! A copy pacman installed (the AUR package) is pacman's to update, never emobie's.

use std::fs;
use std::path::Path;
use std::process::Command;

use super::apply::{which, InstallKind};

pub fn detect_install_kind() -> InstallKind {
    if std::env::var_os("FLATPAK_ID").is_some() {
        return InstallKind::Flatpak;
    }
    if std::env::var_os("APPIMAGE").is_some() {
        return InstallKind::AppImage;
    }
    if let Ok(exe) = std::env::current_exe() {
        if exe
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.contains("AppImage"))
        {
            return InstallKind::AppImage;
        }
        let path = exe.to_string_lossy();
        if path.starts_with("/usr/") {
            if owns_emobie(Path::new(DB)) {
                return InstallKind::System;
            }
            if Path::new("/var/lib/dpkg/info/emobie.list").exists()
                || Path::new("/var/lib/dpkg/info/emobie.md5sums").exists()
            {
                return InstallKind::Deb;
            }
            if Command::new("rpm")
                .args(["-q", "emobie"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                return InstallKind::Rpm;
            }
            // Packaged under /usr but unknown manager — prefer deb if apt exists.
            if which("apt-get") || which("dpkg") {
                return InstallKind::Deb;
            }
            if which("dnf") || which("zypper") || which("rpm") {
                return InstallKind::Rpm;
            }
            // Under /usr with a package manager emobie can't drive: leave it alone.
            return InstallKind::System;
        }
    }
    InstallKind::Native
}

const DB: &str = "/var/lib/pacman/local";

/// True when pacman's database has an `emobie-<ver>-<rel>` entry.
fn owns_emobie(db: &Path) -> bool {
    let Ok(entries) = fs::read_dir(db) else { return false };
    entries.flatten().any(|e| is_emobie_entry(&e.file_name().to_string_lossy()))
}

/// `emobie-0.9.5-1`: the name, then exactly a version and a release. `emobie-inputd-…` or
/// `emobie-git-…` are other packages.
fn is_emobie_entry(name: &str) -> bool {
    name.strip_prefix("emobie-").is_some_and(|rest| {
        rest.split('-').count() == 2 && rest.starts_with(|c: char| c.is_ascii_digit())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_emobie_package_counts() {
        assert!(is_emobie_entry("emobie-0.9.5-1"));
        assert!(is_emobie_entry("emobie-1:0.9.5-2"));
        assert!(!is_emobie_entry("emobie-git-0.9.5.r3.gabc-1"));
        assert!(!is_emobie_entry("emobie-bin-0.9.5-1"));
        assert!(!is_emobie_entry("emobiex-0.9.5-1"));
        assert!(!is_emobie_entry("emobie"));
    }

    #[test]
    fn reads_the_database_folder() {
        let dir = std::env::temp_dir().join(format!("emobie-pacman-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("kitty-0.40.0-1")).unwrap();
        assert!(!owns_emobie(&dir));
        fs::create_dir_all(dir.join("emobie-0.9.5-1")).unwrap();
        assert!(owns_emobie(&dir));
        assert!(!owns_emobie(&dir.join("missing")));
        let _ = fs::remove_dir_all(&dir);
    }
}
