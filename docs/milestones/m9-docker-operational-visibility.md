# Pi-Hub — M9 Docker Operational Visibility

Version: 0.1  
Status: Implemented

> Historical record: later-milestone planning statements describe the M9-era plan. See the [roadmap](../roadmap/post-mvp-product-roadmap.md) for current status.
Date: 2026-08-10  
Depends on: M8 — Alerts and Threshold Governance completed and merged  
Next milestone: M10 — Controlled Device Administration

## 1. Purpose and authority

This document is the implementation source of truth for:

`M9 — Docker Operational Visibility`

It defines the approved Docker visibility, bounded log access, lifecycle hardening, UI behavior, safety boundaries, Work Units, acceptance criteria, validation requirements, and milestone exit criteria.

The implementation may adapt details to the current repository architecture and established conventions, but the functional outcomes and security boundaries in this document must be preserved unless a material deviation is explicitly documented.

This document must be integrated into the repository as:

`docs/milestones/m9-docker-operational-visibility.md`

## 2. Objective

Deepen Pi-Hub's existing Docker support from basic container discovery/lifecycle controls into an operational troubleshooting surface without turning Pi-Hub into Portainer.

At completion, the user should be able to answer:

- What state is each container in?
- Is Docker reporting the container as healthy?
- Which image/tag is running?
- When was the container created and started?
- How many times has it restarted?
- What restart policy is configured?
- What CPU and memory is the container currently using?
- Which ports, mounts, networks, and labels are attached?
- What do the latest bounded container logs show?
- Did a requested Start / Stop / Restart action succeed?
- Which registered Pi-Hub service is associated with the container?

M9 provides current operational visibility and controlled common actions. It does not become a general Docker administration interface.

## 3. Scope

### 3.1 In scope

M9 includes:

- richer typed Docker/container state;
- Docker daemon/version availability information where useful;
- container state and Docker health status;
- image reference/tag and bounded image/container identifiers;
- created/start timestamps;
- restart count;
- restart policy;
- current CPU usage;
- current memory usage and limit where Docker exposes them;
- port mappings;
- mounts/volumes;
- Docker networks;
- bounded labels;
- bounded read-only container logs;
- hardened existing Start / Stop / Restart operations;
- post-action state verification;
- action-in-progress UI behavior and duplicate-action prevention;
- M7 Activity evidence for container lifecycle actions;
- M7 service-to-container navigation/context;
- container detail UI;
- representative automated frontend/backend coverage;
- documentation/AIQT reconciliation and milestone validation.

### 3.2 Conditionally in scope

Pause / Unpause may be implemented only when:

- the existing lifecycle abstraction supports it cleanly;
- it does not require a parallel command model;
- it can be tested and surfaced consistently with Start / Stop / Restart.

Pause / Unpause is not required for M9 closure if the implementation review concludes that it would add disproportionate complexity. Any defer decision must be recorded in the milestone closure report.

### 3.3 Out of scope

M9 does not include:

- creating containers;
- deleting/removing containers;
- pulling arbitrary images from the UI;
- Docker Compose editing;
- stack management;
- arbitrary `docker exec`;
- embedded shell inside a container;
- environment-variable viewing/editing;
- secrets/config management;
- Dockerfile editing;
- arbitrary Docker CLI execution;
- exposing the Docker daemon over TCP;
- image vulnerability scanning;
- automatic container update/watchtower behavior;
- historical per-container time-series charts;
- configurable Docker alert rules beyond reliable signals already governed by M8;
- device restart/shutdown;
- Power On.

## 4. Architecture and security boundaries

M9 must preserve Pi-Hub's existing security model:

Frontend  
→ typed Tauri/backend operations  
→ application/domain services  
→ predefined SSH/Docker collection and action operations

Requirements:

- Docker remains accessed through the existing managed-device path; do not expose Docker over TCP;
- do not add a frontend API that accepts arbitrary Docker commands;
- do not add arbitrary shell fragments or `docker exec`;
- remote commands remain predefined, typed, bounded, and timeout-controlled;
- SSH host-key verification remains mandatory;
- a Docker failure must not fail the entire device snapshot;
- a container-specific failure must not block unrelated containers/devices;
- logs and metadata must be bounded;
- do not collect or display container environment variables;
- do not persist container log content unless existing architecture explicitly requires transient caching; default behavior is on-demand only;
- preserve M6 Device Health, M7 Service Health/Activity, and M8 Alert semantics;
- avoid unrelated refactoring.

## 5. Docker state model

### 5.1 Container runtime state

Represent the equivalent of Docker runtime states when available:

- `Created`
- `Running`
- `Paused`
- `Restarting`
- `Exited`
- `Dead`
- `Removing`
- `Unknown`

The implementation may map Docker-specific variants to a smaller stable domain enum if necessary, but it must not collapse all non-running states into a single ambiguous "Stopped" value when richer state is available.

### 5.2 Docker health status

Container runtime state and Docker health-check state are separate concepts.

Health status should represent the equivalent of:

- `Healthy`
- `Unhealthy`
- `Starting`
- `None` / no Docker health check
- `Unknown`

A running container with no Docker HEALTHCHECK must not be shown as `Healthy` merely because it is running.

### 5.3 Docker availability

Docker availability should remain distinguishable from device availability.

Possible conditions include:

- Docker available;
- Docker CLI/daemon unavailable;
- permission denied;
- command timeout;
- malformed/unexpected output.

Docker failure must not cause Pi-Hub to misclassify SSH connectivity as failed.

## 6. Container operational data

### 6.1 Identity and image

Expose when available:

- container name;
- bounded container ID;
- image reference;
- image tag/digest information where naturally available;
- bounded image ID;
- created timestamp;
- started timestamp.

Prefer user-readable image references such as:

`homeassistant/home-assistant:stable`

over opaque image IDs in primary UI.

### 6.2 Restart information

Expose:

- restart count;
- restart policy name;
- restart policy maximum retry count where applicable.

Examples of policy names may include Docker-standard values such as:

- `no`
- `always`
- `unless-stopped`
- `on-failure`

Do not invent policy semantics; display normalized Docker-provided configuration.

### 6.3 Current resource usage

Expose a current point-in-time resource snapshot:

- CPU percentage;
- memory used;
- memory limit where available;
- memory percentage where available/derivable safely.

M9 does not store historical container resource time series.

Resource metrics must be optional:

- failure to collect `docker stats` must not remove otherwise valid container metadata;
- missing stats render as unavailable, not zero;
- stats collection must be bounded;
- avoid one expensive SSH process per container when Docker can return multiple container stats safely in one bounded call.

### 6.4 Port mappings

Expose normalized mappings:

- container/private port;
- protocol;
- host IP when applicable;
- published host port when applicable.

Do not confuse `EXPOSE` metadata with an actual published host binding.

### 6.5 Mounts and volumes

Expose bounded mount metadata:

- mount type;
- source/name;
- destination;
- read/write or read-only mode.

Do not read files from mounted paths.

### 6.6 Networks

Expose:

- attached Docker network names;
- container IP addresses where Docker provides them and where display is useful;
- primary/available aliases when bounded.

Do not turn M9 into a Docker network editor.

### 6.7 Labels

Expose bounded container labels for troubleshooting.

Requirements:

- cap total labels displayed/returned per container;
- cap key/value length;
- avoid logging labels unnecessarily;
- values whose keys are clearly secret-like (`secret`, `password`, `token`, `apikey`, `api_key`, `credential`, etc.) must be redacted rather than shown verbatim;
- no environment variables are collected as a substitute for labels.

The exact redaction helper may reuse existing sensitive-data handling if present.

## 7. Bounded container logs

### 7.1 Purpose

Provide recent container logs for troubleshooting without implementing a full log-management platform.

### 7.2 M9 log modes

Support one-shot, bounded retrieval only.

At minimum:

- last 100 lines;
- last 500 lines;
- last 15 minutes;
- last 1 hour.

"Since container start" may be included only when still subject to a hard output bound.

Live/follow streaming is not required in M9.

### 7.3 Output bounds

Backend-enforced bounds are mandatory.

Recommended hard limits:

- maximum requested tail: 500 lines;
- maximum returned log payload: 512 KiB per request;
- command timeout: 10 seconds.

If output exceeds the byte bound:

- truncate safely;
- return an explicit `truncated` indicator;
- do not fail the entire UI request solely because truncation occurred.

### 7.4 Log semantics

Return typed metadata equivalent to:

- container ID/name;
- requested mode;
- collected timestamp;
- text/content;
- truncated flag;
- failure classification where applicable.

Preserve log text as operational output, but:

- do not persist it into Activity;
- do not persist it into Alert records;
- do not include full log content in application error logs;
- do not automatically interpret log text as commands, alerts, or health state.

### 7.5 Failure behavior

Distinguish where practical:

- container not found;
- Docker unavailable;
- permission denied;
- timeout;
- command failure;
- invalid log request.

A failed log request must not change the container's runtime/health state.

## 8. Lifecycle action hardening

Pi-Hub already has Docker lifecycle actions. M9 hardens them rather than replacing them.

### 8.1 Supported required actions

- Start
- Stop
- Restart

Pause / Unpause is conditional per Section 3.2.

### 8.2 Typed operation boundary

Frontend actions must invoke dedicated typed backend operations.

Do not accept:

- arbitrary Docker command text;
- arbitrary container CLI arguments;
- shell fragments.

### 8.3 Action lifecycle

The exact implementation may follow existing conventions, but UI/application behavior must represent the equivalent of:

- idle;
- action requested/in progress;
- command succeeded but awaiting refresh/verification where applicable;
- completed;
- failed;
- timed out.

Only one lifecycle action should execute for a given container at a time from Pi-Hub.

Duplicate button presses must not launch concurrent conflicting operations.

### 8.4 State-aware behavior

Actions should be enabled/disabled according to known current state where reliable.

Examples:

- Start should not normally be offered for a Running container;
- Stop should not normally be offered for an Exited/Dead container;
- Restart is meaningful for a Running container;
- Pause is meaningful only for Running;
- Unpause is meaningful only for Paused.

If current state is stale/unknown, backend validation remains authoritative.

### 8.5 Post-action verification

After an action reports success:

- refresh/re-read container state using the existing monitoring/refresh model;
- display the observed resulting state;
- do not claim the container is Running/Stopped solely because the Docker command returned success if verification subsequently contradicts it.

Verification timeout/failure should be represented explicitly without converting a successful command invocation into fabricated state.

### 8.6 Failure classification

Distinguish where practical:

- container not found;
- Docker unavailable;
- permission denied;
- invalid state/conflict;
- command timeout;
- command rejected/failure;
- post-action verification timeout/failure.

### 8.7 Activity integration

Use the M7 Activity model for meaningful lifecycle evidence.

Record, at minimum:

- action requested or initiated when current conventions support it;
- action completed;
- action failed/timed out.

Activity must include:

- device;
- container;
- action type;
- result;
- concise non-sensitive summary.

Do not store command strings or unrelated Docker output.

### 8.8 Alert compatibility

M9 must not create a second alert system.

Reliable Docker health/state signals may continue to feed M8 only through existing governed alert integration.

Lifecycle actions must not automatically resolve an unrelated alert unless the subsequent observed condition actually clears and the M8 evaluator resolves it.

## 9. Service-to-container integration

M7 introduced optional manual Service → Container association.

M9 should make that association operationally useful.

Required behavior:

- container detail shows associated Pi-Hub services, if any;
- service UI can navigate/open the associated container detail where the current navigation model permits;
- a missing/stale container association remains visible as missing rather than silently rewritten;
- container recreation using the same stable configured identifier should recover association when consistent with the M7 model;
- M9 must not automatically infer associations.

Service health remains based on HTTP(S) checks, not container runtime state.

## 10. Work Units

### WU9-01 — Extended Docker/container operational model

#### Objective

Introduce the richer typed container inspection model required by M9.

#### Required implementation

Collect and normalize, when available:

- runtime state;
- Docker health status;
- image reference/tag;
- container/image identifiers;
- created/started timestamps;
- restart count;
- restart policy;
- port mappings;
- mounts;
- networks;
- bounded/redacted labels;
- Docker availability/error classification.

Use the smallest number of bounded Docker/SSH operations practical without making parsing brittle.

#### Acceptance criteria

- running, paused, exited, restarting, unhealthy, and no-healthcheck scenarios are distinguishable;
- no Docker HEALTHCHECK is not rendered as Healthy;
- missing optional metadata does not fail the full container list;
- Docker unavailable remains separate from SSH/device availability;
- ports represent actual bindings distinctly from unbound/exposed ports;
- mount/network parsing is deterministic;
- secret-like label values are redacted;
- existing Docker lifecycle functionality is not regressed;
- parser/domain tests cover representative valid, malformed, missing, and unsupported output.

### WU9-02 — Current container resource metrics

#### Objective

Add bounded point-in-time CPU/memory visibility.

#### Required implementation

Collect current resource stats for relevant discovered containers.

Prefer one bounded Docker stats invocation for multiple containers where practical.

Represent:

- CPU percentage;
- memory used;
- memory limit;
- memory percentage.

#### Acceptance criteria

- current stats populate for Running containers when supported;
- stopped/non-running containers safely show unavailable/no current stats;
- stats command failure does not remove inspection metadata;
- missing stats are not represented as zero;
- malformed per-container stats do not fail unrelated containers;
- collection remains bounded;
- tests cover multi-container parsing, unavailable stats, and partial malformed output.

### WU9-03 — Bounded container logs

#### Objective

Provide safe on-demand recent logs.

#### Required implementation

Implement the bounded log contract from Section 7 through a dedicated typed backend operation.

Required modes:

- last 100 lines;
- last 500 lines;
- last 15 minutes;
- last 1 hour.

Enforce backend limits regardless of frontend input.

#### Acceptance criteria

- each required mode produces the expected bounded Docker request;
- invalid modes/values are rejected;
- output larger than 512 KiB is truncated with explicit metadata;
- timeout is bounded;
- logs are not persisted in Activity/Alerts;
- no arbitrary command API is introduced;
- failure classifications are user-presentable;
- tests cover line limits, time modes, truncation, timeout/error semantics, and invalid request handling.

### WU9-04 — Lifecycle action hardening and Activity evidence

#### Objective

Harden Start / Stop / Restart operations and integrate reliable action evidence.

#### Required implementation

- preserve dedicated typed actions;
- prevent conflicting duplicate actions per container;
- provide in-progress state;
- classify failures;
- re-read state after successful command;
- record Activity lifecycle evidence;
- preserve M8 alert governance.

Pause / Unpause may be included only under the conditional rule in Section 3.2.

#### Acceptance criteria

- duplicate/conflicting action requests are prevented or serialized deterministically;
- state-aware UI actions match known container state;
- backend remains authoritative for stale-state conflicts;
- action success triggers state verification;
- UI does not fabricate resulting state;
- failure/timeout is explicit;
- Activity receives concise action result evidence;
- command text/secrets are not stored;
- tests cover Start/Stop/Restart success, conflict, timeout/failure, and verification mismatch/failure;
- existing associated service health remains independent.

### WU9-05 — Container detail and operational UI

#### Objective

Expose M9 capabilities using the existing Pi-Hub visual language.

#### Container list

Preserve the existing scan-friendly list/table.

Add only information that materially improves overview, such as:

- runtime state;
- Docker health;
- concise CPU/memory summary where available;
- clear action-in-progress state.

Do not place ports, mounts, labels, and full metadata directly in the main list.

#### Container detail

Add a detail view/drawer/screen consistent with existing navigation.

Expose sections equivalent to:

##### Overview

- name;
- runtime state;
- Docker health;
- image;
- created;
- started;
- restart count/policy;
- CPU;
- memory.

##### Ports

- normalized port mappings.

##### Storage

- mounts/volumes.

##### Networks

- network names/address context.

##### Labels

- bounded/redacted labels.

##### Services

- associated Pi-Hub services.

##### Logs

- on-demand log viewer;
- required bounded modes;
- truncation indicator;
- loading/error/empty states.

##### Actions

- Start / Stop / Restart;
- optional Pause / Unpause if implemented;
- disabled/busy state while action runs;
- clear action result/error feedback.

#### Acceptance criteria

Frontend behavioral tests cover:

- running Healthy container;
- running container with no healthcheck;
- unhealthy container;
- paused/exited state;
- missing resource stats;
- port/mount/network rendering;
- redacted label;
- associated service;
- log loading/content/truncated/error states;
- state-aware actions;
- action busy state;
- action failure;
- post-action refreshed state.

Do not rely solely on color for runtime/health status.

## 11. Cross-Work-Unit scenarios

| Scenario | Required behavior |
|---|---|
| Running container with Docker healthcheck Healthy | Running + Healthy shown separately |
| Running container with no healthcheck | Running + No health check |
| Running container becomes Unhealthy | Docker health shows Unhealthy; M8 may govern existing alert signal |
| Container is Paused | Paused state shown; normal Start/Stop controls adapt |
| Container is Exited | Exited shown; current CPU/memory unavailable |
| Docker daemon unavailable | Device can remain online; Docker section shows specific failure |
| Stats fail for one/all containers | Inspection metadata remains visible |
| Service is associated with container | Container detail shows service; service health remains HTTP-based |
| Associated service is Healthy while container metadata is stale | Service Health remains independent |
| Logs requested for last 100 lines | Bounded output returned |
| Logs exceed 512 KiB | Output truncated with visible indicator |
| Container disappears before log/action request | Specific not-found error; no device failure |
| User double-clicks Restart | One effective action; no conflicting duplicate operation |
| Restart command succeeds but verification fails | Show verification uncertainty; do not fabricate final state |
| Stop succeeds | Activity records result; subsequent observed state drives alerts |
| Secret-like label exists | Value is redacted |
| App restarts | No container log history needs restoration; normal inspection resumes |

## 12. Persistence and migration

M9 should avoid new persistent container state unless required.

Container inspection and resource stats are current-state data and should normally be refreshed from Docker.

Persist only configuration/domain information that is already intended to survive, such as:

- existing M7 service-to-container association.

Do not persist:

- current CPU/memory stats;
- container logs;
- transient action-in-progress state after application restart.

After application restart:

- stale transient action state must not remain indefinitely;
- container current state is re-observed;
- M7 associations remain compatible;
- M8 alerts remain governed by actual observed signals.

Existing persisted data from M6–M8 must remain readable.

## 13. Performance and boundedness

M9 must avoid turning normal refresh into an expensive sequence proportional to containers × SSH round trips where aggregation is possible.

Requirements:

- bound command timeouts;
- bound inspection output;
- bound stats output;
- bound labels;
- bound log output;
- isolate expensive on-demand logs from normal monitoring;
- do not fetch logs during routine refresh;
- do not fetch detailed inspection repeatedly in the UI if the monitoring snapshot already has fresh equivalent data;
- preserve responsive UI and per-device isolation.

If the implementation introduces a measurable refresh regression, optimize the collection path before closure or document a justified bounded tradeoff.

## 14. Testing and validation strategy

### 14.1 During implementation

Use focused tests after each Work Unit.

Do not run the complete repository suite after every Work Unit unless repository policy requires it.

Do not weaken/remove valuable tests to obtain green status.

### 14.2 Required automated coverage

At minimum:

#### Container inspection

- runtime states;
- health states;
- no healthcheck;
- restart policy/count;
- ports;
- mounts;
- networks;
- labels/redaction;
- malformed/partial output;
- Docker unavailable.

#### Stats

- multi-container stats;
- running container metrics;
- stopped container/no metrics;
- malformed one-container entry;
- stats unavailable.

#### Logs

- 100/500 line modes;
- 15m/1h modes;
- output truncation;
- timeout/failure;
- invalid request;
- not found;
- no persistence side effect.

#### Lifecycle

- Start;
- Stop;
- Restart;
- duplicate/conflicting request;
- timeout/failure;
- state verification;
- verification mismatch/failure;
- Activity integration.

#### Frontend

- representative state/health combinations;
- detail sections;
- unavailable stats;
- redacted labels;
- service association;
- log viewer modes;
- truncation/error;
- state-aware lifecycle actions;
- busy/failure/result states.

### 14.3 Milestone closure validation

Run the repository's canonical complete validation after implementation.

Use repository-defined commands.

Closure evidence should include the normal equivalents of:

- Rust format/static checks;
- Rust tests;
- frontend lint;
- frontend tests;
- frontend type/build validation;
- production build;
- Tauri/backend check/build where required;
- AIQT/state validation;
- `git diff --check`.

Prefer any canonical CI-equivalent command if the repository defines one.

Existing unrelated warnings may remain only when confirmed pre-existing and non-regressive.

## 15. Safe live validation

If configured Pi devices are safely accessible, M9 may perform read-only Docker validation and non-destructive lifecycle validation only where explicitly safe.

Read-only permitted validation includes:

- inspect real containers;
- collect current Docker stats;
- view bounded logs;
- validate ports/mounts/networks/labels;
- validate service associations.

Do not stop/restart/pause a real container merely to satisfy automated milestone evidence unless the user explicitly authorizes that disruptive action.

Lifecycle behavior may be validated through deterministic mocked/local test infrastructure when live action would disrupt real services.

Do not:

- delete containers;
- pull/change images;
- modify Compose;
- change container environment;
- change volumes/networks;
- restart/shut down the Pi;
- weaken SSH/Docker security.

If live lifecycle validation is not authorized, record it as residual manual validation.

## 16. Documentation and AIQT completion

At milestone start/integration:

- place this document at `docs/milestones/m9-docker-operational-visibility.md`;
- update README/documentation navigation where appropriate;
- preserve M1–M8 completed history;
- transition M9 using the repository's native AIQT workflow;
- keep M10–M13 planned/not started.

At milestone closure:

- update this document only where actual implementation materially differs;
- record material deviations/rationale;
- update affected functional/technical specifications;
- update AIQT Work Unit/milestone state from implementation evidence;
- follow `.github/pull_request_template.md` for the PR;
- do not mark unimplemented/unvalidated requirements complete.

## 17. Exit criteria

M9 is implementation-complete when:

1. WU9-01 through WU9-05 are implemented.
2. Container runtime state and Docker health are represented separately and correctly.
3. Image, restart, port, mount, network, and bounded/redacted label metadata are available.
4. Current CPU/memory metrics are available when supported and safely unavailable otherwise.
5. Bounded on-demand logs support the required modes.
6. Log payloads/timeouts are backend-bounded and truncation is explicit.
7. Start / Stop / Restart actions are hardened against duplicate/conflicting execution.
8. Successful lifecycle commands are followed by observed state verification.
9. Container lifecycle results create appropriate M7 Activity evidence.
10. M7 service-to-container association is visible/useful in the Docker UI without changing Service Health semantics.
11. Existing M6–M8 health/alert semantics remain intact.
12. Docker failure remains isolated from device SSH/connectivity state.
13. Required automated tests pass.
14. Canonical milestone validation passes.
15. Documentation and AIQT state reflect actual implementation.
16. M10–M13 remain planned/not started.

Pause / Unpause may be implemented or explicitly deferred under Section 3.2 without blocking closure.

Safe real-container inspection should be included when available. Disruptive container lifecycle smoke tests remain optional unless explicitly authorized.

## 18. Non-goals carried forward

M9 completion must not introduce:

- arbitrary Docker execution;
- container creation/deletion;
- Compose editing;
- environment/secrets editing;
- image auto-update;
- historical container metrics;
- automatic remediation;
- host restart/shutdown;
- host Power On.

M10 remains responsible for Controlled Device Administration.
