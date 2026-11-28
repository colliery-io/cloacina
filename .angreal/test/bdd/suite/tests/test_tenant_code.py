"""The chapter tenant_code (CLOACINA-T-1079): its feature, its rules, and
the reading of a server's mapped libraries. No server, no cluster."""

from __future__ import annotations

import copy
import os
import sys
import unittest

from suite import engine, promises, rules, sources

HERE = os.path.dirname(os.path.abspath(__file__))
BDD = os.path.dirname(os.path.dirname(HERE))
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(BDD)))
HEALTHY = os.path.join(HERE, "recorded", "healthy.json")

# The folder that holds the `bdd` package.
sys.path.insert(0, os.path.dirname(BDD))

from bdd import chapters  # noqa: E402
from bdd import tenant_code_steps as chapter  # noqa: E402

# The claims of CLOACINA-A-0019, "Claims a test must show", in its order:
# the scenario of each, and its promise in the report.
CLAIMS = (
    ("A Rust package's load-time code never runs in the server process", "tenants.code_rust_load"),
    ("A Python package's import never runs in the server process", "tenants.code_python_import"),
    ("A default chart install runs no packaged work in the server process", "tenants.code_default_chart"),
    ("A graph firing with no agent fails with a reason and does not run in the server",
     "tenants.code_graph_no_agent"),
    ("A poll trigger's code runs on the tenant's agent, not in the server", "tenants.code_poll_trigger"),
    ("Native provider and Python accumulator code do not run in the server, and a WASM provider "
     "runs under its grants", "tenants.code_reactive_layer"),
    ("A package of public is treated as a package of a named tenant", "tenants.code_public"),
    ("A tenant's compiler holds a credential that opens only that tenant's build tables",
     "tenants.code_build_role"),
    ("A package built before the change runs after an upgrade with no publish", "tenants.code_older_package"),
    ("A query from a tenant's pool on another tenant's schema is refused by Postgres", "tenants.code_pool_role"),
)


def judge(data):
    registry = rules.load()
    snapshot = engine.Snapshot(data)
    return {rule.id: rule.judge(snapshot) for rule in registry.rules}


def healthy():
    return sources.read_snapshot(HEALTHY)


class FeatureTest(unittest.TestCase):
    def setUp(self):
        self.feature = chapters.load_features(chapters.find("tenant_code"), ROOT)[0]

    def test_one_scenario_for_each_claim_in_the_order_of_the_adr(self):
        self.assertEqual([s.name for s in self.feature.scenarios], [name for name, _ in CLAIMS])

    def test_each_claim_has_its_promise_in_the_tenants_row(self):
        for _name, promise_id in CLAIMS:
            self.assertEqual(promises.row_of(promise_id), "Tenants", promise_id)

    def test_no_scenario_is_a_gap(self):
        # The pool role was the gap of CLOACINA-T-1080, which is in.
        gaps = {s.name: s.gap_tasks for s in self.feature.scenarios if s.is_gap}
        self.assertEqual(gaps, {})

    def test_every_step_has_exactly_one_definition(self):
        from bdd import gherkin

        problems, _notes = gherkin.check(self.feature, chapter.steps)
        self.assertEqual(problems, [])

    def test_the_fixtures_the_chapter_publishes_are_there(self):
        for fixture in (chapter.RUST_FIXTURE, chapter.PY_FIXTURE, chapter.GRAPH_FIXTURE,
                        chapter.POLLING_FIXTURE, chapter.NATIVE_FIXTURE, chapter.WASM_FIXTURE):
            self.assertTrue(os.path.exists(os.path.join(ROOT, "examples", "fixtures", fixture,
                                                        "package.toml")), fixture)


class MarkTest(unittest.TestCase):
    MAPS = (
        "55d0c0a00000-55d0c0c00000 r-xp 00000000 08:01 1 /usr/local/bin/cloacina-server\n"
        "7f0000000000-7f0000100000 r-xp 00000000 08:01 2 /usr/lib/x86_64-linux-gnu/libc.so.6\n"
        "7f0000200000-7f0000300000 r-xp 00000000 08:01 3 /lib/x86_64-linux-gnu/libssl.so.3\n"
        "7f0000400000-7f0000500000 rw-p 00000000 00:00 0 [heap]\n"
        "7f0000600000-7f0000700000 r-xp 00000000 00:01 4 /memfd:wasm-memory-image (deleted)\n"
    )

    def test_the_libraries_of_the_image_are_not_reported(self):
        self.assertEqual(chapter._foreign_libraries(self.MAPS), [])

    def test_a_tenant_library_is_reported_by_its_path(self):
        maps = self.MAPS + ("7f0000800000-7f0000900000 r-xp 00000000 08:01 5 "
                            "/tmp/cloacina-packages/abc/libtenant_code_mark_rust.so (deleted)\n")
        self.assertEqual(chapter._foreign_libraries(maps),
                         ["/tmp/cloacina-packages/abc/libtenant_code_mark_rust.so (deleted)"])

    def test_executable_memory_from_a_memfd_is_reported(self):
        maps = self.MAPS + "7f0000a00000-7f0000b00000 r-xp 00000000 00:01 6 /memfd:package (deleted)\n"
        self.assertEqual(chapter._foreign_libraries(maps), ["/memfd:package (deleted)"])


class BuildRoleRuleTest(unittest.TestCase):
    def test_it_holds_on_the_healthy_snapshot(self):
        verdict = judge(healthy())["tenant_code.build_role_reach"]
        self.assertEqual(verdict.state, engine.HOLDS, verdict.evidence)
        self.assertIn("4 build roles", verdict.evidence)

    def test_a_right_on_another_tenants_table(self):
        data = healthy()
        rows = data["admin"]["db"]["build_role_reach"]["rows"]
        rows.append({"role": "team_a$build", "tenant": "team_a", "schema": "team_b",
                     "name": "workflow_packages"})
        verdict = judge(data)["tenant_code.build_role_reach"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("team_a$build has a right on team_b.workflow_packages, outside its tenant",
                      verdict.details)

    def test_a_right_on_the_servers_own_table(self):
        data = healthy()
        data["admin"]["db"]["build_role_reach"]["rows"].append(
            {"role": "public$build", "tenant": "public", "schema": "public", "name": "api_keys"})
        verdict = judge(data)["tenant_code.build_role_reach"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("which a build does not need", verdict.details[0])

    def test_a_tenant_with_no_build_role_cannot_be_judged(self):
        data = healthy()
        rows = data["admin"]["db"]["build_role_reach"]["rows"]
        rows[:] = [row for row in rows if row["tenant"] != "team_c"]
        verdict = judge(data)["tenant_code.build_role_reach"]
        self.assertEqual(verdict.state, engine.UNKNOWN)
        self.assertIn("team_c", verdict.evidence)


class PoolRoleRuleTest(unittest.TestCase):
    def test_the_rule_is_no_gap(self):
        rule = next(r for r in rules.load().rules if r.id == "tenant_code.tenant_pool_role")
        self.assertIsNone(rule.gap)

    def test_it_holds_when_each_tenant_has_connections_of_its_role(self):
        verdict = judge(healthy())["tenant_code.tenant_pool_role"]
        self.assertEqual(verdict.state, engine.HOLDS, verdict.evidence)
        self.assertIn("team_a$server 3", verdict.evidence)

    def test_the_admin_role_only_is_broken(self):
        data = healthy()
        data["admin"]["db"]["connections"]["rows"] = [{"role": "cloacina", "connections": 20}]
        verdict = judge(data)["tenant_code.tenant_pool_role"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertEqual(len(verdict.details), 3)
        self.assertIn("no connection uses the role team_a$server", verdict.details[0])

    def test_the_tenant_name_role_is_not_the_server_role(self):
        # Before CLOACINA-T-1080 the role had the tenant's name; that is not enough now.
        data = healthy()
        data["admin"]["db"]["connections"]["rows"] = [{"role": "team_a", "connections": 3}]
        verdict = judge(data)["tenant_code.tenant_pool_role"]
        self.assertEqual(verdict.state, engine.BROKEN)

    def test_a_tenant_role_with_rights_on_another_tenant(self):
        data = healthy()
        data["admin"]["db"]["tenant_role_reach"]["rows"].append(
            {"role": "team_a$server", "tenant": "team_a", "schema": "team_b", "tables": 2})
        verdict = judge(data)["tenant_code.tenant_pool_role"]
        self.assertEqual(verdict.state, engine.BROKEN)
        self.assertIn("the role team_a$server has rights on 2 table(s) of the tenant team_b",
                      verdict.details)

    def test_rights_in_public_are_not_another_tenant(self):
        data = copy.deepcopy(healthy())
        data["admin"]["db"]["tenant_role_reach"]["rows"].append(
            {"role": "team_a$server", "tenant": "team_a", "schema": "public", "tables": 3})
        self.assertEqual(judge(data)["tenant_code.tenant_pool_role"].state, engine.HOLDS)


class SqlTest(unittest.TestCase):
    def test_the_queries_read_the_catalogue_only(self):
        from suite.rules import tenant_code

        for sql in (tenant_code.BUILD_ROLE_REACH_SQL, tenant_code.TENANT_ROLE_REACH_SQL,
                    tenant_code.CONNECTIONS_SQL):
            self.assertNotIn("{schema}", sql)
            self.assertTrue(sql.startswith("SELECT"))
        self.assertIn("has_table_privilege", tenant_code.BUILD_ROLE_REACH_SQL)
        self.assertIn("'%$build'", tenant_code.BUILD_ROLE_REACH_SQL)


if __name__ == "__main__":
    unittest.main()
