# Pi-Hub — M10 Controlled Device Administration

Version: 0.1  
Status: Implemented  
Date: 2026-08-10  
Depends on: M9 — Docker Operational Visibility  
Related requirement: [`controlled-device-shutdown-requirement.md`](../requirements/controlled-device-shutdown-requirement.md)

## Authority, objective, and scope

This is the canonical approved M10 milestone document. It adds only four deliberate, typed, safety-governed device administration operations: Restart Device, Shut Down Device, Restart Docker, and Restart Tailscale. It does not make Pi-Hub a remote command console. The shutdown requirement remains normative for shutdown-specific behavior and is reconciled through the same confirmation, noninteractive privilege, verification, planned-offline, Activity, and notification rules described here.

The frontend supplies a device ID and a closed operation enum only. Every action has a dedicated typed Tauri/backend path and one predefined SSH command. No raw shell text, `systemctl` unit name, sudo argument, password, private-key content, user script, or arbitrary service command is accepted. Existing mandatory SSH host-key verification remains in force.

Privileged commands use noninteractive `sudo -n`. Missing privilege fails quickly with an actionable typed result; Pi-Hub never prompts for, captures, retries with, or stores a sudo password. Dispatch and verification are bounded, and a failure for one device does not block another device. Activity never records raw commands, credentials, or unbounded SSH output.

Out of scope: Power On; Wake-on-LAN; smart plugs, PoE, GPIO, or hard power cuts; arbitrary service restart; arbitrary shell/remote execution; scheduling, batches, custom scripts; OS/package updates; Docker create/delete/Compose administration; and M11 visibility work.

## Administration model and conflict handling

The domain records operation ID, device ID, type, requested/accepted/completed timestamps, state, bounded non-sensitive detail, failure classification, and expected-disruption context. States are `Requested`, `Dispatching`, `CommandAccepted`, `Verifying`, `WaitingForOffline`, `WaitingForOnline`, `Completed`, `Failed`, `TimedOut`, and `OutcomeUncertain`.

Only one M10 operation may run for a device at one time. Backend conflict validation is authoritative; the UI disables all conflicting actions. Other devices remain independent.

## Expected disruption and alerts

M10 persists an operation-scoped expected-disruption marker where needed for restart recovery. It contains device and operation identity/type, start and absolute expiry, and phase. Invalid or stale records are discarded safely on read, so a restart can never create permanent suppression.

Default safety bounds are: Restart Device 10 minutes; Shut Down Device 24 hours; Restart Docker 5 minutes; Restart Tailscale 5 minutes. Restart Device, Docker, and Tailscale clear early on verified recovery. A shutdown marker remains after observed offline completion until the device returns online or the 24-hour bound expires.

M8 remains the only alert lifecycle authority. During a valid marker M10 suppresses/defer only the new unexpected-device-offline path attributable to the controlled operation. Existing or unrelated health, service, container, and alert conditions remain governed normally. Observed state is retained as evidence; expiry never fabricates health. M7 Activity is the durable audit history and records requested, accepted, completed, failed, timed-out, and uncertain transitions with concise non-sensitive summaries.

## Operation behavior

### Restart Device

Confirmation says that containers/services stop temporarily, terminal/SSH may disconnect, and Pi-Hub waits for online recovery. The fixed command is equivalent to `sudo -n systemctl reboot`. Command acceptance alone is not success: Pi-Hub expects offline/unreachable evidence, then bounded online recovery. A dispatch disconnect is an uncertain outcome followed by safe bounded verification; successful restart requires observed return online.

### Shut Down Device

Confirmation is destructive and states that services/containers stop, SSH sessions end, and **Pi-Hub cannot turn the device back on**; power-on is manual or external infrastructure. The fixed command is equivalent to `sudo -n systemctl poweroff`. Success requires offline evidence within the bounded verification period and does not wait for online return. The resulting state is planned offline, not an unexpected outage, until valid marker expiry or observed return. No Power On capability is added.

### Restart Docker and Restart Tailscale

Docker confirmation warns that containers and hosted services may stop/restart, then Pi-Hub verifies Docker availability through the existing M9 refresh path after the fixed equivalent of `sudo -n systemctl restart docker`. Tailscale confirmation warns that connectivity and a Tailscale SSH route may disconnect, then Pi-Hub performs bounded recovery after the fixed equivalent of `sudo -n systemctl restart tailscaled`. Command exit success alone is insufficient for either action. Lack of verification becomes timeout/uncertain, never fabricated success.

## Work Units

| Work Unit | Delivered scope |
| --- | --- |
| WU10-01 | Typed administration domain, fixed commands, `sudo -n`, bounded dispatch, per-device guard, and failure classification. |
| WU10-02 | Restart Device confirmation, offline→online verification, expected disruption, M8 suppression, Activity, persistence and expiry. |
| WU10-03 | Controlled shutdown confirmation/no-Power-On boundary, offline completion, planned-offline persistence, expiry and online-return clearing. |
| WU10-04 | Narrow Docker and Tailscale restarts with operation-specific confirmation, expected disconnect handling, bounded verification, Activity, and attributed suppression. |
| WU10-05 | Device Details administration UI, conflict/busy/result presentation, planned-offline presentation, documentation, AIQT and closure validation. |

## Validation and closure

Automated coverage includes fixed command catalogue/noninteractive privilege classification, expected-disruption persistence and fail-safe expiry, M8 scoped offline suppression with unrelated service alert preservation, frontend action catalogue and Power-On exclusion, and the repository frontend and Rust test suites. Live Restart Device, Shut Down Device, Restart Docker, and Restart Tailscale validation is intentionally not performed without explicit authorization; it remains residual manual validation.

## Historical status note

M10 is complete. Its M11-M13 planning statements describe the status at M10 closure; current milestone status is maintained in the [roadmap](../roadmap/post-mvp-product-roadmap.md).

At M10 closure, M11-M13 were planned/not started. This historical record does not claim later milestone status; M13 Document Hygiene remains outside M10 scope.
