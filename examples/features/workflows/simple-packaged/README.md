# Simple Packaged Workflow

**The canonical Cloacina example.** A packaged workflow is the unit of deployment
in Cloacina: you author tasks in Rust, pack the source into a `.cloacina`
archive, and hand it to a running server — which compiles it, registers it, and
executes it. Everything happens through the primary interface:

```
pack  →  upload  →  compile  →  reconcile  →  execute  →  observe
```

This example implements a three-task data processing pipeline:

```
collect_data → process_data → generate_report
```

## Layout

| File | Role |
|---|---|
| `package.toml` | Package manifest — name, version, and the workflow it exposes (`data_processing`) |
| `Cargo.toml` | Ordinary crate manifest; cloacina deps are crates.io version deps |
| `src/lib.rs` | The workflow: `#[workflow]` module with three `#[task]` functions |
| `build.rs` | `cloacina-build` boilerplate (emits the plugin interface) |

This is exactly the shape `cloacinactl package new <name>` scaffolds — start
new packages from there rather than copying this directory.

## Run it

Everything below is also automated as `angreal demos features simple-packaged`
(the CI examples lane runs exactly that) — the steps here are the same flow,
by hand, against the demo stack.

### 1. Bring up the stack

```bash
angreal ui up
```

Server + web UI come up at <http://localhost:8080> with a seeded `public`
tenant. (Any running Cloacina server works; the demo stack is just the
batteries-included one.)

### 2. Point the CLI at it

```bash
cloacinactl config profile set demo http://localhost:8080 \
    --api-key clk_demo_public_key_0003 --tenant public --default
```

### 3. Pack the source

```bash
cloacinactl package pack . --out simple-packaged-demo.cloacina
```

The archive contains **source**, not binaries — the server's compiler builds it
for the fleet's architectures.

### 4. Upload

```bash
cloacinactl package upload simple-packaged-demo.cloacina
```

The server registers the package and queues a build. Watch it go
`pending → building → success`:

```bash
cloacinactl package list
```

Once the build succeeds, the reconciler loads the workflow and
`data_processing` becomes executable.

### 5. Execute

```bash
cloacinactl workflow run data_processing
```

### 6. Observe

```bash
cloacinactl execution list --workflow data_processing
cloacinactl execution status <execution-id>
```

Or watch the run in the web UI at <http://localhost:8080> — executions, task
states, and the workflow DAG are all there.

## How the workflow is authored

`src/lib.rs` declares the pipeline with the workflow macro; dependencies
between tasks are declared per-task and the engine derives the execution
order:

```rust
#[workflow(name = "data_processing", description = "...", author = "...")]
pub mod data_processing {
    #[task(retry_attempts = 2)]
    pub async fn collect_data(ctx: &mut Context<Value>) -> Result<(), TaskError> { ... }

    #[task(dependencies = ["collect_data"], retry_attempts = 3)]
    pub async fn process_data(ctx: &mut Context<Value>) -> Result<(), TaskError> { ... }

    #[task(dependencies = ["process_data"])]
    pub async fn generate_report(ctx: &mut Context<Value>) -> Result<(), TaskError> { ... }
}
```

The task id is the function name. Data flows between tasks through the
execution `Context` (`ctx.insert(...)` / `ctx.get(...)`); retry policy is
declared on the `#[task]` attributes.

## A note on dependencies

`Cargo.toml` declares the cloacina crates as **crates.io version deps**
(`cloacina-workflow = "0.10"`) — the form real distributed packages ship.
Development stacks (the demo compose stack, the `angreal test e2e compiler
--version-deps` harness) resolve these against the local workspace via the
compiler's `--dev-workspace` flag; production compilers resolve them from
crates.io. Nothing about the package changes between the two.
