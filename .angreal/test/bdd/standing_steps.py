"""Step definitions of the standing workload's feature (CLOACINA-T-1107).

The feature brings the workload up and says that it runs. Each step says what
a team does or sees; the reach into the deployment is on the `StandingWorld`
the step receives. The product's promises are not judged here: the checker
judges them, all the time, against the work these steps start.

Where a step reads a channel that is not the public API (a run's context in
postgres), it says so: that is a fact about the harness, not the product.

A chapter's steps file names its chapter and its feature files; see
`chapters.py`.
"""

from __future__ import annotations

import json
import os
import stat
import time

from .gherkin import StepRegistry
from .standing_world import TERMINAL
from .suite import workload
from .suite.engine import parse_time

steps = StepRegistry()

CHAPTER = "standing"
FEATURES = ("features/standing/standing_workload.feature",)

FIVE_MINUTE_JOBS = ("standing_rollup", "standing_extract", "standing_flaky", workload.DRIVER_WORKFLOW)
# Where each data job writes the interval it was given.
INTERVAL_KEYS = {"standing_rollup": "standing_rollup_done", "standing_extract": "standing_extract_loaded"}
RUST_PACKAGE = "standing-rollup-rust"


def _wait(seconds, check, every=5):
    """Call `check` until it gives something true, for at most `seconds`."""
    deadline = time.time() + seconds
    while True:
        value = check()
        if value or time.time() >= deadline:
            return value
        time.sleep(every)


def _accepted(record):
    status = record.get("status")
    return isinstance(status, int) and 200 <= status < 300


# --- Background ---------------------------------------------------------------

@steps.given("a shared orchestrator runs in Kubernetes with a compiler for each team")
def bg_orchestrator(world):
    code, _ = world.api("GET", "/v1/agents", key=world.bootstrap_key, expect=None)
    assert code == 200, f"the orchestrator's agent roster answered {code}"


@steps.given("team A, team B and team C each have their own tenant, keys and agents")
def bg_tenants(world):
    counts = world.agent_counts()
    for tenant in world.tenants:
        assert tenant in world.keys, f"no publish key for {tenant}"
        assert tenant in world.checker_keys, f"no read key for the checker in {tenant}"
        assert counts.get(tenant, 0) >= workload.AGENTS_PER_TENANT, (
            f"{tenant} has {counts.get(tenant, 0)} registered agents, "
            f"the workload needs {workload.AGENTS_PER_TENANT}")
    world.note("registered agents: " + ", ".join(f"{t} {counts[t]}" for t in world.tenants))


# --- Scenario 1: the packages -------------------------------------------------

@steps.when("each team publishes the packages of the standing workload")
def s1_publish(world):
    for tenant in world.tenants:
        started = time.time()
        rows = world.publish_workload(tenant)
        world.note(f"{tenant}: {len(rows)} packages in service after {int(time.time() - started)} s")


@steps.then("every package of every team is built and in service")
def s1_in_service(world):
    wrong = []
    for tenant in world.tenants:
        for package in workload.PACKAGES:
            active = world.active_version(tenant, package.name, key=world.keys[tenant])
            if active is None:
                wrong.append(f"{tenant}/{package.name}: no version in service")
            elif active.get("build_status") != "success":
                wrong.append(f"{tenant}/{package.name}: in service and {active.get('build_status')}")
    assert not wrong, "; ".join(wrong)


@steps.then("the Rust package of each team was built in that team's tenant")
def s1_rust(world):
    # A compiler serves one tenant's schema. A Rust package that is built in
    # a tenant was built by that tenant's compiler.
    for tenant in world.tenants:
        active = world.active_version(tenant, RUST_PACKAGE, key=world.keys[tenant])
        assert active is not None, f"{tenant}: {RUST_PACKAGE} has no version in service"
        world.note(f"{tenant}: {RUST_PACKAGE} {active.get('version')} "
                   f"build_status={active.get('build_status')}, compiled_at={active.get('compiled_at')}")
        assert active.get("build_status") == "success", (
            f"{tenant}: {RUST_PACKAGE} is {active.get('build_status')}: "
            f"{str(active.get('build_error'))[-400:]}")


# --- Scenario 2: the schedules ------------------------------------------------

@steps.given("the packages of the standing workload are in service for each team")
def s2_packages(world):
    for tenant in world.tenants:
        world.publish_workload(tenant)


@steps.given("each team keeps a write key of its own as a secret for its self-driving workflow")
def s2_secret(world):
    for tenant in world.tenants:
        world.ensure_driver_secret(tenant)
        code, _ = world.api("GET", f"/v1/tenants/{tenant}/secrets/{workload.DRIVER_SECRET}",
                            key=world.admin_keys[tenant], expect=None)
        assert code == 200, f"{tenant}: the secret of the self-driving workflow is not there ({code})"


@steps.when("each team sets up the schedules of the standing workload")
def s2_setup(world):
    for tenant in world.tenants:
        made = world.setup_schedules(tenant)
        world.note(f"{tenant}: created {', '.join(made) or 'nothing (all were there)'}")
    world.workload_ready = True


def _expected_schedules():
    expected = {}
    for schedule in workload.SCHEDULES:
        expected[schedule.workflow] = expected.get(schedule.workflow, 0) + 1
    return expected


def _missing_schedules(world):
    missing = []
    for tenant in world.tenants:
        listed = world.cron_schedules(tenant)
        for workflow, count in sorted(_expected_schedules().items()):
            have = len(listed.get(workflow, []))
            if have < count:
                missing.append(f"{tenant}/{workflow}: {have} of {count}")
    return missing


@steps.then("each team's list of schedules holds every schedule of the workload")
def s2_listed(world):
    # A package's own schedule appears when the server has loaded the package.
    _wait(120, lambda: not _missing_schedules(world))
    missing = _missing_schedules(world)
    assert not missing, "schedules not listed: " + "; ".join(missing)


@steps.then("each of those schedules is switched on")
def s2_enabled(world):
    off = []
    for tenant in world.tenants:
        listed = world.cron_schedules(tenant)
        for workflow in _expected_schedules():
            for item in listed.get(workflow, []):
                if not item.get("enabled") or item.get("paused"):
                    off.append(f"{tenant}/{workflow} ({item.get('cron_expression')})")
    assert not off, "schedules that are off or paused: " + "; ".join(off)


# --- Scenario 3: the scheduled jobs run ---------------------------------------

@steps.given("the schedules of the standing workload are in place for each team")
def s3_ready(world):
    world.ensure_workload()


@steps.when("one period of the five-minute jobs has passed")
def s3_wait(world):
    now = time.time()
    world.scratch["period_started"] = now
    # The next fire time, and a little after it for the runs to start.
    next_fire = (int(now) // workload.SLOT_SECONDS + 1) * workload.SLOT_SECONDS
    world.note(f"waiting {int(next_fire + 30 - now)} s for the next fire time")
    time.sleep(max(0.0, next_fire + 30 - now))


def _finished_since(world, tenant, workflow, since):
    return [run for run in world.runs(tenant, workflow)
            if run.get("status") in TERMINAL and (parse_time(run.get("started_at")) or 0) >= since]


@steps.then("each five-minute job of each team has a finished run")
def s3_finished(world):
    since = world.scratch["period_started"]

    def waiting():
        return [f"{tenant}/{workflow}" for tenant in world.tenants for workflow in FIVE_MINUTE_JOBS
                if not _finished_since(world, tenant, workflow, since)]

    _wait(300, lambda: not waiting(), every=10)
    still = waiting()
    assert not still, "no finished run since the fire time for: " + ", ".join(still)


@steps.then("the data jobs wrote back the interval they were given")
def s3_interval(world):
    since = world.scratch["period_started"]
    wrong = []
    for tenant in world.tenants:
        for workflow, key in sorted(INTERVAL_KEYS.items()):
            done = [run for run in _finished_since(world, tenant, workflow, since)
                    if run.get("status") == "Completed"]
            if not done:
                wrong.append(f"{tenant}/{workflow}: no completed run to read")
                continue
            # Read from the run's context in postgres: the API has no field for it.
            written = world.execution_context(tenant, done[0]["id"]).get(key) or {}
            start = parse_time(written.get("interval_start"))
            end = parse_time(written.get("interval_end"))
            world.note(f"{tenant}/{workflow}: interval {written.get('interval_start')} "
                       f"to {written.get('interval_end')}")
            if start is None or end is None:
                wrong.append(f"{tenant}/{workflow}: the run wrote no interval")
            elif end - start != workload.SLOT_SECONDS:
                wrong.append(f"{tenant}/{workflow}: the interval is {int(end - start)} s long, "
                             f"not {workload.SLOT_SECONDS}")
    assert not wrong, "; ".join(wrong)


# --- Scenario 4: the self-driving workflow ------------------------------------

@steps.when("the self-driving workflow of each team has finished a run")
def s4_wait(world):
    def waiting():
        return [tenant for tenant in world.tenants
                if not any(run.get("status") == "Completed"
                           for run in world.runs(tenant, workload.DRIVER_WORKFLOW))]

    _wait(2 * workload.SLOT_SECONDS + 120, lambda: not waiting(), every=10)
    still = waiting()
    assert not still, "no completed run of the self-driving workflow in: " + ", ".join(still)
    world.scratch["actions"] = {tenant: world.driver_actions(tenant) for tenant in world.tenants}
    for tenant, actions in world.scratch["actions"].items():
        world.note(f"{tenant}: recorded " + ", ".join(
            f"{a.get('action')}={a.get('status', a.get('error', ''))}" for a in actions))


@steps.then("each team's self-driving workflow published its two target packages")
def s4_published(world):
    def unbuilt():
        waiting = []
        for tenant in world.tenants:
            for package in sorted(workload.TARGETS.values()):
                rows = world.list_versions(tenant, package, key=world.keys[tenant]) or []
                if not any(row.get("build_status") == "success" for row in rows):
                    waiting.append(f"{tenant}/{package}")
        return waiting

    # The Rust target is compiled in the cluster.
    _wait(900, lambda: not unbuilt(), every=10)
    still = unbuilt()
    assert not still, "no built version, published by the workflow itself, of: " + ", ".join(still)


@steps.then("each team's self-driving workflow started a run by hand")
def s4_by_hand(world):
    for tenant in world.tenants:
        started = [a for a in world.scratch["actions"][tenant]
                   if a.get("action") == "run_by_hand" and _accepted(a)]
        assert started, f"{tenant}: the self-driving workflow recorded no accepted run by hand"


@steps.then("each team's self-driving workflow got nothing from another team's tenant")
def s4_other_tenant(world):
    wrong = []
    for tenant in world.tenants:
        tries = [a for a in world.scratch["actions"][tenant] if a.get("action") == "other_tenant"]
        if not tries:
            wrong.append(f"{tenant}: no try on another tenant is on record")
            continue
        last = tries[-1]
        world.note(f"{tenant} on {last.get('other_tenant')}: packages {last.get('packages_status')}, "
                   f"executions {last.get('executions_status')}, secrets {last.get('secrets_status')}, "
                   f"secret by name {last.get('secret_by_name')}")
        for field in ("packages_status", "executions_status", "secrets_status"):
            if last.get(field) not in (401, 403, 404):
                wrong.append(f"{tenant}: {field} on {last.get('other_tenant')} is {last.get(field)}")
        if last.get("secret_by_name") == "read":
            wrong.append(f"{tenant}: read a secret named for {last.get('other_tenant')}")
    assert not wrong, "; ".join(wrong)


# --- Scenario 5: the checker's settings ---------------------------------------

@steps.when("the driver writes the settings of the checker")
def s5_write(world):
    world.write_checker_settings()
    world.note(f"settings of the checker: {world.settings_path}")


@steps.then("the settings hold the server, the database and a key for each team")
def s5_settings(world):
    with open(world.settings_path, "r", encoding="utf-8") as handle:
        settings = json.load(handle)
    assert settings.get("server"), "no server in the settings"
    assert str(settings.get("database_url", "")).startswith("postgres://"), "no database URL"
    assert sorted(settings.get("keys", {})) == sorted(world.tenants), (
        f"keys for {sorted(settings.get('keys', {}))}, tenants are {sorted(world.tenants)}")


@steps.then("only the owner of the settings file can read it")
def s5_owner_only(world):
    mode = os.stat(world.settings_path).st_mode
    assert not mode & (stat.S_IRWXG | stat.S_IRWXO), f"the settings file has mode {oct(mode & 0o777)}"
