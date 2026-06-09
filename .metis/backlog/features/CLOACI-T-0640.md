---
id: server-level-default-executor
level: task
title: "Server-level default executor (Airflow-style) — execution as a deployment knob, glob routing as override"
short_code: "CLOACI-T-0640"
created_at: 2026-06-08T15:03:27.135237+00:00
updated_at: 2026-06-09T09:45:31.812644+00:00
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

# Server-level default executor (Airflow-style) — execution as a deployment knob, glob routing as override

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Make the executor a single **server-level deployment setting** (like Airflow's
`[core] executor`) instead of expressing the common case through the glob router.
All executions go through one configured executor by default; per-task glob
routing becomes an *advanced override*, not the primary surface.

Today "send everything to the fleet" is `CLOACINA_FLEET_ROUTES='**=fleet'` — which
makes operators learn the glob DSL (and the `**`-not-`*` footgun) just to pick a
default execution path. Workflow authors already don't encode the executor
(routing is server-side), so this is purely operator ergonomics + framing:
execution topology should be one obvious knob, with globs as the power-user
escape hatch.

**Pre-release freedom (noted 2026-06-08):** nothing is released yet → no adopters
to break. We can change defaults, rename flags, and even reconsider whether glob
routing is the right *primary* surface vs a power-user feature — no backward-compat
constraints. Decide the defaults deliberately now.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [x] Feature - New functionality or enhancement
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [x] P2 - Medium (nice to have) — ergonomics/framing; do before first release
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

## Acceptance Criteria **[REQUIRED]**

*(Rewritten 2026-06-08 after the scope pivot: glob routing is DELETED, not
demoted; execution is a single deployment knob; config.toml is the preferred
surface. See Status Updates for the decision record.)*

- [x] **Single executor knob.** The server has one `default_executor` setting.
      Unset → every task runs on `default` (thread, in-process). Set to `fleet` →
      every task dispatches to the execution-agent fleet.
- [x] **config.toml is the preferred surface.** A `[server]` section
      (`default_executor = "..."`) in `~/.cloacina/config.toml` configures it.
      `cloacinactl server start` reads it and forwards to `cloacina-server`.
      Precedence (mirrors `resolve_database_url`): explicit `--default-executor` /
      `CLOACINA_DEFAULT_EXECUTOR` > config.toml `[server].default_executor` >
      built-in `default`.
- [x] **Hard validation at boot.** `--default-executor <key>` must match a
      registered executor key. An unknown key fails fast at startup with an error
      listing the valid keys (no silent "all work to a nonexistent executor" stall).
- [x] **Glob routing fully removed.** `Router` / `RoutingConfig` / `RoutingRule`,
      the `--route` flag + `CLOACINA_FLEET_ROUTES` env var, `build_routing_config`,
      and all glob-routing docs/tests are deleted. The dispatcher sends every task
      to the one configured executor key (no per-task matching).
- [x] **Forwarding gap closed.** `cloacinactl server start` forwards the resolved
      default executor to the `cloacina-server` binary (today it forwards neither
      `--route` nor an executor setting).
- [x] **Docs reframed.** Execution documented as a single deployment knob set via
      config.toml; fleet is opt-in (`default_executor = "fleet"`). Decision
      recorded: per-task routing is deferred to a future executor-internal feature
      (executors route work to specific nodes/compute; the scheduler does not).

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
The routing engine already supports a catch-all (`**=<key>`), so `--default-executor X`
is, mechanically, just injecting `**=X` as the lowest-priority rule and letting
explicit `--route` globs override it. Touch points (from the I-0114 work):
- `crates/cloacina-server/src/main.rs` — add `--default-executor` /
  `CLOACINA_DEFAULT_EXECUTOR` clap arg → `run()`.
- `crates/cloacina-server/src/lib.rs` — fold the default into the routing config
  build (lowest-priority `**` rule), then layer `--route` rules over it.
- `crates/cloacina/src/dispatcher/router.rs` — confirm rule precedence is
  most-specific / explicit-wins so the default is the fallback.
- `crates/cloacinactl/.../server start` — forward both the default-executor flag
  AND `--route` (the wrapper forwards neither today).
- Docs: `platform/explanation/execution-agent-fleet`, `reference/cli`,
  `reference/environment-variables` — lead with the deployment knob; demote globs.

Open design question to settle first: keep `default` (thread) as the shipped
default, or make `fleet` opt-in only? And do we keep per-task globs at all for v1,
or ship just the single knob and add globs later?

### Dependencies
Builds on the I-0114 fleet routing (`CLOACINA_FLEET_ROUTES`, dispatcher executor
keys `default`/`fleet`). No blockers — I-0114 is complete.

### Risk Considerations
- Low blast radius (config/CLI + framing); pre-release so no compat constraints.
- Don't regress the existing glob behavior for power users — default must compose
  with explicit routes (override semantics), with tests for precedence.

## Status Updates **[REQUIRED]**

### 2026-06-08 — Review + decisions (scope pivot)

**Design decisions (user, 2026-06-08):**
1. **Shipped default executor = `default` (thread).** Zero-config servers run
   everything in-process; operators opt into the fleet with
   `--default-executor fleet` / `CLOACINA_DEFAULT_EXECUTOR=fleet`.
2. **DELETE the entire glob-route surface** — not "demote globs to power-user".
   Rationale (user): *routing is an executor-internal concern* — executors will
   route work to specific nodes/compute capabilities in the future; the
   scheduler/dispatcher should NOT know about per-task routing. So the dispatcher
   sends every task to ONE configured executor key. Globs/`RoutingConfig`/
   `RoutingRule`/`Router` are removed wholesale. This supersedes the AC items that
   assume globs-as-override (rewrite those: default executor is the ONLY surface).

**Key finding:** `Router::resolve` already falls back to `default_executor` when
no rule matches, so executor-by-key selection collapses cleanly to "always use the
configured key" once the rule list is gone. No new precedence logic needed.

### Deletion / change surface (mapped)

**cloacina core:**
- DELETE `crates/cloacina/src/dispatcher/router.rs` (struct `Router` + glob engine
  + its unit tests).
- `dispatcher/types.rs`: remove `RoutingConfig` + `RoutingRule`.
- `dispatcher/mod.rs`: drop `pub use router::Router;` (64) and `RoutingConfig,
  RoutingRule` from re-export (67); rewrite module docs (32-48).
- `dispatcher/default.rs`: replace `Router` field + `router.resolve()` (138) with a
  stored `default_executor_key: String`; `DefaultDispatcher::new(dal, key)`;
  `resolve_executor_key()` returns the static key; fix routing tests (256-274).
- `runner/default_runner/config.rs`: replace `routing_config: Option<RoutingConfig>`
  field + both `routing_config(...)` builders + build assembly with a
  `default_executor: String` (default `"default"`).
- `runner/default_runner/mod.rs`: pass `default_executor` into `DefaultDispatcher::new`.
- `crates/cloacina/src/lib.rs:567`: remove `RoutingConfig, RoutingRule` from prelude.

**cloacina-server:**
- `main.rs`: remove `--route`/`CLOACINA_FLEET_ROUTES`/`routes`; add
  `--default-executor` / `CLOACINA_DEFAULT_EXECUTOR` (default `"default"`) -> run().
- `lib.rs`: delete `build_routing_config`; change `run()` param
  `fleet_routes: Vec<String>` -> `default_executor: String`; replace routing_config
  block (590-603); thread `default_executor` into `runner_config_for_tenant_cache`.
  Gate per-tenant fleet registrar (673) on `default_executor == "fleet"`; global
  fleet registration (752-774) stays unconditional (registered-but-idle when default
  is thread); tidy comment at 746 (refers to old RoutingRule).

**cloacinactl:**
- `nouns/server/mod.rs`: add `--default-executor` arg to the `Start` verb.
- `nouns/server/start.rs`: forward `--default-executor` (closes the forwarding gap).

**Tests:**
- `.angreal/test/e2e/fleet.py:110,126` + `.angreal/test/soak/fleet.py:176`:
  swap `CLOACINA_FLEET_ROUTES="**=fleet"` -> `CLOACINA_DEFAULT_EXECUTOR=fleet`.

**Docs / charts:**
- DELETE `docs/content/workflows/how-to-guides/custom-task-routing.md`.
- Rewrite `platform/explanation/execution-agent-fleet.md`,
  `platform/how-to-guides/deploy-an-execution-agent-fleet.md`,
  `reference/cli.md` (202-210), `reference/environment-variables.md` (59-62),
  `workflows/explanation/dispatcher-architecture.md`.
- `charts/cloacina-agent/templates/NOTES.txt:21` + `README.md:9` — `--route` ->
  `--default-executor fleet`.

### 2026-06-08 — Two added requirements (user)

1. **Hard-match executor keys at boot (CONFIRMED, not optional).** After the
   server registers its executors (`default`, and `fleet` when opted in),
   validate the resolved `default_executor` against the registry; bail with an
   error listing valid keys if unknown. Validation site: `cloacina-server/src/lib.rs`
   right after the fleet registration block (~774), where the registry is known.
2. **config.toml is the preferred surface over CLI.** Add a `[server]` section to
   `CloacinaConfig` (`crates/cloacinactl/src/commands/config.rs`) with
   `default_executor: String` (default `"default"`). Add a
   `resolve_default_executor(cli: Option<&str>, config_path)` mirroring
   `resolve_database_url` — precedence **explicit CLI/env > config.toml > "default"**.
   `cloacinactl server start` calls it and forwards `--default-executor` to the
   binary. `cloacina-server` keeps the `--default-executor`/`CLOACINA_DEFAULT_EXECUTOR`
   flag for direct runs; it does NOT load config.toml itself (cloacinactl is the
   config-aware front door, same as `--database-url` today).

### 2026-06-08 — Implementation complete (pending compile/test verification)

Branch `feat/T-0640-default-executor`. All planned edits landed:

**Core (cloacina):** deleted `dispatcher/router.rs`; removed `RoutingConfig`/
`RoutingRule` from `types.rs`, `mod.rs` re-exports, and the prelude (`lib.rs`).
`DefaultDispatcher` now stores `default_executor_key: String` and dispatches every
task to it; `new(dal, impl Into<String>)`, added `default_executor_key()` getter,
`with_defaults` → `"default"`. Added `Dispatcher::has_executor(key)` +
`DefaultDispatcher` impl + `DefaultRunner::has_executor` passthrough (for boot
validation). Runner config: `routing_config` field/builders → `default_executor:
String` (default `"default"`), getter `default_executor()`. De-routed module/trait
docs.

**Server (cloacina-server):** `main.rs` dropped `--route`/`CLOACINA_FLEET_ROUTES`;
added `--default-executor` / `CLOACINA_DEFAULT_EXECUTOR` (default `default`).
`lib.rs`: deleted `build_routing_config`; `run()` param `fleet_routes` →
`default_executor: String` (moved the misplaced `#[allow(too_many_arguments)]`
onto `run`); `runner_config_for_tenant_cache(reconcile, &str)`; `use_fleet =
default_executor=="fleet"` gates BOTH the per-tenant fleet registrar and the
global fleet registration; added boot-time hard-match — bail listing available
keys if `default_executor` isn't registered. Fixed two test call sites
`(None, "default")`.

**cloacinactl:** added `[server].default_executor` (`ServerSection`, default
`"default"`) to `CloacinaConfig` + `resolve_default_executor(cli, path)` mirroring
`resolve_database_url`; `server start` resolves (CLI/env > config.toml > default)
and forwards `--default-executor`. Added precedence unit test.

**Tests/docs/charts:** e2e + soak fleet harnesses → `CLOACINA_DEFAULT_EXECUTOR=fleet`;
`task_helm.py` server values → `CLOACINA_DEFAULT_EXECUTOR: fleet`. Deleted
`custom-task-routing.md` + nav entry; rewrote deploy-fleet how-to, dispatcher-
architecture, task-execution-sequence, architecture-overview, horizontal-scaling,
execution-agent-fleet, configuration, cli, environment-variables to the single-knob
model. Agent chart NOTES.txt + README reframed. Generated trees (`docs/public/`,
`docs/content/api-reference/`, `.metis/code-index.md`) left to regenerate from
source. Historical Metis/`review/` records left as point-in-time.

### 2026-06-08 — Integration test added

New e2e scenario `angreal test e2e default-executor`
(`.angreal/test/e2e/cli.py`): boots the real `cloacina-server` against the
containerized Postgres and asserts (1) **negative** — `--default-executor nope`
exits non-zero with "not a registered executor" (boot-time hard-match), and
(2) **positive** — `--default-executor fleet` boots and serves `/health` (proves
`use_fleet` registers the fleet executor so validation passes; no agents needed).
The `default`/thread boot path is already covered by the `cli` scenario (boots
with no flag). Registered in `angreal tree` ✓.

### 2026-06-08 — Docs audited (not just keyword-swapped)

Read the rewritten authored docs end-to-end and confirmed they reflect the new
pattern accurately + consistently (single default-executor knob, config.toml
`[server].default_executor` preferred surface, precedence CLI/env > config.toml >
`default`, boot-time hard-match w/ no silent fallback, fleet opt-in, "why no
per-task routing" rationale). Fixes applied on top of the subagent pass:
- `dispatcher-architecture.md`: error-variant name `NoExecutor` → `ExecutorNotFound`
  (the real variant), tied to the hard-match note.
- `execution-agent-fleet.md` front-matter + `architecture-overview.md` See-Also:
  dropped lingering "routing" wording.
Verified: zero authored-content references to the old surface (only the GENERATED
`api-reference/rust/` rustdoc tree still shows `Router`/`RoutingConfig` — regen
from source clears it); no dangling links to the deleted `custom-task-routing.md`;
`_index.md` Configuration section still non-empty.

### 2026-06-08 — Committed + PR open (task stays ACTIVE until merge)

Branch `feat/T-0640-default-executor`, commit `850a7250`. **PR #121**:
https://github.com/colliery-io/cloacina/pull/121 (base `main`, squash-merge).
Pre-commit `Cargo check (both backends)` passed at commit time. Generated
`.metis/code-index*` + `.index-dirty` churn deliberately excluded from the commit
(regenerates via `metis index`). Per user direction: leave this task **active**
until the PR merges.

### 2026-06-09 — CI: flaky timeout on PR #121 (not the change), reran

PR #121 still OPEN. 50+ checks green (postgres build/test, unit both OSes, all
integration incl. sqlite-ubuntu, macros, tutorials/examples, docs, helm e2e).
TWO reds, same root cause: `test_scenario_31_task_handle.py` hit the 180s pytest
timeout and was killed (`Feature Build (sqlite-only)` + `Integration Tests
(sqlite, macos-14)`). That's the Python `TaskHandle` async scenario — NOT touched
by T-0640 (`git show --stat 850a7250` has no task_handle/scenario_31); dispatcher
logged `Registered executor default ThreadTaskExecutor` normally. Flaky hang, not
a regression. Reran failed jobs of run 27177461193; both IN_PROGRESS. If they fail
again on the same scenario it's a pre-existing flake to file separately, not a
T-0640 blocker.

### 2026-06-09 — MERGED + closed out

PR #121 squash-merged to `main` (squash commit `f742b3d`, 2026-06-09 09:41 UTC);
feature branch deleted. CI was fully green at merge (the earlier 2 reds were the
`test_scenario_31_task_handle.py` 180s flake, which cleared on rerun). All six
acceptance criteria met → checked.

On-merge cleanup done:
- Pulled `main`; verified router.rs + custom-task-routing.md are gone.
- Regenerated `.metis/code-index.md` via `metis index` (full reindex: 518 files,
  8412 symbols) — 0 references to the deleted `Router`/`RoutingConfig`/`RoutingRule`
  remain; new `default_executor`/`has_executor` symbols present.
- Authored docs confirmed clean of the old surface.

Remaining (handed to user — heavy build, not run in-tool): regenerate the rustdoc
**api-reference** + Hugo site via `angreal docs`. 5 generated files under
`docs/content/api-reference/rust/cloacina/dispatcher|runner` still show the deleted
`Router`/`RoutingConfig`/`RoutingRule` until that runs. These are generated-from-
source artifacts; running `angreal docs` and committing the regenerated tree (with
the code-index churn) is the only follow-up. Task complete.
