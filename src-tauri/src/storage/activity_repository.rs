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
    fn load_for_device(&self, device_id: &str) -> Vec<ActivityEvent>;
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
    fn load_for_device(&self, device_id: &str) -> Vec<ActivityEvent> { self.load_all().into_iter().filter(|event| event.device_id.as_deref() == Some(device_id)).collect() }
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
        let now = Utc::now();
        repo.append(event(&(now - Duration::hours(1)).to_rfc3339(), "old")).unwrap();
        repo.append(event(&now.to_rfc3339(), "new")).unwrap();
        assert_eq!(repo.load_all().iter().map(|item| item.id.as_str()).collect::<Vec<_>>(), vec!["new", "old"]);
    }

    #[test]
    fn pruning_removes_expired_and_excess_events() {
        let mut events = vec![event("2000-01-01T00:00:00Z", "expired")];
        let retained_timestamp = Utc::now().to_rfc3339();
        events.extend((0..(MAX_ACTIVITY_EVENTS + 2)).map(|index| event(&retained_timestamp, &format!("{index:04}"))));
        JsonActivityRepository::prune(&mut events);
        assert_eq!(events.len(), MAX_ACTIVITY_EVENTS);
        assert!(!events.iter().any(|item| item.id == "expired"));
    }

    #[test]
    fn device_query_preserves_global_history_and_isolates_device_identity() {
        let dir = tempdir().unwrap(); let repo = JsonActivityRepository::new(dir.path());
        let now = Utc::now();
        let mut device_a = event(&(now - Duration::hours(2)).to_rfc3339(), "device-a"); device_a.device_id = Some("device-a-id".into());
        let mut device_b = event(&(now - Duration::hours(1)).to_rfc3339(), "device-b"); device_b.device_id = Some("device-b-id".into());
        let mut global = event(&now.to_rfc3339(), "global"); global.device_id = None;
        repo.append(device_a).unwrap(); repo.append(device_b).unwrap(); repo.append(global).unwrap();
        assert_eq!(repo.load_all().len(), 3);
        assert_eq!(repo.load_for_device("device-a-id").iter().map(|event| event.id.as_str()).collect::<Vec<_>>(), vec!["device-a"]);
        assert!(repo.load_for_device("device-with-no-activity").is_empty());
    }
}
