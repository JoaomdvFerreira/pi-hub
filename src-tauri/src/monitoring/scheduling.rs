use std::time::Duration;

/// A device with no prior snapshot (or an unparseable `capturedAt`) is
/// always due immediately.
pub fn is_due(last_captured_at: Option<&str>, interval: Duration) -> bool {
    let Some(raw) = last_captured_at else {
        return true;
    };
    let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(raw) else {
        return true;
    };
    let elapsed = chrono::Utc::now().signed_duration_since(parsed.with_timezone(&chrono::Utc));
    match elapsed.to_std() {
        Ok(elapsed) => elapsed >= interval,
        // A negative duration (clock skew, or captured_at in the future)
        // is not due yet.
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_previous_capture_is_always_due() {
        assert!(is_due(None, Duration::from_secs(60)));
    }

    #[test]
    fn unparseable_timestamp_is_treated_as_due() {
        assert!(is_due(Some("not a timestamp"), Duration::from_secs(60)));
    }

    #[test]
    fn recent_capture_is_not_due() {
        let now = chrono::Utc::now().to_rfc3339();
        assert!(!is_due(Some(&now), Duration::from_secs(60)));
    }

    #[test]
    fn old_capture_is_due() {
        let old = (chrono::Utc::now() - chrono::Duration::seconds(120)).to_rfc3339();
        assert!(is_due(Some(&old), Duration::from_secs(60)));
    }

    #[test]
    fn future_timestamp_is_not_due() {
        let future = (chrono::Utc::now() + chrono::Duration::seconds(120)).to_rfc3339();
        assert!(!is_due(Some(&future), Duration::from_secs(60)));
    }
}
