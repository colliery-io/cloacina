"""The promises table of the behaviour suite.

One entry for each promise: its row, its sentence, and the Kairos task of the
chapter that judges it. A checker rule names the promise it judges by id. A
promise with no rule yet still shows in the report, with its chapter's task,
so the report never looks more complete than it is.

To add a promise, add a line here. To add a row, add it to `ROWS` too.
"""

from __future__ import annotations

# The order of the report. "Workload" is not a promise of the product: it
# says whether the standing workload itself is alive. When it does not hold,
# the verdicts below it have little work to judge.
ROWS = (
    "Workload",
    "Publish",
    "Run once",
    "Schedules",
    "Failure",
    "Tenants",
    "Capacity",
    "Operations",
    "Upgrade",
)

# The task of each row's chapter.
CHAPTER_TASK = {
    "Workload": "CLOACINA-T-1107",
    "Publish": "CLOACINA-T-1026",
    "Run once": "CLOACINA-T-1110",
    "Schedules": "CLOACINA-T-1111",
    "Failure": "CLOACINA-T-1112",
    "Tenants": "CLOACINA-T-1113",
    "Capacity": "CLOACINA-T-1114",
    "Operations": "CLOACINA-T-1115",
    "Upgrade": "CLOACINA-T-1046",
}

# id -> (row, sentence)
PROMISES = {
    "workload.agents": ("Workload", "Each tenant has its own agents, and they are alive."),
    "workload.schedules": ("Workload", "Each schedule of the workload has a recent run."),
    "workload.self_driving": ("Workload", "The self-driving workflow of each tenant acted recently."),
    "publish.own_package": ("Publish", "A team builds and publishes its own package with no restart."),
    "publish.bad_build": ("Publish", "A bad build leaves the running version in service."),
    "publish.step_back": ("Publish", "A team can go back a version."),
    "run_once.long_task": ("Run once", "A long task runs one time."),
    "run_once.no_free_agent": ("Run once", "A task with no free agent runs one time."),
    "run_once.other_replica": ("Run once", "A task whose result lands on another replica runs one time."),
    "schedules.on_time": ("Schedules", "A schedule fires on time while other work is slow."),
    "schedules.interval": ("Schedules", "A run knows its interval."),
    "schedules.backlog": ("Schedules", "A backlog drains in order at its limit."),
    "failure.agent_dies": ("Failure", "An agent dies mid-task and the task finishes on another agent."),
    "failure.server_restart": ("Failure", "The server restarts and runs survive."),
    "failure.postgres_away": (
        "Failure",
        "Postgres goes away for minutes and nothing is lost or doubled beyond the stated rule.",
    ),
    "tenants.key": ("Tenants", "A key of one team does nothing in another."),
    "tenants.task": ("Tenants", "A task of one team cannot read another team's packages or secrets."),
    "capacity.waits": ("Capacity", "Work waits when a tenant is full and runs when capacity arrives."),
    "capacity.fits": ("Capacity", "A task lands on an agent that fits it."),
    "operations.alert": ("Operations", "A failed run raises an alert."),
    "operations.audit": ("Operations", "Who did what is on record."),
    "operations.backlog": ("Operations", "The backlog and the lag are visible."),
    "operations.retention": ("Operations", "Old history is removed when retention is set."),
    "upgrade.in_flight": ("Upgrade", "The server is upgraded with runs in flight and they finish."),
}



# Each kind of disturbance of the schedule (CLOACINA-T-1109), and the
# promises it puts to the test. The report shows them next to each
# disturbance; the runner refuses a kind that is not here.
DISTURBANCE_TESTS = {
    "kill_agent": ("failure.agent_dies", "run_once.long_task"),
    "restart_server": ("failure.server_restart", "run_once.other_replica", "schedules.on_time"),
    "postgres_away": ("failure.postgres_away", "schedules.backlog"),
    "fill_capacity": ("capacity.waits", "run_once.no_free_agent", "schedules.on_time"),
}


def row_of(promise_id):
    return PROMISES[promise_id][0]


def sentence_of(promise_id):
    return PROMISES[promise_id][1]


def problems():
    """What is wrong with the table itself. Empty when it is sound."""
    found = []
    for promise_id, (row, sentence) in PROMISES.items():
        if row not in ROWS:
            found.append(f"promise {promise_id}: row {row!r} is not in ROWS")
        if not sentence.endswith("."):
            found.append(f"promise {promise_id}: the sentence does not end with a full stop")
    for row in ROWS:
        if row not in CHAPTER_TASK:
            found.append(f"row {row!r} has no chapter task")
    for kind, tested in DISTURBANCE_TESTS.items():
        for promise_id in tested:
            if promise_id not in PROMISES:
                found.append(f"disturbance {kind} tests an unknown promise: {promise_id}")
    return found
