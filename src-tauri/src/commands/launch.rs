use tauri::{AppHandle, Manager};

use crate::error::ApplicationError;
use crate::platform::terminal;
use crate::storage::device_repository::{DeviceRepository, JsonDeviceRepository};

fn repository(app: &AppHandle) -> Result<JsonDeviceRepository, ApplicationError> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|err| ApplicationError {
            code: "ConfigurationError".into(),
            message: format!("could not resolve the application config directory: {err}"),
            remediation: None,
            retryable: false,
        })?;
    Ok(JsonDeviceRepository::new(config_dir))
}

/// Opens a new Windows Terminal tab with an SSH session to the device.
/// Embedded terminal UI is explicitly out of MVP scope; this hands off to
/// the user's own Windows Terminal + OpenSSH setup entirely.
#[tauri::command]
pub fn open_device_terminal(app: AppHandle, device_id: String) -> Result<(), ApplicationError> {
    let device = repository(&app)?.get(&device_id).ok_or_else(|| ApplicationError {
        code: "NotFoundError".into(),
        message: format!("device '{device_id}' was not found"),
        remediation: None,
        retryable: false,
    })?;

    terminal::launch_ssh_terminal(&device.host, device.ssh_port, &device.ssh_username).map_err(
        |err| ApplicationError {
            code: "PlatformIntegrationError".into(),
            message: err.0,
            remediation: Some(
                "Make sure Windows Terminal is installed (it ships with Windows 11 by default; \
                 otherwise install it from the Microsoft Store)."
                    .into(),
            ),
            retryable: true,
        },
    )
}
