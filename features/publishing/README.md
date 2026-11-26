# Publishing behaviour spec (CLOACINA-T-1026)

`independent_publish.feature` is one behaviour spec: a team authors, compiles
and links its jobs to a shared orchestrator on its own, safely, with no central
repository of jobs. The eight scenarios are the promise broken into parts; the
spec's job is to find the rough spots, not to pass on day one.

## How to run it

The driver is one angreal task, in `.angreal/test/bdd/`.

```
angreal test bdd publish --dry-run      # parse + check step coverage; no cluster
angreal test bdd publish                # full run against k3s (maintainer only)
angreal test bdd publish --skip-build   # reuse built server/agent/compiler images
angreal test bdd publish --no-cleanup   # leave the cluster up afterwards
angreal test bdd publish --only 2,6     # run only these scenario numbers
```

The dry run starts no containers and needs no Docker. The full run stands up a
k3s cluster and a local registry in Docker (the substrate of
`.angreal/test/e2e/k8s_fleet.py`), builds and pushes the server, agent and
compiler images, installs the server with the Helm chart and the fleet
executor, and brings up two tenants (`team_a`, `team_b`), each with one agent, a
write key, a secret, and a compiler scoped to its schema.

## What it needs that the plain k3s harness does not

- the server runs with `CLOACINA_DEFAULT_EXECUTOR=fleet`, or no task reaches an
  agent;
- `CLOACINA_SECRET_KEK` is set, or the secrets routes answer 503;
- a compiler per tenant (`--tenant-schema`), because the chart's compiler serves
  the public schema only; the driver applies one Deployment per tenant;
- the compiler image (`docker/Dockerfile.compiler`) with `--dev-workspace`,
  because the cloacina crates are not on crates.io; the Rust fixtures carry
  version deps and the driver injects the same `[patch.crates-io]` for the
  host-side build in scenario 1.

## The runner

The angreal tasks run in angreal's own Python and have no dependency file, so
`behave` and `pytest-bdd` cannot be added cleanly. `.angreal/test/bdd/gherkin.py`
is a small parser and runner for the subset of Gherkin the feature uses.

## Known gaps

A scenario tagged `@gap` does not hold today. It still runs; its failure is
reported as a known gap and does not fail the run, and the tag after `@gap`
names the Kairos task that owns it. If such a scenario passes, the report says
so loudly — the gap may be closed, so check and remove the tags.

- scenarios 3, 4, 5 (`@CLOACINA-T-1018`): versions listing, the version pin and
  going back to an earlier version. The steps are written against that task's
  API (`…/workflows/{name}/versions`, `…/versions/{version}/activate`,
  `cloacinactl package versions|activate`), which is in a PR, not on main.
- scenario 7 (`@CLOACINA-T-1025`): signing. No route stores a signature or a
  per-team trusted key today, and `package pack --sign` fails hard, so the
  scenario's own `Given` reports the gap.
- scenario 8 (`@CLOACINA-T-1022`): a task of one team reading another team's
  packages or secrets. The server and the compiler load every team's code; this
  is the weak one. The probe fixture reports what it could reach.

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
