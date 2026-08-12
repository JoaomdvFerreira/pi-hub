# Pi-Hub — M14 Performance Benchmarking, Runtime Hardening & Device Detail UX

Status: In progress  
Target release: v0.3.1  
Baseline: Pi-Hub v0.3.0  
Risk: Medium

## Objective

M14 makes performance measurable before further background maintenance work. It delivers an opt-in in-application Performance Diagnostics capability, a reusable Rust benchmark core, deterministic local scenarios, evidence-led runtime hardening, tab-scoped Device Detail loading, the Historical Trends containment fix, and evidenced Windows Start/tray lifecycle remediation.

The detailed contracts are [Performance Diagnostics architecture](m14-performance-diagnostics-architecture.md) and [Device Detail tabs UX and query contract](m14-device-detail-tabs-ux-query-contract.md). WU14-01 freezes the implementation design in [the baseline plan](../design/m14-performance-contract-baseline.md).

## Governing principles

- Diagnostics are opt-in, bounded, in-memory by default, and have negligible inactive overhead.
- The reusable Rust core has no Pi-Hub domain, SSH, Docker, or Tauri dependency.
- Production abstractions with fixtures support local deterministic scenarios; no VM or live Pi is required.
- Optimize measured hot paths rather than theoretical ones. CI protects deterministic counts and bounds, not arbitrary machine CPU or memory thresholds.
- Device Detail tabs gate expensive view-specific work; they do not change background monitoring or service/container semantics.
- No arbitrary command API, credential storage, or weaker SSH host-key verification is in scope.

## Known defects

1. Device Detail is crowded and currently loads multiple detail surfaces together.
2. Historical 7d/30d charts may overflow or overlap their card.
3. Installed/Start icon discoverability needs Windows packaging evidence and correction.
4. Close/explicit Exit tray and process lifecycle needs evidence and hardening.

## Work units

| Work unit | Dependency | Outcome |
| --- | --- | --- |
| WU14-01 | None | Contract, baseline design, runtime and query ownership inventory. |
| WU14-02 | WU14-01 | Reusable core, sampler, typed API, minimum diagnostics UI. |
| WU14-03 | WU14-02 | Pi-Hub instrumentation and deterministic fixture workloads. |
| WU14-04 | WU14-03 | Pre-optimization baseline evidence and request/backend audit. |
| WU14-05 | WU14-04 | Device tabs, gated loading, and Historical containment fix. |
| WU14-06 | WU14-04 | Windows launch/icon and tray lifecycle hardening. |
| WU14-07 | WU14-05, WU14-06 | Remaining evidence-backed runtime optimization. |
| WU14-08 | WU14-07 | Regression gates, final report, validation, and closure. |

## Exit criteria

M14 closes only when in-app diagnostics and a reusable decoupled core work; deterministic local scenarios run without VMs/Pis; idle/active and recurring-request evidence is documented; Device Detail implements the approved tabs and query gates; Historical charts are contained; Windows defects are resolved or precisely evidenced; before/after evidence and deterministic regression gates exist; and canonical validation passes. M15 Device Update Intelligence must remain unstarted until this milestone closes.
