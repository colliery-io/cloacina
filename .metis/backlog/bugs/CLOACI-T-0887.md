---
id: bug-server-compiler-build-fails
level: task
title: "BUG: server compiler build fails — cloacina-computation-graph missing from staged crate set (packaged builds broken)"
short_code: "CLOACI-T-0887"
created_at: 2026-07-10T09:11:26.062829+00:00
updated_at: 2026-07-10T09:11:26.062829+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


exit_criteria_met: false
initiative_id: NULL
---

# BUG: server compiler build fails — cloacina-computation-graph missing from staged crate set (packaged builds broken)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Server-side compilation of a packaged workflow that depends on `cloacina-computation-graph` fails — surfaced by the first real gold-path E2E (I-0138/T-0884): pack `simple-packaged` → upload to the demo server → compiler build fails.

**Server audit event:**
```
compiler.build.finished outcome=failed exit_status=101 sandbox_level=landlock
package_name=simple-packaged-demo package_version=0.1.0
failure_reason="cargo build failed: failed to load manifest for dependency
  `cloacina-computation-graph` … failed to read
  `/root/.cloacina/crates/cloacina-computation-graph/Cargo.toml`: No such file (os error 2)"
```

**Two findings (likely both need fixing):**
1. **Compiler crate provisioning gap.** The server compiler resolves the package's cloacina path-deps against `/root/.cloacina/crates/` (the runtime `default_home` = `$HOME/.cloacina`; note the Dockerfile.compiler bakes/sets `CLOACINA_HOME=/workspace/.cloacina` — a HOME mismatch worth checking) and that staged crate set is MISSING `cloacina-computation-graph`. Seeded demo rust fixtures never hit it because none depend on the CG crate; `simple-packaged` is the first that does.
2. **Stale example Cargo.toml.** `examples/features/workflows/simple-packaged/Cargo.toml` is the OLD verbose 6-dep form (cloacina, cloacina-macros, cloacina-computation-graph, cloacina-workflow, cloacina-workflow-plugin, cloacina-build) with `path = "../../../../crates/…"`, NOT the I-0125 minimal 4-dep shell. Part of T-0884 (canonicalizing it) is modernizing this to the minimal shell — which may or may not still need CG.

**Check for T-0865 interaction:** I-0134 converted `cloacina-workflow-plugin`'s `cloacina-computation-graph` dep from explicit `path` to `{ workspace = true }`. If the compiler's crate staging enumerated explicit path-deps, the inheritance could drop CG from the staged set. Verify whether this is a regression vs. a long-standing provisioning gap.

**Impact:** any packaged workflow needing `cloacina-computation-graph` cannot be built by the server → the packaged/server gold path (now the intended e2e/functional-test path, see [[project_packaged_first_examples]]) is broken for CG-using packages. Type=Bug, likely P1.

**Fix location:** `crates/cloacina-compiler/src/build.rs` (path rewrite + crate staging) + `docker/Dockerfile.compiler` (CLOACINA_HOME/crate baking). Verify by re-running the T-0884 gold-path E2E to Completed.

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

### 2026-07-10 (update 6) — VERIFIED GREEN: version-dep gold path passes end-to-end
`angreal test e2e compiler --version-deps` (renamed from `--prod-deps` — maintainer read "prod" as "released crates"; it always compiles the LOCAL dev workspace, offline, crates.io never contacted) → **full pass, exit 0**: happy path (version-dep manifest) build_status=success; workflow_name/tasks/task_graph populated; **reconciler e2e → execution Completed**; failed-build/content-hash/stale-heartbeat/upgrade/rollback/concurrent lanes all green on path-dep fixtures under the same `--dev-workspace` compiler (hatch is a no-op for path-dep packages, as designed). Default (no-flag) lane rerun in progress as the regression check.

Two infra fixes landed en route:
- **Dev stack moved off port 5432 → host 15432** (maintainer decision; `kairos-postgres` — another project, untouchable — holds 5432). `.angreal/docker-compose.yaml` publishes `15432:5432`; every harness db_url, Rust test fallback (server lib.rs, cloacina tests, cloacina-python), `tests/python/conftest.py`, Dockerfile.test, and nightly CI (service mapping + DATABASE_URL + macos setup-postgres) now use `localhost:15432`. In-cluster URLs (`postgres:5432`, helm) and user-facing docs stay 5432. Preflight in `_start_postgres` checks 15432 with a remedy message.
- **Pack-guard fix**: in `--version-deps` mode, residual `__WORKSPACE__` literals in fixture COMMENTS tripped cloacinactl's pack guard; the staging now scrubs them after the dep rewrite.

Remaining to close: default-lane green (running) + commit; the demo-compose `--dev-workspace /workspace` wiring is committed but its in-container run is verified only transitively (same compiler code + dep form proven here).

### 2026-07-10 (update 5) — e2e coverage for the SHIPPING dep form (angreal, not a scratchpad)
Maintainer flagged that the gold-path check shouldn't be a `/tmp` script — the compiler-path e2e IS the right test path; add the non-escape-hatch (production version-dep) form "as a flag." Done, in the existing `angreal test e2e compiler` harness (it already owns the server+compiler+postgres subprocess lifecycle):

- New **`--prod-deps`** flag on `angreal test e2e compiler` (`.angreal/test/e2e/compiler.py`). When set: (1) the happy fixture's Cargo.toml is rewritten from `__WORKSPACE__` path-deps to crates.io **version-deps** on the fly (`_to_version_deps` regex — same workflow code, shipping dep form); (2) the compiler subprocess is started with `--dev-workspace <repo-root>`. Same assertions (build_status=success → workflow run → execution completes) now prove a REAL distributed-package dep form builds + runs. Default run is unchanged (escape hatch); CI runs both.
- Key insight this closes: the escape-hatch (`__WORKSPACE__`) form was tested, the SHIPPING (version-dep) form had ZERO e2e coverage — same "should've seen loud failures" blind spot as I-0137. Now both are covered.
- Verified: transform produces the exact `cloacinactl package new` form; `py_compile` clean; `--prod-deps` flag registers in `angreal test e2e compiler --help`.

**Proof command (rebuilds the compiler from source, no docker):** `angreal test e2e compiler --prod-deps` → expect build_status=success + execution completes. Supersedes the scratchpad demo-stack driver.

### 2026-07-10 (update 4) — IMPLEMENTED: `--dev-workspace` patch-injection hatch (host-validated, image rebuilding)
Chose a lighter mechanism than update-3's vendored-registry: a `[patch.crates-io]` injection, which needs no `cargo vendor`/local-registry generation.

**Compiler change (new `--dev-workspace <root>` flag / `CLOACINA_COMPILER_DEV_WORKSPACE`, `dev_workspace: Option<PathBuf>` on CompilerConfig):**
- `build.rs::inject_dev_patch` — before each build, read `<root>/crates/*/Cargo.toml`, and append a `[patch.crates-io]` table to the unpacked package's Cargo.toml mapping every local crate name → its path. So a package that ships **production crates.io version deps** (`cloacina-workflow = "0.10"`) resolves against the UNPUBLISHED local crates. No-op unless `--dev-workspace` set → production compilers resolve from crates.io untouched.
- `sandbox.rs` — new `BuildMounts.patch_crates_dir` bound **read-only**; bound to the workspace **ROOT** (not `crates/`) because the patched path-crates use `{ workspace = true }` inheritance and cargo walks up to the root `Cargo.toml`. bwrap binds it before the RW target sub-bind (order preserved); landlock unions the RO-root + RW-target rules so `/workspace/target` stays writable. Wired through `wrap_command` (bwrap `--ro-bind`), `apply_landlock` (extra RO PathBeneath), and `config_hash`.
- `main.rs` — clap arg + config wiring.

**Example (T-0884):** re-authored `examples/features/workflows/simple-packaged/Cargo.toml` to the `cloacinactl package new` version-dep form — `cloacina-workflow = { version = "0.10", features=["packaged"] }`, `cloacina-workflow-plugin = "0.10"`, + dev/build cloacina deps as version deps. No more `../../../../crates/`.

**Demo stack:** `docker/docker-compose.demo.yml` — added `--dev-workspace /workspace` to both `compiler` and `compiler-acme` (the repo is already mounted at `/workspace`).

**Host validation (all green):** (1) `cargo check -p cloacina-compiler` → Finished, no errors. (2) Replicated `inject_dev_patch` on a copy of simple-packaged living OUTSIDE the workspace: `cargo generate-lockfile --offline` locked 310 pkgs (workspace-inheritance path-crates resolved); `cargo check --lib --offline` → Finished (cloacina-workflow w/ `packaged`, macros, workflow-plugin, computation-graph all compiled). Proves the full mechanism end-to-end offline.

**Remaining:** rebuild the demo compiler image (in progress) + re-run the gold-path E2E to a **Completed** execution to close this + T-0884.

### 2026-07-10 (update 3) — MAINTAINER STEER: don't prop up the vendor/workspace path; crates.io is the model
Maintainer clarified the architecture: the `__WORKSPACE__`/code-vendor path is a **dev-cycle ESCAPE HATCH**, not production. Production = a package ships deps via **crates.io version deps** (`cloacina-workflow = "0.10"`), which is exactly what `cloacinactl package new` emits. So binding `/workspace/crates` into the sandbox (update-2 idea) is WRONG — it dresses up the hatch as the real model.

**Correct fix (two parts):**
1. **Examples use version deps.** Re-author the canonical example (simple-packaged, T-0884) to the `cloacinactl package new` form (`cloacina-workflow = { version = "…", features=["packaged","macros"] }`, `cloacina-workflow-plugin = "…"`, minimal shell) — NOT path/`__WORKSPACE__` deps.
2. **Dev/demo compiler resolves them via a proper vendor hatch.** Because cloacina isn't published to crates.io yet, the demo/e2e compiler needs a vendored registry of the (unpublished) cloacina crates so version-dep packages resolve OFFLINE under the sandbox — via `--vendor-dir` (already supported: sandbox.rs binds vendor_dir RO). This is the legit dev-cycle use of the hatch: package stays production-shaped (version deps), infra supplies the not-yet-published crates. Requires generating a vendor/local-registry that source-replaces crates.io with the workspace crates (cargo vendor alone won't do unpublished path members — needs a local registry or `[source]`/`[patch]` replacement), mounting it, and pointing `--vendor-dir` at it in docker-compose.demo.yml.

Fix location: `docker/docker-compose.demo.yml` + `Dockerfile.compiler` (build the vendor + `--vendor-dir`), example Cargo.toml (T-0884). Verify: gold-path E2E to a Completed execution. This is real infra (+ compiler image rebuild + re-test), not a one-liner.

### 2026-07-10 (update 2) — REAL root cause: landlock sandbox denies /workspace/crates
Took the E2E further: re-authored simple-packaged to `__WORKSPACE__/crates/`, re-uploaded. It STILL failed — did NOT reach a Completed execution (12 min "compiling…", `data_processing` never registered; `workflow run` → "Workflow not found in registry"). The compiler log on the demo's OWN `packaged-graph-example` fixture shows the true error:
`failed to read /workspace/crates/cloacina-computation-graph/Cargo.toml: Permission denied (os error 13)`.

**REAL ROOT CAUSE:** the compiler builds under `sandbox_level=landlock`, whose curated RO mounts are source_dir + target_dir + vendor_dir + toolchain — but NOT `/workspace/crates/` (the baked cloacina source that `__WORKSPACE__` path-deps resolve to). So ANY package with path-deps into the workspace fails EACCES under the sandbox; and there's no vendor dir for the version-dep form either. This hits the demo's OWN CG-dependent fixtures — pre-existing, NOT a T-0865 regression, NOT the dep form (that was necessary but not sufficient).

**Fix options (real infra work):** (a) bind `/workspace/crates` (or the whole baked workspace) RO into the compiler sandbox for the demo/e2e stack; (b) provide a vendored registry (`cargo vendor` → `--vendor-dir`) so packaged workflows use version deps (the real distributed form) and never need the workspace; (c) run the demo compiler at `sandbox_level=off` (weakest). Likely (b) is the "real distributed package" answer aligned with `cloacinactl package new`. Fix location: `crates/cloacina-compiler/src/sandbox.rs` (mount set) + `docker/docker-compose.demo.yml` / `Dockerfile.compiler` (vendor provisioning). Re-run the T-0884 gold-path E2E to a Completed execution to verify.

### 2026-07-10 — RUN DOWN: not a T-0865 regression; it's a stale-example dep form
Inspected the running compiler container: `/root/.cloacina/crates/` does not exist (only `build-tmp` + `logs`); the crates ARE baked at `/workspace/crates/` (incl. cloacina-computation-graph). So NOTHING is "missing from a staged set" — my original finding #1 was wrong.

**Real cause:** three dep conventions —
- `cloacinactl package new` (canonical, distributed): version deps `cloacina-workflow = { version = "0.10", features=[...] }` (new.rs:323; a unit test asserts NO `path=`/`__WORKSPACE__`).
- Working demo fixtures (demo-fanout-rust): `path = "__WORKSPACE__/crates/…"` — the server REWRITES `__WORKSPACE__` → the abs workspace root at `registry/reconciler/loading.rs:3204`, so these build on the demo server.
- `simple-packaged`: `path = "../../../../crates/…"` — a THIRD, broken form. When the server stages the pkg under `/root/.cloacina/build-tmp/…`, `../../../../crates/` resolves to the nonexistent `/root/.cloacina/crates/`. Only ever worked for the in-repo `cargo run --example end_to_end_demo`.

**T-0865 verdict: NOT a regression.** cloacina-workflow-plugin's `{ workspace = true }` is irrelevant; the failure is entirely simple-packaged's own relative-path deps.

**Fix (folds into [[CLOACI-T-0884]]):** re-author `simple-packaged` (and the canonical packaged example) off `../../../../crates/`. For a demo-runnable example, use `__WORKSPACE__/crates/` like the fixtures; for the "real distributed package" story, version deps (needs a vendored registry on the demo compiler, which currently has none — separate consideration). This closes T-0887 as "diagnosis done, fix is example re-authoring in T-0884," not a compiler/infra bug. Re-run the T-0884 gold-path E2E to Completed after re-authoring to confirm.
