use tauri::{AppHandle, Manager};

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

#[tauri::command]
pub fn get_devices(app: AppHandle) -> Result<Vec<Device>, ApplicationError> {
    Ok(repository(&app)?.load_all())
}

#[tauri::command]
pub fn get_device(app: AppHandle, id: String) -> Result<Option<Device>, ApplicationError> {
    Ok(repository(&app)?.get(&id))
}
