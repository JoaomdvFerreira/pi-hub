import { DeviceForm } from "@/features/devices/DeviceForm";
import { useRouter } from "@/app/router";

export function AddDeviceScreen() {
  const { goDashboard } = useRouter();

  return (
    <div className="flex flex-col gap-4">
      <div>
        <h1 className="text-xl font-semibold text-foreground">Add device</h1>
        <p className="mt-0.5 text-sm text-muted-foreground">
          Register a Raspberry Pi or Linux server to start monitoring it.
        </p>
      </div>
      <DeviceForm mode="create" onSaved={goDashboard} onCancel={goDashboard} />
    </div>
  );
}
