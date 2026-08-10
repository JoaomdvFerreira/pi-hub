use serde::{Deserialize, Serialize};

use super::{connection_status::DeviceConnectionStatus, system_metrics::SystemMetrics};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DeviceHealthState {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HealthReasonSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HealthReason {
    pub code: String,
    pub severity: HealthReasonSeverity,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RaspberryPiPowerState {
    pub raw: Option<String>,
    pub undervoltage_now: Option<bool>,
    pub undervoltage_since_boot: Option<bool>,
    pub frequency_capped_now: Option<bool>,
    pub frequency_capped_since_boot: Option<bool>,
    pub throttled_now: Option<bool>,
    pub throttled_since_boot: Option<bool>,
    pub soft_temperature_limit_now: Option<bool>,
    pub soft_temperature_limit_since_boot: Option<bool>,
}

pub fn parse_throttling(raw: Option<&str>) -> RaspberryPiPowerState {
    let Some(raw) = raw.map(str::trim).filter(|v| !v.is_empty()) else {
        return RaspberryPiPowerState::default();
    };
    let value = raw
        .strip_prefix("throttled=")
        .unwrap_or(raw)
        .trim()
        .strip_prefix("0x")
        .or_else(|| {
            raw.strip_prefix("throttled=")
                .and_then(|v| v.trim().strip_prefix("0x"))
        })
        .and_then(|v| u32::from_str_radix(v, 16).ok());
    let Some(value) = value else {
        return RaspberryPiPowerState {
            raw: Some(raw.to_string()),
            ..Default::default()
        };
    };
    let bit = |n: u32| Some(value & (1u32 << n) != 0);
    RaspberryPiPowerState {
        raw: Some(raw.to_string()),
        undervoltage_now: bit(0),
        frequency_capped_now: bit(1),
        throttled_now: bit(2),
        soft_temperature_limit_now: bit(3),
        undervoltage_since_boot: bit(16),
        frequency_capped_since_boot: bit(17),
        throttled_since_boot: bit(18),
        soft_temperature_limit_since_boot: bit(19),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceHealthAssessment {
    pub state: DeviceHealthState,
    pub reasons: Vec<HealthReason>,
    pub power: RaspberryPiPowerState,
}

pub fn assess_health(
    status: &DeviceConnectionStatus,
    metrics: Option<&SystemMetrics>,
) -> DeviceHealthAssessment {
    let power = metrics
        .map(|m| parse_throttling(m.throttling_raw.as_deref()))
        .unwrap_or_default();
    if *status != DeviceConnectionStatus::Online {
        return DeviceHealthAssessment {
            state: DeviceHealthState::Unknown,
            reasons: vec![],
            power,
        };
    }
    let Some(metrics) = metrics else {
        return DeviceHealthAssessment {
            state: DeviceHealthState::Unknown,
            reasons: vec![],
            power,
        };
    };
    let mut reasons = Vec::new();
    let mut add = |code: &str, severity, summary: &str| {
        reasons.push(HealthReason {
            code: code.into(),
            severity,
            summary: summary.into(),
        })
    };
    let disk = match (metrics.disk_used_bytes, metrics.disk_total_bytes) {
        (Some(used), Some(total)) if total > 0 => Some(used as f64 / total as f64 * 100.0),
        _ => None,
    };
    if disk.is_some_and(|v| v >= 95.0) {
        add(
            "disk_critical",
            HealthReasonSeverity::Critical,
            "Root disk usage is critical",
        );
    } else if disk.is_some_and(|v| v >= 85.0) {
        add(
            "disk_warning",
            HealthReasonSeverity::Warning,
            "Root disk usage is high",
        );
    }
    if metrics.temperature_celsius.is_some_and(|v| v >= 80.0) {
        add(
            "temperature_critical",
            HealthReasonSeverity::Critical,
            "Device temperature is critical",
        );
    } else if metrics.temperature_celsius.is_some_and(|v| v >= 70.0) {
        add(
            "temperature_warning",
            HealthReasonSeverity::Warning,
            "Device temperature is high",
        );
    }
    if metrics.root_filesystem_read_only == Some(true) {
        add(
            "root_filesystem_read_only",
            HealthReasonSeverity::Critical,
            "Root filesystem is read-only",
        );
    }
    if power.undervoltage_now == Some(true) {
        add(
            "undervoltage_current",
            HealthReasonSeverity::Warning,
            "Undervoltage is currently detected",
        );
    } else if power.undervoltage_since_boot == Some(true) {
        add(
            "undervoltage_historical",
            HealthReasonSeverity::Warning,
            "Undervoltage occurred since boot",
        );
    }
    if power.frequency_capped_now == Some(true) {
        add(
            "frequency_capped_current",
            HealthReasonSeverity::Warning,
            "CPU frequency is currently capped",
        );
    }
    if power.throttled_now == Some(true) {
        add(
            "throttling_current",
            HealthReasonSeverity::Warning,
            "Device is currently throttled",
        );
    }
    if metrics.reboot_required == Some(true) {
        add(
            "reboot_required",
            HealthReasonSeverity::Info,
            "A reboot is required",
        );
    }
    let state = if reasons
        .iter()
        .any(|r| r.severity == HealthReasonSeverity::Critical)
    {
        DeviceHealthState::Critical
    } else if reasons
        .iter()
        .any(|r| r.severity == HealthReasonSeverity::Warning)
    {
        DeviceHealthState::Warning
    } else if metrics.temperature_celsius.is_some()
        || disk.is_some()
        || metrics.cpu_usage_percent.is_some()
    {
        DeviceHealthState::Healthy
    } else {
        DeviceHealthState::Unknown
    };
    DeviceHealthAssessment {
        state,
        reasons,
        power,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn throttling_zero_is_known_clear() {
        let p = parse_throttling(Some("throttled=0x0"));
        assert_eq!(p.undervoltage_now, Some(false));
        assert_eq!(p.soft_temperature_limit_since_boot, Some(false));
    }
    #[test]
    fn maps_current_and_historical_bits_independently() {
        let p = parse_throttling(Some("throttled=0xF000F"));
        assert_eq!(p.undervoltage_now, Some(true));
        assert_eq!(p.frequency_capped_now, Some(true));
        assert_eq!(p.throttled_now, Some(true));
        assert_eq!(p.soft_temperature_limit_now, Some(true));
        assert_eq!(p.undervoltage_since_boot, Some(true));
        assert_eq!(p.frequency_capped_since_boot, Some(true));
        assert_eq!(p.throttled_since_boot, Some(true));
        assert_eq!(p.soft_temperature_limit_since_boot, Some(true));
    }
    #[test]
    fn malformed_and_absent_throttling_are_unknown() {
        assert_eq!(parse_throttling(Some("bad")).undervoltage_now, None);
        assert_eq!(parse_throttling(None).raw, None);
    }
    #[test]
    fn exact_thresholds_and_precedence_are_deterministic() {
        let warning = SystemMetrics {
            disk_total_bytes: Some(100),
            disk_used_bytes: Some(85),
            temperature_celsius: Some(70.0),
            ..Default::default()
        };
        assert_eq!(
            assess_health(&DeviceConnectionStatus::Online, Some(&warning)).state,
            DeviceHealthState::Warning
        );
        let critical = SystemMetrics {
            disk_total_bytes: Some(100),
            disk_used_bytes: Some(95),
            temperature_celsius: Some(80.0),
            ..Default::default()
        };
        let a = assess_health(&DeviceConnectionStatus::Online, Some(&critical));
        assert_eq!(a.state, DeviceHealthState::Critical);
        assert_eq!(a.reasons.len(), 2);
    }
    #[test]
    fn offline_and_unsupported_are_unknown() {
        assert_eq!(
            assess_health(&DeviceConnectionStatus::Timeout, None).state,
            DeviceHealthState::Unknown
        );
        assert_eq!(
            assess_health(
                &DeviceConnectionStatus::Online,
                Some(&SystemMetrics::default())
            )
            .state,
            DeviceHealthState::Unknown
        );
    }
    #[test]
    fn reboot_only_is_healthy_with_info_reason() {
        let m = SystemMetrics {
            cpu_usage_percent: Some(1.0),
            reboot_required: Some(true),
            ..Default::default()
        };
        let a = assess_health(&DeviceConnectionStatus::Online, Some(&m));
        assert_eq!(a.state, DeviceHealthState::Healthy);
        assert_eq!(a.reasons[0].severity, HealthReasonSeverity::Info);
    }
}
