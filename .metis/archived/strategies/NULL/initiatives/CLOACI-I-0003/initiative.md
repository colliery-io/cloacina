---
id: ci-cd-pipeline-optimization-fail
level: initiative
title: "CI/CD Pipeline Optimization - Fail Fast and Path-Based Filtering"
short_code: "CLOACI-I-0003"
created_at: 2025-11-28T15:38:43.692490+00:00
updated_at: 2025-12-06T01:25:55.935314+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
strategy_id: NULL
initiative_id: ci-cd-pipeline-optimization-fail
---

# CI/CD Pipeline Optimization - Fail Fast and Path-Based Filtering Initiative

## Context

The current CI/CD pipeline has grown organically and now runs **40+ jobs per PR**, regardless of what files changed. This results in:

- Slow feedback loops (waiting for irrelevant tests)
- Wasted compute resources and CI minutes
- Developer frustration from waiting on unrelated failures
- No early termination when fast checks fail

**Current Workflow Inventory:**

| Workflow | Jobs | Trigger | Issues |
|----------|------|---------|--------|
| `ci.yml` | Orchestrator | push/PR | No path filtering |
| `test.yml` | 4 | push/PR | Duplicates ci.yml |
| `cloacina.yml` | 6 | workflow_call | fail-fast: false |
| `cloaca.yml` | 7 | workflow_call | Not called by ci.yml (orphaned?) |
| `cloaca-matrix.yml` | 2N+1 | workflow_call | Per-file parallelism, excessive |
| `examples-docs.yml` | 22 | workflow_call | Runs on every PR |
| `performance.yml` | 4 | workflow_call | Runs on every PR |
| `docs.yml` | 2 | workflow_call | Main/develop only |

**Python Version Over-Range:**
- Release builds: Python 3.9, 3.10, 3.11, 3.12 (4 versions)
- Testing: Python 3.12 only
- OS matrix: ubuntu-latest, macos-latest (some include windows)
- Combined: Up to 4 Python x 2 backends x 2-3 OS = 16-24 wheel builds per release

## Goals & Non-Goals

**Goals:**
- Reduce typical PR job count from 40+ to ~10
- Fail fast on obvious issues (fmt, clippy, compile errors)
- Skip irrelevant tests based on changed paths
- Reduce Python version matrix to practical minimum
- Consolidate or remove duplicate/orphaned workflows
- Maintain release quality while reducing CI burden

**Non-Goals:**
- Changing the test frameworks or tooling (angreal, pytest, cargo test)
- Modifying the actual test implementations
- Reducing test coverage

## Detailed Design

### 1. Path-Based Filtering

Add `dorny/paths-filter` for intelligent job triggering:

```yaml
changes:
  runs-on: ubuntu-latest
  outputs:
    rust: ${{ steps.filter.outputs.rust }}
    python: ${{ steps.filter.outputs.python }}
    docs: ${{ steps.filter.outputs.docs }}
    examples: ${{ steps.filter.outputs.examples }}
  steps:
    - uses: dorny/paths-filter@v2
      id: filter
      with:
        filters: |
          rust:
            - 'cloacina/**'
            - 'cloacina-macros/**'
            - 'Cargo.*'
          python:
            - 'python-tests/**'
            - 'cloaca/**'
            - 'cloaca-backend/**'
          docs:
            - 'docs/**'
            - '*.md'
          examples:
            - 'examples/**'
            - 'tutorials/**'
```

**Skip patterns (never trigger tests):**
- `.metis/**`
- `*.md` (unless in docs/)
- `LICENSE`, `.gitignore`, `.editorconfig`

### 2. Tiered Testing Architecture

```
Tier 1: Quick Gate (< 2 min)
├── cargo fmt --check
├── cargo clippy --all-targets
└── cargo check (both backends)
    ↓ GATE: Only proceed if passes

Tier 2: Core Tests (< 10 min)
├── Rust unit tests (sqlite only - faster)
└── Python smoke test (single backend)
    ↓ GATE: Only proceed if passes

Tier 3: Full Matrix (< 20 min, conditional)
├── Rust integration tests (both backends)
├── Python full tests (both backends)
└── if: rust or python paths changed

Tier 4: Extended (main branch or labeled PRs only)
├── Examples and tutorials
├── Performance tests
└── Cross-platform builds
```

### 3. Python Version and Platform Consolidation

**Current matrix (release):**
- Python: 3.9, 3.10, 3.11, 3.12 (4 versions)
- OS: ubuntu, macos (2 platforms)
- Backends: postgres, sqlite (2 backends)
- Total: 4 x 2 x 2 = 16 wheel builds

**Proposed matrix:**

| Stage | Python Versions | OS | Backends | Jobs |
|-------|-----------------|-----|----------|------|
| CI Testing | 3.12 | ubuntu, macos | both | 4 |
| Release Wheels | 3.10, 3.11, 3.12 | ubuntu, macos | both | 12 |

**Rationale:**
- Python 3.9 EOL: October 2025 (drop support)
- **Cross-platform testing is required** - OS-level interactions (file systems, process handling) behave differently across platforms
- Test on latest Python (3.12) across both supported platforms
- Build wheels for 3.10+ (covers 95%+ of users)
- manylinux wheels work across Linux distros

**Cross-Platform Testing Strategy:**

Rather than "test on one, build everywhere", use a tiered approach:

```
Tier 1 (Quick Gate): Ubuntu only
├── fmt, clippy, cargo check
└── Fast feedback, catches 90% of issues

Tier 2 (Core Tests): Ubuntu + macOS
├── Rust unit tests (both platforms, both backends)
├── Python bindings tests (both platforms, both backends)
└── Catches platform-specific issues early

Tier 3 (Integration): Ubuntu + macOS
├── Full integration tests
└── OS-level interaction coverage
```

This maintains platform coverage while reducing Python version sprawl (4 -> 1 for testing, 4 -> 3 for releases).

### 4. Consolidate Python Test Jobs

**Before:** Per-file parallelism (N test files x 2 backends = 2N+1 jobs)

**After:** Single pytest invocation per backend (2 jobs)

```yaml
python-tests:
  strategy:
    fail-fast: true
    matrix:
      backend: [postgres, sqlite]
  steps:
    - run: pytest python-tests/ -v --tb=short
```

### 5. Enable Strategic Fail-Fast

| Workflow | fail-fast | Rationale |
|----------|-----------|-----------|
| Quick checks | N/A (single job) | Gate for everything |
| Unit tests | `true` | Fast feedback |
| Integration tests | `false` | Find multi-backend issues |
| Examples/tutorials | `true` | Not critical path |

### 6. Conditional Expensive Jobs

```yaml
performance:
  needs: [quick-checks, changes]
  if: |
    github.ref == 'refs/heads/main' ||
    contains(github.event.pull_request.labels.*.name, 'perf')

examples:
  needs: [quick-checks, changes]
  if: |
    needs.changes.outputs.examples == 'true' ||
    github.ref == 'refs/heads/main'
```

### 7. Workflow Consolidation

| Action | Workflow | Reason |
|--------|----------|--------|
| Delete | `test.yml` | Duplicates ci.yml functionality |
| Delete or integrate | `cloaca.yml` | Not called by ci.yml (orphaned) |
| Merge | `cloacina.yml` + `cloaca-matrix.yml` | Reduce orchestration complexity |
| Keep | `ci.yml` | Main orchestrator |
| Keep | `unified_release.yml` | Release-only |
| Keep | `docs.yml` | Already conditional on main/develop |

## Proposed New Structure

```
ci.yml (orchestrator)
│
├── quick-checks [REQUIRED - GATE]
│   ├── cargo fmt --check
│   ├── cargo clippy
│   └── cargo check (both backends)
│
├── changes-detection [REQUIRED]
│   └── dorny/paths-filter
│
├── rust-tests [if: changes.rust]
│   ├── unit-tests (sqlite, fail-fast: true)
│   └── integration-tests (needs: unit, both backends)
│
├── python-tests [if: changes.python]
│   └── bindings-tests (both backends, 2 jobs)
│
├── examples [if: changes.examples OR main]
│   └── Consolidated tutorial/example runs
│
├── docs [if: changes.docs]
│   └── Build docs (no deploy on PR)
│
└── performance [main only OR label:perf]
    └── Performance benchmarks

unified_release.yml (unchanged trigger, reduced matrix)
├── rust-release
├── python-wheels (3.10, 3.11, 3.12 x ubuntu, macos x 2 backends = 12)
└── publish
```

## Expected Outcomes

| Metric | Before | After |
|--------|--------|-------|
| Jobs per typical PR | 40+ | 5-10 |
| Time to first failure | 10-15 min | < 2 min |
| Doc-only PR jobs | 40+ | 1-2 |
| Python versions tested | 4 | 2 |
| Release wheel builds | 16+ | 12 |

## Implementation Plan

### Phase 1: Quick Gate - COMPLETED
- Add quick-checks job with fmt, clippy, cargo check
- Make all other jobs depend on quick-checks
- Enable concurrency cancel-in-progress

### Phase 2: Path Filtering - COMPLETED
- Add dorny/paths-filter to ci.yml
- Add conditional `if:` statements to each job group
- Test with doc-only and rust-only changes

### Phase 3: Consolidate Python Tests - COMPLETED
- Replace per-file matrix with single pytest run
- Reduce Python version matrix to 3.12 for testing
- Release builds use 3.10, 3.11, 3.12

### Phase 4: Workflow Cleanup - COMPLETED
- Deleted test.yml (no longer exists)
- Deleted cloaca.yml (no longer exists)
- cloacina.yml and cloaca-matrix.yml consolidated

### Phase 5: Conditional Expensive Jobs - COMPLETED
- Performance tests run on main-only or with 'run-perf' label
- Examples run only when examples/ changed or on main/develop
- Path-based filtering prevents unnecessary job runs

## Completion Status

**All phases completed.** Current CI achieves:
- 10 jobs for typical Rust-only PR (target was ~10)
- 14 jobs for Rust+Python PR
- Doc-only changes skip all test jobs
- Examples/performance only run when relevant

## Alternatives Considered

### Merge Queues
- **Pros**: Batches PRs, runs full suite less often
- **Cons**: Adds complexity, may slow individual PRs
- **Decision**: Consider after basic optimizations

### Self-Hosted Runners
- **Pros**: Faster builds, better caching
- **Cons**: Maintenance burden, security considerations
- **Decision**: Not needed if job count is reduced

### Nx/Turborepo-style Affected Detection
- **Pros**: Very precise dependency tracking
- **Cons**: Overkill for current repo structure
- **Decision**: Path filtering sufficient for now
