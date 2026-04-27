"""
Scenario 32: @cloaca.task(invokes=GraphHandle, post_invocation=fn)

Covers Python parity for the Rust split-CG / trigger-less CG flow landed in
T-0540 + T-0541. A workflow task names a trigger-less computation graph via
`invokes=`; after the user body runs, the graph executes, its terminal
outputs are routed back into the task context under their node names, and an
optional `post_invocation` callback runs before `on_success`.

These scenarios exercise the runner end-to-end (sqlite or postgres backend
selected by `CLOACA_BACKEND`).
"""

import pytest


# ---------------------------------------------------------------------------
# Happy path: trigger-less CG invoked by a workflow task
# ---------------------------------------------------------------------------


class TestTaskInvokesTriggerLessGraph:
    """Trigger-less CG invocation routes terminal outputs into the task context."""

    def test_single_terminal_routes_into_context(self, shared_runner):
        """One terminal node — its return value lands in `final_context["score"]`."""
        import cloaca

        with cloaca.WorkflowBuilder("s32_single_terminal_wf") as builder:
            builder.description("Trigger-less CG")

            score_graph = cloaca.ComputationGraphBuilder(
                "s32_score_graph_single",
                graph={"score": {}},
            )
            with score_graph:

                @cloaca.node
                def score(ctx):
                    return {"score": 99}

            @cloaca.task(id="s32_run_score", invokes=score_graph)
            def run_score(ctx):
                return ctx

        result = shared_runner.execute("s32_single_terminal_wf", cloaca.Context())

        assert result.status == "Completed", f"status={result.status}"
        ctx = result.final_context
        assert "score" in ctx, f"terminal 'score' missing from context: keys={list(ctx)}"
        assert ctx.get("score") == {"score": 99}

    def test_multiple_terminals_route_under_their_names(self, shared_runner):
        """A linear-into-fanout topology produces two terminals; both must land."""
        import cloaca

        with cloaca.WorkflowBuilder("s32_multi_terminal_wf") as builder:
            builder.description("Trigger-less CG: multiple terminals")

            multi_graph = cloaca.ComputationGraphBuilder(
                "s32_multi_graph",
                graph={
                    "decision": {
                        "routes": {
                            "Hit": "hit",
                            "Miss": "miss",
                        }
                    },
                    "hit": {},
                    "miss": {},
                },
            )
            with multi_graph:

                @cloaca.node
                def decision(ctx):
                    # Always pick the Hit branch in this test.
                    return ("Hit", {"value": 7})

                @cloaca.node
                def hit(payload):
                    return {"hit_value": payload["value"]}

                @cloaca.node
                def miss(payload):
                    return {"missed": True}

            @cloaca.task(id="s32_router", invokes=multi_graph)
            def router(ctx):
                return ctx

        result = shared_runner.execute("s32_multi_terminal_wf", cloaca.Context())

        assert result.status == "Completed", f"status={result.status}"
        ctx = result.final_context
        # The "Hit" branch fired, so its terminal lands and the "miss" branch is skipped.
        assert ctx.get("hit") == {"hit_value": 7}
        assert "miss" not in ctx, f"non-selected branch should not produce a terminal: {list(ctx)}"


class TestPostInvocationHook:
    """post_invocation runs after CG terminals route, before on_success."""

    def test_post_invocation_can_mutate_context(self, shared_runner):
        import cloaca

        with cloaca.WorkflowBuilder("s32_post_wf"):
            graph = cloaca.ComputationGraphBuilder(
                "s32_post_graph",
                graph={"emit": {}},
            )
            with graph:

                @cloaca.node
                def emit(ctx):
                    return {"emitted": True}

            def post(ctx):
                # ctx already contains the terminal "emit" key.
                assert ctx.get("emit") == {"emitted": True}
                ctx.set("post_ran", True)
                return ctx

            @cloaca.task(id="s32_post_task", invokes=graph, post_invocation=post)
            def post_task(ctx):
                return ctx

        result = shared_runner.execute("s32_post_wf", cloaca.Context())

        assert result.status == "Completed", f"status={result.status}"
        ctx = result.final_context
        assert ctx.get("emit") == {"emitted": True}
        assert ctx.get("post_ran") is True


# ---------------------------------------------------------------------------
# Decoration-time validation — these errors fire before the runner is involved
# ---------------------------------------------------------------------------


class TestDecorationTimeValidation:
    def test_post_invocation_without_invokes_errors(self):
        import cloaca

        with cloaca.WorkflowBuilder("s32_post_without_invokes_wf"):
            # Workflow needs at least one task or `__exit__` rejects "Workflow cannot be empty"
            @cloaca.task(id="s32_post_without_invokes_filler")
            def _filler(ctx):
                return ctx

            def post(ctx):
                return ctx

            with pytest.raises(ValueError) as exc:

                @cloaca.task(id="s32_bad_post", post_invocation=post)
                def bad(ctx):
                    return ctx

            assert "post_invocation" in str(exc.value)

    def test_invokes_unregistered_graph_errors(self):
        """A graph builder whose `with` block hasn't run yet has no executor."""
        import cloaca

        with cloaca.WorkflowBuilder("s32_unregistered_graph_wf"):
            @cloaca.task(id="s32_unregistered_filler")
            def _filler(ctx):
                return ctx

            unbuilt = cloaca.ComputationGraphBuilder(
                "s32_unbuilt_graph",
                graph={"x": {}},
            )
            # Notice: no `with unbuilt: ...` block, so the executor is never registered.

            with pytest.raises(ValueError) as exc:

                @cloaca.task(id="s32_bad_invokes", invokes=unbuilt)
                def bad(ctx):
                    return ctx

            assert "not registered" in str(exc.value)

    def test_invokes_reactor_triggered_graph_errors(self):
        """invokes= a reactor-triggered graph should be rejected."""
        import cloaca

        @cloaca.reactor(
            name="s32_reactor_for_invokes",
            accumulators=["alpha"],
            mode="when_any",
        )
        class _S32ReactorForInvokes:
            pass

        reactor_graph = cloaca.ComputationGraphBuilder(
            "s32_reactor_graph_for_invokes",
            reactor=_S32ReactorForInvokes,
            graph={"e": {"inputs": ["alpha"]}},
        )
        with reactor_graph:

            @cloaca.node
            def e(alpha):
                return {"x": 1}

        with cloaca.WorkflowBuilder("s32_reactor_invokes_wf"):
            @cloaca.task(id="s32_reactor_invokes_filler")
            def _filler(ctx):
                return ctx

            with pytest.raises(ValueError) as exc:

                @cloaca.task(id="s32_bad_reactor_invokes", invokes=reactor_graph)
                def bad(ctx):
                    return ctx

            assert "reactor-triggered" in str(exc.value)


# ---------------------------------------------------------------------------
# Error path — graph-side failure surfaces as a TaskError
# ---------------------------------------------------------------------------


class TestGraphFailurePropagatesAsTaskError:
    def test_graph_node_exception_fails_the_task(self, shared_runner):
        import cloaca

        with cloaca.WorkflowBuilder("s32_graph_failure_wf"):
            graph = cloaca.ComputationGraphBuilder(
                "s32_graph_that_raises",
                graph={"boom": {}},
            )
            with graph:

                @cloaca.node
                def boom(ctx):
                    raise RuntimeError("intentional graph failure")

            @cloaca.task(id="s32_raiser", invokes=graph)
            def raiser(ctx):
                return ctx

        result = shared_runner.execute("s32_graph_failure_wf", cloaca.Context())

        # The task should have failed, not silently completed.
        assert result.status != "Completed", (
            f"expected task to fail when graph raises, got status={result.status}"
        )
