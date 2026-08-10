use tauri::{AppHandle, Manager};

use crate::domain::snapshot::DeviceSnapshot;
use crate::error::ApplicationError;
use crate::monitoring::scheduler;
use crate::storage::snapshot_repository::{JsonSnapshotRepository, SnapshotRepository};
use crate::storage::activity_repository::{ActivityRepository, JsonActivityRepository};
use crate::domain::activity::ActivityEvent;
use crate::domain::historical::{HistoricalMetric, HistoricalRange, HistoricalSeries};
use crate::storage::historical_repository::{HistoricalRepository, JsonHistoricalRepository};

fn snapshot_repository(app: &AppHandle) -> Result<JsonSnapshotRepository, ApplicationError> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|err| ApplicationError {
            code: "ConfigurationError".into(),
            message: format!("could not resolve the application config directory: {err}"),
            remediation: None,
            retryable: false,
        })?;
    Ok(JsonSnapshotRepository::new(dir))
}

/// Refreshes one device and returns its new snapshot. Runs the SSH probe
/// and collection off the async runtime's worker threads, so it never
/// blocks the UI.
#[tauri::command]
pub async fn refresh_device(
    app: AppHandle,
    id: String,
) -> Result<DeviceSnapshot, ApplicationError> {
    scheduler::refresh_one(&app, &id).await
}

/// Refreshes every monitoring-enabled device concurrently (bounded by the
/// scheduler's concurrency limit) and returns whichever snapshots
/// completed; a failure on one device never prevents the others'
/// snapshots from being returned.
#[tauri::command]
pub async fn refresh_all_devices(app: AppHandle) -> Result<Vec<DeviceSnapshot>, ApplicationError> {
    scheduler::refresh_all(&app).await
}

/// Returns the last-known snapshot for a device, if any, without
/// triggering a new refresh.
#[tauri::command]
pub fn get_latest_snapshot(
    app: AppHandle,
    id: String,
) -> Result<Option<DeviceSnapshot>, ApplicationError> {
    Ok(snapshot_repository(&app)?.get(&id))
}

#[tauri::command]
pub fn get_activity(app: AppHandle) -> Result<Vec<ActivityEvent>, ApplicationError> {
    let dir = app.path().app_config_dir().map_err(|err| ApplicationError { code: "ConfigurationError".into(), message: format!("could not resolve the application config directory: {err}"), remediation: None, retryable: false })?;
    Ok(JsonActivityRepository::new(dir).load_all())
}

#[tauri::command]
pub fn get_historical_series(app: AppHandle, device_id: String, entity_id: Option<String>, metric: HistoricalMetric, range: HistoricalRange) -> Result<HistoricalSeries, ApplicationError> {
    let dir = app.path().app_config_dir().map_err(|err| ApplicationError { code: "ConfigurationError".into(), message: format!("could not resolve the application config directory: {err}"), remediation: None, retryable: false })?;
    Ok(JsonHistoricalRepository::new(dir).query(&device_id, entity_id.as_deref(), metric, range, chrono::Utc::now()))
}
