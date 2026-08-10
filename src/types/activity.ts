export type ActivityCategory = "device" | "health" | "service" | "container" | "diagnostic" | "administration";
export interface ActivityEvent { id: string; timestamp: string; code: string; category: ActivityCategory; deviceId?: string; entityId?: string; entityName?: string; summary: string; detail?: string; }
