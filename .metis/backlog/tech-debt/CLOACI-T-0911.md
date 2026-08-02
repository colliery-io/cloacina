---
id: docs-overhaul-full-corpus
level: task
title: "Docs overhaul — full-corpus correctness and usefulness pass at 0.10.0"
short_code: "CLOACI-T-0911"
created_at: 2026-08-02T05:02:48.000484+00:00
updated_at: 2026-08-02T14:56:15.068437+00:00
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

# Docs overhaul — full-corpus correctness and usefulness pass at 0.10.0

## Objective

Audit and overhaul the entire docs corpus (Hugo site under docs/content/ + README + rustdoc drift) so consumers can (a) get up and running and (b) hold a verified mental model of how the system works. Executed via the docs-diataxis 4-phase process: discovery inventory -> plan -> parallel rewrite -> adversarial 4-reviewer gate.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P1 - High (important for user experience)

### Technical Debt Impact
- **Current Problems**: Verified doc rot at 0.10.0 — README teaches a nonexistent workflow!{} macro and pins 0.7.0; reference/configuration.md wrong on 4 fields it canonically documents; reference/cli.md missing/wrong verbs; three incompatible packaged-workflow authoring stories coexist; broken tutorial commands (API-key capture, --context, multipart field, /v1 prefixes, port 5432-vs-15432); FFI vtable documented three different wrong ways (truth: interface v5, 11 methods); stale JSON-in-debug fidius wire claims (~7-10 pages); python exceptions page documents a hierarchy that does not exist; S-0011 nomenclature violations (glossary Reactor entry, macros.md, api-reference/_index.md, 3 rustdoc comments).
- **Benefits of Fixing**: Consumers stop hitting broken first-touch paths; docs match code; single authoring story.
- **Risk Assessment**: Unfixed, every new consumer trips on the front door (README/quick-start) and reference pages actively mislead.

## Session artifacts (survive compaction)

Scratchpad: /private/tmp/claude-501/-Users-dstorey-Desktop-cloacina/c2f5127d-4859-4767-9044-b86a640f93b5/scratchpad/
- PLAN.md — shared truth brief + 9 work packages (WP-A..WP-I, disjoint file ownership) + deferred list
- inventory-{cli,rust-embedded,python-clients,server-deploy,cg-providers-packaging}.md — Phase 1 code-surface inventory (all claims file+symbol cited)
- docs-audit-{start-embed-engine,service-reference}.md — per-page audits of all ~190 hand-written pages
- changes-wp-{a..i}.md — per-WP change logs (written by writer agents as they finish)

## Work packages (Phase 3, all launched in parallel 2026-08-02)

- WP-A README + start/ + embed/quick-start (kill workflow!{}, 0.10.0, embedded-first framing)
- WP-B embed/tutorials (Accumulator trait truth, cron format, version pins)
- WP-C embed/how-to + explanation (monitoring rewrite, daemon manifest claim, AI-note leak, retry fallbacks, production tribal knowledge)
- WP-D engine/** (SyntaxError fix, depends_on->dependencies, invokes syntax, crate table, FFI truth, constructors/grants)
- WP-E reference/ core (configuration 4 errors, cli rewrite, ffi-vtable v5/11, glossary, troubleshooting + tribal knowledge, env vars)
- WP-F reference/python-api + sdks (exceptions rewrite to no-custom-hierarchy truth, wheel-only delta, lockstep)
- WP-G service/** (broken commands, ONE packaged-workflow story, signing reality, 15432)
- WP-H contributing + plissken.toml + api-reference/_index (both generation pipelines documented)
- WP-I rustdoc drift (workflow!{} crate-root doc, Runtime::from_global, 3 banned-phrase comments)

Deferred with reason (in PLAN.md): api-reference plissken regen, openapi.json regen (build tasks; configs fixed), provider-wave internals docs (await T-0886), HA caveats beyond current reality (T-0851 open).

## Acceptance Criteria

## Acceptance Criteria

- [x] All 9 WP change logs written; every edit code-cited
- [x] Phase 4 gate: accuracy/completeness/diataxis report zero blockers+majors; clarity zero blockers; minors waived with reason (see 2026-08-02 review entry)
- [x] Every inventory surface covered or explicitly deferred with reason
- [x] S-0011 nomenclature clean across corpus (grep-verified: zero banned phrases in docs + crates)

## Status Updates

- 2026-08-02: Phase 1+2 complete (7 discovery agents; inventory + plan artifacts). Phase 3 launched: 9 parallel writer agents (WP-A..WP-I). Memory correction: cloaca dual-registration is RESOLVED (I-0137, shared register_authoring).
- 2026-08-02: WP-A/B/C/F/H/I complete (change logs in scratchpad). ADJUDICATED sqlite URL truth: materialize_sqlite_connection (connection/mod.rs:526-530) strips sqlite:// and passes the remainder as a LITERAL path to diesel; diesel 2.3.7 sets SQLITE_OPEN_URI but sqlite only URI-parses file:-prefixed names, so query strings (mode=rwc, _journal_mode etc.) become part of the on-disk filename and are inert (WAL + busy_timeout=30000 applied as pragmas in run_migrations regardless). Docs canonical forms: sqlite://app.db, sqlite:///abs/path, sqlite://:memory:. Fixed in README, embed/quick-start, tutorials 01/07/09, reference/python-api/runner.md. DEFERRED same fix: engine/workflows/runner.md (WP-D running), service/explanation/database-backends.md + service/how-to/multi-tenant-recovery.md (WP-G running; database-backends.md also invents _synchronous/_busy_timeout URL params) — queue for post-writer cleanup + Phase 4 accuracy check. CODE FINDINGS for user (not docs): scripts/install.sh:26 wrong default org colliery-software; cloacinactl daemon.rs:193 builds the inert ?mode=rwc&_journal_mode=WAL URL (junk filename); connection/mod.rs:507-513 comment claims diesel never sets SQLITE_OPEN_URI — false in diesel 2.3.7 (raw.rs:51), workaround still valid.
- 2026-08-02: Phase 3 COMPLETE — all 9 WPs done (change logs changes-wp-{a..i}.md). Post-writer cleanups applied: sqlite junk URLs purged corpus-wide (incl. WP-D/G files after they finished); Python-cron adjudication (cloaca cron IS supported, packaged too — examples/features/workflows/python-cron; only the package-new scaffold refuses Python cron and its error text is stale) fixed in running-the-daemon.md + python-api/trigger.md + PLAN.md brief. NEW org contradiction adjudicated: release workflows push ghcr.io/colliery-io/* (owner-derived, unified_release.yml:463) but Helm charts pin ghcr.io/colliery-software/* (charts/*/values.yaml) and install.sh defaults colliery-software — docs now teach colliery-io with a chart-pin caveat in glossary. MORE CODE FINDINGS: charts pin wrong org; package/new.rs:24-25,70-74 stale cron-is-Rust-only error; rust --kind graph scaffold omits cloacina-macros dep (WP-G); docker-compose.production.yml + examples/fixtures carry retired serve/build.rs shapes (WP-G); seed providers path-dep the examples contract crate (discovery). Phase 4 reviewers about to launch (5 agents: accuracy x2, completeness, clarity, diataxis) — first attempt blocked by transient classifier outage, retrying.
- 2026-08-02 (later): Subagent dispatch remained blocked by the platform classifier outage through 30 attempts over several hours. Per explicit user instruction, Phase 4 ran INLINE (main context) instead of via review agents. Results: ACCURACY — ~25 highest-risk claims verified against source across all four sections (FFI v5/11-method chain, all sampled configuration.md rows + validation rules, errors.md variants, Accumulator trait, subscribe_workflow_to_reactor signature, tutorial-01 key mint flow incl. keys.rs plaintext field, WS reactor command surface vs ws.rs, multipart `file` field, cli.md constructor/package-new/--sign-fail-hard/graph-reactor split incl. /v1/health/reactors/{name}/fire route, crate-root macro re-exports for README imports, workflow.md + cg-in-workflow syntax fixes, legacy authoring-story purge) — ZERO failures; global sweeps grep-clean (workflow!{}, JSON-in-debug, 0.7.x pins, mode=rwc, banned phrases). COMPLETENESS — all 32 DefaultRunnerConfig fields in configuration.md, all cloacinactl nouns in cli.md, 5 accumulator kinds, boundary_schema/triggerless/CG-as-task, operational controls (trigger pause is HTTP-only in code — CLI has list/inspect only, docs match), troubleshooting #28-31 present + cross-indexed, env-var sample clean; deferrals stand. CLARITY — newcomer path coherent; two minors WAIVED: (1) embed tutorials 01-05/07 lack a formal Prerequisites heading (numbered sequence w/ self-contained compilable snippets; service tutorials do carry prereq sections), (2) start/concepts.md defines only the 7 primitives (deliberate scoping, explicit glossary pointer; glossary carries tenant/reconciler/packaged-workflow/shell-macro substance). DIATAXIS — former misfiles (monitoring-executions, observe-execution-state) verified in-lane; zero advice-phrases in core reference; numbered lists in explanation are descriptive sequences, not instructions. GATE MET. Deviation note: inline review (same-context) instead of independent agents, user-authorized due to outage; independent agent re-review possible later if desired.
