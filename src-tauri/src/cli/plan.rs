//! The plan document of `--dry-run --json`, as uQ's confirmation sheet shows it
//! (uq `docs/settings-pages.md`, "preview and the confirmation sheet").

use serde_json::{json, Value};

pub struct Change {
    pub group: &'static str,
    pub label: String,
    pub detail: String,
    pub reversible: bool,
}

pub struct Plan {
    pub summary: String,
    pub changes: Vec<Change>,
    pub would_refuse: bool,
    pub reason: String,
}

impl Plan {
    pub fn new(summary: impl Into<String>, changes: Vec<Change>) -> Self {
        Self {
            summary: summary.into(),
            changes,
            would_refuse: false,
            reason: String::new(),
        }
    }

    pub fn refuse(reason: impl Into<String>) -> Self {
        Self {
            summary: String::new(),
            changes: Vec::new(),
            would_refuse: true,
            reason: reason.into(),
        }
    }

    pub fn to_json(&self) -> Value {
        json!({
            "summary": self.summary,
            "changes": self.changes.iter().map(|c| json!({
                "group": c.group,
                "label": c.label,
                "detail": c.detail,
                "reversible": c.reversible,
            })).collect::<Vec<_>>(),
            "would_refuse": self.would_refuse,
            "reason": self.reason,
        })
    }
}
