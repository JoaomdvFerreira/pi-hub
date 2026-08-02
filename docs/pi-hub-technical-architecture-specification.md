# Pi-Hub — Technical Architecture Specification

**Version:** 0.1  
**Status:** Draft for implementation  
**Target platform:** Windows 11 x64  
**Application type:** Desktop system-tray utility  
**Primary stack:** Tauri 2, React, TypeScript, and Rust

## 1. Purpose

This document defines the technical architecture for the Pi-Hub MVP.

Pi-Hub is a lightweight Windows desktop application for monitoring and accessing personal Linux devices connected through a private Tailscale network.

The MVP initially supports:

- Raspberry Pi 2 running Home Assistant in Docker;
- Raspberry Pi 5 running the Personal Finance application and related containers;
- future Linux devices with SSH and Docker support.

The application provides:

- device availability;
- system metrics;
- Docker container status;
- web-service shortcuts;
- SSH terminal launch;
- Windows notifications;
- system-tray operation.

Pi-Hub does not replace SSH and does not provide unrestricted remote administration.

## 2. Architectural goals

The architecture shall prioritize:

1. low desktop resource consumption;
2. minimal remote-device configuration;
3. no permanent Pi-Hub agent on managed devices;
4. secure communication through SSH;
5. compatibility with Tailscale hostnames and IPs;
6. clear separation between UI and privileged operations;
7. controlled command execution;
8. independent failure isolation per device;
9. simple local installation;
10. future extensibility beyond Raspberry Pi devices.

## 3. Architecture principles

### 3.1 Agentless remote management

The MVP shall not install a Pi-Hub-specific service or agent on managed devices.

Remote information shall be collected through standard Linux commands executed over SSH.

### 3.2 SSH as the remote transport

All remote system and Docker operations shall use SSH.

Pi-Hub shall not:

- expose the Docker daemon over an unauthenticated TCP port;
- communicate directly with `/var/run/docker.sock` over the network;
- install a generic command-execution API;
- store SSH passwords.

### 3.3 Tailscale as the private network layer

Pi-Hub may connect through:

- Tailscale MagicDNS names;
- full tailnet DNS names;
- Tailscale IP addresses;
- local DNS hostnames;
- local IP addresses.

Tailscale remains an external environmental dependency. Pi-Hub does not manage the tailnet in the MVP.

### 3.4 Explicit operations only

The Rust backend shall expose a closed set of application operations.

Examples:

```text
get_devices
create_device
update_device
delete_device
refresh_device
refresh_all_devices
open_device_terminal
open_device_service
get_app_settings
save_app_settings
```

The frontend shall not send arbitrary shell commands to the backend.

### 3.5 Local-first storage

Pi-Hub configuration and runtime state shall be stored locally on the Windows computer.

No cloud service or central Pi-Hub server is required.

## 4. System context

```text
┌─────────────────────────────────────────────────────┐
│ Windows 11 PC                                      │
│                                                     │
│  Pi-Hub                                             │
│  ├── React user interface                          │
│  ├── Tauri runtime                                 │
│  ├── Rust application services                     │
│  ├── Local configuration                            │
│  ├── Monitoring scheduler                           │
│  └── Windows notifications                          │
│                                                     │
│  Windows OpenSSH Client                            │
│  Windows Terminal                                  │
│  Tailscale Client                                  │
└──────────────────────┬──────────────────────────────┘
                       │
                       │ SSH over Tailscale or LAN
                       │
       ┌───────────────┴────────────────┐
       │                                │
┌──────▼──────────────┐        ┌────────▼─────────────┐
│ Raspberry Pi 2     │        │ Raspberry Pi 5      │
│                    │        │                     │
│ Linux              │        │ Linux               │
│ SSH server         │        │ SSH server          │
│ Docker             │        │ Docker              │
│ Home Assistant     │        │ Finance application │
└────────────────────┘        └─────────────────────┘
```

## 5. Technology stack

### 5.1 Desktop runtime

Use **Tauri 2** for:

- native desktop shell;
- Rust backend;
- application-window lifecycle;
- system-tray integration;
- native notifications;
- autostart integration;
- controlled frontend-to-backend invocation.

### 5.2 Frontend

- React;
- TypeScript;
- Vite;
- CSS variables and design tokens;
- optional Radix UI primitives;
- optional Lucide icons.

The frontend is responsible for:

- rendering device and container state;
- navigation;
- forms;
- loading and error presentation;
- invoking typed Tauri commands;
- subscribing to backend events.

The frontend shall not execute SSH or operating-system commands directly.

### 5.3 Backend

- Rust;
- Tauri command handlers;
- Tokio asynchronous runtime;
- Serde serialization;
- structured error types;
- controlled process execution.

The Rust backend is responsible for:

- configuration persistence;
- device validation;
- SSH execution;
- command timeouts;
- output parsing;
- monitoring scheduling;
- status-transition detection;
- notification dispatch;
- terminal and browser launching;
- application logging.

### 5.4 Remote dependencies

Managed devices require:

- Linux;
- reachable SSH server;
- supported shell utilities;
- Docker CLI for Docker monitoring;
- SSH user with permission to execute required commands;
- Tailscale or local-network connectivity.

No Pi-Hub package is installed remotely.

## 6. Logical component architecture

```text
React Frontend
│
├── Dashboard
├── Device Details
├── Device Configuration
├── Global Settings
├── Services
└── Status and Notification UI
        │
        │ Tauri invoke/events
        ▼
Rust Application Layer
│
├── Device Service
├── Monitoring Coordinator
├── Snapshot Service
├── Notification Service
├── Terminal Launcher
├── Service Launcher
└── Settings Service
        │
        ▼
Infrastructure Layer
│
├── SSH Adapter
├── Remote Command Catalogue
├── Metrics Parser
├── Docker Parser
├── Local Storage Adapter
├── Process Launcher
└── Clock and Scheduler
        │
        ▼
External Systems
├── Windows OpenSSH
├── Windows Terminal
├── Default Browser
├── Tailscale Network
└── Linux Devices
```

## 7. Frontend architecture

### 7.1 Application shell

Suggested routes:

```text
/
├── /devices
├── /devices/:deviceId
├── /devices/:deviceId/settings
└── /settings
```

A full browser router is optional. A lightweight internal routing mechanism is acceptable for the MVP.

### 7.2 State categories

#### Persistent configuration state

- devices;
- services;
- notification preferences;
- refresh intervals;
- theme;
- autostart preference.

#### Runtime monitoring state

- connectivity;
- system metrics;
- containers;
- refresh status;
- last error;
- last successful refresh.

#### Ephemeral UI state

- dialogs;
- active tab;
- expanded rows;
- form validation;
- toast messages.

### 7.3 State management

Recommended MVP approach:

- React context for global application state;
- reducer or small store for device snapshots;
- component-local state for forms and dialogs;
- event-driven updates from the Rust backend.

A library such as Zustand may be introduced if it materially simplifies the implementation.

### 7.4 Frontend/backend boundary

All Tauri commands shall have typed wrappers.

Example:

```typescript
type DeviceId = string;

interface DeviceSnapshot {
  deviceId: DeviceId;
  connectionStatus: DeviceConnectionStatus;
  capturedAt: string;
  metrics?: SystemMetrics;
  containers: DockerContainerSummary[];
  error?: DeviceError;
}
```

Command names and request/response types shall be centralized.

Suggested frontend structure:

```text
src/
├── app/
├── components/
│   ├── ui/
│   └── layout/
├── features/
│   ├── dashboard/
│   ├── devices/
│   ├── containers/
│   ├── services/
│   └── settings/
├── lib/
│   ├── tauri/
│   ├── formatting/
│   └── validation/
├── stores/
├── types/
└── main.tsx
```

## 8. Rust backend architecture

### 8.1 Layers

```text
src-tauri/src/
├── commands/
├── application/
├── domain/
├── infrastructure/
├── monitoring/
├── platform/
├── storage/
└── main.rs
```

#### Commands

Tauri command entry points.

Responsibilities:

- deserialize input;
- validate basic command arguments;
- call application services;
- convert internal errors into frontend-safe errors.

Commands contain minimal business logic.

#### Application

Coordinates use cases:

- register device;
- refresh device;
- refresh all devices;
- open terminal;
- process state transitions;
- save settings.

#### Domain

Contains core models and rules:

- device;
- service shortcut;
- snapshot;
- container state;
- connection status;
- notification rule.

#### Infrastructure

Contains technical implementations:

- SSH process execution;
- local-file persistence;
- output parsing;
- process launching.

#### Monitoring

Contains:

- scheduler;
- refresh coordinator;
- concurrency limits;
- snapshot comparison;
- notification decisions.

#### Platform

Contains Windows-specific integrations:

- Windows Terminal;
- default browser;
- system notifications;
- autostart;
- application directories.

### 8.2 Shared application state

Conceptual model:

```rust
pub struct AppState {
    pub device_repository: Arc<dyn DeviceRepository>,
    pub settings_repository: Arc<dyn SettingsRepository>,
    pub monitoring_service: Arc<MonitoringService>,
    pub ssh_executor: Arc<dyn RemoteExecutor>,
    pub notification_service: Arc<dyn NotificationService>,
}
```

Long-running network operations must not hold global locks.

## 9. Domain model

### 9.1 Device

```typescript
interface Device {
  id: string;
  name: string;
  host: string;
  sshPort: number;
  sshUsername: string;
  description?: string;
  deviceType: DeviceType;
  monitoringEnabled: boolean;
  refreshIntervalSeconds?: number;
  notificationsEnabled: boolean;
  services: DeviceService[];
  createdAt: string;
  updatedAt: string;
}
```

```typescript
type DeviceType =
  | "raspberry-pi"
  | "linux-server"
  | "mini-pc"
  | "nas"
  | "other";
```

Device type is descriptive. Monitoring behavior should remain capability-based.

### 9.2 Device service

```typescript
interface DeviceService {
  id: string;
  name: string;
  url: string;
  icon?: string;
  description?: string;
  enabled: boolean;
}
```

### 9.3 Device connection status

```typescript
type DeviceConnectionStatus =
  | "unknown"
  | "checking"
  | "online"
  | "offline"
  | "timeout"
  | "authentication_error"
  | "host_key_error"
  | "command_error";
```

### 9.4 System metrics

```typescript
interface SystemMetrics {
  hostname: string;
  model?: string;
  operatingSystem?: string;
  kernelVersion?: string;
  architecture?: string;
  uptimeSeconds: number;
  cpuUsagePercent?: number;
  loadAverage1m?: number;
  loadAverage5m?: number;
  loadAverage15m?: number;
  memoryTotalBytes: number;
  memoryUsedBytes: number;
  diskTotalBytes: number;
  diskUsedBytes: number;
  temperatureCelsius?: number;
}
```

### 9.5 Docker container summary

```typescript
interface DockerContainerSummary {
  id: string;
  name: string;
  image: string;
  state: DockerContainerState;
  statusText: string;
  health?: DockerHealthStatus;
  ports: DockerPortBinding[];
  createdAt?: string;
  startedAt?: string;
}
```

```typescript
type DockerContainerState =
  | "running"
  | "stopped"
  | "exited"
  | "restarting"
  | "paused"
  | "dead"
  | "unknown";
```

```typescript
type DockerHealthStatus =
  | "healthy"
  | "unhealthy"
  | "starting"
  | "none"
  | "unknown";
```

### 9.6 Device snapshot

```typescript
interface DeviceSnapshot {
  deviceId: string;
  connectionStatus: DeviceConnectionStatus;
  capturedAt: string;
  durationMs: number;
  metrics?: SystemMetrics;
  dockerAvailable: boolean;
  containers: DockerContainerSummary[];
  warnings: SnapshotWarning[];
  error?: DeviceError;
}
```

A snapshot represents one monitoring cycle.

## 10. Local persistence

### 10.1 Storage model

The MVP shall use versioned local JSON files.

Suggested files:

```text
Pi-Hub/
├── config.json
├── devices.json
├── state.json
└── logs/
    └── pihub.log
```

Files shall be stored in the application-specific data directory.

### 10.2 Configuration separation

#### `config.json`

```json
{
  "schemaVersion": 1,
  "refreshIntervalSeconds": 60,
  "startWithWindows": false,
  "minimizeToTray": true,
  "notificationsEnabled": true,
  "theme": "dark"
}
```

#### `devices.json`

```json
{
  "schemaVersion": 1,
  "devices": [
    {
      "id": "pi2",
      "name": "Home Assistant",
      "host": "raspberrypi",
      "sshPort": 22,
      "sshUsername": "joao",
      "deviceType": "raspberry-pi",
      "monitoringEnabled": true,
      "notificationsEnabled": true,
      "services": [
        {
          "id": "home-assistant",
          "name": "Home Assistant",
          "url": "http://raspberrypi:8123",
          "enabled": true
        }
      ]
    }
  ]
}
```

#### `state.json`

Contains non-authoritative runtime state:

- last-known snapshots;
- last notification states;
- application-window preferences.

Pi-Hub must remain functional if `state.json` is deleted.

### 10.3 Atomic writes

Configuration updates shall:

1. serialize to a temporary file;
2. flush the data;
3. replace the original file;
4. preserve the previous valid file on failure.

### 10.4 Schema versioning

Every persistent document must contain `schemaVersion`.

Future migrations must run before data is exposed to the application.

### 10.5 Future SQLite migration

SQLite may replace JSON when Pi-Hub requires:

- historical metrics;
- audit history;
- large notification history;
- indexed search;
- transactional operations.

SQLite is not required for the MVP.

## 11. SSH architecture

### 11.1 Implementation strategy

Use the Windows OpenSSH client through controlled process execution.

Example:

```powershell
ssh -o BatchMode=yes -o ConnectTimeout=5 joao@raspberrypi5 "<approved-command>"
```

This approach:

- reuses existing SSH keys;
- reuses `ssh-agent`;
- preserves standard host-key verification;
- avoids embedding SSH credential handling in Pi-Hub.

### 11.2 SSH assumptions

Pi-Hub assumes:

- the host key has been accepted;
- the SSH user exists;
- public-key authentication is configured;
- any passphrase is available through `ssh-agent`;
- background monitoring must not prompt for passwords.

Use:

```text
BatchMode=yes
```

for all background monitoring.

### 11.3 Host-key errors

Pi-Hub shall never silently accept or remove a host key.

Host-key failures must be reported with remediation guidance.

Pi-Hub shall not automatically run:

```powershell
ssh-keygen -R <host>
```

### 11.4 Remote command catalogue

Remote commands are defined in backend code.

Conceptual model:

```rust
pub enum RemoteOperation {
    Probe,
    SystemIdentity,
    SystemMetrics,
    DockerContainers,
}
```

The frontend cannot supply arbitrary shell commands.

### 11.5 Connectivity probe

Use an SSH-based probe:

```bash
printf 'PIHUB_OK'
```

This verifies:

- DNS resolution;
- network connectivity;
- SSH availability;
- host-key acceptance;
- authentication.

### 11.6 Remote execution result

```rust
pub struct RemoteExecutionResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub timed_out: bool,
}
```

Every remote execution shall have a hard timeout.

## 12. Remote metrics collection

### 12.1 Collection strategy

Collect system data in one SSH session where practical.

Prefer a stable key-value payload over parsing human-readable command output.

Example:

```text
PIHUB_HOSTNAME=raspberrypi5
PIHUB_UPTIME_SECONDS=84922
PIHUB_MEMORY_TOTAL_BYTES=8589934592
PIHUB_MEMORY_AVAILABLE_BYTES=6432382976
PIHUB_DISK_TOTAL_BYTES=250000000000
PIHUB_DISK_AVAILABLE_BYTES=183000000000
PIHUB_TEMP_MILLIC=45200
```

The parser shall:

- ignore unknown fields;
- reject invalid numeric values;
- preserve valid fields;
- return partial warnings;
- never treat remote output as executable content.

### 12.2 CPU usage

Preferred MVP approach:

- read `/proc/stat`;
- wait for a bounded interval;
- read `/proc/stat` again;
- calculate CPU usage from the delta.

Do not parse interactive tools such as `top`.

### 12.3 Temperature

Primary source:

```text
/sys/class/thermal/thermal_zone0/temp
```

Temperature remains optional.

### 12.4 Memory

Read from:

```text
/proc/meminfo
```

Derive used memory from total and available values.

### 12.5 Disk

Monitor the root filesystem in the MVP:

```text
/
```

Additional mount points may be added later.

## 13. Docker monitoring

### 13.1 Docker discovery

Check whether Docker is available:

```bash
command -v docker
```

If Docker is unavailable:

- device remains online;
- system metrics remain valid;
- `dockerAvailable` is false;
- the UI displays Docker as unavailable.

### 13.2 Container collection

Recommended command:

```bash
docker ps -a --no-trunc --format '{{json .}}'
```

Each line is parsed independently.

Malformed lines create warnings without discarding valid entries.

### 13.3 Permissions

The SSH user must be permitted to execute Docker commands.

Pi-Hub does not automatically alter:

- Docker group membership;
- sudoers;
- daemon configuration.

### 13.4 MVP Docker scope

Allowed:

```text
List containers
```

Excluded:

```text
Start
Stop
Restart
Remove
Pull
Create
Exec
Full logs
Edit Compose
```

## 14. Monitoring engine

### 14.1 Scheduler responsibilities

The Rust scheduler shall:

- determine which devices require refresh;
- schedule refreshes;
- limit concurrent SSH sessions;
- ensure one active refresh per device;
- emit snapshot events;
- compare current and previous states;
- trigger deduplicated notifications.

### 14.2 Refresh intervals

Global default:

```text
60 seconds
```

Recommended validation range:

```text
Minimum: 15 seconds
Maximum: 3,600 seconds
```

### 14.3 Concurrency

Recommended initial limit:

```text
Maximum concurrent device refreshes: 4
```

If a scheduled refresh occurs while the same device is still running, the second refresh is skipped or coalesced.

### 14.4 Refresh lifecycle

```text
Scheduled refresh
      │
      ▼
Check device enabled
      │
      ▼
Mark as checking
      │
      ▼
Run SSH probe
      │
      ├── Failure ──► classify error ──► save snapshot
      │
      ▼
Collect system metrics
      │
      ▼
Collect Docker data
      │
      ▼
Parse partial results
      │
      ▼
Create immutable snapshot
      │
      ▼
Compare with previous snapshot
      │
      ▼
Emit frontend event
      │
      ▼
Evaluate notifications
```

### 14.5 Failure isolation

A failed refresh must not:

- stop the scheduler;
- cancel other devices;
- block the UI;
- overwrite valid configuration;
- erase the previous successful metrics.

### 14.6 Backend events

Suggested events:

```text
device://snapshot-updated
device://status-changed
container://status-changed
monitoring://refresh-started
monitoring://refresh-completed
```

## 15. Status transition rules

### 15.1 Device transitions

Notify on:

```text
online → offline
offline → online
online → authentication_error
online → host_key_error
```

Do not notify repeatedly when the state remains unchanged.

### 15.2 Container transitions

Notify when a previously observed running container changes to:

```text
exited
stopped
restarting
unhealthy
```

Do not notify for containers already stopped when first discovered.

### 15.3 Deduplication

Suggested key:

```text
deviceId + resourceId + previousState + currentState
```

Persist enough state to avoid duplicate notifications after restart.

## 16. System tray architecture

### 16.1 Lifecycle

Pi-Hub shall:

- create the tray icon during startup;
- remain active when the main window closes;
- restore or focus the main window from the tray;
- terminate only through explicit Exit or operating-system shutdown.

### 16.2 Tray menu

Initial menu:

```text
Open Pi-Hub
Refresh All
────────────
Raspberry Pi 2
  Open Terminal
  Open Home Assistant
Raspberry Pi 5
  Open Terminal
  Open Finance
────────────
Exit
```

### 16.3 Tray status

MVP:

- static application icon;
- concise status text in the menu.

Dynamic tray icon variants are future scope.

## 17. Terminal launching

### 17.1 Windows Terminal

Default:

```powershell
wt.exe ssh <username>@<host>
```

Custom port:

```powershell
wt.exe ssh -p <port> <username>@<host>
```

### 17.2 Argument handling

Arguments must be passed separately to the process API.

Do not construct one interpolated command string.

### 17.3 Validation

Validate:

- DNS hostname;
- IPv4;
- IPv6;
- MagicDNS or tailnet hostname;
- SSH port;
- Linux username.

Reject shell metacharacters.

## 18. Service launching

Parse service URLs using a URL parser.

Allowed schemes:

```text
http
https
```

Disallowed:

```text
file
javascript
data
shell
powershell
```

Open valid URLs in the default Windows browser.

## 19. Notifications

### 19.1 Sources

Notifications may originate from:

- device status transitions;
- Docker container transitions;
- repeated authentication failures;
- host-key failures;
- application-level configuration errors.

### 19.2 Controls

Support:

- global enable or disable;
- per-device enable or disable.

Per-event-type controls are future scope.

## 20. Autostart

Pi-Hub may launch when the user signs into Windows.

Default:

```text
Autostart disabled until explicitly enabled
```

Use the official Tauri autostart integration rather than custom startup-folder manipulation.

## 21. Security architecture

### 21.1 Credential handling

Pi-Hub shall not:

- store SSH passwords;
- store private-key contents;
- request private keys through the UI;
- copy keys automatically;
- disable host-key checking.

Pi-Hub may rely on:

- Windows OpenSSH configuration;
- standard private-key locations;
- `ssh-agent`;
- user-configured SSH host aliases.

### 21.2 Privilege model

Pi-Hub runs as a normal Windows user.

Administrative privileges are not required for normal MVP operation.

Remote commands run with the configured SSH user's permissions.

### 21.3 Input validation

Strictly validate:

- host;
- SSH port;
- username;
- service URL;
- refresh interval;
- device ID;
- service ID.

Display names and descriptions may allow broader Unicode text.

### 21.4 Command injection prevention

Requirements:

- commands predefined in Rust;
- process arguments passed separately;
- no generic shell execution from the frontend;
- no user-defined remote command templates;
- no direct frontend access to broad shell plugins.

### 21.5 Tauri permissions

Use least-privilege capabilities.

Expose only commands and plugins required for:

- application operations;
- tray behavior;
- notifications;
- controlled autostart;
- safe browser opening.

### 21.6 Local data sensitivity

Local configuration may expose:

- device names;
- hostnames;
- usernames;
- service URLs;
- infrastructure structure.

Logs must not contain:

- private keys;
- passwords;
- environment dumps;
- arbitrary remote file contents.

## 22. Error handling

### 22.1 Error taxonomy

```text
ConfigurationError
ValidationError
DnsResolutionError
ConnectionTimeout
ConnectionRefused
AuthenticationError
HostKeyError
RemoteCommandError
RemoteCommandTimeout
ParseError
DockerUnavailable
DockerPermissionError
StorageError
PlatformIntegrationError
```

### 22.2 Frontend-safe error model

```typescript
interface ApplicationError {
  code: string;
  message: string;
  remediation?: string;
  retryable: boolean;
}
```

### 22.3 Partial success

Example:

```text
Device: Online
System metrics: Available
Docker: Permission error
```

A Docker failure must not mark the whole device offline.

## 23. Logging

### 23.1 Local logs

Use rotating local logs.

Levels:

- error;
- warn;
- info;
- debug.

Default production level:

```text
info
```

### 23.2 Events to log

- application startup and shutdown;
- configuration migration;
- monitoring-cycle summaries;
- device refresh failures;
- parser warnings;
- notification dispatch;
- terminal launch failures;
- storage failures.

### 23.3 Redaction

Do not log:

- authentication material;
- private-key contents;
- complete SSH configuration;
- complete environment variables;
- arbitrary remote content.

## 24. Performance requirements

### 24.1 Startup

Target:

```text
Usable main window within 3 seconds
```

Monitoring may continue after the UI becomes available.

### 24.2 Idle behavior

Pi-Hub should:

- avoid continuous polling;
- sleep between refresh intervals;
- avoid unnecessary rerenders;
- keep only required background tasks active.

### 24.3 Timeouts

Recommended defaults:

```text
SSH connection timeout: 5 seconds
Remote metrics timeout: 10 seconds
Docker collection timeout: 10 seconds
Complete device refresh timeout: 20 seconds
```

### 24.4 Scale target

MVP design target:

```text
1–10 devices
```

## 25. Testing strategy

### 25.1 Rust unit tests

Test:

- domain validation;
- state transitions;
- notification rules;
- metrics parsing;
- Docker parsing;
- configuration migrations;
- error classification;
- command construction;
- URL validation.

### 25.2 Frontend unit tests

Test:

- device-card rendering;
- metric formatting;
- status indicators;
- loading states;
- partial errors;
- forms;
- notification settings;
- stale-data presentation.

### 25.3 Integration tests

Use a fake `RemoteExecutor`.

Scenarios:

- online device;
- offline device;
- timeout;
- authentication failure;
- host-key failure;
- malformed metrics;
- Docker unavailable;
- Docker permission failure;
- running container becomes exited.

### 25.4 Manual tests

Validate against:

- Raspberry Pi 2;
- Raspberry Pi 5;
- local network;
- Tailscale network;
- disconnected Tailscale;
- powered-off device;
- invalid username;
- unknown host key;
- changed host key;
- Docker permission denied;
- Home Assistant service URL;
- Personal Finance service URL.

### 25.5 End-to-end tests

Focus on:

- first launch;
- adding a device;
- editing a device;
- deleting a device;
- dashboard refresh;
- opening device details;
- opening a service;
- changing settings;
- minimize-to-tray behavior.

## 26. Build and distribution

### 26.1 Target

```text
Windows 11 x64
```

### 26.2 Installer

Preferred:

- MSI; or
- NSIS installer.

Include:

- Pi-Hub executable;
- Tauri runtime assets;
- application icon;
- uninstall support.

### 26.3 External prerequisites

- Tailscale installed and authenticated;
- Windows OpenSSH client;
- Windows Terminal recommended;
- SSH keys configured;
- target devices reachable.

Pi-Hub should detect missing prerequisites and provide diagnostics.

### 26.4 Updates

MVP:

- manual releases;
- version displayed in About;
- no automatic updater.

## 27. Suggested repository structure

```text
pi-hub/
├── src/
│   ├── app/
│   ├── components/
│   │   ├── ui/
│   │   └── layout/
│   ├── features/
│   │   ├── dashboard/
│   │   ├── devices/
│   │   ├── containers/
│   │   ├── services/
│   │   ├── notifications/
│   │   └── settings/
│   ├── lib/
│   │   ├── tauri/
│   │   ├── formatting/
│   │   └── validation/
│   ├── stores/
│   ├── types/
│   └── main.tsx
│
├── src-tauri/
│   ├── capabilities/
│   ├── icons/
│   ├── src/
│   │   ├── commands/
│   │   ├── application/
│   │   ├── domain/
│   │   ├── infrastructure/
│   │   │   ├── ssh/
│   │   │   ├── parsers/
│   │   │   └── storage/
│   │   ├── monitoring/
│   │   ├── platform/
│   │   ├── error.rs
│   │   ├── state.rs
│   │   └── main.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── tests/
├── docs/
│   ├── functional-specification.md
│   ├── technical-architecture.md
│   └── ssh-setup.md
├── package.json
├── tsconfig.json
└── README.md
```

## 28. Initial Tauri command boundary

```text
get_app_settings
save_app_settings
get_devices
get_device
create_device
update_device
delete_device
test_device_connection
refresh_device
refresh_all_devices
get_latest_snapshot
open_device_terminal
open_device_service
set_autostart
get_autostart_status
```

Command inputs and responses shall be updated atomically within the same application release.

## 29. Implementation milestones

### M1 — Application foundation

Scope:

- Tauri 2 shell;
- React and TypeScript frontend;
- design-system foundations;
- main window;
- system tray;
- close and minimize behavior;
- settings storage;
- logging.

Acceptance criteria:

- application starts on Windows;
- main window opens;
- tray icon exists;
- closing the window keeps Pi-Hub running;
- Exit terminates the application;
- settings survive restart.

### M2 — Device registry and SSH connectivity

Scope:

- device domain model;
- device repository;
- add, edit, and delete device;
- validation;
- SSH adapter;
- connectivity test;
- timeout and authentication classification;
- host-key error handling.

Acceptance criteria:

- Pi 2 and Pi 5 can be registered;
- connection can be tested;
- authentication and host-key failures are distinguishable;
- passwords are never requested or stored;
- configuration survives restart.

### M3 — Monitoring engine

Scope:

- system metrics;
- metrics parser;
- Docker discovery;
- Docker parser;
- snapshots;
- scheduler;
- bounded concurrency;
- manual refresh;
- automatic refresh;
- partial-failure handling.

Acceptance criteria:

- CPU, memory, disk, temperature, and uptime are displayed;
- containers are listed;
- one unavailable device does not affect others;
- refresh does not freeze the UI;
- stale data is clearly identified.

### M4 — Desktop experience

Scope:

- final dashboard;
- device detail view;
- service shortcuts;
- Windows Terminal launch;
- browser launch;
- notifications;
- autostart preference;
- empty, loading, stale, and error states.

Acceptance criteria:

- system health is visible from the dashboard;
- SSH terminals open for both Raspberry Pis;
- Home Assistant and Personal Finance open from Pi-Hub;
- notifications are deduplicated;
- Pi-Hub can optionally start with Windows.

## 30. Architectural decisions

### ADR-001 — Tauri instead of Electron

**Decision:** Use Tauri 2 with React, TypeScript, and Rust.

**Rationale:**

- appropriate for a small resident tray application;
- native Rust backend for controlled process execution;
- tray and autostart support;
- avoids bundling a complete Node.js desktop runtime.

### ADR-002 — Agentless remote monitoring

**Decision:** Do not install a Pi-Hub agent on managed devices.

**Rationale:**

- lower setup and maintenance cost;
- standard SSH commands are sufficient for the MVP;
- easier onboarding of additional Linux devices.

### ADR-003 — Windows OpenSSH adapter

**Decision:** Use the installed Windows OpenSSH client instead of embedding an SSH library.

**Rationale:**

- reuses existing keys and host verification;
- reuses `ssh-agent`;
- aligns manual and application behavior;
- reduces credential-management responsibility.

### ADR-004 — JSON persistence for MVP

**Decision:** Use versioned local JSON files.

**Rationale:**

- small data volume;
- easy backup and inspection;
- no historical telemetry in MVP;
- avoids premature database complexity.

### ADR-005 — Read-only Docker monitoring

**Decision:** Only read Docker status in the MVP.

**Rationale:**

- reduces destructive-action risk;
- simplifies authorization;
- validates the monitoring value before administration is added.

### ADR-006 — Predefined remote command catalogue

**Decision:** Implement remote operations as fixed backend commands.

**Rationale:**

- prevents arbitrary command execution;
- improves parser reliability;
- supports unit and integration tests;
- creates an explicit security boundary.

## 31. Risks and mitigations

| Risk | Impact | Mitigation |
|---|---:|---|
| SSH key not configured | High | Connection diagnostics and setup documentation |
| Host key changes after reinstall | High | Report host-key error and require manual verification |
| Different Linux command output | Medium | Prefer `/proc`, `/sys`, and structured Docker output |
| SSH user lacks Docker permission | Medium | Partial success and clear remediation |
| Tailscale disconnected | Medium | Distinguish network failure from authentication failure |
| Remote refresh hangs | High | Hard timeouts and process termination |
| Concurrent refresh overload | Medium | Bounded concurrency and one refresh per device |
| Configuration corruption | High | Atomic writes, backups, and schema validation |
| Command injection | Critical | Fixed commands, strict validation, and separate arguments |
| Notification noise | Medium | Transition-based deduplication |
| Raspberry Pi 2 resource limits | Low | Short commands, conservative polling, and no agent |

## 32. MVP definition of done

The technical MVP is complete when:

1. Pi-Hub installs and runs on Windows 11.
2. Pi-Hub operates from the system tray.
3. Pi 2 and Pi 5 can be configured independently.
4. SSH uses existing keys and host verification.
5. Device metrics are collected through SSH.
6. Docker containers are listed without exposing Docker over TCP.
7. Device failures are isolated.
8. Monitoring runs automatically at a bounded interval.
9. Terminal sessions open in Windows Terminal.
10. Registered web services open in the default browser.
11. Device and container transitions produce controlled notifications.
12. Configuration uses atomic, versioned persistence.
13. No SSH password or private key is stored.
14. No arbitrary remote-command interface exists.
15. Automated tests cover parsers, validation, transitions, and command construction.

## 33. Future architectural extensions

Potential future additions:

- explicit container start, stop, and restart;
- bounded container log retrieval;
- systemd service monitoring;
- device reboot and shutdown;
- backup monitoring;
- historical metrics in SQLite;
- charts and alert thresholds;
- NAS and mini-PC support;
- application update mechanism;
- signed installers;
- Tailscale API integration;
- optional remote agent for advanced telemetry;
- adapter model for non-Linux devices.

Future extensions must preserve the rule that privileged actions are explicit, bounded, and auditable.

## 34. Final architecture summary

```text
Tauri 2 desktop application
        │
        ├── React and TypeScript interface
        ├── Rust application services
        ├── Local versioned JSON configuration
        ├── Background monitoring scheduler
        ├── Native tray, notifications, and autostart
        │
        ▼
Windows OpenSSH client
        │
        ▼
SSH over Tailscale or LAN
        │
        ├── Raspberry Pi 2
        │   └── Home Assistant Docker container
        │
        └── Raspberry Pi 5
            └── Personal Finance containers
```

The architecture deliberately avoids:

- a Pi-Hub cloud service;
- remote agents;
- stored SSH passwords;
- unrestricted command execution;
- Docker daemon TCP exposure;
- premature infrastructure complexity.
