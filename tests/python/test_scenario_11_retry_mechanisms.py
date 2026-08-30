"""
Test Retry Mechanisms

This test file verifies configurable retry policies for tasks.
Tests include retry attempts, backoff strategies, and delay configurations.

Uses shared_runner fixture for actual workflow execution.
"""



class TestRetryMechanisms:
    """Test configurable retry policies."""

    def test_task_with_retry_policy(self, shared_runner):
        """Test task with retry configuration executes successfully."""
        import cloaca

        with cloaca.WorkflowBuilder("retry_workflow") as builder:
            builder.description("Retry policy test")

            @cloaca.task(
                id="retry_task",
                retry_attempts=3,
                retry_backoff="exponential",
                retry_delay_ms=100
            )
            def retry_task(context):
                context.set("retry_task_executed", True)
                context.set("retry_attempts_configured", 3)
                return context

        # Execute workflow
        context = cloaca.Context({"test_type": "retry"})
        result = shared_runner.execute("retry_workflow", context)

        assert result is not None
        assert result.status == "Completed"

    def test_retry_policy_value_objects_construct(self):
        """CLOACI-T-0882: the RetryPolicy value objects are exported and usable.

        These classes were exported and asserted-present by the I-0137
        authorship-contract test, but NOTHING had ever constructed one, so
        nobody could say whether they worked. Build one through the full
        builder chain and read every getter back.
        """
        import cloaca

        policy = (
            cloaca.RetryPolicy.builder()
            .max_attempts(5)
            .initial_delay(0.25)
            .max_delay(10.0)
            .backoff_strategy(cloaca.BackoffStrategy.exponential(2.0))
            .retry_condition(cloaca.RetryCondition.transient_only())
            .with_jitter(True)
            .build()
        )

        assert policy.max_attempts == 5
        assert policy.initial_delay == 0.25
        assert policy.max_delay == 10.0
        assert policy.with_jitter is True
        assert "RetryPolicy(" in repr(policy)

        # The backoff must actually grow — a Fixed strategy would return the
        # initial delay for every attempt, which is how a silently-defaulted
        # strategy would present itself.
        assert policy.calculate_delay(2) > policy.calculate_delay(1)

        assert cloaca.RetryPolicy.default().max_attempts >= 1

    def test_task_accepts_retry_policy_object(self, shared_runner):
        """CLOACI-T-0882: @task(retry=RetryPolicy(...)) is wired end to end."""
        import cloaca

        policy = cloaca.RetryPolicy.builder().max_attempts(4).initial_delay(0.05).build()

        with cloaca.WorkflowBuilder("retry_object_workflow") as builder:
            builder.description("RetryPolicy object test")

            @cloaca.task(id="retry_object_task", retry=policy)
            def retry_object_task(context):
                context.set("retry_object_task_executed", True)
                return context

        context = cloaca.Context({"test_type": "retry_object"})
        result = shared_runner.execute("retry_object_workflow", context)

        assert result is not None
        assert result.status == "Completed"

    def test_retry_object_and_kwargs_are_mutually_exclusive(self):
        """CLOACI-T-0882: mixing the two surfaces fails loudly.

        Silently preferring one would leave the ignored surface looking
        effective — the exact "configured it, nothing happened" failure this
        ticket exists to prevent.
        """
        import cloaca
        import pytest

        policy = cloaca.RetryPolicy.builder().max_attempts(4).build()

        with pytest.raises(ValueError) as excinfo:
            @cloaca.task(id="conflicting_task", retry=policy, retry_attempts=2)
            def conflicting_task(context):
                return context

        message = str(excinfo.value)
        assert "retry_attempts" in message
        assert "not both" in message

    def test_unknown_retry_strings_are_rejected(self):
        """CLOACI-T-0930: a typo'd retry string must FAIL, not silently default.

        The old fallthrough mapped retry_backoff="exponentail" to Fixed and any
        unknown retry_condition to AllErrors -- the workflow ran with retry
        semantics its author never asked for, invisibly. The population hitting
        the fallback is exactly the misconfigured one, so it fails loudly now.
        """
        import cloaca
        import pytest

        with pytest.raises(ValueError) as excinfo:
            @cloaca.task(id="typo_backoff_task", retry_backoff="exponentail")
            def typo_backoff_task(context):
                return context
        assert "exponentail" in str(excinfo.value)
        assert "exponential" in str(excinfo.value)  # the message teaches the fix

        with pytest.raises(ValueError) as excinfo:
            @cloaca.task(id="typo_condition_task", retry_condition="transiant")
            def typo_condition_task(context):
                return context
        assert "transiant" in str(excinfo.value)
        assert "transient" in str(excinfo.value)

    def test_all_valid_retry_strings_still_accepted(self):
        """CLOACI-T-0930 regression guard: tightening must not over-tighten.

        Every documented value must still construct -- the risk of the loud-
        failure change is breaking a legitimate string that used to work.
        """
        import cloaca

        # Inside builder contexts: valid strings get PAST retry parsing, so
        # they reach the workflow-context requirement the typo tests never do.
        with cloaca.WorkflowBuilder("valid_backoff_wf") as builder:
            builder.description("T-0930 valid backoff values")
            for i, backoff in enumerate(("fixed", "linear", "exponential")):
                @cloaca.task(id=f"valid_backoff_{i}", retry_backoff=backoff)
                def _t(context):
                    return context

        with cloaca.WorkflowBuilder("valid_condition_wf") as builder:
            builder.description("T-0930 valid condition values")
            for i, condition in enumerate(("never", "transient", "all")):
                @cloaca.task(id=f"valid_condition_{i}", retry_condition=condition)
                def _t2(context):
                    return context
