"""A job of the behaviour suite's "Operations" chapter (CLOACINA-T-1115).

One task of the agent pool `keeper`. No agent of the standing deployment is
in that pool, so a run of it waits and stays active. The retention job must
leave an active run, whatever its age: the chapter starts one such run for
team A and the checker looks for it days later.
"""
from __future__ import annotations

import cloaca

WORKFLOW = "operations_keeper"
RESULT_KEY = "operations_keeper_done"
POOL = "keeper"


@cloaca.task(dependencies=[], retry_attempts=0, pool=POOL)
def keep(context):
    context.set(RESULT_KEY, True)
    return context
