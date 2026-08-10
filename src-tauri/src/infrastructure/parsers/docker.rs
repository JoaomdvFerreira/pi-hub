use std::{collections::HashMap, time::Duration};

use serde::Deserialize;

use super::key_value::ParseWarning;
use crate::domain::docker_container::{
    DockerContainerState, DockerContainerSummary, DockerHealthStatus, DockerLabel, DockerPortBinding,
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
    let mut inspection_payloads = Vec::new();
    let mut stats_payloads = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        match trimmed {
            "PIHUB_DOCKER_AVAILABLE=0" => available = false,
            "PIHUB_DOCKER_AVAILABLE=1" => {}
            _ if trimmed.starts_with("PIHUB_DOCKER_INSPECT=") => inspection_payloads.push(&trimmed[21..]),
            _ if trimmed.starts_with("PIHUB_DOCKER_STATS=") => stats_payloads.push(&trimmed[19..]),
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

    let (mut containers, mut warnings) = parse_docker_containers(&container_payload);
    for inspect in inspection_payloads {
        match serde_json::from_str::<RawDockerInspect>(inspect) {
            Ok(inspect) => apply_inspection(&mut containers, inspect),
            Err(err) => warnings.push(ParseWarning(format!("ignored malformed docker inspect line: {err}"))),
        }
    }
    for stats in stats_payloads {
        match serde_json::from_str::<RawDockerStats>(stats) {
            Ok(stats) => apply_stats(&mut containers, stats),
            Err(err) => warnings.push(ParseWarning(format!("ignored malformed docker stats line: {err}"))),
        }
    }
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
    #[serde(rename = "ImageID", default)]
    image_id: String,
    #[serde(rename = "Labels", default)]
    labels: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDockerInspect {
    id: String,
    image_id: String,
    created_at: String,
    started_at: String,
    restart_count: u64,
    restart_policy: String,
    restart_maximum_retry_count: u64,
    #[serde(default)] mounts: Vec<RawMount>,
    #[serde(default)] networks: HashMap<String, RawNetwork>,
    #[serde(default)] labels: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct RawMount { #[serde(rename = "Type")] mount_type: String, #[serde(rename = "Source")] source: String, #[serde(rename = "Destination")] destination: String, #[serde(rename = "RW")] read_write: bool }

#[derive(Debug, Deserialize)]
struct RawNetwork { #[serde(rename = "IPAddress", default)] ip_address: String, #[serde(rename = "Aliases", default)] aliases: Option<Vec<String>> }

#[derive(Debug, Deserialize)]
struct RawDockerStats { #[serde(rename = "ID")] id: String, #[serde(rename = "CPUPerc")] cpu_percent: String, #[serde(rename = "MemUsage")] memory_usage: String, #[serde(rename = "MemPerc")] memory_percent: String }

fn apply_inspection(containers: &mut [DockerContainerSummary], inspect: RawDockerInspect) {
    let Some(container) = containers.iter_mut().find(|container| container.id == inspect.id) else { return; };
    container.image_id = (!inspect.image_id.is_empty()).then_some(inspect.image_id);
    container.created_at = (!inspect.created_at.is_empty()).then_some(inspect.created_at);
    container.started_at = (!inspect.started_at.is_empty() && !inspect.started_at.starts_with("0001-")).then_some(inspect.started_at);
    container.restart_count = Some(inspect.restart_count);
    container.restart_policy = (!inspect.restart_policy.is_empty()).then_some(inspect.restart_policy);
    container.restart_maximum_retry_count = (inspect.restart_maximum_retry_count > 0).then_some(inspect.restart_maximum_retry_count);
    container.mounts = inspect.mounts.into_iter().take(32).map(|mount| crate::domain::docker_container::DockerMount { mount_type: mount.mount_type, source: truncate_component(&mount.source), destination: truncate_component(&mount.destination), read_only: !mount.read_write }).collect();
    container.networks = inspect.networks.into_iter().take(16).map(|(name, network)| crate::domain::docker_container::DockerNetwork { name: truncate_component(&name), ip_address: (!network.ip_address.is_empty()).then_some(network.ip_address), aliases: network.aliases.unwrap_or_default().into_iter().take(16).map(|value| truncate_component(&value)).collect() }).collect();
    container.labels = inspect.labels.into_iter().take(MAX_LABELS).map(|(key, value)| label_from_pair(key, value)).collect();
}

fn apply_stats(containers: &mut [DockerContainerSummary], stats: RawDockerStats) {
    let Some(container) = containers.iter_mut().find(|container| container.id.starts_with(&stats.id)) else { return; };
    let (used, limit) = stats.memory_usage.split_once('/').map(|(used, limit)| (parse_docker_bytes(used.trim()), parse_docker_bytes(limit.trim()))).unwrap_or((None, None));
    let cpu_percent = parse_percent(&stats.cpu_percent);
    let memory_percent = parse_percent(&stats.memory_percent).or_else(|| match (used, limit) { (Some(used), Some(limit)) if limit > 0 => Some(used as f64 * 100.0 / limit as f64), _ => None });
    if cpu_percent.is_some() || used.is_some() || limit.is_some() { container.resource_usage = Some(crate::domain::docker_container::DockerResourceUsage { cpu_percent, memory_used_bytes: used, memory_limit_bytes: limit, memory_percent }); }
}

fn parse_percent(raw: &str) -> Option<f64> { raw.trim().strip_suffix('%')?.trim().parse().ok() }

fn parse_docker_bytes(raw: &str) -> Option<u64> {
    let raw = raw.trim();
    let split_at = raw.find(|character: char| character.is_ascii_alphabetic())?;
    let (number, unit) = raw.split_at(split_at);
    let multiplier = match unit.trim().to_ascii_lowercase().as_str() { "b" => 1.0, "kb" | "kib" => 1024.0, "mb" | "mib" => 1024.0 * 1024.0, "gb" | "gib" => 1024.0 * 1024.0 * 1024.0, "tb" | "tib" => 1024.0 * 1024.0 * 1024.0 * 1024.0, _ => return None };
    Some((number.trim().parse::<f64>().ok()? * multiplier) as u64)
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
        image_id: (!raw.image_id.is_empty()).then_some(raw.image_id),
        restart_count: None,
        restart_policy: None,
        restart_maximum_retry_count: None,
        mounts: Vec::new(),
        networks: Vec::new(),
        labels: parse_labels(&raw.labels),
        resource_usage: None,
    }
}

fn parse_state(raw: &str) -> DockerContainerState {
    match raw {
        "created" => DockerContainerState::Created,
        "running" => DockerContainerState::Running,
        "restarting" => DockerContainerState::Restarting,
        "paused" => DockerContainerState::Paused,
        "exited" => DockerContainerState::Exited,
        "dead" => DockerContainerState::Dead,
        "removing" => DockerContainerState::Removing,
        _ => DockerContainerState::Unknown,
    }
}

const MAX_LABELS: usize = 32;
const MAX_LABEL_COMPONENT_LENGTH: usize = 256;

/// Docker's `Labels` ps field is a comma-separated key=value list. Keep it
/// bounded and redact obviously credential-bearing values before it can leave
/// the backend. Environment variables are never read by this parser.
fn parse_labels(raw: &str) -> Vec<DockerLabel> {
    raw.split(',')
        .filter_map(|entry| entry.split_once('='))
        .take(MAX_LABELS)
        .map(|(key, value)| label_from_pair(key.to_owned(), value.to_owned()))
        .collect()
}

fn label_from_pair(key: String, value: String) -> DockerLabel {
    let key = truncate_component(&key);
    let secret_like = ["secret", "password", "token", "apikey", "api_key", "credential"].iter().any(|needle| key.to_ascii_lowercase().contains(needle));
    DockerLabel { key, value: if secret_like { "[redacted]".into() } else { truncate_component(&value) }, redacted: secret_like }
}

fn truncate_component(value: &str) -> String {
    value.chars().take(MAX_LABEL_COMPONENT_LENGTH).collect()
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
        assert_eq!(containers[0].state, DockerContainerState::Created);
    }

    #[test]
    fn labels_are_bounded_and_secret_like_values_are_redacted() {
        let labels = parse_labels("app=home,api_token=do-not-display,password=hunter2");
        assert_eq!(labels.len(), 3);
        assert_eq!(labels[0].value, "home");
        assert_eq!(labels[1].value, "[redacted]");
        assert!(labels[1].redacted);
        assert_eq!(labels[2].value, "[redacted]");
    }

    #[test]
    fn inspection_enriches_the_matching_container_without_exposing_environment() {
        let payload = format!("PIHUB_DOCKER_AVAILABLE=1\n{HOME_ASSISTANT_LINE}\nPIHUB_DOCKER_INSPECT={{\"id\":\"a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2\",\"imageId\":\"sha256:image\",\"createdAt\":\"2024-01-01T00:00:00Z\",\"startedAt\":\"2024-01-01T01:00:00Z\",\"restartCount\":2,\"restartPolicy\":\"unless-stopped\",\"restartMaximumRetryCount\":0,\"mounts\":[{{\"Type\":\"volume\",\"Source\":\"data\",\"Destination\":\"/data\",\"RW\":true}}],\"networks\":{{\"bridge\":{{\"IPAddress\":\"172.17.0.2\",\"Aliases\":[\"homeassistant\"]}}}},\"labels\":{{\"app\":\"home\",\"api_key\":\"hidden\"}}}}\n");
        let DockerCollectionResult::Available { containers, warnings } = parse_docker_collection_output(&payload) else { panic!("expected available"); };
        assert!(warnings.is_empty());
        let container = &containers[0];
        assert_eq!(container.restart_count, Some(2));
        assert_eq!(container.mounts[0].destination, "/data");
        assert_eq!(container.networks[0].ip_address.as_deref(), Some("172.17.0.2"));
        assert_eq!(container.labels.iter().find(|label| label.key == "api_key").unwrap().value, "[redacted]");
    }

    #[test]
    fn stats_enrich_running_container_and_ignores_malformed_values() {
        let payload = format!("PIHUB_DOCKER_AVAILABLE=1\n{HOME_ASSISTANT_LINE}\nPIHUB_DOCKER_STATS={{\"ID\":\"a1b2c3d4e5f6\",\"CPUPerc\":\"12.50%\",\"MemUsage\":\"128MiB / 1GiB\",\"MemPerc\":\"12.50%\"}}\nPIHUB_DOCKER_STATS={{\"ID\":\"missing\",\"CPUPerc\":\"bad\",\"MemUsage\":\"unknown\",\"MemPerc\":\"bad\"}}\n");
        let DockerCollectionResult::Available { containers, warnings } = parse_docker_collection_output(&payload) else { panic!("expected available"); };
        assert!(warnings.is_empty());
        let usage = containers[0].resource_usage.as_ref().unwrap();
        assert_eq!(usage.cpu_percent, Some(12.5));
        assert_eq!(usage.memory_used_bytes, Some(128 * 1024 * 1024));
        assert_eq!(usage.memory_limit_bytes, Some(1024 * 1024 * 1024));
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
        let result =
            collect_docker_containers(&executor, &target(), Duration::from_secs(10)).unwrap();
        assert_eq!(result, DockerCollectionResult::Unavailable);
    }

    #[test]
    fn collect_reports_available_containers_on_success() {
        let payload = format!("PIHUB_DOCKER_AVAILABLE=1\n{HOME_ASSISTANT_LINE}\n");
        let executor = FakeRemoteExecutor::online(payload);
        let result =
            collect_docker_containers(&executor, &target(), Duration::from_secs(10)).unwrap();
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
            stderr: "permission denied while trying to connect to the Docker daemon socket".into(),
        }));
        let result =
            collect_docker_containers(&executor, &target(), Duration::from_secs(10)).unwrap();
        assert_eq!(result, DockerCollectionResult::PermissionDenied);
    }

    #[test]
    fn collect_propagates_connection_level_failures() {
        let executor = FakeRemoteExecutor::offline();
        let err =
            collect_docker_containers(&executor, &target(), Duration::from_secs(10)).unwrap_err();
        assert_eq!(err, SshError::ConnectionRefused);
    }

    #[test]
    fn collect_propagates_unrecognized_command_failures() {
        let executor = FakeRemoteExecutor::returning(Err(SshError::RemoteCommandError {
            exit_code: Some(127),
            stderr: "sh: docker: command not found".into(),
        }));
        let err =
            collect_docker_containers(&executor, &target(), Duration::from_secs(10)).unwrap_err();
        assert!(matches!(err, SshError::RemoteCommandError { .. }));
    }
}
