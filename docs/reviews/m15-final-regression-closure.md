# M15 - Final Regression and Closure Review

## Deterministic gates

| Gate | Result | Evidence |
| --- | --- | --- |
| Supported check uses five fixed SSH operations | PASS | Scripted production orchestration test records `ssh.execute = 5`. |
| Unsupported exits after detection | PASS | Unsupported fixture returns after one execution. |
| Bounded package/hold/kept-back/output behavior | PASS | 201 packages retain exact total, 200 details, and `truncated`; stdout/stderr are hard-capped during SSH capture and oversized evidence is discarded; kept-back is separate from dpkg holds; the C-locale `N not upgraded` count prevents a false up-to-date state; malformed protocol is rejected. |
| Partial/failure semantics | PASS | Optional hold timeout remains partial; malformed required package evidence, transport timeout, and a genuine package-command exit 100 become explicit failure. |
| Latest result persists/reloads | PASS | Temporary atomic JSON repository round-trip is asserted. |
| Same-device protection/different-device independence | PASS | Update claim tests coalesce same device and allow independent IDs. |
| System cache/action gate and no polling | PASS | UI test proves cached local read until explicit action; scheduler contains no update operation. |
| Diagnostics labels/privacy | PASS | Active session captures every `device_updates.*` label and excludes fixture/package/command content. |

## Exit criteria

| Criterion | Assessment |
| --- | --- |
| Read-only supported-device check | PASS |
| Explicit unsupported/failed/unknown state | PASS |
| Metadata freshness and bounded details | PASS |
| Held/reboot/security contracts | PASS |
| Restart-persistent latest result | PASS |
| No automatic update polling; System placement/gating | PASS |
| Diagnostics and deterministic fixture coverage | PASS |
| Real-device validation | PASS with distro residual |
| Canonical validation | PASS - closure canonical validation passed; the operator-evidence correction additionally passed focused Rust, frontend, type, and diff checks. |

## Risk assessment

**30/100 - Yellow.** The remote surface remains fixed and read-only, with deterministic bounds, partial isolation, explicit concurrency protection, corrected regression evidence, and successful validation on both available real Pi-Hub devices. The remaining risk is limited to distro coverage: no separate plain-Debian operator device was validated.

## Release-readiness correction

The final review found that the documented 256 KiB stdout and 16 KiB stderr limits were applied after OpenSSH had buffered output. The shared OpenSSH execution path now reads both streams concurrently, terminates the child when either limit is exceeded, returns a typed bounded-output failure, and discards oversized evidence before the update result can be persisted or presented. Focused process, update-orchestration, diagnostics, and compile validation passed. The risk remains **30/100 - Yellow**: the correction removes the capture-bound implementation defect; the only remaining risk is optional plain-Debian coverage.

## Completed real-device evidence

Operator validation completed on both available real Pi-Hub devices. On each device, the operator verified the concise System Updates UX, explicit Check for Updates only, loading/Checking feedback, successful read-only APT simulation, update counts and deferred packages, View updates installed-to-candidate versions, persistence after restart, absence of automatic/background polling, and Performance Diagnostics. No package mutation, `apt update`, reboot, install, upgrade, or remove was performed.

The evidence confirms the supported Raspberry Pi OS path on both available devices. It does not constitute separate plain-Debian validation; a plain-Debian device remains optional follow-up coverage if one becomes available and is not a merge blocker.
