//! Privileged setup assets, embedded at compile time.
//!
//! The setup script, udev rule, polkit policy and SELinux module are executed
//! or installed *as root*. They must therefore never be taken from a
//! user-writable location (`~/.local/share/emobie`, the AppImage mount, the
//! build tree...): any process running as the user could replace them and wait
//! for the next Grant prompt. Instead the exact bytes this build was compiled
//! with are staged into root-owned paths.

pub(super) const SETUP_SCRIPT: &[u8] =
    include_bytes!("../../../../packaging/setup-input-access.sh");
pub(super) const UDEV_RULES: &[u8] =
    include_bytes!("../../../../packaging/udev/99-emobie-input.rules");
pub(super) const POLKIT_POLICY: &[u8] = include_bytes!(
    "../../../../packaging/polkit/io.github.asafelobotomy.emobie.inputd.policy"
);
pub(super) const SELINUX_TE: &[u8] =
    include_bytes!("../../../../packaging/selinux/emobie-inputd.te");

pub(super) const POLKIT_POLICY_NAME: &str = "io.github.asafelobotomy.emobie.inputd.policy";
pub(super) const UDEV_RULES_NAME: &str = "99-emobie-input.rules";

/// (bytes, install mode, path relative to the staging directory).
pub(super) const STAGED_FILES: [(&[u8], &str, &str); 4] = [
    (SETUP_SCRIPT, "755", "setup-input-access.sh"),
    (UDEV_RULES, "644", UDEV_RULES_NAME),
    (POLKIT_POLICY, "644", POLKIT_POLICY_NAME),
    (SELINUX_TE, "644", "selinux/emobie-inputd.te"),
];
