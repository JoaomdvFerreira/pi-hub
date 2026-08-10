import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

const { getAlerts, acknowledgeAlert } = vi.hoisted(() => ({ getAlerts: vi.fn(), acknowledgeAlert: vi.fn() }));
vi.mock("@/lib/tauri/monitoring", () => ({ getAlerts, acknowledgeAlert }));
import { AlertCenterScreen } from "./AlertCenterScreen";

describe("AlertCenterScreen", () => {
  it("filters resolved history and acknowledges an active alert", async () => {
    getAlerts.mockResolvedValueOnce([{ id: "a", severity: "warning", state: "active", category: "health", summary: "Disk usage is high", lastSeen: "2026-08-10T00:00:00Z", firstSeen: "2026-08-10T00:00:00Z", deduplicationKey: "a", ruleCode: "disk", occurrenceCount: 1 }, { id: "b", severity: "critical", state: "resolved", category: "service", summary: "Service recovered", lastSeen: "2026-08-09T00:00:00Z", firstSeen: "2026-08-09T00:00:00Z", deduplicationKey: "b", ruleCode: "service", occurrenceCount: 1 }]);
    getAlerts.mockResolvedValueOnce([]); acknowledgeAlert.mockResolvedValue({});
    render(<AlertCenterScreen />);
    expect(await screen.findByText("Disk usage is high")).toBeInTheDocument();
    expect(screen.queryByText("Service recovered")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Acknowledge Disk usage is high" }));
    expect(acknowledgeAlert).toHaveBeenCalledWith("a");
  });
});
