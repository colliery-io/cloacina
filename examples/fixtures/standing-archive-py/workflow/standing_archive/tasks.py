"""A long job of the behaviour suite's standing workload (CLOACINA-T-1107):
one task that holds for six minutes, every hour.

Six minutes is longer than the periods a short task never meets: the push of
work to an agent, the heartbeat, and the default time limit of a task. The
task sets its own time limit above its hold. `hold_seconds` in the context
changes the hold, for a run started by hand.
"""
from __future__ import annotations

import json
import os
import socket
import time

import cloaca

WORKFLOW = "standing_archive"
RESULT_KEY = "standing_archive_done"
HOLD_SECONDS = 360


def _mark(context, task):
    """One marked line for each start of a task body. `schedule_id` and
    `scheduled_time` name a scheduled run, `run_once_tag` a run by hand."""
    line = "STANDING_MARK " + json.dumps({
        "workflow": WORKFLOW, "task": task, "scheduled_time": context.get("scheduled_time", None),
        "schedule_id": context.get("schedule_id", None),
        "run_once_tag": context.get("run_once_tag", None),
        "host": socket.gethostname(), "pid": os.getpid(), "at": time.time()}, sort_keys=True)
    print(line, flush=True)


@cloaca.task(dependencies=[], retry_attempts=0, timeout_seconds=900)
def archive(context):
    _mark(context, "archive")
    hold = context.get("hold_seconds", None)
    hold = float(HOLD_SECONDS if hold is None else hold)
    started = time.time()
    while time.time() - started < hold:
        time.sleep(min(5.0, max(0.0, hold - (time.time() - started))))
    context.set(RESULT_KEY, {
        "held_seconds": round(time.time() - started, 1),
        "host": socket.gethostname(),
        "interval_end": context.get("interval_end", None),
    })
    return context


# At minute 0 of every hour.
@cloaca.trigger(on="standing_archive", cron="0 * * * *")
def standing_archive_schedule():
    pass
