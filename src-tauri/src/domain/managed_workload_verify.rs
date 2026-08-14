use serde::Deserialize;
use crate::domain::managed_workload_prepare::TargetRevision;

#[derive(Debug,Clone,Copy,PartialEq,Eq)] pub enum VerifyContractError { Malformed,UnsupportedVersion }
#[derive(Deserialize)] #[serde(deny_unknown_fields,rename_all="camelCase")] struct Wire { protocol_version:u32,status:String,deployed_revision:String }
/// Parses the sole bounded M17 VERIFY response. The returned revision remains
/// typed so callers can only compare exact canonical object identifiers.
pub fn parse_verify(raw:&str)->Result<TargetRevision,VerifyContractError>{if raw.len()>32*1024{return Err(VerifyContractError::Malformed)}let value:Wire=serde_json::from_str(raw).map_err(|_|VerifyContractError::Malformed)?;if value.protocol_version!=1{return Err(VerifyContractError::UnsupportedVersion)}if value.status!="ok"{return Err(VerifyContractError::Malformed)}TargetRevision::parse(&value.deployed_revision).map_err(|_|VerifyContractError::Malformed)}
#[cfg(test)] mod tests {use super::*;#[test] fn verify_requires_one_exact_typed_revision(){let revision="a".repeat(40);assert_eq!(parse_verify(&format!(r#"{{"protocolVersion":1,"status":"ok","deployedRevision":"{revision}"}}"#)).unwrap().as_str(),revision);for value in [r#"{}"#,r#"{"protocolVersion":2,"status":"ok","deployedRevision":"main"}"#,r#"{"protocolVersion":1,"status":"ok","deployedRevision":"main"}"#,r#"{"protocolVersion":1,"status":"latest","deployedRevision":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#,r#"{"protocolVersion":1,"status":"ok","deployedRevision":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","extra":"x"}"#]{assert!(parse_verify(value).is_err());}assert!(parse_verify(&"x".repeat(32*1024+1)).is_err());}}
