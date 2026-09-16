# M21 WU105 — Personal Finance Managed-Workload Registration Evidence

Stage A/A.1/A.2 and B0.5/B1 established that PI 5 (`100.116.232.83`, Tailscale device `raspberrypi5`) is a fail-closed transport target: an unresolvable mDNS hostname and a direct-IP connection timeout were both observed while the device was offline, and no fabricated result was reported for either attempt. Once the device returned online, SSH was only trusted after the offered host key fingerprint (`SHA256:YSQ0ye2+UtsMqfLLp9Y4t3AJWrJQ5EBbXrNqKezC2L0` ED25519) was confirmed out of band by the operator — no prior `known_hosts` entry existed for this host from the operating machine.

## B1 provisioning and Compose prerequisite

The Personal Finance repository (`/home/joaoadmin/projects/financas-pessoais`) required a machine-neutral Compose port binding before registration: `100.x.y.z:3010:3000` → `127.0.0.1:3010:3000`, committed as `fix: make compose port binding host-neutral` (`7eb5143c4a88068524d3d9fff83f43d6938aee11`). The repo-local Git identity was unset; the unambiguous existing `main` author (`JoaomdvFerreira <joao.mdvferreira@gmail.com>`) was applied repo-locally only. The initial push failed (`403`, embedded PAT lacked write access); the plaintext-credential remote was replaced with a credential-free HTTPS URL, and push was left for operator action. The operator subsequently completed provisioning with a dedicated read-only Deploy Key path (`git@github-financas-pessoais:...`), synchronizing `main`/`origin/main` at `e16a31d11dc5097086529a1ccd5bae84b1dc1630` with a clean worktree.

Both read-only Compose proofs passed against the synchronized repo:
- Base only: one service, `host_ip: 127.0.0.1`, `published: 3010 → target: 3000`.
- Base + root-owned M17 override (`/etc/pi-hub/personal-finance.compose.yml`) + fake canonical image (`PIHUB_PERSONAL_FINANCE_IMAGE=pi-hub-fake/personal-finance:proof-only`): one service, `100.116.232.83:3010 → 3000`, no loopback binding, no `build`, image resolved only to the fake placeholder.

Trust artifacts were confirmed unchanged throughout (owner `root:root`, modes `755`/`644`, no symlink substitution):

| Artifact | Expected SHA-256 |
| --- | --- |
| `/usr/local/libexec/pi-hub/actions/personal-finance` | `0cc3110b5c0400dea28df6aa8425e2ccefdeb47874d4140b506c3134d9a41e8e` |
| `/etc/pi-hub/personal-finance.compose.yml` | `c96e136508db01784b640c47c38882c93193f98ba85ad9f8da199c4238f028f0` |

## B2 registration

`managed-workloads.json` was written directly in the typed M17 schema (no Tauri command exists to create a workload — only `list_managed_workload_deployments` reads it), then loaded through the production `JsonManagedWorkloadRepository` and `list_managed_workloads_with` code paths via temporary, reverted unit tests (no diff left behind):

| Field | Value |
| --- | --- |
| `id` | `personal-finance` |
| `deviceId` | `65560b8a-974a-45da-8f9e-181b19970c54` (PI 5) |
| `name` | `Personal Finance` |
| `actionId` | `personal-finance` |
| `composeProject` | `financas-pessoais` |
| `requiredComposeServices` | `["financas-pessoais"]` |
| `requiredServiceHealthIds` | `["80238ce1-b3f6-46af-b323-2fc0bfb44fc3"]` (existing PI 5 Service Health target, `Finanças`) |
| `enabled` | `true` |
| `configRevision` | `1` |

Results:
- File validates against `ManagedWorkloadFile::validate()`; reload returns an identical typed entry.
- The M17 trust probe (the same read-only stat/sha256 shell fixed by `trust_probe_command`, not the action protocol itself) resolved `personal-finance` → `/usr/local/libexec/pi-hub/actions/personal-finance` as `trusted` with digest `0cc3110b…` matching the expected value exactly.
- `list_managed_workloads_with` against the real config directory returns exactly one card (`workloadId: personal-finance`, `name: Personal Finance`, `enabled: true`, `eligibleToPrepare: true`, `deployment: null`); its serialized form contains no internal action path, digest, compose project, or command text.

No `prepare`/`apply`/`verify` action was invoked, no Docker mutation occurred, and no sudoers change was made. Post-registration, the running Personal Finance container remained the same instance throughout (ID `a3f710774b48…`, image `sha256:c8706708…`, `RestartCount: 0`, unchanged `StartedAt`), bound at `100.116.232.83:3010 → 3000`, and reachable (`HTTP 200`).

## Stage C: real PREPARE/Cancel, operator-discovered defect, remediation, and live re-validation

The first real Stage C PREPARE attempt (through the actual Tauri UI, `Prepare update`) failed with the generic `PrepareProtocolInvalid`-backed message ("Pi-Hub could not prepare the managed workload safely… No deployment was started."). Diagnosis (read-only; the trusted action's `prepare` subcommand is contractually non-mutating) found the true cause: the action's v1 PREPARE contract legitimately defines three outcomes — `ready`, `upToDate`, `blocked` — but WU102's parser (`domain::managed_workload_prepare::parse_prepare`) only ever accepted `status: "ready"`, rejecting the other two as malformed. Since PI 5's repository was already synchronized to `origin/main` at `e16a31d1…` (both `currentRevision`/`targetRevision` equal, `changeCount: 0`), the action correctly emitted `upToDate`, which the parser had no way to represent. Running the action directly, read-only, as the same non-root deploy user confirmed the exact well-formed `upToDate` response the app was rejecting. PI 5, the trusted action, and the device's SSH/trust path all behaved correctly throughout — this was purely a Pi-Hub-side contract gap, classified as **an operator-validation-discovered Pi-Hub contract defect** (not a PI 5, transport, or trust failure), now remediated.

**Remediation** — `8db551a369b8d8d796281acc9d503f0f3301b084` ("fix(WU105): support complete workload prepare outcomes"). The parser now returns a strict `PreparedOutcome::{Ready, UpToDate, Blocked}` discriminated union (contradictory `upToDate` evidence and unknown `blocked` reasons still fail closed as malformed); orchestration only persists a deployable `ManagedWorkloadOperation` for `Ready`; the Tauri API returns a typed `{"outcome":"prepared"|"upToDate"|"blocked"}` response with no arbitrary action text; the UI opens the confirmation dialog only for `prepared`, and shows fixed, non-error copy for `upToDate`/`blocked`. Deterministic validation: `cargo test --lib` 327/327 passed, `cargo check` clean, `npx tsc --noEmit` clean, `npx vitest run` 66/66 passed, `npm run build` succeeded, `git diff --check` clean (LF/CRLF notices only).

**Real corrected retry** — performed manually by the operator through the live Tauri UI against PI 5. Performance Diagnostics was started first and kept running through the attempt:

| Diagnostics metric | Value |
| --- | --- |
| Session duration | ~19.8 s |
| `ssh.execute` | 2 calls, 660 ms total, 346 ms max, 0 failures |
| `tauri.get_latest_snapshot` | 3 calls, 3 ms total, 1 ms max, 0 failures |
| Resident memory | ~35.8–36.0 MB |
| Process CPU | negligible |
| Diagnostics warnings | none |

`Prepare update` was clicked once. The corrected backend returned `UpToDate`; the UI showed "Personal Finance is up to date." with no error and no confirmation dialog; Performance Diagnostics was stopped and exported cleanly afterward.

**Post-attempt persistence proof** — `managed-workload-operations.json` does not exist at all (no operation, for this or any prior PREPARE attempt, has ever been persisted for this workload) — consistent with the code path: `UpToDate` returns before any `ManagedWorkloadOperation` is constructed. No `operationId` requiring confirmation and no continuation/APPLY eligibility exists. `managed-workloads.json` registration is unchanged and still valid.

**Post-attempt zero-mutation proof (PI 5, read-only)** — repo `HEAD` unchanged at `e16a31d11dc5097086529a1ccd5bae84b1dc1630`, `main == origin/main`, worktree clean; no `pihub-workload-*` transient unit exists; trust action (owner `root:root`, mode `755`, digest `0cc3110b…`) and override (owner `root:root`, mode `644`, digest `c96e1365…`) both unchanged, no symlink substitution; container identical (ID `a3f710774b48…`, image `sha256:c8706708…`, `RestartCount: 0`, unchanged `StartedAt`), bound `100.116.232.83:3010 → 3000`, `HTTP 200`; local `activity.json` has no new deployment-related event (latest entry remains from 2026-08-13, predating this session).

**Compiler-warning finding (non-blocking, unmodified this turn)** — `npm run tauri dev` surfaced two pre-existing unused-variant warnings unrelated to this fix: `RuntimeVerificationError::Internal` and `FinalVerificationError::Internal` are never constructed. Recorded as a small future cleanup/closure candidate (remove the variant if genuinely unreachable, or wire a call site if the architecture intends it to be reachable) — not suppressed with `#[allow(dead_code)]`, and out of scope for WU105.

WU105 remains active; this is registration, remediation, and Stage C validation evidence only, not a checkpoint.

## Stage D preparation: operator-validation-discovered update-discovery defect

Before any Stage D APPLY, a second real operator validation found a false `UpToDate`: PI 5 had a clean worktree and `HEAD` plus local `refs/remotes/origin/main` at `e16a31d11dc5097086529a1ccd5bae84b1dc1630`, while the authoritative fixed `origin` `refs/heads/main` result from `git ls-remote origin refs/heads/main` was `c1f5c492717198629000279ac9007820032cc6ad`. The real UI nevertheless said “Personal Finance is up to date.” No APPLY, manual `git fetch origin`, checkpoint, or WU106 start occurred.

**Root cause and safety decision** — the installed root-owned adapter's PREPARE derived its target solely from the stale local remote-tracking ref, and its APPLY refreshed that ref. A remote-tracking ref is cache state, so this prevented Pi-Hub from autonomously discovering a newer workload revision. This is classified as an **operator-validation-discovered update-discovery defect**. The replacement adapter treats only a fixed, non-interactive `git ls-remote origin refs/heads/main` result as authority: exactly one `refs/heads/main` line with a lowercase 40-hex SHA is required. Malformed, missing, multiple, authentication, transport, timeout, and exact-fetch failures fail closed as `repositoryUnavailable`.

**Revised PREPARE boundary** — after clean fixed-policy/local-HEAD checks, PREPARE compares the authoritative SHA to `HEAD`. Equality returns canonical `upToDate` without fetching. Otherwise it performs only `git fetch --no-tags --no-write-fetch-head origin <validated-sha>`, proves that exact commit exists, enforces `HEAD → target` ancestry, calculates the bounded count, and emits `ready` with that remote SHA. This exact object-database population is the sole permitted PREPARE metadata effect; it does not update `refs/remotes/origin/main`, `FETCH_HEAD`, the current branch, or the worktree, and it does not invoke Docker or systemd.

**Revalidation** — the existing backend’s confirmation path executes a fresh PREPARE and compares its consent fingerprint before creating a transient unit. Consequently, a moved remote produces a different exact target/fingerprint and returns `PlanChanged` with no dispatch. The adapter also performs fresh discovery and exact-target equality immediately before switch/build/Compose, so a direct action invocation stops before mutation if main moved.

**Deterministic remediation validation** — `scripts/test-personal-finance-action.sh` runs the artifact against a fixed fake Git/Docker boundary. It proves stale cached `origin/main` cannot influence equal/ahead results; only an exact no-write-fetch-head target object is fetched; malformed/missing/multiple/auth/transport/timeout discovery and object-fetch failure are fail closed; divergence and the 10,000 commit cap block; PREPARE never switches or references/updates the tracking ref; and moved revalidation stops before Docker while an unchanged exact target is eligible to switch/build/Compose. The script completed successfully under Git for Windows Bash. `git diff --check` completed cleanly (existing CRLF notices for AIQT state only).

**Trusted artifact status** — the reviewed LF/no-BOM source artifact is `scripts/trusted-actions/personal-finance`, SHA-256 `14c5ca0e32911d6efd12ea3bfec84be4768cd5ddd187208d91dae96486f28079`. Its exact diff and deterministic action suite were reviewed; managed-workload regression coverage passed and frontend regression coverage passed 66/66. The live PI 5 SCP/install replacement is deferred pending explicit authorization for this persistent root-owned executable change; no upload, replacement, PREPARE, or APPLY occurred.

## Authorized trusted-action replacement and validation

With explicit owner authorization, the reviewed artifact was transferred over the previously qualified strict-host-key SCP transport to PI 5 (`joaoadmin@100.116.232.83`; the pinned `known_hosts` entry includes the operator-confirmed ED25519 host key). The staging file was `/tmp/pihub-wu105-personal-finance.JzE3te`.

**Pre-install proof (PI 5)** — the staging path was a regular non-symlink file; a byte-level check found `0` CR bytes (LF-only); its first three bytes were `23 21 2f` (no UTF-8 BOM); `bash -n` passed; and SHA-256 was exactly `14c5ca0e32911d6efd12ea3bfec84be4768cd5ddd187208d91dae96486f28079`.

**Installation and post-install proof (PI 5)** — only the authorized target `/usr/local/libexec/pi-hub/actions/personal-finance` was replaced using `sudo -n install -o root -g root -m 0755` from that verified staging file. The installed path is a regular, non-symlink file, owned `root:root`, mode `0755`, with SHA-256 exactly `14c5ca0e32911d6efd12ea3bfec84be4768cd5ddd187208d91dae96486f28079`. Both parent directories (`/usr/local/libexec/pi-hub` and `/usr/local/libexec/pi-hub/actions`) remained root-owned, non-symlink directories with no group/world write bits. The existing canonical M17 non-mutating trust probe returned `PIHUB_M17_TRUST=trusted` and the same digest.

No canonical expected-digest trust metadata exists in the typed managed-workload schema or trust probe, so `managed-workloads.json` was not modified. No `prepare`, `apply`, `verify`, transient unit, Docker, Compose, repository, sudoers, or override operation was run.

**Workload unchanged proof** — immediately before and after installation, the existing Personal Finance container was the identical running instance: ID `a3f710774b4833c4269355639b3fb10bd1d34ec543976d9b095636374c67f2de`, image `sha256:c8706708defd07edb8320ed72c62b7166ea99effa3fb790d62255a42dfd67b98`, restart count `0`, started at `2026-09-16T07:31:20.899728058Z`. WU105 remains active; this records only the authorized trusted-action replacement and non-mutating trust/capability validation, not a checkpoint or WU106 start.

## Stage D APPLY success and verification-panic remediation

**Single authorized APPLY proof (PI 5)** — the existing persisted Stage D operation dispatched exactly one detached APPLY unit for target `4c1c90d3a20b7696b0210d932329ac4b8f97498b`. Independent post-APPLY evidence recorded `Result=success`, `ExecMainCode=1`, `ExecMainStatus=0`, `ActiveState=active`, `SubState=exited`, and `MainPID=0`. Its bounded action journal ended with `{"protocolVersion":1,"status":"applied","deployedRevision":"4c1c90d3a20b7696b0210d932329ac4b8f97498b"}`. The Personal Finance repository was at the exact detached, clean target; the running container used `pihub-personal-finance:4c1c90d3a20b7696b0210d932329ac4b8f97498b`, with restart count `0`, and both the running and inspected target-tag image IDs were `sha256:602d663f247b9b0e296fceeaba52215cedd3dce5fc63faef31d981cb9eb5aec5`. OCI revision was the exact target.

**Verification panic root cause** — the persisted-operation path is `reconcile_managed_workload_deployment` → `reconcile_managed_workload_with` → detached-unit classification → exact revision verification → Docker/Compose runtime verification → `finalize_workload_verification` → `check_service`. The async Tauri command directly called the synchronous reconciler; final verification constructed and dropped `reqwest::blocking::Client` in `check_service` while still on Tokio's async worker. Reqwest therefore panicked on runtime drop before fresh Service Health could complete.

**Remediation** — the reconcile command now runs only the existing persisted-operation reconciler on `tauri::async_runtime::spawn_blocking`. The final verifier, exact revision gate, Docker/Compose gate, 120-second bounded fresh Service Health loop, typed failure outcomes, and persisted lifecycle are unchanged. The blocking closure receives only the persisted status request and app storage path and creates an `OpenSshExecutor`; it does not call prepare, consent revalidation, detached dispatch, APPLY, or retry. Consequently reconciliation can only query the same persisted operation and its already-persisted transient unit.

**Deterministic evidence** — a local-listener Tokio regression test executes a real fresh `reqwest::blocking` Service Health request through Tauri's blocking pool and passes without a runtime-drop panic. The managed-workload library suite also covers fresh Service Health completion, health failure and bounded timeout, mandatory exact-revision/Docker gates, and persisted-unit reconciliation without redispatch. `cargo check` passed. `cargo test --lib` completed 325 passing, 1 ignored, and the three known independent activity-retention fixture failures caused by dated `2026-08-10` records. Frontend tests passed 66/66 and the production frontend build passed. `git diff --check` passed (only existing LF/CRLF notices). Repository-wide `cargo fmt --check` reports unrelated pre-existing formatting drift and was not used to rewrite unrelated files.

No second APPLY, PREPARE, new operation, new transient unit, auto-retry, or live reconciliation was performed during this remediation. WU105 remains active and is not checkpointed; WU106 has not started. Before an owner invokes Check status, repeat the read-only PI 5 checks for the exact detached clean HEAD, successful exited known unit, exact running/tagged image and OCI revision, and an independent successful Service Health endpoint response; stop on any drift.

## WU105 closure: final Stage D reconciliation

The authorized Stage D deployment and the post-fix reconciliation have now completed successfully. This section consolidates the canonical operator evidence; it intentionally records identifiers and outcomes needed for audit, but does not retain raw SSH, diagnostic, credential, or service-response payloads.

| Closure item | Canonical evidence |
| --- | --- |
| Approved/deployed target | `4c1c90d3a20b7696b0210d932329ac4b8f97498b` |
| Persisted systemd unit | `pihub-workload-8ebaa31bcf0148eeb2d63121e84ee865.service` |
| Final unit state | `Result=success`, `ExecMainStatus=0`, `ActiveState=active`, `SubState=exited`, `MainPID=0` |
| Action result | `status=applied`, with `deployedRevision` exactly equal to the approved target |
| Repository identity | `HEAD` exactly target, detached HEAD, clean worktree |
| Runtime identity | The running container uses the exact target-tag image; its ImageID equals the separately inspected target-image ID; OCI `org.opencontainers.image.revision` equals the exact target; `RestartCount=0`; `Running=true` |
| Service Health | HTTP 200 was observed before reconciliation and final Pi-Hub Service Health reconciliation completed successfully |

The Stage D review/confirmation preceded the one authorized APPLY. The persisted operation is the sole deployment record and the unit above is the sole transient unit used for it. The action completed successfully, then Pi-Hub was restarted/hot-reloaded after the verification-panic remediation (`de9832fca1901e6821cf7f97f83c42fd468cc9e1`, `fix(WU105): avoid blocking runtime panic during deployment verification`). Pi-Hub recovered the existing persisted operation, queried the same unit, and completed final verification successfully.

**No-second-dispatch proof** — recovery performed no PREPARE, no consent revalidation, no APPLY, no new operation creation, and no new systemd-unit creation. It reused the same persisted operation and `pihub-workload-8ebaa31bcf0148eeb2d63121e84ee865.service`; the final action/repository/image/OCI and Service Health evidence above all reconcile to the original approved target. Therefore exactly one APPLY was dispatched for Stage D.

**Operator-discovered defects remediated in WU105**

1. Incomplete PREPARE outcome handling: valid `upToDate`/`blocked` outcomes were treated as malformed. Remediated in `8db551a369b8d8d796281acc9d503f0f3301b084`.
2. Stale remote update discovery: PREPARE trusted a cached remote-tracking ref and could falsely report up to date. Remediated in `2e96e13` by using bounded authoritative remote discovery and exact-target preparation.
3. Blocking reqwest/Tokio verification panic: final Service Health verification dropped a blocking client on an async worker. Remediated in `de9832fca1901e6821cf7f97f83c42fd468cc9e1` by using the blocking pool for persisted-operation reconciliation.

**Performance Diagnostics** — the available Stage C export showed a short (~19.8-second) session, two SSH executions (660 ms total; 346 ms maximum), three snapshot reads (3 ms total), ~35.8–36.0 MB resident memory, negligible CPU, and no diagnostics warnings. No sensitive raw export data is stored here.

### Closure residuals carried forward

- A user in the Docker group can spoof local Docker/image metadata within the present device privilege boundary; the exact Git, systemd, action, and OCI checks reduce but do not eliminate that local-privilege limitation.
- PI 5 has a pre-existing broad `NOPASSWD` sudo capability. It was not widened or changed by WU105; device-hardening review is outside this work unit.
- Three activity-retention tests with dated `2026-08-10` fixtures remain independent test debt. They are not caused by WU105 and must be carried into WU106 closure/remediation rather than silently ignored.
- `RuntimeVerificationError::Internal` and `FinalVerificationError::Internal` remain unused Internal enum variants if the final compiler output still reports them. They are existing warnings, were not suppressed, and are out of WU105 scope.

WU105 is accepted for closure: discovery and baseline, trusted-action provisioning, host-neutral Compose prerequisite, read-only Deploy Key, Stage B registration/trust, Stage C UpToDate validation, all three remediations, Stage D review/confirmation and one APPLY, exact deployment/runtime identity, Service Health, and same-operation final recovery are evidenced above. WU106 has not started.

### Final closure validation (2026-09-16)

- `cargo test managed_workload --lib`: passed, 66/66 managed-workload tests; this includes final Service Health, exact revision/runtime identity, persisted-unit reconciliation, and no-redispatch coverage.
- `cargo check`: passed. The only warnings are the two carried-forward unused `Internal` enum variants named above.
- `npx tsc --noEmit`: passed.
- `npm test`: passed, 13 files / 66 tests.
- `npm run build`: passed.
- `git diff --check`: passed; Git emitted only the repository's LF-to-CRLF notice for this evidence file.
- `cargo test --lib` and the one required `node scripts/validate-agent.mjs` run: not fully green solely because `storage::activity_repository::{append_round_trips_newest_first,device_query_preserves_global_history_and_isolates_device_identity,pruning_removes_expired_and_excess_events}` use dated `2026-08-10` fixtures. Result: 325 passed, 3 failed, 1 ignored. The failures are pre-existing activity-retention fixture debt, outside WU105's managed-workload changes, and are explicitly assigned as WU106 carry-forward remediation rather than waived or suppressed.
