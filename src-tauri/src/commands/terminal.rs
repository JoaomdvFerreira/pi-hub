use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::error::ApplicationError;
use crate::platform::pty::PtySessionManager;
use crate::storage::device_repository::{DeviceRepository, JsonDeviceRepository};

/// Bounds the *whole* session-open operation, not just SSH's own TCP
/// connect phase (which `-o ConnectTimeout=5` in platform/pty.rs already
/// covers). Generous relative to that 5s, since it also has to cover
/// authentication -- but unlike ConnectTimeout, nothing bounds a device
/// that legitimately blocks on stdin after connecting (e.g. password auth
/// instead of key auth) or any future, still-unknown way a session could
/// stall. Without this, such a session would sit open forever with only a
/// manual close as a way out, unlike every other SSH-backed operation in
/// this app (infrastructure/ssh/process.rs's run_with_timeout), which
/// always has a hard, enforced ceiling.
const OPEN_TIMEOUT: Duration = Duration::from_secs(15);

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
/// must already be subscribed to the stable `terminal://output` /
/// `terminal://exit` events (filtering by session id in the payload)
/// *before* calling this -- output can start flowing the instant the
/// child process is spawned, well before this command returns, so
/// subscribing afterward can silently miss it.
///
/// `async` + `spawn_blocking` here is load-bearing, not a style choice: a
/// plain (non-async) `#[tauri::command]` runs on the main thread, and
/// creating a Windows ConPTY console (inside `PtySessionManager::open`)
/// has been observed taking upwards of 30 seconds in this environment
/// (most likely antivirus scanning the freshly-spawned child process) --
/// long enough that a synchronous command here previously froze the
/// entire window and got it killed by Windows as unresponsive. Moving the
/// blocking work onto a worker thread keeps the UI (and every other
/// command) responsive while this is in flight.
#[tauri::command]
pub async fn open_terminal_session(
    app: AppHandle,
    device_id: String,
) -> Result<String, ApplicationError> {
    let device = repository(&app)?
        .get(&device_id)
        .ok_or_else(|| ApplicationError {
            code: "NotFoundError".into(),
            message: format!("device '{device_id}' was not found"),
            remediation: None,
            retryable: false,
        })?;

    let app_for_task = app.clone();
    let mut join_handle = tauri::async_runtime::spawn_blocking(move || {
        app_for_task.state::<PtySessionManager>().open(
            &app_for_task,
            &device.id,
            &device.host,
            device.ssh_port,
            &device.ssh_username,
        )
    });

    tokio::select! {
        result = &mut join_handle => {
            result
                .map_err(|err| ApplicationError {
                    code: "PlatformIntegrationError".into(),
                    message: format!("opening the terminal session did not complete: {err}"),
                    remediation: Some("Try again.".into()),
                    retryable: true,
                })?
                .map_err(pty_error)
        }
        _ = tokio::time::sleep(OPEN_TIMEOUT) => {
            // The blocking task can't be cancelled (spawn_blocking never
            // can be), so it may still complete later and hand back a
            // real, live session -- one the frontend has already given up
            // on and will never learn the id of. Keep `join_handle` alive
            // in a background reaper instead of dropping it here, so that
            // if that happens, the orphaned session gets closed instead
            // of leaking indefinitely (the exact failure mode this
            // timeout exists to prevent, just arriving late).
            let app_for_reaper = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Ok(Ok(session_id)) = join_handle.await {
                    log::warn!(
                        "a terminal session ({session_id}) finished opening only after its own {OPEN_TIMEOUT:?} timeout had already been reported to the frontend; closing it immediately instead of leaving it orphaned"
                    );
                    let _ = app_for_reaper.state::<PtySessionManager>().close(&session_id);
                }
            });
            Err(ApplicationError {
                code: "PlatformIntegrationError".into(),
                message: format!("opening the terminal session timed out after {OPEN_TIMEOUT:?}"),
                remediation: Some("Check that the device is reachable and try again.".into()),
                retryable: true,
            })
        }
    }
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
