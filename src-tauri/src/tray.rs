use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

/// Holds references to tray menu items for dynamic label updates (i18n).
pub struct TrayMenuState {
    pub items: Mutex<HashMap<String, MenuItem<tauri::Wry>>>,
}

/// Create the custom tray popup window on Windows.
///
/// The window is built dynamically (NOT declared in tauri.conf.json) so that
/// macOS and Linux never instantiate it.  It starts hidden and is shown/hidden
/// on right-click via `on_tray_icon_event`.
#[cfg(target_os = "windows")]
fn ensure_tray_popup(app: &AppHandle) {
    use tauri::WebviewWindowBuilder;

    // Only create once — subsequent calls are no-ops.
    if app.get_webview_window("tray-menu").is_some() {
        return;
    }

    let _popup = WebviewWindowBuilder::new(app, "tray-menu", tauri::WebviewUrl::App("/tray-menu".into()))
        .title("")
        .inner_size(232.0, 280.0)
        .visible(false)
        .decorations(false)
        .transparent(true)
        .skip_taskbar(true)
        .always_on_top(true)
        .resizable(false)
        .build();
}

pub fn setup_tray(app: &AppHandle) -> Result<TrayMenuState, Box<dyn std::error::Error>> {
    let show_item = MenuItem::with_id(app, "show", "Show Motrix Next", true, None::<&str>)?;
    let new_task_item = MenuItem::with_id(app, "tray-new-task", "New Task", true, None::<&str>)?;
    let resume_all_item =
        MenuItem::with_id(app, "tray-resume-all", "Resume All", true, None::<&str>)?;
    let pause_all_item = MenuItem::with_id(app, "tray-pause-all", "Pause All", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "tray-quit", "Quit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;

    // Clone refs before moving into menu
    let mut items_map: HashMap<String, MenuItem<tauri::Wry>> = HashMap::new();
    items_map.insert("show".to_string(), show_item.clone());
    items_map.insert("tray-new-task".to_string(), new_task_item.clone());
    items_map.insert("tray-resume-all".to_string(), resume_all_item.clone());
    items_map.insert("tray-pause-all".to_string(), pause_all_item.clone());
    items_map.insert("tray-quit".to_string(), quit_item.clone());

    let menu = Menu::with_items(
        app,
        &[
            &show_item,
            &separator,
            &new_task_item,
            &resume_all_item,
            &pause_all_item,
            &PredefinedMenuItem::separator(app)?,
            &quit_item,
        ],
    )?;

    // On Windows: eagerly create the hidden popup window.
    #[cfg(target_os = "windows")]
    ensure_tray_popup(app);

    let mut builder = TrayIconBuilder::with_id("main")
        .icon(tauri::image::Image::from_bytes(include_bytes!(
            "../icons/tray-icon.png"
        ))?)
        .on_tray_icon_event(|tray, event| {
            match event {
                // Left-click: show main window (all platforms)
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    let app = tray.app_handle();
                    #[cfg(target_os = "macos")]
                    {
                        use tauri::ActivationPolicy;
                        let _ = app.set_activation_policy(ActivationPolicy::Regular);
                    }
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                // Windows: right-click shows the custom tray popup window
                #[cfg(target_os = "windows")]
                TrayIconEvent::Click {
                    button: MouseButton::Right,
                    button_state: MouseButtonState::Up,
                    position,
                    ..
                } => {
                    let app = tray.app_handle();
                    // Lazily create the popup if it was destroyed or not yet ready.
                    ensure_tray_popup(app);
                    if let Some(popup) = app.get_webview_window("tray-menu") {
                        // Position the popup near the click (above the tray icon)
                        let popup_width = 232.0_f64;
                        let popup_height = 280.0_f64;
                        let x = position.x - popup_width / 2.0;
                        let y = position.y - popup_height;
                        let _ = popup.set_position(tauri::LogicalPosition::new(x, y));
                        let _ = popup.show();
                        let _ = popup.set_focus();
                    }
                }
                _ => {}
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                #[cfg(target_os = "macos")]
                {
                    use tauri::ActivationPolicy;
                    let _ = app.set_activation_policy(ActivationPolicy::Regular);
                }
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "tray-new-task" => {
                #[cfg(target_os = "macos")]
                {
                    use tauri::ActivationPolicy;
                    let _ = app.set_activation_policy(ActivationPolicy::Regular);
                }
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
                let _ = app.emit("menu-event", "new-task");
            }
            "tray-resume-all" => {
                let _ = app.emit("menu-event", "resume-all");
            }
            "tray-pause-all" => {
                let _ = app.emit("menu-event", "pause-all");
            }
            "tray-quit" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.destroy();
                }
                app.exit(0);
            }
            _ => {}
        });

    // On Windows: no native menu (custom popup used instead).
    // On macOS/Linux: native system menu.
    #[cfg(not(target_os = "windows"))]
    {
        builder = builder.menu(&menu);
    }

    let _tray = builder.build(app)?;

    Ok(TrayMenuState {
        items: Mutex::new(items_map),
    })
}
