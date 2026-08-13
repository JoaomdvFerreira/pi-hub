import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { checkForUpdates, getUpdateResult } from "@/lib/tauri/monitoring";
import type { UpdateCheckResult } from "@/types/snapshot";

function stateLabel(result: UpdateCheckResult) {
  switch (result.status) {
    case "upToDate": return "System is up to date";
    case "updatesAvailable": return result.updates.totalCount ? `${result.updates.totalCount} updates available` : "System updates are available";
    case "stale": return result.updates.totalCount ? `${result.updates.totalCount} updates available` : "Update information may be out of date";
    case "unsupported": return "System updates are unavailable on this device";
    case "checkFailed": return "Could not check for system updates";
    case "unknown": return "System update status is unknown";
    default: return "System updates have not been checked";
  }
}

function lastChecked(value?: string) {
  if (!value) return "Not checked yet";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "Not checked yet" : date.toLocaleString();
}

export function SoftwareUpdates({ deviceId }: { deviceId: string }) {
  const [result, setResult] = useState<UpdateCheckResult | null | undefined>();
  const [checking, setChecking] = useState(false);
  const [showUpdates, setShowUpdates] = useState(false);

  useEffect(() => {
    let active = true;
    getUpdateResult(deviceId).then(x => active && setResult(x)).catch(() => active && setResult(null));
    return () => { active = false; };
  }, [deviceId]);

  const check = async () => {
    if (checking) return;
    setChecking(true);
    try { setResult(await checkForUpdates(deviceId)); setShowUpdates(false); }
    finally { setChecking(false); }
  };

  const updateCount = result?.updates.totalCount ?? 0;
  const deferredCount = result?.keptBackPackages.totalCount ?? 0;
  return <section className="rounded-lg border border-border bg-card p-3.5">
    <div className="flex flex-wrap items-start justify-between gap-3">
      <div>
        <h2 className="text-xs font-bold text-muted-foreground">SYSTEM UPDATES</h2>
        <p className="mt-1 text-sm font-medium" aria-live="polite">{result ? stateLabel(result) : "System updates have not been checked"}</p>
        {deferredCount > 0 ? <p className="mt-1 text-xs text-muted-foreground">{deferredCount} {deferredCount === 1 ? "package" : "packages"} deferred</p> : null}
        <p className="mt-1 text-xs text-muted-foreground">Last checked: {lastChecked(result?.checkedAt)}</p>
      </div>
      <div className="flex flex-wrap gap-2">
        {updateCount > 0 ? <Button size="sm" variant="outline" onClick={() => setShowUpdates(value => !value)} aria-expanded={showUpdates} aria-controls="system-update-list">{showUpdates ? "Hide updates" : "View updates"}</Button> : null}
        <Button size="sm" disabled={checking} onClick={() => void check()}>{checking ? "Checking…" : result ? "Check again" : "Check for Updates"}</Button>
      </div>
    </div>
    {result === undefined ? <p className="mt-3 text-xs text-muted-foreground">Loading system update status…</p> : null}
    {showUpdates && result ? <div id="system-update-list" className="mt-3 overflow-x-auto">
      <table className="w-full min-w-[480px] text-left text-sm">
        <caption className="sr-only">Available system updates</caption>
        <thead className="text-xs text-muted-foreground"><tr><th className="pb-2 pr-3">Package</th><th className="pb-2 pr-3">Installed version</th><th className="pb-2">Candidate version</th></tr></thead>
        <tbody>{result.updates.packages.map(item => <tr key={`${item.name}-${item.candidateVersion}`} className="border-t border-border"><td className="py-2 pr-3 font-medium">{item.name}</td><td className="py-2 pr-3">{item.installedVersion}</td><td className="py-2">{item.candidateVersion}</td></tr>)}</tbody>
      </table>
    </div> : null}
  </section>;
}
