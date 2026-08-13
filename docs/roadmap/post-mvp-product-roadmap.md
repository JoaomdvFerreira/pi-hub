# Pi-Hub post-MVP product roadmap

Version: 0.4 planning
Status: M1-M15 implemented; M16 WU16-02 complete
Date: 2026-08-13

## Authority

This document is the roadmap-level source for milestone history and sequencing. Current product behavior is maintained in [the current product contract](../current-product-contract.md); approved requirements take precedence for durable normative behavior; milestone documents are historical scope and closure records.

## Delivered milestones

| Milestone | Status | Delivered focus |
| --- | --- | --- |
| M1-M5 | Implemented | Desktop foundation, device registry, SSH connectivity, baseline monitoring, Docker/service access, tray UX, release foundation. |
| M6 | Implemented | Device Health and diagnostics. |
| M7 | Implemented | Service Health and bounded operational Activity. |
| M8 | Implemented | Alert lifecycle, thresholds, and governed notifications. |
| M9 | Implemented | Docker operational visibility and controlled container lifecycle actions. |
| M10 | Implemented | Controlled Restart Device, Shut Down Device, Restart Docker, and Restart Tailscale actions. |
| M11 | Implemented | Read-only network, storage, and system visibility. |
| M12 | Implemented | Local, bounded historical monitoring for device, service, and container trends. |
| M13 | Implemented | Documentation-governance reconciliation; final planned milestone. |
| M14 | Implemented | Performance benchmarking, runtime hardening, and Device Detail tab UX. |
| M15 | Implemented — operator validation recorded | Device Update Intelligence & Maintenance Readiness; read-only, manual-check-first. |
| M16 | In progress — WU16-02 complete | Controlled Device Software Updates; explicit, safety-governed APT standard upgrades. |

The approved scope for M6-M14 is recorded in [milestone documents](../milestones/). M10's shutdown-specific requirement remains [the controlled shutdown requirement](../requirements/controlled-device-shutdown-requirement.md).

## Product boundaries

Pi-Hub is a lightweight desktop control center for managed Linux devices and the infrastructure hosted on them. It may observe health and visibility data, perform bounded service checks, retain bounded local Activity/Alerts/history, and invoke its implemented closed set of confirmed administration operations.

It will not become a general SSH client, file manager, Docker Compose editor, arbitrary remote-command system, or generic infrastructure-orchestration platform. It does not store SSH/sudo secrets, expose Docker over TCP, automatically trust changed SSH host keys, or support Power On, Wake-on-LAN, smart-plug, PoE, GPIO, or hard-power control.

## Documentation and planning rules

Human-readable product, requirement, and milestone documentation belongs under `docs/`. AIQT is the canonical structured execution/workflow state in `.aiqt/`. They must agree on materially represented milestone identity, scope, and status without duplicating complete Markdown documents into the workflow state.

M13 was the final milestone in the prior roadmap. M14 and M15 are implemented. M15 operator validation completed successfully on both available Pi-Hub devices; separate plain-Debian coverage remains optional and is documented in its final closure review. M16 has completed WU16-01 architecture and WU16-02 coordination/state/persistence only; no APT metadata refresh or package mutation is authorized or implemented. M17 has not started.
