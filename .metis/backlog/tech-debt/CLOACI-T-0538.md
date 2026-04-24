---
id: reorganize-angreal-test-harness
level: task
title: "Reorganize angreal test harness for clearer hierarchy and ergonomics"
short_code: "CLOACI-T-0538"
created_at: 2026-04-23T14:21:24.903964+00:00
updated_at: 2026-04-23T16:43:59.717783+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Reorganize angreal test harness for clearer hierarchy and ergonomics

## Objective

The angreal harness in `.angreal/` is functional but its top-level hierarchy
obscures what lives where. Reorganize the command tree and helper modules so a
newcomer running `angreal tree` can tell at a glance where tests, demos,
performance, linting, and service management live — and so there is one
obvious home for each concept.

Note on loader convention: root-level files **must** keep the `task_*.py`
prefix because angreal's loader discovers them by regex. Subpackages
(`cloacina/`, `demos/`, ...) do not need the prefix. The goal of this task is
to push more work *into* subpackages/namespaces, not to flatten.

## Technical Debt Impact

### Current Problems

- **`cloacina` is too generic a group name for "core engine tests"** — every
  test in the repo is "cloacina."
- **Performance work lives in two places**: top-level `performance/*` and
  `demos perf-*` run overlapping workloads.
- **`demos` is 40 flat entries** mixing Rust tutorials, Python tutorials,
  feature examples, and performance runners with no subgrouping.
- **`coverage` and `purge` are lone top-level leaves** next to groups —
  visually inconsistent and hard to discover.
- **`check metrics-format` is a live-server Docker+Postgres test** living
  alongside static cargo checks.
- **Util module sprawl**: `utils.py`, `database_reset.py` (orphan, not
  imported), `cloacina/cloacina_utils.py` (23 lines), `cloacina/python_utils.py`
  (461 lines), `demos/demos_utils.py` — unclear ownership.
- **Flag inconsistency**: short flags only in `performance.py`; ad-hoc
  `--skip-*`, `--no-*`, `--warnings-only` without a convention; test filtering
  only on `cloacina unit`.
- **No `test`, `lint`, or `ci` aggregators** — newcomers' first guesses.
- **No risk levels** on destructive tasks (`purge`, `services clean/reset`,
  demos that reset volumes).

### Benefits of Fixing

- `angreal tree` is self-documenting — newcomers and AI agents route correctly
  without reading source.
- One home per concept (perf, tests, linting) eliminates "which command do I
  run?" cognitive tax.
- Subgroup structure scales: more demos and test types won't flatten into
  noise.
- Consistent flags across the harness reduce per-command memorization.

### Risk Assessment

- **Breaking change for muscle memory and CI**: `angreal cloacina unit` →
  `angreal test unit`, etc. Must update CI workflows, docs, READMEs, and any
  scripts in `examples/` or `docs/`.
- Low technical risk — this is moving code between files and renaming groups,
  not changing what the tasks do.

## Proposed Target CLI Structure

```
angreal tree
├── test                            # (renamed from `cloacina`)
│   ├── all                         # runs every suite below
│   ├── unit [FILTER]
│   ├── integration                 # Rust + Python pytest w/ backing services
│   ├── macros
│   ├── auth
│   ├── e2e
│   │   ├── cli                     # cloacinactl against live server
│   │   ├── compiler                # cloacina-compiler end-to-end
│   │   └── ws                      # WebSocket / computation graph endpoints
│   ├── soak
│   │   ├── daemon                  # sustained package load/exec
│   │   └── server                  # HTTP API verification
│   ├── coverage                    # (promoted from top-level `coverage`)
│   └── metrics-format              # (moved from `check metrics-format`)
│
├── check                           # static-only code quality
│   ├── all-crates
│   ├── crate <path>
│   └── credential-logging
│
├── lint                            # NEW — code quality aggregator
│   ├── all                         # fmt + clippy + credential-logging
│   ├── fmt                         # cargo fmt + ruff/black
│   ├── clippy
│   └── credential-logging          # (alias or re-export of check variant)
│
├── performance                     # SINGLE home for perf work
│   ├── all
│   ├── quick
│   ├── simple
│   ├── pipeline
│   ├── parallel
│   └── computation-graph-bench
│
├── demos                           # subgrouped — no more 40-flat
│   ├── tutorials
│   │   ├── rust
│   │   │   ├── 01  .. 10
│   │   │   └── all
│   │   └── python
│   │       ├── 01  .. 11
│   │       └── all
│   ├── features
│   │   ├── continuous-scheduling
│   │   ├── cron-scheduling
│   │   ├── deferred-tasks
│   │   ├── event-triggers
│   │   ├── multi-tenant
│   │   ├── packaged-graph
│   │   ├── per-tenant-credentials
│   │   ├── python-packaged-graph
│   │   ├── python-workflow
│   │   ├── registry-execution
│   │   └── all
│   └── (perf demos removed — fold into top-level `performance`)
│
├── docs                            # unchanged
│   ├── build
│   ├── clean
│   └── serve
│
├── services                        # unchanged surface; purge folded in
│   ├── up
│   ├── down
│   ├── reset
│   ├── clean
│   └── purge                       # (moved from top-level `purge`)
│
└── ci                              # NEW — mirror CI locally
    ├── fast                        # lint + unit
    └── full                        # lint + test all + coverage
```

### What moves, at a glance

| From | To |
| --- | --- |
| `cloacina *` | `test *` |
| `cloacina all` | `test all` |
| `cloacina cli-e2e` | `test e2e cli` |
| `cloacina compiler-e2e` | `test e2e compiler` |
| `cloacina ws-integration` | `test e2e ws` |
| `cloacina soak` | `test soak daemon` |
| `cloacina server-soak` | `test soak server` |
| `cloacina auth-integration` | `test auth` |
| `coverage` (top-level) | `test coverage` |
| `check metrics-format` | `test metrics-format` |
| `check credential-logging` | stays in `check`, aliased under `lint` |
| `demos tutorial-0N` | `demos tutorials rust 0N` |
| `demos python-tutorial-0N` | `demos tutorials python 0N` |
| `demos perf-*` | merged into `performance *` |
| `demos multi-tenant`, etc. | `demos features *` |
| `purge` (top-level) | `services purge` |
| (new) | `lint *`, `ci fast`, `ci full` |

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `angreal tree` matches the proposed structure above.
- [x] Root `.angreal/task_*.py` files are thin shims that import their
      corresponding subpackage — `task_project.py` is a pure registrar,
      no command logic at the root.
- [x] New subpackages created: `.angreal/test/` (replaces `cloacina/`),
      `.angreal/lint/`, `.angreal/ci/`. `demos/` gains nested
      `tutorials/{rust,python}/` and `features/` modules.
- [x] Utility consolidation: orphan `database_reset.py` deleted;
      `cloacina_utils.py` → `test/_utils.py`; each util module has a
      module-level docstring stating its scope.
- [x] Every task has `when_to_use` / `when_not_to_use`. Destructive tasks
      (`services purge`, `services clean`, `services reset`,
      `demos tutorials python NN --backend postgres`) carry
      `tool=angreal.ToolDescription(..., risk_level="destructive")`.
- [x] Flag conventions documented in `.angreal/README.md` and applied:
      `--skip-<thing>`, `--no-<thing>`, `-v`/`--verbose`, positional `FILTER`
      on test tasks where applicable.
- [x] CI workflows (`.github/workflows/*.yml`) updated to new command names.
- [x] `docs/` and `examples/` references updated to new command names
      (root `README.md` has no angreal refs; no `CONTRIBUTING.md` exists).
- [x] `angreal test all` produces the same coverage as the previous
      `angreal cloacina all` — body unchanged, only group rename.

## Implementation Notes

### Technical Approach

1. Introduce new subpackages alongside the old ones, re-exporting existing
   task functions (no behavior change).
2. Add thin `task_test.py`, `task_lint.py`, `task_ci.py` root shims so angreal
   registers the new commands.
3. Land a grep-wide rename of call sites (CI, docs, examples) in one commit
   after the new tree is green.
4. Remove the old `cloacina/` group and `task_*.py` shims in a follow-up
   commit once CI is fully migrated.
5. Keep the rename mechanical — do not refactor test internals in the same PR.

### Dependencies

None. Standalone harness cleanup.

### Risk Considerations

- CI breakage during the migration window — stage the rollout: (a) add new
  names as aliases, (b) flip CI, (c) remove old names.
- Muscle-memory impact on contributors — announce in PR description, update
  `CONTRIBUTING.md`, consider a short alias table that lives in
  `.angreal/README.md` for one release.

## Status Updates

### 2026-04-24 — Pass 1 landed (clean rename, option B)

Verified upstream: angreal's three-level `command_group` stacking works
end-to-end (user added a functional test invoking
`stack-outer stack-inner leaf`). Decorator nearest `def` = innermost group.

**Moves (via `git mv` to preserve history):**

- `.angreal/cloacina/` → `.angreal/test/`
- `test/soak.py` → `test/soak/daemon.py`
- `test/server_soak.py` → `test/soak/server.py`
- `test/cloacinactl_e2e.py` → `test/e2e/cli.py`
- `test/compiler_e2e.py` → `test/e2e/compiler.py`
- `test/ws_integration.py` → `test/e2e/ws.py`
- `test/auth_integration.py` → `test/auth.py`
- `test/cloacina_utils.py` → `test/_utils.py`
- `test/python_utils.py` → `test/_python_utils.py`
- `.angreal/task_coverage.py` → `.angreal/test/coverage.py`
- `.angreal/demos/demos_utils.py` → `.angreal/demos/_utils.py`
- Deleted: `task_purge.py` (folded into `task_services.py` as `services purge`)
- Deleted: `database_reset.py` (orphan — no imports)
- Deleted: `demos/rust_demos.py`, `demos/python_demos.py` (replaced by subpackages)

**New subpackages:**

- `test/metrics_format.py` (moved from `task_check.py`)
- `lint/` with `fmt.py`, `clippy.py`, `credential_logging.py`, `all.py`
- `ci/` with `fast.py`, `full.py`
- `demos/tutorials/rust.py` and `demos/tutorials/python.py`
- `demos/features/features.py`

**Group/leaf renames inside files:**

- Every `command_group(name="cloacina", ...)` → `name="test"`, with stacked
  `e2e` / `soak` subgroups where applicable (outer-first:
  `@test() @e2e() @angreal.command(...)`).
- Leaf renames: `cli-e2e`→`cli`, `compiler-e2e`→`compiler`,
  `ws-integration`→`ws`, `soak`→`daemon`, `server-soak`→`server`,
  `auth-integration`→`auth`.
- Tutorial leaves are the zero-padded number (`01`..`11`); feature
  demos keep their directory name under `demos features`.
- `demos perf-*` removed — `performance` is the sole home.
- Note: decided to move `credential-logging` outright into `lint` (no alias
  under `check`); the From→To table's alias note is superseded.

**External consumers updated:**

- `.github/workflows/cloacina.yml`, `nightly.yml`, `examples-docs.yml`
- `docs/content/**/tutorials/library/*.md` (8 files)
- `docs/content/contributing/_index.md`, `docs/operations/metrics.md`
- `examples/features/workflows/multi-tenant/README.md`

Stale reference to `angreal demos validation-failures` in
`examples/features/workflows/validation-failures/README.md` was left alone —
that command never existed (the directory is excluded from auto-registration);
that's a pre-existing doc bug, out of scope here.

**Verification:**

- `angreal tree` renders the full new tree; every group/subgroup present.
- `angreal test --help`, `angreal test e2e --help`, `angreal test e2e cli --help`
  all route through the nested groups correctly.
- Grep confirms no remaining references to old command forms anywhere
  outside `.metis/` or archive paths.

**Not yet done (follow-ups inside this PR):**

- Smoke-test a leaf end-to-end (e.g. `angreal lint fmt --check`,
  `angreal test unit`) to confirm relative imports resolve at runtime.
- `ToolDescription` audit + `risk_level` on destructive tasks
  (`services purge/clean/reset`, any demo that wipes volumes).
- `.angreal/README.md` documenting the flag convention.
- Tick exit-criteria checkboxes in this task body.
