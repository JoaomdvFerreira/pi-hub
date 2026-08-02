import { createContext, useContext, useState, type ReactNode } from "react";

export type Screen =
  | { name: "dashboard" }
  | { name: "addDevice" }
  | { name: "device"; deviceId: string }
  | { name: "deviceSettings"; deviceId: string }
  | { name: "services"; deviceId?: string }
  | { name: "globalSettings" };

interface RouterContextValue {
  screen: Screen;
  goDashboard: () => void;
  goAddDevice: () => void;
  goServices: (deviceId?: string) => void;
  goGlobalSettings: () => void;
  goDevice: (deviceId: string) => void;
  goDeviceSettings: (deviceId: string) => void;
}

const RouterContext = createContext<RouterContextValue | null>(null);

export function RouterProvider({ children }: { children: ReactNode }) {
  const [screen, setScreen] = useState<Screen>({ name: "dashboard" });

  const value: RouterContextValue = {
    screen,
    goDashboard: () => setScreen({ name: "dashboard" }),
    goAddDevice: () => setScreen({ name: "addDevice" }),
    goServices: (deviceId) => setScreen({ name: "services", deviceId }),
    goGlobalSettings: () => setScreen({ name: "globalSettings" }),
    goDevice: (deviceId) => setScreen({ name: "device", deviceId }),
    goDeviceSettings: (deviceId) =>
      setScreen({ name: "deviceSettings", deviceId }),
  };

  return (
    <RouterContext.Provider value={value}>{children}</RouterContext.Provider>
  );
}

export function useRouter(): RouterContextValue {
  const ctx = useContext(RouterContext);
  if (!ctx) {
    throw new Error("useRouter must be used within a RouterProvider");
  }
  return ctx;
}
