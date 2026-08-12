# M14 WU14-04 — Pre-Optimization Baseline & Request Audit

Date: 2026-08-12  
Evidence scope: deterministic local fixture execution plus static ownership audit. This is not a live-device or interactive-WebView performance claim.

## Executed baseline scenarios

| Scenario | Evidence | Result |
| --- | --- | --- |
| Idle | Core inactive-path test | No active report, sampler, operation aggregate, or UI polling work. |
| Normal device refresh | Fixture-backed production refresh orchestration | One refresh produces six SSH operations: probe, metrics, Docker, Network, Storage, and System collection. |
| Synthetic small | 2 fixed fixture devices | 2 refreshes, 12 SSH operations, and 2 each Docker/Network/Storage/System collection operations. |
| Synthetic medium | 5 fixed fixture devices | 5 refreshes, 30 SSH operations, and 5 each Docker/Network/Storage/System collection operations. |
| Synthetic larger-local | 10 fixed fixture devices | 10 refreshes, 60 SSH operations, and 10 each Docker/Network/Storage/System collection operations. |
| Dashboard / Device Detail | Unavailable | Requires a running interactive Tauri/WebView session; no UI automation or live app session was authorized/provided. |
| Historical 1h/24h/7d/30d | Unavailable | Requires populated persisted history and a running typed-command host. Query/downsample instrumentation is in place for WU14-05/real-session evidence. |
| Pi-Hub process resources | Unavailable | Current-process sampler requires a live Pi-Hub process. No claim is made from test-runner process resources. |
| Explicit Exit/tray lifecycle | Deferred | WU14-06 owns Windows lifecycle evidence. |

The synthetic profiles use `FakeRemoteExecutor` and the existing refresh orchestration/parsers; no VM, Pi, DNS, HTTP request, or SSH process is used. Fixture work completes below the aggregate millisecond resolution, so elapsed values are intentionally not used as a machine-performance baseline.

## Deterministic request model

| Operation | Per successful fixture device refresh | 2 devices | 5 devices | 10 devices |
| --- | ---: | ---: | ---: | ---: |
| `monitoring.device_refresh` | 1 | 2 | 5 | 10 |
| `ssh.execute` | 6 | 12 | 30 | 60 |
| `docker.collect` | 1 | 2 | 5 | 10 |
| `visibility.network` | 1 | 2 | 5 | 10 |
| `visibility.storage` | 1 | 2 | 5 | 10 |
| `visibility.system` | 1 | 2 | 5 | 10 |

The profile test asserts this linear relationship. `service.health_check` is zero because these fixture profiles contain no configured services. Snapshot/history/activity/alert persistence requires the Tauri storage host and is therefore not fabricated in the fixture evidence. Instrumentation records failures and safe serialized result-byte totals when those production paths execute.

## Refresh concurrency and overlap

The request model is linear per device. Existing behavior remains unchanged: a 5-second scheduler tick identifies due devices; `RefreshCoordinator` permits at most four concurrent refreshes and one claim per device. A duplicate request for an already-claimed device returns its current snapshot instead of starting another refresh. No profile evidence indicates refresh amplification; the synthetic runner is intentionally sequential, so it validates per-device count scaling rather than scheduler timing.

## Findings

### Confirmed optimization candidate

Each successful device refresh makes six separate SSH executions. This is confirmed by deterministic counts and is the dominant recurring remote-operation multiplier. It is a candidate for WU14-07 only after live baseline evidence establishes its actual cost; no transport consolidation is changed in this work unit.

### Acceptable measured cost

The local fixture orchestration/parsing scales linearly from 2 to 10 devices with no duplicate operation classes. The inactive diagnostic path has no report or aggregation state. No numeric CPU/memory threshold is set.

### WU14-05 investigation

Device Detail currently mounts Activity and Historical surfaces before tabs exist. An interactive Tauri/WebView session with populated history is required to quantify view request counts and range-change behavior. Historical query/downsample instrumentation and typed payload bytes are ready for that evidence.

### WU14-06 investigation

Windows tray/explicit Exit process and shell-icon behavior require a packaged/live Windows session. This baseline does not infer lifecycle correctness from unit tests.
