"""`angreal test bdd publish` — the behaviour-spec driver (CLOACINA-T-1026).

Runs `features/publishing/independent_publish.feature` against a full
Kubernetes deployment: server (fleet executor), a compiler per tenant, and one
agent per tenant, for two tenants `team_a` and `team_b`. The bring-up reuses
the k3s helpers of `.angreal/test/e2e/k8s_fleet.py` (substrate, images, helm,
port-forward); it does not copy them.

Dry run: `angreal test bdd publish --dry-run` parses the feature and checks
that every step has exactly one definition. It needs no cluster and starts no
containers, so it is safe to run anywhere. The real run needs Docker + k3s and
is the maintainer's to run.

Reading the result: at the end the run prints the rough-spots report and
writes it to `bdd-publish-report.txt` in the working directory. Each scenario
is pass / known gap / new failure / unexpected pass, with the observed facts
(what the probe task could read, the HTTP status a foreign key got, which
version ran a task). A @gap scenario that fails does not fail the run; a @gap
scenario that passes is called out loudly.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
import uuid
from pathlib import Path

import angreal  # type: ignore

from .._utils import print_section_header
from . import gherkin
from .world import PublishWorld, TENANTS

# The e2e k3s harness. We reuse its bring-up verbatim.
from ..e2e import k8s_fleet as k8s

test = angreal.command_group(
    name="test", about="Cloacina test suites (unit, integration, e2e, soak)"
)
bdd = angreal.command_group(
    name="bdd", about="behaviour specs driven against a live deployment"
)

PROJECT_ROOT = Path(angreal.get_root()).parent
FEATURE = PROJECT_ROOT / "features" / "publishing" / "independent_publish.feature"
REPORT_FILE = PROJECT_ROOT / "bdd-publish-report.txt"

# This lane's own identity, so it never clashes with the e2e / soak lanes.
RELEASE = "bdd-publish"
NS = "cloacina-bdd-publish"
BOOTSTRAP_KEY = "bdd-publish-bootstrap-key"
BOOTSTRAP_SECRET = "cloacina-bootstrap"
# A 32-byte KEK, base64 — so the secrets routes answer instead of 503. This is
# a throwaway test key, never a real one.
SECRET_KEK = "YmRkLXB1Ymxpc2gta2VrLWJkZC1wdWJsaXNoLWsxMjM="
FWD_PORT = 18099
REUSE_PROJECT = "cloacina-k8s-bdd-publish-keep"


def _load_feature():
    text = FEATURE.read_text()
    return gherkin.parse(text, path=str(FEATURE.relative_to(PROJECT_ROOT)))


def _build_registry():
    """Import the step module and return its populated registry."""
    from . import publish_steps

    return publish_steps.steps


def _dry_run(feature, registry):
    print_section_header("bdd publish — dry run (no cluster)")
    print(f"feature: {feature.path}")
    print(f"scenarios: {len(feature.scenarios)}")
    problems, notes = gherkin.check(feature, registry)
    for note in notes:
        print(f"  note: {note}")
    if problems:
        print("\nDRY RUN FAILED:")
        for problem in problems:
            print(f"  {problem}")
        return 1
    total_steps = len(feature.background) + sum(len(s.steps) for s in feature.scenarios)
    print(f"\nok: every step has exactly one definition ({total_steps} steps, "
          f"{len(registry.definitions)} definitions).")
    for number, scenario in enumerate(feature.scenarios, start=1):
        mark = f"  [gap {', '.join(scenario.gap_tasks)}]" if scenario.is_gap else ""
        print(f"  {number}. {scenario.name}{mark}")
    return 0


def _server_values_with_fleet(tag, agent_ref, chart_actuator, compiler_ref=None):
    """The e2e server values, plus fleet executor + a secret KEK.

    With `chart_actuator` false the server runs with `fleet.actuator=none` and
    the driver applies the agents (`PublishWorld.deploy_agent`).

    The e2e harness runs the server with the default executor (no task reaches
    an agent) and no KEK (the secrets routes answer 503). The publishing spec
    needs both, so we append them to the extraEnv the e2e values already open.
    """
    base = k8s._server_values(tag, agent_ref, BOOTSTRAP_SECRET)
    # Let tenant creation bring up one agent, so a publish has somewhere to run
    # without a separate provision call. The e2e values set 0.
    zero = '{name: CLOACINA_INITIAL_AGENTS, value: "0"}'
    assert zero in base, "the e2e server values changed: CLOACINA_INITIAL_AGENTS not found"
    base = base.replace(zero, '{name: CLOACINA_INITIAL_AGENTS, value: "1"}')
    assert "  actuator: kubernetes\n" in base, "the e2e server values changed: actuator not found"
    if not chart_actuator:
        base = base.replace("  actuator: kubernetes\n", "  actuator: none\n")
    else:
        # The server's actuator makes each tenant's compiler (CLOACINA-T-1034),
        # with what the driver's own manifest gave it: the test image as it is
        # built (it carries the workspace and a warm target directory, and
        # runs as root, so not hardened), a release build, the workspace for
        # the unpublished crates. The long cooldown keeps the compiler pod,
        # and its log, for the whole run.
        base = base.replace(
            "  actuator: kubernetes\n",
            "  actuator: kubernetes\n"
            "  compiler:\n"
            "    enabled: true\n"
            "    image:\n"
            f"      repository: {compiler_ref.rsplit(':', 1)[0]}\n"
            f"      tag: {compiler_ref.rsplit(':', 1)[1]}\n"
            "    cargoFlags: [build, --release, --lib]\n"
            "    devWorkspace: /workspace\n"
            "    targetDir: /workspace/target\n"
            "    hardened: false\n"
            "    egress: all\n"
            "    cooldownSeconds: 86400\n",
        )
    extra = (
        '    - {name: CLOACINA_DEFAULT_EXECUTOR, value: "fleet"}\n'
        f'    - {{name: CLOACINA_SECRET_KEK, value: "{SECRET_KEK}"}}\n'
    )
    return base + extra


@test()
@bdd()
@angreal.command(
    name="publish",
    about="behaviour spec: a team publishes its own jobs, safely (CLOACINA-T-1026)",
    when_to_use=[
        "running the publishing behaviour spec against a k3s deployment",
        "checking which of the eight publishing scenarios hold and which are gaps",
        "parsing the feature and checking step coverage with --dry-run (no cluster)",
    ],
    when_not_to_use=["unit testing", "a dry run needs no docker; a full run needs docker + k3s"],
)
@angreal.argument(name="dry_run", long="dry-run", takes_value=False, is_flag=True,
                  help="Parse the feature and check step coverage; start no containers")
@angreal.argument(name="no_cleanup", long="no-cleanup", takes_value=False, is_flag=True,
                  help="Leave the k3s cluster + helm release up for inspection")
@angreal.argument(name="skip_build", long="skip-build", takes_value=False, is_flag=True,
                  help="Reuse existing server/agent/compiler images (no rebuild)")
@angreal.argument(name="only", long="only", default_value="",
                  help="Run only these scenario numbers, comma-separated (e.g. 2,6)")
@angreal.argument(name="tag", long="tag", default_value="bdd-publish",
                  help="Image tag pushed to the local registry")
@angreal.argument(name="reuse_cluster", long="reuse-cluster", takes_value=False, is_flag=True,
                  help="Use one named cluster and keep it: the first run makes it, later runs reuse it")
@angreal.argument(name="chart_actuator", long="chart-actuator", takes_value=False, is_flag=True,
                  help="Let the server's Kubernetes actuator make the agents (fleet.actuator=kubernetes)")
def publish(dry_run=False, no_cleanup=False, skip_build=False, only="", tag="bdd-publish",
            reuse_cluster=False, chart_actuator=False):
    feature = _load_feature()
    registry = _build_registry()

    if dry_run:
        code = _dry_run(feature, registry)
        sys.stdout.flush()
        sys.exit(code)

    # A full run still checks coverage first: a missing step definition should
    # fail before any container starts, not halfway through a scenario.
    problems, _ = gherkin.check(feature, registry)
    if problems:
        print("step coverage check failed — run with --dry-run for the list", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        sys.exit(1)

    only_set = {int(n) for n in only.split(",") if n.strip()} if only else None

    k8s._check_tool("docker", "install Docker Desktop or colima")
    k8s._check_tool("kubectl", "sudo port install kubectl")
    k8s._check_tool("helm", k8s.HELM_HINT)

    if reuse_cluster:
        # One named project and one known folder, so the next run finds them.
        no_cleanup = True
        project = REUSE_PROJECT
        hostdir = Path(tempfile.gettempdir()) / REUSE_PROJECT
        hostdir.mkdir(exist_ok=True)
    else:
        project = f"cloacina-k8s-bdd-publish-{uuid.uuid4().hex[:8]}"
        hostdir = Path(tempfile.mkdtemp(prefix="cloacina-k8s-bdd-publish-"))
    kubeconfig = hostdir / "kubeconfig.host.yaml"
    print_section_header(f"cloacina publishing behaviour spec (project: {project})")
    print(f"host kubeconfig dir: {hostdir}")

    compose_env = os.environ.copy()
    compose_env["CLOACINA_K8S_E2E_HOSTDIR"] = str(hostdir)

    world = PublishWorld(
        project_root=PROJECT_ROOT,
        kubeconfig=kubeconfig,
        release=RELEASE,
        namespace=NS,
        bootstrap_key=BOOTSTRAP_KEY,
        secret_kek=SECRET_KEK,
        tag=tag,
    )
    fwd = None
    try:
        print("\n--- bring up k3s + registry ---\n")
        k8s.bring_up_cluster(project, hostdir, compose_env, kubeconfig, reuse=reuse_cluster)

        print("\n--- build/push server + agent images ---\n")
        _server_ref, agent_ref = k8s._prepare_images(tag, skip_build)
        print("\n--- build/push the compiler image ---\n")
        compiler_ref = world.prepare_compiler_image(skip_build)

        print("\n--- helm install server (fleet executor, secret KEK) ---\n")
        # Reuse the e2e helm bring-up, but with our fleet+KEK values and names.
        k8s.helm_deploy_server(
            kubeconfig, hostdir, tag, agent_ref,
            release=RELEASE, ns=NS,
            bootstrap_secret=BOOTSTRAP_SECRET, bootstrap_key=BOOTSTRAP_KEY,
            values_text=_server_values_with_fleet(tag, agent_ref, chart_actuator, compiler_ref),
            reuse=reuse_cluster,
        )

        print("\n--- port-forward the server API ---\n")
        fwd, base = k8s.start_port_forward(kubeconfig, release=RELEASE, ns=NS, port=FWD_PORT)
        world.base_url = base
        if not k8s._wait_http(f"{base}/ready", timeout_s=90, proc=fwd):
            print(k8s._server_logs(kubeconfig))
            raise AssertionError("server /ready never became healthy via port-forward")

        print("\n--- create the two tenants, their agents, keys and compilers ---\n")
        # With the chart's actuator the driver applies no compiler manifest.
        world.chart_compilers = chart_actuator
        world.provision_tenants(compiler_ref, agent_ref=None if chart_actuator else agent_ref)

        print("\n--- build cloacinactl (host) ---\n")
        world.build_cloacinactl()

        results = gherkin.run(feature, registry, world, only=only_set)

        text = gherkin.report(feature, results)
        print("\n" + "=" * 64)
        print(text)
        REPORT_FILE.write_text(text)
        print(f"(written to {REPORT_FILE})")

        if gherkin.run_failed(results):
            sys.exit(1)
    finally:
        # angreal's Python does not flush on sys.exit: a run that fails would
        # lose its scenario lines.
        sys.stdout.flush()
        sys.stderr.flush()
        if fwd is not None and fwd.poll() is None:
            fwd.terminate()
        if no_cleanup:
            print(f"\n--no-cleanup: cluster left up.\n"
                  f"  KUBECONFIG={kubeconfig}\n"
                  f"  teardown: CLOACINA_K8S_E2E_HOSTDIR={hostdir} "
                  f"docker compose -f {k8s.COMPOSE_FILE} -p {project} down -v")
        else:
            print(f"\n--- cleanup: docker compose -p {project} down -v ---\n")
            k8s._compose(["down", "-v"], project, env=compose_env)
            shutil.rmtree(hostdir, ignore_errors=True)
