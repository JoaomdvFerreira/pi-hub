import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getLatestSnapshot } from "@/lib/tauri/monitoring";
import type { DeviceSnapshot } from "@/types/snapshot";

/**
 * Tracks the latest known snapshot per device: fetches whatever was last
 * persisted to state.json for each device on mount, then stays live by
 * subscribing to the backend's `device://snapshot-updated` event for the
 * component's lifetime.
 */
export function useDeviceSnapshots(
  deviceIds: string[],
): Record<string, DeviceSnapshot> {
  const [snapshots, setSnapshots] = useState<Record<string, DeviceSnapshot>>(
    {},
  );
  const deviceIdsKey = deviceIds.join(",");

  useEffect(() => {
    let cancelled = false;
    const ids = deviceIdsKey ? deviceIdsKey.split(",") : [];

    Promise.all(
      ids.map((id) =>
        getLatestSnapshot(id)
          .then((snapshot) => [id, snapshot] as const)
          .catch(() => [id, null] as const),
      ),
    ).then((pairs) => {
      if (cancelled) return;
      setSnapshots((prev) => {
        const next = { ...prev };
        for (const [id, snapshot] of pairs) {
          if (snapshot) {
            next[id] = snapshot;
          }
        }
        return next;
      });
    });

    return () => {
      cancelled = true;
    };
  }, [deviceIdsKey]);

  useEffect(() => {
    const unlistenPromise = listen<DeviceSnapshot>(
      "device://snapshot-updated",
      (event) => {
        setSnapshots((prev) => ({
          ...prev,
          [event.payload.deviceId]: event.payload,
        }));
      },
    );

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  return snapshots;
}
