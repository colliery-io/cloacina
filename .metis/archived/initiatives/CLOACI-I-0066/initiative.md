---
id: documentation-review-diátaxis
level: initiative
title: "Documentation Review — Diátaxis Compliance, Gap Fill, Accuracy"
short_code: "CLOACI-I-0066"
created_at: 2026-04-02T22:50:55.468029+00:00
updated_at: 2026-04-02T23:47:58.417763+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: documentation-review-diátaxis
---

# Documentation Review — Diátaxis Compliance, Gap Fill, Accuracy Initiative

## Context

Cloacina's documentation site follows the Diátaxis framework (tutorials, how-to guides, reference, explanation). A deep audit revealed that tutorials, Python bindings, and explanation docs are largely complete, but significant gaps exist in Reference and How-To categories. Several existing docs also have accuracy issues (stale API signatures, missing imports, incomplete content).

## Goals & Non-Goals

**Goals:**
- Fill all documentation gaps — especially CLI, HTTP API, and Configuration reference docs
- Fix accuracy issues in existing tutorials (API mismatches, missing deps)
- Expand stub/incomplete pages (multi-tenant-recovery, performance-characteristics, index pages)
- Ensure every user-facing feature is documented somewhere
- Cross-link between documents per Diátaxis best practices

**Non-Goals:**
- Rewriting docs that are already complete and accurate
- Changing the Hugo site structure or theme
- Adding new features to the codebase

## Detailed Design

### Phase 3a — Write New Documents (12 docs)

**Reference (6 docs):**
1. reference/_index.md — Un-draft, add real content with category overview
2. CLI Reference (cloacinactl) — daemon, serve, config, admin commands, all flags, env vars
3. HTTP API Reference — All 15+ endpoints, request/response schemas, auth, error codes
4. Configuration Reference — DefaultRunnerConfig (30+ fields), config.toml, env vars
5. Macro Reference — #[task], #[workflow], #[trigger] all attributes
6. Error Reference — All error enums and variants

**How-To Guides (4 docs):**
7. Running the Daemon — Watch dirs, package deployment, logs, config
8. Deploying the API Server — Bootstrap key, bind, health checks, production
9. Monitoring Executions via API — List executions, event logs, status polling
10. Cleaning Up Old Events — cloacinactl admin cleanup-events

**Explanation (2 docs):**
11. Architecture Overview — Deployment modes (embedded, daemon, server), component map
12. Horizontal Scaling & Task Claiming — Atomic claiming, heartbeats, distributed execution

### Phase 3b — Fix Existing Documents (~10 docs)
- Tutorial 06: Fix API signature mismatch (Context/PipelineError → Context<Value>/TaskError)
- Tutorial 05: Fix malformed reviewer date, improve output examples
- Tutorials 03-04: Add rand crate to listed dependencies
- how-to-guides/multi-tenant-recovery.md: Expand from ~300 words to full guide
- explanation/performance-characteristics.md: Complete with real content
- how-to-guides/_index.md: Add proper index content listing all guides
- explanation/_index.md: Add proper index content listing all explanations
- reference/api-test.md: Mark as draft or remove from navigation

### Phase 4 — Parallel Review
Launch 4 review agents: Accuracy, Completeness, Clarity, Diátaxis Compliance.

## Implementation Plan

Tasks will be decomposed into parallel workstreams:
- T1: Write all new Reference docs (6 docs)
- T2: Write all new How-To guides (4 docs)
- T3: Write all new Explanation docs (2 docs)
- T4: Fix existing document accuracy issues (~10 docs)
- T5: Parallel review pass with 4 agents
