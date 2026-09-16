# M17 — Managed Workload Deployment Actions

Status: Complete — WU17-01 through WU17-07 closed; final evidence: [M17 final review and release readiness](../reviews/m17-final-review-release-readiness.md).

M17 deploys only explicitly registered workloads through a typed, review-and-confirm flow. It is not a generic SSH, shell, Docker, Git, secret, retry, rollback, scheduler, fleet, OS-update, or root-execution surface. The initial dogfood target is Personal Finance on PI 5.

## Canonical WU17-01 decisions

The durable contract is [M17 Managed Workload Action Safety Contract](../design/m17-managed-workload-action-safety-contract.md). M17 extends—not duplicates—M16's bounded `RemoteExecutor`, transient-unit persistence and full result classification, one-shot reconciliation, raw journal discard, 5-second UI-owned non-overlapping cadence, and persisted 60-minute observation deadline. Expiry never kills a remote unit; restart resumes the same nonterminal dispatched operation and never redispatches.

`DeviceMaintenanceCoordinator` remains the sole same-device owner. M17 adds a conflicting `WorkloadDeploy` kind; M10/M15/M16/M17 all conflict per device but devices remain independent. PREPARE releases its claim before review; APPLY reacquires and repeats trust/PREPARE.

The fixed root is `/usr/local/libexec/pi-hub/actions`; `actionId` is `^[a-z0-9][a-z0-9-]{0,63}$`. The backend derives the path. Before PREPARE and immediately before APPLY it proves `/usr/local/libexec/pi-hub` and `actions` are root-owned directories not group/world writable; the derived entrypoint is a root-owned executable regular non-symlink file not group/world writable; and captures SHA-256. Drift is `PlanChanged` and causes no dispatch.

The future versioned `ManagedWorkload` configuration contains only `id`, `deviceId`, name, `actionId`, optional stable `composeProject`, bounded required Compose services, bounded existing Service Health IDs, `enabled`, and config revision. No paths, shell, Git refs, credentials, secrets, environment, Docker context, or executable is configurable. At least one independent Compose or Service Health target is required.

Protocol v1 allows only fixed action argv `prepare`, `apply <target>`, and `verify <target>`. PREPARE/VERIFY are 15 seconds, 32 KiB stdout and 16 KiB stderr; stdout is a single typed JSON object and all raw output is discarded. PREPARE returns only v1 `ready`, `upToDate`, or fixed-enum `blocked`; ready carries canonical lowercase 40/64-hex current/target revisions and bounded count. VERIFY returns v1 `ok` and exact deployed revision. SHA-256 consent fingerprint v1 uses length-prefixed protocol/config/action/digest/current/target/Compose/Service Health fields. APPLY repeats trust and PREPARE; any difference is terminal `PlanChanged`, requiring fresh review.

M17 uses backend-only `pihub-workload-<32 lowercase hex>.service` and persists operation/workload/device IDs, revision, configuration/action digest/fingerprint versions, unit ID, dispatch state, times, and deadline before its one dispatch. Exact dispatch is `sudo -n systemd-run --unit=<unit> --service-type=exec --property=RemainAfterExit=yes --property=User=<configured-device-ssh-user> -- <derived-action> apply <validated-target>`. It has no shell, PTY, `--wait`, `--pipe`, frontend environment, or runtime kill limit. Fixed `sudo -n` status/journal/reset-failed operations are separate typed constructors, never a privileged command API. The action runs as the normal device user; root actions are unsupported. `DispatchUncertain` only follows an attempted dispatch and reconciles the known unit without retry.

`Completed` requires successful unit result, action VERIFY exact target, durable Compose project/service-label presence/running state (and healthy when Docker reports health), plus every declared existing Service Health check passing in bounded non-overlapping attempts for up to 120 seconds. Otherwise it is `VerificationFailed`. Activity is initiated plus exactly one completed/failed terminal event. Static diagnostics labels are `workload_deploy.prepare`, `.trust_check`, `.dispatch`, `.reconcile`, `.verify_action`, `.verify_docker`, `.verify_service`, `.persist`. Workload names, IDs, revisions, paths, containers, URLs, secrets, and raw output are excluded. Current device-wide disruption cannot safely scope workloads, so WU17-04 must implement scoped behavior or no suppression.

No Personal Finance updater/action was available in this checkout or supplied material for safe read-only inspection. WU17-01 makes no claim about its behavior and does not execute, install, modify, chmod, or chown it. A root-owned fixed `personal-finance` adapter must own fixed repo/ref/Compose settings and local credentials; implement v1; reject dirty/diverged state without clean/reset/merge/rebase; pin APPLY to PREPARE's object; keep PREPARE non-mutating; avoid unrelated mutation/OS update/reboot/Docker-daemon restart/rollback; and not expose secrets.

WU17-02–06 require deterministic coverage for grammar/trust/digest drift, protocol bounds/parser failures, dirty/diverged blocks, fingerprint drift, coordinator conflicts, one dispatch, uncertainty/no retry, recovery/expiry/no kill, verification mismatches, raw-output exclusion, and scoped disruption. No live deployment before WU17-06; stages are trust inspection, real PREPARE+Cancel, review, explicitly approved one APPLY, independent verification, then only a naturally available later deployment for recovery evidence.

## Work units

1. WU17-01 — Managed Action Contract & Personal-Finance Adapter Design — complete.
2. WU17-02 — Managed Workload Registry, Persistence & Trust Validation — complete.
3. WU17-03 — PREPARE, Fingerprint & Detached Deployment Execution — complete.
4. WU17-04 — Verification, Recovery, Activity & Scoped Disruption — complete.
5. WU17-05 — Managed Workload UX — complete.
6. WU17-06 — Deterministic & Real-Workload Operator Validation — complete.
7. WU17-07 — Regression Gates & Milestone Closure — complete.
