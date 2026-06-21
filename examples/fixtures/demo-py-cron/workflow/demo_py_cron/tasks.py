"""Demo Python cron-trigger workflow (CLOACI-T-0688).

Showcases the Python packaged cron-trigger authoring surface closed in T-0688:
`@cloaca.trigger(on=…, cron=…)` mirrors Rust's `#[trigger(on=…, cron=…)]`.
Python `@cloaca.trigger` was previously poll-only (no cron/timezone params), so
cron scheduling could only be driven via the runner-level API — never authored
into a package. This fixture is the Python mirror of `demo-cron-rust`.

The reconciler imports `entry_module` (this file) at load time; the
`@cloaca.task` decorators assemble the workflow named in package.toml
(`demo_py_cron_workflow`), and the `@cloaca.trigger(cron=…)` registers a cron
schedule for that workflow — no `triggers` section in package.toml, the
decorator is the declaration.

Shape (branching so the cron runs aren't single-dot DAGs in the UI; Python tasks
have no trigger-rule gating, so this branches + fans in but does not skip — the
skipped-node demos are the Rust cron/branch fixtures):

    py_poll ──┬─▶ py_process ─▶ py_record
              └─▶ py_audit ────┘
"""
from __future__ import annotations

import cloaca


@cloaca.task(dependencies=[])
def py_poll(context):
    context.set("demo_py_cron_polled", True)
    return context


@cloaca.task(dependencies=["py_poll"])
def py_process(context):
    context.set("demo_py_cron_ran", True)
    return context


@cloaca.task(dependencies=["py_poll"])
def py_audit(context):
    context.set("demo_py_cron_audited", True)
    return context


@cloaca.task(dependencies=["py_process", "py_audit"])
def py_record(context):
    context.set("demo_py_cron_recorded", True)
    return context


# Fire `demo_py_cron_workflow` every 15 seconds — frequent enough to watch the
# Triggers view tick and executions auto-appear. The cron function body is
# unused for cron triggers (the cron scheduler fires the `on` workflow directly);
# `cron` and `poll_interval` are mutually exclusive, and `on` is required for cron.
@cloaca.trigger(on="demo_py_cron_workflow", cron="*/15 * * * * *")
def demo_py_cron_trigger():
    pass
