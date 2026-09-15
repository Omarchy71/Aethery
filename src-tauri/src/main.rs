#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod aether;
mod autostart;
mod commands;
mod error;
mod events;
mod focus;
mod state;
mod tray;
mod tun;

use state::AppState;
use tauri::{Manager, WindowEvent};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(AppState::default())
        .setup(|app| {
            let data_dir = app.handle().path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            autostart::sync_on_boot(app.handle());
            // Reap any Aether process left running from a prior crash before
            // the user can click Connect and spawn a second one onto the
            // same port.
            aether::orphan::reap_orphan(&data_dir);
            focus::spawn_watcher(app.handle().clone());
            // Best-effort on Linux: many Omarchy/Hyprland setups run without
            // a StatusNotifier/hosted tray, and tray init failing must not
            // take the whole app down — the window remains fully usable.
            if let Err(e) = tray::init(app) {
                eprintln!("[aethery] tray unavailable, continuing without it: {e}");
            }
            // "Start minimized" preference: open directly into the taskbar
            // so a boot/autostart launch stays out of the user's way.
            // When the tray is available it remains the way back;
            // without one the taskbar entry does.
            if tray::get_start_minimized() {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.minimize();
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::connect,
            commands::disconnect,
            commands::submit_access_code,
            commands::get_status,
            commands::get_default_profile,
            commands::set_default_profile,
            commands::get_vpn_status,
            commands::get_close_to_tray,
            commands::set_close_to_tray,
            commands::get_start_minimized,
            commands::set_start_minimized,
            autostart::get_autostart,
            autostart::set_autostart,
        ])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if tray::get_close_to_tray() {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                // TUN first, while elevation can still prompt: otherwise the
                // default route keeps pointing into a tunnel whose proxy is
                // dead and the machine loses its network after quit.
                tun::bring_down_blocking(app_handle);
                let state = app_handle.state::<AppState>();
                let data_dir = app_handle
                    .path()
                    .app_data_dir()
                    .unwrap_or_else(|_| std::env::temp_dir());
                aether::shutdown_blocking(&state.manager, &data_dir);
            }
        });
}
