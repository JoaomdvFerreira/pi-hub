import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { listen } from "@tauri-apps/api/event";
import { Minus, X } from "lucide-react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { cn } from "@/lib/utils";
import {
  closeTerminalSession,
  openTerminalSession,
  resizeTerminalSession,
  writeTerminalInput,
} from "@/lib/tauri/terminal";
import type { ApplicationError } from "@/types/settings";

function isApplicationError(err: unknown): err is ApplicationError {
  return typeof err === "object" && err !== null && "code" in err && "message" in err;
}

type SessionStatus = "connecting" | "connected" | "closed" | "error";

const DEFAULT_WIDTH = 680;
const DEFAULT_HEIGHT = 420;
const MIN_WIDTH = 420;
const MIN_HEIGHT = 260;
// Multiple terminals opened in a row start slightly offset from each other
// instead of stacking in exactly the same spot.
const CASCADE_OFFSET = 28;

interface TerminalOutputPayload {
  sessionId: string;
  data: string;
}

interface TerminalExitPayload {
  sessionId: string;
  exitCode?: number;
}

interface TerminalWindowProps {
  deviceId: string;
  deviceName: string;
  minimized: boolean;
  offsetIndex: number;
  onMinimize: () => void;
  onClose: () => void;
}

export function TerminalWindow({
  deviceId,
  deviceName,
  minimized,
  offsetIndex,
  onMinimize,
  onClose,
}: TerminalWindowProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const sessionIdRef = useRef<string | null>(null);
  const [status, setStatus] = useState<SessionStatus>("connecting");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [size, setSize] = useState({ width: DEFAULT_WIDTH, height: DEFAULT_HEIGHT });
  const [position, setPosition] = useState(() => ({
    x: Math.max(0, (window.innerWidth - DEFAULT_WIDTH) / 2 + offsetIndex * CASCADE_OFFSET),
    y: Math.max(0, (window.innerHeight - DEFAULT_HEIGHT) / 2 + offsetIndex * CASCADE_OFFSET),
  }));

  const fitAndResizeSession = useCallback(() => {
    fitAddonRef.current?.fit();
    const sessionId = sessionIdRef.current;
    const term = termRef.current;
    if (sessionId && term) {
      void resizeTerminalSession(sessionId, term.cols, term.rows);
    }
  }, []);

  const handleDragStart = useCallback(
    (event: React.MouseEvent) => {
      // Only the header background should drag; buttons handle their own
      // clicks.
      if ((event.target as HTMLElement).closest("button")) return;
      event.preventDefault();
      const startX = event.clientX;
      const startY = event.clientY;
      const startPos = position;

      function handleMove(moveEvent: MouseEvent) {
        setPosition({
          x: startPos.x + (moveEvent.clientX - startX),
          y: startPos.y + (moveEvent.clientY - startY),
        });
      }
      function handleUp() {
        window.removeEventListener("mousemove", handleMove);
        window.removeEventListener("mouseup", handleUp);
      }
      window.addEventListener("mousemove", handleMove);
      window.addEventListener("mouseup", handleUp);
    },
    [position],
  );

  const handleResizeStart = useCallback(
    (event: React.MouseEvent) => {
      event.preventDefault();
      event.stopPropagation();
      const startX = event.clientX;
      const startY = event.clientY;
      const startSize = size;

      function handleMove(moveEvent: MouseEvent) {
        setSize({
          width: Math.max(MIN_WIDTH, startSize.width + (moveEvent.clientX - startX)),
          height: Math.max(MIN_HEIGHT, startSize.height + (moveEvent.clientY - startY)),
        });
      }
      function handleUp() {
        window.removeEventListener("mousemove", handleMove);
        window.removeEventListener("mouseup", handleUp);
      }
      window.addEventListener("mousemove", handleMove);
      window.addEventListener("mouseup", handleUp);
    },
    [size],
  );

  useEffect(() => {
    let disposed = false;
    let unlistenOutput: (() => void) | undefined;
    let unlistenExit: (() => void) | undefined;
    // Output belonging to our session but received before we know our own
    // session id (see the race explained below) is held here and flushed
    // once we do, in order.
    const pendingOutput: string[] = [];
    // Keystrokes typed before the session is confirmed open. xterm.js
    // accepts input the instant it's mounted, well before
    // open_terminal_session resolves; registering onData only after that
    // (as an earlier version did) silently dropped anything typed in the
    // "Connecting..." window instead of buffering it.
    const pendingInput: string[] = [];

    const term = new Terminal({
      theme: { background: "#0c0c0c", foreground: "#d6d6d6", cursor: "#d6d6d6" },
      fontFamily: "Consolas, monospace",
      fontSize: 13,
      cursorBlink: true,
      scrollback: 5000,
    });
    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    termRef.current = term;
    fitAddonRef.current = fitAddon;
    if (containerRef.current) {
      term.open(containerRef.current);
      fitAddon.fit();
    }

    term.onData((data) => {
      if (sessionIdRef.current) {
        void writeTerminalInput(sessionIdRef.current, data);
      } else {
        pendingInput.push(data);
      }
    });

    async function start() {
      // Subscribed *before* the session is opened, and deliberately not
      // filtered to a not-yet-known session id: a pty starts producing
      // output (which can include a terminal-capability query the remote
      // side blocks on getting an answer to -- this is exactly what made
      // sessions hang indefinitely before this fix) the instant the child
      // process is spawned on the Rust side, well before
      // open_terminal_session's response reaches us with our session id.
      // Subscribing only after that response would create a window where
      // early output arrives with nobody listening and is silently lost.
      unlistenOutput = await listen<TerminalOutputPayload>("terminal://output", (event) => {
        if (sessionIdRef.current === null) {
          // Don't know our session id yet -- could be ours or a stale
          // event for an already-closed session; hold onto it and sort
          // it out once we do know.
          if (event.payload.sessionId) {
            pendingOutput.push(event.payload.sessionId + " " + event.payload.data);
          }
          return;
        }
        if (event.payload.sessionId === sessionIdRef.current) {
          term.write(event.payload.data);
        }
      });
      unlistenExit = await listen<TerminalExitPayload>("terminal://exit", (event) => {
        if (event.payload.sessionId === sessionIdRef.current) {
          setStatus("closed");
        }
      });

      // The cleanup function below closes over these same `let` bindings,
      // but if unmount already happened while the two listen() calls above
      // were still in flight, cleanup ran its `unlistenOutput?.()` /
      // `unlistenExit?.()` against `undefined` and will never be called
      // again -- the listeners would otherwise leak for the rest of the
      // app's life, each one still pushing every future event from every
      // other session into this now-orphaned pendingOutput array forever.
      if (disposed) {
        unlistenOutput();
        unlistenExit();
        return;
      }

      try {
        const sessionId = await openTerminalSession(deviceId);
        if (disposed) {
          void closeTerminalSession(sessionId);
          return;
        }
        sessionIdRef.current = sessionId;
        setStatus("connected");

        // Flush anything that arrived for this session while we didn't
        // yet know our own id, in the order it was received.
        for (const entry of pendingOutput) {
          const separatorIndex = entry.indexOf(" ");
          const entrySessionId = entry.slice(0, separatorIndex);
          if (entrySessionId === sessionId) {
            term.write(entry.slice(separatorIndex + 1));
          }
        }
        pendingOutput.length = 0;

        // Flush any keystrokes typed while still "connecting", in order,
        // before letting new ones (now handled directly since
        // sessionIdRef.current is set) get ahead of them.
        for (const data of pendingInput) {
          void writeTerminalInput(sessionId, data);
        }
        pendingInput.length = 0;

        void resizeTerminalSession(sessionId, term.cols, term.rows);
      } catch (err) {
        if (disposed) return;
        setStatus("error");
        setErrorMessage(
          isApplicationError(err) ? err.message : "Could not open a terminal session.",
        );
      }
    }

    void start();

    return () => {
      disposed = true;
      unlistenOutput?.();
      unlistenExit?.();
      if (sessionIdRef.current) {
        void closeTerminalSession(sessionIdRef.current);
      }
      term.dispose();
      termRef.current = null;
      fitAddonRef.current = null;
    };
  }, [deviceId]);

  useEffect(() => {
    window.addEventListener("resize", fitAndResizeSession);
    return () => window.removeEventListener("resize", fitAndResizeSession);
  }, [fitAndResizeSession]);

  // Re-fit after the DOM has actually painted the new size -- from a drag
  // resize, or from restoring out of the minimized (display: none) state,
  // where the container briefly had zero size. Calling fit() inside the
  // mousemove handler itself would run against the previous render's
  // dimensions.
  useEffect(() => {
    if (!minimized) fitAndResizeSession();
  }, [size, minimized, fitAndResizeSession]);

  return createPortal(
    // No dimming backdrop and no click-outside-to-close: this behaves like
    // a real floating window, not a blocking modal dialog, so the rest of
    // the app stays reachable underneath it. While minimized the whole
    // window is hidden (not unmounted, so the session and xterm buffer
    // stay alive) -- TerminalDock's footer is what represents it on
    // screen instead.
    <div
      className={cn("pointer-events-none fixed inset-0 z-20", minimized && "hidden")}
    >
      <div
        className="pointer-events-auto absolute flex flex-col overflow-hidden rounded-lg border border-white/10 bg-[#0c0c0c] shadow-2xl"
        style={{
          left: position.x,
          top: position.y,
          width: size.width,
          height: size.height,
        }}
        // React portal events bubble along the React tree, not the DOM
        // tree -- this window is rendered into document.body via a portal,
        // but stays a React descendant of whatever mounted TerminalDock
        // (currently AppShell). Defensive: without stopping propagation
        // here, a click/mousedown inside the terminal (minimize, drag,
        // typing) could still reach an ancestor's own click handler (this
        // is exactly what happened when terminals were previously opened
        // directly inside DeviceCard, whose card itself is a big onClick
        // that opens the device).
        onClick={(event) => event.stopPropagation()}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <div
          className="flex h-9 shrink-0 cursor-move items-center justify-between border-b border-white/6 bg-[#161616] px-3 select-none"
          onMouseDown={handleDragStart}
          onDoubleClick={onMinimize}
        >
          <span className="font-mono text-xs text-muted-foreground">
            {deviceName} — SSH Terminal
          </span>
          <div className="flex items-center gap-1">
            <button
              type="button"
              onClick={onMinimize}
              aria-label="Minimize terminal"
              className="flex size-5.5 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-white/10"
            >
              <Minus className="size-3.5" />
            </button>
            <button
              type="button"
              onClick={onClose}
              aria-label="Close terminal"
              className="flex size-5.5 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-white/10"
            >
              <X className="size-3.5" />
            </button>
          </div>
        </div>

        <div className="relative flex-1">
          <div
            ref={containerRef}
            className={cn("absolute inset-0 p-2", status === "error" && "invisible")}
          />
          {status === "error" ? (
            <div className="absolute inset-0 flex items-center justify-center bg-[#0c0c0c] p-4 text-center text-sm text-destructive">
              {errorMessage}
            </div>
          ) : null}
          {status === "connecting" ? (
            <div className="pointer-events-none absolute inset-x-0 top-0 bg-black/70 px-3 py-1.5 text-center text-[11px] text-muted-foreground">
              Connecting…
            </div>
          ) : null}
          {status === "closed" ? (
            <div className="pointer-events-none absolute inset-x-0 bottom-0 bg-black/70 px-3 py-1.5 text-center text-[11px] text-muted-foreground">
              Session ended.
            </div>
          ) : null}
        </div>

        <div
          onMouseDown={handleResizeStart}
          aria-hidden
          className="absolute bottom-0 right-0 size-4 cursor-nwse-resize"
        >
          <svg viewBox="0 0 16 16" className="size-full text-white/25">
            <path
              d="M15 15L15 9M15 15L9 15M15 15L5 15M15 15L15 5"
              stroke="currentColor"
              strokeWidth="1.2"
            />
          </svg>
        </div>
      </div>
    </div>,
    document.body,
  );
}
