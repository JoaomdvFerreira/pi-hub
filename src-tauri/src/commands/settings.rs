use tauri::{AppHandle, Manager};

use crate::domain::settings::AppSettings;
use crate::error::ApplicationError;
use crate::platform::autostart;
use crate::storage::config_repository::{JsonSettingsRepository, SettingsRepository};

fn repository(app: &AppHandle) -> Result<JsonSettingsRepository, ApplicationError> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|err| ApplicationError {
            code: "ConfigurationError".into(),
            message: format!("could not resolve the application config directory: {err}"),
            remediation: None,
            retryable: false,
        })?;
    Ok(JsonSettingsRepository::new(config_dir))
}

#[tauri::command]
pub fn get_app_settings(app: AppHandle) -> Result<AppSettings, ApplicationError> {
    Ok(repository(&app)?.load())
}

#[tauri::command]
pub fn save_app_settings(
    app: AppHandle,
    settings: AppSettings,
) -> Result<AppSettings, ApplicationError> {
    settings.validate().map_err(|err| ApplicationError {
        code: "ValidationError".into(),
        message: err.0,
        remediation: Some("Adjust the setting to a valid value and try again.".into()),
        retryable: true,
    })?;

    // Sync the OS-level registration before persisting, so a failure here
    // (e.g. no permission to write the Run registry key) leaves the saved
    // preference unchanged rather than claiming a state that isn't real.
    autostart::sync(&app, settings.start_with_windows).map_err(|err| ApplicationError {
        code: "PlatformIntegrationError".into(),
        message: format!("could not update the Windows startup registration: {}", err.0),
        remediation: Some("Try again, or check that Pi-Hub has permission to modify startup settings.".into()),
        retryable: true,
    })?;

    let repo = repository(&app)?;
    repo.save(&settings).map_err(|err| ApplicationError {
        code: "StorageError".into(),
        message: err.to_string(),
        remediation: Some("Check disk space and file permissions, then try again.".into()),
        retryable: true,
    })?;

    Ok(settings)
}
