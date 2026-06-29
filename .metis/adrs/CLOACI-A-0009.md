---
id: 001-constructor-capabilities-tenant
level: adr
title: "Constructor capabilities: tenant-granted at construction time, default-closed"
number: 1
short_code: "CLOACI-A-0009"
created_at: 2026-06-29T16:10:54.220882+00:00
updated_at: 2026-06-29T16:18:58.499237+00:00
decision_date: 2026-06-29
decision_maker: dylan.storey
parent: 
archived: false

tags:
  - "#adr"
  - "#phase/decided"


exit_criteria_met: false
initiative_id: NULL
---

# ADR-1: Constructor capabilities: tenant-granted at construction time, default-closed

## Context **[REQUIRED]**

Constructors ([[CLOACI-I-0132]]) run as sandboxed WASM components (WASI, deny-by-default). The genuinely useful ones — HTTP fetch, database queries (TCP), file I/O, reading env/secrets — all require **host capabilities** the sandbox denies by default. Constructors are also potentially **third-party** (provider packages). We need a model for **who decides** what a given constructor *instance* may access, and **how that's enforced**, without trusting the constructor code itself.

fidius already enforces a **two-key gate** for egress: a capability is reachable only if (1) the wasm component *imports* it (`wasi:http`, `wasi:sockets`) **and** (2) the host supplies a policy that authorizes it (`EgressPolicy::authorize` for HTTP, `EgressPolicy::authorize_tcp` for TCP by host:port; both default-deny). Filesystem and env are the host-built WASI `WasiCtx` (preopened dirs, explicit env vars). The open question is the **policy layer above fidius**: who authors the authorization, at what point, and with what default.

## Decision **[REQUIRED]**

**Capabilities are granted by the tenant at construction/definition time, default-closed.**

- The grant is written into the **workflow definition**, alongside the constructor instantiation — a `grants = { http=[…], tcp=[…], fs=[…], env=[…] }` on `constructor!` / `#[constructor]` / `#[reactor]`. The **tenant** authors it; it is reviewed as part of the tenant's existing workflow **review/promotion** process.
- A constructor **cannot grant itself access.** The most it can do is *import* a capability type, which is **structurally visible in its wasm** and therefore auditable. The tenant authorizes the **specifics** (which host:port, path, env key).
- **Default-closed / fail-closed**: absent a grant, the capability is denied (fidius already default-denies TCP/UDP; cloacina supplies a deny-all policy until a grant says otherwise).
- Cloacina **translates** an approved grant into a fidius `EgressPolicy` (`authorize`/`authorize_tcp`) plus a scoped `WasiCtx` (preopens/env) at load — riding fidius's two-key enforcement rather than inventing a parallel one.
- **Globs are permitted** in the grant grammar (`http=["*"]`) for unconstrained "use at your own risk" cases; cloacina's own **core constructors follow strict least-access by convention** (their docs/examples grant the minimum).
- A tenant-wide **"blanket policy"** is explicitly deferred to the future secrets/variables work.

## Alternatives Analysis **[CONDITIONAL: Complex Decision]**

| Option | Pros | Cons | Risk Level | Implementation Cost |
|--------|------|------|------------|-------------------|
| **Tenant-granted at construction time (CHOSEN)** | Untrusted code can't self-grant; explicit, reviewable, least-privilege per instance; maps directly onto fidius's two-key gate; default-closed | Tenant must know what to grant (mitigated by structural import visibility + docs); grants restated per workflow until a blanket policy exists | Low | Low–Medium |
| Constructor self-declares needs + host allow-list intersect | Self-documenting | Bases enforcement partly on an *untrusted* self-declaration; redundant (imports already reveal capability types); a malicious manifest could over-ask to social-engineer a grant | Medium | Medium |
| Global/deployment-wide policy only | Central, one place | Too coarse; no per-instance least-privilege; premature without the secrets/variables surface | Medium | Medium |
| Trust constructors / no capability gate | Simplest | Defeats the sandbox — arbitrary third-party code gets ambient host access | High | Low |

## Rationale **[REQUIRED]**

- The constructor is **untrusted** (third-party providers). It must never be the *basis* of its own authorization. The **tenant** — who owns the deployment and the promotion process — is the right authority.
- fidius **already** enforces a two-key gate; tenant construction-time grants simply *are* the host-policy key, so this reuses existing enforcement instead of building a second one.
- Capability **types** are already structurally evident from the wasm imports (auditable), so a self-declared manifest field would be redundant for *enforcement* — it survives at most as advisory docs.
- **Default-closed + tenant review** delivers least-privilege with a human checkpoint, without per-object authZ machinery — consistent with "tenant is the isolation boundary" ([[CLOACI-I-0118]] / the tenant-isolation principle).

## Consequences **[REQUIRED]**

### Positive
- Untrusted constructors **cannot widen their own access**; grants are explicit, least-privilege, and reviewed at promotion.
- Reuses fidius's two-key enforcement — no parallel security mechanism.
- Capability use is **auditable from the wasm** (imports) and from the workflow source (grants).

### Negative
- The tenant must **author grants** (an ergonomics/learning cost — mitigated by deriving hints from the component's imports and by core-constructor docs).
- **No central policy yet** — every workflow restates its grants until the future blanket policy lands.

### Neutral
- **Globs are allowed** (the yolo path): the safety story for broad grants lives in *convention + review*, not the type system.
- A tenant-wide **blanket policy** + secrets-backed env values are deferred to the secrets/variables work.

## Review Schedule **[CONDITIONAL: Temporary Decision]**

### Review Triggers
- The secrets/variables work lands (introduces the tenant-wide blanket policy + secret-backed env).
- Per-object authZ is ever reconsidered (currently a permanent non-goal).

### Scheduled Review
- **Review Criteria**: whether construction-time grants + review remain sufficient as constructor usage and provider count grow.