# Pi-Hub Post-Roadmap Product and Release Readiness Review

Date: 2026-08-10  
Scope: merged M1-M13 baseline on `main`  
Source of truth: approved post-roadmap product and release readiness review

## Executive summary

- Release readiness: **NOT READY**
- Overall release risk: **65/100 - Orange (50-74)**
- Release Blockers: **1**
- High-priority defects: **0**
- Post-release improvements: **1**
- Future capabilities: **0**
- Recommendation: **fix named blockers then release**.

The merged M13 baseline is otherwise coherent and has strong static and automated evidence: typed, bounded backend operations; mandatory SSH host-key verification; separated and bounded persistence; failure isolation; 205 Rust tests; 14 frontend tests; and successful Rust and production-frontend builds. However, every historical-trends caller passes a new `trends` array on each render, while `HistoricalTrends` treats that array identity as an effect dependency and commits a fresh state object after every query. This forms an unbounded renderer/query loop on device, service, and container detail views. It makes a promised M12 workflow operationally unsafe to release.

## Baseline reviewed

| Item | Evidence |
| --- | --- |
| Branch and commit | Clean `main` at `11201cff9869f23cd4ae02e9daff34589f3ef979` (`Merge pull request #9 from .../milestone/m13-document-hygiene`) |
| M13 relationship | M13 closure commit `7b4acee` is an ancestor of `main`; local `origin/main` resolves to the same merge commit |
| Product version | `0.2.0` in `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml` |
| Documentation baseline | README, current product contract, roadmap, M10-M13 records, and release procedure were reviewed |
| Automated validation | `cargo check`, `cargo test`, `cargo build`, `npm test`, `npm run build`, `npm run lint`, `aiqt status`, and `git diff --check` completed successfully; lint reported four existing Fast Refresh warnings and no errors |
| Test inventory | 205 Rust unit tests listed by Cargo; 5 frontend test files / 14 tests passed |
| Published release evidence | Read-only GitHub release inspection found signed `v0.2.0` NSIS/MSI installers, `.sig` files, and `latest.json` |
| Review limits | No real device, destructive administration, clean-install, or updater migration was executed. `git fetch --prune origin` could not write `.git/FETCH_HEAD` in this environment; local `origin/main` already matched `HEAD`. |

The initial canonical validator invocation stopped at frontend-test startup because the filesystem sandbox prevented esbuild from reading above the workspace. Running the affected frontend test and production-build commands with the needed filesystem access succeeded. This is an environment restriction, not a product test failure.

## Domain assessments

### R1 - Product completeness and core workflows: PASS WITH FINDINGS

The UI exposes registration, SSH test/diagnostics, dashboard health, services, Docker details/actions, network/storage/system visibility, Activity, Alerts, history, and the four controlled administration actions. Unknown and unavailable states are rendered explicitly. Destructive actions use confirmation dialogs. Finding PRRR-001 prevents the historical-monitoring part of the detail workflow from being releasable.

### R2 - Cross-feature integration and state consistency: PASS WITH FINDINGS

The refresh scheduler persists snapshots, writes history best-effort, evaluates governed alert transitions before notification dispatch, records Activity separately, and limits planned-offline suppression to new device-offline alerts. Administration markers expire or clear on confirmed online recovery. Per-device claims and a four-refresh semaphore isolate refreshes. PRRR-001 is a cross-feature UI/history integration failure.

### R3 - Security and privilege boundaries: PASS

OpenSSH uses batch mode, separate process arguments, bounded timeouts, and explicit host-key-error classification. Remote operations are a closed catalogue; Docker identifiers are character-validated before remote command construction; Docker logs/actions are bounded; and administration uses only fixed `sudo -n systemctl` literals. No arbitrary frontend command surface or persisted password/private-key/sudo-secret path was found.

### R4 - Reliability, failure isolation, and recovery: PASS

Probe, metrics, Docker, and visibility collection have explicit bounds and preserve unavailable/unknown states. Refreshes run off the UI runtime; each device is independently scheduled and a failure does not stop other devices. Malformed remote Docker/visibility data is isolated, historical persistence is best-effort, and corrupt local files fall back safely. M10 exposes outcome-uncertain and timed-out states with bounded verification instead of claiming success.

### R5 - Persistence and data integrity: PASS

Device, snapshot, settings, Activity, alerts, administration markers, and history have separated JSON repositories and atomic writes. Activity is retained for 30 days / 2,000 events; resolved alerts are bounded; history has 30-day pruning, per-entity sampling no more often than 60 seconds, 500-point downsampling, and corrupt-file fallback. IDs for device/service/container samples are stable persisted/observed identifiers. No cross-domain pruning was found.

### R6 - Performance and SSH efficiency: FAIL

Collection combines system metrics and identity in one SSH operation, uses bounded concurrency, and limits output/log sizes. However, PRRR-001 creates unbounded local history queries and frontend rerenders whenever a historical detail section is mounted. This is a concrete, not speculative, performance defect.

### R7 - Test and regression confidence: PASS WITH FINDINGS

Rust coverage directly exercises parsers, host-key classification, health, alert lifecycle/suppression, notification governance, Docker validation/parsing, administration failure mapping, persistence, visibility parsing, and history retention/downsampling. Frontend tests cover critical state rendering. PRRR-002 records the missing caller-level regression case that allowed PRRR-001 through: the component test uses a stable module-level trend array and does not exercise production callers that allocate it inline.

### R8 - UX and operator clarity: PASS WITH FINDINGS

The navigation hierarchy separates dashboard, services, Activity, Alerts, and settings. Health diagnostics, unavailable visibility, alert state/severity, administration confirmation, Power On exclusion, and empty/error history states are clear in code and frontend tests. PRRR-001 makes the otherwise clear historical-trends UI unusable after data arrives.

### R9 - Documentation and product truth: PASS

README, roadmap, current product contract, M13 record, and release procedure agree on M1-M13 scope, SSH/security boundaries, controlled administration, M12 limits, and unsupported Power On. AIQT reports a valid status but names legacy M005 as current; M13 explicitly documents retained historical duplicate planning entries and assigns current human-readable status to the roadmap. This is known workflow-history debt, not a release-facing contradiction.

### R10 - Build, packaging, installation, and release mechanics: NOT FULLY VALIDATED

The Tauri configuration has Windows bundle targets, updater artifacts, an embedded updater public key, and the expected GitHub `latest.json` endpoint. The signing-key path exists locally. Published `v0.2.0` evidence includes signed NSIS/MSI artifacts and `latest.json`. That tag predates all M6-M13 commits, though the working tree still declares `0.2.0`; a new release cannot reuse that tag/version. Current-baseline signed packaging, clean installation, and an in-app update remain required release validation.

### R11 - Residual manual/live validation: NOT FULLY VALIDATED

No live device was in review scope. Safe pre-release validation remains appropriate for: one representative M10 restart/shutdown outcome path on a non-critical device, M11 collection against a supported Linux host, M12 sample accumulation over time, fresh installer installation, and update from `v0.2.0`. These are release conditions, not separate product defects; no disruptive operation was executed here.

### R12 - Release readiness and release plan: FAIL

PRRR-001 requires remediation before release. Once fixed and validated, the repository history supports a minor `v0.3.0` release rather than reusing `v0.2.0`.

## Findings register

| ID | Class | Severity / risk contribution | Domain | Release-blocking |
| --- | --- | --- | --- | --- |
| PRRR-001 | Release Blocker | Critical / 55 | R1, R2, R6, R8, R12 | Yes |
| PRRR-002 | Post-release improvement | Low / 10 | R7 | No |

### PRRR-001 - Historical-trends detail views enter an unbounded query/rerender loop

- **Evidence:** `HistoricalTrends` includes `trends` in its `useEffect` dependency array and calls `setSeries(Object.fromEntries(items))` after each query (`src/features/monitoring/HistoricalTrends.tsx:8`). The component is called with inline array literals from device detail (`DeviceDetailScreen.tsx:386`), every enabled service card (`DeviceDetailScreen.tsx:503`), and container detail (`ContainerDetailDialog.tsx:29`). Each render therefore changes `trends`; each completed effect creates fresh state and causes the next render/effect cycle.
- **Impact:** Opening a device, service, or container detail with historical monitoring continuously issues bounded local history queries and rerenders the renderer. The effect is not SSH-bound, but it can consume CPU and make a core operator-detail workflow unresponsive; more active service cards multiply the work.
- **Recommendation:** Stabilize the trend definitions/effect dependency relationship and add regression coverage that mounts the real call pattern and asserts no repeated queries after data resolves.
- **Release-blocking:** Yes. M12 history is a promised, reachable baseline capability and the defect is self-sustaining in normal production use.

### PRRR-002 - Frontend history regression coverage does not model production caller identity

- **Evidence:** `HistoricalTrends.test.tsx` passes a module-level `trends` constant, while production callers use inline arrays. The current 14 frontend tests pass but do not assert that history querying stabilizes after the first successful render.
- **Impact:** A small dependency-identity regression can produce repeated local backend requests without being detected.
- **Recommendation:** Add a caller-level test after PRRR-001 is fixed that verifies a settled detail/history render does not issue further history queries until a device/entity/range change.
- **Release-blocking:** No. This is test-strengthening work; it does not replace remediation of PRRR-001.

## Release conditions

1. Fix PRRR-001 and run focused regression coverage demonstrating stable history querying from device, service, and container detail call sites.
2. Run the canonical validator again from an environment where all of its child commands have required filesystem access; retain the green output.
3. Bump all three version authorities from `0.2.0` to `0.3.0` and build signed Windows artifacts. `v0.2.0` already exists and is earlier than the delivered M6-M13 scope.
4. Perform non-destructive release acceptance: clean install of the signed installer and updater verification from a `v0.2.0` installation using the published `latest.json`/signature path.
5. Before publication, perform bounded representative live validation of M10, M11, and M12 on a non-critical device. Do not use production-critical infrastructure for restart/shutdown validation.

## Proposed release plan

- **Proposed version/tag:** `0.3.0` / `v0.3.0` (minor: M6-M12 capability additions are not part of published `v0.2.0`; M13 is documentation reconciliation on top of that baseline).
- **Pre-release validation:** all release conditions above, plus inspect generated NSIS/MSI filenames, signatures, and `latest.json` version/URLs before uploading.
- **Artifact expectations:** attach the signed NSIS installer, signed MSI if distributed, their updater signatures, and exactly one `latest.json` that references the intended Windows updater artifact.
- **Release-note limitations:** agentless SSH over Tailscale/LAN only; verified host keys; no stored credentials; no arbitrary remote commands; Power On unsupported; M11 is read-only; history is local, 30-day, 60-second-minimum sampled, and query-bounded.
- **Post-release dogfooding priorities:** real-device M10 verification behavior, M11 partial-data rendering, M12 trend accumulation/gaps, and installer-to-updater migration.

## Residual risks

### Accepted residual risk

- The review was static plus automated-local validation. It did not and must not perform disruptive device administration.
- The current product has no broad end-to-end device-lab suite; the specified non-critical-device acceptance checks should be retained as release evidence.
- Existing lint has four Fast Refresh warnings, with no lint errors. They do not affect the production bundle and are not release blockers.

### Future ideas

No missing capability was classified as a defect. In particular, Power On, Wake-on-LAN, arbitrary service/container/host administration, Compose editing, historical export, and expanded observability remain out of scope.

## Final recommendation

**FIX BLOCKERS THEN RELEASE**

