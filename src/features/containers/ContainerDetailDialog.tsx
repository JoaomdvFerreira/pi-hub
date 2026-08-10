import { useState, type ReactNode } from "react";
import { Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { AlertDialog, AlertDialogContent, AlertDialogDescription, AlertDialogHeader, AlertDialogTitle } from "@/components/ui/alert-dialog";
import { getContainerLogs } from "@/lib/tauri/containers";
import type { ContainerLogMode, DockerContainerSummary } from "@/types/snapshot";

interface ContainerDetailDialogProps { deviceId: string; container: DockerContainerSummary | null; onOpenChange: (open: boolean) => void; services: { id: string; name: string; containerName?: string }[]; }

const LOG_MODES: { value: ContainerLogMode; label: string }[] = [
  { value: "last100", label: "Last 100 lines" }, { value: "last500", label: "Last 500 lines" },
  { value: "last15Minutes", label: "Last 15 minutes" }, { value: "last1Hour", label: "Last hour" },
];

export function ContainerDetailDialog({ deviceId, container, onOpenChange, services }: ContainerDetailDialogProps) {
  const [mode, setMode] = useState<ContainerLogMode>("last100");
  const [logs, setLogs] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  if (!container) return null;
  const current = container;
  const associated = services.filter((service) => service.containerName === container.name);
  async function loadLogs() { setLoading(true); setError(null); try { const result = await getContainerLogs(deviceId, current.id, mode); setLogs(result.content + (result.truncated ? "\n\n[Output truncated at 512 KiB]" : "")); } catch { setError("Recent logs could not be collected."); } finally { setLoading(false); } }
  return <AlertDialog open={Boolean(container)} onOpenChange={onOpenChange}>
    <AlertDialogContent className="max-h-[90vh] max-w-3xl overflow-y-auto">
      <AlertDialogHeader><AlertDialogTitle>{container.name}</AlertDialogTitle><AlertDialogDescription>Current operational Docker information. Container environment variables are not collected or displayed.</AlertDialogDescription></AlertDialogHeader>
      <div className="grid grid-cols-2 gap-3 text-sm"><Field label="State" value={container.state}/><Field label="Health" value={container.health}/><Field label="Image" value={container.image}/><Field label="Container ID" value={container.id}/><Field label="Created" value={container.createdAt ?? "Unavailable"}/><Field label="Started" value={container.startedAt ?? "Unavailable"}/><Field label="Restarts" value={container.restartCount?.toString() ?? "Unavailable"}/><Field label="Restart policy" value={container.restartPolicy ?? "Unavailable"}/><Field label="CPU" value={container.resourceUsage?.cpuPercent !== undefined ? `${container.resourceUsage.cpuPercent.toFixed(1)}%` : "Unavailable"}/><Field label="Memory" value={container.resourceUsage?.memoryUsedBytes !== undefined ? `${Math.round(container.resourceUsage.memoryUsedBytes / 1048576)} MiB` : "Unavailable"}/></div>
      <Section title="Ports">{container.ports.length ? container.ports.map((port) => <p key={`${port.hostIp}-${port.hostPort}-${port.containerPort}`} className="font-mono text-xs">{port.hostIp ?? ""}{port.hostPort !== undefined ? `:${port.hostPort} → ` : ""}{port.containerPort}/{port.protocol}</p>) : <Empty/>}</Section>
      <Section title="Storage">{container.mounts.length ? container.mounts.map((mount) => <p key={`${mount.source}-${mount.destination}`} className="font-mono text-xs">{mount.mountType}: {mount.source} → {mount.destination} ({mount.readOnly ? "read-only" : "read-write"})</p>) : <Empty/>}</Section>
      <Section title="Networks">{container.networks.length ? container.networks.map((network) => <p key={network.name} className="text-xs">{network.name}{network.ipAddress ? ` — ${network.ipAddress}` : ""}{network.aliases.length ? ` (${network.aliases.join(", ")})` : ""}</p>) : <Empty/>}</Section>
      <Section title="Labels">{container.labels.length ? container.labels.map((label) => <p key={label.key} className="break-all font-mono text-xs">{label.key}={label.value}</p>) : <Empty/>}</Section>
      <Section title="Associated Pi-Hub services">{associated.length ? associated.map((service) => <p key={service.id} className="text-xs">{service.name}</p>) : <Empty/>}</Section>
      <Section title="Recent logs"><div className="mb-2 flex flex-wrap gap-2">{LOG_MODES.map((item) => <Button key={item.value} variant={mode === item.value ? "default" : "outline"} size="sm" onClick={() => setMode(item.value)}>{item.label}</Button>)}<Button size="sm" disabled={loading} onClick={loadLogs}>{loading ? <Loader2 className="animate-spin"/> : null}Load logs</Button></div>{error ? <p className="text-sm text-destructive">{error}</p> : null}{logs ? <pre className="max-h-72 overflow-auto rounded bg-muted p-3 text-xs whitespace-pre-wrap">{logs}</pre> : <p className="text-xs text-muted-foreground">Logs are retrieved on demand and are not persisted.</p>}</Section>
    </AlertDialogContent>
  </AlertDialog>;
}
function Field({ label, value }: { label: string; value: string }) { return <div><div className="text-xs text-muted-foreground">{label}</div><div className="break-all text-xs text-foreground">{value}</div></div>; }
function Section({ title, children }: { title: string; children: ReactNode }) { return <section><h3 className="mb-1 text-xs font-bold tracking-wide text-muted-foreground">{title.toUpperCase()}</h3>{children}</section>; }
function Empty() { return <p className="text-xs text-muted-foreground">None available.</p>; }
