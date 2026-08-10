import { describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { getDeviceActivity } from "./monitoring";

describe("getDeviceActivity", () => {
  it("uses the device activity command and serializes the registered device ID", () => {
    getDeviceActivity("device-a-id");
    expect(invoke).toHaveBeenCalledWith("get_device_activity", { deviceId: "device-a-id" });
  });
});
