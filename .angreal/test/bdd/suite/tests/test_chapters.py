"""The dry run of the chapters: every feature parsed, every step resolved.

The driver's chapter files import no angreal, so the same check that
`angreal test bdd standing --dry-run` prints can run here with the system
Python.
"""

from __future__ import annotations

import os
import shutil
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
BDD = os.path.dirname(os.path.dirname(HERE))
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(BDD)))

# The folder that holds the `bdd` package.
sys.path.insert(0, os.path.dirname(BDD))

from bdd import chapters, gherkin


class DryRunTest(unittest.TestCase):
    def test_the_dry_run_of_this_checkout_passes(self):
        lines, problems = chapters.dry_run(ROOT)
        self.assertEqual(problems, [])
        text = "\n".join(lines)
        self.assertIn("chapter publishing", text)
        self.assertIn("chapter standing", text)
        self.assertIn("features/standing/standing_workload.feature: 5 scenarios", text)
        self.assertIn("no rule yet", text)

    def test_the_chapters_are_found_by_their_files(self):
        found = {chapter.name: chapter for chapter in chapters.discover()}
        self.assertEqual(sorted(found), ["publishing", "schedules", "standing"])
        self.assertEqual(found["standing"].steps_file, "standing_steps.py")
        self.assertTrue(found["standing"].has_rules)
        self.assertEqual(found["publishing"].features,
                         ("features/publishing/independent_publish.feature",))
        with self.assertRaises(gherkin.GherkinError):
            chapters.find("no-such-chapter")

    def test_each_scenario_of_the_workload_feature_ends_in_a_then(self):
        feature = chapters.load_features(chapters.find("standing"), ROOT)[0]
        self.assertEqual(len(feature.background), 2)
        for scenario in feature.scenarios:
            self.assertEqual(scenario.steps[-1].kind, "Then", scenario.name)
            self.assertFalse(scenario.is_gap, scenario.name)

    def test_a_step_with_no_definition_and_a_feature_nobody_owns_are_problems(self):
        with tempfile.TemporaryDirectory() as root:
            shutil.copytree(os.path.join(ROOT, "features"), os.path.join(root, "features"))
            shutil.copytree(os.path.join(ROOT, "examples", "fixtures"),
                            os.path.join(root, "examples", "fixtures"),
                            ignore=shutil.ignore_patterns("target", "*.rs", "Cargo.*"))
            feature = os.path.join(root, "features", "standing", "standing_workload.feature")
            with open(feature, "a", encoding="utf-8") as handle:
                handle.write("\n  Scenario: A step nobody wrote\n"
                             "    When a team does a thing with no definition\n"
                             "    Then nothing can be said about it\n")
            os.makedirs(os.path.join(root, "features", "orphan"))
            with open(os.path.join(root, "features", "orphan", "orphan.feature"), "w",
                      encoding="utf-8") as handle:
                handle.write("Feature: Nobody owns this\n\n  Scenario: One\n    Then it is unowned\n")
            _lines, problems = chapters.dry_run(root)
            text = "\n".join(problems)
            self.assertIn("no step definition for: When a team does a thing with no definition", text)
            self.assertIn("no step definition for: Then nothing can be said about it", text)
            self.assertIn("features/orphan/orphan.feature: no steps file names this feature", text)
            self.assertEqual(len(problems), 3, text)


if __name__ == "__main__":
    unittest.main()
