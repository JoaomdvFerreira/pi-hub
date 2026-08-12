# M15 - Final Regression and Closure Review

## Deterministic gates

| Gate | Result | Evidence |
| --- | --- | --- |
| Supported check uses five fixed SSH operations | PASS | Scripted production orchestration test records `ssh.execute = 5`. |
| Unsupported exits after detection | PASS | Unsupported fixture returns after one execution. |
| Bounded package/hold/kept-back/output behavior | PASS | 201 packages retain exact total, 200 details, and `truncated`; kept-back is separate from dpkg holds; the C-locale `N not upgraded` count prevents a false up-to-date state; malformed protocol is rejected. |
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
| Real Raspberry Pi OS/Debian validation | PASS with correction retest residual |
| Canonical validation | PASS - closure canonical validation passed; the operator-evidence correction additionally passed focused Rust, frontend, type, and diff checks. |

## Risk assessment

**38/100 - Yellow.** The remote surface remains fixed and read-only, with deterministic bounds, partial isolation, explicit concurrency protection, and corrected regression evidence. Raspberry Pi OS operator evidence invalidated the original `--no-download` simulation; the replacement command and kept-back state require one read-only operator retest before merge.

## Operator residual before merge

On one reachable Raspberry Pi OS device and one Debian device, run only `Check for Updates`; verify `apt-get -s upgrade` completes without `--no-download`, normal and kept-back counts (including any `N not upgraded`) do not yield a false up-to-date status, explicit dpkg holds remain separate, cache after restart, duplicate-action response, and Diagnostics export. Do not run `apt update`, mutate packages/holds, or reboot.
