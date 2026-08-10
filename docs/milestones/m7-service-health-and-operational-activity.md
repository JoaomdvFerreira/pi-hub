# Pi-Hub — M7 Service Health and Operational Activity

Version: 0.1  
Status: Approved for implementation  
Date: 2026-08-10  
Depends on: M6 — Device Health and Diagnostics completed and merged  
Next milestone: M8 — Alerts and Threshold Governance

## 1. Purpose and authority

This document is the implementation source of truth for:

`M7 — Service Health and Operational Activity`

It defines the approved scope, domain semantics, Work Units, safety boundaries, acceptance criteria, validation requirements, and milestone exit criteria.

The implementation may adapt details to the current repository architecture and established conventions, but the functional outcomes and boundaries in this document must be preserved unless a material deviation is explicitly documented.

This document must be integrated into the repository as:

`docs/milestones/m7-service-health-and-operational-activity.md`

## 2. Objective

Evolve Pi-Hub services from registered URL shortcuts into monitored workloads and introduce a persistent, bounded operational activity history.

At completion, Pi-Hub should answer:

- Is a registered service responding?
- How long did the latest service check take?
- When was the service last checked?
- When was it last known healthy?
- Is the service experiencing a transient failure or a sustained outage?
- Which Docker container, if any, is associated with the service?
- What meaningful operational changes happened recently across devices, services, containers, and diagnostics?

M7 must provide service-health state and operational evidence without introducing the persistent alert lifecycle planned for M8.

## 3. Scope

### 3.1 In scope

M7 includes:

- read-only HTTP/HTTPS health checks for registered services;
- response-time measurement;
- persistent latest service-health state;
- consecutive-failure tracking;
- last-check and last-success timestamps;
- deterministic service-health transitions;
- optional manual service-to-container association;
- persistent bounded operational activity history;
- activity generation from meaningful state transitions and explicit actions;
- Services UI health presentation;
- Device Details service-health presentation;
- Dashboard Recent Activity population;
- a global Activity view or equivalent existing navigation surface;
- focused automated test coverage;
- milestone-level validation and documentation/AIQT reconciliation.

### 3.2 Out of scope

M7 does not include:

- TCP-only service monitoring;
- ICMP/ping service monitoring;
- authenticated HTTP health checks;
- custom request headers;
- API tokens, cookies, or credential storage for service checks;
- arbitrary/custom health-check commands;
- per-service custom HTTP methods;
- per-service custom expected-status configuration;
- configurable service-check timeout;
- configurable failure thresholds;
- response-time alert thresholds;
- service availability notifications driven by a persistent alert lifecycle;
- Alert acknowledgement/resolution;
- device-health degradation caused solely by service state;
- historical time-series charts;
- Docker log expansion;
- device restart/shutdown;
- remote Power On.

Configurable thresholds and persistent alert governance belong to M8.

## 4. Product and domain decisions

### 4.1 Health-check transport

M7 health checks support only:

- `http://`
- `https://`

A service URL is checked from the Pi-Hub desktop host using normal HTTP(S) networking. The check is not tunneled through SSH.

This measures whether the registered service endpoint is reachable from the machine running Pi-Hub.

### 4.2 HTTP request semantics

Requirements:

- bounded timeout;
- M7 default timeout: 5 seconds;
- redirects may be followed, but redirect depth must be bounded;
- recommended maximum: 5 redirects;
- response bodies must not be persisted;
- the complete response body must not be required to determine health;
- no credentials or custom authentication headers are added;
- TLS certificate validation remains enabled;
- M7 must not add an "ignore invalid certificate" option.

The implementation may use existing repository/network abstractions if they provide equivalent behavior.

### 4.3 Successful HTTP response

A completed request is successful when the final HTTP status is:

`200–399`

inclusive.

Responses `400–599` are health-check failures.

Transport, DNS, timeout, redirect-limit, and TLS failures are also failures with distinct machine-readable failure reasons where practical.

### 4.4 Service-health states

Use the equivalent of:

- `Unknown`
- `Healthy`
- `Degraded`
- `Unavailable`

`Unknown` means Pi-Hub does not yet have sufficient completed-check evidence, or the service cannot currently be evaluated because of unsupported/invalid configuration. Unknown must not be represented as Healthy.

`Healthy` means the latest completed health check succeeded. A successful check resets the consecutive-failure count to zero.

`Degraded` means the latest check failed, but the sustained-failure threshold has not been reached:

- 1 consecutive failure → `Degraded`
- 2 consecutive failures → `Degraded`

`Unavailable` means:

- 3 or more consecutive failures.

M8 may later make this threshold configurable.

### 4.5 Recovery semantics

A successful check after any number of failures:

- resets consecutive failures to `0`;
- transitions the service to `Healthy`;
- updates `lastSuccessfulCheckAt`;
- creates a service-health transition activity event when the prior state was not Healthy.

### 4.6 Response time

Response time is measured from request start until enough response metadata is available to classify the request, normally receipt of response headers.

Do not delay the recorded latency by downloading a complete response body.

Store/display response time in milliseconds.

A slow response does not by itself change service health in M7.

### 4.7 Device health independence

Service health is workload health, not host health.

In M7:

- a service becoming Degraded or Unavailable does not directly change the M6 `DeviceHealth` state;
- an unhealthy device does not automatically force a service health result without a service check;
- service health and device health may be displayed together but remain separate domain concepts.

M8 may consume both as alert signals.

### 4.8 Service-check scheduling

Service checks should integrate with the existing monitoring lifecycle rather than create an unrelated polling engine.

Required behavior:

- configured services are checked at most once per normal monitoring/refresh cycle;
- manual device refresh includes service-health refresh for that device;
- individual service checks are isolated;
- one slow/failing service must not fail the device snapshot or unrelated service checks;
- service checks may run even when SSH/device monitoring reports a connection problem because the endpoint may still be reachable through another path;
- monitoring remains responsive and time-bounded according to existing architecture.

The implementation may batch or limit concurrency to protect the application and network.

## 5. Architecture and safety boundaries

Preserve the established Pi-Hub dependency direction:

Frontend  
→ typed Tauri/backend commands and application services  
→ domain models/state transitions  
→ HTTP/network and persistence infrastructure

M7 must preserve:

- agentless SSH management for device operations;
- mandatory SSH host-key verification for SSH functionality;
- predefined/typed backend operations;
- per-device and per-service failure isolation;
- bounded network operations;
- partial-data tolerance;
- no arbitrary remote-command API;
- no storage of SSH, sudo, HTTP, or application credentials;
- existing embedded-terminal behavior;
- existing Docker lifecycle behavior;
- M6 health and diagnostics semantics.

Do not place service-health transition logic inside React components.

Do not place activity-retention logic inside UI components.

Avoid unrelated refactoring.

## 6. Work Units

### WU7-01 — Service health-check domain and execution

#### Objective

Introduce typed service-health models and a bounded HTTP(S) checker for registered services.

#### Required domain data

The model must represent the equivalent of:

- service ID;
- current service-health state;
- latest HTTP status when available;
- latest response time in milliseconds when available;
- consecutive failure count;
- latest check timestamp;
- latest successful check timestamp;
- latest failure classification/reason when available.

The result must distinguish at minimum:

- success;
- HTTP error status;
- DNS/connection failure;
- timeout;
- TLS/certificate failure where distinguishable;
- redirect failure/limit where distinguishable;
- invalid/unsupported URL;
- unknown/unclassified transport failure.

Do not persist response bodies.

#### Required execution behavior

- support HTTP and HTTPS only;
- use the M7 5-second timeout;
- preserve TLS certificate validation;
- bounded redirects;
- classify 200–399 as successful;
- classify 400–599 as failed;
- measure response time;
- return typed results;
- do not panic on malformed URLs or network failures;
- do not mutate the remote service.

#### Acceptance criteria

- valid 2xx endpoint → success;
- valid 3xx/final redirect success → success;
- 4xx endpoint → failed check;
- 5xx endpoint → failed check;
- connection refused/unreachable → failed check with transport reason;
- timeout → failed check with timeout reason;
- malformed/unsupported URL → safe typed failure/unsupported result;
- response body is not persisted;
- no authentication material is added;
- tests use deterministic local/mocked endpoints rather than external internet dependencies.

### WU7-02 — Service monitoring state and transitions

#### Objective

Integrate service checks into monitoring and persist the latest service-health state and failure streak.

#### Required state

Persist enough information to restore the latest known service-health record after application restart, including:

- state;
- consecutive failures;
- last checked;
- last successful check;
- latest status code when available;
- latest response time when available;
- latest failure reason when available.

Use current storage conventions rather than introducing a new database solely for M7 unless the existing architecture genuinely requires it.

#### State transitions

| Prior state | Check result | Consecutive failures | New state |
|---|---|---:|---|
| Any | Success | 0 | Healthy |
| Any | Failure | 1 | Degraded |
| Any | Failure | 2 | Degraded |
| Any | Failure | >=3 | Unavailable |

A success after failure must immediately recover to Healthy.

Repeated checks that leave the service in the same state update current metadata but do not create duplicate transition events.

#### Acceptance criteria

- failure counts increment deterministically;
- success resets failure count;
- Degraded occurs for failures 1–2;
- Unavailable occurs from failure 3 onward;
- recovery updates `lastSuccessfulCheckAt`;
- repeated same-state checks do not generate transition spam;
- state survives application restart/storage reload;
- one service failure does not block other service/device monitoring;
- tests cover transition boundaries, recovery, persistence round-trip, and partial failures.

### WU7-03 — Optional service-to-container association

#### Objective

Allow a registered service to be associated with the Docker container that provides it.

#### Association semantics

Association is:

- optional;
- manually selected by the user;
- metadata only;
- independent from service-health classification.

Prefer a stable human-facing container identifier, such as container name, over ephemeral container IDs when consistent with the existing Docker model.

Required behavior:

- service may have no associated container;
- user can select from containers discovered for the same device;
- user can clear the association;
- if the associated container is later missing, stopped, or recreated, the service configuration remains valid;
- missing association target is displayed clearly rather than silently deleted;
- the association must not automatically change service health;
- no automatic name-matching or inference is required for M7.

#### Acceptance criteria

- association persists with service configuration;
- existing services without association continue to load;
- association can be added/changed/cleared;
- only containers belonging to the service's device are selectable;
- missing/stale container association does not corrupt service configuration;
- tests cover persistence/default/backward-compatible behavior.

### WU7-04 — Persistent operational activity log

#### Objective

Turn Pi-Hub's Recent Activity concept into a durable, bounded record of meaningful operational events.

#### Activity model

Each event must contain the equivalent of:

- unique event ID;
- timestamp;
- stable event code/type;
- category/source;
- device ID when applicable;
- related entity ID when applicable;
- related entity display name snapshot where useful;
- concise user-facing summary;
- action/result metadata where applicable;
- bounded non-sensitive technical detail where useful.

Categories should support at minimum:

- device;
- health;
- service;
- container;
- diagnostic;
- administration.

The model must be extensible for M10 device administration without requiring a redesign.

#### Events to record in M7

Record meaningful transitions/actions, including where the existing system exposes them reliably:

- device online transition;
- device offline transition;
- M6 device-health state transition;
- service health transitions;
- container lifecycle action requested/completed/failed where actions already exist;
- observed container state transitions where this can be captured without polling noise;
- diagnostic run completion with overall result;
- future administrative actions through the extensible event model.

#### Events not to record

Do not create activity events for:

- every routine refresh;
- every successful service-health poll when state is unchanged;
- every metric sample;
- terminal command text;
- terminal output;
- SSH secrets;
- URL query strings containing possible tokens;
- full HTTP responses;
- sensitive authentication details.

Activity is evidence of meaningful changes/actions, not a debug log.

#### Retention policy

M7 uses:

- maximum age: 30 days;
- maximum count: 2,000 events globally.

Prune oldest events when either bound is exceeded.

Retention must be deterministic and testable.

The implementation may prune on write/startup or through an equivalent simple mechanism; it must not require a long-running background maintenance service.

#### Failure behavior

Activity persistence must not break primary monitoring functionality.

If an event cannot be persisted:

- do not fail an otherwise successful device/service operation solely because logging failed;
- surface/log the persistence failure according to repository conventions;
- avoid recursive activity-log errors.

#### Acceptance criteria

- events persist across application restart;
- events are newest-first when queried for UI;
- retention prunes events older than 30 days;
- retention prunes oldest records beyond 2,000;
- unchanged polling does not produce event spam;
- service transitions produce one meaningful transition event;
- sensitive terminal/HTTP/authentication content is not persisted;
- persistence failure does not falsely fail the originating operation;
- tests cover event creation, ordering, retention, transition deduplication, and storage round-trip.

### WU7-05 — Service health and activity UI integration

#### Objective

Expose service health and operational history using the existing Pi-Hub visual language.

#### Services UI

Extend the existing Services experience to show, where available:

- service-health badge/state;
- latest response time;
- last checked;
- last successful check;
- current consecutive failure count when non-zero or useful;
- associated container;
- concise failure reason when Degraded/Unavailable.

Required states must be visually and textually distinguishable:

- Unknown;
- Healthy;
- Degraded;
- Unavailable.

Do not rely solely on color.

Registered service links must continue to work.

#### Device Details

Expose service-health information without overloading the M6 host-health presentation.

Device health and service health must remain visibly separate concepts.

Associated-container information should be visible where useful.

#### Service configuration

Extend the existing service editing surface to support optional container association.

Do not introduce M8 threshold configuration.

#### Activity UI

Populate the existing Dashboard Recent Activity panel from persistent activity data.

Dashboard should show a concise recent subset, recommended:

- latest 10 events.

Add a global Activity view, or use an existing equivalent navigation surface if one already exists.

The Activity view should support at least:

- newest-first list;
- device filtering;
- category/type filtering;
- concise timestamp;
- event summary;
- related device/entity context.

Search, export, advanced date filters, and complex pagination are not required in M7.

#### UI behavior

- use existing component/style patterns;
- preserve responsive desktop behavior;
- loading/empty/error states must be explicit;
- stale/missing service-health data must render as Unknown/unavailable, not false Healthy;
- UI must consume domain-classified states rather than reproduce the failure-streak state machine;
- activity summaries must not expose sensitive data.

#### Acceptance criteria

Behavioral frontend tests cover representative cases:

- Unknown service;
- Healthy service with response time;
- Degraded service after failure;
- Unavailable service;
- recovered service;
- associated container present;
- associated container missing;
- Dashboard Recent Activity rendering;
- Activity filtering;
- empty activity state;
- no sensitive detail rendering where bounded technical metadata exists.

## 7. Cross-Work-Unit behavior

| Scenario | Required behavior |
|---|---|
| Service returns HTTP 200 | Healthy, response time captured, failure count 0 |
| Service redirects to successful endpoint within bound | Healthy |
| Service returns HTTP 404 | Failure; Degraded/Unavailable based on streak |
| Service returns HTTP 500 | Failure; Degraded/Unavailable based on streak |
| Service times out | Failure classified as timeout |
| TLS validation fails | Failure; TLS validation is not bypassed |
| First failed check after Healthy | Degraded |
| Second consecutive failed check | Degraded |
| Third consecutive failed check | Unavailable |
| Service succeeds after Unavailable | Healthy, streak reset, recovery activity event |
| Service remains Healthy across repeated polls | Metadata updates; no repeated activity spam |
| Device SSH is unavailable but service URL responds | Service may still be Healthy |
| Service is Unavailable | M6 DeviceHealth is not automatically changed |
| Service has no container association | Works normally |
| Associated container disappears | Association remains, UI marks target missing |
| Application restarts | Latest service-health state and activity history reload |
| Activity exceeds retention | Oldest/expired events are pruned deterministically |

## 8. Notification and alert boundary

M7 creates service-health state and activity evidence.

M7 must not introduce a competing persistent alert lifecycle.

Do not add:

- Active/Acknowledged/Resolved alerts;
- configurable service thresholds;
- repeated service-outage notification logic that bypasses M8 governance.

Preserve existing notifications unrelated to M7 unless a narrowly scoped compatibility fix is required.

M8 will consume M6/M7 signals to implement governed alerts and threshold configuration.

## 9. Testing and validation strategy

### 9.1 During implementation

Run focused tests after each Work Unit.

Do not run the complete repository suite after every Work Unit unless repository policy explicitly requires it.

Do not remove, weaken, or skip valuable tests to make validation pass.

### 9.2 Required automated coverage

At minimum:

#### HTTP checker

- 2xx;
- bounded redirect/3xx;
- 4xx;
- 5xx;
- timeout;
- connection failure;
- malformed/unsupported URL;
- response-time population;
- no external internet dependency.

#### Service state machine

- Unknown initial state;
- success → Healthy;
- first failure → Degraded;
- second failure → Degraded;
- third failure → Unavailable;
- further failures remain Unavailable without duplicate transition spam;
- recovery → Healthy;
- last-success timestamp behavior;
- persistence round-trip.

#### Association

- no association;
- valid association;
- change/clear association;
- stale/missing container;
- backward-compatible loading of existing service configuration.

#### Activity

- event persistence;
- newest-first ordering;
- state-transition generation;
- unchanged-state deduplication;
- 30-day pruning;
- 2,000-event count pruning;
- persistence failure isolation;
- sensitive-data exclusion where testable.

#### Frontend

- representative service-health states;
- response time/timestamps;
- association display;
- missing association target;
- Recent Activity;
- Activity filters;
- empty/error states.

### 9.3 Milestone closure validation

Run the repository's canonical complete validation after implementation.

Use repository-defined commands and conventions rather than inventing substitutes.

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

Existing unrelated warnings may remain only when explicitly identified as pre-existing and non-regressive.

## 10. Live/read-only validation

If the development environment can safely access configured services, perform read-only smoke checks against representative registered services.

Permitted examples:

- endpoint responds successfully;
- response time is shown;
- service state displays correctly;
- associated container metadata displays correctly.

Do not:

- stop containers solely to create a failure;
- restart devices;
- shut down devices;
- modify firewall/network configuration;
- disable TLS validation;
- mutate application data.

If a safe failure/recovery case cannot be produced, automated local HTTP test fixtures are sufficient for implementation evidence and the missing real-world scenario should be recorded as residual manual validation.

## 11. Documentation and AIQT completion

At milestone start/integration:

- place this document at `docs/milestones/m7-service-health-and-operational-activity.md`;
- update README/documentation navigation where appropriate;
- preserve M6 as completed history;
- transition M7 through the repository's native AIQT workflow;
- preserve later milestones as planned/not started except for the roadmap amendment in Section 12.

At milestone closure:

- update this document only where actual implementation materially differs;
- record material deviations and rationale;
- update affected functional/technical specifications;
- update AIQT Work Unit and milestone state based on actual evidence;
- do not mark unimplemented or unvalidated requirements complete.

## 12. Roadmap amendment — add final Document Hygiene milestone

As part of M7 planning reconciliation, extend the existing post-MVP roadmap and AIQT planned milestone list with one final milestone after M12:

### M13 — Document Hygiene

Objective:

Perform a final documentation-quality and governance-reconciliation pass after the planned product milestones are complete.

M13 is intentionally the final planned milestone.

High-level scope:

- audit README and all `docs/` content against the implemented product;
- identify stale, contradictory, duplicated, orphaned, or misleading documentation;
- reconcile roadmap, milestone documents, functional specification, technical specification, release documentation, and AIQT milestone/status references;
- ensure completed work is described as completed and future work is not presented as implemented;
- normalize terminology and milestone naming;
- improve documentation navigation/indexing;
- validate internal documentation links/references;
- archive, merge, or remove obsolete documents where evidence supports doing so;
- remove temporary/generated documentation artifacts that should not remain canonical;
- ensure security/operational documentation matches actual behavior;
- record residual documentation debt explicitly.

M13 must not be used to introduce unrelated product functionality.

Minor code/configuration changes are allowed only when required to repair documentation tooling, broken documentation validation, or an obvious mismatch where documentation cannot be made truthful without a narrowly scoped correction.

During M7:

- add M13 to `docs/roadmap/post-mvp-product-roadmap.md`;
- add M13 to the canonical AIQT plan as `planned`;
- do not start M13;
- do not create a full M13 implementation specification unless repository conventions require one.

Detailed M13 Work Units and acceptance criteria will be defined when M13 is started.

## 13. Exit criteria

M7 is implementation-complete when:

1. WU7-01 through WU7-05 are implemented.
2. Registered HTTP/HTTPS services have deterministic Unknown/Healthy/Degraded/Unavailable health state.
3. Response time, latest check, latest success, and consecutive failures are represented.
4. Three consecutive failures produce Unavailable and a successful check recovers immediately to Healthy.
5. Service checks are bounded and isolated from unrelated device/service monitoring.
6. Optional service-to-container association persists and handles missing targets safely.
7. Meaningful operational activity is persisted.
8. Activity retention enforces both 30-day and 2,000-event bounds.
9. Repeated unchanged polling does not create activity spam.
10. Dashboard Recent Activity uses persistent activity data.
11. Service-health and Activity UI requirements are implemented.
12. M6 device-health semantics remain intact and service state does not directly mutate DeviceHealth.
13. Required automated tests pass.
14. Canonical milestone validation passes.
15. Documentation and AIQT state reflect the actual implementation.
16. M8–M12 remain planned/not started.
17. M13 — Document Hygiene is added as the final planned milestone and remains not started.

Safe live/read-only service validation should be included when available. If unavailable, it must be listed accurately as residual validation rather than falsely reported as performed.

## 14. Non-goals carried forward

M7 completion must not introduce:

- persistent Alert acknowledgement/resolution;
- configurable health thresholds;
- custom authenticated service checks;
- TCP monitoring;
- time-series monitoring;
- device shutdown/restart;
- device power-on;
- arbitrary remote administration.

Those remain assigned to later milestones or future product decisions.
