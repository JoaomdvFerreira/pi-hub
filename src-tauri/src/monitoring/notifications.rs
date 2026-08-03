use crate::domain::device::Device;
use crate::domain::notification_rule::{
    container_health_transition_event, container_state_transition_event, device_transition_event,
    NotificationEvent,
};
use crate::domain::snapshot::DeviceSnapshot;
use crate::storage::snapshot_repository::SnapshotRepository;

/// Dispatches a notification-ready event as a native notification. See
/// `platform::notifications::TauriNotificationService` for the concrete
/// implementation used at runtime.
pub trait NotificationService: Send + Sync {
    fn notify(&self, event: &NotificationEvent);
}

/// Compares two consecutive snapshots for a device, evaluates every
/// device- and container-level transition rule the device's own per-
/// category preference allows (Phase 2: replaces a single per-device
/// on/off switch with independent gates for offline transitions,
/// container failures, and containers becoming unhealthy), and returns
/// only the events that haven't already been notified (per the dedup key
/// persisted in state.json, so this is restart-safe). As a side effect,
/// every returned event's dedup key is recorded so it is never returned
/// again for the same transition. A category that's turned off is never
/// even evaluated into a candidate, so its dedup key is never spent --
/// re-enabling it later does not retroactively fire for a transition
/// that already happened while it was off, and does not consume a "slot"
/// that would suppress a genuine future transition.
pub fn evaluate_snapshot_transition(
    repo: &dyn SnapshotRepository,
    device: &Device,
    previous: Option<&DeviceSnapshot>,
    current: &DeviceSnapshot,
) -> Vec<NotificationEvent> {
    let device_id = device.id.as_str();
    let mut candidates = Vec::new();

    if device.notify_on_device_offline {
        if let Some(event) = device_transition_event(
            device_id,
            previous.map(|p| p.connection_status),
            current.connection_status,
        ) {
            candidates.push(event);
        }
    }

    let previous_containers = previous.map(|p| p.containers.as_slice()).unwrap_or(&[]);
    for container in &current.containers {
        let prev = previous_containers.iter().find(|p| p.id == container.id);

        if device.notify_on_container_failure {
            if let Some(event) = container_state_transition_event(
                device_id,
                &container.id,
                &container.name,
                prev.map(|p| p.state),
                container.state,
            ) {
                candidates.push(event);
            }
        }

        if device.notify_on_container_unhealthy {
            if let Some(event) = container_health_transition_event(
                device_id,
                &container.id,
                &container.name,
                prev.map(|p| p.health),
                container.health,
            ) {
                candidates.push(event);
            }
        }
    }

    let mut events = Vec::new();
    for event in candidates {
        let key = event.dedup_key();
        if !repo.has_notified(&key) {
            if let Err(err) = repo.mark_notified(&key) {
                log::warn!("failed to persist notification dedup key ({err}), notifying anyway");
            }
            events.push(event);
        }
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::connection_status::DeviceConnectionStatus;
    use crate::domain::device::DeviceType;
    use crate::domain::docker_container::{
        DockerContainerState, DockerContainerSummary, DockerHealthStatus,
    };
    use crate::storage::snapshot_repository::JsonSnapshotRepository;
    use tempfile::tempdir;

    fn sample_device() -> Device {
        Device {
            id: "pi5".into(),
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

    fn snapshot(
        status: DeviceConnectionStatus,
        containers: Vec<DockerContainerSummary>,
    ) -> DeviceSnapshot {
        DeviceSnapshot {
            device_id: "pi5".into(),
            connection_status: status,
            captured_at: "2026-01-01T00:00:00Z".into(),
            duration_ms: 10,
            metrics: None,
            docker_available: true,
            containers,
            warnings: Vec::new(),
            error: None,
            stale: false,
            last_successful_refresh: None,
        }
    }

    fn container(
        id: &str,
        state: DockerContainerState,
        health: DockerHealthStatus,
    ) -> DockerContainerSummary {
        DockerContainerSummary {
            id: id.into(),
            name: format!("container-{id}"),
            image: "example:latest".into(),
            state,
            status_text: String::new(),
            health,
            ports: Vec::new(),
            created_at: None,
            started_at: None,
        }
    }

    #[test]
    fn first_ever_snapshot_produces_no_events() {
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let current = snapshot(DeviceConnectionStatus::Offline, Vec::new());

        let events = evaluate_snapshot_transition(&repo, &sample_device(), None, &current);

        assert!(events.is_empty());
    }

    #[test]
    fn device_transition_produces_exactly_one_event() {
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let previous = snapshot(DeviceConnectionStatus::Online, Vec::new());
        let current = snapshot(DeviceConnectionStatus::Offline, Vec::new());

        let events = evaluate_snapshot_transition(&repo, &sample_device(), Some(&previous), &current);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].resource_id, "pi5");
    }

    #[test]
    fn repeated_evaluation_of_the_same_transition_is_deduplicated() {
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let previous = snapshot(DeviceConnectionStatus::Online, Vec::new());
        let current = snapshot(DeviceConnectionStatus::Offline, Vec::new());

        let first = evaluate_snapshot_transition(&repo, &sample_device(), Some(&previous), &current);
        // Evaluating the exact same (previous, current) pair again -- as
        // could happen if this were ever invoked twice for one refresh --
        // must not produce a second notification.
        let second = evaluate_snapshot_transition(&repo, &sample_device(), Some(&previous), &current);

        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
    }

    #[test]
    fn dedup_survives_a_simulated_restart() {
        let dir = tempdir().unwrap();
        let previous = snapshot(DeviceConnectionStatus::Online, Vec::new());
        let current = snapshot(DeviceConnectionStatus::Offline, Vec::new());

        {
            let repo = JsonSnapshotRepository::new(dir.path());
            let events = evaluate_snapshot_transition(&repo, &sample_device(), Some(&previous), &current);
            assert_eq!(events.len(), 1);
        }

        // A fresh repository instance over the same directory simulates
        // the app restarting between the two evaluations.
        let repo_after_restart = JsonSnapshotRepository::new(dir.path());
        let events =
            evaluate_snapshot_transition(&repo_after_restart, &sample_device(), Some(&previous), &current);
        assert!(events.is_empty());
    }

    #[test]
    fn repeated_refreshes_with_an_unchanged_state_never_notify() {
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let snap = snapshot(DeviceConnectionStatus::Online, Vec::new());

        let events = evaluate_snapshot_transition(&repo, &sample_device(), Some(&snap), &snap);

        assert!(events.is_empty());
    }

    #[test]
    fn container_state_and_health_transitions_both_produce_events() {
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let previous = snapshot(
            DeviceConnectionStatus::Online,
            vec![
                container(
                    "a",
                    DockerContainerState::Running,
                    DockerHealthStatus::Healthy,
                ),
                container(
                    "b",
                    DockerContainerState::Running,
                    DockerHealthStatus::Healthy,
                ),
            ],
        );
        let current = snapshot(
            DeviceConnectionStatus::Online,
            vec![
                container(
                    "a",
                    DockerContainerState::Exited,
                    DockerHealthStatus::Unhealthy,
                ),
                container(
                    "b",
                    DockerContainerState::Running,
                    DockerHealthStatus::Unhealthy,
                ),
            ],
        );

        let events = evaluate_snapshot_transition(&repo, &sample_device(), Some(&previous), &current);

        // container "a": both a state transition and a health transition.
        // container "b": only a health transition.
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn container_already_stopped_on_first_discovery_does_not_notify() {
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let current = snapshot(
            DeviceConnectionStatus::Online,
            vec![container(
                "a",
                DockerContainerState::Exited,
                DockerHealthStatus::None,
            )],
        );

        let events = evaluate_snapshot_transition(&repo, &sample_device(), None, &current);

        assert!(events.is_empty());
    }

    #[test]
    fn disabling_only_the_device_offline_category_suppresses_only_that_category() {
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let mut device = sample_device();
        device.notify_on_device_offline = false;
        let previous = snapshot(
            DeviceConnectionStatus::Online,
            vec![container(
                "a",
                DockerContainerState::Running,
                DockerHealthStatus::Healthy,
            )],
        );
        let current = snapshot(
            DeviceConnectionStatus::Offline,
            vec![container(
                "a",
                DockerContainerState::Exited,
                DockerHealthStatus::Healthy,
            )],
        );

        let events = evaluate_snapshot_transition(&repo, &device, Some(&previous), &current);

        // The device went offline (suppressed) *and* a container exited
        // (not suppressed) in the same refresh -- only the container
        // event should come through.
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].resource_id, "a");
    }

    #[test]
    fn disabling_only_container_failure_suppresses_only_that_category() {
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let mut device = sample_device();
        device.notify_on_container_failure = false;
        let previous = snapshot(DeviceConnectionStatus::Online, Vec::new());
        let current = snapshot(DeviceConnectionStatus::Offline, Vec::new());

        let events = evaluate_snapshot_transition(&repo, &device, Some(&previous), &current);

        // Only a device-offline transition happened here; it must still
        // fire since that category is untouched.
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].resource_id, "pi5");
    }

    #[test]
    fn disabling_container_unhealthy_leaves_container_failure_and_device_categories_intact() {
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let mut device = sample_device();
        device.notify_on_container_unhealthy = false;
        let previous = snapshot(
            DeviceConnectionStatus::Online,
            vec![container(
                "a",
                DockerContainerState::Running,
                DockerHealthStatus::Healthy,
            )],
        );
        let current = snapshot(
            DeviceConnectionStatus::Online,
            vec![container(
                "a",
                DockerContainerState::Exited,
                DockerHealthStatus::Unhealthy,
            )],
        );

        let events = evaluate_snapshot_transition(&repo, &device, Some(&previous), &current);

        // The state transition (Running -> Exited) still fires; the
        // simultaneous health transition (-> Unhealthy) does not.
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].current_state, "Exited");
    }

    #[test]
    fn a_disabled_category_never_spends_its_dedup_key_so_re_enabling_it_still_works_later() {
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let mut device = sample_device();
        device.notify_on_device_offline = false;
        let online = snapshot(DeviceConnectionStatus::Online, Vec::new());
        let offline = snapshot(DeviceConnectionStatus::Offline, Vec::new());

        // Transition happens once while the category is disabled: nothing
        // notifies, and -- unlike a real dedup hit -- nothing should have
        // been persisted as already-notified either.
        let suppressed = evaluate_snapshot_transition(&repo, &device, Some(&online), &offline);
        assert!(suppressed.is_empty());

        // Re-enable, then feed the exact same transition again (as if the
        // device flapped offline a second time): it must still notify.
        device.notify_on_device_offline = true;
        let events = evaluate_snapshot_transition(&repo, &device, Some(&online), &offline);
        assert_eq!(events.len(), 1);
    }
}
