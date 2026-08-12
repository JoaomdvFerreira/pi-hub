use std::sync::{Arc, OnceLock};
use pihub_benchmark_core::{BenchmarkConfig, BenchmarkController, BenchmarkReport, BenchmarkStatus, OperationMeasurement, RuntimeSample, RuntimeSampler};

static CONTROLLER: OnceLock<Arc<BenchmarkController>> = OnceLock::new();
#[cfg(test)] thread_local! { static TEST_MEASUREMENT_SCOPE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
pub fn measure(label: &'static str) -> Option<OperationMeasurement<'static>> {
    #[cfg(test)] if !TEST_MEASUREMENT_SCOPE.with(|scope| scope.get()) { return None; }
    CONTROLLER.get().map(|controller| controller.measure(label))
}

pub struct PerformanceDiagnostics { controller: Arc<BenchmarkController> }
impl Default for PerformanceDiagnostics { fn default() -> Self { let controller = CONTROLLER.get_or_init(|| Arc::new(BenchmarkController::new())).clone(); Self { controller } } }
impl PerformanceDiagnostics {
    pub fn start(&self, config: BenchmarkConfig) -> Result<BenchmarkStatus, String> {
        self.controller.start(config)?;
        let controller = self.controller.clone();
        tauri::async_runtime::spawn(async move {
            let sampler = PlatformRuntimeSampler;
            while controller.is_active() {
                let Some(interval) = controller.sample_interval() else { break };
                tokio::time::sleep(interval).await;
                if !controller.is_active() { break; }
                controller.record_sample(sampler.sample());
            }
        });
        Ok(self.controller.status())
    }
    pub fn stop(&self) -> Option<BenchmarkReport> { self.controller.stop() }
    pub fn status(&self) -> BenchmarkStatus { self.controller.status() }
    pub fn report(&self) -> Option<BenchmarkReport> { self.controller.report() }
}

struct PlatformRuntimeSampler;
#[cfg(windows)]
impl RuntimeSampler for PlatformRuntimeSampler {
    fn sample(&self) -> RuntimeSample {
        use windows::Win32::System::{ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX}, Threading::{GetCurrentProcess, GetProcessHandleCount}};
        let process = unsafe { GetCurrentProcess() };
        let mut memory = PROCESS_MEMORY_COUNTERS_EX::default();
        memory.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
        let memory_ok = unsafe { GetProcessMemoryInfo(process, &mut memory as *mut _ as _, memory.cb) }.is_ok();
        let mut handles = 0;
        let handles_ok = unsafe { GetProcessHandleCount(process, &mut handles) }.is_ok();
        RuntimeSample { process_cpu_percent: None, resident_memory_bytes: memory_ok.then_some(memory.WorkingSetSize as u64), private_memory_bytes: memory_ok.then_some(memory.PrivateUsage as u64), thread_count: None, handle_count: handles_ok.then_some(handles) }
    }
}
#[cfg(not(windows))]
impl RuntimeSampler for PlatformRuntimeSampler { fn sample(&self) -> RuntimeSample { RuntimeSample::default() } }

#[cfg(test)] mod tests {
    use super::*;
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    struct TestMeasurementScope;
    impl TestMeasurementScope { fn enable() -> Self { TEST_MEASUREMENT_SCOPE.with(|scope| scope.set(true)); Self } }
    impl Drop for TestMeasurementScope { fn drop(&mut self) { TEST_MEASUREMENT_SCOPE.with(|scope| scope.set(false)); } }
    #[test] fn synthetic_refresh_aggregates_only_stable_labels_when_active() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let _scope = TestMeasurementScope::enable();
        let diagnostics=PerformanceDiagnostics::default(); let _=diagnostics.stop();
        diagnostics.start(BenchmarkConfig { max_samples: 4, ..Default::default() }).unwrap();
        crate::monitoring::synthetic::run(crate::monitoring::synthetic::SyntheticProfile::Small);
        let report=diagnostics.stop().unwrap(); let labels: Vec<_>=report.operations.iter().map(|item| item.name.as_str()).collect();
        assert!(labels.contains(&"monitoring.device_refresh")); assert!(labels.contains(&"docker.collect")); assert!(labels.iter().all(|label| !label.contains("fixture") && !label.contains('@')));
    }
    #[test] fn deterministic_profiles_produce_linear_refresh_request_counts() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let _scope = TestMeasurementScope::enable();
        let diagnostics=PerformanceDiagnostics::default(); let _=diagnostics.stop();
        for (profile, devices) in [(crate::monitoring::synthetic::SyntheticProfile::Small,2_u64),(crate::monitoring::synthetic::SyntheticProfile::Medium,5),(crate::monitoring::synthetic::SyntheticProfile::LargerLocal,10)] {
            diagnostics.start(BenchmarkConfig { name:"Synthetic baseline".into(), scenario:"synthetic".into(), max_samples:1, ..Default::default() }).unwrap();
            crate::monitoring::synthetic::run(profile); let report=diagnostics.stop().unwrap();
            let count=|name:&str| report.operations.iter().find(|item|item.name==name).map(|item|item.count).unwrap_or(0);
            assert_eq!(count("monitoring.device_refresh"),devices); assert_eq!(count("ssh.execute"),devices*6); assert_eq!(count("docker.collect"),devices); assert_eq!(count("visibility.network"),devices); assert_eq!(count("visibility.storage"),devices); assert_eq!(count("visibility.system"),devices);
        }
    }
}
