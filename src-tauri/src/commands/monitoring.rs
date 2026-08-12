use tauri::{AppHandle, Manager};

use crate::domain::snapshot::DeviceSnapshot;
use crate::error::ApplicationError;
use crate::monitoring::scheduler;
use crate::storage::snapshot_repository::{JsonSnapshotRepository, SnapshotRepository};
use crate::storage::activity_repository::{ActivityRepository, JsonActivityRepository};
use crate::domain::activity::ActivityEvent;
use crate::domain::historical::{HistoricalMetric, HistoricalRange, HistoricalSeries};
use crate::storage::historical_repository::{HistoricalRepository, JsonHistoricalRepository};
use crate::performance_diagnostics::PerformanceDiagnostics;
use pihub_benchmark_core::{BenchmarkConfig, BenchmarkReport, BenchmarkStatus};
use crate::monitoring::synthetic::{self, SyntheticProfile, SyntheticWorkloadResult};

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
    let mut measurement = crate::performance_diagnostics::measure("tauri.refresh_device");
    let result = scheduler::refresh_one(&app, &id).await;
    if let Ok(snapshot)=&result { if let Some(item)=measurement.as_mut() { item.set_bytes(serde_json::to_vec(snapshot).map(|value| value.len() as u64).unwrap_or(0)); } } else if let Some(item)=measurement.as_mut() { item.fail(); }
    result
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
    let mut measurement = crate::performance_diagnostics::measure("tauri.get_latest_snapshot");
    let result = snapshot_repository(&app)?.get(&id);
    if let Some(snapshot)=&result { if let Some(item)=measurement.as_mut() { item.set_bytes(serde_json::to_vec(snapshot).map(|value| value.len() as u64).unwrap_or(0)); } }
    Ok(result)
}

#[tauri::command]
pub fn get_activity(app: AppHandle) -> Result<Vec<ActivityEvent>, ApplicationError> {
    let dir = app.path().app_config_dir().map_err(|err| ApplicationError { code: "ConfigurationError".into(), message: format!("could not resolve the application config directory: {err}"), remediation: None, retryable: false })?;
    Ok(JsonActivityRepository::new(dir).load_all())
}

#[tauri::command]
pub fn get_device_activity(app: AppHandle, device_id: String) -> Result<Vec<ActivityEvent>, ApplicationError> {
    let mut measurement = crate::performance_diagnostics::measure("activity.read");
    let dir = app.path().app_config_dir().map_err(|err| ApplicationError { code: "ConfigurationError".into(), message: format!("could not resolve the application config directory: {err}"), remediation: None, retryable: false })?;
    let result = JsonActivityRepository::new(dir).load_for_device(&device_id);
    if let Some(item)=measurement.as_mut() { item.set_bytes(serde_json::to_vec(&result).map(|value| value.len() as u64).unwrap_or(0)); }
    Ok(result)
}

#[tauri::command]
pub fn get_historical_series(app: AppHandle, device_id: String, entity_id: Option<String>, metric: HistoricalMetric, range: HistoricalRange) -> Result<HistoricalSeries, ApplicationError> {
    let mut measurement = crate::performance_diagnostics::measure("history.query");
    let dir = app.path().app_config_dir().map_err(|err| ApplicationError { code: "ConfigurationError".into(), message: format!("could not resolve the application config directory: {err}"), remediation: None, retryable: false })?;
    let result = JsonHistoricalRepository::new(dir).query(&device_id, entity_id.as_deref(), metric, range, chrono::Utc::now());
    if let Some(item)=measurement.as_mut() { item.set_bytes(serde_json::to_vec(&result).map(|value| value.len() as u64).unwrap_or(0)); }
    Ok(result)
}

fn benchmark_error(message: String) -> ApplicationError { ApplicationError { code: "BenchmarkError".into(), message, remediation: Some("Stop the active session or review the benchmark configuration.".into()), retryable: false } }

#[tauri::command]
pub fn start_performance_benchmark(app: AppHandle, config: Option<BenchmarkConfig>) -> Result<BenchmarkStatus, ApplicationError> {
    app.state::<PerformanceDiagnostics>().start(config.unwrap_or_default()).map_err(benchmark_error)
}
#[tauri::command]
pub fn stop_performance_benchmark(app: AppHandle) -> Result<Option<BenchmarkReport>, ApplicationError> { Ok(app.state::<PerformanceDiagnostics>().stop()) }
#[tauri::command]
pub fn get_performance_benchmark_status(app: AppHandle) -> BenchmarkStatus { app.state::<PerformanceDiagnostics>().status() }
#[tauri::command]
pub fn get_performance_benchmark_report(app: AppHandle) -> Option<BenchmarkReport> { app.state::<PerformanceDiagnostics>().report() }
#[tauri::command]
pub fn run_synthetic_benchmark_profile(profile: SyntheticProfile) -> SyntheticWorkloadResult { synthetic::run(profile) }
