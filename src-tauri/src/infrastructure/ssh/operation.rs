/// The fixed set of remote operations Pi-Hub is ever allowed to run. The
/// frontend cannot supply arbitrary commands: every command string
/// executed over SSH is one of these predefined variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteOperation {
    Probe,
    // Reserved for the M3 monitoring work units, which pair the command
    // body with the parser that consumes its output; not constructed yet.
    #[allow(dead_code)]
    SystemIdentity,
    #[allow(dead_code)]
    SystemMetrics,
    #[allow(dead_code)]
    DockerContainers,
}

pub const PROBE_COMMAND: &str = "printf 'PIHUB_OK'";

impl RemoteOperation {
    /// The fixed remote shell command for this operation, if defined yet.
    pub fn command(&self) -> Option<&'static str> {
        match self {
            RemoteOperation::Probe => Some(PROBE_COMMAND),
            RemoteOperation::SystemIdentity
            | RemoteOperation::SystemMetrics
            | RemoteOperation::DockerContainers => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_has_a_fixed_command() {
        assert_eq!(RemoteOperation::Probe.command(), Some(PROBE_COMMAND));
    }

    #[test]
    fn m3_operations_are_reserved_and_undefined_for_now() {
        assert_eq!(RemoteOperation::SystemIdentity.command(), None);
        assert_eq!(RemoteOperation::SystemMetrics.command(), None);
        assert_eq!(RemoteOperation::DockerContainers.command(), None);
    }
}
