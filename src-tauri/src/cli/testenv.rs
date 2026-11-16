//! Tests point HOME at a temp dir, so they run one at a time.

use std::path::Path;
use std::sync::Mutex;

static ENV: Mutex<()> = Mutex::new(());

pub fn with_home<T>(name: &str, f: impl FnOnce(&Path) -> T) -> T {
    let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("emobie-cli-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_var("HOME", &dir);
    std::env::remove_var("XDG_DATA_HOME");
    let out = f(&dir);
    let _ = std::fs::remove_dir_all(&dir);
    out
}
