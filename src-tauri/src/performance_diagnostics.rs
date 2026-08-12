use std::sync::Arc;
use pihub_benchmark_core::{BenchmarkConfig, BenchmarkController, BenchmarkReport, BenchmarkStatus, RuntimeSample, RuntimeSampler};

pub struct PerformanceDiagnostics { controller: Arc<BenchmarkController> }
impl Default for PerformanceDiagnostics { fn default() -> Self { Self { controller: Arc::new(BenchmarkController::new()) } } }
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
