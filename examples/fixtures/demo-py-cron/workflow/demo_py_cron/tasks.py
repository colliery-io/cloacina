"""Demo Python cron-trigger workflow (CLOACI-T-0688 + CLOACI-T-0763).

Showcases the Python packaged cron-trigger authoring surface (T-0688:
`@cloaca.trigger(on=…, cron=…)`) AND the Python trigger-rule parity closed in
T-0763 — `@cloaca.task(trigger_rules=…)` now gates a Python task so it lands in
the real `Skipped` state, exactly like Rust's `#[task(trigger_rules = …)]`.

    py_poll ──┬─▶ py_process ─▶ py_record
              └─▶ py_audit (SKIPPED)

`py_poll` sets `do_audit = False`, so `py_audit`'s trigger rule never fires →
the planner Skips it; `py_record` fans in on `py_process` + `py_audit` (a
Skipped dep counts as resolved) and still runs. This is the Python mirror of
`demo-cron-rust`.
"""
from __future__ import annotations

import cloaca


# CLOACI-T-0768: declared injectors (typed params). All defaulted — the cron
# trigger fires this workflow unattended.
@cloaca.workflow_params(
    region=(str, "us-east"),
    batch_size=(int, 200),
)
@cloaca.task(dependencies=[])
def py_poll(context):
    # Gate the audit branch off → py_audit is Skipped every run.
    context.set("do_audit", False)
    context.set("demo_py_cron_polled", True)
    return context


@cloaca.task(dependencies=["py_poll"])
def py_process(context):
    context.set("demo_py_cron_ran", True)
    return context


# Real trigger-rule gating (T-0763): the rule wants do_audit == True, which never
# holds → this task Skips (its body never runs). Kept so the DAG has the node.
@cloaca.task(
    dependencies=["py_poll"],
    trigger_rules=cloaca.context_value("do_audit", "Equals", True),
)
def py_audit(context):
    context.set("demo_py_cron_audited", True)
    return context


# Fan-in on the run path + the skipped path.
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
