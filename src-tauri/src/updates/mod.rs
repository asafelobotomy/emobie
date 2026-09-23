//! GitHub Releases update check and simple package install.

mod apply;
mod native;
mod verified_install;

use serde::{Deserialize, Serialize};
use std::io::Read;

pub use apply::InstallKind;

use apply::ApplyUpdateResult;

#[tauri::command]
pub async fn apply_update(
    release_tag: String,
    download_url: String,
    asset_name: String,
) -> Result<ApplyUpdateResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        apply::apply_update(release_tag, download_url, asset_name)
    })
    .await
    .map_err(|err| format!("update task failed: {err}"))?
}

const REPO: &str = "asafelobotomy/emobie";
/// Release asset listing `sha256sum`-format hashes for every package.
const CHECKSUM_ASSET: &str = "SHA256SUMS";
const MAX_CHECKSUM_BYTES: u64 = 64 * 1024;
const USER_AGENT: &str = concat!("emobie/", env!("CARGO_PKG_VERSION"));

/// ureq's default agent has no read timeout, so a stalled connection would
/// hang forever. The read timeout is per socket read, so large downloads
/// still work as long as bytes keep arriving.
pub(crate) fn http_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(10))
        .timeout_read(std::time::Duration::from_secs(30))
        .timeout_write(std::time::Duration::from_secs(30))
        .build()
}

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

fn pick_asset(assets: &[GithubAsset], kind: InstallKind) -> Option<&GithubAsset> {
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
            && asset.browser_download_url.starts_with(apply::ALLOWED_PREFIX)
    })
}

fn checksum_asset(assets: &[GithubAsset]) -> Option<&GithubAsset> {
    assets.iter().find(|asset| {
        asset.name == CHECKSUM_ASSET && asset.browser_download_url.starts_with(apply::ALLOWED_PREFIX)
    })
}

/// Accept only plain `X.Y.Z` / `vX.Y.Z` tags. The tag is interpolated into an
/// API URL, so anything else (`..`, `?`, `/`, prerelease suffixes) is refused.
pub(crate) fn normalize_release_tag(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    let bare = trimmed.strip_prefix('v').unwrap_or(trimmed);
    let parts: Vec<&str> = bare.split('.').collect();
    let valid = parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 6 && p.bytes().all(|b| b.is_ascii_digit()));
    if !valid {
        return Err("Invalid release tag.".into());
    }
    Ok(format!("v{bare}"))
}

/// Find `asset_name`'s hash in `sha256sum`-format text (`<hex>  <name>`).
pub(crate) fn parse_sha256sums(text: &str, asset_name: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (hash, rest) = line.trim().split_once(char::is_whitespace)?;
        let name = rest.trim_start().trim_start_matches('*');
        (name == asset_name && hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| hash.to_ascii_lowercase())
    })
}

fn fetch_expected_sha256(release: &GithubRelease, asset_name: &str) -> Result<String, String> {
    let sums = checksum_asset(&release.assets).ok_or_else(|| {
        format!("This release publishes no {CHECKSUM_ASSET}; use “Open release” to install manually.")
    })?;
    let response = http_agent().get(&sums.browser_download_url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|_| "Could not download the release checksums.".to_string())?;
    let mut text = String::new();
    response
        .into_reader()
        .take(MAX_CHECKSUM_BYTES)
        .read_to_string(&mut text)
        .map_err(|_| "Could not read the release checksums.".to_string())?;
    parse_sha256sums(&text, asset_name)
        .ok_or_else(|| format!("{CHECKSUM_ASSET} has no entry for {asset_name}."))
}

/// Ensure the frontend-provided asset matches a real, newer, stable release on
/// GitHub, and return the expected SHA-256 of that asset from the release's
/// `SHA256SUMS`.
pub(crate) fn verify_update_asset(
    release_tag: &str,
    download_url: &str,
    asset_name: &str,
    kind: InstallKind,
) -> Result<String, String> {
    let tag = normalize_release_tag(release_tag)?;
    if !is_newer(&tag, env!("CARGO_PKG_VERSION")) {
        return Err("Refusing to install a version that is not newer than the running one.".into());
    }
    let url = format!("https://api.github.com/repos/{REPO}/releases/tags/{tag}");
    let response = http_agent().get(&url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|_| "Could not verify release on GitHub.".to_string())?;
    let release = response
        .into_json::<GithubRelease>()
        .map_err(|_| "Unexpected GitHub Releases response.".to_string())?;
    if release.draft || release.prerelease {
        return Err("Refusing to install draft or prerelease.".into());
    }
    if release.tag_name != tag {
        return Err("Release tag mismatch.".into());
    }
    let asset = pick_asset(&release.assets, kind)
        .ok_or_else(|| "No matching asset for this install type.".to_string())?;
    if asset.browser_download_url != download_url || asset.name != asset_name {
        return Err(
            "Update metadata mismatch — check for updates again before installing.".into(),
        );
    }
    fetch_expected_sha256(&release, asset_name)
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
pub async fn check_for_updates() -> UpdateCheckResult {
    tauri::async_runtime::spawn_blocking(check_for_updates_blocking)
        .await
        .unwrap_or_else(|_| {
            offline_result(
                env!("CARGO_PKG_VERSION").to_string(),
                "Update check failed.",
                apply::detect_install_kind(),
            )
        })
}

fn check_for_updates_blocking() -> UpdateCheckResult {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let kind = apply::detect_install_kind();
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");

    let response = http_agent().get(&url)
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
    // Only offer one-click install when the release ships checksums to verify
    // the download against.
    let asset = if newer && checksum_asset(&release.assets).is_some() {
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
    const BASE: &str = "https://github.com/asafelobotomy/emobie";
    let allowed = url == BASE
        || url
            .strip_prefix(BASE)
            .is_some_and(|rest| rest.starts_with('/') || rest.starts_with('?') || rest.starts_with('#'));
    if !allowed || url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("Refusing to open unexpected URL.".into());
    }
    open::that(url).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::{is_newer, normalize_release_tag, parse_semver, parse_sha256sums};

    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn tag_normalization_rejects_path_tricks() {
        assert_eq!(normalize_release_tag("0.7.1").as_deref(), Ok("v0.7.1"));
        assert_eq!(normalize_release_tag(" v0.7.1 ").as_deref(), Ok("v0.7.1"));
        for bad in ["", "v1.2", "v1.2.3.4", "../latest", "v1.2.3/../x", "v1.2.3?x=1", "v1.2.3-rc1", "v1.2.x"] {
            assert!(normalize_release_tag(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn sha256sums_parsing() {
        let text = format!("{HASH}  emobie_0.7.1_amd64.deb\n{HASH}  other.rpm\n");
        assert_eq!(parse_sha256sums(&text, "emobie_0.7.1_amd64.deb").as_deref(), Some(HASH));
        assert_eq!(parse_sha256sums(&format!("{HASH} *x.deb"), "x.deb").as_deref(), Some(HASH));
        assert_eq!(parse_sha256sums(&text, "missing.deb"), None);
        assert_eq!(parse_sha256sums("nothex  x.deb", "x.deb"), None);
    }

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
