# M16 WU16-05 — Operator Validation and Recovery Review

Date: 2026-08-14  
AIQT work unit: WU098  
Scope: deterministic recovery/performance regression evidence and explicitly authorized real-device validation only.

## Authorized real-device evidence

| Device | Authorized action | Result |
| --- | --- | --- |
| PI 2 | PREPARE then Cancel | Passed without mutation. |
| PI 2 | One fresh 34-package APPLY | Completed successfully with exactly one systemd transient-unit dispatch. `dpkg --audit` was clean; M15 verification found zero pending and zero deferred packages; no reboot was required. SSH, Tailscale, NTFY, Home Assistant, Docker/container, and System health checks passed. Terminal maintenance state, Activity, and expected-disruption cleanup passed. |
| PI 5 | One real APPLY | Completed successfully with 162 packages updated. Five packages remained deferred by the conservative `apt-get upgrade` contract. Terminal state persisted and the post-update device remained healthy. Deferred-only presentation now states that no standard updates are available and the five packages require manual review. |

No further package mutation, retry, repair, rollback, reboot, or manual package-management command was used to create evidence.

## Operator-discovered corrections

| Correction | Evidence / regression coverage | Commit |
| --- | --- | --- |
| A transport failure before the dispatch boundary is definitely-not-started, not an uncertain update outcome. | Pre-dispatch failure stage, `notAttempted` persistence, no Activity/expected disruption, and no automatic retry are deterministic assertions. | `eeb5f54` |
| Completed-unit recovery runs the frozen terminal verifier; verifier failure becomes terminal rather than remaining stuck. Manual status after an expired window still observes the known unit; numeric `ExecMainCode=1` is accepted as `CLD_EXITED`; stale `stillRunning` state normalizes terminally. | Fake executor covers recovery verification, known unit observation without redispatch, expiry/manual observation, and both systemd exit-code representations. | `b7abb29` |
| Rejected explicit M15 check requests are shown as actionable UI feedback instead of being swallowed. | Frontend test covers terminal M16 state, coordinator rejection, and cached-result preservation. | `a5e1ae1` |
| Deferred-only evidence is manual-review-only rather than directly installable standard updates; successful Check again refreshes Last checked. | Frontend test covers zero normal/five deferred, hidden Update device action, manual-review wording, and refreshed timestamp. | `0861b8d` |
| Confirmation package review closes the dialog to make the bounded list usable and retains the prepared review path. | Frontend confirmation/View packages regression coverage. | `eeb5f54` |

## Deterministic closure gates

| Gate | Result |
| --- | --- |
| Rust deterministic maintenance, M10/M15 coordination, persistence, execution/recovery, expected-disruption, and diagnostics tests | PASS — 258 passed, 1 ignored. |
| Frontend System Updates and Device Detail behavior | PASS — 12 files, 48 tests passed. |
| Frontend production type/build | PASS. |
| Lint | PASS with four existing Fast Refresh warnings and no errors. The initial-load effect was moved to a microtask so the current React hook rule is satisfied without changing timer-based recovery behavior. |

## Explicit residuals

- A live app restart while an APPLY was actively running was not exercised. Deterministic persisted-operation/recovery coverage exists.
- Performance Diagnostics was not started before either real APPLY. Deterministic label/bounds coverage exists; no retrospective diagnostic export is claimed.

Neither residual warrants another package update. They remain operator follow-up evidence for release review, not an automatic or destructive validation task.

## Risk assessment

**28/100 — Yellow (low-medium residual risk).** The fixed command surface, per-device coordinator, pre-dispatch persistence boundary, one-dispatch rule, terminal verification, and bounded recovery behavior now have deterministic coverage plus successful authorized updates on both available device classes. Remaining risk is limited to the unexercised live app-restart-during-APPLY and diagnostics-capture paths; both are explicitly disclosed. No merge-blocking safety contradiction is known.
