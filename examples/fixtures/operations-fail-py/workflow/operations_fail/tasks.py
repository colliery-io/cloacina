"""A job of the behaviour suite's "Operations" chapter (CLOACINA-T-1115).

One task, `wobble`. The run's context says what it does:

- `mode` `fail`: every attempt fails. The run ends Failed, after its retries.
- `mode` `fail_once`: the first attempt on an agent fails, later ones pass.
  It remembers with a small file in the agent's temporary folder, keyed by
  `marker`, so with two agents it fails at most twice; its retries cover
  that. The run ends Completed.
- anything else: it passes.

Each attempt prints one line `OPERATIONS_OUTPUT <marker> ...`: the chapter
looks for it in the server's log (it must not be there) and in the
attempt's stored output.
"""
from __future__ import annotations

import hashlib
import os
import tempfile

import cloaca

WORKFLOW = "operations_fail"
RESULT_KEY = "operations_fail_done"
OUTPUT_PREFIX = "OPERATIONS_OUTPUT"
MEMORY_PREFIX = "operations-fail-"


def _memory_path(marker):
    digest = hashlib.sha256(str(marker).encode("utf-8")).hexdigest()[:16]
    return os.path.join(tempfile.gettempdir(), f"{MEMORY_PREFIX}{digest}")


@cloaca.task(dependencies=[], retry_attempts=3, timeout_seconds=120)
def wobble(context):
    marker = str(context.get("marker", None) or "")
    mode = context.get("mode", None)
    print(f"{OUTPUT_PREFIX} {marker} mode={mode}", flush=True)
    # The errors do not hold the marker: an error may be logged, the output
    # must not be.
    if mode == "fail":
        raise RuntimeError("operations_fail: fails for good")
    if mode == "fail_once":
        path = _memory_path(marker)
        if not os.path.exists(path):
            with open(path, "w", encoding="utf-8") as handle:
                handle.write("seen\n")
            raise RuntimeError("operations_fail: fails once")
    context.set(RESULT_KEY, {"marker": marker, "mode": mode})
    return context
