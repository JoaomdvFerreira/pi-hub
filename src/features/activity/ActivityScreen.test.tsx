import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { getActivity, getDevices } = vi.hoisted(() => ({ getActivity: vi.fn(), getDevices: vi.fn() }));
vi.mock("@/lib/tauri/monitoring", () => ({ getActivity }));
vi.mock("@/lib/tauri/devices", () => ({ getDevices }));

import { ActivityScreen } from "./ActivityScreen";

const events = [
  { id: "event-a", timestamp: "2026-08-12T12:00:00Z", code: "device.updated", category: "device" as const, deviceId: "device-a-id", summary: "Kitchen Pi updated" },
  { id: "event-b", timestamp: "2026-08-12T11:00:00Z", code: "device.updated", category: "device" as const, deviceId: "deleted-device-id", summary: "Deleted device updated" },
];

beforeEach(() => {
  getActivity.mockReset().mockResolvedValue(events);
  getDevices.mockReset().mockResolvedValue([{ id: "device-a-id", name: "Kitchen Pi" }]);
});
afterEach(cleanup);

describe("ActivityScreen", () => {
  it("labels activity device filters from inventory while retaining exact device IDs as values", async () => {
    render(<ActivityScreen />);
    await screen.findByText("Kitchen Pi updated");

    const deviceFilter = screen.getAllByRole("combobox")[1];
    expect(screen.getByRole("option", { name: "All devices" })).toHaveValue("all");
    expect(screen.getByRole("option", { name: "Kitchen Pi" })).toHaveValue("device-a-id");
    expect(screen.queryByRole("option", { name: "device-a-id" })).not.toBeInTheDocument();

    fireEvent.change(deviceFilter, { target: { value: "device-a-id" } });
    expect(screen.getByText("Kitchen Pi updated")).toBeInTheDocument();
    expect(screen.queryByText("Deleted device updated")).not.toBeInTheDocument();
  });

  it("keeps activity for missing inventory devices selectable with a deleted-device fallback", async () => {
    render(<ActivityScreen />);
    await screen.findByText("Deleted device updated");

    const deviceFilter = screen.getAllByRole("combobox")[1];
    expect(screen.getByRole("option", { name: "Deleted device" })).toHaveValue("deleted-device-id");
    fireEvent.change(deviceFilter, { target: { value: "deleted-device-id" } });
    expect(screen.getByText("Deleted device updated")).toBeInTheDocument();
    expect(screen.queryByText("Kitchen Pi updated")).not.toBeInTheDocument();
  });
});
