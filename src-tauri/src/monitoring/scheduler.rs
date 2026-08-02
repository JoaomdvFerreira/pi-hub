use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::domain::device::Device;
use crate::domain::notification_rule::NotificationEvent;
use crate::domain::settings::AppSettings;
use crate::domain::snapshot::DeviceSnapshot;
use crate::error::ApplicationError;
use crate::infrastructure::ssh::OpenSshExecutor;
use crate::monitoring::concurrency::RefreshCoordinator;
use crate::monitoring::container_diff::detect_container_changes;
use crate::monitoring::notifications::{evaluate_snapshot_transition, NotificationService};
use crate::monitoring::refresh::refresh_device_sync;
use crate::monitoring::scheduling::is_due;
use crate::platform::notifications::TauriNotificationService;
use crate::storage::config_repository::{JsonSettingsRepository, SettingsRepository};
use crate::storage::device_repository::{DeviceRepository, JsonDeviceRepository};
use crate::storage::snapshot_repository::{JsonSnapshotRepository, SnapshotRepository};

/// Max concurrent device refreshes (spec section 14.3).
pub const MAX_CONCURRENT_REFRESHES: usize = 4;
/// How often the background loop checks whether any device is due for an
/// automatic refresh. Independent of any individual device's own refresh
/// interval.
const TICK_INTERVAL: Duration = Duration::from_secs(5);

fn config_error(err: impl std::fmt::Display) -> ApplicationError {
    ApplicationError {
        code: "ConfigurationError".into(),
        message: format!("could not resolve the application config directory: {err}"),
        remediation: None,
        retryable: false,
    }
}

fn device_repository(app: &AppHandle) -> Result<JsonDeviceRepository, ApplicationError> {
    let dir = app.path().app_config_dir().map_err(config_error)?;
    Ok(JsonDeviceRepository::new(dir))
}

fn snapshot_repository(app: &AppHandle) -> Result<JsonSnapshotRepository, ApplicationError> {
    let dir = app.path().app_config_dir().map_err(config_error)?;
    Ok(JsonSnapshotRepository::new(dir))
}

fn settings_repository(app: &AppHandle) -> Result<JsonSettingsRepository, ApplicationError> {
    let dir = app.path().app_config_dir().map_err(config_error)?;
    Ok(JsonSettingsRepository::new(dir))
}

fn not_found_error(device_id: &str) -> ApplicationError {
    ApplicationError {
        code: "NotFoundError".into(),
        message: format!("device '{device_id}' was not found"),
        remediation: None,
        retryable: false,
    }
}

/// Refreshes exactly one device, respecting the `RefreshCoordinator`
/// (managed Tauri state, registered at startup) for per-device dedup and
/// bounded concurrency, persisting the resulting snapshot, and emitting
/// the backend events. If a refresh for this device is already in
/// progress, returns the latest known snapshot instead of starting a
/// second one.
pub async fn refresh_one(app: &AppHandle, device_id: &str) -> Result<DeviceSnapshot, ApplicationError> {
    let coordinator = app.state::<RefreshCoordinator>();
    if !coordinator.try_claim(device_id) {
        let repo = snapshot_repository(app)?;
        return repo.get(device_id).ok_or_else(|| not_found_error(device_id));
    }

    let result = do_refresh(app, device_id).await;
    coordinator.release(device_id);
    result
}

async fn do_refresh(app: &AppHandle, device_id: &str) -> Result<DeviceSnapshot, ApplicationError> {
    let device_repo = device_repository(app)?;
    let device = device_repo
        .get(device_id)
        .ok_or_else(|| not_found_error(device_id))?;

    let snapshot_repo = snapshot_repository(app)?;
    let previous = snapshot_repo.get(device_id);

    let _ = app.emit("monitoring://refresh-started", device_id);

    let coordinator = app.state::<RefreshCoordinator>();
    let _permit = coordinator.acquire_permit().await;

    let device_for_task = device.clone();
    let previous_for_task = previous.clone();
    let snapshot = tauri::async_runtime::spawn_blocking(move || {
        let executor = OpenSshExecutor::default();
        refresh_device_sync(&executor, &device_for_task, previous_for_task.as_ref())
    })
    .await
    .map_err(|err| ApplicationError {
        code: "PlatformIntegrationError".into(),
        message: format!("the refresh task failed unexpectedly: {err}"),
        remediation: Some("Try again.".into()),
        retryable: true,
    })?;

    snapshot_repo.upsert(&snapshot).map_err(|err| ApplicationError {
        code: "StorageError".into(),
        message: err.to_string(),
        remediation: Some("Check disk space and file permissions, then try again.".into()),
        retryable: true,
    })?;

    let _ = app.emit("device://snapshot-updated", &snapshot);

    if previous
        .as_ref()
        .map(|p| p.connection_status)
        .is_none_or(|prev_status| prev_status != snapshot.connection_status)
    {
        let _ = app.emit("device://status-changed", &snapshot);
    }

    let container_changes = detect_container_changes(
        previous.as_ref().map(|p| p.containers.as_slice()).unwrap_or(&[]),
        &snapshot.containers,
    );
    if !container_changes.is_empty() {
        let _ = app.emit("container://status-changed", (device_id, &container_changes));
    }

    let notifications =
        evaluate_snapshot_transition(&snapshot_repo, device_id, previous.as_ref(), &snapshot);
    if !notifications.is_empty() {
        let _ = app.emit("notification://ready", &notifications);
        dispatch_notifications(app, &device, &notifications);
    }

    let _ = app.emit("monitoring://refresh-completed", device_id);

    Ok(snapshot)
}

/// Shows a native notification for each event, gated by both the global
/// and the per-device `notificationsEnabled` preference (spec: "can be
/// disabled globally or per device"). A settings-load failure defaults to
/// enabled rather than silently suppressing notifications.
fn dispatch_notifications(app: &AppHandle, device: &Device, events: &[NotificationEvent]) {
    if !device.notifications_enabled {
        return;
    }
    let global_enabled = settings_repository(app)
        .map(|repo| repo.load().notifications_enabled)
        .unwrap_or(true);
    if !global_enabled {
        return;
    }

    let service = TauriNotificationService::new(app, device.name.clone());
    for event in events {
        service.notify(event);
    }
}

/// Refreshes every monitoring-enabled device concurrently (bounded by the
/// managed `RefreshCoordinator`'s semaphore), never letting one device's
/// failure prevent the others from being attempted.
pub async fn refresh_all(app: &AppHandle) -> Result<Vec<DeviceSnapshot>, ApplicationError> {
    let devices = device_repository(app)?.load_all();

    let mut handles = Vec::new();
    for device in devices.into_iter().filter(|d| d.monitoring_enabled) {
        let app = app.clone();
        handles.push(tauri::async_runtime::spawn(async move {
            refresh_one(&app, &device.id).await
        }));
    }

    let mut snapshots = Vec::new();
    for handle in handles {
        if let Ok(Ok(snapshot)) = handle.await {
            snapshots.push(snapshot);
        }
        // A join failure or a per-device error is intentionally swallowed
        // here: one device's failure must never prevent the others' results
        // from being returned.
    }
    Ok(snapshots)
}

fn refresh_interval_for(device_interval: Option<u32>, default_interval: u32) -> Duration {
    Duration::from_secs(device_interval.unwrap_or(default_interval) as u64)
}

async fn tick(app: &AppHandle) {
    let devices = match device_repository(app) {
        Ok(repo) => repo.load_all(),
        Err(err) => {
            log::warn!("monitoring tick could not load devices: {}", err.message);
            return;
        }
    };
    let default_interval = settings_repository(app)
        .map(|repo| repo.load())
        .unwrap_or_else(|_| AppSettings::default())
        .refresh_interval_seconds;
    let snapshot_repo = match snapshot_repository(app) {
        Ok(repo) => repo,
        Err(err) => {
            log::warn!(
                "monitoring tick could not open snapshot storage: {}",
                err.message
            );
            return;
        }
    };

    for device in devices.into_iter().filter(|d| d.monitoring_enabled) {
        let interval = refresh_interval_for(device.refresh_interval_seconds, default_interval);
        let last_captured_at = snapshot_repo.get(&device.id).map(|s| s.captured_at);
        if is_due(last_captured_at.as_deref(), interval) {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let _ = refresh_one(&app, &device.id).await;
            });
        }
    }
}

/// Starts the background tick loop that drives automatic refresh. Runs for
/// the lifetime of the application; intended to be called once from the
/// Tauri `setup()` hook, after `RefreshCoordinator` has been registered as
/// managed state.
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(TICK_INTERVAL).await;
            tick(&app).await;
        }
    });
}
