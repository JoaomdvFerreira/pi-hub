use std::collections::BTreeMap;
use std::time::Duration;

use crate::domain::device_visibility::{DefaultRoute, MountedFilesystem, NetworkInterface, NetworkLinkState, NetworkVisibility, StorageVisibility, SystemVisibility};
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

const PSEUDO_FILESYSTEMS: &[&str] = &["proc", "sysfs", "tmpfs", "devtmpfs", "overlay", "squashfs", "cgroup", "cgroup2", "nsfs", "tracefs", "debugfs", "securityfs", "pstore"];

pub fn parse_storage_visibility(raw: &str) -> (StorageVisibility, Vec<ParseWarning>) {
    let mut filesystems = Vec::new();
    let mut warnings = Vec::new();
    for value in raw.lines().filter_map(|line| line.strip_prefix("PIHUB_STORAGE=")) {
        let parts: Vec<_> = value.split('|').collect();
        if parts.len() != 9 || parts[..3].iter().any(|part| part.is_empty()) {
            warnings.push(ParseWarning(format!("invalid storage record: '{value}'")));
            continue;
        }
        if PSEUDO_FILESYSTEMS.contains(&parts[2]) { continue; }
        let parse_number = |raw: &str, field: &str, warnings: &mut Vec<ParseWarning>| -> Option<u64> {
            if raw.is_empty() || raw == "-" { return None; }
            raw.parse().map_err(|_| warnings.push(ParseWarning(format!("invalid {field} in storage record: '{raw}'")))).ok()
        };
        let total_bytes = parse_number(parts[3], "total bytes", &mut warnings);
        let used_bytes = parse_number(parts[4], "used bytes", &mut warnings);
        let available_bytes = parse_number(parts[5], "available bytes", &mut warnings);
        let usage_percent = if parts[6].is_empty() || parts[6] == "-" { None } else { parts[6].parse::<u8>().ok().filter(|value| *value <= 100).or_else(|| { warnings.push(ParseWarning(format!("invalid usage percent in storage record: '{}'", parts[6]))); None }) };
        let read_only = match parts[7] { "ro" => Some(true), "rw" => Some(false), "-" | "" => None, other => { warnings.push(ParseWarning(format!("invalid storage access state: '{other}'"))); None } };
        filesystems.push(MountedFilesystem { source: parts[0].to_string(), mount_point: parts[1].to_string(), filesystem_type: parts[2].to_string(), total_bytes, used_bytes, available_bytes, usage_percent, read_only });
    }
    (StorageVisibility { filesystems }, warnings)
}

pub fn collect_storage_visibility(executor: &dyn RemoteExecutor, target: &SshTarget, timeout: Duration) -> Result<(StorageVisibility, Vec<ParseWarning>), SshError> {
    let command = RemoteOperation::StorageVisibility.command().expect("storage visibility command must be fixed");
    let result = executor.execute(target, command, timeout)?;
    Ok(parse_storage_visibility(&result.stdout))
}

pub fn parse_system_visibility(raw: &str) -> (SystemVisibility, Vec<ParseWarning>) {
    let mut fields = std::collections::HashMap::new(); let mut warnings = Vec::new();
    for line in raw.lines() { if let Some((key, value)) = line.split_once('=') { fields.insert(key, value); } }
    let number = |key: &str, warnings: &mut Vec<ParseWarning>| -> Option<u64> { fields.get(key).and_then(|value| value.parse().map_err(|_| warnings.push(ParseWarning(format!("invalid system value for {key}: '{value}'")))).ok()) };
    let cores = number("PIHUB_SYS_CORES", &mut warnings).and_then(|value| u32::try_from(value).ok());
    (SystemVisibility { hostname: fields.get("PIHUB_SYS_HOSTNAME").map(|v| (*v).to_string()), operating_system: fields.get("PIHUB_SYS_OS").map(|v| (*v).to_string()), kernel_version: fields.get("PIHUB_SYS_KERNEL").map(|v| (*v).to_string()), architecture: fields.get("PIHUB_SYS_ARCH").map(|v| (*v).to_string()), model: fields.get("PIHUB_SYS_MODEL").map(|v| (*v).to_string()), cpu_model: fields.get("PIHUB_SYS_CPU").map(|v| (*v).to_string()), logical_core_count: cores, total_memory_bytes: number("PIHUB_SYS_MEMORY_BYTES", &mut warnings), boot_timestamp: number("PIHUB_SYS_BOOT_TIMESTAMP", &mut warnings), uptime_seconds: number("PIHUB_SYS_UPTIME_SECONDS", &mut warnings) }, warnings)
}

pub fn collect_system_visibility(executor: &dyn RemoteExecutor, target: &SshTarget, timeout: Duration) -> Result<(SystemVisibility, Vec<ParseWarning>), SshError> {
    let result = executor.execute(target, RemoteOperation::SystemVisibility.command().expect("system visibility command must be fixed"), timeout)?;
    Ok(parse_system_visibility(&result.stdout))
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

    #[test]
    fn storage_filters_pseudo_mounts_and_preserves_unknown_capacity() {
        let raw = "PIHUB_STORAGE=/dev/mmcblk0p2|/|ext4|1000|400|600|40|ro|ignored\nPIHUB_STORAGE=overlay|/var/lib/docker|overlay|100|10|90|10|rw|ignored\nPIHUB_STORAGE=/dev/sda1|/media/usb|ext4|-|-|-|-|rw|ignored\n";
        let (storage, warnings) = parse_storage_visibility(raw);
        assert!(warnings.is_empty());
        assert_eq!(storage.filesystems.len(), 2);
        assert_eq!(storage.filesystems[0].mount_point, "/");
        assert_eq!(storage.filesystems[0].read_only, Some(true));
        assert_eq!(storage.filesystems[1].total_bytes, None);
    }

    #[test]
    fn system_parser_supports_pi_and_generic_linux_partial_data() {
        let (pi, warnings) = parse_system_visibility("PIHUB_SYS_HOSTNAME=pi5\nPIHUB_SYS_OS=Debian 12\nPIHUB_SYS_MODEL=Raspberry Pi 5\nPIHUB_SYS_CORES=4\nPIHUB_SYS_MEMORY_BYTES=8589934592\n");
        assert!(warnings.is_empty()); assert_eq!(pi.model.as_deref(), Some("Raspberry Pi 5")); assert_eq!(pi.logical_core_count, Some(4));
        let (linux, warnings) = parse_system_visibility("PIHUB_SYS_OS=Ubuntu\nPIHUB_SYS_CORES=bad\n");
        assert_eq!(linux.operating_system.as_deref(), Some("Ubuntu")); assert_eq!(linux.model, None); assert_eq!(warnings.len(), 1);
    }
}
