<!--
Copyright 2026 Colliery Software
SPDX-License-Identifier: Apache-2.0
-->

# Cloacina seed / demo harness

A small workload driver (CLOACI-I-0117 / T-0660) that exercises a live
`cloacina-server` through the same SDK the UI ships on (`@cloacina/client`).
It ensures a tenant, uploads the demo `.cloacina` packages, and runs
executions in one of two modes:

| Mode   | What it does                                                                 | Used by |
| ------ | ---------------------------------------------------------------------------- | ------- |
| `seed` | Produces a deterministic state — one **completed**, one **failed**, one **in-flight** run — then exits. | automated UAT (T-0661) |
| `loop` | Fires runs forever on an interval, mixing fast / slow / failing executions, so the UI always has live activity. | the demo |

## Fixtures

The harness drives two fixtures (compiled from `examples/fixtures/`):

- **`demo-slow-rust`** → workflow `demo_slow_workflow`: a five-step chain where
  each step sleeps a per-task jittered duration (`transform` is the deliberate
  hot-spot), so runs emit a visible event sequence and a realistic spread for
  the runtime / Gantt / distribution views — the live-stream centerpiece. A
  `context.step_seconds` override pins every step to a flat duration for CI.
- **`demo-fail-rust`** → workflow `demo_fail_workflow`: does a little work then
  deterministically errors — the failed-state / debug view.

> **Naming note:** executions are keyed by **workflow name**
> (`demo_slow_workflow`), not package name (`demo-slow-rust`). The server's
> execute route resolves against the scheduler registry (workflow name) while
> list/detail use the package name — a known platform naming-drift gap. The
> harness executes by the workflow names above.

## Run it

The harness runs as **compose services** in the demo stack — there is no
host-process demo. The `fixtures` service packs the demo packages (per
`docker/pack-demo-fixtures.sh`), the `harness` service seeds a deterministic
completed/failed/in-flight state, a `driver` service drives continuous runs,
and a `producer` service streams data into the computation graphs.

### Via the demo stack (recommended)

```sh
angreal ui up        # docker compose -f docker/docker-compose.demo.yml up -d --build
# → UI at http://localhost:8082, fleet + harness + driver + producer running
```

> **Packages build asynchronously.** `cloacina-server` does not compile
> uploaded packages — a separate `cloacina-compiler` polls the DB and builds
> them (`pending → success`) before the workflow registers. The demo stack
> runs one; the harness retries `execute` until the workflow is registered, so
> the first runs may take a few seconds (and the first `up` a few minutes)
> while packages build.

### Directly (node) — for harness development

Point it at any reachable server with a directory of `.cloacina` packages:

```sh
HARNESS_SERVER_URL=http://localhost:8080 \
HARNESS_API_KEY=clk_demo_bootstrap_key_0001 \
HARNESS_TENANT=public \
HARNESS_PACKAGE_DIR=/path/to/packages \
HARNESS_MODE=seed \
node src/main.mjs
```

## Configuration (env)

| Var                       | Default                  | Meaning                                            |
| ------------------------- | ------------------------ | -------------------------------------------------- |
| `HARNESS_SERVER_URL`      | `http://localhost:8080`  | Target server.                                     |
| `HARNESS_API_KEY`         | _(required)_             | API key (`Authorization: Bearer`).                 |
| `HARNESS_TENANT`          | `public`                 | Tenant; non-`public` is created if missing.        |
| `HARNESS_PACKAGE_DIR`     | `./packages`             | Directory of `.cloacina` files to upload.          |
| `HARNESS_MODE`            | `seed`                   | `seed` or `loop`.                                  |
| `HARNESS_INTERVAL_MS`     | `8000`                   | Loop mode: ms between runs.                         |
| `HARNESS_SLOW_WORKFLOW`   | `demo_slow_workflow`     | Workflow name for the slow/streaming run.          |
| `HARNESS_FAIL_WORKFLOW`   | `demo_fail_workflow`     | Workflow name for the failing run.                 |
| `HARNESS_HEALTH_TIMEOUT_MS` | `60000`                | How long to wait for `/health` on startup.         |
| `HARNESS_RUN_TIMEOUT_MS`  | `60000`                  | Seed mode: how long to wait for a run to finish.   |
