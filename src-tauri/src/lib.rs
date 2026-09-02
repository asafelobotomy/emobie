use tauri::{AppHandle, Manager, State, WindowEvent};

mod autostart;
mod input_helper;
mod pin;
mod prefs;
mod tray;
mod updates;

const FLATPAK_APP_ID: &str = "io.github.asafelobotomy.emobie";

pub struct TrayAvailable(pub tray::TrayStatus);

#[tauri::command]
fn is_tray_available(tray: State<'_, TrayAvailable>) -> bool {
    tray.0.available
}

#[tauri::command]
fn tray_status(tray: State<'_, TrayAvailable>) -> tray::TrayStatus {
    tray.0.clone()
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
        pin::apply_from_prefs(&window);
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
                let status = tray::setup(app, allow_multiple);
                let tray_ok = status.available;
                app.manage(TrayAvailable(status));
                apply_startup_visibility(app, tray_ok);
            }
            #[cfg(not(desktop))]
            {
                app.manage(TrayAvailable(tray::TrayStatus {
                    available: false,
                    detail: "Tray is desktop-only.".into(),
                }));
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let tray_ok = window
                    .app_handle()
                    .try_state::<TrayAvailable>()
                    .map(|state| state.0.available)
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
            tray_status,
            release_single_instance_lock,
            quit_app,
            pin::apply_window_pin,
            pin::pin_capability,
            input_helper::input_helper_status,
            input_helper::input_helper_ensure_started,
            input_helper::input_helper_set_enabled,
            input_helper::input_helper_sync_matches,
            input_helper::input_helper_set_options,
            input_helper::input_helper_inject_paste,
            input_helper::input_helper_run_access_setup,
            updates::check_for_updates,
            updates::open_release_page,
            updates::apply_update,
            prefs::load_preference_snapshots,
            prefs::save_durable_preferences,
        ])
        .run(tauri::generate_context!())
        .expect("error while running emobie");
}
