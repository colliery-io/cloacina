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
  each step sleeps `context.step_seconds` (default 4s), so one run emits a
  visible event sequence over ~20s — the live-stream centerpiece.
- **`demo-fail-rust`** → workflow `demo_fail_workflow`: does a little work then
  deterministically errors — the failed-state / debug view.

> **Naming note:** executions are keyed by **workflow name**
> (`demo_slow_workflow`), not package name (`demo-slow-rust`). The server's
> execute route resolves against the scheduler registry (workflow name) while
> list/detail use the package name — a known platform naming-drift gap. The
> harness executes by the workflow names above.

## Run it

### Via angreal (local dev — recommended)

```sh
# 1. compile the fixtures once
angreal ui build-fixtures            # → examples/fixtures/dist/*.cloacina

# 2. in one terminal, bring up the stack
angreal ui up

# 3. in another, seed it (deterministic) or drive it continuously
angreal ui seed                      # seed mode (completed / failed / in-flight)
angreal ui seed --loop               # continuous demo activity
```

`angreal ui seed` flags: `--server <url>`, `--key <api-key>`, `--tenant <t>`,
`--loop`. It auto-builds fixtures + installs harness deps if needed.

> **Packages build asynchronously.** `cloacina-server` does not compile
> uploaded packages — a separate `cloacina-compiler` polls the DB and builds
> them (`pending → success`) before the workflow registers. `angreal ui up`
> now starts a compiler alongside the server, and the demo compose includes
> one; the harness retries `execute` until the workflow is registered, so the
> first runs may take a few seconds while the package builds.

### Directly (node)

```sh
HARNESS_SERVER_URL=http://localhost:8080 \
HARNESS_API_KEY=clk_dev_ui_bootstrap_key_0001 \
HARNESS_TENANT=public \
HARNESS_PACKAGE_DIR=../../examples/fixtures/dist \
HARNESS_MODE=seed \
node src/main.mjs
```

### Via the demo compose profile

```sh
angreal ui build-fixtures
docker compose -f docker/docker-compose.demo.yml up --build
# → UI at http://localhost:8081, harness driving continuous activity
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
| `HARNESS_STEP_SECONDS`    | `4`                      | Per-step pause of the slow workflow (~×5 total).   |
| `HARNESS_SLOW_WORKFLOW`   | `demo_slow_workflow`     | Workflow name for the slow/streaming run.          |
| `HARNESS_FAIL_WORKFLOW`   | `demo_fail_workflow`     | Workflow name for the failing run.                 |
| `HARNESS_HEALTH_TIMEOUT_MS` | `60000`                | How long to wait for `/health` on startup.         |
| `HARNESS_RUN_TIMEOUT_MS`  | `60000`                  | Seed mode: how long to wait for a run to finish.   |
