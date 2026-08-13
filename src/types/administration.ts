export type AdministrationOperationType = "restartDevice" | "shutdownDevice" | "restartDocker" | "restartTailscale";
export type AdministrationOperationState = "requested" | "dispatching" | "commandAccepted" | "verifying" | "waitingForOffline" | "waitingForOnline" | "completed" | "failed" | "timedOut" | "outcomeUncertain";
export interface AdministrationOperation { id: string; deviceId: string; operationType: AdministrationOperationType; requestedAt: string; state: AdministrationOperationState; acceptedAt?: string; completedAt?: string; failure?: string; detail?: string; }
export type ExpectedDisruptionOperationType = AdministrationOperationType | "updateDevice";
export type ExpectedDisruptionPhase = "commandAccepted" | "updateActive";
export interface ExpectedDisruption { deviceId: string; operationId: string; operationType: ExpectedDisruptionOperationType; startedAt: string; expectedUntil: string; phase: ExpectedDisruptionPhase; }
