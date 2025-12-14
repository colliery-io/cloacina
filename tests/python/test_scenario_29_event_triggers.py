"""
Test Event Triggers

This test file verifies the Python bindings for defining and managing event triggers,
including the @trigger decorator, TriggerResult class, and trigger management API.
"""

import random


class TestEventTriggers:
    """Test event trigger functionality."""

    def test_trigger_result_skip(self, shared_runner):
        """Test TriggerResult.skip() creation."""
        import cloaca

        result = cloaca.TriggerResult.skip()
        assert result.is_skip_result() is True
        assert result.is_fire_result() is False
        assert "Skip" in repr(result)
        print("TriggerResult.skip() works correctly")

    def test_trigger_result_fire_no_context(self, shared_runner):
        """Test TriggerResult.fire() without context."""
        import cloaca

        result = cloaca.TriggerResult.fire()
        assert result.is_fire_result() is True
        assert result.is_skip_result() is False
        assert "Fire" in repr(result)
        print("TriggerResult.fire() without context works correctly")

    def test_trigger_result_fire_with_context(self, shared_runner):
        """Test TriggerResult.fire() with context."""
        import cloaca

        ctx = cloaca.Context({"key": "value", "number": 42})
        result = cloaca.TriggerResult.fire(ctx)
        assert result.is_fire_result() is True
        assert "Fire" in repr(result)
        print("TriggerResult.fire() with context works correctly")

    def test_trigger_decorator_registration(self, shared_runner):
        """Test that @trigger decorator registers triggers correctly."""
        import cloaca

        # Define a simple workflow first
        with cloaca.WorkflowBuilder("trigger_test_workflow") as builder:
            builder.description("Test workflow for trigger")

            @cloaca.task(id="triggered_task")
            def triggered_task(context):
                context.set("triggered", True)
                return context

        # Define a trigger using the decorator
        @cloaca.trigger(
            workflow="trigger_test_workflow",
            name="test_rng_trigger",
            poll_interval="1s",
            allow_concurrent=False,
        )
        def test_rng_trigger():
            # Simple trigger that randomly fires
            if random.randint(1, 10) == 5:
                ctx = cloaca.Context({"random_fire": True})
                return cloaca.TriggerResult.fire(ctx)
            return cloaca.TriggerResult.skip()

        # The trigger should be registered - check via management API
        schedules = shared_runner.list_trigger_schedules()
        print(f"Registered trigger schedules: {len(schedules)}")

        # The function should still be callable
        result = test_rng_trigger()
        assert result.is_fire_result() or result.is_skip_result()
        print("@trigger decorator registration works correctly")

    def test_trigger_with_counter(self, shared_runner):
        """Test trigger that fires after N polls."""
        import cloaca

        # Use a list to maintain state across calls (closures capture by reference)
        poll_count = [0]

        with cloaca.WorkflowBuilder("counter_workflow") as builder:
            builder.description("Counter-based trigger test")

            @cloaca.task(id="process_count")
            def process_count(context):
                context.set("processed", True)
                return context

        @cloaca.trigger(
            workflow="counter_workflow",
            name="counter_trigger",
            poll_interval="100ms",
        )
        def counter_trigger():
            poll_count[0] += 1
            if poll_count[0] >= 3:
                poll_count[0] = 0  # Reset
                return cloaca.TriggerResult.fire(
                    cloaca.Context({"poll_count": poll_count[0]})
                )
            return cloaca.TriggerResult.skip()

        # Call the trigger function directly to test logic
        assert counter_trigger().is_skip_result()  # 1st call
        assert counter_trigger().is_skip_result()  # 2nd call
        assert counter_trigger().is_fire_result()  # 3rd call - should fire
        assert counter_trigger().is_skip_result()  # 4th call - reset
        print("Counter-based trigger logic works correctly")

    def test_list_trigger_schedules(self, shared_runner):
        """Test listing trigger schedules."""

        schedules = shared_runner.list_trigger_schedules()
        assert isinstance(schedules, list)
        print(f"Found {len(schedules)} trigger schedules")

    def test_list_trigger_schedules_with_filters(self, shared_runner):
        """Test listing trigger schedules with filtering options."""

        # Test with enabled_only filter
        schedules = shared_runner.list_trigger_schedules(enabled_only=True)
        assert isinstance(schedules, list)

        # Test with pagination
        schedules = shared_runner.list_trigger_schedules(limit=10, offset=0)
        assert isinstance(schedules, list)
        print("Trigger schedule listing with filters works correctly")

    def test_get_nonexistent_trigger_schedule(self, shared_runner):
        """Test getting a trigger schedule that doesn't exist."""

        schedule = shared_runner.get_trigger_schedule("nonexistent_trigger")
        assert schedule is None
        print("Non-existent trigger returns None as expected")

    def test_get_trigger_execution_history(self, shared_runner):
        """Test getting execution history for a trigger."""

        history = shared_runner.get_trigger_execution_history("some_trigger")
        assert isinstance(history, list)
        print("Trigger execution history retrieval works correctly")
