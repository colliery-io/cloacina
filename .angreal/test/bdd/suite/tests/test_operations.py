"""The chapter "Operations" (CLOACINA-T-1115): its feature, its rules on
recorded snapshots, the alert receiver, the metrics reader and the
fixtures. No server, no cluster: the receiver runs on localhost."""

from __future__ import annotations

import contextlib
import io
import json
import os
import sys
import tempfile
import unittest
import urllib.request

from suite import alerts, engine, rules, sources, workload
from suite.rules import operations
from suite.tests.test_workload import Context, load_fixture

HERE = os.path.dirname(os.path.abspath(__file__))
BDD = os.path.dirname(os.path.dirname(HERE))
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(BDD)))
HEALTHY = os.path.join(HERE, "recorded", "healthy.json")

# The folder that holds the `bdd` package.
sys.path.insert(0, os.path.dirname(BDD))

from bdd import chapters, gherkin  # noqa: E402
from bdd import operations_steps as chapter  # noqa: E402


def healthy():
    return sources.read_snapshot(HEALTHY)


def judge(data):
    registry = rules.load()
    snapshot = engine.Snapshot(data)
    return {rule.id: rule.judge(snapshot) for rule in registry.rules}


def tenant_rows(data, tenant, name):
    return data["tenants"][tenant]["db"][name]["rows"]


def audit_rows(data):
    return data["admin"]["db"]["operations_audit"]["rows"]


class FeatureTest(unittest.TestCase):
    def setUp(self):
        self.feature = chapters.load_features(chapters.find("operations"), ROOT)[0]

    def test_every_step_has_exactly_one_definition(self):
        problems, _notes = gherkin.check(self.feature, chapter.steps)
        self.assertEqual(problems, [])

    def test_no_scenario_is_a_gap(self):
        # The attempt output is CLOACINA-T-1091, which is in.
        gaps = {s.name: s.gap_tasks for s in self.feature.scenarios if s.is_gap}
        self.assertEqual(gaps, {})

    def test_the_fixtures(self):
        with contextlib.redirect_stdout(io.StringIO()) as printed:
            self.check_the_fixtures()
        self.assertIn(f"{chapter.OUTPUT_PREFIX} m1 mode=fail", printed.getvalue())

    def check_the_fixtures(self):
        fail = load_fixture(chapter.FAIL_PACKAGE, chapter.FAIL_WORKFLOW)
        with self.assertRaises(RuntimeError) as raised:
            fail.wobble(Context({"mode": "fail", "marker": "m1"}))
        self.assertNotIn("m1", str(raised.exception))
        marker = f"test-{os.getpid()}-{os.urandom(4).hex()}"
        path = fail._memory_path(marker)
        try:
            with self.assertRaises(RuntimeError):
                fail.wobble(Context({"mode": "fail_once", "marker": marker}))
            context = Context({"mode": "fail_once", "marker": marker})
            fail.wobble(context)
            self.assertEqual(context.values[fail.RESULT_KEY]["marker"], marker)
        finally:
            if os.path.exists(path):
                os.remove(path)
        self.assertEqual(fail.OUTPUT_PREFIX, chapter.OUTPUT_PREFIX)
        keeper = load_fixture(chapter.KEEPER_PACKAGE, chapter.KEEPER_WORKFLOW)
        self.assertEqual(keeper.POOL, "keeper")
        for package, module in ((chapter.FAIL_PACKAGE, chapter.FAIL_WORKFLOW),
                                (chapter.KEEPER_PACKAGE, chapter.KEEPER_WORKFLOW)):
            with open(os.path.join(ROOT, "examples", "fixtures", package, "package.toml"),
                      encoding="utf-8") as handle:
                self.assertIn(f'workflow_name = "{module}"', handle.read())

    def test_the_driver_key_name_is_the_one_the_world_makes(self):
        with open(os.path.join(BDD, "standing_world.py"), encoding="utf-8") as handle:
            self.assertIn('f"{tenant}-self-driving"', handle.read())
        self.assertEqual(operations.DRIVER_KEY_NAME.format(tenant="team_a"), "team_a-self-driving")

    def test_the_retention_period_is_the_deployment_s(self):
        with open(os.path.join(BDD, "standing.py"), encoding="utf-8") as handle:
            self.assertIn("RUN_HISTORY_DAYS = workload.RUN_HISTORY_DAYS", handle.read())
        self.assertEqual(operations.RETENTION_SECONDS, workload.RUN_HISTORY_DAYS * 86400)


class ReceiverTest(unittest.TestCase):
    def test_it_keeps_each_alert_and_serves_them_back(self):
        with tempfile.TemporaryDirectory() as folder:
            path = os.path.join(folder, "alerts.jsonl")
            store = alerts.Store(path)
            server = alerts.serve(store, "127.0.0.1", 0)
            try:
                port = server.server_address[1]
                body = json.dumps({"event": "workflow_execution.failed", "tenant": "team_a",
                                   "execution_id": "e1"}).encode()
                request = urllib.request.Request(f"http://127.0.0.1:{port}/alerts", data=body,
                                                 method="POST")
                request.add_header("Content-Type", "application/json")
                request.add_header("X-Cloacina-Event", "workflow_execution.failed")
                request.add_header("X-Cloacina-Delivery", "e1")
                with urllib.request.urlopen(request, timeout=5) as answer:
                    self.assertEqual(answer.status, 200)
                with urllib.request.urlopen(f"http://127.0.0.1:{port}/alerts?since=0", timeout=5) as answer:
                    listed = json.loads(answer.read())
                with urllib.request.urlopen(f"http://127.0.0.1:{port}/health", timeout=5) as answer:
                    self.assertEqual(answer.status, 200)
                future = store.started_at + 3600
                with urllib.request.urlopen(f"http://127.0.0.1:{port}/alerts?since={future}",
                                            timeout=5) as answer:
                    self.assertEqual(json.loads(answer.read())["records"], [])
            finally:
                server.shutdown()
                server.server_close()
            self.assertEqual(len(listed["records"]), 1)
            record = listed["records"][0]
            self.assertEqual(record["delivery"], "e1")
            self.assertFalse(record["signed"])
            self.assertEqual(record["body"]["tenant"], "team_a")
            # A receiver that comes up again reads what it kept.
            self.assertEqual(len(alerts.Store(path).records), 1)

    def test_the_manifests_and_the_url(self):
        items = alerts.manifests("ns-x", "registry.local/agent:tag", "print('x')\n")
        self.assertEqual([item["kind"] for item in items], ["ConfigMap", "Deployment", "Service"])
        container = items[1]["spec"]["template"]["spec"]["containers"][0]
        self.assertEqual(container["image"], "registry.local/agent:tag")
        self.assertEqual(container["command"][:2], ["python3.11", "/receiver/alerts.py"])
        self.assertTrue(container["securityContext"]["readOnlyRootFilesystem"])
        self.assertEqual(items[0]["data"]["alerts.py"], "print('x')\n")
        self.assertEqual(alerts.webhook_url("ns-x"),
                         "http://cloacina-bdd-alerts.ns-x.svc.cluster.local:8090/alerts")

    def test_the_receiver_runs_on_the_python_of_the_agent_image(self):
        with open(os.path.join(ROOT, "docker", "Dockerfile.agent"), encoding="utf-8") as handle:
            self.assertIn("python3.11", handle.read())


class MetricsTest(unittest.TestCase):
    TEXT = ('# HELP cloacina_waiting_tasks Tasks that wait\n'
            '# TYPE cloacina_waiting_tasks gauge\n'
            'cloacina_waiting_tasks{pool="default",priority="5",tenant="team_a"} 3\n'
            'cloacina_waiting_tasks{pool="keeper",priority="5",tenant="team_a"} 1\n'
            'cloacina_schedule_lag_seconds{tenant="team_a"} 41.5\n'
            'process_open_fds 12\n'
            'not a series\n')

    def test_a_page_is_read_into_series(self):
        series = engine.parse_metrics(self.TEXT)
        self.assertIn(("process_open_fds", {}, 12.0), series)
        self.assertEqual(len(series), 4)
        self.assertEqual(chapter._gauge(series, "cloacina_waiting_tasks", tenant="team_a"), 4.0)
        self.assertEqual(chapter._gauge(series, "cloacina_waiting_tasks", tenant="team_a",
                                        pool="default"), 3.0)
        self.assertIsNone(chapter._gauge(series, "cloacina_waiting_tasks", tenant="team_b"))

    def test_a_snapshot_gives_the_series_of_one_metric(self):
        snapshot = engine.Snapshot({"taken_at": 1, "metrics": {"text": self.TEXT}})
        self.assertEqual(snapshot.metrics("cloacina_schedule_lag_seconds"),
                         [({"tenant": "team_a"}, 41.5)])

    def test_a_channel_that_is_away_is_cannot_judge(self):
        for data in ({"taken_at": 1}, {"taken_at": 1, "metrics": {"error": "refused"},
                                       "alerts": {"error": "refused"}}):
            snapshot = engine.Snapshot(data)
            with self.assertRaises(engine.Missing):
                snapshot.metrics("x")
            with self.assertRaises(engine.Missing):
                snapshot.alerts()

    def test_collect_reads_the_channels_when_it_is_given_them(self):
        store = alerts.Store()
        server = alerts.serve(store, "127.0.0.1", 0)
        try:
            port = server.server_address[1]
            data = sources.collect(rules.load(), _NoApi(), None, {}, now=1795770000.0,
                                   alerts_url=f"http://127.0.0.1:{port}/alerts",
                                   metrics_url=f"http://127.0.0.1:{port}/nothing")
        finally:
            server.shutdown()
            server.server_close()
        self.assertEqual(data["alerts"]["body"]["records"], [])
        self.assertIn("error", data["metrics"])
        self.assertNotIn("alerts", sources.collect(rules.load(), _NoApi(), None, {}, now=1.0))


class _NoApi:
    def get(self, path, key):
        raise AssertionError("no tenant, no call")


class HealthyTest(unittest.TestCase):
    def test_each_rule_holds_and_says_what_it_read(self):
        verdicts = judge(healthy())
        for rule_id in ("operations.alert_per_failed_run", "operations.no_alert_after_retry",
                        "operations.audit_of_driver", "operations.audit_rows_named",
                        "operations.backlog_gauge", "operations.lag_gauge", "operations.retention"):
            self.assertEqual(verdicts[rule_id].state, engine.HOLDS,
                             f"{rule_id}: {verdicts[rule_id].evidence}")
        self.assertIn("2 failed runs", verdicts["operations.alert_per_failed_run"].evidence)
        self.assertIn("11 actions", verdicts["operations.audit_of_driver"].evidence)
        self.assertIn("team_a 4/4", verdicts["operations.backlog_gauge"].evidence)
        self.assertIn("team_b 95/93 s", verdicts["operations.lag_gauge"].evidence)
        self.assertIn("1 of them keep", verdicts["operations.retention"].evidence)


class AlertTest(unittest.TestCase):
    def test_a_failed_run_with_no_alert(self):
        data = healthy()
        data["alerts"]["body"]["records"].pop(1)
        verdict = judge(data)["operations.alert_per_failed_run"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("team_b/standing_flaky run 6f1a0000-0000-4000-8000-0000000b1200", verdict.details[0])
        self.assertIn("no alert", verdict.details[0])

    def test_an_alert_that_names_another_tenant(self):
        data = healthy()
        data["alerts"]["body"]["records"][0]["body"]["tenant"] = "team_b"
        verdict = judge(data)["operations.alert_per_failed_run"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("the alert's tenant is 'team_b', not 'team_a'", verdict.details[0])

    def test_a_repeat_is_not_a_fault(self):
        data = healthy()
        records = data["alerts"]["body"]["records"]
        records.append(json.loads(json.dumps(records[0])))
        verdict = judge(data)["operations.alert_per_failed_run"]
        self.assertEqual(verdict.state, engine.HOLDS)
        self.assertIn("1 alerts came twice", verdict.evidence)

    def test_a_run_that_failed_before_the_receiver_came_up_is_not_judged(self):
        data = healthy()
        data["alerts"]["body"]["started_at"] = "2026-11-27T12:10:00Z"
        self.assertEqual(judge(data)["operations.alert_per_failed_run"].state, engine.UNKNOWN)

    def test_a_fresh_failure_is_not_judged_yet(self):
        data = healthy()
        data["alerts"]["body"]["records"].pop(1)
        tenant_rows(data, "team_b", "operations_failed_runs")[0]["completed_at"] = "2026-11-27T12:19:00"
        self.assertEqual(judge(data)["operations.alert_per_failed_run"].state, engine.HOLDS)

    def test_the_receiver_away_is_cannot_judge(self):
        data = healthy()
        data["alerts"] = {"error": "URLError: connection refused"}
        verdicts = judge(data)
        self.assertEqual(verdicts["operations.alert_per_failed_run"].state, engine.UNKNOWN)
        self.assertIn("receiver is away", verdicts["operations.alert_per_failed_run"].evidence)
        self.assertEqual(verdicts["operations.no_alert_after_retry"].state, engine.UNKNOWN)

    def test_an_alert_for_a_run_that_completed_after_a_retry(self):
        data = healthy()
        data["alerts"]["body"]["records"].append(
            {"received_at": 1795781110.0, "event": "workflow_execution.failed",
             "body": {"event": "workflow_execution.failed", "tenant": "team_a",
                      "workflow": "standing_flaky",
                      "execution_id": "6f1a0000-0000-4000-8000-0000000a1205"}})
        verdict = judge(data)["operations.no_alert_after_retry"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("completed after a retry and raised an alert", verdict.details[0])


class AuditTest(unittest.TestCase):
    def test_a_publish_with_no_row(self):
        data = healthy()
        audit_rows(data)[:] = [row for row in audit_rows(data) if row["id"] != 4130]
        verdict = judge(data)["operations.audit_of_driver"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("team_a: publish_broken at 2026-11-27T11:50:03Z: no package.upload row",
                      verdict.details)

    def test_a_row_written_by_another_key(self):
        data = healthy()
        row = next(row for row in audit_rows(data) if row["id"] == 4140)
        row["actor_key_name"] = "team_a-publish"
        verdict = judge(data)["operations.audit_of_driver"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("names the key 'team_a-publish', not 'team_a-self-driving'", verdict.details[0])

    def test_a_run_by_hand_whose_row_names_another_run(self):
        data = healthy()
        row = next(row for row in audit_rows(data) if row["id"] == 4143)
        row["detail"] = json.dumps({"execution_id": "7b1e0000-0000-4000-8000-00000000ffff"})
        self.assertEqual(judge(data)["operations.audit_of_driver"].state, engine.BROKEN)

    def test_a_burst_of_holds_needs_one_row_for_each_start(self):
        data = healthy()
        context = json.loads(data["tenants"]["team_a"]["db"]["driver_runs"]["rows"][2]["context"])
        context["standing_driver_actions"].append(
            {"action": "start_holds", "workflow": "standing_hold", "count": 2,
             "statuses": [202, 202], "at": "2026-11-27T12:15:05Z"})
        data["tenants"]["team_a"]["db"]["driver_runs"]["rows"][2]["context"] = json.dumps(context)
        audit_rows(data).append(
            {"id": 4150, "created_at": "2026-11-27T12:15:05.100", "tenant_id": "team_a",
             "actor_key_id": "c0de0000-0000-4000-8000-0000000a0001",
             "actor_key_name": "team_a-self-driving", "action": "workflow.run",
             "target": "standing_hold", "outcome": "ok", "detail": "{}"})
        verdict = judge(data)["operations.audit_of_driver"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("2 runs started, 1 audit rows", verdict.details[0])

    def test_a_row_with_no_key(self):
        data = healthy()
        audit_rows(data)[0]["actor_key_id"] = ""
        verdict = judge(data)["operations.audit_rows_named"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("audit row 4101 (package.upload, ok) names no key", verdict.details)

    def test_a_key_of_one_tenant_that_acted_in_another(self):
        data = healthy()
        audit_rows(data).append(
            {"id": 4160, "created_at": "2026-11-27T12:17:00", "tenant_id": "team_b",
             "actor_key_id": "c0de0000-0000-4000-8000-0000000a0001",
             "actor_key_name": "team_a-self-driving", "action": "workflow.run",
             "target": "standing_target_py", "outcome": "ok", "detail": "{}"})
        verdict = judge(data)["operations.audit_rows_named"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("the key team_a-self-driving of team_a acted in team_b", verdict.details[0])

    def test_a_denied_try_on_another_tenant_is_not_a_fault(self):
        data = healthy()
        audit_rows(data).append(
            {"id": 4161, "created_at": "2026-11-27T12:17:00", "tenant_id": "team_b",
             "actor_key_id": "c0de0000-0000-4000-8000-0000000a0001",
             "actor_key_name": "team_a-self-driving", "action": "workflow.run",
             "target": None, "outcome": "denied", "detail": "{\"reason\": \"tenant_access_denied\"}"})
        self.assertEqual(judge(data)["operations.audit_rows_named"].state, engine.HOLDS)


class GaugeTest(unittest.TestCase):
    def set_metrics(self, data, waiting=None, lag=None):
        lines = []
        for tenant, value in (waiting or {}).items():
            lines.append(f'cloacina_waiting_tasks{{pool="default",priority="5",tenant="{tenant}"}} {value}')
        for tenant, value in (lag or {}).items():
            lines.append(f'cloacina_schedule_lag_seconds{{tenant="{tenant}"}} {value}')
        data["metrics"] = {"text": "\n".join(lines) + "\n"}

    def test_a_gauge_at_0_while_tasks_wait(self):
        data = healthy()
        self.set_metrics(data, waiting={"team_a": 0, "team_b": 0, "team_c": 0},
                         lag={"team_b": 93})
        verdict = judge(data)["operations.backlog_gauge"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("team_a: 4 tasks wait, and the gauge says 0", verdict.evidence)

    def test_a_gauge_that_stays_up_after(self):
        data = healthy()
        tenant_rows(data, "team_a", "operations_waiting")[0]["waiting"] = 0
        verdict = judge(data)["operations.backlog_gauge"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("no task waits, and the gauge says 4", verdict.evidence)

    def test_nothing_waits_is_cannot_judge_and_not_a_pass(self):
        data = healthy()
        tenant_rows(data, "team_a", "operations_waiting")[0]["waiting"] = 0
        self.set_metrics(data, waiting={"team_a": 0, "team_b": 0, "team_c": 0})
        self.assertEqual(judge(data)["operations.backlog_gauge"].state, engine.UNKNOWN)

    def test_a_lag_gauge_at_0_while_a_schedule_is_late(self):
        data = healthy()
        tenant_rows(data, "team_b", "operations_lag")[0]["lag_seconds"] = 600
        self.set_metrics(data, waiting={"team_a": 4}, lag={"team_a": 0, "team_b": 0, "team_c": 0})
        verdict = judge(data)["operations.lag_gauge"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("team_b: a schedule is 600 s late, and the gauge says 0", verdict.evidence)

    def test_a_lag_gauge_with_no_late_schedule(self):
        data = healthy()
        self.set_metrics(data, waiting={"team_a": 4}, lag={"team_a": 400, "team_b": 93})
        verdict = judge(data)["operations.lag_gauge"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("team_a: no schedule is late, and the gauge says 400", verdict.evidence)

    def test_the_metrics_away_is_cannot_judge(self):
        data = healthy()
        data["metrics"] = {"error": "URLError: refused"}
        verdicts = judge(data)
        self.assertEqual(verdicts["operations.backlog_gauge"].state, engine.UNKNOWN)
        self.assertEqual(verdicts["operations.lag_gauge"].state, engine.UNKNOWN)


class RetentionTest(unittest.TestCase):
    def test_a_finished_run_older_than_the_period(self):
        data = healthy()
        tenant_rows(data, "team_c", "operations_retention")[0]["old_finished"] = 12
        verdict = judge(data)["operations.retention"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("team_c: 12 finished runs are older than 2 days", verdict.evidence)

    def test_task_rows_left_with_no_run(self):
        data = healthy()
        tenant_rows(data, "team_a", "operations_retention")[0]["orphan_tasks"] = 3
        verdict = judge(data)["operations.retention"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("team_a: 3 task rows have no run", verdict.evidence)

    def test_a_young_deployment_is_cannot_judge(self):
        data = healthy()
        for tenant in data["tenants"]:
            tenant_rows(data, tenant, "operations_retention")[0]["tenant_since"] = "2026-11-26T09:00:00"
        verdict = judge(data)["operations.retention"]
        self.assertEqual(verdict.state, engine.UNKNOWN)
        self.assertIn("older than the retention period of 2 days", verdict.evidence)


class SqlTest(unittest.TestCase):
    def test_the_queries(self):
        for sql in (operations.FAILED_RUNS_SQL, operations.RETRIED_RUNS_SQL, operations.WAITING_SQL,
                    operations.LAG_SQL, operations.RETENTION_SQL):
            self.assertIn("{schema}.", sql)
            self.assertTrue(sql.startswith("SELECT"))
        self.assertNotIn("{schema}", operations.AUDIT_SQL)
        self.assertIn(f"{operations.RETENTION_SECONDS + operations.RETENTION_SLACK_SECONDS} seconds",
                      operations.RETENTION_SQL)


if __name__ == "__main__":
    unittest.main()
