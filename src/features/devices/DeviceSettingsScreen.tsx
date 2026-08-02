import { ArrowLeft } from "lucide-react";
import { useRouter } from "@/app/router";

interface DeviceSettingsScreenProps {
  deviceId: string;
}

export function DeviceSettingsScreen({ deviceId }: DeviceSettingsScreenProps) {
  const { goDevice } = useRouter();

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-2.5">
        <button
          type="button"
          onClick={() => goDevice(deviceId)}
          aria-label="Back to device"
          className="flex h-[30px] w-[30px] items-center justify-center rounded-md border border-border text-foreground transition-colors hover:bg-white/[0.06] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          <ArrowLeft className="h-[15px] w-[15px]" strokeWidth={2.3} />
        </button>
        <h1 className="text-lg font-semibold text-foreground">
          Settings &middot; Device {deviceId}
        </h1>
      </div>
      <div className="max-w-xl rounded-lg border border-border bg-card p-6 text-sm text-muted-foreground">
        Device configuration, notification toggles, and registered services
        will appear here once the device registry exists.
      </div>
    </div>
  );
}
