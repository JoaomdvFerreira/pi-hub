use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

#[derive(Debug)]
pub struct AutostartError(pub String);

/// Registers or removes Pi-Hub's Windows autostart entry so it matches
/// `wanted`. Idempotent -- only touches the registration when it actually
/// differs from the current OS state, since this is called on every
/// settings save, not just when the toggle changes.
pub fn sync(app: &AppHandle, wanted: bool) -> Result<(), AutostartError> {
    let manager = app.autolaunch();
    let currently_enabled = manager.is_enabled().map_err(|err| AutostartError(err.to_string()))?;

    if wanted == currently_enabled {
        return Ok(());
    }

    if wanted {
        manager.enable().map_err(|err| AutostartError(err.to_string()))
    } else {
        manager.disable().map_err(|err| AutostartError(err.to_string()))
    }
}
