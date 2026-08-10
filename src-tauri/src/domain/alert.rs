use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AlertState {
    Active,
    Acknowledged,
    Resolved,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AlertCategory {
    Device,
    Health,
    Service,
    Container,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AlertResolutionReason {
    ConditionCleared,
    SourceRemoved,
    SourceDisabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    pub id: String,
    pub deduplication_key: String,
    pub category: AlertCategory,
    pub device_id: Option<String>,
    pub entity_id: Option<String>,
    pub rule_code: String,
    pub severity: AlertSeverity,
    pub state: AlertState,
    pub first_seen: String,
    pub last_seen: String,
    pub occurrence_count: u32,
    pub acknowledged_at: Option<String>,
    pub resolved_at: Option<String>,
    pub resolution_reason: Option<AlertResolutionReason>,
    pub summary: String,
    pub detail: Option<String>,
    pub threshold: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlertTransitionKind {
    Activated,
    Escalated,
    Acknowledged,
    Resolved,
}

#[derive(Debug, Clone)]
pub struct AlertTransition {
    pub kind: AlertTransitionKind,
    pub alert: Alert,
}

impl Alert {
    pub fn new(
        key: String,
        category: AlertCategory,
        device_id: Option<String>,
        entity_id: Option<String>,
        rule_code: impl Into<String>,
        severity: AlertSeverity,
        timestamp: String,
        summary: impl Into<String>,
        detail: Option<String>,
        threshold: Option<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            deduplication_key: key,
            category,
            device_id,
            entity_id,
            rule_code: rule_code.into(),
            severity,
            state: AlertState::Active,
            first_seen: timestamp.clone(),
            last_seen: timestamp,
            occurrence_count: 1,
            acknowledged_at: None,
            resolved_at: None,
            resolution_reason: None,
            summary: summary.into(),
            detail,
            threshold,
        }
    }
}
