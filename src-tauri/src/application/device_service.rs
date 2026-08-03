use serde::Deserialize;

use crate::domain::device::{
    Device, DeviceService as DeviceServiceEntry, DeviceType, DeviceValidationError,
};
use crate::storage::device_repository::DeviceRepository;
use crate::storage::StorageError;

/// The editable fields of a device, as submitted from the frontend. Shared
/// by create and update: id/createdAt/updatedAt are always assigned or
/// preserved by this module, never taken from client input.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInput {
    pub name: String,
    pub host: String,
    pub ssh_port: u16,
    pub ssh_username: String,
    pub description: Option<String>,
    pub device_type: DeviceType,
    pub monitoring_enabled: bool,
    pub refresh_interval_seconds: Option<u32>,
    pub notify_on_device_offline: bool,
    pub notify_on_container_failure: bool,
    pub notify_on_container_unhealthy: bool,
    pub services: Vec<DeviceServiceEntry>,
}

#[derive(Debug)]
pub enum DeviceServiceError {
    Validation(DeviceValidationError),
    NotFound(String),
    Storage(StorageError),
}

impl From<DeviceValidationError> for DeviceServiceError {
    fn from(err: DeviceValidationError) -> Self {
        DeviceServiceError::Validation(err)
    }
}

impl From<StorageError> for DeviceServiceError {
    fn from(err: StorageError) -> Self {
        DeviceServiceError::Storage(err)
    }
}

pub fn create_device(
    repository: &dyn DeviceRepository,
    input: DeviceInput,
) -> Result<Device, DeviceServiceError> {
    let now = chrono::Utc::now().to_rfc3339();
    let device = Device {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        host: input.host,
        ssh_port: input.ssh_port,
        ssh_username: input.ssh_username,
        description: input.description,
        device_type: input.device_type,
        monitoring_enabled: input.monitoring_enabled,
        refresh_interval_seconds: input.refresh_interval_seconds,
        notify_on_device_offline: input.notify_on_device_offline,
        notify_on_container_failure: input.notify_on_container_failure,
        notify_on_container_unhealthy: input.notify_on_container_unhealthy,
        services: input.services,
        created_at: now.clone(),
        updated_at: now,
    };
    device.validate()?;

    let mut devices = repository.load_all();
    devices.push(device.clone());
    repository.save_all(&devices)?;
    Ok(device)
}

pub fn update_device(
    repository: &dyn DeviceRepository,
    id: &str,
    input: DeviceInput,
) -> Result<Device, DeviceServiceError> {
    let mut devices = repository.load_all();
    let index = devices
        .iter()
        .position(|device| device.id == id)
        .ok_or_else(|| DeviceServiceError::NotFound(id.to_string()))?;

    let updated = Device {
        id: devices[index].id.clone(),
        name: input.name,
        host: input.host,
        ssh_port: input.ssh_port,
        ssh_username: input.ssh_username,
        description: input.description,
        device_type: input.device_type,
        monitoring_enabled: input.monitoring_enabled,
        refresh_interval_seconds: input.refresh_interval_seconds,
        notify_on_device_offline: input.notify_on_device_offline,
        notify_on_container_failure: input.notify_on_container_failure,
        notify_on_container_unhealthy: input.notify_on_container_unhealthy,
        services: input.services,
        created_at: devices[index].created_at.clone(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    updated.validate()?;

    devices[index] = updated.clone();
    repository.save_all(&devices)?;
    Ok(updated)
}

pub fn delete_device(
    repository: &dyn DeviceRepository,
    id: &str,
) -> Result<(), DeviceServiceError> {
    let mut devices = repository.load_all();
    let original_len = devices.len();
    devices.retain(|device| device.id != id);
    if devices.len() == original_len {
        return Err(DeviceServiceError::NotFound(id.to_string()));
    }
    repository.save_all(&devices)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::device_repository::JsonDeviceRepository;
    use tempfile::tempdir;

    fn sample_input() -> DeviceInput {
        DeviceInput {
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
        }
    }

    #[test]
    fn create_persists_a_valid_device_with_generated_id_and_timestamps() {
        let dir = tempdir().unwrap();
        let repo = JsonDeviceRepository::new(dir.path());

        let device = create_device(&repo, sample_input()).unwrap();

        assert!(!device.id.is_empty());
        assert_eq!(device.created_at, device.updated_at);
        assert_eq!(repo.load_all(), vec![device]);
    }

    #[test]
    fn create_rejects_invalid_input_without_persisting() {
        let dir = tempdir().unwrap();
        let repo = JsonDeviceRepository::new(dir.path());
        let mut input = sample_input();
        input.ssh_port = 0;

        let result = create_device(&repo, input);

        assert!(matches!(result, Err(DeviceServiceError::Validation(_))));
        assert!(repo.load_all().is_empty());
    }

    #[test]
    fn update_preserves_id_and_created_at_but_bumps_updated_at() {
        let dir = tempdir().unwrap();
        let repo = JsonDeviceRepository::new(dir.path());
        let created = create_device(&repo, sample_input()).unwrap();

        let mut input = sample_input();
        input.name = "Renamed".into();
        let updated = update_device(&repo, &created.id, input).unwrap();

        assert_eq!(updated.id, created.id);
        assert_eq!(updated.created_at, created.created_at);
        assert_eq!(updated.name, "Renamed");
        assert_eq!(repo.load_all(), vec![updated]);
    }

    #[test]
    fn update_missing_device_returns_not_found() {
        let dir = tempdir().unwrap();
        let repo = JsonDeviceRepository::new(dir.path());

        let result = update_device(&repo, "missing", sample_input());

        assert!(matches!(result, Err(DeviceServiceError::NotFound(_))));
    }

    #[test]
    fn update_rejects_invalid_input_leaving_original_untouched() {
        let dir = tempdir().unwrap();
        let repo = JsonDeviceRepository::new(dir.path());
        let created = create_device(&repo, sample_input()).unwrap();

        let mut input = sample_input();
        input.ssh_username = "Invalid Name".into();
        let result = update_device(&repo, &created.id, input);

        assert!(matches!(result, Err(DeviceServiceError::Validation(_))));
        assert_eq!(repo.load_all(), vec![created]);
    }

    #[test]
    fn delete_removes_the_device() {
        let dir = tempdir().unwrap();
        let repo = JsonDeviceRepository::new(dir.path());
        let created = create_device(&repo, sample_input()).unwrap();

        delete_device(&repo, &created.id).unwrap();

        assert!(repo.load_all().is_empty());
    }

    #[test]
    fn delete_missing_device_returns_not_found() {
        let dir = tempdir().unwrap();
        let repo = JsonDeviceRepository::new(dir.path());

        let result = delete_device(&repo, "missing");

        assert!(matches!(result, Err(DeviceServiceError::NotFound(_))));
    }
}
