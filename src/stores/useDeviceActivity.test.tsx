import { render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ActivityEvent } from "@/types/activity";

const { getDeviceActivity, listen } = vi.hoisted(() => ({ getDeviceActivity: vi.fn(), listen: vi.fn() }));
vi.mock("@/lib/tauri/monitoring", () => ({ getDeviceActivity }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

import { useDeviceActivity } from "./useDeviceActivity";

function RecentActivity({ deviceId }: { deviceId: string }) {
  const activity = useDeviceActivity(deviceId);
  return <>{activity.length === 0 ? "empty" : activity.map((event) => <p key={event.id}>{event.summary}</p>)}</>;
}

const deviceAActivity: ActivityEvent = {
  id: "activity-a", timestamp: "2026-08-10T01:00:00Z", code: "service.health_changed", category: "service", deviceId: "device-a-id", summary: "Device A service changed",
};
const deviceBActivity: ActivityEvent = { ...deviceAActivity, id: "activity-b", deviceId: "device-b-id", summary: "Device B service changed" };

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

beforeEach(() => { listen.mockResolvedValue(vi.fn()); });

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

  it("never renders or commits previous-device activity during a rapid device switch", async () => {
    const deviceA = deferred<ActivityEvent[]>();
    const deviceB = deferred<ActivityEvent[]>();
    getDeviceActivity.mockReturnValueOnce(deviceA.promise).mockReturnValueOnce(deviceB.promise);
    const view = render(<RecentActivity deviceId="device-a-id" />);

    view.rerender(<RecentActivity deviceId="device-b-id" />);
    expect(within(view.container).getByText("empty")).toBeInTheDocument();

    deviceA.resolve([deviceAActivity]);
    deviceB.resolve([deviceBActivity]);
    expect(await within(view.container).findByText("Device B service changed")).toBeInTheDocument();
    expect(within(view.container).queryByText("Device A service changed")).not.toBeInTheDocument();
  });
});
