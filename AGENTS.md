# Nodus Studio: Agent Instructions

## Read before working

1. Read `master_project_blueprint_ai_context_specification.md`, the single source of truth for architecture, stack, roadmap, and acceptance criteria.
2. Read `PROGRESS.md` for the current phase, blockers, and next action.
3. Read the latest numbered report linked there and any reports relevant to the requested work. Check actual files and available tooling; reports may describe an older environment.
4. Stay within the authorized phase and task. Do not treat a successful substep as completion of its entire phase.

## Mandatory stage reports

- For each meaningful development substep or phase, create a Markdown report in `docs/reports/` using `docs/reports/TEMPLATE.md`.
- Name reports with the next unused repository-wide sequence: `NNN-phase-N-short-topic.md`. Start at `001`; never overwrite an earlier stage to reuse its number.
- Write a report when a stage completes, is partially completed, or is blocked. A blocked stage still needs a handoff. For a stage spanning sessions, save a partial report before handing off and record follow-up work in the next report. Do not rewrite earlier outcomes as if later work had already existed.
- Before the final response, record the scope, actual changes/files, decisions, verification commands/results, limitations, blockers, and concrete next steps. Distinguish work completed from work merely planned.
- Update `PROGRESS.md` with the current phase/substep, latest report link, roadmap status, blockers, and next action. Do not require reading every historical report to resume work.
- Use these status labels consistently: `not-started`, `in-progress`, `partial`, `blocked`, `completed`. A phase is `completed` only after all blueprint acceptance criteria are supported by evidence.
- Never fabricate test results, tool versions, installation success, or desktop checks. Use `not-run`, `failed`, `passed`, or `blocked` for each verification outcome, with the reason and scope.
- Keep reports concise and factual. Do not include secrets, access tokens, private credentials, or unnecessary raw logs. Source code and technical comments use English; project progress reports may use Indonesian for handoff clarity.
- Routine read-only explanations and trivial edits do not need separate reports; if they affect the current stage, include them in that stage's report. Explicit user requests for reporting still apply.

## Safety and scope

Preserve user work. Do not change the approved stack, skip roadmap gates, execute Lua in Phase 1, or put hardware simulation in the frontend. Do not install system packages, create commits/branches/worktrees, stage, push, or publish without explicit authorization. Use relevant checks and report any verification that remains unavailable.

These are repository instructions for agents that support `AGENTS.md`; other agents must be directed to this file if they do not discover it automatically.
