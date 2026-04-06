"""
Auth integration tests — verify authorization enforcement end-to-end.

Boots the server with Postgres, exercises:
- Single-tenant flow (bootstrap key does everything)
- Key scoping (global vs tenant-scoped)
- Tenant isolation (cross-tenant denied)
- Role enforcement (read/write/admin)
- God mode (is_admin bypasses tenant checks)
- Deny scenarios (no auth, revoked key, wrong tenant, wrong role)
"""

import json
import signal
import subprocess
import time
from pathlib import Path

import angreal  # type: ignore

from .cloacina_utils import print_section_header, print_final_success


def build_server():
    """Build the server binary."""
    print("Building cloacinactl server (debug)...")
    subprocess.run(["cargo", "build", "-p", "cloacinactl"], check=True)


def find_server_binary():
    binary = Path("target/debug/cloacinactl")
    if not binary.exists():
        raise FileNotFoundError(f"Server binary not found at {binary}.")
    return str(binary)


def start_postgres():
    subprocess.run(
        ["docker", "compose", "-f", ".angreal/docker-compose.yaml", "up", "-d"],
        check=True,
    )
    for _ in range(30):
        result = subprocess.run(
            ["docker", "compose", "-f", ".angreal/docker-compose.yaml", "exec", "-T",
             "postgres", "pg_isready", "-U", "cloacina"],
            capture_output=True,
        )
        if result.returncode == 0:
            print("  Postgres is ready.")
            return
        time.sleep(1)
    raise RuntimeError("Postgres failed to start")


def stop_postgres():
    subprocess.run(
        ["docker", "compose", "-f", ".angreal/docker-compose.yaml", "down", "-v"],
        capture_output=True,
    )


def api_request(method, url, token=None, data=None):
    """Make an HTTP request and return (status_code, json_body)."""
    import urllib.request
    import urllib.error

    headers = {}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    if data is not None:
        body = json.dumps(data).encode()
        headers["Content-Type"] = "application/json"
        req = urllib.request.Request(url, data=body, headers=headers, method=method)
    else:
        req = urllib.request.Request(url, headers=headers, method=method)

    try:
        with urllib.request.urlopen(req) as resp:
            return resp.status, json.loads(resp.read())
    except urllib.error.HTTPError as e:
        try:
            body = json.loads(e.read())
        except Exception:
            body = {"error": str(e)}
        return e.code, body


cloacina = angreal.command_group(
    name="cloacina", about="commands for Cloacina core engine tests"
)


@cloacina()
@angreal.command(
    name="auth-integration",
    about="run auth integration tests — tenant isolation, roles, god mode, deny scenarios",
    when_to_use=[
        "validating authorization enforcement",
        "testing tenant isolation",
        "verifying role-based access",
    ],
    when_not_to_use=[
        "unit testing",
        "soak testing (use server-soak instead)",
    ],
)
def auth_integration_test():
    """Run authorization integration tests with full server bootstrap."""

    print_section_header("Setup: Build server")
    build_server()
    server_binary = find_server_binary()

    print_section_header("Setup: Start Postgres")
    start_postgres()

    base_url = "http://127.0.0.1:18082"
    db_url = "postgres://cloacina:cloacina@localhost:5432/cloacina"
    test_home = Path("target/auth-integration-test")
    if test_home.exists():
        import shutil
        shutil.rmtree(test_home)

    server_proc = None
    bootstrap_key = "clk_auth_integration_test_key_000000000000000"
    passed = 0
    failed = 0

    def check(name, condition, detail=""):
        nonlocal passed, failed
        if condition:
            print(f"  PASS: {name}")
            passed += 1
        else:
            print(f"  FAIL: {name} — {detail}")
            failed += 1

    try:
        # Start server
        print_section_header("Setup: Start server")
        test_home.mkdir(parents=True, exist_ok=True)
        stderr_path = test_home / "server_stderr.log"
        stderr_file = open(stderr_path, "w")

        server_proc = subprocess.Popen(
            [server_binary, "serve", "--home", str(test_home),
             "--database-url", db_url, "--bind", "127.0.0.1:18082",
             "--bootstrap-key", bootstrap_key],
            stdout=subprocess.PIPE,
            stderr=stderr_file,
        )
        print(f"  PID: {server_proc.pid}")

        # Wait for health
        for i in range(30):
            time.sleep(1)
            try:
                status, _ = api_request("GET", f"{base_url}/health")
                if status == 200:
                    print(f"  Server healthy after {i+1}s")
                    break
            except Exception:
                continue
        else:
            raise RuntimeError("Server failed to become healthy")

        token = bootstrap_key

        # =================================================================
        # Test group 1: Deny scenarios
        # =================================================================
        print_section_header("Test group 1: Deny scenarios")

        # No auth → 401
        s, _ = api_request("GET", f"{base_url}/auth/keys")
        check("no auth → 401", s == 401, f"got {s}")

        # Invalid token → 401
        s, _ = api_request("GET", f"{base_url}/auth/keys", token="clk_bogus_invalid_key")
        check("invalid token → 401", s == 401, f"got {s}")

        # =================================================================
        # Test group 2: Bootstrap key (god mode)
        # =================================================================
        print_section_header("Test group 2: Bootstrap key (god mode)")

        # Bootstrap can list keys
        s, body = api_request("GET", f"{base_url}/auth/keys", token=token)
        check("bootstrap lists keys", s == 200, f"got {s}")

        # Bootstrap key response includes is_admin and tenant_id
        keys = body.get("keys", [])
        bootstrap_info = next((k for k in keys if k.get("name") == "bootstrap-admin"), None)
        check("bootstrap key in list", bootstrap_info is not None)
        if bootstrap_info:
            check("bootstrap is_admin=true", bootstrap_info.get("is_admin") is True,
                  f"got {bootstrap_info.get('is_admin')}")
            check("bootstrap tenant_id=null", bootstrap_info.get("tenant_id") is None,
                  f"got {bootstrap_info.get('tenant_id')}")

        # Bootstrap can access public tenant
        s, _ = api_request("GET", f"{base_url}/tenants/public/workflows", token=token)
        check("bootstrap accesses public tenant", s == 200, f"got {s}")

        # =================================================================
        # Test group 3: Global key (tenant_id=NULL, not admin)
        # =================================================================
        print_section_header("Test group 3: Global key")

        s, body = api_request("POST", f"{base_url}/auth/keys", token=token,
                              data={"name": "global-key"})
        check("create global key", s == 201, f"got {s}")
        global_key = body.get("key", "")

        # Global key can access public tenant
        s, _ = api_request("GET", f"{base_url}/tenants/public/workflows", token=global_key)
        check("global key → public tenant", s == 200, f"got {s}")

        # Global key cannot create tenants (not admin)
        s, _ = api_request("POST", f"{base_url}/tenants", token=global_key,
                           data={"schema_name": "denied_tenant", "username": "denied_user"})
        check("global key cannot create tenant", s == 403, f"got {s}")

        # =================================================================
        # Test group 4: Role enforcement
        # =================================================================
        print_section_header("Test group 4: Role enforcement")

        # Create a read-only key
        s, body = api_request("POST", f"{base_url}/auth/keys", token=token,
                              data={"name": "read-only-key", "role": "read"})
        check("create read-only key", s == 201, f"got {s}")
        read_key = body.get("key", "")

        # Read key can list workflows
        s, _ = api_request("GET", f"{base_url}/tenants/public/workflows", token=read_key)
        check("read key lists workflows", s == 200, f"got {s}")

        # Read key cannot execute workflow (needs write)
        s, _ = api_request("POST", f"{base_url}/tenants/public/workflows/nonexistent/execute",
                           token=read_key, data={"context": {}})
        check("read key cannot execute → 403", s == 403, f"got {s}")

        # Create a write key
        s, body = api_request("POST", f"{base_url}/auth/keys", token=token,
                              data={"name": "write-key", "role": "write"})
        check("create write key", s == 201, f"got {s}")
        write_key = body.get("key", "")

        # Write key can execute (even if workflow doesn't exist, should get past auth)
        s, _ = api_request("POST", f"{base_url}/tenants/public/workflows/nonexistent/execute",
                           token=write_key, data={"context": {}})
        check("write key can attempt execute (not 403)", s != 403, f"got {s}")

        # Write key cannot revoke keys (needs admin)
        dummy_id = "00000000-0000-0000-0000-000000000000"
        s, _ = api_request("DELETE", f"{base_url}/auth/keys/{dummy_id}", token=write_key)
        check("write key cannot revoke → 403", s == 403, f"got {s}")

        # =================================================================
        # Test group 5: Tenant-scoped key isolation
        # =================================================================
        print_section_header("Test group 5: Tenant isolation")

        # Create a tenant-scoped key via bootstrap (admin)
        s, body = api_request("POST", f"{base_url}/tenants/public/keys", token=token,
                              data={"name": "public-tenant-key"})
        check("create tenant-scoped key", s == 201, f"got {s}")
        tenant_key = body.get("key", "")
        if s == 201:
            check("tenant key has tenant_id=public",
                  body.get("tenant_id") == "public",
                  f"got tenant_id={body.get('tenant_id')}")

        # Tenant key can access its own tenant
        if tenant_key:
            s, _ = api_request("GET", f"{base_url}/tenants/public/workflows", token=tenant_key)
            check("tenant key → own tenant", s == 200, f"got {s}")

            # Tenant key cannot access a different tenant
            s, _ = api_request("GET", f"{base_url}/tenants/other_tenant/workflows", token=tenant_key)
            check("tenant key → other tenant → 403", s == 403, f"got {s}")

        # =================================================================
        # Test group 6: Revoked key
        # =================================================================
        print_section_header("Test group 6: Revoked key")

        # Create a key to revoke
        s, body = api_request("POST", f"{base_url}/auth/keys", token=token,
                              data={"name": "to-revoke"})
        revoke_key_token = body.get("key", "")
        revoke_key_id = body.get("id", "")

        # Verify it works before revocation
        s, _ = api_request("GET", f"{base_url}/auth/keys", token=revoke_key_token)
        check("key works before revoke", s == 200, f"got {s}")

        # Revoke it
        s, _ = api_request("DELETE", f"{base_url}/auth/keys/{revoke_key_id}", token=token)
        check("revoke key", s == 200, f"got {s}")

        # Wait for cache to expire (cache TTL is 30s, but clear happens on revoke)
        time.sleep(1)

        # Revoked key → 401
        s, _ = api_request("GET", f"{base_url}/auth/keys", token=revoke_key_token)
        check("revoked key → 401", s == 401, f"got {s}")

        # =================================================================
        # Results
        # =================================================================
        print_section_header("Results")
        print(f"  Passed: {passed}")
        print(f"  Failed: {failed}")

        if failed == 0:
            print_final_success("All auth integration tests passed!")
        else:
            raise RuntimeError(f"{failed} auth integration test(s) FAILED")

    except Exception:
        if stderr_path.exists():
            stderr = stderr_path.read_text()
            if stderr.strip():
                print("\n  === Server stderr (last 20 lines) ===")
                for line in stderr.splitlines()[-20:]:
                    print(f"    {line}")
        raise
    finally:
        if server_proc:
            server_proc.send_signal(signal.SIGTERM)
            try:
                server_proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                server_proc.kill()
            print("  Server stopped.")
        stop_postgres()
