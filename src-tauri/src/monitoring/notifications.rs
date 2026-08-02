use crate::domain::notification_rule::{
    container_health_transition_event, container_state_transition_event, device_transition_event,
    NotificationEvent,
};
use crate::domain::snapshot::DeviceSnapshot;
use crate::storage::snapshot_repository::SnapshotRepository;

/// Dispatches a notification-ready event. Producing the *decision* that a
/// transition is notification-worthy is this work unit's job; an actual
/// implementation that shows a native Windows toast is M4's job.
#[allow(dead_code)]
pub trait NotificationService: Send + Sync {
    fn notify(&self, event: &NotificationEvent);
}

/// Compares two consecutive snapshots for a device, evaluates every
/// device- and container-level transition rule, and returns only the
/// events that haven't already been notified (per the dedup key
/// persisted in state.json, so this is restart-safe). As a side effect,
/// every returned event's dedup key is recorded so it is never returned
/// again for the same transition.
pub fn evaluate_snapshot_transition(
    repo: &dyn SnapshotRepository,
    device_id: &str,
    previous: Option<&DeviceSnapshot>,
    current: &DeviceSnapshot,
) -> Vec<NotificationEvent> {
    let mut candidates = Vec::new();

    if let Some(event) = device_transition_event(
        device_id,
        previous.map(|p| p.connection_status),
        current.connection_status,
    ) {
        candidates.push(event);
    }

    let previous_containers = previous.map(|p| p.containers.as_slice()).unwrap_or(&[]);
    for container in &current.containers {
        let prev = previous_containers.iter().find(|p| p.id == container.id);

        if let Some(event) = container_state_transition_event(
            device_id,
            &container.id,
            &container.name,
            prev.map(|p| p.state),
            container.state,
        ) {
            candidates.push(event);
        }

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
    use crate::domain::docker_container::{
        DockerContainerState, DockerContainerSummary, DockerHealthStatus,
    };
    use crate::storage::snapshot_repository::JsonSnapshotRepository;
    use tempfile::tempdir;

    fn snapshot(status: DeviceConnectionStatus, containers: Vec<DockerContainerSummary>) -> DeviceSnapshot {
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

    fn container(id: &str, state: DockerContainerState, health: DockerHealthStatus) -> DockerContainerSummary {
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

        let events = evaluate_snapshot_transition(&repo, "pi5", None, &current);

        assert!(events.is_empty());
    }

    #[test]
    fn device_transition_produces_exactly_one_event() {
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let previous = snapshot(DeviceConnectionStatus::Online, Vec::new());
        let current = snapshot(DeviceConnectionStatus::Offline, Vec::new());

        let events = evaluate_snapshot_transition(&repo, "pi5", Some(&previous), &current);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].resource_id, "pi5");
    }

    #[test]
    fn repeated_evaluation_of_the_same_transition_is_deduplicated() {
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let previous = snapshot(DeviceConnectionStatus::Online, Vec::new());
        let current = snapshot(DeviceConnectionStatus::Offline, Vec::new());

        let first = evaluate_snapshot_transition(&repo, "pi5", Some(&previous), &current);
        // Evaluating the exact same (previous, current) pair again -- as
        // could happen if this were ever invoked twice for one refresh --
        // must not produce a second notification.
        let second = evaluate_snapshot_transition(&repo, "pi5", Some(&previous), &current);

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
            let events = evaluate_snapshot_transition(&repo, "pi5", Some(&previous), &current);
            assert_eq!(events.len(), 1);
        }

        // A fresh repository instance over the same directory simulates
        // the app restarting between the two evaluations.
        let repo_after_restart = JsonSnapshotRepository::new(dir.path());
        let events = evaluate_snapshot_transition(&repo_after_restart, "pi5", Some(&previous), &current);
        assert!(events.is_empty());
    }

    #[test]
    fn repeated_refreshes_with_an_unchanged_state_never_notify() {
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let snap = snapshot(DeviceConnectionStatus::Online, Vec::new());

        let events = evaluate_snapshot_transition(&repo, "pi5", Some(&snap), &snap);

        assert!(events.is_empty());
    }

    #[test]
    fn container_state_and_health_transitions_both_produce_events() {
        let dir = tempdir().unwrap();
        let repo = JsonSnapshotRepository::new(dir.path());
        let previous = snapshot(
            DeviceConnectionStatus::Online,
            vec![
                container("a", DockerContainerState::Running, DockerHealthStatus::Healthy),
                container("b", DockerContainerState::Running, DockerHealthStatus::Healthy),
            ],
        );
        let current = snapshot(
            DeviceConnectionStatus::Online,
            vec![
                container("a", DockerContainerState::Exited, DockerHealthStatus::Unhealthy),
                container("b", DockerContainerState::Running, DockerHealthStatus::Unhealthy),
            ],
        );

        let events = evaluate_snapshot_transition(&repo, "pi5", Some(&previous), &current);

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
            vec![container("a", DockerContainerState::Exited, DockerHealthStatus::None)],
        );

        let events = evaluate_snapshot_transition(&repo, "pi5", None, &current);

        assert!(events.is_empty());
    }
}
