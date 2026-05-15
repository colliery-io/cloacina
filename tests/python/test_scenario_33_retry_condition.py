"""
Test scenario 33: retry_condition end-to-end (CLOACI-T-0042).

Exercises the per-task `retry_condition` parameter through the Python
bindings. Mirrors the Rust integration test in
`crates/cloacina/tests/integration/executor/retry_condition.rs`.

- `transient` retries on a "connection refused" style error and succeeds
  on the third attempt
- `never` skips retries even with `retry_attempts > 0`
"""

import threading

import cloaca


_FLAKY_ATTEMPTS = {"count": 0, "lock": threading.Lock()}
_NEVER_ATTEMPTS = {"count": 0, "lock": threading.Lock()}


def _bump(counter):
    with counter["lock"]:
        counter["count"] += 1
        return counter["count"]


class TestRetryCondition:
    """Per-task retry-condition policies (CLOACI-T-0042)."""

    def test_transient_retries_then_succeeds(self, shared_runner):
        """retry_condition='transient' retries on connection-flavored errors."""
        _FLAKY_ATTEMPTS["count"] = 0

        with cloaca.WorkflowBuilder("retry_condition_transient") as builder:
            builder.description("Transient retries should land on success")

            @cloaca.task(
                id="flaky_api_call",
                retry_attempts=3,
                retry_delay_ms=50,
                retry_max_delay_ms=200,
                retry_jitter=False,
                retry_condition="transient",
            )
            def flaky_api_call(context):
                attempt = _bump(_FLAKY_ATTEMPTS)
                if attempt < 3:
                    # Substring "connection" matches the TransientOnly
                    # pattern matcher in cloacina-workflow::retry.
                    raise RuntimeError("connection refused (simulated)")
                context.set("flaky_attempt", attempt)
                return context

        context = cloaca.Context({})
        result = shared_runner.execute("retry_condition_transient", context)

        assert result is not None
        assert result.status == "Completed", (
            f"expected transient retries to succeed; got {result.status}"
        )
        assert _FLAKY_ATTEMPTS["count"] == 3, (
            f"expected 3 attempts (2 transient failures + 1 success); "
            f"got {_FLAKY_ATTEMPTS['count']}"
        )

    def test_never_skips_retries(self, shared_runner):
        """retry_condition='never' stops at the first failure."""
        _NEVER_ATTEMPTS["count"] = 0

        with cloaca.WorkflowBuilder("retry_condition_never") as builder:
            builder.description("Never-retry policy must not retry on failure")

            @cloaca.task(
                id="validation_check",
                retry_attempts=5,
                retry_delay_ms=50,
                retry_max_delay_ms=200,
                retry_jitter=False,
                retry_condition="never",
            )
            def validation_check(context):
                _bump(_NEVER_ATTEMPTS)
                raise RuntimeError("input failed schema validation")

        context = cloaca.Context({})
        result = shared_runner.execute("retry_condition_never", context)

        assert result is not None
        assert result.status == "Failed", (
            f"retry_condition=never must surface the failure; got {result.status}"
        )
        assert _NEVER_ATTEMPTS["count"] == 1, (
            f"expected exactly 1 attempt with retry_condition=never; "
            f"got {_NEVER_ATTEMPTS['count']}"
        )
