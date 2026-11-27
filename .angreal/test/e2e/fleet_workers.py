"""Fleet e2e test of worker processes (CLOACINA-T-1028).

One live stack from `_fleet_stack.py` and one agent with four slots. The
fixture `fleet-workers-rust` has four tasks with no dependency between them:
`steady` (requests 256Mi, ends well), `greedy` (requests 64Mi, takes 512 MB),
`slow` (time limit 3 s, blocks 60 s) and `quick` (marked inline). Each writes
its start and end, with its process id, to a marker file.

What it shows:

1. Each task runs in a worker process of its own; `quick` runs in the agent
   process.
2. `greedy` fails alone, with the over-limit error. `steady`, on the same
   agent at the same time, completes.
3. `slow` is killed at its time limit: it never writes its end line.
4. The agent stays up through all of it.
5. A cancel from the server stops an attempt the server gave up on. The
   agent is stopped (SIGSTOP) before a run, so the packet of `slow` waits in
   its socket. The server's wait for the result ends and it sends the cancel.
   The agent is started again: it gets the packet and then the cancel, and
   the task does not run to its own time limit.

Requires Docker. Run with: `angreal test e2e fleet-workers`.
"""

import os
import signal
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

FIXTURE = "fleet-workers-rust"
WORKFLOW = "fleet_workers_workflow"
STEADY_SECONDS = 6
TERMINAL = {"Completed", "Failed", "Cancelled"}
# With a long heartbeat interval the server does not declare the stopped
# agent dead during the cancel part.
HEARTBEAT_SECONDS = 60

OVER_LIMIT = "exceeded its memory limit of 64Mi"
PAST_TIME_LIMIT = "past its time limit"
CANCEL_SENT = "cancel sent for the dispatch"
CANCEL_TAKEN = "the server cancelled a dispatch"


def _marker(marker: Path) -> list[dict]:
    """The marker file as records: task, event, process id."""
    if not marker.exists():
        return []
    records = []
    for line in marker.read_text().splitlines():
        parts = line.split()
        if len(parts) == 3:
            records.append({"task": parts[0], "event": parts[1], "pid": int(parts[2])})
    return records


def _by_task(tasks: list[dict]) -> dict:
    """The tasks of an execution by their short name."""
    return {t["task_name"].split("::")[-1]: t for t in tasks}


def _wait_for(what: str, found, timeout_s: float) -> None:
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        if found():
            return
        time.sleep(0.5)
    raise AssertionError(f"timed out after {timeout_s:.0f} s waiting for {what}")


@test()
@e2e()
@angreal.command(
    name="fleet-workers",
    about="fleet e2e: tasks run in worker processes; memory limit, time limit, inline, cancel",
    when_to_use=[
        "checking that a task over its memory limit fails alone",
        "checking that a task past its time limit is killed",
        "checking that an inline task runs in the agent process",
        "checking that a cancel from the server stops an abandoned attempt",
    ],
    when_not_to_use=["unit testing", "running without docker"],
)
def fleet_workers():
    print_section_header("cloacina fleet workers e2e")
    with FleetStack(
        "fleet-workers-e2e",
        18087,
        19007,
        server_env={"CLOACINA_AGENT_HEARTBEAT_INTERVAL_S": str(HEARTBEAT_SECONDS)},
    ) as stack:
        marker = stack.home / "workers-marker.txt"
        stack.compile_fixture(FIXTURE)
        agent_log = stack.start_agent(
            max_concurrency=4,
            env={
                "FLEET_WORKERS_MARKER": str(marker),
                "FLEET_WORKERS_STEADY_SECONDS": str(STEADY_SECONDS),
            },
        )
        agent = stack.agent_procs[-1]

        # --- 1. one run: four tasks together on the agent --------------------
        execution_id = _poll_run_workflow(stack.home, WORKFLOW, timeout_s=120.0)
        print(f"  triggered execution: {execution_id}")
        status = _poll_execution_status(stack.home, execution_id, TERMINAL, timeout_s=180.0)
        time.sleep(1.0)
        tasks = _by_task(stack.tasks(execution_id))
        print(
            f"  execution {status}; tasks: "
            f"{[(name, t['status'], t.get('last_error')) for name, t in tasks.items()]}"
        )
        records = _marker(marker)
        print("  marker file:")
        for r in records:
            print(f"    {r['task']} {r['event']} {r['pid']}")

        def pid_of(task: str, event: str = "start") -> int | None:
            found = [r["pid"] for r in records if r["task"] == task and r["event"] == event]
            return found[0] if found else None

        if set(tasks) != {"steady", "greedy", "slow", "quick"}:
            raise AssertionError(f"expected the four tasks, got {sorted(tasks)}")

        # steady: in a worker, to its end.
        if tasks["steady"]["status"] != "Completed":
            raise AssertionError(f"'steady' ended {tasks['steady']['status']!r}, expected Completed")
        if pid_of("steady") is None or pid_of("steady", "end") != pid_of("steady"):
            raise AssertionError("'steady' must write its start and its end from one process")

        # greedy: stopped at its memory limit, alone.
        error = tasks["greedy"].get("last_error") or ""
        if tasks["greedy"]["status"] != "Failed" or OVER_LIMIT not in error:
            raise AssertionError(
                f"'greedy' must fail with the over-limit error; got "
                f"{tasks['greedy']['status']!r}: {error!r}"
            )
        if pid_of("greedy") is None or pid_of("greedy", "end") is not None:
            raise AssertionError("'greedy' must start and must not reach its end")
        print(f"  ok: 'greedy' failed alone ({error}); 'steady' completed beside it")

        # slow: killed at its time limit.
        if tasks["slow"]["status"] != "Failed":
            raise AssertionError(f"'slow' ended {tasks['slow']['status']!r}, expected Failed")
        if pid_of("slow") is None or pid_of("slow", "end") is not None:
            raise AssertionError("'slow' must start and must not reach its end")
        limit_kills = stack.log_text(agent_log.name).count(PAST_TIME_LIMIT)
        if limit_kills != 1:
            raise AssertionError(f"the agent must stop one worker at its time limit; saw {limit_kills}")
        print("  ok: 'slow' was killed at its time limit")

        # quick: in the agent process. The others: one worker process each.
        if tasks["quick"]["status"] != "Completed":
            raise AssertionError(f"'quick' ended {tasks['quick']['status']!r}, expected Completed")
        if pid_of("quick") != agent.pid:
            raise AssertionError(
                f"the inline task must run in the agent process {agent.pid}; "
                f"it ran in {pid_of('quick')}"
            )
        worker_pids = [pid_of(t) for t in ("steady", "greedy", "slow")]
        if agent.pid in worker_pids or len(set(worker_pids)) != 3:
            raise AssertionError(
                f"each task must run in a worker process of its own; agent {agent.pid}, "
                f"workers {worker_pids}"
            )
        print(f"  ok: 'quick' ran in the agent ({agent.pid}); the others in workers {worker_pids}")

        if agent.poll() is not None:
            raise AssertionError(f"the agent exited: code={agent.returncode}")
        print("  ok: the agent stayed up")

        # --- 2. a cancel from the server -------------------------------------
        # The agent sleeps through the dispatch. The server waits for the
        # result of `slow` for its 3 s limit and a grace, gives up, and sends
        # the cancel. Both wait in the agent's socket.
        os.kill(agent.pid, signal.SIGSTOP)
        try:
            second = _poll_run_workflow(stack.home, WORKFLOW, timeout_s=120.0)
            print(f"  triggered execution: {second} with the agent stopped")
            _wait_for(
                "the server to give up on 'slow' and send the cancel",
                lambda: CANCEL_SENT in stack.log_text("server.log"),
                timeout_s=120.0,
            )
            print("  ok: the server gave up on the dispatch and sent a cancel")
        finally:
            os.kill(agent.pid, signal.SIGCONT)

        _wait_for(
            "the agent to take the cancel",
            lambda: CANCEL_TAKEN in stack.log_text(agent_log.name),
            timeout_s=60.0,
        )
        # Longer than the 3 s limit of `slow`: without the cancel the agent
        # would run it and stop it at its own limit.
        time.sleep(8.0)
        if agent.poll() is not None:
            raise AssertionError(f"the agent exited: code={agent.returncode}")
        limit_kills = stack.log_text(agent_log.name).count(PAST_TIME_LIMIT)
        if limit_kills != 1:
            raise AssertionError(
                "the cancelled attempt ran to its own time limit: the agent logged "
                f"{limit_kills} time-limit kills, expected the one of the first run"
            )
        slow_ends = [r for r in _marker(marker) if r["task"] == "slow" and r["event"] == "end"]
        if slow_ends:
            raise AssertionError(f"'slow' reached its end: {slow_ends}")
        print("  ok: the cancel stopped the abandoned attempt; it did not run to its time limit")

    print_final_success("cloacina fleet workers e2e")
