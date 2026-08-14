use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    domain::{
        activity::{ActivityCategory, ActivityEvent},
        administration::{
            AdministrationFailure, AdministrationOperation, AdministrationOperationState,
            AdministrationOperationType, ExpectedDisruption,
        },
        connection_status::DeviceConnectionStatus,
    },
    error::ApplicationError,
    infrastructure::ssh::{OpenSshExecutor, RemoteExecutor, RemoteOperation, SshError, SshTarget},
    storage::{
        activity_repository::{ActivityRepository, JsonActivityRepository},
        administration_repository::{AdministrationRepository, JsonAdministrationRepository},
        device_repository::{DeviceRepository, JsonDeviceRepository},
    },
};

const DISPATCH_TIMEOUT: Duration = Duration::from_secs(15);
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const VERIFY_ATTEMPTS: usize = 18;
const VERIFY_INTERVAL: Duration = Duration::from_secs(2);

fn config_dir(app: &AppHandle) -> Result<std::path::PathBuf, ApplicationError> {
    app.path().app_config_dir().map_err(|err| ApplicationError {
        code: "ConfigurationError".into(),
        message: format!("could not resolve the application config directory: {err}"),
        remediation: None,
        retryable: false,
    })
}
fn activity(app: &AppHandle, operation: &AdministrationOperation, code: &str, summary: String) {
    if let Ok(dir) = config_dir(app) {
        let _ = JsonActivityRepository::new(dir).append(ActivityEvent::new(
            ActivityCategory::Administration,
            code,
            Some(operation.device_id.clone()),
            Some(operation.id.clone()),
            Some(operation.operation_type.label().into()),
            summary,
        ));
    }
}
fn emit(app: &AppHandle, operation: &AdministrationOperation) {
    let _ = app.emit("administration://operation-updated", operation);
}
fn transition(
    app: &AppHandle,
    operation: &mut AdministrationOperation,
    state: AdministrationOperationState,
    detail: Option<String>,
) {
    operation.transition(state, detail);
    emit(app, operation);
}
fn command(kind: AdministrationOperationType) -> &'static str {
    match kind {
        AdministrationOperationType::RestartDevice => {
            RemoteOperation::RestartDevice.command().unwrap()
        }
        AdministrationOperationType::ShutdownDevice => {
            RemoteOperation::ShutdownDevice.command().unwrap()
        }
        AdministrationOperationType::RestartDocker => {
            RemoteOperation::RestartDocker.command().unwrap()
        }
        AdministrationOperationType::RestartTailscale => {
            RemoteOperation::RestartTailscale.command().unwrap()
        }
    }
}
fn failure(error: &SshError) -> AdministrationFailure {
    match error {
        SshError::HostKeyError => AdministrationFailure::HostKey,
        SshError::AuthenticationError => AdministrationFailure::Authentication,
        SshError::RemoteCommandTimeout | SshError::ConnectionTimeout => {
            AdministrationFailure::CommandTimeout
        }
        SshError::RemoteCommandError { stderr, .. }
            if stderr.to_ascii_lowercase().contains("password") =>
        {
            AdministrationFailure::SudoPasswordRequired
        }
        SshError::RemoteCommandError { stderr, .. }
            if stderr.to_ascii_lowercase().contains("sudo")
                || stderr.to_ascii_lowercase().contains("permission denied") =>
        {
            AdministrationFailure::InsufficientPrivileges
        }
        SshError::RemoteCommandError { .. } => AdministrationFailure::CommandRejected,
        SshError::OutputLimitExceeded => AdministrationFailure::CommandRejected,
        _ => AdministrationFailure::Transport,
    }
}
fn is_disconnect(error: &SshError) -> bool {
    matches!(
        error,
        SshError::ConnectionRefused
            | SshError::ConnectionTimeout
            | SshError::RemoteCommandTimeout
            | SshError::DnsResolutionError
    )
}

async fn reachable(target: SshTarget) -> bool {
    tauri::async_runtime::spawn_blocking(move || {
        OpenSshExecutor::default().probe(&target, PROBE_TIMEOUT)
    })
    .await
    .ok()
    .and_then(Result::ok)
    .is_some()
}
async fn wait_for(target: &SshTarget, want_online: bool) -> bool {
    for _ in 0..VERIFY_ATTEMPTS {
        if reachable(target.clone()).await == want_online {
            return true;
        }
        tokio::time::sleep(VERIFY_INTERVAL).await;
    }
    false
}

/// Runs one member of the M10 closed operation catalogue. The function returns
/// a typed terminal state for ordinary remote failures; only malformed device
/// identity and concurrent requests reject at the command boundary.
#[tauri::command]
pub async fn perform_administration_operation(
    app: AppHandle,
    device_id: String,
    operation_type: AdministrationOperationType,
) -> Result<AdministrationOperation, ApplicationError> {
    let dir = config_dir(&app)?;
    let device = JsonDeviceRepository::new(&dir)
        .get(&device_id)
        .ok_or_else(|| ApplicationError {
            code: "NotFoundError".into(),
            message: format!("device '{device_id}' was not found"),
            remediation: None,
            retryable: false,
        })?;
    let coordinator =
        app.state::<crate::monitoring::maintenance_coordinator::DeviceMaintenanceCoordinator>();
    let _claim = coordinator
        .try_claim(
            &device_id,
            crate::monitoring::maintenance_coordinator::MaintenanceOperationKind::Administration(
                operation_type,
            ),
        )
        .ok_or_else(|| ApplicationError {
            // Preserve M10's established frontend contract even though the
            // conflicting owner may now be M15 or M16.
            code: "ActionConflict".into(),
            message: "another maintenance operation is already in progress for this device".into(),
            remediation: Some("Wait for it to finish before trying another action.".into()),
            retryable: true,
        })?;
    let target = SshTarget {
        host: device.host.clone(),
        port: device.ssh_port,
        username: device.ssh_username.clone(),
    };
    let mut operation = AdministrationOperation::requested(device_id, operation_type);
    activity(
        &app,
        &operation,
        "administration.requested",
        format!("{} requested for {}", operation_type.label(), device.name),
    );
    transition(
        &app,
        &mut operation,
        AdministrationOperationState::Dispatching,
        None,
    );
    let remote_command = command(operation_type).to_string();
    let dispatch = tauri::async_runtime::spawn_blocking(move || {
        OpenSshExecutor::default().execute(&target, &remote_command, DISPATCH_TIMEOUT)
    })
    .await;
    let target = SshTarget {
        host: device.host.clone(),
        port: device.ssh_port,
        username: device.ssh_username.clone(),
    };
    match dispatch {
        Ok(Ok(_)) => {
            transition(
                &app,
                &mut operation,
                AdministrationOperationState::CommandAccepted,
                None,
            );
        }
        Ok(Err(err)) if is_disconnect(&err) => {
            // a reboot/poweroff can close SSH before ssh.exe reports a final exit code
            transition(&app, &mut operation, AdministrationOperationState::OutcomeUncertain, Some("The SSH connection closed while the operation was being dispatched; Pi-Hub will verify the observed state.".into()));
        }
        Ok(Err(err)) => {
            operation.failure = Some(failure(&err));
            transition(
                &app,
                &mut operation,
                AdministrationOperationState::Failed,
                Some(err.remediation().into()),
            );
            activity(
                &app,
                &operation,
                "administration.failed",
                format!("{} failed for {}", operation_type.label(), device.name),
            );
            return Ok(operation);
        }
        Err(err) => {
            operation.failure = Some(AdministrationFailure::Transport);
            transition(
                &app,
                &mut operation,
                AdministrationOperationState::Failed,
                Some(format!("The operation task did not complete: {err}")),
            );
            activity(
                &app,
                &operation,
                "administration.failed",
                format!("{} failed for {}", operation_type.label(), device.name),
            );
            return Ok(operation);
        }
    }
    let marker = ExpectedDisruption::new(&operation);
    JsonAdministrationRepository::new(&dir)
        .save(marker)
        .map_err(|err| ApplicationError {
            code: "StorageError".into(),
            message: format!("could not persist expected disruption: {err}"),
            remediation: Some("Check disk space and try again.".into()),
            retryable: true,
        })?;
    activity(
        &app,
        &operation,
        "administration.command_accepted",
        format!(
            "{} command accepted for {}",
            operation_type.label(),
            device.name
        ),
    );
    let completed = match operation_type {
        AdministrationOperationType::ShutdownDevice => {
            transition(
                &app,
                &mut operation,
                AdministrationOperationState::WaitingForOffline,
                None,
            );
            wait_for(&target, false).await
        }
        AdministrationOperationType::RestartDevice => {
            transition(
                &app,
                &mut operation,
                AdministrationOperationState::WaitingForOffline,
                None,
            );
            let offline = wait_for(&target, false).await;
            if offline {
                transition(
                    &app,
                    &mut operation,
                    AdministrationOperationState::WaitingForOnline,
                    None,
                );
                wait_for(&target, true).await
            } else {
                false
            }
        }
        AdministrationOperationType::RestartDocker => {
            transition(
                &app,
                &mut operation,
                AdministrationOperationState::Verifying,
                None,
            );
            crate::monitoring::scheduler::refresh_one(&app, &operation.device_id)
                .await
                .map(|snapshot| {
                    snapshot.connection_status == DeviceConnectionStatus::Online
                        && snapshot.docker_available
                })
                .unwrap_or(false)
        }
        AdministrationOperationType::RestartTailscale => {
            transition(
                &app,
                &mut operation,
                AdministrationOperationState::Verifying,
                None,
            );
            wait_for(&target, true).await
        }
    };
    if completed {
        transition(
            &app,
            &mut operation,
            AdministrationOperationState::Completed,
            None,
        );
        if operation_type != AdministrationOperationType::ShutdownDevice {
            let _ = JsonAdministrationRepository::new(&dir).clear(&operation.device_id);
        }
        activity(
            &app,
            &operation,
            "administration.completed",
            format!("{} completed for {}", operation_type.label(), device.name),
        );
    } else {
        operation.failure = Some(AdministrationFailure::VerificationTimeout);
        transition(&app, &mut operation, AdministrationOperationState::TimedOut, Some("Pi-Hub could not verify the expected result before the bounded verification deadline.".into()));
        activity(
            &app,
            &operation,
            "administration.timed_out",
            format!("{} timed out for {}", operation_type.label(), device.name),
        );
    }
    Ok(operation)
}

#[tauri::command]
pub fn get_expected_disruption(
    app: AppHandle,
    device_id: String,
) -> Result<Option<ExpectedDisruption>, ApplicationError> {
    Ok(JsonAdministrationRepository::new(config_dir(&app)?).get_valid(&device_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sudo_errors_are_actionable_without_prompting() {
        let error = SshError::RemoteCommandError {
            exit_code: Some(1),
            stderr: "sudo: a password is required".into(),
        };
        assert_eq!(failure(&error), AdministrationFailure::SudoPasswordRequired);
        let error = SshError::RemoteCommandError {
            exit_code: Some(1),
            stderr: "sudo: a terminal is required".into(),
        };
        assert_eq!(
            failure(&error),
            AdministrationFailure::InsufficientPrivileges
        );
    }
    #[test]
    fn disconnect_is_only_an_uncertain_dispatch_condition() {
        assert!(is_disconnect(&SshError::ConnectionRefused));
        assert!(!is_disconnect(&SshError::AuthenticationError));
    }
}
