pub mod error;
pub mod executor;
pub mod openssh;
pub mod operation;
mod process;

pub mod fake;

// Re-exported for the M3 monitoring work units (snapshot/scheduler code
// that will match on SshError, hold a RemoteExecutionResult, or dispatch
// on RemoteOperation), which don't exist yet.
#[allow(unused_imports)]
pub use error::SshError;
#[allow(unused_imports)]
pub use executor::{RemoteExecutionResult, RemoteExecutor, RemoteOutputLimits, SshTarget};
pub use openssh::OpenSshExecutor;
#[allow(unused_imports)]
pub use operation::RemoteOperation;
pub use operation::{docker_container_action_command, docker_container_logs_command};
