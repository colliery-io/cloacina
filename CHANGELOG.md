# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.10.0] - UNRELEASED

### Added

- **Secrets — encrypted named-field objects** (CLOACI-I-0133) — the encrypted sibling of parameters. A tenant-scoped `Secret` is a named object of named fields (`db_prod = { host, user, password }`), encrypted at rest under a per-tenant data key. A workflow declares required secrets (`#[workflow(secrets(...))]` / `@cloaca.workflow_secrets`) and reads them at run time via `context.secret(name)` / `secret_field(name, field)` (Rust **and** Python); an instance param binds one with the `{"$secret":"name"}` reference form. The resolved value **never** enters the durable `Context`, `schedules.params`, or the fires log (a leak test gates it). On the execution-agent fleet, secrets are delivered per-execution via RFC 9180 HPKE envelope wrap to a one-time agent key (true per-execution forward secrecy). Managed with `cloacinactl secret create/rotate/list/delete` and the embedded-UI Secrets view; the server reads its key encryption key from `CLOACINA_SECRET_KEK`. Authorization is by tenant scope. See [Secrets]({{< ref "/service/explanation/secrets" >}}).
- **Compiler build sandbox** (CLOACI-I-0105) — `cargo build` runs attacker-controlled `build.rs` / proc-macros, so each build now runs under a fail-closed isolation ladder selected by `CLOACINA_COMPILER_SANDBOX = required|preferred|off`: bubblewrap namespaces (no network, cleared env, RO toolchain) or a landlock fallback. `required` refuses to boot when it can't sandbox. Templated into the Helm chart (`compiler.enabled`, `seccompProfile: Unconfined`). See [Compiler Build Sandbox]({{< ref "/service/compiler-sandbox" >}}).
- **Embedded web UI** (CLOACI-I-0130) — a single `cloacina-server` binary now serves the engine, API, and the web UI (the `embedded-ui` feature; SPA embedded via `rust-embed`, same-origin by default). One container, no separate UI service.
- **Constructors — reusable polyglot configured-instance factories** (CLOACI-I-0132) — author a WASM operator (task / trigger / accumulator / reactor) once, distribute it as a signed, independently-versioned **provider package** (rides Cargo's dependency model, ADR A-0010/A-0011), and instantiate it with bound config via `constructor!(from = "name@version", …)` (Rust and Python, embedded and packaged, including on the fleet). Capabilities/egress are tenant-granted at construction time, default-closed (ADR A-0009).
- **Parameterized workflow instances** (CLOACI-I-0116) — register named, scheduled, param-bound instances of a workflow; the instance's declared params are snapshot at registration (immutable) and merged into each fire's context (reserved scheduler keys win).
- **Computation graphs on the execution-agent fleet** (CLOACI-T-0722 / T-0841) — a whole-graph firing can dispatch to the fleet (Rust and Python), not just run in-process.
- **Graph operational instrumentation & health API** (CLOACI-I-0117 / I-0131) — accumulator buffer/fill + events-rate, reactor fires log + per-minute cadence, surfaced in the health API and the graph view.
- **Kubernetes-first execution-agent fleet** (CLOACI-I-0127) — UI-driven agent/tenant assignment, a pluggable `FleetActuator` (K8s + Docker), per-tenant autoscaling within limits, and K8s production-hardening (agent pod securityContext/probes, per-tenant NetworkPolicy, server PDB + anti-affinity).

### Changed (breaking)

- **Authoring-surface minimization** (CLOACI-I-0125) — a Rust package is now a 4-dependency shell (`cloacina-workflow` + `cloacina-workflow-plugin` + serde) with `cloacina_workflow_plugin::package!()`; the compiler injects the cdylib crate-type + `packaged` feature. **No more `build.rs`, `[lib] crate-type`, `[features]`, or `cloacina-build` in a package.** `package.toml` is minimized (name + version + `workflow_name`; the rest is defaulted/inferred). **Migration:** re-scaffold with `cloacinactl package new` or delete the retired ceremony.
- **`ReactionMode` / `ReactionCriteria` collapsed** (CLOACI-T-0740) — the two enums merged; `criteria = when_any` with no accumulator list now means "all declared". Reactor declarations built against the split enums must update.
- **Standalone Nginx UI container retired** (CLOACI-I-0130) — the web UI is served by the server (embedded). The separate `ui/Dockerfile` + `docker-compose.ui.yml` path is gone; deploy the embedded UI (or the `charts/cloacina-ui` chart for a standalone SPA).

### Changed

- **Single source of version truth** (CLOACI-I-0134) — the whole workspace version now lives in the root `Cargo.toml` `[workspace.package]` + `[workspace.dependencies]` (crates inherit via `{ workspace = true }`); `angreal release bump <version>` sets every core touchpoint (Rust / npm / python / scaffold) from one input, and a `version-lockstep` pre-commit hook fails CI on any drift. Providers version independently (ADR A-0010).

### Fixed

- Scaffold dependency pin (`cloacinactl package new`) tracked `cloacina-workflow = "0.7"`, three minors stale — generated packages couldn't resolve against the current toolchain. Now tracks the release (CLOACI-T-0869 / I-0134).
- Web UI displayed a hardcoded, drifted version (shell showed `v0.8.0` while the release was 0.9.0); the version is now injected from `package.json` at build time (CLOACI-I-0134).

## [0.9.0] - 2026-06-24

### Added

- **Server authentication & authorization model** (CLOACI-I-0118) — a complete identity + authorization layer for `cloacina-server`, **server-only** (the embedded library and daemon are untouched). It comprises:
  - **ABAC route-table authorization** — every `/v1/*` route is classified once at startup into a declarative `(method, path) → {scope, level}` table, and a small total matcher evaluates each authenticated request. The layer is **fail-closed**: a route absent from the table is denied, never allowed by default. This replaces the per-handler `can_access_tenant` / `can_write` / `can_admin` checks.
  - **Local accounts (self-managed, no IdP)** — username/password accounts a tenant-admin provisions under `/v1/tenants/{t}/accounts` (passwords hashed with argon2id, unique per `(tenant, username)`). `POST /v1/auth/local/login` mints a short-TTL, tenant-scoped bearer key.
  - **OIDC single sign-on** — configure an issuer with `CLOACINA_OIDC_*` and `/v1/auth/oidc/login` runs the authorization-code + PKCE flow (spec-compliant discovery, JWKS, ID-token validation via the `openidconnect` crate). A **god-owned allowlist** (`CLOACINA_OIDC_MAP`: group / email-domain / subject → tenant+role) maps the validated identity to tenant memberships; an unmapped identity is denied.
  - **Multi-tenant single sign-on** — one OIDC login resolves to the *set* of `{tenant, role}` memberships the allowlist grants, minting a scoped key per tenant; the web UI shows a tenant picker. The bearer key always stays scoped to one tenant — multi-tenancy is "hold N scoped keys + switch", never a multi-tenant subject.
  - **Session lifecycle** — minted keys carry a TTL and `issued_via` provenance. `POST /v1/auth/refresh` re-validates and re-mints (the UI refreshes silently before expiry); `POST /v1/auth/logout` revokes the key and forgets the server-side session.
  - **Web UI** — username/password and SSO sign-in on the connect screen, a tenant-admin account-management view, and the tenant switcher for multi-tenant operators. Controls are **role-gated**: write/admin actions (run, upload, pause, fire/inject, key + account management) are hidden for read-only keys, resolved via a new `GET /v1/auth/whoami`.
  - See [Security Model]({{< ref "/service/explanation/security-model" >}}), [Configure Local Accounts]({{< ref "/service/how-to/configure-local-accounts" >}}), and [Configure OIDC Single Sign-On]({{< ref "/service/how-to/configure-oidc-sso" >}}).
- **Declared, typed workflow inputs** (CLOACI-I-0128) — a `#[workflow(params(name: Type [= default], …))]` clause (and `@cloaca.workflow_params(...)` in Python) declares typed inputs; their JSON-Schema `InputSlot`s surface on `WorkflowDetail.declared_params` and are validated at execute time. See [Declare Workflow Inputs]({{< ref "/embed/how-to/declare-workflow-inputs" >}}).
- **Opinionated task documentation** (CLOACI-I-0126) — a `what:` / `why:` convention in `#[task]` doc-comments (and Python docstrings) is parsed by the compiler into the package manifest and surfaced on `WorkflowTaskNode`.
- **Workflow source retrieval** — `GET /v1/tenants/{t}/workflows/{name}/source` returns the retained package's source files (read-only, language-tagged, tenant-scoped). (CLOACI-T-0750)
- **Operator inject / fire surfaces** (CLOACI-I-0128) — manually drive graph surfaces with typed JSON: reactor `force_fire` / `fire_with` (`POST /v1/health/reactors/{name}/fire`), accumulator inject (`POST /v1/health/accumulators/{name}/inject`), and typed-slot discovery (`GET /v1/health/{reactors,accumulators}/{name}/interface`), plus `cloacinactl reactor fire/force-fire` and `accumulator inject`. Every manual fire/inject is audit-logged.
- **Manual trigger fire** — `POST /v1/tenants/{t}/triggers/{name}/fire` pushes a typed event and fans out to every subscribed workflow; `GET /v1/tenants/{t}/triggers/{name}/interface` returns the trigger's typed event surface — the *union of its subscribed workflows' declared params* (there is no per-trigger `params(...)` declaration). (CLOACI-T-0777)
- **Universal pause / resume** — pause and resume triggers and workflows; paused state is surfaced on summary/detail responses and excluded from all fire paths. (CLOACI-T-0749)
- **Multi-architecture execution-agent fleet** (CLOACI-T-0780) — a package can carry per-target cdylibs (`package_artifacts`), and dispatch hands each agent the artifact matching its `target_triple`, so heterogeneous fleets (e.g. aarch64 + x86) are supported. Interpreted (Python) packages run on any agent architecture.
- **Per-tenant operational metrics & build isolation** (CLOACI-T-0779) — the ops-metrics stream is tenant-scoped (each connected view sees its own tenant's build queue / reconciler / fleet / graph health), and per-tenant compilers isolate builds.
- **Aurora Dark web-UI redesign** (CLOACI-I-0129 / I-0131) — operational graph instrumentation, manual-intervention indicators on manual runs, and a graph-node code inspector.
- **SDK coverage gate** (CLOACI-T-0772) — all OpenAPI operations (fire / inject / pause / resume / interface / source, …) are reachable from the TypeScript, Rust, and Python clients.

### Changed (breaking)

- **Plugin ABI `CloacinaPlugin` v2 → v3** (CLOACI-I-0128) — adds the optional FFI method `get_input_interface`. Packages built against the old ABI still load (the host treats it as an empty interface), but **a package must be recompiled against 0.9.0 to declare params or expose typed interfaces**.
- **Authorization moved to the fail-closed ABAC route table** (CLOACI-I-0118), which **fixes a cross-tenant key-management leak**: the global `GET/POST/DELETE /v1/auth/keys` surface is now platform-scoped (god-only). A tenant-admin key can no longer enumerate or mint keys outside its tenant — it manages its own keys via `/v1/tenants/{t}/keys`. A tenant-scoped key calling the global surface now returns `403`. **Operator action:** if any tooling used a tenant key against the global `/auth/keys` surface, repoint it at the tenant-scoped surface.
- **Trigger fan-out is now a single named fan-out point** (CLOACI-T-0777 / T-0778) — a trigger fires *all* workflows subscribed via `#[workflow(triggers=[…])]`, across packages, on both manual and auto-poll firing (previously only the trigger's primary `on` workflow ran). Fan-out is resolved from registry workflow metadata, not the schedules table, so cross-package subscribers that were silently dropped now run. Secondary fan-out is best-effort (a secondary failure is logged, never fails the primary); cron schedules still bind exactly one workflow.
- **Workflows with declared params are validated at execute time** — a missing required param or a top-level type mismatch is rejected with `400 workflow_input_invalid` before the run starts. Workflows that declare no params keep free-form, unvalidated context (non-breaking for them).
- **Executing a paused workflow is refused** with `409 workflow_paused`.

### Security

- **Cross-tenant key-management leak fixed** (CLOACI-I-0118) — see *Changed (breaking)*.
- Local-account passwords are hashed with **argon2id**; OIDC refresh-token material (when present) is stored **server-side encrypted (AES-256-GCM)** and never returned to the browser; OIDC in-flight login state is **Postgres-backed** (multi-replica safe, no sticky sessions).

### Fixed

- **Tenant execution isolation** (CLOACI-T-0781 / T-0779) — tenant tasks are namespaced `tenant::pkg::wf::task` (previously hardcoded `public::…`), so they route to the tenant's own agents and fetch their cdylib from the tenant schema. Tenant runs previously hung.
- **PyO3 ↔ tokio GIL deadlock** (CLOACI-T-0622) — computation-graph invocation, `post_invocation`, `on_failure`, and `on_success` now run off the async worker via `spawn_blocking`, fixing the deadlock that hung scenario_30 / scenario_32.
- **Macros: typed terminal outputs restored** in the reactor computation-graph path (a T-0775 regression had pushed only the JSON copy, breaking in-process accumulator/reactor consumers that downcast to the concrete type); the task-embedded computation-graph match arm tolerates the new `GraphResult::Completed { outputs_json, .. }` field.
- **Web UI** — skipped DAG nodes render distinctly; trigger-gated task source displays correctly; the ops-metrics stream is an app-level warm connection (no cold-start "connecting…" flash on navigation).

### Migrations

- New Postgres-only auth tables (`api_keys` TTL/provenance columns, `oidc_sessions`, `local_accounts`, `oidc_login_flows`; migrations 032–035) and `package_artifacts` (Postgres 031 / SQLite 027) for multi-arch fleets, plus `schedules.paused` / `workflow_packages.paused` (Postgres 029 / SQLite 025). All applied automatically on server boot.

## [0.8.0] - 2026-06-18

> Backfilled — 0.8.0 shipped without a CHANGELOG entry. Headline items below; see the git history (`v0.7.0..v0.8.0`) for the full set.

### Added

- **Server SDKs** (CLOACI-I-0113) — a published OpenAPI contract plus generated Rust, Python, and TypeScript clients for the `cloacina-server` HTTP API.
- **Web UI control plane** (CLOACI-I-0117) — the operator web interface for the server (workflows, executions, triggers, graphs, keys), with subsequent UI-polish passes (task Gantt + runtime distributions, agent-fleet views, demo overhaul).
- **Packaged-workflow authoring DX** (CLOACI-I-0119) — ergonomics improvements for authoring and packaging workflows.
- **Timer-driven cron scheduling** (CLOACI-T-0743) — sleep-until-next-due scheduling with change-notify, replacing tight polling.
- **Python authoring parity** (CLOACI-T-0688) — state-accumulator and cron-trigger authoring in `cloaca`.

### Changed

- **Documentation overhaul** (CLOACI-I-0120) — audience-first restructure into two co-equal entry points (embedded library and server), a `/engine` primitives section, and dual-language tutorials.

### Fixed

- Closed SDK endpoint-coverage gaps blocking the release; migrated remaining `cloacinactl` list commands to the items-envelope response shape (CLOACI-T-0681).

## [0.7.0] - 2026-06-09

### Added

- **Execution-agent fleet** (CLOACI-I-0114 / CLOACI-I-0115) — a horizontally scalable, database-less execution tier. Remote `cloacina-agent` workers pull ready work from the server, run it, and report results back over HTTP, so task execution scales out independently of the scheduler and without granting workers direct database access. Ships with a `cloacina-agent` Helm chart (server + compiler + N agents), in-flight reclaim on agent loss, and a fleet soak harness (roster/outbox/drift). See [Execution-Agent Fleet](https://cloacina.colliery.io/platform/explanation/execution-agent-fleet/) and [Deploy an Execution-Agent Fleet](https://cloacina.colliery.io/platform/how-to-guides/deploy-an-execution-agent-fleet/).
- **Server-level default executor** (CLOACI-T-0640) — execution topology is now a single deployment knob (Airflow `[core] executor` style). The preferred surface is `[server].default_executor` in `~/.cloacina/config.toml`; override it ad-hoc with `--default-executor` or the `CLOACINA_DEFAULT_EXECUTOR` env var (precedence: explicit CLI/env > `config.toml` > built-in `default`). Set it to `fleet` to route all work to the execution-agent fleet. The configured key is **hard-matched** against the registered executors at startup — an unknown key fails fast with an error listing the valid keys, never a silent fallback.

### Security

- **`cloacina-compiler` Phase 1 hardening** (CLOACI-I-0104) — bounded-cost mitigations for malicious `build.rs` source. `cargo build` now runs with `--frozen --offline` by default against an operator-curated `CARGO_HOME` (`--vendor-dir` / `CLOACINA_COMPILER_VENDOR_DIR`); packages whose deps aren't vendored fail fast with a structured rejection naming the missing crates. Builds are bounded by a wall-clock timeout (`--build-timeout-s`, default 600s) — overruns are SIGKILL'd and reclaimed by the existing stale-build sweeper. On Linux, four kernel-enforced rlimits are applied to the cargo subprocess via `setrlimit` in a `pre_exec` hook: CPU-seconds (tracks `--build-timeout-s`), virtual address space (`--build-rlimit-mem`, default `4G`, accepts `K`/`M`/`G` suffixes), open file descriptors (`--build-rlimit-files`, default 1024), and user processes (`--build-rlimit-procs`, default 256). Every build emits a `compiler.build.started` and `compiler.build.finished` structured audit event via `tracing`, including build-claim id, `Cargo.toml` / `Cargo.lock` SHA-256, outcome, cargo exit code/signal, wall-clock duration, and a process-wide `compiler_instance_id`. See [Run cloacina-compiler in Production](https://cloacina.colliery.io/platform/how-to-guides/running-the-compiler/) for the deployment runbook. Phase 2 (CLOACI-I-0105) adds a bubblewrap + landlock sandbox.

  **Operator action required** for existing deployments:

  - Run `cargo vendor` against your known-good source tree and point `--vendor-dir` at the result, or pre-populate `~/.cargo` on the compiler host with the deps your authors submit. Packages that previously succeeded by fetching deps online will now fail until vendored.
  - Verify the compiler runs under an unprivileged UID with a database role limited to `workflow_packages` (`SELECT`/`UPDATE`) — a shared admin DB role would be a privilege-escalation path for any malicious `build.rs`.
  - Tune `--build-rlimit-mem` if your release builds peak above 4 GiB (large generic-heavy crates can).

### Changed (breaking)

- **Computation graphs and reactors are now declared separately** (CLOACI-I-0101) — the bundled `#[computation_graph(react = when_any(...), graph = { ... })]` form is removed. A reactor is now its own top-level primitive declared with `#[reactor(...)]`, and computation graphs reference it by string name via `trigger = reactor("name")`. Trigger-less graphs (no `trigger =` clause) are first-class and can be invoked from a workflow task with `#[task(invokes = computation_graph("name"))]`. Multiple computation graphs may now subscribe to the same reactor. Python mirrors the split: `@cloaca.reactor(...)` declares the reactor class; `ComputationGraphBuilder(..., reactor=ReactorClass, ...)` binds the graph; `@cloaca.task(invokes=graph_builder)` wraps a trigger-less graph as a workflow task. No deprecation window; rewrite required.

  **Before (Rust):**

  ```rust
  #[cloacina_macros::computation_graph(
      react = when_any(orderbook),
      graph = { ingest(orderbook) -> output },
  )]
  pub mod pricing_pipeline { /* ... */ }
  ```

  **After (Rust):**

  ```rust
  #[cloacina_macros::reactor(
      name = "pricing_pipeline_reactor",
      accumulators = [orderbook],
      criteria = when_any(orderbook),
  )]
  pub struct PricingPipelineReactor;

  #[cloacina_macros::computation_graph(
      trigger = reactor("pricing_pipeline_reactor"),
      graph = { ingest(orderbook) -> output },
  )]
  pub mod pricing_pipeline { /* ... */ }
  ```

  **Before (Python):**

  ```python
  with cloaca.ComputationGraphBuilder(
      "pricing_pipeline",
      react={"mode": "when_any", "accumulators": ["orderbook"]},
      graph={...},
  ) as builder:
      ...
  ```

  **After (Python):**

  ```python
  @cloaca.reactor(
      name="pricing_pipeline_reactor",
      accumulators=["orderbook"],
      mode="when_any",
  )
  class PricingPipelineReactor:
      pass

  with cloaca.ComputationGraphBuilder(
      "pricing_pipeline",
      reactor=PricingPipelineReactor,
      graph={...},
  ) as builder:
      ...
  ```

  See [CLOACI-S-0011](https://github.com/colliery-io/cloacina/blob/main/.metis/specs/CLOACI-S-0011.md) for the primitive nomenclature and the [Computation Graph in a Workflow Task](https://cloacina.colliery.io/computation-graphs/how-to-guides/computation-graph-in-workflow/) how-to for the new embedded-CG pattern.

- **Glob-based task routing removed** (CLOACI-T-0640) — the per-task routing surface is gone. `Router`, `RoutingConfig`, and `RoutingRule` are removed from the public prelude (`cloacina::dispatcher`), and `cloacina-server` no longer accepts `--route` / `CLOACINA_FLEET_ROUTES`. The dispatcher now sends every task to the single configured default executor (see **Added → Server-level default executor**); choosing which node or compute a task lands on is an executor-internal concern, not a scheduler/dispatcher one. **Migration:** replace any `--route "**=fleet"` / `CLOACINA_FLEET_ROUTES` usage with `--default-executor fleet` (or `[server].default_executor = "fleet"`); library consumers that referenced `RoutingConfig`/`RoutingRule`/`Router` should remove them and configure the default executor via `DefaultRunnerConfig::default_executor`.

## [0.6.1] - 2026-05-09

### Fixed

- **Cron `schedule_executions` never marked complete → cron_recovery infinite loop** (CLOACI-T-0572) — `process_cron_schedule` created the audit row with `started_at` populated but never called `.complete()`. `cron_recovery::find_lost_executions` selects rows where `completed_at IS NULL` and `started_at < threshold`, so every successful firing was rescheduled on every recovery tick. Workloads with `cron_enable_recovery(true)` (the default) saw ~37x execution amplification (e.g. 906 firings in 6 hours instead of 24). Both the success and failure branches now call `schedule_execution().complete(audit_record_id, Utc::now())`, mirroring the existing trigger-failure pattern. Workaround for older versions: `.cron_enable_recovery(false)` on the runner config.

## [0.6.0] - 2026-05-07

### Added

- **Unified `cloacina::package!()` plugin shell** (CLOACI-I-0102) — single fidius plugin per cdylib emitted by one shell macro for any combination of declared primitives (tasks, workflows, reactors, triggers, computation graphs). Replaces per-macro plugin emission; primitives self-declare via `#[workflow]`, `#[reactor]`, `#[trigger]`, `#[computation_graph]` etc.
- **Trigger FFI bridge for packaged cdylibs** — workflow→trigger subscriptions cross the ABI; cron registration runs through the reconciler at load/unload, closing the gap where packaged cron triggers never fired under `cloacina-server`.
- **Trigger-less computation graphs over FFI** — packaged cdylibs can declare CGs that bind to externally-owned reactors (cross-package fan-out) and run through the unified pipeline.
- **Server-side opt-in signature verification** (CLOACI-I-0103) — `cloacina-server --require-signatures --verification-org-id <UUID>` (or `CLOACINA_VERIFICATION_ORG_ID` env) gates uploads against `package_signatures` matched to a trusted org. Misconfiguration is rejected at boot, not at first upload. Verified + rejected uploads emit structured audit events via `audit::log_package_load_*`. New ADR-0005 codifies the deployment-mode trust model: daemon = high-trust hobbyist, server = enterprise multi-tenant, server is Linux-only.
- **`org_id` column on `package_signatures`** (T-0566) — additive migrations on Postgres + SQLite, prerequisite for scoped signature trust.
- **Reverse-order package unload pipeline** (T-0554 Phase 2) — Python paths route through the unified pipeline; subscribers are unbound before owners are torn down.
- **fidius 0.2.0** across the workspace (T-0546) — adopts `#[optional(since)]` for ABI evolvability.

### Changed

- **Per-macro plugin emission stripped** (T-0549) — the unified shell macro is now the only path. In-tree packaged crates migrated.
- **Manifest cleanup** (T-0551) — `[[triggers]]` and `package_type` removed from `package.toml`. `#[serde(deny_unknown_fields)]` on `CloacinaMetadata` produces friendly migration hints.
- **Reconciler `load_package` precedence-pipeline restructure** (T-0554) — single canonical lifecycle for tasks, workflows, triggers, reactors, computation graphs.
- **Documentation sweep** — tutorials, how-tos, and reference brought in line with the unified package shell; CLI and HTTP API reference rewritten; post-I-0096 ctor references refreshed; I-0102 / pipeline / vtable docs authored.
- **Internal API surface narrowed** (T-0565) — selected `pub` → `pub(crate)` to reduce accidental cross-crate coupling.

### Fixed

- **Server signature verification at upload** (T-0557 Bug 2) — verifies via `package_signer` against the configured trusted org; structured 403 codes (`package_tampered`, `untrusted_signer`, `invalid_signature`, `signature_not_found`).
- **API key escalation guard** (T-0557 Bug 3) — non-admins can no longer create admin keys.
- **Workflow `build_status` accuracy** (T-0557 Bug 4) — package status now reflects compile outcome, not just upload acceptance.
- **Server tests + startup banner + banned-phrase scrubbing** (T-0557 Bugs 1, 6, 7).
- **Python TriggerResult API unified** (T-0557 Bug 5).
- **Reconciler reactor-unload arm + canonical method-index constants** (T-0564).
- **Build/test config drift** (T-0561) — workspace lints, feature flag flags, and angreal harness reorganization aligned.
- **Stale comments + dead daemon manifest plumbing** (T-0562) — runtime strings and code references brought current with closed work.

### Removed

- **Dead code uncovered by post-I-0102 audit** (T-0555, T-0556, T-0563) — orphan modules, dead branches in reconciler `load_package` and scheduler, dead back-compat shims, register_package_triggers shim, unused `PackageLoadView.tasks`, six `todo!()` placeholder signing tests (T-0569; multi-org SaaS scenarios are deferred per ADR-0005 — re-introduce when CLOACI-I-0106 ships).

### Migration notes

- **`package_signatures.org_id`** is added as a nullable column on both backends. Existing rows have `org_id IS NULL` and will not pass verification once `--require-signatures` is enabled — operators upgrading to the verification gate must re-sign packages with `org_id` populated.
- **`package_type` and `[[triggers]]` removed from manifest** — packages still using either fail to load with a friendly migration hint pointing at the unified macro shell.

## [0.5.1] - 2026-04-14

### Changed

- **Pipeline-to-workflow terminology migration** — complete rename across code, DB schema (Diesel migrations for Postgres + SQLite), DAL, models, error messages, metrics, and all tests. `pipeline_executions` table renamed to `workflow_executions`; all `pipeline_*` columns and fields renamed to `workflow_*`. `ExecutionEventType` variants renamed (`PipelineStarted` → `WorkflowStarted`, etc.) with backward-compatible `from_str` parsing.
- **Config builder returns Result** — `DefaultRunnerConfigBuilder::build()` now returns `Result<DefaultRunnerConfig, ConfigError>` with validation for `scheduler_poll_interval >= 10ms` and `cron_max_catchup_executions <= 1000`. Default `cron_max_catchup_executions` capped at 100.
- **Python runner DRY refactor** — 40% reduction in `runner.rs` via extracted `run_event_loop()`, `spawn_runtime()`, `send_and_recv()`, and dict conversion helpers. Fixed `with_schema()` double-construction bug.

### Added

- **Daemon health observability** — Unix domain socket (`daemon.sock`) serves JSON health on connect. Structured log pulse every 60s. `cloacinactl status` command for querying daemon health.
- **Claim ownership guard** — `mark_completed`/`mark_failed` now check `claimed_by` column before updating, preventing race conditions between concurrent runners.
- **TOML config validation** — `deny_unknown_fields` on `CloacinaConfig`, `DaemonSection`, `WatchSection` for early typo detection.

### Fixed

- Integration test `.build()` calls updated for `Result` return type.

## [0.5.0] - 2026-04-10

### Added

- **Computation Graph System** — reactive, event-driven data processing primitive alongside the existing workflow system:
  - `#[computation_graph]` proc macro with compile-time topology validation, cycle detection, and code generation
  - Accumulator trait and built-in types: passthrough, polling, batch, stream (Kafka), and state accumulators
  - Reactor with `WhenAny`/`WhenAll` reaction criteria and `Latest`/`Sequential` input strategies
  - Reactive Scheduler for spawning, supervising, and restarting accumulator/reactor task trees
  - Checkpoint-based crash recovery for accumulators and reactor input cache via DAL
  - Health state machines: `Starting` → `Warming` → `Live` → `Degraded` for both accumulators and reactors
  - Supervisor with exponential backoff (max 5 retries, 1-60s backoff, 60s success reset)
  - Reactor manual commands: `ForceFire`, `FireWith`, `Pause`, `Resume`, `GetState`
- **WebSocket integration** for computation graphs:
  - Accumulator endpoints for pushing events from external producers
  - Reactor endpoints for manual commands and state queries
  - Single-use ticket authentication (`POST /auth/ws-ticket`)
  - Per-endpoint authorization policies scoped to tenant
- **Computation graph packaging** for both Rust and Python:
  - `cdylib` shared library packages with FFI plugin interface via fidius
  - Python computation graph loading via `import_python_computation_graph`
  - Reconciler routing: detects `has_computation_graph()` and routes to reactive scheduler
  - `package.toml` metadata for graph declarations, accumulator config, and reaction mode
- **Kafka stream backend** — `StreamBackend` trait with `KafkaStreamBackend` implementation (KRaft mode, no ZooKeeper)
- **Python computation graph bindings** — `@node`, `@passthrough_accumulator`, `@stream_accumulator`, `@polling_accumulator`, `@batch_accumulator` decorators and `ComputationGraphBuilder`
- **Variable registry** — `CLOACINA_VAR_{NAME}` environment variable convention with `var()`, `var_or()`, and `resolve_template()` for runtime configuration
- **Routing graphs in soak tests** — market maker scenario with enum dispatch routing
- **7 new documentation pages** following Diataxis framework:
  - Tutorial: Python packaged triggers
  - Reference: package manifest schema
  - How-to guides: packaging Python workflows, custom task routing, migrating library to service mode, variable registry
  - Explanation: reactive scheduling architecture
- **REST health endpoints** for computation graphs: `/v1/health/accumulators`, `/v1/health/reactors`, `/v1/health/reactors/{name}`

### Changed

- Documentation site restructured by feature area (workflows, computation graphs, Python, platform)
- Reconciler now routes Python packages through workflow or computation graph paths based on package metadata

### Fixed

- Quick start guide referenced stale version number (0.1.0 → 0.5.0)
- Stale API references in examples and tutorials updated
- CI: shared build cache, libpq-dev installation, retry logic for flaky tutorial tests
- Release pipeline: Python wheel build restored, crate publish ordering fixed, macOS x86_64 wheel dropped

## [0.4.0] - 2026-03-15

Initial public release with workflow orchestration, cron scheduling, multi-tenancy, packaging, Python bindings, and HTTP API server.
