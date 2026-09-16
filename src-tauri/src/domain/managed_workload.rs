use serde::{Deserialize, Serialize};

pub const MANAGED_WORKLOAD_SCHEMA_VERSION: u32 = 1;
pub const ACTION_ID_MAX_LEN: usize = 64;
pub const WORKLOAD_ID_MAX_LEN: usize = 64;
pub const DISPLAY_NAME_MAX_LEN: usize = 96;
pub const REFERENCE_ID_MAX_LEN: usize = 96;
pub const MAX_WORKLOADS: usize = 32;
pub const MAX_VERIFICATION_REFERENCES: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManagedWorkload {
    pub id: String,
    pub device_id: String,
    pub name: String,
    pub action_id: String,
    pub compose_project: Option<String>,
    #[serde(default)]
    pub required_compose_services: Vec<String>,
    #[serde(default)]
    pub required_service_health_ids: Vec<String>,
    pub enabled: bool,
    pub config_revision: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedWorkloadValidationError {
    InvalidWorkloadId,
    InvalidDeviceId,
    InvalidName,
    InvalidActionId,
    InvalidVerificationReference,
    MissingVerificationTarget,
    TooManyVerificationTargets,
    DuplicateWorkloadId,
    TooManyWorkloads,
    InvalidSchemaVersion,
}

impl ManagedWorkload {
    pub fn validate(&self) -> Result<(), ManagedWorkloadValidationError> {
        if !valid_identifier(&self.id, WORKLOAD_ID_MAX_LEN) {
            return Err(ManagedWorkloadValidationError::InvalidWorkloadId);
        }
        if !valid_identifier(&self.device_id, WORKLOAD_ID_MAX_LEN) {
            return Err(ManagedWorkloadValidationError::InvalidDeviceId);
        }
        if self.name.is_empty()
            || self.name.len() > DISPLAY_NAME_MAX_LEN
            || self.name.chars().any(char::is_control)
        {
            return Err(ManagedWorkloadValidationError::InvalidName);
        }
        if !is_valid_action_id(&self.action_id) {
            return Err(ManagedWorkloadValidationError::InvalidActionId);
        }
        if self
            .compose_project
            .as_deref()
            .is_some_and(|v| !valid_identifier(v, REFERENCE_ID_MAX_LEN))
        {
            return Err(ManagedWorkloadValidationError::InvalidVerificationReference);
        }
        let all = self.required_compose_services.len() + self.required_service_health_ids.len();
        if all == 0 {
            return Err(ManagedWorkloadValidationError::MissingVerificationTarget);
        }
        if all > MAX_VERIFICATION_REFERENCES {
            return Err(ManagedWorkloadValidationError::TooManyVerificationTargets);
        }
        if self
            .required_compose_services
            .iter()
            .chain(&self.required_service_health_ids)
            .any(|v| !valid_identifier(v, REFERENCE_ID_MAX_LEN))
        {
            return Err(ManagedWorkloadValidationError::InvalidVerificationReference);
        }
        if !self.required_compose_services.is_empty() && self.compose_project.is_none() {
            return Err(ManagedWorkloadValidationError::InvalidVerificationReference);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManagedWorkloadFile {
    pub schema_version: u32,
    #[serde(default)]
    pub workloads: Vec<ManagedWorkload>,
}

impl Default for ManagedWorkloadFile {
    fn default() -> Self {
        Self {
            schema_version: MANAGED_WORKLOAD_SCHEMA_VERSION,
            workloads: vec![],
        }
    }
}

impl ManagedWorkloadFile {
    pub fn validate(&self) -> Result<(), ManagedWorkloadValidationError> {
        if self.schema_version != MANAGED_WORKLOAD_SCHEMA_VERSION {
            return Err(ManagedWorkloadValidationError::InvalidSchemaVersion);
        }
        if self.workloads.len() > MAX_WORKLOADS {
            return Err(ManagedWorkloadValidationError::TooManyWorkloads);
        }
        for (index, workload) in self.workloads.iter().enumerate() {
            workload.validate()?;
            if self.workloads[..index]
                .iter()
                .any(|prior| prior.id == workload.id)
            {
                return Err(ManagedWorkloadValidationError::DuplicateWorkloadId);
            }
        }
        Ok(())
    }
}

pub fn is_valid_action_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= ACTION_ID_MAX_LEN
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn valid_identifier(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn action_id_grammar_rejects_paths_and_oversize() {
        for bad in [
            "../x",
            "/x",
            "a/b",
            "a\\b",
            "a%2fb",
            "A",
            "-a",
            "a_",
            "a ",
            &"a".repeat(65),
        ] {
            assert!(!is_valid_action_id(bad), "{bad}");
        }
        assert!(is_valid_action_id("personal-finance"));
    }
}
