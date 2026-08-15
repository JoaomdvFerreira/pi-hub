use serde::Deserialize;
use crate::commands::maintenance::sha256_hex;
const MAX_FIELD:usize=128;
#[derive(Debug,Clone,PartialEq,Eq)] pub struct TargetRevision(String);
impl TargetRevision { pub fn parse(v:&str)->Result<Self,PrepareContractError>{if (v.len()==40||v.len()==64)&&v.bytes().all(|b|b.is_ascii_hexdigit()&&!b.is_ascii_uppercase()){Ok(Self(v.into()))}else{Err(PrepareContractError::InvalidTarget)}} pub fn as_str(&self)->&str{&self.0} }
#[derive(Debug,Clone,PartialEq,Eq)] pub struct DeploymentFingerprint(pub String);
#[derive(Debug,Clone,PartialEq,Eq)] pub struct PreparedPlan { pub current_revision:TargetRevision,pub target_revision:TargetRevision,pub change_count:u32 }
/// The fixed set of `blocked` reasons the trusted action's `state()` guard can
/// emit. Any other reason string fails closed as a protocol error rather than
/// becoming arbitrary UI text.
#[derive(Debug,Clone,Copy,PartialEq,Eq)] pub enum BlockedReason { DirtyWorktree, UpstreamPolicy, DivergedHistory, ChangeCountExceeded, RepositoryUnavailable }
impl BlockedReason { fn parse(v:&str)->Option<Self>{match v{"dirtyWorktree"=>Some(Self::DirtyWorktree),"upstreamPolicy"=>Some(Self::UpstreamPolicy),"divergedHistory"=>Some(Self::DivergedHistory),"changeCountExceeded"=>Some(Self::ChangeCountExceeded),"repositoryUnavailable"=>Some(Self::RepositoryUnavailable),_=>None}} }
/// The three v1 PREPARE outcomes the trusted action's contract defines.
#[derive(Debug,Clone,PartialEq,Eq)] pub enum PreparedOutcome { Ready(PreparedPlan), UpToDate(TargetRevision), Blocked(BlockedReason) }
#[derive(Debug,Clone,Copy,PartialEq,Eq)] pub enum PrepareContractError { Malformed, UnsupportedVersion, InvalidTarget }
#[derive(Deserialize)] #[serde(deny_unknown_fields,rename_all="camelCase")] struct Wire { protocol_version:u32,status:String,#[serde(default)] current_revision:Option<String>,#[serde(default)] target_revision:Option<String>,#[serde(default)] change_count:Option<u32>,#[serde(default)] reason:Option<String> }
pub fn parse_prepare(raw:&str)->Result<PreparedOutcome,PrepareContractError>{
    if raw.len()>32*1024{return Err(PrepareContractError::Malformed)}
    let w:Wire=serde_json::from_str(raw).map_err(|_|PrepareContractError::Malformed)?;
    if w.protocol_version!=1{return Err(PrepareContractError::UnsupportedVersion)}
    match w.status.as_str() {
        "ready" => {
            if w.reason.is_some(){return Err(PrepareContractError::Malformed)}
            let cur=w.current_revision.ok_or(PrepareContractError::Malformed)?;
            let tar=w.target_revision.ok_or(PrepareContractError::Malformed)?;
            let cc=w.change_count.ok_or(PrepareContractError::Malformed)?;
            if cur.len()>MAX_FIELD||tar.len()>MAX_FIELD{return Err(PrepareContractError::Malformed)}
            Ok(PreparedOutcome::Ready(PreparedPlan{current_revision:TargetRevision::parse(&cur)?,target_revision:TargetRevision::parse(&tar)?,change_count:cc}))
        }
        "upToDate" => {
            if w.reason.is_some(){return Err(PrepareContractError::Malformed)}
            let cur=w.current_revision.ok_or(PrepareContractError::Malformed)?;
            let tar=w.target_revision.ok_or(PrepareContractError::Malformed)?;
            let cc=w.change_count.ok_or(PrepareContractError::Malformed)?;
            if cur.len()>MAX_FIELD||tar.len()>MAX_FIELD{return Err(PrepareContractError::Malformed)}
            let cur=TargetRevision::parse(&cur)?;
            let tar=TargetRevision::parse(&tar)?;
            if cc!=0||cur!=tar{return Err(PrepareContractError::Malformed)}
            Ok(PreparedOutcome::UpToDate(cur))
        }
        "blocked" => {
            if w.current_revision.is_some()||w.target_revision.is_some()||w.change_count.is_some(){return Err(PrepareContractError::Malformed)}
            let reason=w.reason.ok_or(PrepareContractError::Malformed)?;
            let reason=BlockedReason::parse(&reason).ok_or(PrepareContractError::Malformed)?;
            Ok(PreparedOutcome::Blocked(reason))
        }
        _ => Err(PrepareContractError::Malformed),
    }
}
pub fn fingerprint(workload:&str,revision:u32,digest:&str,target:&TargetRevision,verification:&[String])->DeploymentFingerprint{let mut v=verification.to_vec();v.sort();let s=format!("v1\x1f{}\x1f{}\x1f{}\x1f{}\x1f{}",workload,revision,digest,target.as_str(),v.join("\x1e"));DeploymentFingerprint(sha256_hex(s.as_bytes()))}
#[cfg(test)] mod tests {
    use super::*;
    fn ready_json(t:&str)->String{format!(r#"{{"protocolVersion":1,"status":"ready","currentRevision":"{}","targetRevision":"{}","changeCount":1}}"#,"a".repeat(40),t)}
    fn up_to_date_json(rev:&str)->String{format!(r#"{{"protocolVersion":1,"status":"upToDate","currentRevision":"{rev}","targetRevision":"{rev}","changeCount":0}}"#)}
    fn blocked_json(reason:&str)->String{format!(r#"{{"protocolVersion":1,"status":"blocked","reason":"{reason}"}}"#)}
    #[test] fn valid_ready_is_accepted(){
        let outcome=parse_prepare(&ready_json(&"b".repeat(40))).unwrap();
        assert_eq!(outcome,PreparedOutcome::Ready(PreparedPlan{current_revision:TargetRevision::parse(&"a".repeat(40)).unwrap(),target_revision:TargetRevision::parse(&"b".repeat(40)).unwrap(),change_count:1}));
    }
    #[test] fn valid_up_to_date_is_accepted(){
        let rev="a".repeat(40);
        let outcome=parse_prepare(&up_to_date_json(&rev)).unwrap();
        assert_eq!(outcome,PreparedOutcome::UpToDate(TargetRevision::parse(&rev).unwrap()));
    }
    #[test] fn valid_blocked_is_accepted(){
        for (wire,reason) in [("dirtyWorktree",BlockedReason::DirtyWorktree),("upstreamPolicy",BlockedReason::UpstreamPolicy),("divergedHistory",BlockedReason::DivergedHistory),("changeCountExceeded",BlockedReason::ChangeCountExceeded),("repositoryUnavailable",BlockedReason::RepositoryUnavailable)] {
            assert_eq!(parse_prepare(&blocked_json(wire)).unwrap(),PreparedOutcome::Blocked(reason));
        }
    }
    #[test] fn up_to_date_with_unequal_revisions_is_rejected(){
        let json=format!(r#"{{"protocolVersion":1,"status":"upToDate","currentRevision":"{}","targetRevision":"{}","changeCount":0}}"#,"a".repeat(40),"b".repeat(40));
        assert_eq!(parse_prepare(&json),Err(PrepareContractError::Malformed));
    }
    #[test] fn up_to_date_with_nonzero_change_count_is_rejected(){
        let rev="a".repeat(40);
        let json=format!(r#"{{"protocolVersion":1,"status":"upToDate","currentRevision":"{rev}","targetRevision":"{rev}","changeCount":1}}"#);
        assert_eq!(parse_prepare(&json),Err(PrepareContractError::Malformed));
    }
    #[test] fn blocked_without_revision_or_count_fields_is_accepted(){
        assert!(parse_prepare(&blocked_json("dirtyWorktree")).is_ok());
    }
    #[test] fn blocked_with_revision_or_count_fields_is_rejected(){
        let json=format!(r#"{{"protocolVersion":1,"status":"blocked","reason":"dirtyWorktree","currentRevision":"{}"}}"#,"a".repeat(40));
        assert_eq!(parse_prepare(&json),Err(PrepareContractError::Malformed));
    }
    #[test] fn unknown_blocked_reason_fails_closed(){
        assert_eq!(parse_prepare(&blocked_json("somethingUnexpected")),Err(PrepareContractError::Malformed));
    }
    #[test] fn unknown_status_is_rejected(){
        assert_eq!(parse_prepare(r#"{"protocolVersion":1,"status":"weird"}"#),Err(PrepareContractError::Malformed));
    }
    #[test] fn malformed_and_extra_fields_remain_rejected(){
        assert!(parse_prepare("{}").is_err());
        assert!(parse_prepare(r#"{"protocolVersion":2,"status":"ready"}"#).is_err());
        for x in ["main","../x","a b",&"a".repeat(65)]{assert!(TargetRevision::parse(x).is_err())}
        let json=format!(r#"{{"protocolVersion":1,"status":"ready","currentRevision":"{}","targetRevision":"{}","changeCount":1,"extra":"x"}}"#,"a".repeat(40),"b".repeat(40));
        assert!(parse_prepare(&json).is_err());
    }
    #[test] fn fingerprint_changes_only_with_evidence(){let t=TargetRevision::parse(&"a".repeat(40)).unwrap();let a=fingerprint("w",1,&"b".repeat(64),&t,&["svc".into()]);assert_eq!(a,fingerprint("w",1,&"b".repeat(64),&t,&["svc".into()]));assert_ne!(a,fingerprint("w",2,&"b".repeat(64),&t,&["svc".into()]));assert_ne!(a,fingerprint("w",1,&"c".repeat(64),&t,&["svc".into()]));assert_ne!(a,fingerprint("w",1,&"b".repeat(64),&t,&["x".into()]));}
}
