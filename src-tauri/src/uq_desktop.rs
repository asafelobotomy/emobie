//! Whether uQ is the running desktop: `org.uq.Desktop1` has an owner on the session bus. uQ
//! binds Super+. to its own emoji search (emobie-provider, see uq/README.md), so the app leaves
//! its summon hotkey off there (src/lib/uqDesktop.ts).

const UQ_DESKTOP_NAME: &str = "org.uq.Desktop1";

/// zbus 6 has no blocking API: `zbus::block_on` drives the connection on this thread (callers are
/// on `spawn_blocking`, never inside an async task).
fn desktop_active() -> Result<bool, String> {
    zbus::block_on(async {
        let conn = zbus::Connection::session().await?;
        let dbus = zbus::fdo::DBusProxy::new(&conn).await?;
        let name = zbus::names::BusName::try_from(UQ_DESKTOP_NAME)?;
        Ok::<_, zbus::Error>(dbus.name_has_owner(name).await?)
    })
    .map_err(|e| e.to_string())
}

/// False when there is no session bus (or inside a sandbox that hides the name).
#[tauri::command]
pub async fn uq_desktop_active() -> bool {
    tauri::async_runtime::spawn_blocking(|| desktop_active().unwrap_or(false))
        .await
        .unwrap_or(false)
}
