# M14 WU14-01 — Performance Contract & Baseline Design

Status: Complete design artifact; implementation begins with WU14-02.  
Evidence boundary: This document specifies what to measure and protect. It does not claim a performance baseline or optimize product code.

## Production runtime-path inventory

| Path | Current ownership and flow | WU14 measurement boundary |
| --- | --- | --- |
| Monitoring scheduler | `monitoring/scheduler.rs`: a single `setup()`-started 5-second tick evaluates due enabled devices; `RefreshCoordinator` limits global concurrency to four and deduplicates per device. | `monitoring.scheduler_tick`, `monitoring.device_refresh`, refresh claims/skips, concurrency and overlap counters. |
| Normal/manual/tray refresh | Tauri `refresh_device`/`refresh_all_devices`, Dashboard/Device Detail actions and tray Refresh All converge on scheduler `refresh_one`; refresh runs in `spawn_blocking`. | Typed IPC count, one refresh elapsed, one SSH probe/metrics/Docker/visibility/service count, snapshot/history writes. |
| SSH and remote collection | `monitoring/refresh.rs` calls the typed OpenSSH executor for probe, metrics, Docker and independent visibility collectors with bounded timeouts; individual failures become partial/unavailable state. | `ssh.execute`, timeout/failure, elapsed and optional safe byte aggregate; never host or command text. |
| Service checks | Refresh invokes the existing threshold-aware HTTP(S) health path while assembling a snapshot. | `service.health_check`, success/failure, duration and safe count. |
| Docker and visibility | The same refresh gathers Docker and M11 Network/Storage/System visibility through independent typed parsers/collectors. | `docker.collect`, `visibility.network`, `.storage`, `.system`, duration/failure counts. |
| Persistence and history | Snapshot upsert is required; bounded JSON history append is best-effort after each refresh. Historical command queries/downsamples persisted series. | `snapshot.read/write`, `history.persist/query/downsample`, record/point counts and elapsed. |
| Activity and alerts | Refresh evaluates/persists alert transitions and writes meaningful Activity; `get_device_activity` reads exact-device persisted records. | `alert.evaluate`, `activity.read/write`, records inspected/returned where safely measurable. |
| Tauri IPC | Commands in `commands/monitoring.rs` expose refresh, latest snapshot, Activity and historical series; frontend wrappers use `invoke`. | `tauri.refresh_device`, `.get_latest_snapshot`, `.get_device_activity`, `.get_historical_series`, duration/serialization bytes where practical. |
| Device Detail loaders | `DeviceDetailScreen` loads device plus latest snapshot; it currently mounts `useDeviceActivity` and multiple `HistoricalTrends` surfaces immediately. `useDeviceSnapshots` subscribes before fetching persisted snapshots. | Scenario markers, exact IPC request counts, render/query work by active tab after WU14-05. |
| Historical Trends | `HistoricalTrends` issues one typed series query per metric/range; its metric-key dependency prevents an unchanged inline array from re-querying on settled parent renders. | `history.query/downsample`, series/point count, range-change count, render containment. |
| Tray and lifecycle | `platform/tray.rs` creates a tray menu; explicit Exit closes PTY sessions, removes tray, then exits. Window close hides the window; main setup starts scheduler once and single-instance prevents a second app/scheduler. | `lifecycle.window_close`, `lifecycle.explicit_exit`, tray removal attempt, PTY close count and process-exit elapsed. |

## Vocabulary and session contract

Operation names are stable opaque labels. Use these label families: `ssh.execute`, `ssh.timeout`, `ssh.failure`, `monitoring.device_refresh`, `service.health_check`, `docker.collect`, `visibility.network`, `visibility.storage`, `visibility.system`, `snapshot.read`, `snapshot.write`, `history.persist`, `history.query`, `history.downsample`, `activity.read`, `activity.write`, `alert.evaluate`, and `tauri.<command>`.

The reusable core exposes a `BenchmarkSession` created from `BenchmarkConfig`, plus `RuntimeSampler`, `OperationMeasurement`, `OperationAggregate`, `CounterSnapshot`, and `BenchmarkReport`. The frozen implementation contract is:

- Lifecycle: `Idle → Running → Stopped → Reviewed/Exported`; start is rejected when a session is Running; stop and configured expiry are idempotent.
- Defaults/bounds: 1-second sample interval; 15-minute maximum duration; one active session; `max_samples` is enforced from config; aggregates retain counts, total/max duration, failures, and optional safe byte total rather than operation events.
- Report: session ID, start/end/duration, scenario, config, build/environment metadata without secrets, bounded resource samples, sorted operation aggregates, counters, warnings, and optional operator markers.
- Inactive fast path: a single cheap active-state check; no sampler task, allocation, report write, or periodic diagnostics UI polling while inactive. The exact implementation must avoid a contended global mutex in ordinary operations.
- Privacy: labels/metadata/report warnings exclude passwords, private keys, auth headers, sensitive environment values, raw SSH command text, device hostnames/IPs, and secret-bearing URLs. Export is explicit JSON only.

The core is a separate local Rust crate/module with only standard Rust/time/serialization dependencies and its sampler trait. Pi-Hub supplies adapters and labels outside that boundary; the core imports no SSH, Docker, Tauri, device, Activity, or Alert type.

## Deterministic local scenarios and baseline run plan

Fixture-backed production abstractions provide `small` (2 devices), `medium` (5), and `larger-local` (10) profiles with bounded configured services/containers and repeated refresh cycles. Fake executors and a deterministic sampler make count/bound assertions runnable without a VM or Pi.

Baseline evidence sessions are: Idle; Dashboard; one normal Device Refresh; Device Detail Overview; Monitoring; Services; Containers; System; Activity; Historical 1h, 24h, 7d, and 30d; and explicit Exit/tray lifecycle. Each records scenario metadata, operation counts, elapsed time, resource samples, warnings, and a short operator note identifying local or real-device execution. Real devices are optional smoke evidence, never a prerequisite.

CPU/memory/threads/handles and elapsed/process exit time are machine-specific benchmark evidence. Request counts, duplicate-query absence, bounded sample/report/history points, no inactive sampler, no refresh overlap outside existing policy, activity/history tab gating, exact-device stale-result safety, and timeout/retention behavior are deterministic regression invariants. No final CPU/memory pass/fail threshold is set before WU14-04 evidence.

## Device Detail section mapping and query ownership

| Existing section | Target tab | Current data owner/query | WU14-05 contract |
| --- | --- | --- | --- |
| Header, connection badge, Refresh, terminal, edit/back | Persistent header | Device load; latest snapshot; typed action commands | Remain outside tabs; Refresh retains current scheduler path. |
| Current metric cards, uptime, compact health/connection state | Overview | Latest snapshot via `useDeviceSnapshots` | Reuse snapshot only; no new detail request. |
| Health Diagnostics and Power/Throttling | Monitoring | Snapshot health; `diagnose_device_connection` only after operator action | Mount only when Monitoring active; diagnostics remains explicit action. |
| Device Historical Trends | Monitoring | `get_historical_series` per metric/range | No mount/query outside Monitoring; retain active-range cache only when freshness is valid. |
| Service cards, health details, open links, service history | Services | Device service config and snapshot service health; current service `HistoricalTrends` queries | Keep configuration/snapshot reuse; gate service history to the active Services tab. |
| Docker table, detail dialog, M9 actions | Containers | Snapshot containers; typed container actions/details | Mount only on Containers; no extra refresh/scheduler. |
| Network, Storage, System visibility | System | Snapshot visibility fields | Reuse snapshot and mount only on System. |
| Recent Activity | Activity | `useDeviceActivity` calls persisted exact-device `get_device_activity` and listens to refresh completion | Do not instantiate hook outside Activity; preserve request identity/cancellation guard and exact-device empty state. |
| Device administration controls | Persistent header | Existing typed controlled administration commands | Must remain available regardless of tab; do not change semantics. |

The current in-memory router has no URL/query state. WU14-05 must decide whether its smallest safe native evolution can add the approved `tab` state; introducing a second routing mechanism is prohibited. Tab/device changes must ignore stale promises and clear/re-key state before commit. No new tab may start its own refresh loop.

## WU14-02 to WU14-08 checkpoints

| Work unit | Prerequisite / checkpoint |
| --- | --- |
| WU14-02 | Implement the frozen isolated core, fake/Windows sampler boundary, typed commands, and low-frequency UI status access; unit-test session bounds and inactive fast path. |
| WU14-03 | Add adapters only at selected production boundaries and deterministic 2/5/10-device profiles; test label redaction and bounded aggregation. |
| WU14-04 | Run the declared scenario matrix, publish baseline/request audit, and name measured hotspots; do not optimize before this evidence. |
| WU14-05 | Use WU14-04 evidence to implement accessible tabs, route decision, query gates, stale-result protection, no duplicate refresh loop, and chart containment tests. |
| WU14-06 | Capture packaging/process/tray evidence on Windows; correct icon/Start metadata and explicit Exit behavior, distinguishing shell cache residue from live orphan processes. |
| WU14-07 | For each improvement record problem, before evidence, change, after evidence, and deterministic regression coverage; preserve all monitoring/domain semantics. |
| WU14-08 | Lock deterministic gates, final before/after report, direct documentation updates, AIQT evidence, canonical validation, and closure; M15 remains planned. |

## Open design questions for WU14-02

1. Should the reusable core be a workspace crate now, or an internal crate-like module with a CI import-boundary test? Both meet M14; the workspace crate more visibly proves reuse but adds Cargo workspace scope.
2. Which supported Windows API/sampler library can reliably distinguish Pi-Hub from WebView2 child-process attribution without excessive platform dependency or sampling overhead? The report may mark child attribution unavailable.
3. What build/environment metadata is useful while still excluding user/device identity? Proposed minimum: app version, OS family/version, architecture, and diagnostics-core version.

## WU14-02 implementation note

WU14-02 resolves the core packaging decision as the local workspace crate `src-tauri/crates/pihub-benchmark-core`. It contains no Pi-Hub, SSH, Docker, or Tauri imports; Pi-Hub owns the Tauri state, typed commands, sampler task, and Settings surface. Windows sampling uses the current Pi-Hub process handle for working set, private bytes, and handle count. CPU/thread values remain unavailable in this work unit rather than relying on fragile attribution. WebView2 child processes are not included: reliable ownership cannot be established from the parent process handle alone, so their absence is explicit rather than heuristic.
