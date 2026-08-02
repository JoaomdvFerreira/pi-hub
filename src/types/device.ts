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
