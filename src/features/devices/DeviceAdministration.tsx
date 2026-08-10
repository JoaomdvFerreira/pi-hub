import { useCallback, useEffect, useState } from "react";
import { AlertTriangle, Loader2, Power, RotateCw, Server, Wifi } from "lucide-react";
import { Button } from "@/components/ui/button";
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle, AlertDialogTrigger } from "@/components/ui/alert-dialog";
import { getExpectedDisruption, performAdministrationOperation } from "@/lib/tauri/administration";
import type { AdministrationOperation, AdministrationOperationState, AdministrationOperationType, ExpectedDisruption } from "@/types/administration";
import type { ApplicationError } from "@/types/settings";

export const ADMINISTRATION_ACTIONS: { type: AdministrationOperationType; label: string; confirmation: string; danger?: boolean; Icon: typeof RotateCw }[] = [
  { type: "restartDocker", label: "Restart Docker", confirmation: "Running containers and hosted services may stop or restart temporarily. Pi-Hub will verify Docker availability afterwards.", Icon: Server },
  { type: "restartTailscale", label: "Restart Tailscale", confirmation: "Tailscale connectivity may temporarily drop. If this device is reached through Tailscale, SSH may disconnect while Pi-Hub attempts bounded recovery verification.", Icon: Wifi },
  { type: "restartDevice", label: "Restart Device", confirmation: "The device will restart. Containers and services will temporarily stop, terminal and SSH sessions may disconnect, and Pi-Hub will wait for it to return online.", danger: true, Icon: RotateCw },
  { type: "shutdownDevice", label: "Shut Down Device", confirmation: "This will stop all services and containers and disconnect active SSH terminal sessions. Pi-Hub cannot turn this device back on; power-on must be manual or provided by infrastructure outside Pi-Hub.", danger: true, Icon: Power },
];
const stateLabel: Record<AdministrationOperationState, string> = { requested: "Requested", dispatching: "Dispatching", commandAccepted: "Command accepted", verifying: "Verifying", waitingForOffline: "Waiting for offline", waitingForOnline: "Waiting for online recovery", completed: "Completed", failed: "Failed", timedOut: "Timed out", outcomeUncertain: "Outcome uncertain" };
function isApplicationError(error: unknown): error is ApplicationError { return typeof error === "object" && error !== null && "message" in error; }

export function DeviceAdministration({ deviceId, deviceName }: { deviceId: string; deviceName: string }) {
  const [running, setRunning] = useState<AdministrationOperationType | null>(null);
  const [result, setResult] = useState<AdministrationOperation | null>(null);
  const [expected, setExpected] = useState<ExpectedDisruption | null>(null);
  const [error, setError] = useState<string | null>(null);
  const loadExpected = useCallback(() => { getExpectedDisruption(deviceId).then(setExpected).catch(() => setExpected(null)); }, [deviceId]);
  useEffect(() => { loadExpected(); }, [loadExpected]);
  async function execute(type: AdministrationOperationType) { setRunning(type); setResult(null); setError(null); try { setResult(await performAdministrationOperation(deviceId, type)); } catch (err) { setError(isApplicationError(err) ? err.message : "The administration operation could not be started."); } finally { setRunning(null); loadExpected(); } }
  return <section className="rounded-lg border border-border bg-card p-3.5" aria-label="Device administration">
    <h2 className="mb-1 text-xs font-bold tracking-wide text-muted-foreground">DEVICE ADMINISTRATION</h2>
    <p className="mb-3 text-xs text-muted-foreground">Only the four deliberate operations below are supported. Pi-Hub does not provide Power On or arbitrary remote commands.</p>
    {expected?.operationType === "shutdownDevice" ? <p className="mb-3 rounded-md border border-status-warning/40 bg-status-warning/10 p-2 text-xs text-foreground"><AlertTriangle className="mr-1 inline size-3.5 text-status-warning" /> Offline — planned after controlled shutdown until {new Date(expected.expectedUntil).toLocaleString()}.</p> : null}
    <div className="grid gap-2 sm:grid-cols-2">{ADMINISTRATION_ACTIONS.map(({ type, label, confirmation, danger, Icon }) => <AlertDialog key={type}><AlertDialogTrigger asChild><Button variant={danger ? "destructive" : "outline"} size="sm" className="justify-start" disabled={running !== null || expected?.operationType === "shutdownDevice"}><Icon className="size-3.5" />{running === type ? <Loader2 className="ml-auto size-3.5 animate-spin" /> : null}{label}</Button></AlertDialogTrigger><AlertDialogContent><AlertDialogHeader><AlertDialogTitle>{label} “{deviceName}”?</AlertDialogTitle><AlertDialogDescription>{confirmation}</AlertDialogDescription></AlertDialogHeader><AlertDialogFooter><AlertDialogCancel>Cancel</AlertDialogCancel><AlertDialogAction variant={danger ? "destructive" : "default"} onClick={() => execute(type)}>{label}</AlertDialogAction></AlertDialogFooter></AlertDialogContent></AlertDialog>)}</div>
    {running ? <p className="mt-3 text-xs text-muted-foreground" role="status">{stateLabel.dispatching}: {ADMINISTRATION_ACTIONS.find((action) => action.type === running)?.label}</p> : null}
    {result ? <p className={"mt-3 text-xs " + (result.state === "completed" ? "text-status-healthy" : "text-destructive")} role="status">{stateLabel[result.state]}: {result.detail ?? `${ADMINISTRATION_ACTIONS.find((action) => action.type === result.operationType)?.label} ${result.state === "completed" ? "was verified." : "requires attention."}`}</p> : null}
    {error ? <p className="mt-3 text-xs text-destructive" role="alert">{error}</p> : null}
  </section>;
}
