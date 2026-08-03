import { NavRail } from "@/components/layout/NavRail";
import { useRouter } from "@/app/router";
import { DashboardScreen } from "@/features/dashboard/DashboardScreen";
import { ServicesScreen } from "@/features/services/ServicesScreen";
import { GlobalSettingsScreen } from "@/features/settings/GlobalSettingsScreen";
import { AddDeviceScreen } from "@/features/devices/AddDeviceScreen";
import { DeviceDetailScreen } from "@/features/devices/DeviceDetailScreen";
import { DeviceSettingsScreen } from "@/features/devices/DeviceSettingsScreen";
import { TerminalDock } from "@/features/terminal/TerminalDock";
import { useTerminalSessions } from "@/stores/useTerminalSessions";
import { cn } from "@/lib/utils";

export function AppShell() {
  const { screen } = useRouter();
  const { sessions } = useTerminalSessions();
  const hasMinimizedTerminal = sessions.some((s) => s.minimized);

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-background text-foreground">
      <NavRail />
      <main
        className={cn(
          "min-w-0 flex-1 overflow-y-auto px-6.5 py-5",
          hasMinimizedTerminal && "pb-14",
        )}
      >
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
      <TerminalDock />
    </div>
  );
}
