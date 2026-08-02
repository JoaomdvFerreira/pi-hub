import { ArrowLeft } from "lucide-react";
import { useRouter } from "@/app/router";

interface DeviceDetailScreenProps {
  deviceId: string;
}

export function DeviceDetailScreen({ deviceId }: DeviceDetailScreenProps) {
  const { goDashboard } = useRouter();

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-2.5">
        <button
          type="button"
          onClick={goDashboard}
          aria-label="Back to dashboard"
          className="flex h-[30px] w-[30px] items-center justify-center rounded-md border border-border text-foreground transition-colors hover:bg-white/[0.06] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          <ArrowLeft className="h-[15px] w-[15px]" strokeWidth={2.3} />
        </button>
        <h1 className="text-lg font-semibold text-foreground">
          Device {deviceId}
        </h1>
      </div>
      <div className="rounded-lg border border-border bg-card p-6 text-sm text-muted-foreground">
        Device metrics, containers, activity, and services will appear here
        once monitoring is implemented.
      </div>
    </div>
  );
}
