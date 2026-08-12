import type { DeviceConnectionStatus } from "./device";

export type DockerContainerState =
  | "created"
  | "running"
  | "stopped"
  | "exited"
  | "restarting"
  | "paused"
  | "dead"
  | "removing"
  | "unknown";

export type DockerHealthStatus =
  | "healthy"
  | "unhealthy"
  | "starting"
  | "none"
  | "unknown";

export type ContainerAction = "start" | "stop" | "restart";
export type ContainerLogMode = "last100" | "last500" | "last15Minutes" | "last1Hour";
export interface ContainerLogResult { containerId: string; mode: ContainerLogMode; collectedAt: string; content: string; truncated: boolean; }

export interface DockerPortBinding {
  hostIp?: string;
  hostPort?: number;
  containerPort: number;
  protocol: string;
}
export interface DockerMount { mountType: string; source: string; destination: string; readOnly: boolean; }
export interface DockerNetwork { name: string; ipAddress?: string; aliases: string[]; }
export interface DockerLabel { key: string; value: string; redacted: boolean; }
export interface DockerResourceUsage { cpuPercent?: number; memoryUsedBytes?: number; memoryLimitBytes?: number; memoryPercent?: number; }

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
  imageId?: string;
  restartCount?: number;
  restartPolicy?: string;
  restartMaximumRetryCount?: number;
  mounts: DockerMount[];
  networks: DockerNetwork[];
  labels: DockerLabel[];
  resourceUsage?: DockerResourceUsage;
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
export interface NetworkInterface { name: string; linkState: "up" | "down" | "unknown"; macAddress?: string; ipv4Addresses: string[]; ipv6Addresses: string[]; }
export interface NetworkVisibility { interfaces: NetworkInterface[]; defaultRoute?: { interface: string; gateway?: string }; dnsServers: string[]; }
export interface MountedFilesystem { source: string; mountPoint: string; filesystemType: string; totalBytes?: number; usedBytes?: number; availableBytes?: number; usagePercent?: number; readOnly?: boolean; }
export interface StorageVisibility { filesystems: MountedFilesystem[]; }
export interface SystemVisibility { hostname?: string; operatingSystem?: string; kernelVersion?: string; architecture?: string; model?: string; cpuModel?: string; logicalCoreCount?: number; totalMemoryBytes?: number; bootTimestamp?: number; uptimeSeconds?: number; }
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
  networkVisibility?: NetworkVisibility;
  storageVisibility?: StorageVisibility;
  systemVisibility?: SystemVisibility;
}

export type HistoricalRange = "oneHour" | "twentyFourHours" | "sevenDays" | "thirtyDays";
export type HistoricalMetric = "cpuUsagePercent" | "memoryUsagePercent" | "rootFilesystemUsagePercent" | "temperatureCelsius" | "responseTimeMs" | "deviceHealth" | "serviceHealth" | "containerCpuPercent" | "containerMemoryPercent";
export interface HistoricalNumericPoint { timestamp: string; value: number; minimum: number; maximum: number; }
export interface HistoricalStatePoint { timestamp: string; state: string; }
export interface HistoricalSeries { numericPoints: HistoricalNumericPoint[]; statePoints: HistoricalStatePoint[]; }
export type UpdateStatus = "notChecked" | "checking" | "upToDate" | "updatesAvailable" | "stale" | "unsupported" | "checkFailed" | "unknown";
export interface UpdateCheckResult { schemaVersion:number; deviceId:string; status:UpdateStatus; checkedAt?:string; support:{status:"supported"|"unsupported"|"unknown";osId?:string;osVersionId?:string;packageManager?:string;reason?:string}; packageMetadata:{status:"fresh"|"stale"|"unavailable"|"unknown";newestMetadataAt?:string;ageSeconds?:number;staleAfterSeconds:number}; updates:{totalCount?:number;packages:{name:string;installedVersion:string;candidateVersion:string;architecture?:string;held?:boolean}[];truncated:boolean}; heldPackages:{status:"known"|"unknown";totalCount?:number;packages:string[];truncated:boolean}; reboot:"required"|"notRequired"|"unknown";securityUpdates:{status:"unavailable"};warnings:string[];failure?:{kind:string;message:string} }
