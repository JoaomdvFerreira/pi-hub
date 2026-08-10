<!--
Pi-Hub Pull Request Template

Purpose:
Create PRs that are easy to review now and useful as historical product/change records later.

IMPORTANT FOR AGENTS:
- Populate this template from the actual implementation, milestone document, diff, validation evidence, and AIQT state.
- Do NOT merely copy Work Unit names into the Summary or Features Delivered sections.
- Explain what changed in the product from the user's/operator's perspective.
- Keep the PR scan-friendly: concise, structured, and evidence-based.
- Remove placeholder comments/instructions before creating or updating the PR.

Official risk bands:
🟢 Green  = 0–24
🟡 Yellow = 25–49
🟠 Orange = 50–74
🔴 Red    = 75–100

Governance:
- Risk < 50: may proceed through the repository's normal automated/agent approval flow when all other gates pass.
- Risk >= 50: requires human review/approval before merge.
- Never use informal labels such as "low", "medium", or "high" in place of the official risk band.
-->

# <Milestone / Change Title>

<!-- Example: M8 — Alerts and Threshold Governance -->

**<🟢/🟡/🟠/🔴> Risk Score: <0–100>/100 — <Green/Yellow/Orange/Red>**

<!--
Immediately below the score, include one concise sentence explaining the main risk driver(s) and why the score is appropriate.
-->

## Summary

<!--
REQUIRED.

Write a concise product-level summary (normally 2–5 bullets or a short paragraph).

Derive it from the actual implementation and approved scope.

Explain:
- the purpose of the change;
- the most important outcome;
- the user/operator value.

Do NOT:
- repeat only Work Unit names;
- list implementation files;
- write an execution diary.
-->

-
-

## Features Delivered

<!--
REQUIRED for milestone PRs.

Describe the functionality actually delivered, based on the implementation, milestone requirements, and diff.

This is the primary section that tells a reviewer what the application can now do.

Cover, where applicable:
- new user-visible capabilities;
- new operational behavior;
- new screens/views/actions;
- new configuration/options;
- new state/lifecycle behavior;
- integrations with existing functionality;
- important fallback/compatibility behavior.

Use meaningful subheadings when the milestone introduces multiple capabilities.

Do NOT simply restate:
"WUxx-01 completed", "WUxx-02 completed", etc.

Do NOT invent functionality that is only planned or not validated.
-->

### <Feature / Capability>

-

### <Feature / Capability>

-

## Work Units / Scope

<!--
For milestone PRs, list every Work Unit and its completion status.
For non-milestone PRs, replace with the relevant scoped deliverables or remove if genuinely not applicable.
-->

- [ ] `<WU>` — <name>
- [ ] `<WU>` — <name>

## Technical Implementation

<!--
Summarize the most material technical/architectural changes.

Derive this from the actual diff.

Focus on:
- domain/model changes;
- persistence/migrations;
- monitoring/background behavior;
- backend/Tauri boundaries;
- frontend integration;
- compatibility/migration approach;
- security-relevant implementation decisions.

Do not list every changed file.
-->

- 
- 

## Validation

<!--
Include the canonical validation actually run.

Do not claim a check passed unless it was executed successfully.
Use repository-defined commands and add/remove checks to match the current canonical validation.
-->

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] Focused Rust tests for changed functionality
- [ ] `npm test`
- [ ] `npm run build`
- [ ] `npm run lint`
- [ ] `aiqt status`
- [ ] `git diff --check`

<!--
Record warnings separately.

A pre-existing warning may remain only when confirmed unrelated and non-regressive.
Example:
- `npm run lint`: passed with 4 confirmed pre-existing Fast Refresh warnings.
-->

## Documentation / AIQT

<!--
State exactly what changed and what remains planned.

For milestones, confirm:
- milestone documentation integration/update;
- AIQT Work Unit/milestone state;
- future milestones remain planned/not started unless explicitly in scope.
-->

- [ ] Relevant documentation added/updated
- [ ] AIQT Work Unit / milestone state reconciled with implementation evidence
- [ ] Later milestones remain planned/not started unless explicitly in scope
- [ ] Roadmap/specification references updated where required

## Compatibility / Migration / Fallback Behavior

<!--
Describe important compatibility and upgrade behavior.

Cover where applicable:
- existing settings/data migration;
- previous milestone compatibility;
- unsupported-platform behavior;
- partial-data behavior;
- fallback/default behavior;
- preserved security boundaries.

Use "N/A" only when genuinely not applicable.
-->

- 

## Live / Manual Validation

<!--
Do not imply destructive or live-device validation occurred unless it actually did.

If unavailable, explicitly state it as pending residual validation.
-->

- **Status:** <Performed / Partially performed / Pending / Not applicable>
- **Evidence:**
- **Remaining manual checks:**

## Risks

<!--
List the main factors contributing to the numerical risk score.

Focus on actual failure modes introduced by this PR, such as:
- persistence/migration;
- monitoring/concurrency;
- network behavior;
- security boundaries;
- state-machine changes;
- changes to existing policy/behavior;
- hardware/platform compatibility.
-->

- 
- 

## Mitigations

<!--
Map important risks to concrete mitigations:
- automated tests;
- architecture boundaries;
- migrations/defaults;
- timeouts/fallbacks;
- validation evidence;
- feature isolation.
-->

- 
- 

## Residual Risks

<!--
State what remains true after mitigations.

Explicitly include pending live/manual validation where relevant.

Use "None identified" only when justified.
-->

- 

## Merge Readiness

- [ ] Scope matches the approved milestone/change requirements
- [ ] Features Delivered accurately reflects the implemented product behavior
- [ ] Required Work Units / acceptance criteria are complete
- [ ] Canonical automated validation is green
- [ ] No valuable tests/checks were weakened or skipped to obtain green status
- [ ] Working tree / PR diff contains only intentional changes
- [ ] Temporary/debug artifacts were removed
- [ ] Documentation and AIQT state match the actual implementation
- [ ] Compatibility/migration behavior is documented where relevant
- [ ] Residual/manual validation is accurately disclosed
- [ ] Risk band matches the numeric score
- [ ] Required approval level for the risk band is satisfied

<!--
Before creating/updating the PR, perform a final scan:
1. Could a reviewer understand what functionality was delivered without opening the milestone document?
2. Does the PR distinguish product functionality from technical implementation?
3. Are all validation claims supported by executed evidence?
4. Are residual risks explicit?
5. Is the PR concise enough to scan quickly?

Do not merge the PR automatically unless repository governance explicitly permits it.
Do not publish a release as part of the PR unless the milestone/release workflow explicitly requires it.
-->
