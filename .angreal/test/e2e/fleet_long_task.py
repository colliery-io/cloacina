"""End-to-end fleet tests for work that outlasts the easy path.

`fleet-long-task` (CLOACINA-T-1011): a task that runs longer than the delivery
sweeper's stuck threshold must run one time. The agent acks a work packet only
after the task ends, and the sweeper pushed an unacked row again after 60 s.
The fixture appends a line to a marker file at each start and each end of the
task body; the test counts them.

`fleet-wait-for-agent` (CLOACINA-T-1012): a task with no agent in its tenant
must wait, not fail. It must run on its first attempt when an agent registers.

`fleet-full-agent` (CLOACINA-T-1043): a packet that reaches an agent with no
free slot is refused. The task must wait and then run on its first attempt.

All use the stack of `_fleet_stack.py`. They need Docker.
"""

import re
import time
from pathlib import Path

import angreal  # type: ignore

from .._utils import print_final_success, print_section_header
from ._fleet_stack import FleetStack
from .compiler import _poll_execution_status, _poll_run_workflow

test = angreal.command_group(
    name="test", about="Cloacina test suites (unit, integration, e2e, soak)"
)
e2e = angreal.command_group(name="e2e", about="end-to-end tests against a live server")

LONG_FIXTURE = "fleet-long-task-rust"
LONG_WORKFLOW = "fleet_long_task_workflow"
# Longer than the sweeper's 60 s threshold plus one 30 s sweep, shorter than
# the 300 s fleet result timeout.
TASK_SECONDS = 150

HAPPY_FIXTURE = "compiler-happy-rust"
HAPPY_WORKFLOW = "compiler_happy_workflow"
# Before CLOACINA-T-1012 a task with no agent failed after about 4 s.
NO_AGENT_SECONDS = 15
# A second tenant: its agent has free capacity and can run no task of `public`.
OTHER_TENANT = "fleet_wait_other"

# Long enough for several refused dispatch tries (one each 2 s), short enough
# for a pull request.
FULL_AGENT_TASK_SECONDS = 20
# The server learns that an agent is full from the agent's heartbeat. With a
# long interval the roster still shows a free slot while the first task runs,
# so the server sends the second packet and the agent refuses it.
FULL_AGENT_HEARTBEAT_SECONDS = 60

TERMINAL = {"Completed", "Failed", "Cancelled"}


def _marker_lines(marker: Path) -> list[str]:
    if not marker.exists():
        return []
    return [line for line in marker.read_text().splitlines() if line.strip()]


@test()
@e2e()
@angreal.command(
    name="fleet-long-task",
    about="fleet e2e: a 150 s task must run one time",
    when_to_use=[
        "checking that a task longer than the delivery stuck threshold is not pushed again",
    ],
    when_not_to_use=["unit testing", "running without docker"],
)
def fleet_long_task():
    print_section_header("cloacina fleet long-task e2e")
    with FleetStack("fleet-long-task-e2e", 18084, 19004) as stack:
        marker = stack.home / "long-task-marker.txt"
        stack.compile_fixture(LONG_FIXTURE)
        # Capacity 2, so a second push of the same packet would be accepted and
        # run, not refused as "saturated".
        agent_log = stack.start_agent(
            max_concurrency=2,
            env={
                "FLEET_LONG_TASK_MARKER": str(marker),
                "FLEET_LONG_TASK_SECONDS": str(TASK_SECONDS),
            },
        )

        execution_id = _poll_run_workflow(stack.home, LONG_WORKFLOW, timeout_s=120.0)
        started_at = time.time()
        print(f"  triggered execution: {execution_id} (task runs {TASK_SECONDS} s)")
        status = _poll_execution_status(stack.home, execution_id, TERMINAL, timeout_s=900.0)
        print(f"  execution {status} after {time.time() - started_at:.0f} s")
        # Let a second copy, if one started late, write its start line.
        time.sleep(5.0)

        lines = _marker_lines(marker)
        starts = [line for line in lines if line.startswith("start ")]
        ends = [line for line in lines if line.startswith("end ")]
        server_text = stack.log_text("server.log")
        reported = server_text.count("agent reported result")
        print("  marker file:")
        for line in lines:
            print(f"    {line}")
        print(
            f"  task starts: {len(starts)}, task ends: {len(ends)}, "
            f"results reported to the server: {reported}"
        )

        if len(starts) == 0:
            raise AssertionError(
                "the task body never wrote a start line: it did not run on the agent"
            )
        if len(starts) != 1:
            raise AssertionError(
                f"the task body started {len(starts)} times for one execution "
                f"(status {status!r}). A {TASK_SECONDS} s task must run one time."
            )
        if status != "Completed":
            raise AssertionError(f"execution ended in {status!r}, expected Completed")
        if len(ends) != 1:
            raise AssertionError(f"the task body ended {len(ends)} times, expected 1")

        # The agent's heartbeat renews the lease of the packet it works on, so
        # the delivery sweeper must not have reset the row for a re-push.
        resets = re.findall(r"delivery sweep complete reset=([1-9][0-9]*)", server_text)
        if resets:
            raise AssertionError(
                f"the delivery sweeper reset a row {len(resets)} time(s) while its "
                "agent was alive and working on it"
            )
        if "refusing packet" in stack.log_text(agent_log.name):
            raise AssertionError("the agent refused a packet; no second push was expected")
        print("  ok: one start, one end, no re-push, no refusal")

    print_final_success("cloacina fleet long-task e2e")


@test()
@e2e()
@angreal.command(
    name="fleet-wait-for-agent",
    about="fleet e2e: a task with no agent waits, then runs on its first attempt",
    when_to_use=[
        "checking that a wait for agent capacity does not use retry attempts",
    ],
    when_not_to_use=["unit testing", "running without docker"],
)
def fleet_wait_for_agent():
    print_section_header("cloacina fleet wait-for-agent e2e")
    with FleetStack("fleet-wait-for-agent-e2e", 18085, 19005) as stack:
        stack.compile_fixture(HAPPY_FIXTURE)

        # One agent with free capacity, but of another tenant: the dispatcher's
        # capacity gate opens, and selection finds no agent for the task. (With
        # no agent at all the gate stays shut and nothing is dispatched.)
        stack.create_tenant(OTHER_TENANT)
        stack.start_agent(tenant=OTHER_TENANT)

        # No agent of the task's tenant yet. The task must wait.
        execution_id = _poll_run_workflow(stack.home, HAPPY_WORKFLOW, timeout_s=120.0)
        print(f"  triggered execution: {execution_id} with no agent of its tenant; waiting {NO_AGENT_SECONDS} s")
        time.sleep(NO_AGENT_SECONDS)
        tasks = stack.tasks(execution_id)
        print(f"  tasks with no agent: {[(t['status'], t['attempt']) for t in tasks]}")
        if len(tasks) != 1:
            raise AssertionError(f"expected 1 task, got {tasks}")
        if tasks[0]["status"] in ("Failed", "Completed") or tasks[0]["attempt"] != 1:
            raise AssertionError(
                f"with no agent the task must wait on attempt 1; got status "
                f"{tasks[0]['status']!r}, attempt {tasks[0]['attempt']}"
            )
        deferred = stack.log_text("server.log").count("no capacity now")
        if deferred < 2:
            raise AssertionError(
                f"expected several deferred dispatch tries in {NO_AGENT_SECONDS} s, saw {deferred}"
            )
        print(f"  ok: the task waits on attempt 1 ({deferred} deferred dispatch tries)")

        # An agent of the tenant registers. The task must run on attempt 1.
        stack.start_agent()
        status = _poll_execution_status(stack.home, execution_id, TERMINAL, timeout_s=120.0)
        tasks = stack.tasks(execution_id)
        print(f"  execution {status}; tasks: {[(t['status'], t['attempt']) for t in tasks]}")
        if status != "Completed":
            raise AssertionError(f"execution ended in {status!r}, expected Completed")
        if tasks[0]["attempt"] != 1:
            raise AssertionError(f"the wait used retry attempts: attempt {tasks[0]['attempt']}")
        if "agent reported result" not in stack.log_text("server.log"):
            raise AssertionError("the task did not run on the agent")
        print("  ok: the task ran on the agent on attempt 1")

    print_final_success("cloacina fleet wait-for-agent e2e")


@test()
@e2e()
@angreal.command(
    name="fleet-full-agent",
    about="fleet e2e: a task refused by a full agent waits, then runs on its first attempt",
    when_to_use=[
        "checking that a refusal for capacity puts the task back in the queue",
    ],
    when_not_to_use=["unit testing", "running without docker"],
)
def fleet_full_agent():
    print_section_header("cloacina fleet full-agent e2e")
    with FleetStack(
        "fleet-full-agent-e2e",
        18086,
        19006,
        server_env={"CLOACINA_AGENT_HEARTBEAT_INTERVAL_S": str(FULL_AGENT_HEARTBEAT_SECONDS)},
    ) as stack:
        marker = stack.home / "full-agent-marker.txt"
        stack.compile_fixture(LONG_FIXTURE)
        # One slot. The fixture task has `retry_attempts = 0`: a refusal that
        # used an attempt would fail the task.
        agent_log = stack.start_agent(
            max_concurrency=1,
            env={
                "FLEET_LONG_TASK_MARKER": str(marker),
                "FLEET_LONG_TASK_SECONDS": str(FULL_AGENT_TASK_SECONDS),
            },
        )

        # Two executions, one after the other with no pause: both dispatches
        # fall between two heartbeats of the agent.
        first = _poll_run_workflow(stack.home, LONG_WORKFLOW, timeout_s=120.0)
        second = _poll_run_workflow(stack.home, LONG_WORKFLOW, timeout_s=120.0)
        started_at = time.time()
        print(f"  triggered executions: {first}, {second} (each task runs {FULL_AGENT_TASK_SECONDS} s)")

        # While one task runs, the other must be Ready on attempt 1. A sample
        # can fall inside a dispatch try (the row is Running until the refusal
        # arrives), so take several.
        waits_as_ready = False
        samples = []
        for _ in range(6):
            time.sleep(1.5)
            states = sorted(
                (t["status"], t["attempt"])
                for execution_id in (first, second)
                for t in stack.tasks(execution_id)
            )
            samples.append(states)
            if states == [("Ready", 1), ("Running", 1)]:
                waits_as_ready = True
                break
        print(f"  tasks while the agent is full: {samples}")
        if not waits_as_ready:
            raise AssertionError(
                "while the agent runs one task, the other must be Ready on attempt 1; "
                f"saw {samples}"
            )

        statuses = {}
        for execution_id in (first, second):
            statuses[execution_id] = _poll_execution_status(
                stack.home, execution_id, TERMINAL, timeout_s=300.0
            )
        print(f"  executions {list(statuses.values())} after {time.time() - started_at:.0f} s")
        time.sleep(2.0)

        refused = stack.log_text(agent_log.name).count("refusing packet")
        waits = stack.log_text("server.log").count("the agent refused: full")
        lines = _marker_lines(marker)
        starts = [line for line in lines if line.startswith("start ")]
        ends = [line for line in lines if line.startswith("end ")]
        print(
            f"  packets refused by the agent: {refused}, waits logged by the server: {waits}, "
            f"task starts: {len(starts)}, task ends: {len(ends)}"
        )

        if refused == 0:
            raise AssertionError(
                "the agent refused no packet, so this run did not test the refusal. "
                "A heartbeat fell between the two dispatches, or the server now "
                "counts the slot itself."
            )
        for execution_id, status in statuses.items():
            if status != "Completed":
                raise AssertionError(f"execution {execution_id} ended in {status!r}, expected Completed")
            tasks = stack.tasks(execution_id)
            if len(tasks) != 1 or tasks[0]["attempt"] != 1:
                raise AssertionError(
                    f"execution {execution_id}: a refusal for capacity must use no attempt; "
                    f"got {[(t['status'], t['attempt']) for t in tasks]}"
                )
        if len(starts) != 2 or len(ends) != 2:
            raise AssertionError(
                f"each task must run one time: {len(starts)} starts, {len(ends)} ends"
            )
        print("  ok: the refused task waited as Ready and both ran on attempt 1")

    print_final_success("cloacina fleet full-agent e2e")
