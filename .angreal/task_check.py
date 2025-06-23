"""
Code quality checking tasks for Cloacina.
"""

import subprocess
import sys
from pathlib import Path
from typing import List, Dict, Tuple, Optional

import angreal  # type: ignore

# Project root for accessing all crates (one level up from .angreal)
PROJECT_ROOT = Path(angreal.get_root()).parent

# Define command group
check = angreal.command_group(name="check", about="commands for checking code quality")


def find_all_cargo_projects() -> List[Path]:
    """Find all Cargo.toml files, excluding target directories."""
    # Note: cloaca-backend is handled specially with generate/test/cleanup cycle
    excluded_paths = set()
    
    try:
        result = subprocess.run(
            ["find", ".", "-name", "Cargo.toml", "-not", "-path", "./target/*"],
            capture_output=True,
            text=True,
            check=True,
            cwd=PROJECT_ROOT
        )
        
        paths = []
        for line in result.stdout.strip().split('\n'):
            if line:
                cargo_path = Path(line)
                project_path = PROJECT_ROOT / cargo_path.parent
                relative_path = cargo_path.parent.as_posix().lstrip('./')
                
                # Skip excluded paths
                if relative_path in excluded_paths:
                    continue
                    
                paths.append(project_path)
        
        return sorted(paths)
    except subprocess.CalledProcessError as e:
        print(f"Error finding cargo projects: {e}")
        return []


def run_cargo_command(project_path: Path, command: List[str]) -> Tuple[bool, str, str]:
    """Run a cargo command in the given project directory."""
    try:
        result = subprocess.run(
            ["cargo"] + command,
            cwd=project_path,
            capture_output=True,
            text=True,
            timeout=300  # 5 minute timeout
        )
        return result.returncode == 0, result.stdout, result.stderr
    except subprocess.TimeoutExpired:
        return False, "", "Command timed out after 5 minutes"
    except Exception as e:
        return False, "", f"Error running command: {e}"


def extract_warnings(stderr: str) -> List[str]:
    """Extract warning messages from cargo stderr output."""
    warnings = []
    lines = stderr.split('\n')
    
    current_warning = []
    in_warning = False
    
    for line in lines:
        if line.startswith('warning:'):
            if current_warning and in_warning:
                warnings.append('\n'.join(current_warning))
            current_warning = [line]
            in_warning = True
        elif in_warning and (line.startswith('   ') or line.startswith('  ') or line.strip() == ''):
            current_warning.append(line)
        elif in_warning and line.strip():
            # End of current warning
            warnings.append('\n'.join(current_warning))
            current_warning = []
            in_warning = False
    
    if current_warning and in_warning:
        warnings.append('\n'.join(current_warning))
    
    return warnings


def check_cloaca_backend(project_path: Path, show_warnings: bool = True) -> Dict:
    """Special handling for cloaca-backend: generate, test, cleanup."""
    result = {
        "path": "cloaca-backend",
        "check": {"success": False, "warnings": [], "errors": []},
        "build": {"success": False, "warnings": [], "errors": []},
    }
    
    # Test both backends
    backends = ["sqlite", "postgres"]
    all_warnings = []
    all_errors = []
    any_success = False
    
    for backend in backends:
        print(f"    Testing {backend} backend...")
        
        try:
            # Generate backend files
            generate_result = subprocess.run(
                ["angreal", "cloaca", "generate", "--backend", backend],
                cwd=PROJECT_ROOT,
                capture_output=True,
                text=True,
                timeout=120
            )
            
            if generate_result.returncode != 0:
                all_errors.append(f"Generate {backend} failed: {generate_result.stderr.strip()}")
                continue
            
            # Check the generated crate
            success, stdout, stderr = run_cargo_command(project_path, ["check", "--all-targets"])
            warnings = extract_warnings(stderr)
            all_warnings.extend([f"[{backend}] {w}" for w in warnings])
            
            if success:
                any_success = True
                # Try build if check succeeded
                success, stdout, stderr = run_cargo_command(project_path, ["build", "--all-targets"])
                build_warnings = extract_warnings(stderr)
                all_warnings.extend([f"[{backend}] {w}" for w in build_warnings])
                
                if not success and not build_warnings:
                    all_errors.append(f"Build {backend} failed: {stderr.strip()}")
            else:
                if not warnings:
                    all_errors.append(f"Check {backend} failed: {stderr.strip()}")
                    
        except Exception as e:
            all_errors.append(f"Error testing {backend}: {e}")
        
        finally:
            # Cleanup after each backend test
            try:
                subprocess.run(
                    ["angreal", "cloaca", "scrub"],
                    cwd=PROJECT_ROOT,
                    capture_output=True,
                    timeout=60
                )
            except Exception as e:
                print(f"      Warning: Cleanup failed: {e}")
    
    # Aggregate results
    result["check"]["success"] = any_success
    result["build"]["success"] = any_success
    result["check"]["warnings"] = all_warnings
    result["build"]["warnings"] = []  # All warnings are in check
    result["check"]["errors"] = all_errors
    
    return result


def check_single_crate(project_path: Path, show_warnings: bool = True) -> Dict:
    """Check a single crate with cargo check and cargo build."""
    relative_path = str(project_path.relative_to(PROJECT_ROOT))
    result = {
        "path": relative_path,
        "check": {"success": False, "warnings": [], "errors": []},
        "build": {"success": False, "warnings": [], "errors": []},
    }
    
    # Special handling for cloaca-backend: generate, test, cleanup
    if relative_path == "cloaca-backend":
        return check_cloaca_backend(project_path, show_warnings)
    
    # Run cargo check
    success, stdout, stderr = run_cargo_command(project_path, ["check", "--all-targets"])
    result["check"]["success"] = success
    result["check"]["warnings"] = extract_warnings(stderr)
    if not success and not result["check"]["warnings"]:
        result["check"]["errors"] = [stderr.strip()]
    
    # Run cargo build if check succeeded
    if success:
        success, stdout, stderr = run_cargo_command(project_path, ["build", "--all-targets"])
        result["build"]["success"] = success
        result["build"]["warnings"] = extract_warnings(stderr)
        if not success and not result["build"]["warnings"]:
            result["build"]["errors"] = [stderr.strip()]
    else:
        result["build"]["errors"] = ["Skipped due to check failure"]
    
    return result


def print_crate_result(result: Dict, show_warnings: bool = True):
    """Print results for a single crate."""
    path = result["path"]
    check_success = result["check"]["success"]
    build_success = result["build"]["success"]
    
    # Status indicator
    if check_success and build_success:
        status = "✅"
    elif check_success:
        status = "⚠️ "
    else:
        status = "❌"
    
    print(f"{status} {path}")
    
    # Check errors
    if result["check"]["errors"]:
        print("  Check errors:")
        for error in result["check"]["errors"]:
            for line in error.split('\n'):
                if line.strip():
                    print(f"    {line}")
    
    # Build errors  
    if result["build"]["errors"] and result["build"]["errors"][0] != "Skipped due to check failure":
        print("  Build errors:")
        for error in result["build"]["errors"]:
            for line in error.split('\n'):
                if line.strip():
                    print(f"    {line}")
    
    # Warnings
    if show_warnings:
        all_warnings = result["check"]["warnings"] + result["build"]["warnings"]
        if all_warnings:
            print(f"  Warnings ({len(all_warnings)}):")
            for warning in all_warnings:
                for line in warning.split('\n'):
                    if line.strip():
                        print(f"    {line}")


@check()
@angreal.command(
    name="crate",
    about="run cargo check and build on a specific crate",
    when_to_use=["checking single crate during development", "debugging specific compilation issues", "focused testing"],
    when_not_to_use=["checking entire codebase", "CI validation", "pre-commit checks"]
)
@angreal.argument(
    name="path",
    help="path to crate directory (relative to project root)",
    required=True
)
@angreal.argument(
    name="no_warnings",
    long="no-warnings", 
    help="hide warnings, show only errors",
    takes_value=False,
    is_flag=True
)
def crate(path: str, no_warnings=False):
    """Check a specific crate with cargo check and build."""
    crate_path = PROJECT_ROOT / path
    
    if not crate_path.exists():
        print(f"❌ Path does not exist: {path}")
        return 1
    
    cargo_toml = crate_path / "Cargo.toml"
    if not cargo_toml.exists():
        print(f"❌ No Cargo.toml found in: {path}")
        return 1
    
    print(f"🔍 Checking crate: {path}")
    
    result = check_single_crate(crate_path, show_warnings=not no_warnings)
    print_crate_result(result, show_warnings=not no_warnings)
    
    # Return appropriate exit code
    if not result["check"]["success"]:
        return 1
    elif not result["build"]["success"]:
        return 1
    else:
        return 0


@check()
@angreal.command(
    name="all-crates", 
    about="run cargo check and build on all crates (workspace and standalone)",
    when_to_use=["before commits", "CI validation", "comprehensive code quality checks", "finding compilation issues across codebase"],
    when_not_to_use=["quick local testing", "when focusing on specific crate", "during active development of single component"]
)
@angreal.argument(
    name="warnings_only",
    long="warnings-only",
    help="only show crates with warnings, skip successful builds",
    takes_value=False,
    is_flag=True
)
@angreal.argument(
    name="no_warnings", 
    long="no-warnings",
    help="hide warnings, show only errors", 
    takes_value=False,
    is_flag=True
)
def all_crates(warnings_only=False, no_warnings=False):
    """Check all crates systematically."""
    print("🔍 Finding all cargo projects...")
    projects = find_all_cargo_projects()
    
    if not projects:
        print("❌ No cargo projects found")
        return 1
    
    print(f"Found {len(projects)} projects\n")
    
    results = []
    failed_crates = []
    
    for project in projects:
        try:
            result = check_single_crate(project, show_warnings=not no_warnings)
            results.append(result)
            
            # Show individual results based on filtering
            if warnings_only:
                # Only show if there are warnings or errors
                total_warnings = len(result["check"]["warnings"]) + len(result["build"]["warnings"])
                has_errors = result["check"]["errors"] or (result["build"]["errors"] and result["build"]["errors"][0] != "Skipped due to check failure")
                if total_warnings > 0 or has_errors:
                    print_crate_result(result, show_warnings=not no_warnings)
            else:
                # Show all results
                print_crate_result(result, show_warnings=not no_warnings)
            
            # Track failures for exit code
            if not result["check"]["success"] or not result["build"]["success"]:
                failed_crates.append(result["path"])
                
        except KeyboardInterrupt:
            print("\n⚠️  Interrupted by user")
            return 1
        except Exception as e:
            print(f"❌ Error checking {project.relative_to(PROJECT_ROOT)}: {e}")
            failed_crates.append(str(project.relative_to(PROJECT_ROOT)))
    
    # Print summary
    total_projects = len(results)
    check_failures = sum(1 for r in results if not r["check"]["success"])
    build_failures = sum(1 for r in results if not r["build"]["success"])
    total_warnings = sum(len(r["check"]["warnings"]) + len(r["build"]["warnings"]) for r in results)
    
    print(f"\n{'='*60}")
    print(f"SUMMARY")
    print(f"{'='*60}")
    print(f"Total projects: {total_projects}")
    print(f"Check failures: {check_failures}")
    print(f"Build failures: {build_failures}")
    print(f"Total warnings: {total_warnings}")
    
    if failed_crates:
        print(f"\nFailed crates:")
        for crate in failed_crates:
            print(f"  - {crate}")
    
    # Return exit code based on failures
    if check_failures > 0 or build_failures > 0:
        return 1
    else:
        return 0