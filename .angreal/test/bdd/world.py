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
        self.base_url = None
        self.home = Path(tempfile.mkdtemp(prefix="bdd-publish-home-"))
        # cargo honours CARGO_TARGET_DIR, so the driver does too: a worktree can
        # share one target folder with other checkouts.
        target = Path(os.environ.get("CARGO_TARGET_DIR") or (self.root / "target"))
        self.cloacinactl = target / "debug" / "cloacinactl"
        self.db_url_incluster = (
            f"postgres://cloacina:cloacina@{release}-postgresql:5432/cloacina"
        )
        # Per-tenant write keys, filled by provision_tenants.
        self.keys = {}
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
                                 "app.kubernetes.io/component=server")
        total = 0
        for pod in (data or {}).get("items", []):
            for cs in pod.get("status", {}).get("containerStatuses", []):
                total += cs.get("restartCount", 0)
        return total

    def agent_pod_logs(self, tenant):
        ns = self.tenant_namespace(tenant)
        proc = self.kubectl("logs", "-n", ns, "-l",
                            "app.kubernetes.io/name=cloacina-agent",
                            "--tail=500", "--all-containers=true", check=False)
        return (proc.stdout or "") + (proc.stderr or "")

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

    def provision_tenants(self, compiler_ref):
        """Create both tenants, mint a write key each, deploy a per-tenant
        compiler, and wait for one agent of each tenant to register."""
        for tenant in TENANTS:
            code, _ = self.api("POST", "/v1/tenants", key=self.bootstrap_key,
                               body={"name": tenant}, expect=(200, 201, 409))
            self.api("POST", f"/v1/tenants/{tenant}/limits", key=self.bootstrap_key,
                     body={"max_agents": 4}, expect=(200, 201))
            _, created = self.api("POST", f"/v1/tenants/{tenant}/keys", key=self.bootstrap_key,
                                  body={"name": f"{tenant}-publish", "role": "write"},
                                  expect=(200, 201))
            self.keys[tenant] = created["key"]
            self.deploy_compiler(tenant, compiler_ref)
        self.wait_for_agents()

    def deploy_compiler(self, tenant, compiler_ref):
        """kubectl apply a compiler Deployment scoped to the tenant's schema.

        The chart's compiler serves the public schema only; a tenant-scoped
        compiler (`--tenant-schema`) claims and builds only that tenant's
        packages, mirroring the per-tenant agent fleet. --dev-workspace resolves
        the fixtures' version deps against the baked /workspace, since the
        cloacina crates are not on crates.io.
        """
        manifest = {
            "apiVersion": "apps/v1", "kind": "Deployment",
            "metadata": {"name": f"cloacina-compiler-{tenant}", "namespace": self.ns},
            "spec": {
                "replicas": 1,
                "selector": {"matchLabels": {"app": f"cloacina-compiler-{tenant}"}},
                "template": {
                    "metadata": {"labels": {"app": f"cloacina-compiler-{tenant}"}},
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
            have = {t for t in TENANTS if any(a.get("tenant_id") == t for a in items)}
            if have == set(TENANTS):
                return
            time.sleep(5)
        raise AssertionError("not every tenant has a registered agent within the timeout")

    # -- waits used by the steps ------------------------------------------

    def wait_build_status(self, tenant, name, expected, *, key, timeout_s=900):
        """Poll `package versions`/`inspect` until a version of `name` reaches
        one of `expected` build states. Returns the matching row dict.

        Uses the CLOACINA-T-1018 `package versions` route when it is present,
        and falls back to the workflow-detail build_status otherwise.
        """
        deadline = time.time() + timeout_s
        last = None
        while time.time() < deadline:
            rows = self.list_versions(tenant, name, key=key)
            if rows is not None:
                for row in rows:
                    if row.get("build_status") in expected:
                        return row
                last = rows
            else:
                _, detail = self.api("GET", f"/v1/tenants/{tenant}/workflows/{name}",
                                     key=key, expect=None)
                if isinstance(detail, dict) and detail.get("build_status") in expected:
                    return detail
                last = detail
            time.sleep(3)
        raise AssertionError(f"no version of {name} reached {expected} within {timeout_s}s; last: {last}")

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
