# M15 WU15-04 — Deterministic Validation Evidence

The update orchestration is exercised with the production `RemoteExecutor` and `SnapshotRepository` contracts using a scripted executor and temporary atomic JSON repository. This does not contact a device or alter package state.

| Scenario | Result |
| --- | --- |
| Supported zero updates | `upToDate`, five fixed executions, persisted/reloaded. |
| Available/truncated updates | 201 records preserve total, retain 200 details, set `truncated`; stale metadata yields `stale`. |
| Unsupported platform | `unsupported` after detect only (one execution). |
| Holds/reboot/partial optional evidence | Holds join to package data; held timeout becomes warning while valid package result survives; reboot required is retained. |
| Required malformed/timeout evidence | `checkFailed` with malformed-output or timeout failure. |
| Diagnostics | Active deterministic session records `ssh.execute` count 5 and every `device_updates.*` label. Result-byte aggregates are non-zero for the complete result; labels contain no fixture host, package, or command text. Scripted execution duration is 5 ms per remote result; this is a fixture property, not a live performance threshold. |

The check model is fixed: detect, packages, holds, metadata age, reboot state (five SSH executions) for supported devices; unsupported devices stop after detect. No monitoring scheduler path calls update operations. `get_update_result` is local persistence only; the System UI issues a remote check only after the explicit action. Same-device claims return `AlreadyChecking`; different device IDs remain independent.

No configured, safe real device target is present in this workspace. Before merge, an operator must run one explicit read-only check on a reachable Raspberry Pi OS device and one Debian device, verify natural zero/available/held/metadata/reboot states as present, restart Pi-Hub to confirm cached state, attempt a duplicate click, and export Diagnostics to confirm the label/count/byte report. Do not run `apt update`, modify holds, install, upgrade, remove, or reboot.
