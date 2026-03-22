#!/usr/bin/env python3
"""
Cloacina Server Soak Test — Real Workflow Execution

Builds real workflow packages, uploads them to the server, triggers executions,
polls for completion, and validates results. Runs for a configurable duration
under configurable load.

Zero external dependencies — uses only Python stdlib.

Prerequisites:
    - Running cloacinactl serve (with postgres)
    - Rust toolchain (for building workflow packages)
    - Pre-built workflow package OR --build flag to build during setup

Usage:
    # Build package + run 5 minute soak
    python tests/soak/soak_test.py --build --bootstrap --duration 5m

    # Overnight soak with pre-built package
    python tests/soak/soak_test.py --package ./simple-packaged-demo.cloacina \
        --admin-key cloacina_live__xxx --duration 8h --concurrency 5

    # Quick smoke test
    python tests/soak/soak_test.py --build --bootstrap --duration 30s
"""

import argparse
import json
import os
import random
import signal
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.request
from collections import defaultdict
from datetime import datetime
from pathlib import Path


# ---------------------------------------------------------------------------
# Stats
# ---------------------------------------------------------------------------

class Stats:
    def __init__(self):
        self.lock = threading.Lock()
        self.counters = defaultdict(int)
        self.errors = []
        self.latencies = defaultdict(list)
        self.start_time = time.time()

    def increment(self, name, n=1):
        with self.lock:
            self.counters[name] += n

    def record_latency(self, name, seconds):
        with self.lock:
            self.latencies[name].append(seconds)

    def record_error(self, message):
        with self.lock:
            self.errors.append((datetime.now().isoformat(), message))
            self.counters["errors_total"] += 1

    def report(self):
        with self.lock:
            elapsed = time.time() - self.start_time
            print("\n" + "=" * 70)
            print(f"SOAK TEST RESULTS — {elapsed:.0f}s elapsed")
            print("=" * 70)

            print("\nCounters:")
            for name, value in sorted(self.counters.items()):
                print(f"  {name}: {value}")

            print("\nLatencies (p50 / p95 / p99 / max):")
            for name, values in sorted(self.latencies.items()):
                if values:
                    s = sorted(values)
                    n = len(s)
                    p50 = s[int(n * 0.50)]
                    p95 = s[min(int(n * 0.95), n - 1)]
                    p99 = s[min(int(n * 0.99), n - 1)]
                    mx = s[-1]
                    avg = sum(s) / n
                    print(
                        f"  {name}: avg={avg*1000:.0f}ms "
                        f"p50={p50*1000:.0f}ms p95={p95*1000:.0f}ms "
                        f"p99={p99*1000:.0f}ms max={mx*1000:.0f}ms (n={n})"
                    )

            if self.errors:
                print(f"\nErrors ({len(self.errors)} total):")
                for ts, msg in self.errors[:10]:
                    print(f"  [{ts}] {msg}")
                if len(self.errors) > 10:
                    print(f"  ... and {len(self.errors) - 10} more")

            total_triggered = self.counters.get("executions_triggered", 0)
            completed = self.counters.get("executions_completed", 0)
            failed = self.counters.get("executions_failed", 0)
            errors = self.counters.get("errors_total", 0)

            print("\nSummary:")
            print(f"  Executions triggered: {total_triggered}")
            print(f"  Completed:            {completed}")
            print(f"  Failed:               {failed}")
            print(f"  Errors:               {errors}")
            if total_triggered > 0:
                print(f"  Success rate:         {completed/total_triggered*100:.1f}%")
            rate = total_triggered / elapsed if elapsed > 0 else 0
            print(f"  Throughput:           {rate:.2f}/s")

            passed = errors == 0 and completed > 0
            print(f"\n{'PASS' if passed else 'FAIL'}")
            print("=" * 70)
            return passed


# ---------------------------------------------------------------------------
# HTTP client
# ---------------------------------------------------------------------------

class Response:
    def __init__(self, status_code, text, headers):
        self.status_code = status_code
        self.text = text
        self.headers = headers

    def json(self):
        return json.loads(self.text)


class HttpClient:
    def __init__(self, base_url, api_key=None, stats=None):
        self.base_url = base_url.rstrip("/")
        self.api_key = api_key
        self.stats = stats

    def _request(self, method, path, body=None, raw_data=None, content_type=None):
        url = f"{self.base_url}{path}"
        label = f"{method} {path.split('{')[0]}"
        start = time.time()

        headers = {}
        if self.api_key:
            headers["Authorization"] = f"Bearer {self.api_key}"

        data = None
        if body is not None:
            headers["Content-Type"] = "application/json"
            data = json.dumps(body).encode("utf-8")
        elif raw_data is not None:
            if content_type:
                headers["Content-Type"] = content_type
            data = raw_data

        req = urllib.request.Request(url, data=data, headers=headers, method=method)
        try:
            with urllib.request.urlopen(req, timeout=60) as resp:
                elapsed = time.time() - start
                if self.stats:
                    self.stats.increment("requests_total")
                    self.stats.record_latency(label, elapsed)
                return Response(resp.status, resp.read().decode("utf-8"), dict(resp.headers))
        except urllib.error.HTTPError as e:
            elapsed = time.time() - start
            resp_body = ""
            try:
                resp_body = e.read().decode("utf-8") if e.fp else ""
            except Exception:
                pass
            if self.stats:
                self.stats.increment("requests_total")
                self.stats.record_latency(label, elapsed)
                if e.code >= 500:
                    self.stats.record_error(f"{label} -> {e.code}: {resp_body[:200]}")
            return Response(e.code, resp_body, {})
        except Exception as e:
            elapsed = time.time() - start
            if self.stats:
                self.stats.record_error(f"{label} -> {type(e).__name__}: {e}")
            return None

    def get(self, path):
        return self._request("GET", path)

    def post(self, path, body=None):
        return self._request("POST", path, body=body)

    def post_file(self, path, file_path):
        """Upload a file via multipart form."""
        boundary = f"----SoakTest{random.randint(100000,999999)}"
        filename = os.path.basename(file_path)
        with open(file_path, "rb") as f:
            file_data = f.read()

        body = (
            f"--{boundary}\r\n"
            f'Content-Disposition: form-data; name="package"; filename="{filename}"\r\n'
            f"Content-Type: application/octet-stream\r\n\r\n"
        ).encode("utf-8") + file_data + f"\r\n--{boundary}--\r\n".encode("utf-8")

        return self._request(
            "POST", path,
            raw_data=body,
            content_type=f"multipart/form-data; boundary={boundary}",
        )

    def delete(self, path):
        return self._request("DELETE", path)


# ---------------------------------------------------------------------------
# Setup: build package, bootstrap auth, upload
# ---------------------------------------------------------------------------

def build_package(project_dir):
    """Build a .cloacina package (tar.gz wrapping cdylib) from a Rust workflow project."""
    print(f"  Building {project_dir}...", end=" ", flush=True)

    # Build the cdylib
    result = subprocess.run(
        ["cargo", "build", "--release"],
        cwd=project_dir, capture_output=True, text=True, timeout=300,
    )
    if result.returncode != 0:
        print(f"FAILED\n{result.stderr[:500]}")
        return None
    print("OK")

    # Find the .so/.dylib
    import tarfile
    target_dir = os.path.join(project_dir, "target", "release")
    lib_path = None
    for f in os.listdir(target_dir):
        if f.endswith(".so") or f.endswith(".dylib") or f.endswith(".dll"):
            if "simple_packaged" in f or "packaged_workflow" in f:
                lib_path = os.path.join(target_dir, f)
                break

    if lib_path is None:
        print(f"  WARNING: No shared library found in {target_dir}")
        return None

    # Wrap the shared library in a tar.gz to create a .cloacina package
    package_path = os.path.join(target_dir, "simple-packaged-demo.cloacina")
    print(f"  Packaging {os.path.basename(lib_path)} -> {os.path.basename(package_path)}...", end=" ", flush=True)
    with tarfile.open(package_path, "w:gz") as tar:
        tar.add(lib_path, arcname=os.path.basename(lib_path))
    print("OK")

    return package_path


def bootstrap_admin_key(cloacinactl, db_url):
    """Create an admin API key via CLI."""
    result = subprocess.run(
        [cloacinactl, "api-key", "create-admin",
         "--name", "soak-test",
         "--database-url", db_url],
        capture_output=True, text=True, timeout=30,
    )
    if result.returncode != 0:
        return None
    for line in result.stdout.splitlines():
        line = line.strip()
        if line.startswith("cloacina_"):
            return line
    return None


# ---------------------------------------------------------------------------
# Soak operations
# ---------------------------------------------------------------------------

def trigger_and_poll(client, workflow_name, stats, poll_timeout=30):
    """Trigger a workflow execution and poll until complete or timeout."""
    # Trigger
    start = time.time()
    resp = client.post("/executions", {
        "workflow_name": workflow_name,
        "context": {"soak": True, "ts": datetime.now().isoformat()},
    })

    if not resp or resp.status_code not in (200, 201, 202):
        status = resp.status_code if resp else "no_response"
        stats.increment(f"trigger_{status}")
        if resp and resp.status_code >= 500:
            stats.record_error(f"trigger failed: {status}")
        return

    stats.increment("executions_triggered")
    body = resp.json()
    eid = body.get("execution_id")
    if not eid:
        stats.record_error("trigger returned no execution_id")
        return

    # Poll for completion
    poll_start = time.time()
    final_status = None
    while time.time() - poll_start < poll_timeout:
        time.sleep(0.5)
        resp = client.get(f"/executions/{eid}")
        if not resp:
            continue
        if resp.status_code == 200:
            body = resp.json()
            status = body.get("status", "")
            if "Completed" in status:
                elapsed = time.time() - start
                stats.increment("executions_completed")
                stats.record_latency("execution_e2e", elapsed)
                final_status = "Completed"
                break
            elif "Failed" in status:
                elapsed = time.time() - start
                stats.increment("executions_failed")
                stats.record_latency("execution_e2e", elapsed)
                error = body.get("error_message", "unknown")
                stats.record_error(f"execution {eid} failed: {error}")
                final_status = "Failed"
                break

    if final_status is None:
        stats.increment("executions_timed_out")
        stats.record_error(f"execution {eid} timed out after {poll_timeout}s")


def health_check(client, stats):
    """Quick health check."""
    resp = client.get("/health")
    if resp and resp.status_code == 200:
        stats.increment("health_ok")
    else:
        stats.record_error(f"health failed: {resp.status_code if resp else 'none'}")


def metrics_check(client, stats):
    """Check metrics endpoint."""
    resp = client.get("/metrics")
    if resp and resp.status_code == 200:
        stats.increment("metrics_ok")


# ---------------------------------------------------------------------------
# Load profiles
# ---------------------------------------------------------------------------

def light_load(client, delay, stats, workflows):
    """Light: one workflow at a time, full trigger-poll cycle."""
    wf = random.choice(workflows)
    trigger_and_poll(client, wf, stats)
    time.sleep(delay)


def medium_load(client, delay, stats, workflows):
    """Medium: mix of workflow executions and health checks."""
    r = random.random()
    if r < 0.7:
        wf = random.choice(workflows)
        trigger_and_poll(client, wf, stats)
    elif r < 0.85:
        health_check(client, stats)
    else:
        metrics_check(client, stats)
    time.sleep(delay)


def heavy_load(client, delay, stats, workflows):
    """Heavy: rapid fire-and-forget triggers with periodic polling."""
    for _ in range(3):
        wf = random.choice(workflows)
        trigger_and_poll(client, wf, stats, poll_timeout=60)
    time.sleep(delay)


PROFILES = {"light": light_load, "medium": medium_load, "heavy": heavy_load}


# ---------------------------------------------------------------------------
# Worker
# ---------------------------------------------------------------------------

def worker(base_url, api_key, profile_fn, delay, stop_event, stats, workflows, worker_id):
    client = HttpClient(base_url, api_key, stats)
    while not stop_event.is_set():
        try:
            profile_fn(client, delay, stats, workflows)
        except Exception as e:
            stats.record_error(f"worker-{worker_id}: {e}")
            time.sleep(1)


# ---------------------------------------------------------------------------
# Duration parsing
# ---------------------------------------------------------------------------

def parse_duration(s):
    s = s.strip().lower()
    if s.endswith("s"):
        return int(s[:-1])
    if s.endswith("m"):
        return int(s[:-1]) * 60
    if s.endswith("h"):
        return int(s[:-1]) * 3600
    if s.endswith("d"):
        return int(s[:-1]) * 86400
    return int(s)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="Cloacina Soak Test — Real Workflow Execution")
    parser.add_argument("--url", default="http://localhost:8080", help="Server URL")
    parser.add_argument("--duration", default="5m", help="Duration (30s, 5m, 1h, 8h)")
    parser.add_argument("--concurrency", type=int, default=1, help="Worker threads")
    parser.add_argument("--delay", type=float, default=0.5, help="Delay between ops per worker")
    parser.add_argument("--profile", choices=PROFILES.keys(), default="medium")

    # Package
    parser.add_argument("--build", action="store_true",
                        help="Build workflow package from examples/features/simple-packaged")
    parser.add_argument("--package", help="Path to pre-built .cloacina or .so/.dylib package")
    parser.add_argument("--workflow", default="data_processing",
                        help="Workflow name to execute")

    # Auth
    parser.add_argument("--admin-key", help="Pre-created admin API key")
    parser.add_argument("--bootstrap", action="store_true",
                        help="Create admin key via cloacinactl CLI")
    parser.add_argument("--cloacinactl", default="./target/debug/cloacinactl")
    parser.add_argument("--database-url",
                        default=os.environ.get("CLOACINA_DATABASE_URL",
                                               os.environ.get("DATABASE_URL", "")))
    args = parser.parse_args()

    duration_secs = parse_duration(args.duration)
    profile_fn = PROFILES[args.profile]
    stats = Stats()
    workflows = [args.workflow]

    print("Cloacina Soak Test — Real Workflow Execution")
    print(f"  URL:         {args.url}")
    print(f"  Duration:    {args.duration} ({duration_secs}s)")
    print(f"  Concurrency: {args.concurrency}")
    print(f"  Profile:     {args.profile}")
    print(f"  Workflows:   {', '.join(workflows)}")
    print()

    # --- Pre-flight ---
    print("Pre-flight: checking /health...", end=" ", flush=True)
    client = HttpClient(args.url)
    resp = client.get("/health")
    if not resp or resp.status_code != 200:
        print(f"FAILED — server not reachable at {args.url}")
        sys.exit(1)
    info = resp.json()
    print(f"OK (v{info.get('version','?')}, mode={info.get('mode','?')})")

    # --- Auth ---
    api_key = args.admin_key
    if args.bootstrap:
        if not args.database_url:
            print("ERROR: --database-url required for --bootstrap")
            sys.exit(1)
        print("Bootstrapping admin key...", end=" ", flush=True)
        api_key = bootstrap_admin_key(args.cloacinactl, args.database_url)
        if not api_key:
            print("FAILED")
            sys.exit(1)
        print("OK")

    if not api_key:
        print("ERROR: API key required. Use --admin-key or --bootstrap")
        sys.exit(1)

    authed = HttpClient(args.url, api_key, stats)

    # --- Build / upload package ---
    package_path = args.package
    if args.build:
        repo_root = Path(__file__).parent.parent.parent
        project = repo_root / "examples" / "features" / "simple-packaged"
        package_path = build_package(str(project))
        if not package_path:
            print("ERROR: Failed to build workflow package")
            sys.exit(1)

    # Upload Rust package
    if package_path:
        print(f"Uploading Rust package {os.path.basename(package_path)}...", end=" ", flush=True)
        resp = authed.post_file("/workflows/packages", package_path)
        if resp and resp.status_code in (200, 201):
            print(f"OK ({resp.json()})")
        else:
            status = resp.status_code if resp else "no response"
            text = resp.text[:200] if resp else ""
            print(f"WARNING: upload returned {status}: {text}")
            print("  (Continuing — workflow may already be registered)")

    # Upload Python package if available (containerized soak builds both)
    python_package = os.path.join(os.path.dirname(package_path or ""), "python-workflow.cloacina")
    if not os.path.isfile(python_package):
        # Also check in the soak test's own directory
        python_package = os.path.join(os.path.dirname(__file__) or ".", "python-workflow.cloacina")
    if not os.path.isfile(python_package):
        python_package = "/opt/soak/python-workflow.cloacina"

    if os.path.isfile(python_package):
        print(f"Uploading Python package {os.path.basename(python_package)}...", end=" ", flush=True)
        resp = authed.post_file("/workflows/packages", python_package)
        if resp and resp.status_code in (200, 201):
            print(f"OK ({resp.json()})")
            # Add the Python workflow to the execution list so soak exercises both
            # The Python workflow name comes from the manifest package name
            python_wf_name = "data-pipeline-example"
            if python_wf_name not in workflows:
                workflows.append(python_wf_name)
                print(f"  Added '{python_wf_name}' to soak workflow list")
        else:
            status = resp.status_code if resp else "no response"
            text = resp.text[:200] if resp else ""
            print(f"WARNING: Python package upload returned {status}: {text}")
            print("  (Continuing — Python workflows may not be supported in this mode)")

    # --- Wait for reconciler to load the workflow into global registry ---
    if package_path:
        print("Waiting for registry reconciler to load workflow (10s)...", end=" ", flush=True)
        time.sleep(10)
        print("OK")

    # --- Smoke test: on-demand execution ---
    print("Smoke test: triggering one execution...", end=" ", flush=True)
    smoke_stats = Stats()
    smoke_client = HttpClient(args.url, api_key, smoke_stats)
    trigger_and_poll(smoke_client, args.workflow, smoke_stats, poll_timeout=30)
    triggered = smoke_stats.counters.get("executions_triggered", 0)
    completed = smoke_stats.counters.get("executions_completed", 0)
    if completed > 0:
        print("OK (completed)")
    elif triggered > 0:
        print("WARNING: triggered but not completed (may need workflow registered)")
    else:
        print("WARNING: could not trigger execution")

    # --- Smoke test: schedule CRUD ---
    print("Smoke test: schedule API...", end=" ", flush=True)
    schedule_ok = True

    # Create schedule
    sched_resp = authed.post(
        f"/workflows/{args.workflow}/schedules",
        {"cron": "*/30 * * * * *", "timezone": "UTC"},
    )
    if not sched_resp or sched_resp.status_code != 201:
        status = sched_resp.status_code if sched_resp else "no response"
        print(f"FAILED (create: {status})")
        schedule_ok = False

    schedule_id = None
    if schedule_ok:
        schedule_id = sched_resp.json().get("id")

        # List schedules
        list_resp = authed.get(f"/workflows/{args.workflow}/schedules")
        if not list_resp or list_resp.status_code != 200:
            print(f"FAILED (list: {list_resp.status_code if list_resp else 'no response'})")
            schedule_ok = False
        else:
            schedules = list_resp.json()
            if not any(s.get("id") == schedule_id for s in schedules):
                print("FAILED (created schedule not in list)")
                schedule_ok = False

    if schedule_ok and schedule_id:
        # Get single schedule
        get_resp = authed.get(f"/workflows/schedules/{schedule_id}")
        if not get_resp or get_resp.status_code != 200:
            print(f"FAILED (get: {get_resp.status_code if get_resp else 'no response'})")
            schedule_ok = False

    if schedule_ok and schedule_id:
        # Wait for at least one scheduled execution
        print("waiting for scheduled run...", end=" ", flush=True)
        scheduled_ran = False
        for _ in range(40):
            time.sleep(1)
            hist_resp = authed.get(f"/workflows/schedules/{schedule_id}/history")
            if hist_resp and hist_resp.status_code == 200:
                history = hist_resp.json()
                if len(history) > 0:
                    scheduled_ran = True
                    break
        if not scheduled_ran:
            print("FAILED (no scheduled execution after 40s)")
            schedule_ok = False

    if schedule_ok and schedule_id:
        # Delete schedule
        del_resp = authed.delete(f"/workflows/schedules/{schedule_id}")
        if not del_resp or del_resp.status_code != 204:
            print(f"FAILED (delete: {del_resp.status_code if del_resp else 'no response'})")
            schedule_ok = False

    if schedule_ok:
        print("OK (create/list/get/history/delete)")
    print()

    # --- Run ---
    stop_event = threading.Event()
    signal.signal(signal.SIGINT, lambda s, f: stop_event.set())
    signal.signal(signal.SIGTERM, lambda s, f: stop_event.set())

    threads = []
    for i in range(args.concurrency):
        t = threading.Thread(
            target=worker,
            args=(args.url, api_key, profile_fn, args.delay, stop_event, stats, workflows, i),
            daemon=True,
        )
        t.start()
        threads.append(t)

    print(f"Started {args.concurrency} worker(s). Running for {args.duration}...")
    print("Press Ctrl+C to stop early.\n")

    end_time = time.time() + duration_secs
    last_report = time.time()
    report_interval = min(60, max(5, duration_secs / 10))

    while not stop_event.is_set() and time.time() < end_time:
        stop_event.wait(timeout=1.0)
        if time.time() - last_report >= report_interval:
            elapsed = time.time() - stats.start_time
            remaining = max(0, end_time - time.time())
            total = stats.counters.get("executions_triggered", 0)
            done = stats.counters.get("executions_completed", 0)
            errs = stats.counters.get("errors_total", 0)
            rate = total / elapsed if elapsed > 0 else 0
            print(
                f"  [{elapsed:.0f}s / {elapsed+remaining:.0f}s] "
                f"{total} triggered, {done} completed ({rate:.1f}/s), {errs} errors"
            )
            last_report = time.time()

    stop_event.set()
    for t in threads:
        t.join(timeout=10)

    passed = stats.report()
    sys.exit(0 if passed else 1)


if __name__ == "__main__":
    main()
