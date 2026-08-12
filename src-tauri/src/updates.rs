//! Check GitHub Releases for a newer Emobie version.

use serde::{Deserialize, Serialize};

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
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    draft: bool,
    prerelease: bool,
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

#[tauri::command]
pub fn check_for_updates() -> UpdateCheckResult {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");

    let response = ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/vnd.github+json")
        .call();

    let Ok(response) = response else {
        return UpdateCheckResult {
            current,
            latest: None,
            newer_available: false,
            release_url: None,
            detail: "Could not reach GitHub Releases.".into(),
        };
    };

    let Ok(release) = response.into_json::<GithubRelease>() else {
        return UpdateCheckResult {
            current,
            latest: None,
            newer_available: false,
            release_url: None,
            detail: "Unexpected GitHub Releases response.".into(),
        };
    };

    if release.draft || release.prerelease {
        return UpdateCheckResult {
            current,
            latest: None,
            newer_available: false,
            release_url: None,
            detail: "No stable release found.".into(),
        };
    }

    let latest = release.tag_name.trim_start_matches('v').to_string();
    let newer = is_newer(&latest, &current);
    UpdateCheckResult {
        newer_available: newer,
        release_url: Some(release.html_url),
        detail: if newer {
            format!("Update available: v{latest}")
        } else {
            format!("Up to date (v{current})")
        },
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
