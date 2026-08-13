# M16 — APT Execution Safety & Recovery Contract

Status: Canonical WU16-01 repository contract
Milestone: M16 — Controlled Device Software Updates
Contract type: Durable mutation/security/recovery policy

## 1. Purpose

This contract defines the safety boundary for applying APT package updates from Pi-Hub.

It exists separately from the milestone build specification because package mutation has failure modes that cross SSH transport, privilege handling, dpkg integrity, application restart, and systemd process lifetime. Later implementation must not weaken these rules merely to simplify UI or command execution.

## 2. Trust boundary

The only user-controlled input to an M16 update request is the stable device identity and the explicit confirmation decision.

The frontend must not supply:

- command text;
- APT arguments/options;
- package names for mutation;
- repository names/URLs;
- environment variables;
- systemd unit names;
- script paths;
- sudo arguments.

All remote operations are fixed typed backend operations.

SSH host-key verification and existing credential boundaries remain unchanged.

Pi-Hub must not edit sudoers, store a sudo password, or prompt for one. `sudo -n` failure is terminal for the current attempt.

Do not publish a naive broad passwordless-sudo recipe for `systemd-run *`; privilege configuration remains an operator responsibility and must be reviewed separately if narrow sudoers guidance is later added.

## 3. Supported update class

M16 v1 applies only normal APT upgrades of already-installed packages.

The execution policy is semantically equivalent to:

```text
DEBIAN_FRONTEND=noninteractive
APT_LISTCHANGES_FRONTEND=text
NEEDRESTART_MODE=l
apt-get -y --no-remove \
  -o Dpkg::Options::=--force-confdef \
  -o Dpkg::Options::=--force-confold \
  upgrade
```

The implementation may represent these as argv/environment entries rather than shell text. Direct argv execution is preferred; a shell must not be introduced merely for convenience.

M16 deliberately does not pipe an unbounded `yes` stream into package maintainer scripts. Known APT, debconf, dpkg conffile, apt-listchanges, and needrestart interaction points are made noninteractive explicitly; the transient service has no TTY/stdin interaction path. An unexpected package-specific interactive hook should fail or remain diagnosable rather than receive an invented blanket answer from Pi-Hub.

Mandatory prohibitions:

- no `dist-upgrade`;
- no `full-upgrade`;
- no `--with-new-pkgs`;
- no `autoremove`;
- no automatic install/remove action outside the upgrade plan;
- no `--fix-broken` repair;
- no `--ignore-missing`/`--fix-missing` masking;
- no `--force-yes`;
- no unauthenticated/insecure repository bypass;
- no forced downgrade;
- no essential-package removal override;
- no held-package override;
- no release-info-change auto-accept;
- no automatic reboot.

If APT cannot complete within these restrictions, Pi-Hub fails safely and leaves manual administration to the terminal.

## 4. Metadata refresh contract

Before any package transaction, M16 performs a fixed package-index refresh equivalent to:

```text
sudo -n apt-get --error-on=any update
```

The purpose is to ensure the package plan is built from refreshed repository metadata.

Rules:

- any repository error is a failed refresh;
- do not downgrade an error to a warning merely because some indexes succeeded;
- do not enable insecure repository options;
- do not auto-accept changed release metadata;
- bounded SSH timeout/output rules apply to the refresh operation;
- refresh failure prevents package installation.

The metadata refresh is mutation of APT index state. It is allowed only as part of an explicitly confirmed M16 update attempt.

## 5. Reviewed-plan binding

A package transaction may proceed only against a plan the operator had a chance to review.

### 5.1 Initial plan

The starting plan is the complete latest M15 System Updates result.

Required:

- supported platform;
- normal update count > 0;
- package details complete and not truncated;
- each planned update has package identity, installed version, candidate version.

### 5.2 Fingerprint

Backend calculates a deterministic fingerprint over canonical sorted mutation entries.

The fingerprint is an internal integrity token, not a security credential and not a frontend authority.

At minimum include:

`package identity + installed version + candidate version`

Include deferred/kept-back identity in the comparison when a change would alter the reviewed “will update / will not update” boundary.

### 5.3 Revalidation

After metadata refresh, rerun the M15-compatible simulation and derive the refreshed fingerprint.

- equal plan → installation may proceed;
- different plan → persist refreshed M15 result, return `PlanChanged`, release the mutation claim when safe, and require a new review/confirmation;
- incomplete refreshed plan → no installation;
- plan contains add/remove semantics → no installation.

No override exists to accept an unseen changed plan.

## 6. Package-manager integrity preflight

Run read-only `dpkg --audit` before mutation.

If it reports partially installed packages, missing/wrong control data, or another inconsistent package state:

- classify `PackageStateInconsistent`;
- do not run automatic repair;
- do not run `dpkg --configure -a`;
- do not run `apt --fix-broken`;
- instruct the operator to repair manually through the terminal and retry later.

Lock handling:

- do not infer “unlocked” from lock-file existence alone;
- do not delete lock files;
- do not kill another APT/dpkg process;
- do not disable package timers merely to seize the lock;
- authoritative APT/dpkg lock contention becomes `PackageManagerBusy`.

## 7. Detached transaction requirement

The mutating package transaction must run independently of the SSH client/session.

M16 uses a system **transient service** created through systemd-run on supported devices.

Required properties:

- system service, not `--scope` and not a user service;
- clean/detached service-manager parent;
- `Type=exec` semantics;
- generated backend-controlled unit identity;
- no frontend-provided unit/command text;
- service uses `RemainAfterExit=yes` (or exact equivalent) so successful completion remains inspectable until Pi-Hub has safely recorded/reconciled the result;
- no `--wait`, `--pipe`, or PTY dependency for the long-running package transaction;
- standard input must not require an interactive terminal.

The purpose is safety, not background automation: Pi-Hub still starts the transaction only after explicit confirmation.

## 8. No-kill transaction rule

Once the detached APT/dpkg transaction begins, Pi-Hub does not forcibly terminate it because of a local timeout or transport failure.

This includes:

- SSH timeout/disconnect;
- Tailscale disruption;
- application close/crash/restart;
- monitoring failure;
- UI navigation;
- bounded observation window expiry.

A package-manager process can be in an integrity-critical unpack/configure stage. Killing it to satisfy a desktop timeout can leave the device in a worse state.

Therefore:

- every SSH observation call is individually bounded;
- the overall active observation period is bounded;
- the remote transaction lifetime is not forcibly bounded by killing it;
- observation expiry returns `StillRunning` when remote evidence says the unit is active;
- unknown remote state produces recovery/`OutcomeUncertain`, not a kill attempt.

M16 v1 exposes no Cancel/Kill action after dispatch.

## 9. Observation contract

Use machine-readable `systemctl show` properties for the known transient unit.

Freeze the exact property set in WU16-01. It should be sufficient to determine:

- loaded/not-found state;
- active/sub state;
- main process exit code/status;
- execution result where available;
- timestamps needed for recovery.

Do not parse human-formatted `systemctl status` as the primary state contract.

Polling:

- only while an M16 operation is persisted as nonterminal;
- bounded cadence;
- bounded observation window;
- no M16 polling when no update is active;
- different-device observations isolated;
- duplicate UI renders must not create duplicate pollers.

Suggested initial target: 5-second status interval with a 60-minute observation window, subject to WU16-01 validation against current Pi-Hub scheduler/query architecture.

## 10. Failure diagnostics and output privacy

The package transaction's stdout/stderr belongs to the remote service/journal and is not streamed unbounded into Pi-Hub.

Primary success/failure evidence comes from:

- systemd unit state/exit status;
- post-run `dpkg --audit`;
- refreshed M15 update evidence.

For a terminal failure, Pi-Hub may issue one bounded journal-tail query for classifier input.

Rules:

- use the existing hard SSH output-cap implementation;
- keep the requested tail small and bounded;
- discard raw diagnostic text after classification;
- do not persist raw journal/APT output;
- do not copy raw output into Activity;
- do not include repository URLs, credentials, hostnames, package names, or unit IDs in Performance Diagnostics labels;
- if a journal query itself exceeds bounds/fails, use a generic typed package-operation failure.

Do not create a fragile parser that claims download-vs-install precision from arbitrary human prose. Prefer conservative `PackageOperationFailed` unless a stable signal exists.

## 11. App restart and recovery

Persist the operation ID, generated unit identity, reviewed-plan fingerprint, and a pre-dispatch state **before** invoking systemd-run. If dispatch returns an uncertain transport result, Pi-Hub must recover by querying that known unit identity rather than blindly retrying.

A Pi-Hub restart before detached dispatch completed must never automatically resume into package mutation. It requires fresh reconciliation and, unless dispatch can be proven to have occurred, a new operator action/confirmation. Only an already-dispatched known transient unit may be resumed automatically for observation.

Recovery algorithm:

1. load nonterminal M16 operation record;
2. wait until the device can be reached through its validated configured path;
3. query the backend-generated transient unit identity;
4. if active → resume bounded observation;
5. if terminal → run verification;
6. if missing → inspect dpkg state and M15 plan to infer only what evidence supports;
7. if no trustworthy terminal conclusion exists → `OutcomeUncertain`.

App restart must not cause a second APT transaction for the same persisted operation.

## 12. Device reboot/power-loss recovery

M16 does not assume a package transaction can survive device power loss/reboot.

If the device reboots or loses power during installation:

- reconnect when possible;
- do not redispatch automatically;
- run `dpkg --audit`;
- run M15 update inspection;
- inspect available transient-unit evidence if it survived;
- classify success only if post-state proves it;
- otherwise report `VerificationFailed` or `OutcomeUncertain` and require manual repair where appropriate.

No automatic `dpkg --configure -a` or repair attempt is permitted.

## 13. Verification and success definition

`systemd-run accepted` is not success. `apt-get exit 0` alone is not sufficient either.

Plain `Completed` requires all applicable evidence:

- remote package unit terminal outcome is successful;
- `dpkg --audit` is clean;
- refreshed M15 evidence no longer shows the approved normal-update plan pending;
- no update-caused verification contradiction exists.

Deferred/kept-back packages may legitimately remain and do not fail the operation.

`CompletedRebootRequired` additionally requires the existing supported explicit reboot-required signal.

If an approved package unexpectedly remains pending after apparent success, classify verification failure/residual state; do not silently call it complete.

## 14. Cleanup of transient state

After Pi-Hub has persisted a terminal result and completed verification, it may remove/reset the generated transient systemd unit state through a fixed typed cleanup operation where necessary.

Cleanup failure is non-destructive and must not change a verified package result into false success/failure. Record it as bounded maintenance residue if it materially affects future recovery.

Do not clean up the unit before terminal evidence has been durably recorded.

## 15. Expected-disruption safety

An active M16 update may trigger temporary service/network disruption.

Reuse bounded expected-disruption governance:

- suppress/defer only new user-facing noise attributable to the explicit update;
- never erase current health evidence;
- never blanket-suppress unrelated alerts;
- expire suppression automatically;
- clear early after verified recovery;
- if the update remains active beyond the bounded window, normal alerts may resume.

No maintenance suppression may survive indefinitely because the app crashed.

## 16. Sudo and privilege failure

Each privileged fixed operation uses noninteractive sudo behavior.

If sudo requires a password or rejects the command:

- fail quickly;
- do not open a password prompt;
- do not store a password;
- do not retry interactively;
- return `PrivilegeUnavailable` where the classification is deterministic.

Pi-Hub does not weaken host or sudo security settings automatically.

## 17. WU16-01 frozen repository mapping

This section resolves the implementation choices left open by the reviewed policy. It is the binding input to WU16-02 through WU16-05.

### 17.0 WU16-03 two-phase consent boundary

WU16-03 separates non-mutating preparation from package mutation. **PREPARE** is: explicit Update device intent → preflight → metadata refresh → privileged simulation → fresh plan/fingerprint → return for final confirmation. It releases the shared maintenance claim before waiting for the operator. **APPLY** is: final confirmation → reacquire the claim → immediate privileged re-simulation → fingerprint comparison → dispatch only when unchanged. A mismatch is `PlanChanged`, persists refreshed M15 evidence, and never has a continue-anyway path.

The fingerprint is an immediate pre-dispatch consent gate, not a claim of atomic package-state locking. The frontend supplies only the final confirmation decision and device identity; it does not generate or validate the fingerprint. A prepared, pre-dispatch record requires a fresh confirmation after restart and is never dispatched automatically.

### 17.1 One per-device maintenance owner

M10 currently owns a process-global `ACTIVE_OPERATIONS` set in `commands/administration.rs`; M15 owns a separate `UpdateCheckCoordinator` in `monitoring/update_concurrency.rs`. WU16-02 replaces both with one App-managed `DeviceMaintenanceCoordinator` in `monitoring`, keyed by device ID and holding an operation kind. It is the only same-device claim owner for M10 administration, M15 checks, and M16 apply/recovery. It returns a typed conflict before dispatch; it never nests the two existing locks. Claims use RAII for in-process work, while a recovered persisted M16 nonterminal operation reclaims its device before observation. Different device IDs remain independent.

The initial conflict matrix is all-to-all between M10 Restart Device/Shut Down Device/Restart Docker/Restart Tailscale, M15 Check for Updates, and M16 Apply Updates. A read-only cached-result fetch takes no claim. The M16 claim starts before preflight and is released on every pre-dispatch failure, `PlanChanged`, or terminal persisted outcome; a `StillRunning` or otherwise nonterminal persisted operation retains/re-establishes it.

### 17.2 Fixed operations and execution capability

M16 requires M15 support plus fixed-command evidence that `sudo -n systemd-run`, `sudo -n systemctl show`, `sudo -n systemctl reset-failed`, and `dpkg --audit` are usable. Capability detection is read-only and must not use a broad sudo probe. Every privileged invocation is a distinct fixed `RemoteOperation`; the frontend supplies only the device ID and confirmation token.

The fixed metadata-refresh remote command is:

```text
sudo -n apt-get --error-on=any update
```

It has a 120-second SSH timeout and the existing M15 hard capture caps of 256 KiB stdout and 16 KiB stderr. Any nonzero exit, SSH output-cap breach, or repository error fails as `MetadataRefreshFailed`; no package unit is dispatched.

The backend generates `pihub-update-` plus a lower-case, hyphenless UUID (32 hexadecimal characters), validates that exact format locally, and stores the resulting `<name>.service` before dispatch. The fixed detached dispatch argv semantics are:

```text
sudo -n systemd-run --unit=<generated-name> --service-type=exec \
  --property=RemainAfterExit=yes \
  --setenv=DEBIAN_FRONTEND=noninteractive \
  --setenv=APT_LISTCHANGES_FRONTEND=none \
  --setenv=NEEDRESTART_MODE=l -- \
  apt-get -y --no-remove \
  -o Dpkg::Options::=--force-confdef \
  -o Dpkg::Options::=--force-confold upgrade
```

`systemd-run` is invoked without `--scope`, `--user`, `--wait`, `--pipe`, or a PTY. The service-manager execution context supplies no stdin. WU16-03 must construct this as fixed argv entries (not frontend text and not a new general shell API). The dispatch SSH call is bounded to 15 seconds and uses the same 256 KiB/16 KiB caps. The service is not killed when that call is uncertain.

### 17.3 Plan, observation, cleanup, and recovery

The authoritative plan fingerprint is SHA-256 over UTF-8 canonical lines sorted by package identity: `upgrade\u001f<package>\u001f<installed>\u001f<candidate>\n`, followed by sorted deferred lines `deferred\u001f<package>\n`. The version of this representation is stored with the digest. The backend compares it after refresh using the M15 parser; incomplete output, add/remove semantics, or a different digest returns `PlanChanged`, persists the refreshed M15 result, and performs no dispatch.

The fixed status query is `sudo -n systemctl show --property=LoadState --property=ActiveState --property=SubState --property=Result --property=ExecMainCode --property=ExecMainStatus --property=ActiveEnterTimestampMonotonic --property=ActiveExitTimestampMonotonic --property=InactiveEnterTimestampMonotonic -- <generated-unit>`. It is parsed only as `KEY=VALUE` records. It runs at most once every five seconds, with a 15-second timeout and 16 KiB stdout/16 KiB stderr cap, for an absolute 60-minute observation window. Only persisted nonterminal M16 operations poll; zero such records means zero M16 polling. Unit cleanup is the fixed `sudo -n systemctl reset-failed -- <generated-unit>` only after terminal outcome and verification are atomically persisted. Cleanup failure is recorded as residue and cannot rewrite the verified outcome.

Before `systemd-run`, atomically persist operation ID, device ID, unit ID, fingerprint version/digest, requested time, `Dispatching` state, and the 60-minute observation deadline. A dispatch transport ambiguity is never retried blindly: recovery queries that known unit. On app restart, only a known persisted unit is reconciled: active resumes observation; terminal runs verification; missing triggers `dpkg --audit` and M15 inspection, then `OutcomeUncertain` unless those prove an outcome. A pre-dispatch record without proof of dispatch requires fresh user confirmation and never auto-starts APT.

### 17.4 Diagnostics, Activity, alerts, and disk-space decision

All M16 SSH work uses the existing bounded executor. A terminal nonzero result may make exactly one `journalctl -u <generated-unit> -n 80 --no-pager` typed query with a 16 KiB stdout/16 KiB stderr cap; the raw result is classified then discarded. It is never persisted, emitted to Activity, or added to diagnostic labels.

M16 adds only these stable labels: `device_updates.apply`, `.apply.preflight`, `.apply.metadata_refresh`, `.apply.plan_verify`, `.apply.dispatch`, `.apply.status`, `.apply.verify`, and `.apply.persist`. Existing M15 labels remain unchanged. Activity uses `ActivityCategory::Administration` with `updates.initiated` and one terminal code: `updates.completed`, `updates.reboot_required`, `updates.failed`, `updates.still_running`, or `updates.outcome_uncertain`; it contains only count, duration, and typed class. Expected disruption reuses the M10 repository/alert path with a new update operation type and an absolute 60-minute expiry, cleared early after verification. It suppresses only new device-offline noise and never deletes health evidence or existing/unrelated alerts.

No disk-space advisory is added in M16 v1. The current `df` snapshot is not transaction-specific and M15 does not expose a stable, bounded APT download-size value; an advisory would duplicate or contradict APT policy without safely predicting unpack space. APT disk failures remain typed, actionable failures with no automatic cleanup.

### 17.5 Deterministic and staged operator validation

WU16-03 uses the existing injectable `RemoteExecutor` seam plus a scripted process harness for fixed argv/unit responses. It must assert the full scenario matrix in sections 17-18, including no dispatch on refresh/plan/audit/privilege/busy failure, one dispatch across conflicts, SSH/app interruption recovery, no kill on expiry, terminal verification contradictions, bounded journal discard, and zero idle polls. Assertions include exact fixed command construction, environment entries, state persistence before dispatch, and diagnostic-label redaction.

Operator validation is staged: (1) read-only capability/preview and fake-systemd recovery checks; (2) an explicitly authorized metadata-refresh test on a named non-critical device; (3) an explicitly authorized upgrade in a maintenance window, recording plan, dispatch, interruption/recovery evidence, dpkg audit, refreshed M15 state, reboot signal, Activity, and diagnostics. WU16-01 authorizes none of stages 2 or 3.

## 18. Deterministic safety tests

At minimum prove with fake/process harnesses:

- refreshed identical plan proceeds;
- refreshed changed plan never dispatches package mutation;
- truncated preview never dispatches;
- plan with add/remove semantics never dispatches;
- metadata update partial/error never dispatches;
- sudo rejection never dispatches;
- pre-existing `dpkg --audit` issue never dispatches;
- lock contention never manipulates lock files;
- same-device duplicate/conflicting action dispatches once;
- detached transaction remains logically active after simulated SSH loss;
- app restart does not duplicate dispatch;
- observation-window expiry does not call process kill/cancel;
- remote success + clean verification → Completed;
- remote success + reboot marker → CompletedRebootRequired;
- remote nonzero → Failed;
- dpkg inconsistent after transaction → VerificationFailed/OutcomeUncertain;
- approved packages still pending → not Completed;
- missing transient unit uses recovery evidence and can remain OutcomeUncertain;
- bounded diagnostic overflow does not persist raw output;
- zero active M16 operations means zero M16-specific polling.

## 19. Real-device validation gate

No agent may perform the mutating transaction without explicit current-task operator authorization.

Before real mutation, perform read-only/deterministic validation first.

For an approved non-critical device, record:

- exact reviewed package count/list state;
- confirmation UX;
- metadata refresh result;
- whether the plan changed;
- detached dispatch evidence;
- temporary SSH/service disruption if any;
- recovery/observation behavior;
- final dpkg audit;
- final M15 package state;
- reboot-required state;
- Activity result;
- Performance Diagnostics operation counts/durations;
- confirmation that no automatic reboot occurred.

If a real test cannot be performed safely, record it as an operator residual rather than fabricating evidence.

## 20. Review invariants

Any future M16 change that violates one of these requires explicit contract revision and review:

1. no unseen package plan is installed;
2. no add/remove/full-upgrade policy enters M16 v1;
3. no SSH disconnect kills an already-running package transaction;
4. no desktop timeout kills dpkg;
5. no automatic repair or reboot;
6. no lock-file manipulation;
7. no arbitrary frontend command/arguments;
8. no raw package-manager output persistence;
9. no success without post-state verification;
10. no idle/background M16 polling.
