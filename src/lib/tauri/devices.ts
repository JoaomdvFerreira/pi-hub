import { invoke } from "@tauri-apps/api/core";
import type {
  ConnectionTestResult,
  Device,
  DeviceInput,
  TestConnectionInput,
} from "../../types/device";

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

export function testDeviceConnection(
  input: TestConnectionInput,
): Promise<ConnectionTestResult> {
  return invoke<ConnectionTestResult>("test_device_connection", { input });
}

export function openDeviceService(deviceId: string, serviceId: string): Promise<void> {
  return invoke<void>("open_device_service", { deviceId, serviceId });
}

export function openDeviceTerminal(deviceId: string): Promise<void> {
  return invoke<void>("open_device_terminal", { deviceId });
}
