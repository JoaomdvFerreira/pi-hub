use serde::{Deserialize, Serialize};

use crate::domain::connection_status::DeviceConnectionStatus;
use crate::domain::docker_container::{DockerContainerState, DockerHealthStatus};

/// A notification-ready decision: some transition worth telling the user
/// about was detected. Producing this event is this work unit's job;
/// actually dispatching it as a native Windows toast is M4's job.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationEvent {
    pub device_id: String,
    pub resource_id: String,
    pub previous_state: String,
    pub current_state: String,
    pub message: String,
}

impl NotificationEvent {
    /// The deduplication key from spec section 15.3:
    /// `deviceId + resourceId + previousState + currentState`.
    pub fn dedup_key(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.device_id, self.resource_id, self.previous_state, self.current_state
        )
    }
}

/// Device transition rules (spec section 15.1): notify only on
/// online<->offline and online->authentication_error/host_key_error.
/// A device observed for the first time (`previous: None`) never
/// notifies -- there is nothing to transition *from* yet.
pub fn device_transition_event(
    device_id: &str,
    previous: Option<DeviceConnectionStatus>,
    current: DeviceConnectionStatus,
) -> Option<NotificationEvent> {
    use DeviceConnectionStatus::{AuthenticationError, HostKeyError, Offline, Online};

    let previous = previous?;
    let message = match (previous, current) {
        (Online, Offline) => "Device went offline",
        (Offline, Online) => "Device is back online",
        (Online, AuthenticationError) => "Device authentication failed",
        (Online, HostKeyError) => "Device SSH host key verification failed",
        _ => return None,
    };

    Some(NotificationEvent {
        device_id: device_id.to_string(),
        resource_id: device_id.to_string(),
        previous_state: format!("{previous:?}"),
        current_state: format!("{current:?}"),
        message: message.to_string(),
    })
}

/// Container state transition rule (spec section 15.2): notify when a
/// previously *running* container changes to exited/stopped/restarting.
/// A container discovered for the first time already in a non-running
/// state (`previous: None`) never notifies.
pub fn container_state_transition_event(
    device_id: &str,
    container_id: &str,
    container_name: &str,
    previous: Option<DockerContainerState>,
    current: DockerContainerState,
) -> Option<NotificationEvent> {
    use DockerContainerState::{Exited, Restarting, Running, Stopped};

    let previous = previous?;
    if previous != Running || !matches!(current, Exited | Stopped | Restarting) {
        return None;
    }

    Some(NotificationEvent {
        device_id: device_id.to_string(),
        resource_id: container_id.to_string(),
        previous_state: format!("{previous:?}"),
        current_state: format!("{current:?}"),
        message: format!("Container '{container_name}' changed to {current:?}"),
    })
}

/// Container health transition rule (spec section 15.2): notify when a
/// container becomes unhealthy. Requires a genuine previous observation
/// (`previous: None` never notifies), so a container already unhealthy
/// when first discovered doesn't fire either, matching the same
/// first-discovery principle as the state-transition rule.
pub fn container_health_transition_event(
    device_id: &str,
    container_id: &str,
    container_name: &str,
    previous: Option<DockerHealthStatus>,
    current: DockerHealthStatus,
) -> Option<NotificationEvent> {
    let previous = previous?;
    if current != DockerHealthStatus::Unhealthy || previous == DockerHealthStatus::Unhealthy {
        return None;
    }

    Some(NotificationEvent {
        device_id: device_id.to_string(),
        resource_id: container_id.to_string(),
        previous_state: format!("{previous:?}"),
        current_state: format!("{current:?}"),
        message: format!("Container '{container_name}' became unhealthy"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use DeviceConnectionStatus::*;

    #[test]
    fn device_online_to_offline_notifies() {
        let event = device_transition_event("pi5", Some(Online), Offline).unwrap();
        assert_eq!(event.message, "Device went offline");
        assert_eq!(event.resource_id, "pi5");
    }

    #[test]
    fn device_offline_to_online_notifies() {
        let event = device_transition_event("pi5", Some(Offline), Online).unwrap();
        assert_eq!(event.message, "Device is back online");
    }

    #[test]
    fn device_online_to_authentication_error_notifies() {
        assert!(device_transition_event("pi5", Some(Online), AuthenticationError).is_some());
    }

    #[test]
    fn device_online_to_host_key_error_notifies() {
        assert!(device_transition_event("pi5", Some(Online), HostKeyError).is_some());
    }

    #[test]
    fn device_first_observation_never_notifies() {
        assert_eq!(device_transition_event("pi5", None, Offline), None);
        assert_eq!(device_transition_event("pi5", None, Online), None);
    }

    #[test]
    fn device_unchanged_state_never_notifies() {
        assert_eq!(device_transition_event("pi5", Some(Online), Online), None);
        assert_eq!(device_transition_event("pi5", Some(Offline), Offline), None);
    }

    #[test]
    fn device_untracked_transitions_do_not_notify() {
        // Only the four explicit spec transitions notify; e.g. a
        // transition into Timeout or CommandError does not.
        assert_eq!(
            device_transition_event("pi5", Some(Online), DeviceConnectionStatus::Timeout),
            None
        );
        assert_eq!(
            device_transition_event("pi5", Some(Online), DeviceConnectionStatus::CommandError),
            None
        );
    }

    use DockerContainerState::*;

    #[test]
    fn container_running_to_exited_notifies() {
        let event =
            container_state_transition_event("pi5", "abc", "homeassistant", Some(Running), Exited)
                .unwrap();
        assert_eq!(event.resource_id, "abc");
        assert!(event.message.contains("homeassistant"));
    }

    #[test]
    fn container_running_to_stopped_and_restarting_notify() {
        assert!(
            container_state_transition_event("pi5", "abc", "x", Some(Running), Stopped).is_some()
        );
        assert!(
            container_state_transition_event("pi5", "abc", "x", Some(Running), Restarting)
                .is_some()
        );
    }

    #[test]
    fn container_first_discovery_already_stopped_does_not_notify() {
        // The explicit acceptance-criteria case: a container observed for
        // the first time already in a stopped state must not notify.
        assert_eq!(
            container_state_transition_event("pi5", "abc", "x", None, Exited),
            None
        );
        assert_eq!(
            container_state_transition_event("pi5", "abc", "x", None, Stopped),
            None
        );
    }

    #[test]
    fn container_first_discovery_running_does_not_notify() {
        assert_eq!(
            container_state_transition_event("pi5", "abc", "x", None, Running),
            None
        );
    }

    #[test]
    fn container_non_running_to_exited_does_not_notify() {
        // Only a transition *from* Running is notification-worthy.
        assert_eq!(
            container_state_transition_event("pi5", "abc", "x", Some(Paused), Exited),
            None
        );
    }

    #[test]
    fn container_running_to_running_does_not_notify() {
        assert_eq!(
            container_state_transition_event("pi5", "abc", "x", Some(Running), Running),
            None
        );
    }

    // Not glob-imported: DockerHealthStatus::None would shadow
    // std::option::Option::None used throughout these assertions.
    use DockerHealthStatus::{Healthy, Unhealthy};

    #[test]
    fn container_becomes_unhealthy_notifies() {
        let event = container_health_transition_event(
            "pi5",
            "abc",
            "homeassistant",
            Some(Healthy),
            Unhealthy,
        )
        .unwrap();
        assert!(event.message.contains("unhealthy"));
    }

    #[test]
    fn container_health_first_discovery_never_notifies() {
        assert_eq!(
            container_health_transition_event("pi5", "abc", "x", None, Unhealthy),
            None
        );
    }

    #[test]
    fn container_already_unhealthy_does_not_notify_again() {
        assert_eq!(
            container_health_transition_event("pi5", "abc", "x", Some(Unhealthy), Unhealthy),
            None
        );
    }

    #[test]
    fn container_recovering_from_unhealthy_does_not_notify() {
        assert_eq!(
            container_health_transition_event("pi5", "abc", "x", Some(Unhealthy), Healthy),
            None
        );
    }

    #[test]
    fn dedup_key_combines_all_four_components() {
        let event = device_transition_event("pi5", Some(Online), Offline).unwrap();
        assert_eq!(event.dedup_key(), "pi5|pi5|Online|Offline");
    }

    #[test]
    fn dedup_key_includes_resource_id_so_different_containers_never_collide() {
        let event_a =
            container_state_transition_event("pi5", "container-a", "x", Some(Running), Exited)
                .unwrap();
        let event_b =
            container_state_transition_event("pi5", "container-b", "x", Some(Running), Exited)
                .unwrap();
        assert_ne!(event_a.dedup_key(), event_b.dedup_key());
    }
}
