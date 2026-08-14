# M16 — Final Regression and Closure Review

## Adversarial safety review

| Claim | Assessment | Evidence |
| --- | --- | --- |
| No full-upgrade, dist-upgrade, autoremove, add/remove path | PASS | Production-source audit finds none. The only mutation argv is a fixed `apt-get … upgrade` with `--no-remove`; the preview rejects new/removal counts. |
| No arbitrary APT, systemd, or shell input | PASS | React invokes only typed Tauri commands. Backend builds fixed commands and accepts only backend-generated `pihub-update-<32 lowercase hex>.service` unit IDs. |
| Non-interactive privilege and single-dispatch boundary | PASS | Every privileged M16 command uses `sudo -n`; APPLY persists `Dispatching` before the one `systemd-run` attempt and returns the existing operation on any later APPLY. |
| No PlanChanged continuation or automatic redispatch | PASS | Final simulation fingerprint mismatch becomes terminal `PlanChanged`; it requires a new PREPARE/review. Uncertain dispatch reconciles the known unit and has no dispatch path. |
| Observation cannot terminate the package transaction | PASS | The UI stops its own five-second observations at the persisted 60-minute deadline; it never sends a kill/cancel command. |
| Output, journal, persistence, and rendering are bounded | PASS | SSH capture has hard per-stream limits and discards oversized evidence; status/journal use 16 KiB capture and journal is limited to 80 lines. State persists one latest operation per device and tests reject raw stdout/stderr/journal/APT content. UI renders only bounded update/deferred details. |
| Success requires more than systemd exit | PASS | Successful unit state enters post-terminal `dpkg --audit`, M15 verification, approved-package disappearance, reboot, persistence, Activity, and disruption cleanup before completion. |

## Lifecycle, coordinator, and recovery review

One App-managed `DeviceMaintenanceCoordinator` is registered by the Tauri application and is used by M10 administration, M15 explicit checks, M16 APPLY, and M16 recovery. Its RAII claims conflict all maintenance kinds per device, allow separate devices independently, and release on return or unwinding.

Persisted operations distinguish `notAttempted`, accepted, and uncertain dispatch. `preDispatchFailed` is terminal and definitely not started; uncertain dispatch is created only after the `systemd-run` invocation is attempted. Observation deadline and generated unit identity persist across restart. Terminal records do not require recovery.

The closure audit found and corrected one recovery defect: a known failed unit observed after restart could remain `verifying`. It now becomes persisted terminal `failed`, clears the operation-scoped disruption, appends `updates.failed`, runs bounded cleanup, and is covered by `recovered_known_failed_unit_is_terminal_and_cleans_up_once`.

## Polling, UX, Activity, and diagnostics review

- No M16 scheduler/background poll exists. Automatic reconciliation is card-mounted, one-at-a-time, scheduled only after the preceding request completes, and stops at the persisted deadline. Explicit **Check status** is one-shot.
- **Update device** requires supported, complete, untruncated, non-held, normal standard updates. Deferred-only results say **No standard updates available**, require manual review, and expose no mutation action.
- PREPARE is preview/metadata/audit only; final confirmation precedes APPLY. PlanChanged returns to renewed review. Pre-dispatch transport failure says that no update started; uncertain dispatch says not to retry. No progress percentage or internal fingerprint/unit identifier is exposed.
- `updates.initiated` is appended only after a known running unit; completion/failure Activity is appended after terminal classification. Expected disruption begins only for a recovery-relevant dispatched operation and is cleared terminally or expires.
- M16 diagnostics use static labels `device_updates.apply.preflight`, `.metadata_refresh`, `.plan_verify`, `.dispatch`, `.status`, and `.verify`; they contain no device, host, package, command, or journal data.

## Real-device evidence and correction history

- PI 2: PREPARE/Cancel passed; one fresh 34-package APPLY dispatched exactly one unit and completed with clean audit, zero pending/deferred packages, no reboot requirement, healthy SSH/Tailscale/NTFY/Home Assistant/Docker/System state, persisted terminal state, Activity, and disruption cleanup.
- PI 5: one 162-package APPLY completed and remained healthy; five packages remained deferred by conservative standard upgrade and are manual-review-only.
- Operator corrections are recorded in [WU16-05 evidence](m16-wu098-operator-validation.md): pre-dispatch classification (`eeb5f54`), terminal recovery (`b7abb29`), explicit check feedback (`a5e1ae1`), deferred-only UX (`0861b8d`), and confirmation package review. This closure additionally corrects recovered known-failed-unit terminal cleanup.

## Exit criteria

| Criterion group | Result |
| --- | --- |
| M15 reuse, complete reviewed preview, fresh metadata, fingerprint reconfirmation, safe standard-upgrade class | PASS |
| Fixed sudo policy, dpkg/lock protection, detached one-dispatch recovery, no observation kill | PASS |
| Restart persistence, bounded evidence, post-terminal verification, no automatic reboot | PASS |
| Shared coordination, coherent Activity/disruption, zero idle work, static diagnostics | PASS |
| Deterministic regressions, honest two-device mutation evidence, canonical validation, no high-risk contradiction | PASS |
| Live restart during active APPLY and diagnostics capture before APPLY | PASS with residual |

## Final risk and residuals

**28/100 — Yellow.** The controlled mutation surface is intentionally high-impact, but its fixed command boundary, persistence/recovery model, deterministic tests, and one successful authorized update on each available device class keep risk below the human-approval threshold.

Residuals before merge:

- A live app restart while an APPLY was active was not performed; deterministic restart/recovery tests passed.
- Performance Diagnostics was not started before either real APPLY; deterministic diagnostic label/bounds tests passed.

Neither residual authorizes or requires another package mutation. M17 remains unstarted.
