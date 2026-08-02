use std::collections::HashSet;
use std::sync::Mutex;

use tokio::sync::{Semaphore, SemaphorePermit};

/// Enforces the scheduler's two concurrency rules (spec section 14.3):
/// at most `max_concurrent` device refreshes running at once, and never
/// more than one active refresh per device at a time.
pub struct RefreshCoordinator {
    semaphore: Semaphore,
    in_progress: Mutex<HashSet<String>>,
}

impl RefreshCoordinator {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Semaphore::new(max_concurrent),
            in_progress: Mutex::new(HashSet::new()),
        }
    }

    /// Attempts to claim `device_id` for a refresh. Returns `true` if this
    /// call claimed it (the caller must call `release` when done); `false`
    /// if a refresh for this device is already in progress, in which case
    /// the caller must not start a second one -- the scheduled refresh is
    /// skipped/coalesced rather than queued.
    pub fn try_claim(&self, device_id: &str) -> bool {
        let mut guard = self.in_progress.lock().expect("in_progress mutex poisoned");
        guard.insert(device_id.to_string())
    }

    pub fn release(&self, device_id: &str) {
        self.in_progress
            .lock()
            .expect("in_progress mutex poisoned")
            .remove(device_id);
    }

    pub async fn acquire_permit(&self) -> SemaphorePermit<'_> {
        self.semaphore
            .acquire()
            .await
            .expect("refresh semaphore is never closed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn try_claim_prevents_a_duplicate_in_progress_refresh() {
        let coordinator = RefreshCoordinator::new(4);
        assert!(coordinator.try_claim("pi5"));
        assert!(!coordinator.try_claim("pi5"));
        coordinator.release("pi5");
        assert!(coordinator.try_claim("pi5"));
    }

    #[test]
    fn try_claim_is_independent_per_device() {
        let coordinator = RefreshCoordinator::new(4);
        assert!(coordinator.try_claim("pi2"));
        assert!(coordinator.try_claim("pi5"));
    }

    #[tokio::test]
    async fn acquire_permit_bounds_concurrency() {
        let coordinator = RefreshCoordinator::new(2);
        let permit1 = coordinator.acquire_permit().await;
        let permit2 = coordinator.acquire_permit().await;

        let third =
            tokio::time::timeout(Duration::from_millis(50), coordinator.acquire_permit()).await;
        assert!(
            third.is_err(),
            "a third permit must not be granted while 2 permits (max_concurrent=2) are held"
        );

        drop(permit1);
        let third_after_release =
            tokio::time::timeout(Duration::from_millis(50), coordinator.acquire_permit()).await;
        assert!(
            third_after_release.is_ok(),
            "releasing one permit must free capacity for a new acquire"
        );

        drop(permit2);
    }
}
