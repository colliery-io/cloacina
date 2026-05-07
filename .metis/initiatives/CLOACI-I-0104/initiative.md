---
id: compiler-hardening-phase-1-build
level: initiative
title: "Compiler hardening Phase 1 — build timeouts, offline-by-default, resource limits"
short_code: "CLOACI-I-0104"
created_at: 2026-05-06T11:05:31.488861+00:00
updated_at: 2026-05-06T11:05:31.488861+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: compiler-hardening-phase-1-build
---

# Compiler hardening Phase 1 — build timeouts, offline-by-default, resource limits Initiative

## Context

`cloacina-compiler` runs `cargo build` on user-supplied source code with no sandbox, no resource limits, and full process privileges. `build.rs` scripts in submitted packages execute with the compiler service's UID, full network access, and read access to `~/.cargo/credentials.toml` and the compiler's `DATABASE_URL`. With that DATABASE_URL a malicious build script has full read/write access to all tenant data in the cluster.

The May 2026 review categorized this as Critical (SEC-06, OPS-07) and called for a phased response. **This initiative covers Phase 1 only**: bounded-cost mitigations that do not require choosing a sandbox primitive. The full sandbox is tracked separately as CLOACI-I-0105 because the primitive choice (bubblewrap, container-spawn, nsjail, landlock) is a separable architectural decision.

Phase 1 mitigations are intentionally lightweight, immediately deployable, and complementary to the Phase 2 sandbox (they remain useful even after the sandbox lands).

## Goals & Non-Goals

**Goals:**
- Bound the worst-case wall-clock cost of any single build via configurable timeout. Closes OPS-10 (SIGTERM-hangs-on-cargo).
- Default to off-network builds via `--frozen --offline` against a curated, pre-vendored cargo registry.
- Apply per-build resource limits via `setrlimit` (CPU, memory, FDs, processes).
- Document the interim deployment posture: unprivileged UID, no outbound network beyond curated paths, no admin credentials beyond build-claim DB user.
- Add audit logging for build start/finish, including the Cargo.toml dep-graph hash for forensic traceability.

**Non-Goals:**
- Process namespace isolation. Tracked under CLOACI-I-0105.
- Tmpfs-backed build root. Tracked under CLOACI-I-0105.
- Per-build container spawn. Tracked under CLOACI-I-0105.
- Landlock filesystem policy. Tracked under CLOACI-I-0105.
- Reworking the build queue or claim model.

## Source Findings (May 2026 review)

- **SEC-06 (Critical)** — `cargo build` runs unsandboxed on attacker source; `build.rs` is RCE on the build host.
- **OPS-07 (Critical-adjusted)** — Compiler service is unsandboxed; multi-tenant deployments see attacker-source builds.
- **OPS-10 (Minor)** — SIGTERM does not interrupt `cargo build`; shutdown can hang for the duration of any in-flight build.

## Discovery Questions

- What's the right default `--build-timeout-s`? 600s (10 min) is conventional but may break legitimate large builds.
- How do we curate the pre-vendored registry? Per-deployment or shipped as a fixture? How do operators add deps for in-house packages?
- How does `--frozen --offline` interact with packages that legitimately need new deps? Probably reject + report; need explicit operator workflow.
- Cross-platform: `setrlimit` is Unix-only. Do we need a Windows path, or is Linux-only acceptable for the compiler service?
- Audit log destination: same audit-log facility as I-0103, or a separate compiler-events stream?

## Initial Sketch

- Wrap the cargo subprocess in `tokio::time::timeout(Duration::from_secs(build_timeout_s), child.wait())` with kill-on-timeout.
- Default cargo flags to `["build", "--release", "--lib", "--frozen", "--offline"]` with override via config.
- Use `std::os::unix::process::CommandExt::pre_exec` to set RLIMIT_CPU / RLIMIT_AS / RLIMIT_NOFILE / RLIMIT_NPROC before exec.
- Document `production-deployment.md` posture: dedicated UID, no outbound network beyond curated cargo paths, no Cloacina admin credentials beyond build-claim DB user.
- Audit-log entries on build start/finish with Cargo.toml hash and build-claim id.
- Wire `--build-timeout-s`, `--build-rlimit-cpu`, `--build-rlimit-mem`, `--build-rlimit-procs`, `--build-rlimit-files`, `--vendor-dir` config flags.

## Acceptance Criteria

- `cloacina-compiler` rejects packages whose `Cargo.toml` requires fetching new dependencies (with `--frozen --offline` enabled by default).
- A build process exceeding `--build-timeout-s` is killed; the build row is reset to `pending` by the sweeper.
- `setrlimit` wrapper verifiably bounds the cargo subprocess (validated by integration test that submits a build that overshoots).
- `production-deployment.md` documents the threat model and the operator's responsibility for network/UID isolation.
- Audit-log entries appear on every build start/finish with Cargo.toml hash and build-claim id.

## References

- `review/07-security.md` — SEC-06
- `review/06-operability.md` — OPS-07, OPS-10
- `review/10-recommendations.md` — REC-02 (Phase 1 portion)
- Phase 2 successor: CLOACI-I-0105
- Prior task: CLOACI-T-0526 (Docker Compose template + two-process runbook, completed)
