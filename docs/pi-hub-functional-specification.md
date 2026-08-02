# Pi-Hub — Functional Specification

**Version:** 0.1  
**Status:** Draft for implementation  
**Target platform:** Windows 11  
**Application type:** Desktop system-tray utility

## 1. Overview

### Project name

**Pi-Hub**

### Objective

Pi-Hub is a lightweight Windows desktop application that provides a single management interface for personal Linux devices connected through a private Tailscale network.

The application focuses on monitoring, quick administration, and rapid access to services running on Raspberry Pi devices and future home-lab servers.

Pi-Hub is not intended to replace SSH. It provides visibility and shortcuts while leaving advanced administration to the terminal.

## 2. Target user

Pi-Hub is designed for a single technical user managing personal infrastructure composed of:

- Raspberry Pi devices;
- mini PCs;
- Docker containers;
- self-hosted applications;
- Home Assistant;
- future Linux servers.

Multi-user support is outside the MVP scope.

## 3. Problem statement

Managing multiple Raspberry Pi devices currently requires the user to:

- open SSH sessions manually;
- remember hostnames or IP addresses;
- remember application URLs and ports;
- execute Docker commands manually;
- check device health individually.

Pi-Hub centralizes this information in a single desktop application.

## 4. Product goals

The application should immediately answer:

- Which devices are online?
- Is any device unhealthy?
- Which Docker containers are running?
- How busy is each device?
- How can the user quickly open a service or terminal?

## 5. Functional requirements

### 5.1 Device management

The user can register, edit, and remove devices.

Each device contains:

- friendly name;
- hostname;
- Tailscale hostname or IP;
- SSH username;
- SSH port;
- description;
- device type;
- refresh interval;
- monitoring enabled or disabled;
- notifications enabled or disabled;
- registered services.

Example:

```text
Name: Raspberry Pi 5
Hostname: raspberrypi5
Description: Application server
```

### 5.2 Connectivity monitoring

The application periodically verifies whether each device is reachable.

Supported states:

- Online;
- Offline;
- Authentication Error;
- Host Key Error;
- Timeout;
- Command Error;
- Unknown.

Each device status includes:

- last refresh time;
- last successful refresh;
- last failed refresh;
- current error, when applicable.

### 5.3 Device metrics

For every online device, Pi-Hub displays:

- CPU usage;
- memory usage;
- disk usage;
- temperature;
- uptime;
- load average;
- device model;
- operating system;
- kernel version;
- architecture.

Unavailable metrics should not cause the entire device refresh to fail.

### 5.4 Docker monitoring

Pi-Hub displays all Docker containers detected on the device.

For each container, show:

- name;
- image;
- state;
- status text;
- running time;
- health state;
- exposed ports.

Container states include:

- Running;
- Stopped;
- Restarting;
- Exited;
- Paused;
- Dead;
- Unknown.

Docker functionality is read-only in the MVP.

### 5.5 SSH terminal launcher

Each device provides an **Open Terminal** action.

Selecting it opens Windows Terminal directly into an SSH session.

Example:

```powershell
wt.exe ssh joao@raspberrypi5
```

Pi-Hub does not embed a terminal in the MVP.

### 5.6 Service launcher

Each device may expose one or more web services.

Examples:

- Home Assistant;
- Personal Finance application;
- Portainer;
- Frigate.

Each service contains:

- name;
- URL;
- optional icon;
- optional description;
- enabled or disabled state.

Selecting a service opens it in the default browser.

### 5.7 System tray

Pi-Hub primarily runs from the Windows notification area.

Closing the main window minimizes the application instead of terminating it.

The tray menu includes:

- Open Pi-Hub;
- Refresh All;
- device terminal shortcuts;
- device service shortcuts;
- Exit.

### 5.8 Automatic refresh

Device information refreshes automatically.

Default refresh interval:

```text
60 seconds
```

Manual refresh is also available.

Background refresh must not freeze the interface.

### 5.9 Notifications

Pi-Hub generates Windows notifications when:

- a device changes from online to offline;
- a device returns online;
- a previously running container stops unexpectedly;
- a container becomes unhealthy;
- authentication repeatedly fails;
- host-key validation fails.

Notifications can be disabled globally or per device.

Repeated notifications for an unchanged failure must be suppressed.

## 6. User interface

### 6.1 Dashboard

The dashboard displays every registered device as a card.

Each card includes:

- device name;
- online status;
- CPU usage;
- memory usage;
- disk usage;
- temperature;
- uptime;
- running container count;
- last refresh time.

Actions:

- Open Terminal;
- Open Services;
- Refresh;
- View Details.

### 6.2 Device details

The device detail view contains:

- general information;
- system metrics;
- Docker containers;
- registered services;
- recent activity;
- last-known errors.

Actions:

- Open Terminal;
- Refresh;
- Edit Device;
- Open Service.

### 6.3 Services

Services are displayed as clickable tiles.

Each tile includes:

- icon;
- name;
- URL or hostname;
- Open action.

### 6.4 Settings

Settings allow the user to configure:

- devices;
- services;
- notifications;
- global refresh interval;
- theme;
- launch on Windows startup;
- minimize-to-tray behavior.

## 7. Functional rules

- Pi-Hub must not store SSH passwords.
- SSH key authentication is preferred.
- Pi-Hub must not expose the Docker daemon over TCP.
- Remote data collection must use SSH.
- The user interface must not provide arbitrary command execution.
- A failed device must not block monitoring of other devices.
- A Docker failure must not mark the whole device as offline.
- Last-known data may remain visible but must be marked as stale.
- Every displayed snapshot must include a refresh timestamp.
- Host-key validation errors must require manual user resolution.

## 8. Non-functional requirements

Pi-Hub shall:

- start in under three seconds on the target Windows device;
- use minimal resources while idle;
- remain available in the system tray;
- keep the interface responsive during remote operations;
- support at least 1 to 10 devices in the MVP;
- isolate failures per device;
- use bounded background concurrency;
- operate without a remote Pi-Hub agent;
- persist configuration locally;
- recover safely from a corrupted runtime-state file.

## 9. Security requirements

- No SSH passwords stored locally.
- No private-key contents stored by Pi-Hub.
- SSH host-key checking must remain enabled.
- No arbitrary remote command interface.
- Docker daemon must not be exposed over unsecured TCP.
- Administrative actions must be explicitly implemented.
- Service URLs must be restricted to HTTP and HTTPS.
- Device hostnames, usernames, ports, and URLs must be validated.
- Pi-Hub should run as a normal Windows user.

## 10. Out of scope for the MVP

The MVP excludes:

- embedded terminal;
- remote file manager;
- Docker Compose editing;
- container creation or removal;
- container start, stop, or restart;
- remote operating-system updates;
- device reboot or shutdown;
- Tailscale administration;
- mobile application;
- web application;
- multi-user support;
- historical monitoring;
- long-term charts;
- plugin system;
- remote Pi-Hub agent;
- automatic application updater.

## 11. Future roadmap

### Phase 2 — Docker administration

- start container;
- stop container;
- restart container;
- view bounded logs;
- confirm destructive actions;
- record administrative actions.

### Phase 3 — Device administration

- reboot device;
- shut down device;
- check operating-system updates;
- monitor systemd services;
- display network statistics;
- display backup status.

### Phase 4 — Advanced monitoring

- historical metrics;
- configurable alert thresholds;
- charts;
- scheduled health reports;
- NAS support;
- mini-PC support;
- virtual-machine support;
- optional Tailscale API integration.

## 12. MVP acceptance criteria

The MVP is complete when:

1. Pi-Hub launches on Windows 11.
2. The application remains active in the system tray.
3. Devices can be added, edited, and removed.
4. Raspberry Pi 2 and Raspberry Pi 5 can be configured independently.
5. Device online and offline states are displayed correctly.
6. CPU, memory, disk, temperature, and uptime are displayed when available.
7. Docker containers are listed without exposing Docker over TCP.
8. SSH terminal sessions open in Windows Terminal.
9. Registered web services open in the default browser.
10. Automatic refresh operates without blocking the UI.
11. Device failure does not affect monitoring of other devices.
12. Device and container transition notifications are deduplicated.
13. No SSH password or private key is stored.
14. Host-key errors are not bypassed automatically.
15. The application contains no arbitrary remote-command interface.

## 13. Implementation milestones

### M1 — Desktop foundation

- Tauri desktop application;
- React and TypeScript frontend;
- system-tray integration;
- main-window lifecycle;
- local settings persistence;
- logging foundation;
- optional Windows autostart.

### M2 — Device registry and SSH connectivity

- device data model;
- add, edit, and delete flows;
- validation;
- Windows OpenSSH integration;
- connection testing;
- timeout and authentication classification;
- host-key error handling.

### M3 — Monitoring engine

- system metrics;
- Docker discovery;
- container parsing;
- device snapshots;
- scheduler;
- bounded concurrency;
- manual and automatic refresh;
- stale-data behavior;
- partial-failure handling.

### M4 — Desktop experience

- dashboard;
- device detail view;
- service launcher;
- Windows Terminal launcher;
- notifications;
- autostart preference;
- loading, empty, stale, and error states.

## 14. Product vision

Pi-Hub aims to become a lightweight control center for personal self-hosted infrastructure, providing immediate visibility into devices, containers, and services while preserving SSH as the primary administration interface.
