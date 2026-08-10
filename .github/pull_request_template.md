<!--
Pi-Hub Pull Request Template

Keep the PR concise and scan-friendly.
Replace all placeholders and remove instructions/comments that do not apply.

Risk bands:
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

<!-- Example: M7 — Service Health and Operational Activity -->

**<🟢/🟡/🟠/🔴> Risk Score: <0–100>/100 — <Green/Yellow/Orange/Red>**

<!--
Provide one short sentence explaining the score.
Example:
Risk is primarily driven by new persisted operational state and network behavior; automated coverage is green, with live smoke validation pending.
-->

## Summary

<!-- 2–5 concise bullets describing the outcome, not the implementation diary. -->

- 
- 

## Work Units / Scope

<!-- For milestone PRs, list every Work Unit and its status. Remove this section for non-milestone changes if not applicable. -->

- [ ] `<WU>` — <name>
- [ ] `<WU>` — <name>

## Key Changes

<!-- Group only the material implementation changes. -->

- 
- 
- 

## Validation

<!--
Include the canonical validation actually run.
Do not claim a check passed unless it was executed successfully.
Add/remove commands to match the repository's current canonical validation.
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
Record warnings separately. A pre-existing warning may remain only when confirmed unrelated and non-regressive.
Example:
- `npm run lint`: passed with 4 confirmed pre-existing Fast Refresh warnings.
-->

## Documentation / AIQT

<!-- State exactly what changed. -->

- [ ] Relevant documentation added/updated
- [ ] AIQT Work Unit / milestone state reconciled with implementation evidence
- [ ] Later milestones remain planned/not started unless explicitly in scope
- [ ] Roadmap/specification references updated where required

## Compatibility / Fallback Behavior

<!--
Describe important compatibility, partial-data, unsupported-platform, migration, or fallback behavior.
Use "N/A" only when genuinely not applicable.
-->

- 

## Live / Manual Validation

<!--
Do not imply destructive or live-device validation occurred unless it actually did.
If unavailable, state it clearly as pending residual validation.
-->

- **Status:** <Performed / Partially performed / Pending / Not applicable>
- **Evidence / remaining checks:** 

## Risks

<!--
List the main factors contributing to the numerical risk score.
Focus on actual failure modes introduced by this PR.
-->

- 
- 

## Mitigations

<!-- Link each important risk to concrete tests, architecture boundaries, fallbacks, or operational controls. -->

- 
- 

## Residual Risks

<!--
What remains true even after mitigation?
Be explicit about pending live/manual validation.
Use "None identified" only when justified.
-->

- 

## Merge Readiness

- [ ] Scope matches the approved milestone/change requirements
- [ ] Required Work Units / acceptance criteria are complete
- [ ] Canonical automated validation is green
- [ ] No valuable tests/checks were weakened or skipped to obtain green status
- [ ] Working tree / PR diff contains only intentional changes
- [ ] Temporary/debug artifacts were removed
- [ ] Documentation and AIQT state match the actual implementation
- [ ] Residual/manual validation is accurately disclosed
- [ ] Risk band matches the numeric score
- [ ] Required approval level for the risk band is satisfied

<!--
Do not merge the PR automatically unless repository governance explicitly permits it.
Do not publish a release as part of the PR unless the milestone/release workflow explicitly requires it.
-->
