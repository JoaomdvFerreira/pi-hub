export type Theme = "dark" | "light" | "system";

export interface AppSettings {
  schemaVersion: number;
  refreshIntervalSeconds: number;
  startWithWindows: boolean;
  minimizeToTray: boolean;
  notificationsEnabled: boolean;
  theme: Theme;
  thresholdPolicy: ThresholdPolicy;
  deviceThresholdOverrides: Record<string, ThresholdPolicyOverrides>;
}

export interface ThresholdPolicy { cpuWarningPercent: number; cpuCriticalPercent: number; cpuDurationSeconds: number; memoryWarningPercent: number; memoryCriticalPercent: number; memoryDurationSeconds: number; diskWarningPercent: number; diskCriticalPercent: number; temperatureWarningCelsius: number; temperatureCriticalCelsius: number; temperatureConsecutiveSamples: number; serviceUnavailableFailures: number; }
export type ThresholdPolicyOverrides = Partial<ThresholdPolicy>;

export interface ApplicationError {
  code: string;
  message: string;
  remediation?: string;
  retryable: boolean;
}
