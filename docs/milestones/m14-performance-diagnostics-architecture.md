# M14 Architecture — Reusable In-Application Performance Diagnostics

Status: Ready for implementation  
Milestone: M14

## Purpose and layers

Pi-Hub needs a compact opt-in diagnostics capability without an always-on telemetry system. The reusable core owns sessions, timers, counters, bounded reports, and a pluggable process sampler. Pi-Hub adapters own labels and instrumentation of SSH, monitoring, persistence, history, Activity, IPC, and UI paths. Typed Tauri commands and the diagnostics UI sit above those adapters.

The core must not import Pi-Hub models, SSH, Docker, Tauri command types, or Activity/Alert types. It is a workspace module/crate boundary for M14; crates.io publication and a public SemVer contract are out of scope.

## Contract constraints

- At most one active session; default sample interval is 1 second and maximum interactive duration is 15 minutes, subject to WU14-01 contract freezing.
- Reports are in memory, samples and operation aggregates are bounded, and export is explicit; no automatic benchmark history is persisted.
- When inactive there is no sampler task, no sample accumulation, no file I/O, and no heavy global lock on every instrumented operation.
- The sampler interface supports Windows first, a deterministic fake sampler, and future platforms without making the core Windows-only.
- Reports use normalized labels only: never credentials, IPs/hostnames, auth headers, raw commands, sensitive environment variables, or secret-bearing URLs.

## Stable label families

`ssh.execute`, `ssh.timeout`, `ssh.failure`, `monitoring.device_refresh`, `service.health_check`, `docker.collect`, `visibility.network`, `visibility.storage`, `visibility.system`, `snapshot.read`, `snapshot.write`, `history.persist`, `history.query`, `history.downsample`, `activity.read`, `activity.write`, `alert.evaluate`, `tauri.<typed-command>`, and `ui.<scenario-marker>`.

Labels are identifiers, not payloads. Optional counters may report safe aggregate payload bytes; no raw request/response content is retained.

## Evidence model

Machine-dependent evidence includes elapsed time, CPU, working/private memory, thread/handle counts, and explicit process exit time. Deterministic gates cover request and operation counts, no duplicate history query after a settled render, bounded samples/points, no inactive sampler, no unexpected refresh overlap, timeout/retention behavior, and no stale device data.

Suggested fixture profiles are small (2 devices), medium (5), and larger-local (10), each with bounded services/containers and repeated refresh cycles through production abstractions. They are workload inputs, not capacity claims.
