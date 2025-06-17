"""
Test Complex Dependency Chains

This test file verifies complex workflow dependency patterns including
diamond patterns, fan-out/fan-in, and multi-level dependency chains.

Uses shared_runner fixture with global registry for task persistence.
"""



class TestComplexDependencyChains:
    """Test complex dependency chain patterns."""

    def test_comprehensive_complex_dependency_patterns(self, shared_runner):
        """Test comprehensive complex dependency chain patterns including diamond, fan-out, fan-in, and multi-level chains."""
        import os
        os.environ['RUST_LOG'] = 'cloacina=debug,cloaca_backend=debug'
        import cloaca

        # Test 1: Diamond dependency pattern
        print("Testing diamond dependency pattern...")

        with cloaca.WorkflowBuilder("diamond_dependency_workflow") as builder:
            builder.description("Diamond dependency pattern test")
            
            @cloaca.task(id="diamond_start")
            def diamond_start(context):
                context.set("diamond_start_executed", True)
                order = context.get("execution_order", [])
                order.append("diamond_start")
                context.set("execution_order", order)
                return context

            @cloaca.task(id="diamond_left", dependencies=["diamond_start"])
            def diamond_left(context):
                context.set("diamond_left_executed", True)
                order = context.get("execution_order", [])
                order.append("diamond_left")
                context.set("execution_order", order)
                return context

            @cloaca.task(id="diamond_right", dependencies=["diamond_start"])
            def diamond_right(context):
                context.set("diamond_right_executed", True)
                order = context.get("execution_order", [])
                order.append("diamond_right")
                context.set("execution_order", order)
                return context

            @cloaca.task(id="diamond_end", dependencies=["diamond_left", "diamond_right"])
            def diamond_end(context):
                context.set("diamond_end_executed", True)
                order = context.get("execution_order", [])
                order.append("diamond_end")
                context.set("execution_order", order)
                return context

        context = cloaca.Context({"test_type": "diamond"})
        result = shared_runner.execute("diamond_dependency_workflow", context)

        assert result is not None
        assert result.status == "Completed"
        execution_order = result.final_context.get("execution_order", [])
        assert "diamond_start" in execution_order
        assert "diamond_left" in execution_order
        assert "diamond_right" in execution_order
        assert "diamond_end" in execution_order
        assert execution_order.index("diamond_start") < execution_order.index("diamond_left")
        assert execution_order.index("diamond_start") < execution_order.index("diamond_right")
        assert execution_order.index("diamond_left") < execution_order.index("diamond_end")
        assert execution_order.index("diamond_right") < execution_order.index("diamond_end")
        print("✓ Diamond dependency pattern works correctly")

        # Test 2: Fan-out pattern (one task triggering many)
        print("Testing fan-out dependency pattern...")

        with cloaca.WorkflowBuilder("fanout_dependency_workflow") as builder:
            builder.description("Fan-out dependency pattern test")
            
            @cloaca.task(id="fanout_trigger")
            def fanout_trigger(context):
                context.set("fanout_trigger_executed", True)
                context.set("fanout_count", 0)
                return context

            @cloaca.task(id="fanout_task_1", dependencies=["fanout_trigger"])
            def fanout_task_1(context):
                context.set("fanout_task_1_completed", True)
                return context

            @cloaca.task(id="fanout_task_2", dependencies=["fanout_trigger"])
            def fanout_task_2(context):
                context.set("fanout_task_2_completed", True)
                return context

            @cloaca.task(id="fanout_task_3", dependencies=["fanout_trigger"])
            def fanout_task_3(context):
                context.set("fanout_task_3_completed", True)
                return context

            @cloaca.task(id="fanout_task_4", dependencies=["fanout_trigger"])
            def fanout_task_4(context):
                context.set("fanout_task_4_completed", True)
                return context

            @cloaca.task(id="fanout_collector", dependencies=["fanout_task_1", "fanout_task_2", "fanout_task_3", "fanout_task_4"])
            def fanout_collector(context):
                context.set("fanout_collector_completed", True)
                # Count completed tasks
                completed_count = sum([
                    1 if context.get("fanout_task_1_completed") else 0,
                    1 if context.get("fanout_task_2_completed") else 0,
                    1 if context.get("fanout_task_3_completed") else 0,
                    1 if context.get("fanout_task_4_completed") else 0,
                ])
                context.set("fanout_completed_count", completed_count)
                return context

        context = cloaca.Context({"test_type": "fanout"})
        result = shared_runner.execute("fanout_dependency_workflow", context)

        assert result is not None
        assert result.status == "Completed"
        assert result.final_context.get("fanout_trigger_executed") is True
        assert result.final_context.get("fanout_task_1_completed") is True
        assert result.final_context.get("fanout_task_2_completed") is True
        assert result.final_context.get("fanout_task_3_completed") is True
        assert result.final_context.get("fanout_task_4_completed") is True
        assert result.final_context.get("fanout_collector_completed") is True
        assert result.final_context.get("fanout_completed_count") == 4
        print("✓ Fan-out dependency pattern works correctly")

        # Test 3: Fan-in pattern (many tasks converging to one)
        print("Testing fan-in dependency pattern...")

        with cloaca.WorkflowBuilder("fanin_dependency_workflow") as builder:
            builder.description("Fan-in dependency pattern test")
            
            @cloaca.task(id="fanin_source_1")
            def fanin_source_1(context):
                context.set("fanin_source_1_executed", True)
                sources = context.get("fanin_sources", [])
                sources.append("source_1")
                context.set("fanin_sources", sources)
                return context

            @cloaca.task(id="fanin_source_2")
            def fanin_source_2(context):
                context.set("fanin_source_2_executed", True)
                sources = context.get("fanin_sources", [])
                sources.append("source_2")
                context.set("fanin_sources", sources)
                return context

            @cloaca.task(id="fanin_source_3")
            def fanin_source_3(context):
                context.set("fanin_source_3_executed", True)
                sources = context.get("fanin_sources", [])
                sources.append("source_3")
                context.set("fanin_sources", sources)
                return context

            @cloaca.task(id="fanin_collector", dependencies=["fanin_source_1", "fanin_source_2", "fanin_source_3"])
            def fanin_collector(context):
                context.set("fanin_collector_executed", True)
                sources = context.get("fanin_sources", [])
                context.set("collected_count", len(sources))
                return context

        context = cloaca.Context({"test_type": "fanin"})
        result = shared_runner.execute("fanin_dependency_workflow", context)

        assert result is not None
        assert result.status == "Completed"
        assert result.final_context.get("fanin_source_1_executed") is True
        assert result.final_context.get("fanin_source_2_executed") is True
        assert result.final_context.get("fanin_source_3_executed") is True
        assert result.final_context.get("fanin_collector_executed") is True
        assert result.final_context.get("collected_count") == 3
        print("✓ Fan-in dependency pattern works correctly")

        # Test 4: Multi-level dependency chains
        print("Testing multi-level dependency chains...")

        with cloaca.WorkflowBuilder("chain_dependency_workflow") as builder:
            builder.description("Multi-level dependency chain test")
            
            @cloaca.task(id="chain_level_1")
            def chain_level_1(context):
                context.set("chain_level_1_executed", True)
                context.set("chain_depth", 1)
                return context

            @cloaca.task(id="chain_level_2", dependencies=["chain_level_1"])
            def chain_level_2(context):
                context.set("chain_level_2_executed", True)
                context.set("chain_depth", 2)
                return context

            @cloaca.task(id="chain_level_3", dependencies=["chain_level_2"])
            def chain_level_3(context):
                context.set("chain_level_3_executed", True)
                context.set("chain_depth", 3)
                return context

            @cloaca.task(id="chain_level_4", dependencies=["chain_level_3"])
            def chain_level_4(context):
                context.set("chain_level_4_executed", True)
                context.set("chain_depth", 4)
                return context

            @cloaca.task(id="chain_level_5", dependencies=["chain_level_4"])
            def chain_level_5(context):
                context.set("chain_level_5_executed", True)
                context.set("chain_depth", 5)
                return context

        context = cloaca.Context({"test_type": "chain"})
        result = shared_runner.execute("chain_dependency_workflow", context)

        assert result is not None
        assert result.status == "Completed"
        assert result.final_context.get("chain_level_1_executed") is True
        assert result.final_context.get("chain_level_2_executed") is True
        assert result.final_context.get("chain_level_3_executed") is True
        assert result.final_context.get("chain_level_4_executed") is True
        assert result.final_context.get("chain_level_5_executed") is True
        assert result.final_context.get("chain_depth") == 5
        print("✓ Multi-level dependency chain works correctly")

        # Test 5: Complex mixed pattern (diamond + fan-out + fan-in)
        print("Testing complex mixed dependency patterns...")

        with cloaca.WorkflowBuilder("mixed_dependency_workflow") as builder:
            builder.description("Complex mixed dependency pattern test")
            
            @cloaca.task(id="mixed_start")
            def mixed_start(context):
                context.set("mixed_start_executed", True)
                return context

            @cloaca.task(id="mixed_branch_a", dependencies=["mixed_start"])
            def mixed_branch_a(context):
                context.set("mixed_branch_a_executed", True)
                return context

            @cloaca.task(id="mixed_branch_b", dependencies=["mixed_start"])
            def mixed_branch_b(context):
                context.set("mixed_branch_b_executed", True)
                return context

            @cloaca.task(id="mixed_fanout_1", dependencies=["mixed_branch_a"])
            def mixed_fanout_1(context):
                context.set("mixed_fanout_1_executed", True)
                return context

            @cloaca.task(id="mixed_fanout_2", dependencies=["mixed_branch_a"])
            def mixed_fanout_2(context):
                context.set("mixed_fanout_2_executed", True)
                return context

            @cloaca.task(id="mixed_fanout_3", dependencies=["mixed_branch_b"])
            def mixed_fanout_3(context):
                context.set("mixed_fanout_3_executed", True)
                return context

            @cloaca.task(id="mixed_end", dependencies=["mixed_fanout_1", "mixed_fanout_2", "mixed_fanout_3"])
            def mixed_end(context):
                context.set("mixed_end_executed", True)
                return context

        context = cloaca.Context({"test_type": "mixed"})
        result = shared_runner.execute("mixed_dependency_workflow", context)

        assert result is not None
        assert result.status == "Completed"
        assert result.final_context.get("mixed_start_executed") is True
        assert result.final_context.get("mixed_branch_a_executed") is True
        assert result.final_context.get("mixed_branch_b_executed") is True
        assert result.final_context.get("mixed_fanout_1_executed") is True
        assert result.final_context.get("mixed_fanout_2_executed") is True
        assert result.final_context.get("mixed_fanout_3_executed") is True
        assert result.final_context.get("mixed_end_executed") is True
        print("✓ Complex mixed dependency pattern works correctly")

        # Summary
        patterns_tested = 5
        print(f"\nComplex dependency patterns tested: {patterns_tested}/5")
        print("✓ All complex dependency chain patterns work correctly")

        print("✓ Comprehensive complex dependency chains test completed")
