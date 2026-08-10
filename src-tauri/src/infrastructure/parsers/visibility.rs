use std::collections::BTreeMap;
use std::time::Duration;

use crate::domain::device_visibility::{DefaultRoute, NetworkInterface, NetworkLinkState, NetworkVisibility};
use crate::infrastructure::parsers::key_value::ParseWarning;
use crate::infrastructure::ssh::{RemoteExecutor, RemoteOperation, SshError, SshTarget};

/// Parse the fixed M11 network payload. Individual malformed records are
/// reported and ignored so a useful interface snapshot survives partial data.
pub fn parse_network_visibility(raw: &str) -> (NetworkVisibility, Vec<ParseWarning>) {
    let mut interfaces: BTreeMap<String, NetworkInterface> = BTreeMap::new();
    let mut default_route = None;
    let mut dns_servers = Vec::new();
    let mut warnings = Vec::new();

    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        let Some((key, value)) = line.split_once('=') else { continue };
        match key {
            "PIHUB_NET_LINK" => {
                let parts: Vec<_> = value.split('|').collect();
                if parts.len() != 3 || parts[0].is_empty() {
                    warnings.push(ParseWarning(format!("invalid network link record: '{value}'")));
                    continue;
                }
                let state = match parts[1] { "up" => NetworkLinkState::Up, "down" => NetworkLinkState::Down, _ => NetworkLinkState::Unknown };
                let entry = interfaces.entry(parts[0].to_string()).or_insert_with(|| NetworkInterface { name: parts[0].to_string(), link_state: NetworkLinkState::Unknown, mac_address: None, ipv4_addresses: Vec::new(), ipv6_addresses: Vec::new() });
                entry.link_state = state;
                entry.mac_address = (!parts[2].is_empty()).then(|| parts[2].to_string());
            }
            "PIHUB_NET_ADDR" => {
                let parts: Vec<_> = value.split('|').collect();
                if parts.len() != 3 || parts[0].is_empty() || parts[2].is_empty() || !matches!(parts[1], "4" | "6") {
                    warnings.push(ParseWarning(format!("invalid network address record: '{value}'")));
                    continue;
                }
                let entry = interfaces.entry(parts[0].to_string()).or_insert_with(|| NetworkInterface { name: parts[0].to_string(), link_state: NetworkLinkState::Unknown, mac_address: None, ipv4_addresses: Vec::new(), ipv6_addresses: Vec::new() });
                if parts[1] == "4" { entry.ipv4_addresses.push(parts[2].to_string()); } else { entry.ipv6_addresses.push(parts[2].to_string()); }
            }
            "PIHUB_NET_ROUTE" => {
                let parts: Vec<_> = value.split('|').collect();
                if parts.len() != 2 || parts[0].is_empty() { warnings.push(ParseWarning(format!("invalid default route record: '{value}'"))); }
                else { default_route = Some(DefaultRoute { interface: parts[0].to_string(), gateway: (!parts[1].is_empty()).then(|| parts[1].to_string()) }); }
            }
            "PIHUB_NET_DNS" if value.is_empty() => warnings.push(ParseWarning("invalid empty DNS record".to_string())),
            "PIHUB_NET_DNS" if !dns_servers.contains(&value.to_string()) => dns_servers.push(value.to_string()),
            "PIHUB_NET_DNS" => {},
            _ => {}
        }
    }
    (NetworkVisibility { interfaces: interfaces.into_values().collect(), default_route, dns_servers }, warnings)
}

pub fn collect_network_visibility(executor: &dyn RemoteExecutor, target: &SshTarget, timeout: Duration) -> Result<(NetworkVisibility, Vec<ParseWarning>), SshError> {
    let command = RemoteOperation::NetworkVisibility.command().expect("network visibility command must be fixed");
    let result = executor.execute(target, command, timeout)?;
    Ok(parse_network_visibility(&result.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multiple_interfaces_ipv4_ipv6_route_and_dns() {
        let raw = "PIHUB_NET_LINK=eth0|up|dc:a6:32:00:00:01\nPIHUB_NET_ADDR=eth0|4|192.168.1.10/24\nPIHUB_NET_ADDR=eth0|6|2001:db8::10/64\nPIHUB_NET_LINK=lo|unknown|\nPIHUB_NET_ADDR=lo|4|127.0.0.1/8\nPIHUB_NET_ROUTE=eth0|192.168.1.1\nPIHUB_NET_DNS=1.1.1.1\nPIHUB_NET_DNS=1.1.1.1\n";
        let (network, warnings) = parse_network_visibility(raw);
        assert!(warnings.is_empty());
        assert_eq!(network.interfaces.len(), 2);
        assert_eq!(network.interfaces[0].ipv4_addresses, ["192.168.1.10/24"]);
        assert_eq!(network.interfaces[0].ipv6_addresses, ["2001:db8::10/64"]);
        assert_eq!(network.default_route.unwrap().gateway.as_deref(), Some("192.168.1.1"));
        assert_eq!(network.dns_servers, ["1.1.1.1"]);
    }

    #[test]
    fn malformed_records_do_not_discard_valid_network_data() {
        let (network, warnings) = parse_network_visibility("PIHUB_NET_LINK=eth0|up|aa:bb:cc:dd:ee:ff\nPIHUB_NET_ADDR=bad\nPIHUB_NET_ROUTE=|\n");
        assert_eq!(network.interfaces.len(), 1);
        assert_eq!(warnings.len(), 2);
        assert!(network.default_route.is_none());
    }
}
