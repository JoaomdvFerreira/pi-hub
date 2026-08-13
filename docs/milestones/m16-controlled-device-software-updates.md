# Pi-Hub — M16 Controlled Device Software Updates

Status: In progress — WU16-01 execution contract frozen; no mutation implementation
Target release: v0.4.0 (paired with merged M15, subject to release review)
Predecessor: M15 — Device Update Intelligence & Maintenance Readiness
Risk class: High
Primary mode: Explicit operator-triggered mutation
Initial platform: Raspberry Pi OS / Debian-family systems using APT/dpkg and systemd

## 1. Purpose and authority

M16 extends the trusted M15 System Updates preview into a controlled package-update action without turning Pi-Hub into a general package-management or remote-command surface.

The milestone must reuse M15 support detection, package evidence, cached update state, package/deferred semantics, persistence boundaries, and Performance Diagnostics instrumentation. It must reuse M10 confirmation, Activity, expected-disruption, and conservative outcome principles where they fit rather than creating parallel administration concepts.

M16 adds one initial mutation capability:

> Update the currently installed packages that APT can upgrade without adding or removing packages.

This milestone does not add distribution upgrades, arbitrary package selection, automatic reboot, scheduling, fleet-wide update orchestration, repair commands, or arbitrary APT arguments.

## 2. Product outcome

From Device Detail → System → System Updates, the operator can:

1. see the current M15 update preview;
2. inspect the complete bounded list of packages Pi-Hub intends to update;
3. choose **Update device**;
4. confirm the impact;
5. allow Pi-Hub to refresh package metadata and revalidate the plan;
6. install the confirmed standard update class through a fixed noninteractive backend operation;
7. follow a small number of truthful progress stages;
8. receive a verified final state;
9. use the existing M10 Restart Device action if an explicit reboot-required signal is present.

The normal UI remains concise. Raw APT output, internal lock state, package-manager diagnostics, and implementation details do not become the default product surface.

## 3. Core safety decisions

### 3.1 Conservative APT update class

The M16 action uses the semantics of `apt-get upgrade`, not `apt upgrade`, `full-upgrade`, or `dist-upgrade`.

The supported package mutation is restricted to already-installed packages that can be upgraded without changing the install set. Packages that APT keeps back/defer remain deferred.

Defense-in-depth rules:

- use `--no-remove`;
- do not enable `--with-new-pkgs`;
- do not use `dist-upgrade` or `full-upgrade`;
- do not use `autoremove`;
- do not use `--fix-broken` automatically;
- do not use `--ignore-missing`/`--fix-missing` to conceal failures;
- do not use `--force-yes`;
- do not use `--allow-unauthenticated`;
- do not use `--allow-downgrades`;
- do not use `--allow-remove-essential`;
- do not use `--allow-change-held-packages`;
- do not use insecure-repository overrides;
- do not automatically accept repository release-information changes.

### 3.2 Fresh metadata, then plan revalidation

M15 intentionally reports against existing local APT metadata. M16 is different: package mutation must not proceed against an unchecked stale plan.

After the operator confirms **Update device**, M16 must:

1. acquire the authoritative per-device maintenance claim;
2. perform preflight checks;
3. refresh package indexes using the fixed equivalent of `sudo -n apt-get --error-on=any update`;
4. rerun the same M15-compatible update simulation against the refreshed indexes;
5. compare the refreshed install/deferred plan with the plan the operator reviewed.

If the plan changed, M16 must stop before package installation, update the M15 cached preview, show the refreshed package list, and require a new explicit confirmation.

No package transaction may begin on a materially changed, unreviewed plan.

### 3.3 Complete preview required

The operator must be able to review every package Pi-Hub intends to update.

If the M15 package detail result is truncated or otherwise incomplete, **Update device is unavailable**. M16 v1 must not silently update packages that were omitted from the preview.

The existing M15 bound remains authoritative unless a later evidence-backed change deliberately revises it.

### 3.4 Package transaction survives SSH loss

The actual APT upgrade must not depend on one long-lived SSH process remaining connected.

Package upgrades can legitimately affect SSH, networking, Tailscale, Docker, systemd, libraries, and other services. M16 therefore launches the fixed package transaction as a detached transient **systemd service** on supported devices and observes it separately.

The service must:

- be created through a typed backend operation;
- use a backend-generated validated operation/unit identifier;
- run in system service context, not a user session scope;
- use `Type=exec` semantics so dispatch is not treated as successful before the target command is actually executed;
- remain inspectable after completion long enough for Pi-Hub to persist the terminal result and recover from application restart;
- never accept command text or unit names from the frontend.

M16 execution support therefore requires the required systemd/systemd-run capability in addition to the M15 APT/dpkg inspection capability. A device may remain M15-supported while being M16-execution-unsupported.

### 3.5 No forced cancellation after mutation begins

Before dispatch, the operator may cancel the confirmation flow.

After the package transaction has been dispatched, Pi-Hub must not offer a Cancel button and must not kill APT/dpkg merely because:

- the UI is closed;
- SSH disconnects;
- the app restarts;
- the observation deadline expires.

Terminating dpkg mid-transaction is more dangerous than losing UI observation.

M16 bounds **observation and recovery work**, not the integrity-critical lifetime of an already-running package transaction.

If the observation window expires while the remote unit is still active, return a truthful `StillRunning`/equivalent state and retain recovery metadata. Do not fabricate `TimedOut` as if the package manager was terminated.

## 4. Noninteractive package policy

The detached update operation must be noninteractive and fixed by backend policy.

Required semantics:

- `DEBIAN_FRONTEND=noninteractive`;
- APT assume-yes (`-y` / equivalent);
- dpkg conffile handling equivalent to `--force-confdef` plus `--force-confold`;
- `NEEDRESTART_MODE=l` or equivalent list-only behavior where needrestart is present;
- no stdin/TTY dependency;
- no blanket `yes`/newline stream supplied to arbitrary package scripts;
- locale/output behavior deterministic where parsing is required.

The conffile policy preserves operator-modified configuration when dpkg cannot choose a default automatically. M16 does not overwrite local configuration merely to avoid a prompt.

The exact argv/environment construction is frozen in WU16-01 after inspection of the current remote-execution and sudo abstractions. No frontend-controlled argument is permitted.

## 5. Support and preflight contract

An update may start only when all required preflight evidence passes.

Required gates:

- device currently reachable through its configured validated SSH path;
- mandatory SSH host-key verification passes;
- M15 identifies a supported APT/dpkg platform;
- M16 execution capability is available (`systemd`/`systemd-run` and required status inspection);
- no conflicting Pi-Hub maintenance/update operation owns the device;
- the reviewed M15 preview is complete and not truncated;
- there is at least one normal update to apply;
- noninteractive privilege is available when each privileged fixed operation is invoked;
- `dpkg --audit` reports no pre-existing partial/inconsistent package state;
- metadata refresh completes successfully with no ignored repository error;
- refreshed simulation remains equivalent to the reviewed plan.

### Package-manager busy state

Do not delete APT/dpkg lock files, stop system package timers, or manipulate package-manager state to obtain a lock.

If APT/dpkg reports lock contention, classify it as `PackageManagerBusy` (or repository-equivalent typed state), perform no package transaction, and tell the operator to retry later.

### Disk space

Do not invent an arbitrary Pi-Hub-specific free-space threshold as a correctness gate.

WU16-01 may add a reliable read-only download-size/free-space advisory if it can be derived from stable APT evidence without duplicating APT policy. APT remains authoritative for whether its own transaction can proceed. Disk-space failures must remain explicit and must never trigger automatic cleanup/autoremove.

## 6. Plan identity and TOCTOU protection

M16 must bind confirmation to a deterministic representation of the reviewed plan.

The backend should calculate a plan fingerprint from canonical sorted package mutation entries, including at minimum:

- package identity;
- installed version;
- candidate version.

Deferred/kept-back state must be included in comparison where a change would alter what the operator believes will or will not be updated.

The frontend does not generate or validate the fingerprint. Backend evidence is authoritative.

Plan mismatch behavior:

`Reviewed → Confirmed → Metadata refreshed → Plan changed → STOP → Persist refreshed M15 result → Review again`

No “continue anyway” bypass exists in M16 v1.

## 7. Per-device concurrency and conflicts

M16 must not overlap on the same device with:

- another M16 software update;
- an M15 remote Check for Updates;
- M10 Restart Device;
- M10 Shut Down Device;
- M10 Restart Docker;
- M10 Restart Tailscale;
- any later workload deployment action that declares a device-maintenance conflict.

Different devices remain independent.

Backend conflict handling is authoritative even when the UI is stale.

Preferred architecture: one shared per-device maintenance/operation coordinator that can express operation kinds and conflicts. Avoid nested acquisition of independent M10/M15/M16 locks in different orders. WU16-01 must inspect current owners and freeze the smallest safe migration path before implementation.

## 8. Operation state model

The domain model should preserve useful distinctions without exposing implementation noise.

Recommended internal states:

- `Requested`
- `Preflight`
- `RefreshingMetadata`
- `VerifyingPlan`
- `PlanChanged`
- `Dispatching`
- `Installing`
- `Verifying`
- `Completed`
- `CompletedRebootRequired`
- `PackageManagerBusy`
- `Failed`
- `StillRunning`
- `OutcomeUncertain`

User-facing stages should be simpler:

- Preparing
- Refreshing package lists
- Verifying update plan
- Installing updates
- Verifying device
- Updated
- Updated — restart required
- Could not update
- Update still running
- Outcome needs attention

Do not show fake percentages. A spinner/stage label and elapsed time are sufficient.

## 9. Detached execution and observation

### Dispatch

The package transaction is launched as a transient systemd service with a generated Pi-Hub operation identity.

The transaction uses the fixed semantic equivalent of:

`apt-get -y --no-remove -o Dpkg::Options::=--force-confdef -o Dpkg::Options::=--force-confold upgrade`

with the noninteractive environment from §4.

The exact systemd-run argv is backend-owned and frozen by WU16-01.

### Observation

Pi-Hub observes the operation through bounded machine-readable `systemctl show` queries against the known unit identity.

Observation rules:

- polling exists only while a persisted M16 operation is nonterminal;
- no normal monitoring cadence or always-on update polling is added;
- status query cadence and total observation window are bounded;
- initial target observation window: 60 minutes, subject to WU16-01 confirmation against current monitoring architecture;
- expiry stops active observation but does not kill the remote transaction;
- app restart resumes recovery from persisted nonterminal state;
- transport loss is treated as an observation problem, not proof that APT failed.

A bounded transient journal tail may be consulted only to classify a terminal failure when necessary. Raw journal/APT output must not be persisted or copied into Activity. If bounded diagnostics cannot be obtained safely, use a coarser failure classification rather than weakening privacy/resource limits.

## 10. Persistence and crash recovery

Persist only the latest M16 maintenance operation state needed for truthful recovery, including where applicable:

- operation ID;
- device ID;
- generated transient unit identity;
- reviewed-plan fingerprint;
- requested/started timestamps;
- current stage/state;
- observation deadline;
- terminal result/failure class;
- concise non-sensitive detail;
- reboot-required result.

Do not persist:

- raw APT output;
- raw journal output;
- raw commands;
- credentials;
- unbounded package logs.

M7 Activity remains the durable historical record.

On Pi-Hub restart, nonterminal operations must be reconciled against the remote transient unit when the device becomes reachable. Missing/ambiguous remote evidence must produce verification or `OutcomeUncertain`, never fabricated success.

## 11. Verification contract

Dispatch success is not update success.

After the transient service reaches a terminal state, M16 must:

1. inspect its machine-readable terminal outcome;
2. run `dpkg --audit`;
3. rerun the M15 update-intelligence path against the now-refreshed package metadata;
4. update the cached M15 System Updates result;
5. confirm that packages in the approved normal-update plan are no longer pending, while legitimate deferred packages may remain;
6. refresh normal device/system/container/service state through existing owners where appropriate;
7. re-evaluate the existing M15 reboot-required evidence.

A zero systemd/apt exit status with inconsistent dpkg state or unexplained approved packages still pending must not become plain `Completed`.

If the remote unit disappears or the device reboots/powers off before a trustworthy terminal result is captured, verification evidence determines whether the outcome can be recovered. Otherwise use `OutcomeUncertain`.

M16 does not run `dpkg --configure -a`, `apt --fix-broken`, or any automatic repair. Pre-existing or post-update inconsistent package state is handed to the operator for manual terminal repair.

## 12. Reboot behavior

M16 never reboots automatically.

When the existing evidence contract reports reboot required:

- final state is `CompletedRebootRequired` or equivalent;
- the UI explains that updates completed but a restart is recommended/required by the supported platform signal;
- the operator may invoke the existing M10 Restart Device action;
- M16 must not implement a second reboot mechanism.

Absence of a reliable reboot-required signal must not be translated into a guarantee that no restart is useful.

## 13. Expected disruption, Alerts, and Activity

Package updates may transiently affect SSH, networking, Docker, and registered services.

Reuse the M10 expected-disruption concept with an update-specific bounded window rather than introducing a general maintenance scheduler.

Initial target:

- bounded suppression/defer window up to 60 minutes while the update is active;
- clear early when verified recovery completes;
- normal alert governance resumes on expiry;
- existing unrelated alerts are not erased or automatically resolved;
- underlying observed health evidence continues to be recorded where current owners require it.

Activity should record meaningful lifecycle evidence without stage spam:

- update initiated;
- final completed / reboot-required / failed / still-running / uncertain outcome.

Activity may include package count, duration, and concise failure classification. It must not contain raw APT/journal output, package-manager command text, credentials, or repository secrets.

## 14. Performance Diagnostics

M16 must be observable using stable non-sensitive labels while adding zero recurring work when idle.

WU16-01 freezes final labels. Recommended namespace:

- `device_updates.apply`
- `device_updates.apply.preflight`
- `device_updates.apply.metadata_refresh`
- `device_updates.apply.plan_verify`
- `device_updates.apply.dispatch`
- `device_updates.apply.status`
- `device_updates.apply.verify`
- `device_updates.apply.persist`

Labels must not contain device IDs, hostnames, package names, unit IDs, command text, repository URLs, or credentials.

Closure evidence must measure:

- SSH/status queries for one representative update;
- metadata-refresh duration;
- detached transaction elapsed time where measurable;
- status-poll count/cadence;
- verification cost;
- persistence cost;
- inactive overhead (must remain zero for M16-specific polling/work);
- recovery behavior after simulated app/SSH interruption.

## 15. UX contract

M16 extends the concise M15 **System Updates** card.

### Ready state

When a complete supported preview contains normal updates:

- `N updates available`
- optional `N packages deferred`
- `View updates`
- `Check again`
- `Update device`

`Update device` is unavailable when:

- no normal updates exist;
- preview is incomplete/truncated;
- support is unavailable;
- a conflicting operation owns the device;
- the previous check failed/unknown and no complete plan exists.

### Confirmation

Confirmation must state:

- how many installed packages are currently planned for update;
- deferred packages will not be forced;
- Pi-Hub will refresh package lists and re-check the plan before installation;
- if the plan changes, installation stops for review;
- services, containers, SSH, or networking may restart/be temporarily unavailable;
- Pi-Hub will not automatically reboot the device.

### During update

Show only the current truthful stage, spinner, and elapsed time. Disable conflicting actions.

Do not expose raw package-manager output or a fake progress percentage.

### Plan changed

Show the refreshed M15 preview and require **Review updates** / a new explicit Update confirmation. No bypass.

### Completion

Prefer concise outcomes:

- `System updated`
- `System updated — restart required`
- `Update failed`
- `Update still running`
- `Update outcome needs attention`

Failure detail should be short and actionable. Terminal/manual administration remains the escape hatch for repair.

## 16. Failure semantics

At minimum distinguish internally/user-presentably where evidence supports it:

- `Unsupported`
- `PrivilegeUnavailable`
- `PackageManagerBusy`
- `PackageStateInconsistent`
- `MetadataRefreshFailed`
- `PlanChanged`
- `DispatchFailed`
- `PackageOperationFailed`
- `TransportUnavailableDuringObservation`
- `VerificationFailed`
- `StillRunning`
- `OutcomeUncertain`

Do not over-classify download-vs-install failure from brittle string parsing. A small bounded diagnostic classifier may refine errors only where deterministic evidence exists.

## 17. Explicit non-goals

M16 does not add:

- arbitrary APT/package-manager arguments;
- per-package checkbox selection;
- package install/remove UI;
- `full-upgrade` / `dist-upgrade`;
- distribution release upgrade;
- automatic repair;
- automatic `autoremove`;
- repository/source editing;
- GPG/trust bypasses;
- automatic reboot;
- Cancel/Kill after package mutation starts;
- scheduled updates;
- fleet/batch updates;
- generic root command execution;
- a persistent Pi-Hub agent;
- M17 workload deployment actions;
- M18 Maintenance Center behavior.

## 18. Work Units

### WU16-01 — Execution Contract, Privilege & Recovery Architecture

Design/freeze only:

- exact supported M16 execution capability contract;
- exact metadata-refresh and package-upgrade argv/environment;
- noninteractive conffile/needrestart behavior;
- M10/M15/M16 concurrency-owner strategy;
- plan fingerprint and PlanChanged semantics;
- systemd transient-unit lifecycle and generated unit naming;
- observation cadence/window and app-restart recovery;
- output/journal bounds and failure-classification policy;
- persistence contract;
- Activity/alert integration;
- Performance Diagnostics labels;
- deterministic fixture/harness and live-validation plan;
- whether any reliable disk-space advisory is justified.

No package mutation implementation and no live update.

### WU16-02 — Maintenance State, Coordination & Persistence

Implement the typed operation model, authoritative conflict coordination, persistence/recovery state, Tauri/backend surfaces, expected-disruption integration, Activity skeleton, and deterministic transition/conflict tests.

No real package transaction yet unless required by a fake executor test path.

### WU16-03 — Detached APT Execution & Verification

Implement:

- preflight diagnostics;
- metadata refresh;
- M15 plan revalidation/fingerprint comparison;
- detached systemd-run dispatch;
- bounded systemctl observation/recovery;
- noninteractive fixed upgrade policy;
- post-run dpkg audit;
- M15 result refresh/verification;
- reboot-required result;
- bounded failure diagnostics;
- deterministic process/transport/restart tests.

### WU16-04 — System Updates Mutation UX

Add Update device, confirmation, PlanChanged review, progress/stage UX, conflict states, restart-required handoff to M10, concise failure states, accessibility, and focused frontend coverage.

### WU16-05 — Failure, Recovery, Performance & Operator Validation

Exercise deterministic scenarios including:

- privilege failure;
- package-manager busy;
- pre-existing dpkg inconsistency;
- metadata-refresh failure;
- refreshed-plan mismatch;
- dispatch failure;
- SSH drop during active update;
- app restart while update active;
- remote unit success/failure/missing;
- observation-window expiry while unit continues;
- post-update dpkg inconsistency;
- approved packages still pending;
- reboot-required signal;
- output bounds;
- duplicate/conflicting operations;
- zero idle polling;
- Performance Diagnostics evidence.

Real package mutation requires explicit current-task operator authorization on a chosen non-critical device. Never manufacture destructive states merely to satisfy a test.

### WU16-06 — Regression Gates & Milestone Closure

Review the full M16 diff, establish deterministic gates, perform canonical validation, reconcile documentation/AIQT, calculate final risk, and prepare the PR.

Do not start M17 automatically.

## 19. Deterministic acceptance scenarios

At minimum cover:

| Scenario | Required result |
| --- | --- |
| Complete M15 preview + unchanged refreshed plan | May proceed after confirmation |
| Preview truncated | Update action blocked |
| Metadata refresh changes candidate version/package set | PlanChanged; no installation |
| Metadata refresh has any repository error | Fail before installation |
| Held/deferred package exists | Not forced; remains deferred |
| APT wants removal/additional package | Abort / unsupported plan; no mutation |
| Sudo needs interaction | PrivilegeUnavailable; no prompt |
| dpkg audit non-empty before update | Block; manual repair required |
| Package manager already owns lock | PackageManagerBusy; no lock deletion |
| Duplicate Update click | One effective operation |
| M10 action conflicts | Backend rejects one deterministically |
| Different devices update | Independent if architecture permits |
| SSH drops after detached dispatch | Remote transaction continues; observation recovers |
| Pi-Hub exits/restarts during update | Remote transaction continues; persisted state reconciles |
| Observation window expires but unit active | StillRunning; do not kill transaction |
| Unit exits 0 + dpkg clean + approved plan gone | Completed |
| Unit exits 0 + reboot marker required | CompletedRebootRequired |
| Unit exits nonzero | Failed; bounded diagnostics only |
| Unit missing / terminal result unknowable | Verify; otherwise OutcomeUncertain |
| dpkg inconsistent after run | VerificationFailed/OutcomeUncertain; no auto-repair |
| Approved normal updates remain unexpectedly | VerificationFailed; no false success |
| App idle with no active update | Zero M16-specific polling |

## 20. Live validation policy

M16 is a disruptive/high-risk milestone.

Agents must not execute a real package update merely because the milestone document exists.

Live validation is split:

1. **Safe read-only validation** — capability detection, complete preview, preflight evidence that does not mutate package state, deterministic systemd capability inspection.
2. **Metadata-refresh validation** — `apt-get update` mutates package indexes and therefore requires explicit authorization for the current task.
3. **Package-mutation validation** — actual upgrade requires explicit authorization for a named non-critical device and a suitable maintenance window.

For a real update test, the operator must understand that services/networking may restart and the device may temporarily become unreachable. Pi-Hub must never automatically reboot it.

## 21. Exit criteria

M16 closes only when:

- M15 preview/state is reused rather than reimplemented;
- complete reviewed preview is required before mutation;
- metadata is refreshed before package mutation;
- changed refreshed plan requires re-review and reconfirmation;
- the supported update class cannot add/remove packages;
- dangerous APT overrides remain absent;
- pre-existing broken dpkg state blocks the update;
- package-manager lock contention is handled without lock manipulation;
- actual package mutation survives SSH/app observation loss through detached service execution;
- observation timeout cannot kill an in-progress dpkg transaction;
- one same-device maintenance operation is authoritative across relevant M10/M15/M16 conflicts;
- state survives app restart sufficiently for recovery;
- raw APT/journal output is not persisted;
- post-update dpkg and M15 verification determine success;
- reboot is never automatic;
- reboot-required state reuses M15 evidence and M10 restart behavior;
- expected-disruption/Activity/Alerts remain coherent;
- M16 introduces zero idle/background update work;
- Performance Diagnostics captures bounded operation cost;
- deterministic failure/recovery coverage passes;
- any real destructive operator validation is recorded honestly;
- canonical closure validation passes;
- final PR review finds no unresolved high-risk safety contradiction.

## 22. M17 handoff

M17 Managed Workload Deployment Actions may reuse the proven M16 maintenance-operation coordination, detached/recoverable operation principles, Activity, verification, and outcome semantics where appropriate.

M17 must not broaden M16 into arbitrary remote commands or generic root execution.
