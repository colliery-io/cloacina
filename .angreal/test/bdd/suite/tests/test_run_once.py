"""The chapter run_once (CLOACINA-T-1110): its feature, its rules, the log
source that reads the marks, and the marks the fixtures print. No server, no
cluster."""

from __future__ import annotations

import ast
import json
import os
import sys
import tempfile
import unittest
from unittest import mock

from suite import engine, promises, rules, sources, workload

HERE = os.path.dirname(os.path.abspath(__file__))
BDD = os.path.dirname(os.path.dirname(HERE))
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(BDD)))
HEALTHY = os.path.join(HERE, "recorded", "healthy.json")

# The folder that holds the `bdd` package.
sys.path.insert(0, os.path.dirname(BDD))

from bdd import chapters, gherkin  # noqa: E402
from bdd import run_once_steps as chapter  # noqa: E402

LONG = "run_once.long_task"
WAITED = "run_once.no_free_agent"
OTHER = "run_once.other_replica"


def judge(data):
    registry = rules.load()
    snapshot = engine.Snapshot(data)
    return {rule.id: rule.judge(snapshot) for rule in registry.rules}


def healthy():
    return sources.read_snapshot(HEALTHY)


def team_a(data):
    return data["tenants"]["team_a"]


def task_row(data, task_id_end):
    return next(row for row in team_a(data)["db"]["run_once_tasks"]["rows"]
                if row["task_id"].endswith(task_id_end))


def marks(data):
    return team_a(data)["logs"]["marks"]["rows"]


class FeatureTest(unittest.TestCase):
    def setUp(self):
        self.feature = chapters.load_features(chapters.find("run_once"), ROOT)[0]

    def test_one_scenario_for_each_promise_of_the_row(self):
        self.assertEqual(len(self.feature.scenarios), 3)
        ids = [p for p in promises.PROMISES if promises.row_of(p) == "Run once"]
        self.assertEqual(ids, [LONG, WAITED, OTHER])
        for scenario in self.feature.scenarios:
            self.assertEqual(scenario.steps[-1].kind, "Then", scenario.name)

    def test_no_scenario_is_a_gap_yet(self):
        self.assertEqual([s.name for s in self.feature.scenarios if s.is_gap], [])

    def test_every_step_has_exactly_one_definition(self):
        problems, _notes = gherkin.check(self.feature, chapter.steps)
        self.assertEqual(problems, [])

    def test_the_long_scenario_holds_past_five_minutes(self):
        self.assertGreater(chapter.LONG_HOLD_SECONDS, 300)
        self.assertGreater(chapter.FILL_HOLD_SECONDS, 300 + chapter.WAIT_CHECK_SECONDS)


class RuleTest(unittest.TestCase):
    def test_each_rule_holds_on_the_healthy_snapshot(self):
        verdicts = judge(healthy())
        self.assertEqual(verdicts[LONG].state, engine.HOLDS, verdicts[LONG].evidence)
        self.assertIn("1 long tasks each ran one time", verdicts[LONG].evidence)
        self.assertEqual(verdicts[WAITED].state, engine.HOLDS, verdicts[WAITED].evidence)
        self.assertEqual(verdicts[OTHER].state, engine.HOLDS, verdicts[OTHER].evidence)

    def test_a_long_task_that_ran_twice_is_a_break_with_its_run_and_task(self):
        data = healthy()
        second = dict(marks(data)[1], pod="cloacina-agent-7d4b9-a2", at="2026-11-27T12:01:05.000Z")
        marks(data).append(second)
        verdict = judge(data)[LONG]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("team_a/standing_archive run 5e100000-0000-4000-8000-00000000a001",
                      verdict.details[0])
        self.assertIn("task archive: its body started 2 times", verdict.details[0])

    def test_a_task_that_never_ran_is_a_break(self):
        data = healthy()
        marks(data)[:] = [m for m in marks(data) if "standing_archive" not in m["text"]]
        verdict = judge(data)[LONG]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("never ran (no mark)", verdict.details[0])

    def test_a_second_start_next_to_a_killed_agent_is_not_counted(self):
        data = healthy()
        marks(data).append(dict(marks(data)[1], pod="cloacina-agent-7d4b9-a2",
                                at="2026-11-27T12:02:00.000Z"))
        data["disturbances"] = [{"id": "d1", "kind": "kill_agent", "target": "team_a",
                                 "start": "2026-11-27T12:01:50Z", "end": "2026-11-27T12:01:55Z"}]
        verdict = judge(data)[LONG]
        self.assertEqual(verdict.state, engine.HOLDS, verdict.evidence)
        self.assertIn("1 next to a killed agent not counted", verdict.evidence)

    def test_a_long_task_that_held_too_little(self):
        data = healthy()
        task_row(data, "a001")["held_seconds"] = "120.0"
        verdict = judge(data)[LONG]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("held 120.0 s", verdict.details[0])

    def test_a_retry_is_one_more_start_not_a_second_run(self):
        data = healthy()
        task_row(data, "a001")["attempt"] = 2
        marks(data).append(dict(marks(data)[1], at="2026-11-27T12:01:05.000Z"))
        self.assertEqual(judge(data)[LONG].state, engine.HOLDS)

    def test_a_task_that_ended_just_now_is_not_judged_yet(self):
        data = healthy()
        task_row(data, "a001")["completed_at"] = "2026-11-27T12:19:00.000"
        self.assertEqual(judge(data)[LONG].state, engine.UNKNOWN)

    def test_no_mark_at_all_cannot_be_judged(self):
        data = healthy()
        marks(data)[:] = []
        verdict = judge(data)[WAITED]
        self.assertEqual(verdict.state, engine.UNKNOWN)
        self.assertIn("the agents' output is not read", verdict.evidence)

    def test_the_marks_are_away_without_a_kubeconfig(self):
        data = healthy()
        team_a(data)["logs"]["marks"] = {"error": "no kubeconfig was given"}
        self.assertEqual(judge(data)[LONG].state, engine.UNKNOWN)

    def test_a_waiting_task_that_failed_without_running(self):
        data = healthy()
        row = task_row(data, "a002")
        row["status"] = "Failed"
        row["attempt"] = 3
        marks(data)[:] = [m for m in marks(data) if "standing_hold" not in m["text"]]
        verdict = judge(data)[WAITED]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("standing_hold", verdict.details[0])
        self.assertIn("never ran (no mark), the server recorded 3 attempt(s) and Failed",
                      verdict.details[0])

    def test_one_replica_cannot_judge_the_other_replica(self):
        data = healthy()
        data["admin"]["logs"]["results_elsewhere"]["pods"] = ["bdd-standing-cloacina-server-5f7c9-a"]
        verdict = judge(data)[OTHER]
        self.assertEqual(verdict.state, engine.UNKNOWN)
        self.assertIn("1 ready replica(s)", verdict.evidence)

    def test_a_result_on_the_other_replica_that_ran_twice(self):
        data = healthy()
        marks(data).append(dict(marks(data)[0], pod="cloacina-agent-7d4b9-a1",
                                at="2026-11-27T11:23:40.000Z"))
        verdict = judge(data)[OTHER]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("standing_hold", verdict.details[0])

    def test_a_stored_result_that_nobody_took(self):
        data = healthy()
        task_row(data, "a004")["status"] = "Running"
        data["admin"]["db"]["run_once_global_inbox"]["rows"].append(
            {"task_execution_id": "7a500000-0000-4000-8000-00000000a004",
             "created_at": "2026-11-27T12:05:01"})
        verdict = judge(data)[OTHER]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("was not taken, the task is Running", verdict.details[0])

    def test_a_stored_duplicate_of_a_completed_task_is_not_a_fault(self):
        data = healthy()
        data["admin"]["db"]["run_once_global_inbox"]["rows"].append(
            {"task_execution_id": "7a500000-0000-4000-8000-00000000a003",
             "created_at": "2026-11-27T12:05:01"})
        self.assertEqual(judge(data)[OTHER].state, engine.HOLDS)

    def test_the_task_id_of_a_json_log_line(self):
        from suite.rules import run_once

        text = ('{"level":"INFO","fields":{"task_id":"7A500000-0000-4000-8000-00000000A002",'
                '"message":"agent result is for no dispatch this replica waits for: stored for '
                'the replica that does"}}')
        self.assertEqual(run_once.TASK_ID.search(text).group(1).lower(),
                         "7a500000-0000-4000-8000-00000000a002")


class LogSourceTest(unittest.TestCase):
    OUTPUT = (
        "[pod/cloacina-agent-7d4b9-a1/agent] 2026-11-27T12:00:02.611234567Z INFO started\n"
        "[pod/cloacina-agent-7d4b9-a1/agent] 2026-11-27T12:00:02.611234567Z "
        'STANDING_MARK {"workflow": "standing_archive", "task": "archive"}\n'
        "[pod/cloacina-agent-7d4b9-a2/agent] 2026-11-27T12:00:03.000000001Z "
        'worker 3: STANDING_MARK {"workflow": "standing_hold", "task": "hold"}\n'
    )

    def test_the_lines_with_the_marker_keep_their_pod_and_time(self):
        rows = sources.parse_log_lines(self.OUTPUT, "STANDING_MARK")
        self.assertEqual(len(rows), 2)
        self.assertEqual(rows[0]["pod"], "cloacina-agent-7d4b9-a1")
        self.assertEqual(rows[0]["at"], "2026-11-27T12:00:02.611234567Z")
        self.assertTrue(rows[0]["text"].startswith("STANDING_MARK "))
        from suite.rules import run_once

        self.assertEqual(run_once.parse_mark(rows[1]["text"])["task"], "hold")

    def test_lines_read_once_stay_after_their_pod_is_gone(self):
        answers = []

        def run(args, **_kwargs):
            proc = mock.Mock(returncode=0, stderr="")
            proc.stdout = answers.pop(0)
            return proc

        with tempfile.TemporaryDirectory() as folder:
            logs = sources.Logs("/nowhere/kubeconfig", "ns", folder)
            now = engine.parse_time("2026-11-27T12:01:00Z")
            with mock.patch.object(sources.subprocess, "run", side_effect=run) as called:
                answers.extend(["cloacina-agent-7d4b9-a1 true\ncloacina-agent-7d4b9-a2 true\n",
                                self.OUTPUT])
                first = logs.read("agents", "team_a", "marks", "STANDING_MARK", now=now)
                args = called.call_args_list[1][0][0]
                self.assertIn("--since=10800s", args)
                self.assertIn("cloacina-tenant-team-a", args)
                # The pod a1 is gone; a2 prints nothing new.
                answers.extend(["cloacina-agent-7d4b9-a2 true\n", ""])
                second = logs.read("agents", "team_a", "marks", "STANDING_MARK", now=now + 60)
                self.assertIn("--since=300s", called.call_args_list[3][0][0])
            self.assertEqual(len(first["rows"]), 2)
            self.assertEqual(second["rows"], first["rows"])
            self.assertEqual(second["pods"], ["cloacina-agent-7d4b9-a2"])

    def test_a_failed_kubectl_is_a_source_that_is_away(self):
        def run(args, **_kwargs):
            return mock.Mock(returncode=1, stdout="", stderr="Unable to connect to the server")

        with tempfile.TemporaryDirectory() as folder:
            logs = sources.Logs(None, "ns", folder)
            with mock.patch.object(sources.subprocess, "run", side_effect=run):
                entry = logs.read("server", None, "results_elsewhere", "stored")
        self.assertIn("Unable to connect", entry["error"])

    def test_the_snapshot_holds_a_log_source_of_each_place(self):
        registry = rules.load()

        class Reader:
            def read(self, place, tenant, name, marker):
                return {"rows": [], "pods": [f"{place}-{tenant}"]}

        class Quiet:
            def get(self, path, key):
                return {"status": 200, "body": {"items": []}}

            def query(self, sql):
                return {"rows": []}

        data = sources.collect(registry, Quiet(), Quiet(), {"team_a": "k"}, logs=Reader())
        self.assertEqual(data["admin"]["logs"]["results_elsewhere"]["pods"], ["server-None"])
        self.assertEqual(data["tenants"]["team_a"]["logs"]["marks"]["pods"], ["agents-team_a"])
        bare = sources.collect(registry, Quiet(), Quiet(), {"team_a": "k"})
        self.assertIn("no kubeconfig", bare["tenants"]["team_a"]["logs"]["marks"]["error"])

    def test_the_agent_namespace_is_the_drivers(self):
        from bdd.world import PublishWorld

        for tenant in workload.TENANTS + ("public", "team_x_1"):
            self.assertEqual(workload.agent_namespace(tenant),
                             PublishWorld.tenant_namespace(None, tenant))


class FixtureMarkTest(unittest.TestCase):
    """Each task of the row's workflows prints a mark that names its run."""

    PYTHON = ("standing-archive-py", "standing-hold-py", "standing-extract-py")

    def test_the_python_marks_carry_the_schedule_and_the_tag(self):
        for fixture in self.PYTHON:
            folder = os.path.join(ROOT, "examples", "fixtures", fixture, "workflow")
            (package,) = os.listdir(folder)
            with open(os.path.join(folder, package, "tasks.py"), encoding="utf-8") as handle:
                source = handle.read()
            ast.parse(source)
            for key in ("schedule_id", "scheduled_time", workload.RUN_TAG_KEY):
                self.assertIn(f'context.get("{key}", None)', source, f"{fixture}: {key}")
            self.assertIn(f'"{workload.MARK}"', source, fixture)

    def test_the_rust_marks_carry_the_schedule_and_the_tag(self):
        path = os.path.join(ROOT, "examples", "fixtures", "standing-rollup-rust", "src", "lib.rs")
        with open(path, encoding="utf-8") as handle:
            source = handle.read()
        self.assertEqual(source.count('"schedule_id": context.get("schedule_id").cloned()'), 2)
        self.assertEqual(source.count('"run_once_tag": context.get("run_once_tag").cloned()'), 2)

    def test_the_self_driving_holds_carry_a_tag(self):
        path = os.path.join(ROOT, "examples", "fixtures", "standing-driver-py", "workflow",
                            "standing_driver", "tasks.py")
        with open(path, encoding="utf-8") as handle:
            source = handle.read()
        self.assertIn('"run_once_tag": f"hold-{uuid.uuid4()}"', source)

    def test_a_mark_names_its_run(self):
        from suite.rules import run_once

        scheduled = {"schedule_id": "s1", "scheduled_time": "2026-11-27T12:00:00+00:00"}
        self.assertEqual(run_once.run_key(scheduled), "schedule s1 at 2026-11-27T12:00:00+00:00")
        self.assertEqual(run_once.run_key({"run_once_tag": "t1", **scheduled}), "tag t1")
        self.assertIsNone(run_once.run_key({"scheduled_time": None}))
        self.assertIsNone(run_once.parse_mark("STANDING_MARK not json"))
        self.assertEqual(json.loads('{"a": 1}'), run_once.parse_mark('STANDING_MARK {"a": 1}'))


if __name__ == "__main__":
    unittest.main()
