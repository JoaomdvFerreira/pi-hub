import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { NotificationEvent } from "@/types/notification";

const MAX_ENTRIES = 20;

export interface ActivityEntry extends NotificationEvent {
  receivedAt: string;
}

/**
 * Session-scoped recent-activity feed for one device: the backend emits
 * `notification://ready` only for transitions detected while the app is
 * running (WU012), there is no persisted notification history to query, so
 * this starts empty on every app launch and fills in as transitions occur.
 *
 * Callers must remount this hook's component (e.g. `key={deviceId}`) when
 * switching devices -- it does not reset its own state on a deviceId change.
 */
export function useDeviceActivity(deviceId: string): ActivityEntry[] {
  const [entries, setEntries] = useState<ActivityEntry[]>([]);

  useEffect(() => {
    const unlistenPromise = listen<NotificationEvent[]>(
      "notification://ready",
      (event) => {
        const forDevice = event.payload.filter((n) => n.deviceId === deviceId);
        if (forDevice.length === 0) return;

        const receivedAt = new Date().toISOString();
        setEntries((prev) =>
          [...forDevice.map((n) => ({ ...n, receivedAt })).reverse(), ...prev].slice(
            0,
            MAX_ENTRIES,
          ),
        );
      },
    );

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [deviceId]);

  return entries;
}
