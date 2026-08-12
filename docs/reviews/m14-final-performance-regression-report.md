# M14 — Final Performance Regression & Closure Report

Date: 2026-08-12  
Evidence scope: deterministic Rust and component tests, fixture-backed refresh profiles, static Windows bundle/lifecycle inspection, and operator validation of the final installed Windows 0.3.1 build. No claim below substitutes fixtures for live transport-duration measurement or WebView child-process attribution.

## Deterministic regression gates

| Proven behavior | Gate | Evidence |
| --- | --- | --- |
| Inactive diagnostics fast path and bounded session/report retention | `inactive_measurement_has_no_report`, `session_is_single_and_bounded`, and `operation_aggregates_without_events` | Inactive measurement creates no report; a second session is rejected; sample retention stops at the configured cap; aggregates retain totals rather than events. |
| Synthetic refresh scaling | `deterministic_profiles_produce_linear_refresh_request_counts` | 2/5/10 fixture devices produce 12/30/60 SSH executions, zero SSH failures, and 1,080/2,700/5,400 safe result bytes. |
| Device Detail query gating | `DeviceDetailScreen query gating` and `useDeviceActivity` inactive-path test | Overview mounts neither Historical nor Activity work; Monitoring mounts Historical only; Activity mounts persisted Activity only. |
| Settled Historical query identity | `HistoricalTrends` settled-rerender test | An unchanged inline production trend configuration creates no additional query after settlement. |
| Historical point/result bounds | `retention_and_downsampling_are_bounded` | Retention removes expired samples and numeric series are capped at `MAX_HISTORY_POINTS` (500). |
| Refresh overlap protection | `try_claim_prevents_a_duplicate_in_progress_refresh` | A device cannot claim a second concurrent refresh; release restores eligibility. |

The accepted refresh model remains six independently bounded SSH executions for each successful device refresh. This work unit adds no consolidation, cadence, or scheduler change.

## Final before/after findings

| Area | Before | Final evidence |
| --- | --- | --- |
| Backend refresh | Six SSH executions per successful fixture device; no duplicate operation class or refresh amplification. | Unchanged: 2/5/10 devices remain 12/30/60 SSH executions and zero failures. Result-byte accounting is now deterministic and linear at 1,080/2,700/5,400 B. Fixture duration remains below aggregate millisecond resolution. |
| Device Detail default load | Activity plus five device Historical requests, and two service Historical requests per enabled service, mounted before an operator selected those surfaces. | Overview makes zero view-specific Activity/Historical requests. Monitoring makes five device Historical requests for its current range; Activity makes one exact-device Activity request; Services makes two service-history requests only for enabled services. |
| Historical behavior | Long-range layout and query containment were not protected by the new tab model. | Card/grid/chart boundaries contain long series; an unchanged settled render adds zero queries; a range switch issues one query per rendered metric; history remains bounded to 30 days and 500 numeric points. |
| Diagnostics idle state | No always-on diagnostics behavior was permitted. | Inactive measurement creates no report or sampler work. Active reports retain bounded samples and aggregates only. |
| Runtime optimization review | Six SSH operations were the confirmed recurring candidate. | No transport change accepted: batching, reuse, probe removal, multiplexing, and collector parallelism lacked a safe measured benefit. Active-only SSH result-byte accounting was added for evidence, not as a transport optimization. |

## M14 exit-criteria assessment

| Criterion | Assessment | Evidence |
| --- | --- | --- |
| In-app diagnostics and reusable decoupled core work | PASS | Isolated core tests, typed command boundary tests, and Settings diagnostics surface. |
| Deterministic local scenarios run without VMs/Pis | PASS | Fixture-backed 2/5/10 profiles. |
| Idle/active and recurring-request evidence is documented | PASS | WU14-04/WU14-07 reports and the gates above. |
| Device Detail tabs and query gates work | PASS | Component gating, Activity identity, and Historical request tests. |
| Historical charts are contained | PASS | Long-series containment regression. |
| Windows defects are resolved or precisely evidenced | PASS | Final installed Windows 0.3.1 validation passed Start Menu/icon, diagnostics timer/export/CPU, and close/minimize-to-tray/Exit checks. |
| Before/after evidence and deterministic regression gates exist | PASS | This report and WU14-04 through WU14-07 evidence. |
| Canonical validation passes | PASS | Recorded in the WU14-08 AIQT checkpoint after this report was finalized. |

## Accepted residuals and operator validation

- Final installed Windows 0.3.1 operator validation passed installer/version, diagnostics Running state/timer, Windows CPU sampling, native JSON export, trusted/redacted benchmark metadata, Activity device-name labels, exact filtering, deleted-device fallback, Start Menu/icon, close/minimize-to-tray/Exit lifecycle, Device Detail tabs, and Historical 7d/30d containment and resize smoke.
- Live SSH transport-duration measurements remain unavailable; fixture timings are not capacity claims. This is an evidence limitation, not a release blocker.
- WebView2 child-process attribution remains intentionally excluded because no reliable ownership relation is available without heuristics.

The operator checklist is complete for the final 0.3.1 installer:

1. Install the current signed Pi-Hub artifact after removing older Pi-Hub installs; open Start and confirm the `Pi-Hub` entry, product name, and icon.
2. Launch from Start, enable and disable **Minimize to tray**, then close the main window in each state; verify the documented hide/explicit-exit behavior.
3. Use the tray **Exit** command, wait five seconds, and confirm no `Pi-Hub.exe` or managed `ssh.exe` process remains in Task Manager. Ignore a stale notification-area image unless a live process remains.
4. Start one diagnostics session, run one normal device refresh, stop/export the report, and record SSH aggregate duration, Pi-Hub working/private bytes, handles, and the session timestamp with device identities redacted.

All requested checks passed. M14 has no remaining release-readiness blockers.

M15 remains planned and unstarted.

## Post-closure installed-build correction

Operator evidence confirmed that completed diagnostics sessions work but browser-style report export is not reliable in the installed Windows WebView. The original frontend handler only created a Blob and clicked a temporary anchor; it never invoked Tauri, opened a native dialog, requested a capability, or wrote a file. The installed build could therefore ignore the browser download with no feedback.

The correction replaces that path with the typed `export_performance_benchmark_report` Tauri command. It exports only a stopped, already-redacted report to the native Downloads directory using the existing atomic-write helper, returns the exact saved path, and presents explicit success or failure feedback. No Tauri dialog/filesystem plugin or capability was previously installed, so no new plugin or permission surface was added.

While a benchmark is Running, the Settings panel now shows `Running · mm:ss` with the existing sample count. The elapsed display derives from the active session's existing start timestamp and creates one local one-second timer only while running; it does not poll diagnostics or add inactive diagnostics work. Start clears a prior report, and Stop/unmount/status transition clears the timer.

The correction is covered by native atomic-write, typed-command, success/failure-feedback, running-timer, Stop cleanup, and no-idle-polling component tests. A newly built Windows installer is required for operator retest.

## Post-closure Windows CPU measurement correction

Installed-build operator evidence found valid memory, private-memory, handle, and operation samples while `processCpuPercent` was always `null`. The cause was explicit: the Windows sampler returned `None` for that optional field and had no process-timing collection.

The sampler now reads the current Pi-Hub process's Windows kernel and user CPU times with `GetProcessTimes`. It derives usage from successive timing samples and wall-time samples, then normalizes by the active logical-processor count: `100%` means the Pi-Hub process consumed all logical-processor capacity during that interval; a process using one fully busy core on a four-logical-processor system reports `25%`. The first sample, failed timing read, non-monotonic time, zero elapsed interval, unavailable processor count, and out-of-range result remain unavailable rather than fabricated. This does not attribute WebView2 child processes, add polling, alter the report schema, or create sampler work while diagnostics are inactive.

## Post-closure Activity device-selector correction

Installed-build operator validation found that the Activity device selector derived its options from persisted activity IDs and rendered those IDs directly, exposing raw UUIDs. It now resolves each activity device ID against the current inventory for the option label while retaining the exact ID as the option value and filter key; `All devices` is unchanged. Activity for a device no longer present in inventory remains selectable as `Deleted device`, rather than disappearing or exposing its UUID.

Focused component coverage proves inventory-name labels, exact-ID filtering, the unchanged all-devices option, and the deleted-device fallback. A newly built Windows installer should be checked in Activity to confirm current inventory names and the deleted-device label in the selector.

## Pre-merge benchmark export privacy correction

Pre-merge review found that `BenchmarkConfig` accepted free-form `name` and `scenario` strings and serialized them into session and export metadata. The command now accepts no configuration from the frontend, and the core uses a closed `Manual`/`Synthetic` scenario enum with trusted derived display names. The report retains its bounded timing and configuration fields but contains no caller-supplied benchmark text.

The core regression submits credential-like JSON metadata and proves deserialization rejects it, then serializes an exported report and proves the secret is absent while the trusted `Manual`/`manual` values remain. The typed frontend boundary proves the start command has no configuration payload. This resolves the export-privacy blocker. Final installed-build validation passed the CPU, export, redaction, and Activity selector checks. The final M14 risk is **24/100 — Green**.
