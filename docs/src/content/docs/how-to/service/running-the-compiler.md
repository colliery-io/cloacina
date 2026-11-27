---
title: "How to Run `cloacina-compiler` in Production"
description: "Deployment posture for the cloacina-compiler service: threat model, mitigations, vendor curation, audit reading."
sidebar:
  order: 28
  label: "Run cloacina-compiler in Production"
---

This guide covers operating the long-running `cloacina-compiler`
service: the binary that claims pending build rows from the database,
compiles user-supplied package source into `.so` / `.dylib`
artifacts, and writes them back for runner instances to load.

This is the **service** path. For laptop / CI use of
`cloacinactl package build` + `pack`, see
[Use cloacina-compiler Locally](../use-cloacina-compiler-locally/).

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
burns that tenant's compiler. Within a compiler, every child process of a
build (`cargo fetch`, the build, and the provider bundling steps: `cargo
metadata`, the provider crate's build and its manifest binary) gets a
cleared environment, kernel `setrlimit` ceilings (see below) and the
wall-clock timeout. The child inherits only `PATH`, `HOME`, `CARGO_HOME`,
`RUSTUP_HOME`, `RUSTUP_TOOLCHAIN`, `TMPDIR`, `TERM` and
`CARGO_PROFILE_DEV_DEBUG`; `cargo fetch` and `cargo metadata` also inherit
the proxy variables (`HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`, `ALL_PROXY`
and their lower-case forms), `SSL_CERT_FILE`, `SSL_CERT_DIR`,
`CARGO_HTTP_PROXY`, `CARGO_HTTP_CAINFO`, `CARGO_HTTP_TIMEOUT`,
`CARGO_HTTP_MULTIPLEXING`, `CARGO_NET_RETRY`, `CARGO_NET_GIT_FETCH_WITH_CLI`,
`CARGO_NET_OFFLINE` and `CARGO_REGISTRIES_CRATES_IO_PROTOCOL`. Other cargo
settings, registry credentials included, go in the cargo home's files.
There is no per-build process sandbox; the operator responsibilities below
bound *what* a build can resolve and *how much* it can consume.

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
  uses. A build does not inherit `DATABASE_URL`, but build code has the
  compiler's UID: on Linux it can read the compiler's command line and
  environment through `/proc`, and with the admin role it would gain
  admin DB access to every tenant.

### 2. No outbound network beyond the vendor dir

`--frozen --offline` is the default of the binary. The cargo subprocess
fails fast on any dep that isn't in the vendor dir. (Without
`--frozen`/`--offline` the compiler runs a two-phase build instead: `cargo
fetch` resolves and downloads the graph, then the compile runs with
`--offline` appended. The Helm chart's default `compiler.extraArgs` replace
the flags with `build --lib`, so a chart deployment builds this way.)

`--offline` stops cargo from downloading. It does not stop a build script,
a proc-macro or a provider's manifest binary from opening a connection, and
provider bundling runs its own cargo steps without these flags. So the
firewall is the control: drop outbound connections from the
`cloacina-compiler` UID (or, on Kubernetes, give the compiler pods an
egress NetworkPolicy), and allow only the registry hosts when you run
online builds.

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

## How many compilers a deployment runs

A compiler serves one schema. A deployment runs:

- **One compiler for `public`**, that you deploy: the Helm chart's
  `compiler.enabled=true`, or your own process. It runs all the time.
- **One compiler per tenant**, that you deploy with `--tenant-schema`, or that
  the server makes for you. Without one, a package published to the tenant
  stays `pending`: a Rust package and a Python package alike, because the
  compiler is the process that completes every package (it runs no cargo for
  a Python one).

### Compilers the server makes

With a fleet actuator (`CLOACINA_FLEET_ACTUATOR=kubernetes` or `docker`) and
`CLOACINA_TENANT_COMPILERS=true`, the server runs the tenant compilers itself.
The Helm chart sets both with `fleet.actuator=kubernetes`
(`fleet.compiler.enabled` is `true` by default). No step is needed when a
tenant is created.

On each tick of the fleet loop (`CLOACINA_AUTOSCALE_INTERVAL_S`, default 30 s)
the server reads the build queue of every tenant schema: the `pending` and
`building` rows of `workflow_packages`, the rows the compiler claims.

| The tenant's queue | The tenant's compiler |
|---|---|
| has a `pending` or `building` package | is created if there is none, and set to 1 replica |
| is empty, and the last build ended or failed less than the cooldown ago | stays as it is |
| has been empty for the cooldown (`CLOACINA_TENANT_COMPILER_COOLDOWN_S`, default 300 s) | is set to 0 replicas |
| never held a package | does not exist |

A tenant has at most one compiler. The server keeps no record of the wanted
state: it works it out from the queue on each tick, so any replica can lead a
tick. A removed tenant loses its compiler: `DELETE /v1/tenants/{name}` deletes
the Deployment and its NetworkPolicy and writes an empty value over the
database URL Secret.

On Kubernetes the compiler of a tenant is a Deployment `cloacina-compiler` in
the tenant's namespace (`cloacina-tenant-<tenant>`), next to its agents. With
the Docker actuator it is one container with the labels
`cloacina.compiler=true` and `cloacina.compiler-tenant=<tenant>`; the
resources, the hardening, the cache volumes and the NetworkPolicy below do not
apply there.

**A start is a cold start.** The pod's cargo home and target directory are
`emptyDir` volumes, so a compiler that starts has empty caches: its first
build downloads and compiles every dependency of the package. Builds that
follow while it runs reuse them. A longer cooldown keeps the caches longer; it
also keeps the pod's requested CPU and memory reserved.

**Settings** (environment of the server; the chart renders them from
`fleet.compiler.*`):

| Variable | Chart value | Default | What it sets |
|---|---|---|---|
| `CLOACINA_TENANT_COMPILERS` | `enabled` | off (chart: on) | Run a compiler per tenant. |
| `CLOACINA_TENANT_COMPILER_IMAGE` | `image` | `cloacina-compiler:latest` (chart: the image of `compiler.image`) | The compiler image. |
| `CLOACINA_TENANT_COMPILER_CARGO_FLAGS` | `cargoFlags` | empty (chart: `build --lib`) | The whole cargo flag list, separated by spaces (`--cargo-flags-replace`). Empty keeps `build --release --lib --frozen --offline`. |
| `CLOACINA_TENANT_COMPILER_VENDOR_DIR` | `vendorDir` | unset | `--vendor-dir`: a path inside the image. |
| `CLOACINA_TENANT_COMPILER_DEV_WORKSPACE` | `devWorkspace` | unset | `--dev-workspace`: a path inside the image. Development only. |
| `CLOACINA_TENANT_COMPILER_TARGET_DIR` | `targetDir` | `/workspace/target` | `--cargo-target-dir`. |
| `CLOACINA_TENANT_COMPILER_BUILD_TIMEOUT_S` | `buildTimeoutSeconds` | unset (the compiler's 600) | `--build-timeout-s`. |
| `CLOACINA_TENANT_COMPILER_POLL_INTERVAL_MS` | `pollIntervalMs` | `1000` | `--poll-interval-ms`. |
| `CLOACINA_TENANT_COMPILER_COOLDOWN_S` | `cooldownSeconds` | `300` | How long the queue must be empty before the compiler is set to 0. |
| `CLOACINA_TENANT_COMPILER_CPU_REQUEST`, `_MEMORY_REQUEST`, `_CPU_LIMIT`, `_MEMORY_LIMIT` | `resources` | `500m`, `1Gi`, `2`, `4Gi` | Requests and limits of the compiler container. |
| `CLOACINA_TENANT_COMPILER_CACHE_SIZE` | `cacheSize` | `8Gi` | Size limit of each cache volume (cargo home, target directory). |
| `CLOACINA_TENANT_COMPILER_HARDENED` | `hardened` | `true` | The pod hardening below. |
| `CLOACINA_TENANT_COMPILER_DATABASE_URL` | `databaseUrlSecretRef` | the server's own URL | The database URL of the tenant compilers. |
| `CLOACINA_TENANT_COMPILER_EGRESS` | `egress` | `internet` | Where a build may fetch crates from: `internet`, `all` or `none`. |
| `CLOACINA_TENANT_COMPILER_EGRESS_CIDRS` | `egressCidrs` | empty | More egress CIDR blocks, every port. |
| `CLOACINA_TENANT_COMPILER_DATABASE_CIDRS` | `databaseCidrs` | empty | CIDR blocks of the database. |

A value the server cannot use (a CIDR block that does not parse, an egress
mode it does not know) stops the server at start with a message that names
the variable.

**The pod.** With `hardened` the pod runs as uid and gid `10001`, not root,
with the `RuntimeDefault` seccomp profile, a read-only root filesystem, no
capabilities and no privilege escalation, and mounts no service account
token. The paths a build writes are `emptyDir` volumes: the compiler's home
(`/var/cloacina/home`: unpacked sources and logs), the cargo home
(`/var/cloacina/cargo`), the target directory and `/tmp`. The image must be
usable by uid `10001`: the toolchain readable, and nothing it needs written
outside those four paths. `hardened=false` runs the image as it is built, on
its own filesystem; use it only for a development image that carries its
caches. The compiler listens on `127.0.0.1:9000` only, and the pod has no
probe.

**The database URL.** The server writes the URL into a Secret
`cloacina-compiler-db` in the tenant's namespace, and the pod reads it as the
environment variable `DATABASE_URL`. It is not on the pod's command line. By
default it is the server's own URL, which reaches every schema: the same
credential the chart's own compiler has. To give the tenant compilers a
weaker one, set `CLOACINA_TENANT_COMPILER_DATABASE_URL` (chart:
`fleet.compiler.databaseUrlSecretRef`) to the URL of a role you made for
builds. A tenant compiler reads and writes four tables of its tenant's
schema, `workflow_packages`, `workflow_registry`, `package_providers` and
`package_artifacts`, and nothing in `public`. All tenant compilers get the
same URL. The host in the URL must resolve from the tenant namespaces: use a
full Service name (`<service>.<namespace>.svc.cluster.local`), not a short
one. A changed URL reaches a compiler when it next starts.

**The network.** With `CLOACINA_FLEET_NETWORK_POLICY` on (the default), the
server writes a NetworkPolicy `cloacina-compiler` for the compiler pods:

- No ingress.
- Egress to cluster DNS.
- Egress to the database: TCP on the port of the database URL, to the blocks
  in `CLOACINA_TENANT_COMPILER_DATABASE_CIDRS`, or to every destination when
  that list is empty (a NetworkPolicy cannot name a host).
- Egress for crates, by `CLOACINA_TENANT_COMPILER_EGRESS`:
  - `internet` (the default): TCP 443 to every public IPv4 address. The
    private ranges (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`,
    `100.64.0.0/10`) and the link-local range (`169.254.0.0/16`) are left
    out, so this rule opens no path to the cluster, the nodes or a cloud
    metadata address.
  - `all`: every destination and port.
  - `none`: nothing more.
- Egress to each block in `CLOACINA_TENANT_COMPILER_EGRESS_CIDRS`, on every
  port: a registry mirror, a git host, a proxy.

The pods also get what the tenant's agent policy allows: DNS and the server.

**An install with no route to a registry** sets `egress` to `none`, leaves
`cargoFlags` empty so that builds run `--frozen --offline`, and sets
`vendorDir` to a cargo home inside the compiler image that holds every crate
a build may use (see [The `cargo vendor` workflow](#the-cargo-vendor-workflow)).
With a mirror inside the network, list its CIDR in `egressCidrs` instead.

## Flag reference

All compiler flags accept env equivalents (`CLOACINA_COMPILER_*`).

| Flag | Env | Default |
|---|---|---|
| `--build-timeout-s` | `CLOACINA_COMPILER_BUILD_TIMEOUT_S` | 600 |
| `--vendor-dir` | `CLOACINA_COMPILER_VENDOR_DIR` | unset (cargo `~/.cargo`) |
| `--cargo-flag` (repeatable) | — | appends to `build --release --lib --frozen --offline` |
| `--cargo-flags-replace` (repeatable) | — | unset (full override of the default flag list) |
| `--build-rlimit-cpu` | `CLOACINA_COMPILER_BUILD_RLIMIT_CPU` | = `--build-timeout-s` |
| `--build-rlimit-mem` | `CLOACINA_COMPILER_BUILD_RLIMIT_MEM` | `4G` |
| `--build-rlimit-files` | `CLOACINA_COMPILER_BUILD_RLIMIT_FILES` | 1024 |
| `--build-rlimit-procs` | `CLOACINA_COMPILER_BUILD_RLIMIT_PROCS` | 256 |
| `--max-unpacked-bytes` | `CLOACINA_MAX_UNPACKED_BYTES` | 524288000 (500 MiB) |
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
  leakage). Run one compiler per tenant, or let the server run them (see
  [How many compilers a deployment runs](#how-many-compilers-a-deployment-runs)).
  Omit for the default public schema.
- `--build-target` runs the compiler as a per-target builder producing
  cdylibs for a given triple (e.g. `x86_64-linux`), backfilling
  `package_artifacts` for success packages that lack that architecture. Run
  the container on that arch. `--build-target-package` restricts that scan to
  a single package name (only meaningful with `--build-target`).

> **`--cargo-flag` appends to the default list** (`build --release --lib
> --frozen --offline`), so adding a flag never silently drops the offline
> posture. To replace the whole list — e.g. to deliberately run an online
> build without `--frozen --offline` — use the explicit
> `--cargo-flags-replace` escape hatch; the list you supply must start with
> the cargo subcommand (usually `build`). The two flags conflict.

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

`cloacina-compiler` exposes a `/metrics` Prometheus endpoint on the same port as `/health` and `/v1/status` (default `127.0.0.1:9000`). With `--metrics-bind <ADDR>` (env `CLOACINA_COMPILER_METRICS_BIND`) it is served only on that address, and the `--bind` listener answers 404 for `/metrics`. The relevant counters/histograms/gauges are documented in the [Metrics Catalog](../../../reference/metrics-catalog/#compiler-metrics) — `cloacina_compiler_builds_total{status}`, `cloacina_compiler_queue_depth{state}`, `cloacina_compiler_sweep_resets_total`, `cloacina_compiler_build_duration_seconds`.

Daily-rotated structured logs land in `~/.cloacina/logs/cloacina-compiler.log`. Retention is controlled by `--log-retention-days <N>` (default 14, `0` disables pruning). The same flag is supported on `cloacinactl daemon start` and `cloacinactl server start` for symmetry across the three deployables.

## Related

- [Compiler + Server Deployment Runbook](../compiler-deployment-runbook/) — long-form runbook for the server + compiler pair across bare-metal, Compose, and Kubernetes.
- [Production Deployment](../production-deployment/) — TLS termination for the `cloacinactl server start` server. Separate concern from the compiler.
- [Use cloacina-compiler Locally](../use-cloacina-compiler-locally/) — local laptop / CI path, no service.
- [Metrics Catalog](../../../reference/metrics-catalog/) — the full `cloacina_*` and `cloacina_compiler_*` metric surface.
- **ADR-0005** — Deployment-mode trust model (why the compiler is Linux-only, single-tenant build).
