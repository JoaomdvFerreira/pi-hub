use std::{collections::HashSet, sync::{Mutex, OnceLock}, time::Duration};

use tauri::{AppHandle, Manager};

use crate::domain::docker_container::{ContainerAction, ContainerLogMode, ContainerLogResult};
use crate::error::ApplicationError;
use crate::infrastructure::ssh::{
    docker_container_action_command, docker_container_logs_command, OpenSshExecutor, RemoteExecutor, SshTarget,
};
use crate::storage::device_repository::{DeviceRepository, JsonDeviceRepository};
use crate::storage::activity_repository::{ActivityRepository, JsonActivityRepository};
use crate::domain::activity::{ActivityCategory, ActivityEvent};

/// Generous relative to the metrics/Docker-listing timeouts (spec section
/// 24.3), since starting a heavier container can genuinely take longer
/// than reading its status.
const CONTAINER_ACTION_TIMEOUT: Duration = Duration::from_secs(20);
const CONTAINER_LOG_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_LOG_BYTES: usize = 512 * 1024;
static ACTIVE_ACTIONS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

struct ActionGuard { key: String }
impl Drop for ActionGuard { fn drop(&mut self) { if let Some(actions) = ACTIVE_ACTIONS.get() { if let Ok(mut actions) = actions.lock() { actions.remove(&self.key); } } } }

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

fn record_activity(app: &AppHandle, device_id: &str, container_id: &str, container_name: &str, code: &str, summary: String) {
    if let Ok(directory) = app.path().app_config_dir() { let _ = JsonActivityRepository::new(directory).append(ActivityEvent::new(ActivityCategory::Container, code, Some(device_id.into()), Some(container_id.into()), Some(container_name.into()), summary)); }
}

/// Retrieves one bounded, non-streaming log snapshot. The mode is a closed
/// enum and the container identifier is validated before it reaches SSH.
#[tauri::command]
pub async fn get_container_logs(app: AppHandle, device_id: String, container_id: String, mode: ContainerLogMode) -> Result<ContainerLogResult, ApplicationError> {
    let device = repository(&app)?.get(&device_id).ok_or_else(|| ApplicationError { code: "NotFoundError".into(), message: format!("device '{device_id}' was not found"), remediation: None, retryable: false })?;
    let command = docker_container_logs_command(mode, &container_id).map_err(|err| ApplicationError { code: "ValidationError".into(), message: err.to_string(), remediation: None, retryable: false })?;
    let target = SshTarget { host: device.host, port: device.ssh_port, username: device.ssh_username };
    let result = tauri::async_runtime::spawn_blocking(move || OpenSshExecutor::default().execute(&target, &command, CONTAINER_LOG_TIMEOUT)).await.map_err(|err| ApplicationError { code: "PlatformIntegrationError".into(), message: format!("the log request did not complete: {err}"), remediation: Some("Try again.".into()), retryable: true })?.map_err(|err| ApplicationError { code: err.code().to_string(), message: err.to_string(), remediation: Some(err.remediation().to_string()), retryable: true })?;
    let (content, truncated) = truncate_log_content(result.stdout);
    Ok(ContainerLogResult { container_id, mode, collected_at: chrono::Utc::now().to_rfc3339(), content, truncated })
}

fn truncate_log_content(content: String) -> (String, bool) {
    if content.len() <= MAX_LOG_BYTES { return (content, false); }
    let mut end = MAX_LOG_BYTES;
    while !content.is_char_boundary(end) { end -= 1; }
    (content[..end].to_string(), true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn log_truncation_respects_byte_limit_without_breaking_utf8() {
        let source = format!("{}€", "a".repeat(MAX_LOG_BYTES));
        let (content, truncated) = truncate_log_content(source);
        assert!(truncated);
        assert!(content.len() <= MAX_LOG_BYTES);
        assert!(std::str::from_utf8(content.as_bytes()).is_ok());
    }
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

    let key = format!("{device_id}:{container_id}");
    {
        let actions = ACTIVE_ACTIONS.get_or_init(|| Mutex::new(HashSet::new()));
        let mut actions = actions.lock().map_err(|_| ApplicationError { code: "ActionConflict".into(), message: "a container action is already in progress".into(), remediation: Some("Wait for the current action to finish.".into()), retryable: true })?;
        if !actions.insert(key.clone()) { return Err(ApplicationError { code: "ActionConflict".into(), message: "another action is already in progress for this container".into(), remediation: Some("Wait for it to finish before trying another action.".into()), retryable: true }); }
    }
    let _guard = ActionGuard { key };
    record_activity(&app, &device_id, &container_id, &device.name, "container.action_requested", format!("{} requested for {}", action.docker_verb(), device.name));

    let target = SshTarget {
        host: device.host,
        port: device.ssh_port,
        username: device.ssh_username,
    };

    let action_result = tauri::async_runtime::spawn_blocking(move || {
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
    });
    if let Err(error) = action_result { record_activity(&app, &device_id, &container_id, &device.name, "container.action_failed", format!("{} failed for {}", action.docker_verb(), device.name)); return Err(error); }
    match crate::monitoring::scheduler::refresh_one(&app, &device_id).await {
        Ok(_) => { record_activity(&app, &device_id, &container_id, &device.name, "container.action_completed", format!("{} completed and state was refreshed for {}", action.docker_verb(), device.name)); Ok(()) }
        Err(_) => { record_activity(&app, &device_id, &container_id, &device.name, "container.action_verification_failed", format!("{} completed for {}, but verification refresh failed", action.docker_verb(), device.name)); Err(ApplicationError { code: "PostActionVerificationFailed".into(), message: "Docker accepted the action, but Pi-Hub could not verify the resulting state.".into(), remediation: Some("Refresh the device and check the observed container state.".into()), retryable: true }) }
    }
}
