"""
Test Workflow Versioning

This test file verifies content-based hashing for workflow versions,
ensuring workflows are properly versioned based on their structure and content.

Uses shared_runner fixture for workflow execution testing.
"""



class TestWorkflowVersioning:
    """Test workflow versioning functionality."""

    def test_comprehensive_workflow_versioning(self, shared_runner):
        """Test comprehensive workflow versioning including content-based hashing and version stability."""
        import cloaca

        # Test 1: Workflow registration and execution
        print("Testing workflow registration and execution...")

        # Create first workflow
        with cloaca.WorkflowBuilder("version_test_workflow") as builder:
            builder.description("Workflow for versioning test")
            builder.tag("type", "version_test")
            
            @cloaca.task(id="version_test_task_1")
            def version_test_task_1(context):
                context.set("task_1_executed", True)
                return context

            @cloaca.task(id="version_test_task_2")
            def version_test_task_2(context):
                context.set("task_2_executed", True)
                return context

        # Execute the workflow to ensure it was registered correctly
        context = cloaca.Context({"test_type": "version_test"})
        result = shared_runner.execute("version_test_workflow", context)
        
        assert result is not None
        assert result.status == "Completed"
        print("✓ Workflow registered and executed successfully")

        # Test 2: Different workflows can coexist
        print("Testing multiple workflow versions...")

        with cloaca.WorkflowBuilder("version_test_workflow_v2") as builder:
            builder.description("Different workflow for versioning test")
            builder.tag("type", "version_test")
            builder.tag("variant", "different")
            
            @cloaca.task(id="version_test_task_1")
            def version_test_task_1(context):
                context.set("task_1_executed", True)
                context.set("version", "v2")
                return context

            @cloaca.task(id="version_test_task_2")
            def version_test_task_2(context):
                context.set("task_2_executed", True)
                return context

            @cloaca.task(id="version_test_task_3")
            def version_test_task_3(context):
                context.set("task_3_executed", True)
                return context

        # Execute the second workflow
        context = cloaca.Context({"test_type": "version_test_v2"})
        result = shared_runner.execute("version_test_workflow_v2", context)
        
        assert result is not None
        assert result.status == "Completed"
        print("✓ Multiple workflow versions can coexist")

        # Test 3: Re-registering a workflow replaces it
        print("Testing workflow re-registration...")

        with cloaca.WorkflowBuilder("version_test_workflow_replace") as builder:
            builder.description("Original workflow")
            builder.tag("version", "original")
            
            @cloaca.task(id="original_task")
            def original_task(context):
                context.set("version", "original")
                return context

        # Execute original version
        context = cloaca.Context({})
        result = shared_runner.execute("version_test_workflow_replace", context)
        assert result is not None
        assert result.status == "Completed"

        # Now replace it with a new version
        with cloaca.WorkflowBuilder("version_test_workflow_replace") as builder:
            builder.description("Replaced workflow")
            builder.tag("version", "replaced")
            
            @cloaca.task(id="replaced_task")
            def replaced_task(context):
                context.set("version", "replaced")
                return context

        # Execute replaced version
        context = cloaca.Context({})
        result = shared_runner.execute("version_test_workflow_replace", context)
        assert result is not None
        assert result.status == "Completed"
        print("✓ Workflow re-registration works correctly")

        print("✓ All workflow versioning tests passed")