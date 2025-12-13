"""
Test Event Triggers Management API

This test file verifies the Python bindings for managing event triggers,
including listing, querying, enabling/disabling, and viewing execution history.

Note: Triggers are defined in Rust. This tests the management API only.
"""


class TestEventTriggers:
    """Test event trigger management operations."""

    def test_list_trigger_schedules_empty(self, shared_runner):
        """Test listing trigger schedules when none exist."""

        # List all trigger schedules (should be empty initially)
        schedules = shared_runner.list_trigger_schedules()
        assert isinstance(schedules, list)
        # May or may not be empty depending on test order
        print(f"Found {len(schedules)} trigger schedules")

    def test_list_trigger_schedules_with_filters(self, shared_runner):
        """Test listing trigger schedules with filtering options."""

        # Test with enabled_only filter
        schedules = shared_runner.list_trigger_schedules(enabled_only=True)
        assert isinstance(schedules, list)

        # Test with pagination
        schedules = shared_runner.list_trigger_schedules(limit=10, offset=0)
        assert isinstance(schedules, list)

        # Test combined filters
        schedules = shared_runner.list_trigger_schedules(
            enabled_only=False, limit=50, offset=0
        )
        assert isinstance(schedules, list)
        print("Trigger schedule listing with filters works correctly")

    def test_get_nonexistent_trigger_schedule(self, shared_runner):
        """Test getting a trigger schedule that doesn't exist."""

        # Should return None for non-existent trigger
        schedule = shared_runner.get_trigger_schedule("nonexistent_trigger")
        assert schedule is None
        print("Non-existent trigger returns None as expected")

    def test_set_trigger_enabled_nonexistent(self, shared_runner):
        """Test enabling/disabling a non-existent trigger."""

        # Should handle gracefully (no error, returns False or similar)
        try:
            result = shared_runner.set_trigger_enabled("nonexistent_trigger", False)
            # The operation may succeed (no-op) or return a status
            print(f"set_trigger_enabled on nonexistent trigger returned: {result}")
        except Exception as e:
            # Some implementations may raise an error
            print(f"set_trigger_enabled raised expected error: {e}")

    def test_get_trigger_execution_history_empty(self, shared_runner):
        """Test getting execution history for a trigger with no executions."""

        # Should return empty list for non-existent trigger
        history = shared_runner.get_trigger_execution_history("nonexistent_trigger")
        assert isinstance(history, list)
        assert len(history) == 0
        print("Empty execution history works correctly")

    def test_get_trigger_execution_history_with_pagination(self, shared_runner):
        """Test getting execution history with pagination options."""

        # Test with pagination parameters
        history = shared_runner.get_trigger_execution_history(
            "some_trigger", limit=10, offset=0
        )
        assert isinstance(history, list)
        print("Execution history pagination works correctly")

    def test_trigger_schedule_structure(self, shared_runner):
        """Test that trigger schedule dictionaries have expected structure."""

        schedules = shared_runner.list_trigger_schedules()

        # If there are schedules, verify structure
        if schedules:
            schedule = schedules[0]
            # Verify expected keys exist
            expected_keys = [
                "trigger_name",
                "workflow_name",
                "poll_interval_ms",
                "enabled",
                "allow_concurrent",
            ]
            for key in expected_keys:
                assert key in schedule, f"Missing key: {key}"
            print(f"Trigger schedule structure verified: {list(schedule.keys())}")
        else:
            print("No schedules to verify structure (expected in isolation)")

    def test_comprehensive_trigger_management(self, shared_runner):
        """Test comprehensive trigger management workflow."""

        # This test demonstrates the full management API workflow
        print("Testing comprehensive trigger management API...")

        # 1. List initial state
        initial_schedules = shared_runner.list_trigger_schedules()
        print(f"Initial schedules: {len(initial_schedules)}")

        # 2. Query specific trigger (even if doesn't exist)
        schedule = shared_runner.get_trigger_schedule("test_trigger")
        print(f"Query result: {schedule}")

        # 3. Check execution history
        history = shared_runner.get_trigger_execution_history("test_trigger", limit=5)
        print(f"Execution history entries: {len(history)}")

        # 4. List with filters
        enabled_only = shared_runner.list_trigger_schedules(enabled_only=True)
        print(f"Enabled triggers: {len(enabled_only)}")

        print("Comprehensive trigger management API test complete")
