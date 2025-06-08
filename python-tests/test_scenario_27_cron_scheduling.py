"""
Test Cron Scheduling

This test file verifies cron scheduling functionality including schedule creation,
management, and execution monitoring.

Uses shared_runner fixture for workflow execution testing.
"""

from datetime import datetime, timezone


class TestCronScheduling:
    """Test comprehensive cron scheduling functionality."""

    def test_comprehensive_cron_scheduling(self, shared_runner):
        """Test comprehensive cron scheduling including CRUD operations and monitoring."""
        import cloaca

        # Define test tasks
        @cloaca.task(id="cron_test_task_1")
        def cron_test_task_1(context):
            context.set("cron_task_1_executed", True)
            context.set("execution_time", str(datetime.now(timezone.utc)))
            return context

        @cloaca.task(id="cron_test_task_2", dependencies=["cron_test_task_1"])
        def cron_test_task_2(context):
            context.set("cron_task_2_executed", True)
            return context

        # Register test workflow
        def create_cron_test_workflow():
            builder = cloaca.WorkflowBuilder("cron_test_workflow")
            builder.description("Test workflow for cron scheduling")
            builder.add_task("cron_test_task_1")
            builder.add_task("cron_test_task_2")
            return builder.build()

        cloaca.register_workflow_constructor("cron_test_workflow", create_cron_test_workflow)

        # Test 1: Register cron workflow
        print("Testing cron workflow registration...")

        schedule_id = shared_runner.register_cron_workflow(
            "cron_test_workflow",
            "0 0 * * *",  # Daily at midnight
            "UTC"
        )

        assert schedule_id is not None
        assert len(schedule_id) > 0
        print(f"✓ Cron schedule registered: {schedule_id}")

        # Test 2: List cron schedules
        print("Testing cron schedule listing...")

        schedules = shared_runner.list_cron_schedules(None, None, None)
        assert len(schedules) >= 1

        # Find our schedule
        our_schedule = None
        for schedule in schedules:
            if schedule["id"] == schedule_id:
                our_schedule = schedule
                break

        assert our_schedule is not None
        assert our_schedule["workflow_name"] == "cron_test_workflow"
        assert our_schedule["cron_expression"] == "0 0 * * *"
        assert our_schedule["timezone"] == "UTC"
        assert our_schedule["enabled"] is True
        print("✓ Cron schedule listing works correctly")

        # Test 3: Get specific cron schedule
        print("Testing get cron schedule...")

        schedule_details = shared_runner.get_cron_schedule(schedule_id)
        assert schedule_details["id"] == schedule_id
        assert schedule_details["workflow_name"] == "cron_test_workflow"
        assert schedule_details["cron_expression"] == "0 0 * * *"
        assert schedule_details["timezone"] == "UTC"
        assert schedule_details["enabled"] is True
        print("✓ Get cron schedule works correctly")

        # Test 4: Update cron schedule
        print("Testing cron schedule updates...")

        shared_runner.update_cron_schedule(
            schedule_id,
            "0 12 * * *",  # Daily at noon
            "America/New_York"
        )

        updated_schedule = shared_runner.get_cron_schedule(schedule_id)
        assert updated_schedule["cron_expression"] == "0 12 * * *"
        assert updated_schedule["timezone"] == "America/New_York"
        print("✓ Cron schedule update works correctly")

        # Test 5: Enable/disable cron schedule
        print("Testing cron schedule enable/disable...")

        # Disable schedule
        shared_runner.set_cron_schedule_enabled(schedule_id, False)
        disabled_schedule = shared_runner.get_cron_schedule(schedule_id)
        assert disabled_schedule["enabled"] is False

        # Re-enable schedule
        shared_runner.set_cron_schedule_enabled(schedule_id, True)
        enabled_schedule = shared_runner.get_cron_schedule(schedule_id)
        assert enabled_schedule["enabled"] is True
        print("✓ Cron schedule enable/disable works correctly")

        # Test 6: Get cron execution history
        print("Testing cron execution history...")

        history = shared_runner.get_cron_execution_history(schedule_id, None, None)
        assert isinstance(history, list)
        # History might be empty since we haven't had time-based executions
        print(f"✓ Cron execution history retrieved: {len(history)} entries")

        # Test 7: Get cron execution statistics
        print("Testing cron execution statistics...")

        # Get stats from last hour
        since_time = datetime.now(timezone.utc).isoformat()
        stats = shared_runner.get_cron_execution_stats(since_time)

        assert "total_executions" in stats
        assert "successful_executions" in stats
        assert "lost_executions" in stats
        assert "success_rate" in stats
        assert isinstance(stats["total_executions"], int)
        assert isinstance(stats["successful_executions"], int)
        assert isinstance(stats["lost_executions"], int)
        assert isinstance(stats["success_rate"], float)
        print("✓ Cron execution statistics work correctly")

        # Test 8: List with pagination and filtering
        print("Testing cron schedule listing with options...")

        # Test enabled only filter
        enabled_schedules = shared_runner.list_cron_schedules(True, 10, 0)
        assert len(enabled_schedules) >= 1

        for schedule in enabled_schedules:
            assert schedule["enabled"] is True

        # Test pagination
        first_page = shared_runner.list_cron_schedules(None, 1, 0)
        assert len(first_page) <= 1
        print("✓ Cron schedule listing with options works correctly")

        # Test 9: Delete cron schedule
        print("Testing cron schedule deletion...")

        shared_runner.delete_cron_schedule(schedule_id)

        # Verify schedule is deleted
        remaining_schedules = shared_runner.list_cron_schedules(None, None, None)
        schedule_still_exists = any(s["id"] == schedule_id for s in remaining_schedules)
        assert not schedule_still_exists
        print("✓ Cron schedule deletion works correctly")

        # Test 10: Error handling
        print("Testing cron error handling...")

        # Try to get deleted schedule
        try:
            shared_runner.get_cron_schedule(schedule_id)
            assert False, "Expected error when getting deleted schedule"
        except ValueError as e:
            assert "Failed to get cron schedule" in str(e)

        # Try to update non-existent schedule
        try:
            shared_runner.update_cron_schedule(schedule_id, "0 0 * * *", "UTC")
            assert False, "Expected error when updating deleted schedule"
        except ValueError as e:
            assert "Failed to update cron schedule" in str(e)

        # Try invalid schedule ID format
        try:
            shared_runner.get_cron_schedule("invalid-id")
            assert False, "Expected error with invalid schedule ID"
        except ValueError as e:
            assert "Invalid schedule ID" in str(e) or "Failed to get cron schedule" in str(e)

        # Try invalid datetime format for stats
        try:
            shared_runner.get_cron_execution_stats("invalid-datetime")
            assert False, "Expected error with invalid datetime"
        except ValueError as e:
            assert "Invalid datetime format" in str(e)

        print("✓ Cron error handling works correctly")

        # Summary
        cron_features_tested = 10
        print(f"\nCron scheduling features tested: {cron_features_tested}/10")
        print("✓ All cron scheduling features work correctly")

        print("✓ Comprehensive cron scheduling test completed")
