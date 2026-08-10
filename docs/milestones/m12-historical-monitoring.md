# Pi-Hub — M12 Historical Monitoring

Status: Approved and in implementation  
Depends on: M11 — Network, Storage, and System Visibility  
Next milestone: M13 — Document Hygiene

M12 adds local, bounded historical monitoring. It stores sampled device, service, and container metrics for recent trends without introducing a cloud telemetry service, remote metrics backend, arbitrary metrics, log history, exports, retention configuration, or M13 document hygiene.

## Product contract

- Historical data is local, contains no credentials, SSH output, or container logs, and is retained for 30 days.
- Sampling is at most once every 60 seconds per entity and metric family; a manual refresh updates current state but does not create history spam.
- Device samples include available CPU, memory, root filesystem, temperature, and Device Health state.
- Service samples include available response time and Service Health state; a failed check can have state without a response time.
- Container samples include available CPU and memory and use the stable M9 container identity.
- Missing/unsupported values remain absent. They are never stored or drawn as zero, and time gaps are retained.
- Queries support only 1h, 24h, 7d, and 30d. The backend caps each series at 500 points.
- Numeric responses downsample deterministically with timestamp, average, minimum, and maximum information. Categorical state responses preserve changes without unchanged-sample spam.
- History failures are isolated from current monitoring, Activity, Alerts, and M11 current-state visibility.

## Work units

1. Typed historical sample domain, migration/storage, reload, deterministic retention and corruption/failure handling.
2. Bounded sampling in the monitoring refresh path for device, service, and container metrics.
3. Typed bounded range-query API, entity isolation, aggregation, gaps, and state transitions.
4. Device, service, and container detail trend charts with range controls and explicit loading, empty, partial, and error states.
5. Integration, focused tests, AIQT evidence, canonical validation, and closure.

M13 remains planned/not started and is solely responsible for document hygiene.
