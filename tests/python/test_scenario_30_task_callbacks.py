"""
Test Task Callbacks

This test file verifies the Python bindings for on_success and on_failure
callbacks on the @task decorator.
"""


class TestTaskCallbacks:
    """Test task callback functionality."""

    def test_on_success_callback_called(self, shared_runner):
        """Test that on_success callback is called on successful task completion."""
        import cloaca

        # Track callback invocations
        callback_calls = []

        def track_success(task_id, context):
            callback_calls.append(("success", task_id))

        with cloaca.WorkflowBuilder("callback_success_workflow") as builder:
            builder.description("Test on_success callback")

            @cloaca.task(id="success_task", on_success=track_success)
            def success_task(context):
                context.set("result", "done")
                return context

        context = cloaca.Context({"input": "test"})
        result = shared_runner.execute("callback_success_workflow", context)

        assert result.status == "Completed"
        assert ("success", "success_task") in callback_calls
        print("on_success callback was called correctly")

    def test_on_failure_callback_called(self, shared_runner):
        """Test that on_failure callback is called on task failure."""
        import cloaca

        # Track callback invocations
        callback_calls = []

        def track_failure(task_id, error, context):
            callback_calls.append(("failure", task_id, error))

        with cloaca.WorkflowBuilder("callback_failure_workflow") as builder:
            builder.description("Test on_failure callback")

            @cloaca.task(
                id="failing_task",
                on_failure=track_failure,
                retry_attempts=0  # No retries for faster test
            )
            def failing_task(context):
                raise ValueError("Intentional failure")

        context = cloaca.Context({"input": "test"})
        shared_runner.execute("callback_failure_workflow", context)

        # Check that failure callback was called (main purpose of this test)
        assert len([c for c in callback_calls if c[0] == "failure"]) > 0
        # Verify the callback received the task ID
        failure_callbacks = [c for c in callback_calls if c[0] == "failure"]
        assert failure_callbacks[0][1] == "failing_task"
        # Verify the error message contains our intentional failure
        assert "Intentional failure" in failure_callbacks[0][2]
        print("on_failure callback was called correctly")

    def test_both_callbacks_on_same_task(self, shared_runner):
        """Test that both callbacks can be set on the same task."""
        import cloaca

        callback_calls = []

        def on_success(task_id, context):
            callback_calls.append(("success", task_id))

        def on_failure(task_id, error, context):
            callback_calls.append(("failure", task_id))

        with cloaca.WorkflowBuilder("dual_callback_workflow") as builder:
            builder.description("Test both callbacks")

            @cloaca.task(
                id="dual_callback_task",
                on_success=on_success,
                on_failure=on_failure
            )
            def dual_callback_task(context):
                context.set("result", "success")
                return context

        context = cloaca.Context({})
        result = shared_runner.execute("dual_callback_workflow", context)

        assert result.status == "Completed"
        assert ("success", "dual_callback_task") in callback_calls
        print("Both callbacks can be set on the same task")

    def test_callback_error_isolation(self, shared_runner):
        """Test that errors in callbacks don't fail the task."""
        import cloaca

        def buggy_callback(task_id, context):
            raise Exception("Callback error!")

        with cloaca.WorkflowBuilder("isolated_callback_workflow") as builder:
            builder.description("Test callback error isolation")

            @cloaca.task(id="isolated_task", on_success=buggy_callback)
            def isolated_task(context):
                context.set("result", "done")
                return context

        context = cloaca.Context({})
        result = shared_runner.execute("isolated_callback_workflow", context)

        # Task should still complete successfully despite callback error
        assert result.status == "Completed"
        print("Callback errors are isolated from task execution")

    def test_callback_receives_correct_context(self, shared_runner):
        """Test that callbacks receive the correct context data."""
        import cloaca

        received_context = {}

        def capture_context(task_id, context):
            received_context["task_id"] = task_id
            received_context["result"] = context.get("computed_value")

        with cloaca.WorkflowBuilder("context_callback_workflow") as builder:
            builder.description("Test callback context")

            @cloaca.task(id="compute_task", on_success=capture_context)
            def compute_task(context):
                context.set("computed_value", 42)
                return context

        context = cloaca.Context({"input": 10})
        result = shared_runner.execute("context_callback_workflow", context)

        assert result.status == "Completed"
        assert received_context.get("task_id") == "compute_task"
        assert received_context.get("result") == 42
        print("Callback receives correct context data")

    def test_callbacks_with_dependencies(self, shared_runner):
        """Test callbacks work correctly with task dependencies."""
        import cloaca

        callback_order = []

        def track_order(task_id, context):
            callback_order.append(task_id)

        with cloaca.WorkflowBuilder("dep_callback_workflow") as builder:
            builder.description("Test callbacks with dependencies")

            @cloaca.task(id="first_task", on_success=track_order)
            def first_task(context):
                context.set("step", 1)
                return context

            @cloaca.task(
                id="second_task",
                dependencies=["first_task"],
                on_success=track_order
            )
            def second_task(context):
                context.set("step", 2)
                return context

        context = cloaca.Context({})
        result = shared_runner.execute("dep_callback_workflow", context)

        assert result.status == "Completed"
        assert callback_order == ["first_task", "second_task"]
        print("Callbacks work correctly with task dependencies")
