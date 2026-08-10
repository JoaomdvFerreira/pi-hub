# Pi-Hub — M13 Document Hygiene

Status: Implemented
Depends on: M12 — Historical Monitoring completed and merged  
Roadmap position: Final planned milestone

M13 reconciles repository documentation with the product implemented in M1–M12. It is a documentation-governance milestone only: no product functionality, policy changes, broad refactoring, release publication, or new milestone is in scope.

## Governing rules

- Implementation is the source of truth for current behavior. A suspected conflict with an approved requirement is reported rather than silently rewritten.
- Current guidance has one canonical owner where practical: the README for overview/setup, roadmap for milestone history, milestone documents for approved scope and closure, requirements for durable contracts, `AGENTS.md` for repository invariants, the milestone skill for execution workflow, and the PR template for review governance.
- Completed milestone documents remain historical records. They may receive link, status, supersession, or restructuring corrections, but are not rewritten to imply later functionality existed earlier.
- Current behavior, deferred work, unsupported behavior, and speculative ideas are kept distinct.
- Security and operational documentation preserves the established boundaries: verified SSH host keys; no stored passwords, private keys, or service secrets; no arbitrary remote command API; bounded typed remote operations; deliberate approved administration only; unsupported Power On; and unknown/unavailable data is neither Healthy nor zero.

## Work units

1. Inventory and truth-audit the meaningful documentation locations and identify canonical ownership or genuine conflicts.
2. Reconcile README, roadmap, milestone status, setup, architecture/product overview, and primary navigation with M1–M12.
3. Reconcile technical, security, operational, and developer guidance, including connectivity, health, Activity, Alerts, Docker, administration, M11 visibility, M12 history, retention, and AIQT references.
4. Consolidate duplicated guidance; archive, supersede, or remove stale material as appropriate; normalize terminology and internal navigation without a cosmetic mass rewrite.
5. Run documentation validation, record bounded residual debt where necessary, reconcile AIQT evidence, and close M13.

## Completion criteria

M13 is complete when the primary product overview, roadmap/status, technical/security/operational documentation, terminology, and navigation accurately represent M1–M12; historical records remain truthful; no genuine implementation-versus-requirement conflict is hidden; repository governance documents agree; validation passes; residual debt is explicit; and no unrelated product functionality or new milestone is introduced.

## Closure and residual documentation debt

M13 is complete. The README, roadmap, current product contract, completed milestone status, historical baseline labels, and Markdown navigation were reconciled against the implemented M1-M12 product. No product code changed.

The only bounded residual documentation debt is AIQT's retained historical duplicate planning entries for M10-M13 alongside the completed implementation entries. They are preserved as workflow history rather than deleted; the final implemented M13 work graph is `M017` with work units WU076-WU080. Current human-readable status is maintained by the roadmap.
