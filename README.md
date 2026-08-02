# Pi-Hub

A lightweight Windows system-tray desktop app for monitoring and quickly accessing personal self-hosted infrastructure (Raspberry Pi devices, future Linux servers) over Tailscale/SSH. Pi-Hub does not replace SSH — it gives visibility and one-click shortcuts while leaving administration to the terminal.

See [`docs/pi-hub-functional-specification.md`](docs/pi-hub-functional-specification.md) and [`docs/pi-hub-technical-architecture-specification.md`](docs/pi-hub-technical-architecture-specification.md) for the full product and architecture specs, and [`docs/design/Pi Control.dc.html`](<docs/design/Pi Control.dc.html>) for the UI reference.

## Stack

- **Desktop shell:** Tauri 2
- **Frontend:** React + TypeScript + Vite
- **Backend:** Rust (Tokio, Serde)
- **Remote access:** Windows OpenSSH client over Tailscale/LAN

## Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain via `rustup`)
- Windows: MSVC build tools (Visual Studio Build Tools with the "Desktop development with C++" workload)
- Windows OpenSSH client and Windows Terminal (used for `Open Terminal`)
- Tailscale, installed and authenticated, if managing devices over a tailnet

## Development

```bash
npm install
npm run tauri dev
```

This launches the Tauri app with hot-reloading for the frontend.

## Build

```bash
npm run build
cargo build --manifest-path src-tauri/Cargo.toml
```

## Project layout

```text
src/                  React + TypeScript frontend
  app/                Application shell and routing
  components/         Shared UI (ui/) and layout (layout/) components
  features/           Feature modules: dashboard, devices, containers, services, settings
  lib/                Tauri command wrappers, formatting, validation
  stores/             Frontend state stores
  types/              Shared TypeScript types

src-tauri/            Rust backend
  src/commands/        Tauri command entry points
  src/application/     Use-case coordination
  src/domain/          Core models and rules
  src/infrastructure/  SSH execution, storage, parsers
  src/monitoring/      Scheduler, concurrency, notification decisions
  src/platform/        Windows-specific integrations (tray, terminal, notifications, autostart)

docs/                 Product spec, architecture spec, and UI design reference
```

## Project management

This project is planned and tracked with [AIQT](https://www.npmjs.com/package/aiqt) — see `.aiqt/` for the canonical work graph. Run `aiqt status` to check progress or `aiqt next` to fetch the next work unit.
