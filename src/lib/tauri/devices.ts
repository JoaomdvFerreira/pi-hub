import { invoke } from "@tauri-apps/api/core";
import type { Device } from "../../types/device";

export function getDevices(): Promise<Device[]> {
  return invoke<Device[]>("get_devices");
}

export function getDevice(id: string): Promise<Device | null> {
  return invoke<Device | null>("get_device", { id });
}
