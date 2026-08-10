import { Badge } from "@/components/ui/badge";
import { formatRelativeTime } from "@/lib/formatting/relativeTime";
import type { ServiceHealthRecord } from "@/types/snapshot";

const labels = { unknown: "Unknown", healthy: "Healthy", degraded: "Degraded", unavailable: "Unavailable" } as const;
const variants = { unknown: "secondary", healthy: "default", degraded: "secondary", unavailable: "destructive" } as const;

export function ServiceHealthBadge({ health }: { health?: ServiceHealthRecord }) {
  const state = health?.state ?? "unknown";
  return <Badge variant={variants[state]}>{labels[state]}</Badge>;
}

export function ServiceHealthDetails({ health }: { health?: ServiceHealthRecord }) {
  if (!health || health.state === "unknown") return <p className="text-xs text-muted-foreground">Not checked yet.</p>;
  return <div className="flex flex-wrap gap-x-3 gap-y-1 text-xs text-muted-foreground">
    {health.latestResponseTimeMs !== undefined ? <span>{health.latestResponseTimeMs} ms</span> : null}
    <span>Checked {formatRelativeTime(health.lastCheckedAt)}</span>
    {health.lastSuccessfulCheckAt ? <span>Last healthy {formatRelativeTime(health.lastSuccessfulCheckAt)}</span> : null}
    {health.consecutiveFailures > 0 ? <span>{health.consecutiveFailures} consecutive failures</span> : null}
    {health.latestFailureReason ? <span>Failure: {health.latestFailureReason.replace("_", " ")}</span> : null}
  </div>;
}
