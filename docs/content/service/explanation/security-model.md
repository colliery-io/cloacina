---
title: "Security Model"
description: "Trust by deployment mode, auth roles, signature verification rationale, the compiler threat model, and the limits of multi-tenant isolation."
weight: 35
aliases:
  - "/platform/explanation/security-model/"

---

# Security Model

Cloacina's security posture is **set by deployment mode, not by configuration**. The threat model the library handles in embedded / daemon mode is fundamentally different from the threat model the server handles, and the codebase treats them as two different products. This page is the operator-facing version of that distinction — what the system protects against in each mode, what it does not, and why.

The authoritative architectural decision lives in **CLOACI-A-0005** (Deployment-mode trust model). This page is the prose form.

## Trust by deployment mode

### Embedded library (in-process)

The `cloacina` crate linked into a Rust binary inherits that binary's trust boundary entirely. There is no auth layer, no signature verification, no multi-tenant isolation surface — the workflows are *the host application's code*. A workflow author here is the same person as the application author; an attacker who can replace a workflow can also replace the binary.

Threat-model posture:

- **In scope:** correctness and durability — workflow restarts, deferred-task semantics, atomic completes, claim/heartbeat coordination, deterministic finalization (CLOACI-I-0110).
- **Out of scope:** authentication, package authenticity, tenant isolation, signed-binary attestation.

Configuration is by API surface (`DefaultRunnerConfigBuilder`), not by privilege boundary. There is no "admin" vs "user" in embedded mode.

### Local daemon (`cloacinactl daemon`)

The daemon adds *packaging* (`.cloacina` cdylibs loaded from a watch directory) but is still single-operator. The operator owns the host, owns the watch directory, and owns the cdylibs. There is no remote upload surface; the daemon does not expose an HTTP API; the only external surface is a Unix-domain health socket on `~/.cloacina/daemon.sock`.

Threat-model posture:

- **In scope:** same as embedded, plus FFI-vtable invariants (`cloacina-workflow-plugin` magic-byte ABI hash check rejects mismatched cdylibs at load).
- **Out of scope:** untrusted package authors. The daemon assumes everything in the watch directory was put there by the operator.

The daemon does not require `/metrics`, does not require auth, and does not enforce signature verification — those are server concerns.

### Server (`cloacina-server` + `cloacina-compiler`)

The server is the **only** Cloacina mode with a low-trust posture. It assumes:

- Multiple independent tenants share the same Postgres instance.
- Multiple operators authenticate via API keys with different privilege levels.
- Package authors may not be the same people as server operators.
- Package code is *not* trusted to be benign — the compiler-build pipeline is hardened (per CLOACI-I-0104) and signatures can be required at upload (CLOACI-I-0103).

This is where the threat model gets real, and the rest of this page is about what the server does to enforce it.

## Server auth model

### API keys, bearer-token, and the cache

Every authenticated route under `/v1/*` requires an `Authorization: Bearer <key>` header. The full validation flow:

1. Extract the bearer token.
2. SHA-256 hash it.
3. Check the LRU auth cache (256 entries, 30-second TTL).
4. On miss, validate against the database; insert on success.
5. Inject an `AuthenticatedKey` into request extensions, carrying the key's `role` and tenant scope.

The cache is small and short-lived deliberately: it shaves the per-request DB hit at high throughput, but the 30-second TTL bounds how long a revoked key can keep working. Explicit revocation via `DELETE /v1/auth/keys/{key_id}` clears the *entire* cache (not just the revoked entry) so revocation is immediate at the cost of a brief revalidation spike.

To create, list, and revoke keys in practice, see [Manage API Keys]({{< ref "/service/how-to/manage-api-keys" >}}).

### Roles vs the `is_admin` god-mode

Cloacina keys carry two pieces of authority: a **tenant-scoped role** (`admin` / `write` / `read`) and an **`is_admin` flag** that grants god-mode across all tenants.

- **`is_admin` keys** can do anything: create tenants, list tenants, create keys for any tenant, delete tenants, mint new admin keys. These should be issued sparingly and rotated regularly. The bootstrap key generated on first server startup is `is_admin`; production deployments rotate it immediately.
- **Tenant-scoped keys** are sandboxed to a single tenant. Within that tenant, the `role` discriminates `admin` (manage tenant resources) / `write` (execute workflows, upload packages) / `read` (list / inspect only). They cannot enumerate or affect other tenants.

The `is_admin = false` + `role = admin` combination — i.e., "I'm admin within my own tenant" — is the expected production posture for tenant operators. Reserve `is_admin = true` for the operations team running the platform.

### Tenant-access enforcement

Every `/v1/tenants/{tenant_id}/...` route checks `auth.can_access_tenant(&tenant_id)` before doing any work, returning `403 tenant_access_denied` if the caller's key isn't authorized for the named tenant. Tenant-scoped keys' single permitted tenant is set at key creation and cannot be changed; cross-tenant access requires a new key.

### Bootstrap key invariants

The first server startup is a chicken-and-egg situation — there's no key yet, and you need a key to create one. Cloacina solves it by **generating an `is_admin` bootstrap key on first startup if no API keys exist in the database**, writing the plaintext to `~/.cloacina/bootstrap-key` (mode 0600) exactly once.

The invariant: the file is written *exactly* once. If `~/.cloacina/bootstrap-key` already exists, the server refuses to overwrite it. This prevents the "I restarted the server and now I have a different admin key in the file" surprise that would otherwise be a confusing footgun. Operators who lose the file must read the existing key from the database directly.

In production, set `--bootstrap-key` (or `CLOACINA_BOOTSTRAP_KEY`) to pin the initial key from a secrets manager rather than relying on the auto-generated path — the file write is then skipped.

## Package signature verification

### What it is, why it's optional

When the server is started with `--require-signatures` + `--verification-org-id <UUID>`, every package upload runs through cryptographic signature verification before the package is registered. The verification:

1. Resolves the package signer for the target tenant.
2. Looks up the trusted public key for the configured org ID.
3. Verifies the package bytes against the signature stored in the trust database.
4. Logs the outcome (success or failure) to the audit log with the package fingerprint.

The pair is **fail-fast** — if `--require-signatures` is set but `--verification-org-id` is unconfigured, the server refuses to start (`signature_verification_unconfigured`). There is no soft-fail path: a misconfigured signature pipeline means *no uploads are accepted*, not "uploads accepted unverified."

It's optional because the threat models diverge:

- **Single-org deployments** where all package authors are trusted operators don't need it. The compiler-build path (CLOACI-I-0104) already enforces sandboxing.
- **Multi-org deployments** where tenants upload their own packages need it. Without signature verification, a tenant-write key (which can upload packages) becomes equivalent to a code-execution capability on the server host.

Reach for `--require-signatures` whenever the package author is not the server operator.

### What it doesn't protect against

Signature verification proves *who* signed the package, not *what's in the package*. A signed malicious package still runs malicious code; signatures don't sandbox. The sandboxing is the compiler's job (see below). Use both together.

## The compiler threat model

`cloacina-compiler` runs `cargo build` on packages submitted by — possibly untrusted — package authors. This is the most dangerous code path in the entire system: arbitrary Rust code is being compiled, which means arbitrary build scripts (`build.rs`) and arbitrary proc-macro code can run with the compiler's privileges.

CLOACI-I-0104 (Phase 1 hardening) added defenses:

- **Configurable build timeout** (`--build-timeout-s`) bounds the worst-case wall-clock cost of any single build. Past timeout, the build is marked `timed_out` and the stale-build sweeper reclaims the row.
- **`--frozen --offline` defaults.** Cargo runs against a curated, pre-vendored registry; no network access during build. This blocks the dominant exfiltration / dependency-substitution vector.
- **`setrlimit` per-build resource caps** (CPU, memory, file descriptors, processes). Linux only.
- **Interim deployment posture documented:** unprivileged UID, no outbound network beyond the curated vendor paths, no admin credentials beyond the build-claim DB user.

CLOACI-I-0105 (Phase 2) will add process-level sandboxing (Linux namespaces); until then, the operational posture in [Running the Compiler]({{< ref "/service/how-to/running-the-compiler" >}}) is your bound.

The threat model the compiler addresses:

- **Resource exhaustion** by malicious or pathological builds — bounded by timeouts + rlimits.
- **Network exfiltration** during build — blocked by `--offline` + vendored registry.
- **Compiler-time data exposure** via shared filesystem — bounded by per-build `tmp_root` directories that get cleaned up post-build.

The threat model the compiler does NOT address (Phase 2 work):

- **Kernel exploits** from malicious build scripts — no namespace isolation yet.
- **CPU-side-channel attacks** between concurrent builds on the same host — not currently in scope.
- **Operator-level account compromise** of the compiler service account — the operator is trusted; the package author is not.

If the threat model includes nation-state actors targeting the compiler host, do not run the public-package-upload flow until I-0105 lands. For closed-org deployments (single-team, vetted package authors), Phase 1 is sufficient.

## Multi-tenant isolation

CLOACI-I-0106 closed the historical "isolation gaps" that existed in earlier server releases. The current state is **strong by default**:

- **Fail-closed `SET search_path`.** Per-tenant connection acquisition sets `search_path` strictly to the tenant's schema; a failed `SET search_path` is a hard error, not a silent fall-through to `public`. This closes the cross-tenant data-leak risk that existed pre-I-0106.
- **Per-tenant `DefaultRunner` instances** cached in `TenantRunnerCache` (default 256-entry LRU). Workflow execution lands in the tenant's schema, not in a shared global runner.
- **Per-tenant trigger filtering.** `GET /v1/tenants/{id}/triggers` routes through a tenant-scoped `Database`; the underlying SQL hits the tenant's schedules table, not a shared global table.
- **4-step teardown orchestration** on `DELETE /v1/tenants/{name}` (CLOACI-T-0581): revoke keys → evict runner cache → evict DB cache → drop schema. Cache eviction is part of the teardown; the historical "restart the server to reclaim the cache" workaround is gone.

What multi-tenant isolation **does not** guarantee:

- **CPU side-channels** between executions running concurrently on the same server process. Tenants share the OS and the runtime; an attacker who can run code in one tenant can in principle observe timing of code in another. If this threat is in scope, run a separate `cloacina-server` instance per tenant (one host process per tenant) rather than relying on schema isolation.
- **Privileged operator actions.** An `is_admin` key can do anything; key compromise = total compromise. Treat `is_admin` keys like AWS root credentials.
- **Postgres-level row-level security.** Cloacina relies on schema-level isolation and the `SET search_path` enforcement above; it does not layer Postgres RLS policies on top. If your compliance posture requires defense-in-depth at the DB layer, layer RLS yourself.

For the operational mechanics — how to provision tenants, how to safely decommission them, how to rotate per-tenant credentials — see the [Multi-tenancy explanation]({{< ref "multi-tenancy" >}}) and the multi-tenant how-tos under `/service/how-to/`.

## `/metrics` and `/health` posture

The Prometheus `/metrics` endpoint and the `/health` + `/ready` probes are **unauthenticated**, per ADR-0005. This is a deliberate trade-off:

- **In favor:** no credential management on the scraping path; standard Prometheus deployment shape works out of the box; health probes work with off-the-shelf Kubernetes / load-balancer tooling.
- **Against:** metric values are observable side channels. From `cloacina_workflows_total{status="completed"}` an outside observer can infer whether a tenant is active and roughly how much work it's doing. Similar for `cloacina_active_tasks`.

Deployments where this is unacceptable should terminate `/metrics` at the reverse proxy and require client-cert auth (or IP allowlisting) there. Cloacina does not enforce this in the server — putting the auth boundary at the proxy keeps the server's posture predictable and matches the most common scraping deployment shape.

## See also

- [Multi-tenancy]({{< ref "multi-tenancy" >}}) — operational mechanics of tenant isolation.
- [Package Format]({{< ref "package-format" >}}) — how `.cloacina` archives are structured.
- [Packaged Workflow Architecture]({{< ref "packaged-workflow-architecture" >}}) — the FFI / cdylib trust boundary.
- `require-signed-packages` how-to (DOC-C deliverable; not yet written) will cover the operator-side mechanics of turning on signature enforcement — `--require-signatures` + `--verification-org-id` setup, audit-log expectations, and recovery if you lock yourself out.
- [Package Signing]({{< ref "/service/how-to/security/package-signing" >}}) — how to set up the signing pipeline.
- [Running the Compiler]({{< ref "/service/how-to/running-the-compiler" >}}) — compiler threat model + Phase 1 hardening flags.
- **CLOACI-A-0005** — Deployment-mode trust model (the authoritative ADR for this page).
- **CLOACI-I-0103** — Signature verification at the canonical load path.
- **CLOACI-I-0104** — Compiler hardening Phase 1.
- **CLOACI-I-0106** — Multi-tenant abstraction (fail-closed search_path, 4-step teardown).
