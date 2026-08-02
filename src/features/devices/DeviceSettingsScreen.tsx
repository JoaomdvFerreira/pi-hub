import { useEffect, useState } from "react";
import { ArrowLeft, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { DeviceForm } from "@/features/devices/DeviceForm";
import { useRouter } from "@/app/router";
import { getDevice } from "@/lib/tauri/devices";
import type { Device } from "@/types/device";

interface DeviceSettingsScreenProps {
  deviceId: string;
}

export function DeviceSettingsScreen({ deviceId }: DeviceSettingsScreenProps) {
  const { goDashboard } = useRouter();
  const [device, setDevice] = useState<Device | null | undefined>(undefined);

  useEffect(() => {
    let cancelled = false;
    // Fetch-on-mount: no data-fetching library is in scope for the MVP yet,
    // so the lint rule's suggested alternative (a query library) doesn't
    // apply here.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setDevice(undefined);
    getDevice(deviceId).then((result) => {
      if (!cancelled) {
        setDevice(result);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [deviceId]);

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-2.5">
        <Button
          type="button"
          variant="outline"
          size="icon"
          onClick={goDashboard}
          aria-label="Back to dashboard"
        >
          <ArrowLeft />
        </Button>
        <h1 className="text-lg font-semibold text-foreground">
          {device ? `Settings · ${device.name}` : "Device settings"}
        </h1>
      </div>

      {device === undefined ? (
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Loader2 className="animate-spin" /> Loading…
        </div>
      ) : device === null ? (
        <p className="text-sm text-muted-foreground">
          This device no longer exists.
        </p>
      ) : (
        <DeviceForm
          mode="edit"
          device={device}
          onSaved={goDashboard}
          onCancel={goDashboard}
        />
      )}
    </div>
  );
}
