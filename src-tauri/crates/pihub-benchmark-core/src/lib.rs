use std::{sync::{atomic::{AtomicBool, Ordering}, Mutex}, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)] #[serde(rename_all="camelCase")]
pub enum BenchmarkScenario { Manual, Synthetic }
impl BenchmarkScenario { fn display_name(self) -> &'static str { match self { Self::Manual => "Manual", Self::Synthetic => "Synthetic baseline" } } }
#[derive(Clone, Debug, Serialize, Deserialize)] #[serde(rename_all="camelCase", deny_unknown_fields)]
pub struct BenchmarkConfig { pub scenario: BenchmarkScenario, pub sample_interval_ms: u64, pub max_duration_ms: u64, pub max_samples: usize }
impl Default for BenchmarkConfig { fn default() -> Self { Self { scenario: BenchmarkScenario::Manual, sample_interval_ms: 1_000, max_duration_ms: 900_000, max_samples: 900 } } }
impl BenchmarkConfig { pub fn validate(&self) -> Result<(), String> { if self.sample_interval_ms == 0 || self.max_duration_ms == 0 || self.max_samples == 0 { return Err("Benchmark bounds must be positive.".into()) } Ok(()) } }

#[derive(Clone, Debug, Default, Serialize, Deserialize)] #[serde(rename_all="camelCase")]
pub struct RuntimeSample { pub process_cpu_percent: Option<f32>, pub resident_memory_bytes: Option<u64>, pub private_memory_bytes: Option<u64>, pub thread_count: Option<u32>, pub handle_count: Option<u32> }
pub trait RuntimeSampler: Send + Sync { fn sample(&mut self) -> RuntimeSample; }
#[derive(Clone, Debug, Default)] pub struct FakeRuntimeSampler { pub sample: RuntimeSample }
impl RuntimeSampler for FakeRuntimeSampler { fn sample(&mut self) -> RuntimeSample { self.sample.clone() } }
#[derive(Clone, Debug, Serialize, Deserialize)] #[serde(rename_all="camelCase")]
pub struct MetricSample { pub elapsed_ms: u64, #[serde(flatten)] pub metrics: RuntimeSample }
#[derive(Clone, Debug, Default, Serialize, Deserialize)] #[serde(rename_all="camelCase")]
pub struct OperationAggregate { pub name: String, pub count: u64, pub total_duration_ms: u64, pub max_duration_ms: u64, pub failures: u64, pub bytes: Option<u64> }
#[derive(Clone, Debug, Serialize, Deserialize)] #[serde(rename_all="camelCase")]
pub struct SessionSummary { pub id: String, pub name: String, pub scenario: BenchmarkScenario, pub started_at_unix_ms: u128, pub ended_at_unix_ms: Option<u128>, pub duration_ms: u64 }
#[derive(Clone, Debug, Serialize, Deserialize)] #[serde(rename_all="camelCase")]
pub struct BenchmarkReport { pub session: SessionSummary, pub config: BenchmarkConfig, pub samples: Vec<MetricSample>, pub operations: Vec<OperationAggregate>, pub warnings: Vec<String> }
#[derive(Clone, Debug, Serialize, Deserialize)] #[serde(rename_all="camelCase", tag="state")]
pub enum BenchmarkStatus { Idle, Running { session: SessionSummary, sample_count: usize }, Stopped { session: SessionSummary } }

struct Active { report: BenchmarkReport, started: Instant }
enum State { Idle, Active(Active), Stopped(BenchmarkReport) }
pub struct BenchmarkController { active: AtomicBool, state: Mutex<State> }
impl Default for BenchmarkController { fn default() -> Self { Self::new() } }
impl BenchmarkController {
 pub fn new() -> Self { Self { active: AtomicBool::new(false), state: Mutex::new(State::Idle) } }
 pub fn start(&self, config: BenchmarkConfig) -> Result<SessionSummary, String> { config.validate()?; let mut state=self.state.lock().expect("benchmark state poisoned"); if matches!(*state, State::Active(_)) { return Err("A benchmark session is already running.".into()) }; let now=unix_ms(); let summary=SessionSummary { id: uuid::Uuid::new_v4().to_string(), name: config.scenario.display_name().into(), scenario: config.scenario, started_at_unix_ms: now, ended_at_unix_ms: None, duration_ms: 0 }; *state=State::Active(Active { report: BenchmarkReport { session: summary.clone(), config, samples: vec![], operations: vec![], warnings: vec![] }, started: Instant::now() }); self.active.store(true, Ordering::Release); Ok(summary) }
 pub fn is_active(&self) -> bool { self.active.load(Ordering::Acquire) }
 pub fn sample_interval(&self) -> Option<Duration> { let state=self.state.lock().ok()?; match &*state { State::Active(active) => Some(Duration::from_millis(active.report.config.sample_interval_ms)), _ => None } }
 pub fn record_sample(&self, sample: RuntimeSample) { if !self.is_active() { return } let mut state=self.state.lock().expect("benchmark state poisoned"); let State::Active(active)=&mut *state else{return}; if expired(active) { stop_active(&mut *state, &self.active); return } if active.report.samples.len() < active.report.config.max_samples { active.report.samples.push(MetricSample { elapsed_ms: active.started.elapsed().as_millis() as u64, metrics: sample }); } else { active.report.warnings.push("Sample limit reached; retaining bounded report.".into()); stop_active(&mut *state, &self.active); } }
 pub fn measure<'a>(&'a self, name: &'a str) -> OperationMeasurement<'a> { if self.is_active() { OperationMeasurement { controller: Some(self), name, started: Some(Instant::now()), failed: false, bytes: None } } else { OperationMeasurement { controller: None, name, started: None, failed: false, bytes: None } } }
 fn record_operation(&self, name: &str, duration: Duration, failed: bool, bytes: Option<u64>) { let mut state=self.state.lock().expect("benchmark state poisoned"); let State::Active(active)=&mut *state else{return}; if expired(active) { stop_active(&mut *state, &self.active); return }; let item=active.report.operations.iter_mut().find(|item| item.name==name); let item=match item { Some(item)=>item, None=>{active.report.operations.push(OperationAggregate { name:name.into(), ..Default::default() }); active.report.operations.last_mut().unwrap()} }; item.count+=1; let ms=duration.as_millis() as u64; item.total_duration_ms+=ms; item.max_duration_ms=item.max_duration_ms.max(ms); if failed {item.failures+=1}; if let Some(bytes)=bytes { item.bytes=Some(item.bytes.unwrap_or(0)+bytes) } }
 pub fn stop(&self) -> Option<BenchmarkReport> { let mut state=self.state.lock().ok()?; stop_active(&mut *state, &self.active); self.report_locked(&state) }
 pub fn report(&self) -> Option<BenchmarkReport> { let mut state=self.state.lock().ok()?; if matches!(*state, State::Active(_)) { stop_if_expired(&mut *state, &self.active) }; self.report_locked(&state) }
 fn report_locked(&self, state:&State)->Option<BenchmarkReport> { match state { State::Active(active)=>Some(active.report.clone()), State::Stopped(report)=>Some(report.clone()), State::Idle=>None } }
 pub fn status(&self) -> BenchmarkStatus { let mut state=self.state.lock().expect("benchmark state poisoned"); stop_if_expired(&mut *state,&self.active); match &*state { State::Idle=>BenchmarkStatus::Idle, State::Active(active)=>BenchmarkStatus::Running { session: active.report.session.clone(), sample_count:active.report.samples.len() }, State::Stopped(report)=>BenchmarkStatus::Stopped { session: report.session.clone() } } }
}
pub struct OperationMeasurement<'a> { controller: Option<&'a BenchmarkController>, name: &'a str, started: Option<Instant>, failed: bool, bytes: Option<u64> }
impl<'a> OperationMeasurement<'a> { pub fn fail(&mut self) { self.failed=true } pub fn set_bytes(&mut self, bytes:u64) { self.bytes=Some(bytes) } }
impl Drop for OperationMeasurement<'_> { fn drop(&mut self) { if let (Some(controller),Some(started))=(self.controller,self.started) { controller.record_operation(self.name,started.elapsed(),self.failed,self.bytes) } } }
fn expired(active:&Active)->bool { active.started.elapsed().as_millis() as u64 >= active.report.config.max_duration_ms }
fn stop_if_expired(state:&mut State, active:&AtomicBool) { if matches!(state,State::Active(a) if expired(a)) { stop_active(state,active) } }
fn stop_active(state:&mut State, active:&AtomicBool) { if let State::Active(mut current)=std::mem::replace(state,State::Idle) { current.report.session.duration_ms=current.started.elapsed().as_millis() as u64; current.report.session.ended_at_unix_ms=Some(unix_ms()); current.report.operations.sort_by(|a,b| b.total_duration_ms.cmp(&a.total_duration_ms)); *state=State::Stopped(current.report); active.store(false,Ordering::Release) } }
fn unix_ms()->u128 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() }

#[cfg(test)] mod tests { use super::*; #[test] fn inactive_measurement_has_no_report() { let c=BenchmarkController::new(); drop(c.measure("ssh.execute")); assert!(c.report().is_none()); } #[test] fn session_is_single_and_bounded() { let c=BenchmarkController::new(); let config=BenchmarkConfig { max_samples:1, ..Default::default() }; c.start(config.clone()).unwrap(); assert!(c.start(config).is_err()); c.record_sample(RuntimeSample::default()); c.record_sample(RuntimeSample::default()); let r=c.report().unwrap(); assert_eq!(r.samples.len(),1); assert!(!c.is_active()); } #[test] fn operation_aggregates_without_events() { let c=BenchmarkController::new(); c.start(Default::default()).unwrap(); { let mut m=c.measure("history.query"); m.set_bytes(12); } { let mut m=c.measure("history.query"); m.fail(); } let r=c.stop().unwrap(); assert_eq!(r.operations[0].count,2); assert_eq!(r.operations[0].failures,1); assert_eq!(r.operations[0].bytes,Some(12)); } #[test] fn exported_metadata_rejects_free_text_and_contains_only_trusted_scenario_values() { let secret="password=not-for-export"; let supplied=format!(r#"{{"name":"{secret}","scenario":"manual","sampleIntervalMs":1000,"maxDurationMs":900000,"maxSamples":900}}"#); assert!(serde_json::from_str::<BenchmarkConfig>(&supplied).is_err()); let c=BenchmarkController::new(); c.start(Default::default()).unwrap(); let exported=serde_json::to_string(&c.stop().unwrap()).unwrap(); assert!(!exported.contains(secret)); assert!(exported.contains(r#""name":"Manual""#)); assert!(exported.contains(r#""scenario":"manual""#)); assert!(!exported.contains(r#""config":{"name""#)); } }
