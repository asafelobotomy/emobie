//! `org.uq.Launcher1.Provider` at `/org/uq/Launcher1`, as uQ's launcher bridge calls it:
//!
//! - `Search(s query) -> a(sssssd)`: up to 20 rows of (id, title, subtitle, icon, actions as a
//!   JSON list, score 0-1). uQ strips the `:` prefix before it asks.
//! - `Activate(s id, s action)`: the id is the emoji itself; it must be one from the catalog.

use crate::activate::{Action, Clipboard, Paster, ACTIONS_JSON, MAX_ID_BYTES};
use crate::data::Catalog;
use crate::prefs::PrefsSource;
use crate::search;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

pub const BUS_NAME: &str = "org.emobie.Provider";
pub const PATH: &str = "/org/uq/Launcher1";
pub const INTERFACE: &str = "org.uq.Launcher1.Provider";
/// emobie's icon, installed by its package.
pub const ICON: &str = "io.github.asafelobotomy.emobie";
/// Let uQ close Spotlight and focus return to the previous window before pasting.
pub const PASTE_DELAY: Duration = Duration::from_millis(250);

/// `(id, title, subtitle, icon, actions as a JSON list, score)`.
pub type Row = (String, String, String, String, String, f64);

pub struct Provider {
    pub catalog: Arc<Catalog>,
    pub prefs: Arc<PrefsSource>,
    pub clipboard: Clipboard,
    pub paster: Arc<dyn Paster>,
    pub paste_delay: Duration,
    /// Poked on every call so the idle timer starts over.
    pub touched: Arc<Notify>,
}

impl Provider {
    pub fn rows(&self, query: &str) -> Vec<Row> {
        let prefs = self.prefs.get();
        search::search(&self.catalog, &prefs, query)
            .into_iter()
            .map(|h| {
                (
                    h.emoji.clone(),
                    h.emoji,
                    h.name,
                    ICON.into(),
                    ACTIONS_JSON.into(),
                    h.score,
                )
            })
            .collect()
    }

    /// The emoji an id names, if it is one from the catalog.
    pub fn emoji_for(&self, id: &str) -> Option<String> {
        (id.len() <= MAX_ID_BYTES && self.catalog.by_char(id).is_some()).then(|| id.to_string())
    }
}

fn invalid(msg: impl Into<String>) -> zbus::fdo::Error {
    zbus::fdo::Error::InvalidArgs(msg.into())
}

#[zbus::interface(name = "org.uq.Launcher1.Provider")]
impl Provider {
    async fn search(&self, query: String) -> Vec<Row> {
        self.touched.notify_one();
        self.rows(&query)
    }

    async fn activate(&self, id: String, action: String) -> zbus::fdo::Result<()> {
        self.touched.notify_one();
        let action = Action::parse(&action).map_err(invalid)?;
        let emoji = self.emoji_for(&id).ok_or_else(|| invalid("unknown item"))?;

        let clipboard = self.clipboard.clone();
        let text = emoji.clone();
        tokio::task::spawn_blocking(move || clipboard.copy(&text))
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?
            .map_err(zbus::fdo::Error::Failed)?;

        // `type` without a running helper stays a copy: the emoji is already on the clipboard.
        if action == Action::Type && self.paster.available() {
            let paster = self.paster.clone();
            let delay = self.paste_delay;
            tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                let result = tokio::task::spawn_blocking(move || paster.paste()).await;
                if let Ok(Err(e)) = result {
                    eprintln!(
                        "emobie-provider: paste failed, the emoji stays on the clipboard: {e}"
                    );
                }
            });
        }
        Ok(())
    }
}
