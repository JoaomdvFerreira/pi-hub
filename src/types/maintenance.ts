export type MaintenanceOperationState =
  | "requested"
  | "preflight"
  | "refreshingMetadata"
  | "verifyingPlan"
  | "preDispatchFailed"
  | "planChanged"
  | "dispatching"
  | "installing"
  | "verifying"
  | "completed"
  | "completedRebootRequired"
  | "packageManagerBusy"
  | "failed"
  | "stillRunning"
  | "outcomeUncertain";

export type MaintenanceDispatchState = "notAttempted" | "accepted" | "uncertain";
export type MaintenanceFailure =
  | "unsupported"
  | "privilegeUnavailable"
  | "packageManagerBusy"
  | "packageStateInconsistent"
  | "metadataRefreshFailed"
  | "planChanged"
  | "dispatchFailed"
  | "packageOperationFailed"
  | "transportUnavailableDuringObservation"
  | "verificationFailed"
  | "stillRunning"
  | "outcomeUncertain";

export interface MaintenanceOperation {
  schemaVersion: number;
  id: string;
  deviceId: string;
  transientUnitId: string;
  reviewedPlan?: { version: number; digest: string };
  packageCount?: number;
  requestedAt: string;
  startedAt?: string;
  observationDeadline?: string;
  state: MaintenanceOperationState;
  dispatchState: MaintenanceDispatchState;
  completedAt?: string;
  failure?: MaintenanceFailure;
  rebootRequired?: boolean;
  preDispatchFailureStage?: "capability" | "dpkgAudit" | "planVerification";
}
