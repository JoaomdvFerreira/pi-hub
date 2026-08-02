use serde::{Deserialize, Serialize};

pub const CONFIG_SCHEMA_VERSION: u32 = 1;
pub const MIN_REFRESH_INTERVAL_SECONDS: u32 = 15;
pub const MAX_REFRESH_INTERVAL_SECONDS: u32 = 3600;

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
        Ok(())
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
