import { invoke } from "@tauri-apps/api/core";
import type { ContainerAction, ContainerLogMode, ContainerLogResult } from "@/types/snapshot";

export function performContainerAction(
  deviceId: string,
  containerId: string,
  action: ContainerAction,
): Promise<void> {
  return invoke<void>("perform_container_action", { deviceId, containerId, action });
}

export function getContainerLogs(deviceId: string, containerId: string, mode: ContainerLogMode): Promise<ContainerLogResult> {
  return invoke<ContainerLogResult>("get_container_logs", { deviceId, containerId, mode });
}
