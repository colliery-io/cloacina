"""A job of the behaviour suite's standing workload (CLOACINA-T-1107) that
fails on purpose, every five minutes.

Two kinds of failure, both planned:

- `wobble` fails the first time it runs for a fire time on an agent, and
  passes after that. It remembers with a small file in the agent's temporary
  folder, so with two agents it fails at most twice for one run. Its retry
  policy covers that. A run that ends Completed here has used its retries.
- `settle` fails, with no retry, when the minute of the run's fire time is 0
  or 30. That run ends Failed: twice an hour, on a schedule anyone can
  compute. A run started by hand has no fire time and does not fail.

The suite's checker reads both from the run history: the attempts of
`wobble`, and the runs that failed for good against the fire times that
should.
"""
from __future__ import annotations

import hashlib
import json
import os
import socket
import tempfile
import time
from datetime import datetime

import cloaca

WORKFLOW = "standing_flaky"
RESULT_KEY = "standing_flaky_done"
PLANNED_FAILURE_MINUTES = (0, 30)
MEMORY_PREFIX = "standing-flaky-"
MEMORY_MAX_AGE_SECONDS = 3600


def _mark(context, task):
    """One marked line for each start of a task body."""
    line = "STANDING_MARK " + json.dumps({
        "workflow": WORKFLOW, "task": task, "scheduled_time": context.get("scheduled_time", None),
        "host": socket.gethostname(), "pid": os.getpid(), "at": time.time()}, sort_keys=True)
    print(line, flush=True)


def planned_failure(scheduled_time):
    """True when a run with this fire time fails for good."""
    if not scheduled_time or not isinstance(scheduled_time, str):
        return False
    try:
        # Before Python 3.11, `fromisoformat` does not read a trailing Z.
        fired = datetime.fromisoformat(scheduled_time.replace("Z", "+00:00"))  # noqa: FURB162
    except ValueError:
        return False
    return fired.minute in PLANNED_FAILURE_MINUTES


def _memory_path(scheduled_time):
    digest = hashlib.sha256(str(scheduled_time).encode("utf-8")).hexdigest()[:16]
    return os.path.join(tempfile.gettempdir(), f"{MEMORY_PREFIX}{digest}")


def _forget_old():
    folder = tempfile.gettempdir()
    now = time.time()
    try:
        names = os.listdir(folder)
    except OSError:
        return
    for name in names:
        if not name.startswith(MEMORY_PREFIX):
            continue
        path = os.path.join(folder, name)
        try:
            if now - os.path.getmtime(path) > MEMORY_MAX_AGE_SECONDS:
                os.remove(path)
        except OSError:
            pass


@cloaca.task(dependencies=[], retry_attempts=4)
def wobble(context):
    _mark(context, "wobble")
    scheduled_time = context.get("scheduled_time", None)
    if scheduled_time:
        _forget_old()
        path = _memory_path(scheduled_time)
        if not os.path.exists(path):
            with open(path, "w", encoding="utf-8") as handle:
                handle.write("seen\n")
            raise RuntimeError("standing_flaky: planned failure of the first attempt")
    context.set("standing_flaky_wobbled", True)
    return context


@cloaca.task(dependencies=["wobble"], retry_attempts=0)
def settle(context):
    _mark(context, "settle")
    scheduled_time = context.get("scheduled_time", None)
    if planned_failure(scheduled_time):
        raise RuntimeError("standing_flaky: planned failure of the run")
    context.set(RESULT_KEY, {"scheduled_time": scheduled_time})
    return context


# Every fifth minute.
@cloaca.trigger(on="standing_flaky", cron="*/5 * * * *")
def standing_flaky_schedule():
    pass
