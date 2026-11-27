"""Fleet e2e test of agent pools and task requests (CLOACINA-T-1027).

One live stack from `_fleet_stack.py`, two agents started by hand in the
tenant `public`:

- one in the pool `default`, which reports no totals;
- one in the pool `large` (`--pool large --cpu 4 --memory 32Gi`).

The fixture `fleet-pools-rust` has a task `small` (no pool, no request) and a
task `big` (`pool = "large"`, `memory = "24Gi"`) that blocks for a few seconds.
Each task writes its start and end, with the label of the agent it ran on, to
a marker file.

What it shows:

1. A package whose task names a pool the tenant does not have is refused: its
   build status ends `failed` and the error names the task and the pool.
2. `small` runs on the `default` agent, `big` on the `large` agent. Never the
   other way.
3. Two runs of the workflow at the same time: the two `big` tasks (24Gi each,
   on a 32Gi agent) do not run together. The second one waits and runs on its
   first attempt.

Requires Docker. Run with: `angreal test e2e fleet-pools`.
"""

import time
from pathlib import Path

import angreal  # type: ignore

from .._utils import print_final_success, print_section_header
from ._fleet_stack import FleetStack
from .compiler import (
    _poll_build_status,
    _poll_execution_status,
    _poll_run_workflow,
    _stage_fixture,
    _upload,
)

test = angreal.command_group(
    name="test", about="Cloacina test suites (unit, integration, e2e, soak)"
)
e2e = angreal.command_group(name="e2e", about="end-to-end tests against a live server")

FIXTURE = "fleet-pools-rust"
WORKFLOW = "fleet_pools_workflow"
BIG_SECONDS = 8
TERMINAL = {"Completed", "Failed", "Cancelled"}


def _marker(marker: Path) -> list[dict]:
    """The marker file as records: task, event, unix millis, agent label."""
    if not marker.exists():
        return []
    records = []
    for line in marker.read_text().splitlines():
        parts = line.split()
        if len(parts) == 4:
            records.append(
                {"task": parts[0], "event": parts[1], "at": int(parts[2]), "agent": parts[3]}
            )
    return records


@test()
@e2e()
@angreal.command(
    name="fleet-pools",
    about="fleet e2e: two agent pools; tasks land in their pool and are packed by their requests",
    when_to_use=[
        "checking that a task with a pool runs only on an agent of that pool",
        "checking that a task that does not fit an agent waits and uses no retry attempt",
        "checking that a package with a task of an unknown pool is refused at publish",
    ],
    when_not_to_use=["unit testing", "running without docker"],
)
def fleet_pools():
    print_section_header("cloacina fleet pools e2e")
    with FleetStack("fleet-pools-e2e", 18086, 19006) as stack:
        marker = stack.home / "pools-marker.txt"

        # --- 1. a pool the tenant does not have: refused at publish ---------
        # The same fixture, renamed, with its `big` task in the pool `gpu`.
        refused_dir = _stage_fixture(stack.home, FIXTURE, rename_to="fleet-pools-refused-rust")
        lib = refused_dir / "src" / "lib.rs"
        source = lib.read_text()
        if 'pool = "large"' not in source:
            raise AssertionError("the fixture no longer declares pool = \"large\"")
        lib.write_text(source.replace('pool = "large"', 'pool = "gpu"'))
        print("  compiling the package with a task of the unknown pool 'gpu'")
        refused_id = _upload(stack.home, refused_dir)
        body = _poll_build_status(stack.home, refused_id, {"failed"}, timeout_s=900.0)
        error = body.get("build_error") or ""
        for needle in ("task 'big'", "pool 'gpu'", "does not have"):
            if needle not in error:
                raise AssertionError(
                    f"the refusal must name the task and the pool; {needle!r} is not in: {error!r}"
                )
        print(f"  ok: refused at publish: {error}")

        # --- 2. the pool exists: the package loads --------------------------
        # No managed agents (max_agents 0): the test starts the agents by hand.
        stack.api(
            "PUT",
            "/v1/tenants/public/pools/large",
            {"cpu_limit": "4", "memory_limit": "32Gi", "max_agents": 0},
        )
        print("  ok: pool 'large' created (one agent holds 4 CPU, 32Gi)")
        stack.compile_fixture(FIXTURE)

        task_env = {
            "FLEET_POOLS_MARKER": str(marker),
            "FLEET_POOLS_BIG_SECONDS": str(BIG_SECONDS),
        }
        stack.start_agent(max_concurrency=2, env={**task_env, "FLEET_POOLS_AGENT": "default"})
        stack.start_agent(
            max_concurrency=4,
            env={**task_env, "FLEET_POOLS_AGENT": "large"},
            pool="large",
            cpu="4",
            memory="32Gi",
        )

        # --- 3. two runs at the same time -----------------------------------
        first = _poll_run_workflow(stack.home, WORKFLOW, timeout_s=120.0)
        second = _poll_run_workflow(stack.home, WORKFLOW, timeout_s=120.0)
        print(f"  triggered executions: {first}, {second}")
        for execution_id in (first, second):
            status = _poll_execution_status(stack.home, execution_id, TERMINAL, timeout_s=180.0)
            if status != "Completed":
                raise AssertionError(f"execution {execution_id} ended in {status!r}")
            tasks = stack.tasks(execution_id)
            attempts = {t["task_name"]: t["attempt"] for t in tasks}
            if len(tasks) != 2 or any(a != 1 for a in attempts.values()):
                raise AssertionError(f"each task must complete on attempt 1; got {attempts}")
        print("  ok: both executions Completed, every task on attempt 1")
        time.sleep(1.0)

        records = _marker(marker)
        print("  marker file:")
        for r in records:
            print(f"    {r['task']} {r['event']} {r['at']} {r['agent']}")

        # Each task ran in its pool, and only there.
        wrong = [
            r for r in records
            if (r["task"] == "big") != (r["agent"] == "large")
        ]
        if wrong:
            raise AssertionError(f"a task ran outside its pool: {wrong}")
        starts = {t: [r for r in records if r["task"] == t and r["event"] == "start"] for t in ("small", "big")}
        ends = [r for r in records if r["task"] == "big" and r["event"] == "end"]
        if len(starts["small"]) != 2 or len(starts["big"]) != 2 or len(ends) != 2:
            raise AssertionError(
                f"expected 2 runs of each task; small {len(starts['small'])}, "
                f"big {len(starts['big'])} starts and {len(ends)} ends"
            )
        print("  ok: 'small' ran on the default agent, 'big' on the large agent")

        # The two 24Gi tasks did not share the 32Gi agent: the second started
        # after the first ended.
        big_starts = sorted(r["at"] for r in starts["big"])
        first_end = min(r["at"] for r in ends)
        if big_starts[1] < first_end:
            raise AssertionError(
                f"two 24Gi tasks ran together on a 32Gi agent: the second started at "
                f"{big_starts[1]}, the first ended at {first_end}"
            )
        waited = stack.log_text("server.log").count("no capacity now")
        if waited < 1:
            raise AssertionError("the second 24Gi task was not put off for capacity")
        print(
            f"  ok: the second 24Gi task waited {big_starts[1] - big_starts[0]} ms for the first "
            f"({waited} deferred dispatch tries) and ran on attempt 1"
        )

    print_final_success("cloacina fleet pools e2e")
