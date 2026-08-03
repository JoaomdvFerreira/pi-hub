use tauri::{AppHandle, Manager};

use crate::error::ApplicationError;
use crate::platform::pty::PtySessionManager;
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

fn pty_error(err: crate::platform::pty::PtyError) -> ApplicationError {
    ApplicationError {
        code: "PlatformIntegrationError".into(),
        message: err.0,
        remediation: Some(
            "Make sure the Windows OpenSSH client is installed and try opening the terminal again."
                .into(),
        ),
        retryable: true,
    }
}

/// Opens a new in-app terminal session (an `ssh.exe` process attached to a
/// pseudo-console) for the device and returns its session id. The frontend
/// then listens for `terminal://output:<sessionId>` /
/// `terminal://exit:<sessionId>` and calls write_terminal_input /
/// resize_terminal_session / close_terminal_session with that id.
#[tauri::command]
pub fn open_terminal_session(app: AppHandle, device_id: String) -> Result<String, ApplicationError> {
    let device = repository(&app)?
        .get(&device_id)
        .ok_or_else(|| ApplicationError {
            code: "NotFoundError".into(),
            message: format!("device '{device_id}' was not found"),
            remediation: None,
            retryable: false,
        })?;

    app.state::<PtySessionManager>()
        .open(&app, &device.host, device.ssh_port, &device.ssh_username)
        .map_err(pty_error)
}

#[tauri::command]
pub fn write_terminal_input(
    app: AppHandle,
    session_id: String,
    data: String,
) -> Result<(), ApplicationError> {
    app.state::<PtySessionManager>()
        .write(&session_id, &data)
        .map_err(pty_error)
}

#[tauri::command]
pub fn resize_terminal_session(
    app: AppHandle,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), ApplicationError> {
    app.state::<PtySessionManager>()
        .resize(&session_id, cols, rows)
        .map_err(pty_error)
}

#[tauri::command]
pub fn close_terminal_session(app: AppHandle, session_id: String) -> Result<(), ApplicationError> {
    app.state::<PtySessionManager>()
        .close(&session_id)
        .map_err(pty_error)
}
