//! Signed `SHA256SUMS` verification for one-click updates.

use std::io::Read;

use super::{apply, http_agent, GithubAsset, GithubRelease, USER_AGENT};

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
pub(super) fn release_is_signed(assets: &[GithubAsset]) -> bool {
    update_public_key().is_some()
        && named_asset(assets, CHECKSUM_ASSET).is_some()
        && named_asset(assets, SIGNATURE_ASSET).is_some()
}

/// Verify `sums` against `signature` with `key`. The trusted comment must name
/// this exact tag, so a validly signed checksum file from another release
/// cannot be replayed.
pub(super) fn verify_signed_sums(
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

/// Find `asset_name`'s hash in `sha256sum`-format text (`<hex>  <name>`).
pub(super) fn parse_sha256sums(text: &str, asset_name: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (hash, rest) = line.trim().split_once(char::is_whitespace)?;
        let name = rest.trim_start().trim_start_matches('*');
        (name == asset_name && hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| hash.to_ascii_lowercase())
    })
}

pub(super) fn fetch_expected_sha256(release: &GithubRelease, asset_name: &str) -> Result<String, String> {
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

#[cfg(test)]
mod tests {
    use super::{parse_sha256sums, verify_signed_sums};

    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

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
}
