use tauri::{AppHandle, Manager};

use crate::{
    domain::{
        activity::{ActivityCategory, ActivityEvent},
        alert::{Alert, AlertState},
    },
    error::ApplicationError,
    monitoring::alerts::acknowledge,
    storage::{
        activity_repository::{ActivityRepository, JsonActivityRepository},
        alert_repository::{AlertRepository, JsonAlertRepository},
    },
};

fn repository(app: &AppHandle) -> Result<JsonAlertRepository, ApplicationError> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|err| ApplicationError {
            code: "ConfigurationError".into(),
            message: format!("could not resolve the application config directory: {err}"),
            remediation: None,
            retryable: false,
        })?;
    Ok(JsonAlertRepository::new(dir))
}

#[tauri::command]
pub fn get_alerts(app: AppHandle) -> Result<Vec<Alert>, ApplicationError> {
    let mut alerts = repository(&app)?.load().alerts;
    alerts.sort_by(|a, b| {
        let rank = |alert: &Alert| match alert.state {
            AlertState::Resolved => 3,
            _ => match alert.severity {
                crate::domain::alert::AlertSeverity::Critical => 0,
                crate::domain::alert::AlertSeverity::Warning => 1,
                crate::domain::alert::AlertSeverity::Info => 2,
            },
        };
        rank(a)
            .cmp(&rank(b))
            .then_with(|| b.last_seen.cmp(&a.last_seen))
    });
    Ok(alerts)
}

#[tauri::command]
pub fn acknowledge_alert(app: AppHandle, id: String) -> Result<Alert, ApplicationError> {
    let repo = repository(&app)?;
    let mut file = repo.load();
    let transition =
        acknowledge(&mut file, &id, chrono::Utc::now().to_rfc3339()).ok_or_else(|| {
            ApplicationError {
                code: "InvalidAlertState".into(),
                message: "Only active alerts can be acknowledged.".into(),
                remediation: None,
                retryable: false,
            }
        })?;
    repo.save(&file).map_err(|err| ApplicationError {
        code: "StorageError".into(),
        message: err.to_string(),
        remediation: Some("Check disk space and file permissions, then try again.".into()),
        retryable: true,
    })?;
    if let Ok(dir) = app.path().app_config_dir() {
        let activity = ActivityEvent::new(
            ActivityCategory::Health,
            "alert.acknowledged",
            transition.alert.device_id.clone(),
            Some(transition.alert.id.clone()),
            None,
            format!("Alert acknowledged: {}", transition.alert.summary),
        );
        if let Err(err) = JsonActivityRepository::new(dir).append(activity) {
            log::warn!("could not persist alert acknowledgement activity: {err}");
        }
    }
    Ok(transition.alert)
}
