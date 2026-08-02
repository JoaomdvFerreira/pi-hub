use tauri::{AppHandle, Manager};

use crate::application::device_service::{self, DeviceInput, DeviceServiceError};
use crate::domain::device::Device;
use crate::error::ApplicationError;
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
    device_service::create_device(&repo, input).map_err(to_application_error)
}

#[tauri::command]
pub fn update_device(
    app: AppHandle,
    id: String,
    input: DeviceInput,
) -> Result<Device, ApplicationError> {
    let repo = repository(&app)?;
    device_service::update_device(&repo, &id, input).map_err(to_application_error)
}

#[tauri::command]
pub fn delete_device(app: AppHandle, id: String) -> Result<(), ApplicationError> {
    let repo = repository(&app)?;
    device_service::delete_device(&repo, &id).map_err(to_application_error)
}
