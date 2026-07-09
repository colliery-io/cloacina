---
title: "Compiler Build Sandbox"
description: "How cloacina-compiler isolates the attacker-controlled cargo build (build.rs, proc-macros)."
weight: 50
---

# Compiler Build Sandbox

`cloacina-compiler` compiles uploaded packages, which means it runs
**attacker-controlled code** at build time — `build.rs` and proc-macros
execute on the build host. Phase 1 capped *cost* (rlimits,
`--frozen --offline`, a curated vendor registry). Phase 2 isolates the
*process*.

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

On Kubernetes, the `cloacina-server` Helm chart templates a compiler
Deployment (`compiler.enabled=true`) that sets
`securityContext.seccompProfile.type: Unconfined` for you whenever the sandbox
is active, and defaults `CLOACINA_COMPILER_SANDBOX` to `required`:

```yaml
# values.yaml
compiler:
  enabled: true
  sandbox:
    mode: required          # fail-closed; or preferred / off
    seccompProfile: Unconfined
```

Relaxing seccomp is defensible: bwrap's **per-build** namespace isolation is
stronger than default-seccomp with no build sandbox at all. A container that
still can't run bwrap (no userns even with seccomp relaxed) correctly
downgrades to landlock under `preferred`, or fails to boot under `required`.

The compiler image must include `bubblewrap` (the demo image does).

## Verifying

`cargo test -p cloacina-compiler --lib sandbox` proves the contract at two
levels:

- **Deterministic (runs everywhere):** `bwrap_command_enforces_isolation_contract`
  asserts the bwrap command composition carries the per-namespace unshares
  (`--unshare-net` — the security-critical one, plus `--unshare-user`),
  `--die-with-parent`, and RO-binds `/proc` rather than mounting a fresh procfs
  (`--unshare-all` with a fresh `--proc` mount fails unprivileged in a
  container, so the sandbox deliberately does not emit it); that it `--clearenv`
  and rebuilds the environment; binds the curated registry read-only; binds
  **only** the staged source writable (no other writable host path); and
  launches cargo inside the sandbox. `build_env_is_an_allowlist_not_a_denylist`
  proves `build_env` is an *allowlist* — a secret set in the parent process
  never crosses into the build. `non_bwrap_levels_run_cargo_directly` confirms
  the landlock/none levels invoke cargo unwrapped.
- **End-to-end (runs where bwrap works):** `bwrap_denies_host_write` executes a
  real sandboxed command that attempts to write outside its staged directory
  and asserts the write is denied and leaves no host trace. It **skips** (not
  fails) where bwrap is unusable — so the suite is green on macOS/dev while CI
  proves the enforcement on Linux.

The compiler also logs its probed level at boot and stamps `sandbox_level` on
every build audit event.
