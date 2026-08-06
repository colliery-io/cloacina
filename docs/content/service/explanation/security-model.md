---
title: "Security Model"
description: "Trust by deployment mode, the server's ABAC authorization layer and identity providers (API keys, local accounts, OIDC SSO), signature verification, the compiler threat model, and the limits of multi-tenant isolation."
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
- Multiple operators authenticate via API keys, self-managed local accounts, or OIDC single sign-on, at different privilege levels.
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

#### The 30-second cross-replica revocation window

The cache is **per replica, and only the replica that served the revocation clears it.** There is no cache-invalidation broadcast between replicas. So in a multi-replica deployment:

> A revoked API key may continue to be accepted by *other* replicas for **up to 30 seconds** after revocation. On the replica that handled the `DELETE`, revocation is immediate.

The same bound applies to every other reason a key stops being valid mid-flight — expiry of a minted login key, the rotation half of `POST /v1/auth/refresh`, and the key revocation performed by `POST /v1/auth/logout`. Each is enforced immediately on the acting replica and within the cache TTL everywhere else.

This is a deliberate trade, not an oversight:

- The window is **short and fixed**. It cannot grow with load, replica count, or clock skew between replicas — the TTL is measured from the moment each replica cached the entry.
- The alternative — validating every request against the database — puts the auth path on the DB for every call on the hottest code path in the server, for a difference of at most 30 seconds in a scenario (compromised key *and* an attacker actively using it in the same half-minute) where the attacker has already had unbounded time before you noticed.
- A cross-replica invalidation channel (pub/sub, LISTEN/NOTIFY) would buy a smaller window at the cost of a new failure mode: if the channel breaks, revocation silently stops propagating and the window becomes unbounded rather than 30 seconds. A TTL fails safe; a broadcast fails open.

Operationally: when you revoke a key in response to a *confirmed* compromise and need a hard cut-off, wait 30 seconds before declaring the key dead, or restart the replicas (a fresh process has an empty cache). Do not treat revocation as instantaneous across the fleet.

To create, list, and revoke keys in practice, see [Manage API Keys]({{< ref "/service/how-to/manage-api-keys" >}}).

### Roles vs the `is_admin` god-mode

Cloacina keys carry two pieces of authority: a **tenant-scoped role** (`admin` / `write` / `read`) and an **`is_admin` flag** that grants god-mode across all tenants.

- **`is_admin` keys** can do anything: create tenants, list tenants, create keys for any tenant, delete tenants, mint new admin keys. These should be issued sparingly and rotated regularly. The bootstrap key generated on first server startup is `is_admin`; production deployments rotate it immediately.
- **Tenant-scoped keys** are sandboxed to a single tenant. Within that tenant, the `role` discriminates `admin` (manage tenant resources) / `write` (execute workflows, upload packages) / `read` (list / inspect only). They cannot enumerate or affect other tenants.

The `is_admin = false` + `role = admin` combination — i.e., "I'm admin within my own tenant" — is the expected production posture for tenant operators. Reserve `is_admin = true` for the operations team running the platform.

### Authorization: the ABAC route table

As of CLOACI-I-0118, authorization is a single, fail-closed middleware layer that lives in **`cloacina-server` only** — the embedded library and the daemon have no such layer because they have no multi-tenant surface to gate. Every `/v1/*` route is classified once, at startup, into a declarative table keyed by `(method, path)` and mapping to a required **scope** (platform / tenant / any) and **level** (read / write / admin). A small, total matcher — `evaluate(principal, scope, level) -> Permit | Deny` — runs on every request after authentication:

- A request whose matched route is **not in the table is denied**, never allowed by default. Adding a route without classifying it fails closed.
- **Platform-scoped** routes (create / list / delete tenants, the global key surface) require an `is_admin` key.
- **Tenant-scoped** routes resolve the `{tenant_id}` path parameter and require the caller's key to be scoped to that tenant with a sufficient role.

This replaced the older per-handler `can_access_tenant` / `can_write` / `can_admin` checks that were scattered across every handler. The matcher reproduces the previous decisions for the common cases but closes a class of gaps the scattered checks left — most notably the **cross-tenant key-management leak**: the global `GET/POST/DELETE /v1/auth/keys` surface is now platform-scoped (god-only), so a tenant-admin key can no longer enumerate or mint keys outside its tenant. Tenant operators manage their own keys through `/v1/tenants/{tenant_id}/keys`; a tenant-scoped key calling the global surface gets `403`.

Tenant-scoped keys' single permitted tenant is set at key creation and cannot be changed; cross-tenant access requires a separate key.

### Authenticating: identity providers

A bearer key is always the *subject* the matcher evaluates, but there are three ways to obtain one:

1. **API keys** — created directly by an admin (`POST /v1/tenants/{t}/keys`, or the global surface for `is_admin`). Long-lived, no expiry, shown once at creation.
2. **Local accounts (self-managed, no IdP)** — username/password accounts a tenant-admin provisions under `/v1/tenants/{t}/accounts`. Passwords are hashed with **argon2id**; accounts are unique per `(tenant, username)`. `POST /v1/auth/local/login` validates the credentials and **mints a short-TTL bearer key**. This is the path for deployments that don't want to run an IdP.
3. **OIDC single sign-on** — when the server is configured with an issuer (`CLOACINA_OIDC_*`), `/v1/auth/oidc/login` runs the authorization-code + PKCE flow. The callback validates the ID token (JWKS signature, `iss` / `aud` / `exp`, nonce), maps the validated identity to one or more `{tenant, role}` memberships through a **god-owned allowlist** (group / email-domain / subject → tenant + role), and mints a scoped key per membership. An identity that matches no rule is denied.

In every case the result is an ordinary bearer key; authorization does not care how it was minted. Whether a key came from a paste, a password, or an IdP is invisible to the matcher.

### Session lifecycle: mint, refresh, logout

Minted keys (local + OIDC) carry an `expires_at` and an `issued_via` provenance (`local:<account_id>` or `oidc:<issuer>:<sub>`); directly-created API keys have neither and never expire.

- **Refresh** — `POST /v1/auth/refresh` re-checks that the underlying account / identity is still valid, mints a fresh key, and revokes the old one. The UI calls this silently before a key's TTL lapses, so a session survives without storing a long-lived credential in the browser. Refresh is *authenticated*: it presents the current, still-valid key. A key that has already lapsed cannot be refreshed — the browser must sign in again, which is why the UI refreshes on a timer rather than on demand.
- **Logout** — `POST /v1/auth/logout` revokes the caller's key, deletes the server-side login session, and clears the auth cache.

A refreshed key is **re-minted with exactly the scope the original login resolved to** — tenant and role are read from the stored key record, not from anything the caller sends — and the mint path takes no `is_admin` argument at all. A refresh therefore cannot broaden a session's authority, and god-mode remains unreachable through any login flow.

#### What bounds an OIDC session

Each OIDC sign-in opens a **server-side login session** with an absolute deadline (`CLOACINA_OIDC_SESSION_MAX_AGE_S`, default 8 hours). Refresh rotates the 15-minute key inside that window; it never extends the deadline. Past it, refreshing fails and the user goes through the full OIDC flow again — which re-checks the identity at the IdP *and* re-resolves the current allowlist.

Cloacina deliberately does **not** spend the IdP's own refresh token on each refresh. Doing so would require `offline_access` (not universally granted), would put the IdP on the hot path of every session in the deployment, and would mean taking custody of a long-lived credential — the very thing short-TTL minted keys exist to avoid. The cost of the simpler design is that a user deactivated *at the IdP* keeps their cloacina session until the absolute deadline expires. Deployments where the IdP is the authority on employment status should lower `CLOACINA_OIDC_SESSION_MAX_AGE_S` accordingly (e.g. `3600` for same-hour offboarding); disabling the corresponding local account or revoking the key cuts a session off immediately, subject to the 30-second cross-replica window above.

The `oidc_sessions` table retains an encrypted (AES-256-GCM) column for IdP refresh material for deployments that later opt into token custody; it is unused — and NULL — by default.

OIDC in-flight login state (the `state` / nonce / PKCE verifier between the redirect and the callback) and the login session itself both live in Postgres, so refresh and the callback can land on any replica — no sticky sessions.

### Brute-force defense on password login

`POST /v1/auth/local/login` is unauthenticated by construction and is the one endpoint where an attacker can guess. Argon2id makes each guess expensive *for the server*; it does nothing to limit how many guesses an attacker gets. Failed attempts are therefore counted and locked out.

Counters are **two independent scopes**, and an attempt is refused if either is locked:

| Scope | What it catches | Default threshold |
|-------|-----------------|-------------------|
| Username (per tenant) | an attacker rotating source addresses against one account | 5 consecutive failures |
| Source address | one host spraying many usernames, none of which alone trips | 50 consecutive failures |

Neither scope alone is sufficient — username-only lets a single host walk an entire user list, address-only is defeated by a proxy pool — and a combined `(username, address)` key is worse than either, because it resets for every new source address. The address threshold is an order of magnitude looser so a shared-NAT office does not lock itself out over a handful of typos.

Once locked, the endpoint returns **`429` with a `Retry-After` header** and refuses the attempt *before* checking the password, so a correct guess during the window does not get in. The window is exponential (30 seconds, doubling per further failure, capped at 15 minutes) and lifts by itself — no operator unlock. A successful login clears the username counter, so a legitimate user who eventually remembers their password does not stay one strike from a lockout. Counters idle for 15 minutes are forgotten.

The response never reveals whether an account exists: failures against unknown usernames are counted exactly like real ones (so the `429` arrives on the same attempt either way), and an unknown username pays the same argon2id cost as a wrong password so the two cannot be told apart by timing.

Lockouts emit the `auth.login.lockout` audit event (once per lock, not once per refused attempt) and increment `cloacina_auth_login_lockouts_total{scope}`; every attempt increments `cloacina_auth_login_attempts_total{outcome=ok|denied|throttled}`.

Counters are stored in Postgres, not in each replica's memory, so a lockout is enforced by every replica — an attacker cannot escape one by reconnecting through the load balancer. If the throttle store itself errors, the request is allowed through to the ordinary password check: the throttle is a mitigation, and a database blip must not become a platform-wide lockout.

Tuning knobs: `CLOACINA_LOGIN_THROTTLE_USER_THRESHOLD`, `CLOACINA_LOGIN_THROTTLE_IP_THRESHOLD` (`0` disables the address scope), `CLOACINA_LOGIN_THROTTLE_BASE_LOCK_S`, `CLOACINA_LOGIN_THROTTLE_MAX_LOCK_S`.

> **Behind a reverse proxy**, the socket peer is the proxy for *every* user, so the address scope collapses into one counter covering the whole deployment. Either set `CLOACINA_TRUST_PROXY_HEADERS=1` — only safe when the proxy *overwrites* `X-Forwarded-For` rather than appending to a client-supplied value — so real client addresses are used, or set `CLOACINA_LOGIN_THROTTLE_IP_THRESHOLD=0` and rely on the username scope. `X-Forwarded-For` is not trusted by default because an unauthenticated caller controls it outright: believing it would let an attacker mint a fresh address identity per request and forge failures against someone else's.

### Multi-tenant individuals

The tenant is the isolation boundary, and a bearer key is always scoped to exactly one tenant — there is **no multi-tenant subject**. An individual who belongs to several tenants holds **one scoped key per tenant** and switches between them in the UI's tenant switcher. Both identity paths populate this:

- **Local:** the person has a separate account per tenant (`alice` in `acme` and `alice` in `public` are independent rows with their own passwords and roles).
- **OIDC:** a single sign-on resolves to the *set* of memberships the allowlist grants; the server mints a key per tenant and the UI shows a picker.

Object- or workflow-level authorization *within* a tenant is a deliberate non-goal — to isolate a sensitive set of work, spin up a separate tenant. That choice is what lets authorization stay a single, coarse, URL-derivable route table rather than a per-row policy engine.

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

### Signing keys and the trust model

Package signing uses **Ed25519** asymmetric cryptography. Each key is a pair:

- **Private key** — used to sign packages. Kept secret and stored encrypted at
  rest (the 32-byte AES-256 *key encryption key* protects it in the database).
- **Public key** — used to verify signatures. Distributed to verifiers.
- **Key fingerprint** — the SHA-256 hash of the public key, used to identify
  which key signed a package.

Keys are managed **per-organization**, and the trust model has three roles:

- **Signing keys** — owned by an organization and used to sign its packages.
- **Trusted keys** — public keys an organization accepts signatures from during
  verification. A key must be trusted before packages signed with it will verify.
- **Trust ACLs** — a parent organization can be granted trust in a child
  organization, so it accepts everything the child trusts. This is how
  organizational trust hierarchies are expressed.

Verification proves the package was signed by a key the verifying organization
trusts; it does not inspect what the package does. See the next section.

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

- [Manage API Keys]({{< ref "/service/how-to/manage-api-keys" >}}) — create / list / revoke tenant + global keys.
- [Configure Local Accounts]({{< ref "/service/how-to/configure-local-accounts" >}}) — self-managed username/password login with no IdP.
- [Configure OIDC Single Sign-On]({{< ref "/service/how-to/configure-oidc-sso" >}}) — wire an issuer, the identity→tenant allowlist, and multi-tenant memberships.
- [Multi-tenancy]({{< ref "multi-tenancy" >}}) — operational mechanics of tenant isolation.
- [Package Format]({{< ref "package-format" >}}) — how `.cloacina` archives are structured.
- [Packaged Workflow Architecture]({{< ref "packaged-workflow-architecture" >}}) — the FFI / cdylib trust boundary.
- [Require Signed Packages]({{< ref "/service/how-to/require-signed-packages" >}}) — the operator-side mechanics of turning on signature enforcement: `--require-signatures` + `--verification-org-id` setup, audit-log expectations, and recovery if you lock yourself out.
- [Package Signing]({{< ref "/service/how-to/security/package-signing" >}}) — how to set up the signing pipeline.
- [Running the Compiler]({{< ref "/service/how-to/running-the-compiler" >}}) — compiler threat model + Phase 1 hardening flags.
- **CLOACI-A-0005** — Deployment-mode trust model (the authoritative ADR for this page).
- **CLOACI-I-0103** — Signature verification at the canonical load path.
- **CLOACI-I-0104** — Compiler hardening Phase 1.
- **CLOACI-I-0106** — Multi-tenant abstraction (fail-closed search_path, 4-step teardown).
