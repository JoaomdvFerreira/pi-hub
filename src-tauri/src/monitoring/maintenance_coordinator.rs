use std::{collections::HashMap, sync::Mutex};

use crate::domain::administration::AdministrationOperationType;

/// The sole same-device maintenance claim owner for M10, M15, and M16.
/// Every currently supported kind conflicts with every other kind for the
/// same device; operations on different devices remain independent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaintenanceOperationKind {
    Administration(AdministrationOperationType),
    UpdateCheck,
    #[allow(dead_code)]
    UpdateApply,
}

pub struct DeviceMaintenanceCoordinator {
    active: Mutex<HashMap<String, MaintenanceOperationKind>>,
}

/// A claim is deliberately RAII-only. Dropping it releases the device on
/// success, early error returns, task cancellation, and panic unwinding.
pub struct DeviceMaintenanceClaim<'a> {
    coordinator: &'a DeviceMaintenanceCoordinator,
    device_id: String,
}

impl Drop for DeviceMaintenanceClaim<'_> {
    fn drop(&mut self) {
        self.coordinator.release(&self.device_id);
    }
}

impl DeviceMaintenanceCoordinator {
    pub fn new() -> Self {
        Self {
            active: Mutex::new(HashMap::new()),
        }
    }

    pub fn try_claim(
        &self,
        device_id: &str,
        kind: MaintenanceOperationKind,
    ) -> Option<DeviceMaintenanceClaim<'_>> {
        let mut active = self.lock_active();
        if active.contains_key(device_id) {
            return None;
        }
        active.insert(device_id.to_string(), kind);
        Some(DeviceMaintenanceClaim {
            coordinator: self,
            device_id: device_id.to_string(),
        })
    }

    /// Recovery uses the same claim path as a newly requested update. It is
    /// intentionally not a persistent lock: only a loaded nonterminal M16
    /// record causes a future recovery owner to claim its device.
    #[allow(dead_code)]
    pub fn try_claim_recovery(&self, device_id: &str) -> Option<DeviceMaintenanceClaim<'_>> {
        self.try_claim(device_id, MaintenanceOperationKind::UpdateApply)
    }

    fn release(&self, device_id: &str) {
        self.lock_active().remove(device_id);
    }

    fn lock_active(&self) -> std::sync::MutexGuard<'_, HashMap<String, MaintenanceOperationKind>> {
        self.active
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Default for DeviceMaintenanceCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_same_device_m10_m15_m16_combinations_conflict() {
        let coordinator = DeviceMaintenanceCoordinator::new();
        let m10 = coordinator
            .try_claim(
                "pi5",
                MaintenanceOperationKind::Administration(
                    AdministrationOperationType::RestartDevice,
                ),
            )
            .unwrap();
        assert!(coordinator
            .try_claim("pi5", MaintenanceOperationKind::UpdateCheck)
            .is_none());
        assert!(coordinator
            .try_claim("pi5", MaintenanceOperationKind::UpdateApply)
            .is_none());
        drop(m10);

        let m15 = coordinator
            .try_claim("pi5", MaintenanceOperationKind::UpdateCheck)
            .unwrap();
        assert!(coordinator
            .try_claim(
                "pi5",
                MaintenanceOperationKind::Administration(
                    AdministrationOperationType::RestartDocker
                ),
            )
            .is_none());
        assert!(coordinator
            .try_claim("pi5", MaintenanceOperationKind::UpdateApply)
            .is_none());
        drop(m15);

        let m16 = coordinator
            .try_claim("pi5", MaintenanceOperationKind::UpdateApply)
            .unwrap();
        assert!(coordinator
            .try_claim("pi5", MaintenanceOperationKind::UpdateCheck)
            .is_none());
        assert!(coordinator
            .try_claim(
                "pi5",
                MaintenanceOperationKind::Administration(
                    AdministrationOperationType::ShutdownDevice
                ),
            )
            .is_none());
        drop(m16);
    }

    #[test]
    fn claims_are_independent_per_device_and_release_on_drop() {
        let coordinator = DeviceMaintenanceCoordinator::new();
        let claim = coordinator
            .try_claim("pi2", MaintenanceOperationKind::UpdateCheck)
            .unwrap();
        assert!(coordinator
            .try_claim("pi5", MaintenanceOperationKind::UpdateApply)
            .is_some());
        drop(claim);
        assert!(coordinator.try_claim_recovery("pi2").is_some());
    }
}
