// Constructed by infrastructure::parsers::docker, which nothing calls yet
// -- the scheduler that assembles device snapshots is a later M3 work
// unit.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// A container lifecycle action a user can trigger from the UI (spec:
/// Phase 2 lifts the MVP's read-only-only Docker constraint for exactly
/// these three). Container creation/removal, exec, logs, and compose
/// editing remain out of scope.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContainerAction {
    Start,
    Stop,
    Restart,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ContainerLogMode { Last100, Last500, Last15Minutes, Last1Hour }

impl ContainerLogMode {
    pub fn docker_arguments(self) -> &'static str { match self { Self::Last100 => "--tail 100", Self::Last500 => "--tail 500", Self::Last15Minutes => "--since 15m", Self::Last1Hour => "--since 1h" } }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContainerLogResult { pub container_id: String, pub mode: ContainerLogMode, pub collected_at: String, pub content: String, pub truncated: bool }

impl ContainerAction {
    pub fn docker_verb(&self) -> &'static str {
        match self {
            ContainerAction::Start => "start",
            ContainerAction::Stop => "stop",
            ContainerAction::Restart => "restart",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DockerContainerState {
    Created,
    Running,
    /// Retained for compatibility with snapshots written by earlier milestones.
    Stopped,
    Exited,
    Restarting,
    Paused,
    Dead,
    Removing,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DockerHealthStatus {
    Healthy,
    Unhealthy,
    Starting,
    None,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DockerPortBinding {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_port: Option<u16>,
    pub container_port: u16,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DockerMount {
    pub mount_type: String,
    pub source: String,
    pub destination: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DockerNetwork {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DockerLabel {
    pub key: String,
    pub value: String,
    pub redacted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DockerResourceUsage {
    pub cpu_percent: Option<f64>,
    pub memory_used_bytes: Option<u64>,
    pub memory_limit_bytes: Option<u64>,
    pub memory_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DockerContainerSummary {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: DockerContainerState,
    pub status_text: String,
    pub health: DockerHealthStatus,
    pub ports: Vec<DockerPortBinding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart_maximum_retry_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mounts: Vec<DockerMount>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub networks: Vec<DockerNetwork>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<DockerLabel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_usage: Option<DockerResourceUsage>,
}
