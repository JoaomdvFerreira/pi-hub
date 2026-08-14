use tauri::{AppHandle, Manager};

use crate::{
    application::managed_workload_prepare::{
        dispatch_prepared_with, finalize_workload_verification, prepare_detached_dispatch,
        prepare_with, reconcile_detached_workload_with, revalidate_dispatch_consent_with,
        verify_deployed_revision_with, verify_workload_runtime_with, DispatchConsent,
        FinalVerificationError, PrepareError, RuntimeVerificationError, VerificationError,
    },
    domain::managed_workload_operation::{
        ManagedWorkloadFailure, ManagedWorkloadOperation, ManagedWorkloadOperationState,
    },
    error::ApplicationError,
    infrastructure::ssh::{OpenSshExecutor, RemoteExecutor},
    monitoring::maintenance_coordinator::DeviceMaintenanceCoordinator,
    storage::{
        managed_workload_operation_repository::JsonManagedWorkloadOperationRepository,
        managed_workload_repository::{JsonManagedWorkloadRepository, ManagedWorkloadRepository},
    },
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedWorkloadPrepareRequest {
    pub workload_id: String,
}

/// Opaque persisted-operation identity. It intentionally carries no plan,
/// action, remote target, unit, or command material.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManagedWorkloadOperationRef {
    pub workload_id: String,
    pub operation_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedWorkloadStatusRequest {
    pub workload_id: String,
}

/// Bounded display data for the device-detail deployment section. This is a
/// read-only view of registered workloads, not a configuration or execution
/// surface.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManagedWorkloadCardDto {
    pub workload_id: String,
    pub name: String,
    pub enabled: bool,
    pub eligible_to_prepare: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment: Option<ManagedWorkloadDeploymentDto>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ManagedWorkloadDeploymentStateDto {
    Prepared,
    PlanChanged,
    PreDispatchFailed,
    DispatchPrepared,
    Dispatching,
    DispatchUncertain,
    Deploying,
    StillRunning,
    AwaitingVerification,
    RevisionVerified,
    WorkloadRuntimeVerified,
    Completed,
    VerificationFailed,
    DeploymentFailed,
}

#[derive(Debug, Clone, Copy, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ManagedWorkloadProblemDto {
    WorkloadUnavailable,
    ActionUntrusted,
    CommunicationFailed,
    OutputLimitExceeded,
    ProtocolInvalid,
    DeploymentFailed,
    VerificationFailed,
    RequiredServiceMissing,
    ObservationExpired,
    DispatchOutcomeUncertain,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManagedWorkloadDeploymentDto {
    pub operation: ManagedWorkloadOperationRef,
    pub state: ManagedWorkloadDeploymentStateDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<ManagedWorkloadProblemDto>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreparedManagedWorkloadDto {
    pub operation: ManagedWorkloadDeploymentDto,
    pub review_target_revision: String,
    pub change_count: u32,
}

#[derive(Debug, Clone, Copy, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ManagedWorkloadContinuationDto {
    DispatchStarted,
    PlanChanged,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManagedWorkloadContinueResponse {
    pub outcome: ManagedWorkloadContinuationDto,
    pub operation: ManagedWorkloadDeploymentDto,
}

fn api_error(code: &str, message: &str, remediation: &str, retryable: bool) -> ApplicationError {
    ApplicationError {
        code: code.into(),
        message: message.into(),
        remediation: Some(remediation.into()),
        retryable,
    }
}

fn map_prepare_error(error: PrepareError) -> ApplicationError {
    match error {
        PrepareError::MaintenanceConflict => api_error("MaintenanceConflict", "Another maintenance operation is active for this device.", "Wait for that operation to finish.", true),
        PrepareError::ActionUnavailable => api_error("ActionUnavailable", "The managed deployment action is unavailable.", "Restore the approved managed action, then prepare again.", false),
        PrepareError::ActionTrustFailed => api_error("ActionUntrusted", "The managed deployment action is no longer trusted.", "Resolve the action trust issue, then prepare again.", false),
        PrepareError::TimedOut => api_error("DeploymentTimedOut", "The managed workload request timed out.", "Check the device connection and try a fresh prepare.", true),
        PrepareError::OutputLimitExceeded => api_error("OutputLimitExceeded", "The managed workload response exceeded its safety limit.", "Inspect the managed action outside Pi-Hub, then prepare again.", false),
        PrepareError::TransportFailed | PrepareError::DeviceTargetUnavailable => api_error("CommunicationFailed", "Pi-Hub could not communicate with the configured device.", "Check the device connection and try again.", true),
        PrepareError::WorkloadNotFound | PrepareError::WorkloadDisabled => api_error("WorkloadUnavailable", "The configured managed workload is unavailable.", "Review the managed workload configuration.", false),
        PrepareError::PersistenceFailed => api_error("PersistenceFailed", "Pi-Hub could not save the managed workload operation.", "Check local application storage and try again.", true),
        _ => api_error("ManagedWorkloadPreparationFailed", "Pi-Hub could not prepare the managed workload safely.", "Prepare the workload again after resolving its configuration.", false),
    }
}

fn map_verification_error(error: VerificationError) -> ApplicationError {
    match error {
        VerificationError::TrustFailed => api_error("ActionUntrusted", "The managed deployment action is no longer trusted.", "Resolve the action trust issue and start a fresh deployment.", false),
        VerificationError::DeployedRevisionMismatch => api_error("VerificationFailed", "The deployed revision does not match the confirmed deployment.", "Review the deployment result and prepare a fresh deployment if needed.", false),
        VerificationError::TimedOut => api_error("DeploymentTimedOut", "Revision verification timed out.", "Check the device connection and request status again.", true),
        VerificationError::OutputLimitExceeded => api_error("OutputLimitExceeded", "Revision verification exceeded its safety limit.", "Inspect the managed action outside Pi-Hub.", false),
        VerificationError::TransportFailed | VerificationError::DeviceTargetUnavailable => api_error("CommunicationFailed", "Pi-Hub could not verify the deployment on the configured device.", "Check the device connection and request status again.", true),
        _ => api_error("VerificationFailed", "Managed workload revision verification failed.", "Review the persisted deployment status.", false),
    }
}

fn map_runtime_error(error: RuntimeVerificationError) -> ApplicationError {
    match error {
        RuntimeVerificationError::DockerUnavailable => api_error("VerificationFailed", "Docker runtime verification is unavailable.", "Restore Docker access and request status again.", true),
        RuntimeVerificationError::TimedOut => api_error("DeploymentTimedOut", "Docker runtime verification timed out.", "Check the device connection and request status again.", true),
        RuntimeVerificationError::OutputLimitExceeded => api_error("OutputLimitExceeded", "Docker runtime verification exceeded its safety limit.", "Inspect the managed workload outside Pi-Hub.", false),
        RuntimeVerificationError::TransportFailed | RuntimeVerificationError::DeviceTargetUnavailable => api_error("CommunicationFailed", "Pi-Hub could not inspect the managed workload runtime.", "Check the device connection and request status again.", true),
        _ => api_error("VerificationFailed", "Managed workload runtime verification failed.", "Review the persisted deployment status.", false),
    }
}

fn map_final_error(error: FinalVerificationError) -> ApplicationError {
    match error {
        FinalVerificationError::ServiceMissing => api_error("VerificationFailed", "A required Service Health target is unavailable.", "Restore the configured service check and start a fresh deployment.", false),
        FinalVerificationError::ServiceFailed => api_error("VerificationFailed", "Required Service Health checks did not pass in time.", "Resolve service health and start a fresh deployment if needed.", true),
        _ => api_error("VerificationFailed", "Managed workload final verification failed.", "Review the persisted deployment status.", false),
    }
}

fn dto_state(state: ManagedWorkloadOperationState) -> ManagedWorkloadDeploymentStateDto {
    match state {
        ManagedWorkloadOperationState::Prepared => ManagedWorkloadDeploymentStateDto::Prepared,
        ManagedWorkloadOperationState::PlanChanged => ManagedWorkloadDeploymentStateDto::PlanChanged,
        ManagedWorkloadOperationState::PreDispatchFailed => ManagedWorkloadDeploymentStateDto::PreDispatchFailed,
        ManagedWorkloadOperationState::DispatchPrepared => ManagedWorkloadDeploymentStateDto::DispatchPrepared,
        ManagedWorkloadOperationState::Dispatching => ManagedWorkloadDeploymentStateDto::Dispatching,
        ManagedWorkloadOperationState::DispatchUncertain => ManagedWorkloadDeploymentStateDto::DispatchUncertain,
        ManagedWorkloadOperationState::Deploying => ManagedWorkloadDeploymentStateDto::Deploying,
        ManagedWorkloadOperationState::StillRunning => ManagedWorkloadDeploymentStateDto::StillRunning,
        ManagedWorkloadOperationState::AwaitingVerification => ManagedWorkloadDeploymentStateDto::AwaitingVerification,
        ManagedWorkloadOperationState::RevisionVerified => ManagedWorkloadDeploymentStateDto::RevisionVerified,
        ManagedWorkloadOperationState::WorkloadRuntimeVerified => ManagedWorkloadDeploymentStateDto::WorkloadRuntimeVerified,
        ManagedWorkloadOperationState::Completed => ManagedWorkloadDeploymentStateDto::Completed,
        ManagedWorkloadOperationState::VerificationFailed => ManagedWorkloadDeploymentStateDto::VerificationFailed,
        ManagedWorkloadOperationState::DeploymentFailed => ManagedWorkloadDeploymentStateDto::DeploymentFailed,
    }
}

fn dto_problem(failure: Option<ManagedWorkloadFailure>) -> Option<ManagedWorkloadProblemDto> {
    match failure? {
        ManagedWorkloadFailure::WorkloadUnavailable | ManagedWorkloadFailure::Disabled | ManagedWorkloadFailure::VerificationConfigurationInvalid => Some(ManagedWorkloadProblemDto::WorkloadUnavailable),
        ManagedWorkloadFailure::TrustFailure => Some(ManagedWorkloadProblemDto::ActionUntrusted),
        ManagedWorkloadFailure::Transport => Some(ManagedWorkloadProblemDto::CommunicationFailed),
        ManagedWorkloadFailure::OutputLimit => Some(ManagedWorkloadProblemDto::OutputLimitExceeded),
        ManagedWorkloadFailure::Protocol | ManagedWorkloadFailure::InvalidTarget => Some(ManagedWorkloadProblemDto::ProtocolInvalid),
        ManagedWorkloadFailure::DeploymentFailed => Some(ManagedWorkloadProblemDto::DeploymentFailed),
        ManagedWorkloadFailure::VerificationFailed | ManagedWorkloadFailure::VerificationServiceFailed => Some(ManagedWorkloadProblemDto::VerificationFailed),
        ManagedWorkloadFailure::VerificationServiceMissing => Some(ManagedWorkloadProblemDto::RequiredServiceMissing),
        ManagedWorkloadFailure::StillRunning => Some(ManagedWorkloadProblemDto::ObservationExpired),
        ManagedWorkloadFailure::OutcomeUncertain => Some(ManagedWorkloadProblemDto::DispatchOutcomeUncertain),
        ManagedWorkloadFailure::CoordinatorConflict | ManagedWorkloadFailure::Persistence => None,
    }
}

fn deployment_dto(operation: &ManagedWorkloadOperation) -> ManagedWorkloadDeploymentDto {
    ManagedWorkloadDeploymentDto {
        operation: ManagedWorkloadOperationRef { workload_id: operation.workload_id.clone(), operation_id: operation.id.clone() },
        state: dto_state(operation.state),
        problem: dto_problem(operation.failure),
    }
}

fn blocks_fresh_prepare(state: ManagedWorkloadOperationState) -> bool {
    matches!(state,
        ManagedWorkloadOperationState::DispatchPrepared
            | ManagedWorkloadOperationState::Dispatching
            | ManagedWorkloadOperationState::DispatchUncertain
            | ManagedWorkloadOperationState::Deploying
            | ManagedWorkloadOperationState::StillRunning
            | ManagedWorkloadOperationState::AwaitingVerification
            | ManagedWorkloadOperationState::RevisionVerified
            | ManagedWorkloadOperationState::WorkloadRuntimeVerified
    )
}

pub(crate) fn list_managed_workloads_with(dir: &std::path::Path, device_id: &str) -> Vec<ManagedWorkloadCardDto> {
    let operations = JsonManagedWorkloadOperationRepository::new(dir);
    JsonManagedWorkloadRepository::new(dir).load().workloads.into_iter()
        .filter(|workload| workload.device_id == device_id)
        .map(|workload| {
            let operation = operations.get(&workload.device_id, &workload.id);
            let eligible_to_prepare = workload.enabled && operation.as_ref().is_none_or(|entry| !blocks_fresh_prepare(entry.state));
            ManagedWorkloadCardDto { workload_id: workload.id, name: workload.name, enabled: workload.enabled, eligible_to_prepare, deployment: operation.as_ref().map(deployment_dto) }
        })
        .collect()
}

fn config_dir(app: &AppHandle) -> Result<std::path::PathBuf, ApplicationError> {
    app.path().app_config_dir().map_err(|_| api_error("ConfigurationError", "Pi-Hub could not access its application storage.", "Check local application storage and try again.", true))
}

fn latest_operation(dir: &std::path::Path, workload_id: &str) -> Result<ManagedWorkloadOperation, ApplicationError> {
    let workload = JsonManagedWorkloadRepository::new(dir).load().workloads.into_iter().find(|workload| workload.id == workload_id)
        .ok_or_else(|| api_error("WorkloadUnavailable", "The configured managed workload is unavailable.", "Review the managed workload configuration.", false))?;
    let operation = JsonManagedWorkloadOperationRepository::new(dir).get(&workload.device_id, &workload.id)
        .ok_or_else(|| api_error("OperationUnavailable", "The requested managed workload operation is unavailable.", "Prepare the workload again.", false))?;
    Ok(operation)
}

fn persisted_operation(dir: &std::path::Path, reference: &ManagedWorkloadOperationRef) -> Result<ManagedWorkloadOperation, ApplicationError> {
    let operation = latest_operation(dir, &reference.workload_id)?;
    (operation.id == reference.operation_id).then_some(operation)
        .ok_or_else(|| api_error("OperationUnavailable", "The requested managed workload operation is unavailable.", "Prepare the workload again.", false))
}

pub(crate) fn prepare_managed_workload_with(dir: &std::path::Path, request: ManagedWorkloadPrepareRequest, executor: &dyn RemoteExecutor, coordinator: &DeviceMaintenanceCoordinator) -> Result<PreparedManagedWorkloadDto, ApplicationError> {
    let prepared = prepare_with(dir, &request.workload_id, executor, coordinator).map_err(map_prepare_error)?;
    Ok(PreparedManagedWorkloadDto { operation: deployment_dto(&prepared.operation), review_target_revision: prepared.operation.target_revision, change_count: prepared.change_count })
}

pub(crate) fn continue_managed_workload_with(dir: &std::path::Path, reference: ManagedWorkloadOperationRef, executor: &dyn RemoteExecutor, coordinator: &DeviceMaintenanceCoordinator) -> Result<ManagedWorkloadContinueResponse, ApplicationError> {
    let operation = persisted_operation(dir, &reference)?;
    let consent = revalidate_dispatch_consent_with(dir, &operation, executor, coordinator).map_err(map_prepare_error)?;
    match consent {
        DispatchConsent::PlanChanged => Ok(ManagedWorkloadContinueResponse {
            outcome: ManagedWorkloadContinuationDto::PlanChanged,
            operation: ManagedWorkloadDeploymentDto { state: ManagedWorkloadDeploymentStateDto::PlanChanged, ..deployment_dto(&operation) },
        }),
        DispatchConsent::DispatchEligible(eligible) => {
            let prepared = prepare_detached_dispatch(dir, eligible).map_err(map_prepare_error)?;
            let operation = dispatch_prepared_with(dir, prepared, executor).map_err(map_prepare_error)?;
            Ok(ManagedWorkloadContinueResponse { outcome: ManagedWorkloadContinuationDto::DispatchStarted, operation: deployment_dto(&operation) })
        }
    }
}

pub(crate) fn reconcile_managed_workload_with(dir: &std::path::Path, request: ManagedWorkloadStatusRequest, executor: &dyn RemoteExecutor) -> Result<ManagedWorkloadDeploymentDto, ApplicationError> {
    let operation = latest_operation(dir, &request.workload_id)?;
    let operation = match operation.state {
        ManagedWorkloadOperationState::Dispatching | ManagedWorkloadOperationState::DispatchUncertain | ManagedWorkloadOperationState::Deploying | ManagedWorkloadOperationState::StillRunning => reconcile_detached_workload_with(dir, &operation, executor).map_err(map_prepare_error)?,
        ManagedWorkloadOperationState::AwaitingVerification => verify_deployed_revision_with(dir, &operation, executor).map_err(map_verification_error)?,
        ManagedWorkloadOperationState::RevisionVerified => verify_workload_runtime_with(dir, &operation, executor).map_err(map_runtime_error)?,
        ManagedWorkloadOperationState::WorkloadRuntimeVerified => finalize_workload_verification(dir, &operation).map_err(map_final_error)?,
        _ => operation,
    };
    Ok(deployment_dto(&operation))
}

#[tauri::command]
pub async fn prepare_managed_workload_deployment(app: AppHandle, request: ManagedWorkloadPrepareRequest) -> Result<PreparedManagedWorkloadDto, ApplicationError> {
    let dir = config_dir(&app)?;
    prepare_managed_workload_with(&dir, request, &OpenSshExecutor::default(), app.state::<DeviceMaintenanceCoordinator>().inner())
}

#[tauri::command]
pub async fn continue_managed_workload_deployment(app: AppHandle, operation: ManagedWorkloadOperationRef) -> Result<ManagedWorkloadContinueResponse, ApplicationError> {
    let dir = config_dir(&app)?;
    continue_managed_workload_with(&dir, operation, &OpenSshExecutor::default(), app.state::<DeviceMaintenanceCoordinator>().inner())
}

#[tauri::command]
pub async fn reconcile_managed_workload_deployment(app: AppHandle, request: ManagedWorkloadStatusRequest) -> Result<ManagedWorkloadDeploymentDto, ApplicationError> {
    let dir = config_dir(&app)?;
    reconcile_managed_workload_with(&dir, request, &OpenSshExecutor::default())
}

#[tauri::command]
pub async fn list_managed_workload_deployments(app: AppHandle, device_id: String) -> Result<Vec<ManagedWorkloadCardDto>, ApplicationError> {
    let dir = config_dir(&app)?;
    Ok(list_managed_workloads_with(&dir, &device_id))
}

#[cfg(test)]
#[path = "managed_workloads_tests.rs"]
mod tests;
