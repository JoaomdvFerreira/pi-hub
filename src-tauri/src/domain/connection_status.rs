use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceConnectionStatus {
    Unknown,
    Checking,
    Online,
    Offline,
    Timeout,
    AuthenticationError,
    HostKeyError,
    CommandError,
}
