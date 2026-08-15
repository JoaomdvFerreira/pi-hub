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

WU105 remains active; this is registration and validation evidence only, not a checkpoint.
