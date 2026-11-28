---
title: "Secrets"
description: "The encrypted sibling of a parameter — what a Secret is, how it is encrypted at rest, why its plaintext never reaches the context or logs, and how it is delivered to the fleet."
sidebar:
  order: 36
---

A **Secret** is the encrypted sibling of a parameter: a named, tenant-scoped
object of **named fields** — a `{field: value}` map such as
`db_prod = { host, user, password }` or `stripe = { api_key }` — encrypted at
rest. One primitive fits a database connection, an API token, or anything else
that a workflow must not carry in the clear.

This page explains the model. To create and rotate secrets, see
[Manage Secrets](../../../how-to/service/manage-secrets/); to declare and
read them from a workflow, see
[Use Secrets in a Workflow](../../../how-to/workflows/use-secrets/).

## Params you can see, secrets you can't

A [workflow instance](../../primitives/scheduling/workflow-instances/) binds
**parameters** — plain values that travel with the run: they land in the
`schedules.params` row and are merged into the run's `Context` as ordinary
top-level keys, so they are visible in the fires log and execution history. That
visibility is the point of a parameter.

A secret is *shaped* like a parameter — a named object of named fields, declared
alongside params — but *delivered differently*. Its plaintext must never appear
in any durable record. Secrets exist precisely because "just store the param
encrypted" is not enough: the value would still surface in the `Context` and the
fires log at run time. The whole design is the separate resolution boundary.

## Encrypted at rest: per-tenant data keys

Secrets are encrypted with **envelope encryption**:

- Each tenant has its own **data key (DEK)** — 32 random bytes generated on
  first use. A secret's field map is serialized and encrypted under the tenant
  DEK with AES-256-GCM.
- The DEK itself is never stored in the clear. It is **wrapped** (encrypted)
  under the server's **key-encryption key (KEK)**, supplied to the server as the
  `CLOACINA_SECRET_KEK` environment variable (base64 or hex of exactly 32
  bytes). Unwrapping happens only server-side, and the plaintext DEK and field
  values live only transiently in memory.
- Each ciphertext is **bound** to where it belongs, through the AES-GCM
  associated data. The wrapped DEK is bound to its tenant
  (`cloacina:dek:v1:<org_id>`), and a secret's field map to its tenant and its
  name (`cloacina:secret:v1:<org_id>:<name>`). A row copied in the database to
  another tenant's schema, or to another secret name, does not decrypt there.
  `<org_id>` is derived from the tenant name, so a tenant's secrets are bound
  to its name. `v1` marks the format.

Secrets stored before the binding carry no associated data. The first server
of the release that adds it rewrites them once at start, for `public` and each
tenant schema: one transaction per schema, under the migration lock. After
that no row is read in the old form. A schema whose data key does not unwrap
under the server KEK is left as it is and logged at start.

Per-tenant keys align with Cloacina's tenant-is-the-isolation-boundary posture:
rotation and compromise blast-radius are scoped to a single tenant. If
`CLOACINA_SECRET_KEK` is unset or malformed, the secrets subsystem is simply not
configured — the CRUD routes return `503` and workflow resolution returns a
clear "secrets backend not configured" error rather than failing open.

## The no-leak guarantee

The load-bearing property is that a secret's plaintext **never** enters a
durable record: not the `Context`'s serialized data, not `schedules.params`, not
the fires log, not audit rows, not execution history.

This is enforced structurally, not by convention:

- A task reads secrets through a dedicated accessor
  (`context.secret("name")`), which is backed by a runtime-only resolver handle
  on the execution scope. That handle is **not** part of the serialized context
  — serializing a `Context` writes only its data map, so a resolved value has
  nowhere to be written to.
- When an instance binds a param to a secret reference (see below), the
  reference is routed away from the plaintext param map into a separate
  name-to-name alias map that carries **no values** — only which secret a
  declared binding points at.
- The resolver handle is redacted from debug output, and the server KEK is never
  logged or rendered.

The value appears in plaintext **only** transiently, inside the task's execution
scope, at the moment it reads the accessor.

## Delivery to the fleet: per-execution envelope wrap

On the embedded / in-process path, the server *is* the executor: it resolves a
secret directly into the accessor, and no wire delivery is involved.

On the [execution-agent fleet](../execution-agent-fleet/),
the executing agent is a separate process, and the at-rest KEK must never leave
the server. Delivery therefore uses a **per-execution envelope wrap**, built on
HPKE (hybrid public-key encryption, RFC 9180):

1. Each agent mints a **pool of one-time ephemeral keypairs** and advertises the
   public halves — each with a key id — at registration, replenishing the pool
   through its heartbeat when it runs low.
2. For each secret-bearing dispatch, the server resolves the secret, **consumes
   exactly one** of that agent's one-time public keys, and HPKE-wraps the value
   to it. Only the resulting ciphertext and the key id ride in the work packet.
3. The agent looks up the matching private key, unwraps **once** into memory,
   discards the key, and serves the value to the task body through the same
   `context.secret(...)` accessor.

The result is true per-execution forward secrecy with no dispatch latency: each
execution gets a fresh single-use key, so a captured ciphertext cannot be
replayed — it is bound to one agent, one execution, and one secret name by the
AEAD associated data. The agent never holds the at-rest KEK and never persists
the plaintext. Pool exhaustion fails the dispatch cleanly; a key is never
reused.

## Authorization is tenant scope

**Secrets are tenant-scoped, and that scope is the authorization boundary.** A
workflow resolves secrets in its own tenant; cross-tenant resolution is
impossible on every path — the store keys every lookup by the tenant's `org_id`,
and the same secret name in two tenants is two independent, isolated secrets.

This is the same boundary the rest of Cloacina uses: the tenant *is* the
isolation unit — to isolate two workloads, give them separate tenants. Inside
a tenant, a task reads only the secrets its package declares (below). Managing a
tenant's secrets (create / rotate / list / delete) is tenant-**admin**
self-service; even a secret's field *names* are treated as sensitive, so reads
are admin-gated.

## Inside a tenant: a task reads the secrets its package declares

On `cloacina-server`, a task reads only the secrets its package declares. The
compiler stores the declared names in the package's metadata document. The
grant of a task is, for each declared name, the stored secret the run binds to
it with a `$secret` reference, or the name itself when the run binds none.
The bindings come from the run's first context, as the caller wrote it: a
task cannot change them for a later task by writing the reserved key into its
output.

- **On the fleet**, the server sends an agent only the secrets of the grant.
  A dispatch whose context names a secret outside the grant is refused before
  anything is resolved. On the agent, a name the work packet does not carry is
  refused.
- **In the server process** (single trust), the server's resolver answers each
  task only the names of its grant. Tenant code in the server process shares
  the process and the server KEK, so this gate does not hold against code that
  reads the process memory; on agents it does.
- **A refusal** is `secret not granted: <name>: package <package> does not
  declare it` (`PermissionError` in Python). The server writes an audit row,
  action `secret.refused`, with the tenant, the secret name, the package, its
  version, the task and the run. A refusal on an agent is in the task's error
  and the agent's log.
- **A package with no declaration** reads no secret: one with no metadata
  document, and one built before `secrets(...)` that declares none.

Embedded use and `cloacinactl daemon` keep the trusted resolver: the host owns
the KEK and the code, and a task reads any secret of its tenant.

A WASM constructor is handed a resolver restricted to the secret names its
constructor grant lists, denying an ungranted name before any decrypt.

## Reads are metadata-only

No read surface ever returns a value. Listing or getting a secret returns its
name, its field **names**, and timestamps — never a plaintext or ciphertext
value. Rotation is in place: it replaces the whole field map, the next fire sees
the new value, and there is no versioning (an in-flight execution keeps the copy
it already resolved). Rotating a secret needs no code change and no instance
re-registration, because workflows reference secrets by name.

## See also

- [Manage Secrets](../../../how-to/service/manage-secrets/) — create,
  rotate, list, and delete with `cloacinactl` or the web UI.
- [Use Secrets in a Workflow](../../../how-to/workflows/use-secrets/)
  — declare required secrets, read them from a task, bind them on an instance.
- [Security Model](../security-model/) — the
  wider trust model, deployment modes, and the limits of multi-tenant isolation.
- [Capability Grants](../../primitives/constructors/grants/) — the grant
  grammar the `secrets` allow-list rides on.
