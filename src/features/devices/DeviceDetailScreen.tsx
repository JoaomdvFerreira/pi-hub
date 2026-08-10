import { useCallback, useEffect, useMemo, useState } from "react";
import { Activity, ArrowLeft, ExternalLink, Loader2, RefreshCw, Settings, TerminalSquare } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { useRouter } from "@/app/router";
import { diagnoseDeviceConnection, getDevice, openDeviceService, openDeviceTerminal } from "@/lib/tauri/devices";
import type { ConnectivityDiagnosticReport } from "@/types/snapshot";
import { DiagnosticsList, HealthDetails, PowerThrottling } from "@/features/devices/HealthDiagnostics";
import { ServiceHealthBadge, ServiceHealthDetails } from "@/features/services/ServiceHealth";
import { refreshDevice } from "@/lib/tauri/monitoring";
import { useDeviceSnapshots } from "@/stores/useDeviceSnapshots";
import { useDeviceActivity } from "@/stores/useDeviceActivity";
import { formatUptime } from "@/lib/formatting/uptime";
import { formatRelativeTime } from "@/lib/formatting/relativeTime";
import { diskPercent, memoryPercent } from "@/lib/formatting/deviceMetrics";
import {
  clampPercent,
  levelBarClass,
  levelColorClass,
  temperatureBarClass,
  temperatureColorClass,
} from "@/lib/formatting/metricThresholds";
import { connectionStatusColorClass, connectionStatusLabel } from "@/lib/formatting/connectionStatus";
import {
  containerHealthColorClass,
  containerHealthLabel,
  containerStateColorClass,
  containerStateLabel,
  formatPorts,
} from "@/lib/formatting/containerStatus";
import { ContainerActionsCell } from "@/features/devices/ContainerActionsCell";
import { ContainerDetailDialog } from "@/features/containers/ContainerDetailDialog";
import { DeviceAdministration } from "@/features/devices/DeviceAdministration";
import { DeviceVisibility } from "@/features/devices/DeviceVisibility";
import { HistoricalTrends } from "@/features/monitoring/HistoricalTrends";
import { useTerminalSessions } from "@/stores/useTerminalSessions";
import type { Device } from "@/types/device";
import type { ApplicationError } from "@/types/settings";

function isApplicationError(err: unknown): err is ApplicationError {
  return typeof err === "object" && err !== null && "code" in err && "message" in err;
}

interface DeviceDetailScreenProps {
  deviceId: string;
  initialContainerId?: string;
}

interface MetricStatProps {
  label: string;
  value: number | undefined;
  unit: string;
  colorClass: string;
  barClass: string;
}

function MetricStat({ label, value, unit, colorClass, barClass }: MetricStatProps) {
  return (
    <div>
      <div className="text-[11px] font-bold text-muted-foreground">{label}</div>
      <div className={cn("mt-0.5 text-[22px] font-semibold", colorClass)}>
        {value === undefined ? "—" : `${Math.round(value)}${unit}`}
      </div>
      <div className="mt-2 h-[5px] rounded-full bg-muted">
        <div
          className={cn("h-[5px] rounded-full", barClass)}
          style={{ width: value === undefined ? "0%" : `${clampPercent(value)}%` }}
        />
      </div>
    </div>
  );
}

export function DeviceDetailScreen({ deviceId, initialContainerId }: DeviceDetailScreenProps) {
  const { goDashboard, goDeviceSettings } = useRouter();
  const [device, setDevice] = useState<Device | null | undefined>(undefined);
  const [refreshing, setRefreshing] = useState(false);
  const [openingServiceId, setOpeningServiceId] = useState<string | null>(null);
  const [serviceOpenError, setServiceOpenError] = useState<string | null>(null);
  const { openTerminal } = useTerminalSessions();
  const [openingExternalTerminal, setOpeningExternalTerminal] = useState(false);
  const [terminalError, setTerminalError] = useState<string | null>(null);
  const [diagnostics, setDiagnostics] = useState<ConnectivityDiagnosticReport | null>(null);
  const [diagnosing, setDiagnosing] = useState(false);
  const [selectedContainerId, setSelectedContainerId] = useState<string | null>(initialContainerId ?? null);

  const deviceIds = useMemo(() => [deviceId], [deviceId]);
  const snapshots = useDeviceSnapshots(deviceIds);
  const snapshot = snapshots[deviceId];
  const activity = useDeviceActivity(deviceId);

  const load = useCallback(async () => {
    try {
      const result = await getDevice(deviceId);
      setDevice(result);
    } catch {
      setDevice(null);
    }
  }, [deviceId]);

  useEffect(() => {
    // Fetch-on-mount: no data-fetching library is in scope for the MVP yet.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    load();
  }, [load]);

  async function handleOpenService(serviceId: string) {
    setOpeningServiceId(serviceId);
    setServiceOpenError(null);
    try {
      await openDeviceService(deviceId, serviceId);
    } catch {
      setServiceOpenError("Could not open this service. Check that a default browser is configured.");
    } finally {
      setOpeningServiceId(null);
    }
  }

  async function handleOpenExternalTerminal() {
    setOpeningExternalTerminal(true);
    setTerminalError(null);
    try {
      await openDeviceTerminal(deviceId);
    } catch (err) {
      setTerminalError(
        isApplicationError(err) ? err.message : "Could not open a terminal for this device.",
      );
    } finally {
      setOpeningExternalTerminal(false);
    }
  }

  async function handleRefresh() {
    setRefreshing(true);
    try {
      await refreshDevice(deviceId);
    } catch {
      // Reflected in the snapshot's connectionStatus/error, rendered below.
    } finally {
      setRefreshing(false);
    }
  }

  async function handleDiagnostics() {
    if (!device) return;
    setDiagnosing(true);
    try {
      setDiagnostics(await diagnoseDeviceConnection({ host: device.host, sshPort: device.sshPort, sshUsername: device.sshUsername }));
    } finally {
      setDiagnosing(false);
    }
  }

  if (device === undefined) {
    return (
      <div className="flex items-center justify-center gap-2 rounded-lg border border-dashed border-border py-16 text-sm text-muted-foreground">
        <Loader2 className="animate-spin" /> Loading…
      </div>
    );
  }

  if (device === null) {
    return (
      <div className="flex flex-col gap-4">
        <BackButton onClick={goDashboard} />
        <div className="rounded-lg border border-border bg-card p-6 text-sm text-muted-foreground">
          This device could not be found. It may have been deleted.
        </div>
      </div>
    );
  }

  const hasEverRefreshed = snapshot !== undefined;
  const isOnline = snapshot?.connectionStatus === "online";
  const metrics = snapshot?.metrics;
  const mem = metrics ? memoryPercent(metrics) : undefined;
  const disk = metrics ? diskPercent(metrics) : undefined;
  const containers = snapshot?.containers ?? [];

  return (
    <div className="flex flex-col gap-3.5">
      <div className="flex items-center gap-2.5">
        <BackButton onClick={goDashboard} />
        <span
          className={cn(
            "size-2.5 shrink-0 rounded-full",
            !hasEverRefreshed
              ? "bg-muted-foreground"
              : isOnline
                ? "bg-status-healthy animate-pulse"
                : "bg-status-offline",
          )}
        />
        <h1 className="text-lg font-semibold text-foreground">{device.name}</h1>
        {hasEverRefreshed ? (
          <span
            className={cn(
              "rounded-full bg-white/[0.06] px-2.5 py-0.5 text-[11px] font-semibold",
              connectionStatusColorClass(snapshot.connectionStatus),
            )}
          >
            {connectionStatusLabel(snapshot.connectionStatus)}
          </span>
        ) : null}
        <div className="flex-1" />
        <Button
          title="Open an in-app SSH terminal"
          onClick={() => openTerminal(device.id, device.name)}
        >
          <TerminalSquare />
          Open Terminal
        </Button>
        <Button
          variant="outline"
          size="icon"
          className="size-[30px]"
          disabled={openingExternalTerminal}
          title="Open in an external terminal (Windows Terminal/PowerShell)"
          onClick={handleOpenExternalTerminal}
        >
          {openingExternalTerminal ? <Loader2 className="animate-spin" /> : <ExternalLink />}
        </Button>
        <Button
          variant="outline"
          size="icon"
          className="size-[30px]"
          title="Refresh"
          disabled={refreshing}
          onClick={handleRefresh}
        >
          {refreshing ? <Loader2 className="animate-spin" /> : <RefreshCw />}
        </Button>
        <Button variant="outline" size="icon" className="size-[30px]" title="Run connection diagnostics" disabled={diagnosing} onClick={handleDiagnostics}>
          {diagnosing ? <Loader2 className="animate-spin" /> : <Activity />}
        </Button>
        <Button
          variant="outline"
          size="icon"
          className="size-[30px]"
          title="Device settings"
          onClick={() => goDeviceSettings(deviceId)}
        >
          <Settings />
        </Button>
      </div>

      {terminalError ? <p className="text-sm text-destructive">{terminalError}</p> : null}

      <div className="grid grid-cols-1 gap-3.5 lg:grid-cols-[1.3fr_1fr]">
        <div className="flex flex-col gap-3.5">
          {snapshot ? <HealthDetails health={snapshot.health} /> : null}
          <section className="rounded-lg border border-border bg-card p-3.5">
            <h2 className="mb-2.5 text-xs font-bold tracking-wide text-muted-foreground">
              GENERAL INFORMATION
            </h2>
            <div className="grid grid-cols-2 gap-x-4 gap-y-2.5">
              <InfoField label="Hostname" value={metrics?.hostname ?? device.host} mono />
              <InfoField label="IP address / host" value={device.host} mono />
              <InfoField label="Model" value={metrics?.model ?? "—"} />
              <InfoField label="SSH user" value={device.sshUsername} />
            </div>
          </section>

          <section className="rounded-lg border border-border bg-card p-3.5">
            <h2 className="mb-3 text-xs font-bold tracking-wide text-muted-foreground">
              SYSTEM METRICS
            </h2>
            {metrics ? (
              <div className="grid grid-cols-4 gap-3.5">
                <MetricStat
                  label="CPU"
                  value={metrics.cpuUsagePercent}
                  unit="%"
                  colorClass={
                    metrics.cpuUsagePercent === undefined
                      ? "text-muted-foreground"
                      : levelColorClass(metrics.cpuUsagePercent)
                  }
                  barClass={
                    metrics.cpuUsagePercent === undefined
                      ? "bg-status-neutral"
                      : levelBarClass(metrics.cpuUsagePercent)
                  }
                />
                <MetricStat
                  label="MEMORY"
                  value={mem}
                  unit="%"
                  colorClass={mem === undefined ? "text-muted-foreground" : levelColorClass(mem)}
                  barClass={mem === undefined ? "bg-status-neutral" : levelBarClass(mem)}
                />
                <MetricStat
                  label="DISK"
                  value={disk}
                  unit="%"
                  colorClass={disk === undefined ? "text-muted-foreground" : levelColorClass(disk)}
                  barClass={disk === undefined ? "bg-status-neutral" : levelBarClass(disk)}
                />
                <MetricStat
                  label="TEMP"
                  value={metrics.temperatureCelsius}
                  unit="°C"
                  colorClass={
                    metrics.temperatureCelsius === undefined
                      ? "text-muted-foreground"
                      : temperatureColorClass(metrics.temperatureCelsius)
                  }
                  barClass={
                    metrics.temperatureCelsius === undefined
                      ? "bg-status-neutral"
                      : temperatureBarClass(metrics.temperatureCelsius)
                  }
                />
              </div>
            ) : (
              <p className="py-2 text-sm text-muted-foreground">
                {hasEverRefreshed ? "No metrics available." : "Not refreshed yet."}
              </p>
            )}
            <div className="mt-3.5 flex flex-wrap gap-4 text-xs text-muted-foreground">
              <span>
                Uptime:{" "}
                <b className="font-semibold text-foreground">
                  {metrics?.uptimeSeconds !== undefined ? formatUptime(metrics.uptimeSeconds) : "—"}
                </b>
              </span>
              <span>
                Containers:{" "}
                <b className="font-semibold text-foreground">
                  {containers.filter((c) => c.state === "running").length}/{containers.length}
                </b>
              </span>
              <span>
                Last refresh:{" "}
                <b className="font-semibold text-foreground">
                  {formatRelativeTime(snapshot?.capturedAt)}
                </b>
              </span>
            </div>
            <div className="mt-3 grid grid-cols-2 gap-2 text-xs text-muted-foreground">
              <span>Load: {metrics?.loadAverage1m ?? "-"} / {metrics?.loadAverage5m ?? "-"} / {metrics?.loadAverage15m ?? "-"}</span>
              <span>CPU frequency: {metrics?.cpuFrequencyMhz !== undefined ? `${Math.round(metrics.cpuFrequencyMhz)} MHz` : "Unavailable"}</span>
              <span>Swap: {metrics?.swapUsedBytes !== undefined && metrics?.swapTotalBytes !== undefined ? `${Math.round(metrics.swapUsedBytes / 1048576)} / ${Math.round(metrics.swapTotalBytes / 1048576)} MiB` : "Unavailable"}</span>
              <span>Root filesystem: {metrics?.rootFilesystemReadOnly === undefined ? "Unavailable" : metrics.rootFilesystemReadOnly ? "Read-only" : "Read-write"}</span>
              <span>Boot time: {metrics?.bootTimestamp !== undefined ? new Date(metrics.bootTimestamp * 1000).toLocaleString() : "Unavailable"}</span>
              <span>Reboot required: {metrics?.rebootRequired === undefined ? "Unavailable" : metrics.rebootRequired ? "Yes" : "No"}</span>
            </div>
          </section>

          {snapshot ? <PowerThrottling health={snapshot.health} /> : null}
        </div>

        <section className="flex max-h-[340px] flex-col rounded-lg border border-border bg-card p-3.5">
          <h2 className="mb-2.5 text-xs font-bold tracking-wide text-muted-foreground">
            RECENT ACTIVITY
          </h2>
          {activity.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              No activity observed yet this session.
            </p>
          ) : (
            <div className="flex flex-col gap-2.5 overflow-y-auto">
              {activity.map((entry, index) => (
                <div key={`${entry.resourceId}-${entry.receivedAt}-${index}`} className="flex gap-2">
                  <span className="mt-1.5 size-[7px] shrink-0 rounded-full bg-status-warning" />
                  <div>
                    <div className="text-[12.5px] leading-snug text-foreground/90">
                      {entry.message}
                    </div>
                    <div className="mt-0.5 text-[11px] text-muted-foreground">
                      {formatRelativeTime(entry.receivedAt)}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </section>
      </div>

      {diagnostics ? <DiagnosticsList diagnostics={diagnostics} /> : null}

      <DeviceVisibility network={snapshot?.networkVisibility} storage={snapshot?.storageVisibility} system={snapshot?.systemVisibility} />

      <HistoricalTrends deviceId={deviceId} trends={[{ label: "CPU usage", metric: "cpuUsagePercent", unit: "%" }, { label: "Memory usage", metric: "memoryUsagePercent", unit: "%" }, { label: "Root filesystem", metric: "rootFilesystemUsagePercent", unit: "%" }, { label: "Temperature", metric: "temperatureCelsius", unit: "°C" }, { label: "Device health", metric: "deviceHealth", unit: "" }]} />

      <DeviceAdministration deviceId={deviceId} deviceName={device.name} />

      <section className="rounded-lg border border-border bg-card p-3.5">
        <h2 className="mb-2.5 text-xs font-bold tracking-wide text-muted-foreground">
          DOCKER CONTAINERS
        </h2>
        {!hasEverRefreshed || !isOnline ? (
          <p className="py-6 text-center text-sm text-muted-foreground">
            Device is offline — container status unavailable.
          </p>
        ) : snapshot && !snapshot.dockerAvailable ? (
          <p className="py-6 text-center text-sm text-muted-foreground">
            Docker is not available on this device
            {snapshot.warnings.length > 0 ? `: ${snapshot.warnings.join(" ")}` : "."}
          </p>
        ) : containers.length === 0 ? (
          <p className="py-6 text-center text-sm text-muted-foreground">
            No containers found on this device.
          </p>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full min-w-[760px] border-collapse text-sm">
              <thead>
                <tr className="border-b border-border text-[11px] font-bold tracking-wide text-muted-foreground">
                  <th className="px-2 py-1.5 text-left">NAME</th>
                  <th className="px-2 py-1.5 text-left">STATUS</th>
                  <th className="px-2 py-1.5 text-left">IMAGE</th>
                  <th className="px-2 py-1.5 text-left">UPTIME</th>
                  <th className="px-2 py-1.5 text-left">HEALTH</th>
                  <th className="px-2 py-1.5 text-left">PORT</th>
                  <th className="px-2 py-1.5 text-left">ACTIONS</th>
                </tr>
              </thead>
              <tbody>
                {containers.map((c) => (
                  <tr key={c.id} className="border-b border-border/50 last:border-b-0">
                    <td className="max-w-[180px] truncate px-2 py-2 font-semibold text-foreground"><button type="button" className="hover:underline" onClick={() => setSelectedContainerId(c.id)}>{c.name}</button></td>
                    <td className={cn("px-2 py-2 font-semibold", containerStateColorClass(c.state))}>
                      <span className="flex items-center gap-1.5">
                        <span
                          className={cn(
                            "size-1.5 rounded-full",
                            containerStateColorClass(c.state).replace("text-", "bg-"),
                          )}
                        />
                        {containerStateLabel(c.state)}
                      </span>
                    </td>
                    <td className="max-w-[220px] truncate px-2 py-2 font-mono text-xs text-muted-foreground">
                      {c.image}
                    </td>
                    <td className="px-2 py-2 text-muted-foreground">
                      {c.startedAt ? formatRelativeTime(c.startedAt) : "—"}
                    </td>
                    <td className={cn("px-2 py-2 font-semibold", containerHealthColorClass(c.health))}>
                      {containerHealthLabel(c.health)}
                    </td>
                    <td className="px-2 py-2 font-mono text-xs text-muted-foreground">
                      {formatPorts(c.ports)}
                    </td>
                    <td className="px-2 py-2">
                      <ContainerActionsCell
                        deviceId={deviceId}
                        containerId={c.id}
                        containerName={c.name}
                        state={c.state}
                      />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
      <ContainerDetailDialog deviceId={deviceId} container={containers.find((container) => container.id === selectedContainerId) ?? null} onOpenChange={(open) => { if (!open) setSelectedContainerId(null); }} services={device.services} />

      <section>
        <h2 className="mb-2.5 text-xs font-bold tracking-wide text-muted-foreground">
          SERVICES ON THIS DEVICE
        </h2>
        {device.services.length === 0 ? (
          <div className="rounded-lg border border-dashed border-border bg-card px-5 py-5 text-center text-sm text-muted-foreground">
            No services registered on this device yet.
          </div>
        ) : (
          <div className="flex flex-wrap gap-2.5">
            {device.services.map((svc) => (
              <div key={svc.id} className="flex min-w-[260px] flex-col gap-2 rounded-lg border border-border bg-card px-3 py-2.5">
              <button
                type="button"
                disabled={!svc.enabled || openingServiceId === svc.id}
                title={svc.enabled ? "Open in default browser" : "This service is disabled"}
                onClick={() => handleOpenService(svc.id)}
                className="flex items-center gap-2.5 text-left transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
              >
                <span className="flex size-[34px] shrink-0 items-center justify-center rounded-md bg-primary text-sm font-bold text-primary-foreground">
                  {svc.name.slice(0, 2).toUpperCase()}
                </span>
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-[13px] font-semibold text-foreground">
                    {svc.name}
                  </span>
                  <span className="block truncate font-mono text-[11px] text-muted-foreground">
                    {svc.url}
                  </span>
                </span>
                {openingServiceId === svc.id ? (
                  <Loader2 className="size-3.5 shrink-0 animate-spin" />
                ) : !svc.enabled ? (
                  <Badge variant="secondary" className="shrink-0">
                    Disabled
                  </Badge>
                ) : null}
              </button>
              {svc.enabled ? <HistoricalTrends deviceId={deviceId} entityId={svc.id} trends={[{ label: "Response time", metric: "responseTimeMs", unit: " ms" }, { label: "Service health", metric: "serviceHealth", unit: "" }]} /> : null}
              {svc.enabled ? <><ServiceHealthDetails health={snapshot?.serviceHealth?.[svc.id]} />{svc.containerName ? <p className="text-xs text-muted-foreground">Container: {svc.containerName}{!containers.some((container) => container.name === svc.containerName) ? " (not currently found)" : ""}</p> : null}</> : null}
              {svc.enabled ? <ServiceHealthBadge health={snapshot?.serviceHealth?.[svc.id]} /> : null}
              </div>
            ))}
          </div>
        )}
        {serviceOpenError ? (
          <p className="mt-2 text-sm text-destructive">{serviceOpenError}</p>
        ) : null}
      </section>
    </div>
  );
}

function InfoField({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div>
      <div className="text-[11px] text-muted-foreground">{label}</div>
      <div className={cn("mt-0.5 text-[13px] text-foreground", mono && "font-mono")}>{value}</div>
    </div>
  );
}


function BackButton({ onClick }: { onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label="Back to dashboard"
      className="flex h-[30px] w-[30px] shrink-0 items-center justify-center rounded-md border border-border text-foreground transition-colors hover:bg-white/[0.06] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
    >
      <ArrowLeft className="h-[15px] w-[15px]" strokeWidth={2.3} />
    </button>
  );
}
