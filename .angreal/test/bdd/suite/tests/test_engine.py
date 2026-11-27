"""The checker's bookkeeping: streaks, breaks, gaps, disturbance windows."""

from __future__ import annotations

import os
import tempfile
import unittest

from suite import engine

T0 = 1_800_000_000.0  # an arbitrary start; passes are a minute apart


def snapshot(at, disturbances=None):
    return engine.Snapshot({"taken_at": at, "tenants": {}, "disturbances": disturbances or []})


class Scripted:
    """A rule function that answers from a list, one verdict for each pass."""

    def __init__(self, *verdicts):
        self.verdicts = list(verdicts)

    def __call__(self, _snapshot):
        return self.verdicts.pop(0)


def rule(fn, gap=None, tolerates=()):
    return engine.Rule("test.rule", "publish.own_package", "test", fn, gap=gap,
                       tolerates=tolerates, about="a rule of the tests")


def run(rule_, ledger, times, disturbances=None):
    entry = None
    for at in times:
        snap = snapshot(at, disturbances)
        entry = ledger.record(rule_, rule_.judge(snap), snap.now, snap.windows())
    return entry


OK = engine.holds("fine")
BAD = engine.broken("wrong")
NO = engine.unknown("nothing to read")


class HoldsTest(unittest.TestCase):
    def test_a_promise_that_holds_keeps_one_streak(self):
        ledger = engine.Ledger()
        entry = run(rule(Scripted(OK, OK, OK, OK)), ledger, [T0, T0 + 60, T0 + 120, T0 + 180])
        self.assertEqual(entry["state"], engine.SHOW_HOLDS)
        self.assertEqual(entry["held_since"], T0)
        self.assertEqual(entry["since"], T0)
        self.assertEqual(entry["breaks"], 0)
        self.assertEqual(entry["passes"], 4)
        self.assertEqual(ledger.held_seconds("test.rule", T0 + 180), 180)
        self.assertEqual(entry["longest_held_seconds"], 180)
        self.assertEqual(len(entry["transitions"]), 1)

    def test_a_promise_that_breaks_and_recovers(self):
        ledger = engine.Ledger()
        times = [T0 + 60 * n for n in range(7)]
        entry = run(rule(Scripted(OK, OK, OK, BAD, BAD, OK, OK)), ledger, times)
        self.assertEqual(entry["state"], engine.SHOW_HOLDS)
        self.assertEqual(entry["breaks"], 1)
        self.assertEqual(entry["last_break_at"], T0 + 180)
        self.assertEqual(entry["last_recovered_at"], T0 + 300)
        # The streak starts again at the recovery, not at the first pass.
        self.assertEqual(entry["held_since"], T0 + 300)
        self.assertEqual(ledger.held_seconds("test.rule", T0 + 360), 60)
        # The first streak ran from the first pass to the break.
        self.assertEqual(entry["longest_held_seconds"], 180)
        self.assertEqual([t["to"] for t in entry["transitions"]],
                         [engine.SHOW_HOLDS, engine.SHOW_BROKEN, engine.SHOW_HOLDS])

    def test_two_broken_passes_are_one_break(self):
        ledger = engine.Ledger()
        entry = run(rule(Scripted(OK, BAD, BAD, BAD)), ledger, [T0, T0 + 60, T0 + 120, T0 + 180])
        self.assertEqual(entry["breaks"], 1)
        self.assertEqual(entry["state"], engine.SHOW_BROKEN)
        self.assertIsNone(entry["held_since"])
        self.assertEqual(ledger.held_seconds("test.rule", T0 + 180), 0)

    def test_a_promise_broken_from_the_first_pass(self):
        ledger = engine.Ledger()
        entry = run(rule(Scripted(BAD, OK)), ledger, [T0, T0 + 60])
        self.assertEqual(entry["breaks"], 1)
        self.assertEqual(entry["held_since"], T0 + 60)


class CannotJudgeTest(unittest.TestCase):
    def test_cannot_judge_does_not_break_a_streak_and_is_not_time_held(self):
        ledger = engine.Ledger()
        times = [T0, T0 + 60, T0 + 120, T0 + 180, T0 + 240]
        entry = run(rule(Scripted(OK, OK, NO, NO, OK)), ledger, times)
        self.assertEqual(entry["state"], engine.SHOW_HOLDS)
        self.assertEqual(entry["breaks"], 0)
        self.assertEqual(entry["held_since"], T0)
        # Two minutes of the four followed a pass that could not judge.
        self.assertEqual(ledger.held_seconds("test.rule", T0 + 240), 120)

    def test_cannot_judge_from_the_start_is_not_holding(self):
        ledger = engine.Ledger()
        entry = run(rule(Scripted(NO, NO)), ledger, [T0, T0 + 60])
        self.assertEqual(entry["state"], engine.SHOW_UNKNOWN)
        self.assertIsNone(entry["held_since"])
        self.assertEqual(ledger.held_seconds("test.rule", T0 + 60), 0)

    def test_a_missing_source_is_cannot_judge(self):
        def needs_a_source(snap):
            snap.db("team_a", "runs")
            return OK

        verdict = rule(needs_a_source).judge(snapshot(T0))
        self.assertEqual(verdict.state, engine.UNKNOWN)
        self.assertIn("team_a", verdict.evidence)

    def test_a_source_that_failed_is_cannot_judge(self):
        snap = engine.Snapshot({"taken_at": T0, "tenants": {
            "team_a": {"db": {"runs": {"error": "connection refused"}}, "api": {}}}})
        verdict = rule(lambda s: engine.holds(str(s.db("team_a", "runs")))).judge(snap)
        self.assertEqual(verdict.state, engine.UNKNOWN)
        self.assertIn("connection refused", verdict.evidence)

    def test_a_rule_that_fails_is_cannot_judge_and_is_flagged(self):
        def faulty(_snap):
            raise KeyError("oops")

        ledger = engine.Ledger()
        rule_ = rule(faulty)
        entry = ledger.record(rule_, rule_.judge(snapshot(T0)), T0)
        self.assertEqual(entry["state"], engine.SHOW_UNKNOWN)
        self.assertTrue(entry["rule_error"])

    def test_a_rule_that_returns_no_verdict_is_flagged(self):
        verdict = rule(lambda _snap: True).judge(snapshot(T0))
        self.assertEqual(verdict.state, engine.UNKNOWN)
        self.assertTrue(verdict.rule_error)


class NamedGapTest(unittest.TestCase):
    def test_a_named_gap_that_fails_is_a_gap_and_not_red(self):
        ledger = engine.Ledger()
        rule_ = rule(Scripted(BAD), gap="CLOACINA-T-1084")
        entry = run(rule_, ledger, [T0])
        self.assertEqual(engine.shown(rule_, entry), engine.SHOW_GAP)
        self.assertFalse(engine.is_red([(rule_, entry)]))
        # The break is still counted: the ledger keeps the facts.
        self.assertEqual(entry["breaks"], 1)

    def test_a_named_gap_that_holds_is_called_out(self):
        ledger = engine.Ledger()
        rule_ = rule(Scripted(OK), gap="unfiled")
        entry = run(rule_, ledger, [T0])
        self.assertEqual(engine.shown(rule_, entry), engine.SHOW_GAP_HOLDS)

    def test_a_failure_with_no_owner_is_red(self):
        ledger = engine.Ledger()
        rule_ = rule(Scripted(BAD))
        entry = run(rule_, ledger, [T0])
        self.assertEqual(engine.shown(rule_, entry), engine.SHOW_BROKEN)
        self.assertTrue(engine.is_red([(rule_, entry)]))


class DisturbanceWindowTest(unittest.TestCase):
    WINDOW = ({"id": "d1", "kind": "kill_agent", "target": "team_a",
               "start": T0 + 100, "end": T0 + 200, "settle_until": T0 + 260},)

    def test_a_tolerated_window_does_not_break_the_streak(self):
        ledger = engine.Ledger()
        rule_ = rule(Scripted(OK, OK, BAD, BAD, OK), tolerates=("kill_agent",))
        times = [T0, T0 + 60, T0 + 120, T0 + 240, T0 + 300]
        states = []
        for at in times:
            entry = run(rule_, ledger, [at], self.WINDOW)
            states.append(entry["state"])
        self.assertEqual(states, [engine.SHOW_HOLDS, engine.SHOW_HOLDS, engine.SHOW_DISTURBED,
                                  engine.SHOW_DISTURBED, engine.SHOW_HOLDS])
        self.assertEqual(entry["breaks"], 0)
        self.assertEqual(entry["held_since"], T0)
        # The time after a disturbed pass is not time held: 120 to 240, 240 to 300.
        self.assertEqual(ledger.held_seconds("test.rule", T0 + 300), 300 - 180)

    def test_still_broken_after_the_window_is_a_break(self):
        ledger = engine.Ledger()
        rule_ = rule(Scripted(OK, BAD, BAD), tolerates=("kill_agent",))
        entry = run(rule_, ledger, [T0, T0 + 120, T0 + 300], self.WINDOW)
        self.assertEqual(entry["state"], engine.SHOW_BROKEN)
        self.assertEqual(entry["breaks"], 1)
        self.assertEqual(entry["last_break_at"], T0 + 300)
        self.assertIsNone(entry["window"])

    def test_a_window_the_rule_does_not_tolerate_is_a_break_with_the_window_named(self):
        ledger = engine.Ledger()
        rule_ = rule(Scripted(OK, BAD))
        entry = run(rule_, ledger, [T0, T0 + 120], self.WINDOW)
        self.assertEqual(entry["state"], engine.SHOW_BROKEN)
        self.assertEqual(entry["breaks"], 1)
        self.assertIn("kill_agent of team_a", entry["window"])
        self.assertIn("kill_agent", entry["transitions"][-1]["window"])

    def test_a_window_with_no_end_is_open(self):
        window = engine.Window.from_dict({"kind": "postgres_away", "start": T0})
        self.assertTrue(window.active_at(T0 + 10_000))
        self.assertFalse(window.active_at(T0 - 1))
        self.assertIsNone(window.closes)

    def test_rules_can_read_the_windows(self):
        snap = snapshot(T0 + 150, self.WINDOW)
        self.assertEqual([w.kind for w in snap.windows("kill_agent")], ["kill_agent"])
        self.assertEqual(snap.windows("postgres_away"), [])
        self.assertEqual(len(snap.active_windows()), 1)
        self.assertTrue(snap.windows()[0].overlaps(T0, T0 + 120))
        self.assertFalse(snap.windows()[0].overlaps(T0 + 300, T0 + 400))

    def test_a_record_with_bad_lines_gives_the_good_ones(self):
        lines = ['{"kind": "kill_agent", "start": "2026-11-27T12:00:00Z", "end": "2026-11-27T12:00:05Z"}',
                 "", "# a comment", '{"kind": "no start"}', "{half a line"]
        windows = engine.read_windows(lines)
        self.assertEqual(len(windows), 1)
        self.assertEqual(windows[0].end - windows[0].start, 5)


class RestartTest(unittest.TestCase):
    def test_the_state_survives_a_restart_of_the_checker(self):
        with tempfile.TemporaryDirectory() as folder:
            path = os.path.join(folder, "state.json")
            first = engine.Ledger(path)
            rule_ = rule(Scripted(OK, OK, OK))
            run(rule_, first, [T0, T0 + 60])
            first.save()

            second = engine.Ledger(path)
            entry = run(rule_, second, [T0 + 120])
            self.assertEqual(entry["held_since"], T0)
            self.assertEqual(entry["passes"], 3)
            self.assertEqual(second.held_seconds("test.rule", T0 + 120), 120)

    def test_time_the_checker_was_down_is_not_time_held(self):
        with tempfile.TemporaryDirectory() as folder:
            path = os.path.join(folder, "state.json")
            first = engine.Ledger(path, max_gap_s=300)
            rule_ = rule(Scripted(OK, OK, OK))
            run(rule_, first, [T0, T0 + 60])
            first.save()

            second = engine.Ledger(path, max_gap_s=300)
            entry = run(rule_, second, [T0 + 60 + 7200])
            # Still the same streak, but the two hours of silence are not counted.
            self.assertEqual(entry["held_since"], T0)
            self.assertEqual(second.held_seconds("test.rule", T0 + 60 + 7200), 60)


class RegistryTest(unittest.TestCase):
    def test_a_sound_registry_has_no_problems(self):
        registry = engine.Registry()

        @registry.rule("a.one", "publish.own_package", "a", about="checks one thing")
        def one(_snap):
            return OK

        self.assertEqual(registry.problems(), [])

    def test_problems_are_named(self):
        registry = engine.Registry()
        registry.rule("a.one", "no.such.promise", "a", about="x")(lambda s: OK)
        registry.rule("a.one", "publish.own_package", "a", gap="T-1", about="x")(lambda s: OK)
        text = "\n".join(registry.problems())
        self.assertIn("unknown promise", text)
        self.assertIn("two rules have the id", text)
        self.assertIn("not a task code", text)

    def test_two_sources_with_one_name_must_agree(self):
        registry = engine.Registry()
        registry.need_db("runs", "SELECT 1")
        registry.need_db("runs", "SELECT 1")
        with self.assertRaises(ValueError):
            registry.need_db("runs", "SELECT 2")


class TimeTest(unittest.TestCase):
    def test_the_forms_of_time_the_sources_give(self):
        base = engine.parse_time("2026-11-27T12:00:00Z")
        for text in ("2026-11-27T12:00:00+00:00", "2026-11-27 12:00:00+00", "2026-11-27T12:00:00",
                     "2026-11-27T12:00:00.000000", "2026-11-27T13:00:00+01:00"):
            self.assertEqual(engine.parse_time(text), base, text)
        self.assertAlmostEqual(engine.parse_time("2026-11-27T12:00:00.5"), base + 0.5)
        self.assertAlmostEqual(engine.parse_time("2026-11-27T12:00:00.123456789+00:00"), base + 0.123456)
        self.assertEqual(engine.parse_time(base), base)
        for bad in (None, "", "soon", True):
            self.assertIsNone(engine.parse_time(bad))
        self.assertEqual(engine.format_time(base), "2026-11-27T12:00:00Z")


if __name__ == "__main__":
    unittest.main()
