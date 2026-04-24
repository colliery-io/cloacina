"""
Code coverage measurement using cargo-llvm-cov.

Runs multiple test styles and merges coverage data into a single report.
Supports unit tests, integration tests, cloacinactl tests, and macro tests.
"""

import subprocess
import sys
from pathlib import Path

import angreal  # type: ignore

PROJECT_ROOT = Path(angreal.get_root()).parent


def run_cmd(cmd, description, env=None, check=True, cwd=None):
    """Run a command with status output."""
    print(f"\n--- {description} ---")
    print(f"  $ {' '.join(cmd)}")
    result = subprocess.run(
        cmd,
        cwd=str(cwd or PROJECT_ROOT),
        env=env,
        capture_output=False,
    )
    if check and result.returncode != 0:
        print(f"  FAILED (exit {result.returncode})")
        return False
    return True


test = angreal.command_group(name="test", about="Cloacina test suites (unit, integration, e2e, soak)")


@test()
@angreal.command(
    name="coverage",
    about="merged cargo-llvm-cov coverage across unit, integration, macros, cloacinactl",
    when_to_use=[
        "measuring test coverage across all test styles",
        "finding untested code paths",
        "pre-release coverage audit",
    ],
    when_not_to_use=[
        "quick testing (use angreal test unit instead)",
        "CI coverage (use the nightly workflow)",
    ],
)
@angreal.argument(
    name="html",
    long="html",
    help="Generate HTML report (opens in browser)",
    takes_value=False,
    is_flag=True,
)
@angreal.argument(
    name="json",
    long="json",
    help="Generate JSON summary report",
    takes_value=False,
    is_flag=True,
)
@angreal.argument(
    name="skip_integration",
    long="skip-integration",
    help="Skip integration tests (no Postgres required)",
    takes_value=False,
    is_flag=True,
)
@angreal.argument(
    name="skip_cloacinactl",
    long="skip-cloacinactl",
    help="Skip cloacinactl server/daemon tests",
    takes_value=False,
    is_flag=True,
)
def coverage(html=False, json=False, skip_integration=False, skip_cloacinactl=False):
    """Run all test styles and generate a merged coverage report."""

    # Check cargo-llvm-cov is installed
    result = subprocess.run(
        ["cargo", "+nightly", "llvm-cov", "--version"],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print("ERROR: cargo-llvm-cov not installed.")
        print("Install with: cargo install cargo-llvm-cov")
        sys.exit(1)

    print("=" * 60)
    print("Coverage measurement — cargo-llvm-cov")
    print("=" * 60)

    # 1. Clean previous coverage data
    run_cmd(
        ["cargo", "+nightly", "llvm-cov", "clean", "--workspace"],
        "Cleaning previous coverage data",
    )

    # 2. cloacina unit tests (no DB required)
    run_cmd(
        [
            "cargo", "+nightly", "llvm-cov", "--no-report",
            "-p", "cloacina",
            "--lib",
            "--features", "postgres,sqlite,macros",
        ],
        "cloacina unit tests",
        check=False,
    )

    # 3. cloacina integration tests (requires Postgres)
    if not skip_integration:
        run_cmd(
            [
                "cargo", "+nightly", "llvm-cov", "--no-report",
                "-p", "cloacina",
                "--test", "integration",
                "--features", "postgres,sqlite,macros",
                "--", "--test-threads=1",
            ],
            "cloacina integration tests",
            check=False,
        )
    else:
        print("\n--- Skipping integration tests ---")

    # 4. cloacina macro tests
    run_cmd(
        [
            "cargo", "+nightly", "llvm-cov", "--no-report",
            "-p", "cloacina",
            "--test", "fixtures",
            "--features", "postgres,sqlite,macros",
        ],
        "cloacina fixture tests",
        check=False,
    )

    # 5. cloacinactl tests (handler, daemon, watcher, config)
    if not skip_cloacinactl:
        run_cmd(
            [
                "cargo", "+nightly", "llvm-cov", "--no-report",
                "-p", "cloacinactl",
                "--", "--test-threads=1",
            ],
            "cloacinactl tests (server handlers, daemon, watcher, config)",
            check=False,
        )
    else:
        print("\n--- Skipping cloacinactl tests ---")

    # 6. Generate merged report
    print("\n" + "=" * 60)
    print("Generating merged coverage report")
    print("=" * 60)

    if html:
        html_cmd = [
            "cargo", "+nightly", "llvm-cov", "report",
            "--html",
            "--output-dir", "target/llvm-cov/html",
        ]
        run_cmd(html_cmd, "Generating HTML report")
        html_path = PROJECT_ROOT / "target" / "llvm-cov" / "html" / "index.html"
        print(f"\n  HTML report: {html_path}")
        # Open in browser
        subprocess.run(["open", str(html_path)], check=False)

    if json:
        json_cmd = [
            "cargo", "+nightly", "llvm-cov", "report",
            "--json",
            "--output-path", "target/llvm-cov/coverage.json",
        ]
        run_cmd(json_cmd, "Generating JSON report")
        print(f"\n  JSON report: {PROJECT_ROOT / 'target' / 'llvm-cov' / 'coverage.json'}")

    # Always print text summary
    run_cmd(
        ["cargo", "+nightly", "llvm-cov", "report", "--branch"],
        "Coverage summary",
        check=False,
    )
