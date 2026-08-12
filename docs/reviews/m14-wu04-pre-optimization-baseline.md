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

### WU14-05 result — frontend request model

Before the tab refactor, Device Detail mounted the persisted Activity hook and device Historical Trends on every detail load; configured service history also mounted with the single page. The deterministic pre-refactor request model was therefore one Activity request plus five device Historical requests, plus two Historical requests for every enabled service, before an operator selected any of those surfaces.

After the refactor, default Overview mounts neither `useDeviceActivity` nor `HistoricalTrends`: detail load has zero view-specific Activity/Historical Tauri requests. Selecting Monitoring issues exactly five device-history requests for its current range; selecting Activity issues exactly one exact-device Activity request. Switching to Services can mount two history requests per enabled service, while Containers and System introduce no view-specific typed request. The existing Historical metric-configuration identity test proves settled rerenders add zero requests; a range switch issues one request per currently rendered metric only. This is deterministic component evidence, not a live WebView timing claim.

Historical cards now use `min-width: 0` and `overflow-hidden` containment at every card/grid/chart boundary; range controls scroll within the card rather than causing page-level overflow. The representative long-series regression renders 160 points and asserts that containment boundary.

### WU14-06 investigation

The current NSIS/MSI artifact inventory contains the signed `Pi-Hub_0.3.0_x64` installers. The local Tauri 2.11.3 schema confirms that NSIS creates a Start Menu shortcut under `%AppData%\\Microsoft\\Windows\\Start Menu\\Programs\\Pi-Hub.lnk` when `startMenuFolder` is unset. The bundle has an explicit product name, identifier, publisher, and ICO source, so no Desktop shortcut is added. An interactive installed-session launch was not available in this WU; the reported discoverability symptom is therefore recorded as a likely stale or older installation issue rather than attributed to current package metadata.

The application-controlled tray defect was reproduced by inspection: `CloseRequested` always hid the window, ignoring the persisted `minimizeToTray` preference. The close path now reads that setting: enabled prevents the close and keeps the process, tray, and scheduler alive; disabled delegates to the same deliberate exit path as the tray Exit command. Deliberate exit closes managed PTY sessions, removes `main-tray` before `app.exit(0)`, and then terminates the backend. A stale Windows notification-area image after process termination is a Shell cache artifact, not evidence of an orphan Pi-Hub process; no live installed session was available to measure its duration or process resources.

### WU14-07 review — evidence-based runtime optimization

#### Candidates reviewed

| Candidate | Evidence | Decision |
| --- | --- | --- |
| Combine the six successful-refresh SSH operations into one transport session | The 2/5/10 profiles prove six `ssh.execute` calls per online device; the separate metrics, Docker, network, storage, and system collectors each retain their own parser, warning, fallback, and timeout. | Rejected. A shared command timeout or transport failure would hide independent collector outcomes; preserving separate remote limits inside a batch would add a new shell orchestration contract without deterministic evidence of a safe gain. |
| Reuse metrics data for System visibility | Metrics and System overlap only for a subset of fields. System still owns CPU model and logical-core collection, and its failed collection currently falls back independently. | Rejected. It would either remove currently supplied typed fields or couple the two failure domains. |
| Remove the connectivity probe | Probe is the only operation bounded by the five-second connection timeout and classifies connection, authentication, and host-key errors before collector work. | Rejected. Replacing it with a ten-second metrics request weakens the offline bound and changes classification timing. |
| Enable OpenSSH connection multiplexing | Current executions retain independent process timeouts and host-key verification for every operation. | Rejected. It introduces platform-specific control-path/session cleanup and can defer changed-host-key detection while a master is live; no live transport evidence justifies that security-sensitive change. |
| Run post-probe collectors concurrently | The fixture profiles finish below millisecond resolution; no duration evidence demonstrates a gain. It would increase one device from one to five simultaneous SSH processes. | Rejected. The extra per-device concurrency is not supported by current capacity evidence. |

No product runtime optimization was accepted in WU14-07. Preserving the existing independent request model is the low-risk result. The only accepted change is diagnostics evidence correction: successful SSH executions now record aggregate stdout/stderr byte length while diagnostics are active. It has no inactive-path work and does not retain payload content.

#### Deterministic before/after profiles

The same fixed fixture payload is 90 bytes per successful SSH result. The profiles remain local, sequential, and below aggregate millisecond resolution; duration is therefore reported as `0 ms`, not treated as a performance threshold.

| Profile | Before SSH / failures / result bytes / duration | After SSH / failures / result bytes / duration |
| --- | --- | --- |
| 2 devices | 12 / 0 / unmeasured / 0 ms | 12 / 0 / 1,080 B / 0 ms |
| 5 devices | 30 / 0 / unmeasured / 0 ms | 30 / 0 / 2,700 B / 0 ms |
| 10 devices | 60 / 0 / unmeasured / 0 ms | 60 / 0 / 5,400 B / 0 ms |

`monitoring.device_refresh`, Docker, Network, Storage, and System collection counts remain one per device. SSH count and result bytes both scale linearly: `6 × devices` and `540 × devices` bytes. The regression profile test asserts those counts, zero SSH failures, and the byte total for all three sizes.

#### Regression coverage

- `deterministic_profiles_produce_linear_refresh_request_counts` preserves the 2/5/10 request model and now proves successful-result byte accounting and zero SSH failures.
- `synthetic_refresh_aggregates_only_stable_labels_when_active` preserves active-only bounded labels; inactive diagnostics still create no measurement state.
- Existing refresh tests continue to cover failed-probe stale snapshots and independently isolated Docker/visibility failures.
