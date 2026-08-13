use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::maintenance::MaintenanceOperation;
use crate::domain::snapshot::DeviceSnapshot;
use crate::domain::update_intelligence::UpdateCheckResult;
use crate::storage::atomic::write_atomic;
use crate::storage::StorageError;

pub const STATE_SCHEMA_VERSION: u32 = 1;

/// Persists the last-known snapshot per device, and the set of already-
/// notified transition dedup keys, to state.json. This is explicitly
/// non-authoritative runtime data (spec section 10.2): the app must
/// remain fully functional if this file is missing or deleted, so every
/// read recovers to an empty state rather than failing.
///
/// Both concerns share one repository (and one file, read-modify-written
/// as a single unit) rather than two independent ones, specifically so a
/// snapshot write can never clobber a dedup-key write or vice versa.
pub trait SnapshotRepository: Send + Sync {
    // Kept for API completeness (e.g. a future "all last-known snapshots"
    // dashboard query) and exercised directly by tests; no production
    // caller needs the whole map yet since refresh/tick work per-device.
    #[allow(dead_code)]
    fn load_all(&self) -> HashMap<String, DeviceSnapshot>;
    fn get(&self, device_id: &str) -> Option<DeviceSnapshot>;
    fn upsert(&self, snapshot: &DeviceSnapshot) -> Result<(), StorageError>;

    /// Whether a notification dedup key has already been recorded (spec
    /// section 15.3: `deviceId + resourceId + previousState +
    /// currentState`), persisted so dedup survives an app restart.
    fn has_notified(&self, dedup_key: &str) -> bool;
    fn mark_notified(&self, dedup_key: &str) -> Result<(), StorageError>;
    fn get_update_result(&self, device_id: &str) -> Option<UpdateCheckResult>;
    fn upsert_update_result(&self, result: &UpdateCheckResult) -> Result<(), StorageError>;
    fn get_maintenance_operation(&self, device_id: &str) -> Option<MaintenanceOperation>;
    #[allow(dead_code)]
    fn load_maintenance_operations(&self) -> HashMap<String, MaintenanceOperation>;
    #[allow(dead_code)]
    fn upsert_maintenance_operation(
        &self,
        operation: &MaintenanceOperation,
    ) -> Result<(), StorageError>;
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StateFile {
    schema_version: u32,
    #[serde(default)]
    snapshots: HashMap<String, DeviceSnapshot>,
    #[serde(default)]
    notified_transitions: HashSet<String>,
    #[serde(default)]
    update_results: HashMap<String, UpdateCheckResult>,
    /// M16 adds only the most recent operation per device. Older state.json
    /// files omit this field and deserialize to an empty map.
    #[serde(default)]
    maintenance_operations: HashMap<String, MaintenanceOperation>,
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

    fn load_state(&self) -> StateFile {
        let bytes = match fs::read(&self.state_path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return StateFile::default(),
            Err(err) => {
                log::warn!(
                    "failed to read state.json ({err}), continuing with no last-known state"
                );
                return StateFile::default();
            }
        };

        match serde_json::from_slice::<StateFile>(&bytes) {
            Ok(file) => file,
            Err(err) => {
                log::warn!("state.json is corrupted ({err}), continuing with no last-known state");
                self.quarantine_corrupt_file();
                StateFile::default()
            }
        }
    }

    fn save_state(&self, state: &StateFile) -> Result<(), StorageError> {
        let bytes = serde_json::to_vec_pretty(state)?;
        write_atomic(&self.state_path, &bytes)?;
        Ok(())
    }
}

impl SnapshotRepository for JsonSnapshotRepository {
    fn load_all(&self) -> HashMap<String, DeviceSnapshot> {
        self.load_state().snapshots
    }

    fn get(&self, device_id: &str) -> Option<DeviceSnapshot> {
        self.load_state().snapshots.remove(device_id)
    }

    fn upsert(&self, snapshot: &DeviceSnapshot) -> Result<(), StorageError> {
        let mut state = self.load_state();
        state.schema_version = STATE_SCHEMA_VERSION;
        state
            .snapshots
            .insert(snapshot.device_id.clone(), snapshot.clone());
        self.save_state(&state)
    }

    fn has_notified(&self, dedup_key: &str) -> bool {
        self.load_state().notified_transitions.contains(dedup_key)
    }

    fn mark_notified(&self, dedup_key: &str) -> Result<(), StorageError> {
        let mut state = self.load_state();
        state.schema_version = STATE_SCHEMA_VERSION;
        state.notified_transitions.insert(dedup_key.to_string());
        self.save_state(&state)
    }
    fn get_update_result(&self, device_id: &str) -> Option<UpdateCheckResult> {
        self.load_state().update_results.remove(device_id)
    }
    fn upsert_update_result(&self, result: &UpdateCheckResult) -> Result<(), StorageError> {
        let mut state = self.load_state();
        state.schema_version = STATE_SCHEMA_VERSION;
        state
            .update_results
            .insert(result.device_id.clone(), result.clone());
        self.save_state(&state)
    }
    fn get_maintenance_operation(&self, device_id: &str) -> Option<MaintenanceOperation> {
        self.load_state().maintenance_operations.remove(device_id)
    }
    fn load_maintenance_operations(&self) -> HashMap<String, MaintenanceOperation> {
        self.load_state().maintenance_operations
    }
    fn upsert_maintenance_operation(
        &self,
        operation: &MaintenanceOperation,
    ) -> Result<(), StorageError> {
        let mut state = self.load_state();
        state.schema_version = STATE_SCHEMA_VERSION;
        state
            .maintenance_operations
            .insert(operation.device_id.clone(), operation.clone());
        self.save_state(&state)
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
            health: crate::domain::health::assess_health(&DeviceConnectionStatus::Online, None),
            service_health: HashMap::new(),
            network_visibility: None,
            storage_visibility: None,
            system_visibility: None,
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

    #[test]
    fn mark_notified_then_has_notified_round_trips() {
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());

        assert!(!repo.has_notified("pi5|pi5|online|offline"));
        repo.mark_notified("pi5|pi5|online|offline").unwrap();
        assert!(repo.has_notified("pi5|pi5|online|offline"));
        assert!(!repo.has_notified("pi5|pi5|offline|online"));
    }

    #[test]
    fn snapshot_upsert_and_notification_dedup_do_not_clobber_each_other() {
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());

        repo.upsert(&sample_snapshot("pi5")).unwrap();
        repo.mark_notified("pi5|pi5|online|offline").unwrap();
        repo.upsert(&sample_snapshot("pi2")).unwrap();
        repo.mark_notified("pi2|pi2|online|offline").unwrap();

        assert_eq!(repo.load_all().len(), 2);
        assert!(repo.has_notified("pi5|pi5|online|offline"));
        assert!(repo.has_notified("pi2|pi2|online|offline"));
    }

    #[test]
    fn dedup_state_survives_being_loaded_fresh_simulating_a_restart() {
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        repo.mark_notified("pi5|pi5|online|offline").unwrap();
        drop(repo);

        // A brand new repository instance pointed at the same directory
        // simulates the app restarting.
        let repo_after_restart = JsonSnapshotRepository::new(dir.path());
        assert!(repo_after_restart.has_notified("pi5|pi5|online|offline"));
    }

    #[test]
    fn m16_latest_operations_round_trip_all_recovery_relevant_lifecycle_states() {
        use crate::domain::maintenance::{
            MaintenanceDispatchState, MaintenanceOperation, MaintenanceOperationState,
        };
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let states = [
            MaintenanceOperationState::Requested,
            MaintenanceOperationState::Preflight,
            MaintenanceOperationState::RefreshingMetadata,
            MaintenanceOperationState::VerifyingPlan,
            MaintenanceOperationState::PreDispatchFailed,
            MaintenanceOperationState::Dispatching,
            MaintenanceOperationState::Installing,
            MaintenanceOperationState::Verifying,
            MaintenanceOperationState::StillRunning,
            MaintenanceOperationState::OutcomeUncertain,
            MaintenanceOperationState::PlanChanged,
            MaintenanceOperationState::Completed,
            MaintenanceOperationState::CompletedRebootRequired,
            MaintenanceOperationState::PackageManagerBusy,
            MaintenanceOperationState::Failed,
        ];
        for (index, state) in states.into_iter().enumerate() {
            let mut operation = MaintenanceOperation::requested(format!("pi{index}"));
            operation.state = state;
            operation.dispatch_state = match state {
                MaintenanceOperationState::Dispatching => MaintenanceDispatchState::Uncertain,
                MaintenanceOperationState::PreDispatchFailed => {
                    MaintenanceDispatchState::NotAttempted
                }
                _ => MaintenanceDispatchState::Accepted,
            };
            repo.upsert_maintenance_operation(&operation).unwrap();
        }
        let after_restart = JsonSnapshotRepository::new(dir.path());
        assert_eq!(
            after_restart.load_maintenance_operations().len(),
            states.len()
        );
        assert_eq!(
            after_restart
                .get_maintenance_operation("pi5")
                .unwrap()
                .dispatch_state,
            MaintenanceDispatchState::Uncertain
        );
        assert!(after_restart
            .get_maintenance_operation("pi7")
            .unwrap()
            .requires_recovery());
        assert!(after_restart
            .get_maintenance_operation("pi4")
            .unwrap()
            .requires_fresh_confirmation_after_restart());
        assert!(!after_restart
            .get_maintenance_operation("pi10")
            .unwrap()
            .requires_recovery());
    }

    #[test]
    fn legacy_pre_m16_state_loads_without_losing_existing_data() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("state.json"),
            br#"{"schemaVersion":1,"snapshots":{},"notifiedTransitions":["a"],"updateResults":{}}"#,
        )
        .unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        assert!(repo.load_maintenance_operations().is_empty());
        assert!(repo.has_notified("a"));
    }

    #[test]
    fn maintenance_state_is_bounded_and_never_serializes_raw_remote_output() {
        use crate::domain::maintenance::MaintenanceOperation;
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let operation = MaintenanceOperation::requested("pi5".into());
        repo.upsert_maintenance_operation(&operation).unwrap();
        let serialized = fs::read_to_string(dir.path().join("state.json")).unwrap();
        assert!(serialized.contains("maintenanceOperations"));
        for forbidden in ["stdout", "stderr", "journal", "apt-get", "password"] {
            assert!(
                !serialized.contains(forbidden),
                "unexpected raw field: {forbidden}"
            );
        }
        let mut newer = MaintenanceOperation::requested("pi5".into());
        newer.id = "replacement".into();
        repo.upsert_maintenance_operation(&newer).unwrap();
        assert_eq!(repo.load_maintenance_operations().len(), 1);
        assert_eq!(
            repo.get_maintenance_operation("pi5").unwrap().id,
            "replacement"
        );
    }
}
