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

/// Releases its device's claim when dropped -- including on an early
/// return or a panic unwinding through the holder's scope, not only on a
/// normal return. A previous version required the caller to remember to
/// call `release(device_id)` manually after the refresh; a panic partway
/// through a refresh (nothing currently does this deliberately, but
/// nothing ruled it out either) would have skipped that call and left the
/// device claimed forever, silently freezing its monitoring until the app
/// restarted.
pub struct RefreshClaim<'a> {
    coordinator: &'a RefreshCoordinator,
    device_id: String,
}

impl Drop for RefreshClaim<'_> {
    fn drop(&mut self) {
        self.coordinator.release(&self.device_id);
    }
}

impl RefreshCoordinator {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Semaphore::new(max_concurrent),
            in_progress: Mutex::new(HashSet::new()),
        }
    }

    /// Attempts to claim `device_id` for a refresh. Returns `Some(claim)`
    /// if this call claimed it -- the claim releases itself when dropped,
    /// there is no separate release call -- or `None` if a refresh for
    /// this device is already in progress, in which case the caller must
    /// not start a second one -- the scheduled refresh is skipped/
    /// coalesced rather than queued.
    pub fn try_claim(&self, device_id: &str) -> Option<RefreshClaim<'_>> {
        let mut guard = self.lock_in_progress();
        if guard.insert(device_id.to_string()) {
            Some(RefreshClaim {
                coordinator: self,
                device_id: device_id.to_string(),
            })
        } else {
            None
        }
    }

    fn release(&self, device_id: &str) {
        self.lock_in_progress().remove(device_id);
    }

    /// See `platform::pty::PtySessionManager::lock_sessions` for why a
    /// poisoned lock is recovered here rather than propagated: a panic
    /// elsewhere doesn't make this `HashSet` unsafe to keep using, and
    /// propagating the poison would permanently disable monitoring for
    /// every device (not just the one involved in the original panic) for
    /// the rest of the app's lifetime.
    fn lock_in_progress(&self) -> std::sync::MutexGuard<'_, HashSet<String>> {
        self.in_progress.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
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
        let claim = coordinator.try_claim("pi5");
        assert!(claim.is_some());
        assert!(coordinator.try_claim("pi5").is_none());
        drop(claim);
        assert!(coordinator.try_claim("pi5").is_some());
    }

    #[test]
    fn try_claim_is_independent_per_device() {
        let coordinator = RefreshCoordinator::new(4);
        assert!(coordinator.try_claim("pi2").is_some());
        assert!(coordinator.try_claim("pi5").is_some());
    }

    #[test]
    fn dropping_the_claim_releases_it_even_without_an_explicit_release_call() {
        let coordinator = RefreshCoordinator::new(4);
        {
            let _claim = coordinator.try_claim("pi5").unwrap();
            assert!(coordinator.try_claim("pi5").is_none());
        }
        // _claim went out of scope (an early return or a panic unwinding
        // through it would do the same) -- the device must be released.
        assert!(coordinator.try_claim("pi5").is_some());
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
