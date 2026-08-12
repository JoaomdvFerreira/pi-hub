import { describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { exportPerformanceBenchmarkReport, getPerformanceBenchmarkReport, getPerformanceBenchmarkStatus, startPerformanceBenchmark, stopPerformanceBenchmark } from "./performance";

describe("performance diagnostics commands", () => {
  it("uses only the typed diagnostics command boundary", () => {
    startPerformanceBenchmark();
    getPerformanceBenchmarkStatus();
    getPerformanceBenchmarkReport();
    stopPerformanceBenchmark();
    exportPerformanceBenchmarkReport();
    expect(invoke).toHaveBeenNthCalledWith(1, "start_performance_benchmark", { config: undefined });
    expect(invoke).toHaveBeenNthCalledWith(2, "get_performance_benchmark_status");
    expect(invoke).toHaveBeenNthCalledWith(3, "get_performance_benchmark_report");
    expect(invoke).toHaveBeenNthCalledWith(4, "stop_performance_benchmark");
    expect(invoke).toHaveBeenNthCalledWith(5, "export_performance_benchmark_report");
  });
});
