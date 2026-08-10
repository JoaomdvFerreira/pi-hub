import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { HistoricalTrends } from "./HistoricalTrends";

const { getHistoricalSeries } = vi.hoisted(() => ({ getHistoricalSeries: vi.fn() }));
vi.mock("@/lib/tauri/monitoring", () => ({ getHistoricalSeries }));
const trends = [{ label: "CPU usage", metric: "cpuUsagePercent" as const, unit: "%" }, { label: "Device health", metric: "deviceHealth" as const, unit: "" }];
describe("HistoricalTrends", () => {
  it("shows numeric points, state transitions, and requests only the selected bounded range", async () => { getHistoricalSeries.mockImplementation((_d: string, _e: string, metric: string) => Promise.resolve(metric === "deviceHealth" ? { numericPoints: [], statePoints: [{ timestamp: "2026-08-10T00:00:00Z", state: "healthy" }] } : { numericPoints: [{ timestamp: "2026-08-10T00:00:00Z", value: 42, minimum: 40, maximum: 60 }], statePoints: [] })); render(<HistoricalTrends deviceId="device" trends={trends} />); await waitFor(() => expect(document.body.textContent).toContain("healthy")); expect(screen.getByText(/1 samples/)).toBeInTheDocument(); fireEvent.click(screen.getByRole("button", { name: "7d" })); await waitFor(() => expect(getHistoricalSeries).toHaveBeenLastCalledWith("device", undefined, "deviceHealth", "sevenDays")); });
  it("makes empty history explicit", async () => { getHistoricalSeries.mockResolvedValue({ numericPoints: [], statePoints: [] }); render(<HistoricalTrends deviceId="device" trends={[trends[0]]} />); expect(await screen.findByText("No historical samples in this range.")).toBeInTheDocument(); });
});
