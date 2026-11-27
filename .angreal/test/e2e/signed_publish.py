"""End-to-end test of package signing (CLOACINA-T-1121, CLOACINA-A-0018).

A server started with `--require-signatures`, a tenant with one trusted key,
and `cloacinactl package publish`:

  1. with no `--sign`: refused, `signature_required`;
  2. signed by a key the tenant does not trust: refused, `untrusted_signer`;
  3. signed by the trusted key: accepted. The tenant's compiler builds it,
     the tenant's runner loads it (the load checks the signature again), and
     a run of the workflow reaches Completed with the task's output in its
     context.

The package is the Rust fixture `compiler-happy-rust`. Its tasks run in the
server: no agents.

The two signing keys are made here with OpenSSL, in the run's home, mode 600.
They are never printed and never stored in the repository.

Requires Docker, cargo and openssl. Run with: `angreal test e2e signed-publish`.
"""

import json
import os
import re
import subprocess
import tempfile
import time
import urllib.request
from pathlib import Path

import angreal  # type: ignore

from .._utils import print_final_success, print_section_header
from .compiler import (
    REPO_ROOT,
    _assert_ports_free,
    _build_binaries,
    _cloacinactl,
    _kill,
    _psql,
    _stage_fixture,
    _start_postgres,
    _wait_http,
)

test = angreal.command_group(
    name="test", about="Cloacina test suites (unit, integration, e2e, soak)"
)
e2e = angreal.command_group(name="e2e", about="end-to-end tests against a live server")

DB_URL = "postgres://cloacina:cloacina@localhost:15432/cloacina"
BOOTSTRAP_KEY = "test-bootstrap-signed-publish"
SERVER_BIND = "127.0.0.1:18089"
COMPILER_BIND = "127.0.0.1:19009"
SERVER_URL = f"http://{SERVER_BIND}"
TENANT = "signed_e2e"
FIXTURE = "compiler-happy-rust"
WORKFLOW = "compiler_happy_workflow"
TERMINAL = {"Completed", "Failed", "Cancelled"}
_ANSI = re.compile(r"\x1b\[[0-9;]*m")


def _make_key(home: Path, name: str) -> tuple[Path, Path]:
    """An Ed25519 key pair: the private key (PKCS#8 PEM, mode 600) and its
    public key (PEM)."""
    private = home / f"{name}.pem"
    public = home / f"{name}.pub.pem"
    subprocess.run(
        ["openssl", "genpkey", "-algorithm", "ed25519", "-out", str(private)],
        check=True,
        capture_output=True,
    )
    os.chmod(private, 0o600)
    subprocess.run(
        ["openssl", "pkey", "-in", str(private), "-pubout", "-out", str(public)],
        check=True,
        capture_output=True,
    )
    return private, public


def _tctl(home: Path, *args, check=True, env=None):
    return _cloacinactl(home, "--tenant", TENANT, *args, check=check, env=env)


def _log_text(path: Path) -> str:
    return _ANSI.sub("", path.read_text(errors="replace"))


def _assert_refused(server_log: Path, result, code: str, message: str):
    """A publish refused by the signature check: `cloacinactl` exits 4 (the
    403 of the route) with the route's message, and the server's audit line
    of the upload names the code."""
    exit_code, out, err = result
    if exit_code != 4 or message not in err:
        raise AssertionError(
            f"expected the publish refused with {code} (exit 4, {message!r}); "
            f"got exit {exit_code}\nSTDOUT:\n{out}\nSTDERR:\n{err}"
        )
    audit = [
        line
        for line in _log_text(server_log).splitlines()
        if f"failure_reason={code}" in line and f"upload:tenant={TENANT}" in line
    ]
    if not audit:
        raise AssertionError(
            f"the server logged no package.load.failure with failure_reason={code} "
            f"for tenant {TENANT}"
        )


def _poll_tenant_build(home: Path, pkg_id: str, timeout_s: float) -> dict:
    deadline = time.time() + timeout_s
    body: dict = {}
    while time.time() < deadline:
        _, out, _ = _tctl(home, "-o", "json", "package", "inspect", pkg_id, check=False)
        try:
            body = json.loads(out)
        except json.JSONDecodeError:
            body = {}
        if body.get("build_status") in ("success", "failed"):
            return body
        time.sleep(2.0)
    raise AssertionError(f"package {pkg_id} never built within {timeout_s}s: {body!r}")


def _run_in_tenant(home: Path, timeout_s: float) -> str:
    """`workflow run` until the tenant's runner has loaded the workflow.
    Returns the execution id."""
    deadline = time.time() + timeout_s
    last = ""
    while time.time() < deadline:
        code, out, err = _tctl(home, "-o", "json", "workflow", "run", WORKFLOW, check=False)
        if code == 0:
            try:
                exec_id = json.loads(out).get("execution_id")
            except json.JSONDecodeError:
                exec_id = (out.strip().splitlines() or [""])[-1].strip()
            if exec_id and len(exec_id) >= 32:
                return exec_id
        last = err.strip() or out.strip()
        time.sleep(2.0)
    raise AssertionError(
        f"workflow run {WORKFLOW} in tenant {TENANT} never accepted within "
        f"{timeout_s}s: {last}"
    )


def _wait_execution(home: Path, exec_id: str, timeout_s: float) -> str:
    deadline = time.time() + timeout_s
    status = None
    while time.time() < deadline:
        _, out, _ = _tctl(home, "-o", "json", "execution", "status", exec_id, check=False)
        try:
            status = json.loads(out).get("status")
        except json.JSONDecodeError:
            status = None
        if status in TERMINAL:
            return status
        time.sleep(1.0)
    raise AssertionError(f"execution {exec_id} never ended; last status {status!r}")


def _get_json(path: str) -> dict:
    req = urllib.request.Request(
        f"{SERVER_URL}{path}", headers={"Authorization": f"Bearer {BOOTSTRAP_KEY}"}
    )
    with urllib.request.urlopen(req, timeout=10) as resp:
        return json.loads(resp.read())


@test()
@e2e()
@angreal.command(
    name="signed-publish",
    about="package signing e2e: publish --sign to a server with --require-signatures (CLOACINA-T-1121)",
    when_to_use=[
        "validating package signing end to end against a live server",
        "pre-release check of publish --sign, trusted keys and the signed load",
    ],
    when_not_to_use=["unit testing", "running without docker"],
)
def signed_publish():
    print_section_header("package signing e2e")
    _build_binaries()
    _start_postgres()
    _assert_ports_free(18089, 19009)

    home = Path(tempfile.mkdtemp(prefix="signed-publish-e2e-"))
    print(f"signed-publish-e2e home: {home}")
    server_log_path = home / "server.log"
    compiler_log_path = home / "compiler.log"
    server_proc = None
    compiler_proc = None
    # The publish builds the package on this machine, and the compiler builds
    # it again: one target dir for both, so the dependencies compile once.
    shared_target = REPO_ROOT / "target" / "compiler-e2e-cache"
    shared_target.mkdir(parents=True, exist_ok=True)
    publish_env = os.environ.copy()
    publish_env["CARGO_TARGET_DIR"] = str(shared_target)

    try:
        server_log = open(server_log_path, "w")  # noqa: SIM115 - stays open for the subprocess
        server_proc = subprocess.Popen(
            [
                "target/debug/cloacina-server",
                "--home", str(home),
                "--database-url", DB_URL,
                "--bind", SERVER_BIND,
                "--bootstrap-key", BOOTSTRAP_KEY,
                "--require-signatures",
                "--verbose",
            ],
            cwd=REPO_ROOT,
            stdout=server_log,
            stderr=subprocess.STDOUT,
        )
        _wait_http(f"{SERVER_URL}/health", "server", proc=server_proc)
        print("  ok: server up with --require-signatures")

        _cloacinactl(
            home,
            "config", "profile", "set", "local", SERVER_URL,
            "--api-key", BOOTSTRAP_KEY,
            "--default",
        )
        _cloacinactl(home, "tenant", "create", TENANT)
        print(f"  ok: tenant {TENANT} created")

        # The default compiler builds `public` only; a tenant's packages are
        # built by a compiler scoped to its schema.
        compiler_log = open(compiler_log_path, "w")  # noqa: SIM115 - stays open for the subprocess
        compiler_proc = subprocess.Popen(
            [
                "target/debug/cloacina-compiler",
                "--home", str(home),
                "--database-url", DB_URL,
                "--bind", COMPILER_BIND,
                "--tenant-schema", TENANT,
                "--poll-interval-ms", "500",
                "--cargo-target-dir", str(shared_target),
                "--cargo-flags-replace=build",
                "--cargo-flags-replace=--lib",
                "--verbose",
            ],
            cwd=REPO_ROOT,
            stdout=compiler_log,
            stderr=subprocess.STDOUT,
        )
        _wait_http(
            f"http://{COMPILER_BIND}/health", "compiler", timeout_s=60.0, proc=compiler_proc
        )
        print(f"  ok: compiler up for tenant {TENANT}")

        trusted_key, trusted_pub = _make_key(home, "trusted-key")
        other_key, _ = _make_key(home, "other-key")
        _tctl(home, "trusted-key", "add", "--pem-file", str(trusted_pub), "--name", "signed-e2e")
        _, out, _ = _tctl(home, "-o", "json", "trusted-key", "list")
        keys = json.loads(out)
        if len(keys) != 1 or keys[0].get("revoked"):
            raise AssertionError(f"expected one live trusted key for {TENANT}: {keys!r}")
        print(f"  ok: one trusted key added for {TENANT}")

        pkg_dir = _stage_fixture(home, FIXTURE)

        # --- 1. no --sign: refused ----------------------------------------
        print("  publishing with no --sign (builds the package here; first run is a cold build)")
        result = _tctl(home, "package", "publish", str(pkg_dir), check=False, env=publish_env)
        _assert_refused(server_log_path, result, "signature_required", "requires signed packages")
        print("  ok: publish with no --sign refused: signature_required")

        # --- 2. a key the tenant does not trust: refused -------------------
        result = _tctl(
            home, "package", "publish", str(pkg_dir), "--sign", str(other_key),
            check=False, env=publish_env,
        )
        _assert_refused(server_log_path, result, "untrusted_signer", "untrusted key")
        print("  ok: publish signed by an untrusted key refused: untrusted_signer")

        stored = _psql(f"SELECT count(*) FROM {TENANT}.workflow_packages;")
        if stored != "0":
            raise AssertionError(f"a refused publish stored a package: {stored} rows")
        print("  ok: no package stored by the refused publishes")

        # --- 3. the trusted key: accepted, built, loaded, run ---------------
        _, out, _ = _tctl(
            home, "package", "publish", str(pkg_dir), "--sign", str(trusted_key),
            env=publish_env,
        )
        pkg_id = out.strip().splitlines()[-1].strip()
        if len(pkg_id) < 32:
            raise AssertionError(f"publish --sign printed no package id: {out!r}")
        print(f"  ok: publish --sign accepted (package {pkg_id})")

        signatures = _psql(f"SELECT count(*) FROM {TENANT}.package_signatures;")
        if signatures != "1":
            raise AssertionError(f"expected one signature row in {TENANT}: {signatures}")
        print(f"  ok: the signature is stored in {TENANT}'s schema")

        body = _poll_tenant_build(home, pkg_id, timeout_s=900.0)
        if body.get("build_status") != "success":
            raise AssertionError(f"the signed package did not build: {body!r}")
        print("  ok: the tenant's compiler built the package")

        # The run is accepted once the tenant's runner has loaded the
        # package; with --require-signatures that load verifies the
        # signature against the tenant's trusted key.
        exec_id = _run_in_tenant(home, timeout_s=180.0)
        print(f"  ok: the tenant's runner loaded the workflow; execution {exec_id}")
        status = _wait_execution(home, exec_id, timeout_s=120.0)
        if status != "Completed":
            raise AssertionError(f"execution {exec_id} ended in {status!r}, expected Completed")

        tasks = _get_json(f"/v1/tenants/{TENANT}/executions/{exec_id}/tasks")["tasks"]
        if [t.get("status") for t in tasks] != ["Completed"]:
            raise AssertionError(f"expected one Completed task: {tasks!r}")
        ctx = json.loads(
            _psql(
                f"SELECT c.value FROM {TENANT}.contexts c "
                f"JOIN {TENANT}.workflow_executions we ON we.context_id = c.id "
                f"WHERE we.id = '{exec_id}';"
            )
        )
        if ctx.get("compiler_e2e_happy_ran") is not True:
            raise AssertionError(f"the task's output is not in the run's context: {ctx!r}")
        print("  ok: execution Completed; the task wrote its output (compiler_e2e_happy_ran)")

        print_final_success("package signing e2e")
    except BaseException:
        for log in (server_log_path, compiler_log_path):
            if log.exists():
                print(f"\n---- last 80 lines of {log.name} ----")
                for line in _log_text(log).splitlines()[-80:]:
                    print(line)
        raise
    finally:
        _kill(compiler_proc)
        _kill(server_proc)
