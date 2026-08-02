// Constructed by infrastructure::parsers::docker, which nothing calls yet
// -- the scheduler that assembles device snapshots is a later M3 work
// unit.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DockerContainerState {
    Running,
    Stopped,
    Exited,
    Restarting,
    Paused,
    Dead,
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
}
