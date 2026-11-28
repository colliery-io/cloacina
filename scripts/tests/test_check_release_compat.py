"""Unit tests of the release gate, `scripts/check_release_compat.py` (CLOACINA-T-1089).

    python3 -I scripts/tests/test_check_release_compat.py

Needs the standard library and PyYAML. No git, no helm: the tests compare two
fixture folders (`fixtures/release_compat/base` and `head`). A fixture file
named `*.fixture` is copied without that suffix, so that the repository's
Rust and Cargo tooling does not read it.
"""

from __future__ import annotations

import os
import re
import shutil
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPTS = os.path.dirname(HERE)
ROOT = os.path.dirname(SCRIPTS)
sys.path.insert(0, SCRIPTS)
sys.path.insert(0, os.path.join(ROOT, ".angreal"))

import check_release_compat as gate  # noqa: E402
import version_lockstep  # noqa: E402

FIXTURES = os.path.join(HERE, "fixtures", "release_compat")


def stage(name: str, dest: str) -> str:
    """Copy a fixture folder to `dest`, dropping the `.fixture` suffixes."""
    out = os.path.join(dest, name)
    shutil.copytree(os.path.join(FIXTURES, name), out)
    for dirpath, _dirs, files in os.walk(out):
        for fname in files:
            if fname.endswith(".fixture"):
                path = os.path.join(dirpath, fname)
                os.replace(path, path[: -len(".fixture")])
    return out


class FixtureCase(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="release-gate-test-")
        self.base = gate.TreeSource(stage("base", self.tmp), "v0.13.0")
        self.head_root = stage("head", self.tmp)
        self.head = gate.TreeSource(self.head_root, "head")

    def tearDown(self):
        shutil.rmtree(self.tmp)

    def write_head(self, rel: str, text: str):
        with open(os.path.join(self.head_root, rel), "w", encoding="utf-8") as fh:
            fh.write(text)

    def read_head(self, rel: str) -> str:
        return self.head.read(rel)


def changes(findings):
    return [f.change for f in findings]


class OpenApiTests(FixtureCase):
    def findings(self):
        return gate.compare_openapi(self.base.read(gate.OPENAPI), self.head.read(gate.OPENAPI))

    def test_a_removed_path_is_a_break(self):
        found = [f for f in self.findings() if "DELETE /v1/things/{id}" in f.change]
        self.assertEqual(len(found), 1)
        self.assertEqual(found[0].kind, gate.BREAK)
        self.assertIn("DELETE /v1/things/{id}", found[0].names)

    def test_a_request_field_made_required_is_a_break(self):
        found = [f for f in self.findings() if "NewThing.colour" in f.change]
        self.assertEqual([f.kind for f in found], [gate.BREAK])
        self.assertIn("now required", found[0].change)

    def test_an_enum_value_added_to_a_response_is_a_note(self):
        found = [f for f in self.findings() if "Thing.state" in f.change]
        self.assertEqual([f.kind for f in found], [gate.NOTE])
        self.assertIn("failed", found[0].change)

    def test_an_enum_value_removed_from_a_response_is_a_break(self):
        old = self.head.read(gate.OPENAPI)
        new = old.replace('"enum": ["new", "done", "failed"]', '"enum": ["new"]')
        found = gate.compare_openapi(old, new)
        self.assertEqual([f.kind for f in found], [gate.BREAK])
        self.assertIn("done, failed", found[0].change)

    def test_a_removed_response_field_is_a_break(self):
        old = self.head.read(gate.OPENAPI)
        new = old.replace('"id": {"type": "string"},\n', "").replace('["id", "state"]', '["state"]')
        found = gate.compare_openapi(old, new)
        self.assertEqual(changes(found), ["response field `Thing.id` removed (in `GET /v1/things`)"])


class ValuesTests(FixtureCase):
    def findings(self):
        path = "charts/cloacina-server/values.yaml"
        return gate.compare_values("cloacina-server", self.base.read(path), self.head.read(path))

    def test_a_renamed_value_is_a_break(self):
        found = [f for f in self.findings() if f.kind == gate.BREAK]
        self.assertEqual(changes(found), ["value `server.drainSeconds` removed or renamed"])
        self.assertIn("server.drainSeconds", found[0].names)

    def test_a_changed_default_is_a_note(self):
        found = [f for f in self.findings() if f.kind == gate.NOTE]
        self.assertEqual(len(found), 1)
        self.assertIn("server.logFormat", found[0].change)

    def test_an_empty_map_that_gets_keys_is_no_change(self):
        self.assertFalse([f for f in self.findings() if "resources" in f.change])


class MetricsTests(FixtureCase):
    def test_a_dropped_label_is_a_break(self):
        found = gate.compare_catalog(self.base.read(gate.CATALOG), self.head.read(gate.CATALOG))
        self.assertEqual(changes(found), ["label `tenant` of metric `cloacina_jobs_total` removed"])
        self.assertEqual(found[0].kind, gate.BREAK)

    def test_the_code_is_checked_against_the_catalog(self):
        found = gate.check_code_against_catalog(self.head)
        self.assertTrue(all(f.kind == gate.ERROR for f in found))
        text = "\n".join(changes(found))
        self.assertIn("`cloacina_unlisted_total` is in the code", text)
        self.assertIn("label `state` of metric `cloacina_queue_depth`", text)
        # A const name is resolved, a comment and a test module are not read.
        self.assertNotIn("cloacina_sweeps_total", text)
        self.assertNotIn("cloacina_commented_out_total", text)
        self.assertNotIn("cloacina_test_only_total", text)

    def test_a_catalog_row_with_no_code_fails(self):
        self.write_head("crates/demo/src/lib.rs", "pub fn nothing() {}\n")
        text = "\n".join(changes(gate.check_code_against_catalog(self.head)))
        self.assertIn("`cloacina_jobs_total` is in the catalog and no code", text)


class GateTests(FixtureCase):
    def test_a_marked_break_passes_and_an_unmarked_one_fails(self):
        result = gate.run_gate(self.base, self.head, render=False)
        passed = "\n".join(changes(result.passed))
        failed = "\n".join(changes(result.failures))
        # Marked in the Unreleased section of the fixture changelog.
        self.assertIn("operation `DELETE /v1/things/{id}` removed", passed)
        self.assertIn("NewThing.colour", passed)
        self.assertIn("`AGENT_PROTOCOL_VERSION` changed from 1 to 2", passed)
        self.assertIn("migration `002_add_thing_state`", passed)
        # Not marked.
        self.assertIn("value `server.drainSeconds` removed or renamed", failed)
        self.assertIn("label `tenant` of metric `cloacina_jobs_total` removed", failed)
        # No changelog entry fixes these.
        self.assertIn("`cloacina_unlisted_total` is in the code", failed)
        # Notes do not fail a minor.
        self.assertTrue(any("Thing.state" in c for c in changes(result.notes)))

    def test_a_name_outside_the_two_sections_does_not_mark(self):
        text = self.read_head(gate.CHANGELOG).replace(
            "### Changed (breaking)", "### Fixed")
        text = text.replace("### Upgrade notes", "### Added")
        self.write_head(gate.CHANGELOG, text)
        result = gate.run_gate(self.base, self.head, render=False)
        self.assertFalse(result.passed)

    def test_a_name_in_the_other_section_does_not_mark(self):
        # The migration number under Changed (breaking), the protocol under
        # Upgrade notes: each is in the wrong section.
        text = self.read_head(gate.CHANGELOG)
        text = text.replace("(`AGENT_PROTOCOL_VERSION`)", "(protocol 2)")
        text = text.replace("Migration `002`", "Migration 002")
        text = text.replace("- Things can no longer", "- Migration `002` too.\n- Things can no longer")
        text += "\n"
        text = text.replace("### Upgrade notes\n", "### Upgrade notes\n\n- `AGENT_PROTOCOL_VERSION` is 2.\n")
        self.write_head(gate.CHANGELOG, text)
        result = gate.run_gate(self.base, self.head, render=False)
        failed = "\n".join(changes(result.failures))
        self.assertIn("`AGENT_PROTOCOL_VERSION` changed from 1 to 2", failed)
        self.assertIn("migration `002_add_thing_state`", failed)

    def test_the_failure_names_file_change_and_section(self):
        result = gate.run_gate(self.base, self.head, render=False)
        f = next(f for f in result.failures if "drainSeconds" in f.change)
        text = gate.describe(f, "v0.13.0", "Unreleased", patch=False)
        self.assertEqual(
            text,
            "charts/cloacina-server/values.yaml: value `server.drainSeconds` removed or renamed "
            "since v0.13.0; name `server.drainSeconds` under `## [Unreleased]` / "
            "`### Changed (breaking)`")

    def test_a_patch_release_fails_every_change_even_marked(self):
        self.write_head("Cargo.toml", '[workspace.package]\nversion = "0.13.1"\n')
        changelog = self.read_head(gate.CHANGELOG).replace(
            "## [Unreleased]", "## [Unreleased]\n\n## [0.13.1] - 2026-12-02", 1)
        self.write_head(gate.CHANGELOG, changelog)
        result = gate.run_gate(self.base, self.head, render=False)
        self.assertFalse(result.passed)
        failed = "\n".join(changes(result.failures))
        self.assertIn("operation `DELETE /v1/things/{id}` removed", failed)
        self.assertIn("Thing.state", failed)  # a note fails a patch too
        self.assertIn("has entries under `### Changed (breaking)`", failed)
        self.assertIn("patch release", result.header[0])

    def test_a_minor_bump_reads_the_new_version_section(self):
        self.write_head("Cargo.toml", '[workspace.package]\nversion = "0.14.0"\n')
        changelog = self.read_head(gate.CHANGELOG).replace(
            "## [Unreleased]", "## [Unreleased]\n\n## [0.14.0] - 2026-12-02", 1)
        self.write_head(gate.CHANGELOG, changelog)
        result = gate.run_gate(self.base, self.head, render=False)
        self.assertIn("`## [0.14.0]`", result.header[1])
        self.assertIn("operation `DELETE /v1/things/{id}` removed", "\n".join(changes(result.passed)))

    def test_a_bump_with_entries_left_in_unreleased_fails(self):
        self.write_head("Cargo.toml", '[workspace.package]\nversion = "0.14.0"\n')
        changelog = self.read_head(gate.CHANGELOG).replace(
            "## [0.13.0]", "## [0.14.0] - 2026-12-02\n\n## [0.13.0]", 1)
        self.write_head(gate.CHANGELOG, changelog)
        result = gate.run_gate(self.base, self.head, render=False)
        self.assertIn("`## [Unreleased]` has entries that are not under `## [0.14.0]`",
                      "\n".join(changes(result.failures)))


class FlagsTests(unittest.TestCase):
    OLD = '''
struct Cli {
    /// Verbose.
    #[arg(short, long)]
    verbose: bool,
    #[arg(long, env = "CLOACINA_DRAIN_S", default_value_t = 30)]
    drain_s: u64,
    #[arg(
        long,
        env = "CLOACINA_MODE",
        default_value = "inline"
    )]
    mode: String,
    #[command(subcommand)]
    command: Option<Command>,
}
'''

    def test_the_command_is_parsed(self):
        flags = gate.parse_server_flags(self.OLD)
        self.assertEqual(sorted(flags), ["--drain-s", "--mode", "--verbose"])
        self.assertEqual(flags["--drain-s"]["env"], "CLOACINA_DRAIN_S")
        self.assertEqual(flags["--drain-s"]["default"], "30")
        self.assertEqual(flags["--mode"]["default"], '"inline"')
        self.assertEqual(flags["--verbose"]["short"], "v")

    def test_a_renamed_flag_is_a_break_and_a_new_default_a_note(self):
        new = self.OLD.replace("drain_s: u64", "drain_seconds: u64").replace('"inline"', '"worker"')

        class Src:
            def __init__(self, text):
                self.text = text

            def read(self, path):
                return self.text if path == gate.SERVER_MAIN else None

            def files(self, prefix, suffix):
                return []

        found = gate.compare_flags(Src(self.OLD), Src(new))
        kinds = {f.change: f.kind for f in found}
        self.assertEqual(kinds.get("flag `--drain-s` removed or renamed"), gate.BREAK)
        self.assertEqual(kinds.get('default of `--mode` changed from `"inline"` to `"worker"`'),
                         gate.NOTE)


class ChangelogBumpTests(unittest.TestCase):
    TEXT = (
        "# Changelog\n\n## [Unreleased]\n\n### Changed (breaking)\n\n- A break (`X`).\n\n"
        "### Upgrade notes\n\n## [0.13.0] - 2026-12-01\n\n- Old.\n"
    )

    def test_unreleased_becomes_the_version_under_a_new_empty_unreleased(self):
        out = version_lockstep.release_changelog(self.TEXT, "0.14.0", "2026-12-09")
        sections = gate.changelog_sections(out)
        self.assertEqual([k for k, _ in sections], ["Unreleased", "0.14.0", "0.13.0"])
        unreleased, released = sections[0][1], sections[1][1]
        self.assertFalse(gate.has_entries(unreleased))
        self.assertIn("### Changed (breaking)", unreleased)
        self.assertIn("### Upgrade notes", unreleased)
        self.assertIn("- A break (`X`).", released)
        # The empty Upgrade notes of the released version is dropped.
        self.assertNotIn("### Upgrade notes", released)
        self.assertTrue(re.search(r"^## \[0\.14\.0\] - 2026-12-09$", out, re.MULTILINE))

    def test_a_second_bump_changes_nothing(self):
        once = version_lockstep.release_changelog(self.TEXT, "0.14.0", "2026-12-09")
        self.assertEqual(version_lockstep.release_changelog(once, "0.14.0", "2026-12-10"), once)


if __name__ == "__main__":
    unittest.main()
