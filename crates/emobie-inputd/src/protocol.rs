use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TriggerMode {
    Immediate,
    #[default]
    Space,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchRule {
    pub trigger: String,
    pub expansion: String,
    #[serde(default)]
    pub mode: TriggerMode,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Request {
    Status,
    SetEnabled { enabled: bool },
    SyncMatches { matches: Vec<MatchRule> },
    /// Update inject options without touching matches.
    SetOptions {
        #[serde(default)]
        restore_clipboard: Option<bool>,
    },
    InjectPaste,
}

#[derive(Debug, Serialize)]
pub struct Response {
    pub ok: bool,
    pub daemon: bool,
    pub can_inject: bool,
    pub can_listen: bool,
    pub enabled: bool,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress_jobs: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restore_clipboard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_inject_backend: Option<String>,
}

impl Response {
    pub fn status(can_inject: bool, can_listen: bool, enabled: bool, detail: &str) -> Self {
        Self {
            ok: true,
            daemon: true,
            can_inject,
            can_listen,
            enabled,
            detail: detail.to_string(),
            error: None,
            suppress_jobs: Some(crate::inject::suppress_job_count()),
            restore_clipboard: Some(crate::inject::restore_clipboard_enabled()),
            last_inject_backend: crate::inject::last_inject_backend().map(|s| s.to_string()),
        }
    }

    /// Error that preserves live capability flags so clients do not look "offline".
    pub fn err(
        can_inject: bool,
        can_listen: bool,
        enabled: bool,
        detail: &str,
    ) -> Self {
        Self {
            ok: false,
            daemon: true,
            can_inject,
            can_listen,
            enabled,
            detail: detail.to_string(),
            error: Some(detail.to_string()),
            suppress_jobs: Some(crate::inject::suppress_job_count()),
            restore_clipboard: Some(crate::inject::restore_clipboard_enabled()),
            last_inject_backend: crate::inject::last_inject_backend().map(|s| s.to_string()),
        }
    }
}
