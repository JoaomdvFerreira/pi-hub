use std::{collections::BTreeMap, time::Duration};

use chrono::{Duration as ChronoDuration, Utc};
use tauri::{AppHandle, Manager};

use crate::{
    commands::updates::{check_with, kept_back, package},
    domain::{
        device::Device,
        maintenance::{
            MaintenanceDispatchState, MaintenanceFailure, MaintenanceOperation,
            MaintenanceOperationState, PlanFingerprint, PreDispatchFailureStage,
            PLAN_FINGERPRINT_VERSION,
        },
        update_intelligence::{
            MetadataStatus, RebootState, SupportStatus, UpdateCheckResult, UpdateStatus,
        },
    },
    error::ApplicationError,
    infrastructure::ssh::{
        operation::{
            maintenance_cleanup_command, maintenance_dispatch_command, maintenance_journal_command,
            maintenance_status_command,
        },
        OpenSshExecutor, RemoteExecutionResult, RemoteExecutor, RemoteOperation,
        RemoteOutputLimits, SshError, SshTarget,
    },
    monitoring::maintenance_coordinator::{DeviceMaintenanceCoordinator, MaintenanceOperationKind},
    storage::{
        activity_repository::{ActivityRepository, JsonActivityRepository},
        administration_repository::{AdministrationRepository, JsonAdministrationRepository},
        device_repository::{DeviceRepository, JsonDeviceRepository},
        snapshot_repository::{JsonSnapshotRepository, SnapshotRepository},
    },
};

const OUTPUT: RemoteOutputLimits = RemoteOutputLimits {
    stdout: 256 * 1024,
    stderr: 16 * 1024,
};
const STATUS_OUTPUT: RemoteOutputLimits = RemoteOutputLimits {
    stdout: 16 * 1024,
    stderr: 16 * 1024,
};
const SHORT: Duration = Duration::from_secs(15);
const REFRESH: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedDeviceUpdate {
    pub operation: MaintenanceOperation,
    pub plan: UpdateCheckResult,
}

fn app_error(code: &str, message: &str) -> ApplicationError {
    ApplicationError {
        code: code.into(),
        message: message.into(),
        remediation: Some("Resolve the device condition and prepare the update again.".into()),
        retryable: true,
    }
}
fn dir(app: &AppHandle) -> Result<std::path::PathBuf, ApplicationError> {
    app.path()
        .app_config_dir()
        .map_err(|e| app_error("ConfigurationError", &e.to_string()))
}
fn target(device: &Device) -> SshTarget {
    SshTarget {
        host: device.host.clone(),
        port: device.ssh_port,
        username: device.ssh_username.clone(),
    }
}
fn run(
    executor: &dyn RemoteExecutor,
    target: &SshTarget,
    command: &str,
    timeout: Duration,
    label: &'static str,
    limits: RemoteOutputLimits,
) -> Result<RemoteExecutionResult, SshError> {
    let mut measurement = crate::performance_diagnostics::measure(label);
    let result = executor.execute_bounded(target, command, timeout, limits);
    if let Some(m) = measurement.as_mut() {
        match &result {
            Ok(value) => m.set_bytes((value.stdout.len() + value.stderr.len()) as u64),
            Err(_) => m.fail(),
        }
    }
    result
}
fn maintenance_failure(error: SshError) -> MaintenanceFailure {
    match error {
        SshError::RemoteCommandError { stderr, .. }
            if stderr.contains("Could not get lock")
                || stderr.contains("Unable to acquire the dpkg") =>
        {
            MaintenanceFailure::PackageManagerBusy
        }
        SshError::RemoteCommandError { stderr, .. }
            if stderr.to_ascii_lowercase().contains("sudo")
                || stderr.to_ascii_lowercase().contains("password") =>
        {
            MaintenanceFailure::PrivilegeUnavailable
        }
        SshError::ConnectionRefused
        | SshError::ConnectionTimeout
        | SshError::RemoteCommandTimeout => {
            MaintenanceFailure::TransportUnavailableDuringObservation
        }
        _ => MaintenanceFailure::PackageOperationFailed,
    }
}
fn complete_preview(result: &UpdateCheckResult) -> bool {
    result.support.status == SupportStatus::Supported
        && result.updates.total_count.unwrap_or(0) > 0
        && !result.updates.truncated
        && !result.kept_back_packages.truncated
}
fn fingerprint(result: &UpdateCheckResult) -> PlanFingerprint {
    let mut lines: Vec<String> = result
        .updates
        .packages
        .iter()
        .map(|p| {
            format!(
                "upgrade\u{1f}{}{}\u{1f}{}\u{1f}{}\n",
                p.name,
                p.architecture
                    .as_ref()
                    .map(|a| format!(":{a}"))
                    .unwrap_or_default(),
                p.installed_version,
                p.candidate_version
            )
        })
        .collect();
    lines.extend(
        result
            .kept_back_packages
            .packages
            .iter()
            .map(|p| format!("deferred\u{1f}{p}\n")),
    );
    lines.sort();
    PlanFingerprint {
        version: PLAN_FINGERPRINT_VERSION,
        digest: sha256_hex(lines.concat().as_bytes()),
    }
}
fn simulation_result(
    mut base: UpdateCheckResult,
    raw: &str,
) -> Result<UpdateCheckResult, MaintenanceFailure> {
    if raw
        .lines()
        .any(|l| l == "PIHUB_UPDATE_NEW=0" || l == "PIHUB_UPDATE_REMOVE=0")
        == false
    {
        return Err(MaintenanceFailure::PackageOperationFailed);
    }
    if raw
        .lines()
        .any(|l| l.starts_with("PIHUB_UPDATE_NEW=") && l != "PIHUB_UPDATE_NEW=0")
        || raw
            .lines()
            .any(|l| l.starts_with("PIHUB_UPDATE_REMOVE=") && l != "PIHUB_UPDATE_REMOVE=0")
    {
        return Err(MaintenanceFailure::PackageOperationFailed);
    }
    base.updates = package(raw).map_err(|_| MaintenanceFailure::PackageOperationFailed)?;
    base.kept_back_packages =
        kept_back(raw).map_err(|_| MaintenanceFailure::PackageOperationFailed)?;
    if base.updates.truncated || base.kept_back_packages.truncated {
        return Err(MaintenanceFailure::PlanChanged);
    }
    base.status = UpdateStatus::UpdatesAvailable;
    base.package_metadata.status = MetadataStatus::Fresh;
    base.checked_at = Some(Utc::now().to_rfc3339());
    Ok(base)
}
fn capability(executor: &dyn RemoteExecutor, target: &SshTarget) -> Result<(), MaintenanceFailure> {
    let value = run(
        executor,
        target,
        RemoteOperation::MaintenanceCapability.command().unwrap(),
        SHORT,
        "device_updates.apply.preflight",
        OUTPUT,
    )
    .map_err(maintenance_failure)?;
    ["APT_GET", "DPKG", "SYSTEMD_RUN", "SYSTEMCTL"]
        .iter()
        .all(|key| value.stdout.contains(&format!("PIHUB_M16_{key}=1")))
        .then_some(())
        .ok_or(MaintenanceFailure::Unsupported)
}
fn audit(executor: &dyn RemoteExecutor, target: &SshTarget) -> Result<(), MaintenanceFailure> {
    let value = run(
        executor,
        target,
        RemoteOperation::MaintenanceDpkgAudit.command().unwrap(),
        SHORT,
        "device_updates.apply.preflight",
        OUTPUT,
    )
    .map_err(maintenance_failure)?;
    value
        .stdout
        .trim()
        .is_empty()
        .then_some(())
        .ok_or(MaintenanceFailure::PackageStateInconsistent)
}
fn simulate(
    executor: &dyn RemoteExecutor,
    target: &SshTarget,
    base: UpdateCheckResult,
) -> Result<UpdateCheckResult, MaintenanceFailure> {
    let value = run(
        executor,
        target,
        RemoteOperation::MaintenanceSimulation.command().unwrap(),
        SHORT,
        "device_updates.apply.plan_verify",
        OUTPUT,
    )
    .map_err(maintenance_failure)?;
    simulation_result(base, &value.stdout)
}

pub(crate) fn prepare_with(
    device: &Device,
    executor: &dyn RemoteExecutor,
    repo: &dyn SnapshotRepository,
) -> Result<PreparedDeviceUpdate, MaintenanceFailure> {
    let current = repo
        .get_update_result(&device.id)
        .ok_or(MaintenanceFailure::Unsupported)?;
    if !complete_preview(&current) {
        return Err(MaintenanceFailure::Unsupported);
    }
    let target = target(device);
    capability(executor, &target)?;
    audit(executor, &target)?;
    run(
        executor,
        &target,
        RemoteOperation::MaintenanceMetadataRefresh
            .command()
            .unwrap(),
        REFRESH,
        "device_updates.apply.metadata_refresh",
        OUTPUT,
    )
    .map_err(|e| match maintenance_failure(e) {
        MaintenanceFailure::PackageManagerBusy => MaintenanceFailure::PackageManagerBusy,
        _ => MaintenanceFailure::MetadataRefreshFailed,
    })?;
    let fresh = simulate(executor, &target, current)?;
    let mut operation = MaintenanceOperation::requested(device.id.clone());
    operation.reviewed_plan = Some(fingerprint(&fresh));
    operation.package_count = fresh.updates.total_count;
    repo.upsert_update_result(&fresh)
        .map_err(|_| MaintenanceFailure::VerificationFailed)?;
    repo.upsert_maintenance_operation(&operation)
        .map_err(|_| MaintenanceFailure::VerificationFailed)?;
    Ok(PreparedDeviceUpdate {
        operation,
        plan: fresh,
    })
}

#[tauri::command]
pub async fn prepare_device_update(
    app: AppHandle,
    device_id: String,
) -> Result<PreparedDeviceUpdate, ApplicationError> {
    let coordinator = app.state::<DeviceMaintenanceCoordinator>();
    let _claim = coordinator
        .try_claim(&device_id, MaintenanceOperationKind::UpdateApply)
        .ok_or_else(|| {
            app_error(
                "ActionConflict",
                "another maintenance operation is already in progress for this device",
            )
        })?;
    let dir = dir(&app)?;
    let device = JsonDeviceRepository::new(&dir)
        .get(&device_id)
        .ok_or_else(|| app_error("NotFoundError", "device was not found"))?;
    prepare_with(
        &device,
        &OpenSshExecutor::default(),
        &JsonSnapshotRepository::new(&dir),
    )
    .map_err(|failure| {
        app_error(
            &format!("{failure:?}"),
            "update preparation could not establish a safe reviewed plan",
        )
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnitState {
    Missing,
    Running,
    Success,
    Failed,
}
fn unit_state(raw: &str) -> Option<UnitState> {
    let fields: BTreeMap<_, _> = raw
        .lines()
        .filter_map(|line| line.split_once('='))
        .collect();
    if fields.get("LoadState") == Some(&"not-found") {
        return Some(UnitState::Missing);
    }
    let active = *fields.get("ActiveState")?;
    let sub = *fields.get("SubState")?;
    if active == "active" && sub != "exited" || active == "activating" {
        return Some(UnitState::Running);
    }
    if active == "active"
        && sub == "exited"
        && fields.get("Result") == Some(&"success")
        && fields.get("ExecMainCode") == Some(&"exited")
        && fields.get("ExecMainStatus") == Some(&"0")
    {
        return Some(UnitState::Success);
    }
    Some(UnitState::Failed)
}
pub(crate) fn reconcile_with(
    operation: &mut MaintenanceOperation,
    executor: &dyn RemoteExecutor,
    target: &SshTarget,
) -> Result<UnitState, MaintenanceFailure> {
    let command =
        maintenance_status_command(&operation.transient_unit_id).expect("validated backend unit");
    let result = run(
        executor,
        target,
        &command,
        SHORT,
        "device_updates.apply.status",
        STATUS_OUTPUT,
    )
    .map_err(maintenance_failure)?;
    let state = unit_state(&result.stdout).ok_or(MaintenanceFailure::OutcomeUncertain)?;
    match state {
        UnitState::Running => operation.transition(MaintenanceOperationState::Installing),
        UnitState::Missing => operation.transition(MaintenanceOperationState::OutcomeUncertain),
        UnitState::Success | UnitState::Failed => {
            operation.transition(MaintenanceOperationState::Verifying)
        }
    }
    Ok(state)
}

fn approved_entries_are_gone(before: &UpdateCheckResult, after: &UpdateCheckResult) -> bool {
    before.updates.packages.iter().all(|entry| {
        !after.updates.packages.iter().any(|current| {
            current.name == entry.name
                && current.architecture == entry.architecture
                && current.candidate_version == entry.candidate_version
        })
    })
}
pub(crate) fn reconcile_persisted_with(
    device: &Device,
    executor: &dyn RemoteExecutor,
    repo: &dyn SnapshotRepository,
) -> Result<MaintenanceOperation, MaintenanceFailure> {
    let mut operation = repo
        .get_maintenance_operation(&device.id)
        .ok_or(MaintenanceFailure::OutcomeUncertain)?;
    if operation.dispatch_state == MaintenanceDispatchState::NotAttempted
        || operation.state.is_terminal()
    {
        return Ok(operation);
    }
    if operation
        .observation_deadline
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .is_some_and(|deadline| deadline.with_timezone(&Utc) <= Utc::now())
    {
        operation.failure = Some(MaintenanceFailure::StillRunning);
        operation.transition(MaintenanceOperationState::StillRunning);
        repo.upsert_maintenance_operation(&operation)
            .map_err(|_| MaintenanceFailure::VerificationFailed)?;
        return Ok(operation);
    }
    if reconcile_with(&mut operation, executor, &target(device)).is_err() {
        operation.failure = Some(MaintenanceFailure::OutcomeUncertain);
        operation.transition(MaintenanceOperationState::OutcomeUncertain);
    }
    repo.upsert_maintenance_operation(&operation)
        .map_err(|_| MaintenanceFailure::VerificationFailed)?;
    Ok(operation)
}
#[tauri::command]
pub async fn reconcile_device_update(
    app: AppHandle,
    device_id: String,
) -> Result<MaintenanceOperation, ApplicationError> {
    let coordinator = app.state::<DeviceMaintenanceCoordinator>();
    let _claim = coordinator.try_claim_recovery(&device_id).ok_or_else(|| {
        app_error(
            "ActionConflict",
            "another maintenance operation is already in progress for this device",
        )
    })?;
    let directory = dir(&app)?;
    let device = JsonDeviceRepository::new(&directory)
        .get(&device_id)
        .ok_or_else(|| app_error("NotFoundError", "device was not found"))?;
    reconcile_persisted_with(
        &device,
        &OpenSshExecutor::default(),
        &JsonSnapshotRepository::new(&directory),
    )
    .map_err(|failure| {
        app_error(
            &format!("{failure:?}"),
            "the persisted update operation could not be reconciled",
        )
    })
}
pub(crate) fn apply_with(
    device: &Device,
    executor: &dyn RemoteExecutor,
    repo: &dyn SnapshotRepository,
    disruption: &JsonAdministrationRepository,
    activity: &JsonActivityRepository,
) -> Result<MaintenanceOperation, MaintenanceFailure> {
    let mut operation = repo
        .get_maintenance_operation(&device.id)
        .ok_or(MaintenanceFailure::PlanChanged)?;
    if operation.dispatch_state != MaintenanceDispatchState::NotAttempted {
        return Ok(operation);
    }
    let before = repo
        .get_update_result(&device.id)
        .ok_or(MaintenanceFailure::PlanChanged)?;
    let target = target(device);
    let persist_pre_dispatch_failure = |operation: &mut MaintenanceOperation,
                                        stage: PreDispatchFailureStage,
                                        failure: MaintenanceFailure|
     -> Result<MaintenanceOperation, MaintenanceFailure> {
        operation.failure = Some(failure);
        operation.pre_dispatch_failure_stage = Some(stage);
        operation.transition(MaintenanceOperationState::PreDispatchFailed);
        repo.upsert_maintenance_operation(operation)
            .map_err(|_| MaintenanceFailure::VerificationFailed)?;
        Ok(operation.clone())
    };
    if let Err(failure) = capability(executor, &target) {
        return persist_pre_dispatch_failure(
            &mut operation,
            PreDispatchFailureStage::Capability,
            failure,
        );
    }
    if let Err(failure) = audit(executor, &target) {
        return persist_pre_dispatch_failure(
            &mut operation,
            PreDispatchFailureStage::DpkgAudit,
            failure,
        );
    }
    let fresh = match simulate(executor, &target, before.clone()) {
        Ok(fresh) => fresh,
        Err(MaintenanceFailure::PlanChanged) => {
            operation.failure = Some(MaintenanceFailure::PlanChanged);
            operation.transition(MaintenanceOperationState::PlanChanged);
            let _ = repo.upsert_update_result(&before);
            let _ = repo.upsert_maintenance_operation(&operation);
            return Ok(operation);
        }
        Err(failure) => {
            return persist_pre_dispatch_failure(
                &mut operation,
                PreDispatchFailureStage::PlanVerification,
                failure,
            );
        }
    };
    if operation.reviewed_plan.as_ref() != Some(&fingerprint(&fresh)) {
        operation.failure = Some(MaintenanceFailure::PlanChanged);
        operation.transition(MaintenanceOperationState::PlanChanged);
        let _ = repo.upsert_update_result(&fresh);
        let _ = repo.upsert_maintenance_operation(&operation);
        return Ok(operation);
    }
    operation.observation_deadline = Some((Utc::now() + ChronoDuration::minutes(60)).to_rfc3339());
    operation.transition(MaintenanceOperationState::Dispatching);
    repo.upsert_maintenance_operation(&operation)
        .map_err(|_| MaintenanceFailure::VerificationFailed)?;
    let dispatch = run(
        executor,
        &target,
        &maintenance_dispatch_command(&operation.transient_unit_id).expect("validated unit"),
        SHORT,
        "device_updates.apply.dispatch",
        OUTPUT,
    );
    operation.dispatch_state = if dispatch.is_ok() {
        MaintenanceDispatchState::Accepted
    } else {
        MaintenanceDispatchState::Uncertain
    };
    if dispatch.is_ok() {
        operation.transition(MaintenanceOperationState::Installing);
    }
    if dispatch.is_err() {
        operation.failure = Some(MaintenanceFailure::OutcomeUncertain);
    }
    let state = reconcile_with(&mut operation, executor, &target);
    match state {
        Ok(UnitState::Running) => {
            if let Some(marker) =
                crate::domain::administration::ExpectedDisruption::for_update(&operation)
            {
                let _ = disruption.save(marker);
            }
            let mut initiated = operation.clone();
            initiated.state = MaintenanceOperationState::Requested;
            if let Some(event) = initiated.activity_event() {
                let _ = activity.append(event);
            }
            let _ = repo.upsert_maintenance_operation(&operation);
            return Ok(operation);
        }
        Ok(UnitState::Missing) | Err(_) => {
            operation.failure = Some(MaintenanceFailure::OutcomeUncertain);
            operation.transition(MaintenanceOperationState::OutcomeUncertain);
            let _ = repo.upsert_maintenance_operation(&operation);
            return Ok(operation);
        }
        Ok(UnitState::Failed) => {
            operation.failure = Some(MaintenanceFailure::PackageOperationFailed);
            operation.transition(MaintenanceOperationState::Failed);
            let _ = repo.upsert_maintenance_operation(&operation);
            let _ = run(
                executor,
                &target,
                &maintenance_journal_command(&operation.transient_unit_id).unwrap(),
                SHORT,
                "device_updates.apply.verify",
                STATUS_OUTPUT,
            );
            let _ = disruption.clear_if_operation(&device.id, &operation.id);
            let _ = run(
                executor,
                &target,
                &maintenance_cleanup_command(&operation.transient_unit_id).unwrap(),
                SHORT,
                "device_updates.apply.verify",
                STATUS_OUTPUT,
            );
            return Ok(operation);
        }
        Ok(UnitState::Success) => {}
    }
    audit(executor, &target)?;
    let after = check_with(device, executor, repo, Utc::now())
        .map_err(|_| MaintenanceFailure::VerificationFailed)?;
    if !approved_entries_are_gone(&before, &after) {
        operation.failure = Some(MaintenanceFailure::VerificationFailed);
        operation.transition(MaintenanceOperationState::Failed);
    } else {
        operation.reboot_required = Some(after.reboot == RebootState::Required);
        operation.transition(if after.reboot == RebootState::Required {
            MaintenanceOperationState::CompletedRebootRequired
        } else {
            MaintenanceOperationState::Completed
        });
    }
    repo.upsert_maintenance_operation(&operation)
        .map_err(|_| MaintenanceFailure::VerificationFailed)?;
    let _ = disruption.clear_if_operation(&device.id, &operation.id);
    if let Some(event) = operation.activity_event() {
        let _ = activity.append(event);
    }
    let _ = run(
        executor,
        &target,
        &maintenance_cleanup_command(&operation.transient_unit_id).unwrap(),
        SHORT,
        "device_updates.apply.verify",
        STATUS_OUTPUT,
    );
    Ok(operation)
}

#[tauri::command]
pub async fn apply_prepared_device_update(
    app: AppHandle,
    device_id: String,
) -> Result<MaintenanceOperation, ApplicationError> {
    let coordinator = app.state::<DeviceMaintenanceCoordinator>();
    let _claim = coordinator
        .try_claim(&device_id, MaintenanceOperationKind::UpdateApply)
        .ok_or_else(|| {
            app_error(
                "ActionConflict",
                "another maintenance operation is already in progress for this device",
            )
        })?;
    let directory = dir(&app)?;
    let device = JsonDeviceRepository::new(&directory)
        .get(&device_id)
        .ok_or_else(|| app_error("NotFoundError", "device was not found"))?;
    apply_with(
        &device,
        &OpenSshExecutor::default(),
        &JsonSnapshotRepository::new(&directory),
        &JsonAdministrationRepository::new(&directory),
        &JsonActivityRepository::new(&directory),
    )
    .map_err(|failure| {
        app_error(
            &format!("{failure:?}"),
            "the update could not be applied safely",
        )
    })
}

// Small self-contained SHA-256 implementation keeps the consent digest deterministic without introducing a new remote or frontend dependency.
fn sha256_hex(input: &[u8]) -> String {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut data = input.to_vec();
    let bits = (data.len() as u64) * 8;
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0)
    }
    data.extend_from_slice(&bits.to_be_bytes());
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    for c in data.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(c[i * 4..i * 4 + 4].try_into().unwrap())
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1)
        }
        let (mut a, mut b, mut cc, mut d, mut e, mut f, mut g, mut x) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = x
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & cc) ^ (b & cc);
            let t2 = s0.wrapping_add(maj);
            x = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = cc;
            cc = b;
            b = a;
            a = t1.wrapping_add(t2)
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(cc);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(x)
    }
    h.iter().map(|v| format!("{v:08x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            device::DeviceType,
            update_intelligence::{UpdatePackage, UpdatePackages},
        },
        storage::snapshot_repository::JsonSnapshotRepository,
    };
    use std::{collections::VecDeque, sync::Mutex};
    struct Script(Mutex<VecDeque<Result<RemoteExecutionResult, SshError>>>);
    impl RemoteExecutor for Script {
        fn execute(
            &self,
            _: &SshTarget,
            _: &str,
            _: Duration,
        ) -> Result<RemoteExecutionResult, SshError> {
            self.0.lock().unwrap().pop_front().unwrap()
        }
    }
    fn ok(stdout: &str) -> Result<RemoteExecutionResult, SshError> {
        Ok(RemoteExecutionResult {
            exit_code: Some(0),
            stdout: stdout.into(),
            stderr: String::new(),
            duration_ms: 1,
            timed_out: false,
        })
    }
    fn device() -> Device {
        Device {
            id: "d".into(),
            name: "D".into(),
            host: "fixture".into(),
            ssh_port: 22,
            ssh_username: "u".into(),
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
    fn preview() -> UpdateCheckResult {
        let mut r = UpdateCheckResult::empty("d".into());
        r.support.status = SupportStatus::Supported;
        r.status = UpdateStatus::UpdatesAvailable;
        r.updates = UpdatePackages {
            total_count: Some(1),
            truncated: false,
            packages: vec![UpdatePackage {
                name: "bash".into(),
                architecture: Some("amd64".into()),
                installed_version: "1".into(),
                candidate_version: "2".into(),
                held: None,
            }],
        };
        r.kept_back_packages.total_count = Some(0);
        r
    }
    fn capability() -> &'static str {
        "PIHUB_M16_APT_GET=1\nPIHUB_M16_DPKG=1\nPIHUB_M16_SYSTEMD_RUN=1\nPIHUB_M16_SYSTEMCTL=1\n"
    }
    fn sim() -> &'static str {
        "PIHUB_UPDATE_PACKAGE=Inst bash:amd64 [1] (2)\nPIHUB_UPDATE_NEW=0\nPIHUB_UPDATE_REMOVE=0\nPIHUB_UPDATE_KEPT_BACK_COUNT=0\nPIHUB_UPDATE_PACKAGES_DONE=1\n"
    }
    #[test]
    fn prepare_and_apply_success_are_two_distinct_fake_phases() {
        let dir = tempfile::tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        repo.upsert_update_result(&preview()).unwrap();
        let prepare = Script(Mutex::new(VecDeque::from(vec![
            ok(capability()),
            ok(""),
            ok(""),
            ok(sim()),
        ])));
        let prepared = prepare_with(&device(), &prepare, &repo).unwrap();
        assert!(prepared
            .operation
            .requires_fresh_confirmation_after_restart());
        let apply=Script(Mutex::new(VecDeque::from(vec![ok(capability()),ok(""),ok(sim()),ok(""),ok("LoadState=loaded\nActiveState=active\nSubState=exited\nResult=success\nExecMainCode=exited\nExecMainStatus=0\nExecMainStartTimestampMonotonic=1\nExecMainExitTimestampMonotonic=2\n"),ok(""),ok("PIHUB_UPDATE_OS_ID=debian\nPIHUB_UPDATE_OS_LIKE=debian\nPIHUB_UPDATE_APT_GET=1\nPIHUB_UPDATE_DPKG_QUERY=1\nPIHUB_UPDATE_DPKG=1\n"),ok("PIHUB_UPDATE_KEPT_BACK_COUNT=0\nPIHUB_UPDATE_PACKAGES_DONE=1\n"),ok("PIHUB_UPDATE_HOLDS_DONE=1\n"),ok("PIHUB_UPDATE_METADATA_MTIME=1\n"),ok("PIHUB_UPDATE_REBOOT=unknown\n"),ok("")] )));
        let operation = apply_with(
            &device(),
            &apply,
            &repo,
            &JsonAdministrationRepository::new(dir.path()),
            &JsonActivityRepository::new(dir.path()),
        )
        .unwrap();
        assert_eq!(operation.state, MaintenanceOperationState::Completed);
    }
    #[test]
    fn parser_distinguishes_running_active_exited_success_and_failure() {
        assert_eq!(
            unit_state("LoadState=loaded\nActiveState=active\nSubState=running\n"),
            Some(UnitState::Running)
        );
        assert_eq!(unit_state("LoadState=loaded\nActiveState=active\nSubState=exited\nResult=success\nExecMainCode=exited\nExecMainStatus=0\n"),Some(UnitState::Success));
        assert_eq!(
            unit_state("LoadState=not-found\n"),
            Some(UnitState::Missing)
        );
        assert_eq!(
            unit_state("LoadState=loaded\nActiveState=failed\nSubState=failed\n"),
            Some(UnitState::Failed)
        );
    }
    #[test]
    fn fingerprint_is_sha256_and_includes_architecture_and_deferred_boundary() {
        let one = fingerprint(&preview());
        let mut changed = preview();
        changed.updates.packages[0].architecture = None;
        assert_ne!(one, fingerprint(&changed));
        changed = preview();
        changed.kept_back_packages.packages.push("linux".into());
        assert_ne!(one, fingerprint(&changed));
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
    #[test]
    fn prepare_fails_before_dispatch_for_audit_refresh_and_unsafe_plan() {
        let dir = tempfile::tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        repo.upsert_update_result(&preview()).unwrap();
        for replies in [vec![ok(capability()),ok("broken")],vec![ok(capability()),ok(""),Err(SshError::RemoteCommandError{exit_code:Some(100),stderr:"Could not get lock".into()})],vec![ok(capability()),ok(""),ok(""),ok("PIHUB_UPDATE_NEW=1\nPIHUB_UPDATE_REMOVE=0\nPIHUB_UPDATE_KEPT_BACK_COUNT=0\nPIHUB_UPDATE_PACKAGES_DONE=1\n")]]{assert!(prepare_with(&device(),&Script(Mutex::new(replies.into())),&repo).is_err());}
    }
    #[test]
    fn recovery_is_one_shot_never_redispatches_and_expiry_is_still_running() {
        let dir = tempfile::tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let mut operation = MaintenanceOperation::requested("d".into());
        operation.dispatch_state = MaintenanceDispatchState::Accepted;
        operation.state = MaintenanceOperationState::Installing;
        operation.observation_deadline = Some("2000-01-01T00:00:00Z".into());
        repo.upsert_maintenance_operation(&operation).unwrap();
        let recovered =
            reconcile_persisted_with(&device(), &Script(Mutex::new(VecDeque::new())), &repo)
                .unwrap();
        assert_eq!(recovered.state, MaintenanceOperationState::StillRunning);
        let mut operation = MaintenanceOperation::requested("d".into());
        operation.dispatch_state = MaintenanceDispatchState::Uncertain;
        operation.state = MaintenanceOperationState::Dispatching;
        operation.observation_deadline =
            Some((Utc::now() + ChronoDuration::minutes(60)).to_rfc3339());
        repo.upsert_maintenance_operation(&operation).unwrap();
        let missing = reconcile_persisted_with(
            &device(),
            &Script(Mutex::new(VecDeque::from(vec![ok(
                "LoadState=not-found\n",
            )]))),
            &repo,
        )
        .unwrap();
        assert_eq!(missing.state, MaintenanceOperationState::OutcomeUncertain);
    }
    #[test]
    fn apply_rechecks_consent_and_never_dispatches_a_changed_plan() {
        let dir = tempfile::tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        repo.upsert_update_result(&preview()).unwrap();
        prepare_with(
            &device(),
            &Script(Mutex::new(VecDeque::from(vec![
                ok(capability()),
                ok(""),
                ok(""),
                ok(sim()),
            ]))),
            &repo,
        )
        .unwrap();
        let changed="PIHUB_UPDATE_PACKAGE=Inst bash:amd64 [1] (3)\nPIHUB_UPDATE_NEW=0\nPIHUB_UPDATE_REMOVE=0\nPIHUB_UPDATE_KEPT_BACK_COUNT=0\nPIHUB_UPDATE_PACKAGES_DONE=1\n";
        let result = apply_with(
            &device(),
            &Script(Mutex::new(VecDeque::from(vec![
                ok(capability()),
                ok(""),
                ok(changed),
            ]))),
            &repo,
            &JsonAdministrationRepository::new(dir.path()),
            &JsonActivityRepository::new(dir.path()),
        )
        .unwrap();
        assert_eq!(result.state, MaintenanceOperationState::PlanChanged);
        assert_eq!(result.failure, Some(MaintenanceFailure::PlanChanged));
    }
    #[test]
    fn pre_dispatch_transport_failures_are_persisted_as_definitely_not_started() {
        let cases = [
            (
                PreDispatchFailureStage::Capability,
                vec![Err(SshError::ConnectionTimeout)],
            ),
            (
                PreDispatchFailureStage::DpkgAudit,
                vec![ok(capability()), Err(SshError::ConnectionTimeout)],
            ),
            (
                PreDispatchFailureStage::PlanVerification,
                vec![ok(capability()), ok(""), Err(SshError::ConnectionTimeout)],
            ),
        ];
        for (stage, replies) in cases {
            let dir = tempfile::tempdir().unwrap();
            let repo = JsonSnapshotRepository::new(dir.path());
            repo.upsert_update_result(&preview()).unwrap();
            prepare_with(
                &device(),
                &Script(Mutex::new(VecDeque::from(vec![
                    ok(capability()),
                    ok(""),
                    ok(""),
                    ok(sim()),
                ]))),
                &repo,
            )
            .unwrap();
            let disruptions = JsonAdministrationRepository::new(dir.path());
            let activity = JsonActivityRepository::new(dir.path());
            let operation = apply_with(
                &device(),
                &Script(Mutex::new(VecDeque::from(replies))),
                &repo,
                &disruptions,
                &activity,
            )
            .unwrap();
            assert_eq!(
                operation.state,
                MaintenanceOperationState::PreDispatchFailed
            );
            assert_eq!(
                operation.dispatch_state,
                MaintenanceDispatchState::NotAttempted
            );
            assert_eq!(
                operation.failure,
                Some(MaintenanceFailure::TransportUnavailableDuringObservation)
            );
            assert_eq!(operation.pre_dispatch_failure_stage, Some(stage));
            assert!(operation.started_at.is_none());
            assert!(operation.completed_at.is_some());
            assert!(!operation.requires_recovery());
            assert!(operation.requires_fresh_confirmation_after_restart());
            assert!(disruptions.get_valid("d").is_none());
            assert!(activity.load_for_device("d").is_empty());

            let after_restart = JsonSnapshotRepository::new(dir.path())
                .get_maintenance_operation("d")
                .unwrap();
            assert_eq!(after_restart, operation);
        }
    }
    #[test]
    fn uncertain_dispatch_observes_known_unit_without_redispatch_and_failed_diagnostics_are_discarded(
    ) {
        let dir = tempfile::tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        repo.upsert_update_result(&preview()).unwrap();
        prepare_with(
            &device(),
            &Script(Mutex::new(VecDeque::from(vec![
                ok(capability()),
                ok(""),
                ok(""),
                ok(sim()),
            ]))),
            &repo,
        )
        .unwrap();
        let running = apply_with(
            &device(),
            &Script(Mutex::new(VecDeque::from(vec![
                ok(capability()),
                ok(""),
                ok(sim()),
                Err(SshError::ConnectionTimeout),
                ok("LoadState=loaded\nActiveState=active\nSubState=running\n"),
            ]))),
            &repo,
            &JsonAdministrationRepository::new(dir.path()),
            &JsonActivityRepository::new(dir.path()),
        )
        .unwrap();
        assert_eq!(running.dispatch_state, MaintenanceDispatchState::Uncertain);
        assert_eq!(running.state, MaintenanceOperationState::Installing);
        let persisted = std::fs::read_to_string(dir.path().join("state.json")).unwrap();
        assert!(!persisted.contains("stdout") && !persisted.contains("journal"));
    }
}
