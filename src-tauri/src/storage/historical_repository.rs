use std::{fs, path::PathBuf};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use crate::{domain::{historical::{ContainerHistoricalSample, DeviceHistoricalSample, HistoricalMetric, HistoricalNumericPoint, HistoricalRange, HistoricalSample, HistoricalSeries, HistoricalStatePoint, ServiceHistoricalSample, MAX_HISTORY_POINTS, RETENTION_DAYS, SAMPLE_INTERVAL_SECONDS}, snapshot::DeviceSnapshot}, storage::{atomic::write_atomic, StorageError}};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HistoricalFile { #[serde(default = "schema_version")] schema_version: u32, #[serde(default)] samples: Vec<HistoricalSample> }
fn schema_version() -> u32 { 1 }

pub trait HistoricalRepository: Send + Sync { fn append_snapshot(&self, snapshot: &DeviceSnapshot, now: DateTime<Utc>) -> Result<(), StorageError>; fn query(&self, device_id: &str, entity_id: Option<&str>, metric: HistoricalMetric, range: HistoricalRange, now: DateTime<Utc>) -> HistoricalSeries; }
pub struct JsonHistoricalRepository { path: PathBuf }
impl JsonHistoricalRepository {
 pub fn new(config_dir: impl Into<PathBuf>) -> Self { Self { path: config_dir.into().join("historical-metrics.json") } }
 fn load_file(&self) -> HistoricalFile { match fs::read(&self.path) { Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| { log::warn!("historical monitoring data is corrupted: {e}"); HistoricalFile::default() }), Err(e) if e.kind() == std::io::ErrorKind::NotFound => HistoricalFile::default(), Err(e) => { log::warn!("could not read historical monitoring data: {e}"); HistoricalFile::default() } } }
 pub fn prune(samples: &mut Vec<HistoricalSample>, now: DateTime<Utc>) { let cutoff = now - Duration::days(RETENTION_DAYS); samples.retain(|s| parse(s.timestamp()).is_some_and(|t| t >= cutoff)); samples.sort_by(|a,b| a.timestamp().cmp(b.timestamp())); }
 fn is_due(samples: &[HistoricalSample], key: &str, category: &str, now: DateTime<Utc>) -> bool { samples.iter().rev().find_map(|s| match s { HistoricalSample::Device(x) if category == "device" && x.device_id == key => parse(&x.timestamp), HistoricalSample::Service(x) if category == "service" && format!("{}:{}",x.device_id,x.service_id)==key => parse(&x.timestamp), HistoricalSample::Container(x) if category == "container" && format!("{}:{}",x.device_id,x.container_id)==key => parse(&x.timestamp), _ => None }).is_none_or(|last| now - last >= Duration::seconds(SAMPLE_INTERVAL_SECONDS)) }
}
fn parse(value: &str) -> Option<DateTime<Utc>> { DateTime::parse_from_rfc3339(value).ok().map(|x| x.with_timezone(&Utc)) }
fn percent(used: Option<u64>, total: Option<u64>) -> Option<f64> { used.zip(total).and_then(|(u,t)| (t > 0).then_some(u as f64 * 100.0 / t as f64)) }
impl HistoricalRepository for JsonHistoricalRepository {
 fn append_snapshot(&self, snapshot: &DeviceSnapshot, now: DateTime<Utc>) -> Result<(), StorageError> {
  if snapshot.stale { return Ok(()); }
  let mut file=self.load_file(); Self::prune(&mut file.samples, now); let timestamp=now.to_rfc3339();
  if Self::is_due(&file.samples,&snapshot.device_id,"device",now) { if let Some(m)=&snapshot.metrics { let root=m.disk_used_bytes.zip(m.disk_total_bytes).map(|(u,t)| u as f64*100.0/t as f64); file.samples.push(HistoricalSample::Device(DeviceHistoricalSample{device_id:snapshot.device_id.clone(),timestamp:timestamp.clone(),cpu_usage_percent:m.cpu_usage_percent,memory_usage_percent:percent(m.memory_used_bytes,m.memory_total_bytes),root_filesystem_usage_percent:root,temperature_celsius:m.temperature_celsius,health_state:Some(snapshot.health.state)})); } }
  for (service_id, health) in &snapshot.service_health { let key=format!("{}:{service_id}",snapshot.device_id); if Self::is_due(&file.samples,&key,"service",now) { file.samples.push(HistoricalSample::Service(ServiceHistoricalSample{device_id:snapshot.device_id.clone(),service_id:service_id.clone(),timestamp:timestamp.clone(),response_time_ms:health.latest_response_time_ms,health_state:health.state})); } }
  for container in &snapshot.containers { let Some(usage)=&container.resource_usage else { continue }; if usage.cpu_percent.is_none() && usage.memory_percent.is_none() { continue }; let key=format!("{}:{}",snapshot.device_id,container.id); if Self::is_due(&file.samples,&key,"container",now) { file.samples.push(HistoricalSample::Container(ContainerHistoricalSample{device_id:snapshot.device_id.clone(),container_id:container.id.clone(),timestamp:timestamp.clone(),cpu_percent:usage.cpu_percent,memory_percent:usage.memory_percent})); } }
  let bytes=serde_json::to_vec_pretty(&file)?; write_atomic(&self.path,&bytes)?; Ok(())
 }
 fn query(&self, device_id:&str, entity_id:Option<&str>, metric:HistoricalMetric, range:HistoricalRange, now:DateTime<Utc>) -> HistoricalSeries {
  let cutoff=now-Duration::seconds(range.seconds()); let file=self.load_file(); let mut numeric=Vec::new(); let mut states=Vec::new();
  for sample in file.samples { if parse(sample.timestamp()).is_none_or(|t| t < cutoff || t > now) { continue } match (metric, sample) {
   (HistoricalMetric::CpuUsagePercent,HistoricalSample::Device(x)) if x.device_id==device_id => push_num(&mut numeric,x.timestamp,x.cpu_usage_percent),
   (HistoricalMetric::MemoryUsagePercent,HistoricalSample::Device(x)) if x.device_id==device_id => push_num(&mut numeric,x.timestamp,x.memory_usage_percent),
   (HistoricalMetric::RootFilesystemUsagePercent,HistoricalSample::Device(x)) if x.device_id==device_id => push_num(&mut numeric,x.timestamp,x.root_filesystem_usage_percent),
   (HistoricalMetric::TemperatureCelsius,HistoricalSample::Device(x)) if x.device_id==device_id => push_num(&mut numeric,x.timestamp,x.temperature_celsius),
   (HistoricalMetric::DeviceHealth,HistoricalSample::Device(x)) if x.device_id==device_id => { if let Some(v)=x.health_state { push_state(&mut states,x.timestamp,format!("{:?}",v).to_lowercase()); } },
   (HistoricalMetric::ResponseTimeMs,HistoricalSample::Service(x)) if x.device_id==device_id && entity_id==Some(x.service_id.as_str()) => push_num(&mut numeric,x.timestamp,x.response_time_ms.map(|v|v as f64)),
   (HistoricalMetric::ServiceHealth,HistoricalSample::Service(x)) if x.device_id==device_id && entity_id==Some(x.service_id.as_str()) => push_state(&mut states,x.timestamp,format!("{:?}",x.health_state).to_lowercase()),
   (HistoricalMetric::ContainerCpuPercent,HistoricalSample::Container(x)) if x.device_id==device_id && entity_id==Some(x.container_id.as_str()) => push_num(&mut numeric,x.timestamp,x.cpu_percent),
   (HistoricalMetric::ContainerMemoryPercent,HistoricalSample::Container(x)) if x.device_id==device_id && entity_id==Some(x.container_id.as_str()) => push_num(&mut numeric,x.timestamp,x.memory_percent), _=>{} }
  }
  numeric.sort_by(|a,b|a.timestamp.cmp(&b.timestamp)); states.sort_by(|a,b|a.timestamp.cmp(&b.timestamp)); HistoricalSeries { numeric_points: downsample(numeric), state_points: transitions(states) }
 }
}
fn push_num(out:&mut Vec<HistoricalNumericPoint>,timestamp:String,value:Option<f64>) { if let Some(value)=value.filter(|v|v.is_finite()) { out.push(HistoricalNumericPoint{timestamp,value,minimum:value,maximum:value}); } }
fn push_state(out:&mut Vec<HistoricalStatePoint>,timestamp:String,state:String) { out.push(HistoricalStatePoint{timestamp,state}); }
fn transitions(points:Vec<HistoricalStatePoint>)->Vec<HistoricalStatePoint>{ let mut out=Vec::new(); for point in points { if out.last().is_none_or(|last:&HistoricalStatePoint|last.state!=point.state) { out.push(point); } } out }
fn downsample(points:Vec<HistoricalNumericPoint>)->Vec<HistoricalNumericPoint>{ if points.len()<=MAX_HISTORY_POINTS{return points} let bucket_size=(points.len()+MAX_HISTORY_POINTS-1)/MAX_HISTORY_POINTS; points.chunks(bucket_size).map(|bucket|{let min=bucket.iter().map(|p|p.minimum).fold(f64::INFINITY,f64::min);let max=bucket.iter().map(|p|p.maximum).fold(f64::NEG_INFINITY,f64::max);let value=bucket.iter().map(|p|p.value).sum::<f64>()/bucket.len() as f64;HistoricalNumericPoint{timestamp:bucket[0].timestamp.clone(),value,minimum:min,maximum:max}}).collect() }

#[cfg(test)] mod tests { use super::*; use tempfile::tempdir; #[test] fn retention_and_downsampling_are_bounded(){let mut samples=vec![HistoricalSample::Device(DeviceHistoricalSample{device_id:"d".into(),timestamp:"2000-01-01T00:00:00Z".into(),cpu_usage_percent:Some(1.),memory_usage_percent:None,root_filesystem_usage_percent:None,temperature_celsius:None,health_state:None})];JsonHistoricalRepository::prune(&mut samples,Utc::now());assert!(samples.is_empty());let points=(0..501).map(|i|HistoricalNumericPoint{timestamp:i.to_string(),value:i as f64,minimum:i as f64,maximum:i as f64}).collect();assert!(downsample(points).len()<=MAX_HISTORY_POINTS);} #[test] fn corrupt_file_is_empty(){let dir=tempdir().unwrap();fs::write(dir.path().join("historical-metrics.json"),b"not-json").unwrap();let repo=JsonHistoricalRepository::new(dir.path());assert!(repo.query("d",None,HistoricalMetric::CpuUsagePercent,HistoricalRange::OneHour,Utc::now()).numeric_points.is_empty());} }
