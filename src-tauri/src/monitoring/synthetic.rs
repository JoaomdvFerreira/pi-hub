use serde::{Deserialize, Serialize};
use crate::{domain::{device::{Device, DeviceType}, settings::ThresholdPolicy}, infrastructure::ssh::fake::FakeRemoteExecutor, monitoring::refresh::refresh_device_sync_with_policy};

pub(crate) const ONLINE_FIXTURE_OUTPUT: &str = "PIHUB_HOSTNAME=fixture\nPIHUB_UPTIME_SECONDS=60\nPIHUB_DOCKER_AVAILABLE=0\nPIHUB_SYS_CORES=4\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all="camelCase")] pub enum SyntheticProfile { Small, Medium, LargerLocal }
impl SyntheticProfile { pub fn device_count(self) -> usize { match self { Self::Small=>2, Self::Medium=>5, Self::LargerLocal=>10 } } }
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all="camelCase")] pub struct SyntheticWorkloadResult { pub devices: usize, pub online_snapshots: usize, pub container_count: usize }

/// Runs the production refresh orchestration and parsers against fixed local
/// executor data. It performs no network operation and is intended for the
/// deterministic count/scaling evidence consumed by WU14-04.
pub fn run(profile: SyntheticProfile) -> SyntheticWorkloadResult {
 let executor=FakeRemoteExecutor::instrumented_online(ONLINE_FIXTURE_OUTPUT); let mut online=0; let mut containers=0;
 for index in 0..profile.device_count() { let device=Device { id:format!("fixture-{index}"), name:"Synthetic device".into(), host:"fixture.invalid".into(), ssh_port:22, ssh_username:"fixture".into(), description:None, device_type:DeviceType::RaspberryPi, monitoring_enabled:true, refresh_interval_seconds:None, notify_on_device_offline:true, notify_on_container_failure:true, notify_on_container_unhealthy:true, services:vec![], created_at:"2026-01-01T00:00:00Z".into(), updated_at:"2026-01-01T00:00:00Z".into() }; let snapshot=refresh_device_sync_with_policy(&executor,&device,None,&ThresholdPolicy::default()); if !snapshot.stale { online+=1 }; containers+=snapshot.containers.len(); }
 SyntheticWorkloadResult { devices:profile.device_count(), online_snapshots:online, container_count:containers }
}
#[cfg(test)] mod tests { use super::*; #[test] fn profiles_run_without_network_or_identity_in_output() { for profile in [SyntheticProfile::Small,SyntheticProfile::Medium,SyntheticProfile::LargerLocal] { let result=run(profile); assert_eq!(result.devices,profile.device_count()); assert_eq!(result.online_snapshots,profile.device_count()); assert_eq!(result.container_count,0); } } }
