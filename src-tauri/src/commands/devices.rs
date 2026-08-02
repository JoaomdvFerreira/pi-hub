use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::application::device_service::{self, DeviceInput, DeviceServiceError};
use crate::domain::connection_status::DeviceConnectionStatus;
use crate::domain::device::{
    validate_host, validate_service_url, validate_ssh_port, validate_ssh_username, Device,
};
use crate::error::ApplicationError;
use crate::infrastructure::ssh::{OpenSshExecutor, RemoteExecutor, SshTarget};
use crate::platform::tray;
use crate::storage::device_repository::{DeviceRepository, JsonDeviceRepository};

/// Matches spec section 24.3's SSH connection timeout default.
const SSH_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestConnectionInput {
    pub host: String,
    pub ssh_port: u16,
    pub ssh_username: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestResult {
    pub status: DeviceConnectionStatus,
    pub message: Option<String>,
}

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

fn to_application_error(err: DeviceServiceError) -> ApplicationError {
    match err {
        DeviceServiceError::Validation(e) => ApplicationError {
            code: "ValidationError".into(),
            message: e.0,
            remediation: Some("Correct the highlighted field and try again.".into()),
            retryable: true,
        },
        DeviceServiceError::NotFound(id) => ApplicationError {
            code: "NotFoundError".into(),
            message: format!("device '{id}' was not found"),
            remediation: None,
            retryable: false,
        },
        DeviceServiceError::Storage(e) => ApplicationError {
            code: "StorageError".into(),
            message: e.to_string(),
            remediation: Some("Check disk space and file permissions, then try again.".into()),
            retryable: true,
        },
    }
}

#[tauri::command]
pub fn get_devices(app: AppHandle) -> Result<Vec<Device>, ApplicationError> {
    Ok(repository(&app)?.load_all())
}

#[tauri::command]
pub fn get_device(app: AppHandle, id: String) -> Result<Option<Device>, ApplicationError> {
    Ok(repository(&app)?.get(&id))
}

#[tauri::command]
pub fn create_device(app: AppHandle, input: DeviceInput) -> Result<Device, ApplicationError> {
    let repo = repository(&app)?;
    let device = device_service::create_device(&repo, input).map_err(to_application_error)?;
    tray::rebuild_device_menu(&app);
    Ok(device)
}

#[tauri::command]
pub fn update_device(
    app: AppHandle,
    id: String,
    input: DeviceInput,
) -> Result<Device, ApplicationError> {
    let repo = repository(&app)?;
    let device = device_service::update_device(&repo, &id, input).map_err(to_application_error)?;
    tray::rebuild_device_menu(&app);
    Ok(device)
}

#[tauri::command]
pub fn delete_device(app: AppHandle, id: String) -> Result<(), ApplicationError> {
    let repo = repository(&app)?;
    device_service::delete_device(&repo, &id).map_err(to_application_error)?;
    tray::rebuild_device_menu(&app);
    Ok(())
}

/// Opens a device's registered service URL in the system default browser.
/// Re-validates the http/https scheme at open time (defense in depth on top
/// of the validation already applied when the service was saved) so this
/// command can never be used to hand an arbitrary URI scheme to the OS
/// opener, regardless of how the stored data got there.
#[tauri::command]
pub fn open_device_service(
    app: AppHandle,
    device_id: String,
    service_id: String,
) -> Result<(), ApplicationError> {
    let repo = repository(&app)?;
    let device = repo.get(&device_id).ok_or_else(|| ApplicationError {
        code: "NotFoundError".into(),
        message: format!("device '{device_id}' was not found"),
        remediation: None,
        retryable: false,
    })?;
    let service = device
        .services
        .iter()
        .find(|s| s.id == service_id)
        .ok_or_else(|| ApplicationError {
            code: "NotFoundError".into(),
            message: format!("service '{service_id}' was not found on this device"),
            remediation: None,
            retryable: false,
        })?;

    validate_service_url(&service.url).map_err(|err| ApplicationError {
        code: "ValidationError".into(),
        message: err.0,
        remediation: None,
        retryable: false,
    })?;

    tauri_plugin_opener::open_url(&service.url, None::<&str>).map_err(|err| ApplicationError {
        code: "PlatformIntegrationError".into(),
        message: format!("could not open the service URL: {err}"),
        remediation: Some("Check that a default browser is configured.".into()),
        retryable: true,
    })
}

/// Tests SSH connectivity to a host/port/username combination without
/// requiring the device to already be registered, so the "Test Connection"
/// action works while adding or editing a device. Never requests or stores
/// a password; relies entirely on the Windows OpenSSH client's own key/
/// ssh-agent configuration (BatchMode=yes never prompts).
#[tauri::command]
pub async fn test_device_connection(
    input: TestConnectionInput,
) -> Result<ConnectionTestResult, ApplicationError> {
    validate_host(&input.host).map_err(|err| ApplicationError {
        code: "ValidationError".into(),
        message: err.0,
        remediation: Some("Correct the highlighted field and try again.".into()),
        retryable: true,
    })?;
    validate_ssh_port(input.ssh_port).map_err(|err| ApplicationError {
        code: "ValidationError".into(),
        message: err.0,
        remediation: Some("Correct the highlighted field and try again.".into()),
        retryable: true,
    })?;
    validate_ssh_username(&input.ssh_username).map_err(|err| ApplicationError {
        code: "ValidationError".into(),
        message: err.0,
        remediation: Some("Correct the highlighted field and try again.".into()),
        retryable: true,
    })?;

    tauri::async_runtime::spawn_blocking(move || {
        let target = SshTarget {
            host: input.host,
            port: input.ssh_port,
            username: input.ssh_username,
        };
        let executor = OpenSshExecutor::default();
        match executor.probe(&target, SSH_CONNECT_TIMEOUT) {
            Ok(_) => ConnectionTestResult {
                status: DeviceConnectionStatus::Online,
                message: None,
            },
            Err(err) => ConnectionTestResult {
                status: err.to_connection_status(),
                message: Some(err.remediation().to_string()),
            },
        }
    })
    .await
    .map_err(|_| ApplicationError {
        code: "PlatformIntegrationError".into(),
        message: "the connection test did not complete".into(),
        remediation: Some("Try again.".into()),
        retryable: true,
    })
}
