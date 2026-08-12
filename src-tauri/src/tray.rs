//! System tray setup (Linux ksni + other platforms via Tauri tray).

use serde::Serialize;
use tauri::{AppHandle, Manager};

pub const PIN_EVENT: &str = "tray-pin-toggle";
const FLATPAK_APP_ID: &str = "io.github.asafelobotomy.Emobie";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrayStatus {
    pub available: bool,
    pub detail: String,
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::{hide_main_window, show_main_window, FLATPAK_APP_ID, PIN_EVENT};
    use image::GenericImageView;
    use ksni::blocking::TrayMethods;
    use ksni::{Category, Icon, MenuItem, ToolTip, Tray};
    use tauri::{AppHandle, Emitter, Manager};

    struct EmobieTray {
        app: AppHandle,
        icons: Vec<Icon>,
        id: String,
    }

    fn load_icons() -> Vec<Icon> {
        const PNG: &[u8] = include_bytes!("../icons/64x64.png");
        let Ok(img) = image::load_from_memory(PNG) else {
            return Vec::new();
        };
        let (width, height) = img.dimensions();
        let mut data = img.into_rgba8().into_vec();
        for pixel in data.chunks_exact_mut(4) {
            pixel.rotate_right(1); // RGBA → ARGB
        }
        vec![Icon {
            width: width as i32,
            height: height as i32,
            data,
        }]
    }

    fn tray_id(unique: bool) -> String {
        if unique {
            format!("{FLATPAK_APP_ID}-{}", std::process::id())
        } else {
            FLATPAK_APP_ID.into()
        }
    }

    fn in_flatpak() -> bool {
        std::env::var_os("FLATPAK_ID").is_some()
    }

    impl Tray for EmobieTray {
        fn id(&self) -> String {
            self.id.clone()
        }

        fn category(&self) -> Category {
            Category::ApplicationStatus
        }

        fn title(&self) -> String {
            "emobie".into()
        }

        fn icon_pixmap(&self) -> Vec<Icon> {
            self.icons.clone()
        }

        fn tool_tip(&self) -> ToolTip {
            ToolTip {
                title: "emobie".into(),
                description: "Emoji palette".into(),
                ..Default::default()
            }
        }

        fn activate(&mut self, _x: i32, _y: i32) {
            show_main_window(&self.app);
        }

        fn menu(&self) -> Vec<MenuItem<Self>> {
            use ksni::menu::*;
            vec![
                StandardItem {
                    label: "Show emobie".into(),
                    activate: Box::new(|this: &mut Self| show_main_window(&this.app)),
                    ..Default::default()
                }
                .into(),
                StandardItem {
                    label: "Hide".into(),
                    activate: Box::new(|this: &mut Self| hide_main_window(&this.app)),
                    ..Default::default()
                }
                .into(),
                MenuItem::Separator,
                StandardItem {
                    label: "Toggle pin above windows".into(),
                    activate: Box::new(|this: &mut Self| {
                        let _ = this.app.emit(PIN_EVENT, ());
                    }),
                    ..Default::default()
                }
                .into(),
                MenuItem::Separator,
                StandardItem {
                    label: "Quit".into(),
                    activate: Box::new(|this: &mut Self| {
                        this.app.exit(0);
                    }),
                    ..Default::default()
                }
                .into(),
            ]
        }
    }

    #[allow(dead_code)]
    pub struct TrayGuard(ksni::blocking::Handle<EmobieTray>);

    pub fn setup(app: &tauri::App, unique_id: bool) -> Result<(), String> {
        let handle = app.handle().clone();
        let icons = load_icons();
        if icons.is_empty() {
            return Err("failed to decode tray icon".into());
        }

        let tray = EmobieTray {
            app: handle,
            icons,
            id: tray_id(unique_id),
        };

        // Flatpak Flathub cannot own StatusNotifierItem-* well-known names.
        // Cinnamon/Mint (xapp-sn-watcher) still hosts the item without own-name.
        let sandboxed = in_flatpak();
        let tray_handle = tray
            .disable_dbus_name(sandboxed)
            .assume_sni_available(true)
            .spawn()
            .map_err(|err| {
                format!(
                    "{err} (Cinnamon: enable System Tray applet; Mint uses xapp-sn-watcher)"
                )
            })?;

        app.manage(TrayGuard(tray_handle));
        Ok(())
    }
}

#[cfg(not(target_os = "linux"))]
fn setup_tray_tauri(app: &tauri::App) -> tauri::Result<()> {
    use super::{hide_main_window, show_main_window, PIN_EVENT};
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    use tauri::{
        menu::{Menu, MenuItem, PredefinedMenuItem},
        Emitter,
    };

    let show_item = MenuItem::with_id(app, "show", "Show emobie", true, None::<&str>)?;
    let hide_item = MenuItem::with_id(app, "hide", "Hide", true, None::<&str>)?;
    let pin_item =
        MenuItem::with_id(app, "pin", "Toggle pin above windows", true, None::<&str>)?;
    let sep_top = PredefinedMenuItem::separator(app)?;
    let sep_bottom = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &show_item,
            &hide_item,
            &sep_top,
            &pin_item,
            &sep_bottom,
            &quit_item,
        ],
    )?;

    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut tray = TrayIconBuilder::new()
            .icon(app.default_window_icon().unwrap().clone())
            .tooltip("emobie")
            .menu(&menu)
            .show_menu_on_left_click(false)
            .on_menu_event(|app, event| match event.id.as_ref() {
                "show" => show_main_window(app),
                "hide" => hide_main_window(app),
                "pin" => {
                    let _ = app.emit(PIN_EVENT, ());
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            })
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    show_main_window(tray.app_handle());
                }
            });

        if let Ok(cache_dir) = app.path().app_cache_dir() {
            tray = tray.temp_dir_path(cache_dir);
        }

        tray.build(app)
    }));

    match result {
        Ok(Ok(_tray)) => Ok(()),
        Ok(Err(error)) => Err(error),
        Err(_) => Err(tauri::Error::FailedToReceiveMessage),
    }
}

/// Returns tray availability plus a short diagnostic string.
pub fn setup(app: &tauri::App, unique_id: bool) -> TrayStatus {
    #[cfg(target_os = "linux")]
    {
        match linux::setup(app, unique_id) {
            Ok(()) => TrayStatus {
                available: true,
                detail: "StatusNotifier tray ready".into(),
            },
            Err(error) => {
                eprintln!("emobie: system tray unavailable: {error}");
                TrayStatus {
                    available: false,
                    detail: error,
                }
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = unique_id;
        match setup_tray_tauri(app) {
            Ok(()) => TrayStatus {
                available: true,
                detail: "Tray ready".into(),
            },
            Err(error) => {
                let detail = error.to_string();
                eprintln!("emobie: system tray unavailable: {detail}");
                TrayStatus {
                    available: false,
                    detail,
                }
            }
        }
    }
}
