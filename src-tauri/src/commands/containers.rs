use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::domain::docker_container::ContainerAction;
use crate::error::ApplicationError;
use crate::infrastructure::ssh::{
    docker_container_action_command, OpenSshExecutor, RemoteExecutor, SshTarget,
};
use crate::storage::device_repository::{DeviceRepository, JsonDeviceRepository};

/// Generous relative to the metrics/Docker-listing timeouts (spec section
/// 24.3), since starting a heavier container can genuinely take longer
/// than reading its status.
const CONTAINER_ACTION_TIMEOUT: Duration = Duration::from_secs(20);

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

/// Starts, stops, or restarts one Docker container on a device over SSH.
/// Confirmation is a frontend concern (an AlertDialog before this is ever
/// called); this command itself just validates the container id and runs
/// the fixed `docker <verb>` command -- never an arbitrary one -- and
/// surfaces any failure (offline device, permission error, no such
/// container) as a normal ApplicationError without touching monitoring
/// state. The next scheduled or manual refresh picks up the result.
#[tauri::command]
pub async fn perform_container_action(
    app: AppHandle,
    device_id: String,
    container_id: String,
    action: ContainerAction,
) -> Result<(), ApplicationError> {
    let device = repository(&app)?
        .get(&device_id)
        .ok_or_else(|| ApplicationError {
            code: "NotFoundError".into(),
            message: format!("device '{device_id}' was not found"),
            remediation: None,
            retryable: false,
        })?;

    let command = docker_container_action_command(action, &container_id).map_err(|err| {
        ApplicationError {
            code: "ValidationError".into(),
            message: err.to_string(),
            remediation: None,
            retryable: false,
        }
    })?;

    let target = SshTarget {
        host: device.host,
        port: device.ssh_port,
        username: device.ssh_username,
    };

    tauri::async_runtime::spawn_blocking(move || {
        let executor = OpenSshExecutor::default();
        executor.execute(&target, &command, CONTAINER_ACTION_TIMEOUT)
    })
    .await
    .map_err(|err| ApplicationError {
        code: "PlatformIntegrationError".into(),
        message: format!("the container action did not complete: {err}"),
        remediation: Some("Try again.".into()),
        retryable: true,
    })?
    .map(|_| ())
    .map_err(|err| ApplicationError {
        code: err.code().to_string(),
        message: err.to_string(),
        remediation: Some(err.remediation().to_string()),
        retryable: true,
    })
}
