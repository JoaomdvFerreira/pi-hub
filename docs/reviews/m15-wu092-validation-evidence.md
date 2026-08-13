# M15 WU15-04 — Deterministic Validation Evidence

The update orchestration is exercised with the production `RemoteExecutor` and `SnapshotRepository` contracts using a scripted executor and temporary atomic JSON repository. This does not contact a device or alter package state.

| Scenario | Result |
| --- | --- |
| Supported zero updates | `upToDate` only when normal and kept-back counts are zero; five fixed executions and persisted/reloaded. |
| Available/truncated updates | 201 records preserve total, retain 200 details, set `truncated`; stale metadata yields `stale`. The real-style `apt-get -s upgrade` protocol records normal upgrades, the kept-back block, and its `N not upgraded` summary count. Kept-back packages are independently bounded and make a zero-normal-upgrade result `updatesAvailable`. |
| Unsupported platform | `unsupported` after detect only (one execution). |
| Holds/reboot/partial optional evidence | Holds join to package data; held timeout becomes warning while valid package result survives; reboot required is retained. |
| Required malformed/timeout/non-zero evidence | `checkFailed` with malformed-output, timeout, or genuine non-zero package-command failure. |
| Diagnostics | Active deterministic session records `ssh.execute` count 5 and every `device_updates.*` label. Result-byte aggregates are non-zero for the complete result; labels contain no fixture host, package, or command text. Scripted execution duration is 5 ms per remote result; this is a fixture property, not a live performance threshold. |

The check model is fixed: detect, packages, holds, metadata age, reboot state (five SSH executions) for supported devices; unsupported devices stop after detect. The package simulation is `LC_ALL=C apt-get -s upgrade`: Raspberry Pi OS operator evidence established that `--no-download` causes an invalid exit-100 failure while this simulation remains read-only and succeeds. No monitoring scheduler path calls update operations. `get_update_result` is local persistence only; the System UI issues a remote check only after the explicit action. Same-device claims return `AlreadyChecking`; different device IDs remain independent.

Operator validation subsequently completed on both available real Pi-Hub devices. Each check remained read-only and confirmed the explicit action, successful APT simulation, counts and deferred packages, bounded View updates details, persistence after restart, no automatic/background polling, and Performance Diagnostics. Neither device was a separate plain-Debian target; that coverage remains optional follow-up only. No `apt update`, hold mutation, install, upgrade, remove, or reboot was performed.
