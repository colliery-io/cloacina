---
title: "Compiler Build Sandbox"
description: "How cloacina-compiler isolates the attacker-controlled cargo build (build.rs, proc-macros)."
weight: 50
---

# Compiler Build Sandbox

`cloacina-compiler` compiles uploaded packages, which means it runs
**attacker-controlled code** at build time — `build.rs` and proc-macros
execute on the build host. Phase 1 (CLOACI-I-0104) capped *cost* (rlimits,
`--frozen --offline`, a curated vendor registry). Phase 2 (CLOACI-I-0105)
isolates the *process*.

## Selecting the mode

`CLOACINA_COMPILER_SANDBOX` — probed once at boot into the level every build
runs under:

| Value | Behavior |
|-------|----------|
| `required` | Builds run under **bwrap** or the compiler **refuses to start**. The multi-tenant posture. |
| `preferred` (default) | Best available level; downgrades are logged loudly. |
| `off` | No process sandbox (dev laptops / macOS); logged loudly at boot. |

The bwrap probe actually *runs* bwrap with the real namespace + mount shape,
so a container that can't create the sandbox correctly **downgrades (or, under
`required`, fails)** at boot rather than passing the probe and then breaking
every build.

## The isolation ladder

**Level 1 — bwrap** (namespaced): `--unshare-net` (no network),
`--unshare-user/ipc/uts/cgroup`, `--clearenv` (a `build.rs` cannot read
`DATABASE_URL`), read-only binds for the toolchain + curated registry,
writable binds for **only** the staged source + shared target cache, tmpfs
`/tmp`, RO-bound `/proc`, `--die-with-parent`.

**Level 2 — landlock** (containers without user namespaces, kernel ≥5.13):
kernel filesystem ACLs — read-only everything, read-write only the build dir +
target cache. No namespace isolation; the environment is still scrubbed.

Phase-1 rlimits (CPU / address-space / FD / proc ceilings) apply at every
level. Every build's audit row records the **achieved** isolation level, so
forensics can prove what contained a given build.

## Running the compiler in a container

bwrap needs unprivileged **user namespaces**, which Docker's default seccomp
profile blocks. To get level 1 in a container, relax seccomp for the compiler:

```yaml
# docker-compose
services:
  compiler:
    security_opt:
      - seccomp=unconfined
    environment:
      CLOACINA_COMPILER_SANDBOX: preferred   # or `required` for multi-tenant
```

On Kubernetes, set `securityContext.seccompProfile.type: Unconfined` on the
compiler pod. This is defensible: bwrap's **per-build** namespace isolation is
stronger than default-seccomp with no build sandbox at all. A container that
still can't run bwrap (no userns even with seccomp relaxed) correctly
downgrades to landlock under `preferred`, or fails to boot under `required`.

The compiler image must include `bubblewrap` (the demo image does).

## Verifying

The adversarial test `bwrap_build_cannot_escape_to_host_or_network` builds a
package whose `build.rs` tries to read a host file and open a socket; both must
fail (the build succeeds only because the escapes were blocked). It skips where
bwrap is unusable — the assertion means something only at level 1.
