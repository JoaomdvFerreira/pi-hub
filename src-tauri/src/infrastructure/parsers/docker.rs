use std::time::Duration;

use serde::Deserialize;

use super::key_value::ParseWarning;
use crate::domain::docker_container::{
    DockerContainerState, DockerContainerSummary, DockerHealthStatus, DockerPortBinding,
};
use crate::infrastructure::ssh::{RemoteExecutor, RemoteOperation, SshError, SshTarget};

/// The outcome of a Docker collection attempt. `Unavailable` and
/// `PermissionDenied` are both normal outcomes -- a device with no Docker,
/// or whose SSH user can't reach the Docker CLI, still counts as an online
/// device with `dockerAvailable=false`; they are never surfaced as an
/// `SshError`.
#[derive(Debug, Clone, PartialEq)]
pub enum DockerCollectionResult {
    Available {
        containers: Vec<DockerContainerSummary>,
        warnings: Vec<ParseWarning>,
    },
    Unavailable,
    PermissionDenied,
}

/// Runs the fixed DockerContainers remote operation and classifies the
/// result. Only a genuine SSH/connection-level failure (offline, timeout,
/// auth, host-key, ...) or an unrecognized remote command failure is
/// returned as `SshError`.
pub fn collect_docker_containers(
    executor: &dyn RemoteExecutor,
    target: &SshTarget,
    timeout: Duration,
) -> Result<DockerCollectionResult, SshError> {
    let command = RemoteOperation::DockerContainers
        .command()
        .expect("RemoteOperation::DockerContainers must have a command");

    match executor.execute(target, command, timeout) {
        Ok(result) => Ok(parse_docker_collection_output(&result.stdout)),
        Err(SshError::RemoteCommandError { stderr, .. })
            if stderr.to_lowercase().contains("permission denied") =>
        {
            Ok(DockerCollectionResult::PermissionDenied)
        }
        Err(other) => Err(other),
    }
}

fn parse_docker_collection_output(raw: &str) -> DockerCollectionResult {
    let mut available = true;
    let mut container_payload = String::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        match trimmed {
            "PIHUB_DOCKER_AVAILABLE=0" => available = false,
            "PIHUB_DOCKER_AVAILABLE=1" => {}
            "" => {}
            _ => {
                container_payload.push_str(trimmed);
                container_payload.push('\n');
            }
        }
    }

    if !available {
        return DockerCollectionResult::Unavailable;
    }

    let (containers, warnings) = parse_docker_containers(&container_payload);
    DockerCollectionResult::Available {
        containers,
        warnings,
    }
}

#[derive(Debug, Deserialize)]
struct RawDockerPsLine {
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "Names")]
    names: String,
    #[serde(rename = "Image")]
    image: String,
    #[serde(rename = "State")]
    state: String,
    #[serde(rename = "Status")]
    status: String,
    #[serde(rename = "Ports")]
    ports: String,
    #[serde(rename = "CreatedAt")]
    created_at: String,
}

/// Parses `docker ps -a --no-trunc --format '{{json .}}'` output, one JSON
/// object per line. A malformed line produces a warning and is skipped;
/// every other valid line is still returned.
pub fn parse_docker_containers(raw: &str) -> (Vec<DockerContainerSummary>, Vec<ParseWarning>) {
    let mut containers = Vec::new();
    let mut warnings = Vec::new();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<RawDockerPsLine>(line) {
            Ok(raw_line) => containers.push(convert(raw_line)),
            Err(err) => warnings.push(ParseWarning(format!(
                "ignored malformed docker ps line: {err}"
            ))),
        }
    }

    (containers, warnings)
}

fn convert(raw: RawDockerPsLine) -> DockerContainerSummary {
    DockerContainerSummary {
        id: raw.id,
        name: raw.names,
        image: raw.image,
        state: parse_state(&raw.state),
        health: parse_health(&raw.status),
        status_text: raw.status,
        ports: parse_ports(&raw.ports),
        created_at: if raw.created_at.is_empty() {
            None
        } else {
            Some(raw.created_at)
        },
        started_at: None,
    }
}

fn parse_state(raw: &str) -> DockerContainerState {
    match raw {
        "running" => DockerContainerState::Running,
        "restarting" => DockerContainerState::Restarting,
        "paused" => DockerContainerState::Paused,
        "exited" => DockerContainerState::Exited,
        "dead" => DockerContainerState::Dead,
        _ => DockerContainerState::Unknown,
    }
}

/// `docker ps` has no dedicated health field; a healthcheck's status is
/// embedded in the Status text, e.g. "Up 2 weeks (healthy)" or
/// "Up 3 minutes (health: starting)".
fn parse_health(status: &str) -> DockerHealthStatus {
    let lower = status.to_lowercase();
    if lower.contains("(healthy)") {
        DockerHealthStatus::Healthy
    } else if lower.contains("(unhealthy)") {
        DockerHealthStatus::Unhealthy
    } else if lower.contains("health: starting") {
        DockerHealthStatus::Starting
    } else {
        DockerHealthStatus::None
    }
}

fn parse_ports(raw: &str) -> Vec<DockerPortBinding> {
    if raw.trim().is_empty() {
        return Vec::new();
    }
    raw.split(", ").filter_map(parse_port_entry).collect()
}

fn parse_port_entry(entry: &str) -> Option<DockerPortBinding> {
    let entry = entry.trim();
    let (host_part, container_part) = match entry.split_once("->") {
        Some((h, c)) => (Some(h), c),
        None => (None, entry),
    };

    let (container_port_str, protocol) = container_part.split_once('/')?;
    let container_port = container_port_str.parse::<u16>().ok()?;

    let (host_ip, host_port) = match host_part {
        Some(h) => {
            let (ip, port_str) = h.rsplit_once(':')?;
            let port = port_str.parse::<u16>().ok()?;
            (Some(ip.to_string()), Some(port))
        }
        None => (None, None),
    };

    Some(DockerPortBinding {
        host_ip,
        host_port,
        container_port,
        protocol: protocol.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ssh::fake::FakeRemoteExecutor;

    const HOME_ASSISTANT_LINE: &str = r#"{"ID":"a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2","Names":"homeassistant","Image":"homeassistant/home-assistant:2024.8","State":"running","Status":"Up 2 weeks (healthy)","Ports":"0.0.0.0:8123->8123/tcp","CreatedAt":"2024-08-02 10:00:00 +0000 UTC"}"#;

    #[test]
    fn parses_a_well_formed_line() {
        let (containers, warnings) = parse_docker_containers(HOME_ASSISTANT_LINE);

        assert!(warnings.is_empty());
        assert_eq!(containers.len(), 1);
        let container = &containers[0];
        assert_eq!(container.name, "homeassistant");
        assert_eq!(container.image, "homeassistant/home-assistant:2024.8");
        assert_eq!(container.state, DockerContainerState::Running);
        assert_eq!(container.health, DockerHealthStatus::Healthy);
        assert_eq!(container.status_text, "Up 2 weeks (healthy)");
        assert_eq!(
            container.ports,
            vec![DockerPortBinding {
                host_ip: Some("0.0.0.0".into()),
                host_port: Some(8123),
                container_port: 8123,
                protocol: "tcp".into(),
            }]
        );
        assert_eq!(
            container.created_at.as_deref(),
            Some("2024-08-02 10:00:00 +0000 UTC")
        );
    }

    #[test]
    fn empty_payload_yields_no_containers_or_warnings() {
        let (containers, warnings) = parse_docker_containers("");
        assert!(containers.is_empty());
        assert!(warnings.is_empty());
    }

    #[test]
    fn malformed_lines_produce_warnings_without_discarding_valid_entries() {
        let payload = format!("not valid json at all\n{HOME_ASSISTANT_LINE}\n{{\"incomplete\":");
        let (containers, warnings) = parse_docker_containers(&payload);

        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].name, "homeassistant");
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn unpublished_port_has_no_host_binding() {
        let line = r#"{"ID":"1","Names":"internal","Image":"redis:7","State":"running","Status":"Up 1 hour","Ports":"6379/tcp","CreatedAt":"2024-01-01 00:00:00 +0000 UTC"}"#;
        let (containers, _) = parse_docker_containers(line);
        assert_eq!(
            containers[0].ports,
            vec![DockerPortBinding {
                host_ip: None,
                host_port: None,
                container_port: 6379,
                protocol: "tcp".into(),
            }]
        );
    }

    #[test]
    fn multiple_port_bindings_are_all_parsed() {
        let line = r#"{"ID":"1","Names":"multi","Image":"x","State":"running","Status":"Up","Ports":"0.0.0.0:80->80/tcp, 0.0.0.0:443->443/tcp","CreatedAt":""}"#;
        let (containers, _) = parse_docker_containers(line);
        assert_eq!(containers[0].ports.len(), 2);
    }

    #[test]
    fn unhealthy_and_starting_health_are_detected() {
        let unhealthy = r#"{"ID":"1","Names":"a","Image":"x","State":"restarting","Status":"Restarting (1) 3 seconds ago (unhealthy)","Ports":"","CreatedAt":""}"#;
        let starting = r#"{"ID":"2","Names":"b","Image":"x","State":"running","Status":"Up 5 seconds (health: starting)","Ports":"","CreatedAt":""}"#;

        let (containers, _) = parse_docker_containers(&format!("{unhealthy}\n{starting}"));

        assert_eq!(containers[0].state, DockerContainerState::Restarting);
        assert_eq!(containers[0].health, DockerHealthStatus::Unhealthy);
        assert_eq!(containers[1].health, DockerHealthStatus::Starting);
    }

    #[test]
    fn container_without_healthcheck_reports_none() {
        let line = r#"{"ID":"1","Names":"a","Image":"x","State":"running","Status":"Up 3 days","Ports":"","CreatedAt":""}"#;
        let (containers, _) = parse_docker_containers(line);
        assert_eq!(containers[0].health, DockerHealthStatus::None);
    }

    #[test]
    fn unrecognized_state_maps_to_unknown() {
        let line = r#"{"ID":"1","Names":"a","Image":"x","State":"created","Status":"Created","Ports":"","CreatedAt":""}"#;
        let (containers, _) = parse_docker_containers(line);
        assert_eq!(containers[0].state, DockerContainerState::Unknown);
    }

    #[test]
    fn parses_docker_available_marker_and_containers_together() {
        let payload = format!("PIHUB_DOCKER_AVAILABLE=1\n{HOME_ASSISTANT_LINE}\n");
        let result = parse_docker_collection_output(&payload);
        match result {
            DockerCollectionResult::Available {
                containers,
                warnings,
            } => {
                assert_eq!(containers.len(), 1);
                assert!(warnings.is_empty());
            }
            other => panic!("expected Available, got {other:?}"),
        }
    }

    #[test]
    fn docker_unavailable_marker_yields_unavailable_with_no_containers() {
        let result = parse_docker_collection_output("PIHUB_DOCKER_AVAILABLE=0\n");
        assert_eq!(result, DockerCollectionResult::Unavailable);
    }

    #[test]
    fn empty_container_list_is_still_available() {
        let result = parse_docker_collection_output("PIHUB_DOCKER_AVAILABLE=1\n");
        assert_eq!(
            result,
            DockerCollectionResult::Available {
                containers: Vec::new(),
                warnings: Vec::new(),
            }
        );
    }

    fn target() -> SshTarget {
        SshTarget {
            host: "raspberrypi5.tail3f2a.ts.net".into(),
            port: 22,
            username: "joao".into(),
        }
    }

    #[test]
    fn collect_reports_unavailable_when_docker_is_not_installed() {
        let executor = FakeRemoteExecutor::online("PIHUB_DOCKER_AVAILABLE=0\n");
        let result = collect_docker_containers(&executor, &target(), Duration::from_secs(10))
            .unwrap();
        assert_eq!(result, DockerCollectionResult::Unavailable);
    }

    #[test]
    fn collect_reports_available_containers_on_success() {
        let payload = format!("PIHUB_DOCKER_AVAILABLE=1\n{HOME_ASSISTANT_LINE}\n");
        let executor = FakeRemoteExecutor::online(payload);
        let result = collect_docker_containers(&executor, &target(), Duration::from_secs(10))
            .unwrap();
        match result {
            DockerCollectionResult::Available { containers, .. } => {
                assert_eq!(containers.len(), 1);
            }
            other => panic!("expected Available, got {other:?}"),
        }
    }

    #[test]
    fn collect_classifies_permission_denied_without_failing() {
        let executor = FakeRemoteExecutor::returning(Err(SshError::RemoteCommandError {
            exit_code: Some(1),
            stderr: "permission denied while trying to connect to the Docker daemon socket"
                .into(),
        }));
        let result = collect_docker_containers(&executor, &target(), Duration::from_secs(10))
            .unwrap();
        assert_eq!(result, DockerCollectionResult::PermissionDenied);
    }

    #[test]
    fn collect_propagates_connection_level_failures() {
        let executor = FakeRemoteExecutor::offline();
        let err = collect_docker_containers(&executor, &target(), Duration::from_secs(10))
            .unwrap_err();
        assert_eq!(err, SshError::ConnectionRefused);
    }

    #[test]
    fn collect_propagates_unrecognized_command_failures() {
        let executor = FakeRemoteExecutor::returning(Err(SshError::RemoteCommandError {
            exit_code: Some(127),
            stderr: "sh: docker: command not found".into(),
        }));
        let err = collect_docker_containers(&executor, &target(), Duration::from_secs(10))
            .unwrap_err();
        assert!(matches!(err, SshError::RemoteCommandError { .. }));
    }
}
