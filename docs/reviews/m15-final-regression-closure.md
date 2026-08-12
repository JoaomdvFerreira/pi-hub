# M15 — Final Regression & Closure Review

## Deterministic gates

| Gate | Result | Evidence |
| --- | --- | --- |
| Supported check uses five fixed SSH operations | PASS | Scripted production orchestration test records `ssh.execute = 5`. |
| Unsupported exits after detection | PASS | Unsupported fixture returns after one execution. |
| Bounded package/hold/output behavior | PASS | 201 packages retain exact total, 200 details, and `truncated`; malformed protocol is rejected. |
| Partial/failure semantics | PASS | Optional hold timeout remains partial; malformed required package evidence and transport timeout become explicit failure. |
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
| Real Raspberry Pi OS/Debian validation | PASS with operator residual |
| Canonical validation | PASS — Rust check/test/build, frontend test/build/lint, AIQT status, and diff check passed. |

## Risk assessment

**32/100 — Yellow.** The remote surface remains fixed and read-only, with deterministic bounds, partial isolation, explicit concurrency protection, and regression evidence. The remaining risk is unperformed read-only validation against real Raspberry Pi OS and Debian hosts.

## Operator residual before merge

On one reachable Raspberry Pi OS device and one Debian device, run only `Check for Updates`; verify native support state, naturally occurring zero/available/held/metadata/reboot outcomes, cache after restart, duplicate-action response, and Diagnostics export. Do not run `apt update`, mutate packages/holds, or reboot.
