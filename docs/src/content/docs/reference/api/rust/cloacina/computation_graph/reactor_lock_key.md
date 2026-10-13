---
title: "cloacina::computation_graph::reactor_lock_key"
description: "Advisory-lock key derivation for per-reactor leadership (CLOACI-T-0851, [`ADR CLOACI-A-0012`])."
editUrl: false
sidebar:
  order: 46
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Advisory-lock key derivation for per-reactor leadership (CLOACI-T-0851, [`ADR CLOACI-A-0012`]).

**Why this needs its own module and its own tests**

Reactor ownership is claimed with a Postgres **session-level advisory
lock**, mirroring the fleet control loop
(`cloacina-server::autoscaler::leader`). Advisory locks live in a single
**database-wide** key space: they are NOT scoped by schema.

Cloacina isolates tenants by **schema within one database**
(`SET LOCAL search_path TO <tenant>`; see `database::admin`). Every DAL
table is therefore tenant-separated for free — but advisory locks are not.
Deriving a reactor's lock key from the graph name alone would mean two
tenants running a same-named graph contend for one lock, and exactly one of
them would silently never run its reactor. That is a cross-tenant
correctness bug that no single-tenant test could ever surface, which is why
the key derivation is isolated here with tests rather than inlined at the
lock call site.

Note that `save_reactor_state` keys checkpoints by graph name alone and is
safe precisely BECAUSE the DAL is schema-scoped. That asymmetry is the trap:
the neighbouring persistence code looks like a precedent for
name-only keying, and it is not one for locks.

**Why not `DefaultHasher`**

`std::collections::hash_map::DefaultHasher` (and anything built on
`RandomState`) is seeded randomly **per process**. Two replicas hashing the
same `(tenant, reactor)` would compute DIFFERENT keys, so both would acquire
"the lock" and both would run the reactor — the precise split-brain this
mechanism exists to prevent, presenting as an intermittent duplicate rather
than an error. The hash here is therefore a hand-rolled FNV-1a over a
delimited encoding: fixed constants, no seed, stable across processes,
releases, and architectures.

## Functions

### <span id="fn.reactor_lock_key"></span><code style="font-size: 0.85em">fn reactor_​lock_​key</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn reactor_lock_key(tenant: Option<&str>, reactor_name: &str) -> i64
```

Derive the advisory-lock key identifying ownership of `reactor_name` within `tenant`.

`tenant` is the tenant scope as the scheduler keys it — `None` for
single-tenant / admin-owned reactors, which is a distinct namespace from any
named tenant (see the `none_tenant_is_distinct_from_named_tenant` test).

The encoding is length-delimited rather than concatenated so that
`(tenant="a", reactor="bc")` and `(tenant="ab", reactor="c")` cannot collide.
Plain concatenation with a separator is not enough either, since a tenant or
reactor name may itself contain the separator.

<details>
<summary>Source</summary>

```rust
pub fn reactor_lock_key(tenant: Option<&str>, reactor_name: &str) -> i64 {
    let mut buf = Vec::with_capacity(reactor_name.len() + 32);

    // Domain tag: keeps this hash space distinct from any other FNV use.
    buf.extend_from_slice(b"cloacina.reactor.ownership.v1\0");

    match tenant {
        // A named tenant is encoded with its length; `None` uses a marker that
        // no length-prefixed string can produce.
        Some(t) => {
            buf.push(1);
            buf.extend_from_slice(&(t.len() as u64).to_be_bytes());
            buf.extend_from_slice(t.as_bytes());
        }
        None => buf.push(0),
    }

    buf.extend_from_slice(&(reactor_name.len() as u64).to_be_bytes());
    buf.extend_from_slice(reactor_name.as_bytes());

    // Force the sign bit so reactor keys occupy the negative half of the i64
    // space, disjoint from hand-picked small positive keys like
    // FLEET_CONTROL_LOCK_KEY.
    (fnv1a(&buf) | REACTOR_LOCK_KEY_SIGN_BIT) as i64
}
```

</details>
