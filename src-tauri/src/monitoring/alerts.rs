use std::collections::HashSet;

use chrono::DateTime;

use crate::domain::{
    alert::{
        Alert, AlertCategory, AlertResolutionReason, AlertSeverity, AlertState, AlertTransition,
        AlertTransitionKind,
    },
    connection_status::DeviceConnectionStatus,
    device::Device,
    health::HealthReasonSeverity,
    service_health::ServiceHealthState,
    settings::ThresholdPolicy,
    snapshot::DeviceSnapshot,
};
use crate::storage::alert_repository::{resolve_matching, AlertFile, CandidateState};

struct Condition {
    key: String,
    category: AlertCategory,
    entity_id: Option<String>,
    code: String,
    severity: AlertSeverity,
    summary: String,
    detail: Option<String>,
    threshold: Option<String>,
    duration_seconds: Option<u32>,
    consecutive_samples: Option<u32>,
}

#[cfg(test)]
pub fn evaluate(
    file: &mut AlertFile,
    device: &Device,
    snapshot: &DeviceSnapshot,
    policy: &ThresholdPolicy,
) -> Vec<AlertTransition> {
    evaluate_with_expected_disruption(file, device, snapshot, policy, false)
}

/// M10 may suppress only the operation-attributable device-offline path.
/// Existing alerts remain untouched and all health/service conditions still
/// flow through M8, which stays the lifecycle authority.
pub fn evaluate_with_expected_disruption(
    file: &mut AlertFile,
    device: &Device,
    snapshot: &DeviceSnapshot,
    policy: &ThresholdPolicy,
    suppress_device_offline: bool,
) -> Vec<AlertTransition> {
    let timestamp = snapshot.captured_at.clone();
    let mut conditions = conditions(device, snapshot, policy, suppress_device_offline);
    let mut present = HashSet::new();
    if suppress_device_offline {
        // Do not resolve an unrelated pre-existing offline alert merely
        // because a controlled operation temporarily suppresses evaluation.
        present.insert(format!("device|{}|device|offline", device.id));
    }
    let mut transitions = Vec::new();
    for condition in conditions.drain(..) {
        let mature = mature(file, &condition, &timestamp);
        if !mature {
            continue;
        }
        present.insert(condition.key.clone());
        if let Some(alert) = file.alerts.iter_mut().find(|alert| {
            alert.deduplication_key == condition.key && alert.state != AlertState::Resolved
        }) {
            let escalated = condition.severity > alert.severity;
            alert.last_seen = timestamp.clone();
            alert.occurrence_count = alert.occurrence_count.saturating_add(1);
            alert.severity = condition.severity;
            alert.summary = condition.summary;
            alert.detail = condition.detail;
            alert.threshold = condition.threshold;
            if escalated {
                transitions.push(AlertTransition {
                    kind: AlertTransitionKind::Escalated,
                    alert: alert.clone(),
                });
            }
        } else {
            let alert = Alert::new(
                condition.key.clone(),
                condition.category,
                Some(device.id.clone()),
                condition.entity_id,
                condition.code,
                condition.severity,
                timestamp.clone(),
                condition.summary,
                condition.detail,
                condition.threshold,
            );
            file.alerts.push(alert.clone());
            transitions.push(AlertTransition {
                kind: AlertTransitionKind::Activated,
                alert,
            });
        }
    }
    let unresolved: Vec<String> = file
        .alerts
        .iter()
        .filter(|alert| {
            alert.device_id.as_deref() == Some(device.id.as_str())
                && alert.state != AlertState::Resolved
        })
        .map(|alert| alert.deduplication_key.clone())
        .collect();
    for key in unresolved {
        if !present.contains(&key) {
            if let Some(alert) = resolve_matching(
                file,
                &key,
                &timestamp,
                AlertResolutionReason::ConditionCleared,
            ) {
                transitions.push(AlertTransition {
                    kind: AlertTransitionKind::Resolved,
                    alert,
                });
            }
        }
    }
    transitions
}

pub fn acknowledge(file: &mut AlertFile, id: &str, timestamp: String) -> Option<AlertTransition> {
    let alert = file
        .alerts
        .iter_mut()
        .find(|alert| alert.id == id && alert.state == AlertState::Active)?;
    alert.state = AlertState::Acknowledged;
    alert.acknowledged_at = Some(timestamp);
    Some(AlertTransition {
        kind: AlertTransitionKind::Acknowledged,
        alert: alert.clone(),
    })
}

fn mature(file: &mut AlertFile, condition: &Condition, timestamp: &str) -> bool {
    let state = file
        .candidates
        .entry(condition.key.clone())
        .or_insert_with(|| CandidateState {
            first_seen: timestamp.into(),
            consecutive_samples: 0,
        });
    state.consecutive_samples = state.consecutive_samples.saturating_add(1);
    let duration_ready = condition
        .duration_seconds
        .map(|seconds| {
            DateTime::parse_from_rfc3339(&state.first_seen)
                .ok()
                .zip(DateTime::parse_from_rfc3339(timestamp).ok())
                .is_some_and(|(start, now)| {
                    now.signed_duration_since(start).num_seconds() >= seconds as i64
                })
        })
        .unwrap_or(true);
    let samples_ready = condition
        .consecutive_samples
        .map(|count| state.consecutive_samples >= count)
        .unwrap_or(true);
    duration_ready && samples_ready
}

fn conditions(
    device: &Device,
    snapshot: &DeviceSnapshot,
    policy: &ThresholdPolicy,
    suppress_device_offline: bool,
) -> Vec<Condition> {
    let key = |category: &str, entity: &str, code: &str| {
        format!("{category}|{}|{entity}|{code}", device.id)
    };
    let mut result = Vec::new();
    if !suppress_device_offline
        && !matches!(
            snapshot.connection_status,
            DeviceConnectionStatus::Online
                | DeviceConnectionStatus::Checking
                | DeviceConnectionStatus::Unknown
        )
    {
        result.push(Condition {
            key: key("device", "device", "offline"),
            category: AlertCategory::Device,
            entity_id: None,
            code: "offline".into(),
            severity: AlertSeverity::Critical,
            summary: format!("{} is unreachable", device.name),
            detail: None,
            threshold: None,
            duration_seconds: None,
            consecutive_samples: None,
        });
    }
    for reason in &snapshot.health.reasons {
        if reason.code.starts_with("disk_") {
            continue;
        }
        let severity = match reason.severity {
            HealthReasonSeverity::Info => AlertSeverity::Info,
            HealthReasonSeverity::Warning => AlertSeverity::Warning,
            HealthReasonSeverity::Critical => AlertSeverity::Critical,
        };
        let temperature = reason.code.starts_with("temperature_");
        result.push(Condition {
            key: key("health", "device", &reason.code),
            category: AlertCategory::Health,
            entity_id: None,
            code: reason.code.clone(),
            severity,
            summary: reason.summary.clone(),
            detail: None,
            threshold: temperature.then_some(format!(
                "{} consecutive samples",
                policy.temperature_consecutive_samples
            )),
            duration_seconds: None,
            consecutive_samples: temperature.then_some(policy.temperature_consecutive_samples),
        });
    }
    if let Some(metrics) = &snapshot.metrics {
        metric_condition(
            &mut result,
            &key,
            "cpu",
            metrics.cpu_usage_percent,
            policy.cpu_warning_percent,
            policy.cpu_critical_percent,
            policy.cpu_duration_seconds,
        );
        let memory = metrics
            .memory_used_bytes
            .zip(metrics.memory_total_bytes)
            .filter(|(_, total)| *total > 0)
            .map(|(used, total)| used as f64 / total as f64 * 100.0);
        metric_condition(
            &mut result,
            &key,
            "memory",
            memory,
            policy.memory_warning_percent,
            policy.memory_critical_percent,
            policy.memory_duration_seconds,
        );
        let disk = metrics
            .disk_used_bytes
            .zip(metrics.disk_total_bytes)
            .filter(|(_, total)| *total > 0)
            .map(|(used, total)| used as f64 / total as f64 * 100.0);
        metric_condition(
            &mut result,
            &key,
            "disk",
            disk,
            policy.disk_warning_percent,
            policy.disk_critical_percent,
            0,
        );
    }
    for service in &device.services {
        if let Some(record) = snapshot.service_health.get(&service.id) {
            let severity = match record.state {
                ServiceHealthState::Degraded => Some(AlertSeverity::Warning),
                ServiceHealthState::Unavailable => Some(AlertSeverity::Critical),
                _ => None,
            };
            if let Some(severity) = severity {
                result.push(Condition {
                    key: key("service", &service.id, "health"),
                    category: AlertCategory::Service,
                    entity_id: Some(service.id.clone()),
                    code: "service_health".into(),
                    severity,
                    summary: format!(
                        "{} is {}",
                        service.name,
                        if severity == AlertSeverity::Critical {
                            "unavailable"
                        } else {
                            "degraded"
                        }
                    ),
                    detail: record
                        .latest_failure_reason
                        .map(|reason| format!("{reason:?}")),
                    threshold: Some(format!(
                        "Unavailable after {} failures",
                        policy.service_unavailable_failures
                    )),
                    duration_seconds: None,
                    consecutive_samples: None,
                });
            }
        }
    }
    result
}

fn metric_condition(
    result: &mut Vec<Condition>,
    key: &impl Fn(&str, &str, &str) -> String,
    metric: &str,
    value: Option<f64>,
    warning: f64,
    critical: f64,
    duration_seconds: u32,
) {
    let Some(value) = value else {
        return;
    };
    let severity = if value >= critical {
        Some(AlertSeverity::Critical)
    } else if value >= warning {
        Some(AlertSeverity::Warning)
    } else {
        None
    };
    let Some(severity) = severity else {
        return;
    };
    result.push(Condition {
        key: key("health", "device", metric),
        category: AlertCategory::Health,
        entity_id: None,
        code: format!("{metric}_threshold"),
        severity,
        summary: format!("{metric} usage is {:.0}%", value),
        detail: None,
        threshold: Some(format!("Warning {warning:.0}% / Critical {critical:.0}%")),
        duration_seconds: (duration_seconds > 0).then_some(duration_seconds),
        consecutive_samples: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        device::{DeviceService, DeviceType},
        health::assess_health,
    };
    fn device() -> Device {
        Device {
            id: "d".into(),
            name: "Device".into(),
            host: "host".into(),
            ssh_port: 22,
            ssh_username: "user".into(),
            description: None,
            device_type: DeviceType::RaspberryPi,
            monitoring_enabled: true,
            refresh_interval_seconds: None,
            notify_on_device_offline: true,
            notify_on_container_failure: true,
            notify_on_container_unhealthy: true,
            services: vec![DeviceService {
                id: "s".into(),
                name: "Service".into(),
                url: "https://example.test".into(),
                icon: None,
                description: None,
                enabled: true,
                container_name: None,
            }],
            created_at: "x".into(),
            updated_at: "x".into(),
        }
    }
    fn snapshot(at: &str, state: ServiceHealthState) -> DeviceSnapshot {
        let status = DeviceConnectionStatus::Online;
        let mut services = std::collections::HashMap::new();
        services.insert(
            "s".into(),
            crate::domain::service_health::ServiceHealthRecord {
                service_id: "s".into(),
                state,
                consecutive_failures: 1,
                latest_http_status: None,
                latest_response_time_ms: None,
                last_checked_at: None,
                last_successful_check_at: None,
                latest_failure_reason: None,
            },
        );
        DeviceSnapshot {
            device_id: "d".into(),
            connection_status: status,
            captured_at: at.into(),
            duration_ms: 1,
            metrics: None,
            docker_available: false,
            containers: vec![],
            warnings: vec![],
            error: None,
            stale: false,
            last_successful_refresh: None,
            health: assess_health(&status, None),
            service_health: services,
            network_visibility: None,
            storage_visibility: None,
            system_visibility: None,
        }
    }
    #[test]
    fn dedup_escalate_resolve_recur() {
        let mut file = AlertFile::default();
        let policy = ThresholdPolicy::default();
        let first = evaluate(
            &mut file,
            &device(),
            &snapshot("2026-01-01T00:00:00Z", ServiceHealthState::Degraded),
            &policy,
        );
        assert_eq!(first[0].kind, AlertTransitionKind::Activated);
        let id = first[0].alert.id.clone();
        assert!(evaluate(
            &mut file,
            &device(),
            &snapshot("2026-01-01T00:01:00Z", ServiceHealthState::Degraded),
            &policy
        )
        .is_empty());
        assert_eq!(
            evaluate(
                &mut file,
                &device(),
                &snapshot("2026-01-01T00:02:00Z", ServiceHealthState::Unavailable),
                &policy
            )[0]
            .kind,
            AlertTransitionKind::Escalated
        );
        let recovered = evaluate(
            &mut file,
            &device(),
            &snapshot("2026-01-01T00:03:00Z", ServiceHealthState::Healthy),
            &policy,
        );
        assert_eq!(recovered[0].kind, AlertTransitionKind::Resolved);
        let again = evaluate(
            &mut file,
            &device(),
            &snapshot("2026-01-01T00:04:00Z", ServiceHealthState::Degraded),
            &policy,
        );
        assert_ne!(again[0].alert.id, id);
    }

    #[test]
    fn expected_disruption_suppresses_only_new_device_offline_alerts() {
        let mut file = AlertFile::default();
        let mut offline = snapshot("2026-01-01T00:00:00Z", ServiceHealthState::Unavailable);
        offline.connection_status = DeviceConnectionStatus::Offline;
        offline.health = crate::domain::health::assess_health(&offline.connection_status, None);
        let transitions = evaluate_with_expected_disruption(
            &mut file,
            &device(),
            &offline,
            &ThresholdPolicy::default(),
            true,
        );
        assert!(transitions
            .iter()
            .all(|transition| transition.alert.rule_code != "offline"));
        assert!(transitions
            .iter()
            .any(|transition| transition.alert.rule_code == "service_health"));
    }
}
