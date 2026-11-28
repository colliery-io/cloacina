"""The rules of the workload and of the Publish row, on recorded snapshots.

`recorded/healthy.json` is one snapshot in the form the checker collects. The
other cases are that snapshot with one fact changed.
"""

from __future__ import annotations

import io
import json
import os
import tempfile
import unittest
from contextlib import redirect_stdout

from suite import check, engine, rules, sources

HERE = os.path.dirname(os.path.abspath(__file__))
HEALTHY = os.path.join(HERE, "recorded", "healthy.json")


def healthy():
    return sources.read_snapshot(HEALTHY)


def judge(data, registry=None):
    """rule id -> verdict, for one snapshot."""
    registry = registry or rules.load()
    snapshot = engine.Snapshot(data)
    return {rule.id: rule.judge(snapshot) for rule in registry.rules}


def driver_context(data, tenant, run):
    """The context of one run of the self-driving workflow, as a dictionary
    that the test can change in place."""
    row = data["tenants"][tenant]["db"]["driver_runs"]["rows"][run]
    if isinstance(row["context"], str):
        row["context"] = json.loads(row["context"])
    return row["context"]


def version_row(data, tenant, kind, version):
    items = data["tenants"][tenant]["api"][f"versions_target_{kind}"]["body"]["items"]
    return next(item for item in items if item["version"] == version)


class RegistryTest(unittest.TestCase):
    def test_the_rules_load_and_are_sound(self):
        registry = rules.load()
        self.assertEqual(registry.problems(), [])
        self.assertEqual(rules.chapter_modules(), ["capacity", "publishing", "standing"])
        ids = sorted(rule.id for rule in registry.rules)
        self.assertEqual(ids, ["capacity.fits_in_flight", "capacity.held_work_runs",
                               "capacity.no_failure_while_full", "capacity.others_on_time",
                               "capacity.priority_order",
                               "publishing.bad_build_keeps_running", "publishing.new_version_built",
                               "publishing.step_back", "standing.agents", "standing.schedules",
                               "standing.self_driving"])

    def test_each_rule_names_its_sources(self):
        registry = rules.load()
        self.assertIn("driver_runs", registry.db_needs)
        self.assertIn("{schema}", registry.db_needs["driver_runs"])
        self.assertIn("versions_target_py", registry.api_needs)
        self.assertIn("{tenant}", registry.api_needs["versions_target_rust"])
        self.assertIn("agents", registry.admin_db_needs)


class HealthyTest(unittest.TestCase):
    def test_every_rule_holds_on_the_healthy_snapshot(self):
        verdicts = judge(healthy())
        for rule_id, verdict in verdicts.items():
            self.assertEqual(verdict.state, engine.HOLDS, f"{rule_id}: {verdict.evidence}")

    def test_the_evidence_says_what_was_read(self):
        verdicts = judge(healthy())
        self.assertIn("team_a 2", verdicts["standing.agents"].evidence)
        self.assertIn("15 schedules", verdicts["standing.schedules"].evidence)
        self.assertIn("3 published versions", verdicts["publishing.new_version_built"].evidence)
        self.assertIn("team_a standing-target-py 0.1.2", verdicts["publishing.step_back"].evidence)


class WorkloadRulesTest(unittest.TestCase):
    def test_a_tenant_with_one_agent(self):
        data = healthy()
        data["admin"]["db"]["agents"]["rows"][1]["agents"] = 1
        verdict = judge(data)["standing.agents"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("team_b has 1 of 2", verdict.evidence)

    def test_a_tenant_with_no_agent_row(self):
        data = healthy()
        del data["admin"]["db"]["agents"]["rows"][2]
        verdict = judge(data)["standing.agents"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("team_c has 0 of 2", verdict.evidence)

    def test_a_schedule_that_stopped(self):
        data = healthy()
        rows = data["tenants"]["team_c"]["db"]["last_runs"]["rows"]
        next(r for r in rows if r["workflow_name"] == "standing_rollup")["last_started"] = \
            "2026-11-27T11:55:01.150"
        verdict = judge(data)["standing.schedules"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertEqual(len(verdict.details), 1)
        self.assertIn("team_c/standing_rollup", verdict.details[0])

    def test_a_schedule_with_no_run_in_the_window(self):
        data = healthy()
        rows = data["tenants"]["team_a"]["db"]["last_runs"]["rows"]
        rows[:] = [r for r in rows if r["workflow_name"] != "standing_archive"]
        verdict = judge(data)["standing.schedules"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("team_a/standing_archive: no run", verdict.details[0])

    def test_a_young_tenant_is_not_late_yet(self):
        data = healthy()
        for tenant in data["tenants"].values():
            tenant["db"]["first_run"]["rows"][0]["first_run"] = "2026-11-27T12:10:01"
            rows = tenant["db"]["last_runs"]["rows"]
            rows[:] = [r for r in rows if r["workflow_name"] != "standing_archive"]
        verdict = judge(data)["standing.schedules"]
        self.assertEqual(verdict.state, engine.HOLDS, verdict.evidence)
        self.assertIn("12 schedules", verdict.evidence)

    def test_no_run_anywhere_is_cannot_judge(self):
        data = healthy()
        for tenant in data["tenants"].values():
            tenant["db"]["first_run"]["rows"] = [{"first_run": None, "runs": 0}]
            tenant["db"]["last_runs"]["rows"] = []
            tenant["db"]["driver_runs"]["rows"] = []
        verdicts = judge(data)
        self.assertEqual(verdicts["standing.schedules"].state, engine.UNKNOWN)
        self.assertEqual(verdicts["standing.self_driving"].state, engine.UNKNOWN)

    def test_one_tenant_with_no_run_while_the_others_run(self):
        data = healthy()
        data["tenants"]["team_b"]["db"]["first_run"]["rows"] = [{"first_run": None, "runs": 0}]
        data["tenants"]["team_b"]["db"]["last_runs"]["rows"] = []
        verdict = judge(data)["standing.schedules"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("team_b: no run at all", verdict.details[0])

    def test_a_self_driving_workflow_that_went_quiet(self):
        data = healthy()
        del data["tenants"]["team_b"]["db"]["driver_runs"]["rows"][1]
        verdict = judge(data)["standing.self_driving"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("team_b: no action since 2026-11-27T11:35:04Z", verdict.evidence)

    def test_a_self_driving_workflow_that_cannot_read_its_key(self):
        data = healthy()
        context = driver_context(data, "team_c", 1)
        context["standing_driver_actions"] = [
            {"action": "read_key", "error": "KeyError", "at": "2026-11-27T12:15:03Z"}]
        verdict = judge(data)["standing.self_driving"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("team_c: the self-driving workflow cannot act: read_key KeyError", verdict.evidence)


class PublishRulesTest(unittest.TestCase):
    def test_a_published_version_that_never_built(self):
        data = healthy()
        version_row(data, "team_a", "py", "0.1.3")["build_status"] = "pending"
        verdict = judge(data)["publishing.new_version_built"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("standing-target-py 0.1.3", verdict.details[0])
        self.assertIn("is pending", verdict.details[0])

    def test_a_published_version_that_is_not_listed(self):
        data = healthy()
        items = data["tenants"]["team_b"]["api"]["versions_target_py"]["body"]["items"]
        items[:] = [item for item in items if item["version"] != "0.1.1"]
        verdict = judge(data)["publishing.new_version_built"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("not in the versions list", verdict.details[0])

    def test_a_fresh_publish_is_not_judged_before_its_build_can_be_over(self):
        data = healthy()
        context = driver_context(data, "team_a", 2)
        context["standing_driver_actions"].append(
            {"action": "publish", "package": "standing-target-rust", "version": "0.1.3",
             "status": 201, "at": "2026-11-27T12:15:06Z"})
        verdict = judge(data)["publishing.new_version_built"]
        self.assertEqual(verdict.state, engine.HOLDS, verdict.evidence)

    def test_a_refused_publish(self):
        data = healthy()
        context = driver_context(data, "team_a", 2)
        context["standing_driver_actions"].append(
            {"action": "publish", "package": "standing-target-py", "version": "0.1.4",
             "status": 500, "at": "2026-11-27T12:15:06Z"})
        verdict = judge(data)["publishing.new_version_built"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("answered 500", verdict.details[0])

    def test_a_publish_that_did_not_reach_the_server_is_no_evidence(self):
        data = healthy()
        context = driver_context(data, "team_a", 2)
        context["standing_driver_actions"].append(
            {"action": "publish", "package": "standing-target-py", "version": "0.1.4",
             "status": None, "at": "2026-11-27T12:15:06Z"})
        self.assertEqual(judge(data)["publishing.new_version_built"].state, engine.HOLDS)

    def test_a_build_error_that_took_the_running_version_out_of_service(self):
        data = healthy()
        version_row(data, "team_a", "rust", "0.1.1")["active"] = False
        verdict = judge(data)["publishing.bad_build_keeps_running"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("no version in service", verdict.details[0])

    def test_a_build_error_that_went_into_service(self):
        data = healthy()
        version_row(data, "team_a", "rust", "0.1.2")["active"] = True
        version_row(data, "team_a", "rust", "0.1.1")["active"] = False
        verdict = judge(data)["publishing.bad_build_keeps_running"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertTrue(any("is in service" in detail for detail in verdict.details))

    def test_a_build_error_that_built(self):
        data = healthy()
        version_row(data, "team_a", "rust", "0.1.2")["build_status"] = "success"
        verdict = judge(data)["publishing.bad_build_keeps_running"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("is success, not failed", verdict.details[0])

    def test_no_build_error_yet_is_cannot_judge(self):
        data = healthy()
        del data["tenants"]["team_a"]["db"]["driver_runs"]["rows"][1]
        self.assertEqual(judge(data)["publishing.bad_build_keeps_running"].state, engine.UNKNOWN)

    def test_a_step_back_that_did_not_take(self):
        data = healthy()
        version_row(data, "team_a", "py", "0.1.2")["active"] = False
        version_row(data, "team_a", "py", "0.1.3")["active"] = True
        verdict = judge(data)["publishing.step_back"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("went back to standing-target-py 0.1.2", verdict.evidence)
        self.assertIn("in service now: 0.1.3", verdict.evidence)

    def test_a_publish_after_the_step_back_ends_its_evidence(self):
        data = healthy()
        context = driver_context(data, "team_a", 2)
        context["standing_driver_actions"].append(
            {"action": "publish", "package": "standing-target-py", "version": "0.1.4",
             "status": 201, "at": "2026-11-27T12:19:00Z"})
        self.assertEqual(judge(data)["publishing.step_back"].state, engine.UNKNOWN)

    def test_a_step_back_with_nothing_to_go_back_to_is_no_evidence(self):
        data = healthy()
        context = driver_context(data, "team_a", 2)
        context["standing_driver_actions"][0] = {
            "action": "step_back", "package": "standing-target-py", "version": None,
            "status": None, "skipped": "no older good build", "at": "2026-11-27T12:15:03Z"}
        # The last accepted change of team_a is now its publish of 0.1.3.
        self.assertEqual(judge(data)["publishing.step_back"].state, engine.UNKNOWN)


class SourcesAwayTest(unittest.TestCase):
    def test_the_server_away_is_cannot_judge_for_the_rules_that_read_it(self):
        data = healthy()
        for tenant in data["tenants"].values():
            for name in tenant["api"]:
                tenant["api"][name] = {"error": "URLError: connection refused"}
        verdicts = judge(data)
        self.assertEqual(verdicts["publishing.new_version_built"].state, engine.UNKNOWN)
        self.assertIn("connection refused", verdicts["publishing.new_version_built"].evidence)
        # The rules that read the database only still judge.
        self.assertEqual(verdicts["standing.schedules"].state, engine.HOLDS)

    def test_the_database_away_is_cannot_judge_for_every_rule(self):
        data = healthy()
        data["admin"]["db"] = {name: {"error": "psql: connection refused"}
                               for name in data["admin"]["db"]}
        for tenant in data["tenants"].values():
            tenant["db"] = {name: {"error": "psql: connection refused"} for name in tenant["db"]}
        for rule_id, verdict in judge(data).items():
            self.assertEqual(verdict.state, engine.UNKNOWN, rule_id)

    def test_a_snapshot_with_no_tenant_of_the_workload(self):
        data = healthy()
        data["tenants"] = {}
        for rule_id, verdict in judge(data).items():
            self.assertEqual(verdict.state, engine.UNKNOWN, rule_id)


class FakeApi:
    def __init__(self):
        self.calls = []

    def get(self, path, key):
        self.calls.append((path, key))
        return {"status": 200, "body": {"items": []}}


class FakeDatabase:
    def __init__(self, fail=False):
        self.queries = []
        self.fail = fail

    def query(self, sql):
        self.queries.append(sql)
        return {"error": "away"} if self.fail else {"rows": []}


class CollectTest(unittest.TestCase):
    def test_a_snapshot_holds_every_source_for_every_tenant(self):
        registry = rules.load()
        api, database = FakeApi(), FakeDatabase()
        data = sources.collect(registry, api, database, {"team_a": "key-a", "team_b": "key-b"}, now=5.0)
        self.assertEqual(sorted(data["tenants"]), ["team_a", "team_b"])
        self.assertEqual(sorted(data["tenants"]["team_a"]["api"]), sorted(registry.api_needs))
        self.assertEqual(sorted(data["tenants"]["team_a"]["db"]), sorted(registry.db_needs))
        self.assertEqual(sorted(data["admin"]["db"]), sorted(registry.admin_db_needs))
        # Each tenant is read with its own key and its own schema.
        self.assertIn(("/v1/tenants/team_b/workflows/standing-target-py/versions", "key-b"), api.calls)
        self.assertTrue(any("team_a.workflow_executions" in sql for sql in database.queries))
        self.assertFalse(any("{schema}" in sql or "{tenant}" in sql for sql in database.queries))
        # No key is in the snapshot.
        self.assertNotIn("key-a", json.dumps(data))

    def test_no_database_is_an_error_entry_and_not_a_crash(self):
        registry = rules.load()
        data = sources.collect(registry, FakeApi(), None, {"team_a": "key-a"}, now=5.0)
        self.assertIn("error", data["tenants"]["team_a"]["db"]["driver_runs"])
        self.assertIn("error", data["admin"]["db"]["agents"])

    def test_a_tenant_name_that_is_not_a_schema_name_is_never_put_in_sql(self):
        registry = rules.load()
        database = FakeDatabase()
        data = sources.collect(registry, FakeApi(), database, {"team_a; DROP": "k"}, now=5.0)
        self.assertIn("error", data["tenants"]["team_a; DROP"]["db"]["driver_runs"])
        self.assertFalse(any("DROP" in sql for sql in database.queries))

    def test_psql_gets_its_connection_from_the_environment(self):
        database = sources.Psql("postgres://robin:s3cret%21@127.0.0.1:15432/cloacina",
                                binary="/nonexistent/psql")
        self.assertEqual(database.env["PGHOST"], "127.0.0.1")
        self.assertEqual(database.env["PGPORT"], "15432")
        self.assertEqual(database.env["PGUSER"], "robin")
        self.assertEqual(database.env["PGPASSWORD"], "s3cret!")
        self.assertEqual(database.env["PGDATABASE"], "cloacina")
        answer = database.query("SELECT 1")
        self.assertIn("error", answer)
        self.assertNotIn("s3cret", json.dumps(answer))
        with self.assertRaises(ValueError):
            sources.Psql("mysql://x/y")

    def test_a_missing_disturbance_record_is_no_disturbance(self):
        self.assertEqual(sources.read_disturbances("/nonexistent/disturbances.jsonl"), [])


class ReplayTest(unittest.TestCase):
    def run_check(self, argv):
        out = io.StringIO()
        with redirect_stdout(out):
            code = check.main(argv)
        return code, out.getvalue()

    def test_replay_judges_recorded_snapshots_and_writes_the_report(self):
        with tempfile.TemporaryDirectory() as folder:
            code, text = self.run_check(["--replay", HEALTHY, "--out", folder])
            self.assertEqual(code, 0, text)
            self.assertIn("holds 8", text)
            with open(os.path.join(folder, "report", "report.json"), encoding="utf-8") as handle:
                made = json.load(handle)
            self.assertFalse(made["red"])
            self.assertTrue(os.path.exists(os.path.join(folder, "report", "index.html")))
            self.assertTrue(os.path.exists(os.path.join(folder, "state.json")))

    def test_replay_of_a_break_and_a_recovery(self):
        with tempfile.TemporaryDirectory() as folder:
            paths = []
            for minute, in_service in ((20, "0.1.2"), (21, "0.1.3"), (22, "0.1.2")):
                data = healthy()
                data["taken_at"] = f"2026-11-27T12:{minute}:00Z"
                data["admin"]["db"]["clock"]["rows"][0]["now"] = f"2026-11-27T12:{minute}:00+00:00"
                for version in ("0.1.2", "0.1.3"):
                    version_row(data, "team_a", "py", version)["active"] = version == in_service
                path = os.path.join(folder, f"snapshot-{minute}.json")
                with open(path, "w", encoding="utf-8") as handle:
                    json.dump(data, handle)
                paths.append(path)

            code, _ = self.run_check(["--replay", paths[0], paths[1], "--out", folder])
            self.assertEqual(code, 1)
            code, _ = self.run_check(["--replay", paths[2], "--out", folder])
            self.assertEqual(code, 0)
            with open(os.path.join(folder, "state.json"), encoding="utf-8") as handle:
                entry = json.load(handle)["rules"]["publishing.step_back"]
            self.assertEqual(entry["breaks"], 1)
            self.assertEqual(entry["state"], engine.SHOW_HOLDS)
            self.assertEqual(entry["held_since"], engine.parse_time("2026-11-27T12:22:00Z"))
            self.assertEqual(entry["last_break_at"], engine.parse_time("2026-11-27T12:21:00Z"))

    def test_list_names_the_rules_and_the_promises_with_no_rule(self):
        code, text = self.run_check(["--list"])
        self.assertEqual(code, 0)
        self.assertIn("publishing.step_back", text)
        self.assertIn("Promises with no rule yet:", text)
        self.assertIn("A long task runs one time.", text)
        self.assertIn("CLOACINA-T-1110", text)


if __name__ == "__main__":
    unittest.main()
