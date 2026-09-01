//! Native ~/.local install from a GitHub .deb (binary + desktop/icons).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::apply::{cache_dir, run_checked};

const ICON_ID: &str = "io.github.asafelobotomy.emobie";

pub fn install_native_from_deb(deb: &Path) -> Result<(), String> {
    let work = cache_dir()?.join("native-extract");
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    run_checked(
        Command::new("ar")
            .arg("x")
            .arg(deb)
            .current_dir(&work),
    )?;
    let data_tar = fs::read_dir(&work)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("data.tar"))
        })
        .ok_or_else(|| "deb missing data.tar.*".to_string())?;
    run_checked(
        Command::new("tar")
            .args(["xf", "--no-absolute-names", "--no-overwrite-dir"])
            .arg(&data_tar)
            .current_dir(&work),
    )?;
    let extracted = work.join("usr/bin/emobie");
    if !extracted.is_file() {
        let _ = fs::remove_dir_all(&work);
        return Err("deb did not contain usr/bin/emobie".into());
    }
    let bin_dir = std::env::var_os("XDG_BIN_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/bin"))
        })
        .ok_or_else(|| "HOME is not set".to_string())?;
    fs::create_dir_all(&bin_dir).map_err(|e| e.to_string())?;
    let dest_bin = bin_dir.join("emobie-bin");
    fs::copy(&extracted, &dest_bin).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dest_bin).map_err(|e| e.to_string())?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest_bin, perms).map_err(|e| e.to_string())?;
    }

    // Also install emobie-inputd + host bootstrap assets when present in the deb.
    let inputd_src = work.join("usr/bin/emobie-inputd");
    if inputd_src.is_file() {
        let dest_inputd = bin_dir.join("emobie-inputd");
        fs::copy(&inputd_src, &dest_inputd).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dest_inputd)
                .map_err(|e| e.to_string())?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dest_inputd, perms).map_err(|e| e.to_string())?;
        }
        install_native_inputd_assets(&work, &dest_inputd)?;
    }

    let launcher = bin_dir.join("emobie");
    let script = format!(
        "#!/bin/sh\nset -e\nBIN=\"{}\"\nif [ -x \"$BIN\" ]; then\n  exec \"$BIN\" \"$@\"\nfi\nexec flatpak run --user {ICON_ID} \"$@\"\n",
        dest_bin.display()
    );
    fs::write(&launcher, script).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&launcher)
            .map_err(|e| e.to_string())?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&launcher, perms).map_err(|e| e.to_string())?;
    }
    install_native_desktop_assets(&work, &launcher)?;
    let _ = fs::remove_dir_all(&work);
    Ok(())
}

fn install_native_inputd_assets(extracted_root: &Path, inputd_bin: &Path) -> Result<(), String> {
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share"))
        })
        .ok_or_else(|| "HOME is not set".to_string())?
        .join("emobie");
    fs::create_dir_all(&data).map_err(|e| e.to_string())?;

    let share = extracted_root.join("usr/share/emobie");
    for name in [
        "setup-input-access.sh",
        "99-emobie-input.rules",
        "io.github.asafelobotomy.emobie.inputd.policy",
        "bootstrap-inputd-host.sh",
    ] {
        let src = share.join(name);
        if src.is_file() {
            let dest = data.join(name);
            fs::copy(&src, &dest).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            if name.ends_with(".sh") {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&dest).map_err(|e| e.to_string())?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&dest, perms).map_err(|e| e.to_string())?;
            }
        }
    }

    let bootstrap = data.join("bootstrap-inputd-host.sh");
    if bootstrap.is_file() {
        let _ = Command::new("bash")
            .arg(&bootstrap)
            .arg(inputd_bin)
            .status();
    } else {
        // Minimal user unit when bootstrap script is missing from the package.
        let unit_dir = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))
            })
            .ok_or_else(|| "HOME is not set".to_string())?
            .join("systemd/user");
        fs::create_dir_all(&unit_dir).map_err(|e| e.to_string())?;
        let unit = format!(
            "[Unit]\nDescription=emobie input helper (text expansion / paste)\nAfter=graphical-session.target\nPartOf=graphical-session.target\n\n[Service]\nType=simple\nExecStart={}\nRestart=on-failure\nRestartSec=2\nNoNewPrivileges=true\nPassEnvironment=WAYLAND_DISPLAY DISPLAY XAUTHORITY XDG_RUNTIME_DIR XKB_DEFAULT_LAYOUT XKB_DEFAULT_MODEL XKB_DEFAULT_VARIANT XKB_DEFAULT_OPTIONS\nUMask=0077\nRuntimeDirectory=emobie\nRuntimeDirectoryMode=0700\nPrivateDevices=no\nPrivateNetwork=yes\nProtectSystem=strict\nProtectHome=read-only\nReadWritePaths=%h/.local/share/emobie\nRestrictAddressFamilies=AF_UNIX\nRestrictNamespaces=yes\nProtectKernelTunables=yes\nProtectKernelModules=yes\nProtectControlGroups=yes\nLockPersonality=yes\nRestrictRealtime=yes\nRestrictSUIDSGID=yes\n\n[Install]\nWantedBy=graphical-session.target\n",
            inputd_bin.display()
        );
        fs::write(unit_dir.join("emobie-inputd.service"), unit).map_err(|e| e.to_string())?;
        let _ = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status();
        let _ = Command::new("systemctl")
            .args(["--user", "enable", "emobie-inputd.service"])
            .status();
        let _ = Command::new("systemctl")
            .args(["--user", "restart", "emobie-inputd.service"])
            .status();
    }
    Ok(())
}

fn install_native_desktop_assets(extracted_root: &Path, launcher: &Path) -> Result<(), String> {
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share"))
        })
        .ok_or_else(|| "HOME is not set".to_string())?;
    let apps = data.join("applications");
    fs::create_dir_all(&apps).map_err(|e| e.to_string())?;
    let desktop_src = extracted_root
        .join("usr/share/applications")
        .join(format!("{ICON_ID}.desktop"));
    let desktop_alt = extracted_root.join("usr/share/applications/emobie.desktop");
    let desktop_dest = apps.join(format!("{ICON_ID}.desktop"));
    let src = if desktop_src.is_file() {
        desktop_src
    } else if desktop_alt.is_file() {
        desktop_alt
    } else {
        return Ok(());
    };
    let mut body = fs::read_to_string(&src).map_err(|e| e.to_string())?;
    body = body
        .lines()
        .map(|line| {
            if line.starts_with("Exec=") {
                format!("Exec={}", launcher.display())
            } else if line.starts_with("Icon=") {
                format!("Icon={ICON_ID}")
            } else if line.starts_with("StartupWMClass=") {
                format!("StartupWMClass={ICON_ID}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    if !body.ends_with('\n') {
        body.push('\n');
    }
    fs::write(&desktop_dest, body).map_err(|e| e.to_string())?;
    let _ = fs::remove_file(apps.join("emobie.desktop"));
    #[cfg(unix)]
    {
        let _ = std::os::unix::fs::symlink(
            format!("{ICON_ID}.desktop"),
            apps.join("emobie.desktop"),
        );
    }
    let icons_src = extracted_root.join("usr/share/icons/hicolor");
    let icons_dest = data.join("icons/hicolor");
    if icons_src.is_dir() {
        for entry in fs::read_dir(&icons_src).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let size_name = entry.file_name();
            let src_apps = entry.path().join("apps");
            if !src_apps.is_dir() {
                continue;
            }
            let dest_apps = icons_dest.join(&size_name).join("apps");
            fs::create_dir_all(&dest_apps).map_err(|e| e.to_string())?;
            for icon in fs::read_dir(&src_apps).map_err(|e| e.to_string())? {
                let icon = icon.map_err(|e| e.to_string())?;
                let name_str = icon.file_name().to_string_lossy().into_owned();
                if name_str != "emobie.png" && name_str != format!("{ICON_ID}.png") {
                    continue;
                }
                let dest = dest_apps.join(format!("{ICON_ID}.png"));
                let _ = fs::copy(icon.path(), &dest);
                #[cfg(unix)]
                {
                    let link = dest_apps.join("emobie.png");
                    let _ = fs::remove_file(&link);
                    let _ = std::os::unix::fs::symlink(format!("{ICON_ID}.png"), link);
                }
            }
        }
    }
    let _ = Command::new("update-desktop-database").arg(&apps).status();
    let _ = Command::new("gtk-update-icon-cache")
        .args(["-f", "-t"])
        .arg(data.join("icons/hicolor"))
        .status();
    Ok(())
}
