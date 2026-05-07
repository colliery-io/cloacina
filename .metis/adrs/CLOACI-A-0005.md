---
id: 005-deployment-mode-trust-model
level: adr
title: "Deployment-mode trust model: hobbyist daemon vs enterprise server"
number: 5
short_code: "CLOACI-A-0005"
created_at: 2026-05-06T11:52:08.189221+00:00
updated_at: 2026-05-06T12:05:00.705135+00:00
decision_date:
decision_maker:
parent:
archived: false

tags:
  - "#adr"
  - "#phase/decided"


exit_criteria_met: false
initiative_id: NULL
---

# ADR-0005: Deployment-mode trust model: hobbyist daemon vs enterprise server

## Context

Cloacina ships in two deployment modes (per CLOACI-V-0001):

1. **Embedded library / daemon** — embedded into Rust/Python applications, or run as `cloacinactl daemon` for local-dev / single-user workflows.
2. **Server** — multi-tenant HTTP+WebSocket service distributed as `cloacina-server` and `cloacina-compiler`.

The May 2026 seven-lens code review surfaced a cluster of findings around plugin loading, signature verification, sandboxing, and multi-tenant scoping that span both deployable surfaces. Without a single architectural premise, every initiative has to relitigate the question — should the daemon verify signatures? should the runner be tenant-scoped in daemon mode? should we sandbox daemon plugin loads? — and the implementation will drift.

This ADR codifies the trust premise so all downstream initiatives can scope cleanly.

## Decision

### 1. Two distinct trust modes, by deployable

**Daemon (and embedded library) = high-trust.**

- Audience: hobbyists, individual developers, single-user / single-tenant deployments. Home-computer use.
- Inputs: operator-controlled. Authors and operators are the same person (or a trusted small group).
- No untrusted upload surface. The daemon does not accept arbitrary `.cloacina` packages from third parties.
- Therefore: no signature verification, no plugin sandboxing, no multi-tenant runner abstraction. The daemon is a single trust domain.

**Server = low-trust uploads, multi-tenant, enterprise.**

- Audience: platforms, enterprise teams, implementation partners running managed services.
- Inputs: untrusted package uploads from many tenants. Authors are not always operators.
- Multi-tenant by default with schema isolation. Strong tenant boundaries enforced.
- Signature verification is **opt-in via configuration** (`--require-signatures`). Default accepts unsigned uploads from authorized keys. Operators choose their own posture; the framework provides the knobs.
- Build sandboxing applies to the build worker (`cloacina-compiler`), not the runtime.

### 2. Server is Linux-only, strongly opinionated

The server's deployment target is Linux. Server-only features (sandbox primitives, OS-level resource limits, namespace isolation) may freely depend on Linux primitives without portability hedging.

The daemon remains portable across Linux, macOS, and Windows (within library-side platform support); the threat model permits it.

### 3. Server build sandbox primitive: bubblewrap + landlock

The `cloacina-compiler` build worker isolates each `cargo build` using:

- **bubblewrap (`bwrap`)** — process / network / mount namespaces, tmpfs build root, RO bind mounts for vendored cargo registry and read-only system paths.
- **landlock** as defense-in-depth where the kernel supports it (Linux 5.13+).

Implementation tracked under CLOACI-I-0105.

### 4. Daemon plugin runtime is not isolated, by design

The daemon (and the in-process runtime path) loads cdylibs via `dlopen`. They run in-process with full daemon privileges. This is acceptable because:

- The daemon is high-trust by definition — no untrusted authors.
- The same posture applies to any Rust application loading native plugins.
- Operators who want process-level isolation can run the daemon under their own confinement (UID, container, namespace) at deployment time.

This is **not a security gap** when read against the trust model; it is a property of single-trust-domain deployment.

## Alternatives Analysis

| Option | Pros | Cons | Risk | Cost |
|--------|------|------|------|------|
| **One trust model spanning both deployables** | Uniform code paths | Forces daemon to handle multi-tenancy, sig verification, sandboxing it doesn't need; doubles test surface; slows hobbyist authoring | High (over-engineering) | High |
| **Server portable across OSes** | Wider OS support for service mode | Forces sandbox primitives to lowest-common-denominator (chroot, container-spawn); sacrifices isolation strength; pushes operational complexity into codebase | High | High |
| **Sig verification on by default (server)** | Stronger default posture | Makes day-1 operator onboarding harder; doesn't match enterprise reality where operators run their own key infra on their own timeline | Medium | Medium |
| **Container-spawn sandbox primitive** | Strongest theoretical isolation | Compiler becomes container orchestrator (docker socket); image trust as new attack surface; ~100×–1000× startup overhead vs bwrap | High | High |
| **No daemon mode** | Simpler product surface | Removes the embedded/local-dev story central to the Vision audience definition | High (audience loss) | Low |
| **Chosen: split trust model + Linux-only server + bwrap/landlock** | Decidable rules, smaller initiatives, Linux-native primitives | Documentation must communicate two trust modes clearly; some operators expect a "secure daemon" | Low | Low |

## Rationale

The two deployment modes already serve distinct audiences (per CLOACI-V-0001's "Target Audiences"). Aligning the trust model to the audience eliminates the contradiction of treating both deployables as if they faced the same threats. The server is the place where Cloacina meets untrusted code; the daemon is where Cloacina meets a single trusted user. Decisions follow from there.

For the sandbox primitive: the server's deployment target is Linux, so portability constraints don't apply. bubblewrap is small, well-vetted (Flatpak runs it billions of times daily), in every major distribution, and gives unprivileged user namespaces with tmpfs and RO bind mounts — exactly the surface needed. Container-spawn would invert the trust model (compiler needs container-runtime privileges); nsjail is less common and adds operational unfamiliarity; chroot alone is insufficient. Landlock pairs cleanly with bwrap as filesystem defense-in-depth on supported kernels.

## Consequences

### Positive

- Architectural rules are now decidable. "Should X be tenant-scoped?" → server: yes, daemon: no.
- Initiative scope shrinks. Sig verification (CLOACI-I-0103), multi-tenant abstraction (CLOACI-I-0106), and compiler sandboxing (CLOACI-I-0104, CLOACI-I-0105) are server-only. The daemon path doesn't have to be reconciled.
- Server-only Linux-only stance unblocks bwrap, landlock, and other Linux primitives without portability arguments.
- The daemon stays a portable, low-friction tool for hobbyist authoring.

### Negative

- Documentation and marketing must communicate the two trust modes clearly. Operators running a daemon for "local production" use must understand they're outside the server's threat model.
- Some review findings that look critical when read against a "server-equivalent daemon" framing become non-issues when read against this trust model. The review record should be annotated explicitly so it stays honest.

### Neutral

- The compiler service is server-side infrastructure. Daemon users who build packages locally do so via `cloacinactl package build` without invoking the compiler service.
- Existing daemon code paths that mention or partially handle multi-tenancy are out-of-spec and may be removed; that is evolvability cleanup, not feature loss.
