import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { ActivityEvent } from "@/types/activity";

const { getDeviceActivity } = vi.hoisted(() => ({ getDeviceActivity: vi.fn() }));
vi.mock("@/lib/tauri/monitoring", () => ({ getDeviceActivity }));

import { useDeviceActivity } from "./useDeviceActivity";

function RecentActivity({ deviceId }: { deviceId: string }) {
  const activity = useDeviceActivity(deviceId);
  return <>{activity.length === 0 ? "empty" : activity.map((event) => <p key={event.id}>{event.summary}</p>)}</>;
}

const deviceAActivity: ActivityEvent = {
  id: "activity-a", timestamp: "2026-08-10T01:00:00Z", code: "service.health_changed", category: "service", deviceId: "device-a-id", summary: "Device A service changed",
};

describe("useDeviceActivity", () => {
  it("loads persisted activity through the device-specific boundary and retains a genuine empty state", async () => {
    getDeviceActivity.mockResolvedValueOnce([deviceAActivity]).mockResolvedValueOnce([]);
    const view = render(<RecentActivity deviceId="device-a-id" />);

    expect(await screen.findByText("Device A service changed")).toBeInTheDocument();
    expect(getDeviceActivity).toHaveBeenCalledWith("device-a-id");

    view.rerender(<RecentActivity deviceId="device-with-no-activity" />);
    expect(await screen.findByText("empty")).toBeInTheDocument();
    expect(getDeviceActivity).toHaveBeenLastCalledWith("device-with-no-activity");
  });
});
