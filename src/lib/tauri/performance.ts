import { invoke } from "@tauri-apps/api/core";
import type { BenchmarkReport, BenchmarkStatus } from "@/types/performance";
export const getPerformanceBenchmarkStatus = () => invoke<BenchmarkStatus>("get_performance_benchmark_status");
export const startPerformanceBenchmark = () => invoke<BenchmarkStatus>("start_performance_benchmark");
export const stopPerformanceBenchmark = () => invoke<BenchmarkReport | null>("stop_performance_benchmark");
export const getPerformanceBenchmarkReport = () => invoke<BenchmarkReport | null>("get_performance_benchmark_report");
export const exportPerformanceBenchmarkReport = () => invoke<string>("export_performance_benchmark_report");
