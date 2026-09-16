use crate::{
    domain::managed_workload::{ManagedWorkload, ManagedWorkloadFile},
    storage::{atomic::write_atomic, StorageError},
};
use std::{fs, path::PathBuf};

pub trait ManagedWorkloadRepository: Send + Sync {
    fn load(&self) -> ManagedWorkloadFile;
    fn save(&self, file: &ManagedWorkloadFile) -> Result<(), StorageError>;
}
pub struct JsonManagedWorkloadRepository {
    path: PathBuf,
}
impl JsonManagedWorkloadRepository {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            path: dir.into().join("managed-workloads.json"),
        }
    }
}
impl ManagedWorkloadRepository for JsonManagedWorkloadRepository {
    fn load(&self) -> ManagedWorkloadFile {
        let Ok(bytes) = fs::read(&self.path) else {
            return ManagedWorkloadFile::default();
        };
        match serde_json::from_slice::<ManagedWorkloadFile>(&bytes) {
            Ok(file) if file.validate().is_ok() => file,
            _ => ManagedWorkloadFile::default(),
        }
    }
    fn save(&self, file: &ManagedWorkloadFile) -> Result<(), StorageError> {
        file.validate().map_err(|_| {
            StorageError::Validation("managed workload configuration is invalid".into())
        })?;
        write_atomic(&self.path, &serde_json::to_vec_pretty(file)?)?;
        Ok(())
    }
}
pub fn replace_workload(
    file: &mut ManagedWorkloadFile,
    workload: ManagedWorkload,
) -> Result<(), crate::domain::managed_workload::ManagedWorkloadValidationError> {
    if let Some(index) = file
        .workloads
        .iter()
        .position(|item| item.id == workload.id)
    {
        file.workloads[index] = workload;
    } else {
        file.workloads.push(workload);
    }
    file.validate()
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::managed_workload::{ManagedWorkload, MANAGED_WORKLOAD_SCHEMA_VERSION};
    use tempfile::tempdir;
    fn sample() -> ManagedWorkload {
        ManagedWorkload {
            id: "finance".into(),
            device_id: "pi5".into(),
            name: "Personal Finance".into(),
            action_id: "personal-finance".into(),
            compose_project: Some("personal-finance".into()),
            required_compose_services: vec!["app".into()],
            required_service_health_ids: vec![],
            enabled: true,
            config_revision: 1,
        }
    }
    #[test]
    fn persists_only_typed_configuration() {
        let d = tempdir().unwrap();
        let r = JsonManagedWorkloadRepository::new(d.path());
        let f = ManagedWorkloadFile {
            schema_version: MANAGED_WORKLOAD_SCHEMA_VERSION,
            workloads: vec![sample()],
        };
        r.save(&f).unwrap();
        let raw = fs::read_to_string(d.path().join("managed-workloads.json")).unwrap();
        assert!(!raw.contains(".env"));
        assert!(!raw.contains("#!/"));
        assert_eq!(r.load(), f);
    }
    #[test]
    fn legacy_missing_file_is_empty() {
        assert_eq!(
            JsonManagedWorkloadRepository::new(tempdir().unwrap().path()).load(),
            ManagedWorkloadFile::default()
        );
    }
}
