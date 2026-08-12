# M14 Device Detail Tabs — UX & Query Contract

Status: Ready for implementation  
Milestone: M14

## Target information architecture

Device Detail has a persistent header and six tabs: Overview, Monitoring, Services, Containers, System, and Activity. The header always retains device identity/status, back/edit navigation, and existing primary controlled-administration actions including Restart and Shut Down.

Overview is concise: connection/overall health; current CPU, memory, temperature and uptime; small service/container/alert summaries only when existing snapshot data already makes them available. It does not duplicate tables or charts.

Monitoring owns Health Diagnostics, Historical Trends, and 1h/24h/7d/30d controls. Services owns current service-health presentation and links. Containers owns Docker visibility and existing M9 flows. System owns Network, Storage, and System/OS/hardware visibility. Activity owns persisted exact-device Activity.

## Query and state contract

- Inactive tabs do not initiate expensive view-specific work.
- Historical queries occur only while Monitoring is active. Persisted device Activity occurs only while Activity is active unless another visible surface has a documented need.
- Existing snapshot data may be reused under current freshness rules. Tabs never create a monitoring scheduler or duplicate refresh loop; background monitoring remains independent.
- Device/tab changes abort or ignore stale async results; A-scoped data cannot render under B. Exact device-ID Activity filtering and genuine empty states remain valid.
- Preferred route state is `/devices/:deviceId?tab=<tab>` with `overview` as default/fallback, but the implementation may select the simplest native router evolution rather than introduce a parallel routing system.

## Layout, accessibility, and tests

Use accessible tablist/tab/tabpanel semantics, selected state, visible focus, and component-consistent keyboard navigation. Tab controls may scroll/compact at constrained desktop widths; content must use width containment (`min-width: 0` or equivalent) so 7d/30d charts do not cause page horizontal overflow.

Tests cover default/explicit/invalid tab state, reachability of every existing section, query absence/activation for Monitoring and Activity, stale A→B safety, primary actions, Historical containment, and no duplicate settled request.
