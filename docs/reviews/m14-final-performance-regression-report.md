# M14 — Final Performance Regression & Closure Report

Date: 2026-08-12  
Evidence scope: deterministic Rust and component tests, fixture-backed refresh profiles, and static Windows bundle/lifecycle inspection. No claim below substitutes fixtures for a live device, installed Windows session, or WebView process measurement.

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
| Windows defects are resolved or precisely evidenced | PASS with accepted residual | Bundle metadata and lifecycle path are verified; current clean installed-session evidence remains operator-only. |
| Before/after evidence and deterministic regression gates exist | PASS | This report and WU14-04 through WU14-07 evidence. |
| Canonical validation passes | PASS | Recorded in the WU14-08 AIQT checkpoint after this report was finalized. |

## Accepted residuals and operator validation

- Current clean-install Start Menu/icon reproduction remains unavailable in the agent environment; static bundle metadata is coherent, and no Desktop-shortcut workaround was added.
- Live Pi-Hub process/resource and SSH transport-duration measurements remain unavailable; fixture timings are not capacity claims.
- WebView2 child-process attribution remains intentionally excluded because no reliable ownership relation is available without heuristics.

Operator checklist for a clean Windows installation:

1. Install the current signed Pi-Hub artifact after removing older Pi-Hub installs; open Start and confirm the `Pi-Hub` entry, product name, and icon.
2. Launch from Start, enable and disable **Minimize to tray**, then close the main window in each state; verify the documented hide/explicit-exit behavior.
3. Use the tray **Exit** command, wait five seconds, and confirm no `Pi-Hub.exe` or managed `ssh.exe` process remains in Task Manager. Ignore a stale notification-area image unless a live process remains.
4. Start one diagnostics session, run one normal device refresh, stop/export the report, and record SSH aggregate duration, Pi-Hub working/private bytes, handles, and the session timestamp with device identities redacted.

M15 remains planned and unstarted.

## Post-closure installed-build correction

Operator evidence confirmed that completed diagnostics sessions work but browser-style report export is not reliable in the installed Windows WebView. The original frontend handler only created a Blob and clicked a temporary anchor; it never invoked Tauri, opened a native dialog, requested a capability, or wrote a file. The installed build could therefore ignore the browser download with no feedback.

The correction replaces that path with the typed `export_performance_benchmark_report` Tauri command. It exports only a stopped, already-redacted report to the native Downloads directory using the existing atomic-write helper, returns the exact saved path, and presents explicit success or failure feedback. No Tauri dialog/filesystem plugin or capability was previously installed, so no new plugin or permission surface was added.

While a benchmark is Running, the Settings panel now shows `Running · mm:ss` with the existing sample count. The elapsed display derives from the active session's existing start timestamp and creates one local one-second timer only while running; it does not poll diagnostics or add inactive diagnostics work. Start clears a prior report, and Stop/unmount/status transition clears the timer.

The correction is covered by native atomic-write, typed-command, success/failure-feedback, running-timer, Stop cleanup, and no-idle-polling component tests. A newly built Windows installer is required for operator retest.
