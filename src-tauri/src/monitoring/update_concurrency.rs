use std::{collections::HashSet, sync::Mutex};

pub struct UpdateCheckCoordinator {
    active: Mutex<HashSet<String>>,
}
pub struct UpdateCheckClaim<'a> {
    coordinator: &'a UpdateCheckCoordinator,
    device_id: String,
}
impl Drop for UpdateCheckClaim<'_> {
    fn drop(&mut self) {
        self.coordinator
            .active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&self.device_id);
    }
}
impl UpdateCheckCoordinator {
    pub fn new() -> Self {
        Self {
            active: Mutex::new(HashSet::new()),
        }
    }
    pub fn try_claim(&self, device_id: &str) -> Option<UpdateCheckClaim<'_>> {
        let mut active = self.active.lock().unwrap_or_else(|e| e.into_inner());
        active
            .insert(device_id.to_string())
            .then(|| UpdateCheckClaim {
                coordinator: self,
                device_id: device_id.into(),
            })
    }
}
impl Default for UpdateCheckCoordinator {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn same_device_is_coalesced_but_devices_are_independent() {
        let c = UpdateCheckCoordinator::new();
        let a = c.try_claim("a").unwrap();
        assert!(c.try_claim("a").is_none());
        assert!(c.try_claim("b").is_some());
        drop(a);
        assert!(c.try_claim("a").is_some());
    }
}
