use crate::{
    domain::administration::ExpectedDisruption,
    storage::{atomic::write_atomic, StorageError},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdministrationFile {
    #[serde(default)]
    expected_disruptions: HashMap<String, ExpectedDisruption>,
}

pub trait AdministrationRepository: Send + Sync {
    fn get_valid(&self, device_id: &str) -> Option<ExpectedDisruption>;
    fn save(&self, disruption: ExpectedDisruption) -> Result<(), StorageError>;
    fn clear(&self, device_id: &str) -> Result<(), StorageError>;
    #[allow(dead_code)]
    fn clear_if_operation(&self, device_id: &str, operation_id: &str) -> Result<(), StorageError>;
}
pub struct JsonAdministrationRepository {
    path: PathBuf,
}
impl JsonAdministrationRepository {
    pub fn new(config_dir: impl Into<PathBuf>) -> Self {
        Self {
            path: config_dir.into().join("administration.json"),
        }
    }
    fn load(&self) -> AdministrationFile {
        match fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|err| {
                log::warn!("administration state is corrupted: {err}");
                AdministrationFile::default()
            }),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => AdministrationFile::default(),
            Err(err) => {
                log::warn!("could not read administration state: {err}");
                AdministrationFile::default()
            }
        }
    }
    fn save_file(&self, mut file: AdministrationFile) -> Result<(), StorageError> {
        file.expected_disruptions
            .retain(|_, marker| marker.is_valid_at(Utc::now()));
        write_atomic(&self.path, &serde_json::to_vec_pretty(&file)?)?;
        Ok(())
    }
}
impl AdministrationRepository for JsonAdministrationRepository {
    fn get_valid(&self, device_id: &str) -> Option<ExpectedDisruption> {
        let mut file = self.load();
        let result = file
            .expected_disruptions
            .get(device_id)
            .filter(|marker| marker.is_valid_at(Utc::now()))
            .cloned();
        if result.is_none() && file.expected_disruptions.remove(device_id).is_some() {
            let _ = self.save_file(file);
        }
        result
    }
    fn save(&self, disruption: ExpectedDisruption) -> Result<(), StorageError> {
        let mut file = self.load();
        file.expected_disruptions
            .insert(disruption.device_id.clone(), disruption);
        self.save_file(file)
    }
    fn clear(&self, device_id: &str) -> Result<(), StorageError> {
        let mut file = self.load();
        file.expected_disruptions.remove(device_id);
        self.save_file(file)
    }
    fn clear_if_operation(&self, device_id: &str, operation_id: &str) -> Result<(), StorageError> {
        let mut file = self.load();
        let matches_operation = file
            .expected_disruptions
            .get(device_id)
            .is_some_and(|marker| marker.operation_id == operation_id);
        if matches_operation {
            file.expected_disruptions.remove(device_id);
            self.save_file(file)?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::administration::{AdministrationOperation, AdministrationOperationType};
    use tempfile::tempdir;
    #[test]
    fn valid_marker_survives_repository_recreation_and_expired_marker_is_removed() {
        let dir = tempdir().unwrap();
        let repo = JsonAdministrationRepository::new(dir.path());
        let op = AdministrationOperation::requested(
            "d".into(),
            AdministrationOperationType::ShutdownDevice,
        );
        repo.save(ExpectedDisruption::new(&op)).unwrap();
        assert!(repo.get_valid("d").is_some());
        let mut stale = ExpectedDisruption::new(&op);
        stale.device_id = "stale".into();
        stale.expected_until = "2000-01-01T00:00:00Z".into();
        repo.save(stale).unwrap();
        assert!(JsonAdministrationRepository::new(dir.path())
            .get_valid("stale")
            .is_none());
    }

    #[test]
    fn operation_scoped_cleanup_cannot_remove_another_operations_marker() {
        let dir = tempdir().unwrap();
        let repo = JsonAdministrationRepository::new(dir.path());
        let op = AdministrationOperation::requested(
            "d".into(),
            AdministrationOperationType::RestartDevice,
        );
        let marker = ExpectedDisruption::new(&op);
        repo.save(marker).unwrap();
        repo.clear_if_operation("d", "not-the-owner").unwrap();
        assert!(repo.get_valid("d").is_some());
        repo.clear_if_operation("d", &op.id).unwrap();
        assert!(repo.get_valid("d").is_none());
    }

    #[test]
    fn expired_m16_expected_disruption_is_pruned_on_persistence_and_reload() {
        use crate::domain::{
            administration::ExpectedDisruption,
            maintenance::{MaintenanceDispatchState, MaintenanceOperation, MaintenanceOperationState},
        };
        let dir = tempdir().unwrap();
        let repo = JsonAdministrationRepository::new(dir.path());
        let mut operation = MaintenanceOperation::requested("pi5".into());
        operation.dispatch_state = MaintenanceDispatchState::Accepted;
        operation.state = MaintenanceOperationState::Installing;
        operation.observation_deadline = Some("2000-01-01T00:00:00Z".into());
        let marker = ExpectedDisruption::for_update(&operation).unwrap();
        repo.save(marker).unwrap();
        assert!(JsonAdministrationRepository::new(dir.path())
            .get_valid("pi5")
            .is_none());
    }
}
