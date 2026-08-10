use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::domain::activity::{ActivityCategory, ActivityEvent};
use crate::domain::alert::AlertTransitionKind;
use crate::domain::device::Device;
use crate::domain::notification_rule::NotificationEvent;
use crate::domain::service_health::ServiceHealthState;
use crate::domain::settings::AppSettings;
use crate::domain::snapshot::DeviceSnapshot;
use crate::error::ApplicationError;
use crate::infrastructure::ssh::OpenSshExecutor;
use crate::monitoring::alerts::evaluate_with_expected_disruption as evaluate_alerts;
use crate::monitoring::concurrency::RefreshCoordinator;
use crate::monitoring::container_diff::detect_container_changes;
use crate::monitoring::notifications::{evaluate_snapshot_transition, NotificationService};
use crate::monitoring::refresh::refresh_device_sync_with_policy;
use crate::monitoring::scheduling::is_due;
use crate::platform::notifications::TauriNotificationService;
use crate::storage::activity_repository::{ActivityRepository, JsonActivityRepository};
use crate::storage::administration_repository::{
    AdministrationRepository, JsonAdministrationRepository,
};
use crate::storage::alert_repository::{AlertRepository, JsonAlertRepository};
use crate::storage::config_repository::{JsonSettingsRepository, SettingsRepository};
use crate::storage::device_repository::{DeviceRepository, JsonDeviceRepository};
use crate::storage::snapshot_repository::{JsonSnapshotRepository, SnapshotRepository};
use crate::storage::historical_repository::{HistoricalRepository, JsonHistoricalRepository};

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
fn historical_repository(app: &AppHandle) -> Result<JsonHistoricalRepository, ApplicationError> {
    let dir = app.path().app_config_dir().map_err(config_error)?;
    Ok(JsonHistoricalRepository::new(dir))
}

fn activity_repository(app: &AppHandle) -> Result<JsonActivityRepository, ApplicationError> {
    let dir = app.path().app_config_dir().map_err(config_error)?;
    Ok(JsonActivityRepository::new(dir))
}

fn alert_repository(app: &AppHandle) -> Result<JsonAlertRepository, ApplicationError> {
    let dir = app.path().app_config_dir().map_err(config_error)?;
    Ok(JsonAlertRepository::new(dir))
}

fn record_activity(app: &AppHandle, event: ActivityEvent) {
    if let Ok(repo) = activity_repository(app) {
        if let Err(err) = repo.append(event) {
            log::warn!("could not persist activity event: {err}");
        }
    }
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
pub async fn refresh_one(
    app: &AppHandle,
    device_id: &str,
) -> Result<DeviceSnapshot, ApplicationError> {
    let coordinator = app.state::<RefreshCoordinator>();
    let Some(_claim) = coordinator.try_claim(device_id) else {
        let repo = snapshot_repository(app)?;
        return repo
            .get(device_id)
            .ok_or_else(|| not_found_error(device_id));
    };

    // `_claim` releases the device on drop -- at the end of this scope,
    // regardless of how do_refresh returns (Ok, Err, or a panic unwinding
    // through here) -- so there is no separate release call to forget.
    do_refresh(app, device_id).await
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

    let settings = settings_repository(app)?.load();
    let policy = settings.effective_threshold_policy(&device.id);
    let device_for_task = device.clone();
    let previous_for_task = previous.clone();
    let policy_for_task = policy.clone();
    let snapshot = tauri::async_runtime::spawn_blocking(move || {
        let executor = OpenSshExecutor::default();
        refresh_device_sync_with_policy(
            &executor,
            &device_for_task,
            previous_for_task.as_ref(),
            &policy_for_task,
        )
    })
    .await
    .map_err(|err| ApplicationError {
        code: "PlatformIntegrationError".into(),
        message: format!("the refresh task failed unexpectedly: {err}"),
        remediation: Some("Try again.".into()),
        retryable: true,
    })?;

    snapshot_repo
        .upsert(&snapshot)
        .map_err(|err| ApplicationError {
            code: "StorageError".into(),
            message: err.to_string(),
            remediation: Some("Check disk space and file permissions, then try again.".into()),
            retryable: true,
        })?;

    // Historical persistence is deliberately best-effort: a full disk or a
    // corrupt history file must never make the current refresh unusable.
    if let Ok(repo) = historical_repository(app) {
        if let Err(err) = repo.append_snapshot(&snapshot, chrono::Utc::now()) {
            log::warn!("could not persist historical monitoring sample: {err}");
        }
    }

    let _ = app.emit("device://snapshot-updated", &snapshot);

    // Alert transitions are persisted before any notification can be shown. This is
    // intentionally separate from M7 Activity: alerts are current conditions, while
    // Activity is an audit trail of their meaningful lifecycle changes.
    let alert_transitions = {
        let repo = alert_repository(app)?;
        let mut file = repo.load();
        let disruption_repo =
            JsonAdministrationRepository::new(app.path().app_config_dir().map_err(config_error)?);
        let expected = disruption_repo.get_valid(&device.id);
        let suppress_device_offline = expected.is_some()
            && snapshot.connection_status
                != crate::domain::connection_status::DeviceConnectionStatus::Online;
        // An observed return online proves recovery and clears every valid
        // M10 marker, including planned shutdown after a manual power-on.
        if snapshot.connection_status
            == crate::domain::connection_status::DeviceConnectionStatus::Online
            && expected.is_some()
        {
            let _ = disruption_repo.clear(&device.id);
        }
        let transitions = evaluate_alerts(
            &mut file,
            &device,
            &snapshot,
            &policy,
            suppress_device_offline,
        );
        JsonAlertRepository::prune(&mut file);
        repo.save(&file).map_err(|err| ApplicationError {
            code: "StorageError".into(),
            message: format!("could not persist alerts: {err}"),
            remediation: Some("Check disk space and file permissions, then try again.".into()),
            retryable: true,
        })?;
        transitions
    };
    for transition in &alert_transitions {
        let (code, summary) = match transition.kind {
            AlertTransitionKind::Activated => (
                "alert.activated",
                format!("Alert activated: {}", transition.alert.summary),
            ),
            AlertTransitionKind::Escalated => (
                "alert.escalated",
                format!("Alert escalated: {}", transition.alert.summary),
            ),
            AlertTransitionKind::Acknowledged => (
                "alert.acknowledged",
                format!("Alert acknowledged: {}", transition.alert.summary),
            ),
            AlertTransitionKind::Resolved => (
                "alert.resolved",
                format!("Alert resolved: {}", transition.alert.summary),
            ),
        };
        record_activity(
            app,
            ActivityEvent::new(
                ActivityCategory::Health,
                code,
                transition.alert.device_id.clone(),
                Some(transition.alert.id.clone()),
                None,
                summary,
            ),
        );
    }

    if previous
        .as_ref()
        .map(|p| p.connection_status)
        .is_none_or(|prev_status| prev_status != snapshot.connection_status)
    {
        let _ = app.emit("device://status-changed", &snapshot);
        let code = if snapshot.connection_status
            == crate::domain::connection_status::DeviceConnectionStatus::Online
        {
            "device.online"
        } else {
            "device.offline"
        };
        record_activity(
            app,
            ActivityEvent::new(
                ActivityCategory::Device,
                code,
                Some(device.id.clone()),
                Some(device.id.clone()),
                Some(device.name.clone()),
                format!(
                    "{} is {}",
                    device.name,
                    if code == "device.online" {
                        "online"
                    } else {
                        "offline"
                    }
                ),
            ),
        );
    }

    if previous.as_ref().map(|item| item.health.state) != Some(snapshot.health.state) {
        record_activity(
            app,
            ActivityEvent::new(
                ActivityCategory::Health,
                "device.health_changed",
                Some(device.id.clone()),
                Some(device.id.clone()),
                Some(device.name.clone()),
                format!("{} health is now {:?}", device.name, snapshot.health.state),
            ),
        );
    }

    for service in &device.services {
        let Some(current) = snapshot.service_health.get(&service.id) else {
            continue;
        };
        let prior = previous
            .as_ref()
            .and_then(|item| item.service_health.get(&service.id))
            .map(|item| item.state);
        if prior != Some(current.state)
            && !(prior.is_none() && current.state == ServiceHealthState::Healthy)
        {
            record_activity(
                app,
                ActivityEvent::new(
                    ActivityCategory::Service,
                    "service.health_changed",
                    Some(device.id.clone()),
                    Some(service.id.clone()),
                    Some(service.name.clone()),
                    format!("{} is now {:?}", service.name, current.state),
                ),
            );
        }
    }

    let container_changes = detect_container_changes(
        previous
            .as_ref()
            .map(|p| p.containers.as_slice())
            .unwrap_or(&[]),
        &snapshot.containers,
    );
    if !container_changes.is_empty() {
        let _ = app.emit(
            "container://status-changed",
            (device_id, &container_changes),
        );
    }

    // Device-offline transitions now flow through governed alert transitions; legacy
    // container notifications remain outside M8's Docker expansion boundary.
    let notifications: Vec<_> =
        evaluate_snapshot_transition(&snapshot_repo, &device, previous.as_ref(), &snapshot)
            .into_iter()
            .filter(|event| event.resource_id != device.id)
            .collect();
    if !notifications.is_empty() {
        let _ = app.emit("notification://ready", &notifications);
        dispatch_notifications(app, &device, &notifications);
    }
    let alert_notifications: Vec<NotificationEvent> = alert_transitions
        .iter()
        .filter_map(|transition| match transition.kind {
            AlertTransitionKind::Activated | AlertTransitionKind::Escalated
                if matches!(
                    transition.alert.severity,
                    crate::domain::alert::AlertSeverity::Warning
                        | crate::domain::alert::AlertSeverity::Critical
                ) =>
            {
                Some(NotificationEvent {
                    device_id: device.id.clone(),
                    resource_id: transition.alert.id.clone(),
                    previous_state: "alert".into(),
                    current_state: format!("{:?}", transition.alert.severity),
                    message: transition.alert.summary.clone(),
                })
            }
            _ => None,
        })
        .collect();
    if !alert_notifications.is_empty() {
        let _ = app.emit("notification://ready", &alert_notifications);
        dispatch_notifications(app, &device, &alert_notifications);
    }

    let _ = app.emit("monitoring://refresh-completed", device_id);

    Ok(snapshot)
}

/// Shows a native notification for each event, gated by the global
/// `notificationsEnabled` kill switch. Per-category gating (offline/
/// container-failure/container-unhealthy) already happened inside
/// `evaluate_snapshot_transition` -- `events` here only ever contains
/// categories this device has enabled, so there is nothing left to check
/// per-device. A settings-load failure defaults to enabled rather than
/// silently suppressing notifications.
fn dispatch_notifications(app: &AppHandle, device: &Device, events: &[NotificationEvent]) {
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
