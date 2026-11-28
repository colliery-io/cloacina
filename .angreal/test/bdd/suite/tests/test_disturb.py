"""The disturbance schedule (CLOACINA-T-1109), with a fake kubectl and a fake
API. Nothing here runs kubectl or reaches a server."""

from __future__ import annotations

import json
import os
import tempfile
import unittest

from suite import disturb, engine, promises, report, rules, sources, workload

HERE = os.path.dirname(os.path.abspath(__file__))
BDD = os.path.dirname(os.path.dirname(HERE))
HEALTHY = os.path.join(HERE, "recorded", "healthy.json")

GOOD_CONFIG = {
    "current-context": "default",
    "contexts": [{"name": "default", "context": {"cluster": "default", "user": "default"}}],
    "clusters": [{"name": "default", "cluster": {"server": "https://localhost:6443"}}],
}


class Clock:
    def __init__(self, now=1_800_000_000.0):
        self.now = now

    def __call__(self):
        return self.now

    def sleep(self, seconds):
        self.now += max(seconds, 0.001)


class FakeKubectl:
    """Answers kubectl calls by their arguments, and keeps each one."""

    def __init__(self, config=None, server_pods=("bdd-standing-cloacina-server-a", "bdd-standing-cloacina-server-b"),
                 agent_pods=None, logs=None, fail=()):
        self.calls = []
        self.config = GOOD_CONFIG if config is None else config
        self.server_pods = server_pods
        self.agent_pods = agent_pods or {}
        self.logs = logs or {}
        self.fail = set(fail)
        self.on_call = None

    @staticmethod
    def _pods(names):
        return json.dumps({"items": [{"metadata": {"name": n}, "status": {"phase": "Running"}}
                                     for n in names]})

    def __call__(self, argv, env, timeout):
        assert argv[0] == "kubectl", argv
        assert argv[1:5] == ["--kubeconfig", env["KUBECONFIG"], "--context", "default"], argv
        args = argv[5:]
        self.calls.append(args)
        if self.on_call:
            self.on_call(args)
        for word in self.fail:
            if word in " ".join(args):
                return 1, "", f"failed: {word}"
        if args[:2] == ["config", "view"]:
            return 0, json.dumps(self.config), ""
        if args[:2] == ["get", "deployment"]:
            return 0, f"deployment.apps/{args[2]}\n", ""
        if args[:2] == ["get", "pods"]:
            namespace = args[args.index("-n") + 1]
            if namespace == disturb.NAMESPACE:
                return 0, self._pods(self.server_pods), ""
            return 0, self._pods(self.agent_pods.get(namespace, ())), ""
        if args[0] == "logs":
            return 0, self.logs.get(args[1], ""), ""
        return 0, "", ""

    def commands(self, verb):
        return [args for args in self.calls if args[0] == verb]


class FakeApi:
    def __init__(self, roster=None, ready=True):
        self.calls = []
        self.roster = roster if roster is not None else []
        self.ready = ready

    def call(self, method, path, key=None, body=None):
        self.calls.append((method, path, key, body))
        if path == "/v1/agents":
            return 200, {"items": self.roster}
        if path == "/ready":
            return (200 if self.ready else None), None
        return 202, {"execution_id": "x"}


class Setup:
    def __init__(self, folder, kubectl=None, api=None):
        self.clock = Clock()
        self.kubeconfig = os.path.join(folder, "kubeconfig.host.yaml")
        with open(self.kubeconfig, "w", encoding="utf-8") as handle:
            handle.write("apiVersion: v1\n")
        self.kubectl = kubectl or FakeKubectl()
        self.api = api or FakeApi()
        self.record_path = os.path.join(folder, "disturbances.jsonl")
        self.log_path = os.path.join(folder, "disturbance-log.jsonl")
        self.stop_path = os.path.join(folder, "disturb.stop")
        self.runner = disturb.Runner(
            disturb.Kube(self.kubeconfig, expected=self.kubeconfig, run=self.kubectl),
            self.api,
            {tenant: f"write-{tenant}" for tenant in workload.TENANTS},
            disturb.Record(self.record_path, self.log_path, clock=self.clock),
            read_keys={tenant: f"read-{tenant}" for tenant in workload.TENANTS},
            stop_path=self.stop_path,
            lock_path=os.path.join(folder, "disturb.lock"),
            clock=self.clock, sleep=self.clock.sleep, out=lambda text: None,
        )

    def lines(self, path=None):
        with open(path or self.record_path, "r", encoding="utf-8") as handle:
            return [json.loads(line) for line in handle if line.strip()]


class ScheduleTest(unittest.TestCase):
    def test_the_schedule_is_sound(self):
        self.assertEqual(disturb.schedule_problems(), [])

    def test_each_kind_runs_several_times_a_day_on_every_tenant_and_in_both_modes(self):
        day = disturb.plan(0, 86400 - 1)
        for kind in disturb.KINDS:
            items = [item for item in day if item.kind == kind]
            self.assertEqual(len(items), 6, kind)
            self.assertEqual({item.tenant for item in items}, set(workload.TENANTS), kind)
        kills = [item.mode for item in day if item.kind == "kill_agent"]
        self.assertEqual(sorted(set(kills)), ["force", "sigterm"])

    def test_two_windows_never_overlap(self):
        day = disturb.plan(0, 2 * 86400)
        for first, second in zip(day, day[1:]):
            longest = disturb.MAX_ACTION_SECONDS[first.kind] + disturb.SETTLE_SECONDS[first.kind]
            self.assertLess(first.at + longest, second.at, (first.describe(), second.describe()))

    def test_a_disturbance_falls_while_the_hourly_long_task_runs(self):
        # standing_archive starts each hour and holds for six minutes.
        for item in disturb.plan(0, 86400):
            self.assertLess(item.at % 3600, 6 * 60, item.describe())

    def test_a_schedule_whose_windows_meet_is_refused(self):
        saved = disturb.SCHEDULE
        try:
            disturb.SCHEDULE = saved + (disturb.Slot(0 * 3600 + 600, "postgres_away"),)
            self.assertTrue(any("can reach the next one" in p for p in disturb.schedule_problems()))
        finally:
            disturb.SCHEDULE = saved

    def test_each_kind_names_promises_of_the_table(self):
        self.assertEqual(promises.problems(), [])
        self.assertEqual(sorted(disturb.KINDS),
                         ["fill_capacity", "kill_agent", "postgres_away", "restart_server"])

    def test_the_kinds_are_the_ones_the_rules_tolerate(self):
        tolerated = {kind for rule in rules.load().rules for kind in rule.tolerates}
        self.assertLessEqual(tolerated, set(disturb.KINDS))


class DryRunTest(unittest.TestCase):
    def test_the_dry_run_prints_the_schedule_and_the_commands(self):
        text = disturb.plan_text(1_800_000_000.0)
        for kind in disturb.KINDS:
            self.assertIn(kind, text)
        self.assertIn("kubectl delete pod <the pod of that agent> -n cloacina-tenant-team-", text)
        self.assertIn("--grace-period=0 --force", text)
        self.assertIn("kubectl scale deployment/bdd-standing-postgresql --replicas=0 -n cloacina-bdd-standing", text)
        self.assertIn("POST /v1/tenants/", text)
        self.assertIn("schedule: sound", text)

    def test_the_dry_run_touches_nothing(self):
        # plan_text takes no kubectl and no API: there is nothing it could call.
        with tempfile.TemporaryDirectory() as folder:
            before = os.listdir(folder)
            disturb.plan_text(1_800_000_000.0)
            self.assertEqual(os.listdir(folder), before)


class ContainmentTest(unittest.TestCase):
    def test_another_kubeconfig_is_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            mine = os.path.join(folder, "kubeconfig.host.yaml")
            with self.assertRaises(disturb.Refused):
                disturb.Kube(os.path.expanduser("~/.kube/config"), expected=mine)

    def test_the_default_kubeconfig_is_the_suites_own(self):
        self.assertTrue(disturb.suite_kubeconfig().endswith(
            os.path.join("cloacina-k8s-bdd-standing", "kubeconfig.host.yaml")))

    def test_the_environment_holds_only_the_suites_kubeconfig(self):
        with tempfile.TemporaryDirectory() as folder:
            mine = os.path.join(folder, "kubeconfig.host.yaml")
            saved = os.environ.get("KUBECONFIG")
            os.environ["KUBECONFIG"] = "/elsewhere/config"
            try:
                kube = disturb.Kube(mine, expected=mine)
            finally:
                if saved is None:
                    del os.environ["KUBECONFIG"]
                else:
                    os.environ["KUBECONFIG"] = saved
            self.assertEqual(kube.env["KUBECONFIG"], os.path.realpath(mine))
            self.assertEqual(kube.argv(["get", "pods"])[:5],
                             ["kubectl", "--kubeconfig", os.path.realpath(mine), "--context", "default"])

    def test_commands_outside_the_suite_are_refused(self):
        refused = [
            ["get", "pods", "-A"],
            ["get", "pods", "--all-namespaces"],
            ["get", "pods"],
            ["get", "pods", "-n", "kube-system"],
            ["delete", "pod", "x", "-n", "default"],
            ["delete", "deployment", "bdd-standing-cloacina-server", "-n", disturb.NAMESPACE],
            ["delete", "pod", "-l", "app=x", "-n", disturb.NAMESPACE],
            ["delete", "pod", "--all", "-n", disturb.NAMESPACE],
            ["scale", "deployment/bdd-standing-cloacina-server", "--replicas=0", "-n", disturb.NAMESPACE],
            ["scale", "deployment/bdd-standing-postgresql", "--replicas=5", "-n", disturb.NAMESPACE],
            ["exec", "pod", "-n", disturb.NAMESPACE, "--", "sh"],
            ["get", "pods", "-n", disturb.NAMESPACE, "--context", "docker-desktop"],
            ["get", "pods", "-n", disturb.NAMESPACE, "--kubeconfig=/home/x/.kube/config"],
            ["apply", "-f", "-", "-n", disturb.NAMESPACE],
        ]
        for args in refused:
            self.assertIsNotNone(disturb.check_command(args), args)

    def test_the_commands_of_each_disturbance_are_allowed(self):
        ns = disturb.tenant_namespace("team_a")
        allowed = [
            ["config", "view", "-o", "json"],
            disturb.delete_pod_args("p", ns, "sigterm"),
            disturb.delete_pod_args("p", disturb.NAMESPACE, "force"),
            disturb.rollout_args(disturb.SERVER_DEPLOYMENT, disturb.NAMESPACE),
            disturb.postgres_scale_args(0),
            disturb.postgres_scale_args(1),
            ["logs", "p", "-n", ns],
            ["get", "pods", "-n", ns, "-l", disturb.AGENT_PODS],
        ]
        for args in allowed:
            self.assertIsNone(disturb.check_command(args), args)

    def test_a_refused_command_never_reaches_kubectl(self):
        with tempfile.TemporaryDirectory() as folder:
            setup = Setup(folder)
            with self.assertRaises(disturb.Refused):
                setup.runner.kube.call("delete", "pod", "x", "-n", "default")
            self.assertEqual(setup.kubectl.calls, [])

    def test_a_cluster_that_is_not_the_suites_is_refused_before_any_action(self):
        cases = [
            dict(GOOD_CONFIG, contexts=GOOD_CONFIG["contexts"] + [{"name": "docker-desktop"}]),
            dict(GOOD_CONFIG, clusters=[{"name": "default",
                                         "cluster": {"server": "https://kubernetes.docker.internal:6443"}}]),
            dict(GOOD_CONFIG, **{"current-context": "docker-desktop"}),
        ]
        for config in cases:
            with tempfile.TemporaryDirectory() as folder:
                setup = Setup(folder, kubectl=FakeKubectl(config=config))
                with self.assertRaises(disturb.Refused):
                    setup.runner.run_now("restart_server")
                self.assertEqual(setup.kubectl.commands("delete"), [])
                self.assertFalse(os.path.exists(setup.record_path))
                self.assertEqual(setup.lines(setup.log_path)[-1]["event"], "refused")

    def test_a_cluster_with_no_standing_deployment_is_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            setup = Setup(folder, kubectl=FakeKubectl(fail=("get deployment",)))
            with self.assertRaises(disturb.Refused):
                setup.runner.run_now("postgres_away")
            self.assertEqual(setup.kubectl.commands("scale"), [])

    def test_the_api_must_be_on_loopback(self):
        disturb.Api("http://127.0.0.1:18100")
        with self.assertRaises(disturb.Refused):
            disturb.Api("http://192.0.2.10:8080")

    def test_nothing_in_the_runner_calls_docker(self):
        with open(os.path.join(BDD, "suite", "disturb.py"), "r", encoding="utf-8") as handle:
            text = handle.read()
        self.assertNotIn('"docker"', text)
        self.assertNotIn("'docker'", text)

    def test_the_names_are_those_of_the_standing_driver(self):
        with open(os.path.join(BDD, "standing.py"), "r", encoding="utf-8") as handle:
            text = handle.read()
        self.assertIn(f'PROJECT = "{disturb.PROJECT}"', text)
        self.assertIn(f'RELEASE = "{disturb.RELEASE}"', text)
        self.assertIn(f'NS = "{disturb.NAMESPACE}"', text)
        self.assertIn('"kubeconfig.host.yaml"', text)


class DisturbanceTest(unittest.TestCase):
    def test_a_server_replica_restart_is_on_record_before_and_after(self):
        with tempfile.TemporaryDirectory() as folder:
            setup = Setup(folder)
            seen = []
            setup.kubectl.on_call = lambda args: seen.append(
                (args[0], len(setup.lines()) if os.path.exists(setup.record_path) else 0))
            closed = setup.runner.run_now("restart_server", mode="force")
            # The record had its open line before the delete.
            self.assertIn(("delete", 1), seen)
            lines = setup.lines()
            self.assertEqual(len(lines), 2)
            self.assertNotIn("end", lines[0])
            self.assertEqual(lines[1]["id"], lines[0]["id"])
            self.assertEqual(closed["kind"], "restart_server")
            self.assertEqual(closed["replicas"], 2)
            delete = setup.kubectl.commands("delete")[0]
            self.assertIn("--grace-period=0", delete)
            self.assertEqual(delete[2], "bdd-standing-cloacina-server-a")
            # The checker reads one window, closed, with its settle time.
            windows = engine.read_windows(json.dumps(line) for line in lines)
            self.assertEqual(len(windows), 1)
            self.assertEqual(windows[0].closes - windows[0].end,
                             disturb.SETTLE_SECONDS["restart_server"])
            log = [line["event"] for line in setup.lines(setup.log_path)]
            self.assertEqual(log[0], "checked")
            self.assertIn("command", log)

    def test_an_agent_pod_that_holds_a_task_is_found_by_its_name(self):
        # The agent id is the pod's name (CLOACINA-T-1145).
        ns = disturb.tenant_namespace("team_b")
        kubectl = FakeKubectl(agent_pods={ns: ["cloacina-agent-7f9c-idle", "cloacina-agent-7f9c-busy"]})
        api = FakeApi(roster=[
            {"agent_id": "cloacina-agent-7f9c-idle", "tenant_id": "team_b", "in_flight": 0,
             "seconds_since_heartbeat": 3},
            {"agent_id": "cloacina-agent-7f9c-busy", "tenant_id": "team_b", "in_flight": 1,
             "seconds_since_heartbeat": 2},
            {"agent_id": "other", "tenant_id": "team_a", "in_flight": 4, "seconds_since_heartbeat": 2},
        ])
        with tempfile.TemporaryDirectory() as folder:
            setup = Setup(folder, kubectl=kubectl, api=api)
            closed = setup.runner.run_now("kill_agent", tenant="team_b", mode="sigterm")
            self.assertEqual(closed["pod"], "cloacina-agent-7f9c-busy")
            self.assertEqual(closed["agent_id"], "cloacina-agent-7f9c-busy")
            self.assertEqual(closed["mode"], "sigterm")
            delete = setup.kubectl.commands("delete")[0]
            self.assertEqual(delete[:5], ["delete", "pod", "cloacina-agent-7f9c-busy", "-n", ns])
            self.assertNotIn("--force", delete)
            # No pod's log is read.
            self.assertEqual(setup.kubectl.commands("logs"), [])
            # The roster is read with the tenant's read key.
            self.assertIn(("GET", "/v1/agents", "read-team_b", None), api.calls)

    def test_a_busy_agent_with_no_pod_of_its_name_is_no_disturbance(self):
        ns = disturb.tenant_namespace("team_b")
        kubectl = FakeKubectl(agent_pods={ns: ["cloacina-agent-7f9c-other"]})
        api = FakeApi(roster=[{"agent_id": "agent-busy", "tenant_id": "team_b", "in_flight": 1,
                               "seconds_since_heartbeat": 2}])
        with tempfile.TemporaryDirectory() as folder:
            setup = Setup(folder, kubectl=kubectl, api=api)
            self.assertIsNone(setup.runner.run_now("kill_agent", tenant="team_b"))
            self.assertEqual(setup.kubectl.commands("delete"), [])
            skipped = setup.lines(setup.log_path)[-1]
            self.assertEqual(skipped["event"], "skipped")
            self.assertIn("has the name of one of them", skipped["reason"])

    def test_no_agent_with_work_is_no_disturbance(self):
        ns = disturb.tenant_namespace("team_a")
        kubectl = FakeKubectl(agent_pods={ns: ["cloacina-agent-1"]})
        api = FakeApi(roster=[{"agent_id": "a", "tenant_id": "team_a", "in_flight": 0,
                               "seconds_since_heartbeat": 1}])
        with tempfile.TemporaryDirectory() as folder:
            setup = Setup(folder, kubectl=kubectl, api=api)
            self.assertIsNone(setup.runner.run_now("kill_agent", tenant="team_a"))
            self.assertEqual(setup.kubectl.commands("delete"), [])
            self.assertFalse(os.path.exists(setup.record_path))
            skipped = setup.lines(setup.log_path)[-1]
            self.assertEqual(skipped["event"], "skipped")
            self.assertIn("no agent of team_a holds a task", skipped["reason"])

    def test_postgres_goes_away_and_comes_back(self):
        with tempfile.TemporaryDirectory() as folder:
            setup = Setup(folder)
            started = setup.clock.now
            closed = setup.runner.run_now("postgres_away")
            scales = setup.kubectl.commands("scale")
            self.assertEqual([args[2] for args in scales], ["--replicas=0", "--replicas=1"])
            self.assertTrue(closed["recovered"])
            self.assertIn("server is ready", closed["outcome"])
            self.assertGreaterEqual(setup.clock.now - started, disturb.POSTGRES_AWAY_SECONDS)
            self.assertIsNone(setup.runner.halted)

    def test_a_stop_in_the_middle_brings_postgres_back(self):
        with tempfile.TemporaryDirectory() as folder:
            setup = Setup(folder)

            def stop_after_scale(args):
                if args[0] == "scale" and args[2] == "--replicas=0":
                    disturb.request_stop(setup.stop_path)

            setup.kubectl.on_call = stop_after_scale
            started = setup.clock.now
            closed = setup.runner.run_now("postgres_away")
            self.assertLess(setup.clock.now - started, disturb.POSTGRES_AWAY_SECONDS)
            self.assertEqual([a[2] for a in setup.kubectl.commands("scale")], ["--replicas=0", "--replicas=1"])
            self.assertIn("cut short by a stop", closed["outcome"])
            self.assertEqual(setup.runner.record.open_entries(), [])

    def test_ctrl_c_in_the_middle_brings_postgres_back(self):
        with tempfile.TemporaryDirectory() as folder:
            setup = Setup(folder)

            def interrupt(seconds):
                raise KeyboardInterrupt

            setup.runner.sleep = interrupt
            with self.assertRaises(KeyboardInterrupt):
                setup.runner.run_now("postgres_away")
            self.assertEqual([a[2] for a in setup.kubectl.commands("scale")][:2],
                             ["--replicas=0", "--replicas=1"])
            self.assertEqual(setup.runner.record.open_entries(), [])

    def test_postgres_that_does_not_come_back_stops_the_schedule(self):
        with tempfile.TemporaryDirectory() as folder:
            setup = Setup(folder, api=FakeApi(ready=False))
            closed = setup.runner.run_now("postgres_away")
            self.assertFalse(closed["recovered"])
            self.assertIn("not ready", closed["outcome"])
            self.assertIsNotNone(setup.runner.halted)

    def test_filling_a_tenant_starts_more_held_runs_than_its_agents_take(self):
        with tempfile.TemporaryDirectory() as folder:
            setup = Setup(folder)
            closed = setup.runner.run_now("fill_capacity", tenant="team_c")
            posts = [call for call in setup.api.calls if call[0] == "POST"]
            self.assertEqual(len(posts), disturb.FILL_RUNS)
            self.assertGreater(disturb.FILL_RUNS, workload.AGENTS_PER_TENANT * workload.AGENT_SLOTS)
            self.assertTrue(all(call[1] == "/v1/tenants/team_c/workflows/standing_hold/execute"
                                and call[2] == "write-team_c" for call in posts))
            self.assertEqual(closed["outcome"], f"{disturb.FILL_RUNS} of {disturb.FILL_RUNS} held runs accepted")
            self.assertEqual(setup.kubectl.commands("delete") + setup.kubectl.commands("scale"), [])

    def test_a_window_left_open_is_closed_and_restored_at_start(self):
        with tempfile.TemporaryDirectory() as folder:
            setup = Setup(folder)
            setup.runner.record.open("postgres_away", "postgres")
            closed = setup.runner.recover_open()
            self.assertEqual(len(closed), 1)
            self.assertIn("left open", closed[0]["outcome"])
            self.assertEqual([a[2] for a in setup.kubectl.commands("scale")], ["--replicas=1"])
            self.assertEqual(setup.runner.record.open_entries(), [])

    def test_one_disturbance_at_a_time(self):
        with tempfile.TemporaryDirectory() as folder:
            setup = Setup(folder)
            other = disturb.Lock(os.path.join(folder, "disturb.lock"))
            # A live process holds the lock: this one.
            self.assertTrue(other.take("kill_agent"))
            try:
                self.assertIsNone(setup.runner.run_now("restart_server"))
                self.assertEqual(setup.kubectl.commands("delete"), [])
            finally:
                other.release()

    def test_a_scenario_names_the_pod_to_kill(self):
        ns = disturb.tenant_namespace("team_a")
        kubectl = FakeKubectl(agent_pods={ns: ["cloacina-agent-1", "cloacina-agent-2"]})
        with tempfile.TemporaryDirectory() as folder:
            setup = Setup(folder, kubectl=kubectl)
            closed = setup.runner.run_now("kill_agent", tenant="team_a", mode="force", pod="cloacina-agent-2")
            self.assertEqual(closed["pod"], "cloacina-agent-2")
            self.assertEqual(closed["chosen"], "named by the scenario")
            self.assertEqual(setup.kubectl.commands("delete")[0][2], "cloacina-agent-2")
            # The roster is not read: the scenario knows the pod.
            self.assertEqual(setup.api.calls, [])

    def test_a_named_pod_that_is_not_running_is_no_disturbance(self):
        ns = disturb.tenant_namespace("team_a")
        kubectl = FakeKubectl(agent_pods={ns: ["cloacina-agent-1"]})
        with tempfile.TemporaryDirectory() as folder:
            setup = Setup(folder, kubectl=kubectl)
            self.assertIsNone(setup.runner.run_now("kill_agent", tenant="team_a", pod="cloacina-agent-9"))
            self.assertIsNone(setup.runner.run_now("restart_server", pod="bdd-standing-cloacina-server-z"))
            self.assertEqual(setup.kubectl.commands("delete"), [])

    def test_each_server_pod_in_turn(self):
        with tempfile.TemporaryDirectory() as folder:
            setup = Setup(folder)
            for pod in setup.runner.server_pods():
                setup.runner.run_now("restart_server", mode="force", pod=pod)
            self.assertEqual([args[2] for args in setup.kubectl.commands("delete")],
                             ["bdd-standing-cloacina-server-a", "bdd-standing-cloacina-server-b"])

    def test_postgres_and_capacity_take_no_pod(self):
        with tempfile.TemporaryDirectory() as folder:
            setup = Setup(folder)
            with self.assertRaises(disturb.Refused):
                setup.runner.run_now("postgres_away", pod="x")

    def test_an_unknown_kind_or_tenant_is_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            setup = Setup(folder)
            with self.assertRaises(disturb.Refused):
                setup.runner.run_now("delete_everything")
            with self.assertRaises(disturb.Refused):
                setup.runner.run_now("fill_capacity", tenant="kairos")
            self.assertEqual(setup.kubectl.calls, [])


class RunTest(unittest.TestCase):
    def test_the_schedule_runs_each_kind_in_order_and_stops(self):
        with tempfile.TemporaryDirectory() as folder:
            setup = Setup(folder, api=FakeApi(roster=[]))
            start = disturb.CYCLE_SECONDS * 100
            setup.clock.now = start
            code = setup.runner.run(until=start + disturb.CYCLE_SECONDS - 1)
            self.assertEqual(code, 0)
            kinds = [line["kind"] for line in setup.lines() if "end" in line]
            # kill_agent finds no agent with work: skipped, not on record.
            self.assertEqual(kinds, ["restart_server", "postgres_away", "fill_capacity"])
            events = [line["event"] for line in setup.lines(setup.log_path)]
            self.assertEqual(events[0], "schedule started")
            self.assertEqual(events[-1], "schedule stopped")

    def test_a_stop_file_stops_the_schedule_before_the_next_disturbance(self):
        with tempfile.TemporaryDirectory() as folder:
            setup = Setup(folder)
            setup.clock.now = disturb.CYCLE_SECONDS * 100 + 10

            def sleep_and_stop(seconds):
                setup.clock.sleep(seconds)
                disturb.request_stop(setup.stop_path)

            setup.runner.sleep = sleep_and_stop
            self.assertEqual(setup.runner.run(), 0)
            self.assertFalse(os.path.exists(setup.record_path))
            self.assertFalse(os.path.exists(setup.stop_path))

    def test_a_refusal_halts_the_schedule(self):
        with tempfile.TemporaryDirectory() as folder:
            config = dict(GOOD_CONFIG, clusters=[{"name": "x", "cluster": {"server": "https://192.0.2.1:6443"}}])
            setup = Setup(folder, kubectl=FakeKubectl(config=config))
            setup.clock.now = disturb.CYCLE_SECONDS * 100
            self.assertEqual(setup.runner.run(), 1)
            self.assertIn("not the suite's k3s", setup.runner.halted)
            self.assertEqual(setup.kubectl.commands("delete") + setup.kubectl.commands("scale"), [])


class CheckerReadsTheRecordTest(unittest.TestCase):
    def test_the_last_line_of_an_id_wins(self):
        opened = {"id": "d-1", "kind": "kill_agent", "target": "team_a pod p",
                  "start": "2026-11-27T12:02:00Z"}
        closed = dict(opened, end="2026-11-27T12:03:00Z", settle_until="2026-11-27T12:18:00Z",
                      mode="force", outcome="pod deleted")
        snapshot = engine.Snapshot({"taken_at": "2026-11-27T12:30:00Z", "disturbances": [opened, closed]})
        windows = snapshot.windows()
        self.assertEqual(len(windows), 1)
        self.assertFalse(windows[0].active_at(snapshot.now))
        self.assertEqual(windows[0].extra["mode"], "force")
        # Only the open line: the window is open.
        self.assertTrue(engine.Snapshot({"taken_at": "2026-11-27T12:30:00Z",
                                         "disturbances": [opened]}).windows()[0].active_at(
            engine.parse_time("2026-11-27T12:30:00Z")))

    def test_the_report_shows_each_disturbance_next_to_the_promises_it_tests(self):
        data = sources.read_snapshot(HEALTHY)
        data["disturbances"] = [
            {"id": "d-2", "kind": "postgres_away", "target": "postgres",
             "start": "2026-11-27T12:02:00Z"},
            {"id": "d-2", "kind": "postgres_away", "target": "postgres",
             "start": "2026-11-27T12:02:00Z", "end": "2026-11-27T12:07:00Z",
             "settle_until": "2026-11-27T12:27:00Z", "outcome": "away 240 s; Postgres back"},
        ]
        registry = rules.load()
        ledger = engine.Ledger()
        snapshot = engine.Snapshot(data)
        made = report.build(registry, ledger, engine.judge_all(registry, ledger, snapshot), snapshot)
        self.assertEqual(len(made["disturbances"]), 1)
        shown = made["disturbances"][0]
        self.assertEqual([t["id"] for t in shown["tests"]], list(promises.DISTURBANCE_TESTS["postgres_away"]))
        self.assertEqual(shown["outcome"], "away 240 s; Postgres back")
        page = report.render(made)
        self.assertIn("promises it tests", page)
        self.assertIn(promises.sentence_of("failure.postgres_away"), page)


if __name__ == "__main__":
    unittest.main()
