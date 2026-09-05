---
id: docs-reality-pass-at-0-11-0-post
level: task
title: "Docs reality pass at 0.11.0 — post-Leptos/UAT corpus audit"
short_code: "CLOACI-T-0940"
created_at: 2026-09-05T13:14:42.533965+00:00
updated_at: 2026-09-05T17:53:40.881474+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Docs reality pass at 0.11.0 — post-Leptos/UAT corpus audit

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Deep docs/doc-site accuracy pass at 0.11.0. Last full audit (T-0911) was at
0.10.0; since then the corpus drifted against: the Leptos UI migration (React
tree + npm deleted, UI embedded in cloacina-server, trunk in the build path,
UI no longer consumes the TS SDK), 0.11.0 surfaces (last_poll_at, dual detail
views, availability semantics, demo producer rates), provider waves, and the
CI/CD reshape (#277). Scope: the 192 hand-written docs under docs/content
(start/embed/engine/service/reference/contributing) — api-reference is
generated and gated separately. Method: parallel reviewer agents (accuracy +
completeness) sharded by section; findings triaged, real drift fixed, one PR.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement  
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

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

## Acceptance Criteria **[REQUIRED]**

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

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

## Status Updates **[REQUIRED]**

- 2026-09-05 AUDIT DOSSIER (read-only phase complete; writes pending infra):
  1. NPM GHOST: reference/sdks/typescript.md:17 + reference/sdks/_index.md:18 say `npm install @cloacina/client` — package NOT on npm (registry 404; publish job removed in PR #216). Fix: document in-repo install (clients/typescript) or re-add publishing (maintainer call; default: document reality).
  2. UI CHART GHOST: charts/cloacina-ui still exists AND publish-helm still packages/pushes it every release, but its default image ghcr.io/colliery-io/cloacina-ui was retired (I-0130) → helm install = ImagePullBackOff. deploy-the-web-ui.md "standalone UI via Helm" section also cites wrong org (colliery-software). Fix: delete chart + publish steps + doc section.
  3. PORT 8082 GHOSTS: .angreal/task_ui.py:43 UI_URL=8082 (banner lies — embedded UI is at :8080); service/tutorials/02-the-web-ui.md banner sample + "Open localhost:8082"; env-variables.md:50 CORS example.
  4. VERSION PINS 0.10: embed/quick-start.md:17, embed/tutorials/01:29, start/install.md:36(installer --version v0.10.0)+116+117+145, service/explanation/database-backends.md:55, service/how-to/multi-tenant-setup.md:18, running-the-server-image.md:23, production-deployment.md:183, reference/troubleshooting.md:458-465, env-variables.md:330. Fix: bump to 0.11 AND add docs-pin regexes to .angreal/version_lockstep.py so bumps sweep docs.
  5. UI TOUR DRIFT: service/tutorials/02 tour describes React-era panels the Leptos UI lacks (Schedule/Inputs cards, p50/p95 strip, per-task health, trigger enable switch, Overview inline Pause/Fire) and misses shipped UX (workflows table w/ Pause/Run columns, cron-vs-polling trigger sections + poll cadence, dual detail views Current|Operational history, accumulator availability dots, run summary/outcomes gantt). All 9 docs/static/images/web-ui/*.png are React screenshots → re-shoot on live Leptos stack.
  6. embedded-ui.md:9 calls the UI "@cloacina/ui" (dead npm name; crate is cloacina-ui). env-variables.md:475 SKIP_NPM described as npm rebuild (trunk now).
  7. HTTP-API GAPS: reference/http-api.md trigger example lacks last_poll_at; NO section for named-instance endpoints (POST/GET /v1/tenants/{t}/workflows/{name}/instances, GET/DELETE .../instances/{instance} — routes/instances.rs, T-0894).
  CLEAN: nomenclature (S-0011), kafka-not-core docs, repository-structure.md, CLI command surface (all doc'd nouns exist incl config/admin), server helm chart docs, tutorial 06 DB framing.
  Plan: branch docs/t0940-reality-pass → apply 1-7 → re-shoot screenshots on demo stack → hugo build check → PR.
- 2026-09-05 FIXES APPLIED (uncommitted in working tree — Bash/exec unavailable, all edits via file tools):
  - sdks/typescript.md + _index.md: npm-install replaced with build-from-repo truth.
  - ALL 0.10 crate pins → 0.11 (16 files, incl. brace-form pins the first grep missed); image tags + installer example → 0.11.0.
  - version_lockstep.py: docs touchpoints added (_doc_files + _DOC_CRATE_PIN/_DOC_IMAGE_TAG/_DOC_INSTALLER in found_versions AND set_version); cloacina-ui chart removed from _HELM_CHARTS.
  - unified_release.yml: publish-helm no longer versions/lints/packages/pushes charts/cloacina-ui.
  - task_ui.py UI_URL → 8080; tutorial 02: ports fixed + full view-tour rewrite (tables, dual views, trigger sections, availability); embedded-ui.md crate name; env-variables (SKIP_NPM desc, CORS example, version example).
  - deploy-the-web-ui.md: standalone-Helm section replaced with different-origin guidance.
  - http-api.md: trigger list envelope schedules→items (live API returns items), last_poll_at/paused fields added, T-0929 fire caveat hint, NEW named-instances endpoint section.
  REMAINING (needs shell): delete charts/cloacina-ui dir, git branch/commit/PR, run version_lockstep check to validate new regexes, demo stack up + re-shoot 9 web-ui screenshots, hugo build check.