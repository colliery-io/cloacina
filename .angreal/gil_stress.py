"""CLOACI-I-0140 — GIL-flake stress harness (Phase 0 discovery tooling).

Runs ONE Python scenario file in a loop until it fails (hang, crash, or
assertion), with the diagnostics the nightly lane lacks:

  * PYTHONFAULTHANDLER=1 → faulthandler prints a PYTHON-level backtrace for
    every thread on SIGSEGV/SIGABRT (the nightly's gdb pass is symbol-less).
  * core dumps enabled (ulimit -c unlimited) and the unstripped
    cloaca.abi3.so left in place for a symbolized lldb/gdb pass afterward.
  * per-iteration wall-clock + outcome log, artifacts kept on first failure.

Usage (from the repo root):
    .angreal/../test-env-unified/bin/python .angreal/gil_stress.py \
        tests/python/test_scenario_30_task_callbacks.py --iters 200

Run it via the venv python AFTER building the wheel once:
    python -c "import sys; sys.path.insert(0, '.angreal'); \
        from test._python_utils import _build_and_install_cloaca_unified as b; \
        b('test-env-unified', 'sqlite,macros')"

Deliberately a plain script (not an @angreal.command yet): promoted to a task
once the repro rate is known (I-0140 Phase 3).
"""

import argparse
import os
import resource
import subprocess
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
VENV = REPO / "test-env-unified"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("scenario", help="path to one tests/python/test_scenario_*.py")
    ap.add_argument("--iters", type=int, default=200)
    ap.add_argument("--timeout", type=int, default=120, help="per-iteration subprocess timeout (s)")
    ap.add_argument(
        "--pytest-timeout",
        default="10",
        help="pytest-timeout seconds; pass 0 to disable (axis-2 signal experiment)",
    )
    ap.add_argument(
        "--timeout-method",
        default="signal",
        choices=["signal", "thread"],
        help="pytest-timeout method (axis 2: does 'thread' change the failure class?)",
    )
    ap.add_argument(
        "--until",
        default="any",
        choices=["any", "hang", "crash"],
        help="stop condition: any failure, only hangs, or only hangs+signals "
        "(rc1 assertion failures are logged and skipped — mechanism already known: sqlite busy)",
    )
    args = ap.parse_args()

    scenario = Path(args.scenario)
    if not scenario.exists():
        print(f"no such scenario: {scenario}", file=sys.stderr)
        return 2

    pytest = VENV / "bin" / "pytest"
    if not pytest.exists():
        print("test-env-unified venv missing — build the wheel first (see module docstring)")
        return 2

    # Core dumps on; keep whatever the OS default location is (macOS: /cores —
    # needs `sudo chmod 1777 /cores` once; linux: per core_pattern).
    resource.setrlimit(resource.RLIMIT_CORE, (resource.RLIM_INFINITY, resource.RLIM_INFINITY))

    env = os.environ.copy()
    env["PYTHONFAULTHANDLER"] = "1"
    # sqlite lane parity: scenarios pick their backend from the usual env/conftest.

    art_dir = REPO / "gil-stress-artifacts"
    art_dir.mkdir(exist_ok=True)

    cmd = [str(pytest), str(scenario), "-v", "-x"]
    if args.pytest_timeout != "0":
        cmd += [f"--timeout={args.pytest_timeout}", f"--timeout-method={args.timeout_method}"]

    def dump_stacks(pid: int, tag: str) -> str:
        """Capture Python (py-spy) + native (lldb) stacks of a live hung child."""
        chunks = []
        for label, dump_cmd in [
            ("py-spy", [str(VENV / "bin" / "py-spy"), "dump", "--pid", str(pid), "--nonblocking"]),
            ("py-spy-native", [str(VENV / "bin" / "py-spy"), "dump", "--pid", str(pid), "--native"]),
            ("lldb", ["lldb", "-p", str(pid), "-b", "-o", "thread backtrace all", "-o", "detach"]),
        ]:
            try:
                r = subprocess.run(dump_cmd, capture_output=True, text=True, timeout=60)
                chunks.append(f"===== {label} =====\n{r.stdout}\n{r.stderr}\n")
            except Exception as e:  # noqa: BLE001 — diagnostics best-effort
                chunks.append(f"===== {label} FAILED: {e} =====\n")
        return "\n".join(chunks)

    outcomes = {"ok": 0}
    for i in range(1, args.iters + 1):
        t0 = time.time()
        stacks = ""
        child = subprocess.Popen(
            cmd, cwd=REPO, env=env,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
        )
        try:
            out, err = child.communicate(timeout=args.timeout)
            dt = time.time() - t0
            if child.returncode == 0:
                outcomes["ok"] += 1
                print(f"[{i}/{args.iters}] ok ({dt:.1f}s)", flush=True)
                continue
            kind = (
                f"signal{-child.returncode}" if child.returncode < 0 else f"rc{child.returncode}"
            )
        except subprocess.TimeoutExpired:
            dt = time.time() - t0
            kind = "hang"
            # THE PAYLOAD: stacks of the still-live hung process.
            print(f"[{i}] hang — dumping stacks of pid {child.pid} …", flush=True)
            stacks = dump_stacks(child.pid, f"iter{i}")
            child.kill()
            out, err = child.communicate()

        outcomes[kind] = outcomes.get(kind, 0) + 1
        log = art_dir / f"iter{i:04d}-{kind}.log"
        out = out or ""
        err = err or ""
        # On a native crash, harvest the macOS crash report (.ips) — it carries
        # the full native stacks of every thread, symbolized (CLOACI-I-0140
        # segfault-class hunt).
        if kind.startswith("signal"):
            time.sleep(3)  # ReportCrash needs a beat to write the file
            reports = sorted(
                Path.home().glob("Library/Logs/DiagnosticReports/[Pp]ython*.ips"),
                key=lambda p: p.stat().st_mtime,
                reverse=True,
            )
            if reports and time.time() - reports[0].stat().st_mtime < 60:
                err += f"\n--- CRASH REPORT ({reports[0].name}) ---\n"
                err += reports[0].read_text(errors="replace")
        log.write_text(
            f"# {scenario.name} iter {i} -> {kind} after {dt:.1f}s\n\n"
            f"--- STDOUT ---\n{out}\n--- STDERR ---\n{err}\n"
            + (f"\n--- STACKS AT HANG ---\n{stacks}\n" if stacks else "")
        )
        print(f"[{i}/{args.iters}] *** {kind} ({dt:.1f}s) -> {log}", flush=True)
        stop = (
            args.until == "any"
            or (args.until == "hang" and kind == "hang")
            or (args.until == "crash" and (kind == "hang" or kind.startswith("signal")))
        )
        if stop:
            print(f"HIT after {i} iterations. outcomes={outcomes}", flush=True)
            return 1

    print(f"clean at {args.iters} iterations. outcomes={outcomes}", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
