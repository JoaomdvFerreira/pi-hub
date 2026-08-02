// This module is complete and unit-tested on its own, but nothing calls
// into it yet -- the connection-test command that wires it up is WU008.
#![allow(dead_code, unused_imports)]

pub mod error;
pub mod executor;
pub mod openssh;
pub mod operation;
mod process;

#[cfg(test)]
pub mod fake;

pub use error::SshError;
pub use executor::{RemoteExecutionResult, RemoteExecutor, SshTarget};
pub use openssh::OpenSshExecutor;
pub use operation::RemoteOperation;
