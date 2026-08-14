//! Closed, typed managed-action trust probe. This module deliberately creates
//! no executable command and retains no remote filesystem detail.
use crate::domain::managed_workload::is_valid_action_id;

pub const ACTION_ROOT: &str = "/usr/local/libexec/pi-hub/actions";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionTrustError {
    InvalidActionId,
    ActionUnavailable,
    TrustFailure,
    UnsupportedPlatform,
    CapabilityFailure,
    MalformedResult,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedAction {
    pub action_id: String,
    pub sha256: String,
}

/// Builds the only M17 trust probe. The action id can contain only safe ASCII
/// grammar and is independently quoted as a literal; callers cannot supply a
/// path, a shell fragment, or an operation.
pub fn trust_probe_command(action_id: &str) -> Result<String, ActionTrustError> {
    if !is_valid_action_id(action_id) {
        return Err(ActionTrustError::InvalidActionId);
    }
    Ok(format!(
        r#"LC_ALL=C LANG=C; a='{ACTION_ROOT}/{action_id}'; for p in /usr/local/libexec/pi-hub {ACTION_ROOT}; do m=$(stat -c %a -- "$p" 2>/dev/null) || {{ printf 'PIHUB_M17_TRUST=trust_failure\n'; exit 0; }}; case "$m" in *[2367]?|*?[2367]) printf 'PIHUB_M17_TRUST=trust_failure\n'; exit 0;; esac; test -d "$p" && test ! -L "$p" && test "$(stat -c %u -- "$p")" = 0 || {{ printf 'PIHUB_M17_TRUST=trust_failure\n'; exit 0; }}; done; m=$(stat -c %a -- "$a" 2>/dev/null) || {{ printf 'PIHUB_M17_TRUST=unavailable\n'; exit 0; }}; case "$m" in *[2367]?|*?[2367]) printf 'PIHUB_M17_TRUST=trust_failure\n'; exit 0;; esac; test -f "$a" && test ! -L "$a" && test -x "$a" && test "$(stat -c %u -- "$a")" = 0 || {{ printf 'PIHUB_M17_TRUST=trust_failure\n'; exit 0; }}; d=$(sha256sum -- "$a" | awk '{{print $1}}'); test "$(printf %s "$d" | wc -c)" = 64 && printf 'PIHUB_M17_TRUST=trusted\nPIHUB_M17_ACTION_DIGEST=%s\n' "$d" || printf 'PIHUB_M17_TRUST=malformed\n'"#
    ))
}

/// Parses only the fixed two-key response, so normal UI errors cannot expose
/// ownership, paths, mode values, or action contents.
pub fn parse_trust_probe(action_id: &str, stdout: &str) -> Result<TrustedAction, ActionTrustError> {
    if !is_valid_action_id(action_id) {
        return Err(ActionTrustError::InvalidActionId);
    }
    let mut status = None;
    let mut digest = None;
    for line in stdout.lines() {
        if let Some(v) = line.strip_prefix("PIHUB_M17_TRUST=") {
            if status.replace(v).is_some() {
                return Err(ActionTrustError::MalformedResult);
            }
        } else if let Some(v) = line.strip_prefix("PIHUB_M17_ACTION_DIGEST=") {
            if digest.replace(v).is_some() {
                return Err(ActionTrustError::MalformedResult);
            }
        } else {
            return Err(ActionTrustError::MalformedResult);
        }
    }
    match status {
        Some("trusted") => {
            let d = digest.ok_or(ActionTrustError::MalformedResult)?;
            if d.len() == 64
                && d.bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            {
                Ok(TrustedAction {
                    action_id: action_id.into(),
                    sha256: d.into(),
                })
            } else {
                Err(ActionTrustError::MalformedResult)
            }
        }
        Some("trust_failure") => Err(ActionTrustError::TrustFailure),
        Some("unavailable") => Err(ActionTrustError::ActionUnavailable),
        Some("unsupported") => Err(ActionTrustError::UnsupportedPlatform),
        Some("capability_failure") => Err(ActionTrustError::CapabilityFailure),
        _ => Err(ActionTrustError::MalformedResult),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_root_cannot_be_escaped() {
        for id in ["../x", "/x", "x/y", "x\\y", "x%2fy", "A"] {
            assert_eq!(
                trust_probe_command(id),
                Err(ActionTrustError::InvalidActionId)
            );
        }
        let c = trust_probe_command("personal-finance").unwrap();
        assert!(c.contains(ACTION_ROOT));
        assert!(c.contains("test ! -L"));
        assert!(c.contains("test -f"));
        assert!(c.contains("stat -c %u"));
        assert!(c.contains("sha256sum"));
        assert!(!c.contains("prepare"));
    }
    #[test]
    fn parses_only_trusted_digest_and_detects_drift() {
        let one=parse_trust_probe("finance","PIHUB_M17_TRUST=trusted\nPIHUB_M17_ACTION_DIGEST=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n").unwrap();
        let two=parse_trust_probe("finance","PIHUB_M17_TRUST=trusted\nPIHUB_M17_ACTION_DIGEST=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n").unwrap();
        assert_ne!(one.sha256, two.sha256);
        assert_eq!(
            parse_trust_probe("finance", "PIHUB_M17_TRUST=trust_failure\n"),
            Err(ActionTrustError::TrustFailure)
        );
    }
}
