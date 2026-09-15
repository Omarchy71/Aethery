use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

/// Global flag — toggled from the frontend via the `set_close_to_tray` command
/// and persisted to disk via `tauri-plugin-store`. Using an atomic here instead
/// of the store directly because the `on_window_event` callback fires on every
/// close and reading the store there would be wasteful.
static CLOSE_TO_TRAY: AtomicBool = AtomicBool::new(false);
/// When true, the main window starts minimized (taskbar) on launch.
static START_MINIMIZED: AtomicBool = AtomicBool::new(false);

const STORE_FILE: &str = "settings.json";
const STORE_KEY: &str = "close_to_tray";
const START_MINIMIZED_KEY: &str = "start_minimized";

pub fn get_close_to_tray() -> bool {
    CLOSE_TO_TRAY.load(Ordering::Relaxed)
}

pub fn get_start_minimized() -> bool {
    START_MINIMIZED.load(Ordering::Relaxed)
}

pub fn set_start_minimized(app: &AppHandle, enabled: bool) {
    START_MINIMIZED.store(enabled, Ordering::Relaxed);
    // Persist so it survives restarts.
    use tauri_plugin_store::StoreExt;
    if let Ok(store) = app.store(STORE_FILE) {
        store.set(START_MINIMIZED_KEY, serde_json::Value::Bool(enabled));
        let _ = store.save();
    }
}

pub fn set_close_to_tray(app: &AppHandle, enabled: bool) {
    CLOSE_TO_TRAY.store(enabled, Ordering::Relaxed);
    // Persist so it survives restarts.
    use tauri_plugin_store::StoreExt;
    if let Ok(store) = app.store(STORE_FILE) {
        store.set(STORE_KEY, serde_json::Value::Bool(enabled));
        let _ = store.save();
    }
}

/// Load persisted preferences and sync the atomics.
fn load_preference(app: &AppHandle) {
    use tauri_plugin_store::StoreExt;
    let store = app.store(STORE_FILE).ok();
    let get_bool = |key: &str| {
        store
            .as_ref()
            .and_then(|s| s.get(key))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    };
    CLOSE_TO_TRAY.store(get_bool(STORE_KEY), Ordering::Relaxed);
    START_MINIMIZED.store(get_bool(START_MINIMIZED_KEY), Ordering::Relaxed);
}

/// Create the system-tray icon, menu, and event handlers. Call from `setup`.
pub fn init(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    load_preference(app.handle());

    let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("Aethery")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

fn show_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_flag_round_trips() {
        CLOSE_TO_TRAY.store(false, Ordering::Relaxed);
        assert!(!get_close_to_tray());
        CLOSE_TO_TRAY.store(true, Ordering::Relaxed);
        assert!(get_close_to_tray());
        START_MINIMIZED.store(false, Ordering::Relaxed);
        assert!(!get_start_minimized());
        START_MINIMIZED.store(true, Ordering::Relaxed);
        assert!(get_start_minimized());
        START_MINIMIZED.store(false, Ordering::Relaxed);
    }
}
