"""The chapter schedules (CLOACINA-T-1111): its feature, its rules and the
cron reader the checker computes fire times with. No server, no cluster."""

from __future__ import annotations

import os
import sys
import unittest

from suite import cron, engine, promises, rules, sources, workload

HERE = os.path.dirname(os.path.abspath(__file__))
BDD = os.path.dirname(os.path.dirname(HERE))
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(BDD)))
HEALTHY = os.path.join(HERE, "recorded", "healthy.json")

# The folder that holds the `bdd` package.
sys.path.insert(0, os.path.dirname(BDD))

from bdd import chapters, gherkin  # noqa: E402
from bdd import schedules_steps as chapter  # noqa: E402
from suite.rules import schedules as chapter_rules  # noqa: E402

EVERY = "schedules.every_fire"
ON_TIME = "schedules.on_time"
INTERVAL = "schedules.interval"
BACKLOG = "schedules.backlog_order"
T = engine.parse_time


def judge(data):
    registry = rules.load()
    snapshot = engine.Snapshot(data)
    return {rule.id: rule.judge(snapshot) for rule in registry.rules}


def healthy():
    return sources.read_snapshot(HEALTHY)


def fires(data, tenant="team_a"):
    return data["tenants"][tenant]["db"]["fires"]["rows"]


def fire(data, schedule_end, minute, tenant="team_a"):
    return next(row for row in fires(data, tenant)
                if row["schedule_id"].endswith(schedule_end)
                and row["scheduled_time"].endswith(f"12:{minute}:00"))


class CronTest(unittest.TestCase):
    def test_five_minutes(self):
        every = cron.Cron("*/5 * * * *")
        self.assertEqual(every.next_after(T("2026-11-27T12:03:10Z")), T("2026-11-27T12:05:00Z"))
        self.assertEqual(every.next_after(T("2026-11-27T12:05:00Z")), T("2026-11-27T12:10:00Z"))
        self.assertEqual(every.previous_before(T("2026-11-27T12:05:00Z")), T("2026-11-27T12:00:00Z"))
        self.assertEqual(len(every.between(T("2026-11-27T12:00:00Z"), T("2026-11-27T13:00:00Z"))), 13)

    def test_hourly_and_lists_and_ranges(self):
        self.assertEqual(cron.Cron("0 * * * *").previous_before(T("2026-11-27T12:00:00Z")),
                         T("2026-11-27T11:00:00Z"))
        weekdays = cron.Cron("30 9 * * 1-5")
        # 2026-11-28 is a Saturday: the next fire is Monday.
        self.assertEqual(weekdays.next_after(T("2026-11-27T10:00:00Z")), T("2026-11-30T09:30:00Z"))
        self.assertTrue(cron.Cron("0,15,45 * * * *").matches(T("2026-11-27T12:45:00Z")))
        self.assertFalse(cron.Cron("0,15,45 * * * *").matches(T("2026-11-27T12:30:00Z")))
        self.assertTrue(cron.Cron("0 0 * * 7").matches(T("2026-11-29T00:00:00Z")))

    def test_a_time_zone_shifts_the_minute(self):
        # Asia/Kathmandu is UTC+5:45: minute 0 there is minute 15 in UTC.
        nepal = cron.Cron("0 * * * *", "Asia/Kathmandu")
        self.assertEqual(nepal.next_after(T("2026-11-27T12:00:00Z")), T("2026-11-27T12:15:00Z"))
        self.assertEqual(cron.local_minute_for(T("2026-11-27T12:15:00Z"), "Asia/Kathmandu"), 0)
        self.assertEqual(cron.Cron("0 9 * * *", "Europe/London").next_after(
            T("2026-07-01T00:00:00Z")), T("2026-07-01T08:00:00Z"))

    def test_what_it_refuses(self):
        for text in ("* * * *", "61 * * * *", "*/0 * * * *", "MON * * * *", "5-1 * * * *"):
            with self.assertRaises(cron.CronError, msg=text):
                cron.Cron(text)

    def test_the_workload_crons_are_read(self):
        for schedule in workload.SCHEDULES:
            cron.Cron(schedule.cron)

    def test_iso_is_the_form_the_server_writes(self):
        self.assertEqual(cron.iso(T("2026-11-27T12:00:00Z")), "2026-11-27T12:00:00+00:00")


class FeatureTest(unittest.TestCase):
    def setUp(self):
        self.feature = chapters.load_features(chapters.find("schedules"), ROOT)[0]

    def test_a_scenario_for_each_promise_and_for_the_time_zone(self):
        names = [s.name for s in self.feature.scenarios]
        self.assertIn("A schedule fires on time while other work is slow", names)
        self.assertIn("A run knows its interval", names)
        self.assertIn("A backlog drains in order at its limit", names)
        self.assertIn("A schedule with a time zone fires at the local time it names", names)
        self.assertEqual([p for p in promises.PROMISES if promises.row_of(p) == "Schedules"],
                         ["schedules.on_time", "schedules.interval", "schedules.backlog"])

    def test_every_step_has_exactly_one_definition(self):
        problems, _notes = gherkin.check(self.feature, chapter.steps)
        self.assertEqual(problems, [])

    def test_the_limit_of_the_feature_is_the_limit_of_the_rule(self):
        self.assertEqual(chapter_rules.LATE_LIMIT_SECONDS, 60)
        self.assertIn("within 60 seconds", "\n".join(str(step) for s in self.feature.scenarios
                                                       for step in s.steps))

    def test_no_scenario_is_a_gap_yet(self):
        self.assertEqual([s.name for s in self.feature.scenarios if s.is_gap], [])


class EveryFireTest(unittest.TestCase):
    def test_it_holds_on_the_healthy_snapshot(self):
        verdict = judge(healthy())[EVERY]
        self.assertEqual(verdict.state, engine.HOLDS, verdict.details)
        self.assertIn("20 fire times, each with exactly one run", verdict.evidence)

    def test_a_fire_time_with_no_run(self):
        data = healthy()
        fires(data).remove(fire(data, "a101", 10))
        verdict = judge(data)[EVERY]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertEqual(verdict.details, ["team_a/standing_rollup (its package's own schedule): "
                                           "no run for the fire time 2026-11-27T12:10:00Z"])

    def test_a_fire_time_with_two_runs(self):
        data = healthy()
        fires(data).append(dict(fire(data, "a102", 15), run_id="5e2e0000-0000-4000-8000-00000000a1ff"))
        verdict = judge(data)[EVERY]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("2 runs for the fire time 2026-11-27T12:15:00Z", verdict.details[0])

    def test_a_schedule_registered_twice(self):
        data = healthy()
        rows = data["tenants"]["team_b"]["db"]["schedules"]["rows"]
        rows.append(dict(rows[0], id="5c5e0000-0000-4000-8000-00000000b1ee"))
        verdict = judge(data)[EVERY]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("team_b/standing_rollup (its package's own schedule): registered 2 times",
                      verdict.details[0])

    def test_a_schedule_that_is_not_registered(self):
        data = healthy()
        rows = data["tenants"]["team_c"]["db"]["schedules"]["rows"]
        rows[:] = [row for row in rows if row.get("instance_name") != "drive"]
        verdict = judge(data)[EVERY]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("team_c/standing_driver/drive: no schedule is registered", verdict.details)

    def test_a_missed_fire_while_the_server_restarted_is_not_counted(self):
        data = healthy()
        fires(data).remove(fire(data, "a101", 10))
        data["disturbances"] = [{"id": "d1", "kind": "restart_server", "start": "2026-11-27T12:09:30Z",
                                 "end": "2026-11-27T12:10:40Z"}]
        verdict = judge(data)[EVERY]
        self.assertEqual(verdict.state, engine.HOLDS, verdict.details)
        self.assertIn("1 fire times while no scheduler could run not counted", verdict.evidence)

    def test_a_catch_up_schedule_still_draining_is_not_missing(self):
        data = healthy()
        fires(data).remove(fire(data, "a103", 15))
        self.assertEqual(judge(data)[EVERY].state, engine.HOLDS)
        fires(data).remove(fire(data, "a103", 10))
        fires(data).append(dict(fire(data, "a102", 15), schedule_id="5c5e0000-0000-4000-8000-00000000a103"))
        verdict = judge(data)[EVERY]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("standing_extract/in_order: no run for the fire time 2026-11-27T12:10:00Z",
                      verdict.details[0])

    def test_a_run_at_a_time_that_is_not_a_fire_time(self):
        data = healthy()
        fires(data).append(dict(fire(data, "a101", 15), scheduled_time="2026-11-27T12:13:00"))
        verdict = judge(data)[EVERY]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("a run for 2026-11-27T12:13:00Z, which is not a fire time", verdict.details[0])

    def test_a_backfill_row_is_not_a_schedule_of_the_workload(self):
        data = healthy()
        rows = data["tenants"]["team_a"]["db"]["schedules"]["rows"]
        self.assertTrue(any(row.get("backfill") for row in rows))
        self.assertEqual(judge(data)[EVERY].state, engine.HOLDS)


class OnTimeTest(unittest.TestCase):
    def test_it_holds_and_says_how_many_fires_were_while_work_was_slow(self):
        verdict = judge(healthy())[ON_TIME]
        self.assertEqual(verdict.state, engine.HOLDS, verdict.details)
        self.assertIn("16 runs started on time, 4 of them while the long job or held tasks ran",
                      verdict.evidence)

    def test_a_late_run(self):
        data = healthy()
        fire(data, "a105", 10)["started_at"] = "2026-11-27T12:11:30.000"
        verdict = judge(data)[ON_TIME]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("standing_flaky (its package's own schedule): the run for "
                      "2026-11-27T12:10:00Z started 90 s after it (limit 60 s)", verdict.details[0])

    def test_a_late_run_of_a_backlog_is_judged_by_its_own_rule(self):
        data = healthy()
        fire(data, "a103", 10)["started_at"] = "2026-11-27T12:13:00.000"
        self.assertEqual(judge(data)[ON_TIME].state, engine.HOLDS)


class IntervalTest(unittest.TestCase):
    def test_it_holds_on_the_healthy_snapshot(self):
        verdict = judge(healthy())[INTERVAL]
        self.assertEqual(verdict.state, engine.HOLDS, verdict.details)
        self.assertIn("12 runs each wrote the interval of their fire time", verdict.evidence)

    def test_a_run_with_the_interval_of_another_fire_time(self):
        data = healthy()
        fire(data, "a101", 15)["rollup_done"]["interval_start"] = "2026-11-27T12:00:00+00:00"
        verdict = judge(data)[INTERVAL]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("wrote 2026-11-27T12:00:00Z to 2026-11-27T12:15:00Z, not "
                      "2026-11-27T12:10:00Z to 2026-11-27T12:15:00Z", verdict.details[0])

    def test_a_completed_run_with_no_interval(self):
        data = healthy()
        fire(data, "a102", 10)["extract_loaded"] = None
        verdict = judge(data)[INTERVAL]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("wrote no interval", verdict.details[0])


class BacklogTest(unittest.TestCase):
    def test_it_holds_on_the_healthy_snapshot(self):
        verdict = judge(healthy())[BACKLOG]
        self.assertEqual(verdict.state, engine.HOLDS, verdict.details)
        self.assertIn("4 runs in the order of their fire times", verdict.evidence)

    def test_a_drained_backlog_in_order(self):
        data = healthy()
        fire(data, "a103", 10).update(started_at="2026-11-27T12:16:00.000",
                                      completed_at="2026-11-27T12:16:04.000")
        fire(data, "a103", 15).update(started_at="2026-11-27T12:16:05.000",
                                      completed_at="2026-11-27T12:16:09.000")
        verdict = judge(data)[BACKLOG]
        self.assertEqual(verdict.state, engine.HOLDS, verdict.details)
        self.assertIn("2 of them started late, from a backlog", verdict.evidence)

    def test_a_backlog_that_ran_newest_first(self):
        data = healthy()
        fire(data, "a103", 10).update(started_at="2026-11-27T12:16:05.000",
                                      completed_at="2026-11-27T12:16:09.000")
        fire(data, "a103", 15).update(started_at="2026-11-27T12:16:00.000",
                                      completed_at="2026-11-27T12:16:04.000")
        verdict = judge(data)[BACKLOG]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("the run for 2026-11-27T12:15:00Z started before the run for "
                      "2026-11-27T12:10:00Z", verdict.details[0])

    def test_two_runs_active_at_once_over_a_limit_of_one(self):
        data = healthy()
        fire(data, "a103", 10).update(started_at="2026-11-27T12:16:00.000",
                                      completed_at="2026-11-27T12:17:00.000")
        fire(data, "a103", 15).update(started_at="2026-11-27T12:16:05.000",
                                      completed_at="2026-11-27T12:16:09.000")
        verdict = judge(data)[BACKLOG]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("2 runs active when the run for 2026-11-27T12:15:00Z started (limit 1)",
                      verdict.details[0])


class SqlTest(unittest.TestCase):
    def test_the_queries_are_for_a_tenant_schema(self):
        for sql in (chapter_rules.SCHEDULES_SQL, chapter_rules.FIRES_SQL, chapter_rules.BUSY_SQL):
            self.assertIn("{schema}.", sql)
            self.assertTrue(sql.startswith("SELECT"))
        self.assertIn("schedule_executions", chapter_rules.FIRES_SQL)


if __name__ == "__main__":
    unittest.main()
