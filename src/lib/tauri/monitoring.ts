import { invoke } from "@tauri-apps/api/core";
import type { DeviceSnapshot, HistoricalMetric, HistoricalRange, HistoricalSeries, UpdateCheckResult } from "../../types/snapshot";
import type { ActivityEvent } from "../../types/activity";
import type { Alert } from "../../types/alert";
import type { MaintenanceOperation } from "../../types/maintenance";

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
export function getDeviceActivity(deviceId: string): Promise<ActivityEvent[]> {
  return invoke<ActivityEvent[]>("get_device_activity", { deviceId });
}
export function getHistoricalSeries(deviceId: string, entityId: string | undefined, metric: HistoricalMetric, range: HistoricalRange): Promise<HistoricalSeries> { return invoke<HistoricalSeries>("get_historical_series", { deviceId, entityId, metric, range }); }
export function getAlerts(): Promise<Alert[]> { return invoke<Alert[]>("get_alerts"); }
export function acknowledgeAlert(id: string): Promise<Alert> { return invoke<Alert>("acknowledge_alert", { id }); }
export function getUpdateResult(deviceId:string):Promise<UpdateCheckResult|null>{return invoke("get_update_result",{deviceId});}
export function getMaintenanceOperation(deviceId:string):Promise<MaintenanceOperation|null>{return invoke("get_maintenance_operation",{deviceId});}
export interface PreparedDeviceUpdate { operation: MaintenanceOperation; plan: UpdateCheckResult; }
export function prepareDeviceUpdate(deviceId:string):Promise<PreparedDeviceUpdate>{return invoke("prepare_device_update",{deviceId});}
export function applyPreparedDeviceUpdate(deviceId:string):Promise<MaintenanceOperation>{return invoke("apply_prepared_device_update",{deviceId});}
export function reconcileDeviceUpdate(deviceId:string):Promise<MaintenanceOperation>{return invoke("reconcile_device_update",{deviceId});}
export function checkForUpdates(deviceId:string):Promise<UpdateCheckResult>{return invoke("check_for_updates",{deviceId});}
