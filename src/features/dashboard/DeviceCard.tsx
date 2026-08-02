import { useState, type MouseEvent } from "react";
import { Grid2x2, Loader2, RefreshCw, TerminalSquare, WifiOff } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { openDeviceTerminal } from "@/lib/tauri/devices";
import { formatUptime } from "@/lib/formatting/uptime";
import { formatRelativeTime } from "@/lib/formatting/relativeTime";
import { diskPercent, memoryPercent } from "@/lib/formatting/deviceMetrics";
import {
  levelBarClass,
  levelColorClass,
  temperatureBarClass,
  temperatureColorClass,
  clampPercent,
} from "@/lib/formatting/metricThresholds";
import { connectionStatusLabel } from "@/lib/formatting/connectionStatus";
import type { Device, DeviceType } from "@/types/device";
import type { DeviceSnapshot } from "@/types/snapshot";

const DEVICE_TYPE_LABELS: Record<DeviceType, string> = {
  "raspberry-pi": "Raspberry Pi",
  "linux-server": "Linux server",
  "mini-pc": "Mini PC",
  nas: "NAS",
  other: "Other",
};

interface MetricBarProps {
  label: string;
  value: number | undefined;
  unit: string;
  colorClass: string;
  barClass: string;
}

function MetricBar({ label, value, unit, colorClass, barClass }: MetricBarProps) {
  return (
    <div>
      <div className="text-[10px] font-bold tracking-wide text-muted-foreground">
        {label}
      </div>
      <div className={cn("mt-0.5 text-[15px] font-semibold", colorClass)}>
        {value === undefined ? "—" : `${Math.round(value)}${unit}`}
      </div>
      <div className="mt-1 h-1 rounded-full bg-muted">
        <div
          className={cn("h-1 rounded-full", barClass)}
          style={{ width: value === undefined ? "0%" : `${clampPercent(value)}%` }}
        />
      </div>
    </div>
  );
}

interface DeviceCardProps {
  device: Device;
  snapshot?: DeviceSnapshot;
  refreshing: boolean;
  onOpenDetail: () => void;
  onOpenServices: () => void;
  onRefresh: () => void;
}

export function DeviceCard({
  device,
  snapshot,
  refreshing,
  onOpenDetail,
  onOpenServices,
  onRefresh,
}: DeviceCardProps) {
  const hasEverRefreshed = snapshot !== undefined;
  const isOnline = snapshot?.connectionStatus === "online";
  const metrics = snapshot?.metrics;
  const mem = metrics ? memoryPercent(metrics) : undefined;
  const disk = metrics ? diskPercent(metrics) : undefined;
  const [openingTerminal, setOpeningTerminal] = useState(false);
  const [terminalError, setTerminalError] = useState<string | null>(null);

  function stopPropagation(event: MouseEvent) {
    event.stopPropagation();
  }

  async function handleOpenTerminal(event: MouseEvent) {
    stopPropagation(event);
    setOpeningTerminal(true);
    setTerminalError(null);
    try {
      await openDeviceTerminal(device.id);
    } catch {
      setTerminalError("Could not open a terminal for this device.");
    } finally {
      setOpeningTerminal(false);
    }
  }

  return (
    <div
      onClick={onOpenDetail}
      role="button"
      tabIndex={0}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          onOpenDetail();
        }
      }}
      className={cn(
        "flex cursor-pointer flex-col gap-2.5 rounded-lg border bg-card p-3.5 text-left transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        hasEverRefreshed && !isOnline ? "border-status-offline/30" : "border-border",
      )}
    >
      <div className="flex items-center justify-between gap-2">
        <div className="flex min-w-0 items-center gap-2">
          <span
            className={cn(
              "size-2 shrink-0 rounded-full",
              !hasEverRefreshed
                ? "bg-muted-foreground"
                : isOnline
                  ? "bg-status-healthy animate-pulse"
                  : "bg-status-offline",
            )}
          />
          <span className="truncate text-[15px] font-semibold text-foreground">
            {device.name}
          </span>
        </div>
        <Badge variant="secondary" className="shrink-0">
          {DEVICE_TYPE_LABELS[device.deviceType]}
        </Badge>
      </div>

      <div className="-mt-1.5 truncate font-mono text-xs text-muted-foreground">
        {device.host}
      </div>

      {!hasEverRefreshed ? (
        <p className="py-1 text-xs text-muted-foreground">Not refreshed yet.</p>
      ) : !isOnline ? (
        <div className="flex items-center gap-1.5 py-1 text-xs font-semibold text-status-offline">
          <WifiOff className="size-3.5 shrink-0" />
          <span>
            {connectionStatusLabel(snapshot.connectionStatus)} · last seen{" "}
            {formatRelativeTime(snapshot.lastSuccessfulRefresh)}
          </span>
        </div>
      ) : null}

      {metrics ? (
        <div className={cn("grid grid-cols-4 gap-2.5", snapshot?.stale && "opacity-60")}>
          <MetricBar
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
          <MetricBar
            label="MEM"
            value={mem}
            unit="%"
            colorClass={mem === undefined ? "text-muted-foreground" : levelColorClass(mem)}
            barClass={mem === undefined ? "bg-status-neutral" : levelBarClass(mem)}
          />
          <MetricBar
            label="DISK"
            value={disk}
            unit="%"
            colorClass={disk === undefined ? "text-muted-foreground" : levelColorClass(disk)}
            barClass={disk === undefined ? "bg-status-neutral" : levelBarClass(disk)}
          />
          <MetricBar
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
      ) : null}

      <div className="flex items-center gap-3 text-[11.5px] text-muted-foreground">
        <span>
          Up {metrics?.uptimeSeconds !== undefined ? formatUptime(metrics.uptimeSeconds) : "—"}
        </span>
        <span>·</span>
        <span>
          {snapshot?.containers.filter((c) => c.state === "running").length ?? 0}/
          {snapshot?.containers.length ?? 0} containers
        </span>
        <span>·</span>
        <span>{formatRelativeTime(snapshot?.capturedAt)}</span>
      </div>

      <div className="flex items-center gap-2">
        <Button
          variant="outline"
          size="sm"
          disabled={openingTerminal}
          title="Open an SSH session in Windows Terminal"
          onClick={handleOpenTerminal}
        >
          {openingTerminal ? <Loader2 className="animate-spin" /> : <TerminalSquare />} Terminal
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={(event) => {
            stopPropagation(event);
            onOpenServices();
          }}
        >
          <Grid2x2 /> Services
        </Button>
        <div className="flex-1" />
        <Button
          variant="outline"
          size="icon"
          className="size-7"
          title="Refresh"
          disabled={refreshing}
          onClick={(event) => {
            stopPropagation(event);
            onRefresh();
          }}
        >
          {refreshing ? (
            <Loader2 className="animate-spin" />
          ) : (
            <RefreshCw />
          )}
        </Button>
      </div>
      {terminalError ? <p className="text-[11px] text-destructive">{terminalError}</p> : null}
    </div>
  );
}
