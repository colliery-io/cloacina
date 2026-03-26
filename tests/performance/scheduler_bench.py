#!/usr/bin/env python3
"""
Cloacina Scheduler Performance Benchmark

Builds real .cloacina workflow packages, deploys them through the daemon or
server, and measures end-to-end execution latency.

Daemon mode:  spawns cloacinactl daemon, registers packages via CLI,
              sets cron schedules, monitors completions.
Server mode:  HTTP client against a running cloacinactl serve instance.

Zero external dependencies -- uses only Python stdlib.

Usage:
    # Daemon mode -- build package + run 60s cron benchmark
    python tests/performance/scheduler_bench.py --mode daemon --build --duration 60s

    # Server mode -- against running server
    python tests/performance/scheduler_bench.py --mode server --base-url http://localhost:8080

    # Specific scenario
    python tests/performance/scheduler_bench.py --mode daemon --build --scenario cron-execution

    # JSON output for CI
    python tests/performance/scheduler_bench.py --mode daemon --build --output json
"""

import argparse
import json
import os
import shutil
import signal
import subprocess
import sys
import tarfile
import threading
import time
import urllib.error
import urllib.request
from pathlib import Path


# ---------------------------------------------------------------------------
# Stats
# ---------------------------------------------------------------------------

class Stats:
    """Collects latency samples and computes percentile statistics."""

    def __init__(self, warmup=3):
        self.samples = []
        self.failures = 0
        self.warmup_remaining = warmup
        self.warmup_discarded = 0
        self.start_time = time.monotonic()

    def record_success(self, latency_s):
        if self.warmup_remaining > 0:
            self.warmup_remaining -= 1
            self.warmup_discarded += 1
            return
        self.samples.append(latency_s)

    def record_failure(self):
        self.failures += 1

    def finalize(self, scenario):
        elapsed = time.monotonic() - self.start_time
        samples = sorted(self.samples)
        n = len(samples)
        successful = n
        total = successful + self.failures

        if n == 0:
            latencies = {
                "p50": 0, "p95": 0, "p99": 0,
                "min": 0, "max": 0, "mean": 0,
            }
        else:
            latencies = {
                "p50": samples[int(n * 0.50)],
                "p95": samples[int(n * 0.95)],
                "p99": samples[min(int(n * 0.99), n - 1)],
                "min": samples[0],
                "max": samples[-1],
                "mean": sum(samples) / n,
            }

        throughput = successful / elapsed if elapsed > 0 else 0

        return {
            "scenario": scenario,
            "duration_s": elapsed,
            "total_operations": total,
            "successful": successful,
            "failed": self.failures,
            "latencies": latencies,
            "throughput": throughput,
            "warmup_discarded": self.warmup_discarded,
        }


# ---------------------------------------------------------------------------
# Reporting
# ---------------------------------------------------------------------------

def format_latency(seconds):
    us = seconds * 1_000_000
    if us < 1000:
        return f"{us:.0f}us"
    elif us < 1_000_000:
        return f"{us / 1000:.1f}ms"
    else:
        return f"{seconds:.2f}s"


def print_table(category, results):
    print(f"=== {category} ===")
    print(f"{'Scenario':<25} {'Ops':>8} {'OK':>8} {'Fail':>8} "
          f"{'p50':>10} {'p95':>10} {'p99':>10} {'ops/s':>10}")
    print("-" * 100)

    for r in results:
        lat = r["latencies"]
        print(f"{r['scenario']:<25} {r['total_operations']:>8} "
              f"{r['successful']:>8} {r['failed']:>8} "
              f"{format_latency(lat['p50']):>10} "
              f"{format_latency(lat['p95']):>10} "
              f"{format_latency(lat['p99']):>10} "
              f"{r['throughput']:>10.1f}")
        if r.get("warmup_discarded"):
            print(f"  warmup_discarded = {r['warmup_discarded']}")
        for k, v in r.get("extra", {}).items():
            print(f"  {k} = {v}")
    print()


def print_json(category, results):
    output = []
    for r in results:
        lat = r["latencies"]
        entry = {
            "category": category,
            "scenario": r["scenario"],
            "duration_ms": int(r["duration_s"] * 1000),
            "total_operations": r["total_operations"],
            "successful": r["successful"],
            "failed": r["failed"],
            "throughput_ops_per_sec": r["throughput"],
            "latency": {
                "p50_us": int(lat["p50"] * 1_000_000),
                "p95_us": int(lat["p95"] * 1_000_000),
                "p99_us": int(lat["p99"] * 1_000_000),
                "min_us": int(lat["min"] * 1_000_000),
                "max_us": int(lat["max"] * 1_000_000),
                "mean_us": int(lat["mean"] * 1_000_000),
            },
        }
        if r.get("extra"):
            entry["extra"] = r["extra"]
        output.append(entry)
    print(json.dumps(output, indent=2))


def print_results(category, results, output_format):
    if output_format == "json":
        print_json(category, results)
    else:
        print_table(category, results)


# ---------------------------------------------------------------------------
# Package Building
# ---------------------------------------------------------------------------

def find_repo_root():
    """Walk up from this script to find the repo root (contains Cargo.toml)."""
    p = Path(__file__).resolve().parent
    while p != p.parent:
        if (p / "Cargo.toml").exists() and (p / "crates").exists():
            return p
        p = p.parent
    raise RuntimeError("Cannot find repo root")


def build_package(repo_root):
    """Build a .cloacina package from examples/features/simple-packaged."""
    project = repo_root / "examples" / "features" / "simple-packaged"
    if not project.exists():
        raise RuntimeError(f"Package project not found: {project}")

    print(f"  Building package from {project}...", end=" ", flush=True)

    result = subprocess.run(
        ["cargo", "build", "--release"],
        cwd=str(project), capture_output=True, text=True, timeout=300,
    )
    if result.returncode != 0:
        print(f"FAILED\n{result.stderr[:500]}")
        return None
    print("OK", end=" ", flush=True)

    # Find the shared library
    target_dir = project / "target" / "release"
    lib_path = None
    for f in os.listdir(target_dir):
        if (f.endswith(".so") or f.endswith(".dylib") or f.endswith(".dll")):
            if "simple_packaged" in f or "packaged" in f:
                lib_path = target_dir / f
                break

    if lib_path is None:
        print(f"FAILED (no shared library in {target_dir})")
        return None

    # Wrap in tar.gz
    package_path = target_dir / "bench-package.cloacina"
    print(f"packaging {lib_path.name}...", end=" ", flush=True)
    with tarfile.open(str(package_path), "w:gz") as tar:
        tar.add(str(lib_path), arcname=lib_path.name)
    print("OK")

    return str(package_path)


def find_cloacinactl(repo_root):
    """Find or build cloacinactl."""
    ctl = repo_root / "target" / "release" / "cloacinactl"
    if ctl.exists():
        return str(ctl)

    ctl = repo_root / "target" / "debug" / "cloacinactl"
    if ctl.exists():
        return str(ctl)

    print("  Building cloacinactl...", end=" ", flush=True)
    result = subprocess.run(
        ["cargo", "build", "-p", "cloacinactl"],
        cwd=str(repo_root), capture_output=True, text=True, timeout=300,
    )
    if result.returncode != 0:
        raise RuntimeError(f"Failed to build cloacinactl: {result.stderr[:300]}")
    print("OK")
    return str(repo_root / "target" / "debug" / "cloacinactl")


# ---------------------------------------------------------------------------
# Daemon Helpers
# ---------------------------------------------------------------------------

def run_cmd(args, timeout=30):
    """Run a command, return (returncode, stdout, stderr)."""
    result = subprocess.run(
        args, capture_output=True, text=True, timeout=timeout,
    )
    return result.returncode, result.stdout, result.stderr


def daemon_register(ctl, package_path, db_path, storage_path):
    """Register a .cloacina package with the daemon."""
    rc, stdout, stderr = run_cmd([
        ctl, "daemon", "register", package_path,
        "--db", str(db_path),
        "--storage", str(storage_path),
    ])
    if rc != 0:
        raise RuntimeError(f"Register failed: {(stderr or stdout)[:300]}")
    return stdout + stderr


def daemon_schedule_set(ctl, workflow, cron_expr, db_path):
    """Set a cron schedule for a workflow."""
    rc, stdout, stderr = run_cmd([
        ctl, "daemon", "schedule", "set", workflow,
        "--cron", cron_expr,
        "--db", str(db_path),
    ])
    if rc != 0:
        raise RuntimeError(f"Schedule set failed: {(stderr or stdout)[:300]}")
    return stdout + stderr


def daemon_status(ctl, db_path, storage_path):
    """Get daemon status output."""
    rc, stdout, stderr = run_cmd([
        ctl, "daemon", "status",
        "--db", str(db_path),
        "--storage", str(storage_path),
    ])
    return stdout + stderr


def count_completed_executions(status_output):
    """Parse the status output and count completed (successful) executions.

    The daemon status output contains lines like:
        Total:      5
        Successful: 4
        Failed:     1
    """
    for line in status_output.splitlines():
        stripped = line.strip()
        if stripped.startswith("Successful:"):
            parts = stripped.split()
            if len(parts) >= 2:
                try:
                    return int(parts[1])
                except ValueError:
                    pass
        # Also try "Total:" as fallback
        if stripped.startswith("Total:"):
            parts = stripped.split()
            if len(parts) >= 2:
                try:
                    return int(parts[1])
                except ValueError:
                    pass
    return 0


def spawn_daemon(ctl, packages_dir, db_path, storage_path, poll_interval=2):
    """Spawn cloacinactl daemon as a subprocess."""
    log_path = db_path.parent / "daemon.log"
    log_file = open(str(log_path), "w")
    env = {**os.environ, "RUST_LOG": "debug"}
    proc = subprocess.Popen(
        [
            ctl, "daemon",
            "--packages", str(packages_dir),
            "--db", str(db_path),
            "--storage", str(storage_path),
            "--poll-interval", str(poll_interval),
        ],
        stdout=log_file,
        stderr=subprocess.STDOUT,
        env=env,
    )
    proc._log_file = log_file
    proc._log_path = log_path
    # Give it time to start
    time.sleep(3)
    if proc.poll() is not None:
        output = proc.stdout.read()
        raise RuntimeError(f"Daemon exited early (code {proc.returncode}): {output[:500]}")
    return proc


def stop_daemon(proc):
    """Gracefully stop a daemon process."""
    if proc and proc.poll() is None:
        proc.send_signal(signal.SIGTERM)
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()
    if hasattr(proc, "_log_file"):
        proc._log_file.close()
        print(f"  Daemon log: {proc._log_path}")


# ---------------------------------------------------------------------------
# Daemon Scenarios
# ---------------------------------------------------------------------------

def scenario_cron_execution(ctl, package_path, workflow, duration_s):
    """
    Register package, set cron schedule, start daemon, measure per-execution latency.
    Each cron fire produces a pipeline completion -- we measure the time between
    completions as a proxy for schedule-to-complete latency.
    """
    test_dir = Path("/tmp/cloacina-bench-cron")
    if test_dir.exists():
        shutil.rmtree(test_dir)
    test_dir.mkdir(parents=True)
    db_path = test_dir / "bench.db"
    storage_path = test_dir / "storage"
    packages_dir = test_dir / "packages"
    storage_path.mkdir()
    packages_dir.mkdir()

    cron_expr = "*/2 * * * * *"  # Every 2 seconds

    daemon = None
    try:
        # Setup
        print("  Registering package...", end=" ", flush=True)
        daemon_register(ctl, package_path, db_path, storage_path)
        print("OK")

        print(f"  Setting cron: {cron_expr}...", end=" ", flush=True)
        daemon_schedule_set(ctl, workflow, cron_expr, db_path)
        print("OK")

        print("  Starting daemon...", end=" ", flush=True)
        daemon = spawn_daemon(ctl, packages_dir, db_path, storage_path)
        print("OK")

        # Measure
        stats = Stats(warmup=3)
        start = time.monotonic()
        prev_completed = 0
        last_completion_time = time.monotonic()

        print(f"  Running cron-execution for {duration_s}s...")
        while time.monotonic() - start < duration_s:
            time.sleep(0.5)
            status = daemon_status(ctl, db_path, storage_path)
            completed = count_completed_executions(status)

            if completed > prev_completed:
                now = time.monotonic()
                # Each new completion is a latency sample
                new_completions = completed - prev_completed
                for _ in range(new_completions):
                    latency = now - last_completion_time
                    stats.record_success(latency / new_completions)
                last_completion_time = now
                prev_completed = completed

        result = stats.finalize("cron-execution")
        result["extra"] = {
            "cron_expression": cron_expr,
            "total_completions": str(prev_completed),
        }
        return result

    finally:
        stop_daemon(daemon)
        shutil.rmtree(test_dir, ignore_errors=True)


def scenario_cron_throughput(ctl, package_path, workflow, duration_s):
    """
    Measure sustained cron throughput over the full duration.
    Reports completions/second.
    """
    test_dir = Path("/tmp/cloacina-bench-throughput")
    if test_dir.exists():
        shutil.rmtree(test_dir)
    test_dir.mkdir(parents=True)
    db_path = test_dir / "bench.db"
    storage_path = test_dir / "storage"
    packages_dir = test_dir / "packages"
    storage_path.mkdir()
    packages_dir.mkdir()

    cron_expr = "* * * * * *"  # Every second

    daemon = None
    try:
        print("  Registering package...", end=" ", flush=True)
        daemon_register(ctl, package_path, db_path, storage_path)
        print("OK")

        print(f"  Setting cron: {cron_expr}...", end=" ", flush=True)
        daemon_schedule_set(ctl, workflow, cron_expr, db_path)
        print("OK")

        print("  Starting daemon...", end=" ", flush=True)
        daemon = spawn_daemon(ctl, packages_dir, db_path, storage_path)
        print("OK")

        # Sample completions at intervals
        stats = Stats(warmup=3)
        start = time.monotonic()
        prev_completed = 0
        sample_interval = 2.0

        print(f"  Running cron-throughput for {duration_s}s...")
        while time.monotonic() - start < duration_s:
            time.sleep(sample_interval)
            status = daemon_status(ctl, db_path, storage_path)
            completed = count_completed_executions(status)

            if completed > prev_completed:
                new = completed - prev_completed
                for _ in range(new):
                    stats.record_success(sample_interval / new)
                prev_completed = completed

        elapsed = time.monotonic() - start
        result = stats.finalize("cron-throughput")
        result["extra"] = {
            "cron_expression": cron_expr,
            "total_completions": str(prev_completed),
            "effective_throughput": f"{prev_completed / elapsed:.2f}/s" if elapsed > 0 else "0",
        }
        return result

    finally:
        stop_daemon(daemon)
        shutil.rmtree(test_dir, ignore_errors=True)


DAEMON_SCENARIOS = {
    "cron-execution": scenario_cron_execution,
    "cron-throughput": scenario_cron_throughput,
}


def run_daemon(args, repo_root, package_path):
    """Run daemon-mode benchmarks."""
    ctl = find_cloacinactl(repo_root)
    duration_s = parse_duration(args.duration)
    workflow = args.workflow

    scenarios = DAEMON_SCENARIOS
    if args.scenario:
        if args.scenario not in scenarios:
            print(f"ERROR: Unknown daemon scenario '{args.scenario}'")
            print(f"Available: {', '.join(scenarios.keys())}")
            return []
        scenarios = {args.scenario: scenarios[args.scenario]}

    results = []
    for name, fn in scenarios.items():
        print(f"\n--- {name} ---")
        try:
            result = fn(ctl, package_path, workflow, duration_s)
            results.append(result)
        except Exception as e:
            print(f"  FAILED: {e}")
            results.append({
                "scenario": name,
                "duration_s": 0, "total_operations": 0,
                "successful": 0, "failed": 1,
                "latencies": {"p50": 0, "p95": 0, "p99": 0, "min": 0, "max": 0, "mean": 0},
                "throughput": 0,
            })

    return results


# ---------------------------------------------------------------------------
# Server Helpers
# ---------------------------------------------------------------------------


def _http_request(url, method="GET", data=None, headers=None, timeout=30):
    """Make an HTTP request, return (status_code, response_body_dict)."""
    hdrs = {"Content-Type": "application/json"}
    if headers:
        hdrs.update(headers)
    body = json.dumps(data).encode() if data else None
    req = urllib.request.Request(url, data=body, headers=hdrs, method=method)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return resp.status, json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        try:
            body = json.loads(e.read().decode())
        except Exception:
            body = {"error": str(e)}
        return e.code, body
    except urllib.error.URLError as e:
        raise ConnectionError(f"Cannot reach server: {e}")


def server_health_check(base_url, api_key=None):
    """Check if the server is reachable."""
    headers = {}
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"
    try:
        status, _ = _http_request(f"{base_url}/health", headers=headers, timeout=5)
        return status == 200
    except Exception:
        return False


def server_execute(base_url, workflow, context=None, api_key=None):
    """POST /executions -- trigger a workflow, return execution_id."""
    headers = {}
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"
    data = {"workflow_name": workflow, "context": context or {}}
    status, body = _http_request(
        f"{base_url}/executions", method="POST", data=data, headers=headers
    )
    if status not in (200, 201, 202):
        raise RuntimeError(f"Execute failed ({status}): {body}")
    return body.get("execution_id")


def server_get_execution(base_url, execution_id, api_key=None):
    """GET /executions/{id} -- return status dict."""
    headers = {}
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"
    status, body = _http_request(
        f"{base_url}/executions/{execution_id}", headers=headers
    )
    if status != 200:
        raise RuntimeError(f"Get execution failed ({status}): {body}")
    return body


def server_wait_for_completion(base_url, execution_id, api_key=None,
                                timeout=60, poll_interval=0.1):
    """Poll until execution reaches a terminal state."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = server_get_execution(base_url, execution_id, api_key)
        status = result.get("status", "").lower()
        if "completed" in status or "failed" in status or "cancelled" in status:
            return result
        time.sleep(poll_interval)
    raise TimeoutError(f"Execution {execution_id} did not complete in {timeout}s")


# ---------------------------------------------------------------------------
# Server Scenarios
# ---------------------------------------------------------------------------

def scenario_server_execute(base_url, api_key, workflow, duration_s):
    """Submit workflows one at a time, measure submit-to-complete latency."""
    stats = Stats(warmup=3)
    start = time.monotonic()

    print(f"  Running execute for {duration_s}s...")
    while time.monotonic() - start < duration_s:
        t0 = time.monotonic()
        try:
            eid = server_execute(base_url, workflow, api_key=api_key)
            server_wait_for_completion(base_url, eid, api_key=api_key)
            stats.record_success(time.monotonic() - t0)
        except Exception as e:
            stats.record_failure()
            print(f"    WARN: {e}")

    return stats.finalize("execute")


def scenario_server_execute_concurrent(base_url, api_key, workflow, duration_s):
    """Submit N workflows concurrently, measure throughput."""
    batch_size = 10
    stats = Stats(warmup=3)
    start = time.monotonic()

    print(f"  Running execute-concurrent (batch={batch_size}) for {duration_s}s...")

    def _run_one(results_list, idx):
        t0 = time.monotonic()
        try:
            eid = server_execute(base_url, workflow, api_key=api_key)
            server_wait_for_completion(base_url, eid, api_key=api_key)
            results_list[idx] = ("ok", time.monotonic() - t0)
        except Exception:
            results_list[idx] = ("fail", 0)

    while time.monotonic() - start < duration_s:
        results_list = [None] * batch_size
        threads = []
        for i in range(batch_size):
            t = threading.Thread(target=_run_one, args=(results_list, i))
            t.start()
            threads.append(t)
        for t in threads:
            t.join(timeout=120)
        for r in results_list:
            if r and r[0] == "ok":
                stats.record_success(r[1])
            else:
                stats.record_failure()

    result = stats.finalize("execute-concurrent")
    result["extra"] = {"batch_size": str(batch_size)}
    return result


SERVER_SCENARIOS = {
    "execute": scenario_server_execute,
    "execute-concurrent": scenario_server_execute_concurrent,
}


def bootstrap_api_key(repo_root, database_url):
    """Create an admin API key via cloacinactl."""
    ctl = find_cloacinactl(repo_root)
    rc, stdout, stderr = run_cmd([
        ctl, "api-key", "create-admin",
        "--database-url", database_url,
    ], timeout=30)
    output = stdout + stderr
    for line in output.splitlines():
        line = line.strip()
        if line.startswith("cloacina_live__") or line.startswith("cloacina_test__"):
            return line
    raise RuntimeError(f"Failed to bootstrap API key: {output[:300]}")


def upload_package_to_server(base_url, api_key, package_path):
    """Upload a .cloacina package to the server via multipart POST."""
    boundary = "----BenchUploadBoundary"
    filename = os.path.basename(package_path)
    with open(package_path, "rb") as f:
        file_data = f.read()

    body = (
        f"--{boundary}\r\n"
        f'Content-Disposition: form-data; name="package"; filename="{filename}"\r\n'
        f"Content-Type: application/octet-stream\r\n"
        f"\r\n"
    ).encode() + file_data + f"\r\n--{boundary}--\r\n".encode()

    headers = {
        "Content-Type": f"multipart/form-data; boundary={boundary}",
        "Authorization": f"Bearer {api_key}",
    }

    req = urllib.request.Request(
        f"{base_url}/workflows/packages",
        data=body, headers=headers, method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            result = json.loads(resp.read().decode())
            return result.get("id")
    except urllib.error.HTTPError as e:
        body = e.read().decode()
        raise RuntimeError(f"Upload failed ({e.code}): {body[:300]}")


def run_server(args, repo_root, package_path):
    """Run server-mode benchmarks."""
    base_url = args.base_url.rstrip("/")
    duration_s = parse_duration(args.duration)
    workflow = args.workflow
    database_url = args.database_url or os.environ.get(
        "CLOACINA_DATABASE_URL",
        "postgresql://cloacina:cloacina@localhost:5432/cloacina"
    )

    print(f"  Checking server at {base_url}...", end=" ", flush=True)
    if not server_health_check(base_url):
        print("FAILED")
        print("ERROR: Server is not reachable. Start it with:")
        print("  angreal services up")
        print("  CLOACINA_DATABASE_URL=postgresql://cloacina:cloacina@localhost:5432/cloacina \\")
        print("    cargo run -p cloacinactl -- serve")
        return []
    print("OK")

    # Bootstrap API key if not provided
    api_key = args.api_key
    if not api_key:
        print("  Bootstrapping admin API key...", end=" ", flush=True)
        try:
            api_key = bootstrap_api_key(repo_root, database_url)
            print("OK")
        except Exception as e:
            print(f"FAILED: {e}")
            return []

    # Upload package
    if package_path:
        print("  Uploading package to server...", end=" ", flush=True)
        try:
            pkg_id = upload_package_to_server(base_url, api_key, package_path)
            print(f"OK (id={pkg_id})")
        except Exception as e:
            print(f"FAILED: {e}")
            return []

    scenarios = SERVER_SCENARIOS
    if args.scenario:
        if args.scenario not in scenarios:
            print(f"ERROR: Unknown server scenario '{args.scenario}'")
            print(f"Available: {', '.join(scenarios.keys())}")
            return []
        scenarios = {args.scenario: scenarios[args.scenario]}

    results = []
    for name, fn in scenarios.items():
        print(f"\n--- {name} ---")
        try:
            result = fn(base_url, api_key, workflow, duration_s)
            results.append(result)
        except Exception as e:
            print(f"  FAILED: {e}")
            results.append({
                "scenario": name,
                "duration_s": 0, "total_operations": 0,
                "successful": 0, "failed": 1,
                "latencies": {"p50": 0, "p95": 0, "p99": 0, "min": 0, "max": 0, "mean": 0},
                "throughput": 0,
            })

    return results


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def parse_duration(s):
    s = s.strip().lower()
    if s.endswith("ms"):
        return float(s[:-2]) / 1000
    if s.endswith("s"):
        return float(s[:-1])
    if s.endswith("m"):
        return float(s[:-1]) * 60
    if s.endswith("h"):
        return float(s[:-1]) * 3600
    return float(s)


def main():
    parser = argparse.ArgumentParser(
        description="Cloacina Scheduler Performance Benchmark"
    )
    parser.add_argument("--mode", choices=["daemon", "server"], default="daemon",
                        help="Benchmark mode")
    parser.add_argument("--scenario", help="Run a specific scenario")
    parser.add_argument("--duration", default="60s",
                        help="Duration per scenario (e.g., 30s, 2m)")
    parser.add_argument("--build", action="store_true",
                        help="Build .cloacina package from examples/features/simple-packaged")
    parser.add_argument("--package", help="Path to pre-built .cloacina package")
    parser.add_argument("--workflow", default="data_processing",
                        help="Workflow name inside the package")
    parser.add_argument("--output", choices=["table", "json"], default="table",
                        help="Output format")
    # Server-mode options
    parser.add_argument("--base-url", default="http://localhost:8080",
                        help="Server base URL (server mode)")
    parser.add_argument("--api-key", help="API key (server mode, auto-bootstrapped if omitted)")
    parser.add_argument("--database-url",
                        help="Database URL for bootstrapping (server mode)")

    args = parser.parse_args()

    print("Cloacina Scheduler Benchmark v2")
    print(f"  Mode:     {args.mode}")
    print(f"  Duration: {args.duration}")
    if args.scenario:
        print(f"  Scenario: {args.scenario}")
    print()

    repo_root = find_repo_root()

    # Build or locate package
    package_path = args.package
    if args.build:
        package_path = build_package(repo_root)
        if not package_path:
            print("ERROR: Failed to build package")
            sys.exit(1)
    elif not package_path and args.mode == "daemon":
        print("ERROR: --build or --package required for daemon mode")
        sys.exit(1)

    if args.mode == "daemon":
        results = run_daemon(args, repo_root, package_path)
        print_results("Daemon", results, args.output)
    elif args.mode == "server":
        results = run_server(args, repo_root, package_path)
        print_results("Server", results, args.output)

    # Exit with error if any failures
    if any(r.get("failed", 0) > 0 and r.get("successful", 0) == 0 for r in results):
        sys.exit(1)


if __name__ == "__main__":
    main()
