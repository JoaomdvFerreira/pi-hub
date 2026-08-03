import type { DeviceConnectionStatus } from "./device";

export type DockerContainerState =
  | "running"
  | "stopped"
  | "exited"
  | "restarting"
  | "paused"
  | "dead"
  | "unknown";

export type DockerHealthStatus =
  | "healthy"
  | "unhealthy"
  | "starting"
  | "none"
  | "unknown";

export type ContainerAction = "start" | "stop" | "restart";

export interface DockerPortBinding {
  hostIp?: string;
  hostPort?: number;
  containerPort: number;
  protocol: string;
}

export interface DockerContainerSummary {
  id: string;
  name: string;
  image: string;
  state: DockerContainerState;
  statusText: string;
  health: DockerHealthStatus;
  ports: DockerPortBinding[];
  createdAt?: string;
  startedAt?: string;
}

export interface SystemMetrics {
  hostname?: string;
  model?: string;
  operatingSystem?: string;
  kernelVersion?: string;
  architecture?: string;
  uptimeSeconds?: number;
  cpuUsagePercent?: number;
  loadAverage1m?: number;
  loadAverage5m?: number;
  loadAverage15m?: number;
  memoryTotalBytes?: number;
  memoryUsedBytes?: number;
  diskTotalBytes?: number;
  diskUsedBytes?: number;
  temperatureCelsius?: number;
}

export interface DeviceSnapshot {
  deviceId: string;
  connectionStatus: DeviceConnectionStatus;
  capturedAt: string;
  durationMs: number;
  metrics?: SystemMetrics;
  dockerAvailable: boolean;
  containers: DockerContainerSummary[];
  warnings: string[];
  error?: {
    code: string;
    message: string;
    remediation?: string;
    retryable: boolean;
  };
  stale: boolean;
  lastSuccessfulRefresh?: string;
}
