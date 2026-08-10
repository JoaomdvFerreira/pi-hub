# Pi-Hub â€” Post-MVP Product Roadmap

Version: 0.2  
Status: Approved planning baseline  
Date: 2026-08-09

## 1. Purpose and authority

This document defines the approved post-MVP product direction for Pi-Hub after the completed M1â€“M5 baseline.

It is the roadmap-level source of truth for milestone scope and sequencing. Detailed milestone and requirement documents take precedence for implementation details.

Pi-Hub remains a lightweight desktop control center for managing Raspberry Pi and compatible Linux devices, the workloads running on them, and their operational health.

The product direction is:

> Device inventory + health monitoring + diagnostics + controlled administration + service/container visibility.

Pi-Hub must remain focused on the managed devices and the infrastructure hosted on them. It must not evolve into a general-purpose SSH client, file manager, Docker Compose editor, or home-automation platform.

## 2. Current baseline

M1â€“M5 are considered the completed implementation baseline.

The current application already provides, at minimum:

- device registration and editing;
- SSH connectivity;
- embedded SSH terminal;
- online/offline and connection-state monitoring;
- CPU, memory, disk, temperature, and uptime metrics;
- Docker container discovery;
- Docker lifecycle actions introduced post-MVP;
- registered service shortcuts;
- per-device and global monitoring settings;
- desktop notifications;
- system-tray operation.

Existing specifications may contain stale statements where capabilities originally described as future work are already implemented. Those statements should be corrected incrementally as the affected areas are touched; a wholesale rewrite is not required.

## 3. Product boundaries

### 3.1 In scope

Pi-Hub may:

- observe device health;
- diagnose connectivity;
- inspect Docker and system services;
- execute a closed set of explicitly implemented administrative actions;
- restart or safely shut down a managed device;
- monitor registered applications/services;
- record bounded operational activity and alerts;
- retain bounded historical health information.

### 3.2 Out of scope for the approved roadmap

Pi-Hub will not:

- expose arbitrary remote-command execution through application controls;
- store SSH passwords or private-key contents;
- expose Docker over TCP;
- automatically trust changed SSH host keys;
- provide a remote file manager;
- provide Docker Compose editing;
- provide arbitrary `docker exec` workflows;
- become a generic infrastructure orchestration platform;
- power on a physically powered-off device in the current roadmap;
- integrate smart-plug, managed-PoE, or GPIO power-on control in the current roadmap.

The embedded terminal remains the explicit manual-administration escape hatch and does not change the closed-command boundary for application features.

## 4. Cross-cutting engineering constraints

All planned milestones must preserve:

- agentless management over SSH;
- mandatory SSH host-key verification;
- typed Tauri/backend operations for application actions;
- predefined and bounded remote commands;
- explicit timeouts;
- per-device failure isolation;
- partial-data tolerance where metrics are optional or unsupported;
- responsive UI during remote operations;
- no SSH/sudo secret persistence;
- explicit confirmation for disruptive/destructive operations;
- no arbitrary administration API exposed to the frontend.

## 5. Planned milestone sequence

### M6 â€” Device Health and Diagnostics

Objective: make Pi-Hub explain whether a device is healthy and, when it is not, why.

Authoritative detail: `docs/milestones/m6-device-health-and-diagnostics.md`

Scope summary:

- consolidated device health;
- extended health metrics;
- Raspberry Pi power/throttling interpretation;
- reboot/boot information where available;
- structured connectivity diagnostics;
- health-reason presentation in Dashboard and Device Details.

### M7 â€” Service Health and Operational Activity

Objective: evolve registered services from bookmarks into monitored workloads and make operational changes visible.

Scope:

- HTTP/HTTPS health checks;
- response time;
- consecutive failure tracking;
- last successful service check;
- optional service-to-container association;
- persistent bounded activity log;
- device, container, service, diagnostic, and administrative activity events.

### M8 â€” Alerts and Threshold Governance

Objective: distinguish transient events from persistent operational problems.

Scope:

- Alert domain model;
- Active / Acknowledged / Resolved lifecycle;
- Info / Warning / Critical severity;
- global Alert Center;
- configurable global thresholds;
- per-device threshold overrides;
- duration/consecutive-sample rules for volatile metrics;
- notification decisions driven by alert transitions.

### M9 â€” Docker Operational Visibility

Objective: deepen Docker management without turning Pi-Hub into Portainer.

Scope:

- container detail view;
- CPU and memory usage;
- image/tag details;
- restart policy and restart count;
- ports, volumes, networks, and labels;
- bounded Docker logs;
- hardened start/stop/restart behavior;
- pause/unpause only if justified by implementation review;
- administrative activity recording.

Out of scope:

- create/remove container;
- Compose editor;
- arbitrary Docker execution;
- environment-variable editor.

### M10 â€” Controlled Device Administration

Objective: safely perform common host-level administrative operations.

Scope:

- Restart Device;
- Shut Down Device;
- restart Docker service;
- restart Tailscale service;
- explicit confirmation for disruptive actions;
- action progress and timeout states;
- planned-offline/restart recovery tracking;
- activity/audit events.

Authoritative shutdown detail: `docs/requirements/controlled-device-shutdown-requirement.md`

Power-on is explicitly out of scope.

### M11 â€” Network, Storage, and System Visibility

Objective: provide richer device-level operational context.

Scope:

- network interfaces;
- LAN and Tailscale addresses;
- link state;
- route/gateway and DNS summary;
- filesystem inventory;
- read-only/read-write filesystem state;
- OS distribution/version;
- kernel and architecture;
- Docker/Tailscale versions;
- systemd service visibility;
- available OS updates and reboot-required status, read-only.

### M12 â€” Historical Monitoring

Objective: add bounded historical visibility after current-state monitoring is stable.

Scope:

- local metric history;
- retention/downsampling policy;
- 1-hour, 24-hour, 7-day, and 30-day views where justified;
- CPU, memory, temperature, disk, latency, and service response-time trends;
- restart and undervoltage event history;
- storage-growth detection.

### M13 - Document Hygiene

Objective: perform a final documentation-quality and governance-reconciliation pass after the planned product milestones are complete.

Scope:

- audit canonical documentation against the implemented product;
- reconcile roadmap, milestone documents, specifications, release documentation, and AIQT references;
- remove or archive stale, contradictory, duplicated, or misleading documentation where evidence supports it;
- improve documentation navigation and validate internal links;
- record residual documentation debt explicitly.

M13 is intentionally the final planned milestone. It must not introduce unrelated product functionality.

## 6. Milestone dependencies

Expected dependency chain:

- M6 establishes device-health and diagnostic domain models.
- M7 builds service health and persistent activity on top of the M6 operational model.
- M8 consumes M6/M7 signals to provide governed alerts and thresholds.
- M9 deepens Docker visibility while reusing activity/alert infrastructure.
- M10 uses the activity model from M7 and the alert/planned-state semantics from M8 where applicable.
- M11 expands host visibility using the established snapshot/health patterns.
- M12 adds history only after the current-state data model is stable.

A later milestone may begin only when its required upstream contracts are sufficiently stable.

## 7. Priority order

Recommended delivery order:

1. M6 â€” Device Health and Diagnostics.
2. M7 â€” Service Health and Operational Activity.
3. M8 â€” Alerts and Threshold Governance.
4. M9 â€” Docker Operational Visibility.
5. M10 â€” Controlled Device Administration.
6. M11 â€” Network, Storage, and System Visibility.
7. M12 â€” Historical Monitoring.

## 8. Documentation rules

Human-readable product and milestone documentation belongs under `docs/`.

AIQT remains the canonical structured execution/state model under `.aiqt/`.

The two must agree on milestone identity, scope, and status, but Markdown planning documents must not be copied into `.aiqt/`.

When implementation differs materially from an approved planning assumption:

1. preserve the implementation evidence;
2. document the decision/deviation;
3. update the affected Markdown specification;
4. update AIQT state using repository conventions.

## 9. Definition of product success

The planned evolution is successful when Pi-Hub can answer, without requiring the embedded terminal for routine diagnosis:

- Is the device online?
- Is it healthy?
- Why is it unhealthy?
- Are its containers healthy?
- Are the applications hosted on it responding?
- Is there a power, thermal, storage, network, or service problem?
- What changed recently?
- What requires attention?
- Can the device be safely restarted or shut down from Pi-Hub?

Remote power-on is not required for this roadmap.


