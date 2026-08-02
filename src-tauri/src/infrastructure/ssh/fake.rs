use std::time::Duration;

use super::error::SshError;
use super::executor::{RemoteExecutionResult, RemoteExecutor, SshTarget};

/// A `RemoteExecutor` test double that always returns a preconfigured
/// result, regardless of the target or command. Used by this crate's
/// tests (and later monitoring-scheduler tests) to simulate SSH outcomes
/// without a real SSH server, per the integration-test strategy in
/// docs/pi-hub-technical-architecture-specification.md section 25.3.
pub struct FakeRemoteExecutor {
    result: Result<RemoteExecutionResult, SshError>,
}

impl FakeRemoteExecutor {
    pub fn returning(result: Result<RemoteExecutionResult, SshError>) -> Self {
        Self { result }
    }

    pub fn online(stdout: impl Into<String>) -> Self {
        Self::returning(Ok(RemoteExecutionResult {
            exit_code: Some(0),
            stdout: stdout.into(),
            stderr: String::new(),
            duration_ms: 5,
            timed_out: false,
        }))
    }

    pub fn offline() -> Self {
        Self::returning(Err(SshError::ConnectionRefused))
    }

    pub fn timeout() -> Self {
        Self::returning(Err(SshError::RemoteCommandTimeout))
    }

    pub fn authentication_error() -> Self {
        Self::returning(Err(SshError::AuthenticationError))
    }

    pub fn host_key_error() -> Self {
        Self::returning(Err(SshError::HostKeyError))
    }
}

impl RemoteExecutor for FakeRemoteExecutor {
    fn execute(
        &self,
        _target: &SshTarget,
        _command: &str,
        _timeout: Duration,
    ) -> Result<RemoteExecutionResult, SshError> {
        self.result.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> SshTarget {
        SshTarget {
            host: "raspberrypi5.tail3f2a.ts.net".into(),
            port: 22,
            username: "joao".into(),
        }
    }

    #[test]
    fn simulates_online() {
        let executor = FakeRemoteExecutor::online("PIHUB_OK");
        let result = executor.probe(&target(), Duration::from_secs(5)).unwrap();
        assert_eq!(result.stdout, "PIHUB_OK");
    }

    #[test]
    fn simulates_offline() {
        let executor = FakeRemoteExecutor::offline();
        let err = executor.probe(&target(), Duration::from_secs(5)).unwrap_err();
        assert_eq!(err, SshError::ConnectionRefused);
        assert_eq!(err.to_connection_status(), crate::domain::connection_status::DeviceConnectionStatus::Offline);
    }

    #[test]
    fn simulates_timeout() {
        let executor = FakeRemoteExecutor::timeout();
        let err = executor.probe(&target(), Duration::from_secs(5)).unwrap_err();
        assert_eq!(err, SshError::RemoteCommandTimeout);
    }

    #[test]
    fn simulates_authentication_failure() {
        let executor = FakeRemoteExecutor::authentication_error();
        let err = executor.probe(&target(), Duration::from_secs(5)).unwrap_err();
        assert_eq!(err, SshError::AuthenticationError);
    }

    #[test]
    fn simulates_host_key_failure() {
        let executor = FakeRemoteExecutor::host_key_error();
        let err = executor.probe(&target(), Duration::from_secs(5)).unwrap_err();
        assert_eq!(err, SshError::HostKeyError);
    }
}
