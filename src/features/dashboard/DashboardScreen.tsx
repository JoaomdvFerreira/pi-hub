import { HardDrive } from "lucide-react";
import { EmptyState } from "@/components/layout/EmptyState";

export function DashboardScreen() {
  return (
    <div className="flex flex-col gap-4">
      <div>
        <h1 className="text-xl font-semibold text-foreground">Dashboard</h1>
        <p className="mt-0.5 text-sm text-muted-foreground">
          No devices registered yet.
        </p>
      </div>
      <EmptyState
        icon={HardDrive}
        title="No devices yet"
        description="Register a Raspberry Pi or Linux server to start monitoring it here."
      />
    </div>
  );
}
