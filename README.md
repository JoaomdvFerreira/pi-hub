# Pi-Hub

Pi-Hub is a Windows system-tray desktop application for observing and deliberately administering personal Linux infrastructure over Tailscale or LAN SSH. It is a focused control center for Raspberry Pi devices and compatible Linux servers, not a replacement for SSH, Portainer, or an infrastructure orchestration platform.

## Current product

M1-M12 are implemented. Pi-Hub provides:

- device registration, verified SSH connectivity, monitoring, diagnostics, and an Open Terminal shortcut;
- Device Health, Service Health, Docker visibility, alerts, bounded Activity, and desktop notifications;
- read-only network, storage, and system visibility;
- local, bounded historical trends for device, service, and container metrics; and
- four explicit device administration actions: Restart Device, Shut Down Device, Restart Docker, and Restart Tailscale.

The application uses a closed set of typed backend operations. SSH host-key verification remains mandatory; Pi-Hub stores no SSH password or private-key contents, exposes no arbitrary remote-command API, and does not support Power On. Remote collection and administration are bounded; unknown or unavailable data is never presented as healthy or zero.

## Monitoring concepts

| Concept | Meaning |
| --- | --- |
| Device Health | Current overall device condition and its reasons. |
| Service Health | Current HTTP/HTTPS check result for a registered service. |
| Alerts | Governed operational problems with Active, Acknowledged, and Resolved lifecycle. |
| Activity | Bounded local audit/event history of changes and actions. |
| Historical Monitoring | Bounded local metric trends; it is separate from Activity and Alerts. |

Historical Monitoring retains local samples for 30 days. Sampling is at most once per 60 seconds per entity/metric family; available values only are recorded. Queries are limited to 1h, 24h, 7d, or 30d and at most 500 points per series.

## Documentation

- [Roadmap and implementation history](docs/roadmap/post-mvp-product-roadmap.md)
- [Current product and operational contract](docs/current-product-contract.md)
- [Functional specification (historical MVP baseline)](docs/pi-hub-functional-specification.md)
- [Technical architecture specification (historical MVP baseline)](docs/pi-hub-technical-architecture-specification.md)
- [Milestone records](docs/milestones/)
- [Release procedure](docs/releasing.md)

The README owns the overview and setup; the current product contract owns durable operational/security behavior; the roadmap owns milestone history; milestone documents retain approved milestone scope and closure history.

## Stack

- Desktop shell: Tauri 2
- Frontend: React, TypeScript, Vite
- Backend: Rust with Tokio and Serde
- Remote access: Windows OpenSSH over Tailscale/LAN

## Prerequisites

- Node.js 18+
- Rust stable via `rustup`
- Windows MSVC build tools with the Desktop development with C++ workload
- Windows OpenSSH client and Windows Terminal
- Tailscale, when devices are managed through a tailnet

## Development

```bash
npm install
npm run tauri dev
```

## Build

```bash
npm run build
cargo build --manifest-path src-tauri/Cargo.toml
```

## Project management

AIQT is the canonical structured work graph in `.aiqt/`. Use `aiqt status` to inspect it and `aiqt next` only when beginning the selected work unit.
