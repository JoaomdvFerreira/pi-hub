# M15 Operator APT Evidence Correction

## Root cause and correction

Raspberry Pi OS operator validation established that the frozen packages command `LC_ALL=C apt-get -s --no-download upgrade` exits 100 with `Unable to fetch some archives`. The `--no-download` flag is therefore not a valid supported-platform evidence requirement, even though the APT simulation itself is read-only.

The fixed command is now `LC_ALL=C LANG=C apt-get -s upgrade`. It retains simulation mode and fixed C-locale parsing, and does not use sudo, `apt update`, package mutation, or reboot. Existing command/output/time/detail bounds remain unchanged.

The C-locale protocol now records both the kept-back package block and the authoritative summary `N not upgraded` count. Kept-back/deferred candidates are separate from explicit dpkg holds. A successful result is `upToDate` only when normal upgrades and the kept-back count are both zero; otherwise it is `updatesAvailable` unless stale metadata takes precedence.

## Deterministic correction evidence

- Fixed-operation test verifies `apt-get -s upgrade`, excludes `--no-download`, and requires kept-back-count extraction.
- Production orchestration fixtures cover normal upgrades with kept-back names, zero normal upgrades with kept-back names, summary-only kept-back count, independent dpkg holds, package-detail bounds, persistence/reload, malformed required package evidence, timeout, and genuine exit-100 package failure.
- Existing diagnostics, explicit-action, cache, scheduler, and concurrency evidence remains applicable because the operation count and typed execution path are unchanged.

## Required operator retest before merge

On a reachable Raspberry Pi OS device, use only the Software Updates **Check for Updates** action. Confirm it completes using `apt-get -s upgrade`, reports the observed normal-upgrade count and any kept-back/deferred count separately from explicit dpkg holds, and does not claim up to date when the summary reports packages not upgraded. Confirm cached reload and Performance Diagnostics export. Repeat one read-only check on Debian when available. Do not run `apt update`, alter package state or holds, install, upgrade, remove, or reboot.
