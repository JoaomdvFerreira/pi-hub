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
    RemoteCommandError {
        exit_code: Option<i32>,
        stderr: String,
    },
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

    /// A short remediation hint safe to show directly in the UI. Never
    /// suggests bypassing host-key checking or supplying a password.
    pub fn remediation(&self) -> &'static str {
        match self {
            SshError::DnsResolutionError => {
                "The hostname could not be resolved. Check the hostname and your Tailscale connection."
            }
            SshError::ConnectionRefused => {
                "The device refused the connection. Check that it is powered on and the SSH port is correct."
            }
            SshError::ConnectionTimeout => {
                "The connection timed out. Check that the device is reachable over Tailscale or the local network."
            }
            SshError::AuthenticationError => {
                "Authentication failed. Confirm your SSH public key is authorized on the device and loaded in ssh-agent."
            }
            SshError::HostKeyError => {
                "The device's SSH host key could not be verified, or has changed. Verify it manually with ssh, then try again."
            }
            SshError::RemoteCommandError { .. } => {
                "The device connected but the remote command failed. Check the SSH user's permissions."
            }
            SshError::RemoteCommandTimeout => {
                "The remote command did not finish in time. The device may be overloaded or unreachable."
            }
            SshError::Spawn(_) => {
                "Could not launch the SSH client. Confirm the Windows OpenSSH client is installed."
            }
        }
    }
}

impl std::fmt::Display for SshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SshError::Spawn(reason) => write!(f, "could not launch ssh: {reason}"),
            SshError::DnsResolutionError => write!(f, "DNS resolution failed"),
            SshError::ConnectionRefused => write!(f, "connection refused"),
            SshError::ConnectionTimeout => write!(f, "connection timed out"),
            SshError::AuthenticationError => write!(f, "authentication failed"),
            SshError::HostKeyError => write!(f, "host key verification failed"),
            SshError::RemoteCommandError { exit_code, stderr } => {
                write!(f, "remote command failed (exit code {exit_code:?}): {stderr}")
            }
            SshError::RemoteCommandTimeout => write!(f, "remote command timed out"),
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
            SshError::RemoteCommandError {
                exit_code: Some(1),
                stderr: String::new()
            }
            .to_connection_status(),
            DeviceConnectionStatus::CommandError
        );
        assert_eq!(
            SshError::Spawn("ssh not found".into()).to_connection_status(),
            DeviceConnectionStatus::Unknown
        );
    }
}
