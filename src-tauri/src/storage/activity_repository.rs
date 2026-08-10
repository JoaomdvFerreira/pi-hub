use std::fs;
use std::path::PathBuf;

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::activity::ActivityEvent;
use crate::storage::atomic::write_atomic;
use crate::storage::StorageError;

pub const MAX_ACTIVITY_EVENTS: usize = 2_000;
const RETENTION_DAYS: i64 = 30;

pub trait ActivityRepository: Send + Sync {
    fn load_all(&self) -> Vec<ActivityEvent>;
    fn append(&self, event: ActivityEvent) -> Result<(), StorageError>;
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivityFile { #[serde(default)] events: Vec<ActivityEvent> }

pub struct JsonActivityRepository { path: PathBuf }
impl JsonActivityRepository {
    pub fn new(config_dir: impl Into<PathBuf>) -> Self { Self { path: config_dir.into().join("activity.json") } }
    fn load_file(&self) -> ActivityFile {
        match fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|err| { log::warn!("activity history is corrupted: {err}"); ActivityFile::default() }),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => ActivityFile::default(),
            Err(err) => { log::warn!("could not read activity history: {err}"); ActivityFile::default() }
        }
    }
    fn prune(events: &mut Vec<ActivityEvent>) {
        let cutoff = Utc::now() - Duration::days(RETENTION_DAYS);
        events.retain(|event| chrono::DateTime::parse_from_rfc3339(&event.timestamp).map(|time| time.with_timezone(&Utc) >= cutoff).unwrap_or(false));
        events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        events.truncate(MAX_ACTIVITY_EVENTS);
    }
}
impl ActivityRepository for JsonActivityRepository {
    fn load_all(&self) -> Vec<ActivityEvent> { let mut events = self.load_file().events; Self::prune(&mut events); events }
    fn append(&self, event: ActivityEvent) -> Result<(), StorageError> {
        let mut file = self.load_file(); file.events.push(event); Self::prune(&mut file.events);
        let bytes = serde_json::to_vec_pretty(&file)?;
        write_atomic(&self.path, &bytes)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::activity::{ActivityCategory, ActivityEvent};
    use tempfile::tempdir;

    fn event(timestamp: &str, id: &str) -> ActivityEvent {
        ActivityEvent { id: id.into(), timestamp: timestamp.into(), code: "service.health_changed".into(), category: ActivityCategory::Service, device_id: Some("pi5".into()), entity_id: Some("svc".into()), entity_name: Some("Home".into()), summary: "Home changed state".into(), detail: None }
    }

    #[test]
    fn append_round_trips_newest_first() {
        let dir = tempdir().unwrap(); let repo = JsonActivityRepository::new(dir.path());
        repo.append(event("2026-08-10T00:00:00Z", "old")).unwrap();
        repo.append(event("2026-08-10T01:00:00Z", "new")).unwrap();
        assert_eq!(repo.load_all().iter().map(|item| item.id.as_str()).collect::<Vec<_>>(), vec!["new", "old"]);
    }

    #[test]
    fn pruning_removes_expired_and_excess_events() {
        let mut events = vec![event("2000-01-01T00:00:00Z", "expired")];
        events.extend((0..(MAX_ACTIVITY_EVENTS + 2)).map(|index| event("2026-08-10T00:00:00Z", &format!("{index:04}"))));
        JsonActivityRepository::prune(&mut events);
        assert_eq!(events.len(), MAX_ACTIVITY_EVENTS);
        assert!(!events.iter().any(|item| item.id == "expired"));
    }
}
