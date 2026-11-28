# Behaviour specs

Each folder here is one chapter of the behaviour suite: a row of the system's
promises, written as a Gherkin feature and run against a Kubernetes
deployment of server, compilers and agents.

| Folder | Chapter | Driver |
|---|---|---|
| `publishing/` | A team publishes its own jobs to a shared orchestrator (CLOACINA-T-1026) | `angreal test bdd publish`, or `angreal test bdd standing --chapter publishing` |
| `standing/` | The standing workload of several teams (CLOACINA-T-1107) | `angreal test bdd standing` |
| `tenant_code/` | Tenant code runs only on the tenant's agents and in its compiler: one scenario for each claim of CLOACINA-A-0019 (CLOACINA-T-1079) | `angreal test bdd standing --chart-actuator --chapter tenant_code` |

The `tenant_code` chapter makes a tenant of its own, `team_d`, and takes its
agent away in one scenario. Its compiler scenarios need `--chart-actuator`:
the compilers that the driver applies itself connect with the admin
database URL, not with the tenant's build role.

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
