---
title: "The Behaviour Suite"
description: "A standing deployment that runs real work without end, a checker that judges the system's promises against it, and how to add a chapter."
sidebar:
  order: 62
---

The behaviour suite is a Cloacina deployment that stands, runs work without
end, and has its promises checked against what it really did. It exists
because unit and integration tests pass on behaviour that nobody has run end
to end: a task longer than a minute, a publish while a run is active, a full
tenant.

It has three parts:

| Part | What it is | Where |
|---|---|---|
| The promises | Sentences a user relies on, grouped in rows. Each row is a chapter. Each chapter is a Gherkin feature. | `features/<chapter>/` |
| The standing workload | Three tenants, each with its own agents, running jobs on schedules and a workflow that drives the server itself. | `examples/fixtures/standing-*`, `.angreal/test/bdd/` |
| The checker | A program outside the deployment. It reads the server's API and database, judges each promise on every pass, and writes the report. | `.angreal/test/bdd/suite/` |

## The promises

| Row | Promise |
|---|---|
| Publish | A team builds and publishes its own package with no restart. A bad build leaves the running version in service. A team can go back a version. |
| Run once | A task runs one time: a long task, a task with no free agent, a task whose result lands on another replica. |
| Schedules | A schedule fires on time while other work is slow. A run knows its interval. A backlog drains in order at its limit. |
| Failure | An agent dies mid-task and the task finishes on another agent. The server restarts and runs survive. Postgres goes away for minutes and nothing is lost or doubled beyond the stated rule. |
| Tenants | A key of one team does nothing in another. A task of one team cannot read another team's packages or secrets. |
| Capacity | Work waits when a tenant is full and runs when capacity arrives. A task lands on an agent that fits it. |
| Operations | A failed run raises an alert. Who did what is on record. The backlog and the lag are visible. Old history is removed when retention is set. |
| Upgrade | The server is upgraded with runs in flight and they finish. |

The table is `.angreal/test/bdd/suite/promises.py`. Each promise has an id
(`publish.bad_build`), and the report lists every promise of the table,
also the ones that no rule judges yet.

A promise that fails is not removed. It stays in the suite as a **named
gap**: a scenario tagged `@gap` with the code of the task that owns it, or a
checker rule with `gap="…"`. The suite is the list of what holds and what
does not.

## The standing workload

Three tenants, `team_a`, `team_b` and `team_c`, each with two agents, a
compiler, its own keys and one secret. Each tenant runs the same packages:

| Package | Language | Schedule | What it does |
|---|---|---|---|
| `standing-rollup-rust` | Rust | every five minutes | A short data job. It writes the interval it was given back into its context. |
| `standing-extract-py` | Python | every five minutes, and a second schedule that runs every missed fire, one at a time | Three short tasks in a row. It writes its interval back. |
| `standing-archive-py` | Python | hourly | One task that holds for six minutes. |
| `standing-flaky-py` | Python | every five minutes | Its first task fails once for each run and passes on a retry. At minutes 0 and 30 the run fails for good. |
| `standing-hold-py` | Python | none | Holds an agent for two minutes. Started through the API. |
| `standing-driver-py` | Python | every five minutes | The self-driving workflow. |

The self-driving workflow acts on the server it runs on, with a write key
of its own tenant that it reads from a tenant secret. In an hourly cycle it
publishes new versions of two packages (`standing-target-py` and
`standing-target-rust`, whose source it carries), publishes one Rust version
with a build error, goes back one version, and starts twelve holding jobs
at once, which is more than the tenant's agents take. On every run it starts
one run by hand and tries its key on another tenant. It writes each action,
with its status code, into its context under `standing_driver_actions`. The
checker reads that list.

Every task of the workload prints one line that starts with `STANDING_MARK`
when its body starts: a count of real starts that does not depend on the
server's own records.

The checker computes each schedule's fire times from its cron expression in
the workload description (`.angreal/test/bdd/suite/cron.py`), not from the
server's next fire time, and compares them with the fires the server
recorded (the chapter `schedules`).

The workload is described once, in `.angreal/test/bdd/suite/workload.py`:
the tenants, the packages, the schedules, and which workflow gives evidence
for which promise. The driver deploys from it and the checker judges from
it.

## Check everything without a cluster

```bash
angreal test bdd standing --dry-run
angreal test bdd standing --self-test
```

`--dry-run` finds every chapter, parses every feature, resolves every step
to exactly one definition, loads the checker's rules, and checks the
workload description. It prints each promise with the number of rules that
judge it and the workflows that give it evidence. It starts no container.

`--self-test` runs the unit tests of the checker and of the workload's
fixtures. They need no server. The same tests run with the system Python:

```bash
python3 -I .angreal/test/bdd/suite/tests/run.py
```

## Bring the deployment up

The full run needs `docker`, `kubectl` and `helm` on the host. It builds the
server, agent and compiler images, stands up a k3s cluster and a local
registry in Docker, and leaves the cluster standing.

```bash
angreal test bdd standing                  # bring up, deploy the workload, run its feature
angreal test bdd standing --skip-build     # reuse the images of an earlier run
angreal test bdd standing --chart-actuator # the server makes the agents and the compilers
angreal test bdd standing --chapter all    # run every chapter's features on the deployment
angreal test bdd standing --chapter publishing --only 3,5
angreal test bdd standing --down           # remove the cluster and everything in it
```

The first call makes the cluster `cloacina-k8s-bdd-standing`; later calls
attach to it. It installs the server with the Helm chart, with the fleet
executor, a database that keeps its data across a restart of its pod, and a
run history retention of two days. Then it creates the three tenants, their
agents, keys and compilers, publishes the workload's packages with each
tenant's own key, stores each tenant's secret, creates the scheduled
workflow instances, and runs `features/standing/standing_workload.feature`.
Every step makes what is missing and leaves the rest, so the task can be run
again on a deployment that is up.

Each chapter's feature run writes `bdd-<chapter>-report.txt` at the root of
the repository: each scenario as pass, known gap, new failure or unexpected
pass, with the facts it observed.

## Run the checker

```bash
angreal test bdd standing --check
```

This forwards the server and the database of the standing deployment to
localhost, and runs the checker until it is stopped. The report is at
`http://127.0.0.1:8377/`, and as JSON at `http://127.0.0.1:8377/report.json`.
`--interval` sets the seconds between passes (default 60) and `--port` the
port of the report.

The checker is its own program and runs against any deployment:

```bash
python3 -I .angreal/test/bdd/suite/check.py --config suite.json
```

```json
{
  "server": "http://192.0.2.10:8080",
  "database_url": "postgres://checker:PASSWORD@192.0.2.11:5432/cloacina",
  "keys": {"team_a": "KEY", "team_b": "KEY", "team_c": "KEY"}
}
```

| Option | Meaning |
|---|---|
| `--config FILE` | A JSON file with `server`, `database_url` and `keys`. It holds keys: give it owner-only access. |
| `--server URL`, `--keys FILE` | The same values one by one. A flag wins over the config file. |
| `--database-url URL` | The database. The environment variable `CLOACINA_SUITE_DATABASE_URL` does the same and does not show in the process list. |
| `--psql PATH` | The `psql` program. The checker reads the database with `psql` and looks for `psql`, `psql17`, `psql16` and `psql15` on the path. |
| `--out DIR` | Where the report, the state and the snapshots go. Default `bdd-standing-report`. |
| `--interval N` | Seconds between passes. Default 60. |
| `--port N` | Serve the report on `127.0.0.1:N`. Default 8377; 0 does not serve. |
| `--once` | Make one pass and stop. The exit code is 1 when a promise with no owner task does not hold. |
| `--record` | Keep each snapshot as a file in `<out>/snapshots`. |
| `--replay FILE…` | Judge recorded snapshots again, with no server. |
| `--list` | Print the rules, and the promises with no rule yet. |
| `--disturbances FILE` | The disturbance record. Default `<out>/disturbances.jsonl`. |

The checker needs one key for each tenant with the `read` role, and a
database user that can read each tenant's schema. It uses the Python
standard library only. Its keys and the database password are never written
to the report or to a snapshot.

A source that is away does not stop it. A pass with no server, or no
database, records "cannot judge" for the rules that needed it, and the next
pass tries again.

## Read the report

The report has one block for each promise, in the rows of the table above.
A first row, **Workload**, says whether the standing workload itself is
alive: the agents, the schedules, the self-driving workflow. When that row
does not hold, the other rows have little work to judge.

Each promise shows one state:

| State | Meaning |
|---|---|
| holds | Every rule of the promise found evidence, and it agrees with the promise. The block says for how long, and since when. |
| does not hold | A rule found evidence against the promise, and no task owns it. This is a new failure. The report is red. |
| known gap | A rule found evidence against the promise, and the rule names the task that owns it. |
| known gap, holds now | A rule that is marked as a gap holds. The gap may be closed: check, then remove the mark. |
| disturbed | A rule does not hold inside a disturbance window that it tolerates. The streak is kept. |
| cannot judge | There is no evidence yet, or a source was away. This is not "holds". |
| no rule yet | No rule judges this promise. The row names the task of its chapter. |

Under each promise are its rules. Each rule shows what it checks, the
evidence of the last pass (what it read and how many), since when it is in
its state, how long it has held, its longest streak, the number of breaks,
the time of the last break and of the last recovery, and its last changes
of state.

**Held for** counts time in which a pass judged the rule as holding. Time
after a pass that could not judge, time in a tolerated disturbance window,
and time in which the checker did not run are not counted. The state is
kept in `<out>/state.json`, so a restart of the checker does not reset
"held since".

The page shows the time of the last pass and marks itself stale when no
pass has happened for three intervals, and for three minutes at least.

### Disturbance windows

A disturbance is something broken on purpose: an agent pod killed, a server
replica restarted, Postgres taken away, a tenant filled. The program that
does it writes one JSON line for each disturbance to the disturbance record:

```json
{"id": "d-0001", "kind": "kill_agent", "target": "team_a", "start": "2026-11-27T12:00:00Z", "end": "2026-11-27T12:00:05Z", "settle_until": "2026-11-27T12:05:00Z"}
```

`start` is required. A line with no `end` is a window that is still open.
`settle_until` is the time by which the system must be well again; the
window excuses nothing after it.

A rule names the kinds it tolerates. Inside a window of such a kind, "does
not hold" shows as "disturbed" and the streak is kept. Inside any other
window it is a break, and the report shows the window next to it. After the
window, "does not hold" is a break. A rule can also read the windows itself,
to judge what happened around one: every run that was active when the agent
died ended `Completed`.

The report lists the disturbances of the last 24 hours.

## Add a chapter

A chapter owns three files. It edits no file of another chapter, and there
is no list of chapters to add a line to: the driver and the checker find the
files by their names.

1. **The feature**, `features/<chapter>/<name>.feature`. One scenario for
   each promise of the row, in the words a team would use. The runner reads
   `Feature`, `Background`, `Scenario`, tags, comments and the step keywords
   `Given`, `When`, `Then`, `And`, `But`.

2. **The steps**, `.angreal/test/bdd/<chapter>_steps.py`:

   ```python
   from .gherkin import StepRegistry

   steps = StepRegistry()
   CHAPTER = "schedules"
   FEATURES = ("features/schedules/schedules.feature",)


   @steps.then("each fire time of the last hour has one run")
   def one_run_for_each_fire(world):
       ...
   ```

   A step gets the `StandingWorld` of the deployment: the API with a chosen
   key, `cloacinactl`, `kubectl`, the tenants and their keys. A pattern is a
   regular expression that must match the whole step. A failed `Given` or
   `When` stops the scenario; a failed `Then` is recorded and the scenario
   goes on.

3. **The rules**, `.angreal/test/bdd/suite/rules/<chapter>.py`. The file
   names the sources it needs and registers its rules:

   ```python
   from ..engine import broken, holds, unknown

   CHAPTER = "schedules"

   RUNS_SQL = (
       "SELECT workflow_name, started_at FROM {schema}.workflow_executions "
       "WHERE started_at > now() - interval '1 hour'"
   )


   def register(registry):
       registry.need_db("recent_runs", RUNS_SQL)
       registry.need_api("triggers", "/v1/tenants/{tenant}/triggers")

       @registry.rule("schedules.one_run_per_fire", "schedules.on_time", CHAPTER,
                      tolerates=("postgres_away",),
                      about="each fire time of the last hour has one run")
       def one_run_per_fire(snapshot):
           for tenant in snapshot.tenants:
               runs = snapshot.db(tenant, "recent_runs")
               ...
           return holds("12 fire times, 12 runs, in 3 tenants")
   ```

   A source is collected once for each tenant on every pass: `{tenant}` in
   an API path and `{schema}` in SQL are replaced. `need_admin_db` is a
   query that runs once for the deployment. A rule returns `holds(...)`,
   `broken(...)` or `unknown(...)` with one line of evidence that says what
   it read. A rule with no evidence returns `unknown`. A source that is away
   makes the rule "cannot judge" with no code in the rule.

   The second argument of `registry.rule` is the id of the promise in
   `promises.py`. A rule for a promise that does not hold today gets
   `gap="CLOACINA-T-…"`, the task that owns it, or `gap="unfiled"`.

Then:

- Run `angreal test bdd standing --dry-run`. It fails on a step with no
  definition or with two, a feature that no steps file names, a rule for an
  unknown promise, and a rules file with no steps file.
- Add unit tests for the rules in `.angreal/test/bdd/suite/tests/`, on
  snapshots. `test_rules.py` shows the form: one recorded snapshot, and one
  test for each fact changed. Capture a snapshot from a deployment with
  `check.py --once --record`.
- Run the feature on the standing deployment:
  `angreal test bdd standing --chapter <chapter>`.

If the chapter needs work that the standing workload does not run, add the
package to `examples/fixtures/` and to `workload.py`, and name the workflow
in `EVIDENCE` for the promises it serves.

## Where the features go later

The project's decision on behaviour specs puts Gherkin features in a test
crate, `crates/cloacina-bdd`. That crate is not on the main branch. Until it
is, the features of this suite are in `features/` at the root of the
repository; they move into the crate as a plain file move. The driver stays
an angreal task over a Kubernetes deployment.
