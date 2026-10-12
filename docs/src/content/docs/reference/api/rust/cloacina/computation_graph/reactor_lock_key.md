---
title: "cloacina::computation_graph::reactor_lock_key"
description: "Advisory-lock key derivation for per-reactor leadership (CLOACI-T-0851, [`ADR CLOACI-A-0012`])."
sidebar:
  order: 62
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Advisory-lock key derivation for per-reactor leadership (CLOACI-T-0851, [`ADR CLOACI-A-0012`]).

## Functions

### <span id="fn.fnv1a"></span>`fn fnv1a`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn fnv1a (bytes : & [u8]) -> u64
```

<details>
<summary>Source</summary>

```rust
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}
```

</details>



### <span id="fn.reactor_lock_key"></span>`fn reactor_lock_key`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn reactor_lock_key (tenant : Option < & str > , reactor_name : & str) -> i64
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



