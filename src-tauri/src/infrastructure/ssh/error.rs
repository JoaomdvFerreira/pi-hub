use crate::domain::connection_status::DeviceConnectionStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshError {
    /// The ssh client itself could not be spawned (e.g. missing from PATH).
    Spawn(String),
    DnsResolutionError,
    ConnectionRefused,
    ConnectionTimeout,
    AuthenticationError,
    /// Host-key verification failed, or the remote host key changed.
    /// Pi-Hub never auto-resolves this: it is always surfaced as-is.
    HostKeyError,
    RemoteCommandError { exit_code: Option<i32> },
    RemoteCommandTimeout,
}

impl SshError {
    pub fn to_connection_status(&self) -> DeviceConnectionStatus {
        match self {
            SshError::DnsResolutionError | SshError::ConnectionRefused => {
                DeviceConnectionStatus::Offline
            }
            SshError::ConnectionTimeout | SshError::RemoteCommandTimeout => {
                DeviceConnectionStatus::Timeout
            }
            SshError::AuthenticationError => DeviceConnectionStatus::AuthenticationError,
            SshError::HostKeyError => DeviceConnectionStatus::HostKeyError,
            SshError::RemoteCommandError { .. } => DeviceConnectionStatus::CommandError,
            SshError::Spawn(_) => DeviceConnectionStatus::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_key_error_never_maps_to_a_resolved_status() {
        // Guards the "never auto-resolved or bypassed" requirement: a
        // host-key failure must always surface as HostKeyError, never as
        // Online/Offline/Unknown, which would imply it was silently handled.
        assert_eq!(
            SshError::HostKeyError.to_connection_status(),
            DeviceConnectionStatus::HostKeyError
        );
    }

    #[test]
    fn maps_each_error_to_its_expected_status() {
        assert_eq!(
            SshError::DnsResolutionError.to_connection_status(),
            DeviceConnectionStatus::Offline
        );
        assert_eq!(
            SshError::ConnectionRefused.to_connection_status(),
            DeviceConnectionStatus::Offline
        );
        assert_eq!(
            SshError::ConnectionTimeout.to_connection_status(),
            DeviceConnectionStatus::Timeout
        );
        assert_eq!(
            SshError::RemoteCommandTimeout.to_connection_status(),
            DeviceConnectionStatus::Timeout
        );
        assert_eq!(
            SshError::AuthenticationError.to_connection_status(),
            DeviceConnectionStatus::AuthenticationError
        );
        assert_eq!(
            SshError::RemoteCommandError { exit_code: Some(1) }.to_connection_status(),
            DeviceConnectionStatus::CommandError
        );
        assert_eq!(
            SshError::Spawn("ssh not found".into()).to_connection_status(),
            DeviceConnectionStatus::Unknown
        );
    }
}
