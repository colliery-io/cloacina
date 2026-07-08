---
title: "Secrets"
description: "The encrypted sibling of a parameter — what a Secret is, how it is encrypted at rest, why its plaintext never reaches the context or logs, and how it is delivered to the fleet."
weight: 36
---

# Secrets

A **Secret** is the encrypted sibling of a parameter: a named, tenant-scoped
object of **named fields** — a `{field: value}` map such as
`db_prod = { host, user, password }` or `stripe = { api_key }` — encrypted at
rest. One primitive fits a database connection, an API token, or anything else
that a workflow must not carry in the clear.

This page explains the model. To create and rotate secrets, see
[Manage Secrets]({{< ref "/service/how-to/manage-secrets" >}}); to declare and
read them from a workflow, see
[Use Secrets in a Workflow]({{< ref "/engine/workflows/how-to/use-secrets" >}}).

## Params you can see, secrets you can't

A [workflow instance]({{< ref "/engine/scheduling/workflow-instances" >}}) binds
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

On the [execution-agent fleet]({{< ref "/service/explanation/execution-agent-fleet" >}}),
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
isolation unit, and object-level authorization within a tenant is a deliberate
non-goal — to isolate two workloads, give them separate tenants. Managing a
tenant's secrets (create / rotate / list / delete) is tenant-**admin**
self-service; even a secret's field *names* are treated as sensitive, so reads
are admin-gated.

A per-package `secrets` allow-list also exists as **defense-in-depth**: on the
embedded and constructor paths, an untrusted packaged workflow or WASM
constructor is handed a fail-closed resolver restricted to the secret names it
was granted, denying an ungranted name before any decrypt. This is a cheap extra
gate where the grant is available — it is **not** a hard per-package security
guarantee. The fleet path is tenant-scoped only. Treat "which secret a package
may resolve" as belt-and-suspenders inside the tenant boundary, never as the
boundary itself.

## Reads are metadata-only

No read surface ever returns a value. Listing or getting a secret returns its
name, its field **names**, and timestamps — never a plaintext or ciphertext
value. Rotation is in place: it replaces the whole field map, the next fire sees
the new value, and there is no versioning (an in-flight execution keeps the copy
it already resolved). Rotating a secret needs no code change and no instance
re-registration, because workflows reference secrets by name.

## See also

- [Manage Secrets]({{< ref "/service/how-to/manage-secrets" >}}) — create,
  rotate, list, and delete with `cloacinactl` or the web UI.
- [Use Secrets in a Workflow]({{< ref "/engine/workflows/how-to/use-secrets" >}})
  — declare required secrets, read them from a task, bind them on an instance.
- [Security Model]({{< ref "/service/explanation/security-model" >}}) — the
  wider trust model, deployment modes, and the limits of multi-tenant isolation.
- [Capability Grants]({{< ref "/engine/constructors/grants" >}}) — the grant
  grammar the `secrets` allow-list rides on.
