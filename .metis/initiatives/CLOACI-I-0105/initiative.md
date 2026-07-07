---
id: compiler-hardening-phase-2-process
level: initiative
title: "Compiler hardening Phase 2 — process sandbox for cargo build"
short_code: "CLOACI-I-0105"
created_at: 2026-05-06T11:05:32.632631+00:00
updated_at: 2026-07-07T04:29:37.471281+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: compiler-hardening-phase-2-process
---

# Compiler hardening Phase 2 — process sandbox for cargo build Initiative

## Context

Phase 1 (CLOACI-I-0104) constrains *cost* and *configuration surface* for `cloacina-compiler` builds. It does not isolate the build process. A malicious `build.rs` can still read the host filesystem, exfiltrate `DATABASE_URL`, contact internal services that are reachable from the compiler's network, and write files outside the build directory.

This initiative covers the second phase of REC-02: choose a sandbox primitive and integrate it so each `cargo build` runs in an unprivileged namespace with a tmpfs root, restricted filesystem access, and a constrained network policy.

The primitive choice is deliberately deferred to discovery — it is the central design decision and gates all implementation. The leading candidates have different deployment-posture and portability tradeoffs.

## Goals & Non-Goals

**Goals:**
- Each build runs in a process namespace that cannot read host paths beyond a curated read-only mount + a writable tmpfs build root.
- Build process has no outbound network access except a curated cargo registry endpoint (or none, if Phase 1's `--offline` defaults already cover us).
- Build process cannot send signals to or otherwise interact with sibling processes on the host.
- A forensics path exists: a build's exit, signal status, peak memory, and total syscalls are logged.

**Non-Goals:**
- Sandboxing for non-Linux operators in the first delivery — Linux-only is acceptable for v1; document portability constraints.
- Reworking the build queue or claim model.
- Sandboxing of the `cloacina-server` runtime (out of scope; the workflow runtime is a separate trust boundary).

## Source Findings (May 2026 review)

- **SEC-06 (Critical)** — `cargo build` runs unsandboxed on attacker source; `build.rs` is RCE on the build host.
- **OPS-07 (Critical-adjusted)** — Compiler service is unsandboxed in multi-tenant deployments.

## Decision (per CLOACI-A-0005)

**Primitive: bubblewrap (`bwrap`) + landlock as defense-in-depth.**

- bubblewrap provides process/network/mount namespaces, tmpfs build root, and RO bind mounts for vendored cargo registry and system paths. Small, well-vetted (Flatpak runs it at billions of invocations per day), in every major distro.
- landlock layers kernel-level filesystem ACLs on top where the kernel supports it (Linux 5.13+).
- Linux-only is acceptable: per CLOACI-A-0005 the server is Linux-only by deployment posture; no portability hedging required.
- Container-spawn was rejected (compiler becomes container orchestrator with docker socket; image trust as new attack surface; ~100×–1000× startup overhead vs bwrap).

## Open Discovery Questions

- **Tmpfs sizing** — what's the upper bound? Some builds are large (target/ can be GB-scale).
- **Cache hand-off** — Phase 1 uses a curated `~/.cargo/registry`. How does the sandboxed build see it? Bind mount RO?
- **Network policy** — `--offline` Phase 1 closes most uses; sandbox network policy is then defense-in-depth. Should we remove network entirely or keep an audit-logged egress for the cargo registry?
- **Forensics depth** — what level of detail (cgroup stats? strace? eBPF?) is appropriate vs paranoid?
- **Dev portability** — Linux is fine for the compiler service. But does the dev workflow need an unsandboxed mode for laptop dev?

## Initial Sketch

(All subject to primitive decision in discovery.)

- ADR documenting the primitive choice.
- Wrap `cargo build` invocation in the chosen primitive (e.g., `bwrap --unshare-all --bind <vendor> /vendor --tmpfs /build --chdir /build --ro-bind /usr /usr --proc /proc cargo build ...`).
- Vendor cache mounted RO; build root is tmpfs with size limit.
- Network policy: closed by default; document workflow for in-house registries.
- Audit log includes sandbox configuration hash, exit signal/status, peak memory.
- Integration test that submits a build whose `build.rs` attempts to read `/etc/hostname`, expects failure.

## Design (2026-07-07, maintainer check-in) — isolation ladder, fail-closed

**`CLOACINA_COMPILER_SANDBOX = required | preferred | off`** (the REQ-008 pattern: explicit selection, boot-time probe, loud failure):
- **Level 1 — bwrap**: full namespaces (`--unshare-all`), tmpfs build root (size-capped, configurable), RO binds for the toolchain + curated cargo registry, **network fully closed** (Phase 1 `--offline` covers the registry; no audit-logged egress — resolved).
- **Level 2 — landlock + rlimits**: kernel FS ACLs applied via `pre_exec` on the spawned cargo; works unprivileged in containers (kernel ≥5.13).
- **off**: dev laptops (macOS); logged loudly.
`required` refuses to build below level 1 (multi-tenant posture). Every build's audit row records the ACHIEVED level + sandbox config hash + exit/signal/peak-RSS (rusage-level forensics — no eBPF, resolved).

**Container posture (maintainer decision)**: the containerized compiler gets level 1 by relaxing the container's seccomp for user namespaces — compose/Helm set the documented `security_opt`; defensible because bwrap's per-build isolation is stronger than default-seccomp-with-no-sandbox. Discovery Qs resolved: tmpfs default 4GiB configurable; registry RO-bind; dev unsandboxed mode = `off`.

**Decomposition**: [[CLOACI-T-0852]] ladder/probe/config seam · [[CLOACI-T-0853]] bwrap level · [[CLOACI-T-0854]] landlock+rlimits+forensics · [[CLOACI-T-0855]] adversarial test + compose/Helm/docs.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- `cargo build` runs in a process namespace with no host filesystem access beyond a curated RO mount + writable tmpfs.
- Adversarial integration test confirms `build.rs` cannot read host paths or open network connections.
- Audit log records sandbox config + exit details for each build.
- ADR documents the primitive decision and its tradeoffs.
- `production-deployment.md` updated to reflect Phase 2 posture.

## References

- ADR: CLOACI-A-0005 (deployment-mode trust model — locks bubblewrap + landlock as the chosen primitive; server is Linux-only)
- `review/07-security.md` — SEC-06
- `review/06-operability.md` — OPS-07
- `review/10-recommendations.md` — REC-02 (Phase 2/3 portion)
- Phase 1 predecessor: CLOACI-I-0104