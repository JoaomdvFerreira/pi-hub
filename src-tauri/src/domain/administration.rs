use crate::domain::maintenance::MaintenanceOperation;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// The deliberately small, closed catalogue of privileged device actions.
/// This is serialized across the Tauri boundary; it is never a command string.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AdministrationOperationType {
    RestartDevice,
    ShutdownDevice,
    RestartDocker,
    RestartTailscale,
}

impl AdministrationOperationType {
    pub fn label(self) -> &'static str {
        match self {
            Self::RestartDevice => "Restart Device",
            Self::ShutdownDevice => "Shut Down Device",
            Self::RestartDocker => "Restart Docker",
            Self::RestartTailscale => "Restart Tailscale",
        }
    }
    pub fn expected_window(self) -> Duration {
        match self {
            Self::RestartDevice => Duration::minutes(10),
            Self::ShutdownDevice => Duration::hours(24),
            Self::RestartDocker | Self::RestartTailscale => Duration::minutes(5),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AdministrationOperationState {
    Requested,
    Dispatching,
    CommandAccepted,
    Verifying,
    WaitingForOffline,
    WaitingForOnline,
    Completed,
    Failed,
    TimedOut,
    OutcomeUncertain,
}

/// Expected disruption has a broader owner vocabulary than the M10 command
/// catalogue: M16 reuses its bounded alert semantics without becoming an M10
/// administration command.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ExpectedDisruptionOperationType {
    RestartDevice,
    ShutdownDevice,
    RestartDocker,
    RestartTailscale,
    UpdateDevice,
}

impl From<AdministrationOperationType> for ExpectedDisruptionOperationType {
    fn from(value: AdministrationOperationType) -> Self {
        match value {
            AdministrationOperationType::RestartDevice => Self::RestartDevice,
            AdministrationOperationType::ShutdownDevice => Self::ShutdownDevice,
            AdministrationOperationType::RestartDocker => Self::RestartDocker,
            AdministrationOperationType::RestartTailscale => Self::RestartTailscale,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ExpectedDisruptionPhase {
    CommandAccepted,
    UpdateActive,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AdministrationFailure {
    InsufficientPrivileges,
    SudoPasswordRequired,
    Transport,
    HostKey,
    Authentication,
    CommandRejected,
    CommandTimeout,
    VerificationFailed,
    VerificationTimeout,
    Conflict,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdministrationOperation {
    pub id: String,
    pub device_id: String,
    pub operation_type: AdministrationOperationType,
    pub requested_at: String,
    pub state: AdministrationOperationState,
    pub accepted_at: Option<String>,
    pub completed_at: Option<String>,
    pub failure: Option<AdministrationFailure>,
    pub detail: Option<String>,
}

impl AdministrationOperation {
    pub fn requested(device_id: String, operation_type: AdministrationOperationType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            device_id,
            operation_type,
            requested_at: Utc::now().to_rfc3339(),
            state: AdministrationOperationState::Requested,
            accepted_at: None,
            completed_at: None,
            failure: None,
            detail: None,
        }
    }
    pub fn transition(&mut self, state: AdministrationOperationState, detail: Option<String>) {
        self.state = state;
        self.detail = detail;
        if state == AdministrationOperationState::CommandAccepted {
            self.accepted_at = Some(Utc::now().to_rfc3339());
        }
        if matches!(
            state,
            AdministrationOperationState::Completed
                | AdministrationOperationState::Failed
                | AdministrationOperationState::TimedOut
                | AdministrationOperationState::OutcomeUncertain
        ) {
            self.completed_at = Some(Utc::now().to_rfc3339());
        }
    }
}

/// Persisted only while it changes monitoring semantics; expiry is absolute so
/// a crash or restart can never leave notification suppression permanent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedDisruption {
    pub device_id: String,
    pub operation_id: String,
    pub operation_type: ExpectedDisruptionOperationType,
    pub started_at: String,
    pub expected_until: String,
    pub phase: ExpectedDisruptionPhase,
}

impl ExpectedDisruption {
    pub fn new(operation: &AdministrationOperation) -> Self {
        let started = Utc::now();
        Self {
            device_id: operation.device_id.clone(),
            operation_id: operation.id.clone(),
            operation_type: operation.operation_type.into(),
            started_at: started.to_rfc3339(),
            expected_until: (started + operation.operation_type.expected_window()).to_rfc3339(),
            phase: ExpectedDisruptionPhase::CommandAccepted,
        }
    }

    /// The future M16 execution owner calls this only after its known unit
    /// might be active. Requested/pre-dispatch state deliberately has no
    /// suppression marker, so a crash cannot mask alerts before mutation.
    #[allow(dead_code)]
    pub fn for_update(operation: &MaintenanceOperation) -> Option<Self> {
        if operation.requires_fresh_confirmation_after_restart() || operation.state.is_terminal() {
            return None;
        }
        let expected_until = operation.observation_deadline.clone()?;
        Some(Self {
            device_id: operation.device_id.clone(),
            operation_id: operation.id.clone(),
            operation_type: ExpectedDisruptionOperationType::UpdateDevice,
            started_at: operation
                .started_at
                .clone()
                .unwrap_or_else(|| operation.requested_at.clone()),
            expected_until,
            phase: ExpectedDisruptionPhase::UpdateActive,
        })
    }
    pub fn is_valid_at(&self, now: DateTime<Utc>) -> bool {
        DateTime::parse_from_rfc3339(&self.expected_until)
            .map(|expiry| expiry.with_timezone(&Utc) > now)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_are_bounded_and_shutdown_is_longer() {
        assert_eq!(
            AdministrationOperationType::RestartDevice.expected_window(),
            Duration::minutes(10)
        );
        assert_eq!(
            AdministrationOperationType::ShutdownDevice.expected_window(),
            Duration::hours(24)
        );
    }
    #[test]
    fn malformed_expiry_fails_safe() {
        let op = AdministrationOperation::requested(
            "d".into(),
            AdministrationOperationType::RestartDevice,
        );
        let mut marker = ExpectedDisruption::new(&op);
        marker.expected_until = "not a date".into();
        assert!(!marker.is_valid_at(Utc::now()));
    }

    #[test]
    fn m16_marker_is_only_created_for_a_recovery_relevant_dispatched_operation() {
        use crate::domain::maintenance::{
            MaintenanceDispatchState, MaintenanceOperation, MaintenanceOperationState,
        };
        let mut operation = MaintenanceOperation::requested("d".into());
        operation.observation_deadline = Some((Utc::now() + Duration::minutes(60)).to_rfc3339());
        assert!(ExpectedDisruption::for_update(&operation).is_none());
        operation.dispatch_state = MaintenanceDispatchState::Uncertain;
        operation.state = MaintenanceOperationState::Dispatching;
        let marker = ExpectedDisruption::for_update(&operation).unwrap();
        assert_eq!(
            marker.operation_type,
            ExpectedDisruptionOperationType::UpdateDevice
        );
        assert!(marker.is_valid_at(Utc::now()));
        operation.state = MaintenanceOperationState::Failed;
        assert!(ExpectedDisruption::for_update(&operation).is_none());
    }
}
