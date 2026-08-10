# Pi-Hub agent instructions

## Repository invariants

- Preserve the existing Tauri/React/Rust architecture and extend existing abstractions before creating parallel systems.
- Frontend features must use typed backend/application operations. Do not introduce arbitrary remote shell, Docker, or `systemctl` command APIs.
- SSH host-key verification is mandatory. Never weaken it or auto-accept changed keys.
- Do not store SSH password/private-key contents, sudo credentials, or new service secrets.
- Remote/network operations must be bounded and failures isolated. Unknown/unavailable data is not Healthy and is not zero.
- Preserve established Device Health, Service Health, Activity, Alert, Docker, and administration semantics unless the approved task explicitly changes them.

## Working agreements

- For milestone work, use `$pi-hub-milestone` and treat the approved milestone document as the source of truth for milestone-specific requirements.
- Inspect only code and documentation relevant to the task. Avoid broad repository re-investigation and repeated full-document reads unless evidence requires them.
- Use the repository's native AIQT workflow; state must reflect implementation and validation evidence.
- Run focused tests while developing. At milestone closure run `node scripts/validate-agent.mjs` once as the concise canonical local validation wrapper, plus any milestone-specific validation not covered by it.
- Do not weaken, remove, or skip valuable tests/checks merely to obtain green status.
- Update only documentation directly affected by the current task. Do not perform M13 Document Hygiene before M13.
- Follow `.github/pull_request_template.md` and derive PR content from the approved requirements, actual diff, AIQT evidence, and executed validation.
- Do not merge PRs or publish releases unless explicitly requested.
- Use a single primary agent by default. Do not spawn subagents unless the task has genuinely independent workstreams and the benefit clearly justifies the extra context/usage.
- Do not execute disruptive live device/container administration unless explicitly authorized for the current task.
