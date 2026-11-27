"""The world the publishing behaviour-spec steps act on (CLOACINA-T-1026).

Holds the live deployment and the primitives the steps call: the HTTP API with
a chosen key, `cloacinactl` on the host, fixture staging, packing and
publishing, and `kubectl`. It carries state between steps of one scenario (the
packages published, the executions started) and clears it between scenarios.

Nothing in this file runs against a cluster until a method is called; the dry
run never constructs a live `PublishWorld`.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

# The two tenants. A schema name allows no hyphen (schema_validation.rs), so
# `team_a`, not `team-a`.
TENANTS = ("team_a", "team_b")
AGENT_SECRET_KEY = "api-key"  # the key inside the actuator-made agent Secret

# The compiler image the cluster pulls. Built from docker/Dockerfile.compiler.
COMPILER_SRC_IMAGES = ("cloacina-compiler:latest", "docker-compiler:latest",
                       "cloacina-demo-fleet-compiler:latest")


class PublishWorld:
    def __init__(self, *, project_root, kubeconfig, release, namespace,
                 bootstrap_key, secret_kek, tag):
        self.root = Path(project_root)
        self.kubeconfig = Path(kubeconfig)
        self.release = release
        self.ns = namespace
        self.bootstrap_key = bootstrap_key
        self.secret_kek = secret_kek
        self.tag = tag
        # The tenants of this deployment. A world with more tenants (the
        # standing workload, CLOACINA-T-1107) sets its own list.
        self.tenants = TENANTS
        self.base_url = None
        self.home = Path(tempfile.mkdtemp(prefix="bdd-publish-home-"))
        # cargo honours CARGO_TARGET_DIR, so the driver does too: a worktree can
        # share one target folder with other checkouts.
        target = Path(os.environ.get("CARGO_TARGET_DIR") or (self.root / "target"))
        self.cloacinactl = target / "debug" / "cloacinactl"
        self.db_url_incluster = (
            f"postgres://cloacina:cloacina@{release}-postgresql:5432/cloacina"
        )
        # True when the server's actuator makes each tenant's compiler
        # (`--chart-actuator`); the driver then applies no compiler manifest.
        self.chart_compilers = False
        # Per-tenant write keys, filled by provision_tenants.
        self.keys = {}
        # Per-tenant admin keys (a team's own admin), filled by provision_tenants.
        self.admin_keys = {}
        # Per-scenario scratch, cleared by begin_scenario.
        self.scratch = {}
        self.result = None

    # -- scenario lifecycle ------------------------------------------------

    def begin_scenario(self, result):
        self.result = result
        self.scratch = {}

    def end_scenario(self, result):
        self.result = None

    def note(self, fact):
        """Record an observed fact on the running scenario's result."""
        if self.result is not None:
            self.result.facts.append(fact)

    # -- HTTP API ----------------------------------------------------------

    def api(self, method, path, *, key, body=None, tenant_header=None, expect=None):
        """Call the server with `key`. Returns (status, parsed-or-text)."""
        url = f"{self.base_url}{path}"
        data = json.dumps(body).encode() if body is not None else None
        req = urllib.request.Request(url, data=data, method=method)
        req.add_header("Authorization", f"Bearer {key}")
        if data is not None:
            req.add_header("Content-Type", "application/json")
        try:
            with urllib.request.urlopen(req, timeout=15) as resp:
                raw, code = resp.read().decode(), resp.status
        except urllib.error.HTTPError as exc:
            raw, code = exc.read().decode(), exc.code
        if expect is not None and code not in expect:
            raise AssertionError(f"{method} {path} -> {code} (expected {expect}); {raw[:300]}")
        try:
            return code, (json.loads(raw) if raw else None)
        except json.JSONDecodeError:
            return code, raw

    # -- cloacinactl on the host ------------------------------------------

    def ctl(self, *args, key=None, tenant=None, check=True):
        """Run `cloacinactl` against the deployment. Returns (code, out, err)."""
        cmd = [str(self.cloacinactl), "--home", str(self.home),
               "--server", self.base_url, "--api-key", key or self.bootstrap_key]
        if tenant:
            cmd += ["--tenant", tenant]
        cmd += list(args)
        proc = subprocess.run(cmd, cwd=str(self.root), capture_output=True, text=True, check=False)
        if check and proc.returncode != 0:
            raise AssertionError(
                f"{' '.join(cmd)} exited {proc.returncode}\nSTDOUT:\n{proc.stdout}\nSTDERR:\n{proc.stderr}"
            )
        return proc.returncode, proc.stdout, proc.stderr

    def build_cloacinactl(self):
        subprocess.run(["cargo", "build", "-p", "cloacinactl"], cwd=str(self.root), check=True)

    # -- fixtures ----------------------------------------------------------

    def stage_fixture(self, name, *, version=None, patch_crates=False):
        """Copy a fixture into a fresh dir under $home, optionally rewriting its
        version, and — for a Rust fixture built on the host — injecting a
        `[patch.crates-io]` so its version deps resolve against this checkout's
        unpublished crates (the same patch the compiler's `--dev-workspace`
        does in the cluster). Returns the staged directory.
        """
        src = self.root / "examples" / "fixtures" / name
        dst = self.home / f"staged-{name}-{version or 'v'}-{os.urandom(3).hex()}"
        subprocess.run(["cp", "-R", str(src), str(dst)], check=True)
        if version is not None:
            for rel in ("package.toml", "Cargo.toml"):
                f = dst / rel
                if f.exists():
                    f.write_text(f.read_text().replace('version = "0.1.0"', f'version = "{version}"'))
            tasks = dst / "workflow" / name.replace("-", "_").replace("_py", "") / "tasks.py"
            # Python version marker lives in tasks.py as PACKAGE_VERSION = "...".
            for py in dst.rglob("tasks.py"):
                txt = py.read_text()
                if "PACKAGE_VERSION" in txt:
                    py.write_text(re.sub(r'PACKAGE_VERSION = "[^"]*"',
                                         f'PACKAGE_VERSION = "{version}"', txt))
        if patch_crates:
            self._inject_dev_patch(dst / "Cargo.toml")
        return dst

    def _inject_dev_patch(self, cargo_toml):
        """Append `[patch.crates-io]` mapping cloacina-* to this checkout's
        crates, so a host `cargo build` of a version-dep fixture resolves the
        unpublished crates. Mirrors the compiler's --dev-workspace."""
        if not cargo_toml.exists():
            return
        crates = self.root / "crates"
        lines = ["", "[patch.crates-io]"]
        for entry in sorted(crates.iterdir()):
            manifest = entry / "Cargo.toml"
            if manifest.exists():
                m = re.search(r'(?m)^\s*name\s*=\s*"([^"]+)"', manifest.read_text())
                if m:
                    lines.append(f'{m.group(1)} = {{ path = "{entry}" }}')
        cargo_toml.write_text(cargo_toml.read_text() + "\n".join(lines) + "\n")

    def pack(self, staged_dir):
        """Pack a staged fixture into a .cloacina archive on the host. Returns the path."""
        out = self.home / f"{staged_dir.name}.cloacina"
        self.ctl("package", "pack", str(staged_dir), "--out", str(out))
        return out

    def publish_archive(self, archive, *, tenant, key):
        """Upload a packed archive to a tenant. Returns (code, out, err)."""
        return self.ctl("package", "upload", str(archive), key=key, tenant=tenant, check=False)

    # -- kubectl / diagnostics --------------------------------------------

    def _kube_env(self):
        env = os.environ.copy()
        env["KUBECONFIG"] = str(self.kubeconfig)
        return env

    def kubectl(self, *args, check=True, capture=True):
        proc = subprocess.run(["kubectl", *args], env=self._kube_env(),
                              capture_output=capture, text=True, check=False)
        if check and proc.returncode != 0:
            raise AssertionError(f"kubectl {' '.join(args)} failed: {proc.stderr}")
        return proc

    def kubectl_json(self, *args):
        proc = self.kubectl(*args, "-o", "json", check=False)
        if proc.returncode != 0:
            return None
        try:
            return json.loads(proc.stdout)
        except json.JSONDecodeError:
            return None

    def tenant_namespace(self, tenant):
        # Python port of actuator::kubernetes::tenant_namespace for a schema
        # name that is already a valid DNS label (lowercase, [a-z0-9_]).
        seg = re.sub(r"[^a-z0-9]+", "-", tenant.lower()).strip("-") or "unnamed"
        return f"cloacina-tenant-{seg}"

    def _postgres_target(self):
        """A `kubectl exec` target for the chart's bundled postgres."""
        probe = self.kubectl("get", f"deploy/{self.release}-postgresql", "-n", self.ns, check=False)
        if probe.returncode == 0:
            return f"deploy/{self.release}-postgresql"
        pod = self.kubectl("get", "pods", "-n", self.ns, "-l",
                           "app.kubernetes.io/name=postgresql",
                           "-o", "jsonpath={.items[0].metadata.name}", check=False).stdout.strip()
        if not pod:
            raise AssertionError("no postgres pod found for the deployment")
        return pod

    def psql(self, sql):
        """Run one SQL statement in the deployment's postgres, return stdout.

        Used only to read an execution's stored context (which version ran a
        task), for which the API exposes no field — the same channel the e2e
        compiler and leader lanes use.
        """
        target = self._postgres_target()
        proc = self.kubectl("exec", target, "-n", self.ns, "--",
                            "env", "PGPASSWORD=cloacina", "psql", "-U", "cloacina",
                            "-d", "cloacina", "-tAc", sql, check=False)
        return (proc.stdout or "").strip()

    def execution_context(self, tenant, execution_id):
        """The stored context of an execution, as a dict (via postgres)."""
        raw = self.psql(
            f"SELECT c.value FROM {tenant}.contexts c "
            f"JOIN {tenant}.workflow_executions we ON we.context_id = c.id "
            f"WHERE we.id = '{execution_id}';"
        )
        try:
            return json.loads(raw) if raw else {}
        except json.JSONDecodeError:
            return {}

    def server_restart_count(self):
        data = self.kubectl_json("get", "pods", "-n", self.ns, "-l",
                                 "app.kubernetes.io/name=cloacina-server")
        pods = (data or {}).get("items", [])
        assert pods, "no server pod found: the restart count would be a false zero"
        total = 0
        for pod in pods:
            for cs in pod.get("status", {}).get("containerStatuses", []):
                total += cs.get("restartCount", 0)
        return total

    def agent_pod_logs(self, tenant):
        ns = self.tenant_namespace(tenant)
        proc = self.kubectl("logs", "-n", ns, "-l",
                            "app.kubernetes.io/name=cloacina-agent",
                            "--tail=500", "--all-containers=true", check=False)
        return (proc.stdout or "") + (proc.stderr or "")

    def marked_lines(self, marker):
        """Every log line that starts with `marker`, from the server pod, each
        compiler pod and each agent pod: (where, parsed JSON). Says in which
        processes a package's code ran."""
        places = [("server", self.ns, f"deploy/{self.release}-cloacina-server")]
        for tenant in self.tenants:
            if self.chart_compilers:
                # Made by the server's actuator, in the tenant's namespace.
                places.append((f"compiler of {tenant}", self.tenant_namespace(tenant),
                               "deploy/cloacina-compiler"))
            else:
                places.append((f"compiler of {tenant}", self.ns,
                               "deploy/cloacina-compiler-" + tenant.replace("_", "-")))
            places.append((f"agent of {tenant}", self.tenant_namespace(tenant),
                           "deploy/cloacina-agent"))
        found = []
        for where, ns, target in places:
            proc = self.kubectl("logs", "-n", ns, target, "--all-containers=true", check=False)
            seen = set()
            for line in ((proc.stdout or "") + (proc.stderr or "")).splitlines():
                line = line.strip()
                if line.startswith(marker) and line not in seen:
                    seen.add(line)
                    try:
                        found.append((where, json.loads(line[len(marker):].strip())))
                    except json.JSONDecodeError:
                        pass
        return found

    # -- provisioning ------------------------------------------------------

    def prepare_compiler_image(self, skip_build):
        """Tag+push a compiler image to the local registry, building it from
        docker/Dockerfile.compiler when none is present. Returns the in-cluster ref."""
        host_ref = f"{self._registry_host()}/cloacina-compiler:{self.tag}"
        k8s_ref = f"{self._registry_k8s()}/cloacina-compiler:{self.tag}"
        present = subprocess.run(["docker", "image", "inspect", host_ref],
                                 capture_output=True).returncode == 0
        if not present:
            src = next((s for s in COMPILER_SRC_IMAGES
                        if subprocess.run(["docker", "image", "inspect", s],
                                          capture_output=True).returncode == 0), None)
            if src:
                subprocess.run(["docker", "tag", src, host_ref], check=True)
            elif skip_build:
                raise AssertionError(
                    "--skip-build but no compiler image present; drop --skip-build to build "
                    "docker/Dockerfile.compiler, or `docker build -t cloacina-compiler:latest "
                    "-f docker/Dockerfile.compiler .` first")
            else:
                subprocess.run(["docker", "build", "-t", host_ref, "-f",
                                str(self.root / "docker" / "Dockerfile.compiler"), str(self.root)],
                               check=True)
        subprocess.run(["docker", "push", host_ref], check=True)
        return k8s_ref

    def _registry_host(self):
        from ..e2e.k8s_fleet import REGISTRY_HOST
        return REGISTRY_HOST

    def _registry_k8s(self):
        from ..e2e.k8s_fleet import REGISTRY_K8S
        return REGISTRY_K8S

    def provision_tenants(self, compiler_ref, agent_ref=None):
        """Create both tenants, mint a write key each, deploy a per-tenant
        compiler, and wait for one agent of each tenant to register.

        With `agent_ref`, the driver applies the agent of each tenant itself
        (`deploy_agent`). With none, the server's Kubernetes actuator makes
        them (CLOACINA_INITIAL_AGENTS=1).

        With `self.chart_compilers`, the driver applies no compiler: the
        server's actuator starts one for a tenant when its build queue gets a
        package (CLOACINA-T-1034)."""
        for tenant in self.tenants:
            code, body = self.api("POST", "/v1/tenants", key=self.bootstrap_key,
                                  body={"name": tenant}, expect=None)
            # A tenant that exists answers 400 with the SQL error, not 409.
            if code not in (200, 201, 409) and "already exists" not in str(body):
                raise AssertionError(f"creating tenant {tenant} -> {code}: {str(body)[:300]}")
            self.api("POST", f"/v1/tenants/{tenant}/limits", key=self.bootstrap_key,
                     body={"max_agents": 4}, expect=(200, 201))
            _, created = self.api("POST", f"/v1/tenants/{tenant}/keys", key=self.bootstrap_key,
                                  body={"name": f"{tenant}-publish", "role": "write"},
                                  expect=(200, 201))
            self.keys[tenant] = created["key"]
            # The secrets routes need the tenant's admin role.
            _, created = self.api("POST", f"/v1/tenants/{tenant}/keys", key=self.bootstrap_key,
                                  body={"name": f"{tenant}-admin", "role": "admin"},
                                  expect=(200, 201))
            self.admin_keys[tenant] = created["key"]
            if not self.chart_compilers:
                self.deploy_compiler(tenant, compiler_ref)
            if agent_ref is not None:
                self.deploy_agent(tenant, agent_ref)
        self.wait_for_agents()

    def deploy_agent(self, tenant, agent_ref):
        """Apply what the server's Kubernetes actuator would make for a tenant:
        the namespace, the agent key Secret, the hardened agent Deployment and
        the NetworkPolicy (egress to DNS and to the server pods only).

        The manifests copy `crates/cloacina-server/src/actuator/kubernetes.rs`
        (`agent_deployment_manifest`, `network_policy_manifest`) and the key
        has the actuator's role (`agent`, tenant-scoped). The driver does this
        because the server does not start with `fleet.actuator=kubernetes`
        today (see features/publishing/README.md). Keep the copy in step with
        the actuator, or the probe scenario measures the wrong pod.
        """
        ns = self.tenant_namespace(tenant)
        if self.kubectl_json("get", "deploy", "cloacina-agent", "-n", ns):
            return
        _, created = self.api("POST", f"/v1/tenants/{tenant}/keys", key=self.bootstrap_key,
                              body={"name": f"agent:{tenant}:bdd-driver", "role": "agent"},
                              expect=(200, 201))
        managed = {"cloacina.managed": "true"}
        labels = {"app.kubernetes.io/name": "cloacina-agent", "cloacina.managed": "true",
                  "cloacina.tenant": tenant}
        server_url = f"http://{self.release}-cloacina-server.{self.ns}.svc.cluster.local:8080"
        items = [
            {"apiVersion": "v1", "kind": "Namespace",
             "metadata": {"name": ns, "labels": managed}},
            {"apiVersion": "v1", "kind": "Secret", "type": "Opaque",
             "metadata": {"name": "cloacina-agent-key", "namespace": ns, "labels": managed},
             "stringData": {AGENT_SECRET_KEY: created["key"]}},
            {"apiVersion": "apps/v1", "kind": "Deployment",
             "metadata": {"name": "cloacina-agent", "namespace": ns, "labels": labels},
             "spec": {
                 "replicas": 1,
                 "selector": {"matchLabels": {"app.kubernetes.io/name": "cloacina-agent"}},
                 "template": {
                     "metadata": {"labels": labels},
                     "spec": {
                         "securityContext": {
                             "runAsNonRoot": True, "runAsUser": 10001, "runAsGroup": 10001,
                             "fsGroup": 10001, "seccompProfile": {"type": "RuntimeDefault"}},
                         "containers": [{
                             "name": "cloacina-agent",
                             "image": agent_ref,
                             "securityContext": {
                                 "allowPrivilegeEscalation": False,
                                 "readOnlyRootFilesystem": True,
                                 "capabilities": {"drop": ["ALL"]}},
                             "env": [
                                 {"name": "CLOACINA_SERVER", "value": server_url},
                                 {"name": "CLOACINA_API_KEY", "valueFrom": {"secretKeyRef": {
                                     "name": "cloacina-agent-key", "key": AGENT_SECRET_KEY}}}],
                             "resources": {
                                 "requests": {"cpu": "250m", "memory": "256Mi"},
                                 "limits": {"cpu": "1", "memory": "1Gi"}},
                             "volumeMounts": [
                                 {"name": "home", "mountPath": "/home/cloacina"},
                                 {"name": "tmp", "mountPath": "/tmp"}],
                         }],
                         "volumes": [
                             {"name": "home", "emptyDir": {"sizeLimit": "2Gi"}},
                             {"name": "tmp", "emptyDir": {"sizeLimit": "1Gi"}}],
                     }}}},
            {"apiVersion": "networking.k8s.io/v1", "kind": "NetworkPolicy",
             "metadata": {"name": "cloacina-agent-fleet", "namespace": ns, "labels": managed},
             "spec": {
                 "podSelector": {},
                 "policyTypes": ["Ingress", "Egress"],
                 "ingress": [],
                 "egress": [
                     {"to": [{"namespaceSelector": {"matchLabels": {
                         "kubernetes.io/metadata.name": "kube-system"}}}],
                      "ports": [{"protocol": "UDP", "port": 53}, {"protocol": "TCP", "port": 53}]},
                     {"to": [{"namespaceSelector": {"matchLabels": {
                         "kubernetes.io/metadata.name": self.ns}},
                         "podSelector": {"matchLabels": {
                             "app.kubernetes.io/name": "cloacina-server",
                             "app.kubernetes.io/instance": self.release}}}],
                      "ports": [{"protocol": "TCP", "port": 8080}]},
                 ]}},
        ]
        # The key travels on stdin, never on a command line.
        proc = subprocess.run(["kubectl", "apply", "-f", "-"],
                              input=json.dumps({"apiVersion": "v1", "kind": "List", "items": items}),
                              env=self._kube_env(), text=True, capture_output=True, check=False)
        if proc.returncode != 0:
            raise AssertionError(f"failed to apply the agent of {tenant}: {proc.stderr}")

    def deploy_compiler(self, tenant, compiler_ref):
        """kubectl apply a compiler Deployment scoped to the tenant's schema.

        The chart's compiler serves the public schema only; a tenant-scoped
        compiler (`--tenant-schema`) claims and builds only that tenant's
        packages, mirroring the per-tenant agent fleet. --dev-workspace resolves
        the fixtures' version deps against the baked /workspace, since the
        cloacina crates are not on crates.io.
        """
        # A Kubernetes name allows no underscore; a schema name allows no hyphen.
        name = "cloacina-compiler-" + tenant.replace("_", "-")
        manifest = {
            "apiVersion": "apps/v1", "kind": "Deployment",
            "metadata": {"name": name, "namespace": self.ns},
            "spec": {
                "replicas": 1,
                "selector": {"matchLabels": {"app": name}},
                "template": {
                    "metadata": {"labels": {"app": name}},
                    "spec": {"containers": [{
                        "name": "compiler",
                        "image": compiler_ref,
                        "imagePullPolicy": "IfNotPresent",
                        "args": ["--bind", "0.0.0.0:9000",
                                 "--database-url", self.db_url_incluster,
                                 "--poll-interval-ms", "1000",
                                 "--tenant-schema", tenant,
                                 "--cargo-target-dir", "/workspace/target",
                                 # Release, like the agent image that
                                 # loads the result and like the dependencies
                                 # the image already built. Online: the
                                 # default --frozen --offline needs a vendor dir.
                                 "--cargo-flags-replace=build",
                                 "--cargo-flags-replace=--release",
                                 "--cargo-flags-replace=--lib"],
                        "env": [{"name": "CLOACINA_COMPILER_DEV_WORKSPACE", "value": "/workspace"}],
                    }]},
                },
            },
        }
        proc = subprocess.run(["kubectl", "apply", "-n", self.ns, "-f", "-"],
                              input=json.dumps(manifest), env=self._kube_env(),
                              text=True, capture_output=True, check=False)
        if proc.returncode != 0:
            raise AssertionError(f"failed to apply compiler for {tenant}: {proc.stderr}")

    def wait_for_agents(self, timeout_s=180):
        """Wait until each tenant has at least one registered agent."""
        deadline = time.time() + timeout_s
        while time.time() < deadline:
            _, roster = self.api("GET", "/v1/agents", key=self.bootstrap_key, expect=(200,))
            items = roster.get("items", []) if isinstance(roster, dict) else []
            have = {t for t in self.tenants if any(a.get("tenant_id") == t for a in items)}
            if have == set(self.tenants):
                return
            time.sleep(5)
        raise AssertionError("not every tenant has a registered agent within the timeout")

    # -- waits used by the steps ------------------------------------------

    def schedules(self, tenant, workflow, *, key):
        """The schedules the server holds for a workflow of a tenant."""
        code, body = self.api("GET", f"/v1/tenants/{tenant}/triggers", key=key, expect=None)
        if code != 200 or not isinstance(body, dict):
            return []
        return [t for t in body.get("items", []) if t.get("workflow_name") == workflow]

    def wake_tenant(self, tenant, workflow, *, key, timeout_s=60):
        """Make the server start the tenant's runner, with one execute call.

        The server makes a tenant's runner on the first execute, trigger fire
        or instance call for that tenant, not when a package is published.
        Until then no schedule of the tenant exists. The first call may answer
        "workflow not found": the runner it just made has not loaded yet.
        Returns the HTTP status of that first call."""
        code, _ = self.api("POST", f"/v1/tenants/{tenant}/workflows/{workflow}/execute",
                           key=key, body={"context": {}}, expect=None)
        deadline = time.time() + timeout_s
        while time.time() < deadline and not self.schedules(tenant, workflow, key=key):
            time.sleep(2)
        return code

    def wait_build_status(self, tenant, name, expected, *, key, version=None, timeout_s=900):
        """Poll the versions list until `version` of package `name` (or, with
        no version, the newest upload) reaches one of `expected` build states.
        Returns the row."""
        deadline = time.time() + timeout_s
        last = None
        while time.time() < deadline:
            rows = self.list_versions(tenant, name, key=key) or []
            if version is not None:
                rows = [r for r in rows if r.get("version") == version]
            # Newest upload first: a version uploaded again has an old row too.
            if rows:
                last = rows[0]
                if last.get("build_status") in expected:
                    return last
            time.sleep(3)
        brief = {k: (last or {}).get(k) for k in ("version", "build_status", "active", "superseded")}
        raise AssertionError(
            f"{name} {version or '(newest)'} did not reach {sorted(expected)} within {timeout_s}s; "
            f"last: {brief}; build_error: {str((last or {}).get('build_error'))[-600:]}")

    def ensure_version(self, tenant, name, fixture, version, *, key, timeout_s=900):
        """Make `version` of package `name` the one in service: publish it when
        the tenant does not have a good build of it yet, else make that build
        active again. Returns the row. For `Given` steps, so a scenario does
        not depend on which scenarios ran before it."""
        rows = self.list_versions(tenant, name, key=key) or []
        good = next((r for r in rows if r.get("version") == version
                     and r.get("build_status") == "success"), None)
        if good is None:
            archive = self.pack(self.stage_fixture(fixture, version=version))
            code, out, err = self.publish_archive(archive, tenant=tenant, key=key)
            assert code == 0, f"publishing {name} {version} failed: {err or out}"
            good = self.wait_build_status(tenant, name, {"success", "failed"}, key=key,
                                          version=version, timeout_s=timeout_s)
            assert good.get("build_status") == "success", (
                f"{name} {version} did not build: {str(good.get('build_error'))[-600:]}")
        deadline = time.time() + 60
        while time.time() < deadline:
            active = self.active_version(tenant, name, key=key)
            if active and active.get("version") == version:
                return active
            if good.get("superseded"):
                self.api("POST", f"/v1/tenants/{tenant}/workflows/{name}/versions/{version}/activate",
                         key=key, body={}, expect=None)
            time.sleep(2)
        raise AssertionError(f"{name} {version} is built but did not come into service")

    def list_versions(self, tenant, name, *, key):
        """The CLOACINA-T-1018 versions list, or None when the route is absent."""
        code, body = self.api("GET", f"/v1/tenants/{tenant}/workflows/{name}/versions",
                              key=key, expect=None)
        if code == 200 and isinstance(body, dict):
            return body.get("items", [])
        return None

    def active_version(self, tenant, name, *, key):
        rows = self.list_versions(tenant, name, key=key)
        if rows is None:
            return None
        return next((r for r in rows if r.get("active")), None)
