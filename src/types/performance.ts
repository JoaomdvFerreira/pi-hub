export interface BenchmarkConfig { name: string; scenario: string; sampleIntervalMs: number; maxDurationMs: number; maxSamples: number; }
export interface SessionSummary { id: string; name: string; scenario: string; startedAtUnixMs: number; endedAtUnixMs?: number; durationMs: number; }
export interface BenchmarkStatus { state: "idle" | "running" | "stopped"; session?: SessionSummary; sampleCount?: number; }
export interface BenchmarkReport { session: SessionSummary; config: BenchmarkConfig; samples: unknown[]; operations: { name: string; count: number; totalDurationMs: number; maxDurationMs: number; failures: number }[]; warnings: string[]; }
