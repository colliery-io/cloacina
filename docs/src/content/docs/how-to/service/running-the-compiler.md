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
For a Python package it vendors the dependencies of the package's
lock file: see [Python packages](#python-packages-vendoring-dependencies).
For every package it reads the metadata in a child process and stores it
as one document: see [The describe step](#the-describe-step).

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
build (`cargo fetch`, the build, the provider bundling steps: `cargo
metadata`, the provider crate's build and its manifest binary, and the
describe child) gets a cleared environment, kernel `setrlimit` ceilings
(see below) and a wall-clock timeout. The child inherits only `PATH`, `HOME`, `CARGO_HOME`,
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
- A database role with the rights of a build, **nothing more**: in the
  schema it serves, `SELECT, UPDATE` on `workflow_packages`, `SELECT` on
  `workflow_registry`, `SELECT, INSERT, DELETE` on `package_providers` and
  `package_artifacts`, `SELECT` on `__diesel_schema_migrations` and
  `cloacina_schema_release`, and `USAGE` on the schema. The server makes such a
  role for each tenant and for `public`: `<tenant>$build` and
  `public$build`, with no password (see
  [The build role](#the-build-role-of-a-tenant)). For a compiler you run by
  hand, set one with `ALTER ROLE "<tenant>$build" WITH PASSWORD '…'`. In
  particular, the compiler must **not** share the admin DB role that the
  server uses. A build does not inherit `DATABASE_URL`, but build code has
  the compiler's UID: on Linux it can read the compiler's command line and
  environment through `/proc`, and with the admin role it would gain admin
  DB access to every tenant. A compiler with a build role runs with
  `--tenant-schema <tenant>` (`--tenant-schema public` for `public`).

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

Python builds download wheels unless the compiler runs with
`--python-offline`: see
[What the compiler downloads, and from where](#what-the-compiler-downloads-and-from-where).

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

## The schema check at start

The compiler runs no migration, with or without `--tenant-schema`: only the
server migrates. At start the compiler reads its schema:

- The schema has each migration of the compiler's binary: it starts.
- The schema lacks some (the server has not migrated it yet, or the server is
  older): it waits, logs `waiting for the server to migrate the schema` with
  both releases every 30 seconds, and exits with an error after 10 minutes.
  A compiler started before the server, or upgraded before it, changes no
  schema.
- The schema was last migrated by a release more than one minor ahead of the
  compiler: it exits at once, and the error names both releases.

See [Schema versions](../production-deployment/#schema-versions).

## How many compilers a deployment runs

A compiler serves one schema. A deployment runs:

- **One compiler per tenant, `public` included**, that the server makes for
  you (below), or that you deploy with `--tenant-schema`. Without the server's
  compilers, `public`'s is the Helm chart's `compiler.enabled=true` or your
  own process, and runs all the time. Without one, a package published to the
  tenant
  stays `pending`: a Rust package and a Python package alike, because the
  compiler is the process that completes every package (it runs no cargo for
  a Python one).

### Compilers the server makes

With a fleet actuator (`CLOACINA_FLEET_ACTUATOR=kubernetes` or `docker`) and
`CLOACINA_TENANT_COMPILERS=true`, the server runs the tenant compilers itself.
The Helm chart sets both with `fleet.actuator=kubernetes`
(`fleet.compiler.enabled` is `true` by default). No step is needed when a
tenant is created. `public` is a tenant like the others: its compiler is made
the same way, in `cloacina-tenant-public`, and the chart then renders no
compiler of its own.

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
the Deployment, its NetworkPolicy and its cache claim, and writes an empty
value over the database URL and the Python index URL in the Secret.

On Kubernetes the compiler of a tenant is a Deployment `cloacina-compiler` in
the tenant's namespace (`cloacina-tenant-<tenant>`), next to its agents. With
the Docker actuator it is one container with the labels
`cloacina.compiler=true`, `cloacina.compiler-tenant=<tenant>` and
`cloacina.compiler-credential=build-role`; the resources, the hardening, the
cache volumes, the cache claim and the NetworkPolicy below do not apply there.
The container runs on the image's own filesystem, and its cargo home holds the
toolchain: a volume kept over it would keep an old toolchain after an image
upgrade, so the Docker actuator keeps no cache between starts.

**The caches are kept on a claim.** A hardened tenant compiler mounts a
PersistentVolumeClaim `cloacina-compiler-cache` in the tenant's namespace,
which holds its cargo home, its target directory and its `uv` cache. The
server makes the claim at the first start (with the cluster's default storage
class unless `CLOACINA_TENANT_COMPILER_CACHE_CLAIM_STORAGE_CLASS` names one),
keeps it when the compiler is scaled to 0, and deletes it when the tenant is
removed. So only the first start of a tenant's compiler, or a start on a new
claim, is a cold start: its first build downloads and compiles every
dependency. A claim that exists is not changed: a new size or class applies
to the claim made after the old one is deleted. Nothing prunes the claim; the
target directory grows with the builds, and a full claim fails a build with
no space left. With `CLOACINA_TENANT_COMPILER_CACHE_CLAIM=false`, or with
`hardened=false`, there is no claim: the caches are `emptyDir` volumes, and
every start from 0 replicas is a cold start. A longer cooldown keeps a running
compiler longer; it also keeps the pod's requested CPU and memory reserved.

**Settings** (environment of the server; the chart renders them from
`fleet.compiler.*`):

| Variable | Chart value | Default | What it sets |
|---|---|---|---|
| `CLOACINA_TENANT_COMPILERS` | `enabled` | off (chart: on) | Run a compiler per tenant. |
| `CLOACINA_TENANT_COMPILER_IMAGE` | `image` | `cloacina-compiler:latest` (chart: the image of `compiler.image`) | The compiler image. |
| `CLOACINA_TENANT_COMPILER_CARGO_FLAGS` | `cargoFlags` | empty (chart: `build --lib`) | The whole cargo flag list, separated by spaces (`--cargo-flags-replace`). Empty keeps `build --release --lib --frozen --offline`. |
| `CLOACINA_TENANT_COMPILER_VENDOR_DIR` | `vendorDir` | unset | `--vendor-dir`: a path inside the image. |
| `CLOACINA_TENANT_COMPILER_DEV_WORKSPACE` | `devWorkspace` | unset | `--dev-workspace`: a path inside the image. Development only. |
| `CLOACINA_TENANT_COMPILER_TARGET_DIR` | `targetDir` | `/workspace/target` | `--cargo-target-dir`, when there is no cache claim. With the claim it is `/var/cloacina/cache/target`. |
| `CLOACINA_TENANT_COMPILER_BUILD_TIMEOUT_S` | `buildTimeoutSeconds` | unset (the compiler's 600) | `--build-timeout-s`. |
| `CLOACINA_TENANT_COMPILER_POLL_INTERVAL_MS` | `pollIntervalMs` | `1000` | `--poll-interval-ms`. |
| `CLOACINA_TENANT_COMPILER_COOLDOWN_S` | `cooldownSeconds` | `300` | How long the queue must be empty before the compiler is set to 0. |
| `CLOACINA_TENANT_COMPILER_CPU_REQUEST`, `_MEMORY_REQUEST`, `_CPU_LIMIT`, `_MEMORY_LIMIT` | `resources` | `500m`, `1Gi`, `2`, `4Gi` | Requests and limits of the compiler container. |
| `CLOACINA_TENANT_COMPILER_CACHE_SIZE` | `cacheSize` | `8Gi` | Size limit of each cache `emptyDir` (cargo home, target directory), when there is no cache claim. |
| `CLOACINA_TENANT_COMPILER_CACHE_CLAIM` | `cacheClaim.enabled` | `true` | A PersistentVolumeClaim per tenant compiler for the cargo home, the target directory and the `uv` cache. Hardened pods only. |
| `CLOACINA_TENANT_COMPILER_CACHE_CLAIM_SIZE` | `cacheClaim.size` | `20Gi` | Requested size of the claim: the sum of the `emptyDir` limits it replaces (8Gi, 8Gi, 4Gi). |
| `CLOACINA_TENANT_COMPILER_CACHE_CLAIM_STORAGE_CLASS` | `cacheClaim.storageClass` | unset (the cluster's default class) | Storage class of the claim. |
| `CLOACINA_TENANT_COMPILER_HARDENED` | `hardened` | `true` | The pod hardening below. |
| `CLOACINA_TENANT_COMPILER_DATABASE_URL` | `databaseUrlSecretRef` | the server's own URL | Where the tenant compilers reach the database: host, port, database and parameters. Its user and password are not used: each compiler connects as its tenant's build role. |
| `CLOACINA_TENANT_COMPILER_EGRESS` | `egress` | `internet` | Where a build may fetch crates from: `internet`, `all` or `none`. |
| `CLOACINA_TENANT_COMPILER_EGRESS_CIDRS` | `egressCidrs` | empty | More egress CIDR blocks, every port. |
| `CLOACINA_TENANT_COMPILER_DATABASE_CIDRS` | `databaseCidrs` | empty | CIDR blocks of the database. |
| `CLOACINA_TENANT_COMPILER_HOME_SIZE` | `homeSize` | `4Gi` | Size limit of the home volume of a hardened pod. |
| `CLOACINA_TENANT_COMPILER_PYTHON_PLATFORMS` | `python.platforms` | unset (the compiler's `aarch64-linux`, `x86_64-linux`) | `CLOACINA_COMPILER_PYTHON_PLATFORMS` of the compiler, comma-separated. |
| `CLOACINA_TENANT_COMPILER_PYTHON_VERSION` | `python.version` | unset (the compiler's 3.11) | `CLOACINA_COMPILER_PYTHON_VERSION`. |
| `CLOACINA_TENANT_COMPILER_PYTHON_INDEX_URL` | `python.indexUrl`, or `python.indexUrlSecretRef` | unset (PyPI) | The package index. Written into the Secret `cloacina-compiler-db`, not into the pod spec. |
| `CLOACINA_TENANT_COMPILER_PYTHON_WHEEL_DIR` | `python.wheelDir` | unset | `CLOACINA_COMPILER_PYTHON_WHEEL_DIR`: a folder of wheels inside the image. |
| `CLOACINA_TENANT_COMPILER_PYTHON_OFFLINE` | `python.offline` | `false` | `CLOACINA_COMPILER_PYTHON_OFFLINE`. In a hardened pod it needs a wheel folder. |
| `CLOACINA_TENANT_COMPILER_PYTHON_UV_CACHE_DIR` | `python.uvCacheDir` | unset (hardened: `/var/cloacina/cache/uv` on the claim, or `/var/cloacina/uv-cache` with no claim) | `CLOACINA_COMPILER_PYTHON_UV_CACHE_DIR`. |
| `CLOACINA_TENANT_COMPILER_PYTHON_CACHE_SIZE` | `python.cacheSize` | `4Gi` | Size limit of the `uv` cache `emptyDir` of a hardened pod with no claim. |

A value the server cannot use (a CIDR block that does not parse, an egress
mode it does not know, a Python platform the compiler does not know, a Python
version that is not `major.minor`, a folder that is not an absolute path,
offline in a hardened pod with no wheel folder, a claim size that is not a
storage quantity above zero, or a storage class name that is not a DNS
subdomain) stops the server at start with a message that names the variable.

The server also gives each tenant compiler its own unpack limit
(`CLOACINA_MAX_UNPACKED_BYTES`) when it is not the default, as it does for
the agents it starts.

**The pod.** With `hardened` the pod runs as uid and gid `10001`, not root,
with the `RuntimeDefault` seccomp profile, a read-only root filesystem, no
capabilities and no privilege escalation, and mounts no service account
token. The paths a build writes are volumes. The compiler's home
(`/var/cloacina/home`: unpacked sources, logs and the Python vendoring
folders; `homeSize`, default 4 GiB) and `/tmp` (1 GiB) are `emptyDir`s. With
the cache claim, the claim is mounted at `/var/cloacina/cache`, and the cargo
home (`cargo`), the target directory (`target`) and the `uv` cache (`uv`) are
folders in it that the compiler's user makes. The pod sets `fsGroup` `10001`,
so the claim is writable by that user on a storage class whose driver applies
`fsGroup` (most block storage does; some NFS drivers do not). With no claim they are `emptyDir`s:
the cargo home (`/var/cloacina/cargo`) and the target directory (each
`cacheSize`, default 8 GiB) and the `uv` cache (`/var/cloacina/uv-cache`,
`python.cacheSize`, default 4 GiB). The image must be usable by uid `10001`:
the toolchain readable, and nothing it needs written outside those paths. `hardened=false` runs the image as it is built, on
its own filesystem; use it only for a development image that carries its
caches. The compiler listens on `127.0.0.1:9000` only, and the pod has no
probe.

**The database URL.** The server writes the URL into a Secret
`cloacina-compiler-db` in the tenant's namespace, and the pod reads it as the
environment variable `DATABASE_URL`. It is not on the pod's command line. Its
user is the tenant's build role (below), with a password the server set for
this start; host, port, database and parameters come from
`CLOACINA_TENANT_COMPILER_DATABASE_URL` (chart:
`fleet.compiler.databaseUrlSecretRef`), or the server's own URL. The host must
resolve from the tenant namespaces: use a full Service name
(`<service>.<namespace>.svc.cluster.local`), not a short one. A changed host
reaches a compiler when it next starts. A Python package index URL, when one
is set, is a second key of the same Secret, `python-index-url`, read as
`CLOACINA_COMPILER_PYTHON_INDEX_URL`. The Secret is written only when the
compiler starts, so a changed index URL also reaches a compiler at its next
start.

### The build role of a tenant

Each tenant has a second Postgres role, for builds only: `<tenant>$build`,
lower case (`public$build` for `public`). A tenant name cannot hold `$`, so
no tenant role has that name; a name longer than 57 characters becomes its
first 40 characters, `$build$` and 16 hex digits of its SHA-256. The server
makes it, and it has these rights and no other:

| Table, in the tenant's own schema | Rights |
|---|---|
| `workflow_packages` | `SELECT`, `UPDATE` |
| `workflow_registry` | `SELECT` |
| `package_providers` | `SELECT`, `INSERT`, `DELETE` |
| `package_artifacts` | `SELECT`, `INSERT`, `DELETE` |
| `__diesel_schema_migrations` | `SELECT` |
| `cloacina_schema_release` | `SELECT` |

plus `USAGE` on the schema. The last two are for the compiler's schema check
at start. No right on another schema, on any table of
`public`, or to create a table. A table the tenant schema lacks is refused,
not read from `public`. `public$build` has these tables of `public` and
nothing else there.

- **When it is made.** At tenant create; for every tenant at each server
  start, after the tenant's migrations; and `public$build` at each server
  start. Each time under the migration lock,
  so a tenant made before this version gets its build role at the next start
  with no step.
- **Grants after a migration.** The grants are given again after each
  migration of the tenant schema, whatever role ran it. A table added later
  for builds reaches the role at the start that makes it.
- **The password.** Made with no password: no one can log in. The server sets
  a random password each time it starts the tenant's compiler (no Deployment,
  0 replicas, or no container), writes the URL only into the compiler's
  Secret (Kubernetes) or the container's environment (Docker), and keeps no
  copy. While the compiler runs, the Secret is not written again, so a pod
  that restarts reads a password that still works.
- **Rotation.** At each start of the compiler, so after each scale to zero
  (300 s of empty queue by default). A compiler that never runs out of work
  keeps its password until it stops. To rotate now, scale the tenant's
  `cloacina-compiler` Deployment to 0: the next tick with work starts it with
  a new password.
- **After an upgrade.** A tenant compiler made by an older server holds the
  server's own URL. On Kubernetes its pod template has no annotation
  `cloacina.io/build-credential`: at the first tick with work the server sets
  a build role password, writes the Secret and applies the Deployment with the
  annotation, which replaces the pod. A build in progress is cut; the sweeper
  puts it back in the queue. On Docker a compiler container without the label
  `cloacina.compiler-credential=build-role` is stopped, and a new one starts
  if there is work.
- **The tenant's own role.** Tenant create also makes the tenant role, with
  the password given back once. The server stores it nowhere and does not use
  it; whoever holds it can use it by hand.
- **What the server's database user needs.** `CREATEROLE`, as for tenant
  create, and to own the tenant tables (it runs the migrations). On Postgres
  16 a role with `CREATEROLE` can set the password only of roles it made.

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

## The describe step

After a build the compiler reads the package's metadata and stores it as one
JSON document in `workflow_packages.metadata_document`. The document holds:

- each task: id, dependencies, trigger rules, retry policy, time limit,
  placement (pool, CPU, memory, inline), docs;
- the workflow name, its graph and the triggers it subscribes to;
- each trigger: poll interval, or cron expression with timezone, catch-up
  policy and the limit on active runs;
- reactors with their accumulators, the reactor-bound graph with its
  accumulators, input strategy and topology, trigger-less graphs and
  constructor nodes (for a Python package, each `cloaca.constructor(...)` as
  written);
- each bundled provider with its runtime (`native` or `wasm`), read from its
  `provider.json`;
- what of the package would run tenant code in the process that loads it: a
  stream accumulator on a native provider, a Python stream accumulator, a
  polling accumulator in a Rust package. A server without single trust
  refuses such a package at load. A poll trigger and a Python polling
  accumulator are not in the list: their polls run on the tenant's agents;
- declared params, declared secrets and declared surfaces;
- for a Python package: the entry module, `requires_python` and the Python
  version that imported it;
- `format_version`, the version of the document format (2).

Reading the metadata runs the package's code: opening a Rust library runs its
load-time code, and importing a Python package runs the module. The compiler
process does neither. It starts a child:

```
cloacina-compiler describe --library <file> --out <file>
cloacina-compiler describe --python-archive <file> [--vendored-set <file>]
  [--providers-dir <dir>] --out <file>
```

- **The child is the compiler binary**, started again in describe mode: the
  same fidius version and the same build profile as the compiler.
- **A build child like `cargo`.** It gets the cleared environment (no
  `DATABASE_URL`), its own process group and the `--build-rlimit-*` limits.
  Its time limit is `--describe-timeout-s`, 120 seconds by default.
- **A Python package is imported** with the dependencies of the compiler's own
  platform: the `vendor/` folder of the package, or the set the compiler
  vendored for that platform. A compiler that vendors must have its own
  platform in `--python-platform`: a package with locked dependencies and no
  set for the compiler's platform fails to build. The Python that imports the
  package is the one the compiler binary links; it must be the Python of
  `--python-version`.
- **The compiler binary links the Python interpreter.** The host or the image
  needs the Python shared library (`libpython3.11` on Debian 12).
  `docker/Dockerfile.compiler` installs it.

A describe that fails, or passes its time limit, fails the build: the package
gets `build_status` `failed` and the reason in `build_error`. A describe that
passes its time limit is not retried.

### Packages built before the document existed

A package built by an earlier version has no document. The compiler describes
it from its stored build: the stored library, or for a Python package the
stored archive. No build runs, nothing is uploaded again and the package stays
in service. The compiler looks for such packages on each sweep tick
(`--sweep-interval-s`) and describes up to five per tick.

- A package it can describe gets its document.
- A package it cannot describe keeps `build_status` `success` and gets the
  reason in `describe_error`. `GET /v1/tenants/{tenant}/workflows/{name}` and
  the versions list return `describe_error`, and `cloacinactl package inspect`
  prints it. The compiler does not try that package again until the package is
  built again or the document format changes. An upload of the same archive
  starts no build: to get a document, publish a changed archive.

A package whose document is of an older format is described again the same
way: a compiler of format 2 gives a format 1 document the bundled providers'
runtimes and the list of what would run in process. A package the server has
refused (`build_status` `failed`) is not described again.

A per-target compiler (`--build-target`) does not describe.

With the server's tenant compilers, a tenant that has a package with no
document gets its compiler started, as for a pending build. The compiler's
`/v1/status` reports the count as `undescribed`.

## Python packages: vendoring dependencies

A Python package has nothing to compile. Its build step is its dependencies.
The compiler looks at the unpacked package:

| The package has | The compiler |
|---|---|
| a `vendor/` folder | uses the package as it is. It runs no `uv` and downloads nothing. |
| no `vendor/`, and no dependencies in `pyproject.toml` | does nothing. |
| no `vendor/`, dependencies in `pyproject.toml`, and a `requirements.lock` | vendors: it installs the wheels the lock names, for each platform of its settings, and stores one set per platform. |
| no `vendor/`, dependencies, and no `requirements.lock` | fails the build. It does not resolve versions: what runs is what the publisher locked with `cloacinactl package build`. |

A server or an agent that loads the package gets the set of its own platform
(`GET /v1/agent/python-vendor/{digest}` for an agent) and unpacks it into
`vendor/<platform>/`. The uploaded archive is not changed.

### How it vendors

For each platform the compiler runs:

```
uv pip install --requirement requirements.lock --target <folder>
  --python-platform <platform> --python-version <version>
  --only-binary :all: --require-hashes --no-deps --link-mode copy
  --no-config --no-python-downloads
```

- **Wheels only.** `--only-binary :all:` refuses a dependency that has no wheel
  for the platform. The build fails and names it. No source distribution is
  built, so no code of a dependency runs in the compiler: `uv` unpacks wheels.
- **Exactly what the lock names.** `--no-deps` installs the pins of the lock
  file and nothing else. `--require-hashes` refuses a wheel whose hash is not
  in the lock file.
- **The lock file is checked first.** It may hold pinned versions
  (`name==version`) with `--hash` lines, environment markers and comments. An
  option line (`-r`, `--index-url`, `--find-links`, …), a URL or a path fails
  the build before `uv` starts: such a line would make `uv` read a file or a
  server that the package's author chose.
- **`--no-config`.** `uv` runs in the package's folder. A `uv.toml` or a
  `[tool.uv]` table in the package does not change the index.
- **A build child like `cargo`.** `uv` gets the cleared environment, the
  `--build-timeout-s` limit and the `--build-rlimit-*` limits of every build
  child, plus the proxy and certificate variables (`HTTP_PROXY`, `HTTPS_PROXY`,
  `NO_PROXY`, `ALL_PROXY`, `SSL_CERT_FILE`, `SSL_CERT_DIR`). The limits apply
  to each `uv` run, one per platform.
- `uv` needs a Python interpreter on the machine. It does not download one.

The set of each platform is checked against the archive limits before it is
stored (`--max-unpacked-bytes`, 10,000 entries, 10 times the archive size): a
set that a loader would refuse fails the build, with its size in the error.

### Settings

| Flag | Env | Default | Meaning |
|---|---|---|---|
| `--python-platform <ARCH-OS>` (repeatable, or a comma-separated list) | `CLOACINA_COMPILER_PYTHON_PLATFORMS` | `aarch64-linux`, `x86_64-linux` | The platforms to vendor for: the platforms of the servers and agents that load Python packages. One of `aarch64-linux`, `x86_64-linux`, `aarch64-macos`, `x86_64-macos`, `x86_64-windows`. A package the compiler vendored loads on these platforms only. |
| `--python-version <MAJOR.MINOR>` | `CLOACINA_COMPILER_PYTHON_VERSION` | `3.11` | The Python the wheels are for: the Python of the servers and agents (3.11 in the official images). A package whose `requires_python` excludes it fails to build. |
| `--python-index-url <URL>` | `CLOACINA_COMPILER_PYTHON_INDEX_URL` | PyPI | The package index. The compiler gives it to `uv` in the environment (`UV_DEFAULT_INDEX`), not on the command line. For a URL with a password, set the variable and not the flag. |
| `--python-wheel-dir <DIR>` | `CLOACINA_COMPILER_PYTHON_WHEEL_DIR` | unset | A folder of wheel files (`uv --find-links`): one more place to find wheels, next to the index. The index is still read. With `--python-offline` it is the only source. |
| `--python-uv-cache-dir <DIR>` | `CLOACINA_COMPILER_PYTHON_UV_CACHE_DIR` | `uv`'s own, under the home folder | The cache folder of `uv`. The compiler's user must be able to write to it. |
| `--python-offline` | `CLOACINA_COMPILER_PYTHON_OFFLINE` | off | No network for Python builds (`uv --offline`). With a wheel folder, only the folder is read (`--no-index`). With no wheel folder, the `uv` cache must hold every wheel. The compiler does not start with `--python-offline` and neither a wheel folder nor a cache folder. |
| `--uv <PATH>` | `CLOACINA_UV` | `uv` on `PATH` | The `uv` program. |

A per-target compiler (`--build-target`) does not vendor. The primary compiler
vendors for every platform of its settings in one build.

The Helm chart sets these for its own compiler from `compiler.python.*`
(`platforms`, `version`, `indexUrl`, `wheelDir`, `offline`, `uvCacheDir`). The
chart's default `uvCacheDir` is `/tmp/uv-cache`, on the pod's `/tmp` volume:
the root filesystem of the pod is read-only.

### What the compiler downloads, and from where

With the default settings a Python build opens HTTPS connections to the package
index and to the host that serves its files. For PyPI these are `pypi.org` and
`files.pythonhosted.org`. It downloads the wheels that the lock file names, and
the index pages of those distributions. It sends credentials only when the
operator set them: in the index URL, or in the `.netrc` file of the compiler's
user.

If you restrict the compiler's egress (a firewall, or a NetworkPolicy), choose
one:

- allow TCP 443 to the index and its file host;
- point `--python-index-url` at a mirror inside your network;
- run with `--python-offline` and carry the wheels in the image (next section).

### A compiler image that carries the wheels

A compiler can vendor with no network when its image holds the wheels. Teams
then pin the versions the image holds.

1. Write the stack as a `pyproject.toml` and lock it, as a team does for a
   package:

   ```toml
   [project]
   name = "wheel-stack"
   version = "1.0.0"
   dependencies = ["pyarrow==26.0.1", "duckdb==2.0.1", "pyiceberg==0.12.0"]
   ```

   ```bash
   uv pip compile pyproject.toml --universal --python-version 3.11 \
     --generate-hashes --output-file stack.lock
   ```

2. Build an image on top of the compiler image. A first stage downloads the
   wheels of the lock for both Linux architectures; the second copies them in:

   ```dockerfile
   FROM python:3.11-slim AS wheels
   COPY stack.lock /stack.lock
   RUN for arch in x86_64 aarch64; do \
         pip download --no-deps --only-binary :all: --require-hashes \
           --python-version 3.11 --implementation cp \
           --abi cp311 --abi abi3 --abi none \
           --platform manylinux_2_28_$arch \
           --platform manylinux_2_17_$arch \
           --platform manylinux2014_$arch \
           -r /stack.lock -d /wheels || exit 1; \
       done

   FROM <your cloacina-compiler image>
   COPY --from=wheels /wheels /opt/cloacina/wheels
   ```

   The compiler image must hold `uv` and a Python interpreter.
   `docker/Dockerfile.compiler` has both.

3. Start the compiler with the folder and no network for Python builds:

   ```bash
   cloacina-compiler \
     --python-wheel-dir /opt/cloacina/wheels \
     --python-offline \
     --python-uv-cache-dir /tmp/uv-cache
   ```

   With the chart: `compiler.python.wheelDir=/opt/cloacina/wheels` and
   `compiler.python.offline=true`.

4. Give the teams `stack.lock`. A package whose lock file pins a version that
   is not in the folder fails to build, and the error names the distribution
   and the folder:

   ```
   `pyarrow` was not found for x86_64-linux with Python 3.11 in the wheel source
   of this build (the wheel folder /opt/cloacina/wheels, no network).
   ```

To carry a `uv` cache in place of a wheel folder, fill it in the image build
with the same install the compiler runs, then start the compiler with
`--python-uv-cache-dir <that folder> --python-offline`:

```dockerfile
RUN for platform in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do \
      uv pip install --requirement /stack.lock --target /tmp/fill-$platform \
        --python-platform $platform --python-version 3.11 \
        --only-binary :all: --require-hashes --no-deps \
        --cache-dir /opt/cloacina/uv-cache || exit 1; \
    done && rm -rf /tmp/fill-*
```

The compiler's user must be able to write to the cache folder.

A wheel folder without `--python-offline` adds to the index: `uv` still reads
the index for every distribution, and finds in the folder the wheels the index
does not have (a private wheel, for example).

### Python in the compilers the server makes

A [tenant compiler](#compilers-the-server-makes) vendors as the chart's own
compiler does. The server gives it the settings of
`CLOACINA_TENANT_COMPILER_PYTHON_*` (chart: `fleet.compiler.python.*`) as the
compiler's `CLOACINA_COMPILER_PYTHON_*` variables. One that is not set leaves
the compiler's default. All tenant compilers get the same settings.

**The image.** `fleet.compiler.image` (by default the image of
`compiler.image`) must hold `uv` and a Python interpreter. The image of
`docker/Dockerfile.compiler` copies `uv` to `/usr/local/bin/uv` and installs
`python3` (3.11 on Debian bookworm); a test of `cloacina-server` checks that
the Dockerfile does both. For an image of your own, check it before you use
it:

```bash
docker run --rm --entrypoint uv <image> --version
docker run --rm --entrypoint python3 <image> --version
```

A build in an image with no `uv` fails, and the error says that the compiler
image must hold `uv`.

**The egress.** A Python build downloads from the package index, through the
compiler's NetworkPolicy:

| `fleet.compiler.egress` | What a Python build needs |
|---|---|
| `internet` (the default) | Nothing more for PyPI: the rule allows TCP 443 to public IPv4 addresses, where `pypi.org` and `files.pythonhosted.org` are. |
| `all` | Nothing more. |
| `none` | `python.offline=true` and `python.wheelDir`: an image that carries the wheels (see [A compiler image that carries the wheels](#a-compiler-image-that-carries-the-wheels)). |

An index inside the cluster or the private network has a private address,
which the `internet` rule leaves out. Put its CIDR in
`CLOACINA_TENANT_COMPILER_EGRESS_CIDRS` (chart: `fleet.compiler.egressCidrs`),
and its URL in `python.indexUrl` or `python.indexUrlSecretRef`. The rule is
IPv4 only.

**The index URL** can hold a password. The server writes it into the Secret
`cloacina-compiler-db` of the tenant's namespace, next to the database URL,
and the pod reads it from there. It is in no pod spec and in no log.

**The space.** In a hardened pod, the `uv` cache is a folder of the cache
claim (`cacheClaim.size`, default 20 GiB, shared with the cargo caches), or,
with no claim, its own `emptyDir` at `/var/cloacina/uv-cache`
(`python.cacheSize`, default 4 GiB). The build folder is in the home
(`homeSize`, default 4 GiB). For each platform in turn,
a build holds the filled folder, its archive and an unpacked check of the
archive; then only the archive stays. A heavy set of 286 MiB unpacked (an
81 MiB archive) for the two default platforms needs about 0.75 GiB of the home
at its peak, next to the package's source and the logs; the `uv` cache holds
the wheels on its own volume. A pod that writes past a size limit is evicted:
its build stays `building` until a compiler's sweeper puts it back in the
queue, and runs out of space the same way the next time. Raise `homeSize` and
`python.cacheSize` for larger sets, or vendor for fewer platforms. A `uv` cache baked into the
image does not work in a hardened pod: its root filesystem is read-only. A
wheel folder in the image does, because `uv` only reads it.

**A cold start downloads every wheel.** With the cache claim, the `uv` cache
is kept when a tenant's compiler is scaled to 0, so only the first start of
the tenant's compiler, or a start on a new claim, is cold. The first Python
build then downloads every wheel its lock file names, for each platform, from
the index: the cost is the size of those wheels over the cluster's link to
the index. Later builds, after a restart too, read the cache for the wheels
it holds. With no claim (`cacheClaim.enabled=false`, or `hardened=false`) the
`uv` cache is not kept between starts, and each start from 0 replicas is
cold. A wheel
folder in the image, with `python.offline=true`, downloads nothing at any
start.

## Flag reference

All compiler flags accept env equivalents (`CLOACINA_COMPILER_*`).

| Flag | Env | Default |
|---|---|---|
| `--build-timeout-s` | `CLOACINA_COMPILER_BUILD_TIMEOUT_S` | 600 |
| `--describe-timeout-s` | `CLOACINA_COMPILER_DESCRIBE_TIMEOUT_S` | 120 |
| `--vendor-dir` | `CLOACINA_COMPILER_VENDOR_DIR` | unset (cargo `~/.cargo`) |
| `--cargo-flag` (repeatable) | — | appends to `build --release --lib --frozen --offline` |
| `--cargo-flags-replace` (repeatable) | — | unset (full override of the default flag list) |
| `--build-rlimit-cpu` | `CLOACINA_COMPILER_BUILD_RLIMIT_CPU` | = `--build-timeout-s` |
| `--build-rlimit-mem` | `CLOACINA_COMPILER_BUILD_RLIMIT_MEM` | `4G` |
| `--build-rlimit-files` | `CLOACINA_COMPILER_BUILD_RLIMIT_FILES` | 1024 |
| `--build-rlimit-procs` | `CLOACINA_COMPILER_BUILD_RLIMIT_PROCS` | 256 |
| `--max-unpacked-bytes` | `CLOACINA_MAX_UNPACKED_BYTES` | 524288000 (500 MiB) |
| `--python-platform` (repeatable) | `CLOACINA_COMPILER_PYTHON_PLATFORMS` | `aarch64-linux`, `x86_64-linux` |
| `--python-version` | `CLOACINA_COMPILER_PYTHON_VERSION` | `3.11` |
| `--python-index-url` | `CLOACINA_COMPILER_PYTHON_INDEX_URL` | unset (PyPI) |
| `--python-wheel-dir` | `CLOACINA_COMPILER_PYTHON_WHEEL_DIR` | unset |
| `--python-uv-cache-dir` | `CLOACINA_COMPILER_PYTHON_UV_CACHE_DIR` | unset (`uv`'s own) |
| `--python-offline` | `CLOACINA_COMPILER_PYTHON_OFFLINE` | off |
| `--uv` | `CLOACINA_UV` | `uv` |
| `--cargo-target-dir` | — | unset (per-build `target/`) |
| `--tenant-schema` | `CLOACINA_TENANT_SCHEMA` | unset (public schema) |
| `--build-target` | `CLOACINA_BUILD_TARGET` | unset (native host build) |
| `--build-target-package` | `CLOACINA_BUILD_TARGET_PACKAGE` | unset (all packages) |
| `--home` | `CLOACINA_HOME` | `$HOME/.cloacina` |
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
