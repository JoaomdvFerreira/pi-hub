import { invoke } from "@tauri-apps/api/core";
import type { DeviceSnapshot } from "../../types/snapshot";
import type { ActivityEvent } from "../../types/activity";

export function refreshDevice(id: string): Promise<DeviceSnapshot> {
  return invoke<DeviceSnapshot>("refresh_device", { id });
}

export function refreshAllDevices(): Promise<DeviceSnapshot[]> {
  return invoke<DeviceSnapshot[]>("refresh_all_devices");
}

export function getLatestSnapshot(id: string): Promise<DeviceSnapshot | null> {
  return invoke<DeviceSnapshot | null>("get_latest_snapshot", { id });
}

export function getActivity(): Promise<ActivityEvent[]> {
  return invoke<ActivityEvent[]>("get_activity");
}
