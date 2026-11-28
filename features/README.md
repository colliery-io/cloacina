# Behaviour specs

Each folder here is one chapter of the behaviour suite: a row of the system's
promises, written as a Gherkin feature and run against a Kubernetes
deployment of server, compilers and agents.

| Folder | Chapter | Driver |
|---|---|---|
| `capacity/` | Work waits when a team is full, and a task lands on an agent that fits it (CLOACINA-T-1114) | `angreal test bdd standing --chapter capacity` |
| `publishing/` | A team publishes its own jobs to a shared orchestrator (CLOACINA-T-1026) | `angreal test bdd publish`, or `angreal test bdd standing --chapter publishing` |
| `operations/` | Alerts, the audit record, the backlog and the lag, retention, a server log with no task output (CLOACINA-T-1115) | `angreal test bdd standing --chapter operations` |
| `standing/` | The standing workload of several teams (CLOACINA-T-1107) | `angreal test bdd standing` |
| `tenant_code/` | Tenant code runs only on the tenant's agents and in its compiler: one scenario for each claim of CLOACINA-A-0019 (CLOACINA-T-1079) | `angreal test bdd standing --chart-actuator --chapter tenant_code` |
| `failure/` | An agent dies, the server restarts, Postgres goes away, and work survives (CLOACINA-T-1112) | `angreal test bdd standing --chapter failure` |
| `run_once/` | A task runs one time: a long task, a task with no free agent, a task whose result reaches the other server replica (CLOACINA-T-1110) | `angreal test bdd standing --chapter run_once` |
| `schedules/` | A schedule fires on time, a run knows its interval, a backlog drains in order (CLOACINA-T-1111) | `angreal test bdd standing --chapter schedules` |

The `tenant_code` chapter makes a tenant of its own, `team_d`, and takes its
agent away in one scenario. Its compiler scenarios need `--chart-actuator`:
the compilers that the driver applies itself connect with the admin
database URL, not with the tenant's build role.

The `failure` chapter makes its own disturbances through the runner of the
disturbance schedule, so it acts on the suite's own cluster only. Its
Postgres scenario takes the database away for eleven minutes. Do not run it
while `--disturb` runs a disturbance: the runner's lock makes the second a
skip, and the scenario fails with that reason.

The `schedules` chapter makes two schedules of its own on team B and removes
them at the end of their scenarios. Its last two scenarios restart both
server replicas at once through the runner of the disturbance schedule, which
writes each restart to the disturbance record
(`bdd-standing-report/disturbances.jsonl`), and open a port forward of their
own on port 18110. Do not run them while `--disturb` runs a disturbance: the
runner's lock makes the second a skip, and the scenario fails with that
reason.

```
angreal test bdd standing --dry-run     # every feature parsed, every step resolved; no cluster
angreal test bdd standing --self-test   # unit tests of the checker and the fixtures; no server
angreal test bdd standing --disturb --dry-run   # the disturbance schedule and its commands; touches nothing
```

A chapter owns its feature here, its steps in
`.angreal/test/bdd/<chapter>_steps.py` and its checker rules in
`.angreal/test/bdd/suite/rules/<chapter>.py`. The files are found by their
names; no list names the chapters.

What the suite is, how to bring it up, how to read the checker's report and
how to add a chapter:
`docs/src/content/docs/contributing/behaviour-suite.md`.

A scenario tagged `@gap` does not hold today. It still runs, and its failure
does not fail the run. The tag after it is the Kairos task that owns the gap,
or `@unfiled`.
