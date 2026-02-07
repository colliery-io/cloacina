"""
Test Scenario 31: TaskHandle and defer_until

Tests that:
- Tasks with a `handle` second parameter are detected as handle-aware
- TaskHandle.defer_until works end-to-end through the executor
- The task function receives both context and handle arguments
- Handle detection works with both `handle` and `task_handle` parameter names
- Non-handle tasks continue to work normally alongside handle tasks
"""



class TestTaskHandleDetection:
    """Test that the @task decorator correctly detects handle parameters."""

    def test_task_without_handle_is_callable(self):
        """A normal task (no handle param) should work as before."""
        import cloaca

        with cloaca.WorkflowBuilder("test_no_handle_detection") as _builder:
            @cloaca.task(id="no_handle_task")
            def no_handle_task(context):
                context.set("executed", True)
                return context

            assert callable(no_handle_task)
            ctx = cloaca.Context()
            result = no_handle_task(ctx)
            assert result.get("executed") is True

    def test_task_with_handle_param_is_callable(self):
        """A task with handle param should still be callable as a plain function."""
        import cloaca

        with cloaca.WorkflowBuilder("test_handle_param_detection") as _builder:
            @cloaca.task(id="handle_task")
            def handle_task(context, handle):
                context.set("handle_detected", True)
                return context

            assert callable(handle_task)
            # Direct call passes None for handle since there's no executor
            ctx = cloaca.Context()
            result = handle_task(ctx, None)
            assert result.get("handle_detected") is True

    def test_task_with_task_handle_param(self):
        """A task with task_handle param (alternate name) should be detected."""
        import cloaca

        with cloaca.WorkflowBuilder("test_task_handle_param_detection") as _builder:
            @cloaca.task(id="alt_handle_task")
            def alt_handle_task(context, task_handle):
                context.set("alt_handle_detected", True)
                return context

            assert callable(alt_handle_task)
            ctx = cloaca.Context()
            result = alt_handle_task(ctx, None)
            assert result.get("alt_handle_detected") is True


class TestTaskHandleClass:
    """Test that TaskHandle is importable and has expected attributes."""

    def test_task_handle_is_importable(self):
        """TaskHandle class should be importable from cloaca."""
        import cloaca
        assert hasattr(cloaca, "TaskHandle")

    def test_task_handle_has_defer_until(self):
        """TaskHandle should have a defer_until method."""
        import cloaca
        assert hasattr(cloaca.TaskHandle, "defer_until")

    def test_task_handle_has_is_slot_held(self):
        """TaskHandle should have an is_slot_held method."""
        import cloaca
        assert hasattr(cloaca.TaskHandle, "is_slot_held")


class TestTaskHandleExecution:
    """Test TaskHandle.defer_until through the executor pipeline."""

    def test_deferred_task_completes(self, shared_runner):
        """A task using defer_until should complete successfully."""
        import cloaca

        poll_count = {"value": 0}

        with cloaca.WorkflowBuilder("test_deferred_task_completes") as builder:
            builder.description("Pipeline with deferred task")

            @cloaca.task(id="deferred_task")
            def deferred_task(context, handle):
                def condition():
                    poll_count["value"] += 1
                    return poll_count["value"] >= 3

                handle.defer_until(condition, poll_interval_ms=50)
                context.set("deferred_complete", True)
                context.set("polls", poll_count["value"])
                return context

        result = shared_runner.execute(
            "test_deferred_task_completes",
            cloaca.Context(),
        )

        assert result.status == "Completed"
        assert result.final_context.get("deferred_complete") is True
        assert result.final_context.get("polls") >= 3

    def test_deferred_task_chains_with_downstream(self, shared_runner):
        """A deferred task should correctly chain with a downstream task."""
        import cloaca

        with cloaca.WorkflowBuilder("test_deferred_chain") as builder:
            builder.description("Deferred task chaining test")

            @cloaca.task(id="wait_task")
            def wait_task(context, handle):
                counter = {"n": 0}
                handle.defer_until(
                    lambda: (counter.update(n=counter["n"] + 1) or True)
                    if counter["n"] >= 2
                    else (counter.update(n=counter["n"] + 1) or False),
                    poll_interval_ms=50,
                )
                context.set("wait_done", True)
                return context

            @cloaca.task(id="process_task", dependencies=["wait_task"])
            def process_task(context):
                assert context.get("wait_done") is True
                context.set("processed", True)
                return context

        result = shared_runner.execute(
            "test_deferred_chain",
            cloaca.Context(),
        )

        assert result.status == "Completed"
        assert result.final_context.get("wait_done") is True
        assert result.final_context.get("processed") is True

    def test_non_handle_task_alongside_handle_task(self, shared_runner):
        """Normal tasks and handle tasks should work together in a workflow."""
        import cloaca

        with cloaca.WorkflowBuilder("test_mixed_tasks") as builder:
            builder.description("Mixed handle and non-handle tasks")

            @cloaca.task(id="normal_task")
            def normal_task(context):
                context.set("normal_done", True)
                return context

            @cloaca.task(id="deferred_task", dependencies=["normal_task"])
            def deferred_task(context, handle):
                handle.defer_until(lambda: True, poll_interval_ms=10)
                context.set("deferred_done", True)
                return context

        result = shared_runner.execute(
            "test_mixed_tasks",
            cloaca.Context(),
        )

        assert result.status == "Completed"
        assert result.final_context.get("normal_done") is True
        assert result.final_context.get("deferred_done") is True
