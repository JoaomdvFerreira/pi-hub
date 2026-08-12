import { invoke } from "@tauri-apps/api/core";
import type { BenchmarkConfig, BenchmarkReport, BenchmarkStatus } from "@/types/performance";
export const getPerformanceBenchmarkStatus = () => invoke<BenchmarkStatus>("get_performance_benchmark_status");
export const startPerformanceBenchmark = (config?: BenchmarkConfig) => invoke<BenchmarkStatus>("start_performance_benchmark", { config });
export const stopPerformanceBenchmark = () => invoke<BenchmarkReport | null>("stop_performance_benchmark");
export const getPerformanceBenchmarkReport = () => invoke<BenchmarkReport | null>("get_performance_benchmark_report");
