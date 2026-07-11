---
title: "Run cloacina-compiler in Production"
description: "Deployment posture for the cloacina-compiler service: threat model, mitigations, vendor curation, audit reading."
weight: 28
aliases:
  - "/platform/how-to-guides/running-the-compiler/"

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

Cloacina's isolation boundary is the **tenant**: run one compiler per
tenant (`--tenant-schema`) so a tenant's source, build logs, and artifacts
never mix with another's, and size the deployment so a hostile build only
burns that tenant's compiler. Within a compiler, kernel `setrlimit` ceilings
bound resource cost per build (see below), and the wall-clock timeout kills
runaway builds. There is no per-build process sandbox; the operator
responsibilities below bound *what* a build can resolve and *how much* it
can consume.

## Operator responsibilities

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
on any dep that isn't in the vendor dir. (Without `--frozen`/`--offline`
the compiler runs a two-phase build instead: `cargo fetch` resolves and
downloads the graph, then the compile runs with `--offline` appended.)
As belt-and-suspenders, pair the curated posture with
a host firewall that drops outbound connections from the
`cloacina-compiler` UID.

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

| Flag | Env | Default |
|---|---|---|
| `--build-timeout-s` | `CLOACINA_COMPILER_BUILD_TIMEOUT_S` | 600 |
| `--vendor-dir` | `CLOACINA_COMPILER_VENDOR_DIR` | unset (cargo `~/.cargo`) |
| `--cargo-flag` (repeatable) | — | `build --release --lib --frozen --offline` |
| `--build-rlimit-cpu` | `CLOACINA_COMPILER_BUILD_RLIMIT_CPU` | = `--build-timeout-s` |
| `--build-rlimit-mem` | `CLOACINA_COMPILER_BUILD_RLIMIT_MEM` | `4G` |
| `--build-rlimit-files` | `CLOACINA_COMPILER_BUILD_RLIMIT_FILES` | 1024 |
| `--build-rlimit-procs` | `CLOACINA_COMPILER_BUILD_RLIMIT_PROCS` | 256 |
| `--cargo-target-dir` | — | unset (per-build `target/`) |
| `--tenant-schema` | `CLOACINA_TENANT_SCHEMA` | unset (public schema) |
| `--build-target` | `CLOACINA_BUILD_TARGET` | unset (native host build) |
| `--build-target-package` | `CLOACINA_BUILD_TARGET_PACKAGE` | unset (all packages) |
| `--home` | — | `$HOME/.cloacina` |
| `--database-url` | `DATABASE_URL` | required |

**Tenant / target flags:**

- `--tenant-schema` scopes the compiler to a single tenant's Postgres schema
  for build isolation: it claims and builds **only** that tenant's pending
  packages, with separate source, logs, and target dir (no cross-tenant
  leakage). Run one compiler per tenant. Omit for the default public schema.
- `--build-target` runs the compiler as a per-target builder producing
  cdylibs for a given triple (e.g. `x86_64-linux`), backfilling
  `package_artifacts` for success packages that lack that architecture. Run
  the container on that arch. `--build-target-package` restricts that scan to
  a single package name (only meaningful with `--build-target`).

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
`exit_signal`. A dedicated outcome bucket is intentionally not added —
a heuristic on signal alone is fragile across kernels.)

Find every build of a specific package version:

```bash
grep '"event_type":"compiler.build.finished"' /var/log/cloacina/compiler.log \
  | grep '"package_name":"my-workflow"' \
  | grep '"package_version":"1.2.3"'
```

## Observability

`cloacina-compiler` exposes a `/metrics` Prometheus endpoint on the same port as `/health` and `/v1/status` (default `127.0.0.1:9000`). The relevant counters/histograms/gauges are documented in the [Metrics Catalog]({{< ref "/reference/metrics-catalog" >}}#compiler-metrics) — `cloacina_compiler_builds_total{status}`, `cloacina_compiler_queue_depth{state}`, `cloacina_compiler_sweep_resets_total`, `cloacina_compiler_build_duration_seconds`.

Daily-rotated structured logs land in `~/.cloacina/logs/cloacina-compiler.log`. Retention is controlled by `--log-retention-days <N>` (default 14, `0` disables pruning). The same flag is supported on `cloacinactl daemon start` and `cloacinactl server start` for symmetry across the three deployables.

## Related

- [Compiler + Server Deployment Runbook]({{< ref "/service/how-to/compiler-deployment-runbook" >}}) — long-form runbook for the server + compiler pair across bare-metal, Compose, and Kubernetes.
- [Production Deployment]({{< ref "/service/how-to/production-deployment" >}}) — TLS termination for the `cloacinactl server start` server. Separate concern from the compiler.
- [Use cloacina-compiler Locally]({{< ref "/service/how-to/use-cloacina-compiler-locally" >}}) — local laptop / CI path, no service.
- [Metrics Catalog]({{< ref "/reference/metrics-catalog" >}}) — the full `cloacina_*` and `cloacina_compiler_*` metric surface.
- **ADR-0005** — Deployment-mode trust model (why the compiler is Linux-only, single-tenant build).
