"""
test metrics-format — scrape /metrics from a live cloacina-server *and*
cloacina-compiler, validate both exposition outputs with
`promtool check metrics` (T-0536 + T-0591).
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

    print("Building cloacina-server + cloacina-compiler (debug)...")
    build = subprocess.run(
        ["cargo", "build", "-p", "cloacina-server", "-p", "cloacina-compiler"],
        cwd=PROJECT_ROOT,
    )
    if build.returncode != 0:
        print("Build failed.")
        return build.returncode

    server_binary = PROJECT_ROOT / "target" / "debug" / "cloacina-server"
    compiler_binary = PROJECT_ROOT / "target" / "debug" / "cloacina-compiler"
    if not server_binary.exists():
        print(f"Server binary not found at {server_binary}")
        return 1
    if not compiler_binary.exists():
        print(f"Compiler binary not found at {compiler_binary}")
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

    # CLOACI-T-0806: consecutive-success readiness — a single pg_isready pass
    # can land inside the init-restart bounce (exit 56).
    from ._utils import wait_for_postgres_stable

    try:
        wait_for_postgres_stable(compose_file=str(compose_file))
    except RuntimeError:
        subprocess.run(["docker", "compose", "-f", str(compose_file), "down", "-v"])
        print("Postgres never became ready.")
        return 1

    home = PROJECT_ROOT / "target" / "metrics-format-check"
    if home.exists():
        shutil.rmtree(home)
    home.mkdir(parents=True)

    def wait_for_health(url: str, label: str) -> bool:
        for _ in range(30):
            time.sleep(1)
            try:
                with urllib.request.urlopen(url, timeout=1) as resp:
                    if resp.status == 200:
                        return True
            except Exception:
                continue
        print(f"{label} never became healthy at {url}.")
        return False

    def scrape_and_validate(url: str, label: str) -> int:
        print(f"Scraping {label} /metrics from {url}...")
        try:
            with urllib.request.urlopen(url, timeout=5) as resp:
                if resp.status != 200:
                    print(f"{label} /metrics returned {resp.status}")
                    return 1
                body = resp.read()
        except Exception as e:
            print(f"Failed to scrape {label} /metrics: {e}")
            return 1

        print(f"Running promtool on {len(body)} bytes from {label}...")
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
            print(f"{label} /metrics: promtool reported exposition format problems.")
            return promtool_proc.returncode
        print(f"{label} /metrics passes promtool check.")
        return 0

    server_proc = None
    compiler_proc = None
    try:
        print("Starting server...")
        server_proc = subprocess.Popen(
            [
                str(server_binary),
                "--home", str(home),
                "--database-url", "postgres://cloacina:cloacina@localhost:15432/cloacina",
                "--bind", "127.0.0.1:18181",
                "--bootstrap-key", "clk_metrics_format_check_dummy_key_000000000",
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
        )

        if not wait_for_health("http://127.0.0.1:18181/health", "server"):
            return 1

        try:
            urllib.request.urlopen("http://127.0.0.1:18181/health", timeout=2).read()
        except Exception:
            pass

        rc = scrape_and_validate("http://127.0.0.1:18181/metrics", "server")
        if rc != 0:
            return rc

        # Compiler endpoint (CLOACI-T-0591). Shares the postgres fixture
        # with the server; binds on a separate port so both /metrics
        # endpoints are independently scrapeable.
        compiler_home = PROJECT_ROOT / "target" / "metrics-format-check-compiler"
        if compiler_home.exists():
            shutil.rmtree(compiler_home)
        compiler_home.mkdir(parents=True)
        print("Starting compiler...")
        compiler_proc = subprocess.Popen(
            [
                str(compiler_binary),
                "--home", str(compiler_home),
                "--database-url", "postgres://cloacina:cloacina@localhost:15432/cloacina",
                "--bind", "127.0.0.1:18182",
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
        )
        if not wait_for_health("http://127.0.0.1:18182/health", "compiler"):
            return 1

        rc = scrape_and_validate("http://127.0.0.1:18182/metrics", "compiler")
        if rc != 0:
            return rc

        print("Both /metrics endpoints pass promtool check.")
        return 0
    finally:
        for proc in (compiler_proc, server_proc):
            if proc is not None and proc.poll() is None:
                proc.send_signal(signal.SIGINT)
                try:
                    proc.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    proc.kill()
        subprocess.run(
            ["docker", "compose", "-f", str(compose_file), "down", "-v"],
            capture_output=True,
        )
