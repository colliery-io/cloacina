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

## The five kinds

| Kind | Pattern | Effect |
| --- | --- | --- |
| `fs` | `ro:<path>` / `rw:<path>` | Pre-opens the directory read-only / read-write. Everything outside stays invisible. A bare path with no `ro:`/`rw:` prefix is a hard error — the load fails closed. |
| `env` | `<NAME>` | Passes the **host's** value of that variable through by name (skipped silently if unset). Literal values are not supported. |
| `http` | `<host>[:port][/path-glob]`, `*` globs | Grants the http capability plus a per-request egress policy matching host/port/path. |
| `tcp` | `<host>:<port>`, `*:<port>`, `*` | Grants raw sockets plus a per-connection policy. A DNS host is resolved once at load and matched by `(ip, port)`. |
| `secrets` | `<name>` | Allow-lists the named secrets the member may resolve through the secret store; anything not listed is denied (`NotGranted`, distinct from `NotFound`). |

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

## Native providers bypass enforcement

Grant enforcement applies **only to WASM providers**. A provider packaged with
`runtime = native` runs **trusted and unsandboxed, in-process** — no WASI
sandbox, no capability allow-list, no egress policy
(`ProviderRuntime::grants_enforced()` returns true only for `Wasm`; the native
load path takes no grants at all). Any `grants` you write at a native
constructor's instantiation site are advisory documentation, not a security
boundary. Treat installing a native provider like adding a normal Rust
dependency: review it, pin it, and trust it — or use a WASM provider instead.
`cloacinactl constructor package --native` prints this trust tier at packaging
time.

### You cannot reach unenforced grants by accident

Because `runtime` is an emission target, a provider can ship as WASM in one
version and native in the next — which would quietly turn your `grants` from a
control into decoration. Two rules make that impossible:

- **Declaring `grants` against a provider that resolves NATIVE is a load
  error**, unless you acknowledge the trust tier explicitly. The message names
  all three ways out:

  > grants are not enforced on native providers; either pin `runtime = "wasm"`,
  > acknowledge with `runtime = "native"`, or remove grants

- **You can pin the tier** with an optional `runtime` on the consumption site.
  If the resolved provider disagrees, the load fails and names both runtimes:

  ```rust
  constructor!(
      id = "read_config",
      from = "cloacina-provider-fs@0.1",
      constructor = "read_file",
      config = { path = "/etc/app.toml" },
      grants = { fs = ["ro:/etc"] },
      runtime = "wasm",          // fail the load if this provider ever goes native
  );
  ```

  `#[reactor(.., runtime = "..")]` takes the same pin. Writing
  `runtime = "native"` is the explicit acknowledgement: the node loads, the
  grants stay advisory, and the loader logs a warning saying so.

Grants-free native loads are unaffected — no error, no new requirement. On the
producer side, changing a provider's runtime is a MAJOR/breaking change, checked
against `providers/COMPAT.toml` by the provider wave guard.

The runnable `examples/constructor-contract/fs-grant-demo` demonstrates all
three outcomes side by side: a granted read, the same read denied without the
grant, and a granted write through a second suite member.
