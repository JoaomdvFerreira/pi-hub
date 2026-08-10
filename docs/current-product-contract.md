# Current product and operational contract

This is the canonical current-behavior reference for Pi-Hub after M12. It complements the README; it does not replace approved requirements or historical milestone records.

## Boundaries and connectivity

- Pi-Hub uses the installed Windows OpenSSH client over Tailscale or LAN. Host-key verification is mandatory; changed keys require operator resolution.
- The application stores no SSH password, private-key contents, sudo credentials, or new service secrets.
- Frontend calls are typed. Pi-Hub provides no arbitrary shell, Docker, `systemctl`, or remote command interface.
- Remote operations have fixed, bounded backend behavior and isolated failures. Partial, unknown, unavailable, or unsupported data is not treated as healthy or zero.

## Current visibility and monitoring

- Device Health describes the current device condition and diagnostics; Service Health describes registered HTTP/HTTPS service checks.
- Docker visibility includes current operational information and bounded logs, but not Compose editing, arbitrary exec, create, or delete operations.
- Alerts are governed conditions with Active, Acknowledged, and Resolved states. Activity is a bounded local event/audit history, retained for 30 days and capped at 2,000 events.
- M11 network, storage, and system information is observational/read-only. It never configures network/storage, packages, accounts, or services.
- Historical Monitoring is local and separate from Activity and Alerts: samples are retained for 30 days, captured at most once every 60 seconds per entity/metric family, limited to 1h/24h/7d/30d queries and 500 points per series. Missing values remain absent and visible gaps are preserved.

## Administration

The only host-level actions are Restart Device, Shut Down Device, Restart Docker, and Restart Tailscale. They require deliberate confirmation and use dedicated typed backend operations with noninteractive privilege handling, bounded verification, expected-disruption tracking, and non-sensitive Activity entries.

Power On is unsupported. Pi-Hub does not support Wake-on-LAN, smart-plug, PoE, GPIO, hard-power actions, arbitrary service restarts, custom scripts, scheduling, batches, or remote package/OS administration.

## Documentation ownership

README: overview and setup. Roadmap: milestone history and sequencing. This document: current operational behavior. Requirements: durable normative contracts. Milestone records: approved scope and closure history. `AGENTS.md`: agent invariants. `.aiqt/`: structured workflow state. The PR template: PR governance.
