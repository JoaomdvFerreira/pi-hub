import { invoke } from "@tauri-apps/api/core";
import type { ContainerAction } from "@/types/snapshot";

export function performContainerAction(
  deviceId: string,
  containerId: string,
  action: ContainerAction,
): Promise<void> {
  return invoke<void>("perform_container_action", { deviceId, containerId, action });
}
