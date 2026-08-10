import { cn } from "@/lib/utils";
import type { ConnectivityDiagnosticReport, DeviceHealthAssessment } from "@/types/snapshot";

function healthColor(state: DeviceHealthAssessment["state"]) {
  return state === "critical" ? "text-destructive" : state === "warning" ? "text-status-warning" : state === "healthy" ? "text-status-healthy" : "text-muted-foreground";
}

export function HealthSummary({ health }: { health: DeviceHealthAssessment }) {
  return <div className={cn("text-xs font-semibold", healthColor(health.state))} aria-label={`Health: ${health.state}`}>Health: {health.state}{health.reasons[0] ? ` - ${health.reasons[0].summary}` : ""}</div>;
}

export function HealthDetails({ health }: { health: DeviceHealthAssessment }) {
  return <section className="rounded-lg border border-border bg-card p-3.5"><h2 className="mb-2.5 text-xs font-bold tracking-wide text-muted-foreground">DEVICE HEALTH</h2><div className={cn("text-sm font-semibold", healthColor(health.state))} aria-live="polite">{health.state}</div><ul className="mt-2 space-y-1 text-xs text-muted-foreground">{health.reasons.map((reason) => <li key={reason.code}>{reason.summary}</li>)}</ul></section>;
}

function PowerFlag({ label, current, historical }: { label: string; current?: boolean; historical?: boolean }) {
  const value = (flag: boolean | undefined) => flag === undefined ? "Unavailable" : flag ? "Detected" : "Clear";
  return <div><span className="font-semibold text-foreground">{label}</span><div>Current: {value(current)} · Since boot: {value(historical)}</div></div>;
}

export function PowerThrottling({ health }: { health: DeviceHealthAssessment }) {
  const power = health.power;
  return <section className="rounded-lg border border-border bg-card p-3.5"><h2 className="mb-2.5 text-xs font-bold tracking-wide text-muted-foreground">POWER &amp; THROTTLING</h2><div className="grid grid-cols-1 gap-x-5 gap-y-2 text-xs text-muted-foreground sm:grid-cols-2"><PowerFlag label="Undervoltage" current={power.undervoltageNow} historical={power.undervoltageSinceBoot} /><PowerFlag label="Frequency capping" current={power.frequencyCappedNow} historical={power.frequencyCappedSinceBoot} /><PowerFlag label="Throttling" current={power.throttledNow} historical={power.throttledSinceBoot} /><PowerFlag label="Soft temperature limit" current={power.softTemperatureLimitNow} historical={power.softTemperatureLimitSinceBoot} /></div><p className="mt-2 font-mono text-[11px] text-muted-foreground">Raw: {power.raw ?? "Unavailable"}</p></section>;
}

export function DiagnosticsList({ diagnostics }: { diagnostics: ConnectivityDiagnosticReport }) {
  return <section className="rounded-lg border border-border bg-card p-3.5"><h2 className="mb-2.5 text-xs font-bold tracking-wide text-muted-foreground">CONNECTIVITY DIAGNOSTICS</h2><div className="space-y-2 text-sm">{diagnostics.checks.map((check) => <div key={check.code} className="flex justify-between gap-4"><span>{check.summary}</span><span aria-label={`${check.code}: ${check.status}`} className="font-semibold text-muted-foreground">{check.status}</span></div>)}</div></section>;
}
