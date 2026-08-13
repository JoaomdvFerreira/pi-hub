use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::activity::{ActivityCategory, ActivityEvent};

#[allow(dead_code)]
pub const MAINTENANCE_OPERATION_SCHEMA_VERSION: u32 = 1;
#[allow(dead_code)]
pub const PLAN_FINGERPRINT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MaintenanceOperationState {
    Requested,
    Preflight,
    RefreshingMetadata,
    VerifyingPlan,
    PlanChanged,
    Dispatching,
    Installing,
    Verifying,
    Completed,
    CompletedRebootRequired,
    PackageManagerBusy,
    Failed,
    StillRunning,
    OutcomeUncertain,
}

#[allow(dead_code)]
impl MaintenanceOperationState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::PlanChanged
                | Self::Completed
                | Self::CompletedRebootRequired
                | Self::PackageManagerBusy
                | Self::Failed
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MaintenanceDispatchState {
    NotAttempted,
    Accepted,
    Uncertain,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MaintenanceFailure {
    Unsupported,
    PrivilegeUnavailable,
    PackageManagerBusy,
    PackageStateInconsistent,
    MetadataRefreshFailed,
    PlanChanged,
    DispatchFailed,
    PackageOperationFailed,
    TransportUnavailableDuringObservation,
    VerificationFailed,
    StillRunning,
    OutcomeUncertain,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlanFingerprint {
    pub version: u32,
    pub digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MaintenanceOperation {
    pub schema_version: u32,
    pub id: String,
    pub device_id: String,
    pub transient_unit_id: String,
    pub reviewed_plan: Option<PlanFingerprint>,
    pub package_count: Option<u32>,
    pub requested_at: String,
    pub started_at: Option<String>,
    pub observation_deadline: Option<String>,
    pub state: MaintenanceOperationState,
    pub dispatch_state: MaintenanceDispatchState,
    pub completed_at: Option<String>,
    pub failure: Option<MaintenanceFailure>,
    pub reboot_required: Option<bool>,
}

#[allow(dead_code)]
impl MaintenanceOperation {
    pub fn requested(device_id: String) -> Self {
        let id = Uuid::new_v4().to_string();
        let unit_suffix = id.replace('-', "");
        let transient_unit_id = format!("pihub-update-{unit_suffix}.service");
        debug_assert!(Self::is_valid_transient_unit_id(&transient_unit_id));
        Self {
            schema_version: MAINTENANCE_OPERATION_SCHEMA_VERSION,
            id,
            device_id,
            transient_unit_id,
            reviewed_plan: None,
            package_count: None,
            requested_at: Utc::now().to_rfc3339(),
            started_at: None,
            observation_deadline: None,
            state: MaintenanceOperationState::Requested,
            dispatch_state: MaintenanceDispatchState::NotAttempted,
            completed_at: None,
            failure: None,
            reboot_required: None,
        }
    }

    pub fn is_valid_transient_unit_id(value: &str) -> bool {
        let Some(value) = value.strip_prefix("pihub-update-") else {
            return false;
        };
        let Some(value) = value.strip_suffix(".service") else {
            return false;
        };
        value.len() == 32
            && value
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    }

    pub fn requires_recovery(&self) -> bool {
        self.dispatch_state != MaintenanceDispatchState::NotAttempted && !self.state.is_terminal()
    }

    pub fn requires_fresh_confirmation_after_restart(&self) -> bool {
        self.dispatch_state == MaintenanceDispatchState::NotAttempted
    }

    pub fn transition(&mut self, state: MaintenanceOperationState) {
        self.state = state;
        if self.started_at.is_none() && !matches!(state, MaintenanceOperationState::Requested) {
            self.started_at = Some(Utc::now().to_rfc3339());
        }
        if state.is_terminal() {
            self.completed_at = Some(Utc::now().to_rfc3339());
        }
    }

    /// This creates the bounded Activity shape WU16-03 will append after a
    /// real lifecycle transition. WU16-02 does not invoke it, so no update
    /// activity is fabricated before an execution path exists.
    pub fn activity_event(&self) -> Option<ActivityEvent> {
        let (code, summary) = match self.state {
            MaintenanceOperationState::Requested => (
                "updates.initiated",
                format!(
                    "Software update initiated ({} packages).",
                    self.package_count.unwrap_or(0)
                ),
            ),
            MaintenanceOperationState::Completed => {
                ("updates.completed", "Software update completed.".into())
            }
            MaintenanceOperationState::CompletedRebootRequired => (
                "updates.reboot_required",
                "Software update completed; restart required.".into(),
            ),
            MaintenanceOperationState::Failed | MaintenanceOperationState::PackageManagerBusy => (
                "updates.failed",
                "Software update failed; review the typed failure result.".into(),
            ),
            MaintenanceOperationState::StillRunning => (
                "updates.still_running",
                "Software update is still running and will be recovered.".into(),
            ),
            MaintenanceOperationState::OutcomeUncertain => (
                "updates.outcome_uncertain",
                "Software update outcome needs verification.".into(),
            ),
            _ => return None,
        };
        Some(ActivityEvent::new(
            ActivityCategory::Administration,
            code,
            Some(self.device_id.clone()),
            Some(self.id.clone()),
            Some("Update Device".into()),
            summary,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requested_operation_owns_a_valid_backend_generated_unit_id() {
        let operation = MaintenanceOperation::requested("pi5".into());
        assert!(MaintenanceOperation::is_valid_transient_unit_id(
            &operation.transient_unit_id
        ));
        assert!(operation.requires_fresh_confirmation_after_restart());
        assert!(!operation.requires_recovery());
    }

    #[test]
    fn only_verified_terminal_outcomes_release_recovery_ownership() {
        for state in [
            MaintenanceOperationState::Dispatching,
            MaintenanceOperationState::Installing,
            MaintenanceOperationState::StillRunning,
            MaintenanceOperationState::OutcomeUncertain,
        ] {
            assert!(!state.is_terminal());
        }
        for state in [
            MaintenanceOperationState::PlanChanged,
            MaintenanceOperationState::Completed,
            MaintenanceOperationState::CompletedRebootRequired,
            MaintenanceOperationState::PackageManagerBusy,
            MaintenanceOperationState::Failed,
        ] {
            assert!(state.is_terminal());
        }
    }

    #[test]
    fn activity_shape_never_claims_success_before_a_completed_state() {
        let mut operation = MaintenanceOperation::requested("pi5".into());
        assert_eq!(
            operation.activity_event().unwrap().code,
            "updates.initiated"
        );
        operation.state = MaintenanceOperationState::Installing;
        assert!(operation.activity_event().is_none());
        operation.state = MaintenanceOperationState::Completed;
        assert_eq!(
            operation.activity_event().unwrap().code,
            "updates.completed"
        );
    }
}
