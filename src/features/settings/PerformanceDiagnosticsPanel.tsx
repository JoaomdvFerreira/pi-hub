import { useEffect, useState } from "react";
import { Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { getPerformanceBenchmarkReport, getPerformanceBenchmarkStatus, startPerformanceBenchmark, stopPerformanceBenchmark } from "@/lib/tauri/performance";
import type { BenchmarkReport, BenchmarkStatus } from "@/types/performance";

export function PerformanceDiagnosticsPanel() {
  const [status, setStatus] = useState<BenchmarkStatus | null>(null); const [report, setReport] = useState<BenchmarkReport | null>(null); const [busy, setBusy] = useState(false); const [error, setError] = useState<string | null>(null);
  useEffect(() => { getPerformanceBenchmarkStatus().then(setStatus).catch(() => setError("Could not load diagnostics status.")); getPerformanceBenchmarkReport().then(setReport).catch(() => undefined); }, []);
  async function start() { setBusy(true); setError(null); try { setStatus(await startPerformanceBenchmark()); } catch { setError("Could not start performance diagnostics."); } finally { setBusy(false); } }
  async function stop() { setBusy(true); setError(null); try { const next = await stopPerformanceBenchmark(); setReport(next); setStatus(await getPerformanceBenchmarkStatus()); } catch { setError("Could not stop performance diagnostics."); } finally { setBusy(false); } }
  function exportReport() { if (!report) return; const url = URL.createObjectURL(new Blob([JSON.stringify(report, null, 2)], { type: "application/json" })); const link = document.createElement("a"); link.href = url; link.download = "pihub-performance-report.json"; link.click(); URL.revokeObjectURL(url); }
  const running = status?.state === "running";
  return <section className="rounded-lg border border-border bg-card p-4"><div className="flex items-start justify-between gap-4"><div><h2 className="text-xs font-bold tracking-wide text-muted-foreground">PERFORMANCE DIAGNOSTICS</h2><p className="mt-1 text-xs text-muted-foreground">Opt-in, bounded local session. No background diagnostics work while idle.</p></div><Button size="sm" variant={running ? "outline" : "default"} disabled={busy} onClick={running ? stop : start}>{busy ? <Loader2 className="size-3.5 animate-spin" /> : running ? "Stop benchmark" : "Start benchmark"}</Button></div><p className="mt-3 text-sm text-foreground">Status: {running ? `Running (${status?.sampleCount ?? 0} samples)` : status?.state === "stopped" ? "Stopped" : "Idle"}</p>{report ? <div className="mt-2 flex items-center gap-3 text-xs text-muted-foreground"><span>Last report: {report.session.durationMs} ms · {report.samples.length} samples · {report.operations.length} operation classes</span><Button size="sm" variant="outline" onClick={exportReport}>Export JSON</Button></div> : null}{error ? <p className="mt-2 text-xs text-destructive">{error}</p> : null}</section>;
}
