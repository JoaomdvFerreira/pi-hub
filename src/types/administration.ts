export type AdministrationOperationType = "restartDevice" | "shutdownDevice" | "restartDocker" | "restartTailscale";
export type AdministrationOperationState = "requested" | "dispatching" | "commandAccepted" | "verifying" | "waitingForOffline" | "waitingForOnline" | "completed" | "failed" | "timedOut" | "outcomeUncertain";
export interface AdministrationOperation { id: string; deviceId: string; operationType: AdministrationOperationType; requestedAt: string; state: AdministrationOperationState; acceptedAt?: string; completedAt?: string; failure?: string; detail?: string; }
export interface ExpectedDisruption { deviceId: string; operationId: string; operationType: AdministrationOperationType; startedAt: string; expectedUntil: string; phase: AdministrationOperationState; }
