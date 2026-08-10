use serde::{Deserialize, Serialize};

use super::{health::DeviceHealthState, service_health::ServiceHealthState};

pub const RETENTION_DAYS: i64 = 30;
pub const SAMPLE_INTERVAL_SECONDS: i64 = 60;
pub const MAX_HISTORY_POINTS: usize = 500;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HistoricalRange { OneHour, TwentyFourHours, SevenDays, ThirtyDays }
impl HistoricalRange { pub fn seconds(self) -> i64 { match self { Self::OneHour => 3600, Self::TwentyFourHours => 86400, Self::SevenDays => 604800, Self::ThirtyDays => 2592000 } } }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HistoricalMetric { CpuUsagePercent, MemoryUsagePercent, RootFilesystemUsagePercent, TemperatureCelsius, ResponseTimeMs, DeviceHealth, ServiceHealth, ContainerCpuPercent, ContainerMemoryPercent }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceHistoricalSample { pub device_id: String, pub timestamp: String, pub cpu_usage_percent: Option<f64>, pub memory_usage_percent: Option<f64>, pub root_filesystem_usage_percent: Option<f64>, pub temperature_celsius: Option<f64>, pub health_state: Option<DeviceHealthState> }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServiceHistoricalSample { pub device_id: String, pub service_id: String, pub timestamp: String, pub response_time_ms: Option<u64>, pub health_state: ServiceHealthState }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContainerHistoricalSample { pub device_id: String, pub container_id: String, pub timestamp: String, pub cpu_percent: Option<f64>, pub memory_percent: Option<f64> }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "sampleType", content = "sample", rename_all = "camelCase")]
pub enum HistoricalSample { Device(DeviceHistoricalSample), Service(ServiceHistoricalSample), Container(ContainerHistoricalSample) }
impl HistoricalSample { pub fn timestamp(&self) -> &str { match self { Self::Device(x) => &x.timestamp, Self::Service(x) => &x.timestamp, Self::Container(x) => &x.timestamp } } }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalNumericPoint { pub timestamp: String, pub value: f64, pub minimum: f64, pub maximum: f64 }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalStatePoint { pub timestamp: String, pub state: String }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalSeries { pub numeric_points: Vec<HistoricalNumericPoint>, pub state_points: Vec<HistoricalStatePoint> }
