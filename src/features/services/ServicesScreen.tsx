import { Grid2x2 } from "lucide-react";
import { EmptyState } from "@/components/layout/EmptyState";

interface ServicesScreenProps {
  deviceId?: string;
}

export function ServicesScreen({ deviceId: _deviceId }: ServicesScreenProps) {
  return (
    <div className="flex flex-col gap-4">
      <div>
        <h1 className="text-xl font-semibold text-foreground">Services</h1>
        <p className="mt-0.5 text-sm text-muted-foreground">
          No services registered yet.
        </p>
      </div>
      <EmptyState
        icon={Grid2x2}
        title="No services yet"
        description="Services you register on a device will appear here as quick-launch tiles."
      />
    </div>
  );
}
