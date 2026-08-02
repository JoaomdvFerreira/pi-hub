use serde::Serialize;

use crate::domain::docker_container::{DockerContainerState, DockerContainerSummary};

/// One container's state change between two consecutive snapshots. This is
/// purely a structural diff for the `container://status-changed` event;
/// whether a given change is *worth notifying about* (deduplication,
/// ignoring containers already stopped on first discovery, ...) is decided
/// by the notification rules, which are the next work unit's concern.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerStatusChange {
    pub container_id: String,
    pub name: String,
    pub previous_state: Option<DockerContainerState>,
    pub current_state: DockerContainerState,
}

pub fn detect_container_changes(
    previous: &[DockerContainerSummary],
    current: &[DockerContainerSummary],
) -> Vec<ContainerStatusChange> {
    let mut changes = Vec::new();

    for container in current {
        let prev = previous.iter().find(|p| p.id == container.id);
        match prev {
            Some(prev) if prev.state != container.state => {
                changes.push(ContainerStatusChange {
                    container_id: container.id.clone(),
                    name: container.name.clone(),
                    previous_state: Some(prev.state),
                    current_state: container.state,
                });
            }
            None => changes.push(ContainerStatusChange {
                container_id: container.id.clone(),
                name: container.name.clone(),
                previous_state: None,
                current_state: container.state,
            }),
            _ => {}
        }
    }

    changes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::docker_container::DockerHealthStatus;

    fn container(id: &str, state: DockerContainerState) -> DockerContainerSummary {
        DockerContainerSummary {
            id: id.into(),
            name: format!("container-{id}"),
            image: "example:latest".into(),
            state,
            status_text: String::new(),
            health: DockerHealthStatus::None,
            ports: Vec::new(),
            created_at: None,
            started_at: None,
        }
    }

    #[test]
    fn no_changes_when_states_are_identical() {
        let previous = vec![container("a", DockerContainerState::Running)];
        let current = vec![container("a", DockerContainerState::Running)];
        assert!(detect_container_changes(&previous, &current).is_empty());
    }

    #[test]
    fn detects_a_state_transition() {
        let previous = vec![container("a", DockerContainerState::Running)];
        let current = vec![container("a", DockerContainerState::Exited)];

        let changes = detect_container_changes(&previous, &current);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].previous_state, Some(DockerContainerState::Running));
        assert_eq!(changes[0].current_state, DockerContainerState::Exited);
    }

    #[test]
    fn a_newly_discovered_container_has_no_previous_state() {
        let previous: Vec<DockerContainerSummary> = Vec::new();
        let current = vec![container("a", DockerContainerState::Running)];

        let changes = detect_container_changes(&previous, &current);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].previous_state, None);
    }

    #[test]
    fn a_container_that_disappeared_produces_no_change_entry() {
        // Disappearance (e.g. removed) isn't a state transition on a
        // still-present container; only present-container transitions and
        // newly-discovered containers are reported here.
        let previous = vec![container("a", DockerContainerState::Running)];
        let current: Vec<DockerContainerSummary> = Vec::new();

        assert!(detect_container_changes(&previous, &current).is_empty());
    }

    #[test]
    fn multiple_containers_are_diffed_independently() {
        let previous = vec![
            container("a", DockerContainerState::Running),
            container("b", DockerContainerState::Running),
        ];
        let current = vec![
            container("a", DockerContainerState::Running),
            container("b", DockerContainerState::Exited),
        ];

        let changes = detect_container_changes(&previous, &current);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].container_id, "b");
    }
}
