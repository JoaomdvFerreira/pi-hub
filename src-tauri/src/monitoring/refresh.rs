use std::time::{Duration, Instant};

use crate::domain::connection_status::DeviceConnectionStatus;
use crate::domain::device::Device;
use crate::domain::snapshot::DeviceSnapshot;
use crate::error::ApplicationError;
use crate::infrastructure::parsers::docker::{collect_docker_containers, DockerCollectionResult};
use crate::infrastructure::parsers::metrics::collect_system_metrics;
use crate::infrastructure::ssh::{RemoteExecutor, SshTarget};

/// Matches spec section 24.3's remote-metrics and Docker-collection
/// timeouts.
const METRICS_TIMEOUT: Duration = Duration::from_secs(10);
const DOCKER_TIMEOUT: Duration = Duration::from_secs(10);
/// Spec section 24.3's SSH connection timeout, used for the initial probe.
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// Runs one full refresh cycle for a device: probe, then (on success)
/// collect metrics and Docker data, and assembles an immutable
/// `DeviceSnapshot`. This is synchronous/blocking (the SSH calls it makes
/// are blocking process execution); callers on an async runtime must run
/// it via `spawn_blocking`.
///
/// A failed probe never fails this function: it returns a snapshot with
/// the classified `connectionStatus`, `stale: true`, and metrics/
/// containers carried forward from `previous` (per the architecture
/// spec's failure-isolation rule that a failed refresh must not erase
/// previously successful data).
pub fn refresh_device_sync(
    executor: &dyn RemoteExecutor,
    device: &Device,
    previous: Option<&DeviceSnapshot>,
) -> DeviceSnapshot {
    let started = Instant::now();
    let captured_at = chrono::Utc::now().to_rfc3339();
    let target = SshTarget {
        host: device.host.clone(),
        port: device.ssh_port,
        username: device.ssh_username.clone(),
    };

    if let Err(err) = executor.probe(&target, PROBE_TIMEOUT) {
        return DeviceSnapshot {
            device_id: device.id.clone(),
            connection_status: err.to_connection_status(),
            captured_at,
            duration_ms: started.elapsed().as_millis() as u64,
            metrics: previous.and_then(|p| p.metrics.clone()),
            docker_available: previous.map(|p| p.docker_available).unwrap_or(false),
            containers: previous.map(|p| p.containers.clone()).unwrap_or_default(),
            warnings: Vec::new(),
            error: Some(ApplicationError {
                code: err.code().to_string(),
                message: err.to_string(),
                remediation: Some(err.remediation().to_string()),
                retryable: true,
            }),
            stale: true,
            last_successful_refresh: previous.and_then(|p| p.last_successful_refresh.clone()),
        };
    }

    let mut warnings = Vec::new();

    let metrics = match collect_system_metrics(executor, &target, METRICS_TIMEOUT) {
        Ok((metrics, parse_warnings)) => {
            warnings.extend(parse_warnings.into_iter().map(|w| w.0));
            Some(metrics)
        }
        Err(_) => {
            // The probe just succeeded, so a metrics-collection failure
            // here is unexpected (e.g. a race where the device dropped
            // off between the two SSH calls). Missing metrics must never
            // fail the whole refresh.
            warnings.push("system metrics collection failed".to_string());
            None
        }
    };

    let (docker_available, containers) =
        match collect_docker_containers(executor, &target, DOCKER_TIMEOUT) {
            Ok(DockerCollectionResult::Available {
                containers,
                warnings: docker_warnings,
            }) => {
                warnings.extend(docker_warnings.into_iter().map(|w| w.0));
                (true, containers)
            }
            Ok(DockerCollectionResult::Unavailable) => (false, Vec::new()),
            Ok(DockerCollectionResult::PermissionDenied) => {
                warnings.push("Docker permission denied for the configured SSH user.".to_string());
                (false, Vec::new())
            }
            Err(err) => {
                warnings.push(format!("Docker collection failed: {err}"));
                (false, Vec::new())
            }
        };

    DeviceSnapshot {
        device_id: device.id.clone(),
        connection_status: DeviceConnectionStatus::Online,
        captured_at: captured_at.clone(),
        duration_ms: started.elapsed().as_millis() as u64,
        metrics,
        docker_available,
        containers,
        warnings,
        error: None,
        stale: false,
        last_successful_refresh: Some(captured_at),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::device::DeviceType;
    use crate::domain::docker_container::{
        DockerContainerState, DockerContainerSummary, DockerHealthStatus,
    };
    use crate::domain::system_metrics::SystemMetrics;
    use crate::infrastructure::ssh::fake::FakeRemoteExecutor;
    use crate::infrastructure::ssh::SshError;

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
            notifications_enabled: true,
            services: Vec::new(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    fn sample_container(id: &str) -> DockerContainerSummary {
        DockerContainerSummary {
            id: id.into(),
            name: "homeassistant".into(),
            image: "homeassistant/home-assistant:2024.8".into(),
            state: DockerContainerState::Running,
            status_text: "Up 2 weeks".into(),
            health: DockerHealthStatus::None,
            ports: Vec::new(),
            created_at: None,
            started_at: None,
        }
    }

    #[test]
    fn successful_refresh_reports_online_with_metrics_and_containers() {
        // A single fake response is reused for every command this device
        // refresh issues (probe, metrics, docker) since FakeRemoteExecutor
        // ignores the command argument; this payload happens to be valid
        // for all three because it only exercises fields each parser
        // tolerates being absent for the others.
        let executor = FakeRemoteExecutor::online(
            "PIHUB_HOSTNAME=pi5\nPIHUB_UPTIME_SECONDS=100\nPIHUB_DOCKER_AVAILABLE=0\n",
        );
        let snapshot = refresh_device_sync(&executor, &sample_device(), None);

        assert_eq!(snapshot.connection_status, DeviceConnectionStatus::Online);
        assert!(!snapshot.stale);
        assert!(snapshot.error.is_none());
        assert_eq!(
            snapshot.metrics.as_ref().and_then(|m| m.hostname.clone()),
            Some("pi5".to_string())
        );
        assert!(!snapshot.docker_available);
        assert!(snapshot.last_successful_refresh.is_some());
    }

    #[test]
    fn failed_probe_marks_stale_and_carries_forward_previous_data() {
        let previous_metrics = SystemMetrics {
            hostname: Some("pi5".into()),
            ..Default::default()
        };
        let previous = DeviceSnapshot {
            device_id: "pi5".into(),
            connection_status: DeviceConnectionStatus::Online,
            captured_at: "2026-01-01T00:00:00Z".into(),
            duration_ms: 50,
            metrics: Some(previous_metrics.clone()),
            docker_available: true,
            containers: vec![sample_container("abc")],
            warnings: Vec::new(),
            error: None,
            stale: false,
            last_successful_refresh: Some("2026-01-01T00:00:00Z".into()),
        };

        let executor = FakeRemoteExecutor::offline();
        let snapshot = refresh_device_sync(&executor, &sample_device(), Some(&previous));

        assert_eq!(snapshot.connection_status, DeviceConnectionStatus::Offline);
        assert!(snapshot.stale);
        assert!(snapshot.error.is_some());
        // Previous data is preserved, not erased.
        assert_eq!(snapshot.metrics, Some(previous_metrics));
        assert!(snapshot.docker_available);
        assert_eq!(snapshot.containers.len(), 1);
        // last_successful_refresh is carried forward unchanged.
        assert_eq!(
            snapshot.last_successful_refresh,
            Some("2026-01-01T00:00:00Z".to_string())
        );
    }

    #[test]
    fn failed_probe_with_no_previous_snapshot_has_no_stale_data_to_carry() {
        let executor = FakeRemoteExecutor::timeout();
        let snapshot = refresh_device_sync(&executor, &sample_device(), None);

        assert_eq!(snapshot.connection_status, DeviceConnectionStatus::Timeout);
        assert!(snapshot.stale);
        assert!(snapshot.metrics.is_none());
        assert!(snapshot.containers.is_empty());
        assert_eq!(snapshot.last_successful_refresh, None);
    }

    #[test]
    fn host_key_and_authentication_failures_are_distinguished() {
        let executor = FakeRemoteExecutor::host_key_error();
        let snapshot = refresh_device_sync(&executor, &sample_device(), None);
        assert_eq!(
            snapshot.connection_status,
            DeviceConnectionStatus::HostKeyError
        );

        let executor = FakeRemoteExecutor::authentication_error();
        let snapshot = refresh_device_sync(&executor, &sample_device(), None);
        assert_eq!(
            snapshot.connection_status,
            DeviceConnectionStatus::AuthenticationError
        );
    }

    #[test]
    fn docker_permission_denied_does_not_fail_the_whole_refresh() {
        // Simulate: probe + metrics succeed (same canned response), but
        // docker ps itself fails with a permission error via a custom
        // executor that returns different results per call.
        struct MixedExecutor;
        impl RemoteExecutor for MixedExecutor {
            fn execute(
                &self,
                _target: &SshTarget,
                command: &str,
                _timeout: Duration,
            ) -> Result<crate::infrastructure::ssh::RemoteExecutionResult, SshError> {
                if command.contains("docker") {
                    Err(SshError::RemoteCommandError {
                        exit_code: Some(1),
                        stderr: "permission denied while trying to connect to the Docker daemon"
                            .into(),
                    })
                } else {
                    Ok(crate::infrastructure::ssh::RemoteExecutionResult {
                        exit_code: Some(0),
                        stdout: "PIHUB_HOSTNAME=pi5\n".into(),
                        stderr: String::new(),
                        duration_ms: 5,
                        timed_out: false,
                    })
                }
            }
        }

        let snapshot = refresh_device_sync(&MixedExecutor, &sample_device(), None);

        assert_eq!(snapshot.connection_status, DeviceConnectionStatus::Online);
        assert!(!snapshot.stale);
        assert!(!snapshot.docker_available);
        assert!(snapshot.containers.is_empty());
        assert!(snapshot
            .warnings
            .iter()
            .any(|w| w.to_lowercase().contains("permission")));
        // The device itself is still online with valid metrics.
        assert!(snapshot.metrics.is_some());
    }
}
