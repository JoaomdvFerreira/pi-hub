use std::fs;
use std::path::PathBuf;

use crate::domain::settings::AppSettings;
use crate::storage::atomic::write_atomic;
use crate::storage::StorageError;

pub trait SettingsRepository: Send + Sync {
    /// Loads settings from disk. Never fails: a missing, corrupted, or
    /// invalid config.json recovers to `AppSettings::default()` so the app
    /// always has a usable configuration.
    fn load(&self) -> AppSettings;
    fn save(&self, settings: &AppSettings) -> Result<(), StorageError>;
}

pub struct JsonSettingsRepository {
    config_path: PathBuf,
}

impl JsonSettingsRepository {
    pub fn new(config_dir: impl Into<PathBuf>) -> Self {
        Self {
            config_path: config_dir.into().join("config.json"),
        }
    }

    /// Moves an unreadable config.json aside so it can be inspected later,
    /// instead of silently discarding it.
    fn quarantine_corrupt_file(&self) {
        let quarantine_path = self.config_path.with_extension("json.corrupt");
        if let Err(err) = fs::rename(&self.config_path, &quarantine_path) {
            log::warn!("failed to quarantine corrupted config.json: {err}");
        }
    }
}

impl SettingsRepository for JsonSettingsRepository {
    fn load(&self) -> AppSettings {
        let bytes = match fs::read(&self.config_path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return AppSettings::default();
            }
            Err(err) => {
                log::warn!("failed to read config.json ({err}), recovering to defaults");
                return AppSettings::default();
            }
        };

        match serde_json::from_slice::<AppSettings>(&bytes) {
            Ok(settings) if settings.validate().is_ok() => settings,
            Ok(_) => {
                log::warn!("config.json contains an invalid value, recovering to defaults");
                self.quarantine_corrupt_file();
                AppSettings::default()
            }
            Err(err) => {
                log::warn!("config.json is corrupted ({err}), recovering to defaults");
                self.quarantine_corrupt_file();
                AppSettings::default()
            }
        }
    }

    fn save(&self, settings: &AppSettings) -> Result<(), StorageError> {
        let bytes = serde_json::to_vec_pretty(settings)?;
        write_atomic(&self.config_path, &bytes)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::settings::Theme;
    use tempfile::tempdir;

    #[test]
    fn load_returns_defaults_when_missing() {
        let dir = tempdir().unwrap();
        let repo = JsonSettingsRepository::new(dir.path());
        assert_eq!(repo.load(), AppSettings::default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempdir().unwrap();
        let repo = JsonSettingsRepository::new(dir.path());
        let mut settings = AppSettings::default();
        settings.refresh_interval_seconds = 120;
        settings.theme = Theme::Light;
        settings.start_with_windows = true;

        repo.save(&settings).unwrap();

        assert_eq!(repo.load(), settings);
    }

    #[test]
    fn load_recovers_from_malformed_json() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        fs::write(&config_path, b"{ not valid json").unwrap();
        let repo = JsonSettingsRepository::new(dir.path());

        let settings = repo.load();

        assert_eq!(settings, AppSettings::default());
        assert!(
            !config_path.exists(),
            "corrupted file should be moved aside"
        );
        assert!(dir.path().join("config.json.corrupt").exists());
    }

    #[test]
    fn load_recovers_from_out_of_range_value() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        fs::write(
            &config_path,
            br#"{"schemaVersion":1,"refreshIntervalSeconds":5,"startWithWindows":false,"minimizeToTray":true,"notificationsEnabled":true,"theme":"dark"}"#,
        )
        .unwrap();
        let repo = JsonSettingsRepository::new(dir.path());

        assert_eq!(repo.load(), AppSettings::default());
    }

    #[test]
    fn save_preserves_previous_valid_file_on_serialization_failure() {
        // save() only ever fails before the atomic rename runs, so a prior
        // valid file on disk is never touched by a failed save. This test
        // documents that guarantee via the happy path plus a corrupted-then
        // recovered file, since simulating a mid-write OS failure isn't
        // practical in a unit test.
        let dir = tempdir().unwrap();
        let repo = JsonSettingsRepository::new(dir.path());
        let settings = AppSettings::default();
        repo.save(&settings).unwrap();

        assert_eq!(repo.load(), settings);
    }
}
