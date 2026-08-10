use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::domain::device::DeviceService;

pub const SERVICE_CHECK_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_REDIRECTS: usize = 5;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceHealthState { Unknown, Healthy, Degraded, Unavailable }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceFailureReason { HttpStatus, Connection, Timeout, Tls, Redirect, InvalidUrl, Unknown }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServiceHealthRecord {
    pub service_id: String,
    pub state: ServiceHealthState,
    pub consecutive_failures: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_http_status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_response_time_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_checked_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_successful_check_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_failure_reason: Option<ServiceFailureReason>,
}

impl ServiceHealthRecord {
    pub fn unknown(service_id: impl Into<String>) -> Self {
        Self { service_id: service_id.into(), state: ServiceHealthState::Unknown, consecutive_failures: 0, latest_http_status: None, latest_response_time_ms: None, last_checked_at: None, last_successful_check_at: None, latest_failure_reason: None }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceCheckResult {
    pub http_status: Option<u16>,
    pub response_time_ms: Option<u64>,
    pub failure_reason: Option<ServiceFailureReason>,
}

impl ServiceCheckResult {
    pub fn success(status: u16, response_time_ms: u64) -> Self { Self { http_status: Some(status), response_time_ms: Some(response_time_ms), failure_reason: None } }
    pub fn failure(status: Option<u16>, response_time_ms: Option<u64>, reason: ServiceFailureReason) -> Self { Self { http_status: status, response_time_ms, failure_reason: Some(reason) } }
    pub fn is_success(&self) -> bool { self.failure_reason.is_none() }
}

pub fn apply_check(previous: Option<&ServiceHealthRecord>, service_id: &str, result: ServiceCheckResult, checked_at: String) -> ServiceHealthRecord {
    let mut record = previous.cloned().unwrap_or_else(|| ServiceHealthRecord::unknown(service_id));
    record.service_id = service_id.into();
    record.latest_http_status = result.http_status;
    record.latest_response_time_ms = result.response_time_ms;
    record.last_checked_at = Some(checked_at.clone());
    if result.is_success() {
        record.state = ServiceHealthState::Healthy;
        record.consecutive_failures = 0;
        record.last_successful_check_at = Some(checked_at);
        record.latest_failure_reason = None;
    } else {
        record.consecutive_failures += 1;
        record.state = if record.consecutive_failures >= 3 { ServiceHealthState::Unavailable } else { ServiceHealthState::Degraded };
        record.latest_failure_reason = result.failure_reason;
    }
    record
}

pub fn check_services(services: &[DeviceService], previous: &HashMap<String, ServiceHealthRecord>) -> HashMap<String, ServiceHealthRecord> {
    services.iter().filter(|service| service.enabled).map(|service| {
        let result = check_service(&service.url);
        let record = apply_check(previous.get(&service.id), &service.id, result, chrono::Utc::now().to_rfc3339());
        (service.id.clone(), record)
    }).collect()
}

pub fn check_service(raw_url: &str) -> ServiceCheckResult {
    let url = match reqwest::Url::parse(raw_url) {
        Ok(url) if matches!(url.scheme(), "http" | "https") => url,
        _ => return ServiceCheckResult::failure(None, None, ServiceFailureReason::InvalidUrl),
    };
    let client = match reqwest::blocking::Client::builder().timeout(SERVICE_CHECK_TIMEOUT).redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS)).build() {
        Ok(client) => client,
        Err(_) => return ServiceCheckResult::failure(None, None, ServiceFailureReason::Unknown),
    };
    let started = Instant::now();
    match client.get(url).send() {
        Ok(response) => {
            let status = response.status().as_u16();
            let elapsed = started.elapsed().as_millis() as u64;
            if (200..400).contains(&status) { ServiceCheckResult::success(status, elapsed) } else { ServiceCheckResult::failure(Some(status), Some(elapsed), ServiceFailureReason::HttpStatus) }
        }
        Err(error) => {
            let reason = if error.is_timeout() { ServiceFailureReason::Timeout } else if error.is_redirect() { ServiceFailureReason::Redirect } else if error.is_connect() {
                if error.to_string().to_lowercase().contains("certificate") || error.to_string().to_lowercase().contains("tls") { ServiceFailureReason::Tls } else { ServiceFailureReason::Connection }
            } else { ServiceFailureReason::Unknown };
            ServiceCheckResult::failure(None, Some(started.elapsed().as_millis() as u64), reason)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn state_machine_degrades_unavailable_and_recovers() {
        let failed = ServiceCheckResult::failure(Some(500), Some(4), ServiceFailureReason::HttpStatus);
        let first = apply_check(None, "svc", failed.clone(), "2026-01-01T00:00:00Z".into());
        assert_eq!(first.state, ServiceHealthState::Degraded);
        let second = apply_check(Some(&first), "svc", failed.clone(), "2026-01-01T00:01:00Z".into());
        assert_eq!(second.state, ServiceHealthState::Degraded);
        let third = apply_check(Some(&second), "svc", failed, "2026-01-01T00:02:00Z".into());
        assert_eq!(third.state, ServiceHealthState::Unavailable);
        let recovered = apply_check(Some(&third), "svc", ServiceCheckResult::success(204, 3), "2026-01-01T00:03:00Z".into());
        assert_eq!(recovered.state, ServiceHealthState::Healthy);
        assert_eq!(recovered.consecutive_failures, 0);
        assert_eq!(recovered.last_successful_check_at.as_deref(), Some("2026-01-01T00:03:00Z"));
    }
    #[test]
    fn invalid_urls_are_typed_failures() { assert_eq!(check_service("file:///secret").failure_reason, Some(ServiceFailureReason::InvalidUrl)); }
}
