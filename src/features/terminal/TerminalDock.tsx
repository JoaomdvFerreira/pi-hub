import { Suspense } from "react";
import { createPortal } from "react-dom";
import { TerminalSquare, X } from "lucide-react";
import { useTerminalSessions } from "@/stores/useTerminalSessions";
import { LazyTerminalWindow } from "@/features/terminal/LazyTerminalWindow";

/**
 * Mounted once, above routing, so an in-app terminal survives navigating
 * between screens. Every open session gets a `TerminalWindow` (always
 * mounted for the session's lifetime, so its SSH connection and xterm
 * buffer stay alive across minimize/restore); minimized ones additionally
 * get a chip in the footer bar to restore or close them from.
 */
export function TerminalDock() {
  const { sessions, closeTerminal, minimizeTerminal, restoreTerminal } = useTerminalSessions();

  if (sessions.length === 0) return null;

  const minimizedSessions = sessions.filter((s) => s.minimized);

  return (
    <>
      {sessions.map((session, index) => (
        <Suspense key={session.deviceId} fallback={null}>
          <LazyTerminalWindow
            deviceId={session.deviceId}
            deviceName={session.deviceName}
            minimized={session.minimized}
            offsetIndex={index}
            onMinimize={() => minimizeTerminal(session.deviceId)}
            onClose={() => closeTerminal(session.deviceId)}
          />
        </Suspense>
      ))}

      {minimizedSessions.length > 0
        ? createPortal(
            <div className="pointer-events-none fixed inset-x-0 bottom-0 z-30 flex justify-start">
              <div className="pointer-events-auto flex items-center gap-1.5 rounded-tr-lg border-r border-t border-border bg-card px-2 py-1.5 shadow-lg">
                {minimizedSessions.map((session) => (
                  <div
                    key={session.deviceId}
                    className="flex items-center gap-1 rounded-md border border-border bg-background pl-2.5 pr-1 py-1 text-xs text-foreground"
                  >
                    <button
                      type="button"
                      onClick={() => restoreTerminal(session.deviceId)}
                      title={`Restore ${session.deviceName} terminal`}
                      className="flex items-center gap-1.5 hover:text-primary"
                    >
                      <TerminalSquare className="size-3.5" />
                      {session.deviceName}
                    </button>
                    <button
                      type="button"
                      onClick={() => closeTerminal(session.deviceId)}
                      aria-label={`Close ${session.deviceName} terminal`}
                      className="flex size-4.5 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-white/10 hover:text-foreground"
                    >
                      <X className="size-3" />
                    </button>
                  </div>
                ))}
              </div>
            </div>,
            document.body,
          )
        : null}
    </>
  );
}
