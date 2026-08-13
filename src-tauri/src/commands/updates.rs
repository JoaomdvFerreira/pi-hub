use crate::{
    domain::{device::Device, update_intelligence::*},
    error::ApplicationError,
    infrastructure::ssh::{
        OpenSshExecutor, RemoteExecutor, RemoteOperation, RemoteOutputLimits, SshError, SshTarget,
    },
    storage::{
        device_repository::{DeviceRepository, JsonDeviceRepository},
        snapshot_repository::{JsonSnapshotRepository, SnapshotRepository},
    },
};
use chrono::{DateTime, Utc};
use std::time::Duration;
use tauri::{AppHandle, Manager};

const MAX_STDOUT: usize = 256 * 1024;
const MAX_STDERR: usize = 16 * 1024;
fn app_error(code: &str, message: &str, retryable: bool) -> ApplicationError {
    ApplicationError {
        code: code.into(),
        message: message.into(),
        remediation: Some("Try Check for Updates again after resolving the device issue.".into()),
        retryable,
    }
}
fn dir(app: &AppHandle) -> Result<std::path::PathBuf, ApplicationError> {
    app.path().app_config_dir().map_err(|_| {
        app_error(
            "ConfigurationError",
            "could not resolve application storage",
            false,
        )
    })
}
fn target(device: &Device) -> SshTarget {
    SshTarget {
        host: device.host.clone(),
        port: device.ssh_port,
        username: device.ssh_username.clone(),
    }
}
fn execute(
    executor: &dyn RemoteExecutor,
    target: &SshTarget,
    operation: RemoteOperation,
    timeout: Duration,
    label: &'static str,
) -> Result<String, UpdateFailure> {
    let mut m = crate::performance_diagnostics::measure(label);
    let result = executor
        .execute_bounded(
            target,
            operation.command().expect("fixed update command"),
            timeout,
            RemoteOutputLimits { stdout: MAX_STDOUT, stderr: MAX_STDERR },
        )
        .map_err(|e| failure(e));
    match result {
        Ok(value) => {
            if let Some(x) = m.as_mut() {
                x.set_bytes((value.stdout.len() + value.stderr.len()) as u64)
            }
            Ok(value.stdout)
        }
        Err(e) => {
            if let Some(x) = m.as_mut() {
                x.fail()
            }
            Err(e)
        }
    }
}
fn failure(error: SshError) -> UpdateFailure {
    let kind = match error {
        SshError::RemoteCommandTimeout | SshError::ConnectionTimeout => UpdateFailureKind::Timeout,
        SshError::OutputLimitExceeded => UpdateFailureKind::MalformedOutput,
        SshError::RemoteCommandError { .. } => UpdateFailureKind::RequiredCommand,
        _ => UpdateFailureKind::Transport,
    };
    UpdateFailure {
        kind,
        message: error.code().into(),
    }
}
fn field(raw: &str, key: &str) -> Option<String> {
    raw.lines()
        .find_map(|line| line.strip_prefix(key).map(str::to_string))
}
fn package(raw: &str) -> Result<UpdatePackages, String> {
    let mut packages = vec![];
    let mut total = 0_u32;
    for line in raw
        .lines()
        .filter_map(|x| x.strip_prefix("PIHUB_UPDATE_PACKAGE=Inst "))
    {
        let (name, rest) = line.split_once(" [").ok_or("invalidPackage")?;
        let (installed, rest) = rest.split_once("] (").ok_or("invalidPackage")?;
        let candidate = rest.split_whitespace().next().ok_or("invalidPackage")?;
        if [name, installed, candidate]
            .iter()
            .any(|x| x.is_empty() || x.len() > MAX_FIELD_BYTES)
        {
            return Err("invalidPackage".into());
        }
        total = total.saturating_add(1);
        if packages.len() < MAX_UPDATE_DETAILS {
            let (base, arch) = name
                .split_once(':')
                .map(|(a, b)| (a.to_string(), Some(b.to_string())))
                .unwrap_or((name.to_string(), None));
            packages.push(UpdatePackage {
                name: base,
                installed_version: installed.to_string(),
                candidate_version: candidate.to_string(),
                architecture: arch,
                held: None,
            });
        }
    }
    if !raw.contains("PIHUB_UPDATE_PACKAGES_DONE=1") {
        return Err("missingPackageCompletion".into());
    }
    Ok(UpdatePackages {
        total_count: Some(total),
        truncated: total as usize > MAX_UPDATE_DETAILS,
        packages,
    })
}
fn kept_back(raw: &str) -> Result<KeptBackPackages, String> {
    let mut packages = vec![]; let mut total = 0_u32;
    for name in raw.lines().filter_map(|x| x.strip_prefix("PIHUB_UPDATE_KEPT_BACK=")) {
        if name.is_empty() || name.len() > MAX_FIELD_BYTES { return Err("invalidKeptBack".into()); }
        total = total.saturating_add(1); if packages.len() < MAX_UPDATE_DETAILS { packages.push(name.to_string()); }
    }
    let reported_total = raw.lines().filter_map(|x| x.strip_prefix("PIHUB_UPDATE_KEPT_BACK_COUNT=")).collect::<Vec<_>>();
    if reported_total.len() != 1 { return Err("invalidKeptBackCount".into()); }
    let reported_total = reported_total[0].parse::<u32>().map_err(|_| "invalidKeptBackCount")?;
    if total > reported_total { return Err("invalidKeptBackCount".into()); }
    Ok(KeptBackPackages { total_count: Some(reported_total), packages, truncated: total as usize > MAX_UPDATE_DETAILS })
}
fn holds(raw: &str) -> Result<HeldPackages, String> {
    let mut packages = vec![];
    let mut total = 0_u32;
    for name in raw
        .lines()
        .filter_map(|x| x.strip_prefix("PIHUB_UPDATE_HOLD="))
    {
        if name.is_empty() || name.len() > MAX_FIELD_BYTES {
            return Err("invalidHold".into());
        }
        total = total.saturating_add(1);
        if packages.len() < MAX_UPDATE_DETAILS {
            packages.push(name.to_string())
        }
    }
    if !raw.contains("PIHUB_UPDATE_HOLDS_DONE=1") {
        return Err("missingHoldCompletion".into());
    }
    Ok(HeldPackages {
        status: HeldStatus::Known,
        total_count: Some(total),
        truncated: total as usize > MAX_UPDATE_DETAILS,
        packages,
    })
}

#[tauri::command]
pub fn get_update_result(
    app: AppHandle,
    device_id: String,
) -> Result<Option<UpdateCheckResult>, ApplicationError> {
    Ok(JsonSnapshotRepository::new(dir(&app)?).get_update_result(&device_id))
}
#[tauri::command]
pub async fn check_for_updates(
    app: AppHandle,
    device_id: String,
) -> Result<UpdateCheckResult, ApplicationError> {
    let coordinator = app.state::<crate::monitoring::update_concurrency::UpdateCheckCoordinator>();
    let claim = coordinator.try_claim(&device_id).ok_or_else(|| {
        app_error(
            "AlreadyChecking",
            "an update check is already running for this device",
            true,
        )
    })?;
    let app2 = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || check(&app2, &device_id))
        .await
        .map_err(|_| {
            app_error(
                "PlatformIntegrationError",
                "update check did not complete",
                true,
            )
        })?;
    drop(claim);
    result
}
fn check(app: &AppHandle, device_id: &str) -> Result<UpdateCheckResult, ApplicationError> {
    let device = JsonDeviceRepository::new(dir(app)?)
        .get(device_id)
        .ok_or_else(|| app_error("NotFoundError", "device was not found", false))?;
    let repo = JsonSnapshotRepository::new(dir(app)?);
    check_with(&device, &OpenSshExecutor::default(), &repo, Utc::now())
}
fn check_with(
    device: &Device,
    executor: &dyn RemoteExecutor,
    repo: &dyn SnapshotRepository,
    checked: DateTime<Utc>,
) -> Result<UpdateCheckResult, ApplicationError> {
    let check_measure = crate::performance_diagnostics::measure("device_updates.check");
    let target = target(device);
    let mut result = UpdateCheckResult::empty(device.id.clone());
    result.checked_at = Some(checked.to_rfc3339());
    let detect = match execute(
        executor,
        &target,
        RemoteOperation::UpdateDetect,
        Duration::from_secs(5),
        "device_updates.detect",
    ) {
        Ok(v) => v,
        Err(f) => return save_or_return(repo, result, Some(f), check_measure),
    };
    let os = field(&detect, "PIHUB_UPDATE_OS_ID=");
    let like = field(&detect, "PIHUB_UPDATE_OS_LIKE=");
    let commands = [
        "PIHUB_UPDATE_APT_GET=1",
        "PIHUB_UPDATE_DPKG_QUERY=1",
        "PIHUB_UPDATE_DPKG=1",
    ]
    .iter()
    .all(|x| detect.contains(x));
    let debian = os
        .as_deref()
        .is_some_and(|x| matches!(x, "debian" | "raspbian"))
        || like
            .as_deref()
            .is_some_and(|x| x.split_whitespace().any(|v| v == "debian"));
    result.support.os_id = os;
    result.support.os_version_id = field(&detect, "PIHUB_UPDATE_OS_VERSION_ID=");
    if !debian || !commands {
        result.status = UpdateStatus::Unsupported;
        result.support.status = SupportStatus::Unsupported;
        result.support.reason = Some(
            if !debian {
                "unsupportedPlatform"
            } else {
                "missingCapability"
            }
            .into(),
        );
        return save_or_return(repo, result, None, check_measure);
    }
    result.support.status = SupportStatus::Supported;
    result.support.package_manager = Some("aptDpkg".into());
    let packages_raw = match execute(
        executor,
        &target,
        RemoteOperation::UpdatePackages,
        Duration::from_secs(15),
        "device_updates.packages",
    ) {
        Ok(v) => v,
        Err(f) => return save_or_return(repo, result, Some(f), check_measure),
    };
    result.updates = match package(&packages_raw).map_err(|message| UpdateFailure { kind: UpdateFailureKind::MalformedOutput, message }) { Ok(v) => v, Err(f) => return save_or_return(repo, result, Some(f), check_measure) };
    result.kept_back_packages = match kept_back(&packages_raw).map_err(|message| UpdateFailure { kind: UpdateFailureKind::MalformedOutput, message }) { Ok(v) => v, Err(f) => return save_or_return(repo, result, Some(f), check_measure) };
    match execute(
        executor,
        &target,
        RemoteOperation::UpdateHolds,
        Duration::from_secs(5),
        "device_updates.holds",
    )
    .and_then(|raw| {
        holds(&raw).map_err(|message| UpdateFailure {
            kind: UpdateFailureKind::MalformedOutput,
            message,
        })
    }) {
        Ok(h) => {
            let listed: std::collections::HashSet<_> = h.packages.iter().collect();
            for p in &mut result.updates.packages {
                p.held = Some(
                    listed.contains(&p.name)
                        || p.architecture
                            .as_ref()
                            .is_some_and(|a| listed.contains(&format!("{}:{}", p.name, a))),
                );
            }
            result.held_packages = h
        }
        Err(_) => result.warnings.push("heldPackagesUnknown".into()),
    };
    match execute(
        executor,
        &target,
        RemoteOperation::UpdateMetadataAge,
        Duration::from_secs(5),
        "device_updates.metadata_age",
    ) {
        Ok(raw) => match field(&raw, "PIHUB_UPDATE_METADATA_MTIME=").as_deref() {
            Some("none") => result.package_metadata.status = MetadataStatus::Unavailable,
            Some(value) => {
                if let Ok(seconds) = value.parse::<f64>() {
                    let at = DateTime::from_timestamp(seconds as i64, 0).map(|x| x.to_rfc3339());
                    let age = checked.timestamp().saturating_sub(seconds as i64).max(0) as u64;
                    result.package_metadata.newest_metadata_at = at;
                    result.package_metadata.age_seconds = Some(age);
                    result.package_metadata.status = if age >= STALE_AFTER_SECONDS {
                        MetadataStatus::Stale
                    } else {
                        MetadataStatus::Fresh
                    }
                } else {
                    result.warnings.push("metadataAgeUnknown".into())
                }
            }
            None => result.warnings.push("metadataAgeUnknown".into()),
        },
        Err(_) => result.warnings.push("metadataAgeUnknown".into()),
    };
    if let Ok(raw) = execute(
        executor,
        &target,
        RemoteOperation::UpdateRebootState,
        Duration::from_secs(5),
        "device_updates.reboot_state",
    ) {
        result.reboot = if raw.contains("PIHUB_UPDATE_REBOOT=required") {
            RebootState::Required
        } else {
            RebootState::Unknown
        }
    } else {
        result.warnings.push("rebootStateUnknown".into())
    };
    result.status = if result.package_metadata.status == MetadataStatus::Stale {
        UpdateStatus::Stale
    } else if result.updates.total_count == Some(0) && result.kept_back_packages.total_count == Some(0) {
        UpdateStatus::UpToDate
    } else {
        UpdateStatus::UpdatesAvailable
    };
    save_or_return(repo, result, None, check_measure)
}
fn save_or_return(
    repo: &dyn SnapshotRepository,
    mut result: UpdateCheckResult,
    failure: Option<UpdateFailure>,
    mut measure: Option<pihub_benchmark_core::OperationMeasurement<'static>>,
) -> Result<UpdateCheckResult, ApplicationError> {
    if let Some(f) = failure {
        result.status = UpdateStatus::CheckFailed;
        result.failure = Some(f)
    }
    let mut persist = crate::performance_diagnostics::measure("device_updates.persist");
    if let Err(_) = repo.upsert_update_result(&result) {
        result.status = UpdateStatus::Unknown;
        result.warnings.push("persistenceFailed".into());
        if let Some(m) = persist.as_mut() {
            m.fail()
        }
    } else if let Some(m) = persist.as_mut() {
        m.set_bytes(
            serde_json::to_vec(&result)
                .map(|x| x.len() as u64)
                .unwrap_or(0),
        )
    }
    if let Some(m) = measure.as_mut() {
        m.set_bytes(
            serde_json::to_vec(&result)
                .map(|x| x.len() as u64)
                .unwrap_or(0),
        );
        if matches!(result.status, UpdateStatus::CheckFailed) {
            m.fail()
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::device::DeviceType, infrastructure::ssh::RemoteExecutionResult,
        storage::snapshot_repository::JsonSnapshotRepository,
    };
    use std::{
        collections::VecDeque,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Mutex,
        },
    };
    struct Script {
        replies: Mutex<VecDeque<Result<RemoteExecutionResult, SshError>>>,
        calls: AtomicUsize,
    }
    impl Script {
        fn new(replies: Vec<Result<RemoteExecutionResult, SshError>>) -> Self {
            Self {
                replies: Mutex::new(replies.into()),
                calls: AtomicUsize::new(0),
            }
        }
    }
    impl RemoteExecutor for Script {
        fn execute(
            &self,
            _: &SshTarget,
            _: &str,
            _: Duration,
        ) -> Result<RemoteExecutionResult, SshError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let mut m = crate::performance_diagnostics::measure("ssh.execute");
            let value = self.replies.lock().unwrap().pop_front().unwrap();
            match &value {
                Ok(x) => {
                    if let Some(m) = m.as_mut() {
                        m.set_bytes((x.stdout.len() + x.stderr.len()) as u64)
                    }
                }
                Err(_) => {
                    if let Some(m) = m.as_mut() {
                        m.fail()
                    }
                }
            }
            value
        }
    }
    fn ok(s: &str) -> Result<RemoteExecutionResult, SshError> {
        Ok(RemoteExecutionResult {
            exit_code: Some(0),
            stdout: s.into(),
            stderr: String::new(),
            duration_ms: 5,
            timed_out: false,
        })
    }
    fn device() -> Device {
        Device {
            id: "d".into(),
            name: "D".into(),
            host: "fixture.invalid".into(),
            ssh_port: 22,
            ssh_username: "fixture".into(),
            description: None,
            device_type: DeviceType::RaspberryPi,
            monitoring_enabled: true,
            refresh_interval_seconds: None,
            notify_on_device_offline: true,
            notify_on_container_failure: true,
            notify_on_container_unhealthy: true,
            services: vec![],
            created_at: "x".into(),
            updated_at: "x".into(),
        }
    }
    fn detect() -> &'static str {
        "PIHUB_UPDATE_OS_ID=debian\nPIHUB_UPDATE_OS_LIKE=debian\nPIHUB_UPDATE_APT_GET=1\nPIHUB_UPDATE_DPKG_QUERY=1\nPIHUB_UPDATE_DPKG=1\n"
    }
    fn full(
        packages: &str,
        holds: &str,
        mtime: &str,
    ) -> Vec<Result<RemoteExecutionResult, SshError>> {
        let kept_back_count = if packages.contains("PIHUB_UPDATE_KEPT_BACK_COUNT=") { "" } else { "PIHUB_UPDATE_KEPT_BACK_COUNT=0\n" };
        vec![
            ok(detect()),
            ok(&format!("{packages}{kept_back_count}PIHUB_UPDATE_PACKAGES_DONE=1\n")),
            ok(&format!("{holds}PIHUB_UPDATE_HOLDS_DONE=1\n")),
            ok(&format!("PIHUB_UPDATE_METADATA_MTIME={mtime}\n")),
            ok("PIHUB_UPDATE_REBOOT=unknown\n"),
        ]
    }
    #[test]
    fn parses_bounded_package_fixture() {
        let raw="PIHUB_UPDATE_PACKAGE=Inst bash [5.2] (5.3 Debian:stable [amd64])\nPIHUB_UPDATE_PACKAGES_DONE=1\n";
        let got = package(raw).unwrap();
        assert_eq!(got.total_count, Some(1));
        assert_eq!(got.packages[0].name, "bash");
        assert_eq!(got.packages[0].architecture.as_deref(), None);
    }
    #[test]
    fn rejects_missing_completion() {
        assert!(package("PIHUB_UPDATE_PACKAGE=Inst x [1] (2)\n").is_err());
    }
    #[test]
    fn parses_real_style_upgrade_and_kept_back_records() {
        let raw = "PIHUB_UPDATE_PACKAGE=Inst bash [5.2] (5.3 Debian:stable [amd64])\nPIHUB_UPDATE_KEPT_BACK=linux-image\nPIHUB_UPDATE_KEPT_BACK=firmware\nPIHUB_UPDATE_KEPT_BACK_COUNT=2\nPIHUB_UPDATE_PACKAGES_DONE=1\n";
        assert_eq!(package(raw).unwrap().total_count, Some(1));
        assert_eq!(kept_back(raw).unwrap().packages, ["linux-image", "firmware"]);
    }
    #[test]
    fn kept_back_details_are_bounded() {
        let raw = (0..201)
            .map(|x| format!("PIHUB_UPDATE_KEPT_BACK=p{x}\n"))
            .collect::<String>();
        let got = kept_back(&format!("{raw}PIHUB_UPDATE_KEPT_BACK_COUNT=201\n")).unwrap();
        assert_eq!(got.total_count, Some(201));
        assert_eq!(got.packages.len(), 200);
        assert!(got.truncated);
        assert!(kept_back("PIHUB_UPDATE_KEPT_BACK=linux-image\nPIHUB_UPDATE_KEPT_BACK_COUNT=0\n").is_err());
    }
    #[test]
    fn legacy_persisted_result_defaults_kept_back_evidence() {
        let mut legacy = serde_json::to_value(UpdateCheckResult::empty("d".into())).unwrap();
        legacy.as_object_mut().unwrap().remove("keptBackPackages");
        let restored: UpdateCheckResult = serde_json::from_value(legacy).unwrap();
        assert_eq!(restored.kept_back_packages.total_count, None);
        assert!(restored.kept_back_packages.packages.is_empty());
    }
    #[test]
    fn holds_are_bounded() {
        let raw = format!(
            "{}PIHUB_UPDATE_HOLDS_DONE=1\n",
            (0..201)
                .map(|x| format!("PIHUB_UPDATE_HOLD=p{x}\n"))
                .collect::<String>()
        );
        let got = holds(&raw).unwrap();
        assert_eq!(got.total_count, Some(201));
        assert!(got.truncated);
    }
    #[test]
    fn real_orchestration_covers_matrix_and_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let now = DateTime::from_timestamp(1_000_000, 0).unwrap();
        let zero = Script::new(full("", "PIHUB_UPDATE_HOLD=bash\n", "999999"));
        let result = check_with(&device(), &zero, &repo, now).unwrap();
        assert_eq!(result.status, UpdateStatus::UpToDate);
        assert_eq!(zero.calls.load(Ordering::SeqCst), 5);
        assert_eq!(repo.get_update_result("d"), Some(result));
        let lines = (0..201)
            .map(|i| format!("PIHUB_UPDATE_PACKAGE=Inst p{i} [1] (2)\n"))
            .collect::<String>();
        let stale = check_with(&device(), &Script::new(full(&lines, "", "0")), &repo, now).unwrap();
        assert_eq!(stale.status, UpdateStatus::Stale);
        assert!(stale.updates.truncated);
        assert_eq!(stale.updates.packages.len(), 200);
        let unsupported = Script::new(vec![ok("PIHUB_UPDATE_OS_ID=fedora\n")]);
        assert_eq!(
            check_with(&device(), &unsupported, &repo, now)
                .unwrap()
                .status,
            UpdateStatus::Unsupported
        );
        assert_eq!(unsupported.calls.load(Ordering::SeqCst), 1);
        let partial = check_with(
            &device(),
            &Script::new(vec![
                ok(detect()),
                ok("PIHUB_UPDATE_KEPT_BACK_COUNT=0\nPIHUB_UPDATE_PACKAGES_DONE=1\n"),
                Err(SshError::RemoteCommandTimeout),
                ok("PIHUB_UPDATE_METADATA_MTIME=none\n"),
                ok("PIHUB_UPDATE_REBOOT=required\n"),
            ]),
            &repo,
            now,
        )
        .unwrap();
        assert!(partial.warnings.contains(&"heldPackagesUnknown".into()));
        assert_eq!(partial.reboot, RebootState::Required);
        let normal_and_kept = check_with(&device(), &Script::new(full("PIHUB_UPDATE_PACKAGE=Inst bash [5.2] (5.3 Debian:stable [amd64])\nPIHUB_UPDATE_KEPT_BACK=linux-image\nPIHUB_UPDATE_KEPT_BACK_COUNT=1\n", "", "999999")), &repo, now).unwrap();
        assert_eq!(normal_and_kept.status, UpdateStatus::UpdatesAvailable);
        assert_eq!(normal_and_kept.updates.total_count, Some(1));
        assert_eq!(normal_and_kept.kept_back_packages.packages, ["linux-image"]);
        let kept_only = check_with(&device(), &Script::new(full("PIHUB_UPDATE_KEPT_BACK=linux-image\nPIHUB_UPDATE_KEPT_BACK_COUNT=1\n", "PIHUB_UPDATE_HOLD=unrelated\n", "999999")), &repo, now).unwrap();
        assert_eq!(kept_only.status, UpdateStatus::UpdatesAvailable);
        assert_eq!(kept_only.updates.total_count, Some(0));
        assert_eq!(kept_only.kept_back_packages.packages, ["linux-image"]);
        assert_eq!(kept_only.held_packages.packages, ["unrelated"]);
        let summary_only = check_with(&device(), &Script::new(full("PIHUB_UPDATE_KEPT_BACK_COUNT=5\n", "", "999999")), &repo, now).unwrap();
        assert_eq!(summary_only.status, UpdateStatus::UpdatesAvailable);
        assert_eq!(summary_only.kept_back_packages.total_count, Some(5));
        assert!(summary_only.kept_back_packages.packages.is_empty());
    }
    #[test]
    fn real_orchestration_classifies_required_failure() {
        let dir = tempfile::tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let malformed = check_with(
            &device(),
            &Script::new(vec![ok(detect()), ok("PIHUB_UPDATE_PACKAGE=broken\n")]),
            &repo,
            Utc::now(),
        )
        .unwrap();
        assert_eq!(malformed.status, UpdateStatus::CheckFailed);
        assert_eq!(
            malformed.failure.unwrap().kind,
            UpdateFailureKind::MalformedOutput
        );
        let timeout = check_with(
            &device(),
            &Script::new(vec![Err(SshError::RemoteCommandTimeout)]),
            &repo,
            Utc::now(),
        )
        .unwrap();
        assert_eq!(timeout.failure.unwrap().kind, UpdateFailureKind::Timeout);
        let exit_100 = check_with(
            &device(),
            &Script::new(vec![ok(detect()), Err(SshError::RemoteCommandError { exit_code: Some(100), stderr: "Unable to fetch some archives".into() })]),
            &repo,
            Utc::now(),
        )
        .unwrap();
        assert_eq!(exit_100.status, UpdateStatus::CheckFailed);
        assert_eq!(exit_100.failure.unwrap().kind, UpdateFailureKind::RequiredCommand);
    }
    #[test]
    fn oversized_required_evidence_fails_without_persisting_raw_output() {
        let dir = tempfile::tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let marker = "OVERSIZED_UPDATE_EVIDENCE";
        let oversized = marker.repeat((MAX_STDOUT / marker.len()) + 1);
        let result = check_with(
            &device(),
            &Script::new(vec![ok(detect()), ok(&oversized)]),
            &repo,
            Utc::now(),
        )
        .unwrap();
        assert_eq!(result.status, UpdateStatus::CheckFailed);
        assert_eq!(result.failure.unwrap().kind, UpdateFailureKind::MalformedOutput);
        assert_eq!(
            repo.get_update_result("d").unwrap().status,
            UpdateStatus::CheckFailed
        );
        let persisted = std::fs::read_to_string(dir.path().join("state.json")).unwrap();
        assert!(!persisted.contains(marker));
    }
    #[test]
    fn diagnostics_capture_real_check_labels() {
        let _lock = crate::performance_diagnostics::test_measurement_lock()
            .lock()
            .unwrap();
        let _scope = crate::performance_diagnostics::enable_test_measurement_scope();
        let diagnostics = crate::performance_diagnostics::PerformanceDiagnostics::default();
        let _ = diagnostics.stop();
        diagnostics.start(Default::default()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let script = Script::new(full(
            "PIHUB_UPDATE_PACKAGE=Inst bash [1] (2)\n",
            "",
            "999999",
        ));
        check_with(
            &device(),
            &script,
            &repo,
            DateTime::from_timestamp(1_000_000, 0).unwrap(),
        )
        .unwrap();
        let report = diagnostics.stop().unwrap();
        let f = |name: &str| report.operations.iter().find(|x| x.name == name);
        assert_eq!(f("ssh.execute").unwrap().count, 5);
        for name in [
            "device_updates.check",
            "device_updates.detect",
            "device_updates.packages",
            "device_updates.holds",
            "device_updates.metadata_age",
            "device_updates.reboot_state",
            "device_updates.persist",
        ] {
            assert!(f(name).is_some())
        }
        assert!(f("device_updates.check").unwrap().bytes.unwrap_or(0) > 0);
        assert!(report.operations.iter().all(|x| !x.name.contains("bash")
            && !x.name.contains("fixture")
            && !x.name.contains("apt")));
    }
}
