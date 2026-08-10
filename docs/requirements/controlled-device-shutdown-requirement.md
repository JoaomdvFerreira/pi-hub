# Pi-Hub â€” Controlled Device Shutdown Requirement

Version: 0.2  
Status: Approved requirement  
Target milestone: M10 â€” Controlled Device Administration  
Date: 2026-08-09  
Depends on: M7 activity history; M8 planned/alert-state semantics where applicable

## 1. Purpose and authority

This document is the requirement-level source of truth for controlled device shutdown in Pi-Hub.

It defines the user-visible behavior, safety boundaries, state semantics, privilege handling, and acceptance criteria that M10 must satisfy.

Implementation may adapt to the existing repository architecture, but it must not weaken the boundaries in this document.

## 2. Requirement

Pi-Hub shall allow the user to safely shut down a managed Linux device from the desktop application.

The operation is a controlled application feature, not arbitrary remote-command execution.

Power-on is explicitly excluded.

## 3. User story

As the Pi-Hub user,  
I want to shut down a managed Raspberry Pi from Pi-Hub,  
so that I can stop the device safely without opening the embedded terminal and issuing the command manually.

## 4. Functional flow

1. User opens device-level actions.
2. User selects `Shut Down Device`.
3. Pi-Hub displays an explicit destructive-action confirmation.
4. The confirmation states that:
   - services and containers will stop;
   - active SSH terminal sessions will disconnect;
   - Pi-Hub cannot power the device back on.
5. On confirmation, the frontend invokes a dedicated typed backend operation using the device identity only.
6. The backend verifies that the device is in a state where shutdown can be attempted.
7. The infrastructure layer executes a predefined supported shutdown operation over the existing verified SSH path.
8. The UI enters `Shutting down`.
9. Monitoring observes the expected transition to unreachable/offline.
10. If the command was accepted and the device becomes unreachable inside the bounded expected-offline window, the administrative action becomes `Completed`.
11. The resulting offline state is classified as planned rather than unexpected.

## 5. UI requirements

Recommended placement:

- Device Details action menu;
- optionally Dashboard device-card actions behind the identical confirmation flow.

Shutdown must not be a primary one-click action beside routine controls such as Refresh.

Recommended confirmation:

> Shut down {deviceName}?
>
> This will stop all services and containers and disconnect active SSH terminal sessions. Pi-Hub cannot turn this device back on.
>
> Cancel | Shut Down

Required presentation:

- destructive styling;
- device name visible;
- explicit `Shut Down` action label;
- no ambiguous `OK` confirmation;
- duplicate submission prevented while the action is active;
- action progress/status visible;
- failure state actionable and specific.

## 6. Backend contract and command boundary

The frontend shall invoke a dedicated typed operation conceptually equivalent to:

`shutdown_device(device_id)`

The frontend must not supply:

- shell fragments;
- command strings;
- executable names;
- sudo flags/arguments;
- command templates.

The backend/infrastructure layer selects the predefined supported Linux shutdown command according to the established architecture.

The shutdown feature must not create a general remote-execution API.

## 7. SSH and privilege handling

Existing SSH security remains mandatory:

- host-key verification is not bypassed;
- changed host keys are not trusted automatically;
- normal authentication semantics remain intact;
- remote execution is time-bounded.

The configured SSH user must be able to execute the shutdown operation non-interactively.

Supported outcome classes must include:

- permitted / command accepted;
- device unreachable;
- host-key failure;
- authentication failure;
- insufficient privileges;
- sudo requires interactive password;
- shutdown command unavailable/unsupported;
- command rejected;
- command timeout.

Pi-Hub must never request, capture, or store a sudo password to make shutdown work.

If shutdown requires interactive sudo authentication, the UI must report that the device is not configured for non-interactive Pi-Hub shutdown.

## 8. Administrative action model

The exact domain names may follow repository conventions, but the action must support the equivalent of:

- action ID;
- device ID;
- action type;
- requested timestamp;
- current state;
- completion timestamp;
- stable error code;
- bounded user-facing error detail.

Required lifecycle semantics:

- `requested`;
- `command_accepted`;
- `waiting_for_offline`;
- `completed`;
- `failed`;
- `timed_out`.

UI-only confirmation state may exist before `requested` but should not be persisted as a remote administrative action until the user confirms.

## 9. Planned-offline semantics

Intentional shutdown must not be indistinguishable from device failure.

Once the remote shutdown command is accepted, Pi-Hub must create a bounded expected-offline condition for that device.

During the expected-offline window:

- the device becoming unreachable may complete the shutdown action;
- the normal unexpected-device-offline notification is suppressed;
- the UI may display `Shut down`, `Offline â€” planned`, or an equivalent explicit planned state;
- monitoring of other devices continues normally.

If the device remains reachable after the bounded shutdown window:

- the action becomes `timed_out` or `failed`;
- the expected-offline condition is cleared;
- normal monitoring/notification semantics resume.

If the application restarts while an expected-offline marker exists, recovery behavior must be deterministic and must not leave an indefinite suppression state.

The marker must therefore be bounded by persisted expiry or equivalent safe recovery semantics.

## 10. Interaction with terminals and workloads

When shutdown is confirmed:

- existing terminal sessions for that device may disconnect naturally as the host shuts down;
- Pi-Hub does not need to terminate the remote shell before issuing the system shutdown unless the implementation requires it safely;
- the UI should not treat terminal disconnect caused by the confirmed shutdown as an independent unexpected error;
- Docker containers/services are allowed to stop through the operating system's normal shutdown sequence.

Pi-Hub must not hard-cut power as part of this requirement.

## 11. Availability rules

`Shut Down Device` should only be enabled when Pi-Hub has enough information to make a safe attempt.

At minimum:

- device exists and is configured;
- no shutdown action for that device is already in progress;
- device is not already known to be intentionally shut down/offline;
- the connection state does not already make remote execution impossible.

The implementation may determine final privilege capability only when the action is attempted; persistent sudo-capability configuration is not required.

## 12. Acceptance criteria

### User flow

- user can request shutdown from a device-level action surface;
- explicit destructive confirmation is always required;
- confirmation clearly states that Pi-Hub cannot turn the device back on;
- duplicate requests are prevented while shutdown is in progress.

### Security and architecture

- shutdown uses a dedicated typed backend operation;
- no arbitrary command text reaches the backend from the frontend;
- SSH host-key verification remains enabled;
- no SSH/sudo password is stored or requested;
- all remote execution is bounded by timeout.

### Result semantics

- accepted shutdown transitions into a bounded waiting-for-offline state;
- device going offline within that window completes the action;
- intentional successful shutdown does not create the normal unexpected-offline notification;
- failed or timed-out shutdown does not falsely mark the device as intentionally shut down;
- other devices continue monitoring normally;
- application restart cannot leave indefinite offline-notification suppression.

### Error behavior

Specific actionable errors exist for:

- host-key failure;
- authentication failure;
- insufficient privileges;
- sudo password required;
- unsupported/unavailable shutdown command;
- command timeout;
- device remaining online after accepted command.

### Activity

Once persistent activity history exists, the action records:

- request;
- success/completion;
- failure/timeout.

Sensitive command/authentication detail must not be written to the activity log.

### Automated validation

Tests cover at minimum:

- successful accepted shutdown followed by expected offline;
- SSH failure;
- host-key failure;
- authentication failure;
- privilege failure;
- sudo-password-required behavior;
- command timeout;
- device remaining reachable;
- planned-offline notification suppression;
- expiry/recovery of the expected-offline marker;
- duplicate-action prevention.

## 13. Manual validation

M10 closure should include a controlled live-device test only when the user explicitly authorizes disruptive validation.

The test must confirm:

- normal OS shutdown rather than hard power loss;
- Pi-Hub observes the planned offline transition;
- no unexpected-offline notification is emitted;
- the device remains powered off until manually powered on externally.

M10 must not shut down a real device merely because credentials are available; disruptive live validation requires explicit authorization.

## 14. Out of scope

This requirement does not include:

- Power On;
- Wake-on-LAN;
- smart-plug integration;
- managed PoE-port control;
- GPIO/external power controller;
- hard power cut;
- automatic recovery/power cycle;
- scheduled shutdown;
- batch shutdown of multiple devices.

These require a separate future product decision and are not implied by controlled shutdown support.


