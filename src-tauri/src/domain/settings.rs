use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub const CONFIG_SCHEMA_VERSION: u32 = 1;
pub const MIN_REFRESH_INTERVAL_SECONDS: u32 = 15;
pub const MAX_REFRESH_INTERVAL_SECONDS: u32 = 3600;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThresholdPolicy {
    pub cpu_warning_percent: f64,
    pub cpu_critical_percent: f64,
    pub cpu_duration_seconds: u32,
    pub memory_warning_percent: f64,
    pub memory_critical_percent: f64,
    pub memory_duration_seconds: u32,
    pub disk_warning_percent: f64,
    pub disk_critical_percent: f64,
    pub temperature_warning_celsius: f64,
    pub temperature_critical_celsius: f64,
    pub temperature_consecutive_samples: u32,
    pub service_unavailable_failures: u32,
}

impl Default for ThresholdPolicy {
    fn default() -> Self {
        Self {
            cpu_warning_percent: 85.0,
            cpu_critical_percent: 95.0,
            cpu_duration_seconds: 300,
            memory_warning_percent: 90.0,
            memory_critical_percent: 95.0,
            memory_duration_seconds: 300,
            disk_warning_percent: 85.0,
            disk_critical_percent: 95.0,
            temperature_warning_celsius: 70.0,
            temperature_critical_celsius: 80.0,
            temperature_consecutive_samples: 2,
            service_unavailable_failures: 3,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThresholdPolicyOverrides {
    pub cpu_warning_percent: Option<f64>,
    pub cpu_critical_percent: Option<f64>,
    pub cpu_duration_seconds: Option<u32>,
    pub memory_warning_percent: Option<f64>,
    pub memory_critical_percent: Option<f64>,
    pub memory_duration_seconds: Option<u32>,
    pub disk_warning_percent: Option<f64>,
    pub disk_critical_percent: Option<f64>,
    pub temperature_warning_celsius: Option<f64>,
    pub temperature_critical_celsius: Option<f64>,
    pub temperature_consecutive_samples: Option<u32>,
    pub service_unavailable_failures: Option<u32>,
}

impl ThresholdPolicy {
    pub fn with_overrides(&self, value: Option<&ThresholdPolicyOverrides>) -> Self {
        let Some(value) = value else { return self.clone(); };
        Self {
            cpu_warning_percent: value.cpu_warning_percent.unwrap_or(self.cpu_warning_percent),
            cpu_critical_percent: value.cpu_critical_percent.unwrap_or(self.cpu_critical_percent),
            cpu_duration_seconds: value.cpu_duration_seconds.unwrap_or(self.cpu_duration_seconds),
            memory_warning_percent: value.memory_warning_percent.unwrap_or(self.memory_warning_percent),
            memory_critical_percent: value.memory_critical_percent.unwrap_or(self.memory_critical_percent),
            memory_duration_seconds: value.memory_duration_seconds.unwrap_or(self.memory_duration_seconds),
            disk_warning_percent: value.disk_warning_percent.unwrap_or(self.disk_warning_percent),
            disk_critical_percent: value.disk_critical_percent.unwrap_or(self.disk_critical_percent),
            temperature_warning_celsius: value.temperature_warning_celsius.unwrap_or(self.temperature_warning_celsius),
            temperature_critical_celsius: value.temperature_critical_celsius.unwrap_or(self.temperature_critical_celsius),
            temperature_consecutive_samples: value.temperature_consecutive_samples.unwrap_or(self.temperature_consecutive_samples),
            service_unavailable_failures: value.service_unavailable_failures.unwrap_or(self.service_unavailable_failures),
        }
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        for (name, value) in [("cpu", self.cpu_warning_percent), ("cpu", self.cpu_critical_percent), ("memory", self.memory_warning_percent), ("memory", self.memory_critical_percent), ("disk", self.disk_warning_percent), ("disk", self.disk_critical_percent)] {
            if !(0.0..=100.0).contains(&value) { return Err(ValidationError(format!("{name} percentage must be between 0 and 100"))); }
        }
        for (name, warning, critical) in [("cpu", self.cpu_warning_percent, self.cpu_critical_percent), ("memory", self.memory_warning_percent, self.memory_critical_percent), ("disk", self.disk_warning_percent, self.disk_critical_percent), ("temperature", self.temperature_warning_celsius, self.temperature_critical_celsius)] {
            if warning >= critical { return Err(ValidationError(format!("{name} warning threshold must be lower than critical"))); }
        }
        if !(-20.0..=150.0).contains(&self.temperature_warning_celsius) || !(-20.0..=150.0).contains(&self.temperature_critical_celsius) { return Err(ValidationError("temperature thresholds must be between -20 and 150 Celsius".into())); }
        if self.cpu_duration_seconds > 86_400 || self.memory_duration_seconds > 86_400 { return Err(ValidationError("threshold duration must not exceed 86400 seconds".into())); }
        if !(1..=20).contains(&self.temperature_consecutive_samples) { return Err(ValidationError("temperature consecutive samples must be between 1 and 20".into())); }
        if !(2..=10).contains(&self.service_unavailable_failures) { return Err(ValidationError("service unavailable failures must be between 2 and 10".into())); }
        Ok(())
    }
}

impl ThresholdPolicyOverrides {
    pub fn apply_to(&self, global: &ThresholdPolicy) -> Result<ThresholdPolicy, ValidationError> {
        let effective = global.with_overrides(Some(self));
        effective.validate()?;
        Ok(effective)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
    System,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Dark
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub schema_version: u32,
    pub refresh_interval_seconds: u32,
    pub start_with_windows: bool,
    pub minimize_to_tray: bool,
    pub notifications_enabled: bool,
    pub theme: Theme,
    #[serde(default)]
    pub threshold_policy: ThresholdPolicy,
    #[serde(default)]
    pub device_threshold_overrides: HashMap<String, ThresholdPolicyOverrides>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            refresh_interval_seconds: 60,
            start_with_windows: false,
            minimize_to_tray: true,
            notifications_enabled: true,
            theme: Theme::default(),
            threshold_policy: ThresholdPolicy::default(),
            device_threshold_overrides: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct ValidationError(pub String);

impl AppSettings {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if !(MIN_REFRESH_INTERVAL_SECONDS..=MAX_REFRESH_INTERVAL_SECONDS)
            .contains(&self.refresh_interval_seconds)
        {
            return Err(ValidationError(format!(
                "refreshIntervalSeconds must be between {MIN_REFRESH_INTERVAL_SECONDS} and {MAX_REFRESH_INTERVAL_SECONDS}"
            )));
        }
        self.threshold_policy.validate()?;
        for overrides in self.device_threshold_overrides.values() {
            overrides.apply_to(&self.threshold_policy)?;
        }
        Ok(())
    }

    pub fn effective_threshold_policy(&self, device_id: &str) -> ThresholdPolicy {
        self.threshold_policy.with_overrides(self.device_threshold_overrides.get(device_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_are_valid() {
        assert!(AppSettings::default().validate().is_ok());
    }

    #[test]
    fn rejects_refresh_interval_below_minimum() {
        let mut settings = AppSettings::default();
        settings.refresh_interval_seconds = MIN_REFRESH_INTERVAL_SECONDS - 1;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn rejects_refresh_interval_above_maximum() {
        let mut settings = AppSettings::default();
        settings.refresh_interval_seconds = MAX_REFRESH_INTERVAL_SECONDS + 1;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn accepts_boundary_refresh_intervals() {
        let mut settings = AppSettings::default();
        settings.refresh_interval_seconds = MIN_REFRESH_INTERVAL_SECONDS;
        assert!(settings.validate().is_ok());
        settings.refresh_interval_seconds = MAX_REFRESH_INTERVAL_SECONDS;
        assert!(settings.validate().is_ok());
    }
}
