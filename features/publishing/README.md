# Publishing behaviour spec (CLOACINA-T-1026)

`independent_publish.feature` is one behaviour spec: a team authors, compiles
and links its jobs to a shared orchestrator on its own, safely, with no central
repository of jobs. The eight scenarios are the promise broken into parts; the
spec's job is to find the rough spots, not to pass on day one.

## How to run it

The driver is one angreal task, in `.angreal/test/bdd/`.

```
angreal test bdd publish --dry-run        # parse + check step coverage; no cluster
angreal test bdd publish                  # full run against k3s, then tear down
angreal test bdd publish --skip-build     # reuse built server/agent/compiler images
angreal test bdd publish --no-cleanup     # leave the cluster up afterwards
angreal test bdd publish --only 2,6       # run only these scenario numbers
angreal test bdd publish --reuse-cluster  # one named cluster, made once and kept
angreal test bdd publish --chart-actuator # let the server make the agents (see below)
```

The dry run starts no containers and needs no Docker. The full run needs
`docker`, `kubectl` and `helm` on the host. It stands up a k3s cluster and a
local registry in Docker (the substrate of `.angreal/test/e2e/k8s_fleet.py`),
builds and pushes the server, agent and compiler images, installs the server
with the Helm chart and the fleet executor, and brings up two tenants
(`team_a`, `team_b`), each with one agent, a write key, an admin key, a secret,
and a compiler scoped to its schema.

`cloacinactl` is built on the host in the debug profile. The driver follows
`CARGO_TARGET_DIR`, so a worktree can share a target folder.

To work on one scenario, use `--reuse-cluster --skip-build --only N`: the first
run makes the cluster `cloacina-k8s-bdd-publish-keep`, later runs attach to it
in seconds. The `Given` steps publish a version only when the tenant lacks it,
so a scenario can run again on the same cluster. Scenario 2 is the exception:
it publishes 0.1.0 of the report job for the first time, so run it on a fresh
cluster. Each run prints the `docker compose … down -v` line that removes the
cluster.

## What it needs that the plain k3s harness does not

- the server runs with `CLOACINA_DEFAULT_EXECUTOR=fleet`, or no task reaches an
  agent;
- `CLOACINA_SECRET_KEK` is set, or the secrets routes answer 503;
- a compiler per tenant (`--tenant-schema`), because the chart's compiler serves
  the public schema only; the driver applies one Deployment per tenant;
- the compiler image (`docker/Dockerfile.compiler`) with `--dev-workspace`,
  because the cloacina crates are not on crates.io; the Rust fixtures carry
  version deps and the driver injects the same `[patch.crates-io]` for the
  host-side build in scenario 1. The compiler builds in the release profile,
  like the agent that loads the result;
- the agents. The server does not start with `fleet.actuator=kubernetes` today:
  it panics while it builds its Kubernetes client, because no rustls crypto
  provider is installed. So the driver installs the server with
  `fleet.actuator=none` and applies, for each tenant, what the actuator would
  make: the namespace, the agent key Secret (a tenant key with the role
  `read`), the hardened agent Deployment and the NetworkPolicy. The manifests
  are a copy of `crates/cloacina-server/src/actuator/kubernetes.rs`; keep them
  in step. `--chart-actuator` goes back to the server's own actuator, for when
  the server starts with it.

## The runner

The angreal tasks run in angreal's own Python and have no dependency file, so
`behave` and `pytest-bdd` cannot be added cleanly. `.angreal/test/bdd/gherkin.py`
is a small parser and runner for the subset of Gherkin the feature uses.

## Known gaps

A scenario tagged `@gap` does not hold today. It still runs; its failure is
reported as a known gap and does not fail the run. The tag after `@gap` names
the Kairos task that owns it, or is `@unfiled` when no task owns it yet. If
such a scenario passes, the report says so loudly: the gap may be closed, so
check and remove the tags.

State after the first run on k3s (2026-11-26):

- scenarios 1, 3, 4, 5, 6 pass. Scenarios 3, 4 and 5 were gaps until
  CLOACINA-T-1018 merged. Scenario 4 uses two versions with the same task
  graph: it shows that the running execution's second task ran the code of
  the version it started with. It does not try a version that changes the
  graph.
- scenario 2 (`@unfiled`): the package compiles and loads with no restart, but
  its schedule does not start until some other call makes the server start the
  tenant's runner.
- scenario 7 (`@CLOACINA-T-1025`): signing. No route stores a signature or a
  per-team trusted key today, and `package pack --sign` fails hard, so the
  scenario's own `Given` reports the gap.
- scenario 8 (`@CLOACINA-T-1022`): every wall the probe task tried held, but
  the server process imports a team's Python package, so team code runs where
  the orchestrator keeps its credentials. The probe fixture reports what it
  could reach; it writes names, counts and status codes only.

## Reading the result

At the end the driver prints the rough-spots list and writes it to
`bdd-publish-report.txt` at the repo root. For each scenario: pass / known gap /
new failure / unexpected pass, with the observed facts — the HTTP status a
foreign key got, what the probe task could read, which version ran a task. A
`new failure` (an untagged scenario that failed) fails the run; a known gap does
not.

## Moving into the BDD crate

When `crates/cloacina-bdd` lands on main, the feature moves to
`crates/cloacina-bdd/features/publishing/independent_publish.feature` as a plain
file move. The driver stays an angreal task over the k3s deployment; it is the
"later driver against a running server and agent" that CLOACINA-A-0015 foresaw.
