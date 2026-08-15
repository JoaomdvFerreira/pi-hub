import { useCallback, useEffect, useState } from "react";
import { Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { AlertDialog, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from "@/components/ui/alert-dialog";
import { continueManagedWorkloadDeployment, listManagedWorkloadDeployments, prepareManagedWorkloadDeployment, reconcileManagedWorkloadDeployment } from "@/lib/tauri/managedWorkloads";
import type { ManagedWorkloadBlockedReason, ManagedWorkloadCard, ManagedWorkloadDeployment, PreparedManagedWorkload } from "@/types/managedWorkloads";

const ACTIVE = new Set(["dispatchPrepared", "dispatching", "dispatchUncertain", "deploying", "stillRunning", "awaitingVerification", "revisionVerified", "workloadRuntimeVerified"]);
const CHECKABLE = new Set(["dispatchPrepared", "dispatching", "dispatchUncertain", "deploying", "stillRunning", "awaitingVerification", "revisionVerified", "workloadRuntimeVerified"]);
const AUTO_OBSERVABLE = new Set(["dispatching", "dispatchUncertain", "deploying", "awaitingVerification", "revisionVerified", "workloadRuntimeVerified"]);
const OBSERVATION_INTERVAL_MS = 5_000;
function apiMessage(error: unknown, fallback: string) { const item = error as { message?: string; remediation?: string } | null; return item?.remediation ? `${item.message ?? fallback} ${item.remediation}` : item?.message ?? fallback; }
function observationOpen(deadline?: string) { const parsed = deadline ? new Date(deadline).getTime() : Number.NaN; return Number.isFinite(parsed) && parsed > Date.now(); }
function compactRevision(revision: string) { return revision.length > 16 ? `${revision.slice(0, 12)}…${revision.slice(-4)}` : revision; }
function blockedMessage(name: string, reason: ManagedWorkloadBlockedReason) {
  switch (reason) {
    case "dirtyWorktree": return `${name} has uncommitted changes on the device. Resolve them, then prepare again. No deployment was started.`;
    case "upstreamPolicy": return `${name}'s repository is not configured to track the expected upstream. Fix the repository configuration, then prepare again. No deployment was started.`;
    case "divergedHistory": return `${name}'s local history has diverged from upstream. Resolve this on the device, then prepare again. No deployment was started.`;
    case "changeCountExceeded": return `${name} has too many pending changes to prepare safely. Reduce the change set, then prepare again. No deployment was started.`;
    case "repositoryUnavailable": return `${name}'s repository is unavailable on the device. Restore it, then prepare again. No deployment was started.`;
  }
}
function lifecycleText(deployment?: ManagedWorkloadDeployment) {
  switch (deployment?.state) {
    case "dispatchUncertain": return "Deployment start could not be confirmed. Do not retry the update; check status.";
    case "dispatchPrepared": case "dispatching": return "Starting deployment…";
    case "deploying": return "Deployment in progress.";
    case "stillRunning": return "Deployment is still running. Automatic observation has ended; use Check status.";
    case "awaitingVerification": case "revisionVerified": case "workloadRuntimeVerified": return "Verifying deployment…";
    case "preDispatchFailed": return "Deployment did not start. You can prepare a fresh plan.";
    case "deploymentFailed": return "Deployment failed. Review the workload, then prepare a fresh plan when ready.";
    case "verificationFailed": return "Deployment finished but verification failed. Review the workload, then prepare a fresh plan when ready.";
    case "completed": return "Deployment complete.";
    default: return null;
  }
}

export function ManagedWorkloadDeployments({ deviceId, maintenanceInProgress = false }: { deviceId: string; maintenanceInProgress?: boolean }) {
  const [workloads, setWorkloads] = useState<ManagedWorkloadCard[] | null>(null);
  const load = useCallback(async () => { try { setWorkloads(await listManagedWorkloadDeployments(deviceId)); } catch { setWorkloads([]); } }, [deviceId]);
  useEffect(() => { let mounted = true; void listManagedWorkloadDeployments(deviceId).then(items => { if (mounted) setWorkloads(items); }).catch(() => { if (mounted) setWorkloads([]); }); return () => { mounted = false; }; }, [deviceId]);
  if (!workloads?.length) return null;
  return <section className="rounded-lg border border-border bg-card p-3.5"><h2 className="text-xs font-bold text-muted-foreground">MANAGED WORKLOADS</h2><div className="mt-3 grid gap-3">{workloads.map(workload => <ManagedWorkloadCardView key={`${workload.workloadId}:${workload.deployment?.operation.operationId ?? "new"}:${workload.deployment?.state ?? "new"}`} workload={workload} maintenanceInProgress={maintenanceInProgress} onChange={load}/>)}</div></section>;
}

function ManagedWorkloadCardView({ workload, maintenanceInProgress, onChange }: { workload: ManagedWorkloadCard; maintenanceInProgress: boolean; onChange: () => Promise<void> }) {
  const [prepared, setPrepared] = useState<PreparedManagedWorkload | null>(null); const [preparing, setPreparing] = useState(false); const [continuing, setContinuing] = useState(false); const [checkingStatus, setCheckingStatus] = useState(false); const [notice, setNotice] = useState<string | null>(null); const [observed, setObserved] = useState<ManagedWorkloadDeployment | undefined>(workload.deployment); const [observationTick, setObservationTick] = useState(0);
  const deployment = observed; const active = ACTIVE.has(deployment?.state ?? "");
  const prepare = async () => {
    if (preparing || continuing) return;
    setPreparing(true); setNotice(null);
    try {
      const response = await prepareManagedWorkloadDeployment(workload.workloadId);
      if (response.outcome === "prepared") setPrepared(response);
      else if (response.outcome === "upToDate") setNotice(`${workload.name} is up to date.`);
      else setNotice(blockedMessage(workload.name, response.reason));
    } catch (error) {
      setNotice(`${apiMessage(error, "Pi-Hub could not prepare this deployment safely.")} No deployment was started.`);
    } finally { setPreparing(false); }
  };
  const confirm = async () => { if (!prepared || continuing) return; setContinuing(true); setNotice(null); try { const response = await continueManagedWorkloadDeployment({ workloadId: prepared.operation.operation.workloadId, operationId: prepared.operation.operation.operationId }); setPrepared(null); if (response.outcome === "planChanged") setNotice("The deployment plan changed before starting. No deployment was started. Prepare and review the new plan."); else if (response.operation.state === "dispatchUncertain") setNotice("Deployment start could not be confirmed. Do not retry; status must be checked/reconciled."); else setNotice("Deployment in progress."); await onChange(); } catch (error) { setNotice(`${apiMessage(error, "The deployment did not start.")} The deployment did not start.`); } finally { setContinuing(false); } };
  const checkStatus = useCallback(async () => { if (checkingStatus || !deployment) return; setCheckingStatus(true); setNotice(null); try { setObserved(await reconcileManagedWorkloadDeployment(workload.workloadId)); await onChange(); } catch (error) { setNotice(`${apiMessage(error, "Pi-Hub could not check deployment status.")} The last known deployment state is still shown.`); } finally { setCheckingStatus(false); setObservationTick(tick => tick + 1); } }, [checkingStatus, deployment, onChange, workload.workloadId]);
  const autoObservable = AUTO_OBSERVABLE.has(deployment?.state ?? "") && observationOpen(deployment?.observationDeadline);
  useEffect(() => { if (!autoObservable || checkingStatus) return; const timer = setTimeout(() => { void checkStatus(); }, OBSERVATION_INTERVAL_MS); return () => clearTimeout(timer); }, [autoObservable, checkingStatus, checkStatus, observationTick]);
  const eligible = workload.enabled && workload.eligibleToPrepare && !maintenanceInProgress && !active && !preparing && !continuing;
  return <div className="rounded-md border border-border p-3"><div className="flex flex-wrap items-center gap-2"><h3 className="font-medium">{workload.name}</h3><div className="flex-1"/>{CHECKABLE.has(deployment?.state ?? "") ? <Button size="sm" variant="outline" disabled={checkingStatus || preparing || continuing} onClick={checkStatus}>{checkingStatus ? <><Loader2 className="animate-spin"/>Checking status…</> : "Check status"}</Button> : null}<Button size="sm" disabled={!eligible} onClick={prepare}>{preparing ? <><Loader2 className="animate-spin"/>Preparing…</> : "Prepare update"}</Button></div>{!workload.enabled ? <p className="mt-2 text-xs text-muted-foreground">This managed workload is disabled.</p> : null}{lifecycleText(deployment) ? <p className="mt-2 text-xs text-muted-foreground" role="status">{lifecycleText(deployment)}</p> : null}{notice ? <p className="mt-2 text-xs text-muted-foreground" role="status">{notice}</p> : null}
    <AlertDialog open={Boolean(prepared)} onOpenChange={open => { if (!open && !continuing) setPrepared(null); }}><AlertDialogContent><AlertDialogHeader><AlertDialogTitle>Review update for {workload.name}</AlertDialogTitle><AlertDialogDescription>This will update the workload to revision <span className="font-mono">{prepared ? compactRevision(prepared.reviewTargetRevision) : ""}</span>{prepared?.changeCount ? ` (${prepared.changeCount} planned changes)` : ""}. Pi-Hub will revalidate this plan before starting. If it changed, you will need to review a fresh plan.</AlertDialogDescription></AlertDialogHeader><AlertDialogFooter><AlertDialogCancel disabled={continuing}>Cancel</AlertDialogCancel><Button disabled={continuing} onClick={confirm}>{continuing ? <><Loader2 className="animate-spin"/>Starting deployment…</> : "Confirm and start update"}</Button></AlertDialogFooter></AlertDialogContent></AlertDialog>
  </div>;
}
