# Pi-Hub — M8 Alerts and Threshold Governance

Version: 0.1  
Status: Approved for implementation  
Date: 2026-08-10  
Depends on: M7 — Service Health and Operational Activity completed and merged  
Next milestone: M9 — Docker Operational Visibility

## 1. Purpose and authority

This document is the implementation source of truth for:

`M8 — Alerts and Threshold Governance`

It defines the approved alert lifecycle, threshold policy, persistence semantics, notification governance, UI requirements, Work Units, acceptance criteria, validation requirements, and milestone exit criteria.

The implementation may adapt details to the current repository architecture and established conventions, but the functional outcomes and safety/governance boundaries in this document must be preserved unless a material deviation is explicitly documented.

This document must be integrated into the repository as:

`docs/milestones/m8-alerts-and-threshold-governance.md`

## 2. Objective

Turn Pi-Hub health signals and operational state into governed, persistent alerts without generating notification noise.

At completion, Pi-Hub should answer:

- Which problems currently require attention?
- Which alerts are new versus already acknowledged?
- Which alerts have resolved?
- What device/service condition created each alert?
- Which threshold policy caused the alert?
- Are repeated samples creating one alert occurrence or notification spam?
- Which thresholds are global defaults and which are overridden for a device?
- When should Pi-Hub send a notification for an alert transition?

M8 must distinguish three concepts:

- **Event** — something happened; M7 Activity records meaningful operational events.
- **Alert** — a persistent condition requiring attention.
- **Notification** — a user-facing message emitted because alert governance says a transition should be surfaced.

These concepts must not be conflated.

## 3. Scope

### 3.1 In scope

M8 includes:

- persistent Alert domain model;
- `Active` / `Acknowledged` / `Resolved` lifecycle;
- `Info` / `Warning` / `Critical` severities;
- stable alert identity/deduplication keys;
- alert derivation from supported M6/M7 health signals;
- global threshold configuration;
- per-device threshold overrides;
- effective-threshold resolution;
- duration/consecutive-sample governance for volatile metrics;
- migration of M6 fixed thresholds to policy-driven evaluation;
- configurable service-unavailable failure threshold;
- persistent alert evaluation state needed for debouncing/duration rules;
- notification decisions driven by alert lifecycle transitions;
- prevention of duplicate notifications from legacy and new paths;
- global Alert Center;
- device-level alert presentation;
- threshold settings UI;
- behavioral frontend coverage;
- documentation/AIQT reconciliation and milestone validation.

### 3.2 Out of scope

M8 does not include:

- email/SMS notification providers;
- new third-party notification channels;
- alert escalation to multiple users;
- multi-user ownership/assignment;
- scheduled maintenance windows;
- alert snoozing until a future time;
- alert comments;
- arbitrary alert-rule scripting;
- custom expressions;
- per-service custom HTTP status rules;
- historical metric charts;
- Docker log analysis;
- automatic remediation;
- device restart/shutdown;
- Power On;
- M9 Docker operational expansion;
- M12 historical time-series storage.

Existing desktop notification mechanisms may be governed/reused by M8; introducing new providers is not required.

## 4. Alert domain decisions

### 4.1 Alert lifecycle

Every alert occurrence has one of:

- `Active`
- `Acknowledged`
- `Resolved`

#### Active

The source condition is currently present and the user has not acknowledged this occurrence.

#### Acknowledged

The source condition is still present, but the user has explicitly acknowledged the alert.

Acknowledgement means:

- the alert remains open;
- it remains visible as unresolved;
- repeated unchanged samples do not create new alerts;
- repeated unchanged samples do not create repeat notifications;
- the acknowledgement remains attached to this alert occurrence until it resolves.

#### Resolved

The source condition is no longer present, or the monitored source has been removed/disabled such that the condition can no longer remain active.

Resolution is normally automatic and evidence-driven.

M8 does **not** provide a "force resolve while condition is still active" action. A user may acknowledge an active condition, but Pi-Hub should not let the UI claim a live fault is resolved when the evaluator still observes it.

If the condition later returns after resolution, it creates a **new alert occurrence** with a new alert ID while using the same stable deduplication key.

### 4.2 Severity

Supported severities:

- `Info`
- `Warning`
- `Critical`

Severity is part of the evaluated condition.

If an existing active/acknowledged alert becomes more severe:

- update the same alert occurrence;
- record the severity transition;
- allow a notification for escalation according to Section 8.

If severity decreases while the same condition remains present:

- update severity;
- keep the same lifecycle state;
- do not treat the decrease as resolution;
- do not create a new alert.

### 4.3 Stable identity and deduplication

An alert occurrence has:

- unique alert ID;
- stable deduplication key;
- source type/category;
- device ID when applicable;
- related entity ID when applicable;
- health/reason/rule code;
- current severity;
- lifecycle state;
- first seen;
- last seen;
- occurrence/update count;
- acknowledged timestamp when applicable;
- resolved timestamp when applicable;
- resolution reason when applicable;
- bounded non-sensitive summary/detail;
- effective threshold/rule metadata needed to explain why it fired.

The deduplication key should be stable for the logical condition, conceptually:

`source + device + entity + reason/rule`

Do not use timestamps or transient message text in the deduplication key.

### 4.4 Repeated evaluation

While the same logical condition remains present:

- update `lastSeen`;
- increment or update the bounded occurrence/evaluation count as appropriate;
- keep one alert occurrence;
- do not create duplicate alert rows;
- do not create duplicate transition activity;
- do not repeatedly notify.

### 4.5 Source removal

If a monitored service/device/rule target is deleted or disabled:

- do not leave an impossible active alert indefinitely;
- resolve affected alerts deterministically with a source-removed/source-disabled resolution reason where appropriate.

## 5. Alert sources and default severity mapping

M8 consumes existing deterministic state rather than duplicating M6/M7 logic where possible.

### 5.1 Device connectivity

Supported alert source:

- unexpected device offline / unreachable state.

Default severity:

- device offline → `Critical`.

Specific SSH security/authentication states remain visible through M6 diagnostics and may generate alerts only if the existing monitoring model exposes them as stable ongoing conditions. If included:

- host-key mismatch/change → `Critical`;
- repeated authentication failure → `Warning` or `Critical` according to existing semantics.

Do not weaken or auto-accept SSH host keys.

Planned offline semantics for future M10 must be compatible with this model: a future intentional shutdown must be able to suppress/resolve the unexpected-offline alert path rather than create a false Critical alert.

### 5.2 M6 device-health reasons

Existing M6 machine-readable health reasons are alert inputs.

At minimum:

- disk threshold breaches;
- temperature threshold breaches;
- undervoltage/current power condition;
- historical-since-boot undervoltage where M6 still classifies it as Warning;
- throttling/frequency-capping conditions;
- unexpected read-only root filesystem.

Preserve M6 reason codes where practical rather than creating parallel reason taxonomies.

Default severity follows the effective M6 health-reason severity unless M8 explicitly makes the threshold configurable.

### 5.3 M7 service health

Default mapping:

- `Healthy` → no active service-health alert;
- `Unknown` → no alert by itself;
- `Degraded` → `Warning`;
- `Unavailable` → `Critical`.

The same service occurrence should evolve from Warning to Critical if failures continue from Degraded to Unavailable.

Recovery to Healthy resolves the service-health alert occurrence.

### 5.4 Container signals

M8 may consume existing reliable container health/state signals only when the current M7/M6 implementation already exposes them deterministically.

If included:

- `unhealthy` → `Warning`;
- unexpectedly stopped container that is configured/expected to be running → `Warning` or `Critical` only if the current model can establish that expectation reliably.

Do not invent a new container desired-state system in M8. M9 owns deeper Docker operational visibility.

## 6. Threshold governance

### 6.1 Configuration hierarchy

Thresholds have two configuration levels:

1. **Global defaults**
2. **Per-device overrides**

Effective value resolution:

`device override if present → otherwise global default`

M8 does not require per-service threshold overrides.

A per-device override may be cleared to return to the current global default.

### 6.2 M8 default threshold policy

M8 migrates M6 fixed values into configuration while preserving equivalent default behavior.

Default values:

#### CPU usage

- Warning: `>= 85%`
- Critical: `>= 95%`
- Minimum breach duration: `5 minutes`

CPU threshold evaluation applies only if the existing snapshot provides a reliable CPU-usage metric.

#### Memory usage

- Warning: `>= 90%`
- Critical: `>= 95%`
- Minimum breach duration: `5 minutes`

Memory threshold evaluation applies only if the existing snapshot provides a reliable comparable percentage.

#### Disk usage

- Warning: `>= 85%`
- Critical: `>= 95%`
- Breach duration: immediate / no duration gate

These defaults preserve the M6 disk thresholds.

#### Temperature

- Warning: `>= 70°C`
- Critical: `>= 80°C`
- Minimum consecutive samples: `2`

These defaults preserve the M6 temperature thresholds while reducing one-sample noise.

#### Service unavailability

- `Degraded` starts on first failed check as defined by M7;
- Unavailable threshold default: `3 consecutive failures`.

M8 makes the Unavailable failure-count threshold configurable globally and per device.

The configured Unavailable threshold must be:

- minimum `2`;
- maximum `10`.

#### Device offline

Use the existing stable connection-state transition rather than introducing a raw ping-count rule if the current monitoring architecture already debounces connection status.

If the existing implementation has no meaningful protection from single transient refresh failure, add the smallest deterministic consecutive-failure/debounce rule consistent with the monitoring architecture and document it.

### 6.3 Validation constraints

Threshold settings must reject invalid combinations.

At minimum:

- percentages: `0–100`;
- Warning threshold must be strictly lower than Critical threshold for the same metric;
- temperature values must be within a sane supported range;
- duration must be non-negative and bounded;
- consecutive-sample counts must be bounded positive integers;
- service unavailable failure threshold: `2–10`.

Invalid threshold configuration must not be persisted.

### 6.4 Unsupported metrics

If a metric is unavailable/unsupported:

- do not generate a threshold alert;
- do not treat unavailable as zero;
- do not falsely resolve another independent alert;
- preserve M6 partial-data semantics.

### 6.5 Effective-policy visibility

The UI should make clear whether a displayed threshold is:

- inherited from global defaults;
- overridden for the current device.

Users must be able to clear an override.

## 7. Volatile-metric breach state

Duration-based and consecutive-sample rules require bounded evaluation state.

The implementation must represent enough state to determine whether a candidate breach has matured into an alert.

For duration-based metrics, track the equivalent of:

- rule/source key;
- current candidate severity;
- breach started at;
- last evaluated at.

For consecutive-sample metrics, track:

- current candidate severity;
- consecutive matching sample count.

Required behavior:

- candidate state does not appear as an Active alert before the rule matures;
- returning below the relevant threshold before maturity clears the candidate;
- escalating directly into Critical may use the Critical rule independently of Warning maturity;
- application restart must not create false immediate alerts.

Preferred behavior is to persist candidate state when it fits existing storage cleanly.

If candidate state is intentionally reset on application restart, the implementation must reset safely, document the behavior, and never backdate or fabricate threshold duration.

## 8. Notification governance

### 8.1 Notification triggers

M8 uses alert transitions to decide when existing desktop notifications are emitted.

By default, notify for:

- new `Warning` alert occurrence;
- new `Critical` alert occurrence;
- severity escalation from Warning to Critical.

Do not notify for:

- every repeated sample;
- every `lastSeen` update;
- acknowledgement;
- severity decrease;
- `Info` alerts by default;
- candidate threshold state before an alert matures.

Resolution notifications are not required in M8 unless preserving an existing device-online/recovery notification behavior. If an existing recovery notification remains, it must not produce duplicate messages through both legacy and alert paths.

### 8.2 Acknowledgement behavior

Acknowledgement suppresses repeat notifications for the same unchanged occurrence.

If an acknowledged Warning escalates to Critical:

- the alert remains the same occurrence;
- acknowledgement may remain recorded;
- a Critical escalation notification is allowed because severity materially increased.

Do not create a new occurrence merely to bypass acknowledgement.

### 8.3 Duplicate prevention

Conditions governed by M8 must not also generate duplicate notifications through older direct notification code paths.

When migrating an existing notification condition:

- preserve intended user-visible behavior;
- route the decision through alert governance or explicitly suppress one path;
- add tests proving one transition creates at most one notification.

### 8.4 Notification failure

Failure to deliver a desktop notification:

- does not roll back or delete the alert;
- does not change alert lifecycle state;
- may create bounded diagnostic/log evidence according to existing conventions;
- must not recursively generate another alert about the notification failure in M8.

## 9. Alert persistence and retention

Alerts are durable across application restart.

Persist active, acknowledged, and resolved occurrences.

### 9.1 Active/acknowledged retention

Never prune currently Active or Acknowledged alerts solely because of age/count retention.

### 9.2 Resolved retention

Default resolved-alert retention:

- maximum age: `90 days`;
- maximum count: `1,000 resolved alerts`.

Prune oldest resolved occurrences when either bound is exceeded.

This retention is independent from the M7 Activity retention.

### 9.3 Persistence failure

Alert persistence failure must be surfaced according to repository conventions.

Do not knowingly emit a notification for a new alert that cannot be represented durably if the architecture can avoid that inconsistency.

If persistence and notification cannot be made atomic with the existing architecture, prefer:

1. persist alert transition;
2. then notify.

Document any unavoidable failure window.

## 10. Activity integration

M7 Activity remains the operational event history.

M8 should add meaningful alert lifecycle events, at minimum:

- alert activated;
- alert severity escalated;
- alert acknowledged;
- alert resolved.

Do not create an Activity event for every repeated evaluation.

Activity events should reference:

- alert ID;
- device/entity context;
- stable reason/rule code;
- concise non-sensitive summary.

The Alert Center is the source of truth for current alert lifecycle. Activity is historical evidence.

## 11. Work Units

### WU8-01 — Alert domain, lifecycle, persistence, and deduplication

#### Objective

Introduce persistent governed alert occurrences with deterministic lifecycle and deduplication.

#### Required implementation

Create typed domain concepts for the equivalent of:

- Alert;
- AlertState;
- AlertSeverity;
- AlertSource/Category;
- AlertReason/RuleCode;
- AlertDeduplicationKey;
- resolution reason;
- acknowledgement metadata.

Implement:

- create/activate;
- repeated-condition update;
- severity change;
- acknowledge;
- resolve;
- recreate new occurrence after resolution;
- source removal resolution;
- persistence;
- resolved-alert retention.

#### Acceptance criteria

- same logical ongoing condition creates one alert occurrence;
- repeated evaluations update `lastSeen` without duplicate records;
- acknowledgement preserves unresolved state;
- active condition cannot be falsely force-resolved through the UI/domain API;
- condition clearing resolves automatically;
- condition returning after resolution creates a new alert ID;
- severity escalation updates same occurrence;
- Active/Acknowledged alerts are never age-pruned;
- resolved alerts obey 90-day / 1,000-count retention;
- persistence reload preserves lifecycle accurately;
- tests cover lifecycle, dedupe, escalation, acknowledgement, recovery, reoccurrence, source removal, and retention.

### WU8-02 — Threshold policy and effective configuration

#### Objective

Replace hard-coded M6/M7 threshold policy where specified with validated global defaults and per-device overrides.

#### Required implementation

Represent global and device-effective policy for:

- CPU Warning/Critical/duration;
- Memory Warning/Critical/duration;
- Disk Warning/Critical;
- Temperature Warning/Critical/consecutive samples;
- service Unavailable consecutive-failure threshold;
- any necessary connection debounce setting only if required by existing architecture.

Preserve current M6/M7 behavior through defaults unless this document explicitly changes it.

Implement:

- global settings persistence;
- per-device override persistence;
- effective-policy resolution;
- clear override;
- validation;
- backward-compatible defaults/migration for existing installations.

#### Acceptance criteria

- existing users receive documented defaults without configuration corruption;
- device override wins over global;
- clearing override immediately restores inherited global value;
- invalid values/order are rejected;
- unsupported metrics do not produce alerts;
- M6 health assessment consumes effective threshold policy rather than maintaining conflicting hard-coded values;
- M7 service Unavailable transition consumes the effective failure threshold;
- tests cover defaults, overrides, validation, migration, and effective resolution.

### WU8-03 — Alert evaluation and signal integration

#### Objective

Evaluate supported M6/M7 signals into alert occurrences using the effective policy.

#### Required sources

At minimum integrate:

- unexpected device offline;
- M6 disk threshold reasons;
- M6 temperature threshold reasons;
- reliable CPU/memory threshold signals if supported by current snapshot;
- Raspberry Pi power/throttling reasons already exposed by M6;
- unexpected read-only root filesystem;
- M7 service Degraded;
- M7 service Unavailable;
- service recovery.

Integrate reliable existing container signals only if this can be done without inventing M9 desired-state semantics.

#### Volatile rules

Implement candidate breach behavior required by Section 7.

Required outcomes:

- CPU/memory do not alert before minimum duration;
- temperature does not alert before required consecutive samples;
- disk is immediate;
- service state uses configured failure threshold;
- same logical service alert escalates from Warning to Critical rather than duplicating.

#### Acceptance criteria

- device/service signal transitions deterministically activate/update/resolve alerts;
- device health and alert state remain separate domain concepts;
- one signal does not delete unrelated alerts;
- threshold candidate state cannot create premature notifications;
- recovery resolves the matching alert only;
- tests cover maturity, pre-maturity recovery, escalation, unsupported data, service recovery, and source isolation.

### WU8-04 — Notification and Activity governance

#### Objective

Route user notification decisions through alert transitions and record meaningful lifecycle events without noise.

#### Required behavior

Implement default notification rules from Section 8.

Ensure:

- one new Warning/Critical occurrence → at most one initial notification;
- repeated unchanged evaluation → no repeated notification;
- Warning→Critical escalation → one escalation notification;
- acknowledgement → no notification;
- acknowledgement suppresses unchanged repeat notifications;
- existing direct notification paths do not duplicate M8 notifications;
- notification failure does not corrupt alert state.

Add M7 Activity entries for:

- activation;
- escalation;
- acknowledgement;
- resolution.

#### Acceptance criteria

- automated tests prove duplicate prevention;
- existing unrelated notification behavior remains intact;
- migrated alert-driven notification behavior is not double-fired;
- lifecycle Activity events are one-per-transition, not one-per-poll;
- notification payloads contain no secrets/sensitive URLs/query strings;
- notification delivery failure is isolated.

### WU8-05 — Alert Center and threshold settings UI

#### Objective

Provide clear operational alert management and threshold configuration without overloading the Dashboard.

#### Global Alert Center

Add an `Alerts` navigation surface or equivalent canonical location.

Required capabilities:

- newest/most relevant unresolved alerts visible first;
- filter by:
  - lifecycle state;
  - severity;
  - device;
  - category/source;
- display:
  - severity;
  - state;
  - summary;
  - device/entity context;
  - first seen;
  - last seen;
  - acknowledgement/resolution status;
- acknowledge Active alerts;
- view Resolved history;
- no force-resolve action for a still-active condition;
- clear empty/loading/error states.

Recommended ordering:

1. unresolved Critical;
2. unresolved Warning;
3. unresolved Info;
4. Resolved history by most recent resolution.

Within the same group, newest/latest relevant alert first.

#### Dashboard

Add a concise alert summary without making device cards overly dense.

Recommended:

- count of unresolved Critical/Warning alerts;
- optional top active alerts;
- navigation to Alert Center.

Do not duplicate the entire Alert Center on Dashboard.

#### Device Details

Show active/acknowledged alerts for the selected device.

Resolved history may be linked rather than fully embedded.

#### Threshold settings

Global Settings:

- edit global thresholds;
- validation messages;
- restore documented defaults.

Device Settings:

- show effective value;
- indicate `Inherited` vs `Override`;
- set override;
- clear override.

Do not add per-service threshold overrides in M8.

#### Accessibility

- severity/state must not rely solely on color;
- official text labels must be present;
- controls/actions need accessible names;
- confirmation is not required for acknowledgement because it is reversible only through resolution, but the action must be explicit.

#### Acceptance criteria

Frontend behavioral tests cover:

- Active Warning;
- Active Critical;
- Acknowledged alert;
- Resolved alert;
- severity escalation display;
- filtering;
- acknowledgement action;
- unresolved count;
- inherited threshold;
- overridden threshold;
- clearing override;
- invalid threshold validation;
- empty/loading/error states;
- no force-resolve control for active conditions.

## 12. Cross-Work-Unit scenarios

| Scenario | Required behavior |
|---|---|
| Disk reaches 85% with default policy | Warning alert activates immediately |
| Disk reaches 95% | Same logical disk alert escalates to Critical where rule identity permits |
| Disk falls below warning threshold | Alert resolves |
| CPU spikes above warning briefly | Candidate only; no alert before 5 minutes |
| CPU stays above warning for 5 minutes | Warning alert activates |
| CPU returns normal before maturity | Candidate clears; no alert |
| Temperature breaches threshold once | No alert until 2 qualifying samples |
| Device override changes temperature warning | Effective policy uses override |
| Override cleared | Effective policy immediately inherits global |
| Service first failure | Degraded → Warning service alert |
| Service third failure with default threshold | Same occurrence escalates to Critical / Unavailable |
| Service threshold overridden to 5 | Critical/Unavailable transition occurs at 5 failures |
| Service recovers | Service alert resolves |
| Same condition persists for many refreshes | One alert, one initial notification, no spam |
| User acknowledges alert | State becomes Acknowledged; condition remains open |
| Acknowledged Warning escalates to Critical | Same alert; one escalation notification allowed |
| Condition clears after acknowledgement | Alert resolves automatically |
| Condition returns later | New alert occurrence |
| Active source is deleted | Alert resolves with source-removed reason |
| App restarts | Open alerts and threshold settings reload correctly |
| Resolved history exceeds retention | Oldest resolved alerts are pruned; open alerts retained |
| Notification delivery fails | Alert remains persisted and correct |
| Planned shutdown exists in future M10 | Architecture allows offline alert suppression rather than contradiction |

## 13. Migration and compatibility

M8 touches existing M6/M7 rules and existing notification paths.

Required migration behavior:

- existing installations with no threshold settings receive M8 defaults;
- existing device configuration remains readable;
- M6 health reasons remain stable where practical;
- M7 persisted service-health state remains readable;
- M7 service failure streak is not reset solely because threshold governance is introduced unless unavoidable and documented;
- M7 Activity storage remains compatible;
- existing alerts do not exist pre-M8, so initial alert evaluation after upgrade may create alerts for conditions already present.

For the initial post-upgrade evaluation:

- do not fabricate historical first-seen timestamps;
- use the first M8 observation as `firstSeen`;
- do not emit multiple duplicate notifications for the same current condition;
- each logical current condition may generate one initial governed notification if it qualifies.

## 14. Testing and validation strategy

### 14.1 During implementation

Use focused tests after each Work Unit.

Do not run the complete suite after every Work Unit unless repository policy requires it.

Do not weaken or delete valuable tests to obtain green status.

### 14.2 Required automated coverage

At minimum:

#### Alert domain

- create;
- dedupe;
- acknowledge;
- resolve;
- reoccurrence after resolution;
- escalation;
- de-escalation;
- source removal;
- persistence;
- retention.

#### Threshold policy

- defaults;
- global edits;
- device override;
- clear override;
- invalid ordering;
- bounds validation;
- migration/default loading;
- service failure threshold range.

#### Evaluation

- immediate disk;
- duration CPU/memory;
- temperature consecutive samples;
- candidate clearing;
- unsupported metrics;
- service Degraded;
- service Unavailable;
- service threshold override;
- service recovery;
- alert isolation.

#### Notifications

- initial Warning;
- initial Critical;
- no repeat spam;
- escalation;
- acknowledgement suppression;
- no duplicate legacy path;
- delivery failure isolation.

#### Activity

- activation;
- escalation;
- acknowledgement;
- resolution;
- no per-poll spam.

#### Frontend

- lifecycle/severity states;
- Alert Center filters;
- acknowledgement;
- Dashboard unresolved counts;
- device alerts;
- threshold inheritance/override;
- validation;
- empty/error/loading states.

### 14.3 Milestone closure validation

Run the repository's canonical complete validation after all WUs are implemented.

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

If a canonical CI-equivalent command exists, prefer it for final closure.

Existing unrelated warnings may remain only when confirmed pre-existing and non-regressive.

## 15. Live/read-only validation

If the development environment has safe access to configured devices/services, perform non-destructive validation where practical.

Permitted examples:

- confirm an already-existing current health condition appears as an alert;
- confirm current healthy services do not create false alerts;
- inspect effective threshold UI/configuration;
- acknowledge a test alert only if doing so does not mutate a real operational decision unexpectedly.

Do not:

- fill disks;
- overheat devices;
- deliberately create undervoltage;
- stop containers/services merely to create alerts;
- shut down/restart devices;
- modify firewall/networking;
- weaken TLS/SSH security.

Use deterministic local fixtures/domain tests for failure/escalation scenarios that would require destructive live manipulation.

If real-world evidence is unavailable, record it honestly as residual validation.

## 16. Documentation and AIQT completion

At milestone start/integration:

- place this document at `docs/milestones/m8-alerts-and-threshold-governance.md`;
- update README/documentation navigation where appropriate;
- preserve M1–M7 completed history;
- transition M8 using the repository's native AIQT workflow;
- keep M9–M13 planned/not started.

At milestone closure:

- update this document only where actual implementation materially differs;
- record material deviations/rationale;
- update affected functional/technical specifications;
- update AIQT Work Unit/milestone state from evidence;
- preserve the PR template introduced during M7 and follow it for the M8 PR;
- do not mark unimplemented/unvalidated requirements complete.

## 17. Exit criteria

M8 is implementation-complete when:

1. WU8-01 through WU8-05 are implemented.
2. Alerts persist with deterministic Active/Acknowledged/Resolved lifecycle.
3. Ongoing identical conditions deduplicate into one occurrence.
4. Resolved conditions recurring later create a new occurrence.
5. Severity escalation updates the same open occurrence.
6. Global thresholds and per-device overrides are persisted and validated.
7. M6 fixed disk/temperature thresholds are policy-driven with equivalent defaults.
8. CPU/memory duration rules work where reliable metrics exist.
9. Temperature consecutive-sample governance works.
10. M7 service Unavailable threshold is configurable through effective policy.
11. M6 device health and M7 service health remain distinct from alert lifecycle.
12. Notifications are driven by alert transitions without repeated poll spam.
13. Existing migrated notification paths do not duplicate governed notifications.
14. Alert lifecycle transitions create meaningful M7 Activity evidence.
15. Alert Center and threshold settings UI are implemented.
16. Active/Acknowledged alerts are retained; resolved retention is bounded.
17. Required automated tests pass.
18. Canonical milestone validation passes.
19. Documentation and AIQT state reflect actual implementation.
20. M9–M13 remain planned/not started.

Safe live/read-only validation should be included when available. If unavailable, it must be listed accurately as residual validation rather than falsely reported as performed.

## 18. Non-goals carried forward

M8 completion must not introduce:

- arbitrary alert scripting;
- multi-user alert ownership;
- new notification providers;
- maintenance windows;
- historical charts;
- automatic remediation;
- device restart/shutdown;
- device power-on;
- Docker operational expansion assigned to M9.

Those remain assigned to later milestones or future product decisions.
