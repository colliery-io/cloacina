---
title: "Run cloacina-compiler in Production"
description: "Deployment posture for the cloacina-compiler service: threat model, mitigations, vendor curation, audit reading."
weight: 28
---

# How to Run `cloacina-compiler` in Production

This guide covers operating the long-running `cloacina-compiler`
service: the binary that claims pending build rows from the database,
compiles user-supplied package source into `.so` / `.dylib`
artifacts, and writes them back for runner instances to load.

This is the **service** path. For laptop / CI use of
`cloacinactl package build` + `pack`, see
[Use cloacina-compiler Locally]({{< ref "/service/how-to/use-cloacina-compiler-locally" >}}).

## Threat model

State this plainly so there are no illusions: **a malicious
`build.rs` in a submitted package is code execution on the compiler
host**. The compiler runs `cargo build` on attacker-supplied source.
Cargo executes `build.rs` as part of that build. Anything `build.rs`
can do — read files the compiler can read, contact endpoints
reachable from the compiler, exhaust resources the kernel doesn't
bound — happens on your infrastructure.

Phase 1 mitigations (this guide) bound **what** and **how much**.
They do not prevent code execution. Phase 2 (CLOACI-I-0105) adds a
kernel-enforced process sandbox (bubblewrap + landlock) that confines
the cargo subprocess to a tmpfs root with no host filesystem access
and no outbound network. Until Phase 2 lands, the operator
responsibilities below are how you keep blast radius bounded.

## Operator responsibilities (Phase 1)

The compiler trusts the operator's configuration over the submitter's
source. Five things you must get right:

### 1. Run under a dedicated unprivileged UID

Create a `cloacina-compiler` system user with no shell, no sudo, and
no membership in groups that grant filesystem or service access. The
compiler needs:

- Read/write to its `--home` directory (logs, build tmp).
- Read/write to its `CARGO_TARGET_DIR` (shared target cache, if set).
- Read access to the configured `--vendor-dir` (curated `CARGO_HOME`).
- A database role with `SELECT`/`UPDATE` on `workflow_packages` and
  `INSERT` on the audit-event sink, **nothing more**. In particular,
  the compiler must **not** share the admin DB role that the server
  uses — a malicious `build.rs` reading `DATABASE_URL` from the
  process environment would otherwise gain admin DB access to every
  tenant.

### 2. No outbound network beyond the vendor dir

`--frozen --offline` is the default. The cargo subprocess fails fast
on any dep that isn't in the vendor dir. Pair that with a network
namespace (or, until Phase 2, a host firewall) that drops outbound
connections from the `cloacina-compiler` UID — defense in depth
against any future cargo flag change.

### 3. Configure `--build-timeout-s`

Default 600s. Tune per workload:

- Small workspaces (1-2 crate trees, ~50 dep crates): 300s plenty.
- Large workspaces (workspace member graphs, big proc-macro chains):
  900s+ may be needed for cold-cache builds.

Builds that exceed the timeout are SIGKILL'd; the row's heartbeat
stops and the existing stale-build sweeper resets it to `pending`.
The build can be retried, but the operator should investigate why a
package needs >600s before raising the cap blindly.

### 4. Configure `--build-rlimit-*`

Four kernel-enforced ceilings, Linux-only. Defaults are conservative
starting points; tune per workload.

| Flag | Default | What it bounds |
|---|---|---|
| `--build-rlimit-cpu` | `--build-timeout-s` (600s) | CPU-seconds. Tracks the wall-clock as a generous upper bound. |
| `--build-rlimit-mem` | `4G` | Virtual address space, bytes. Accepts `K`/`M`/`G` suffixes. |
| `--build-rlimit-files` | `1024` | Open file descriptors. |
| `--build-rlimit-procs` | `256` | User processes (bounds fork bombs). |

**Tuning notes:**

- **Memory:** release builds of crates with heavy generics can peak
  >4 GiB. If a known-good package fails with no obvious reason and
  `exit_signal=SIGKILL`/`SIGSEGV` in the audit log, bump `-mem` first.
- **Procs:** parallel cargo with many cores spawns hundreds of rustc
  invocations. If you see builds hang on a `-j32` host, raise this.
- **No "disabled" sentinel.** To remove a ceiling, pass a large value
  (the kernel cap on most systems is `RLIM_INFINITY` = `u64::MAX`).
  There is no `--build-rlimit-mem=none` syntax.

### 5. Curate the vendor dir

The compiler reads cargo deps from `--vendor-dir` (env
`CLOACINA_COMPILER_VENDOR_DIR`), default `~/.cargo`. The operator
populates it via `cargo vendor`; see the next section.

## The `cargo vendor` workflow

The operator's vendor dir is the allowlist of crates submitted
packages can resolve under `--offline`. Curate it explicitly.

1. **Start with a known-good source tree** that lists every crate you
   want to permit (typically the cloacina workspace itself + any
   in-house workflow crates your authors share):

   ```bash
   git clone https://github.com/colliery-io/cloacina /tmp/cloacina
   cd /tmp/cloacina
   ```

2. **Run `cargo vendor`** with output pointing at the compiler's
   vendor dir:

   ```bash
   cargo vendor --locked /var/lib/cloacina-compiler/cargo/registry
   ```

   This populates `<vendor-dir>/registry/{cache,src}` with every
   transitive dep referenced in `Cargo.lock`. The output emits a
   `.cargo/config.toml` snippet that tells cargo to use the
   vendored sources — copy that into
   `<vendor-dir>/config.toml`.

3. **Point the compiler at it:**

   ```bash
   cloacina-compiler \
     --vendor-dir /var/lib/cloacina-compiler/cargo \
     --database-url postgres://cloacina_compiler:...@db/cloacina
   ```

   Or via env:

   ```bash
   export CLOACINA_COMPILER_VENDOR_DIR=/var/lib/cloacina-compiler/cargo
   export CLOACINA_COMPILER_BUILD_TIMEOUT_S=600
   cloacina-compiler --database-url $DATABASE_URL
   ```

4. **Adding an in-house crate.** Vendor it into a sibling source
   tree, then re-run `cargo vendor` against the union, and replace
   the compiler's vendor dir. Restart the compiler.

## Flag reference

All compiler flags accept env equivalents (`CLOACINA_COMPILER_*`).

| Flag | Env | Default | Source |
|---|---|---|---|
| `--build-timeout-s` | `CLOACINA_COMPILER_BUILD_TIMEOUT_S` | 600 | T-0573 |
| `--vendor-dir` | `CLOACINA_COMPILER_VENDOR_DIR` | unset (cargo `~/.cargo`) | T-0574 |
| `--cargo-flag` (repeatable) | — | `build --release --lib --frozen --offline` | T-0574 |
| `--build-rlimit-cpu` | `CLOACINA_COMPILER_BUILD_RLIMIT_CPU` | = `--build-timeout-s` | T-0575 |
| `--build-rlimit-mem` | `CLOACINA_COMPILER_BUILD_RLIMIT_MEM` | `4G` | T-0575 |
| `--build-rlimit-files` | `CLOACINA_COMPILER_BUILD_RLIMIT_FILES` | 1024 | T-0575 |
| `--build-rlimit-procs` | `CLOACINA_COMPILER_BUILD_RLIMIT_PROCS` | 256 | T-0575 |
| `--cargo-target-dir` | — | unset (per-build `target/`) | — |
| `--home` | — | `$HOME/.cloacina` | — |
| `--database-url` | `DATABASE_URL` | required | — |

> **Setting `--cargo-flag` replaces the entire default list.** If you
> override to add a flag, include `--frozen` and `--offline`
> explicitly or you'll lose the offline posture.

## Audit events: what to grep

The compiler emits two structured events per build via `tracing`:

- **`compiler.build.started`** — emitted after the source archive
  unpacks and content hashes are computed, just before the cargo
  subprocess fires.
- **`compiler.build.finished`** — emitted exactly once per build on
  every outcome path (success, failure, timeout-kill).

Pipe `tracing` to your sink of choice (file, journald, Loki, SIEM).
The event fields:

### `compiler.build.started` fields

- `build_claim_id` — UUID of the build row.
- `package_name`, `package_version` — what was submitted.
- `cargo_toml_hash` — SHA-256 hex of the unpacked `Cargo.toml`, or
  `<absent>` for non-Rust packages.
- `cargo_lock_hash` — SHA-256 hex of the unpacked `Cargo.lock`, or
  `<none>`.
- `compiler_instance_id` — UUID of the compiler process. Generated
  at startup and stamped on the "cloacina-compiler starting" log
  line, so you can correlate every build to a specific compiler
  instance.

### `compiler.build.finished` fields

All of the started fields, plus:

- `outcome` — one of `success`, `failed`, `timeout_killed`,
  `internal_error`.
- `exit_status` — cargo's exit code if it exited normally, else
  `<none>`.
- `exit_signal` — signal name (`SIGKILL`, `SIGSEGV`, `SIGABRT`, …)
  if cargo was signal-terminated, else `<none>`.
- `wall_clock_ms` — total time from `run_build` entry to emit.
- `failure_reason` — operator-actionable message on failure (e.g.
  `dependencies not available offline: foo, bar`), else `<none>`.

### Grep recipes

Reconstruct a build by claim id:

```bash
grep '"build_claim_id":"<uuid>"' /var/log/cloacina/compiler.log
```

Find all rlimit-like kills in the last day:

```bash
grep '"event_type":"compiler.build.finished"' /var/log/cloacina/compiler.log \
  | grep '"outcome":"failed"' \
  | grep -E '"exit_signal":"(SIGKILL|SIGSEGV|SIGABRT)"'
```

(`rlimit_killed` collapses into `outcome=failed` with a signal-based
`exit_signal`. A dedicated outcome bucket is intentionally not added
in Phase 1 — heuristic on signal alone is fragile across kernels.)

Find every build of a specific package version:

```bash
grep '"event_type":"compiler.build.finished"' /var/log/cloacina/compiler.log \
  | grep '"package_name":"my-workflow"' \
  | grep '"package_version":"1.2.3"'
```

## What Phase 2 will add

Phase 1 bounds resources and confines the registry. Phase 2
(CLOACI-I-0105) closes the gap with a process sandbox:

- **bubblewrap** for namespace isolation: the cargo subprocess sees
  only a tmpfs build root, the curated vendor dir mounted RO, and
  the bare minimum of `/usr` for the toolchain. The host filesystem
  is invisible.
- **landlock** as defense-in-depth where the kernel supports it
  (Linux 5.13+).
- **Network: closed by default** — no outbound connections, period.
  The vendor dir bind-mount supersedes any registry the build script
  might attempt to reach.

Until that lands, the Phase 1 posture in this guide is your bound.

## Observability

`cloacina-compiler` exposes a `/metrics` Prometheus endpoint on the same port as `/health` and `/v1/status` (default `127.0.0.1:9000`) per CLOACI-I-0109. The relevant counters/histograms/gauges are documented in the [Metrics Catalog]({{< ref "/platform/reference/metrics-catalog" >}}#compiler-metrics) — `cloacina_compiler_builds_total{status}`, `cloacina_compiler_queue_depth{state}`, `cloacina_compiler_sweep_resets_total`, `cloacina_compiler_build_duration_seconds`.

Daily-rotated structured logs land in `~/.cloacina/logs/cloacina-compiler.log`. Retention is controlled by `--log-retention-days <N>` (default 14, `0` disables pruning). The same flag is supported on `cloacinactl daemon start` and `cloacinactl server start` for symmetry across the three deployables.

## Related

- [Compiler + Server Deployment Runbook]({{< ref "/service/how-to/compiler-deployment-runbook" >}}) — long-form runbook for the server + compiler pair across bare-metal, Compose, and Kubernetes.
- [Production Deployment]({{< ref "/service/how-to/production-deployment" >}}) — TLS termination for the `cloacinactl server start` server. Separate concern from the compiler.
- [Use cloacina-compiler Locally]({{< ref "/service/how-to/use-cloacina-compiler-locally" >}}) — local laptop / CI path, no service.
- [Metrics Catalog]({{< ref "/platform/reference/metrics-catalog" >}}) — the full `cloacina_*` and `cloacina_compiler_*` metric surface.
- **ADR-0005** — Deployment-mode trust model (why the compiler is Linux-only, single-tenant build).
- **CLOACI-I-0104** — Phase 1 hardening initiative (timeouts, offline, setrlimit).
- **CLOACI-I-0105** — Phase 2 sandbox initiative (process-level isolation, pending).
- **CLOACI-I-0109** — `/metrics` endpoint + `--log-retention-days`.
