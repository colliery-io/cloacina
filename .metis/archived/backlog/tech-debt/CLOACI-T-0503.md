---
id: investigate-python-execution
level: task
title: "Investigate Python execution sandbox — status and gaps vs. original hardening intent"
short_code: "CLOACI-T-0503"
created_at: 2026-04-16T17:26:59.912489+00:00
updated_at: 2026-04-23T16:42:44.600980+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Investigate Python execution sandbox — status and gaps vs. original hardening intent

## Objective

I-0051 listed "Python sandbox" as a security goal alongside auth, path traversal, and rate limits. Auth/path/rate-limit work landed through I-0083, I-0085, I-0087. It's unclear whether Python sandboxing was explicitly addressed, implicitly covered by tenant/auth isolation, or still open. This is a timeboxed investigation to produce a clear answer and, if gaps exist, concrete follow-up tasks.

## Type
- [x] Tech Debt (investigation)

## Priority
- [x] P2 — If sandboxing is actually missing, a tenant running arbitrary Python could read host filesystem, exfiltrate credentials, or escalate within the server. Needs a clear answer before 1.0.

## Technical Debt Impact

- **Current problems**: Unknown. The original `archive/cloacina-server-week1` branch claimed Python sandboxing but it's not obvious what landed on main.
- **Benefits of fixing**: Clarifies the security posture for multi-tenant server deployments; either confirms coverage or produces an actionable backlog.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Document the current isolation model for Python task/CG execution in server mode
- [ ] Identify what (if anything) restricts filesystem, network, subprocess, and env var access from tenant Python code
- [ ] Compare against the original I-0051 intent and the `archive/cloacina-server-week1` implementation
- [ ] Produce either: (a) a "confirmed covered" writeup citing the mechanism, or (b) concrete follow-up tasks for the specific gaps

## Implementation Notes

Starting points:
- Python execution happens via `cloaca` FFI bindings — check how tasks are dispatched in server mode
- Reference: `archive/cloacina-server-week1` commit `eeebd80` (original "Python sandbox" claim)
- Consider whether tenant schema isolation (T-0485) + credential protection (T-0451) is sufficient for the threat model, or whether process-level sandboxing is still wanted

## Status Updates

### 2026-04-22 — Investigation complete

**Scope covered:** walked the current Python package ingestion + execution paths on `main`, diffed against archive commit `eeebd80` (T-0222 sandbox + T-0220 archive hardening), and traced every call site of the underlying archive extractor.

**What "Python sandbox" actually meant in T-0222** (from archive commit message and diff):
1. `sys.path.append` instead of `insert(0, ...)` — prevents packages from shadowing stdlib imports
2. `STDLIB_DENY_LIST` — static-scan extracted package + vendor dirs for files/dirs whose names match ~26 stdlib modules; reject before import
3. 60s import timeout wrapping the PyO3 `import` call — catches infinite loops at module top-level
4. Archive-extraction hardening (T-0220, same commit): manual tar iteration rejecting symlinks, `..`, absolute paths, and decompression >500 MB or >10× compressed

Note: these are *package-ingestion* hardenings. T-0222 never implemented process-level isolation (seccomp, namespaces, subprocess jail, restricted stdlib). Tenant Python runs in-process via PyO3 with full stdlib access.

**Status on main:**

**✅ Preserved from T-0222 (now in `cloacina-python`, not `cloacina`):**
- `STDLIB_DENY_LIST` (26 entries) — `crates/cloacina-python/src/loader.rs:43`
- `validate_no_stdlib_shadowing()` called before import — `loader.rs:187`, invoked at `loader.rs:243` and `loader.rs:410` (both `import_and_register_python_workflow_named` and computation-graph import path)
- `sys.path.append` (not `insert`) — `loader.rs:268`, `loader.rs:277`
- `IMPORT_TIMEOUT_SECS = 60` with join-polling loop — `loader.rs:251`, `loader.rs:379-392`
- Tests cover the stdlib-shadow check — `crates/cloacina-python/src/lib.rs:343, 362`

**❌ REGRESSED — T-0220 archive-extraction hardening is gone:**
- `extract_python_package` now delegates the tar extract to `fidius_core::package::unpack_package` — `crates/cloacina-python/src/package_loader.rs:145`
- `fidius-core` v0.0.5 uses raw `tar::Archive::unpack()` with zero safety checks — `/Users/dstorey/Desktop/fides/crates/fidius-core/src/package.rs:312`
  - No symlink/hardlink rejection
  - No `..` / absolute-path rejection
  - No decompression-bomb limit
- The same `fidius_core::package::unpack_package` is called from at least 8 other places on `main`, so this regression affects Rust packages too, not just Python:
  - `crates/cloacina/src/registry/reconciler/loading.rs:85`
  - `crates/cloacina/src/registry/workflow_registry/filesystem.rs:110, 190`
  - `crates/cloacina/src/registry/workflow_registry/mod.rs:294`
  - `crates/cloacina/src/packaging/debug.rs:46`
  - `crates/cloacinactl/src/commands/daemon.rs:459`
  - `crates/cloacina-compiler/src/build.rs:82`
  - `crates/cloacina-python/src/package_loader.rs:85, 145`

**❌ Never existed in T-0222, still absent — runtime process-level sandboxing:**
- Tenant Python runs in the same OS process with unrestricted stdlib. A task can `import os; os.environ`, open sockets, spawn subprocesses, read any file the server can, etc.
- Mitigating controls in place: tenant schema isolation (T-0485), credential encryption-at-rest (T-0451), auth/rate-limit (T-0221/T-0223), stdlib-shadowing rejection at import. These cover *data-layer* tenant separation, not *runtime* isolation.
- Threat model: a tenant with package-upload permission can read host filesystem (including other tenants' encryption keys/config if readable by the server UID), make arbitrary outbound network calls, and read env vars (AWS creds, DB passwords, etc.).

### Threat model summary

| Vector | Covered on main? | Where |
|---|---|---|
| Shadow stdlib via uploaded `os.py` | ✅ rejected pre-import | `loader.rs:243` |
| Infinite-loop at import | ✅ 60 s timeout | `loader.rs:379` |
| Tar path traversal (`../../etc/passwd`) | ❌ regressed | `fidius-core/src/package.rs:312` |
| Tar symlink attack | ❌ regressed | same |
| Decompression bomb | ❌ regressed | same |
| `import os` + arbitrary syscalls at runtime | ❌ never covered | by design (PyO3 in-process) |
| Credential exfil via env/fs read at runtime | ❌ never covered | relies on ops hygiene |
| Tenant schema cross-access | ✅ DB layer | T-0485 |
| Credential plaintext at rest | ✅ encrypted | T-0451 |

### Recommendation — split into three follow-ups

1. **P1 — Restore archive-extraction hardening (blocks 1.0).** Either land the safe extraction in `fidius-core` upstream (preferred — single source of truth, benefits all consumers), or wrap the 8 call sites in cloacina with a local safe-unpack helper that does the T-0220 checks before handing the path to fidius. Scope is small and the vulnerability is pre-auth if any endpoint allows package upload.
2. **P2 — Document the tenant-Python trust boundary explicitly.** Current docs imply "sandbox" — reality is "shadow-proof import + 60 s budget, full runtime access." Add an explicit operational note: do not grant package-upload to untrusted tenants without OS-level isolation (containers/VMs).
3. **P3 (deferred) — Runtime sandboxing for multi-tenant server.** True process isolation (e.g. per-tenant Python subprocess under seccomp/landlock, or running the server per-tenant in separate containers) is a larger design decision. Flag as known gap on the I-0051 roadmap; do not block 1.0 if operators are documented as trusting their tenants.

### Decision needed from human
Before creating follow-up tasks, confirm:
- Is "restore T-0220 in fidius-core vs. wrap at cloacina call sites" a decision you want to make now, or should follow-up #1 be an investigation of its own?
- Is runtime sandboxing (follow-up #3) in scope for 1.0 or acceptable as a documented limitation?
