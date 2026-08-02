use serde::{Deserialize, Serialize};

/// A single monitoring cycle's system metrics for a device. Every field is
/// optional: any of them may be genuinely unavailable on a given device
/// (no thermal zone, `/proc/device-tree/model` missing on non-Pi hardware,
/// a malformed value from the remote collection script, ...), and a
/// missing field must never fail the whole collection.
// Constructed by infrastructure::parsers::metrics, which nothing calls
// yet -- the scheduler that assembles device snapshots is a later M3
// work unit.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SystemMetrics {
    pub hostname: Option<String>,
    pub model: Option<String>,
    pub operating_system: Option<String>,
    pub kernel_version: Option<String>,
    pub architecture: Option<String>,
    pub uptime_seconds: Option<u64>,
    pub cpu_usage_percent: Option<f64>,
    pub load_average_1m: Option<f64>,
    pub load_average_5m: Option<f64>,
    pub load_average_15m: Option<f64>,
    pub memory_total_bytes: Option<u64>,
    pub memory_used_bytes: Option<u64>,
    pub disk_total_bytes: Option<u64>,
    pub disk_used_bytes: Option<u64>,
    pub temperature_celsius: Option<f64>,
}
