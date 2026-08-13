use std::time::Duration;

use super::error::SshError;
use super::operation::RemoteOperation;

#[derive(Debug, Clone)]
pub struct SshTarget {
    pub host: String,
    pub port: u16,
    pub username: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteExecutionResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub timed_out: bool,
}

/// Hard per-stream output limits for remote evidence capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteOutputLimits {
    pub stdout: usize,
    pub stderr: usize,
}

/// Executes a fixed command over SSH against a device. Implemented for
/// real by `OpenSshExecutor`; tests substitute a fake implementation to
/// simulate online/offline/timeout/authentication/host-key scenarios
/// without a real SSH server.
pub trait RemoteExecutor: Send + Sync {
    fn execute(
        &self,
        target: &SshTarget,
        command: &str,
        timeout: Duration,
    ) -> Result<RemoteExecutionResult, SshError>;

    /// Executes with hard output limits. Production SSH execution enforces
    /// these while reading process pipes; the default keeps deterministic
    /// executors contract-compatible.
    fn execute_bounded(
        &self,
        target: &SshTarget,
        command: &str,
        timeout: Duration,
        limits: RemoteOutputLimits,
    ) -> Result<RemoteExecutionResult, SshError> {
        let result = self.execute(target, command, timeout)?;
        if result.stdout.len() > limits.stdout || result.stderr.len() > limits.stderr {
            Err(SshError::OutputLimitExceeded)
        } else {
            Ok(result)
        }
    }

    /// Connectivity probe: verifies DNS resolution, network reachability,
    /// SSH availability, host-key acceptance, and authentication.
    fn probe(
        &self,
        target: &SshTarget,
        timeout: Duration,
    ) -> Result<RemoteExecutionResult, SshError> {
        let command = RemoteOperation::Probe
            .command()
            .expect("RemoteOperation::Probe always has a command");
        self.execute(target, command, timeout)
    }
}
