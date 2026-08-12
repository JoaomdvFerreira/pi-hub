# Pi-Hub — M15 Device Update Intelligence & Maintenance Readiness

Status: Complete — read-only operator validation residual
Target release: v0.4.0, paired with M16 only if implementation evidence supports it  
Predecessor: M14 / v0.3.1  
Primary mode: read-only

M15 adds trustworthy, per-device software-update intelligence. It never changes a remote package state: no index refresh, install, upgrade, removal, repair, lock manipulation, or reboot. The initial release is explicit-check-only and does not attach APT work to the monitoring cadence.

The approved detailed design is [M15 APT read-only evidence contract](../design/m15-apt-read-only-evidence-contract.md). This document owns M15 scope and work sequencing; it does not restate stable M14 architecture or repository-wide safety rules.

## Product placement and query contract

Update intelligence is a distinct **Software Updates** section in Device Detail → **System**. It does not add a top-level tab.

- Selecting another Device Detail tab must not load or execute update work.
- Selecting System may read the locally persisted latest result for that exact device; it must not run a remote command.
- Only an explicit `Check for Updates` action may run the typed remote update-check operation.
- A duplicate action for the same device while one check is in flight is disabled/coalesced; device changes ignore or abort stale completion before it is displayed.
- Normal monitoring, snapshot freshness, scheduler, activity, and performance-session semantics remain unchanged.

## Work units and checkpoints

| Work unit | Depends on | Checkpoint before continuing |
| --- | --- | --- |
| WU15-01 — Update Intelligence Contract & APT Evidence Design | M14 closed | Freeze this evidence contract, deterministic fixture matrix, and real-device plan. No product code. |
| WU15-02 — Read-only Backend & Persistence | WU15-01 | Add only typed fixed operations, parsers, bounded latest-result persistence, instrumentation, and deterministic tests. Verify every operation remains read-only and optional-evidence failures stay partial. |
| WU15-03 — Device Detail Software Updates UI | WU15-02 | Put the cached result and explicit action in System; prove tab/query/action gating and stale-device safety. |
| WU15-04 — Performance, Failure & Real-device Validation | WU15-02, WU15-03 | Measure check cost and result bytes; exercise failure/partial paths and safe Raspberry Pi OS/Debian operator checks. No state is manufactured by mutation. |
| WU15-05 — Regression Gates & Milestone Closure | WU15-04 | Review deterministic gates, risk, docs, AIQT evidence, and canonical validation. M16 remains unstarted. |

## Boundary and handoff

M15 produces intelligence only. M16 may add separately governed, explicit update execution and verification, reusing M15 support detection, package state, persistence shape, and diagnostic labels rather than creating a parallel model.

M15 closed with deterministic regression evidence and canonical validation. Its only residual is a read-only operator check on configured Raspberry Pi OS and Debian devices, documented in the [final closure review](../reviews/m15-final-regression-closure.md). M16 remains planned and unstarted.
