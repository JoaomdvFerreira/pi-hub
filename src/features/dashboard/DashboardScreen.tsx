import { useCallback, useEffect, useMemo, useState } from "react";
import { HardDrive, Loader2, Pencil, Plus, RefreshCw, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { EmptyState } from "@/components/layout/EmptyState";
import { useRouter } from "@/app/router";
import { deleteDevice, getDevices } from "@/lib/tauri/devices";
import { refreshAllDevices, refreshDevice } from "@/lib/tauri/monitoring";
import { useDeviceSnapshots } from "@/stores/useDeviceSnapshots";
import { useTerminalSessions } from "@/stores/useTerminalSessions";
import { DeviceCard } from "@/features/dashboard/DeviceCard";
import type { Device } from "@/types/device";

export function DashboardScreen() {
  const { goAddDevice, goDevice, goDeviceSettings, goServices } = useRouter();
  const { closeTerminal } = useTerminalSessions();
  const [devices, setDevices] = useState<Device[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [refreshingIds, setRefreshingIds] = useState<Set<string>>(new Set());
  const [refreshingAll, setRefreshingAll] = useState(false);

  const deviceIds = useMemo(() => (devices ?? []).map((d) => d.id), [devices]);
  const snapshots = useDeviceSnapshots(deviceIds);

  const load = useCallback(async () => {
    try {
      const list = await getDevices();
      setDevices(list);
      setError(null);
    } catch {
      setError("Could not load devices.");
    }
  }, []);

  useEffect(() => {
    // Fetch-on-mount: no data-fetching library is in scope for the MVP yet,
    // so the lint rule's suggested alternative (a query library) doesn't
    // apply here.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    load();
  }, [load]);

  async function handleDelete(id: string) {
    setDeletingId(id);
    try {
      await deleteDevice(id);
      // The Rust side already kills any live PTY session for this device;
      // this drops the now-orphaned floating terminal window (if any) so
      // it doesn't linger on screen referencing a device nothing can
      // navigate back to.
      closeTerminal(id);
      await load();
    } finally {
      setDeletingId(null);
    }
  }

  // useCallback (not a plain function) so this stays referentially stable
  // across renders -- DeviceCard is memoized specifically so one device's
  // snapshot update doesn't re-render every other card, which only works
  // if the callbacks it's passed don't themselves change identity every
  // render.
  const handleRefresh = useCallback(async (id: string) => {
    setRefreshingIds((prev) => new Set(prev).add(id));
    try {
      await refreshDevice(id);
    } catch {
      // The failure is already reflected in the device's snapshot
      // (connectionStatus/error), which the card renders directly.
    } finally {
      setRefreshingIds((prev) => {
        const next = new Set(prev);
        next.delete(id);
        return next;
      });
    }
  }, []);

  async function handleRefreshAll() {
    setRefreshingAll(true);
    try {
      await refreshAllDevices();
    } catch {
      setError("Could not refresh all devices.");
    } finally {
      setRefreshingAll(false);
    }
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-xl font-semibold text-foreground">Dashboard</h1>
          <p className="mt-0.5 text-sm text-muted-foreground">
            {devices === null
              ? "Loading devices…"
              : devices.length === 0
                ? "No devices registered yet."
                : `${devices.length} device${devices.length === 1 ? "" : "s"} registered.`}
          </p>
        </div>
        <div className="flex items-center gap-2">
          {devices !== null && devices.length > 0 ? (
            <Button variant="outline" onClick={handleRefreshAll} disabled={refreshingAll}>
              {refreshingAll ? <Loader2 className="animate-spin" /> : <RefreshCw />}
              Refresh All
            </Button>
          ) : null}
          <Button onClick={goAddDevice}>
            <Plus />
            Add Device
          </Button>
        </div>
      </div>

      {error ? <p className="text-sm text-destructive">{error}</p> : null}

      {devices === null ? (
        <div className="flex items-center justify-center gap-2 rounded-lg border border-dashed border-border py-16 text-sm text-muted-foreground">
          <Loader2 className="animate-spin" /> Loading…
        </div>
      ) : devices.length === 0 ? (
        <EmptyState
          icon={HardDrive}
          title="No devices yet"
          description="Register a Raspberry Pi or Linux server to start monitoring it here."
        />
      ) : (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(320px,1fr))] gap-3.5">
          {devices.map((device) => (
            <div key={device.id} className="flex flex-col gap-2">
              <DeviceCard
                device={device}
                snapshot={snapshots[device.id]}
                refreshing={refreshingIds.has(device.id)}
                onOpenDetail={goDevice}
                onOpenServices={goServices}
                onRefresh={handleRefresh}
              />
              <div className="flex items-center gap-2 px-0.5">
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 px-2 text-xs text-muted-foreground"
                  onClick={() => goDeviceSettings(device.id)}
                >
                  <Pencil className="size-3" /> Edit
                </Button>
                <AlertDialog>
                  <AlertDialogTrigger asChild>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-7 px-2 text-xs text-muted-foreground"
                      disabled={deletingId === device.id}
                    >
                      <Trash2 className="size-3" /> Delete
                    </Button>
                  </AlertDialogTrigger>
                  <AlertDialogContent>
                    <AlertDialogHeader>
                      <AlertDialogTitle>Delete {device.name}?</AlertDialogTitle>
                      <AlertDialogDescription>
                        This removes the device and its registered services
                        from Pi-Hub. This cannot be undone.
                      </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                      <AlertDialogCancel>Cancel</AlertDialogCancel>
                      <AlertDialogAction onClick={() => handleDelete(device.id)}>
                        Delete
                      </AlertDialogAction>
                    </AlertDialogFooter>
                  </AlertDialogContent>
                </AlertDialog>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
