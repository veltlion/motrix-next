use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{
    menu::MenuItem,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};
use tauri_plugin_positioner::{Position, WindowExt as PositionerExt};

/// Holds references to tray menu items for dynamic label updates (i18n).
/// Retained for backward-compatibility with `update_tray_menu_labels` command.
pub struct TrayMenuState {
    pub items: Mutex<HashMap<String, MenuItem<tauri::Wry>>>,
}

/// Create the custom tray popup window.
///
/// The window is built dynamically (NOT declared in tauri.conf.json).
/// It starts hidden and is shown/positioned on click via
/// `on_tray_icon_event` + `tauri-plugin-positioner`.
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
        .accept_first_mouse(true)
        .shadow(false)
        .resizable(false)
        .build();
}

/// Position, show, and focus the custom tray popup window.
///
/// Uses `tauri-plugin-positioner` with `Position::TrayCenter` for
/// cross-platform tray-relative positioning (handles DPI, multi-monitor,
/// and tray orientation automatically).
///
/// **Prerequisite**: `on_tray_event` must be called first in the tray icon
/// event handler so the positioner knows the tray icon's screen coordinates.
fn show_tray_popup(app: &AppHandle) {
    ensure_tray_popup(app);
    if let Some(popup) = app.get_webview_window("tray-menu") {
        let _ = popup.move_window(Position::TrayCenter);
        let _ = popup.show();
        let _ = popup.set_focus();
    }
}

pub fn setup_tray(app: &AppHandle) -> Result<TrayMenuState, Box<dyn std::error::Error>> {
    // Create MenuItem references for TrayMenuState (used by update_tray_menu_labels).
    // These are NOT attached to a native OS menu — all platforms use the custom popup.
    let show_item = MenuItem::with_id(app, "show", "Show Motrix Next", true, None::<&str>)?;
    let new_task_item = MenuItem::with_id(app, "tray-new-task", "New Task", true, None::<&str>)?;
    let resume_all_item =
        MenuItem::with_id(app, "tray-resume-all", "Resume All", true, None::<&str>)?;
    let pause_all_item = MenuItem::with_id(app, "tray-pause-all", "Pause All", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "tray-quit", "Quit", true, None::<&str>)?;

    let mut items_map: HashMap<String, MenuItem<tauri::Wry>> = HashMap::new();
    items_map.insert("show".to_string(), show_item);
    items_map.insert("tray-new-task".to_string(), new_task_item);
    items_map.insert("tray-resume-all".to_string(), resume_all_item);
    items_map.insert("tray-pause-all".to_string(), pause_all_item);
    items_map.insert("tray-quit".to_string(), quit_item);

    // Popup is created lazily on click via ensure_tray_popup / show_tray_popup.
    // No eager creation at startup — prevents blocking the main window.

    let builder = TrayIconBuilder::with_id("main")
        .icon(tauri::image::Image::from_bytes(include_bytes!(
            "../icons/tray-icon.png"
        ))?)
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();

            // CRITICAL: feed every tray event to the positioner plugin so it
            // knows the tray icon's screen coordinates. Without this,
            // `move_window(Position::TrayCenter)` panics with
            // "Tray position not set".
            tauri_plugin_positioner::on_tray_event(app, &event);

            match event {
                // Left-click: show main window (all platforms)
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
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
                // Right-click: show the custom tray popup window (all platforms)
                TrayIconEvent::Click {
                    button: MouseButton::Right,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    show_tray_popup(app);
                }
                _ => {}
            }
        });

    let _tray = builder.build(app)?;

    // Pre-create the popup window (hidden) so the WebView pre-loads the SPA.
    // Without this, the first right-click has a multi-second delay while the
    // JS bundle is fetched and compiled.  Subsequent shows are instant.
    ensure_tray_popup(app);

    Ok(TrayMenuState {
        items: Mutex::new(items_map),
    })
}
