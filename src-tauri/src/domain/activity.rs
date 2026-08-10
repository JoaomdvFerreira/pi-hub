use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActivityCategory { Device, Health, Service, Container, Diagnostic, Administration }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEvent {
    pub id: String,
    pub timestamp: String,
    pub code: String,
    pub category: ActivityCategory,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_name: Option<String>,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl ActivityEvent {
    pub fn new(category: ActivityCategory, code: impl Into<String>, device_id: Option<String>, entity_id: Option<String>, entity_name: Option<String>, summary: impl Into<String>) -> Self {
        Self { id: Uuid::new_v4().to_string(), timestamp: chrono::Utc::now().to_rfc3339(), code: code.into(), category, device_id, entity_id, entity_name, summary: summary.into(), detail: None }
    }
}
