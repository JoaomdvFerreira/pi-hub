import type { DeviceConnectionStatus } from "@/types/device";

const LABELS: Record<DeviceConnectionStatus, string> = {
  unknown: "Unknown",
  checking: "Checking…",
  online: "Online",
  offline: "Offline",
  timeout: "Timed out",
  authentication_error: "Authentication failed",
  host_key_error: "Host key error",
  command_error: "Command error",
};

/** Tailwind color-token classes for text, matching the status palette in src/globals.css. */
const COLOR_CLASSES: Record<DeviceConnectionStatus, string> = {
  unknown: "text-muted-foreground",
  checking: "text-muted-foreground",
  online: "text-status-healthy",
  offline: "text-status-offline",
  timeout: "text-status-warning",
  authentication_error: "text-status-offline",
  host_key_error: "text-status-offline",
  command_error: "text-status-warning",
};

export function connectionStatusLabel(status: DeviceConnectionStatus): string {
  return LABELS[status];
}

export function connectionStatusColorClass(status: DeviceConnectionStatus): string {
  return COLOR_CLASSES[status];
}
