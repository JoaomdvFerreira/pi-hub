import { useCallback, useEffect, useState } from "react";
import { HardDrive, Loader2, Pencil, Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
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
import type { Device, DeviceType } from "@/types/device";

const DEVICE_TYPE_LABELS: Record<DeviceType, string> = {
  "raspberry-pi": "Raspberry Pi",
  "linux-server": "Linux server",
  "mini-pc": "Mini PC",
  nas: "NAS",
  other: "Other",
};

export function DashboardScreen() {
  const { goAddDevice, goDeviceSettings } = useRouter();
  const [devices, setDevices] = useState<Device[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [deletingId, setDeletingId] = useState<string | null>(null);

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
      await load();
    } finally {
      setDeletingId(null);
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
        <Button onClick={goAddDevice}>
          <Plus />
          Add Device
        </Button>
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
        <div className="flex flex-col gap-2">
          {devices.map((device) => (
            <div
              key={device.id}
              className="flex items-center gap-3 rounded-lg border border-border bg-card p-3"
            >
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="truncate text-sm font-semibold text-foreground">
                    {device.name}
                  </span>
                  <Badge variant="secondary">
                    {DEVICE_TYPE_LABELS[device.deviceType]}
                  </Badge>
                </div>
                <div className="truncate font-mono text-xs text-muted-foreground">
                  {device.sshUsername}@{device.host}:{device.sshPort}
                </div>
              </div>
              <Button
                variant="outline"
                size="sm"
                onClick={() => goDeviceSettings(device.id)}
              >
                <Pencil /> Edit
              </Button>
              <AlertDialog>
                <AlertDialogTrigger asChild>
                  <Button
                    variant="outline"
                    size="sm"
                    disabled={deletingId === device.id}
                  >
                    <Trash2 /> Delete
                  </Button>
                </AlertDialogTrigger>
                <AlertDialogContent>
                  <AlertDialogHeader>
                    <AlertDialogTitle>Delete {device.name}?</AlertDialogTitle>
                    <AlertDialogDescription>
                      This removes the device and its registered services from
                      Pi-Hub. This cannot be undone.
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
          ))}
        </div>
      )}
    </div>
  );
}
