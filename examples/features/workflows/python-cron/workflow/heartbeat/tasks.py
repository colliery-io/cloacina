"""Python packaged CRON-TRIGGER workflow — fires on a schedule via the cron
scheduler (the distinct-subsystem peer of `python-triggers`, which polls).

A `@cloaca.trigger(on="heartbeat_workflow", cron="*/3 * * * * *")` fires the
`heartbeat_workflow` every 3 seconds. Unlike a poll trigger, the cron scheduler
fires the `on` workflow directly on the schedule — the decorated function body
is unused (cron and poll_interval are mutually exclusive; `on` is required for
cron). Executions appear automatically; no `workflow run`.

    beat  (records a heartbeat each scheduled fire)

Tasks are declared with bare `@cloaca.task` decorators — the packaged loader
builds the workflow context from `workflow_name` before importing this module,
so they register on import (no `WorkflowBuilder`, that is for in-process runs).
"""
from __future__ import annotations

import cloaca


@cloaca.task(id="beat", dependencies=[])
def beat(context):
    """what: Record a single heartbeat. why: a minimal terminal task so each
    scheduled cron fire produces an observable Completed execution."""
    context.set("heartbeat", True)
    return context


# Fire `heartbeat_workflow` every 3 seconds. The cron scheduler fires the `on`
# workflow directly, so this body is unused — its presence at import IS the
# declaration (there is no triggers section in package.toml).
@cloaca.trigger(on="heartbeat_workflow", cron="*/3 * * * * *")
def heartbeat_cron():
    pass
