import { useEffect, useState } from "react";
import { getDeviceActivity } from "@/lib/tauri/monitoring";
import type { ActivityEvent } from "@/types/activity";

/**
 * Persisted recent-activity feed for one device. Device filtering happens in
 * the backend against ActivityEvent.deviceId, retaining device isolation.
 */
export function useDeviceActivity(deviceId: string): ActivityEvent[] {
  const [entries, setEntries] = useState<ActivityEvent[]>([]);

  useEffect(() => {
    let active = true;
    setEntries([]);
    getDeviceActivity(deviceId).then((events) => {
      if (active) setEntries(events);
    }).catch(() => {
      if (active) setEntries([]);
    });
    return () => { active = false; };
  }, [deviceId]);

  return entries;
}
