---
id: operator-docs-follow-ups-from-qa
level: task
title: "Operator docs follow-ups from QA harness (P4 placement + P5 missing pages)"
short_code: "CLOACI-T-0701"
created_at: 2026-06-15T20:24:00.144224+00:00
updated_at: 2026-06-15T20:53:47.896908+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Operator docs follow-ups from QA harness (P4 placement + P5 missing pages)

## Objective

Deferred documentation improvements surfaced by the operator-docs QA harness
(46 naive-answerer + expert-grader runs against the built site, 2026-06-15). The
**accuracy bugs (P1/P2/P3)** were fixed directly on PR #127; this captures the
**placement (P4)** and **missing-page (P5)** items, which are larger adds rather
than corrections. Full report: `/tmp/cloacina-docs-qa/report.md` (and grades.json).

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement (docs)

### Priority
- [ ] P0
- [ ] P1
- [x] P2 - Medium

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

**P4 — placement / findability (content exists, but not where an operator looks):**
- [x] **Ports in the config reference** (Q05): added a "Server flags (`cloacina-server`)" table to `/reference/configuration.md` (`--bind` default `127.0.0.1:8080` + the main flags/env vars, grounded in `crates/cloacina-server/src/main.rs`), with cross-links to deploying-the-api-server + running-the-server-image.
- [x] **Manage-API-keys how-to** (Q22): created `/service/how-to/manage-api-keys.md` (CLI `key create/list/revoke` per `crates/cloacinactl/src/nouns/key/mod.rs` → `/v1/auth/keys`; tenant scope inferred from calling key, `--tenant` for admin targeting; REST table). Cross-linked from security-model.md (back-link added) and the page links the deploy tutorial.
- [x] **Server-mode multi-tenant recovery** (Q14): added a "Server-mode operators" hint to `multi-tenant-recovery.md` — lazy per-tenant runner cache (LRU, `--tenant-runner-cache-size`), recover by restarting the server (runners rebuilt lazily, recovery re-runs on connect; no per-tenant restart endpoint), SQLite file restore, cross-link to decommission-a-tenant.

**P5 — missing day-2 pages (gap-probes, partial coverage):**
- [x] **Backup & restore** (Q42): created `/service/how-to/backup-and-restore.md` — Postgres full (`pg_dump -Fc`/`pg_restore`) + per-tenant schema, SQLite online backup (`sqlite3 .backup` / `VACUUM INTO`) + stop/cp/start restore. Linked from the production-deployment checklist.
- [x] **Non-Kubernetes upgrade runbook** (Q43): added "## Upgrading the server" to `production-deployment.md` — backup → pin tag → drain/swap (migrations auto-run on startup, verified `crates/cloacina/src/runner/default_runner/mod.rs:110-112`) → gate on `/ready`; single-replica downtime caveat + rollback. Cross-links the Helm Upgrades section.

**Verify item — RESOLVED:**
- [x] **Helm OCI org** (Q03): the release workflow (`.github/workflows/unified_release.yml:631-688`) pushes to `ghcr.io/<owner>/charts/cloacina-server` where `<owner>` = the GitHub repo owner (`colliery-io`). So **the docs (`colliery-io`) are correct**; the `colliery-software` strings are only in `Chart.yaml` `home`/`sources` metadata — a **code-side** inconsistency, out of scope for this docs PR. Flagged below for a separate one-line code fix.

## Notes
- Parity follow-ups (Python `state_accumulator`; single-language tutorials 12-full-pipeline / 14-packaged-triggers) are tracked separately in [[CLOACI-T-0688]].
- The harness itself (questions + workflow script) lives at `/tmp/cloacina-docs-qa/operator-questions.md` and the saved workflow script — re-runnable after these land to confirm the gaps close.

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**:
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates

- 2026-06-15: All P4 + P5 items done and committed to PR #127 (commit `8dfdf9f9`, build-green): server-flags reference table, `manage-api-keys.md`, `backup-and-restore.md` (both new how-tos), server-mode recovery hint, and the "Upgrading the server" runbook. Every command/flag grounded against `crates/`. Helm-org verify resolved (docs already correct). **Remaining (code, not docs, separate PR):** fix `charts/cloacina-server/Chart.yaml` `home`/`sources` URLs `colliery-software` → `colliery-io` to match the publish target + repo. Optional: re-run the 5 affected harness questions (Q05, Q14, Q22, Q42, Q43) against the rebuilt site to confirm the gaps now grade `ok`.
