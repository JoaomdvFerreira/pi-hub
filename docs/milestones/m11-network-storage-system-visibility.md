# Pi-Hub — M11 Network, Storage, and System Visibility

Version: 0.1
Status: Implemented
Date: 2026-08-10
Depends on: M10 — Controlled Device Administration completed and merged
Next milestone: M12 — Historical Monitoring

## Objective

Expand Pi-Hub's read-only device visibility so an operator can understand the current network, storage, and operating-system state of a managed device without opening a terminal.

M11 is observational only. It must not introduce network configuration, storage modification, package management, or arbitrary remote commands.

## Scope and decisions

M11 provides typed network interfaces, IPv4/IPv6 addresses, link/MAC state, default route/gateway, and DNS resolvers; typed mounted-filesystem capacity, usage, filesystem type, source, and read-only state; and hostname, OS, kernel, architecture, hardware/model, CPU/core, memory, and boot/uptime context.

All collection uses predefined, bounded, read-only SSH operations. Unknown, unsupported, and permission-denied data remain distinct from valid empty or zero values. The frontend supplies no shell commands.

Loopback may be collected but must not dominate the UI. No interface state implies internet connectivity. Pseudo and runtime filesystems (`proc`, `sysfs`, `tmpfs`, `devtmpfs`, `overlay`, `squashfs`, and similar) must not flood primary storage. Root `/` remains identifiable and its read-only state remains consistent with M6.

Stable system data may be reused within the refresh model, but M11 adds no historical time series.

## Out of scope

- Network, route, DNS, Wi-Fi, Ethernet, firewall, or Tailscale configuration.
- Packet/traffic inspection, network or port scanning, or arbitrary socket/process inspection.
- Mounting, unmounting, formatting, partitioning, SMART remediation, repair, or filesystem writes.
- Package/OS updates, account administration, arbitrary shell execution, M12 historical monitoring, or M13 Document Hygiene.

## Work Units

1. WU11-01 — Network visibility: interfaces, addresses, default route/gateway, and DNS with partial parser failure isolation.
2. WU11-02 — Storage visibility: mounted filesystem capacity and read-only state, filtering pseudo/runtime mounts.
3. WU11-03 — System visibility: OS/hardware identity across Raspberry Pi and generic Linux devices.
4. WU11-04 — Device Details integration: concise Network, Storage, and System summaries with explicit unavailable states.
5. WU11-05 — Integration and closure: bounded independent refresh domains, documentation, AIQT, and milestone validation.

## Required scenarios

- Ethernet, Tailscale, and loopback are independent interfaces; IPv4 and IPv6 coexist.
- Missing default route or DNS is displayed as unavailable, never fabricated.
- Read-only root and external USB storage are represented; Docker overlays do not flood the primary view.
- Generic Linux hosts without Raspberry Pi metadata retain valid system context.
- Failure of one M11 domain leaves the other M11 domains and M6–M10 data available.
- A restart recollects current data; no M11 historical series is persisted.

## Exit criteria

M11 completed with all five Work Units complete: Device Details shows the three domains clearly; unknown/partial data is not made healthy or zero; collection is bounded and isolated; no modification or arbitrary SSH API exists; and M6-M10 behavior remains intact. At M11 closure, M12-M13 were planned; later status is maintained by the roadmap.

## Live validation

Read-only validation against reachable managed devices is permitted and recommended: inspect interfaces, routes/DNS, mounts, and system identity. Do not modify network, storage, firewall, mounts, packages, services, or devices to manufacture validation scenarios. If unavailable, record this as residual manual validation.

## Historical status note

M11 is complete. Statements above about M12 or M13 being planned describe the M11-era plan; current milestone status is maintained in the [roadmap](../roadmap/post-mvp-product-roadmap.md).
