import { invoke } from "@tauri-apps/api/core";
import type { Device, DeviceInput } from "../../types/device";

export function getDevices(): Promise<Device[]> {
  return invoke<Device[]>("get_devices");
}

export function getDevice(id: string): Promise<Device | null> {
  return invoke<Device | null>("get_device", { id });
}

export function createDevice(input: DeviceInput): Promise<Device> {
  return invoke<Device>("create_device", { input });
}

export function updateDevice(id: string, input: DeviceInput): Promise<Device> {
  return invoke<Device>("update_device", { id, input });
}

export function deleteDevice(id: string): Promise<void> {
  return invoke<void>("delete_device", { id });
}
