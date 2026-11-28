"""The standing workload, described once (CLOACINA-T-1107).

The driver deploys what this module lists; the checker's rules judge it.
Tenants, packages, schedules, and which workflow gives evidence for which
promise. Nothing here talks to a server.

The self-driving workflow runs on an agent and cannot import this module. The
few values it shares with the checker (the cycle, the package names, the key
of its action list) are also in its own source, and a unit test compares the
two copies.
"""

from __future__ import annotations

from . import promises

# A schema name allows no hyphen, so `team_a`, not `team-a`.
TENANTS = ("team_a", "team_b", "team_c")
AGENTS_PER_TENANT = 2
# Tasks one agent runs at once: the default of `cloacina-agent --max-concurrency`.
AGENT_SLOTS = 4
# The tenant's limit on agents. Equal to AGENTS_PER_TENANT, so a full tenant
# stays full until its held work ends.
MAX_AGENTS_PER_TENANT = 2

# The tenant secret that holds a write key of the tenant itself, for its
# self-driving workflow. One field, `key`.
DRIVER_SECRET = "standing_driver_key"
DRIVER_WORKFLOW = "standing_driver"
DRIVER_INSTANCE = "drive"
# The key in the self-driving run's context that holds the list of what it did.
DRIVER_ACTIONS_KEY = "standing_driver_actions"

# The self-driving workflow's cycle: twelve slots of five minutes.
SLOT_SECONDS = 300
CYCLE_SLOTS = 12
SLOT_ACTIONS = {
    0: ("publish", "py"),
    3: ("publish_broken", "rust"),
    4: ("start_holds", ""),
    6: ("publish", "py"),
    7: ("step_back", "py"),
    9: ("publish", "rust"),
}
# Each tenant is shifted in the cycle, so the Rust builds do not start together.
TENANT_OFFSETS = {"team_a": 0, "team_b": 4, "team_c": 8}

# The packages the self-driving workflow publishes itself.
TARGET_PY = "standing-target-py"
TARGET_PY_WORKFLOW = "standing_target_py"
TARGET_RUST = "standing-target-rust"
TARGET_RUST_WORKFLOW = "standing_target_rust"
TARGETS = {"py": TARGET_PY, "rust": TARGET_RUST}

HOLD_WORKFLOW = "standing_hold"
HOLD_SECONDS = 120

# Each task body of the workload prints one line that starts with this when
# it starts, with a JSON object after it (CLOACINA-T-1107, CLOACINA-T-1110):
# the workflow, the task, and what names the run (`schedule_id` and
# `scheduled_time` for a scheduled run, `run_once_tag` for a run by hand).
MARK = "STANDING_MARK "
# The context key a run by hand carries so its marks name it.
RUN_TAG_KEY = "run_once_tag"

# Where the pods are, for the checker's `kubectl logs`.
AGENT_SELECTOR = "app.kubernetes.io/name=cloacina-agent"
SERVER_SELECTOR = "app.kubernetes.io/name=cloacina-server"
SERVER_REPLICAS = 2


def agent_namespace(tenant):
    """The namespace of a tenant's agents: the rule of the server's actuator
    (`actuator::kubernetes::tenant_namespace`) for a schema name."""
    segment = "".join(char if char.isalnum() else "-" for char in tenant.lower()).strip("-")
    while "--" in segment:
        segment = segment.replace("--", "-")
    return f"cloacina-tenant-{segment or 'unnamed'}"
# More holds at once than the tenant's agents can take.
HOLD_BURST = 12


class Package:
    """A package the driver publishes from `examples/fixtures/<fixture>`."""

    def __init__(self, name, workflow, language, what):
        self.name = name
        self.fixture = name
        self.workflow = workflow
        self.language = language
        self.what = what


PACKAGES = (
    Package("standing-rollup-rust", "standing_rollup", "rust",
            "a short data job on a five-minute schedule; writes its interval"),
    Package("standing-extract-py", "standing_extract", "python",
            "three short tasks in a row on a five-minute schedule; writes its interval"),
    Package("standing-archive-py", "standing_archive", "python",
            "one task that holds for six minutes, on an hourly schedule"),
    Package("standing-flaky-py", "standing_flaky", "python",
            "fails once and passes on a retry; twice an hour the run fails for good"),
    Package("standing-hold-py", HOLD_WORKFLOW, "python",
            "holds an agent for two minutes; started through the API"),
    Package("standing-driver-py", DRIVER_WORKFLOW, "python",
            "the self-driving workflow"),
)


class Schedule:
    """A schedule of the workload.

    `kind` is `package` for a cron trigger the package declares itself, and
    `instance` for a workflow instance the driver creates through the API.
    `cron` is the standard five-field form.
    """

    def __init__(self, workflow, cron, period_s, kind="package", instance=None,
                 catchup=None, max_active_runs=None, bind_driver_secret=False):
        self.workflow = workflow
        self.cron = cron
        self.period_s = period_s
        self.kind = kind
        self.instance = instance
        self.catchup = catchup
        self.max_active_runs = max_active_runs
        self.bind_driver_secret = bind_driver_secret


FIVE_MINUTES = "*/5 * * * *"
HOURLY = "0 * * * *"

SCHEDULES = (
    Schedule("standing_rollup", FIVE_MINUTES, 300),
    Schedule("standing_extract", FIVE_MINUTES, 300),
    # A second schedule of the same workflow: every missed fire runs, oldest
    # first, one at a time. It is the backlog the "Schedules" row judges.
    Schedule("standing_extract", FIVE_MINUTES, 300, kind="instance", instance="in_order",
             catchup="run_all", max_active_runs=1),
    Schedule("standing_archive", HOURLY, 3600),
    Schedule("standing_flaky", FIVE_MINUTES, 300),
    Schedule(DRIVER_WORKFLOW, FIVE_MINUTES, 300, kind="instance", instance=DRIVER_INSTANCE,
             bind_driver_secret=True),
)

# `standing_flaky` fails for good, on purpose, when the minute of its fire
# time is one of these.
PLANNED_FAILURE_MINUTES = (0, 30)

# Which workflow gives evidence for which promise. A promise with an empty
# list has no work in this workload yet; the reason is in WITHOUT_EVIDENCE.
EVIDENCE = {
    "workload.agents": [],
    "workload.schedules": ["standing_rollup", "standing_extract", "standing_archive",
                           "standing_flaky", DRIVER_WORKFLOW],
    "workload.self_driving": [DRIVER_WORKFLOW],
    "publish.own_package": [DRIVER_WORKFLOW, TARGET_PY_WORKFLOW, TARGET_RUST_WORKFLOW],
    "publish.bad_build": [DRIVER_WORKFLOW, TARGET_RUST_WORKFLOW],
    "publish.step_back": [DRIVER_WORKFLOW, TARGET_PY_WORKFLOW],
    "run_once.long_task": ["standing_archive"],
    "run_once.no_free_agent": [HOLD_WORKFLOW, "standing_rollup", "standing_extract"],
    "run_once.other_replica": ["standing_rollup", "standing_extract", "standing_archive",
                               HOLD_WORKFLOW],
    "schedules.on_time": ["standing_rollup", "standing_extract", "standing_archive", HOLD_WORKFLOW],
    "schedules.interval": ["standing_rollup", "standing_extract"],
    "schedules.backlog": ["standing_extract"],
    "failure.agent_dies": ["standing_archive", HOLD_WORKFLOW],
    "failure.server_restart": ["standing_archive", HOLD_WORKFLOW, "standing_extract"],
    "failure.postgres_away": ["standing_rollup", "standing_extract", "standing_flaky"],
    "tenants.key": [DRIVER_WORKFLOW],
    "tenants.task": [DRIVER_WORKFLOW],
    "tenants.code_rust_load": [],
    "tenants.code_python_import": [],
    "tenants.code_default_chart": [],
    "tenants.code_graph_no_agent": [],
    "tenants.code_poll_trigger": [],
    "tenants.code_reactive_layer": [],
    "tenants.code_public": [],
    "tenants.code_build_role": [],
    "tenants.code_older_package": [],
    "tenants.code_pool_role": [],
    "capacity.waits": [HOLD_WORKFLOW],
    "capacity.fits": [],
    "operations.alert": ["standing_flaky"],
    "operations.audit": [DRIVER_WORKFLOW],
    "operations.backlog": [HOLD_WORKFLOW],
    "operations.retention": ["standing_rollup", "standing_extract", "standing_flaky"],
    "upgrade.in_flight": ["standing_archive", HOLD_WORKFLOW],
}

WITHOUT_EVIDENCE = {
    "workload.agents": "judged from the agent roster, not from a workflow",
    "capacity.fits": "no workflow of the workload names a pool; the Capacity chapter adds one",
}
# The claims of CLOACINA-A-0019 are shown by the scenarios of the chapter
# tenant_code, on its own tenant, and two of them from Postgres's catalogue.
for _promise in [p for p in EVIDENCE if p.startswith("tenants.code_")]:
    WITHOUT_EVIDENCE[_promise] = ("shown by the scenarios of the chapter tenant_code "
                                  "(CLOACINA-T-1079), not by a workflow of the workload")
del _promise


def scheduled_workflows():
    """workflow name -> the shortest period of its schedules, in seconds."""
    out = {}
    for schedule in SCHEDULES:
        current = out.get(schedule.workflow)
        if current is None or schedule.period_s < current:
            out[schedule.workflow] = schedule.period_s
    return out


def slot_of(epoch_seconds, offset=0):
    """The slot of the self-driving cycle that a fire time falls in."""
    return (int(epoch_seconds) // SLOT_SECONDS + int(offset)) % CYCLE_SLOTS


def problems():
    """What is wrong with this description. Empty when it is sound."""
    found = []
    known = {package.workflow for package in PACKAGES} | {TARGET_PY_WORKFLOW, TARGET_RUST_WORKFLOW}
    for promise_id in promises.PROMISES:
        if promise_id not in EVIDENCE:
            found.append(f"promise {promise_id} has no line in EVIDENCE")
        elif not EVIDENCE[promise_id] and promise_id not in WITHOUT_EVIDENCE:
            found.append(f"promise {promise_id} has no workflow and no reason in WITHOUT_EVIDENCE")
    for promise_id, workflows in EVIDENCE.items():
        if promise_id not in promises.PROMISES:
            found.append(f"EVIDENCE names an unknown promise: {promise_id}")
        for workflow in workflows:
            if workflow not in known:
                found.append(f"EVIDENCE of {promise_id} names an unknown workflow: {workflow}")
    for row in promises.ROWS:
        ids = [p for p in promises.PROMISES if promises.row_of(p) == row]
        if not any(EVIDENCE.get(p) for p in ids) and row != "Workload":
            found.append(f"row {row!r} has no work to judge")
    for schedule in SCHEDULES:
        if schedule.workflow not in known:
            found.append(f"a schedule names an unknown workflow: {schedule.workflow}")
        if len(schedule.cron.split()) != 5:
            found.append(f"the cron of {schedule.workflow} does not have five fields")
        if schedule.kind == "instance" and not schedule.instance:
            found.append(f"an instance schedule of {schedule.workflow} has no name")
    for tenant in TENANTS:
        if tenant not in TENANT_OFFSETS:
            found.append(f"tenant {tenant} has no offset in the self-driving cycle")
    if HOLD_BURST <= AGENTS_PER_TENANT * AGENT_SLOTS:
        found.append("HOLD_BURST does not exceed the tenant's agent slots: the burst fills nothing")
    return found
