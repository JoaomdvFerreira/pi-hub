---
name: pi-hub-milestone
description: Execute a Pi-Hub product milestone end-to-end from an approved milestone document, including Git baseline reconciliation, AIQT workflow, focused implementation/testing, concise closure validation, and PR preparation. Use for M11+ milestone implementation; do not use for small standalone fixes or M13 Document Hygiene unless explicitly requested.
---

# Pi-Hub milestone workflow

Use the attached/identified approved milestone document as the source of truth for **what** to implement. This skill defines **how** to execute it.

## 1. Preflight and Git baseline

1. Run `git fetch --prune origin` before evaluating merge state.
2. Inspect the current branch and `git status --short`.
3. If the worktree is dirty:
   - inspect the exact diff;
   - distinguish substantive content from line-ending/worktree-only artifacts;
   - only restore an artifact when there is no substantive content change;
   - if a real unrelated change exists, stop and report it. Do not stash, discard, commit, or carry it forward without authorization.
4. Verify the previous milestone is present in `origin/main`; do not rely on a stale remote-tracking ref.
5. Switch to `main` and fast-forward with `git pull --ff-only origin main`.
6. Confirm the merged baseline and AIQT state are present and the worktree is clean.
7. Create the milestone branch from `main` using the repository naming convention.
8. Local deletion of a previous milestone branch is optional; never force-delete potentially unique work.

## 2. Integrate planning

- Place the approved milestone document at its canonical path.
- Reconcile/start the milestone through native AIQT commands.
- Preserve completed history and keep future milestones planned/not started.
- Update only directly affected navigation/specification references.
- Do not perform M13 Document Hygiene unless the current milestone is M13.

## 3. Implement efficiently

- Read the milestone document once to build a requirements/WU checklist; revisit only relevant sections as work proceeds.
- Inspect only repository areas needed for the current Work Unit.
- Extend established domain, persistence, backend/Tauri, frontend, monitoring, and test patterns instead of creating parallel systems.
- Work through all milestone Work Units unless a genuine human blocker occurs.
- Avoid unrelated refactoring and speculative cleanup.
- Do not begin the next milestone.
- Use one primary agent by default; do not spawn subagents unless independent parallel investigation is genuinely useful.
- Do not perform disruptive live validation without explicit current-task authorization.

## 4. Validation discipline

For each Work Unit:
- run focused tests/checks for the changed behavior;
- fix regressions before continuing;
- do not run the complete suite after every Work Unit unless repository policy or evidence requires it.

At closure:
1. review every milestone exit criterion against implementation evidence;
2. run `node scripts/validate-agent.mjs` once;
3. run any milestone-specific validation not covered by that wrapper;
4. fix milestone-caused failures;
5. identify pre-existing warnings accurately rather than suppressing them;
6. run `git diff --check` again if changes are made after validation.

## 5. Closure, Git, and PR

Only when exit criteria are satisfied:
- close Work Units/milestone through native AIQT workflow;
- keep later milestones planned/not started;
- reconcile directly affected documentation;
- review the complete diff and remove temporary/debug artifacts;
- commit and push the milestone branch;
- create/update the PR using `.github/pull_request_template.md`;
- populate `Features Delivered` from actual product behavior, not Work Unit names;
- use only validation evidence that was actually executed;
- disclose deferred/live/manual validation and material deviations;
- do not merge the PR or publish a release.

## 6. Final response

Keep the closure response concise because the PR is the detailed record. Return only:

- completion status;
- branch, commit(s), and PR;
- concise validation summary;
- material deviations, if any;
- residual/manual validation or blockers, if any.

Stop when the milestone is complete or a genuine human decision is required.
