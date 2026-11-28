"""The rules of the chapter "Failure" (CLOACINA-T-1112), on the recorded
snapshot and on that snapshot with one fact changed.

`recorded/healthy.json` holds three disturbances that are over: Postgres away
from 09:02 to 09:14, a server replica restarted at 10:02, an agent pod of
team_a killed at 11:02, with the runs and the fire times around them.
"""

from __future__ import annotations

import os
import unittest

from suite import engine, rules, sources
from suite.rules import failure

HERE = os.path.dirname(os.path.abspath(__file__))
HEALTHY = os.path.join(HERE, "recorded", "healthy.json")


def healthy():
    return sources.read_snapshot(HEALTHY)


def judge(data, rule_id):
    snapshot = engine.Snapshot(data)
    rule = next(rule for rule in rules.load().rules if rule.id == rule_id)
    return rule.judge(snapshot)


def run(data, tenant, ident_end):
    return next(row for row in data["tenants"][tenant]["db"]["failure_runs"]["rows"]
                if row["id"].endswith(ident_end))


def fires(data, tenant):
    return data["tenants"][tenant]["db"]["failure_fires"]["rows"]


def window(data, kind):
    return [line for line in data["disturbances"] if line["kind"] == kind][-1]


class HealthyTest(unittest.TestCase):
    def test_each_rule_holds_and_says_what_it_read(self):
        data = healthy()
        expected = {
            "failure.agent_death": "2 runs active at 1 kill_agent",
            "failure.server_restart": "5 runs active at 1 restart_server",
            "failure.postgres_fire_times": "15 schedules over 1 postgres_away",
            "failure.postgres_runs_end": "3 runs active at 1 postgres_away",
        }
        for rule_id, evidence in expected.items():
            verdict = judge(data, rule_id)
            self.assertEqual(verdict.state, engine.HOLDS, f"{rule_id}: {verdict.evidence} {verdict.details}")
            self.assertIn(evidence, verdict.evidence, rule_id)

    def test_the_rules_say_how_each_active_run_ended(self):
        verdict = judge(healthy(), "failure.agent_death")
        self.assertTrue(any("team_a standing_archive f1a00000: Completed" in d and "cloacina-agent-7f6b9-m3n4p" in d
                            for d in verdict.details), verdict.details)
        verdict = judge(healthy(), "failure.server_restart")
        self.assertTrue(all("(force, 2 replicas)" in d for d in verdict.details), verdict.details)


class AgentDeathTest(unittest.TestCase):
    def test_a_run_still_running_after_the_window(self):
        data = healthy()
        row = run(data, "team_a", "a1100")
        row["status"], row["completed_at"] = "Running", None
        verdict = judge(data, "failure.agent_death")
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("not Completed after the window", verdict.details[0])

    def test_a_run_that_failed(self):
        data = healthy()
        run(data, "team_a", "a1101")["status"] = "Failed"
        self.assertEqual(judge(data, "failure.agent_death").state, engine.BROKEN)

    def test_a_forced_kill_and_the_task_finished_on_the_killed_pod(self):
        data = healthy()
        run(data, "team_a", "a1100")["finished_on"] = "cloacina-agent-7f6b9-q8r2s"
        verdict = judge(data, "failure.agent_death")
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("finished on the pod that was killed", verdict.details[0])

    def test_a_plain_stop_lets_the_task_finish_in_its_drain(self):
        data = healthy()
        window(data, "kill_agent")["mode"] = "sigterm"
        run(data, "team_a", "a1100")["finished_on"] = "cloacina-agent-7f6b9-q8r2s"
        self.assertEqual(judge(data, "failure.agent_death").state, engine.HOLDS)

    def test_runs_of_another_tenant_are_not_judged(self):
        data = healthy()
        run(data, "team_b", "b1100")["status"] = "Failed"
        self.assertEqual(judge(data, "failure.agent_death").state, engine.HOLDS)

    def test_a_window_still_settling_is_not_judged_yet(self):
        data = healthy()
        line = window(data, "kill_agent")
        line["settle_until"] = "2026-11-27T12:30:00Z"
        run(data, "team_a", "a1100")["status"] = "Running"
        verdict = judge(data, "failure.agent_death")
        self.assertEqual(verdict.state, engine.UNKNOWN)
        self.assertIn("still settling", verdict.evidence)

    def test_a_kill_that_did_not_happen_is_not_judged(self):
        data = healthy()
        window(data, "kill_agent")["outcome"] = "the delete failed: forbidden"
        verdict = judge(data, "failure.agent_death")
        self.assertEqual(verdict.state, engine.UNKNOWN)

    def test_no_kill_is_cannot_judge(self):
        data = healthy()
        data["disturbances"] = [d for d in data["disturbances"] if d["kind"] != "kill_agent"]
        self.assertEqual(judge(data, "failure.agent_death").state, engine.UNKNOWN)


class ServerRestartTest(unittest.TestCase):
    def test_a_task_sent_again_across_the_restart(self):
        data = healthy()
        run(data, "team_b", "b1001")["recoveries"] = 1
        verdict = judge(data, "failure.server_restart")
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("sent again across the restart", verdict.details[0])

    def test_a_run_that_did_not_complete(self):
        data = healthy()
        run(data, "team_c", "c1000")["status"] = "Failed"
        self.assertEqual(judge(data, "failure.server_restart").state, engine.BROKEN)

    def test_a_run_that_ended_before_the_restart_is_not_judged(self):
        data = healthy()
        row = run(data, "team_c", "c1000")
        row["status"], row["completed_at"] = "Failed", "2026-11-27T10:01:00"
        self.assertEqual(judge(data, "failure.server_restart").state, engine.HOLDS)


class PostgresFireTimesTest(unittest.TestCase):
    def test_a_lost_fire_time_of_a_run_all_schedule(self):
        data = healthy()
        rows = fires(data, "team_a")
        rows[:] = [r for r in rows if not (r["instance_name"] == "in_order"
                                           and r["scheduled_time"].endswith("09:10:00"))]
        verdict = judge(data, "failure.postgres_fire_times")
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("team_a/standing_extract/in_order: run_all, and no run for the fire time(s) "
                      "2026-11-27T09:10:00Z", verdict.details[0])

    def test_a_doubled_fire_time(self):
        data = healthy()
        rows = fires(data, "team_b")
        rows.append(dict(next(r for r in rows if r["workflow_name"] == "standing_rollup")))
        verdict = judge(data, "failure.postgres_fire_times")
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("team_b/standing_rollup: two runs for the fire time(s) 2026-11-27T09:05:00Z",
                      verdict.details[0])

    def test_a_skip_schedule_with_no_run_for_the_window(self):
        data = healthy()
        rows = fires(data, "team_c")
        rows[:] = [r for r in rows if r["workflow_name"] != "standing_flaky"]
        rows.append({"workflow_name": "standing_flaky", "instance_name": None, "catchup_policy": "skip",
                     "cron_expression": "*/5 * * * *", "scheduled_time": None, "status": None})
        verdict = judge(data, "failure.postgres_fire_times")
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("team_c/standing_flaky: skip, and no run for the fire times of the window",
                      verdict.details[0])

    def test_a_skip_schedule_needs_one_run_not_one_for_each(self):
        # team_b's skip schedules have one run (09:05) for the fire times
        # 09:05 and 09:10: that is the rule of `skip`.
        verdict = judge(healthy(), "failure.postgres_fire_times")
        self.assertTrue(any("team_b/standing_rollup (skip)" in d and "runs 1, 0" in d
                            for d in verdict.details), verdict.details)

    def test_a_run_of_the_window_that_has_not_ended(self):
        data = healthy()
        next(r for r in fires(data, "team_a") if r["instance_name"] == "in_order")["status"] = "Running"
        verdict = judge(data, "failure.postgres_fire_times")
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("has not ended", verdict.details[0])

    def test_the_open_line_alone_is_a_window_still_open(self):
        data = healthy()
        data["disturbances"] = [d for d in data["disturbances"]
                                if not (d["kind"] == "postgres_away" and d.get("end"))]
        self.assertEqual(judge(data, "failure.postgres_fire_times").state, engine.UNKNOWN)


class PostgresRunsEndTest(unittest.TestCase):
    def test_a_run_stuck_after_postgres_came_back(self):
        data = healthy()
        row = run(data, "team_b", "b0902")
        row["status"], row["completed_at"] = "Running", None
        verdict = judge(data, "failure.postgres_runs_end")
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("has not ended after the window", verdict.details[0])

    def test_the_job_that_fails_on_purpose_may_fail(self):
        verdict = judge(healthy(), "failure.postgres_runs_end")
        self.assertEqual(verdict.state, engine.HOLDS)
        self.assertTrue(any("standing_flaky" in d and "Failed" in d for d in verdict.details))

    def test_another_job_that_failed_is_lost_work(self):
        data = healthy()
        run(data, "team_a", "a0902")["status"] = "Failed"
        self.assertEqual(judge(data, "failure.postgres_runs_end").state, engine.BROKEN)

    def test_a_task_sent_again(self):
        data = healthy()
        run(data, "team_b", "b0902")["recoveries"] = 2
        self.assertEqual(judge(data, "failure.postgres_runs_end").state, engine.BROKEN)


class FireTimesTest(unittest.TestCase):
    def test_the_forms_of_the_workload(self):
        start = engine.parse_time("2026-11-27T09:02:00Z")
        end = engine.parse_time("2026-11-27T09:14:10Z")
        self.assertEqual([engine.format_time(t) for t in failure.fire_times("*/5 * * * *", start, end)],
                         ["2026-11-27T09:05:00Z", "2026-11-27T09:10:00Z"])
        self.assertEqual(failure.fire_times("0 * * * *", start, end), [])
        self.assertEqual(len(failure.fire_times("0 * * * *", start - 3600, end)), 1)
        self.assertIsNone(failure.fire_times("0 9 * * 1", start, end))

    def test_a_fire_time_on_the_start_counts(self):
        start = engine.parse_time("2026-11-27T09:05:00Z")
        self.assertEqual(failure.fire_times("*/5 * * * *", start, start + 1), [start])


class SourcesTest(unittest.TestCase):
    def test_the_queries_name_the_tenants_schema(self):
        registry = rules.load()
        for name in ("failure_runs", "failure_fires"):
            self.assertIn("{schema}.", registry.db_needs[name])
        self.assertIn("recovery_attempts", registry.db_needs["failure_runs"])
        self.assertIn("schedule_executions", registry.db_needs["failure_fires"])


if __name__ == "__main__":
    unittest.main()
