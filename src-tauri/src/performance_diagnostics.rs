#[cfg(test)]
use pihub_benchmark_core::BenchmarkScenario;
use pihub_benchmark_core::{
    BenchmarkConfig, BenchmarkController, BenchmarkReport, BenchmarkStatus, OperationMeasurement,
    RuntimeSample, RuntimeSampler,
};
#[cfg(test)]
use std::time::Duration;
use std::{
    sync::{Arc, OnceLock},
    time::Instant,
};

static CONTROLLER: OnceLock<Arc<BenchmarkController>> = OnceLock::new();
#[cfg(test)]
thread_local! { static TEST_MEASUREMENT_SCOPE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
#[cfg(test)]
pub(crate) static TEST_MEASUREMENT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
#[cfg(test)]
pub(crate) struct TestMeasurementScope;
#[cfg(test)]
impl Drop for TestMeasurementScope {
    fn drop(&mut self) {
        TEST_MEASUREMENT_SCOPE.with(|scope| scope.set(false));
    }
}
#[cfg(test)]
pub(crate) fn enable_test_measurement_scope() -> TestMeasurementScope {
    TEST_MEASUREMENT_SCOPE.with(|scope| scope.set(true));
    TestMeasurementScope
}
#[cfg(test)]
pub(crate) fn test_measurement_lock() -> &'static std::sync::Mutex<()> {
    &TEST_MEASUREMENT_LOCK
}
pub fn measure(label: &'static str) -> Option<OperationMeasurement<'static>> {
    #[cfg(test)]
    if !TEST_MEASUREMENT_SCOPE.with(|scope| scope.get()) {
        return None;
    }
    CONTROLLER.get().map(|controller| controller.measure(label))
}

pub struct PerformanceDiagnostics {
    controller: Arc<BenchmarkController>,
}
impl Default for PerformanceDiagnostics {
    fn default() -> Self {
        let controller = CONTROLLER
            .get_or_init(|| Arc::new(BenchmarkController::new()))
            .clone();
        Self { controller }
    }
}
impl PerformanceDiagnostics {
    pub fn start(&self, config: BenchmarkConfig) -> Result<BenchmarkStatus, String> {
        self.controller.start(config)?;
        let controller = self.controller.clone();
        tauri::async_runtime::spawn(async move {
            let mut sampler = PlatformRuntimeSampler::default();
            while controller.is_active() {
                let Some(interval) = controller.sample_interval() else {
                    break;
                };
                tokio::time::sleep(interval).await;
                if !controller.is_active() {
                    break;
                }
                controller.record_sample(sampler.sample());
            }
        });
        Ok(self.controller.status())
    }
    pub fn stop(&self) -> Option<BenchmarkReport> {
        self.controller.stop()
    }
    pub fn status(&self) -> BenchmarkStatus {
        self.controller.status()
    }
    pub fn report(&self) -> Option<BenchmarkReport> {
        self.controller.report()
    }
}

#[derive(Clone, Copy)]
struct ProcessCpuTiming {
    total_cpu_100ns: u64,
    observed_at: Instant,
    logical_processors: u32,
}

fn process_cpu_percent(previous: ProcessCpuTiming, current: ProcessCpuTiming) -> Option<f32> {
    let elapsed = current
        .observed_at
        .checked_duration_since(previous.observed_at)?;
    if elapsed.is_zero()
        || current.logical_processors == 0
        || current.total_cpu_100ns < previous.total_cpu_100ns
    {
        return None;
    }
    let used_ns = (current.total_cpu_100ns - previous.total_cpu_100ns) as f64 * 100.0;
    let capacity_ns = elapsed.as_nanos() as f64 * current.logical_processors as f64;
    let percent = used_ns / capacity_ns * 100.0;
    (percent.is_finite() && (0.0..=100.0).contains(&percent)).then_some(percent as f32)
}

fn update_process_cpu(
    previous: &mut Option<ProcessCpuTiming>,
    current: Option<ProcessCpuTiming>,
) -> Option<f32> {
    let current = current?;
    let percent = previous.and_then(|previous| process_cpu_percent(previous, current));
    *previous = Some(current);
    percent
}

#[cfg(windows)]
#[derive(Default)]
struct PlatformRuntimeSampler {
    previous_cpu: Option<ProcessCpuTiming>,
}
#[cfg(windows)]
impl RuntimeSampler for PlatformRuntimeSampler {
    fn sample(&mut self) -> RuntimeSample {
        use windows::Win32::{
            Foundation::FILETIME,
            System::{
                ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX},
                Threading::{GetCurrentProcess, GetProcessHandleCount, GetProcessTimes},
            },
        };
        let process = unsafe { GetCurrentProcess() };
        let mut memory = PROCESS_MEMORY_COUNTERS_EX::default();
        memory.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
        let memory_ok =
            unsafe { GetProcessMemoryInfo(process, &mut memory as *mut _ as _, memory.cb) }.is_ok();
        let mut handles = 0;
        let handles_ok = unsafe { GetProcessHandleCount(process, &mut handles) }.is_ok();
        let mut creation = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        let current_cpu =
            unsafe { GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user) }
                .ok()
                .and_then(|_| {
                    let to_100ns = |time: FILETIME| {
                        (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime)
                    };
                    Some(ProcessCpuTiming {
                        total_cpu_100ns: to_100ns(kernel).checked_add(to_100ns(user))?,
                        observed_at: Instant::now(),
                        logical_processors: std::thread::available_parallelism().ok()?.get() as u32,
                    })
                });
        let process_cpu_percent = if current_cpu.is_some() {
            update_process_cpu(&mut self.previous_cpu, current_cpu)
        } else {
            self.previous_cpu = None;
            None
        };
        RuntimeSample {
            process_cpu_percent,
            resident_memory_bytes: memory_ok.then_some(memory.WorkingSetSize as u64),
            private_memory_bytes: memory_ok.then_some(memory.PrivateUsage as u64),
            thread_count: None,
            handle_count: handles_ok.then_some(handles),
        }
    }
}
#[cfg(not(windows))]
#[derive(Default)]
struct PlatformRuntimeSampler;
#[cfg(not(windows))]
impl RuntimeSampler for PlatformRuntimeSampler {
    fn sample(&mut self) -> RuntimeSample {
        RuntimeSample::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    struct TestMeasurementScope;
    impl TestMeasurementScope {
        fn enable() -> Self {
            TEST_MEASUREMENT_SCOPE.with(|scope| scope.set(true));
            Self
        }
    }
    impl Drop for TestMeasurementScope {
        fn drop(&mut self) {
            TEST_MEASUREMENT_SCOPE.with(|scope| scope.set(false));
        }
    }
    fn timing_at(
        start: Instant,
        total_cpu_100ns: u64,
        elapsed: Duration,
        logical_processors: u32,
    ) -> ProcessCpuTiming {
        ProcessCpuTiming {
            total_cpu_100ns,
            observed_at: start + elapsed,
            logical_processors,
        }
    }
    #[test]
    fn process_cpu_first_sample_is_unavailable_until_a_prior_timing_exists() {
        let start = Instant::now();
        let mut previous = None;
        assert_eq!(
            update_process_cpu(&mut previous, Some(timing_at(start, 0, Duration::ZERO, 4))),
            None
        );
        assert_eq!(
            update_process_cpu(
                &mut previous,
                Some(timing_at(start, 0, Duration::from_secs(1), 4))
            ),
            Some(0.0)
        );
    }
    #[test]
    fn process_cpu_delta_is_normalized_to_total_logical_processor_capacity() {
        let start = Instant::now();
        assert_eq!(
            process_cpu_percent(
                timing_at(start, 0, Duration::ZERO, 4),
                timing_at(start, 20_000_000, Duration::from_secs(1), 4)
            ),
            Some(50.0)
        );
    }
    #[test]
    fn process_cpu_zero_delta_reports_idle_zero() {
        let start = Instant::now();
        assert_eq!(
            process_cpu_percent(
                timing_at(start, 10, Duration::ZERO, 4),
                timing_at(start, 10, Duration::from_secs(1), 4)
            ),
            Some(0.0)
        );
    }
    #[test]
    fn process_cpu_non_zero_delta_reports_usage() {
        let start = Instant::now();
        assert_eq!(
            process_cpu_percent(
                timing_at(start, 0, Duration::ZERO, 2),
                timing_at(start, 5_000_000, Duration::from_secs(1), 2)
            ),
            Some(25.0)
        );
    }
    #[test]
    fn process_cpu_invalid_timing_is_unavailable() {
        let start = Instant::now();
        assert_eq!(
            process_cpu_percent(
                timing_at(start, 20, Duration::from_secs(1), 4),
                timing_at(start, 10, Duration::ZERO, 4)
            ),
            None
        );
        assert_eq!(
            process_cpu_percent(
                timing_at(start, 0, Duration::ZERO, 0),
                timing_at(start, 1, Duration::from_secs(1), 0)
            ),
            None
        );
        assert_eq!(
            process_cpu_percent(
                timing_at(start, 0, Duration::ZERO, 1),
                timing_at(start, 1_000_000_000, Duration::from_nanos(1), 1)
            ),
            None
        );
        let mut previous = Some(timing_at(start, 0, Duration::ZERO, 1));
        assert_eq!(update_process_cpu(&mut previous, None), None);
        assert!(previous.is_some());
    }
    #[test]
    fn synthetic_refresh_aggregates_only_stable_labels_when_active() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let _scope = TestMeasurementScope::enable();
        let diagnostics = PerformanceDiagnostics::default();
        let _ = diagnostics.stop();
        diagnostics
            .start(BenchmarkConfig {
                max_samples: 4,
                ..Default::default()
            })
            .unwrap();
        crate::monitoring::synthetic::run(crate::monitoring::synthetic::SyntheticProfile::Small);
        let report = diagnostics.stop().unwrap();
        let labels: Vec<_> = report
            .operations
            .iter()
            .map(|item| item.name.as_str())
            .collect();
        assert!(labels.contains(&"monitoring.device_refresh"));
        assert!(labels.contains(&"docker.collect"));
        assert!(labels
            .iter()
            .all(|label| !label.contains("fixture") && !label.contains('@')));
    }
    #[test]
    fn deterministic_profiles_produce_linear_refresh_request_counts() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let _scope = TestMeasurementScope::enable();
        let diagnostics = PerformanceDiagnostics::default();
        let _ = diagnostics.stop();
        for (profile, devices) in [
            (crate::monitoring::synthetic::SyntheticProfile::Small, 2_u64),
            (crate::monitoring::synthetic::SyntheticProfile::Medium, 5),
            (
                crate::monitoring::synthetic::SyntheticProfile::LargerLocal,
                10,
            ),
        ] {
            diagnostics
                .start(BenchmarkConfig {
                    scenario: BenchmarkScenario::Synthetic,
                    max_samples: 1,
                    ..Default::default()
                })
                .unwrap();
            crate::monitoring::synthetic::run(profile);
            let report = diagnostics.stop().unwrap();
            let operation = |name: &str| report.operations.iter().find(|item| item.name == name);
            let count = |name: &str| operation(name).map(|item| item.count).unwrap_or(0);
            assert_eq!(count("monitoring.device_refresh"), devices);
            assert_eq!(count("ssh.execute"), devices * 6);
            assert_eq!(count("docker.collect"), devices);
            assert_eq!(count("visibility.network"), devices);
            assert_eq!(count("visibility.storage"), devices);
            assert_eq!(count("visibility.system"), devices);
            assert_eq!(operation("ssh.execute").map(|item| item.failures), Some(0));
            assert_eq!(
                operation("ssh.execute").and_then(|item| item.bytes),
                Some(
                    devices * 6 * crate::monitoring::synthetic::ONLINE_FIXTURE_OUTPUT.len() as u64
                )
            );
        }
    }
}
