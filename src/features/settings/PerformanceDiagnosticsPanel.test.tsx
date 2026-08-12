import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BenchmarkReport, BenchmarkStatus } from "@/types/performance";

const { exportPerformanceBenchmarkReport, getPerformanceBenchmarkReport, getPerformanceBenchmarkStatus, startPerformanceBenchmark, stopPerformanceBenchmark } = vi.hoisted(() => ({
  exportPerformanceBenchmarkReport: vi.fn(),
  getPerformanceBenchmarkReport: vi.fn(),
  getPerformanceBenchmarkStatus: vi.fn(),
  startPerformanceBenchmark: vi.fn(),
  stopPerformanceBenchmark: vi.fn(),
}));

vi.mock("@/lib/tauri/performance", () => ({ exportPerformanceBenchmarkReport, getPerformanceBenchmarkReport, getPerformanceBenchmarkStatus, startPerformanceBenchmark, stopPerformanceBenchmark }));

import { PerformanceDiagnosticsPanel } from "./PerformanceDiagnosticsPanel";

const idle: BenchmarkStatus = { state: "idle" };
const running: BenchmarkStatus = { state: "running", session: { id: "session", name: "Manual", scenario: "manual", startedAtUnixMs: 1_000, durationMs: 0 }, sampleCount: 0 };
const stopped: BenchmarkStatus = { state: "stopped", session: { ...running.session!, durationMs: 2_000, endedAtUnixMs: 3_000 } };
const report: BenchmarkReport = { session: stopped.session!, config: { scenario: "manual", sampleIntervalMs: 1_000, maxDurationMs: 900_000, maxSamples: 900 }, samples: [], operations: [], warnings: [] };

describe("PerformanceDiagnosticsPanel", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(1_000);
    getPerformanceBenchmarkStatus.mockReset().mockResolvedValue(idle);
    getPerformanceBenchmarkReport.mockReset().mockResolvedValue(null);
    startPerformanceBenchmark.mockReset().mockResolvedValue(running);
    stopPerformanceBenchmark.mockReset().mockResolvedValue(report);
    exportPerformanceBenchmarkReport.mockReset();
  });

  afterEach(() => { cleanup(); vi.useRealTimers(); });

  it("shows Running with a local elapsed timer and clears it after Stop without status polling", async () => {
    render(<PerformanceDiagnosticsPanel />);
    await act(async () => {});
    expect(screen.getByText("Status: Idle")).toBeInTheDocument();
    expect(getPerformanceBenchmarkStatus).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "Start benchmark" }));
    await act(async () => {});
    expect(screen.getByText("Status: Running · 00:00 (0 samples)")).toBeInTheDocument();
    await act(async () => { await vi.advanceTimersByTimeAsync(2_000); });
    expect(screen.getByText("Status: Running · 00:02 (0 samples)")).toBeInTheDocument();
    expect(getPerformanceBenchmarkStatus).toHaveBeenCalledTimes(1);

    getPerformanceBenchmarkStatus.mockResolvedValueOnce(stopped);
    fireEvent.click(screen.getByRole("button", { name: "Stop benchmark" }));
    await act(async () => {});
    expect(screen.getByText("Status: Stopped")).toBeInTheDocument();
    await act(async () => { await vi.advanceTimersByTimeAsync(2_000); });
    expect(screen.queryByText(/Running ·/)).not.toBeInTheDocument();
    expect(getPerformanceBenchmarkStatus).toHaveBeenCalledTimes(2);
  });

  it("reports native export success and failure instead of silently downloading", async () => {
    getPerformanceBenchmarkStatus.mockResolvedValue(stopped);
    getPerformanceBenchmarkReport.mockResolvedValue(report);
    exportPerformanceBenchmarkReport.mockResolvedValueOnce("C:\\Users\\operator\\Downloads\\pihub-performance-report-session.json");
    render(<PerformanceDiagnosticsPanel />);
    await act(async () => {});
    expect(screen.getByRole("button", { name: "Export JSON" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Export JSON" }));
    await act(async () => {});
    expect(screen.getByRole("status")).toHaveTextContent("Saved report to C:\\Users\\operator\\Downloads\\pihub-performance-report-session.json");

    exportPerformanceBenchmarkReport.mockRejectedValueOnce(new Error("Downloads unavailable"));
    fireEvent.click(screen.getByRole("button", { name: "Export JSON" }));
    await act(async () => {});
    expect(screen.getByRole("alert")).toHaveTextContent("Could not export performance report.");
  });
});
