use serde::{Deserialize, Serialize};

/// Read-only network context captured during a device refresh. Empty lists
/// mean the command succeeded without entries; missing fields remain unknown.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetworkVisibility {
    pub interfaces: Vec<NetworkInterface>,
    pub default_route: Option<DefaultRoute>,
    pub dns_servers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInterface {
    pub name: String,
    pub link_state: NetworkLinkState,
    pub mac_address: Option<String>,
    pub ipv4_addresses: Vec<String>,
    pub ipv6_addresses: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NetworkLinkState { Up, Down, Unknown }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DefaultRoute {
    pub interface: String,
    pub gateway: Option<String>,
}
