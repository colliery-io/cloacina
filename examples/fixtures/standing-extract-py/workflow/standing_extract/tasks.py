"""A data-shaped job of the behaviour suite's standing workload
(CLOACINA-T-1107): extract, transform, load, every five minutes.

It moves no real data. Each run writes the interval the scheduler gave it
(`interval_start`, `interval_end`) back into its context under
`standing_extract_loaded`, so the suite's checker can compare the interval a
run saw with the interval of its fire time.
"""
from __future__ import annotations

import json
import os
import socket
import time

import cloaca

WORKFLOW = "standing_extract"
RESULT_KEY = "standing_extract_loaded"


def _mark(context, task):
    """One marked line for each start of a task body."""
    line = "STANDING_MARK " + json.dumps({
        "workflow": WORKFLOW, "task": task, "scheduled_time": context.get("scheduled_time", None),
        "host": socket.gethostname(), "pid": os.getpid(), "at": time.time()}, sort_keys=True)
    print(line, flush=True)


@cloaca.task(dependencies=[])
def extract(context):
    _mark(context, "extract")
    start = context.get("interval_start", None)
    end = context.get("interval_end", None)
    # A row count that depends on the interval only, so a second run of the
    # same interval gives the same answer.
    rows = 100 + sum(ord(char) for char in str(end)) % 50
    context.set("standing_extract_rows", rows)
    context.set("standing_extract_interval", {"start": start, "end": end})
    return context


@cloaca.task(dependencies=["extract"])
def transform(context):
    _mark(context, "transform")
    rows = context.get("standing_extract_rows", 0) or 0
    context.set("standing_extract_kept", rows - rows % 7)
    return context


@cloaca.task(dependencies=["transform"])
def load(context):
    _mark(context, "load")
    interval = context.get("standing_extract_interval", None) or {}
    context.set(RESULT_KEY, {
        "interval_start": interval.get("start"),
        "interval_end": interval.get("end"),
        "scheduled_time": context.get("scheduled_time", None),
        "rows": context.get("standing_extract_kept", 0),
    })
    return context


# Every fifth minute.
@cloaca.trigger(on="standing_extract", cron="*/5 * * * *")
def standing_extract_schedule():
    pass
