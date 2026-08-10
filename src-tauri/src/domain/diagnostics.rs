use crate::infrastructure::ssh::SshError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticStatus {
    Passed,
    Warning,
    Failed,
    Skipped,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticCheck {
    pub code: String,
    pub status: DiagnosticStatus,
    pub summary: String,
    pub detail: Option<String>,
    pub duration_ms: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectivityDiagnosticReport {
    pub checks: Vec<DiagnosticCheck>,
    pub duration_ms: u64,
}

fn check(
    code: &str,
    status: DiagnosticStatus,
    summary: &str,
    duration_ms: Option<u64>,
) -> DiagnosticCheck {
    DiagnosticCheck {
        code: code.into(),
        status,
        summary: summary.into(),
        detail: None,
        duration_ms,
    }
}
pub fn diagnostic_failure(error: &SshError) -> ConnectivityDiagnosticReport {
    let (code, summary) = match error {
        SshError::DnsResolutionError => {
            ("target_resolution", "Target hostname could not be resolved")
        }
        SshError::ConnectionRefused => ("tcp_connection", "SSH port could not be reached"),
        SshError::ConnectionTimeout | SshError::RemoteCommandTimeout => {
            ("tcp_connection", "SSH connection timed out")
        }
        SshError::HostKeyError => ("ssh_host_key", "SSH host key verification failed"),
        SshError::AuthenticationError => ("ssh_authentication", "SSH authentication failed"),
        SshError::RemoteCommandError { .. } => {
            ("remote_command", "Remote diagnostic command failed")
        }
        SshError::Spawn(_) => ("ssh_client", "SSH client could not be launched"),
    };
    let stages = [
        "target_resolution",
        "tcp_connection",
        "ssh_host_key",
        "ssh_authentication",
        "remote_command",
        "docker",
        "tailscale",
    ];
    let failed_index = stages.iter().position(|stage| *stage == code).unwrap_or(0);
    let checks = stages
        .iter()
        .enumerate()
        .map(|(index, stage)| {
            if index < failed_index {
                check(stage, DiagnosticStatus::Passed, "Completed", None)
            } else if index == failed_index {
                check(stage, DiagnosticStatus::Failed, summary, None)
            } else {
                check(
                    stage,
                    DiagnosticStatus::Skipped,
                    "Skipped because an earlier connection stage failed",
                    None,
                )
            }
        })
        .collect();
    ConnectivityDiagnosticReport {
        checks,
        duration_ms: 0,
    }
}
pub fn diagnostic_success(stdout: &str, duration_ms: u64) -> ConnectivityDiagnosticReport {
    let docker = if stdout.contains("PIHUB_DIAG_DOCKER=1") {
        DiagnosticStatus::Passed
    } else {
        DiagnosticStatus::Warning
    };
    let tailscale = if stdout.contains("PIHUB_DIAG_TAILSCALE=1") {
        DiagnosticStatus::Passed
    } else {
        DiagnosticStatus::Warning
    };
    let docker_ok = docker == DiagnosticStatus::Passed;
    let tailscale_ok = tailscale == DiagnosticStatus::Passed;
    ConnectivityDiagnosticReport {
        duration_ms,
        checks: vec![
            check(
                "target_resolution",
                DiagnosticStatus::Passed,
                "Target resolved",
                Some(duration_ms),
            ),
            check(
                "tcp_connection",
                DiagnosticStatus::Passed,
                "SSH port reached",
                Some(duration_ms),
            ),
            check(
                "ssh_host_key",
                DiagnosticStatus::Passed,
                "SSH host key verified",
                Some(duration_ms),
            ),
            check(
                "ssh_authentication",
                DiagnosticStatus::Passed,
                "SSH authentication succeeded",
                Some(duration_ms),
            ),
            check(
                "remote_command",
                DiagnosticStatus::Passed,
                "Read-only diagnostic command completed",
                Some(duration_ms),
            ),
            check(
                "docker",
                docker,
                if docker_ok {
                    "Docker is available"
                } else {
                    "Docker is unavailable"
                },
                None,
            ),
            check(
                "tailscale",
                tailscale,
                if tailscale_ok {
                    "Tailscale is available"
                } else {
                    "Tailscale is unavailable"
                },
                None,
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn connection_failure_skips_downstream() {
        let report = diagnostic_failure(&SshError::HostKeyError);
        assert_eq!(report.checks[2].status, DiagnosticStatus::Failed);
        assert_eq!(report.checks[3].status, DiagnosticStatus::Skipped);
    }
    #[test]
    fn docker_and_tailscale_are_non_fatal() {
        let report = diagnostic_success("PIHUB_DIAG_DOCKER=0\nPIHUB_DIAG_TAILSCALE=0", 5);
        assert_eq!(report.checks[5].status, DiagnosticStatus::Warning);
        assert_eq!(report.checks[6].status, DiagnosticStatus::Warning);
    }
}
