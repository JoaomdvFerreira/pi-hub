use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use super::key_value::{parse_key_value_payload, ParseWarning};
use crate::domain::system_metrics::SystemMetrics;
use crate::infrastructure::ssh::{RemoteExecutor, RemoteOperation, SshError, SshTarget};

/// Parses a `PIHUB_*` key-value payload (as produced by the
/// `RemoteOperation::SystemMetrics` command) into `SystemMetrics`. Unknown
/// fields are ignored, invalid numeric values are dropped (with a
/// warning), and every other valid field is preserved.
pub fn parse_system_metrics(raw: &str) -> (SystemMetrics, Vec<ParseWarning>) {
    let (fields, mut warnings) = parse_key_value_payload(raw);

    let mut metrics = SystemMetrics {
        hostname: fields.get("PIHUB_HOSTNAME").cloned(),
        model: fields.get("PIHUB_MODEL").cloned(),
        operating_system: fields.get("PIHUB_OS").cloned(),
        kernel_version: fields.get("PIHUB_KERNEL").cloned(),
        architecture: fields.get("PIHUB_ARCH").cloned(),
        ..Default::default()
    };

    metrics.uptime_seconds = parse_numeric(&fields, "PIHUB_UPTIME_SECONDS", &mut warnings);
    metrics.cpu_usage_percent = parse_numeric(&fields, "PIHUB_CPU_USAGE_PERCENT", &mut warnings);
    metrics.load_average_1m = parse_numeric(&fields, "PIHUB_LOAD_1M", &mut warnings);
    metrics.load_average_5m = parse_numeric(&fields, "PIHUB_LOAD_5M", &mut warnings);
    metrics.load_average_15m = parse_numeric(&fields, "PIHUB_LOAD_15M", &mut warnings);

    metrics.memory_total_bytes = parse_numeric(&fields, "PIHUB_MEMORY_TOTAL_BYTES", &mut warnings);
    let memory_available: Option<u64> =
        parse_numeric(&fields, "PIHUB_MEMORY_AVAILABLE_BYTES", &mut warnings);
    metrics.memory_used_bytes = derive_used(metrics.memory_total_bytes, memory_available);

    metrics.disk_total_bytes = parse_numeric(&fields, "PIHUB_DISK_TOTAL_BYTES", &mut warnings);
    let disk_available: Option<u64> =
        parse_numeric(&fields, "PIHUB_DISK_AVAILABLE_BYTES", &mut warnings);
    metrics.disk_used_bytes = derive_used(metrics.disk_total_bytes, disk_available);

    metrics.temperature_celsius = parse_numeric::<f64>(&fields, "PIHUB_TEMP_MILLIC", &mut warnings)
        .map(|millic| millic / 1000.0);

    (metrics, warnings)
}

/// Runs the fixed SystemMetrics remote operation over SSH and parses the
/// result. A connection/command-level failure (offline, timeout, auth,
/// host-key, ...) is returned as `SshError`; anything that happens after a
/// successful connection is folded into `SystemMetrics`'s optional fields
/// and the returned warnings instead of failing.
pub fn collect_system_metrics(
    executor: &dyn RemoteExecutor,
    target: &SshTarget,
    timeout: Duration,
) -> Result<(SystemMetrics, Vec<ParseWarning>), SshError> {
    let command = RemoteOperation::SystemMetrics
        .command()
        .expect("RemoteOperation::SystemMetrics must have a command");
    let result = executor.execute(target, command, timeout)?;
    Ok(parse_system_metrics(&result.stdout))
}

fn derive_used(total: Option<u64>, available: Option<u64>) -> Option<u64> {
    match (total, available) {
        (Some(total), Some(available)) if available <= total => Some(total - available),
        _ => None,
    }
}

fn parse_numeric<T: FromStr>(
    fields: &HashMap<String, String>,
    key: &str,
    warnings: &mut Vec<ParseWarning>,
) -> Option<T> {
    let raw = fields.get(key)?;
    match raw.parse::<T>() {
        Ok(value) => Some(value),
        Err(_) => {
            warnings.push(ParseWarning(format!(
                "invalid numeric value for '{key}': '{raw}'"
            )));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ssh::fake::FakeRemoteExecutor;

    const WELL_FORMED: &str = "\
PIHUB_HOSTNAME=raspberrypi5
PIHUB_MODEL=Raspberry Pi 5 Model B Rev 1.0
PIHUB_OS=Debian GNU/Linux 12 (bookworm)
PIHUB_KERNEL=6.6.31+rpt-rpi-2712
PIHUB_ARCH=aarch64
PIHUB_UPTIME_SECONDS=84922
PIHUB_CPU_USAGE_PERCENT=18
PIHUB_LOAD_1M=0.42
PIHUB_LOAD_5M=0.38
PIHUB_LOAD_15M=0.30
PIHUB_MEMORY_TOTAL_BYTES=8589934592
PIHUB_MEMORY_AVAILABLE_BYTES=6432382976
PIHUB_DISK_TOTAL_BYTES=250000000000
PIHUB_DISK_AVAILABLE_BYTES=183000000000
PIHUB_TEMP_MILLIC=45200
";

    #[test]
    fn parses_a_well_formed_payload_completely() {
        let (metrics, warnings) = parse_system_metrics(WELL_FORMED);

        assert!(warnings.is_empty());
        assert_eq!(metrics.hostname.as_deref(), Some("raspberrypi5"));
        assert_eq!(metrics.model.as_deref(), Some("Raspberry Pi 5 Model B Rev 1.0"));
        assert_eq!(metrics.uptime_seconds, Some(84922));
        assert_eq!(metrics.cpu_usage_percent, Some(18.0));
        assert_eq!(metrics.load_average_1m, Some(0.42));
        assert_eq!(metrics.memory_total_bytes, Some(8_589_934_592));
        assert_eq!(metrics.memory_used_bytes, Some(8_589_934_592 - 6_432_382_976));
        assert_eq!(metrics.disk_total_bytes, Some(250_000_000_000));
        assert_eq!(metrics.disk_used_bytes, Some(250_000_000_000 - 183_000_000_000));
        assert_eq!(metrics.temperature_celsius, Some(45.2));
    }

    #[test]
    fn empty_payload_yields_all_none_and_no_warnings() {
        let (metrics, warnings) = parse_system_metrics("");
        assert_eq!(metrics, SystemMetrics::default());
        assert!(warnings.is_empty());
    }

    #[test]
    fn missing_optional_fields_do_not_fail_collection() {
        // No temperature line at all -- common on devices without a
        // thermal zone -- must not affect any other field.
        let payload = "PIHUB_HOSTNAME=pi2\nPIHUB_UPTIME_SECONDS=100\n";
        let (metrics, warnings) = parse_system_metrics(payload);
        assert_eq!(metrics.hostname.as_deref(), Some("pi2"));
        assert_eq!(metrics.uptime_seconds, Some(100));
        assert_eq!(metrics.temperature_celsius, None);
        assert!(warnings.is_empty());
    }

    #[test]
    fn invalid_numeric_value_is_dropped_with_a_warning_but_other_fields_survive() {
        let payload =
            "PIHUB_HOSTNAME=pi5\nPIHUB_UPTIME_SECONDS=not-a-number\nPIHUB_CPU_USAGE_PERCENT=22\n";
        let (metrics, warnings) = parse_system_metrics(payload);

        assert_eq!(metrics.hostname.as_deref(), Some("pi5"));
        assert_eq!(metrics.uptime_seconds, None);
        assert_eq!(metrics.cpu_usage_percent, Some(22.0));
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn malformed_and_unrecognized_lines_produce_warnings_without_discarding_valid_fields() {
        let payload = "\
garbage line with no equals
UNKNOWN_FIELD=123
PIHUB_HOSTNAME=pi5
PIHUB_UPTIME_SECONDS=300
";
        let (metrics, warnings) = parse_system_metrics(payload);

        assert_eq!(metrics.hostname.as_deref(), Some("pi5"));
        assert_eq!(metrics.uptime_seconds, Some(300));
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn memory_used_is_none_if_available_exceeds_total() {
        // Guards against a malformed/inconsistent script output producing
        // a nonsensical negative "used" value.
        let payload = "PIHUB_MEMORY_TOTAL_BYTES=100\nPIHUB_MEMORY_AVAILABLE_BYTES=200\n";
        let (metrics, _) = parse_system_metrics(payload);
        assert_eq!(metrics.memory_used_bytes, None);
    }

    #[test]
    fn memory_used_is_none_when_only_total_is_present() {
        let payload = "PIHUB_MEMORY_TOTAL_BYTES=100\n";
        let (metrics, _) = parse_system_metrics(payload);
        assert_eq!(metrics.memory_used_bytes, None);
    }

    #[test]
    fn temperature_is_converted_from_millidegrees() {
        let (metrics, _) = parse_system_metrics("PIHUB_TEMP_MILLIC=52000\n");
        assert_eq!(metrics.temperature_celsius, Some(52.0));
    }

    #[test]
    fn invalid_temperature_value_is_dropped_with_a_warning() {
        let (metrics, warnings) = parse_system_metrics("PIHUB_TEMP_MILLIC=hot\n");
        assert_eq!(metrics.temperature_celsius, None);
        assert_eq!(warnings.len(), 1);
    }

    fn target() -> SshTarget {
        SshTarget {
            host: "raspberrypi5.tail3f2a.ts.net".into(),
            port: 22,
            username: "joao".into(),
        }
    }

    #[test]
    fn collect_system_metrics_parses_a_successful_probe() {
        let executor = FakeRemoteExecutor::online(WELL_FORMED);
        let (metrics, warnings) =
            collect_system_metrics(&executor, &target(), Duration::from_secs(10)).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(metrics.hostname.as_deref(), Some("raspberrypi5"));
    }

    #[test]
    fn collect_system_metrics_propagates_connection_failure() {
        let executor = FakeRemoteExecutor::offline();
        let err = collect_system_metrics(&executor, &target(), Duration::from_secs(10))
            .unwrap_err();
        assert_eq!(err, SshError::ConnectionRefused);
    }
}
