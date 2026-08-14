//! Minimal shared semantics for an already-known transient systemd service.
//! It intentionally contains no command construction, domain failures, or
//! persistence model: those remain owned by each domain.
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetachedUnitState { Missing, Running, Success, Failed }

/// Classifies the fixed `systemctl show` fields used by detached operations.
/// `RemainAfterExit=yes` success is deliberately `active/exited` plus main
/// process evidence, never `ActiveState` alone.
pub fn classify_systemd_show(raw: &str) -> Option<DetachedUnitState> {
    let fields: BTreeMap<_, _> = raw.lines().filter_map(|line| line.split_once('=')).collect();
    if fields.get("LoadState") == Some(&"not-found") { return Some(DetachedUnitState::Missing); }
    let active = *fields.get("ActiveState")?; let sub = *fields.get("SubState")?;
    if (active == "active" && sub != "exited") || active == "activating" { return Some(DetachedUnitState::Running); }
    if active == "active" && sub == "exited" && fields.get("Result") == Some(&"success") && matches!(fields.get("ExecMainCode"), Some(&"exited") | Some(&"1")) && fields.get("ExecMainStatus") == Some(&"0") { return Some(DetachedUnitState::Success); }
    Some(DetachedUnitState::Failed)
}

/// A persisted operation is recoverable only after one dispatch attempt and
/// only until its domain declares it terminal. Recovery observes the known
/// unit; it never authorizes a second dispatch.
pub fn requires_recovery(dispatch_attempted: bool, terminal: bool) -> bool { dispatch_attempted && !terminal }

#[cfg(test)] mod tests { use super::*;
 #[test] fn classification_preserves_remain_after_exit_semantics() { assert_eq!(classify_systemd_show("LoadState=loaded\nActiveState=active\nSubState=running\n"),Some(DetachedUnitState::Running)); assert_eq!(classify_systemd_show("LoadState=loaded\nActiveState=active\nSubState=exited\nResult=success\nExecMainCode=exited\nExecMainStatus=0\n"),Some(DetachedUnitState::Success)); assert_eq!(classify_systemd_show("LoadState=loaded\nActiveState=failed\nSubState=failed\n"),Some(DetachedUnitState::Failed)); assert_eq!(classify_systemd_show("LoadState=not-found\n"),Some(DetachedUnitState::Missing)); }
 #[test] fn recovery_never_implies_redispatch() { assert!(requires_recovery(true,false)); assert!(!requires_recovery(false,false)); assert!(!requires_recovery(true,true)); }
}
