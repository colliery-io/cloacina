"""The description of the workload, and its fixture packages.

The fixtures are loaded with a stand-in for the `cloaca` module, so their
plain Python can be tested with no Cloacina runtime. The self-driving
workflow is run against a small HTTP server of this test, on localhost.
"""

from __future__ import annotations

import http.server
import importlib.util
import io
import json
import os
import re
import sys
import tarfile
import threading
import types
import unittest

from suite import promises, workload

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = HERE
for _ in range(5):
    ROOT = os.path.dirname(ROOT)
FIXTURES = os.path.join(ROOT, "examples", "fixtures")


def fake_cloaca():
    """A `cloaca` whose decorators give the function back unchanged."""
    module = types.ModuleType("cloaca")

    def passthrough(*_args, **_kwargs):
        return lambda fn: fn

    for name in ("task", "trigger", "workflow_params", "workflow_secrets"):
        setattr(module, name, passthrough)
    return module


def load_fixture(package, module_name):
    path = os.path.join(FIXTURES, package, "workflow", module_name, "tasks.py")
    saved = sys.modules.get("cloaca")
    sys.modules["cloaca"] = fake_cloaca()
    # No bytecode cache in a fixture folder: `cloacinactl package pack` would
    # put it into the package.
    no_bytecode, sys.dont_write_bytecode = sys.dont_write_bytecode, True
    try:
        spec = importlib.util.spec_from_file_location(f"fixture_{module_name}", path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
    finally:
        sys.dont_write_bytecode = no_bytecode
        if saved is None:
            del sys.modules["cloaca"]
        else:
            sys.modules["cloaca"] = saved
    return module


class Context:
    """The part of a task's context the fixtures use."""

    def __init__(self, values=None, secrets=None):
        self.values = dict(values or {})
        self.secrets = dict(secrets or {})

    def get(self, key, default=None):
        return self.values.get(key, default)

    def set(self, key, value):
        self.values[key] = value

    def secret(self, name):
        return self.secrets[name]


class DescriptionTest(unittest.TestCase):
    def test_the_description_and_the_promises_table_are_sound(self):
        self.assertEqual(promises.problems(), [])
        self.assertEqual(workload.problems(), [])

    def test_every_row_of_the_promises_table_has_work_to_judge(self):
        for row in promises.ROWS:
            if row == "Workload":
                continue
            ids = [p for p in promises.PROMISES if promises.row_of(p) == row]
            self.assertTrue(any(workload.EVIDENCE[p] for p in ids), row)

    def test_every_package_has_its_fixture(self):
        for package in workload.PACKAGES:
            folder = os.path.join(FIXTURES, package.fixture)
            with open(os.path.join(folder, "package.toml"), encoding="utf-8") as handle:
                manifest = handle.read()
            self.assertIn(f'name = "{package.name}"', manifest)
            self.assertIn(f'workflow_name = "{package.workflow}"', manifest)
            self.assertIn(f'language = "{package.language}"', manifest)
            if package.language == "python":
                module = re.search(r'entry_module = "([a-z_]+)\.tasks"', manifest).group(1)
                self.assertTrue(os.path.exists(os.path.join(folder, "workflow", module, "tasks.py")))
                self.assertTrue(os.path.exists(os.path.join(folder, "workflow", module, "__init__.py")))
            else:
                for rel in ("Cargo.toml", "build.rs", os.path.join("src", "lib.rs")):
                    self.assertTrue(os.path.exists(os.path.join(folder, rel)), rel)

    def test_the_python_fixtures_are_valid_python(self):
        for package in workload.PACKAGES:
            if package.language != "python":
                continue
            folder = os.path.join(FIXTURES, package.fixture, "workflow")
            for base, _dirs, files in os.walk(folder):
                for name in files:
                    if name.endswith(".py"):
                        path = os.path.join(base, name)
                        with open(path, encoding="utf-8") as handle:
                            compile(handle.read(), path, "exec")

    def test_a_package_schedule_is_the_cron_its_fixture_declares(self):
        for schedule in workload.SCHEDULES:
            if schedule.kind != "package":
                continue
            package = next(p for p in workload.PACKAGES if p.workflow == schedule.workflow)
            folder = os.path.join(FIXTURES, package.fixture)
            source = os.path.join(folder, "src", "lib.rs") if package.language == "rust" else \
                os.path.join(folder, "workflow", schedule.workflow, "tasks.py")
            with open(source, encoding="utf-8") as handle:
                text = handle.read()
            self.assertIn(f'cron = "{schedule.cron}"' if package.language == "rust"
                          else f'cron="{schedule.cron}"', text, schedule.workflow)

    def test_the_slot_of_a_fire_time(self):
        noon = 1_795_780_800  # the start of an hour
        self.assertEqual(noon % 3600, 0)
        self.assertEqual(workload.slot_of(noon), 0)
        self.assertEqual(workload.slot_of(noon + 15 * 60), 3)
        self.assertEqual(workload.slot_of(noon + 59 * 60), 11)
        self.assertEqual(workload.slot_of(noon, offset=4), 4)
        self.assertEqual(workload.slot_of(noon + 50 * 60, offset=4), 2)


class DriverFixtureTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.driver = load_fixture("standing-driver-py", "standing_driver")

    def test_the_fixture_and_the_description_agree(self):
        driver = self.driver
        self.assertEqual(driver.WORKFLOW, workload.DRIVER_WORKFLOW)
        self.assertEqual(driver.SECRET, workload.DRIVER_SECRET)
        self.assertEqual(driver.ACTIONS_KEY, workload.DRIVER_ACTIONS_KEY)
        self.assertEqual(driver.SLOT_SECONDS, workload.SLOT_SECONDS)
        self.assertEqual(driver.CYCLE_SLOTS, workload.CYCLE_SLOTS)
        self.assertEqual(driver.SLOT_ACTIONS, workload.SLOT_ACTIONS)
        self.assertEqual(driver.TARGETS, workload.TARGETS)
        self.assertEqual(driver.TARGET_PY_WORKFLOW, workload.TARGET_PY_WORKFLOW)
        self.assertEqual(driver.TARGET_RUST_WORKFLOW, workload.TARGET_RUST_WORKFLOW)
        self.assertEqual(driver.HOLD_WORKFLOW, workload.HOLD_WORKFLOW)
        self.assertEqual(driver.HOLD_SECONDS, workload.HOLD_SECONDS)
        self.assertEqual(driver.HOLD_BURST, workload.HOLD_BURST)
        self.assertEqual(driver.slot_of(1_795_780_800 + 900, 4), workload.slot_of(1_795_780_800 + 900, 4))

    def test_the_cycle_does_each_thing_the_model_asks_for(self):
        done = {}
        for slot in range(self.driver.CYCLE_SLOTS):
            for step in self.driver.plan_actions(slot):
                done[step["action"]] = done.get(step["action"], 0) + 1
        self.assertEqual(done["publish"], 3)
        self.assertEqual(done["publish_broken"], 1)
        self.assertEqual(done["step_back"], 1)
        self.assertEqual(done["start_holds"], 1)
        self.assertEqual(done["run_by_hand"], 12)
        self.assertEqual(done["other_tenant"], 12)

    def test_an_archive_is_a_bzip2_tar_under_name_and_version(self):
        files = self.driver.target_files("py", "0.1.4")
        archive = self.driver.build_archive("standing-target-py", "0.1.4", files)
        with tarfile.open(fileobj=io.BytesIO(archive), mode="r:bz2") as tar:
            names = tar.getnames()
            manifest = tar.extractfile("standing-target-py-0.1.4/package.toml").read().decode()
            tasks = tar.extractfile(
                "standing-target-py-0.1.4/workflow/standing_target_py/tasks.py").read().decode()
        # The same shape as an archive of `cloacinactl package pack`: files
        # only, each under `<name>-<version>/`.
        self.assertEqual(sorted(names), [
            "standing-target-py-0.1.4/package.toml",
            "standing-target-py-0.1.4/workflow/standing_target_py/__init__.py",
            "standing-target-py-0.1.4/workflow/standing_target_py/tasks.py"])
        self.assertIn('version = "0.1.4"', manifest)
        self.assertIn('workflow_name = "standing_target_py"', manifest)
        self.assertIn('PACKAGE_VERSION = "0.1.4"', tasks)
        compile(tasks, "tasks.py", "exec")

    def test_the_rust_target_and_its_broken_version(self):
        good = self.driver.target_files("rust", "0.1.2")
        bad = self.driver.target_files("rust", "0.1.3", broken=True)
        self.assertEqual(sorted(good), ["Cargo.toml", "build.rs", "package.toml", "src/lib.rs"])
        self.assertIn('version = "0.1.2"', good["Cargo.toml"])
        self.assertIn('name = "standing_target_rust"', good["src/lib.rs"])
        self.assertNotIn(self.driver.BROKEN_MARK, good["src/lib.rs"])
        self.assertIn(self.driver.BROKEN_MARK, bad["src/lib.rs"])
        for files in (good, bad):
            for text in files.values():
                self.assertNotIn("@VERSION@", text)
                self.assertNotIn("@BODY@", text)
        # The same crate versions, edition and toolchain as the Rust fixture
        # of the workload. A release that moves the fixture's pins fails here
        # until the source text in the self-driving workflow moves too.
        with open(os.path.join(FIXTURES, "standing-rollup-rust", "Cargo.toml"), encoding="utf-8") as handle:
            pinned = [line.strip() for line in handle
                      if line.startswith(("cloacina-", "serde_json", "edition", "rust-version"))]
        self.assertEqual(len(pinned), 7, pinned)
        for line in pinned:
            self.assertIn(line, good["Cargo.toml"])

    def test_the_next_version(self):
        self.assertEqual(self.driver.next_version([]), "0.1.0")
        self.assertEqual(self.driver.next_version([{"version": "0.1.0"}, {"version": "0.1.9"},
                                                   {"version": "0.1.10"}]), "0.1.11")
        self.assertEqual(self.driver.next_version([{"version": "odd"}]), "0.1.0")

    def test_the_version_to_go_back_to(self):
        rows = [{"version": "0.1.4", "active": True, "build_status": "success"},
                {"version": "0.1.3", "active": False, "build_status": "failed"},
                {"version": "0.1.2", "active": False, "build_status": "success"},
                {"version": "0.1.10", "active": False, "build_status": "success"}]
        self.assertEqual(self.driver.version_to_go_back_to(rows), "0.1.2")
        self.assertIsNone(self.driver.version_to_go_back_to(rows[:2]))
        self.assertIsNone(self.driver.version_to_go_back_to([]))


class StubServer:
    """Answers the calls of the self-driving workflow, and keeps them."""

    def __init__(self):
        self.calls = []
        self.versions = {}
        stub = self

        class Handler(http.server.BaseHTTPRequestHandler):
            def log_message(self, format, *args):
                pass

            def _answer(self, status, body):
                data = json.dumps(body).encode()
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)

            def do_GET(self):
                stub.calls.append(("GET", self.path, self.headers.get("Authorization")))
                match = re.match(r"^/v1/tenants/team_a/workflows/([a-z-]+)/versions$", self.path)
                if match:
                    self._answer(200, {"items": list(reversed(stub.versions.get(match.group(1), [])))})
                elif self.path.startswith("/v1/tenants/team_b/"):
                    self._answer(403, {"error": "access denied for this tenant"})
                else:
                    self._answer(404, {})

            def do_POST(self):
                body = self.rfile.read(int(self.headers.get("Content-Length") or 0))
                stub.calls.append(("POST", self.path, self.headers.get("Authorization")))
                if self.path == "/v1/tenants/team_a/workflows":
                    boundary = self.headers["Content-Type"].split("boundary=")[1].encode()
                    part = body.split(b"--" + boundary)[1]
                    head, _, archive = part.partition(b"\r\n\r\n")
                    assert b'name="file"' in head, head
                    archive = archive[:-2]  # the line break before the closing boundary
                    with tarfile.open(fileobj=io.BytesIO(archive), mode="r:bz2") as tar:
                        top = tar.getnames()[0].split("/")[0]
                        manifest = tar.extractfile(f"{top}/package.toml").read().decode()
                        broken = any(name.endswith("lib.rs") and b"does_not_exist" in
                                     tar.extractfile(name).read() for name in tar.getnames()
                                     if tar.getmember(name).isfile())
                    name = re.search(r'name = "([^"]+)"', manifest).group(1)
                    version = re.search(r'version = "([^"]+)"', manifest).group(1)
                    rows = stub.versions.setdefault(name, [])
                    if not broken:
                        for row in rows:
                            row["active"] = False
                    rows.append({"version": version, "active": not broken,
                                 "build_status": "failed" if broken else "success"})
                    self._answer(201, {"id": "0"})
                elif self.path.endswith("/execute"):
                    self._answer(202, {"execution_id": "7b1e0000-0000-4000-8000-000000000001"})
                elif self.path.endswith("/activate"):
                    self._answer(200, {})
                else:
                    self._answer(404, {})

        self.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.url = f"http://127.0.0.1:{self.server.server_address[1]}"
        threading.Thread(target=self.server.serve_forever, daemon=True).start()

    def close(self):
        self.server.shutdown()
        self.server.server_close()


class DriverRunTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.driver = load_fixture("standing-driver-py", "standing_driver")

    def setUp(self):
        self.stub = StubServer()
        self.saved = os.environ.get("CLOACINA_SERVER")
        os.environ["CLOACINA_SERVER"] = self.stub.url
        self.addCleanup(self.restore)

    def restore(self):
        self.stub.close()
        if self.saved is None:
            os.environ.pop("CLOACINA_SERVER", None)
        else:
            os.environ["CLOACINA_SERVER"] = self.saved

    def run_at(self, scheduled_time):
        context = Context(
            {"tenant": "team_a", "offset": 0, "other_tenant": "team_b", "scheduled_time": scheduled_time},
            {"standing_driver_key": {"key": "write-key-of-team-a"}})
        with open(os.devnull, "w", encoding="utf-8") as quiet:
            saved, sys.stdout = sys.stdout, quiet
            try:
                self.driver.plan(context)
                self.driver.act(context)
            finally:
                sys.stdout = saved
        return context.values

    def test_the_first_run_publishes_both_targets_and_then_follows_its_slot(self):
        values = self.run_at("2026-11-27T12:00:00+00:00")
        self.assertEqual(values["standing_driver_plan"]["slot"], 0)
        actions = values["standing_driver_actions"]
        published = [(a["package"], a["version"], a["status"]) for a in actions if a["action"] == "publish"]
        self.assertEqual(published, [("standing-target-py", "0.1.0", 201),
                                     ("standing-target-rust", "0.1.0", 201),
                                     ("standing-target-py", "0.1.1", 201)])
        by_hand = next(a for a in actions if a["action"] == "run_by_hand")
        self.assertEqual((by_hand["workflow"], by_hand["status"]), ("standing_target_py", 202))
        self.assertTrue(by_hand["execution_id"])
        other = next(a for a in actions if a["action"] == "other_tenant")
        self.assertEqual((other["packages_status"], other["executions_status"], other["secrets_status"]),
                         (403, 403, 403))
        self.assertEqual(other["secret_by_name"], "KeyError")
        for record in actions:
            self.assertRegex(record["at"], r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$")
        # Every call carries the tenant's key, and no record holds it.
        self.assertTrue(all(call[2] == "Bearer write-key-of-team-a" for call in self.stub.calls))
        self.assertNotIn("write-key-of-team-a", json.dumps(values))

    def test_a_cycle_of_publishes_a_build_error_and_a_step_back(self):
        self.run_at("2026-11-27T12:00:00+00:00")
        broken = self.run_at("2026-11-27T12:15:00+00:00")["standing_driver_actions"][0]
        self.assertEqual((broken["action"], broken["package"], broken["version"], broken["status"]),
                         ("publish_broken", "standing-target-rust", "0.1.1", 201))
        holds = self.run_at("2026-11-27T12:20:00+00:00")["standing_driver_actions"][0]
        self.assertEqual((holds["action"], holds["count"]), ("start_holds", workload.HOLD_BURST))
        self.assertEqual(holds["statuses"], [202] * workload.HOLD_BURST)
        back = self.run_at("2026-11-27T12:35:00+00:00")["standing_driver_actions"][0]
        self.assertEqual((back["action"], back["package"], back["version"], back["status"]),
                         ("step_back", "standing-target-py", "0.1.0", 200))
        self.assertIn(("POST", "/v1/tenants/team_a/workflows/standing-target-py/versions/0.1.0/activate",
                       "Bearer write-key-of-team-a"), self.stub.calls)
        rust = self.run_at("2026-11-27T12:45:00+00:00")["standing_driver_actions"][0]
        self.assertEqual((rust["action"], rust["package"], rust["version"]),
                         ("publish", "standing-target-rust", "0.1.2"))

    def test_with_no_key_it_records_that_and_calls_nothing(self):
        context = Context({"tenant": "team_a", "scheduled_time": "2026-11-27T12:00:00+00:00"})
        with open(os.devnull, "w", encoding="utf-8") as quiet:
            saved, sys.stdout = sys.stdout, quiet
            try:
                self.driver.plan(context)
                self.driver.act(context)
            finally:
                sys.stdout = saved
        actions = context.values["standing_driver_actions"]
        self.assertEqual([(a["action"], a["error"]) for a in actions], [("read_key", "KeyError")])
        self.assertEqual(self.stub.calls, [])

    def test_a_server_that_is_away_gives_records_with_no_status(self):
        self.stub.close()
        values = self.run_at("2026-11-27T12:00:00+00:00")
        statuses = [a.get("status") for a in values["standing_driver_actions"] if "status" in a]
        self.assertTrue(statuses)
        self.assertTrue(all(status is None for status in statuses))


class OtherFixturesTest(unittest.TestCase):
    def test_the_planned_failures_of_the_flaky_job(self):
        flaky = load_fixture("standing-flaky-py", "standing_flaky")
        self.assertEqual(flaky.PLANNED_FAILURE_MINUTES, workload.PLANNED_FAILURE_MINUTES)
        self.assertTrue(flaky.planned_failure("2026-11-27T12:00:00+00:00"))
        self.assertTrue(flaky.planned_failure("2026-11-27T12:30:00Z"))
        self.assertFalse(flaky.planned_failure("2026-11-27T12:05:00+00:00"))
        self.assertFalse(flaky.planned_failure(None))
        self.assertFalse(flaky.planned_failure("not a time"))

    def test_the_flaky_job_fails_once_for_a_fire_time_and_then_passes(self):
        flaky = load_fixture("standing-flaky-py", "standing_flaky")
        fired = f"2026-11-27T12:05:00+00:00 test {os.getpid()}"
        path = flaky._memory_path(fired)
        self.addCleanup(lambda: os.path.exists(path) and os.remove(path))
        with open(os.devnull, "w", encoding="utf-8") as quiet:
            saved, sys.stdout = sys.stdout, quiet
            try:
                with self.assertRaises(RuntimeError):
                    flaky.wobble(Context({"scheduled_time": fired}))
                context = flaky.wobble(Context({"scheduled_time": fired}))
                self.assertTrue(context.values["standing_flaky_wobbled"])
                with self.assertRaises(RuntimeError):
                    flaky.settle(Context({"scheduled_time": "2026-11-27T12:30:00+00:00"}))
                done = flaky.settle(Context({"scheduled_time": "2026-11-27T12:35:00+00:00"}))
                self.assertIn(flaky.RESULT_KEY, done.values)
                # A run started by hand has no fire time: it fails nowhere.
                flaky.settle(flaky.wobble(Context({})))
            finally:
                sys.stdout = saved

    def test_the_data_job_writes_back_the_interval_it_was_given(self):
        extract = load_fixture("standing-extract-py", "standing_extract")
        context = Context({"interval_start": "2026-11-27T12:00:00+00:00",
                           "interval_end": "2026-11-27T12:05:00+00:00",
                           "scheduled_time": "2026-11-27T12:05:00+00:00"})
        with open(os.devnull, "w", encoding="utf-8") as quiet:
            saved, sys.stdout = sys.stdout, quiet
            try:
                extract.load(extract.transform(extract.extract(context)))
            finally:
                sys.stdout = saved
        loaded = context.values[extract.RESULT_KEY]
        self.assertEqual(loaded["interval_start"], "2026-11-27T12:00:00+00:00")
        self.assertEqual(loaded["interval_end"], "2026-11-27T12:05:00+00:00")
        self.assertEqual(loaded["rows"] % 7, 0)

    def test_the_holding_jobs_hold_for_what_the_context_says(self):
        for package, module, task in (("standing-hold-py", "standing_hold", "hold"),
                                      ("standing-archive-py", "standing_archive", "archive")):
            fixture = load_fixture(package, module)
            context = Context({"hold_seconds": 0})
            with open(os.devnull, "w", encoding="utf-8") as quiet:
                saved, sys.stdout = sys.stdout, quiet
                try:
                    getattr(fixture, task)(context)
                finally:
                    sys.stdout = saved
            self.assertIn(fixture.RESULT_KEY, context.values)
        self.assertEqual(load_fixture("standing-hold-py", "standing_hold").HOLD_SECONDS,
                         workload.HOLD_SECONDS)


if __name__ == "__main__":
    unittest.main()
