use tauri::{AppHandle, Manager, State, WindowEvent};

mod autostart;
mod prefs;
mod tray;

const FLATPAK_APP_ID: &str = "io.github.asafelobotomy.Emobie";

pub struct TrayAvailable(pub bool);

#[tauri::command]
fn is_tray_available(tray: State<'_, TrayAvailable>) -> bool {
    tray.0
}

#[tauri::command]
fn release_single_instance_lock(app: AppHandle) {
    tauri_plugin_single_instance::destroy(&app);
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

fn apply_startup_visibility(app: &tauri::App, tray_ok: bool) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let start_minimized = prefs::start_minimized_to_tray();
    if start_minimized && tray_ok {
        let _ = window.hide();
    } else {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let allow_multiple = prefs::allow_multiple_instances();

    let mut builder = tauri::Builder::default();

    if !allow_multiple {
        let mut single = tauri_plugin_single_instance::Builder::new().callback(
            |app, _argv, _cwd| {
                tray::show_main_window(app);
            },
        );
        if std::env::var_os("FLATPAK_ID").is_some() {
            single = single.dbus_id(FLATPAK_APP_ID);
        }
        builder = builder.plugin(single.build());
    }

    builder
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(move |app| {
            #[cfg(desktop)]
            {
                let tray_ok = tray::setup(app, allow_multiple);
                app.manage(TrayAvailable(tray_ok));
                apply_startup_visibility(app, tray_ok);
            }
            #[cfg(not(desktop))]
            {
                app.manage(TrayAvailable(false));
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let tray_ok = window
                    .app_handle()
                    .try_state::<TrayAvailable>()
                    .map(|state| state.0)
                    .unwrap_or(false);
                if tray_ok {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    window.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            autostart::set_launch_on_startup,
            autostart::is_launch_on_startup,
            is_tray_available,
            release_single_instance_lock,
            quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running emobie");
}
