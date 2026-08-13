use serde::{Deserialize, Serialize};

pub const UPDATE_SCHEMA_VERSION: u32 = 1;
pub const MAX_UPDATE_DETAILS: usize = 200;
pub const MAX_FIELD_BYTES: usize = 512;
pub const STALE_AFTER_SECONDS: u64 = 7 * 24 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)] #[serde(rename_all="camelCase")]
pub struct UpdateCheckResult { pub schema_version:u32, pub device_id:String, pub status:UpdateStatus, pub checked_at:Option<String>, pub support:UpdateSupport, pub package_metadata:PackageMetadata, pub updates:UpdatePackages, #[serde(default)] pub kept_back_packages:KeptBackPackages, pub held_packages:HeldPackages, pub reboot:RebootState, pub security_updates:SecurityUpdates, pub warnings:Vec<String>, pub failure:Option<UpdateFailure> }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)] #[serde(rename_all="camelCase")]
pub enum UpdateStatus { NotChecked, Checking, UpToDate, UpdatesAvailable, Stale, Unsupported, CheckFailed, Unknown }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)] #[serde(rename_all="camelCase")]
pub struct UpdateSupport { pub status:SupportStatus, pub os_id:Option<String>, pub os_version_id:Option<String>, pub package_manager:Option<String>, pub reason:Option<String> }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)] #[serde(rename_all="camelCase")] pub enum SupportStatus { Supported, Unsupported, Unknown }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)] #[serde(rename_all="camelCase")]
pub struct PackageMetadata { pub status:MetadataStatus, pub newest_metadata_at:Option<String>, pub age_seconds:Option<u64>, pub stale_after_seconds:u64 }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)] #[serde(rename_all="camelCase")] pub enum MetadataStatus { Fresh, Stale, Unavailable, Unknown }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)] #[serde(rename_all="camelCase")]
pub struct UpdatePackages { pub total_count:Option<u32>, pub packages:Vec<UpdatePackage>, pub truncated:bool }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)] #[serde(rename_all="camelCase")]
pub struct KeptBackPackages { pub total_count:Option<u32>, pub packages:Vec<String>, pub truncated:bool }
impl Default for KeptBackPackages { fn default() -> Self { Self { total_count:None, packages:vec![], truncated:false } } }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)] #[serde(rename_all="camelCase")]
pub struct UpdatePackage { pub name:String, pub installed_version:String, pub candidate_version:String, pub architecture:Option<String>, pub held:Option<bool> }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)] #[serde(rename_all="camelCase")]
pub struct HeldPackages { pub status:HeldStatus, pub total_count:Option<u32>, pub packages:Vec<String>, pub truncated:bool }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)] #[serde(rename_all="camelCase")] pub enum HeldStatus { Known, Unknown }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)] #[serde(rename_all="camelCase")] pub enum RebootState { Required, NotRequired, Unknown }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)] #[serde(rename_all="camelCase")] pub enum SecurityUpdates { Unavailable }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)] #[serde(rename_all="camelCase")] pub struct UpdateFailure { pub kind:UpdateFailureKind, pub message:String }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)] #[serde(rename_all="camelCase")] pub enum UpdateFailureKind { Transport, Timeout, RequiredCommand, MalformedOutput, Persistence }
impl UpdateCheckResult { pub fn empty(device_id:String)->Self { Self { schema_version:UPDATE_SCHEMA_VERSION,device_id,status:UpdateStatus::Unknown,checked_at:None,support:UpdateSupport{status:SupportStatus::Unknown,os_id:None,os_version_id:None,package_manager:None,reason:None},package_metadata:PackageMetadata{status:MetadataStatus::Unknown,newest_metadata_at:None,age_seconds:None,stale_after_seconds:STALE_AFTER_SECONDS},updates:UpdatePackages{total_count:None,packages:vec![],truncated:false},kept_back_packages:KeptBackPackages{total_count:None,packages:vec![],truncated:false},held_packages:HeldPackages{status:HeldStatus::Unknown,total_count:None,packages:vec![],truncated:false},reboot:RebootState::Unknown,security_updates:SecurityUpdates::Unavailable,warnings:vec![],failure:None } } }
