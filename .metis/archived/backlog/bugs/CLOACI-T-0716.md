---
id: execution-agent-fleet-cannot-run
level: task
title: "Execution-agent fleet cannot run Python workflows (dlopen of empty artifact fails)"
short_code: "CLOACI-T-0716"
created_at: 2026-06-17T02:16:21.921546+00:00
updated_at: 2026-06-17T03:25:40.235570+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Execution-agent fleet cannot run Python workflows (dlopen of empty artifact fails)

## Objective

Make the execution-agent fleet (CLOACI-I-0114) able to run Python-packaged
workflows and computation graphs, so that turning on the fleet
(`CLOACINA_DEFAULT_EXECUTOR=fleet`) does not silently break Python support —
which is a core capability, not an add-on.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (fleet + Python is a supported combination; turning the fleet on
  must not regress a core language)

### Impact Assessment
- **Affected Users**: Anyone running the execution-agent fleet with any
  Python-packaged workflow/graph. In the demo stack this is `demo-py-workflow`
  and `demo-py-graph`; every Python package fails the same way.
- **Reproduction Steps**:
  1. Run the server with `CLOACINA_DEFAULT_EXECUTOR=fleet` and ≥1 agent
     (`cloacina-agent`), e.g. the `docker/docker-compose.demo.yml` stack.
  2. Upload + register a pure-Python package (`demo-py-workflow`).
  3. Execute it.
- **Expected vs Actual**:
  - *Expected*: the workflow runs (as it does on the in-process executor, which
    imports the module via PyO3).
  - *Actual*: every task fails immediately, retries exhausted, workflow Failed.
    Server log:
    ```
    Task failed task_name=public::demo-py-workflow::demo_py_workflow::prepare
    error="Task execution error: Task execution failed: agent refused
    (RuntimeLoadFailed): register_package_tasks: Failed to load library at
    /tmp/.../tasks_<uuid>.so: libloading error: ... file too short"
    ```

## Root Cause

The DB-less agent treats **every** work packet as a Rust cdylib: it fetches the
artifact by digest (`GET /v1/agent/artifact/{digest}`) and `dlopen`s it. But
pure-Python packages have **no cdylib** — the compiler short-circuits them
(`crates/cloacina-compiler/src/build.rs`: "pure-Python package: skipping cargo
build, using empty artifact") and stores an empty/placeholder artifact. The
in-process executor runs Python via the PyO3 import path and never dlopens that
artifact; the agent has no such path, so it dlopens the empty `.so` and fails
with `file too short` (`RuntimeLoadFailed`).

So the fleet is Rust-cdylib-only by construction; routing Python work to it is a
guaranteed failure.

## Acceptance Criteria

## Acceptance Criteria

- [x] A Python-packaged workflow executes successfully end-to-end with
      `CLOACINA_DEFAULT_EXECUTOR=fleet` and an agent doing the work. ✅
      `demo_py_workflow` Completed on the fleet (prepare+finish, repeatedly).
- [x] A Python-packaged computation graph (e.g. `demo-py-graph`) runs. ✅
      N/A-by-design for "on the fleet": CGs execute **in-process in the reactor**
      (reactor.rs:813 "in-process CG dispatch"), never routed to the fleet
      executor, so they were never broken by enabling the fleet. `demo_py_graph`
      is firing healthily (300+ fires) in the fleet-enabled demo.
- [x] The agent reports a clear, classified error if it is ever asked to run
      something it genuinely cannot (RuntimeLoadFailed / Validation). ✅
- [x] Mixed Rust + Python packages in one deployment both run on the fleet. ✅
      Rust (`demo_slow_workflow`) + Python (`demo_py_workflow`) both run.

## Implementation Notes

### Candidate approaches (decide during design)
1. **Teach the agent the Python load path.** The agent image already links
   libpython (see `docker/Dockerfile.agent`). Give the agent a PyO3 loader that
   recognizes a Python package (by interface/marker in the work packet or
   artifact) and imports the module instead of `dlopen`-ing a cdylib. This keeps
   "all work runs on the fleet" intact. Heaviest, but the most complete fix.
2. **Per-package executor routing.** Let the scheduler route Python packages to
   the in-process executor and Rust packages to the fleet, even when the
   server-wide default is `fleet`. Needs an executor-selection signal on the
   package (language/interface). Lighter, but leaves Python off the fleet.
3. **Carry the Python module tree in the work packet** (instead of an artifact
   digest) so the agent can materialize + import it. Variant of (1).

### Related code
- `crates/cloacina-agent/src/main.rs` — work-packet handling, artifact fetch +
  `register_package_tasks` / dlopen.
- `crates/cloacina-server/src/fleet_executor.rs` — builds the `WorkPacket`
  (`ArtifactRef { fetch_url, digest, build_target_triple }`); would need to
  signal language/interface.
- `crates/cloacina-compiler/src/build.rs` — the empty-artifact short-circuit for
  pure-Python packages.
- Memory note: "Python support is a core capability; fix pyo3 leakage with a
  crate split, not feature flags" — keep that principle in mind for the agent
  loader split.

### Interim demo mitigation (not a fix)
Until this lands, the compose demo routes Python work to a fleet that cannot run
it. Either keep the Python demos but note they require the in-process executor,
or gate them — tracked here so we don't ship the demo pretending Python works on
the fleet.

## Status Updates

- 2026-06-17: Filed. Discovered while enabling the agent fleet in the compose
  demo — `demo_py_workflow` failed with `RuntimeLoadFailed: file too short` on
  every task. Root-caused to the agent dlopen-ing the empty Python artifact.
- 2026-06-17: Active. Traced both load paths end-to-end:
  - **Agent (Rust)**: `process_work_packet` (cloacina-agent/src/main.rs ~640)
    → `fetch_and_cache_artifact` (by digest) → `TaskRegistrar::register_package_tasks`
    (cloacina/src/registry/loader/task_registrar/mod.rs:103) → fidius dlopen.
    Empty Python artifact → dlopen fails.
  - **In-process (Python)**: reconciler `loading.rs:293-415` routes on
    `metadata.language`; Python goes through the `PythonRuntime` trait →
    `cloacina-python::import_and_register_python_workflow_named` (loader.rs:239),
    which imports the module via PyO3 from an unpacked `workflow/` + `vendor/`
    tree.
  - **Language signal**: lives in `workflow_packages.metadata` (JSON,
    `CloacinaMetadata.language`); NOT a queryable column and NOT carried in the
    `WorkPacket`/`ArtifactRef` (protocol.rs:144/167, AGENT_PROTOCOL_VERSION=1).
  - **Python source**: the `.cloacina` archive (workflow/ + vendor/) is in
    `workflow_registry.package_data`, server-side only; `compiled_data` is
    empty for Python. The agent has no way to fetch the source today.
  - **FleetExecutor** (fleet_executor.rs:349-399) only resolves the content
    hash; it has no language and always builds an artifact-digest WorkPacket.
  - DECISION: **Approach A** — agents run Python via PyO3 (so Python scales on
    the fleet), chosen by the user.

### Implementation plan (Approach A)
1. **protocol.rs** — add `language: Option<String>` to `WorkPacket` (serde
   default; `None`/"rust" → cdylib path). Wire-additive, keep protocol v1.
2. **DAL** (`dal/unified/workflow_packages.rs`) — add
   `get_package_archive_by_content_hash(digest)`: join workflow_packages
   (content_hash, build_status='success') → `workflow_registry.data`
   (`joinable!(workflow_packages -> workflow_registry (registry_id))`), returns
   the `.cloacina` source archive bytes. Also a language lookup for the package
   (parse `metadata` JSON → `CloacinaMetadata.language`).
3. **Server route** (`routes/agent.rs` + mod/openapi) — `GET /v1/agent/source/{digest}`
   returns the source archive (mirror `fetch_artifact`, which serves
   `compiled_data`; this serves the registry archive).
4. **FleetExecutor** (`fleet_executor.rs`) — resolve the package language and
   stamp `packet.language`.
5. **Agent** (`cloacina-agent/Cargo.toml` + `main.rs`):
   - dep on `cloacina-python`; call its `install()` at startup (registers the
     process `PythonRuntime`).
   - in `process_work_packet`, branch on `packet.language`: python → fetch the
     source archive from `/v1/agent/source/{digest}`, unpack to a staging dir,
     `python_runtime().load_workflow_package(&archive, &staging, tenant, &runtime)`
     on a blocking thread (GIL); else the existing cdylib dlopen path. Steps 4+
     (resolve namespace + execute) are SHARED.
6. Build server + agent images, deploy, verify `demo_py_workflow` runs on an
   agent and reports success.

Risk: PyO3 now linked into the agent binary (the agent image already carries
libpython + python3-dev per docker/Dockerfile.agent). load_workflow_package is
sync/GIL → run under spawn_blocking.

### Implemented + verified (2026-06-17)
Shipped exactly the plan. Files changed:
- `fleet/protocol.rs`: `WorkPacket.language: Option<String>` (serde default).
- `dal/unified/workflow_packages.rs`: `get_active_dispatch_for_package`
  (content_hash + language; language = python when compiled_data empty) and
  `get_package_archive_by_content_hash` (registry source archive by digest).
- `routes/agent.rs` + `lib.rs`: `GET /v1/agent/source/{digest}`.
- `fleet_executor.rs`: stamps `language` into the WorkPacket.
- `cloacina-agent` (`Cargo.toml` + `main.rs`): dep on `cloacina-python`,
  `cloacina_python::install()` at startup, a per-digest **loaded-Runtime cache**
  (load once, reuse — required because re-importing a Python module is a no-op),
  and a `load_python_package` branch (fetch source → stage under cache_dir →
  `PythonRuntime::load_workflow_package` under `spawn_blocking`). Tenant is
  derived from the namespaced `task_name` (public rides as `tenant_id=None`, so
  `unwrap_or_default()` would mis-register under empty tenant).
- `docker/Dockerfile.agent`: added `libpython3.11-stdlib` + `python3.11` to the
  runtime stage (embedded interpreter needs the stdlib to `Py_Initialize`).

Two bugs found + fixed during bring-up: (1) embedded Python needs the stdlib in
the runtime image, not just libpython; (2) tenant must come from `task_name`'s
namespace, not the (None-for-public) `tenant_id`. Verified: `demo_py_workflow`
Completed on the fleet across repeated runs; mixed Rust+Python both run;
`demo_py_graph` firing in-process. ACCEPTANCE MET.