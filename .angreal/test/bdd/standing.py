"""`angreal test bdd standing` — the standing workload of the behaviour suite
(CLOACINA-T-1107) and its checker (CLOACINA-T-1108).

What it does, by flag:

- `--dry-run`: find every chapter, parse every feature, resolve every step
  to exactly one definition, load the checker's rules and check the workload
  description. No cluster, no containers. Safe anywhere.
- `--self-test`: run the unit tests of the suite's shared code and checker.
  No cluster, no server.
- no flag: bring the deployment up (or attach to it), deploy the tenants and
  the workload, and run the workload's feature. The deployment is left
  standing: this is not a test that tears down.
- `--chapter NAME`: run that chapter's features against the standing
  deployment (`all` for every chapter).
- `--check`: forward the server and the database to localhost and run the
  checker until stopped. The report is served on localhost.
- `--disturb`: run the disturbance schedule (CLOACINA-T-1109) on the
  standing cluster until stopped; with `--dry-run`, print the schedule and
  its commands and touch nothing. `--disturb-stop` stops it.
- `--down`: remove the cluster and everything in it.

The real run needs Docker, `kubectl` and `helm`, and builds three images. It
is the maintainer's to start. See `features/README.md`.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

import angreal  # type: ignore

from .._utils import print_section_header

# The e2e k3s harness. Its bring-up is reused, not copied.
from ..e2e import k8s_fleet as k8s
from . import chapters, gherkin
from . import publish as publish_chapter
from .standing_world import StandingWorld
from .suite import disturb, workload

test = angreal.command_group(
    name="test", about="Cloacina test suites (unit, integration, e2e, soak)"
)
bdd = angreal.command_group(
    name="bdd", about="behaviour specs driven against a live deployment"
)

PROJECT_ROOT = Path(angreal.get_root()).parent

# This deployment's own names, so it never meets the other lanes.
PROJECT = "cloacina-k8s-bdd-standing"
RELEASE = "bdd-standing"
NS = "cloacina-bdd-standing"
BOOTSTRAP_KEY = "bdd-standing-bootstrap-key"
BOOTSTRAP_SECRET = "cloacina-bootstrap"
SERVER_PORT = 18100
DATABASE_PORT = 15432
REPORT_DIR = PROJECT_ROOT / "bdd-standing-report"
# Days of finished runs the standing deployment keeps.
RUN_HISTORY_DAYS = 2
# Server replicas of the standing deployment (Dylan, 2026-11-27, on
# COLLIERY-I-0685): a restarted replica is the common case with two.
SERVER_REPLICAS = 2


def _hostdir():
    return Path(tempfile.gettempdir()) / PROJECT


def _settings_path():
    return _hostdir() / "checker-settings.json"


def _disturb_settings_path():
    return _hostdir() / "disturb-settings.json"


def _dry_run():
    print_section_header("bdd standing — dry run (no cluster)")
    lines, problems = chapters.dry_run(PROJECT_ROOT)
    for line in lines:
        print(line)
    if problems:
        print("\nDRY RUN FAILED:")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print("\nok: every feature parses, every step has exactly one definition, "
          "the rules load, the workload description is sound.")
    return 0


def _self_test():
    print_section_header("bdd standing — unit tests of the suite (no server)")
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from suite.tests import run

    return run.main()


def _server_values(tag, agent_ref, chart_actuator, compiler_ref, replicas=SERVER_REPLICAS):
    """The values of the publishing chapter's deployment, changed for a
    deployment that stands: the database keeps its data across a restart of
    its pod, finished runs are removed after a few days, and the server runs
    `replicas` replicas (two by default, so a result can reach a replica that
    did not dispatch its task: the chapter run_once, CLOACINA-T-1110)."""
    base = publish_chapter._server_values_with_fleet(tag, agent_ref, chart_actuator, compiler_ref)
    kept = "  persistence:\n    enabled: false\n"
    assert kept in base, "the e2e server values changed: postgres persistence not found"
    assert "replicaCount" not in base, "the e2e server values set replicaCount themselves"
    base = base.replace(kept, "  persistence:\n    enabled: true\n")
    return base + (
        f"replicaCount: {int(replicas)}\n"
        "retention:\n"
        f"  runHistoryDays: {RUN_HISTORY_DAYS}\n"
        "  outboxAckedSeconds: 3600\n"
    )


def _chosen_chapters(name):
    if name == "all":
        return chapters.discover()
    return [chapters.find(name)]


def _check_features(chosen):
    """Step coverage of the chosen chapters, before any container starts."""
    problems = []
    for chapter in chosen:
        for feature in chapters.load_features(chapter, PROJECT_ROOT):
            found, _ = gherkin.check(feature, chapter.steps)
            problems.extend(found)
    return problems


class _Forward:
    """A `kubectl port-forward` that is started again when it ends: the pod
    behind a forward is replaced when a disturbance restarts it."""

    def __init__(self, kubeconfig, target, ports, namespace):
        self.args = ["kubectl", "port-forward", target, ports, "-n", namespace]
        self.env = k8s._kube_env(kubeconfig)
        self.stop = threading.Event()
        self.proc = None
        self.thread = threading.Thread(target=self._run, daemon=True)
        self.thread.start()

    def _run(self):
        while not self.stop.is_set():
            self.proc = subprocess.Popen(self.args, env=self.env, stdout=subprocess.DEVNULL,
                                         stderr=subprocess.DEVNULL)
            self.proc.wait()
            self.stop.wait(2)

    def close(self):
        self.stop.set()
        if self.proc is not None and self.proc.poll() is None:
            self.proc.terminate()


def _down():
    hostdir = _hostdir()
    compose_env = os.environ.copy()
    compose_env["CLOACINA_K8S_E2E_HOSTDIR"] = str(hostdir)
    print_section_header(f"bdd standing — remove the cluster (project: {PROJECT})")
    k8s._check_tool("docker", "install Docker Desktop or colima")
    k8s._compose(["down", "-v"], PROJECT, env=compose_env)
    shutil.rmtree(hostdir, ignore_errors=True)
    print(f"removed. The checker's report and state are kept in {REPORT_DIR}.")
    return 0


def _check(interval, port):
    """Forward the server and the database, and run the checker until stopped."""
    kubeconfig = _hostdir() / "kubeconfig.host.yaml"
    settings = _settings_path()
    if not kubeconfig.exists() or not settings.exists():
        print(f"no standing deployment found ({kubeconfig} or {settings} is missing).\n"
              "Bring it up first: angreal test bdd standing", file=sys.stderr)
        return 1
    k8s._check_tool("kubectl", "sudo port install kubectl")
    print_section_header("bdd standing — the checker (stop with Ctrl-C)")
    forwards = [
        _Forward(kubeconfig, f"svc/{RELEASE}-cloacina-server", f"{SERVER_PORT}:8080", NS),
        _Forward(kubeconfig, f"svc/{RELEASE}-postgresql", f"{DATABASE_PORT}:5432", NS),
    ]
    try:
        k8s._wait_http(f"http://127.0.0.1:{SERVER_PORT}/ready", timeout_s=60)
        from .suite import check

        return check.main(["--config", str(settings), "--out", str(REPORT_DIR),
                           "--interval", str(interval), "--port", str(port), "--record",
                           "--kubeconfig", str(kubeconfig), "--namespace", NS])
    finally:
        for forward in forwards:
            forward.close()


def _disturb_dry_run():
    print_section_header("bdd standing — the disturbance schedule, dry run (touches nothing)")
    print(disturb.plan_text(time.time()))
    return 1 if disturb.schedule_problems() else 0


def _disturb_stop():
    path = disturb.request_stop(str(_hostdir() / "disturb.stop"))
    print(f"asked the disturbance schedule to stop ({path}). It ends the disturbance in hand, "
          "brings Postgres back if it is away, closes the record, and exits.")
    return 0


def _disturb():
    """Forward the server API, and run the disturbance schedule until stopped."""
    kubeconfig = _hostdir() / "kubeconfig.host.yaml"
    settings_path = _disturb_settings_path()
    if not kubeconfig.exists() or not settings_path.exists():
        print(f"no standing deployment found ({kubeconfig} or {settings_path} is missing).\n"
              "Bring it up first: angreal test bdd standing", file=sys.stderr)
        return 1
    k8s._check_tool("kubectl", "sudo port install kubectl")
    settings = disturb.read_settings(str(settings_path))
    print_section_header("bdd standing — the disturbance schedule (stop: --disturb-stop or Ctrl-C)")
    forward = _Forward(kubeconfig, f"svc/{RELEASE}-cloacina-server", f"{SERVER_PORT}:8080", NS)
    try:
        k8s._wait_http(f"http://127.0.0.1:{SERVER_PORT}/ready", timeout_s=60)
        runner = disturb.Runner(
            # The Kube refuses any kubeconfig but the suite's own path.
            disturb.Kube(str(kubeconfig)),
            disturb.Api(f"http://127.0.0.1:{SERVER_PORT}"),
            settings["keys"],
            disturb.Record(str(REPORT_DIR / "disturbances.jsonl"),
                           str(REPORT_DIR / "disturbance-log.jsonl")),
            read_keys=settings.get("read_keys"),
            stop_path=str(_hostdir() / "disturb.stop"),
            lock_path=str(_hostdir() / "disturb.lock"),
        )
        print(f"record: {REPORT_DIR / 'disturbances.jsonl'}\nlog:    {REPORT_DIR / 'disturbance-log.jsonl'}")
        return runner.run()
    finally:
        forward.close()


@test()
@bdd()
@angreal.command(
    name="standing",
    about="behaviour suite: the standing workload of several tenants, and its checker "
          "(CLOACINA-T-1107, CLOACINA-T-1108)",
    when_to_use=[
        "checking every chapter's features and steps with --dry-run (no cluster)",
        "running the unit tests of the suite's checker with --self-test (no server)",
        "bringing the standing deployment of the behaviour suite up on k3s",
        "running the checker against the standing deployment with --check",
        "running the disturbance schedule on the standing cluster with --disturb "
        "(--disturb --dry-run prints it and touches nothing)",
    ],
    when_not_to_use=[
        "unit testing the product",
        "a quick check of one change: the full run builds images and leaves a cluster standing",
    ],
)
@angreal.argument(name="dry_run", long="dry-run", takes_value=False, is_flag=True,
                  help="Parse every feature and resolve every step of every chapter; no cluster")
@angreal.argument(name="self_test", long="self-test", takes_value=False, is_flag=True,
                  help="Run the unit tests of the suite's shared code and checker; no server")
@angreal.argument(name="check", long="check", takes_value=False, is_flag=True,
                  help="Run the checker against the standing deployment until stopped")
@angreal.argument(name="disturb_run", long="disturb", takes_value=False, is_flag=True,
                  help="Run the disturbance schedule on the standing cluster until stopped; "
                       "with --dry-run print it and touch nothing")
@angreal.argument(name="disturb_stop", long="disturb-stop", takes_value=False, is_flag=True,
                  help="Stop the disturbance schedule; it ends the disturbance in hand first")
@angreal.argument(name="server_replicas", long="server-replicas", default_value="2",
                  help="Server replicas of the standing deployment (default 2)")
@angreal.argument(name="down", long="down", takes_value=False, is_flag=True,
                  help="Remove the standing cluster and everything in it")
@angreal.argument(name="chapter", long="chapter", default_value="standing",
                  help="The chapter whose features run (default: standing; `all` for every chapter)")
@angreal.argument(name="only", long="only", default_value="",
                  help="Run only these scenario numbers of the chapter, comma-separated")
@angreal.argument(name="skip_build", long="skip-build", takes_value=False, is_flag=True,
                  help="Reuse existing server/agent/compiler images (no rebuild)")
@angreal.argument(name="tag", long="tag", default_value="bdd-standing",
                  help="Image tag pushed to the local registry")
@angreal.argument(name="chart_actuator", long="chart-actuator", takes_value=False, is_flag=True,
                  help="Let the server's Kubernetes actuator make the agents and the compilers")
@angreal.argument(name="interval", long="interval", default_value="60",
                  help="With --check: seconds between passes of the checker")
@angreal.argument(name="port", long="port", default_value="8377",
                  help="With --check: the port on 127.0.0.1 that serves the report")
def standing(dry_run=False, self_test=False, check=False, disturb_run=False, disturb_stop=False,
             server_replicas="2", down=False, chapter="standing", only="", skip_build=False,
             tag="bdd-standing", chart_actuator=False, interval="60", port="8377"):
    if disturb_stop:
        sys.exit(_disturb_stop())
    if disturb_run:
        code = _disturb_dry_run() if dry_run else _disturb()
        sys.stdout.flush()
        sys.exit(code)
    if dry_run:
        code = _dry_run()
        sys.stdout.flush()
        sys.exit(code)
    if self_test:
        code = _self_test()
        sys.stdout.flush()
        sys.exit(code)
    if down:
        sys.exit(_down())
    if check:
        code = _check(int(interval), int(port))
        sys.stdout.flush()
        sys.exit(code)

    chosen = _chosen_chapters(chapter)
    problems = _check_features(chosen)
    if problems:
        print("step coverage check failed — run with --dry-run for the list", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        sys.exit(1)
    only_set = {int(n) for n in only.split(",") if n.strip()} if only else None

    k8s._check_tool("docker", "install Docker Desktop or colima")
    k8s._check_tool("kubectl", "sudo port install kubectl")
    k8s._check_tool("helm", k8s.HELM_HINT)

    hostdir = _hostdir()
    hostdir.mkdir(exist_ok=True)
    kubeconfig = hostdir / "kubeconfig.host.yaml"
    print_section_header(f"cloacina behaviour suite — standing workload (project: {PROJECT})")
    print(f"host kubeconfig dir: {hostdir}")

    compose_env = os.environ.copy()
    compose_env["CLOACINA_K8S_E2E_HOSTDIR"] = str(hostdir)

    world = StandingWorld(
        project_root=PROJECT_ROOT,
        kubeconfig=kubeconfig,
        release=RELEASE,
        namespace=NS,
        bootstrap_key=BOOTSTRAP_KEY,
        secret_kek=publish_chapter.SECRET_KEK,
        tag=tag,
        settings_path=_settings_path(),
        database_port=DATABASE_PORT,
    )
    fwd = None
    failed = False
    try:
        print("\n--- bring up k3s + registry (or attach to them) ---\n")
        k8s.bring_up_cluster(PROJECT, hostdir, compose_env, kubeconfig, reuse=True)

        print("\n--- build/push server + agent images ---\n")
        _server_ref, agent_ref = k8s._prepare_images(tag, skip_build)
        print("\n--- build/push the compiler image ---\n")
        compiler_ref = world.prepare_compiler_image(skip_build)

        print("\n--- helm install server (fleet executor, kept database, retention) ---\n")
        k8s.helm_deploy_server(
            kubeconfig, hostdir, tag, agent_ref,
            release=RELEASE, ns=NS,
            bootstrap_secret=BOOTSTRAP_SECRET, bootstrap_key=BOOTSTRAP_KEY,
            values_text=_server_values(tag, agent_ref, chart_actuator, compiler_ref,
                                       replicas=int(server_replicas)),
            reuse=True,
        )

        print("\n--- port-forward the server API ---\n")
        # A forward that starts again when its pod is replaced: the Failure
        # chapter restarts server pods (CLOACINA-T-1112).
        fwd = _Forward(kubeconfig, f"svc/{RELEASE}-cloacina-server", f"{SERVER_PORT}:8080", NS)
        base = f"http://127.0.0.1:{SERVER_PORT}"
        world.base_url = base
        if not k8s._wait_http(f"{base}/ready", timeout_s=90):
            print(k8s._server_logs(kubeconfig))
            raise AssertionError("server /ready never became healthy via port-forward")

        print(f"\n--- the tenants ({', '.join(workload.TENANTS)}), their agents, keys and compilers ---\n")
        world.chart_compilers = chart_actuator
        world.provision_workload_tenants(compiler_ref, agent_ref=None if chart_actuator else agent_ref)

        print("\n--- build cloacinactl (host) ---\n")
        world.build_cloacinactl()

        for one in chosen:
            for feature in chapters.load_features(one, PROJECT_ROOT):
                results = gherkin.run(feature, one.steps, world, only=only_set)
                text = gherkin.report(feature, results)
                print("\n" + "=" * 64)
                print(text)
                report_file = PROJECT_ROOT / f"bdd-{one.name}-report.txt"
                report_file.write_text(text)
                print(f"(written to {report_file})")
                failed = failed or gherkin.run_failed(results)

        # The checker and the disturbance schedule need their settings
        # whatever chapter ran.
        world.write_checker_settings()
        world.write_disturb_settings(_disturb_settings_path())
    finally:
        # angreal's Python does not flush on sys.exit.
        sys.stdout.flush()
        sys.stderr.flush()
        if fwd is not None:
            fwd.close()
        print(f"\nThe deployment is left standing.\n"
              f"  KUBECONFIG={kubeconfig}\n"
              f"  checker:  angreal test bdd standing --check   (report: http://127.0.0.1:8377/)\n"
              f"  disturb:  angreal test bdd standing --disturb (stop: --disturb-stop)\n"
              f"  remove:   angreal test bdd standing --down")
        sys.stdout.flush()
    if failed:
        sys.exit(1)
