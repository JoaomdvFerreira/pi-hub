use std::process::Command;
use std::time::Duration;

use super::error::SshError;
use super::executor::{RemoteExecutionResult, RemoteExecutor, RemoteOutputLimits, SshTarget};
use super::process::{run_with_timeout, run_with_timeout_and_output_limits, OutputLimits, RawProcessOutcome};

pub struct OpenSshExecutor {
    connect_timeout: Duration,
}

impl OpenSshExecutor {
    pub fn new(connect_timeout: Duration) -> Self {
        Self { connect_timeout }
    }

    fn execute_with_limits(
        &self,
        target: &SshTarget,
        command: &str,
        timeout: Duration,
        limits: Option<RemoteOutputLimits>,
    ) -> Result<RemoteExecutionResult, SshError> {
        let mut measurement = crate::performance_diagnostics::measure("ssh.execute");
        let connect_timeout_secs = self.connect_timeout.as_secs().max(1).to_string();
        let destination = format!("{}@{}", target.username, target.host);
        let port = target.port.to_string();
        let mut cmd = Command::new("ssh");
        cmd.args([
            "-o", "BatchMode=yes", "-o", &format!("ConnectTimeout={connect_timeout_secs}"),
            "-p", &port, &destination, command,
        ]);
        let outcome = match limits {
            Some(limits) => run_with_timeout_and_output_limits(
                &mut cmd,
                timeout,
                OutputLimits { stdout: limits.stdout, stderr: limits.stderr },
            ),
            None => run_with_timeout(&mut cmd, timeout),
        }.map_err(|err| {
            if let Some(item) = measurement.as_mut() { item.fail(); }
            SshError::Spawn(err.to_string())
        })?;
        let result = classify_outcome(outcome);
        if let Ok(execution) = &result {
            if let Some(item) = measurement.as_mut() {
                item.set_bytes((execution.stdout.len() + execution.stderr.len()) as u64);
            }
        } else if let Some(item) = measurement.as_mut() {
            item.fail();
        }
        if matches!(result, Err(SshError::RemoteCommandTimeout | SshError::ConnectionTimeout)) {
            let _timeout = crate::performance_diagnostics::measure("ssh.timeout");
        }
        result
    }
}

impl Default for OpenSshExecutor {
    fn default() -> Self {
        Self::new(Duration::from_secs(5))
    }
}

impl RemoteExecutor for OpenSshExecutor {
    fn execute(
        &self,
        target: &SshTarget,
        command: &str,
        timeout: Duration,
    ) -> Result<RemoteExecutionResult, SshError> {
        self.execute_with_limits(target, command, timeout, None)
    }

    fn execute_bounded(
        &self,
        target: &SshTarget,
        command: &str,
        timeout: Duration,
        limits: RemoteOutputLimits,
    ) -> Result<RemoteExecutionResult, SshError> {
        self.execute_with_limits(target, command, timeout, Some(limits))
    }
}

fn classify_outcome(raw: RawProcessOutcome) -> Result<RemoteExecutionResult, SshError> {
    if raw.output_limit_exceeded {
        return Err(SshError::OutputLimitExceeded);
    }
    if raw.timed_out {
        return Err(SshError::RemoteCommandTimeout);
    }

    let lower_stderr = raw.stderr.to_lowercase();

    if raw.exit_code == Some(255) {
        if lower_stderr.contains("could not resolve hostname") {
            return Err(SshError::DnsResolutionError);
        }
        if lower_stderr.contains("host key verification failed")
            || lower_stderr.contains("remote host identification has changed")
        {
            return Err(SshError::HostKeyError);
        }
        if lower_stderr.contains("permission denied") {
            return Err(SshError::AuthenticationError);
        }
        if lower_stderr.contains("connection timed out")
            || lower_stderr.contains("operation timed out")
        {
            return Err(SshError::ConnectionTimeout);
        }
        // Connection refused, "no route to host", and any other
        // unclassified connection-level failure surface as offline.
        return Err(SshError::ConnectionRefused);
    }

    if raw.exit_code != Some(0) {
        return Err(SshError::RemoteCommandError {
            exit_code: raw.exit_code,
            stderr: raw.stderr,
        });
    }

    Ok(RemoteExecutionResult {
        exit_code: raw.exit_code,
        stdout: raw.stdout,
        stderr: raw.stderr,
        duration_ms: raw.duration_ms,
        timed_out: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(exit_code: Option<i32>, stderr: &str) -> RawProcessOutcome {
        RawProcessOutcome {
            exit_code,
            stdout: String::new(),
            stderr: stderr.into(),
            duration_ms: 10,
            timed_out: false,
            output_limit_exceeded: false,
        }
    }

    #[test]
    fn classifies_dns_resolution_failure() {
        let outcome = raw(Some(255), "ssh: Could not resolve hostname bogus: nodename");
        assert_eq!(classify_outcome(outcome), Err(SshError::DnsResolutionError));
    }

    #[test]
    fn classifies_host_key_verification_failure() {
        let outcome = raw(Some(255), "Host key verification failed.");
        assert_eq!(classify_outcome(outcome), Err(SshError::HostKeyError));
    }

    #[test]
    fn classifies_changed_host_key_as_host_key_error() {
        let outcome = raw(
            Some(255),
            "@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@\nREMOTE HOST IDENTIFICATION HAS CHANGED!",
        );
        assert_eq!(classify_outcome(outcome), Err(SshError::HostKeyError));
    }

    #[test]
    fn classifies_permission_denied_as_authentication_error() {
        let outcome = raw(
            Some(255),
            "joao@raspberrypi5: Permission denied (publickey).",
        );
        assert_eq!(
            classify_outcome(outcome),
            Err(SshError::AuthenticationError)
        );
    }

    #[test]
    fn classifies_connection_timeout() {
        let outcome = raw(
            Some(255),
            "ssh: connect to host raspberrypi5 port 22: Connection timed out",
        );
        assert_eq!(classify_outcome(outcome), Err(SshError::ConnectionTimeout));
    }

    #[test]
    fn classifies_unrecognized_connection_failure_as_connection_refused() {
        let outcome = raw(
            Some(255),
            "ssh: connect to host raspberrypi5 port 22: Connection refused",
        );
        assert_eq!(classify_outcome(outcome), Err(SshError::ConnectionRefused));
    }

    #[test]
    fn classifies_nonzero_non_255_exit_as_remote_command_error() {
        let outcome = raw(Some(127), "sh: docker: command not found");
        assert_eq!(
            classify_outcome(outcome),
            Err(SshError::RemoteCommandError {
                exit_code: Some(127),
                stderr: "sh: docker: command not found".into()
            })
        );
    }

    #[test]
    fn classifies_timed_out_outcome_as_remote_command_timeout_regardless_of_exit_code() {
        let mut outcome = raw(None, "");
        outcome.timed_out = true;
        assert_eq!(
            classify_outcome(outcome),
            Err(SshError::RemoteCommandTimeout)
        );
    }

    #[test]
    fn classifies_bounded_output_without_retaining_it() {
        let mut outcome = raw(None, "");
        outcome.output_limit_exceeded = true;
        assert_eq!(classify_outcome(outcome), Err(SshError::OutputLimitExceeded));
    }

    #[test]
    fn classifies_successful_command_as_ok() {
        let mut outcome = raw(Some(0), "");
        outcome.stdout = "PIHUB_OK".into();
        let result = classify_outcome(outcome).unwrap();
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.stdout, "PIHUB_OK");
        assert!(!result.timed_out);
    }
}
