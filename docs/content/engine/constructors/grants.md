---
title: "Capability Grants"
description: "The default-closed grants grammar — what a constructor's sandbox may reach, decided by the consumer."
weight: 30
---

# Capability Grants

A constructor executes inside a WASI sandbox that can reach **nothing** by
default — no filesystem, no network, no environment. The **consumer** (not the
author) widens it, per instance, with `grants`. The same constructor code can
therefore be run wide-open by one workflow and fully sealed by another.

The grammar is identical on every consumer surface — Rust `constructor!`,
Rust `#[reactor(constructor = ...)]`, and Python `cloaca.constructor(...)`:

```rust,ignore
grants = {
    fs   = ["ro:/data", "rw:/scratch"],
    env  = ["API_REGION"],
    http = ["api.example.com", "*.internal:8443"],
    tcp  = ["db.internal:5432"],
}
```

```python
grants={"fs": ["ro:/data"], "env": ["API_REGION"]}
```

## The four kinds

| Kind | Pattern | Effect |
| --- | --- | --- |
| `fs` | `ro:<path>` / `rw:<path>` | Pre-opens the directory read-only / read-write. Everything outside stays invisible. |
| `env` | `<NAME>` | Passes the **host's** value of that variable through by name (skipped silently if unset). Literal values are not supported. |
| `http` | `<host>[:port][/path-glob]`, `*` globs | Grants the http capability plus a per-request egress policy matching host/port/path. |
| `tcp` | `<host>:<port>`, `*:<port>`, `*` | Grants raw sockets plus a per-connection policy. A DNS host is resolved once at load and matched by `(ip, port)`. |

## Semantics worth knowing

- **Default-closed, fail-closed.** No grant → deny. A malformed grant aborts
  the load rather than silently widening access.
- **Symlinks cannot escape.** A path inside a granted tree that symlinks
  outside it is refused by the sandbox (`Operation not permitted`). Grant the
  real target, or point at a regular file — e.g. prefer `/etc/hostname` over
  `/etc/os-release`, which is a symlink into `/usr/lib` on Debian images.
- **Denial surfaces at the operation**, not at load: an ungranted read fails
  inside the member (a task node fails, a trigger simply never fires) with the
  WASI error naming the path.
- **Load-time lint.** If the provider package declares a capability intent the
  consumer didn't grant, the loader logs a warning at load — the mismatch will
  deny at runtime, so it's surfaced early rather than as a mystery failure.

The runnable `examples/constructor-contract/fs-grant-demo` demonstrates all
three outcomes side by side: a granted read, the same read denied without the
grant, and a granted write through a second suite member.
