import { useCallback, useEffect, useState } from "react";
import { Grid2x2, Loader2, X } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { EmptyState } from "@/components/layout/EmptyState";
import { useRouter } from "@/app/router";
import { getDevices, openDeviceService } from "@/lib/tauri/devices";
import type { Device, DeviceService } from "@/types/device";

interface ServicesScreenProps {
  deviceId?: string;
}

interface ServiceTile {
  device: Device;
  service: DeviceService;
}

export function ServicesScreen({ deviceId }: ServicesScreenProps) {
  const { goServices } = useRouter();
  const [devices, setDevices] = useState<Device[] | null>(null);
  const [openingKey, setOpeningKey] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const list = await getDevices();
      setDevices(list);
    } catch {
      setError("Could not load services.");
    }
  }, []);

  useEffect(() => {
    // Fetch-on-mount: no data-fetching library is in scope for the MVP yet.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    load();
  }, [load]);

  if (devices === null) {
    return (
      <div className="flex items-center justify-center gap-2 rounded-lg border border-dashed border-border py-16 text-sm text-muted-foreground">
        <Loader2 className="animate-spin" /> Loading…
      </div>
    );
  }

  const allTiles: ServiceTile[] = devices.flatMap((device) =>
    device.services.map((service) => ({ device, service })),
  );
  const filteredDevice = deviceId ? devices.find((d) => d.id === deviceId) : undefined;
  const tiles = deviceId ? allTiles.filter((t) => t.device.id === deviceId) : allTiles;

  async function handleOpen(tile: ServiceTile) {
    const key = `${tile.device.id}:${tile.service.id}`;
    setOpeningKey(key);
    setError(null);
    try {
      await openDeviceService(tile.device.id, tile.service.id);
    } catch {
      setError("Could not open this service. Check that a default browser is configured.");
    } finally {
      setOpeningKey(null);
    }
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-xl font-semibold text-foreground">Services</h1>
          <p className="mt-0.5 text-sm text-muted-foreground">
            {allTiles.length === 0
              ? "No services registered yet."
              : `${tiles.length} of ${allTiles.length} service${allTiles.length === 1 ? "" : "s"} shown.`}
          </p>
        </div>
        {filteredDevice ? (
          <button
            type="button"
            onClick={() => goServices()}
            className="flex shrink-0 items-center gap-1.5 rounded-full border border-primary/35 bg-primary/15 px-3 py-1.5 text-xs font-semibold text-primary"
          >
            Filtered: {filteredDevice.name}
            <X className="size-3" />
          </button>
        ) : null}
      </div>

      {error ? <p className="text-sm text-destructive">{error}</p> : null}

      {tiles.length === 0 ? (
        <EmptyState
          icon={Grid2x2}
          title={filteredDevice ? "No services on this device" : "No services yet"}
          description={
            filteredDevice
              ? "Register a service in this device's settings to see it here."
              : "Services you register on a device will appear here as quick-launch tiles."
          }
        />
      ) : (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(250px,1fr))] gap-3">
          {tiles.map((tile) => {
            const key = `${tile.device.id}:${tile.service.id}`;
            return (
              <div
                key={key}
                className="flex flex-col gap-2.5 rounded-lg border border-border bg-card p-3.5 transition-colors hover:border-white/20"
              >
                <div className="flex items-center gap-2.5">
                  <span className="flex size-[38px] shrink-0 items-center justify-center rounded-md bg-primary text-sm font-bold text-primary-foreground">
                    {tile.service.name.slice(0, 2).toUpperCase()}
                  </span>
                  <div className="min-w-0">
                    <div className="truncate text-sm font-semibold text-foreground">
                      {tile.service.name}
                    </div>
                    <div className="truncate text-xs text-muted-foreground">
                      {tile.device.name}
                    </div>
                  </div>
                  {!tile.service.enabled ? (
                    <Badge variant="secondary" className="ml-auto shrink-0">
                      Disabled
                    </Badge>
                  ) : null}
                </div>
                <div className="truncate font-mono text-[11.5px] text-muted-foreground">
                  {tile.service.url}
                </div>
                <button
                  type="button"
                  disabled={!tile.service.enabled || openingKey === key}
                  onClick={() => handleOpen(tile)}
                  className="flex items-center justify-center gap-1.5 rounded-md border border-border py-1.5 text-xs font-semibold text-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:bg-transparent"
                >
                  {openingKey === key ? <Loader2 className="size-3 animate-spin" /> : null}
                  Open
                </button>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
