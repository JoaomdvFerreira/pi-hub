use std::{collections::HashMap, fs, path::PathBuf};

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::alert::{Alert, AlertResolutionReason, AlertState};
use crate::storage::{atomic::write_atomic, StorageError};

pub const MAX_RESOLVED_ALERTS: usize = 1_000;
const RESOLVED_RETENTION_DAYS: i64 = 90;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CandidateState {
    pub first_seen: String,
    pub consecutive_samples: u32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertFile {
    #[serde(default)]
    pub alerts: Vec<Alert>,
    #[serde(default)]
    pub candidates: HashMap<String, CandidateState>,
}

pub trait AlertRepository: Send + Sync {
    fn load(&self) -> AlertFile;
    fn save(&self, file: &AlertFile) -> Result<(), StorageError>;
}

pub struct JsonAlertRepository {
    path: PathBuf,
}
impl JsonAlertRepository {
    pub fn new(config_dir: impl Into<PathBuf>) -> Self {
        Self {
            path: config_dir.into().join("alerts.json"),
        }
    }
    pub fn prune(file: &mut AlertFile) {
        let cutoff = Utc::now() - Duration::days(RESOLVED_RETENTION_DAYS);
        let mut resolved: Vec<_> = file
            .alerts
            .iter()
            .filter(|a| a.state == AlertState::Resolved)
            .cloned()
            .collect();
        resolved.retain(|a| {
            a.resolved_at
                .as_ref()
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .is_some_and(|value| value.with_timezone(&Utc) >= cutoff)
        });
        resolved.sort_by(|a, b| b.resolved_at.cmp(&a.resolved_at));
        resolved.truncate(MAX_RESOLVED_ALERTS);
        let resolved_ids: std::collections::HashSet<_> =
            resolved.iter().map(|a| a.id.as_str()).collect();
        file.alerts
            .retain(|a| a.state != AlertState::Resolved || resolved_ids.contains(a.id.as_str()));
    }
}
impl AlertRepository for JsonAlertRepository {
    fn load(&self) -> AlertFile {
        match fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|err| {
                log::warn!("alert history is corrupted: {err}");
                AlertFile::default()
            }),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => AlertFile::default(),
            Err(err) => {
                log::warn!("could not read alerts: {err}");
                AlertFile::default()
            }
        }
    }
    fn save(&self, file: &AlertFile) -> Result<(), StorageError> {
        write_atomic(&self.path, &serde_json::to_vec_pretty(file)?)?;
        Ok(())
    }
}

pub fn resolve_matching(
    file: &mut AlertFile,
    key: &str,
    timestamp: &str,
    reason: AlertResolutionReason,
) -> Option<Alert> {
    let alert = file
        .alerts
        .iter_mut()
        .find(|alert| alert.deduplication_key == key && alert.state != AlertState::Resolved)?;
    alert.state = AlertState::Resolved;
    alert.resolved_at = Some(timestamp.into());
    alert.resolution_reason = Some(reason);
    alert.last_seen = timestamp.into();
    Some(alert.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::alert::{AlertCategory, AlertSeverity};
    use tempfile::tempdir;
    #[test]
    fn active_alerts_are_never_pruned() {
        let mut file = AlertFile {
            alerts: vec![Alert::new(
                "x".into(),
                AlertCategory::Device,
                None,
                None,
                "offline",
                AlertSeverity::Critical,
                "2000-01-01T00:00:00Z".into(),
                "offline",
                None,
                None,
            )],
            candidates: HashMap::new(),
        };
        JsonAlertRepository::prune(&mut file);
        assert_eq!(file.alerts.len(), 1);
    }
    #[test]
    fn repository_round_trips() {
        let dir = tempdir().unwrap();
        let repo = JsonAlertRepository::new(dir.path());
        let file = AlertFile::default();
        repo.save(&file).unwrap();
        assert!(repo.load().alerts.is_empty());
    }
}
