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
  swapTotalBytes?: number;
  swapUsedBytes?: number;
  cpuFrequencyMhz?: number;
  bootTimestamp?: number;
  rebootRequired?: boolean;
  rootFilesystemReadOnly?: boolean;
  throttlingRaw?: string;
}
export type DeviceHealthState = "healthy" | "warning" | "critical" | "unknown";
export interface HealthReason { code: string; severity: "info" | "warning" | "critical"; summary: string; }
export interface RaspberryPiPowerState { raw?: string; undervoltageNow?: boolean; undervoltageSinceBoot?: boolean; frequencyCappedNow?: boolean; frequencyCappedSinceBoot?: boolean; throttledNow?: boolean; throttledSinceBoot?: boolean; softTemperatureLimitNow?: boolean; softTemperatureLimitSinceBoot?: boolean; }
export interface DeviceHealthAssessment { state: DeviceHealthState; reasons: HealthReason[]; power: RaspberryPiPowerState; }
export type ServiceHealthState = "unknown" | "healthy" | "degraded" | "unavailable";
export type ServiceFailureReason = "http_status" | "connection" | "timeout" | "tls" | "redirect" | "invalid_url" | "unknown";
export interface ServiceHealthRecord { serviceId: string; state: ServiceHealthState; consecutiveFailures: number; latestHttpStatus?: number; latestResponseTimeMs?: number; lastCheckedAt?: string; lastSuccessfulCheckAt?: string; latestFailureReason?: ServiceFailureReason; }
export interface DiagnosticCheck { code: string; status: "passed" | "warning" | "failed" | "skipped"; summary: string; detail?: string; durationMs?: number; }
export interface ConnectivityDiagnosticReport { checks: DiagnosticCheck[]; durationMs: number; }

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
  health: DeviceHealthAssessment;
  serviceHealth: Record<string, ServiceHealthRecord>;
}
