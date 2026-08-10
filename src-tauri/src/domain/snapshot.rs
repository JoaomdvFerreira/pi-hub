use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::domain::connection_status::DeviceConnectionStatus;
use crate::domain::docker_container::DockerContainerSummary;
use crate::domain::health::DeviceHealthAssessment;
use crate::domain::system_metrics::SystemMetrics;
use crate::domain::device_visibility::{NetworkVisibility, StorageVisibility, SystemVisibility};
use crate::domain::service_health::ServiceHealthRecord;
use crate::error::ApplicationError;

/// One monitoring cycle's result for a device. Immutable once built: the
/// scheduler always constructs a brand new snapshot rather than mutating a
/// previous one.
///
/// On a failed refresh (`stale: true`), `metrics`/`docker_available`/
/// `containers` are carried forward from the previous snapshot rather than
/// cleared, per the architecture spec's failure-isolation rule that a
/// failed refresh must never erase previously successful data -- only
/// `connectionStatus`/`error`/`capturedAt` reflect the new failed attempt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSnapshot {
    pub device_id: String,
    pub connection_status: DeviceConnectionStatus,
    pub captured_at: String,
    pub duration_ms: u64,
    pub metrics: Option<SystemMetrics>,
    pub docker_available: bool,
    pub containers: Vec<DockerContainerSummary>,
    pub warnings: Vec<String>,
    pub error: Option<ApplicationError>,
    /// True when this snapshot's metrics/containers are carried-forward
    /// last-known data rather than freshly collected this cycle.
    pub stale: bool,
    /// The `capturedAt` of the most recent snapshot where
    /// `connectionStatus` was `Online`, if any.
    pub last_successful_refresh: Option<String>,
    pub health: DeviceHealthAssessment,
    #[serde(default)]
    pub service_health: HashMap<String, ServiceHealthRecord>,
    #[serde(default)]
    pub network_visibility: Option<NetworkVisibility>,
    #[serde(default)]
    pub storage_visibility: Option<StorageVisibility>,
    #[serde(default)]
    pub system_visibility: Option<SystemVisibility>,
}
