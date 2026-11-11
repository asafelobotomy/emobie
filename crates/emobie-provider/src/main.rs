//! Serves emobie's emoji search to uQ's launcher on the session bus and exits after 60 s
//! without a call. The bus starts it again (`org.emobie.Provider.service`) when uQ's launcher
//! opens, so it only runs while it is useful.

use emobie_provider::activate::Clipboard;
use emobie_provider::data::Catalog;
use emobie_provider::inputd::Inputd;
use emobie_provider::prefs::{default_paths, PrefsSource};
use emobie_provider::provider::PASTE_DELAY;
use emobie_provider::{Provider, BUS_NAME, PATH};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

const IDLE: Duration = Duration::from_secs(60);
/// Testing aid: a shorter idle timeout, 1-3600 seconds.
const IDLE_ENV: &str = "EMOBIE_PROVIDER_IDLE_SECS";

fn idle() -> Duration {
    std::env::var(IDLE_ENV)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|s| (1..=3600).contains(s))
        .map(Duration::from_secs)
        .unwrap_or(IDLE)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    if std::env::args().nth(1).is_some_and(|a| a == "--version") {
        println!("emobie-provider {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    let path = Catalog::default_path();
    let catalog = match Catalog::load(&path) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            eprintln!("emobie-provider: {e}");
            return ExitCode::FAILURE;
        }
    };
    let touched = Arc::new(Notify::new());
    let provider = Provider {
        catalog,
        prefs: Arc::new(PrefsSource::new(default_paths())),
        clipboard: Clipboard::default(),
        paster: Arc::new(Inputd::system()),
        paste_delay: PASTE_DELAY,
        touched: touched.clone(),
    };
    let built = zbus::connection::Builder::session()
        .name(BUS_NAME)
        .serve_at(PATH, provider)
        .build()
        .await;
    let _conn = match built {
        Ok(c) => c,
        Err(e) => {
            eprintln!("emobie-provider: cannot own {BUS_NAME} on the session bus: {e}");
            return ExitCode::FAILURE;
        }
    };
    // Exit once a whole idle period passes without a call; the bus starts us again when needed.
    let idle = idle();
    while tokio::time::timeout(idle, touched.notified()).await.is_ok() {}
    ExitCode::SUCCESS
}
