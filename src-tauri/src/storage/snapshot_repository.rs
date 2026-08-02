use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::snapshot::DeviceSnapshot;
use crate::storage::atomic::write_atomic;
use crate::storage::StorageError;

pub const STATE_SCHEMA_VERSION: u32 = 1;

/// Persists the last-known snapshot per device to state.json. This is
/// explicitly non-authoritative runtime data (spec section 10.2): the app
/// must remain fully functional if this file is missing or deleted, so
/// every read recovers to an empty map rather than failing.
pub trait SnapshotRepository: Send + Sync {
    fn load_all(&self) -> HashMap<String, DeviceSnapshot>;
    fn get(&self, device_id: &str) -> Option<DeviceSnapshot>;
    fn upsert(&self, snapshot: &DeviceSnapshot) -> Result<(), StorageError>;
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StateFile {
    schema_version: u32,
    snapshots: HashMap<String, DeviceSnapshot>,
}

pub struct JsonSnapshotRepository {
    state_path: PathBuf,
}

impl JsonSnapshotRepository {
    pub fn new(config_dir: impl Into<PathBuf>) -> Self {
        Self {
            state_path: config_dir.into().join("state.json"),
        }
    }

    fn quarantine_corrupt_file(&self) {
        let quarantine_path = self.state_path.with_extension("json.corrupt");
        if let Err(err) = fs::rename(&self.state_path, &quarantine_path) {
            log::warn!("failed to quarantine corrupted state.json: {err}");
        }
    }
}

impl SnapshotRepository for JsonSnapshotRepository {
    fn load_all(&self) -> HashMap<String, DeviceSnapshot> {
        let bytes = match fs::read(&self.state_path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return HashMap::new(),
            Err(err) => {
                log::warn!("failed to read state.json ({err}), continuing with no last-known snapshots");
                return HashMap::new();
            }
        };

        match serde_json::from_slice::<StateFile>(&bytes) {
            Ok(file) => file.snapshots,
            Err(err) => {
                log::warn!("state.json is corrupted ({err}), continuing with no last-known snapshots");
                self.quarantine_corrupt_file();
                HashMap::new()
            }
        }
    }

    fn get(&self, device_id: &str) -> Option<DeviceSnapshot> {
        self.load_all().remove(device_id)
    }

    fn upsert(&self, snapshot: &DeviceSnapshot) -> Result<(), StorageError> {
        let mut snapshots = self.load_all();
        snapshots.insert(snapshot.device_id.clone(), snapshot.clone());
        let file = StateFile {
            schema_version: STATE_SCHEMA_VERSION,
            snapshots,
        };
        let bytes = serde_json::to_vec_pretty(&file)?;
        write_atomic(&self.state_path, &bytes)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::connection_status::DeviceConnectionStatus;
    use tempfile::tempdir;

    fn sample_snapshot(device_id: &str) -> DeviceSnapshot {
        DeviceSnapshot {
            device_id: device_id.into(),
            connection_status: DeviceConnectionStatus::Online,
            captured_at: "2026-01-01T00:00:00Z".into(),
            duration_ms: 120,
            metrics: None,
            docker_available: false,
            containers: Vec::new(),
            warnings: Vec::new(),
            error: None,
            stale: false,
            last_successful_refresh: Some("2026-01-01T00:00:00Z".into()),
        }
    }

    #[test]
    fn load_all_returns_empty_map_when_missing() {
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        assert!(repo.load_all().is_empty());
    }

    #[test]
    fn app_remains_functional_after_state_json_is_deleted() {
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        repo.upsert(&sample_snapshot("pi5")).unwrap();
        assert!(repo.get("pi5").is_some());

        fs::remove_file(dir.path().join("state.json")).unwrap();

        assert_eq!(repo.get("pi5"), None);
        // A subsequent write still works fine after the file was deleted.
        repo.upsert(&sample_snapshot("pi5")).unwrap();
        assert!(repo.get("pi5").is_some());
    }

    #[test]
    fn upsert_then_get_round_trips() {
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let snapshot = sample_snapshot("pi5");

        repo.upsert(&snapshot).unwrap();

        assert_eq!(repo.get("pi5"), Some(snapshot));
        assert_eq!(repo.get("missing"), None);
    }

    #[test]
    fn upsert_preserves_other_devices_snapshots() {
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        repo.upsert(&sample_snapshot("pi2")).unwrap();
        repo.upsert(&sample_snapshot("pi5")).unwrap();

        let all = repo.load_all();

        assert_eq!(all.len(), 2);
    }

    #[test]
    fn load_recovers_from_malformed_json() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("state.json");
        fs::write(&state_path, b"{ not valid json").unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());

        assert!(repo.load_all().is_empty());
        assert!(!state_path.exists());
        assert!(dir.path().join("state.json.corrupt").exists());
    }
}
