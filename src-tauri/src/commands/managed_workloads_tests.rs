use super::*;
use crate::{
    domain::{device::{Device, DeviceService, DeviceType}, managed_workload::{ManagedWorkload, ManagedWorkloadFile}, managed_workload_operation::{ManagedWorkloadDispatchState, ManagedWorkloadOperationState}},
    infrastructure::ssh::{RemoteExecutionResult, SshError, SshTarget},
    storage::{device_repository::{DeviceRepository, JsonDeviceRepository}, managed_workload_operation_repository::JsonManagedWorkloadOperationRepository, managed_workload_repository::{JsonManagedWorkloadRepository, ManagedWorkloadRepository}},
};
use std::{collections::VecDeque, sync::Mutex, time::Duration};

struct Script { replies: Mutex<VecDeque<Result<RemoteExecutionResult, SshError>>>, commands: Mutex<Vec<String>> }
impl RemoteExecutor for Script {
    fn execute(&self, _: &SshTarget, command: &str, _: Duration) -> Result<RemoteExecutionResult, SshError> {
        self.commands.lock().unwrap().push(command.into());
        self.replies.lock().unwrap().pop_front().expect("scripted reply")
    }
}
fn reply(stdout: &str) -> Result<RemoteExecutionResult, SshError> { Ok(RemoteExecutionResult { exit_code: Some(0), stdout: stdout.into(), stderr: "FAKE_STDERR_SECRET".into(), duration_ms: 1, timed_out: false }) }
fn trust() -> Result<RemoteExecutionResult, SshError> { reply(&format!("PIHUB_M17_TRUST=trusted\nPIHUB_M17_ACTION_DIGEST={}\n", "a".repeat(64))) }
fn prepare(target: &str) -> Result<RemoteExecutionResult, SshError> { reply(&format!(r#"{{"protocolVersion":1,"status":"ready","currentRevision":"{}","targetRevision":"{target}","changeCount":2}}"#, "b".repeat(40))) }
fn setup() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let device = Device { id: "pi5".into(), name: "Pi".into(), host: "host.example".into(), ssh_port: 22, ssh_username: "deploy".into(), description: None, device_type: DeviceType::RaspberryPi, monitoring_enabled: true, refresh_interval_seconds: None, notify_on_device_offline: true, notify_on_container_failure: true, notify_on_container_unhealthy: true, services: vec![DeviceService { id: "health".into(), name: "Health".into(), url: "https://secret.example.test/health".into(), icon: None, description: None, enabled: true, container_name: None }], created_at: "x".into(), updated_at: "x".into() };
    let workload = ManagedWorkload { id: "finance".into(), device_id: "pi5".into(), name: "Finance".into(), action_id: "personal-finance".into(), compose_project: Some("finance".into()), required_compose_services: vec!["app".into()], required_service_health_ids: vec!["health".into()], enabled: true, config_revision: 1 };
    JsonDeviceRepository::new(dir.path()).save_all(&[device]).unwrap();
    JsonManagedWorkloadRepository::new(dir.path()).save(&ManagedWorkloadFile { schema_version: 1, workloads: vec![workload] }).unwrap();
    dir
}
fn prepared(dir: &std::path::Path) -> PreparedManagedWorkloadDto {
    let script = Script { replies: Mutex::new(VecDeque::from(vec![trust(), prepare(&"c".repeat(40))])), commands: Mutex::new(vec![]) };
    prepare_managed_workload_with(dir, ManagedWorkloadPrepareRequest { workload_id: "finance".into() }, &script, &DeviceMaintenanceCoordinator::new()).unwrap()
}

#[test]
fn prepare_delegates_to_existing_typed_service_and_returns_bounded_review_data() {
    let dir = setup();
    let script = Script { replies: Mutex::new(VecDeque::from(vec![trust(), prepare(&"c".repeat(40))])), commands: Mutex::new(vec![]) };
    let response = prepare_managed_workload_with(dir.path(), ManagedWorkloadPrepareRequest { workload_id: "finance".into() }, &script, &DeviceMaintenanceCoordinator::new()).unwrap();
    assert_eq!(response.operation.state, ManagedWorkloadDeploymentStateDto::Prepared);
    assert_eq!(response.review_target_revision, "c".repeat(40));
    assert_eq!(response.change_count, 2);
    let commands = script.commands.lock().unwrap();
    assert_eq!(commands.len(), 2);
    assert!(commands[1].ends_with("personal-finance' prepare"));
    assert!(!commands.iter().any(|command| command.contains(" apply") || command.contains("systemd-run")));
    let raw = serde_json::to_string(&response).unwrap();
    for forbidden in ["trustedActionDigest", "deploymentFingerprint", "transientUnitId", "/usr/local", "FAKE_STDERR_SECRET", "secret.example"] { assert!(!raw.contains(forbidden), "serialized {forbidden}"); }
}

#[test]
fn continuation_cannot_replace_reviewed_target_and_plan_changed_is_distinct() {
    let dir = setup();
    let prepared = prepared(dir.path());
    let script = Script { replies: Mutex::new(VecDeque::from(vec![trust(), prepare(&"d".repeat(40))])), commands: Mutex::new(vec![]) };
    let response = continue_managed_workload_with(dir.path(), prepared.operation.operation, &script, &DeviceMaintenanceCoordinator::new()).unwrap();
    assert_eq!(response.outcome, ManagedWorkloadContinuationDto::PlanChanged);
    assert_eq!(response.operation.state, ManagedWorkloadDeploymentStateDto::PlanChanged);
    assert!(!script.commands.lock().unwrap().iter().any(|command| command.contains("systemd-run") || command.contains(" apply")));
}

#[test]
fn uncertain_dispatch_is_preserved_without_a_generic_apply_surface() {
    let dir = setup();
    let prepared = prepared(dir.path());
    let script = Script { replies: Mutex::new(VecDeque::from(vec![trust(), prepare(&"c".repeat(40)), Err(SshError::ConnectionTimeout)])), commands: Mutex::new(vec![]) };
    let response = continue_managed_workload_with(dir.path(), prepared.operation.operation, &script, &DeviceMaintenanceCoordinator::new()).unwrap();
    assert_eq!(response.outcome, ManagedWorkloadContinuationDto::DispatchStarted);
    assert_eq!(response.operation.state, ManagedWorkloadDeploymentStateDto::DispatchUncertain);
    assert_eq!(response.operation.problem, Some(ManagedWorkloadProblemDto::DispatchOutcomeUncertain));
    assert_eq!(script.commands.lock().unwrap().iter().filter(|command| command.contains("systemd-run")).count(), 1);
}

#[test]
fn status_uses_only_operation_identity_and_maps_terminal_dtos_without_execution_details() {
    let dir = setup();
    let prepared = prepared(dir.path());
    let reference = prepared.operation.operation;
    let repo = JsonManagedWorkloadOperationRepository::new(dir.path());
    let mut operation = repo.get("pi5", "finance").unwrap();
    operation.state = ManagedWorkloadOperationState::Completed;
    operation.dispatch_state = ManagedWorkloadDispatchState::Accepted;
    repo.save(&operation).unwrap();
    let script = Script { replies: Mutex::new(VecDeque::new()), commands: Mutex::new(vec![]) };
    assert_eq!(reconcile_managed_workload_with(dir.path(), ManagedWorkloadStatusRequest { workload_id: reference.workload_id.clone() }, &script).unwrap().state, ManagedWorkloadDeploymentStateDto::Completed);
    operation.state = ManagedWorkloadOperationState::VerificationFailed;
    operation.failure = Some(ManagedWorkloadFailure::VerificationServiceFailed);
    repo.save(&operation).unwrap();
    let response = reconcile_managed_workload_with(dir.path(), ManagedWorkloadStatusRequest { workload_id: reference.workload_id }, &script).unwrap();
    assert_eq!(response.state, ManagedWorkloadDeploymentStateDto::VerificationFailed);
    assert_eq!(response.problem, Some(ManagedWorkloadProblemDto::VerificationFailed));
    assert!(script.commands.lock().unwrap().is_empty());
    let raw = serde_json::to_string(&response).unwrap();
    for forbidden in ["transientUnitId", "targetRevision", "trustedActionDigest", "deploymentFingerprint", "secret.example", "systemd-run"] { assert!(!raw.contains(forbidden), "serialized {forbidden}"); }
}

#[test]
fn request_dtos_and_error_boundary_have_no_generic_execution_fields() {
    let prepare = serde_json::to_value(ManagedWorkloadPrepareRequest { workload_id: "finance".into() }).unwrap();
    let reference = serde_json::to_value(ManagedWorkloadOperationRef { workload_id: "finance".into(), operation_id: "op-1".into() }).unwrap();
    let status = serde_json::to_value(ManagedWorkloadStatusRequest { workload_id: "finance".into() }).unwrap();
    assert_eq!(prepare.as_object().unwrap().keys().collect::<Vec<_>>(), vec!["workloadId"]);
    assert_eq!(reference.as_object().unwrap().keys().collect::<Vec<_>>(), vec!["operationId", "workloadId"]);
    assert_eq!(status.as_object().unwrap().keys().collect::<Vec<_>>(), vec!["workloadId"]);
    for value in [prepare, reference, status] { let raw = value.to_string(); for forbidden in ["command", "path", "argv", "revision", "unit", "docker", "systemd", "host", "port", "username"] { assert!(!raw.contains(forbidden), "request exposed {forbidden}"); } }
    assert_eq!(map_prepare_error(PrepareError::MaintenanceConflict).code, "MaintenanceConflict");
    assert_eq!(map_prepare_error(PrepareError::ActionUnavailable).code, "ActionUnavailable");
    assert_eq!(map_prepare_error(PrepareError::ActionTrustFailed).code, "ActionUntrusted");
}

#[test]
fn workload_cards_are_read_only_and_keep_active_deployments_ineligible() {
    let dir = setup();
    let cards = list_managed_workloads_with(dir.path(), "pi5");
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].name, "Finance");
    assert!(cards[0].eligible_to_prepare);
    let prepared = prepared(dir.path());
    let repo = JsonManagedWorkloadOperationRepository::new(dir.path());
    let mut operation = repo.get("pi5", "finance").unwrap();
    operation.state = ManagedWorkloadOperationState::Deploying;
    operation.started_at = Some("x".into());
    operation.dispatch_state = ManagedWorkloadDispatchState::Accepted;
    operation.observation_deadline = Some("2099-01-01T00:00:00Z".into());
    repo.save(&operation).unwrap();
    let card = list_managed_workloads_with(dir.path(), "pi5").pop().unwrap();
    assert!(!card.eligible_to_prepare);
    let raw = serde_json::to_string(&card).unwrap();
    for forbidden in ["targetRevision", "transientUnitId", "trustedActionDigest", "deploymentFingerprint", "command", "path", "secret.example"] { assert!(!raw.contains(forbidden), "serialized {forbidden}"); }
    assert!(raw.contains("observationDeadline"));
    assert_eq!(card.deployment.unwrap().operation.operation_id, prepared.operation.operation.operation_id);
}
