import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { listen } from "@tauri-apps/api/event";
import { X } from "lucide-react";
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

interface TerminalModalProps {
  deviceId: string;
  deviceName: string;
  onClose: () => void;
}

export function TerminalModal({ deviceId, deviceName, onClose }: TerminalModalProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const sessionIdRef = useRef<string | null>(null);
  const [status, setStatus] = useState<SessionStatus>("connecting");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    let unlistenOutput: (() => void) | undefined;
    let unlistenExit: (() => void) | undefined;
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

    async function start() {
      try {
        const sessionId = await openTerminalSession(deviceId);
        if (disposed) {
          void closeTerminalSession(sessionId);
          return;
        }
        sessionIdRef.current = sessionId;
        setStatus("connected");

        unlistenOutput = await listen<string>(`terminal://output:${sessionId}`, (event) => {
          term.write(event.payload);
        });
        unlistenExit = await listen(`terminal://exit:${sessionId}`, () => {
          setStatus("closed");
        });

        term.onData((data) => {
          void writeTerminalInput(sessionId, data);
        });

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
    function handleResize() {
      fitAddonRef.current?.fit();
      const sessionId = sessionIdRef.current;
      const term = termRef.current;
      if (sessionId && term) {
        void resizeTerminalSession(sessionId, term.cols, term.rows);
      }
    }
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, []);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  return createPortal(
    <div
      className="fixed inset-0 z-20 flex items-center justify-center bg-black/55"
      onClick={(event) => {
        event.stopPropagation();
        onClose();
      }}
    >
      <div
        className="flex h-[420px] w-[680px] max-w-[90vw] flex-col overflow-hidden rounded-lg border border-white/10 bg-[#0c0c0c] shadow-2xl"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="flex h-9 shrink-0 items-center justify-between border-b border-white/[0.06] bg-[#161616] px-3">
          <span className="font-mono text-xs text-muted-foreground">
            {deviceName} — SSH Terminal
          </span>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close terminal"
            className="flex size-5.5 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-white/10"
          >
            <X className="size-3.5" />
          </button>
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
      </div>
    </div>,
    document.body,
  );
}
