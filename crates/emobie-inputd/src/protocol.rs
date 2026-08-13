use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TriggerMode {
    #[default]
    Immediate,
    Space,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        }
    }

    pub fn err(detail: &str) -> Self {
        Self {
            ok: false,
            daemon: true,
            can_inject: false,
            can_listen: false,
            enabled: false,
            detail: detail.to_string(),
            error: Some(detail.to_string()),
        }
    }
}
