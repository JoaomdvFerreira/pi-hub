use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::device::Device;
use crate::storage::atomic::write_atomic;
use crate::storage::StorageError;

pub const DEVICES_SCHEMA_VERSION: u32 = 1;

pub trait DeviceRepository: Send + Sync {
    /// Loads all devices from disk. Never fails: a missing or corrupted
    /// devices.json recovers to an empty list, and any device that fails
    /// domain validation is dropped (with a warning) rather than surfaced.
    fn load_all(&self) -> Vec<Device>;
    fn get(&self, id: &str) -> Option<Device>;
    fn save_all(&self, devices: &[Device]) -> Result<(), StorageError>;
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DevicesFile {
    schema_version: u32,
    devices: Vec<Device>,
}

pub struct JsonDeviceRepository {
    devices_path: PathBuf,
}

impl JsonDeviceRepository {
    pub fn new(config_dir: impl Into<PathBuf>) -> Self {
        Self {
            devices_path: config_dir.into().join("devices.json"),
        }
    }

    fn quarantine_corrupt_file(&self) {
        let quarantine_path = self.devices_path.with_extension("json.corrupt");
        if let Err(err) = fs::rename(&self.devices_path, &quarantine_path) {
            log::warn!("failed to quarantine corrupted devices.json: {err}");
        }
    }
}

impl DeviceRepository for JsonDeviceRepository {
    fn load_all(&self) -> Vec<Device> {
        let bytes = match fs::read(&self.devices_path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
            Err(err) => {
                log::warn!(
                    "failed to read devices.json ({err}), recovering to an empty device list"
                );
                return Vec::new();
            }
        };

        match serde_json::from_slice::<DevicesFile>(&bytes) {
            Ok(file) => {
                let (valid, invalid): (Vec<_>, Vec<_>) = file
                    .devices
                    .into_iter()
                    .partition(|device| device.validate().is_ok());
                if !invalid.is_empty() {
                    log::warn!(
                        "devices.json contained {} invalid device(s); they were dropped",
                        invalid.len()
                    );
                }
                valid
            }
            Err(err) => {
                log::warn!("devices.json is corrupted ({err}), recovering to an empty device list");
                self.quarantine_corrupt_file();
                Vec::new()
            }
        }
    }

    fn get(&self, id: &str) -> Option<Device> {
        self.load_all().into_iter().find(|device| device.id == id)
    }

    fn save_all(&self, devices: &[Device]) -> Result<(), StorageError> {
        let file = DevicesFile {
            schema_version: DEVICES_SCHEMA_VERSION,
            devices: devices.to_vec(),
        };
        let bytes = serde_json::to_vec_pretty(&file)?;
        write_atomic(&self.devices_path, &bytes)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::device::DeviceType;
    use tempfile::tempdir;

    fn sample_device(id: &str) -> Device {
        Device {
            id: id.into(),
            name: "Raspberry Pi 5".into(),
            host: "raspberrypi5.tail3f2a.ts.net".into(),
            ssh_port: 22,
            ssh_username: "joao".into(),
            description: None,
            device_type: DeviceType::RaspberryPi,
            monitoring_enabled: true,
            refresh_interval_seconds: None,
            notify_on_device_offline: true,
            notify_on_container_failure: true,
            notify_on_container_unhealthy: true,
            services: Vec::new(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn load_all_returns_empty_when_missing() {
        let dir = tempdir().unwrap();
        let repo = JsonDeviceRepository::new(dir.path());
        assert_eq!(repo.load_all(), Vec::new());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempdir().unwrap();
        let repo = JsonDeviceRepository::new(dir.path());
        let devices = vec![sample_device("pi2"), sample_device("pi5")];

        repo.save_all(&devices).unwrap();

        assert_eq!(repo.load_all(), devices);
    }

    #[test]
    fn get_finds_device_by_id() {
        let dir = tempdir().unwrap();
        let repo = JsonDeviceRepository::new(dir.path());
        repo.save_all(&[sample_device("pi2"), sample_device("pi5")])
            .unwrap();

        assert_eq!(repo.get("pi5"), Some(sample_device("pi5")));
        assert_eq!(repo.get("missing"), None);
    }

    #[test]
    fn load_recovers_from_malformed_json() {
        let dir = tempdir().unwrap();
        let devices_path = dir.path().join("devices.json");
        fs::write(&devices_path, b"{ not valid json").unwrap();
        let repo = JsonDeviceRepository::new(dir.path());

        assert_eq!(repo.load_all(), Vec::new());
        assert!(
            !devices_path.exists(),
            "corrupted file should be moved aside"
        );
        assert!(dir.path().join("devices.json.corrupt").exists());
    }

    #[test]
    fn load_drops_invalid_devices_but_keeps_valid_ones() {
        let dir = tempdir().unwrap();
        let devices_path = dir.path().join("devices.json");
        let mut invalid = sample_device("bad");
        invalid.ssh_port = 0;
        let file = DevicesFile {
            schema_version: DEVICES_SCHEMA_VERSION,
            devices: vec![sample_device("good"), invalid],
        };
        fs::write(&devices_path, serde_json::to_vec(&file).unwrap()).unwrap();
        let repo = JsonDeviceRepository::new(dir.path());

        let loaded = repo.load_all();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "good");
    }
}
