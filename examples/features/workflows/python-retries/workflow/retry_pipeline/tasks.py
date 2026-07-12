"""Python packaged RETRY-POLICY workflow — the Python peer of the Rust
`conditional-retries` example.

`fetch_unreliable` simulates a flaky external call: it RAISES on its first two
attempts and only succeeds on the third. The task carries
`@cloaca.task(retry_attempts=4, retry_backoff="fixed", retry_delay_ms=200)`, so
the ONLY way the run reaches `Completed` is if the server honored the retry
policy and re-executed the task. If retries were ignored, attempt 1's exception
would fail the whole execution.

    fetch_unreliable (fails x2, succeeds on 3) -> summarize

The attempt counter is a module global. Packaged Python tasks run in-process in
the server, and the module is imported once, so the counter persists across the
retries of a single execution (this mirrors tutorial 04's `call_count` pattern).
A fresh gold-path lane starts a fresh server, so the counter starts at 0.
"""
from __future__ import annotations

import cloaca

# Attempt counter — persists across retries within one server process.
_attempts = {"fetch_unreliable": 0}

# How many attempts must fail before success (attempts 1 and 2 fail; 3 succeeds).
_FAIL_UNTIL = 3


@cloaca.task(
    id="fetch_unreliable",
    dependencies=[],
    retry_attempts=4,
    retry_backoff="fixed",
    retry_delay_ms=200,
)
def fetch_unreliable(context):
    """what: A flaky fetch that fails twice then succeeds.

    why: The deliberate early failures make the retry policy observable — the
    run can only complete if the server actually re-ran this task. Without
    retries honored, attempt 1's raise would fail the execution.
    """
    _attempts["fetch_unreliable"] += 1
    n = _attempts["fetch_unreliable"]
    if n < _FAIL_UNTIL:
        # Transient failure — the retry policy should re-execute us.
        raise RuntimeError(f"transient failure on attempt {n} (will retry)")
    context.set("succeeded_on_attempt", n)
    context.set("payload", {"id": "data_001", "ok": True})
    return context


@cloaca.task(id="summarize", dependencies=["fetch_unreliable"])
def summarize(context):
    """what: Record which attempt finally succeeded. why: a terminal step so a
    completed run is observably distinct from one that died mid-retry."""
    attempt = context.get("succeeded_on_attempt")
    context.set("summary", f"fetch succeeded on attempt {attempt}")
    return context
