import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Device } from "@/types/device";

const { getDevice } = vi.hoisted(() => ({ getDevice: vi.fn() }));
const { historicalMounts } = vi.hoisted(() => ({ historicalMounts: vi.fn() }));
const { activityMounts } = vi.hoisted(() => ({ activityMounts: vi.fn() }));

vi.mock("@/app/router", () => ({ useRouter: () => ({ goDashboard: vi.fn(), goDeviceSettings: vi.fn() }) }));
vi.mock("@/lib/tauri/devices", () => ({ diagnoseDeviceConnection: vi.fn(), getDevice, openDeviceService: vi.fn(), openDeviceTerminal: vi.fn() }));
vi.mock("@/lib/tauri/monitoring", () => ({ refreshDevice: vi.fn(), getMaintenanceOperation: vi.fn().mockResolvedValue(null) }));
vi.mock("@/stores/useDeviceSnapshots", () => ({ useDeviceSnapshots: () => ({}) }));
vi.mock("@/stores/useDeviceActivity", () => ({ useDeviceActivity: (deviceId: string) => { activityMounts(deviceId); return []; } }));
vi.mock("@/stores/useTerminalSessions", () => ({ useTerminalSessions: () => ({ openTerminal: vi.fn() }) }));
vi.mock("@/features/monitoring/HistoricalTrends", () => ({ HistoricalTrends: ({ deviceId }: { deviceId: string }) => { historicalMounts(deviceId); return <div>Historical Trends</div>; } }));
vi.mock("@/features/devices/DeviceAdministration", () => ({ DeviceAdministration: () => <div /> }));
vi.mock("@/features/devices/HealthDiagnostics", () => ({ DiagnosticsList: () => <div />, HealthDetails: () => <div />, PowerThrottling: () => <div /> }));
vi.mock("@/features/services/ServiceHealth", () => ({ ServiceHealthBadge: () => <div />, ServiceHealthDetails: () => <div /> }));
vi.mock("@/features/devices/ContainerActionsCell", () => ({ ContainerActionsCell: () => <div /> }));
vi.mock("@/features/containers/ContainerDetailDialog", () => ({ ContainerDetailDialog: () => <div /> }));
vi.mock("@/features/devices/DeviceVisibility", () => ({ DeviceVisibility: () => <div /> }));

import { DeviceDetailScreen } from "./DeviceDetailScreen";

const device = {
  id: "device-a",
  name: "Device A",
  host: "fixture.invalid",
  sshPort: 22,
  sshUsername: "fixture",
  deviceType: "raspberry-pi",
  monitoringEnabled: true,
  notifyOnDeviceOffline: true,
  notifyOnContainerFailure: true,
  notifyOnContainerUnhealthy: true,
  services: [],
  createdAt: "2026-08-12T00:00:00Z",
  updatedAt: "2026-08-12T00:00:00Z",
} as Device;

describe("DeviceDetailScreen query gating", () => {
  beforeEach(() => {
    getDevice.mockReset();
    historicalMounts.mockReset();
    activityMounts.mockReset();
    getDevice.mockResolvedValue(device);
  });

  afterEach(cleanup);

  it("mounts Historical and Activity work only after their tabs are selected", async () => {
    render(<DeviceDetailScreen deviceId={device.id} />);

    await screen.findByRole("tab", { name: "overview", selected: true });
    expect(historicalMounts).not.toHaveBeenCalled();
    expect(activityMounts).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("tab", { name: "monitoring" }));
    await waitFor(() => expect(historicalMounts).toHaveBeenCalledWith(device.id));
    expect(activityMounts).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("tab", { name: "activity" }));
    await waitFor(() => expect(activityMounts).toHaveBeenCalledWith(device.id));
    expect(historicalMounts).toHaveBeenCalledTimes(1);
  });
});
