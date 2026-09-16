# M17 final review and Windows release readiness

Date: 2026-09-16

## Closure result

M17 is accepted for closure. WU100-WU106 are evidenced in their AIQT checkpoints and the prior WU105 Personal Finance report. The closure change makes the three legacy activity-retention fixtures date-independent and removes the two genuinely unreachable M17 `Internal` error variants; it does not alter retention or deployment behavior.

## Work-unit evidence

- WU100 froze the managed-action contract and Personal Finance adapter boundary.
- WU101 implemented typed registry persistence and fixed-root action trust.
- WU102 implemented typed PREPARE/consent, persisted-before-dispatch exactly-once execution, and same-unit reconciliation.
- WU103 completed exact revision, Docker/Compose, fresh Service Health, terminal Activity, and fail-closed verification gates.
- WU104 added constrained review/confirmation, recovery display, and bounded UI-owned observation.
- WU105 performed real Personal Finance dogfood through one authorized APPLY, exact runtime and Service Health verification, and same-operation post-fix recovery.
- WU106 closed regression debt, performed this review, and assessed the post-merge Windows release procedure.

## Architecture and security review

The implementation remains aligned with the frozen M17 contract. The persisted operation/unit is written before dispatch and deterministic tests cover one dispatch, post-attempt uncertainty, restart/expiry reconciliation, no retry, no redispatch, and no remote-unit kill. PREPARE releases coordination before review; APPLY reacquires it and repeats trust/PREPARE. Trust remains backend-derived from the fixed action root with root-owned, non-symlink, non-group/world-writable path checks and digest capture. The frontend has only typed DTO/opaque continuation surfaces and cannot supply a command, path, unit, target, environment, or secret.

PlanChanged remains terminal before dispatch when configuration, action digest, revision, or verification identity differs. Final completion remains gated by successful detached-unit evidence, exact action VERIFY revision, fixed read-only Docker/Compose label evidence, and newly performed bounded Service Health checks; cached health cannot complete an operation. Activity remains sanitized and terminally deduplicated.

## Personal Finance dogfood and corrections

WU105 records the real PI 5 deployment: a single authorized APPLY, successful persisted unit, exact Git/image/OCI/container evidence, independent Service Health success, and post-fix recovery of the same persisted operation without a new PREPARE, unit, or dispatch.

Operator validation found and corrected three Pi-Hub defects: incomplete PREPARE outcome handling (`8db551a`), stale local remote-tracking discovery (`2e96e13`), and a blocking-runtime verification panic (`de9832f`). The real action, strict SSH host-key path, and configured trust boundary were not weakened.

## Final validation

- `cargo test --manifest-path src-tauri/Cargo.toml activity_repository`: passed (3/3).
- `cargo test --manifest-path src-tauri/Cargo.toml managed_workload --lib`: passed (66/66).
- `cargo check --manifest-path src-tauri/Cargo.toml`: passed with no M17 `Internal` dead-code warnings.
- `node scripts/validate-agent.mjs`: Rust check/tests/build, frontend tests/build passed. The wrapper stopped before its lint/status/diff tail in this environment; the remaining canonical stages were run directly: `npm run lint` passed with the four recorded pre-existing Fast Refresh warnings and no errors; `aiqt status` passed; `git diff --check` passed.

## Residuals

- PI 5's pre-existing broad `NOPASSWD` sudo capability and Docker-group metadata spoofing concern remain outside M17 and are not release blockers for this scoped closure.
- The four existing frontend Fast Refresh lint warnings remain unrelated to M17; no M17 Rust warnings remain.
- The required installed-Windows release smoke test is post-merge work; no release was built or published from this branch.

## Release readiness

The current release version is `0.3.1`, aligned in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, and `package.json`; Tauri's configuration is the bundle version source. Windows bundle targets are `all`, which produces the platform-available NSIS installer and/or MSI under `src-tauri/target/release/bundle/`. `createUpdaterArtifacts` is enabled. There is no CI/CD release workflow: release construction and GitHub upload are deliberately manual.

Updater configuration is present: the embedded public key and `latest.json` GitHub Releases endpoint are configured, with passive Windows install mode. Release execution remains conditional on the operator-held private key at the documented path and authenticated `gh`; neither credential was inspected or used here. Existing configuration is expected to survive an in-place installer/update because repositories use Tauri's application config directory (not the install directory), including settings, devices, snapshots, activity, alerts, administration, managed-workload configuration, and operation files. Preserve that directory during any uninstall/reinstall choice.

## Post-merge Windows release procedure

1. After M17 is merged, update the release version consistently in Tauri config, Cargo, and package metadata; commit and tag the versioned mainline change.
2. On the Windows release workstation, pull that exact tag, run `node scripts/validate-agent.mjs`, and verify a clean diff.
3. Set `TAURI_SIGNING_PRIVATE_KEY` to the operator-held key and run `npm run tauri build`. Record the exact emitted NSIS/MSI and updater archive/signature paths; do not substitute guessed filenames.
4. Create `latest.json` from the generated updater signature and exact release URL, then create the matching GitHub Release with the fresh-install installer, updater archive/signature, and `latest.json`, following `docs/releasing.md`.
5. Smoke-test both a fresh install and an in-place update from an older installed Pi-Hub build. Confirm launch/tray behavior, existing devices/settings/history/workload configuration, strict SSH host-key behavior, M17 read-only card/recovery display, and Settings -> Check for updates/download/signature verification/restart/version transition. Retain the installer and updater logs/artifact hashes with the release record.
