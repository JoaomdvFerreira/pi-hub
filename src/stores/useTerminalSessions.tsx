import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";

export interface TerminalSessionMeta {
  deviceId: string;
  deviceName: string;
  minimized: boolean;
}

interface TerminalSessionsContextValue {
  sessions: TerminalSessionMeta[];
  openTerminal: (deviceId: string, deviceName: string) => void;
  closeTerminal: (deviceId: string) => void;
  minimizeTerminal: (deviceId: string) => void;
  restoreTerminal: (deviceId: string) => void;
}

const TerminalSessionsContext = createContext<TerminalSessionsContextValue | null>(null);

/**
 * Tracks which devices currently have an open in-app terminal and whether
 * each is minimized. Lifted above individual screens (mounted once in
 * AppShell) so a terminal opened from the dashboard or a device detail
 * screen survives navigating away, and so `TerminalDock` can render every
 * open terminal plus a footer of minimized ones from a single place.
 */
export function TerminalSessionsProvider({ children }: { children: ReactNode }) {
  const [sessions, setSessions] = useState<TerminalSessionMeta[]>([]);

  const openTerminal = useCallback((deviceId: string, deviceName: string) => {
    setSessions((prev) => {
      const existing = prev.find((s) => s.deviceId === deviceId);
      if (existing) {
        // Already open: just bring it back if it was minimized. There is
        // never more than one terminal window per device -- matches the
        // one-session-per-device guarantee on the Rust side.
        return existing.minimized
          ? prev.map((s) => (s.deviceId === deviceId ? { ...s, minimized: false } : s))
          : prev;
      }
      return [...prev, { deviceId, deviceName, minimized: false }];
    });
  }, []);

  const closeTerminal = useCallback((deviceId: string) => {
    setSessions((prev) => prev.filter((s) => s.deviceId !== deviceId));
  }, []);

  const minimizeTerminal = useCallback((deviceId: string) => {
    setSessions((prev) =>
      prev.map((s) => (s.deviceId === deviceId ? { ...s, minimized: true } : s)),
    );
  }, []);

  const restoreTerminal = useCallback((deviceId: string) => {
    setSessions((prev) =>
      prev.map((s) => (s.deviceId === deviceId ? { ...s, minimized: false } : s)),
    );
  }, []);

  const value = useMemo(
    () => ({ sessions, openTerminal, closeTerminal, minimizeTerminal, restoreTerminal }),
    [sessions, openTerminal, closeTerminal, minimizeTerminal, restoreTerminal],
  );

  return (
    <TerminalSessionsContext.Provider value={value}>{children}</TerminalSessionsContext.Provider>
  );
}

export function useTerminalSessions(): TerminalSessionsContextValue {
  const ctx = useContext(TerminalSessionsContext);
  if (!ctx) {
    throw new Error("useTerminalSessions must be used within a TerminalSessionsProvider");
  }
  return ctx;
}
