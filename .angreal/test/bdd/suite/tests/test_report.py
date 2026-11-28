"""The report: the JSON and the page."""

from __future__ import annotations

import json
import os
import tempfile
import unittest
import urllib.request

from suite import engine, promises, report, rules, sources

HERE = os.path.dirname(os.path.abspath(__file__))
HEALTHY = os.path.join(HERE, "recorded", "healthy.json")


def made_from(data, registry=None, ledger=None):
    registry = registry or rules.load()
    ledger = ledger or engine.Ledger()
    snapshot = engine.Snapshot(data)
    results = engine.judge_all(registry, ledger, snapshot)
    return report.build(registry, ledger, results, snapshot, interval_s=60)


def promise(made, promise_id):
    for row in made["rows"]:
        for item in row["promises"]:
            if item["id"] == promise_id:
                return item
    raise KeyError(promise_id)


class BuildTest(unittest.TestCase):
    def test_every_promise_of_the_table_is_in_the_report(self):
        made = made_from(sources.read_snapshot(HEALTHY))
        listed = [item["id"] for row in made["rows"] for item in row["promises"]]
        self.assertEqual(sorted(listed), sorted(promises.PROMISES))
        self.assertEqual([row["row"] for row in made["rows"]], list(promises.ROWS))

    def test_a_promise_with_no_rule_says_so_and_names_its_chapter_task(self):
        made = made_from(sources.read_snapshot(HEALTHY))
        item = promise(made, "run_once.long_task")
        self.assertEqual(item["status"], report.NO_RULE)
        self.assertEqual(item["rules"], [])
        self.assertIsNone(item["held_for"])
        self.assertEqual(item["evidence_from"], ["standing_archive"])
        row = next(row for row in made["rows"] if row["row"] == "Run once")
        self.assertEqual(row["chapter_task"], "CLOACINA-T-1110")
        self.assertEqual(made["summary"][report.NO_RULE], len(promises.PROMISES) - 9)

    def test_a_promise_that_holds_shows_for_how_long(self):
        registry = rules.load()
        ledger = engine.Ledger()
        data = sources.read_snapshot(HEALTHY)
        made_from(data, registry, ledger)
        data["taken_at"] = "2026-11-27T12:21:00Z"
        data["admin"]["db"]["clock"]["rows"][0]["now"] = "2026-11-27T12:21:00+00:00"
        made = made_from(data, registry, ledger)
        item = promise(made, "publish.bad_build")
        self.assertEqual(item["status"], engine.SHOW_HOLDS)
        self.assertEqual(item["held_since"], "2026-11-27T12:20:00Z")
        self.assertEqual(item["held_for_seconds"], 60)
        self.assertEqual(item["held_for"], "1 min")
        self.assertFalse(made["red"])
        self.assertEqual(made["summary"][engine.SHOW_HOLDS], 9)

    def test_a_broken_promise_makes_the_report_red(self):
        data = sources.read_snapshot(HEALTHY)
        data["admin"]["db"]["agents"]["rows"][0]["agents"] = 0
        made = made_from(data)
        item = promise(made, "workload.agents")
        self.assertEqual(item["status"], engine.SHOW_BROKEN)
        self.assertEqual(item["label"], "does not hold")
        self.assertIsNone(item["held_for"])
        self.assertTrue(made["red"])

    def test_a_named_gap_shows_its_task_and_is_not_red(self):
        registry = engine.Registry()
        registry.rule("t.gap", "run_once.long_task", "t", gap="CLOACINA-T-1084",
                      about="a long task ran once")(lambda s: engine.broken("it ran twice"))
        registry.rule("t.closed", "run_once.no_free_agent", "t", gap="unfiled",
                      about="a waiting task ran once")(lambda s: engine.holds("it ran once"))
        made = made_from({"taken_at": 1_800_000_000, "tenants": {}}, registry)
        gap = promise(made, "run_once.long_task")
        self.assertEqual(gap["status"], engine.SHOW_GAP)
        self.assertEqual(gap["gap_tasks"], ["CLOACINA-T-1084"])
        self.assertEqual(promise(made, "run_once.no_free_agent")["status"], engine.SHOW_GAP_HOLDS)
        self.assertFalse(made["red"])
        page = report.render(made)
        self.assertIn("owner task: CLOACINA-T-1084", page)
        self.assertIn("The gap may be closed", page)

    def test_the_worst_rule_of_a_promise_is_its_state(self):
        registry = engine.Registry()
        registry.rule("t.a", "tenants.key", "t", about="a")(lambda s: engine.holds("fine"))
        registry.rule("t.b", "tenants.key", "t", about="b")(lambda s: engine.unknown("no data"))
        made = made_from({"taken_at": 1_800_000_000, "tenants": {}}, registry)
        item = promise(made, "tenants.key")
        self.assertEqual(item["status"], engine.SHOW_UNKNOWN)
        self.assertIsNone(item["held_for"])
        self.assertEqual(len(item["rules"]), 2)

    def test_disturbances_of_the_last_day_are_listed(self):
        data = sources.read_snapshot(HEALTHY)
        data["disturbances"] = [
            {"id": "d1", "kind": "kill_agent", "target": "team_a",
             "start": "2026-11-27T12:19:00Z", "end": "2026-11-27T12:19:05Z",
             "settle_until": "2026-11-27T12:24:00Z"},
            {"id": "d0", "kind": "postgres_away", "target": "",
             "start": "2026-11-25T08:00:00Z", "end": "2026-11-25T08:05:00Z"},
        ]
        made = made_from(data)
        self.assertEqual([w["id"] for w in made["disturbances"]], ["d1"])
        self.assertTrue(made["disturbances"][0]["active"])
        self.assertIn("kill_agent", report.render(made))


class PageTest(unittest.TestCase):
    def test_the_page_holds_every_sentence_and_the_time_of_the_pass(self):
        made = made_from(sources.read_snapshot(HEALTHY))
        page = report.render(made)
        for _row, sentence in promises.PROMISES.values():
            self.assertIn(sentence.replace("'", "&#x27;"), page)
        self.assertIn("2026-11-27T12:20:00Z", page)
        self.assertIn("no rule yet", page)
        self.assertIn('id="stale"', page)
        self.assertTrue(page.startswith("<!doctype html>"))

    def test_text_from_the_system_under_test_is_escaped(self):
        registry = engine.Registry()
        registry.rule("t.a", "tenants.key", "t", about="a")(
            lambda s: engine.broken('<script>alert("x")</script>', ["<img src=x>"]))
        made = made_from({"taken_at": 1_800_000_000, "tenants": {}}, registry)
        page = report.render(made)
        self.assertNotIn("<script>alert", page)
        self.assertNotIn("<img src=x>", page)
        self.assertIn("&lt;script&gt;alert", page)

    def test_durations_in_words(self):
        self.assertEqual(report.duration(0), "0 s")
        self.assertEqual(report.duration(59), "59 s")
        self.assertEqual(report.duration(60), "1 min")
        self.assertEqual(report.duration(3 * 3600 + 120), "3 h 2 min")
        self.assertEqual(report.duration(2 * 86400 + 5 * 3600 + 59), "2 d 5 h")


class WriteAndServeTest(unittest.TestCase):
    def test_the_report_is_written_and_served_on_localhost(self):
        made = made_from(sources.read_snapshot(HEALTHY))
        with tempfile.TemporaryDirectory() as folder:
            report.write(folder, made)
            self.assertEqual(sorted(os.listdir(folder)), ["index.html", "report.json"])
            server = report.serve(folder, 0)
            try:
                host, port = server.server_address[:2]
                self.assertEqual(host, "127.0.0.1")
                with urllib.request.urlopen(f"http://127.0.0.1:{port}/report.json", timeout=5) as answer:
                    served = json.loads(answer.read().decode("utf-8"))
                self.assertEqual(served["generated_at"], "2026-11-27T12:20:00Z")
                with urllib.request.urlopen(f"http://127.0.0.1:{port}/", timeout=5) as answer:
                    self.assertIn("Cloacina behaviour suite", answer.read().decode("utf-8"))
            finally:
                server.shutdown()
                server.server_close()


if __name__ == "__main__":
    unittest.main()
