---
id: constructor-capability-and-egress
level: specification
title: "Constructor capability and egress model"
short_code: "CLOACI-S-0014"
created_at: 2026-06-29T16:11:01.810350+00:00
updated_at: 2026-06-29T16:11:01.810350+00:00
parent: CLOACI-I-0132
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Constructor capability and egress model

## Overview **[REQUIRED]**

How a **tenant** grants a constructor *instance* scoped access to host capabilities (HTTP, TCP, filesystem, env) **at construction time**, how cloacina translates those grants to fidius/WASI enforcement, and the default-closed / fail-closed guarantees. Implements the decision in [[CLOACI-A-0009]]. This is the layer that turns constructors from pure-compute-only into the genuinely useful set (http fetch, db query, file/env) without trusting constructor code.

## System Context **[CONDITIONAL: System-Level Spec]**

### Actors
- **Constructor author**: writes the constructor logic; *may import* capability types (`wasi:http`, `wasi:sockets`) — structurally visible in the wasm. **Does not grant access.**
- **Tenant workflow author**: writes `grants = {…}` at the constructor instantiation; reviewed via the tenant's promotion process.
- **Cloacina loader**: reads grants, validates, translates to a fidius `EgressPolicy` + scoped `WasiCtx`, loads the constructor.
- **fidius / WASI runtime**: enforces (two-key: component imports + host policy).

### External Systems
- **fidius-host**: `EgressPolicy::authorize` (HTTP, per-request), `EgressPolicy::authorize_tcp` (TCP, per-connect by host:port; default-deny), the `WasiCtx` builder (preopens, env). Supplied via `PluginHost::builder().egress(..)` / `from_component_bytes_with_egress`.
- **The constructor wasm component**: its imports declare which capability *types* it can use.

### Boundaries
In scope: the grant grammar, the cloacina→fidius translation, enforcement, the v1 capability vocabulary. Out of scope: secrets storage/injection (future), a tenant-wide blanket policy (future), per-object authZ (permanent non-goal).

## Requirements **[REQUIRED]**

### Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.1.1 | A constructor instance receives a capability **only** via the tenant's `grants` at construction time; absent a grant it is denied (default-closed). | Untrusted code can't self-grant ([[CLOACI-A-0009]]). |
| REQ-1.1.2 | Grant grammar: `http`, `tcp`, `fs`, `env`, each a list of scoped patterns (`host:port` / URL, `host:port`, path, key). **Globs allowed** (`http=["*"]`). | Cover the near-term need; globs = use-at-your-own-risk. |
| REQ-1.2.1 | Translate `http` → `EgressPolicy::authorize` (match request host/path vs allowed patterns); `tcp` → `authorize_tcp` (match host:port); `fs` → `WasiCtx` preopened dirs; `env` → `WasiCtx` env vars. | Ride fidius's two-key enforcement. |
| REQ-1.2.2 | Enforcement is fidius **two-key**: a capability is reachable only if the component imports it AND the grant authorizes it; either missing → deny. | Belt-and-suspenders default-closed. |
| REQ-1.3.1 | Cloacina can inspect a component's imports and SHOULD emit a **load-time lint** when a constructor imports a capability with no matching grant (advisory; runtime still fails closed). | Usability — surface "this needs http you didn't grant" early. |
| REQ-1.3.2 | Core/built-in constructors are authored + documented with **strict least-access** example grants. | Convention sets the norm; broad grants are the exception. |

### Non-Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| NFR-1.1.1 | Untrusted constructor code **cannot widen its own access** — enforcement is host-side. | Core security property. |
| NFR-1.1.2 | **Fail-closed**: any ungranted capability access denies at runtime. | No silent ambient access. |
| NFR-1.1.3 | Grants are **auditable** in the workflow source (reviewed at promotion) and capability *types* auditable from the wasm imports. | Review is the trust anchor. |

## Architecture Framing **[CONDITIONAL: System-Level Spec]**

### Decision Area: Gating authority + default
- **Context**: who authorizes a constructor's host access, and what's the default.
- **Constraints**: fidius two-key (imports + host policy); TCP/UDP default-deny; HTTP per-request; WASI `WasiCtx` for fs/env.
- **Required Capabilities**: per-instance, tenant-authored, default-closed grants translated to fidius.
- **ADR**: [[CLOACI-A-0009]].

## Decision Log **[CONDITIONAL: Has ADRs]**

| ADR | Title | Status | Summary |
|-----|-------|--------|---------|
| CLOACI-A-0009 | Constructor capabilities: tenant-granted at construction time, default-closed | draft | Tenant grants at construction; constructor can't self-grant; default-closed; translated to fidius EgressPolicy + WasiCtx. |

## Constraints **[CONDITIONAL: Has Constraints]**

### Technical Constraints
- fidius two-key model: component must import the capability AND the host policy must authorize.
- `EgressPolicy::authorize` (HTTP) + `authorize_tcp` (TCP host:port); TCP/UDP default-deny.
- Filesystem + env via the host-built WASI `WasiCtx` (preopens + explicit env).
- **Open**: confirm fidius-host exposes the `WasiCtx` fs/env configuration surface (http/tcp confirmed).

## Open Items

**Resolved (2026-06-29, design review):**
- **Load-time capability lint → IN v1.** Inspect the component's imports vs the grants at load and warn on any imported capability with no matching grant. Chosen deliberately for v1: deferring it risks coding the loader into a corner that's hard to retrofit enforcement into later.
- **`grants` syntax → consistent everywhere.** Identical look/feel across `constructor!`, `#[constructor]`, `#[reactor]`, and the Python/cloaca path — non-negotiable.
- **Sequencing → capability layer before ALL seeds.** The seed library ([[CLOACI-T-0825]]) is gated on this layer (not just the networked seeds).

**Remaining:**
- Verify fidius-host exposes the `WasiCtx` fs/env knobs (preopens, env) — expected available (~95%); confirm in code (http/tcp confirmed present).
- `udp` — deferred unless a concrete need appears.
- Secrets-backed `env` values + the tenant-wide **blanket policy** — deferred to the secrets/variables work (this spec assumes literal/env-sourced values for now).
