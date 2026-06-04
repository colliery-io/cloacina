---
id: containerized-fleet-e2e-on-kind
level: task
title: "Containerized fleet e2e on kind — server + compiler + agents, churn/reclaim"
short_code: "CLOACI-T-0637"
created_at: 2026-06-01T15:59:08.367858+00:00
updated_at: 2026-06-01T16:00:30.113643+00:00
parent: CLOACI-I-0114
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0114
---

# Containerized fleet e2e on kind — server + compiler + agents, churn/reclaim

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0114]]

## Objective **[REQUIRED]**

Prove the execution-agent fleet works in a real containerized (Kubernetes) topology,
not just as host subprocesses. Stand the whole stack up on a **kind** cluster — Postgres,
`cloacina-server` (routing `**=fleet`), `cloacina-compiler` (in-cluster, builds the cdylib
to the agents' linux triple), and **N `cloacina-agent` replicas** — compile + run a
workflow, assert it executes on an agent and reconciles, then **kill an agent pod mid-run
and assert the task completes on another via dead-agent reclaim**.

Design decisions (2026-06-01, with user):
- **Compiler in-cluster** (not prebuilt package) — the cdylib must match the agents'
  container triple (OQ-6 fail-closed); building in-cluster matches the local e2e + real prod.
- **Scope: happy path (multi-agent) + churn/reclaim.**
- Mirrors `angreal helm test` (kind) — build images, load, install, assert, teardown.

## Plan / deliverables

1. **Agent image** — `docker/Dockerfile.agent`: mirror root Dockerfile builder, `--bin
   cloacina-agent`; runtime = debian-slim + same libs (libpq5, libpython3.11, libsasl2-2,
   libssl3) since the agent dlopens cdylibs that link them.
2. **Compiler image** — `docker/Dockerfile.compiler`: rust toolchain at runtime (it shells
   `cargo build`) + the cloacina workspace baked in (uploaded packages have path-deps to
   cloacina via `__WORKSPACE__`, rewritten to the in-container path); pre-warm the cargo
   build cache so first compile isn't a cold ~100-dep build. Heaviest image.
3. **k8s manifests** — compiler Deployment + agent Deployment (N replicas) pointed at the
   server Service, agent `CLOACINA_API_KEY` from the bootstrap secret. (Raw manifests in the
   test for v1; a real `charts/cloacina-agent` is a follow-up under T-0635.)
4. **angreal task** — `angreal helm fleet`: build 3 images → kind up → load → install server
   chart (`server.extraEnv: CLOACINA_FLEET_ROUTES=**=fleet`, known bootstrap key) + apply
   compiler/agent manifests → port-forward → upload source (staged with in-container
   workspace path) → poll build success → run → assert Completed + fleet dispatch (agent
   logs / server logs) → churn (kubectl delete one agent pod mid-run) → assert reclaim →
   teardown.

## Key risks
- **Path-dep resolution in-cluster:** uploaded package Cargo.toml must point at the baked-in
  workspace path; compiler image must contain that source. Mirror the host e2e's
  `__WORKSPACE__` rewrite but to the container path.
- **Triple match (OQ-6):** compiler output triple must equal the agent container triple
  (both kind-node linux/arch). Build compiler + agent from the same base arch.
- **Image weight / build time:** compiler image carries rust + workspace + cargo cache;
  expensive to build. Watch disk ([[feedback_angreal_purge_disk]]).
- **kind networking:** agents reach the server via the in-cluster Service DNS, not loopback.

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

### 2026-06-04 — implementation authored + statically verified (pre-run)

All four deliverables authored and statically verified; ready for a real `angreal helm fleet` run.

**Authored:**
- `docker/Dockerfile.agent` — `rust:1.93-slim` builder → `--bin cloacina-agent`; debian-slim runtime + libpq5/libpython3.11/libsasl2-2/libssl3 (cdylib dlopen deps). Non-root uid 10001.
- `docker/Dockerfile.compiler` — full rust toolchain at runtime, bakes `/workspace`, `CARGO_TARGET_DIR=/workspace/target`, pre-warms deps via `cargo build --release --bin cloacina-compiler`. Runs as root.
- `.angreal/task_helm.py::helm_fleet` (`angreal helm fleet`) — build cloacinactl + 3 images → kind up + load → ns + `fleet-bootstrap` secret → `helm install` server chart (bundled postgres, `apiKeySecretRef: fleet-bootstrap`, `server.extraEnv: CLOACINA_FLEET_ROUTES=**=fleet`) → compiler + agent Deployments → port-forward → upload/compile (poll build_status) → run + assert Completed + "agent reported result" in server log → churn (delete one agent pod, re-run, assert Completed on survivor) → teardown.

**Static verification done this session:**
- `py_compile` clean; task registers in `angreal tree` (helm › fleet).
- Agent CLI/env confirmed against `crates/cloacina-agent/src/main.rs`: `--server`/`CLOACINA_SERVER`, `--api-key`/`CLOACINA_API_KEY`, `--max-concurrency` (no env). Manifest passes server+key via env, concurrency via arg. ✓
- Compiler CLI confirmed against `crates/cloacina-compiler/src/main.rs`: `--bind`, `--poll-interval-ms`, `--cargo-flag` (repeatable), `DATABASE_URL` env. ✓
- `--cargo-flag` REPLACES the default `build --release --lib --frozen --offline` (main.rs:73). We pass `build --release --lib` → drops `--frozen --offline` for the online (kind egress) build. No `--vendor-dir`, so `CARGO_HOME` stays the image's warmed `/usr/local/cargo`. ✓
- Service DNS: compiler `DATABASE_URL` → `fleet-e2e-postgresql:5432`; agent → `http://fleet-e2e-cloacina-server:8080` — both match chart release-based naming (`{{ .Release.Name }}-postgresql`, `{fullname}`). ✓

**Bug fixed this session:** `workflow run -o json` returns the id in the `execution_id` JSON field, but the run loop was reading the last output line (`}` for pretty JSON). Replaced both run/status loops with `_run_workflow`/`_wait_exec` helpers that parse `execution_id` (mirrors the proven host-e2e `_poll_run_workflow`).

**Remaining (runtime-only) risks — surface on first real run:**
- compiler cold cargo build duration in-cluster (image pre-warms deps; `--timeout`/poll deadlines set to 900s for build).
- kind image-load weight (compiler image is heavy — watch host disk, [[feedback_angreal_purge_disk]]).
- churn case proves survivor handling, NOT true in-flight reclaim (needs a slow-task fixture — follow-up under T-0635).

**Not yet committed** — all T-0637 work is uncommitted and NOT in PR #115 (which is the fleet + substrate + T-0636 cleanup at commit `93925ccb`).

**Next:** run `angreal helm fleet` (needs docker+kind+kubectl+helm) and fix runtime drift.

### 2026-06-04 — GREEN. Containerized fleet e2e passes end to end on kind.

`angreal helm fleet` is green: server (routes `**=fleet`) + bundled postgres + in-cluster compiler + 2 agents stand up on kind; the workflow is uploaded, compiled in-cluster (~13s, warm cache), runs on an agent (execution `Completed` + server log carries `agent reported result`), then the churn case deletes an agent pod and the survivor completes a re-run.

Four real bugs surfaced + fixed during bring-up (each now instrumented to fail loudly via `_dump_diag`/`_wait_rollout_or_dump` + build-failure dump):
1. **Compiler CrashLoopBackOff** — `--cargo-flag --release` (two-token) parsed by clap as a bogus `--release` flag for the compiler itself; hyphen-led values need the `=` form → `--cargo-flag=--release`.
2. **Build failed: "expected libcompiler_happy_rust.so in .../target/release"** — cargo *succeeded* but the `.so` landed in `/workspace/target` (image's `ENV CARGO_TARGET_DIR`), while the compiler's artifact discovery looked in the package-local `target/` because its own `--cargo-target-dir` flag was unset (`config.cargo_target_dir = None`; build.rs:411-418, 531-536). Fix: pass `--cargo-target-dir /workspace/target` so the compiler owns the dir AND reuses the warm cache.
3. **`workflow run` false timeout** — the command prints the bare execution_id via `println!` (workflow/mod.rs:102), NOT a JSON object, even under `-o json`; `json.loads()` choked and the id was discarded. Fix: last-line fallback in `_run_workflow` (mirrors host e2e `_poll_run_workflow`).
4. **Diagnostics gap** — original failures tore the cluster down before capturing pod state/logs. Added `_dump_diag` (get/describe/logs + previous logs) on rollout + build failure so each failure is conclusive in one run.

Design notes confirmed in practice:
- Containers are all-release, so the cdylib is built `--release` (fidius wire format = bincode); the host e2e builds debug to match its debug server. Profile must follow the consumer.
- In-cluster build is online (drops default `--frozen --offline`) but reuses the image's warm `/workspace/target` + `/usr/local/cargo` registry, so the per-package build is ~13s not a cold ~100-dep build.

Exit criteria met. Committing into PR #115 (i-0114-execution-agent-fleet). Churn proves survivor handling, not true in-flight reclaim — that needs a slow-task fixture, deferred to T-0635 (also: promote the in-test raw agent/compiler manifests to a real `charts/cloacina-agent`).
