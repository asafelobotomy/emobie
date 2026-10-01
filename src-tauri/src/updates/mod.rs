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
    if flathub_build() {
        return Err("This copy updates through Flathub (your software center).".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        apply::apply_update(release_tag, download_url, asset_name)
    })
    .await
    .map_err(|err| format!("update task failed: {err}"))?
}

const REPO: &str = "asafelobotomy/emobie";
/// Release asset listing `sha256sum`-format hashes for every package.
const CHECKSUM_ASSET: &str = "SHA256SUMS";
/// Minisign signature over `SHA256SUMS`, made in CI with the release key.
const SIGNATURE_ASSET: &str = "SHA256SUMS.minisig";
const MAX_CHECKSUM_BYTES: u64 = 64 * 1024;
const MAX_SIGNATURE_BYTES: u64 = 4 * 1024;
/// Public half of the release signing key (minisign `.pub` file). A GitHub
/// release alone cannot vouch for itself: without this check, whoever can
/// publish a release could ship matching checksums and get root via pkexec.
const UPDATE_PUBLIC_KEY: &str = include_str!("../../update-signing.pub");
const USER_AGENT: &str = concat!("emobie/", env!("CARGO_PKG_VERSION"));
/// Set by the Flathub manifest. Flathub ships updates itself; installing a
/// GitHub bundle over it would switch the app's origin.
fn flathub_build() -> bool {
    option_env!("EMOBIE_DISTRIBUTION") == Some("flathub")
}

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

fn named_asset<'a>(assets: &'a [GithubAsset], name: &str) -> Option<&'a GithubAsset> {
    assets.iter().find(|asset| {
        asset.name == name && asset.browser_download_url.starts_with(apply::ALLOWED_PREFIX)
    })
}

/// The embedded release key, or `None` while `update-signing.pub` still holds
/// the placeholder — one-click install then stays off.
fn update_public_key() -> Option<minisign_verify::PublicKey> {
    minisign_verify::PublicKey::decode(UPDATE_PUBLIC_KEY.trim()).ok()
}

/// One-click install needs a configured key and a signed checksum file.
fn release_is_signed(assets: &[GithubAsset]) -> bool {
    update_public_key().is_some()
        && named_asset(assets, CHECKSUM_ASSET).is_some()
        && named_asset(assets, SIGNATURE_ASSET).is_some()
}

/// Verify `sums` against `signature` with `key`. The trusted comment must name
/// this exact tag, so a validly signed checksum file from another release
/// cannot be replayed.
pub(crate) fn verify_signed_sums(
    key: &minisign_verify::PublicKey,
    sums: &str,
    signature: &str,
    tag: &str,
) -> Result<(), String> {
    let signature = minisign_verify::Signature::decode(signature.trim())
        .map_err(|_| format!("{SIGNATURE_ASSET} is malformed."))?;
    key.verify(sums.as_bytes(), &signature, false)
        .map_err(|_| format!("{CHECKSUM_ASSET} signature does not verify — not installing."))?;
    if signature.trusted_comment() != format!("emobie {tag}") {
        return Err(format!("{SIGNATURE_ASSET} was made for a different release — not installing."));
    }
    Ok(())
}

fn fetch_text(url: &str, max_bytes: u64, what: &str) -> Result<String, String> {
    let response = http_agent()
        .get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|_| format!("Could not download the release {what}."))?;
    let mut text = String::new();
    response
        .into_reader()
        .take(max_bytes)
        .read_to_string(&mut text)
        .map_err(|_| format!("Could not read the release {what}."))?;
    Ok(text)
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
    let key = update_public_key().ok_or_else(|| {
        "This build has no update signing key; use “Open release” to install manually.".to_string()
    })?;
    let manual = "use “Open release” to install manually.";
    let sums = named_asset(&release.assets, CHECKSUM_ASSET)
        .ok_or_else(|| format!("This release publishes no {CHECKSUM_ASSET}; {manual}"))?;
    let sig = named_asset(&release.assets, SIGNATURE_ASSET)
        .ok_or_else(|| format!("This release is not signed; {manual}"))?;
    let text = fetch_text(&sums.browser_download_url, MAX_CHECKSUM_BYTES, "checksums")?;
    let signature = fetch_text(&sig.browser_download_url, MAX_SIGNATURE_BYTES, "signature")?;
    verify_signed_sums(&key, &text, &signature, &release.tag_name)?;
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
    // Only offer one-click install when the release ships signed checksums to
    // verify the download against.
    let asset = if newer && !flathub_build() && release_is_signed(&release.assets) {
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
            } else if flathub_build() {
                format!("Update available: v{latest} — it will arrive through Flathub")
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
    use super::{is_newer, normalize_release_tag, parse_semver, parse_sha256sums, verify_signed_sums};

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

    /// Minisign's own published test vector (key + signature over "test").
    const TEST_PUB: &str = "untrusted comment: minisign public key E7620F1842B4E81F
RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
    const TEST_SIG: &str = "untrusted comment: signature from minisign secret key
RWQf6LRCGA9i59SLOFxz6NxvASXDJeRtuZykwQepbDEGt87ig1BNpWaVWuNrm73YiIiJbq71Wi+dP9eKL8OC351vwIasSSbXxwA=
trusted comment: timestamp:1555779966\tfile:test
QtKMXWyYcwdpZAlPF7tE2ENJkRd1ujvKjlj1m9RtHTBnZPa5WKU5uWRs5GoP5M/VqE81QFuMKI5k/SfNQUaOAA==";

    #[test]
    fn signed_sums_reject_tampering_and_legacy() {
        let key = minisign_verify::PublicKey::decode(TEST_PUB).unwrap();
        // That vector is a legacy (non-prehashed) signature: refused outright,
        // as is anything with a wrong body.
        assert!(verify_signed_sums(&key, "test", TEST_SIG, "v1.0.0").is_err());
        assert!(verify_signed_sums(&key, "Test", TEST_SIG, "v1.0.0").is_err());
        assert!(verify_signed_sums(&key, "test", "garbage", "v1.0.0").is_err());
    }

    /// Throwaway test key (secret = bytes 0..32) and prehashed signatures in
    /// the format the minisign CLI writes, over one SHA256SUMS line.
    const SIGNED_PUB: &str = "untrusted comment: minisign public key TEST
RWQBAgMEBQYHCAOhB7/zzhC+HXDdGOdLwJln5NYwm6UNXx3chmQSVTG4";
    const SIGNED_SUMS: &str =
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  emobie_9.9.9_amd64.deb\n";
    const SIG_FOR_V999: &str = "untrusted comment: signature from minisign secret key
RUQBAgMEBQYHCGIIvHPLY01ku5lDhwAkDtVpbLreYHK/niKYiup6k98WI7hypN/IbBbPBJQqXybkpc0lJi73BRWz9hQsHEK9CQs=
trusted comment: emobie v9.9.9
qnj56sdyeOjUBpaFsZpqPwHXkHCw5hZrlF9mz4FmnNwf/W2SDzGrDpHZwdKc5yw4T3e+fg2hVS7lrTBj3IdGAQ==";
    const SIG_FOR_V998: &str = "untrusted comment: signature from minisign secret key
RUQBAgMEBQYHCGIIvHPLY01ku5lDhwAkDtVpbLreYHK/niKYiup6k98WI7hypN/IbBbPBJQqXybkpc0lJi73BRWz9hQsHEK9CQs=
trusted comment: emobie v9.9.8
J3bMI9JP5Z14KgtzhZC/++kuCnHTSW6E2+5Mm5au6IjOzvPFtS3zCPl0cNtVGS+JMA8bKRAH4nNApD4QJ1u9AA==";

    #[test]
    fn signed_sums_verify_only_for_their_own_tag() {
        let key = minisign_verify::PublicKey::decode(SIGNED_PUB).unwrap();
        assert_eq!(verify_signed_sums(&key, SIGNED_SUMS, SIG_FOR_V999, "v9.9.9"), Ok(()));
        // Valid signature, but made for another release: replay refused.
        assert!(verify_signed_sums(&key, SIGNED_SUMS, SIG_FOR_V998, "v9.9.9").is_err());
        // One changed hash byte breaks it.
        let tampered = SIGNED_SUMS.replacen('0', "1", 1);
        assert!(verify_signed_sums(&key, &tampered, SIG_FOR_V999, "v9.9.9").is_err());
        // A different key is refused.
        let other = minisign_verify::PublicKey::decode(TEST_PUB).unwrap();
        assert!(verify_signed_sums(&other, SIGNED_SUMS, SIG_FOR_V999, "v9.9.9").is_err());
    }

    /// Produced by the minisign 0.12 CLI exactly as release.yml signs
    /// (`minisign -S -m SHA256SUMS -t "emobie v9.9.9"`), with a throwaway key.
    #[test]
    fn minisign_cli_signature_verifies() {
        let key = minisign_verify::PublicKey::decode(
            "untrusted comment: minisign public key DBC84A1B4EB82252
RWRSIrhOG0rI2+2Yg3kftSUzRrE8i0IcBBBX2eCFm2OyAP1veoUDld3U",
        )
        .unwrap();
        let sig = "untrusted comment: signature from minisign secret key
RURSIrhOG0rI29cCW9diWYLZPpdZUhKuDZNjewpIJPy2bHjpsFuphdIhe4mUr7c6DRk0oYBF+XMrRLADkvmfYjpBJ0o7N1krqQg=
trusted comment: emobie v9.9.9
SqPPEWNXaVw3wFfhw0MxDGyXysmi03GdTkHsoghMUJJKC4WC5IPcg99HVQ0l4LQyPQfHRk/co1RqENNls4s8CQ==";
        let sums = "abc  emobie_9.9.9_amd64.deb\n";
        assert_eq!(verify_signed_sums(&key, sums, sig, "v9.9.9"), Ok(()));
        assert!(verify_signed_sums(&key, sums, sig, "v9.9.10").is_err());
    }

    #[test]
    fn placeholder_key_disables_one_click_install() {
        // Holds until the real key is committed; then this documents that the
        // shipped key parses.
        let configured = super::update_public_key().is_some();
        let placeholder = super::UPDATE_PUBLIC_KEY.contains("NOT CONFIGURED");
        assert_eq!(configured, !placeholder);
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
