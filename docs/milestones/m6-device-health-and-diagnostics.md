# Pi-Hub â€” M6 Device Health and Diagnostics

Version: 0.2  
Status: Approved for implementation  
Date: 2026-08-09  
Depends on: M1â€“M5 completed baseline  
Next milestone: M7 â€” Service Health and Operational Activity

## 1. Purpose and authority

This document is the implementation source of truth for M6 â€” Device Health and Diagnostics.

The milestone must be delivered as five sequential Work Units. Implementation details may adapt to the existing repository architecture, but the functional outcomes, safety boundaries, acceptance criteria, and exit criteria in this document must be preserved unless a material deviation is explicitly documented.

## 2. Objective

Extend Pi-Hub from basic device-state monitoring into a health and diagnostic control surface.

At completion, the user must be able to determine:

- whether a managed device is reachable;
- whether it is operationally healthy;
- which health condition is responsible for Warning or Critical state;
- whether Raspberry Pi power/throttling conditions are current or historical;
- which stage of connectivity is failing when connection diagnostics fail.

## 3. Scope

### 3.1 In scope

M6 includes:

- extended device-health snapshot data;
- Raspberry Pi power/throttling interpretation;
- deterministic consolidated device health;
- structured connectivity diagnostics;
- health and diagnostic presentation in Dashboard and Device Details;
- graceful behavior for unsupported/partial metrics;
- focused and milestone-level validation.

### 3.2 Out of scope

M6 does not include:

- persistent historical metric storage;
- HTTP/HTTPS service monitoring;
- persistent activity history;
- global alert lifecycle;
- configurable thresholds;
- Docker logs/detail expansion beyond data already available;
- device restart;
- device shutdown;
- system package installation/update;
- smart-plug, PoE, Wake-on-LAN, or GPIO power management.

Device restart and shutdown belong to M10.

## 4. Required architecture and safety boundaries

M6 must preserve the existing Pi-Hub architecture and established repository conventions.

Preferred dependency direction:

Frontend  
â†’ typed Tauri/backend commands  
â†’ application use cases/services  
â†’ domain models/rules  
â†’ SSH/infrastructure collectors and parsers

Requirements:

- health classification must not live in React components;
- no arbitrary remote-command API may be introduced;
- remote commands must remain predefined and bounded;
- SSH host-key verification must remain mandatory;
- one device failure must not affect monitoring of another device;
- optional metric failure must not fail the complete device snapshot;
- Raspberry-Pi-specific commands must degrade gracefully on compatible non-Raspberry-Pi Linux devices;
- no SSH password, private-key content, or sudo password may be stored;
- existing embedded-terminal and Docker lifecycle behavior must not regress;
- avoid unrelated refactoring.

## 5. Work Units

### WU6-01 â€” Extended device-health snapshot

#### Objective

Extend the existing remote snapshot with health-relevant system data while preserving partial-data tolerance.

#### Required data

Collect when available:

- load average:
  - 1 minute;
  - 5 minutes;
  - 15 minutes;
- swap:
  - total;
  - used;
- CPU frequency;
- last boot / boot timestamp;
- reboot-required state where the OS exposes it;
- root-filesystem read-only/read-write state where practical;
- Raspberry Pi throttling raw value where `vcgencmd` is available.

Collection strategy should minimize avoidable SSH round trips where values can safely be obtained together without making parsing brittle.

#### Required behavior

- every new field is independently optional/unsupported;
- missing or unsupported values are represented explicitly, not as zero;
- malformed optional output does not invalidate unrelated metrics;
- remote collection remains bounded by existing or explicit timeouts;
- generic Linux hosts must not fail because `vcgencmd` is absent;
- Pi 2 and Pi 5 must remain compatible with the collector design.

#### Acceptance criteria

- existing CPU, memory, disk, temperature, uptime, Docker, and connection behavior remains functional;
- load average is correctly parsed from valid output;
- swap total/used is correctly parsed and units are consistent with the domain model;
- CPU frequency is parsed where supported and unavailable otherwise;
- boot information is represented without depending on locale-specific output;
- reboot-required is represented as supported true/false or unavailable;
- filesystem state does not report read-only when the value is unknown;
- throttling raw output can be carried forward for WU6-02;
- tests cover valid, empty, malformed, unsupported, and partial-output cases.

#### Validation

Focused validation must cover collector/parser behavior and existing snapshot regression paths affected by the change.

---

### WU6-02 â€” Raspberry Pi power and throttling interpretation

#### Objective

Convert Raspberry Pi `vcgencmd get_throttled` output into typed, user-meaningful state.

#### Required model

Represent, independently:

- undervoltage now;
- undervoltage occurred since boot;
- frequency capping now;
- frequency capping occurred since boot;
- throttling now;
- throttling occurred since boot;
- soft temperature limit now;
- soft temperature limit occurred since boot.

The raw throttling value should remain available for diagnostics/debugging where useful, but must not be the only representation exposed to the UI.

#### Semantic requirements

Three states must be distinguishable for every supported flag:

- condition present;
- condition absent;
- information unavailable/unknown.

Current and historical-since-boot conditions must never be conflated.

Historical flags remain relevant even if the condition is not currently active.

#### Acceptance criteria

- standard hexadecimal throttling output is parsed deterministically;
- `0x0` produces known-clear flags;
- each supported current-state bit is mapped correctly;
- each supported historical bit is mapped correctly;
- representative combined masks are mapped correctly;
- malformed output returns unavailable/error semantics without corrupting the device snapshot;
- absence of `vcgencmd` is treated as unsupported rather than unhealthy;
- tests cover zero, individual bits, combined masks, malformed output, and unsupported command behavior.

#### Validation

Focused tests must demonstrate the bitmask contract independently of SSH execution.

---

### WU6-03 â€” Device health assessment

#### Objective

Produce one deterministic domain-level health assessment from the current snapshot and connection state.

#### Health states

- `Healthy`
- `Warning`
- `Critical`
- `Unknown`

The assessment must contain:

- overall state;
- zero or more machine-readable health-reason codes;
- severity for each reason;
- enough structured data for the frontend to render concise and detailed views without reproducing health rules.

#### Initial health inputs

Use available current-state data from:

- connectivity state;
- CPU temperature;
- disk usage;
- memory pressure only where a reliable current metric exists;
- Raspberry Pi power/throttling state;
- root-filesystem read-only state;
- reboot-required state;
- Docker state only where the existing data model supports a reliable conclusion.

Unsupported metrics must not become health failures.

#### Seed thresholds for M6

Thresholds are constants in M6 and become configurable in M8.

Use:

- disk Warning: `>= 85%`;
- disk Critical: `>= 95%`;
- temperature Warning: `>= 70Â°C`;
- temperature Critical: `>= 80Â°C`;
- current undervoltage: Warning;
- historical undervoltage since boot: Warning;
- current frequency capping/throttling: at least Warning;
- unexpected root filesystem read-only: Critical;
- reboot required: informational reason; it does not by itself force Warning.

If existing supported hardware guidance requires a materially different threshold, document the deviation before changing the rule.

#### Connectivity semantics

Connection/authentication failures retain their existing specific connection states.

Do not flatten:

- timeout;
- host-key error;
- authentication failure;
- command error

into a generic health warning.

When a device snapshot is not trustworthy because the device cannot be reached/authenticated, overall device health should be `Unknown` rather than falsely `Healthy`.

#### Reason precedence

- `Critical` reason present â†’ overall `Critical`;
- otherwise `Warning` reason present â†’ overall `Warning`;
- otherwise, if sufficient supported health data is available â†’ `Healthy`;
- if health cannot be assessed reliably â†’ `Unknown`.

#### Acceptance criteria

- health rules are implemented outside React;
- overall state is deterministic for the same inputs;
- reason codes are stable and machine-readable;
- multiple simultaneous reasons are preserved;
- highest severity determines overall state;
- unsupported metrics do not create false warnings;
- connection failures do not create false health conclusions;
- threshold boundary behavior is tested exactly;
- health assessment has tests for Healthy, Warning, Critical, Unknown, and multi-reason precedence.

#### Validation

Focused domain tests must prove classification and boundary behavior without requiring SSH.

---

### WU6-04 â€” Structured connectivity diagnostics

#### Objective

Evolve the existing Test Connection behavior into a structured, read-only diagnostic report that identifies the failing stage.

#### Required diagnostic stages

Where meaningful for the configured target, diagnostics should evaluate:

1. target/hostname resolution or target validity;
2. TCP connection to the configured SSH port;
3. SSH host-key verification;
4. SSH authentication;
5. predefined remote-command execution;
6. Docker availability;
7. Tailscale presence/state where discoverable.

A stage may be `skipped` when an earlier hard dependency failed or when the check is not applicable.

#### Diagnostic result contract

Each check must expose:

- stable diagnostic code;
- status:
  - `passed`;
  - `warning`;
  - `failed`;
  - `skipped`;
- concise user-facing summary;
- optional bounded technical detail;
- duration where useful.

The report should also expose overall completion/timing information where consistent with the existing architecture.

#### Required semantics

- Host Key Error remains distinct and is never bypassed automatically;
- authentication failure remains distinct from TCP timeout/failure;
- remote-command failure remains distinct from authentication failure;
- Docker unavailable does not mean SSH/device connectivity failed;
- Tailscale absent/unavailable does not fail a valid LAN/other SSH path;
- diagnostics must not mutate remote configuration;
- checks must be time-bounded;
- diagnostics for one device must not block monitoring of another;
- cancellation should be implemented when it fits the existing command/task architecture; otherwise explicit bounded timeout is sufficient for M6.

#### Acceptance criteria

Tests cover at minimum:

- all checks passing;
- target/TCP failure;
- SSH timeout;
- host-key error;
- authentication failure;
- remote-command failure;
- Docker unavailable while SSH remains healthy;
- Tailscale absent/unavailable;
- skipped downstream checks after a hard failure.

The existing connection-status semantics must remain backward compatible unless a documented migration is required.

#### Validation

Focused tests must verify diagnostic classification and failure isolation.

---

### WU6-05 â€” Health and diagnostics UI

#### Objective

Expose M6 information clearly without overloading the existing interface.

#### Dashboard requirements

Preserve the existing primary metrics:

- CPU;
- MEM;
- DISK;
- TEMP.

Add:

- consolidated health state;
- concise primary health reason when Warning/Critical;
- clear Unknown representation.

The card must not display all extended metrics.

At most one primary health reason should be shown on the card; detailed reasons belong in Device Details.

#### Device Details requirements

Add or extend sections to expose:

- overall device health;
- all active health reasons;
- load average;
- swap usage;
- CPU frequency where available;
- boot information;
- reboot-required state;
- root filesystem state where available;
- Power & Throttling state;
- structured diagnostics.

#### Power & Throttling presentation

The UI must distinguish:

- current undervoltage;
- historical undervoltage;
- current frequency capping;
- historical frequency capping;
- current throttling;
- historical throttling;
- current/historical soft temperature limit where supported.

Unknown/unavailable must not render as false/clear.

#### Diagnostics presentation

The existing Test Connection surface should evolve into, or lead to, a readable list of diagnostic checks with status and concise explanation.

The UI must preserve existing Pi-Hub visual language and avoid an unrelated redesign.

#### Acceptance criteria

- Healthy / Warning / Critical / Unknown are visually distinguishable;
- Dashboard remains scan-friendly at the current device-card density;
- detailed reasons are available in Device Details;
- current and historical Raspberry Pi power conditions are not conflated;
- unsupported values render as unavailable;
- diagnostics identify the failing stage;
- frontend does not reimplement health classification;
- UI tests cover representative health and diagnostic states;
- accessibility semantics for status text do not depend solely on color.

#### Validation

Run focused frontend/component tests plus the normal frontend static/build checks affected by the change.

## 6. Cross-Work-Unit compatibility requirements

The complete milestone must handle these cases:

| Scenario | Required behavior |
|---|---|
| Healthy Pi with all metrics | Health is deterministically `Healthy` |
| Pi 2 with undervoltage history | Snapshot succeeds; health shows historical power warning |
| Pi 5 with current power/throttle condition | Current condition is distinguished from history |
| Generic Linux host without `vcgencmd` | Snapshot succeeds; Raspberry Pi power state is unavailable |
| Optional metric command unavailable | Other metrics still populate |
| SSH timeout | Specific connection failure; health does not falsely report Healthy |
| Host key mismatch | Specific host-key failure; no automatic trust |
| Authentication failure | Specific authentication failure |
| Docker unavailable | Diagnostic Docker check fails/warns without declaring SSH offline |
| Tailscale absent | LAN/other SSH management can still be healthy |
| Read-only root filesystem detected | Critical health reason |
| Reboot required only | Informational reason; not automatically Warning |

## 7. Testing and validation strategy

### 7.1 During implementation

Use focused tests after each Work Unit.

Do not run the complete suite after every Work Unit unless repository policy requires it.

Do not remove, weaken, or skip valuable tests to obtain a passing result.

### 7.2 Required automated coverage

At minimum:

- metric parser/collector tests;
- throttling-bitmask tests;
- health-rule and threshold-boundary tests;
- diagnostic-classification tests;
- frontend rendering tests for representative states;
- regression coverage for existing monitoring behavior affected by the changes.

### 7.3 Milestone closure validation

Run the repository's canonical complete validation for the areas affected by M6, including the normal equivalents of:

- Rust formatting/static checks;
- Rust tests;
- frontend type checking;
- frontend tests;
- frontend production build;
- Tauri/backend build/check where normally required;
- AIQT/state validation where defined by the repository.

Use repository-defined commands rather than inventing alternatives.

### 7.4 Live-device validation

If the development environment has safe access to configured devices, perform read-only smoke validation against Pi 2 and Pi 5.

Do not restart, shut down, or mutate either device during M6.

Preferred live checks:

- extended snapshot collection;
- partial metric fallback;
- Pi 2 power/throttling output;
- Pi 5 power/throttling output;
- structured diagnostics;
- Docker/Tailscale diagnostic behavior.

If live-device access is unavailable, this does not justify unsafe workarounds or destructive testing. Record the missing manual validation explicitly as residual validation evidence.

## 8. Documentation and AIQT completion

At milestone closure:

- update this document only where actual implementation materially differs from the approved design;
- record material deviations and rationale;
- update affected functional/technical specifications;
- update AIQT Work Unit/milestone state according to repository conventions;
- preserve M7â€“M12 as planned/not started;
- do not mark unimplemented or unvalidated requirements complete.

## 9. Exit criteria

M6 implementation is complete when:

1. WU6-01 through WU6-05 are implemented.
2. Pi-Hub presents a deterministic `Healthy` / `Warning` / `Critical` / `Unknown` health state.
3. Health state includes stable reason codes/details.
4. Raspberry Pi undervoltage/throttling flags are interpreted rather than exposed only as raw values.
5. Current and historical Raspberry Pi conditions are distinguishable.
6. Structured connectivity diagnostics identify the failing stage without weakening host-key security.
7. Unsupported/partial metrics degrade gracefully.
8. Existing monitoring, terminal, Docker lifecycle, and connection behavior is not materially regressed.
9. Required automated tests pass.
10. Canonical milestone validation passes.
11. Documentation and AIQT state reflect the actual implementation.

Live Pi 2/Pi 5 smoke validation should be included in closure evidence when the environment provides safe access. If unavailable, the milestone may be implementation-complete with that manual evidence explicitly listed as pending; it must not be falsely reported as performed.

## 10. Non-goals carried forward

The completion of M6 must not introduce:

- persistent alert lifecycle;
- configurable thresholds;
- persistent activity log;
- historical charts;
- device restart/shutdown;
- device power-on;
- arbitrary remote administration commands.

Those remain assigned to later roadmap milestones.


