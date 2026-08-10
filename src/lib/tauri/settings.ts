import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, ThresholdPolicy, ThresholdPolicyOverrides } from "../../types/settings";

export function getAppSettings(): Promise<AppSettings> {
  return invoke<AppSettings>("get_app_settings");
}
export function getEffectiveThresholdPolicy(deviceId?: string): Promise<ThresholdPolicy> { return invoke<ThresholdPolicy>("get_effective_threshold_policy", { deviceId }); }
export function saveDeviceThresholdOverrides(deviceId: string, overrides: ThresholdPolicyOverrides): Promise<AppSettings> { return invoke<AppSettings>("save_device_threshold_overrides", { deviceId, overrides }); }
export function clearDeviceThresholdOverrides(deviceId: string): Promise<AppSettings> { return invoke<AppSettings>("clear_device_threshold_overrides", { deviceId }); }

export function saveAppSettings(settings: AppSettings): Promise<AppSettings> {
  return invoke<AppSettings>("save_app_settings", { settings });
}
