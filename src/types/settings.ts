export type Theme = "dark" | "light" | "system";

export interface AppSettings {
  schemaVersion: number;
  refreshIntervalSeconds: number;
  startWithWindows: boolean;
  minimizeToTray: boolean;
  notificationsEnabled: boolean;
  theme: Theme;
}

export interface ApplicationError {
  code: string;
  message: string;
  remediation?: string;
  retryable: boolean;
}
