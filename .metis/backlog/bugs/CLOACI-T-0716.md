---
id: execution-agent-fleet-cannot-run
level: task
title: "Execution-agent fleet cannot run Python workflows (dlopen of empty artifact fails)"
short_code: "CLOACI-T-0716"
created_at: 2026-06-17T02:16:21.921546+00:00
updated_at: 2026-06-17T02:16:21.921546+00:00
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

- [ ] A Python-packaged workflow executes successfully end-to-end with
      `CLOACINA_DEFAULT_EXECUTOR=fleet` and an agent doing the work.
- [ ] A Python-packaged computation graph (e.g. `demo-py-graph`) runs on the
      fleet.
- [ ] The agent reports a clear, classified error (not a raw `file too short`
      dlopen failure) if it is ever asked to run something it genuinely cannot.
- [ ] Mixed Rust + Python packages in one deployment both run on the fleet.

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
