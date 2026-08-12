# M15 WU15-01 — APT Read-only Evidence Contract

Status: Frozen design artifact; implementation begins with WU15-02  
Milestone: M15

## Supported platform and capability decision

M15 supports a device only when all required evidence is available:

- `/etc/os-release` identifies `ID=debian` or `ID=raspbian` (including Raspberry Pi OS variants that retain the `raspbian` ID), or `ID_LIKE` contains `debian`;
- `apt-get`, `dpkg-query`, and `dpkg` resolve on `PATH`;
- `apt-get -s --no-download upgrade` completes using the current local APT lists; and
- the fixed output protocol is complete and within bounds.

This is a capability contract, not a distribution-name guess. A Debian-family identity with a missing command, unusable local lists, malformed protocol, or a required-command failure is `Unsupported` or `Check failed` as classified below—never `Up to date`. Other Linux distributions are `Unsupported` without attempting APT inspection.

All commands are fixed literals behind new typed `RemoteOperation` variants. The frontend supplies only a device ID to a typed Tauri action. SSH host-key verification, timeout classification, and the closed-command boundary remain those already used by Pi-Hub.

## Exact remote evidence strategy

Every operation starts with `LC_ALL=C LANG=C` so the parser receives the frozen C-locale grammar. The implementation must emit only `PIHUB_UPDATE_*` records; command stderr and raw output are never persisted or exposed to the UI.

| Operation | Fixed evidence | Classification and isolation |
| --- | --- | --- |
| `detect` | Read `/etc/os-release`; `command -v apt-get dpkg-query dpkg`; emit identity and boolean capability records. | Required. An unsupported identity/capability ends the check as `Unsupported`; a malformed or transport failure is `Check failed`. |
| `packages` | `apt-get -s --no-download upgrade` and parse only C-locale `Inst <package> [<installed>] (<candidate> ... )` records. Package name (including `:arch` where APT emits it), installed version, and candidate version are retained. | Required after detection. Simulation and `--no-download` must not refresh lists, download archives, or change package state. A non-zero exit/malformed required record is `Check failed`. |
| `holds` | `dpkg --get-selections`, retaining records whose selection is exactly `hold`. | Optional. It is the authoritative dpkg selection signal. Failure produces `heldPackages.status = unknown` and a warning, without discarding package evidence. |
| `metadata_age` | `find /var/lib/apt/lists -type f ! -name lock -printf '%T@\\n'`, retaining the maximum numeric mtime; emit `none` when there is no eligible list file. | Optional evidence. The model stores `newestMetadataAt` and computed age at check time. Command/parse failure is `unknown`; `none` is `unavailable`, not fresh. |
| `reboot_state` | Test only for a readable `/var/run/reboot-required` marker. | `Required` when present. Otherwise `Unknown`: Debian/Raspberry Pi OS has no frozen reliable negative marker contract. M15 therefore never reports `Not required` on this platform. |

The package command has a 15-second remote timeout; detect, holds, metadata age, and reboot each have a 5-second timeout. Command execution must be separately bounded and failure-isolated. No `apt`, `apt-get`, or `dpkg` command may receive a caller-provided argument.

## State and data contract

The backend and frontend share camelCase, versioned typed data. Timestamps are RFC 3339 UTC strings. `unknown` means evidence was not determined; it never maps to zero or healthy.

```text
UpdateCheckResult {
  schemaVersion: 1
  deviceId: string
  status: notChecked | checking | upToDate | updatesAvailable | stale | unsupported | checkFailed | unknown
  checkedAt?: rfc3339
  support: { status: supported | unsupported | unknown, osId?: string, osVersionId?: string,
             packageManager?: aptDpkg, reason?: string }
  packageMetadata: { status: fresh | stale | unavailable | unknown,
                     newestMetadataAt?: rfc3339, ageSeconds?: u64, staleAfterSeconds: 604800 }
  updates: { totalCount?: u32, packages: UpdatePackage[], truncated: boolean }
  heldPackages: { status: known | unknown, totalCount?: u32, packages: string[], truncated: boolean }
  reboot: required | notRequired | unknown
  securityUpdates: { status: unavailable }
  warnings: UpdateWarning[]
  failure?: { kind: transport | timeout | requiredCommand | malformedOutput | persistence, message: safeCode }
}

UpdatePackage { name: string, installedVersion: string, candidateVersion: string, architecture?: string, held?: boolean }
```

`checking` is ephemeral UI/action state and is never persisted. `notChecked` is represented by an absent persisted record, rather than a synthetic success. `upToDate` requires a successful required package command with total count zero. `updatesAvailable` requires a positive total. A successful package result with stale metadata uses `stale` as its primary status while retaining the update count and package details. `unknown` is reserved for a completed result whose support cannot be determined without a classified transport/command failure; it cannot show an up-to-date claim.

Held-package membership is joined to returned package details only when both package evidence and the optional hold signal are known. A hold that is not in the bounded detail list still contributes to held total/count and may be omitted from `packages` under normal truncation rules.

## Freshness, reboot, and security decisions

`ageSeconds` is calculated once as `checkedAt - newestMetadataAt`, rounded down and never recomputed when displayed. Metadata is `fresh` below 604800 seconds (7 days), `stale` at or above it, `unavailable` when no list mtime exists, and `unknown` on optional-evidence failure. Freshness describes local index evidence, not time since Pi-Hub last checked.

The reboot value retains all three enum members for cross-platform/model stability. In M15's supported platform contract, only `required` and `unknown` can be emitted; absence of the Ubuntu-style marker is not proof of `notRequired`.

Security classification is explicitly `{ status: unavailable }`. M15 does not infer it from package names, repository strings, or localized command prose. A future classifier needs its own deterministic, supported-OS evidence contract before adding a count or package flag.

## Bounds and latest-result persistence

- Each remote operation accepts at most 256 KiB stdout and 16 KiB stderr; reaching either limit fails that operation with `malformedOutput`/bounded-output evidence and the process is terminated.
- At most 200 update-package details and 200 held package names are returned and persisted. Parsing continues only until the total exceeds the bound; `truncated` is then true and the exact total count is still retained.
- A package field is at most 512 UTF-8 bytes after validation; invalid/oversized records are malformed evidence, not UI text.
- At most one `UpdateCheckResult` is stored per device. It is replaced atomically only after the complete required result is available; optional warnings may be included. There is no historical update analytics or raw-output retention.
- Store results in the existing versioned `state.json` ownership boundary as an additive, serde-defaulted `updateResults` map. Missing/corrupt state recovers to no result under the existing atomic-write/quarantine behavior. A persistence failure returns the in-memory check result as `unknown` with a persistence warning and is not silently treated as stored.

## Instrumentation and deterministic evidence

M15 adds these stable, non-sensitive Performance Diagnostics labels: `device_updates.check`, `device_updates.detect`, `device_updates.packages`, `device_updates.holds`, `device_updates.metadata_age`, `device_updates.reboot_state`, and `device_updates.persist`. Existing `ssh.execute`, `ssh.timeout`, and typed `tauri.*` instrumentation continues to aggregate operation counts, duration, failures, and result bytes. Labels must never include a device ID, hostname, package name, command, or credential.

WU15-02 fixtures must be literal C-locale protocol captures, not a system APT installation. They cover: supported zero updates; one and over-200 updates; update list with held entries; held-signal failure; fresh/stale/no-list/invalid metadata time; required and unknown reboot; unsupported identity/missing command; non-zero command; timeout; malformed/oversized output; state round-trip/corruption; and exact diagnostic labels/bytes. Tests must prove no inactive monitoring path calls an update operation and no arbitrary command is accepted.

WU15-04 operator validation is read-only and uses naturally occurring supported Raspberry Pi OS and Debian devices: verify detection, zero/available updates as present, any naturally held package, metadata age, reboot semantics, persisted result across restart, explicit action behavior, duplicate-action suppression, and diagnostics capture. Do not run `apt update`, alter holds, install packages, or reboot solely to manufacture evidence.
