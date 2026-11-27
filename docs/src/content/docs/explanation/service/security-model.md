---
title: "Security Model"
description: "Trust by deployment mode, the server's permissions, roles and route table, identity providers (API keys, local accounts, OIDC SSO), signature verification, the compiler threat model, and the limits of multi-tenant isolation."
sidebar:
  order: 35
---

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

To create, list, and revoke keys in practice, see [Manage API Keys](../../../how-to/service/manage-api-keys/).

### Roles and the platform admin flag

A key carries two pieces of authority: a **role** inside one tenant, and the **platform admin flag** (`is_admin`).

- A **role** is a named set of permissions. There are four, fixed in the server: `read`, `write`, `admin` and `agent`. The table below lists what each one holds.
  - `read`: list and inspect.
  - `write`: `read`, plus upload packages, run workflows, operate triggers, fire and pause reactors, inject into accumulators.
  - `admin`: `write`, plus the keys, local accounts, secrets, audit record and fleet size of its tenant.
  - `agent`: act as an execution agent, and nothing else. It cannot read the tenant's workflows, executions or events.
- The **platform admin flag** passes every tenant check and every platform route: create and delete tenants, the global key routes, agent limits, agent pools. It grants every permission except `agent.act`: a key with the flag cannot register as an agent or take an agent's stream. The bootstrap key made on first start has the flag. No route makes a second key with the flag.

A key's role string that is not one of the four holds no permission: the key authenticates and every route answers `403`. The server logs each such key at start.

A key request names its role. `POST /v1/tenants/{tenant_id}/keys` and `POST /v1/auth/keys` answer `400` when the body has no `role` or an unknown one. A tenant `admin` can create a key of any of the four roles in its own tenant.

A local account and an OIDC mapping rule take `read`, `write` or `admin`. `agent` is for API keys only. The server does not start when a rule of `CLOACINA_OIDC_MAP` names another role.

`is_admin = false` with role `admin` is the key of a tenant's operator. Keep `is_admin = true` for the team that runs the platform.

### A key with no tenant

"No tenant" is not "admin". The all-tenant view comes from the `is_admin` flag only.

- The **bootstrap key** has no tenant and `is_admin = true`. It passes every check.
- `public` is a **tenant** like any other. A key of `public` carries the tenant `public`.
- A key with **no tenant and `is_admin = false`** is made only by an OIDC mapping rule for the tenant `_` (see [Configure OIDC SSO](../../../how-to/service/configure-oidc-sso/#what-a-_-rule-grants)). It has its session and no tenant's data:

| It can | It cannot |
|---|---|
| `GET /v1/auth/whoami` | reach a tenant route or a platform route |
| `POST /v1/auth/refresh`, `POST /v1/auth/logout` | see a computation graph, reactor or accumulator |
| `POST /v1/auth/ws-ticket` (the ticket carries the same scope) | inject into an accumulator; fire, pause or resume a reactor, over REST or WebSocket |
| | open the delivery WebSocket: `403`, so no operations stream (`ops_metrics:global`) and no execution events |

Its role (`read` / `write` / `admin`) does not change this. The agent routes (`/v1/agent/*`) take the tenant from the key: an agent registered with such a key has no tenant and is given no tenant's work, and the package fetch routes refuse the key.

On the delivery WebSocket a connection belongs to the key's tenant. Only an `is_admin` key connects with no tenant, and only that connection receives the all-tenant operations snapshot.

### Authorization: permissions, roles and the route table

Authorization is one middleware in `cloacina-server`. The embedded library and the daemon have no such layer because they have no multi-tenant surface.

- **A permission** is one kind of action on one kind of resource, such as `workflows.read` or `keys.manage`.
- **Every route has one row** in a table in the code. The row names the scope, the permission the route needs, the credential it accepts and the audit row it writes.
- **The scope** is one of: `public` (no key), `platform` (the platform admin flag), `tenant in the path` (the key belongs to the tenant named by `{tenant_id}`, or has the flag), `caller's key` (the handler reads the tenant from the key).
- **The matcher** `evaluate(principal, scope, permission) -> Permit | Deny` runs on every request after authentication. It tests the scope, then whether the key's role holds the permission.

The WebSocket routes have rows like the REST routes. The upgrade request goes through the same middleware; it accepts a bearer key or a single-use ticket from `POST /v1/auth/ws-ticket`. No REST route accepts a ticket.

- `/v1/ws/accumulator/{name}` needs `accumulators.inject`: every frame is an inject.
- `/v1/ws/reactor/{name}` needs `graphs.read` to connect and read the state. A fire, pause or resume command needs `reactors.operate`; it is checked and audited when it arrives.
- `/v1/ws/delivery/{recipient}` needs the permission of the recipient's kind: `agent:<id>` needs `agent.act`, `exec_events:<id>` needs `executions.read`, `ops_metrics:global` needs `ops.read`. Any other kind is refused.

**A route with no row stops the server.** At start the server compares the routes it registered with the table, both ways. A route with no row, a row with no route, or a state-changing route whose row names no audit action is an error, and the server exits before it listens.

**The middleware writes the audit row** of a state-changing route after the handler answers, under the action its row names. A handler adds the target and the facts of the action; it cannot leave the row out. A refusal on an audited route writes a row with the outcome `denied`. The rows marked "none:" below write no audit row, for the reason given.

A principal also carries numeric limits. The one limit today is the highest priority it may set (0 is the highest): `write` may set 5 to 10, `admin` and the platform admin flag may set 0 to 10, `read` and `agent` may set none.

Tenant-scoped keys' single permitted tenant is set at key creation and cannot be changed; cross-tenant access requires a separate key.

The two tables below are generated from the code (`crates/cloacina-server/src/routes/authz.rs`). A test fails when this page and the code differ.

<!-- authz-table:start (generated from crates/cloacina-server/src/routes/authz.rs; do not edit) -->

#### Permissions and roles

| Permission | Allows | `read` | `write` | `admin` | `agent` | Platform admin flag |
|---|---|---|---|---|---|---|
| `session.self` | Refresh, log out and read the role of the caller's own key | yes | yes | yes |  | yes |
| `ws.ticket` | Get a WebSocket ticket | yes | yes | yes | yes | yes |
| `workflows.read` | List and read packages, their source, versions, instances and backfills | yes | yes | yes |  | yes |
| `packages.publish` | Upload a package, activate a version, delete a version |  | yes | yes |  | yes |
| `workflows.operate` | Pause and resume a workflow; create and delete an instance |  | yes | yes |  | yes |
| `workflows.run` | Run a workflow by hand; create, cancel and retry a backfill |  | yes | yes |  | yes |
| `triggers.read` | List and read schedules and triggers | yes | yes | yes |  | yes |
| `triggers.operate` | Pause, resume and fire a trigger |  | yes | yes |  | yes |
| `executions.read` | List and read executions, their events and tasks; the `exec_events:<id>` stream | yes | yes | yes |  | yes |
| `graphs.read` | Read graphs, reactors and accumulators; connect to a reactor's WebSocket and read its state | yes | yes | yes |  | yes |
| `reactors.operate` | Fire, pause and resume a reactor |  | yes | yes |  | yes |
| `accumulators.inject` | Inject an event into an accumulator |  | yes | yes |  | yes |
| `ops.read` | The `ops_metrics:global` stream | yes | yes | yes |  | yes |
| `limits.read` | Read the tenant's agent limit | yes | yes | yes |  | yes |
| `pools.read` | Read the tenant's agent pools | yes | yes | yes |  | yes |
| `fleet.read` | Read the tenant's fleet view | yes | yes | yes |  | yes |
| `fleet.scale` | Provision and deprovision agents |  |  | yes |  | yes |
| `agents.read` | Read the agent roster |  |  | yes |  | yes |
| `keys.manage` | Create, list and revoke API keys |  |  | yes |  | yes |
| `accounts.manage` | Create, list and disable local accounts; reset a password |  |  | yes |  | yes |
| `secrets.manage` | Read secret metadata; create, rotate and delete secrets |  |  | yes |  | yes |
| `trusted_keys.manage` | Add, list and revoke the tenant's trusted package-signing keys |  |  | yes |  | yes |
| `audit.read` | Read the audit record |  |  | yes |  | yes |
| `agent.act` | Act as an agent: register, heartbeat, top up one-time keys, report a result, fetch packages; the `agent:<id>` stream |  |  |  | yes | no |
| `tenants.manage` | Create, list and delete tenants |  |  |  |  | yes |
| `compiler.read` | Read the compiler's status |  |  |  |  | yes |
| `limits.manage` | Set and clear a tenant's agent limit |  |  |  |  | yes |
| `pools.manage` | Set and delete an agent pool |  |  |  |  | yes |

#### Every route

| Method | Path | Scope | Permission | Audit |
|---|---|---|---|---|
| GET | `/health` | public | none | none (read) |
| GET | `/ready` | public | none | none (read) |
| GET | `/metrics` | public | none | none (read) |
| GET | `/openapi.json` | public | none | none (read) |
| POST | `/v1/auth/local/login` | public | none | `auth.login` |
| GET | `/v1/auth/oidc/login` | public | none | none (read) |
| GET | `/v1/auth/callback` | public | none | `auth.login` |
| POST | `/v1/auth/ws-ticket` | caller's key | `ws.ticket` | none: a ticket lives 60 seconds and changes nothing that lasts |
| POST | `/v1/auth/refresh` | caller's key | `session.self` | none: a refresh replaces the caller's key with one of the same scope |
| POST | `/v1/auth/logout` | caller's key | `session.self` | `auth.logout` |
| GET | `/v1/auth/whoami` | caller's key | `session.self` | none (read) |
| POST | `/v1/tenants` | platform | `tenants.manage` | `tenant.create` |
| GET | `/v1/tenants` | platform | `tenants.manage` | none (read) |
| DELETE | `/v1/tenants/{schema_name}` | platform | `tenants.manage` | `tenant.delete` |
| GET | `/v1/compiler/status` | platform | `compiler.read` | none (read) |
| GET | `/v1/audit` | platform | `audit.read` | none (read) |
| POST | `/v1/auth/keys` | platform | `keys.manage` | `key.create` |
| GET | `/v1/auth/keys` | platform | `keys.manage` | none (read) |
| DELETE | `/v1/auth/keys/{key_id}` | platform | `keys.manage` | `key.revoke` |
| POST | `/v1/tenants/{tenant_id}/limits` | platform | `limits.manage` | `limit.set` |
| DELETE | `/v1/tenants/{tenant_id}/limits` | platform | `limits.manage` | `limit.clear` |
| PUT | `/v1/tenants/{tenant_id}/pools/{pool}` | platform | `pools.manage` | `pool.set` |
| DELETE | `/v1/tenants/{tenant_id}/pools/{pool}` | platform | `pools.manage` | `pool.delete` |
| GET | `/v1/tenants/{tenant_id}/audit` | tenant in the path | `audit.read` | none (read) |
| POST | `/v1/tenants/{tenant_id}/keys` | tenant in the path | `keys.manage` | `key.create` |
| GET | `/v1/tenants/{tenant_id}/keys` | tenant in the path | `keys.manage` | none (read) |
| DELETE | `/v1/tenants/{tenant_id}/keys/{key_id}` | tenant in the path | `keys.manage` | `key.revoke` |
| POST | `/v1/tenants/{tenant_id}/accounts` | tenant in the path | `accounts.manage` | `account.create` |
| GET | `/v1/tenants/{tenant_id}/accounts` | tenant in the path | `accounts.manage` | none (read) |
| DELETE | `/v1/tenants/{tenant_id}/accounts/{account_id}` | tenant in the path | `accounts.manage` | `account.disable` |
| POST | `/v1/tenants/{tenant_id}/accounts/{account_id}/password` | tenant in the path | `accounts.manage` | `account.password_reset` |
| POST | `/v1/tenants/{tenant_id}/secrets` | tenant in the path | `secrets.manage` | `secret.create` |
| GET | `/v1/tenants/{tenant_id}/secrets` | tenant in the path | `secrets.manage` | none (read) |
| GET | `/v1/tenants/{tenant_id}/secrets/{name}` | tenant in the path | `secrets.manage` | none (read) |
| PUT | `/v1/tenants/{tenant_id}/secrets/{name}` | tenant in the path | `secrets.manage` | `secret.rotate` |
| DELETE | `/v1/tenants/{tenant_id}/secrets/{name}` | tenant in the path | `secrets.manage` | `secret.delete` |
| POST | `/v1/tenants/{tenant_id}/trusted-keys` | tenant in the path | `trusted_keys.manage` | `trusted_key.add` |
| GET | `/v1/tenants/{tenant_id}/trusted-keys` | tenant in the path | `trusted_keys.manage` | none (read) |
| DELETE | `/v1/tenants/{tenant_id}/trusted-keys/{key_id}` | tenant in the path | `trusted_keys.manage` | `trusted_key.revoke` |
| POST | `/v1/tenants/{tenant_id}/fleet/provision` | tenant in the path | `fleet.scale` | `fleet.provision` |
| POST | `/v1/tenants/{tenant_id}/fleet/deprovision` | tenant in the path | `fleet.scale` | `fleet.deprovision` |
| GET | `/v1/agents` | caller's key | `agents.read` | none (read) |
| GET | `/v1/tenants/{tenant_id}/limits` | tenant in the path | `limits.read` | none (read) |
| GET | `/v1/tenants/{tenant_id}/pools` | tenant in the path | `pools.read` | none (read) |
| GET | `/v1/tenants/{tenant_id}/pools/{pool}` | tenant in the path | `pools.read` | none (read) |
| GET | `/v1/tenants/{tenant_id}/fleet` | tenant in the path | `fleet.read` | none (read) |
| GET | `/v1/tenants/{tenant_id}/workflows` | tenant in the path | `workflows.read` | none (read) |
| GET | `/v1/tenants/{tenant_id}/workflows/{name}` | tenant in the path | `workflows.read` | none (read) |
| GET | `/v1/tenants/{tenant_id}/workflows/{name}/source` | tenant in the path | `workflows.read` | none (read) |
| GET | `/v1/tenants/{tenant_id}/workflows/{name}/versions` | tenant in the path | `workflows.read` | none (read) |
| GET | `/v1/tenants/{tenant_id}/workflows/{name}/instances` | tenant in the path | `workflows.read` | none (read) |
| GET | `/v1/tenants/{tenant_id}/workflows/{name}/instances/{instance}` | tenant in the path | `workflows.read` | none (read) |
| GET | `/v1/tenants/{tenant_id}/workflows/{name}/backfills` | tenant in the path | `workflows.read` | none (read) |
| GET | `/v1/tenants/{tenant_id}/workflows/{name}/backfills/{id}` | tenant in the path | `workflows.read` | none (read) |
| GET | `/v1/tenants/{tenant_id}/triggers` | tenant in the path | `triggers.read` | none (read) |
| GET | `/v1/tenants/{tenant_id}/triggers/{name}` | tenant in the path | `triggers.read` | none (read) |
| GET | `/v1/tenants/{tenant_id}/triggers/{name}/interface` | tenant in the path | `triggers.read` | none (read) |
| GET | `/v1/tenants/{tenant_id}/executions` | tenant in the path | `executions.read` | none (read) |
| GET | `/v1/tenants/{tenant_id}/executions/{exec_id}` | tenant in the path | `executions.read` | none (read) |
| GET | `/v1/tenants/{tenant_id}/executions/{exec_id}/events` | tenant in the path | `executions.read` | none (read) |
| GET | `/v1/tenants/{tenant_id}/executions/{exec_id}/tasks` | tenant in the path | `executions.read` | none (read) |
| POST | `/v1/tenants/{tenant_id}/workflows` | tenant in the path | `packages.publish` | `package.upload` |
| POST | `/v1/tenants/{tenant_id}/workflows/{name}/versions/{version}/activate` | tenant in the path | `packages.publish` | `package.activate` |
| DELETE | `/v1/tenants/{tenant_id}/workflows/{name}/{version}` | tenant in the path | `packages.publish` | `package.delete` |
| POST | `/v1/tenants/{tenant_id}/workflows/{name}/pause` | tenant in the path | `workflows.operate` | `workflow.pause` |
| POST | `/v1/tenants/{tenant_id}/workflows/{name}/resume` | tenant in the path | `workflows.operate` | `workflow.resume` |
| POST | `/v1/tenants/{tenant_id}/workflows/{name}/instances` | tenant in the path | `workflows.operate` | `instance.create` |
| DELETE | `/v1/tenants/{tenant_id}/workflows/{name}/instances/{instance}` | tenant in the path | `workflows.operate` | `instance.delete` |
| POST | `/v1/tenants/{tenant_id}/workflows/{name}/execute` | tenant in the path | `workflows.run` | `workflow.run` |
| POST | `/v1/tenants/{tenant_id}/workflows/{name}/backfills` | tenant in the path | `workflows.run` | `backfill.create` |
| POST | `/v1/tenants/{tenant_id}/workflows/{name}/backfills/{id}/cancel` | tenant in the path | `workflows.run` | `backfill.cancel` |
| POST | `/v1/tenants/{tenant_id}/workflows/{name}/backfills/{id}/retry` | tenant in the path | `workflows.run` | `backfill.retry` |
| POST | `/v1/tenants/{tenant_id}/triggers/{name}/pause` | tenant in the path | `triggers.operate` | `trigger.pause` |
| POST | `/v1/tenants/{tenant_id}/triggers/{name}/resume` | tenant in the path | `triggers.operate` | `trigger.resume` |
| POST | `/v1/tenants/{tenant_id}/triggers/{name}/fire` | tenant in the path | `triggers.operate` | `trigger.fire` |
| GET | `/v1/health/accumulators` | caller's key | `graphs.read` | none (read) |
| GET | `/v1/health/reactors` | caller's key | `graphs.read` | none (read) |
| GET | `/v1/health/reactors/{name}/fires` | caller's key | `graphs.read` | none (read) |
| GET | `/v1/health/reactors/{name}/fires/timeseries` | caller's key | `graphs.read` | none (read) |
| GET | `/v1/health/reactors/{name}/interface` | caller's key | `graphs.read` | none (read) |
| GET | `/v1/health/accumulators/{name}/interface` | caller's key | `graphs.read` | none (read) |
| GET | `/v1/health/graphs` | caller's key | `graphs.read` | none (read) |
| GET | `/v1/health/graphs/{name}` | caller's key | `graphs.read` | none (read) |
| POST | `/v1/health/reactors/{name}/fire` | caller's key | `reactors.operate` | `reactor.fire` |
| POST | `/v1/health/accumulators/{name}/inject` | caller's key | `accumulators.inject` | `accumulator.inject` |
| GET | `/v1/ws/accumulator/{name}` | caller's key | `accumulators.inject` | `accumulator.connect` |
| GET | `/v1/ws/reactor/{name}` | caller's key | `graphs.read` | none (read) |
| GET | `/v1/ws/delivery/{recipient}` | caller's key | by recipient | none (read) |
| POST | `/v1/agent/register` | caller's key | `agent.act` | `agent.register` |
| POST | `/v1/agent/heartbeat` | caller's key | `agent.act` | none: one every few seconds for each agent |
| POST | `/v1/agent/keys` | caller's key | `agent.act` | `agent.keys` |
| POST | `/v1/agent/result` | caller's key | `agent.act` | none: one for each task; the execution history holds the outcome |
| POST | `/v1/agent/undecodable` | caller's key | `agent.act` | none: one for each refused packet; the execution history holds the outcome |
| GET | `/v1/agent/artifact/{digest}` | caller's key | `agent.act` | none (read) |
| GET | `/v1/agent/source/{digest}` | caller's key | `agent.act` | none (read) |
| GET | `/v1/agent/python-vendor/{digest}` | caller's key | `agent.act` | none (read) |
| GET | `/v1/agent/providers/{digest}` | caller's key | `agent.act` | none (read) |

<!-- authz-table:end -->

### Upgrading from a version with three roles

- **Agents need a key of role `agent`.** A managed agent gets one: on its next reconcile the fleet actuator mints an `agent` key for the tenant, moves the agents to it, and revokes the keys it minted before. On Kubernetes the tenant's agent Deployments roll once. An agent started by hand on a `read`, `write` or `admin` key gets `403` on `POST /v1/agent/register`. Create a key with `"role": "agent"`, restart the agent with it, and revoke the old key. Each refused register is in the audit record as `agent.register` with the outcome `denied` and the key's name.
- **`read` no longer fires or injects.** `POST /v1/health/reactors/{name}/fire`, `POST /v1/health/accumulators/{name}/inject`, the accumulator WebSocket, and the fire, pause and resume commands of the reactor WebSocket need `write`.
- **A key request needs a role.** A request with no `role` made an `admin` key; it now answers `400`.
- **An unknown role grants nothing.** A stored key or local account whose role is not one of the four read as `read`; it is now refused on every route.
- **The bootstrap key cannot act as an agent.**

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

When the server is started with `--require-signatures`, every package upload runs through cryptographic signature verification before the package is registered. The verification:

1. Reads the signature stored for the hash of the uploaded bytes.
2. Looks the signer's key up in the trusted keys of the tenant in the path, then in the trusted keys of the server-wide org if `--verification-org-id` is set.
3. Verifies the signature against the trusted public key.
4. Logs the outcome (success or failure) to the audit log with the package fingerprint.

There is no soft-fail path: an upload with no signature, or signed by a key that neither list holds live, is refused.

Trust is **per tenant**. Each tenant has its own trusted keys, and its `admin` adds and revokes them (`cloacinactl trusted-key`, or the `/v1/tenants/{tenant_id}/trusted-keys` routes); a platform admin can do the same for any tenant. A package signed for one tenant is not accepted by another. The server-wide org is the operator's own key set: a key in it is trusted for every tenant. An operator that wants strict per-tenant trust sets no `--verification-org-id`; each tenant then trusts only its own keys.

It's optional because the threat models diverge:

- **Single-org deployments** where all package authors are trusted operators don't need it. The compiler has no process sandbox: a package's build code runs with the compiler's OS privileges (see [The compiler threat model](#the-compiler-threat-model)), so this holds only while every author is trusted.
- **Multi-org deployments** where tenants upload their own packages need it. Without signature verification, a tenant-write key (which can upload packages) becomes equivalent to a code-execution capability on the server host.

Reach for `--require-signatures` whenever the package author is not the server operator.

### Signing keys and the trust model

Package signing uses **Ed25519** asymmetric cryptography. Each key is a pair:

- **Private key** — used to sign packages. Kept secret and stored encrypted at
  rest (the 32-byte AES-256 *key encryption key* protects it in the database).
- **Public key** — used to verify signatures. Distributed to verifiers.
- **Key fingerprint** — the SHA-256 hash of the public key, used to identify
  which key signed a package.

Keys are managed **per-organization**. On a server, the organization of a tenant is derived from the tenant's name, and the server-wide org is the UUID of `--verification-org-id`. The trust model has three roles:

- **Signing keys** — owned by an organization and used to sign its packages.
- **Trusted keys** — public keys an organization accepts signatures from during
  verification. A key must be trusted before packages signed with it will verify.
- **Trust ACLs** — a parent organization can be granted trust in a child
  organization, so it accepts everything the child trusts. This is how
  organizational trust hierarchies are expressed.

Verification proves the package was signed by a key the verifying organization
trusts; it does not inspect what the package does. See the next section.

### What it doesn't protect against

Signature verification proves *who* signed the package, not *what's in the package*. A signed malicious package still runs malicious code; signatures don't sandbox. The compiler does not sandbox either: it bounds the environment, the time and the resources of a build (see below). Use signatures together with a compiler per tenant.

## The compiler threat model

`cloacina-compiler` runs `cargo build` on packages submitted by — possibly untrusted — package authors. This is the most dangerous code path in the entire system: arbitrary Rust code is being compiled, which means arbitrary build scripts (`build.rs`) and arbitrary proc-macro code can run with the compiler's privileges. A package can also name a constructor provider crate (a Cargo dependency, or a `[metadata.providers]` entry with a registry, path or git source); the compiler builds that crate and runs its manifest binary.

There is no process sandbox around any of this. The compiler starts these child processes for a package: `cargo fetch`, the build, and for provider bundling `cargo metadata`, the provider crate's build and its manifest binary. Every one of them gets:

- **A cleared environment.** The child inherits only `PATH`, `HOME`, `CARGO_HOME`, `RUSTUP_HOME`, `RUSTUP_TOOLCHAIN`, `TMPDIR`, `TERM` and `CARGO_PROFILE_DEV_DEBUG`, plus the build wiring the compiler sets by name (`CARGO_TARGET_DIR`, the `--vendor-dir` `CARGO_HOME`). The two steps that download and run no package code (`cargo fetch`, `cargo metadata`) also inherit the proxy, certificate and cargo network variables. `DATABASE_URL` and the rest of the compiler's environment are not passed.
- **The build timeout** (`--build-timeout-s`). Past it, the child and its process group are killed. A timed-out `cargo build` is marked `timed_out` and the stale-build sweeper reclaims the row; a timed-out fetch or provider step fails the build.
- **`setrlimit` resource caps** (CPU, memory, file descriptors, processes). Linux only.

Network and dependency resolution depend on the cargo flags:

- **The binary's default flags are `build --release --lib --frozen --offline`.** Cargo resolves the package only against the cargo home the operator curated (`--vendor-dir`) and downloads nothing for it.
- **With `--frozen`/`--offline` dropped** (`--cargo-flags-replace`; the Helm chart's default is `build --lib`), the build has two phases: `cargo fetch` downloads the dependency graph from the network, then the build runs with `--offline` appended.
- **`--offline` binds cargo, not the code cargo runs.** A build script, a proc-macro or a provider's manifest binary can open any connection the host allows. Provider bundling does not use the cargo flags: its cargo steps can download.

### Python packages: what the compiler downloads

A Python package that is published with a `requirements.lock` and no `vendor/` folder gets its dependencies from the compiler. For each platform of its settings the compiler runs `uv pip install` on the lock file and stores the installed wheels as one set per platform. A package that ships `vendor/` is not touched: the compiler runs no `uv` and downloads nothing for it.

- **What runs.** `uv`, as a build child: the cleared environment, the timeout and the resource caps above, plus the proxy and certificate variables. It installs wheels only (`--only-binary :all:`), so no build script and no code of a dependency runs in the compiler: a wheel is unpacked, not executed. A dependency with no wheel fails the build.
- **What it downloads, and from where.** The wheels that the lock file names, and the index pages of those distributions, over HTTPS from the package index of the compiler's settings (`--python-index-url`; PyPI by default: `pypi.org` and `files.pythonhosted.org`). With `--python-wheel-dir` it also reads wheels from a folder of the compiler image. With `--python-offline` it opens no connection: the wheels come from that folder, or from a `uv` cache in the image, and a wheel that is not there fails the build by name.
- **What the package's author controls.** The names, the versions and the hashes in the lock file. Not the source: the compiler refuses a lock file that holds anything other than pinned versions with hashes (an option such as `-r` or `--index-url`, a URL, a path), and it runs `uv` with `--no-config`, so a `uv.toml` in the package does not choose the index. The hashes are the author's own: `--require-hashes` makes the compiler install the files the author locked, and does not make those files safe.
- **Where the files go.** Into the stored set of the package, and from there into `vendor/<platform>/` on the servers and agents that load the package. The dependencies then run with the package's tasks, with the same reach as the package's own code (see [What a task can reach on its agent](#what-a-task-can-reach-on-its-agent)). The compiler imports none of them.
- **What this adds to the compiler's network needs.** Egress to the index, unless the compiler is offline. A compiler with restricted egress needs the index and its file host allowed, an index mirror inside the network, or an image that carries the wheels: see [Running the Compiler](../../../how-to/service/running-the-compiler/#python-packages-vendoring-dependencies).

The threat model the compiler addresses:

- **Resource exhaustion** by malicious or pathological builds — bounded by the timeout and the rlimits.
- **The compiler's environment variables** — not inherited by any child.
- **Dependency substitution** — bounded by `--frozen --offline` with a curated cargo home, when the operator keeps those flags.
- **A lock file that reads the compiler's files or another server** — refused before `uv` starts: a lock file holds pinned versions with hashes only.

The threat model the compiler does NOT address:

- **Network access from build code** — the compiler does not block it. Block it outside the compiler (a firewall or a NetworkPolicy on the compiler's egress).
- **The compiler's files** — build code has the compiler's user id. It can read and write the shared cargo target directory, the cargo home, the temp directories and other packages' sources and build outputs in them. On Linux it can read the compiler's command line and environment through `/proc`, so the compiler's database URL is in its reach: give the compiler a database role with the least rights (see [Running the Compiler](../../../how-to/service/running-the-compiler/)).
- **Other tenants' builds in the same compiler** — run one compiler per tenant (`--tenant-schema`) to separate them.
- **Kernel exploits** from malicious build scripts — no namespace isolation.
- **CPU-side-channel attacks** between concurrent builds on the same host — not currently in scope.
- **Operator-level account compromise** of the compiler service account — the operator is trusted; the package author is not.

### What a tenant's build can reach

Build code is a build script, a proc-macro, or a provider crate's build. It
runs as the compiler's user, inside the compiler's container. What it can
reach depends on which compiler builds the package.

**A tenant compiler that the server made** (Kubernetes actuator,
`fleet.compiler.enabled`; see
[Run cloacina-compiler in Production](../../../how-to/service/running-the-compiler/#how-many-compilers-a-deployment-runs)):

- **Other tenants' source and build output: no.** The compiler serves one
  schema and runs in its own pod in the tenant's namespace. Its target
  directory, cargo home and temp directories are the pod's own volumes, and
  they are emptied each time the compiler starts.
- **The database URL: yes.** The URL is the pod's `DATABASE_URL` environment
  variable, from a Secret. It is not on the command line. Build code has the
  compiler's uid, so it can read `/proc/<compiler pid>/environ`. The same
  holds for a Python package index URL set with
  `fleet.compiler.python.indexUrl`: it is in the same Secret and in the
  compiler's environment.
- **What that URL opens.** By default it is the server's own URL, which
  reaches every tenant's schema. With
  `fleet.compiler.databaseUrlSecretRef` it is the URL of the role the operator
  made for builds, and reaches what that role is granted.
- **The network.** The compiler pods' NetworkPolicy allows DNS, the database
  port and, by default, TCP 443 to public IPv4 addresses. The tenant's agent
  policy adds the server. No ingress is allowed. With
  `fleet.compiler.egress=none` a build reaches DNS, the database port, the
  server and the listed CIDR blocks only. The policy binds on a CNI that
  enforces NetworkPolicy.
- **The Kubernetes API: no token.** The pod mounts no service account token.
- **The node and the container's root filesystem.** The pod runs as uid
  `10001` with a read-only root filesystem, no capabilities and the
  `RuntimeDefault` seccomp profile.
- **The compiler process.** Build code has its uid: it can signal it, and it
  can leave a process behind that outlives the build until the pod stops.

**A compiler shared by tenants, or the `public` compiler:** as above for the
database URL and the process. Its target directory, cargo home and temp
directories are shared by every package it builds, and the chart writes no
egress rule for it.

If package authors are not trusted, do not let them upload to a compiler that other tenants share, and do not give that compiler a database role that reaches other tenants. For closed-org deployments (single-team, vetted package authors), the limits above are the bound.

## Multi-tenant isolation

CLOACI-I-0106 closed the historical "isolation gaps" that existed in earlier server releases. The current state is **strong by default**:

- **Fail-closed `SET search_path`.** Per-tenant connection acquisition sets `search_path` strictly to the tenant's schema; a failed `SET search_path` is a hard error, not a silent fall-through to `public`. This closes the cross-tenant data-leak risk that existed pre-I-0106.
- **Per-tenant `DefaultRunner` instances** held in `TenantRunnerCache`, one for every tenant from server start until the tenant is deleted. Workflow execution lands in the tenant's schema, not in a shared global runner.
- **Per-tenant trigger filtering.** `GET /v1/tenants/{id}/triggers` routes through a tenant-scoped `Database`; the underlying SQL hits the tenant's schedules table, not a shared global table.
- **4-step teardown orchestration** on `DELETE /v1/tenants/{name}` (CLOACI-T-0581): revoke keys → evict runner cache → evict DB cache → drop schema. Before the schema is dropped, the fleet actuator removes the tenant's compiler, when it runs one. Cache eviction is part of the teardown; the historical "restart the server to reclaim the cache" workaround is gone.

- **Computation graphs carry their tenant.** The server stamps every graph, reactor and accumulator with the tenant of the runner that loads it: `public` for the packages of the `public` tenant, the tenant's name for the others. The server refuses to load a graph with no tenant. A key sees, injects into, fires and pauses the graphs of its own tenant only; a key of another tenant cannot reach the graphs of `public`. Reactor ownership and the owner-address table use the same `(tenant, name)` pair. Graphs with no tenant, open to every caller, exist in embedded use only (a `ComputationGraphScheduler` in your own process, no server, no tenants).

What multi-tenant isolation **does not** guarantee:

- **CPU side-channels** between executions running concurrently on the same server process. Tenants share the OS and the runtime; an attacker who can run code in one tenant can in principle observe timing of code in another. If this threat is in scope, run a separate `cloacina-server` instance per tenant (one host process per tenant) rather than relying on schema isolation.
- **Privileged operator actions.** An `is_admin` key can do anything; key compromise = total compromise. Treat `is_admin` keys like AWS root credentials.
- **Postgres-level row-level security.** Cloacina relies on schema-level isolation and the `SET search_path` enforcement above; it does not layer Postgres RLS policies on top. If your compliance posture requires defense-in-depth at the DB layer, layer RLS yourself.

### Ownership on the fleet and the delivery socket

The authorization table decides the level a route needs. Which agent, which dispatch and which outbox row a key may touch is decided by ownership, in one module (`cloacina::security::ownership`) that the agent routes, the fleet result path and the delivery WebSocket all call.

- **An agent has an owner.** The owner is the tenant and the API key that registered the agent. Only that key may register the same agent id again, send its heartbeat, report a result for it, top up its one-time keys, or open its `agent:<id>` delivery socket. A register for an agent id that another tenant or another key holds is refused with `409 agent_id_taken`. A call for an agent id that is not registered is refused with `404 agent_not_registered`, and a call for another key's agent with `403 agent_access_denied`. A platform admin key gets no bypass.
- **Agents that share one key are one owner.** The Kubernetes actuator gives every agent of a tenant the same key, so those agents can act as each other. Agents that must be separate need separate keys.
- **A result counts only from the agent the work went to.** The server accepts a task result only from a registered agent, in the tenant of the dispatch, that is the agent the work packet is addressed to, and only with the `dispatch_id` of that packet. When an agent is evicted and its packet is re-targeted, the packet is addressed to the new agent and its result is the one accepted. A graph firing has no `dispatch_id`; the tenant and agent check is the same. A report stored for another server replica carries the tenant and the agent of its sender, and the replica that takes it runs the same check.
- **A delivery connection has a recipient and a tenant.** Who may open which recipient is in the [WebSocket protocol reference](../../../reference/websocket-protocol/#recipients-who-may-subscribe). An `ack` applies only to an outbox row addressed to the connection's own recipient and tenant.
- **Agent rows from before the owner existed.** A row that an older server wrote has no owner. A key of the row's tenant may heartbeat, report and top up for it, as before; nobody may open its `agent:<id>` socket. The next register for that id by a key of the row's tenant makes that key the owner. An agent registers at the start of every session, before it opens the socket, so the row gets its owner at the agent's first reconnect after the upgrade.

### What a task can reach on its agent

A task runs inside the `cloacina-agent` process of its own tenant: a Rust task as a loaded library, a Python task in the embedded interpreter. It has the agent's user, files, network and memory.

- **The Kubernetes API.** Agent pods mount no service account token (`automountServiceAccountToken: false`, in the actuator's Deployment and in the `cloacina-agent` chart), and the per-tenant `NetworkPolicy` allows egress only to DNS and the server.
- **The agent's API key, by the ordinary routes.** The agent reads the key at start, overwrites the value in its start-time environment and removes `CLOACINA_API_KEY`. `std::env`, Python's `os.environ` and `/proc/self/environ` do not hold the key. The agent keeps no key in a file, and sets no default header on its HTTP client.
- **The agent's API key, in memory.** The agent holds the key in its own memory for its calls to the server, and a task shares that memory. Native code in a task, or a task that reads `/proc/self/mem`, can find it. A key given with `--api-key` is in the argument list, and a key file (`--api-key-file`) is readable by the agent's user: both are readable by a task.
- **What that key opens.** A managed agent's key is scoped to its tenant with the role `agent`. On Kubernetes every agent of the tenant has the same key; on Docker each container has its own. The key can call the agent routes (register, heartbeat, top up one-time keys, report a result, and fetch the tenant's artifacts, source and providers by digest), get a WebSocket ticket, and open an `agent:<id>` delivery stream of its tenant. Every other route answers `403`: it cannot read the tenant's workflows, package source, executions, events, triggers, fleet view or limit, cannot reach the computation-graph routes, cannot fire a reactor or inject into an accumulator, cannot publish or execute, cannot read or manage secrets, keys or accounts, and cannot reach another tenant.
- **Secrets.** A task receives the secrets it declares, unwrapped by the agent for that one execution. Tasks of one tenant that run on the same agent share its memory.

For the operational mechanics — how to provision tenants, how to safely decommission them, how to rotate per-tenant credentials — see the [Multi-tenancy explanation](../multi-tenancy/) and the multi-tenant how-tos under `/service/how-to/`.

## `/metrics` and `/health` posture

The Prometheus `/metrics` endpoint and the `/health` + `/ready` probes are **unauthenticated**, per ADR-0005. This is a deliberate trade-off:

- **In favor:** no credential management on the scraping path; standard Prometheus deployment shape works out of the box; health probes work with off-the-shelf Kubernetes / load-balancer tooling.
- **Against:** metric values are observable side channels. From `cloacina_workflows_total{status="completed"}` an outside observer can infer whether a tenant is active and roughly how much work it's doing. Similar for `cloacina_active_tasks`.

When the server binary is started with no `--metrics-bind`, `/metrics` is on the API port. Every client that can reach the API can read it, and that includes each tenant's execution agents. Metric labels hold tenant names, workflow names and other names that tenants choose (graphs, reactors, accumulators, sources), so one tenant's code can read another tenant's names.

Deployments where this is unacceptable should start the server with `--metrics-bind <ADDR>` (env `CLOACINA_METRICS_BIND`). `/metrics` is then served only on that address, the metrics listener serves nothing else, and the API port answers 404 for `/metrics`. Limit who can reach the metrics address at the network layer. The Helm chart does both by default: `metrics.enabled` and `metrics.networkPolicy.enabled` are `true`, so a chart install has the metrics port, a ClusterIP Service for it, and a NetworkPolicy that admits only the peers listed in `metrics.networkPolicy.from` (nobody until you list your Prometheus). The NetworkPolicy takes effect only on a CNI that enforces NetworkPolicy. The scrape itself stays unauthenticated. `cloacina-compiler` has the same flag (`--metrics-bind`, env `CLOACINA_COMPILER_METRICS_BIND`).

`/ready` gives a fixed `reason` (`database unreachable`, or `python runtime unavailable; restart required`). The package that wedged the Python runtime is named in the server log only.

## See also

- [Manage API Keys](../../../how-to/service/manage-api-keys/) — create / list / revoke tenant + global keys.
- [Configure Local Accounts](../../../how-to/service/configure-local-accounts/) — self-managed username/password login with no IdP.
- [Configure OIDC Single Sign-On](../../../how-to/service/configure-oidc-sso/) — wire an issuer, the identity→tenant allowlist, and multi-tenant memberships.
- [Multi-tenancy](../multi-tenancy/) — operational mechanics of tenant isolation.
- [Package Format](../../engine/package-format/) — how `.cloacina` archives are structured.
- [Packaged Workflow Architecture](../../engine/packaged-workflow-architecture/) — the FFI / cdylib trust boundary.
- [Require Signed Packages](../../../how-to/service/require-signed-packages/) — the operator-side mechanics of turning on signature enforcement: `--require-signatures`, the trusted keys of each tenant and the optional `--verification-org-id` setup, audit-log expectations, and recovery if you lock yourself out.
- [Package Signing](../../../how-to/service/security/package-signing/) — how to set up the signing pipeline.
- [Running the Compiler](../../../how-to/service/running-the-compiler/) — compiler threat model + Phase 1 hardening flags.
- **CLOACI-A-0005** — Deployment-mode trust model (the authoritative ADR for this page).
- **CLOACI-I-0103** — Signature verification at the canonical load path.
- **CLOACI-I-0104** — Compiler hardening Phase 1.
- **CLOACI-I-0106** — Multi-tenant abstraction (fail-closed search_path, 4-step teardown).
