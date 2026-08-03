import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getLatestSnapshot } from "@/lib/tauri/monitoring";
import type { DeviceSnapshot } from "@/types/snapshot";

/**
 * Tracks the latest known snapshot per device: fetches whatever was last
 * persisted to state.json for each device on mount, then stays live by
 * subscribing to the backend's `device://snapshot-updated` event.
 *
 * The subscription is established *before* the initial fetch starts (and
 * awaited before that fetch begins), not concurrently with it -- two
 * separate, uncoordinated effects previously did this at the same time,
 * which left a window where a refresh completing between "fetch resolved"
 * and "listener registered" was silently dropped (the same shape of race
 * that made terminal sessions hang before that was fixed). The merge
 * below is an extra guard on top of that ordering: a fetched snapshot
 * only overwrites what's already in state if it's not older, so even a
 * fetch that resolves after a live update for the same device can't
 * clobber it with stale data.
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
    let unlisten: (() => void) | undefined;
    const ids = deviceIdsKey ? deviceIdsKey.split(",") : [];

    async function start() {
      unlisten = await listen<DeviceSnapshot>(
        "device://snapshot-updated",
        (event) => {
          setSnapshots((prev) => ({
            ...prev,
            [event.payload.deviceId]: event.payload,
          }));
        },
      );
      if (cancelled) {
        unlisten();
        return;
      }

      const pairs = await Promise.all(
        ids.map((id) =>
          getLatestSnapshot(id)
            .then((snapshot) => [id, snapshot] as const)
            .catch(() => [id, null] as const),
        ),
      );
      if (cancelled) return;

      setSnapshots((prev) => {
        const next = { ...prev };
        for (const [id, snapshot] of pairs) {
          if (!snapshot) continue;
          const existing = next[id];
          if (!existing || snapshot.capturedAt >= existing.capturedAt) {
            next[id] = snapshot;
          }
        }
        return next;
      });
    }

    void start();

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [deviceIdsKey]);

  return snapshots;
}
