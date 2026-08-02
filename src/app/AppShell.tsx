import { NavRail } from "@/components/layout/NavRail";
import { useRouter } from "@/app/router";
import { DashboardScreen } from "@/features/dashboard/DashboardScreen";
import { ServicesScreen } from "@/features/services/ServicesScreen";
import { GlobalSettingsScreen } from "@/features/settings/GlobalSettingsScreen";
import { AddDeviceScreen } from "@/features/devices/AddDeviceScreen";
import { DeviceDetailScreen } from "@/features/devices/DeviceDetailScreen";
import { DeviceSettingsScreen } from "@/features/devices/DeviceSettingsScreen";

export function AppShell() {
  const { screen } = useRouter();

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-background text-foreground">
      <NavRail />
      <main className="min-w-0 flex-1 overflow-y-auto px-6.5 py-5">
        {screen.name === "dashboard" && <DashboardScreen />}
        {screen.name === "addDevice" && <AddDeviceScreen />}
        {screen.name === "services" && (
          <ServicesScreen deviceId={screen.deviceId} />
        )}
        {screen.name === "globalSettings" && <GlobalSettingsScreen />}
        {screen.name === "device" && (
          <DeviceDetailScreen key={screen.deviceId} deviceId={screen.deviceId} />
        )}
        {screen.name === "deviceSettings" && (
          <DeviceSettingsScreen deviceId={screen.deviceId} />
        )}
      </main>
    </div>
  );
}
