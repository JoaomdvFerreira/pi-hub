# M17 — Managed Workload Action Safety Contract

Status: Canonical WU17-01 repository contract

Pi-Hub invokes only the backend-derived `/usr/local/libexec/pi-hub/actions/<action-id>` as `prepare`, `apply <validated revision>`, or `verify <validated revision>`. It accepts no executable/command/path/ref/environment/secret/sudo/systemd/additional argument. `actionId` is `^[a-z0-9][a-z0-9-]{0,63}$`; root-owned non-writable directories and root-owned executable regular non-symlink action are SHA-256 checked before PREPARE and APPLY. Pi-Hub never modifies actions.

Actions run detached as the configured non-root device user; `sudo -n` only manages fixed transient units. They require no TTY, SSH agent, frontend environment, or Pi-Hub secret. Protocol v1 is bounded typed JSON: PREPARE is non-mutating and returns ready/upToDate/fixed-enum blocked; APPLY rechecks safety and deploys exactly reviewed ID; VERIFY returns exact deployed ID. Adapters own fixed repo/ref/Compose policy, reject dirty/diverged state without destructive repair, and never update OS, reboot, restart Docker, mutate unrelated workloads, retry, or rollback.

Consent fingerprints versioned workload configuration, action digest, revisions, and verification targets. APPLY repeats trust/PREPARE; drift means `PlanChanged` and zero dispatch. A persisted-before-dispatch generated unit is attempted at most once. Uncertainty, restart, expiry, and Check status reconcile only; they never retry or kill. Completion requires unit success, exact action verification, required stable Compose service labels running/healthy, and required Service Health checks. Raw output is bounded then discarded, never persisted or emitted. Disruption must be workload-scoped or absent.
