import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { checkForUpdates, getUpdateResult } from "@/lib/tauri/monitoring";
import type { UpdateCheckResult } from "@/types/snapshot";

export function SoftwareUpdates({ deviceId }: { deviceId: string }) {
  const [result, setResult] = useState<UpdateCheckResult | null | undefined>();
  const [checking, setChecking] = useState(false);
  useEffect(() => { let active = true; getUpdateResult(deviceId).then(x => active && setResult(x)).catch(() => active && setResult(null)); return () => { active = false; }; }, [deviceId]);
  const check = async () => { if (checking) return; setChecking(true); try { setResult(await checkForUpdates(deviceId)); } finally { setChecking(false); } };
  const labels: Record<string, string> = { notChecked: "Not checked", checking: "Checking", upToDate: "Up to date", updatesAvailable: "Updates available", stale: "Stale metadata", unsupported: "Unsupported", checkFailed: "Check failed", unknown: "Unknown" };
  return <section className="rounded-lg border border-border bg-card p-3.5"><div className="flex items-center justify-between gap-3"><div><h2 className="text-xs font-bold text-muted-foreground">SOFTWARE UPDATES</h2><p className="mt-1 text-sm font-medium">{result ? labels[result.status] : "Not checked"}</p></div><Button size="sm" disabled={checking} onClick={() => void check()}>Check for Updates</Button></div>{result === undefined ? <p className="mt-2 text-xs text-muted-foreground">Loading…</p> : result ? <div className="mt-3 grid grid-cols-2 gap-2 text-xs text-muted-foreground"><span>Updates: {result.updates.totalCount ?? "Unknown"}</span><span>Metadata: {result.packageMetadata.status}</span><span>Held: {result.heldPackages.totalCount ?? "Unknown"}</span><span>Reboot: {result.reboot}</span>{result.updates.packages.length > 0 ? <span className="col-span-2">Packages: {result.updates.packages.map(x => x.name).join(", ")}{result.updates.truncated ? " (truncated)" : ""}</span> : null}</div> : <p className="mt-2 text-xs text-muted-foreground">No update check has been run.</p>}</section>;
}
