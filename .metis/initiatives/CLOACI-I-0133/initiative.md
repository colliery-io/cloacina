---
id: tenant-credential-store-encrypted
level: initiative
title: "Tenant credential store — encrypted, named connection/secret references for packaged-workflow egress"
short_code: "CLOACI-I-0133"
created_at: 2026-07-07T11:11:32.196262+00:00
updated_at: 2026-07-07T11:11:32.196262+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: tenant-credential-store-encrypted
---

# Secrets — encrypted, named-field objects for workflows and constructors (the encrypted sibling of parameters)

> **Phase: discovery.** This document captures the problem, the maintainer-chosen shape, and the open questions. Design decisions, requirements freezing, and task decomposition are DEFERRED to a design check-in (initiative human-in-the-loop rule). Nothing here is committed to implementation.

## Context **[REQUIRED]**

Prompted by the "does Cloacina need Airflow's Variables + Connections?" question (2026-07-07). Analysis resolved two separate asks:

- **Variables (global mutable KV config): NO.** Cloacina already covers the legitimate use — [[CLOACI-I-0116]] parameterized workflow instances (declared, typed, bound-per-named-instance params delivered via `Context`) and [[CLOACI-I-0128]] declared inputs (typed, named, JSON-Schema-validated). Critically, I-0116 params are **immutable snapshots by design** (decision #3: snapshot at register, re-register to change) — a deliberate rejection of the Airflow-Variables failure mode (a global value edited in a UI → silent "it worked Tuesday" config drift). Adding mutable global Variables would fight the architecture. Not pursued.

- **Connections (named credential/endpoint records): a REAL gap — but the shape should be generic Secrets, not Airflow-style typed Connections (maintainer call, 2026-07-07).**

### The gap

There is **no credential/secret store today** (only API keys). A packaged workflow or constructor running in service mode that needs to reach a tenant's Postgres / S3 / Kafka / HTTP API has two bad options:
1. Smuggle the credential through instance params — which land **plaintext** in the `schedules.params` row AND in the `Context` (visible in the fires log / execution history). A real footgun that exists today.
2. Bake it into the package — worse (shared, unrotatable, in the artifact).

In embedded mode this is a non-issue (the host app supplies its own DB pool / clients). The gap is specifically **service / multi-tenant mode with packaged workflows on the agent fleet**.

### The chosen shape — a generic Secret, the encrypted sibling of a parameter

Rejecting Airflow's rigid typed-Connection taxonomy (`postgres`/`http`/`aws`/… — a maintenance burden that never fits everyone), a **Secret** is:

- a **named object** with **named fields** (a `{field: value}` map — e.g. `db_prod = { host, user, password, sslmode }`, `stripe = { api_key }`), so a DB connection, an API token, or anything else fits the same primitive;
- **encrypted at rest** (reusing the existing `crypto/key_encryption.rs` substrate — the same one the OIDC refresh-token store uses);
- **tenant-scoped** (schema-per-tenant → tenant isolation for free);
- **referenced by name** — so rotating the underlying value never touches code or instances (the one genuine Airflow-Connection win, preserved).

Mentally: **"params you can see" vs. "secrets you can't."** Same shape (named object, named fields, optional declared schema), one encrypted. Authors already understand declared params; secrets become their encrypted sibling.

### The one thing that makes it MORE than "encrypted params" — the resolution boundary (the whole security point)

Params land in the `Context` in plaintext and appear in the fires log. A secret's plaintext must **NEVER** touch the `Context`, the `schedules` row, or logs. So Secrets are *shaped* like params but *delivered differently*: a task/constructor references a secret **by name**, and the runtime **resolves + decrypts it into the execution scope at the last possible moment — ideally inside the agent/sandbox** — never serializing it into the durable context or history. This is the load-bearing design constraint.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- A first-class **Secret**: a named, tenant-scoped object of named fields, encrypted at rest, CRUD-managed (create/rotate/list-metadata/delete) — list/metadata NEVER returns plaintext.
- **Resolution without leakage**: a workflow/task/constructor names the secret(s) it needs; the runtime resolves + decrypts into the execution scope at the last moment. Plaintext never enters `Context`, `schedules.params`, the fires log, audit rows, or execution history.
- **Composition with existing surfaces**: an instance param may hold a *secret reference* (e.g. `{"$secret": "db_prod"}`) so binding a secret to a named instance mirrors binding a param — resolved encrypted at fire time.
- **Grant-gated access**: which secrets a constructor/package may resolve is authorized (ties into the constructor capability/egress grants — the credential is the thing behind a granted egress endpoint).
- **Rotation** by name with no code/instance changes.
- **Parity across embedded (optional — host may not need it), packaged, and fleet-agent execution paths.**

**Non-Goals:**
- **Airflow-style Variables** (global mutable KV) — explicitly rejected; use parameterized instances / declared inputs.
- **Typed Connection taxonomy** (per-backend Connection types) — the generic named-fields Secret subsumes it.
- A general-purpose external secrets-manager *replacement* (Vault/AWS SM) — though an external-backend *provider* is a candidate follow-on (see OQ).
- Changing the immutable-snapshot semantics of I-0116 params.

## Requirements (DRAFT — freeze at design) **[CONDITIONAL: Requirements-Heavy]**

- REQ-001: A `secrets` store, tenant-scoped, holding `(name, {field: encrypted_value})`, encrypted at rest via `crypto/key_encryption`.
- REQ-002: CRUD API (create/rotate/list-metadata/delete). **List/get returns metadata only (names, field names, timestamps) — NEVER plaintext values.**
- REQ-003: A resolution path that decrypts a named secret into the execution scope at fire/run time and delivers it to the consuming task/constructor WITHOUT it entering `Context` / `schedules.params` / logs / audit / fires history.
- REQ-004: A consumer-facing way to declare + reference required secrets (Rust + Python + packaged FFI), validated against a declared shape where present (reuse I-0128 declared-input machinery).
- REQ-005: Secret references composable into instance params (`{"$secret": name}`) resolved at fire time.
- REQ-006: Access is grant-gated — a package/constructor resolves only the secrets its tenant granted it.
- NFR-001 (**the load-bearing one**): plaintext appears ONLY transiently in the execution scope; a leak test proves no plaintext in DB rows, logs, audit events, or the fires log.
- NFR-002: rotation of a secret's value takes effect on the next fire with no code/instance edit.
- NFR-003: fleet-agent parity — a secret resolves for a packaged task running on a remote agent (the agent receives the resolved value over the existing authenticated delivery channel, not the ciphertext-at-rest key).

## Use Cases **[CONDITIONAL: User-Facing]**

### UC1 — Packaged workflow reaches a tenant DB
- **Actor:** tenant author/operator.
- **Scenario:** creates secret `db_prod = { host, user, password }`; a packaged task declares it needs `db_prod`; registers/executes. At run time the task receives the resolved fields; nothing plaintext lands in the schedule row or fires log.
- **Outcome:** the task connects; ops rotate the password by updating `db_prod`, no redeploy.

### UC2 — Constructor with egress + credential
- **Actor:** provider author + consuming tenant.
- **Scenario:** an HTTP-poll trigger constructor is granted egress to `api.example.com` AND resolution of secret `example_api = { token }`. The tenant grants both; the constructor resolves the token inside its WASM/agent scope.
- **Outcome:** the credential behind the granted egress is named, encrypted, rotatable — never in the package or params.

### UC3 — Secret reference bound to a named instance
- **Actor:** operator.
- **Scenario:** `sync_file` instance `sync_prod` binds `dst_credentials` as a `{"$secret": "s3_prod"}` param reference; each fire resolves it encrypted.
- **Outcome:** the same instance model as plaintext params, but the secret stays encrypted end to end.

## Architecture (SKETCH — not decided) **[CONDITIONAL: Technically Complex]**

### Reuse of existing substrate (the reason this is M, not L)
- **Encryption at rest:** `crates/cloacina/src/crypto/key_encryption.rs` (already encrypts the OIDC refresh-token store).
- **Tenant isolation:** schema-per-tenant → a `secrets` table per tenant schema, isolated for free.
- **Grants / capability + egress:** `registry/loader/constructor_loader.rs` `load_wasm_configured_with_grants` + `grants::{translate, ResolvedGrants}` — the insertion point for "may resolve secret X".
- **Declared-input machinery (I-0128):** a secret's shape is a declared schema; a consumer declares required secrets like it declares params.
- **Delivery substrate:** the fleet delivery channel (already authenticated) carries the RESOLVED value to an agent — the agent never holds the at-rest key.

### The divergence from params (the security seam)
Params: authored → stored plaintext (`schedules.params`) → merged into `Context` → visible in fires log. Secrets: authored → stored **encrypted** → **resolved + decrypted at the execution boundary into a side channel the task reads (NOT `Context`)** → never persisted/logged. Designing that side channel (a `Secrets` accessor on the task/constructor scope, distinct from `Context`) is the core technical question.

## Alternatives Considered **[REQUIRED]**

- **Airflow Variables (global mutable KV).** Rejected — reintroduces the config-drift failure mode I-0116 deliberately designed out; instances + declared inputs already cover the legitimate use.
- **Airflow-style typed Connections (per-backend Connection records).** Rejected in favor of the generic named-fields Secret — no per-backend taxonomy to maintain, and it matches the existing params mental model.
- **"Just encrypted params"** (store params encrypted, still delivered via `Context`). Rejected — the plaintext would still surface in `Context`/fires log at fire time. The value is precisely the separate resolution boundary; without it this buys nothing over params.
- **External secrets-manager only (Vault/AWS SM), no native store.** Deferred, not rejected — a pluggable external backend is a candidate follow-on, but a native encrypted store is needed for the embedded/self-contained posture and as the default.
- **Bake credentials into packages / pass via params.** The status quo footgun this initiative closes.

## Open Questions (resolve in discovery/design) **[REQUIRED]**

- **OQ-1 — the delivery side channel.** How does a task/constructor READ a resolved secret without it being in `Context`? A `Secrets`/credential accessor injected into the execution scope? What's the Rust + Python + packaged-FFI surface?
- **OQ-2 — fleet parity + key custody.** The agent must get the *resolved* value, not the at-rest key. Resolve server-side and ship over the (authenticated, TLS) delivery channel? Implications for the agent never persisting it.
- **OQ-3 — grant model.** Is "may resolve secret X" a new grant kind alongside capability/egress, or does it ride the existing egress grant (the credential behind an endpoint)? Named-secret grants vs. wildcard.
- **OQ-4 — declaration + reference surface.** How does a workflow/task/constructor DECLARE required secrets, and reference fields? Reuse I-0128 declared inputs with an `encrypted: true` marker? The `{"$secret": name}` param-reference form.
- **OQ-5 — rotation + versioning.** In-place value update vs. versioned secrets; what a mid-flight execution sees during rotation.
- **OQ-6 — encryption key management.** Per-tenant data keys vs. a single server key; envelope encryption; interaction with the existing `db_key_manager` / key-encryption scheme.
- **OQ-7 — pluggable external backend** (Vault/AWS SM/GCP SM) as a resolution provider behind the same named-Secret interface — in scope for v1 or explicit follow-on?
- **OQ-8 — UI/CLI surface** for create/rotate/list (metadata-only). Ties into the embedded UI + cloacinactl.

## Implementation Plan (PLACEHOLDER — decompose after design sign-off) **[REQUIRED]**

Not decomposed. Rough spine once design is frozen: (1) encrypted `secrets` store + tenant DAL + CRUD API (metadata-only reads); (2) the resolution side channel + no-leak guarantee (the NFR-001 leak test is the gate); (3) declaration/reference surface (Rust + Python + packaged FFI) reusing I-0128; (4) grant integration; (5) instance-param `{"$secret"}` composition; (6) fleet-agent resolution parity; (7) UI/CLI + docs. **Do not start until a design check-in freezes OQ-1..OQ-4.**

## Related
- [[CLOACI-I-0116]] — parameterized instances (the plaintext sibling; secret references compose into instance params).
- [[CLOACI-I-0128]] — declared inputs (the declaration/validation machinery to reuse).
- [[CLOACI-I-0132]] — constructors + capability/egress grants (the credential is the thing behind a granted egress).
- [[project_tenant_is_isolation_boundary]] — tenant is the isolation boundary; secrets are tenant-scoped by construction.