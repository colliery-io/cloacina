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

from ._utils import print_section_header, print_final_success


def build_server():
    """Build the server binary."""
    print("Building cloacinactl server (debug)...")
    subprocess.run(["cargo", "build", "-p", "cloacina-server"], check=True)


def find_server_binary():
    binary = Path("target/debug/cloacina-server")
    if not binary.exists():
        raise FileNotFoundError(f"Server binary not found at {binary}.")
    return str(binary)


def start_postgres():
    subprocess.run(
        ["docker", "compose", "-f", ".angreal/docker-compose.yaml", "up", "-d"],
        check=True,
    )
    # CLOACI-T-0806: consecutive-success readiness — a single pg_isready pass
    # can land inside the init-restart bounce (exit 56).
    from ._utils import wait_for_postgres_stable

    wait_for_postgres_stable()
    print("  Postgres is ready.")


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
            raw = resp.read()
            try:
                return resp.status, json.loads(raw)
            except Exception:
                # A 2xx with an empty/non-JSON body (e.g. a no-content response)
                # is valid — don't crash the whole suite on it.
                return resp.status, {}
    except urllib.error.HTTPError as e:
        try:
            body = json.loads(e.read())
        except Exception:
            body = {"error": str(e)}
        return e.code, body


test = angreal.command_group(
    name="test", about="Cloacina test suites (unit, integration, e2e, soak)"
)


@test()
@angreal.command(
    name="auth",
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
            [server_binary, "--home", str(test_home),
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
        s, _ = api_request("GET", f"{base_url}/v1/auth/keys")
        check("no auth → 401", s == 401, f"got {s}")

        # Invalid token → 401
        s, _ = api_request("GET", f"{base_url}/v1/auth/keys", token="clk_bogus_invalid_key")
        check("invalid token → 401", s == 401, f"got {s}")

        # =================================================================
        # Test group 2: Bootstrap key (god mode)
        # =================================================================
        print_section_header("Test group 2: Bootstrap key (god mode)")

        # Bootstrap can list keys
        s, body = api_request("GET", f"{base_url}/v1/auth/keys", token=token)
        check("bootstrap lists keys", s == 200, f"got {s}")

        # Bootstrap key response includes is_admin and tenant_id.
        # The list response uses the standard items envelope (T-0594).
        keys = body.get("items", body.get("keys", []))
        bootstrap_info = next((k for k in keys if k.get("name") == "bootstrap-admin"), None)
        check("bootstrap key in list", bootstrap_info is not None)
        if bootstrap_info:
            check("bootstrap is_admin=true", bootstrap_info.get("is_admin") is True,
                  f"got {bootstrap_info.get('is_admin')}")
            check("bootstrap tenant_id=null", bootstrap_info.get("tenant_id") is None,
                  f"got {bootstrap_info.get('tenant_id')}")

        # Bootstrap can access public tenant
        s, _ = api_request("GET", f"{base_url}/v1/tenants/public/workflows", token=token)
        check("bootstrap accesses public tenant", s == 200, f"got {s}")

        # =================================================================
        # Test group 3: Global key (tenant_id=NULL, not admin)
        # =================================================================
        print_section_header("Test group 3: Global key")

        s, body = api_request("POST", f"{base_url}/v1/auth/keys", token=token,
                              data={"name": "global-key"})
        check("create global key", s == 201, f"got {s}")
        global_key = body.get("key", "")

        # Global key can access public tenant
        s, _ = api_request("GET", f"{base_url}/v1/tenants/public/workflows", token=global_key)
        check("global key → public tenant", s == 200, f"got {s}")

        # Global key cannot create tenants (not admin)
        s, _ = api_request("POST", f"{base_url}/v1/tenants", token=global_key,
                           data={"schema_name": "denied_tenant", "username": "denied_user"})
        check("global key cannot create tenant", s == 403, f"got {s}")

        # =================================================================
        # Test group 4: Role enforcement
        # =================================================================
        print_section_header("Test group 4: Role enforcement")

        # Create a read-only key
        s, body = api_request("POST", f"{base_url}/v1/auth/keys", token=token,
                              data={"name": "read-only-key", "role": "read"})
        check("create read-only key", s == 201, f"got {s}")
        read_key = body.get("key", "")

        # Read key can list workflows
        s, _ = api_request("GET", f"{base_url}/v1/tenants/public/workflows", token=read_key)
        check("read key lists workflows", s == 200, f"got {s}")

        # Read key cannot execute workflow (needs write)
        s, _ = api_request("POST", f"{base_url}/v1/tenants/public/workflows/nonexistent/execute",
                           token=read_key, data={"context": {}})
        check("read key cannot execute → 403", s == 403, f"got {s}")

        # Create a write key
        s, body = api_request("POST", f"{base_url}/v1/auth/keys", token=token,
                              data={"name": "write-key", "role": "write"})
        check("create write key", s == 201, f"got {s}")
        write_key = body.get("key", "")

        # Write key can execute (even if workflow doesn't exist, should get past auth)
        s, _ = api_request("POST", f"{base_url}/v1/tenants/public/workflows/nonexistent/execute",
                           token=write_key, data={"context": {}})
        check("write key can attempt execute (not 403)", s != 403, f"got {s}")

        # Write key cannot revoke keys (needs admin)
        dummy_id = "00000000-0000-0000-0000-000000000000"
        s, _ = api_request("DELETE", f"{base_url}/v1/auth/keys/{dummy_id}", token=write_key)
        check("write key cannot revoke → 403", s == 403, f"got {s}")

        # -- Privilege escalation prevention (SEC-01) --
        # Read key cannot create keys
        s, _ = api_request("POST", f"{base_url}/v1/auth/keys", token=read_key,
                           data={"name": "escalation-attempt", "role": "admin"})
        check("read key cannot create key → 403", s == 403, f"got {s}")

        # Write key cannot create keys
        s, _ = api_request("POST", f"{base_url}/v1/auth/keys", token=write_key,
                           data={"name": "escalation-attempt", "role": "read"})
        check("write key cannot create key → 403", s == 403, f"got {s}")

        # Write key cannot list keys
        s, _ = api_request("GET", f"{base_url}/v1/auth/keys", token=write_key)
        check("write key cannot list keys → 403", s == 403, f"got {s}")

        # Read key cannot list keys
        s, _ = api_request("GET", f"{base_url}/v1/auth/keys", token=read_key)
        check("read key cannot list keys → 403", s == 403, f"got {s}")

        # -- Tenant enumeration prevention (SEC-04) --
        # Write key cannot list tenants
        s, _ = api_request("GET", f"{base_url}/v1/tenants", token=write_key)
        check("write key cannot list tenants → 403", s == 403, f"got {s}")

        # Read key cannot list tenants
        s, _ = api_request("GET", f"{base_url}/v1/tenants", token=read_key)
        check("read key cannot list tenants → 403", s == 403, f"got {s}")

        # Bootstrap (god mode) CAN still list tenants and create keys
        s, _ = api_request("GET", f"{base_url}/v1/tenants", token=token)
        check("bootstrap can list tenants", s == 200, f"got {s}")

        s, body = api_request("POST", f"{base_url}/v1/auth/keys", token=token,
                              data={"name": "admin-created-key", "role": "admin"})
        check("bootstrap can create admin key", s == 201, f"got {s}")

        # =================================================================
        # Test group 5: Tenant-scoped key isolation
        # =================================================================
        print_section_header("Test group 5: Tenant isolation")

        # Create a tenant-scoped key via bootstrap (admin)
        s, body = api_request("POST", f"{base_url}/v1/tenants/public/keys", token=token,
                              data={"name": "public-tenant-key"})
        check("create tenant-scoped key", s == 201, f"got {s}")
        tenant_key = body.get("key", "")
        if s == 201:
            check("tenant key has tenant_id=public",
                  body.get("tenant_id") == "public",
                  f"got tenant_id={body.get('tenant_id')}")

        # Tenant key can access its own tenant
        if tenant_key:
            s, _ = api_request("GET", f"{base_url}/v1/tenants/public/workflows", token=tenant_key)
            check("tenant key → own tenant", s == 200, f"got {s}")

            # Tenant key cannot access a different tenant
            s, _ = api_request("GET", f"{base_url}/v1/tenants/other_tenant/workflows", token=tenant_key)
            check("tenant key → other tenant → 403", s == 403, f"got {s}")

        # =================================================================
        # Test group 6: Revoked key
        # =================================================================
        print_section_header("Test group 6: Revoked key")

        # Create a key to revoke
        s, body = api_request("POST", f"{base_url}/v1/auth/keys", token=token,
                              data={"name": "to-revoke"})
        revoke_key_token = body.get("key", "")
        revoke_key_id = body.get("id", "")

        # Verify it works before revocation. Use whoami, not the global
        # /auth/keys surface — post-leak-fix that surface is god-only, so a
        # plain key correctly gets 403 there (covered in group 8).
        s, _ = api_request("GET", f"{base_url}/v1/auth/whoami", token=revoke_key_token)
        check("key works before revoke", s == 200, f"got {s}")

        # Revoke it
        s, _ = api_request("DELETE", f"{base_url}/v1/auth/keys/{revoke_key_id}", token=token)
        check("revoke key", s == 200, f"got {s}")

        # Wait for cache to expire (cache TTL is 30s, but clear happens on revoke)
        time.sleep(1)

        # Revoked key → 401
        s, _ = api_request("GET", f"{base_url}/v1/auth/keys", token=revoke_key_token)
        check("revoked key → 401", s == 401, f"got {s}")

        # =================================================================
        # Test group 7: Metrics endpoint
        # =================================================================
        print_section_header("Test group 7: Metrics endpoint (Prometheus)")

        # /metrics is public (no auth required)
        s, _ = api_request("GET", f"{base_url}/metrics")
        check("metrics endpoint returns 200", s == 200, f"got {s}")

        # Verify it returns Prometheus text format with real metrics
        import urllib.request
        req = urllib.request.Request(f"{base_url}/metrics")
        with urllib.request.urlopen(req) as resp:
            metrics_text = resp.read().decode()
        check("metrics contains HELP lines", "# HELP" in metrics_text or "# TYPE" in metrics_text,
              f"no HELP/TYPE lines in metrics output (length={len(metrics_text)})")
        # The pipeline/task counters only appear once work has run; this suite
        # executes no workflows, so assert the server exports its metric
        # namespace rather than a specific counter.
        check("metrics export the cloacina_ namespace",
              "cloacina_" in metrics_text,
              "no cloacina_* metrics found in output")

        # =================================================================
        # Test group 8: 0.9.0 auth — whoami, cross-tenant key-management leak
        #   fix, and self-managed local accounts (CLOACI-I-0118)
        # =================================================================
        print_section_header("Test group 8: 0.9.0 auth (whoami, leak fix, local accounts)")

        # -- whoami reflects the calling key's role --
        s, body = api_request("GET", f"{base_url}/v1/auth/whoami", token=token)
        check("whoami(bootstrap) role=admin, is_admin=true",
              s == 200 and body.get("role") == "admin" and body.get("is_admin") is True,
              f"got {s} {body}")
        s, body = api_request("GET", f"{base_url}/v1/auth/whoami", token=read_key)
        check("whoami(read key) role=read",
              s == 200 and body.get("role") == "read", f"got {s} {body}")
        s, body = api_request("GET", f"{base_url}/v1/auth/whoami", token=write_key)
        check("whoami(write key) role=write",
              s == 200 and body.get("role") == "write", f"got {s} {body}")

        # -- Cross-tenant key-management leak fix (the exact bug 0.9.0 fixed):
        #    a tenant-scoped key must NOT reach the god-only global /auth/keys.
        if tenant_key:
            s, _ = api_request("GET", f"{base_url}/v1/auth/keys", token=tenant_key)
            check("LEAK FIX: tenant key → global GET /auth/keys → 403", s == 403, f"got {s}")
            s, _ = api_request("POST", f"{base_url}/v1/auth/keys", token=tenant_key,
                               data={"name": "leak-attempt"})
            check("LEAK FIX: tenant key → global POST /auth/keys → 403", s == 403, f"got {s}")
            # but it CAN manage its own tenant's keys
            s, _ = api_request("GET", f"{base_url}/v1/tenants/public/keys", token=tenant_key)
            check("tenant key → own tenant keys → 200", s == 200, f"got {s}")

        # -- Self-managed local accounts: create → login (mint) → whoami →
        #    refresh (re-mint, revoke old) → logout (revoke) --
        s, _ = api_request("POST", f"{base_url}/v1/tenants/public/accounts", token=token,
                           data={"username": "ci-user", "password": "ci-pass-12345", "role": "write"})
        check("create local account → 201", s == 201, f"got {s}")

        # wrong password is rejected with no account enumeration
        s, _ = api_request("POST", f"{base_url}/v1/auth/local/login",
                           data={"username": "ci-user", "password": "WRONG", "tenant": "public"})
        check("local login wrong password → 401", s == 401, f"got {s}")

        # correct login mints a scoped key
        s, body = api_request("POST", f"{base_url}/v1/auth/local/login",
                              data={"username": "ci-user", "password": "ci-pass-12345", "tenant": "public"})
        check("local login → 200 + minted key", s == 200 and bool(body.get("key")), f"got {s}")
        minted = body.get("key", "")
        if minted:
            s, wb = api_request("GET", f"{base_url}/v1/auth/whoami", token=minted)
            check("minted key whoami role=write",
                  s == 200 and wb.get("role") == "write", f"got {s} {wb}")
            s, _ = api_request("GET", f"{base_url}/v1/tenants/public/workflows", token=minted)
            check("minted key reads its tenant → 200", s == 200, f"got {s}")

            # refresh re-mints and revokes the old key
            s, rb = api_request("POST", f"{base_url}/v1/auth/refresh", token=minted)
            check("refresh → 200 + fresh key", s == 200 and bool(rb.get("key")), f"got {s}")
            refreshed = rb.get("key", "")
            time.sleep(1)
            s, _ = api_request("GET", f"{base_url}/v1/tenants/public/workflows", token=minted)
            check("old key after refresh → 401", s == 401, f"got {s}")

            # logout revokes the current key
            if refreshed:
                s, _ = api_request("POST", f"{base_url}/v1/auth/logout", token=refreshed)
                check("logout → 200", s == 200, f"got {s}")
                time.sleep(1)
                s, _ = api_request("GET", f"{base_url}/v1/tenants/public/workflows", token=refreshed)
                check("logged-out key → 401", s == 401, f"got {s}")

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
