import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getDeviceActivity } from "@/lib/tauri/monitoring";
import type { ActivityEvent } from "@/types/activity";

interface DeviceActivityState {
  deviceId: string;
  entries: ActivityEvent[];
}

/**
 * Persisted recent-activity feed for one device. Device filtering happens in
 * the backend against ActivityEvent.deviceId, retaining device isolation.
 */
export function useDeviceActivity(deviceId: string): ActivityEvent[] {
  const [activity, setActivity] = useState<DeviceActivityState>({ deviceId, entries: [] });

  useEffect(() => {
    let active = true;
    let latestRequest = 0;
    const load = () => {
      const request = ++latestRequest;
      getDeviceActivity(deviceId).then((entries) => {
        if (active && request === latestRequest) setActivity({ deviceId, entries });
      }).catch(() => {
        if (active && request === latestRequest) setActivity({ deviceId, entries: [] });
      });
    };
    load();
    const unlistenPromise = listen<string>("monitoring://refresh-completed", (event) => {
      if (event.payload === deviceId) load();
    });
    return () => { active = false; unlistenPromise.then((unlisten) => unlisten()); };
  }, [deviceId]);

  return activity.deviceId === deviceId ? activity.entries : [];
}
