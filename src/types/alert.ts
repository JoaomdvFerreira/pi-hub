export type AlertState = "active" | "acknowledged" | "resolved";
export type AlertSeverity = "info" | "warning" | "critical";
export type AlertCategory = "device" | "health" | "service" | "container";
export interface Alert {
  id: string; deduplicationKey: string; category: AlertCategory; deviceId?: string; entityId?: string; ruleCode: string;
  severity: AlertSeverity; state: AlertState; firstSeen: string; lastSeen: string; occurrenceCount: number;
  acknowledgedAt?: string; resolvedAt?: string; resolutionReason?: string; summary: string; detail?: string; threshold?: string;
}
