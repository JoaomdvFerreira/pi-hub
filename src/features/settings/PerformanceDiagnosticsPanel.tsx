import { useEffect, useState } from "react";
import { Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { exportPerformanceBenchmarkReport, getPerformanceBenchmarkReport, getPerformanceBenchmarkStatus, startPerformanceBenchmark, stopPerformanceBenchmark } from "@/lib/tauri/performance";
import type { BenchmarkReport, BenchmarkStatus } from "@/types/performance";

export function PerformanceDiagnosticsPanel() {
  const [status, setStatus] = useState<BenchmarkStatus | null>(null); const [report, setReport] = useState<BenchmarkReport | null>(null); const [busy, setBusy] = useState(false); const [error, setError] = useState<string | null>(null); const [exportMessage, setExportMessage] = useState<string | null>(null); const [clockMs, setClockMs] = useState(() => Date.now());
  useEffect(() => { getPerformanceBenchmarkStatus().then(setStatus).catch(() => setError("Could not load diagnostics status.")); getPerformanceBenchmarkReport().then(setReport).catch(() => undefined); }, []);
  const running = status?.state === "running";
  const startedAt = running ? status.session?.startedAtUnixMs : undefined;
  useEffect(() => { if (startedAt === undefined) return; const timer = window.setInterval(() => setClockMs(Date.now()), 1000); return () => window.clearInterval(timer); }, [startedAt]);
  const elapsedMs = startedAt === undefined ? 0 : Math.max(0, clockMs - startedAt);
  async function start() { setBusy(true); setError(null); setExportMessage(null); try { setReport(null); setClockMs(Date.now()); setStatus(await startPerformanceBenchmark()); } catch { setError("Could not start performance diagnostics."); } finally { setBusy(false); } }
  async function stop() { setBusy(true); setError(null); try { const next = await stopPerformanceBenchmark(); setReport(next); setStatus(await getPerformanceBenchmarkStatus()); } catch { setError("Could not stop performance diagnostics."); } finally { setBusy(false); } }
  async function exportReport() { setBusy(true); setError(null); setExportMessage(null); try { const path = await exportPerformanceBenchmarkReport(); setExportMessage(`Saved report to ${path}`); } catch { setError("Could not export performance report. Check that your Downloads folder is available and writable, then try again."); } finally { setBusy(false); } }
  return <section className="rounded-lg border border-border bg-card p-4"><div className="flex items-start justify-between gap-4"><div><h2 className="text-xs font-bold tracking-wide text-muted-foreground">PERFORMANCE DIAGNOSTICS</h2><p className="mt-1 text-xs text-muted-foreground">Opt-in, bounded local session. No background diagnostics work while idle.</p></div><Button size="sm" variant={running ? "outline" : "default"} disabled={busy} onClick={running ? stop : start}>{busy ? <Loader2 className="size-3.5 animate-spin" /> : running ? "Stop benchmark" : "Start benchmark"}</Button></div><p className="mt-3 text-sm text-foreground" aria-live="polite">Status: {running ? `Running · ${formatElapsed(elapsedMs)} (${status?.sampleCount ?? 0} samples)` : status?.state === "stopped" ? "Stopped" : "Idle"}</p>{report ? <div className="mt-2 flex items-center gap-3 text-xs text-muted-foreground"><span>Last report: {report.session.durationMs} ms · {report.samples.length} samples · {report.operations.length} operation classes</span><Button size="sm" variant="outline" disabled={busy} onClick={exportReport}>Export JSON</Button></div> : null}{exportMessage ? <p className="mt-2 text-xs text-status-healthy" role="status">{exportMessage}</p> : null}{error ? <p className="mt-2 text-xs text-destructive" role="alert">{error}</p> : null}</section>;
}

function formatElapsed(elapsedMs: number) { const seconds = Math.floor(elapsedMs / 1000); return `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`; }
