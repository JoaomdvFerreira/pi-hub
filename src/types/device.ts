export type DeviceType =
  | "raspberry-pi"
  | "linux-server"
  | "mini-pc"
  | "nas"
  | "other";

export interface DeviceService {
  id: string;
  name: string;
  url: string;
  icon?: string;
  description?: string;
  enabled: boolean;
}

export interface Device {
  id: string;
  name: string;
  host: string;
  sshPort: number;
  sshUsername: string;
  description?: string;
  deviceType: DeviceType;
  monitoringEnabled: boolean;
  refreshIntervalSeconds?: number;
  notificationsEnabled: boolean;
  services: DeviceService[];
  createdAt: string;
  updatedAt: string;
}

/** The editable fields of a device, used for both create and update. */
export interface DeviceInput {
  name: string;
  host: string;
  sshPort: number;
  sshUsername: string;
  description?: string;
  deviceType: DeviceType;
  monitoringEnabled: boolean;
  refreshIntervalSeconds?: number;
  notificationsEnabled: boolean;
  services: DeviceService[];
}

export type DeviceConnectionStatus =
  | "unknown"
  | "checking"
  | "online"
  | "offline"
  | "timeout"
  | "authentication_error"
  | "host_key_error"
  | "command_error";

export interface TestConnectionInput {
  host: string;
  sshPort: number;
  sshUsername: string;
}

export interface ConnectionTestResult {
  status: DeviceConnectionStatus;
  message?: string;
}
