"""
test metrics-format — scrape /metrics from a live cloacina-server and
validate the exposition output with `promtool check metrics` (T-0536).
"""

import subprocess
from pathlib import Path

import angreal  # type: ignore

PROJECT_ROOT = Path(angreal.get_root()).parent

test = angreal.command_group(name="test", about="Cloacina test suites (unit, integration, e2e, soak)")


@test()
@angreal.command(
    name="metrics-format",
    about="scrape /metrics from a live cloacina-server and validate with promtool (T-0536)",
    when_to_use=["CI validation", "after adding or renaming a Prometheus metric", "release gating"],
    when_not_to_use=["normal development iteration where no metrics code changed"],
)
def metrics_format():
    """Boot cloacina-server, scrape /metrics, pipe it through `promtool check metrics`.

    Relies on Postgres being reachable via the local docker-compose stack (same
    fixture as test soak server). Requires `promtool` on PATH — see the
    Prometheus release binaries if not installed.
    """
    import shutil
    import signal
    import time
    import urllib.request
    import urllib.error

    promtool = shutil.which("promtool")
    if not promtool:
        print("promtool not found on PATH.")
        print("  Install from https://prometheus.io/download/ and try again.")
        return 1

    print("Building cloacina-server (debug)...")
    build = subprocess.run(
        ["cargo", "build", "-p", "cloacina-server"],
        cwd=PROJECT_ROOT,
    )
    if build.returncode != 0:
        print("Server build failed.")
        return build.returncode

    server_binary = PROJECT_ROOT / "target" / "debug" / "cloacina-server"
    if not server_binary.exists():
        print(f"Server binary not found at {server_binary}")
        return 1

    compose_file = PROJECT_ROOT / ".angreal" / "docker-compose.yaml"
    print("Starting Postgres...")
    up = subprocess.run(
        ["docker", "compose", "-f", str(compose_file), "up", "-d"],
        cwd=PROJECT_ROOT,
    )
    if up.returncode != 0:
        print("Failed to start Postgres.")
        return up.returncode

    for _ in range(30):
        ready = subprocess.run(
            ["docker", "compose", "-f", str(compose_file), "exec", "-T",
             "postgres", "pg_isready", "-U", "cloacina"],
            capture_output=True,
        )
        if ready.returncode == 0:
            break
        time.sleep(1)
    else:
        subprocess.run(["docker", "compose", "-f", str(compose_file), "down", "-v"])
        print("Postgres never became ready.")
        return 1

    home = PROJECT_ROOT / "target" / "metrics-format-check"
    if home.exists():
        shutil.rmtree(home)
    home.mkdir(parents=True)

    server_proc = None
    try:
        print("Starting server...")
        server_proc = subprocess.Popen(
            [
                str(server_binary),
                "--home", str(home),
                "--database-url", "postgres://cloacina:cloacina@localhost:5432/cloacina",
                "--bind", "127.0.0.1:18181",
                "--bootstrap-key", "clk_metrics_format_check_dummy_key_000000000",
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
        )

        healthy = False
        for _ in range(30):
            time.sleep(1)
            try:
                with urllib.request.urlopen("http://127.0.0.1:18181/health", timeout=1) as resp:
                    if resp.status == 200:
                        healthy = True
                        break
            except Exception:
                continue
        if not healthy:
            print("Server never became healthy.")
            return 1

        try:
            urllib.request.urlopen("http://127.0.0.1:18181/health", timeout=2).read()
        except Exception:
            pass

        print("Scraping /metrics...")
        try:
            with urllib.request.urlopen("http://127.0.0.1:18181/metrics", timeout=5) as resp:
                if resp.status != 200:
                    print(f"/metrics returned {resp.status}")
                    return 1
                body = resp.read()
        except Exception as e:
            print(f"Failed to scrape /metrics: {e}")
            return 1

        print(f"Running promtool on {len(body)} bytes of exposition data...")
        promtool_proc = subprocess.run(
            [promtool, "check", "metrics"],
            input=body,
            capture_output=True,
        )
        if promtool_proc.stdout:
            print(promtool_proc.stdout.decode(errors="replace"))
        if promtool_proc.stderr:
            print(promtool_proc.stderr.decode(errors="replace"))

        if promtool_proc.returncode != 0:
            print("promtool reported exposition format problems.")
            return promtool_proc.returncode

        print("/metrics passes promtool check.")
        return 0
    finally:
        if server_proc is not None and server_proc.poll() is None:
            server_proc.send_signal(signal.SIGINT)
            try:
                server_proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                server_proc.kill()
        subprocess.run(
            ["docker", "compose", "-f", str(compose_file), "down", "-v"],
            capture_output=True,
        )
