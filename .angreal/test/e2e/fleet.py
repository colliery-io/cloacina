"""End-to-end execution-agent fleet test (CLOACI-I-0114 / T-0634).

The live multi-agent proof for I-0114: boots the FULL pipeline — Postgres,
`cloacina-server` (with every task sent to the "fleet" executor),
`cloacina-compiler`, and a real `cloacina-agent` — compiles a source workflow,
runs it, and asserts it executes on the agent (not the in-process thread
executor) and reconciles to Completed. This proves the substrate -> agent ->
reconcile loop closes end to end against real Postgres.

Why the compiler is in the loop: workflow packages are uploaded as *source*
(the test fixtures are a few hundred bytes), so the compiler service must build
the cdylib and store it before the reconciler will register the workflow and
the agent can fetch + dlopen the artifact.

The stack comes from `_fleet_stack.py`. The agent gets a key scoped to the
tenant of the work (CLOACINA-T-1017): an agent takes its tenant from its key,
and an agent on the bootstrap admin key has no tenant and gets no work.

Requires Docker. Run with: `angreal test e2e fleet`.
"""

import angreal  # type: ignore

from .._utils import print_final_success, print_section_header
from ._fleet_stack import FleetStack, build_binaries
from .compiler import _poll_execution_status, _poll_run_workflow

test = angreal.command_group(
    name="test", about="Cloacina test suites (unit, integration, e2e, soak)"
)
e2e = angreal.command_group(name="e2e", about="end-to-end tests against a live server")

# Source fixture that the compiler builds; the workflow it defines is named
# `compiler_happy_workflow` (same as the compiler e2e happy path).
FIXTURE_SRC = "compiler-happy-rust"
WORKFLOW_NAME = "compiler_happy_workflow"

# Kept for callers that import it from here.
_build_binaries = build_binaries


@test()
@e2e()
@angreal.command(
    name="fleet",
    about="end-to-end execution-agent fleet test (CLOACI-T-0634)",
    when_to_use=[
        "validating the DB-less agent fleet against a live server",
        "confirming substrate -> agent -> reconcile closes end to end",
    ],
    when_not_to_use=["unit testing", "running without docker"],
)
def fleet():
    print_section_header("cloacina fleet e2e")
    with FleetStack("fleet-e2e", 18083, 19003) as stack:
        stack.compile_fixture(FIXTURE_SRC)
        # Start the agent after the artifact exists to fetch.
        stack.start_agent(tenant="public", max_concurrency=2)

        # --- run the workflow → must execute on the agent → Completed ------
        execution_id = _poll_run_workflow(stack.home, WORKFLOW_NAME, timeout_s=120.0)
        print(f"  triggered execution: {execution_id}")
        status = _poll_execution_status(
            stack.home, execution_id, {"Completed", "Failed", "Cancelled"}, timeout_s=120.0
        )
        if status != "Completed":
            raise AssertionError(
                f"fleet execution ended in {status!r}, expected Completed. "
                f"A task sent to the fleet must run on the agent and reconcile."
            )
        print(f"  ok: fleet execution {status}")

        # --- prove it went through the agent, not the thread executor ------
        # "Completed" alone is NOT proof of fleet execution. The proof is the
        # server-side `report_result` INFO log "agent reported result": it
        # fires ONLY when an agent posts the outcome of a work packet.
        server_text = stack.log_text("server.log")
        if "agent reported result" not in server_text:
            raise AssertionError(
                "execution Completed but the server never logged 'agent reported "
                "result' — the task ran on the thread executor, NOT the fleet."
            )
        if (
            'task_name=public::compiler-happy-rust::compiler_happy_workflow::noop executor="default"'
            in server_text
        ):
            raise AssertionError(
                'the workflow task dispatched to executor="default". Fleet path NOT exercised.'
            )
        tasks = stack.tasks(execution_id)
        if [t["attempt"] for t in tasks] != [1]:
            raise AssertionError(f"expected one task on attempt 1, got {tasks}")
        print("  ok: the agent executed and reported the work on attempt 1 (fleet path)")

    print_final_success("cloacina fleet e2e")
