export type ManagedWorkloadDeploymentState = "prepared" | "planChanged" | "preDispatchFailed" | "dispatchPrepared" | "dispatching" | "dispatchUncertain" | "deploying" | "stillRunning" | "awaitingVerification" | "revisionVerified" | "workloadRuntimeVerified" | "completed" | "verificationFailed" | "deploymentFailed";
export type ManagedWorkloadProblem = "workloadUnavailable" | "actionUntrusted" | "communicationFailed" | "outputLimitExceeded" | "protocolInvalid" | "deploymentFailed" | "verificationFailed" | "requiredServiceMissing" | "observationExpired" | "dispatchOutcomeUncertain";

export interface ManagedWorkloadOperationRef { workloadId: string; operationId: string; }
export interface ManagedWorkloadDeployment { operation: ManagedWorkloadOperationRef; state: ManagedWorkloadDeploymentState; problem?: ManagedWorkloadProblem; observationDeadline?: string; }
export interface ManagedWorkloadCard { workloadId: string; name: string; enabled: boolean; eligibleToPrepare: boolean; deployment?: ManagedWorkloadDeployment; }
export interface PreparedManagedWorkload { operation: ManagedWorkloadDeployment; reviewTargetRevision: string; changeCount: number; }
export interface ManagedWorkloadContinuation { outcome: "dispatchStarted" | "planChanged"; operation: ManagedWorkloadDeployment; }
