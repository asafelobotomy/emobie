//! GitHub Releases update check and simple package install.

mod apply;

use serde::{Deserialize, Serialize};

pub use apply::InstallKind;

use apply::ApplyUpdateResult;

#[tauri::command]
pub fn apply_update(download_url: String, asset_name: String) -> Result<ApplyUpdateResult, String> {
    apply::apply_update(download_url, asset_name)
}

const REPO: &str = "asafelobotomy/emobie";
const USER_AGENT: &str = concat!("emobie/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub current: String,
    pub latest: Option<String>,
    pub newer_available: bool,
    pub release_url: Option<String>,
    pub detail: String,
    /// Matching asset for this install, when auto-update is possible.
    pub download_url: Option<String>,
    pub asset_name: Option<String>,
    pub install_kind: InstallKind,
    pub can_auto_update: bool,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    draft: bool,
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

fn parse_semver(raw: &str) -> Option<(u64, u64, u64)> {
    let trimmed = raw.trim().trim_start_matches('v');
    let mut parts = trimmed.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts
        .next()
        .unwrap_or("0")
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()?;
    Some((major, minor, patch))
}

fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_semver(latest), parse_semver(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

fn pick_asset<'a>(
    assets: &'a [GithubAsset],
    kind: InstallKind,
) -> Option<&'a GithubAsset> {
    let prefer = match kind {
        InstallKind::Flatpak => [".flatpak"].as_slice(),
        InstallKind::AppImage => [".AppImage"].as_slice(),
        // Native ~/.local installs: extract the binary from the .deb (AppImage
        // often blanks out under WebKit on Wayland).
        InstallKind::Native | InstallKind::Deb => [".deb"].as_slice(),
        InstallKind::Rpm => [".rpm"].as_slice(),
    };
    assets.iter().find(|asset| {
        prefer
            .iter()
            .any(|suffix| asset.name.ends_with(suffix))
            && asset.browser_download_url.starts_with("https://github.com/")
    })
}

fn offline_result(current: String, detail: &str, kind: InstallKind) -> UpdateCheckResult {
    UpdateCheckResult {
        current,
        latest: None,
        newer_available: false,
        release_url: None,
        detail: detail.into(),
        download_url: None,
        asset_name: None,
        install_kind: kind,
        can_auto_update: false,
    }
}

#[tauri::command]
pub fn check_for_updates() -> UpdateCheckResult {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let kind = apply::detect_install_kind();
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");

    let response = ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/vnd.github+json")
        .call();

    let Ok(response) = response else {
        return offline_result(current, "Could not reach GitHub Releases.", kind);
    };

    let Ok(release) = response.into_json::<GithubRelease>() else {
        return offline_result(current, "Unexpected GitHub Releases response.", kind);
    };

    if release.draft || release.prerelease {
        return offline_result(current, "No stable release found.", kind);
    }

    let latest = release.tag_name.trim_start_matches('v').to_string();
    let newer = is_newer(&latest, &current);
    let asset = if newer {
        pick_asset(&release.assets, kind)
    } else {
        None
    };
    let can_auto = asset.is_some();

    UpdateCheckResult {
        newer_available: newer,
        release_url: Some(release.html_url),
        detail: if newer {
            if can_auto {
                format!("Update available: v{latest} — you can install it here")
            } else {
                format!("Update available: v{latest}")
            }
        } else {
            format!("Up to date (v{current})")
        },
        download_url: asset.map(|a| a.browser_download_url.clone()),
        asset_name: asset.map(|a| a.name.clone()),
        install_kind: kind,
        can_auto_update: can_auto,
        latest: Some(latest),
        current,
    }
}

#[tauri::command]
pub fn open_release_page(url: String) -> Result<(), String> {
    if !(url.starts_with("https://github.com/asafelobotomy/emobie/")
        || url.starts_with("https://github.com/asafelobotomy/emobie"))
    {
        return Err("Refusing to open unexpected URL.".into());
    }
    open::that(url).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::{is_newer, parse_semver};

    #[test]
    fn parses_semver_tags() {
        assert_eq!(parse_semver("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_semver("0.6.4"), Some((0, 6, 4)));
    }

    #[test]
    fn compares_versions() {
        assert!(is_newer("0.7.0", "0.6.4"));
        assert!(!is_newer("0.6.4", "0.6.4"));
        assert!(!is_newer("0.6.3", "0.6.4"));
    }
}
