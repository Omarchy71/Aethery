//! OS-level "start on boot" backed by tauri-plugin-autostart.
//!
//! The toggle itself is persisted on `ConnectionProfile.autostart` (see
//! aether/profiles.rs) so a fresh install picks up the last choice; the
//! functions here apply that choice to the OS (registry entry on Windows,
//! .desktop file on Linux, LaunchAgent on macOS).

use crate::aether::profiles::{self, ConnectionProfile};
use crate::error::AetherError;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch()
        .is_enabled()
        .unwrap_or_else(|_| profiles::load(&app).autostart)
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), AetherError> {
    let manager = app.autolaunch();
    let res = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    res.map_err(|e| AetherError::Internal(format!("autostart failed: {e}")))?;

    // Persist the choice so setup() can re-apply it (e.g. after the app
    // moved location, the OS entry may point nowhere).
    let mut profile: ConnectionProfile = profiles::load(&app);
    profile.autostart = enabled;
    profiles::save(&app, &profile);
    Ok(())
}

/// Best-effort: make the OS entry match the persisted choice. Runs at
/// startup; failures are ignored (no UI yet to show them in).
pub fn sync_on_boot(app: &AppHandle) {
    let wanted = profiles::load(app).autostart;
    if let Ok(current) = app.autolaunch().is_enabled() {
        if current != wanted {
            let _ = if wanted {
                app.autolaunch().enable()
            } else {
                app.autolaunch().disable()
            };
        }
    }
}
