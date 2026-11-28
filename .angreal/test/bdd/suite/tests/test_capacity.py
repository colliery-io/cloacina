"""The chapter "Capacity" (CLOACINA-T-1114): its feature, its rules on
recorded snapshots, and its fixtures. No server, no cluster."""

from __future__ import annotations

import os
import sys
import unittest

from suite import engine, rules, sources
from suite.rules import capacity
from suite.tests.test_workload import Context, load_fixture

HERE = os.path.dirname(os.path.abspath(__file__))
BDD = os.path.dirname(os.path.dirname(HERE))
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(BDD)))
HEALTHY = os.path.join(HERE, "recorded", "healthy.json")

# The folder that holds the `bdd` package.
sys.path.insert(0, os.path.dirname(BDD))

from bdd import capacity_steps as chapter  # noqa: E402
from bdd import chapters, gherkin  # noqa: E402


def healthy():
    return sources.read_snapshot(HEALTHY)


def judge(data):
    registry = rules.load()
    snapshot = engine.Snapshot(data)
    return {rule.id: rule.judge(snapshot) for rule in registry.rules}


def tenant_rows(data, tenant, name):
    return data["tenants"][tenant]["db"][name]["rows"]


def hold(data, number):
    return tenant_rows(data, "team_a", "capacity_hold_tasks")[number]


class FeatureTest(unittest.TestCase):
    def setUp(self):
        self.feature = chapters.load_features(chapters.find("capacity"), ROOT)[0]

    def test_every_step_has_exactly_one_definition(self):
        problems, _notes = gherkin.check(self.feature, chapter.steps)
        self.assertEqual(problems, [])

    def test_the_gaps_name_their_owners(self):
        gaps = {s.name: s.gap_tasks for s in self.feature.scenarios if s.is_gap}
        self.assertEqual(gaps, {
            "A task that fits no agent says why it waits": ["unfiled"],
            "A schedule made with a priority fires its runs at that priority": ["CLOACINA-T-1123"],
        })

    def test_each_scenario_ends_in_a_then(self):
        for scenario in self.feature.scenarios:
            self.assertEqual(scenario.steps[-1].kind, "Then", scenario.name)

    def test_the_fixtures_are_there_and_name_the_pool(self):
        for package, module, task, memory in (
                (chapter.SIZED_PACKAGE, chapter.SIZED_WORKFLOW, "sized", "400Mi"),
                (chapter.OVERSIZED_PACKAGE, chapter.OVERSIZED_WORKFLOW, "oversized", "2Gi")):
            folder = os.path.join(ROOT, "examples", "fixtures", package)
            with open(os.path.join(folder, "package.toml"), encoding="utf-8") as handle:
                self.assertIn(f'workflow_name = "{module}"', handle.read())
            with open(os.path.join(folder, "workflow", module, "tasks.py"), encoding="utf-8") as handle:
                text = handle.read()
            self.assertIn(f'pool="{chapter.POOL}"', text)
            self.assertIn(f'memory="{memory}"', text)
            fixture = load_fixture(package, module)
            context = Context({"hold_seconds": 0})
            getattr(fixture, task)(context)
            self.assertIn(fixture.RESULT_KEY, context.values)
            self.assertIn("host", context.values[fixture.RESULT_KEY])

    def test_the_sizes_leave_room_for_two_sized_tasks_and_not_for_the_oversized_one(self):
        self.assertEqual(chapter.SIZED_AGENT_MEMORY // chapter.SIZED_TASK_MEMORY, 2)
        self.assertLess(chapter.SIZED_AGENT_MEMORY, 2048 * chapter.MIB)
        self.assertGreaterEqual(chapter.ROOMY_AGENT_MEMORY, 2048 * chapter.MIB)


class HelpersTest(unittest.TestCase):
    def test_the_most_tasks_at_once(self):
        rows = [{"started_at": "2026-11-27T12:00:00Z", "completed_at": "2026-11-27T12:00:30Z"},
                {"started_at": "2026-11-27T12:00:10Z", "completed_at": "2026-11-27T12:00:40Z"},
                {"started_at": "2026-11-27T12:00:30Z", "completed_at": "2026-11-27T12:01:00Z"},
                {"started_at": None, "completed_at": None}]
        # The first ends when the third starts: two at once, not three.
        self.assertEqual(chapter._most_at_once(rows), 2)

    def test_a_gauge_is_read_by_its_labels(self):
        text = ('# HELP cloacina_waiting_tasks Tasks that wait\n'
                'cloacina_waiting_tasks{pool="default",priority="5",tenant="team_a"} 3\n'
                'cloacina_waiting_tasks{pool="default",priority="9",tenant="team_a"} 10\n'
                'cloacina_waiting_tasks{pool="default",priority="9",tenant="team_b"} 0\n')
        self.assertEqual(chapter.gauge(text, "cloacina_waiting_tasks",
                                       {"tenant": "team_a", "pool": "default", "priority": 9}), 10.0)
        self.assertIsNone(chapter.gauge(text, "cloacina_waiting_tasks", {"tenant": "team_c"}))

    def test_an_execution_id_that_is_not_one_is_never_put_in_sql(self):
        with self.assertRaises(AssertionError):
            chapter._id_list(["5d0c1a10-0000-4000-8000-0000000a1140", "x'); DROP TABLE t; --"])


class WindowsTest(unittest.TestCase):
    def test_a_tenant_is_full_while_a_hold_waits(self):
        windows = capacity.full_windows(tenant_rows(healthy(), "team_a", "capacity_hold_tasks"), 0)
        self.assertEqual([(engine.format_time(a), engine.format_time(b)) for a, b in windows],
                         [("2026-11-27T11:20:04Z", "2026-11-27T11:25:30Z")])

    def test_a_hold_that_still_waits_keeps_its_tenant_full_until_now(self):
        rows = [{"created_at": "2026-11-27T12:00:00Z", "started_at": None, "status": "Ready"}]
        now = engine.parse_time("2026-11-27T12:05:00Z")
        self.assertEqual(capacity.full_windows(rows, now), [(now - 300, now)])

    def test_ageing(self):
        self.assertEqual(capacity.aged(9, 0, 119), 9)
        self.assertEqual(capacity.aged(9, 0, 240), 7)
        self.assertEqual(capacity.aged(1, 0, 3600), 0)

    def test_the_pool_of_an_agent(self):
        self.assertEqual(capacity.pool_of_agent('["pool:sized"]'), "sized")
        self.assertEqual(capacity.pool_of_agent("[]"), "default")
        self.assertEqual(capacity.pool_of_agent(["x", "pool:large"]), "large")
        self.assertEqual(capacity.pool_of_agent("not json"), "default")

    def test_what_a_task_counts_for(self):
        self.assertEqual(capacity.charge(250, 1000, 4), 250)
        self.assertEqual(capacity.charge(None, 1000, 4), 250)
        self.assertEqual(capacity.charge(250, None, 4), 0)


class HealthyTest(unittest.TestCase):
    def test_each_rule_holds_and_says_what_it_read(self):
        verdicts = judge(healthy())
        for rule_id in ("capacity.held_work_runs", "capacity.no_failure_while_full",
                        "capacity.others_on_time", "capacity.priority_order",
                        "capacity.fits_in_flight"):
            self.assertEqual(verdicts[rule_id].state, engine.HOLDS,
                             f"{rule_id}: {verdicts[rule_id].evidence}")
        self.assertIn("3 of them waited", verdicts["capacity.held_work_runs"].evidence)
        # team_b 11:25 (two jobs) and team_c 11:25; team_b 11:30 is after the window.
        self.assertIn("3 runs of other tenants", verdicts["capacity.others_on_time"].evidence)
        self.assertIn("4 tasks in flight on 3 agents", verdicts["capacity.fits_in_flight"].evidence)


class HeldWorkTest(unittest.TestCase):
    def test_a_hold_that_failed(self):
        data = healthy()
        hold(data, 1)["run_status"] = "Failed"
        hold(data, 1)["status"] = "Failed"
        verdict = judge(data)["capacity.held_work_runs"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("is Failed (task Failed)", verdict.details[0])

    def test_a_hold_that_waits_for_ever(self):
        data = healthy()
        hold(data, 2).update({"run_status": "Running", "status": "Ready", "started_at": None,
                              "completed_at": None})
        verdict = judge(data)["capacity.held_work_runs"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("is Running (task Ready)", verdict.details[0])

    def test_a_hold_that_took_a_retry(self):
        data = healthy()
        hold(data, 3).update({"attempt": 2, "attempt_errors": 1})
        verdict = judge(data)["capacity.held_work_runs"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("took 2 attempts (1 failed)", verdict.details[0])

    def test_holds_that_never_waited_say_nothing_about_a_full_tenant(self):
        data = healthy()
        for row in tenant_rows(data, "team_a", "capacity_hold_tasks"):
            row["started_at"] = row["created_at"]
        self.assertEqual(judge(data)["capacity.held_work_runs"].state, engine.UNKNOWN)

    def test_a_young_burst_is_not_judged(self):
        data = healthy()
        for row in tenant_rows(data, "team_a", "capacity_hold_tasks"):
            row.update({"created_at": "2026-11-27T12:18:00", "started_at": None,
                        "status": "Ready", "run_status": "Pending", "completed_at": None})
        self.assertEqual(judge(data)["capacity.held_work_runs"].state, engine.UNKNOWN)


class FailureWhileFullTest(unittest.TestCase):
    def test_a_task_that_failed_while_its_tenant_was_full(self):
        data = healthy()
        tenant_rows(data, "team_a", "capacity_failures").append(
            {"workflow": "standing_rollup", "task": "rollup", "kind": "task",
             "at": "2026-11-27T11:21:00", "error": "no agent could take the task"})
        verdict = judge(data)["capacity.no_failure_while_full"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("task of standing_rollup/rollup failed at 2026-11-27T11:21:00Z",
                      verdict.details[0])

    def test_an_attempt_that_failed_while_its_tenant_was_full(self):
        data = healthy()
        tenant_rows(data, "team_a", "capacity_failures").append(
            {"workflow": "standing_extract", "task": "load", "kind": "attempt",
             "at": "2026-11-27T11:24:00", "error": "dispatch refused"})
        self.assertEqual(judge(data)["capacity.no_failure_while_full"].state, engine.BROKEN)

    def test_a_failure_outside_the_window_or_planned_is_not_a_fault(self):
        data = healthy()
        tenant_rows(data, "team_a", "capacity_failures").append(
            {"workflow": "standing_rollup", "task": "rollup", "kind": "task",
             "at": "2026-11-27T11:40:00", "error": "a real fault, but the tenant was not full"})
        self.assertEqual(judge(data)["capacity.no_failure_while_full"].state, engine.HOLDS)

    def test_no_full_tenant_is_cannot_judge(self):
        data = healthy()
        tenant_rows(data, "team_a", "capacity_hold_tasks")[:] = []
        self.assertEqual(judge(data)["capacity.no_failure_while_full"].state, engine.UNKNOWN)


class OthersOnTimeTest(unittest.TestCase):
    def test_another_tenant_late_while_one_is_full(self):
        data = healthy()
        tenant_rows(data, "team_b", "capacity_scheduled_runs")[0]["first_task_started"] = \
            "2026-11-27T11:27:00"
        verdict = judge(data)["capacity.others_on_time"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("team_b/standing_rollup: the fire of 2026-11-27T11:25:00Z started its first "
                      "task 120 s late", verdict.details)

    def test_a_fire_that_never_started(self):
        data = healthy()
        tenant_rows(data, "team_c", "capacity_scheduled_runs")[0]["first_task_started"] = None
        verdict = judge(data)["capacity.others_on_time"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("has started no task", verdict.details[0])

    def test_the_full_tenant_itself_may_be_late(self):
        # team_a's own fire of 11:25 started 41 s late, inside its own window.
        data = healthy()
        tenant_rows(data, "team_a", "capacity_scheduled_runs")[0]["first_task_started"] = \
            "2026-11-27T11:29:00"
        self.assertEqual(judge(data)["capacity.others_on_time"].state, engine.HOLDS)


class PriorityTest(unittest.TestCase):
    def test_a_worse_hold_that_started_while_a_better_one_waited(self):
        data = healthy()
        # The hold at 9 starts at 11:21:00, while two holds at 5 still wait.
        hold(data, 2)["started_at"] = "2026-11-27T11:21:00"
        verdict = judge(data)["capacity.priority_order"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("a hold at priority 9 started at 2026-11-27T11:21:00Z", verdict.details[0])

    def test_a_worse_hold_that_aged_level_may_start(self):
        rows = [
            {"task": "a", "priority": 5, "pool": None, "created_at": "2026-11-27T12:00:00Z",
             "started_at": "2026-11-27T12:20:00Z"},
            # Made ten minutes before the other: 9 less five steps is 4, better than 5.
            {"task": "b", "priority": 9, "pool": None, "created_at": "2026-11-27T11:50:00Z",
             "started_at": "2026-11-27T12:10:00Z"},
        ]
        self.assertEqual(capacity.priority_inversions(rows), [])

    def test_holds_of_another_pool_do_not_hold_each_other_back(self):
        rows = [
            {"task": "a", "priority": 1, "pool": "large", "created_at": "2026-11-27T12:00:00Z",
             "started_at": "2026-11-27T12:05:00Z"},
            {"task": "b", "priority": 9, "pool": None, "created_at": "2026-11-27T12:00:01Z",
             "started_at": "2026-11-27T12:00:02Z"},
        ]
        self.assertEqual(capacity.priority_inversions(rows), [])

    def test_one_priority_only_is_cannot_judge(self):
        data = healthy()
        hold(data, 2)["priority"] = 5
        self.assertEqual(judge(data)["capacity.priority_order"].state, engine.UNKNOWN)

    def test_the_order_within_one_priority_is_not_judged(self):
        rows = [
            {"task": "a", "priority": 5, "pool": None, "created_at": "2026-11-27T12:00:00Z",
             "started_at": "2026-11-27T12:10:00Z"},
            {"task": "b", "priority": 5, "pool": None, "created_at": "2026-11-27T12:01:00Z",
             "started_at": "2026-11-27T12:05:00Z"},
        ]
        self.assertEqual(capacity.priority_inversions(rows), [])


class FitsTest(unittest.TestCase):
    def test_a_task_on_an_agent_of_another_pool(self):
        data = healthy()
        tenant_rows(data, "team_c", "capacity_in_flight")[2]["fleet_agent_id"] = "agent-c2"
        verdict = judge(data)["capacity.fits_in_flight"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("names the pool default and is on agent-c2, of the pool sized", verdict.details[0])

    def test_an_agent_that_holds_more_memory_than_it_has(self):
        data = healthy()
        tenant_rows(data, "team_c", "capacity_in_flight").append(
            {"task": "x", "task_name": "team_c::capacity_sized::sized", "status": "Running",
             "pool": "sized", "cpu_request_millis": 250, "memory_request_bytes": 419430400,
             "fleet_agent_id": "agent-c2"})
        verdict = judge(data)["capacity.fits_in_flight"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("agent-c2 holds 1258291200 bytes of memory and has 1073741824", verdict.details)

    def test_an_agent_with_more_tasks_than_slots(self):
        data = healthy()
        rows = tenant_rows(data, "team_a", "capacity_in_flight")
        rows.extend(dict(rows[0], task=f"t{n}") for n in range(4))
        verdict = judge(data)["capacity.fits_in_flight"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("agent-a1 holds 5 tasks and has 4 slots", verdict.details)

    def test_a_task_on_an_agent_of_another_tenant(self):
        data = healthy()
        tenant_rows(data, "team_a", "capacity_in_flight")[0]["fleet_agent_id"] = "agent-b1"
        verdict = judge(data)["capacity.fits_in_flight"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("an agent of team_b", verdict.details[0])

    def test_a_task_on_an_agent_that_is_gone_is_left_to_the_roster_rule(self):
        data = healthy()
        tenant_rows(data, "team_a", "capacity_in_flight")[0]["fleet_agent_id"] = "agent-gone"
        self.assertEqual(judge(data)["capacity.fits_in_flight"].state, engine.HOLDS)

    def test_nothing_in_flight_is_cannot_judge(self):
        data = healthy()
        for tenant in data["tenants"]:
            tenant_rows(data, tenant, "capacity_in_flight")[:] = []
        self.assertEqual(judge(data)["capacity.fits_in_flight"].state, engine.UNKNOWN)


class SqlTest(unittest.TestCase):
    def test_the_tenant_queries_name_the_schema_and_the_roster_query_does_not(self):
        for sql in (capacity.HOLD_TASKS_SQL, capacity.FAILURES_SQL, capacity.SCHEDULED_RUNS_SQL,
                    capacity.IN_FLIGHT_SQL):
            self.assertIn("{schema}.", sql)
            self.assertTrue(sql.startswith("SELECT"))
        self.assertNotIn("{schema}", capacity.LIVE_AGENTS_SQL)

    def test_the_steps_narrow_the_rules_queries_where_they_mean_to(self):
        # The steps replace the three-hour window of two queries; a change of
        # the text there must not leave the window in place unnoticed.
        self.assertEqual(capacity.FAILURES_SQL.count("now() - interval '3 hours'"), 2)
        self.assertIn("se.scheduled_time > now() - interval '3 hours'", capacity.SCHEDULED_RUNS_SQL)


if __name__ == "__main__":
    unittest.main()
