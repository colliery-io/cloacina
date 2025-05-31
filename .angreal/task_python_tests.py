"""
Python binding test tasks for Cloacina.
"""

import subprocess
import sys
import time
import os
import tempfile
import shutil
from pathlib import Path

import angreal  # type: ignore

# Project root for accessing packages
PROJECT_ROOT = Path(angreal.get_root()).parent
VENV_DIR = PROJECT_ROOT / ".venvs"

# Define command group (import from existing tests module)
from task_tests import tests

def ensure_venv(name):
    """Create or ensure virtual environment exists using uv."""
    venv_path = VENV_DIR / name
    if not venv_path.exists():
        print(f"Creating virtual environment: {name}")
        VENV_DIR.mkdir(exist_ok=True)
        result = subprocess.run([
            "uv", "venv", str(venv_path)
        ], capture_output=True, text=True)
        if result.returncode != 0:
            print(f"Failed to create venv {name}: {result.stderr}")
            return None
        print(f"Created venv: {name}")
    return venv_path

def get_venv_python(venv_path):
    """Get Python executable for venv."""
    if sys.platform == "win32":
        return venv_path / "Scripts" / "python.exe"
    else:
        return venv_path / "bin" / "python"

def run_in_venv(venv_path, cmd, **kwargs):
    """Run command in virtual environment using uv."""
    if cmd[0] == "python":
        # Use uv to run python in the venv
        cmd = ["uv", "run", "--python", str(get_venv_python(venv_path))] + cmd
    elif cmd[0] == "pip":
        # Use uv pip for package management
        cmd = ["uv", "pip"] + cmd[1:] + ["--python", str(get_venv_python(venv_path))]
    elif cmd[0] == "maturin":
        # Use uv to run maturin in the venv
        cmd = ["uv", "run", "--python", str(get_venv_python(venv_path))] + cmd
    return subprocess.run(cmd, **kwargs)


@tests()
@angreal.command(name="python-build", about="build Python backend packages")
@angreal.argument(
    name="backend",
    long="backend",
    help="Build specific backend: postgres, sqlite, or both (default: both)",
    required=False
)
@angreal.argument(
    name="install",
    long="install",
    help="Install packages after building",
    takes_value=False,
    is_flag=True
)
def python_build(backend=None, install=False):
    """Build Python backend packages."""
    
    backends_to_build = []
    if backend == "postgres":
        backends_to_build = ["cloacina-postgres"]
    elif backend == "sqlite":
        backends_to_build = ["cloacina-sqlite"]
    elif backend == "both" or backend is None:
        backends_to_build = ["cloacina-postgres", "cloacina-sqlite"]
    else:
        print(f"Error: Invalid backend '{backend}'. Use 'postgres', 'sqlite', or 'both'.", file=sys.stderr)
        return 1
    
    print(f"\n{'='*60}")
    print(f"Building Python backend packages: {', '.join(backends_to_build)}")
    print(f"{'='*60}")
    
    wheels_built = []
    
    for backend_name in backends_to_build:
        backend_path = PROJECT_ROOT / backend_name
        
        if not backend_path.exists():
            print(f"❌ Backend directory not found: {backend_path}")
            continue
            
        print(f"\n🔨 Building {backend_name}...")
        
        try:
            # Build the wheel
            cmd = ["maturin", "build", "--release"]
            result = subprocess.run(
                cmd,
                cwd=str(backend_path),
                capture_output=True,
                text=True
            )
            
            if result.returncode != 0:
                print(f"❌ Failed to build {backend_name}:")
                print(result.stderr)
                return result.returncode
            else:
                print(f"✅ {backend_name} built successfully")
                
                # Find the built wheel
                target_dir = PROJECT_ROOT / "target" / "wheels"
                wheel_pattern = f"{backend_name.replace('-', '_')}-*.whl"
                wheels = list(target_dir.glob(wheel_pattern))
                
                if wheels:
                    # Get the most recent wheel
                    latest_wheel = max(wheels, key=lambda p: p.stat().st_mtime)
                    wheels_built.append(latest_wheel)
                    print(f"   📦 Wheel: {latest_wheel.name}")
                
        except FileNotFoundError:
            print(f"❌ maturin not found. Please install: pip install maturin")
            return 1
        except Exception as e:
            print(f"❌ Error building {backend_name}: {e}")
            return 1
    
    # Install wheels if requested
    if install and wheels_built:
        print(f"\n📥 Installing {len(wheels_built)} wheels...")
        
        for wheel in wheels_built:
            print(f"   Installing {wheel.name}...")
            try:
                cmd = [
                    sys.executable, "-m", "pip", "install", 
                    "--break-system-packages", "--force-reinstall", 
                    str(wheel)
                ]
                result = subprocess.run(cmd, check=True, capture_output=True, text=True)
                print(f"   ✅ {wheel.name} installed")
            except subprocess.CalledProcessError as e:
                print(f"   ❌ Failed to install {wheel.name}: {e}")
                print(f"   {e.stderr}")
                return e.returncode
    
    print(f"\n🎉 All Python packages built successfully!")
    if install:
        print(f"🎉 All Python packages installed successfully!")
    
    return 0


@tests()
@angreal.command(name="python-clean", about="clean Python build artifacts")
def python_clean():
    """Clean Python build artifacts."""
    
    print(f"\n{'='*60}")
    print("Cleaning Python build artifacts")
    print(f"{'='*60}")
    
    # Clean target directory
    target_dir = PROJECT_ROOT / "target"
    if target_dir.exists():
        wheels_dir = target_dir / "wheels"
        if wheels_dir.exists():
            wheels = list(wheels_dir.glob("cloacina*.whl"))
            for wheel in wheels:
                print(f"🗑️  Removing {wheel.name}")
                wheel.unlink()
    
    # Clean Python cache directories in backend packages
    backend_dirs = ["cloacina-postgres", "cloacina-sqlite", "cloacina-dispatcher"]
    
    for backend_name in backend_dirs:
        backend_path = PROJECT_ROOT / backend_name
        if not backend_path.exists():
            continue
            
        # Remove __pycache__ directories
        for pycache in backend_path.rglob("__pycache__"):
            if pycache.is_dir():
                print(f"🗑️  Removing {pycache}")
                subprocess.run(["rm", "-rf", str(pycache)])
        
        # Remove .pytest_cache
        pytest_cache = backend_path / ".pytest_cache"
        if pytest_cache.exists():
            print(f"🗑️  Removing {pytest_cache}")
            subprocess.run(["rm", "-rf", str(pytest_cache)])
    
    # Clean python-tests cache
    python_tests_dir = PROJECT_ROOT / "python-tests"
    if python_tests_dir.exists():
        pytest_cache = python_tests_dir / ".pytest_cache"
        if pytest_cache.exists():
            print(f"🗑️  Removing {pytest_cache}")
            subprocess.run(["rm", "-rf", str(pytest_cache)])
    
    print("✅ Python build artifacts cleaned")
    return 0


@tests()
@angreal.command(name="python-setup", about="setup Python test environment")
def python_setup():
    """Setup Python test environment with dependencies."""
    
    print(f"\n{'='*60}")
    print("Setting up Python test environment")
    print(f"{'='*60}")
    
    python_tests_dir = PROJECT_ROOT / "python-tests"
    
    if not python_tests_dir.exists():
        print(f"❌ Python tests directory not found: {python_tests_dir}")
        return 1
    
    requirements_file = python_tests_dir / "requirements-test.txt"
    
    if not requirements_file.exists():
        print(f"❌ Requirements file not found: {requirements_file}")
        return 1
    
    print("📦 Installing Python test dependencies...")
    
    try:
        cmd = [
            sys.executable, "-m", "pip", "install", 
            "--break-system-packages", "-r", str(requirements_file)
        ]
        result = subprocess.run(cmd, check=True, capture_output=True, text=True)
        print("✅ Python test dependencies installed")
        
        # Verify pytest is available
        pytest_cmd = [sys.executable, "-m", "pytest", "--version"]
        result = subprocess.run(pytest_cmd, check=True, capture_output=True, text=True)
        print(f"✅ {result.stdout.strip()}")
        
        return 0
        
    except subprocess.CalledProcessError as e:
        print(f"❌ Failed to install dependencies: {e}")
        print(f"   {e.stderr}")
        return e.returncode
    except FileNotFoundError:
        print(f"❌ Python not found at: {sys.executable}")
        return 1


@tests()
@angreal.command(name="python-test", about="run Python binding tests with automatic setup")
@angreal.argument(
    name="backend",
    long="backend", 
    help="Run tests for specific backend: postgres or sqlite (default: both)",
    required=False
)
@angreal.argument(
    name="test_type",
    long="type",
    help="Type of tests to run: unit, integration, dispatcher, all (default: all)",
    required=False
)
@angreal.argument(
    name="build_packages",
    long="build",
    help="Build packages before testing",
    takes_value=False,
    is_flag=True
)
@angreal.argument(
    name="clean_first",
    long="clean",
    help="Clean build artifacts before testing",
    takes_value=False,
    is_flag=True
)
@angreal.argument(
    name="verbose",
    long="verbose",
    help="Verbose test output",
    takes_value=False,
    is_flag=True
)
def python_test(backend=None, test_type=None, build_packages=False, clean_first=False, verbose=False):
    """Run Python binding tests with automatic setup."""
    
    print(f"\n{'='*60}")
    print("Running Python binding tests")
    if backend:
        print(f"Backend: {backend}")
    if test_type:
        print(f"Type: {test_type}")
    print(f"{'='*60}")
    
    # Step 1: Clean if requested
    if clean_first:
        print("\n🧹 Cleaning build artifacts...")
        clean_result = python_clean()
        if clean_result != 0:
            return clean_result
    
    # Step 2: Setup test environment
    print("\n🔧 Setting up test environment...")
    setup_result = python_setup()
    if setup_result != 0:
        return setup_result
    
    # Step 3: Build packages if requested
    if build_packages:
        print("\n🔨 Building packages...")
        build_backend = "both" if backend is None else backend
        build_result = python_build(backend=build_backend, install=True)
        if build_result != 0:
            return build_result
    
    # Step 4: Run tests
    print("\n🧪 Running tests...")
    
    python_tests_dir = PROJECT_ROOT / "python-tests"
    
    if not python_tests_dir.exists():
        print(f"❌ Python tests directory not found: {python_tests_dir}")
        return 1
    
    # Build pytest command
    cmd = [sys.executable, "-m", "pytest", str(python_tests_dir / "tests")]
    
    if verbose:
        cmd.append("-v")
    else:
        cmd.append("-q")
    
    # Add backend-specific markers
    if backend == "postgres":
        cmd.extend(["-m", "postgres"])
    elif backend == "sqlite":
        cmd.extend(["-m", "sqlite"]) 
    elif backend is not None:
        print(f"Error: Invalid backend '{backend}'. Use 'postgres' or 'sqlite'.", file=sys.stderr)
        return 1
    
    # Add test type filters
    if test_type == "unit":
        cmd.extend(["-m", "unit"])
    elif test_type == "integration":
        cmd.extend(["-m", "integration"])
    elif test_type == "dispatcher":
        cmd.extend(["-m", "dispatcher"])
    elif test_type == "all" or test_type is None:
        # Run all test types
        pass
    else:
        print(f"Error: Invalid test type '{test_type}'.", file=sys.stderr)
        return 1
    
    # Add warning suppression for cleaner output
    cmd.extend(["--disable-warnings"])
    
    try:
        start_time = time.time()
        
        result = subprocess.run(cmd, cwd=str(PROJECT_ROOT))
        
        end_time = time.time()
        execution_time = end_time - start_time
        
        print(f"\n{'='*60}")
        if result.returncode == 0:
            print(f"✅ Python binding tests passed in {execution_time:.2f}s")
        else:
            print(f"❌ Python binding tests failed in {execution_time:.2f}s")
        print(f"{'='*60}")
        
        return result.returncode
        
    except subprocess.CalledProcessError as e:
        print(f"❌ Python tests failed with error: {e}", file=sys.stderr)
        return e.returncode
    except FileNotFoundError:
        print("❌ pytest not found. Please run with --build to install dependencies", file=sys.stderr)
        return 1


@tests()
@angreal.command(name="python-full", about="complete Python test suite (clean, build, test)")
@angreal.argument(
    name="backend",
    long="backend", 
    help="Test specific backend: postgres or sqlite (default: both)",
    required=False
)
def python_full(backend=None):
    """Run complete Python test suite with clean, build, and test."""
    
    print(f"\n{'='*60}")
    print("Running COMPLETE Python test suite")
    print("This will: clean → setup → build → install → test")
    if backend:
        print(f"Backend: {backend}")
    print(f"{'='*60}")
    
    # Run the full sequence
    result = python_test(
        backend=backend,
        test_type="all",
        build_packages=True,
        clean_first=True,
        verbose=True
    )
    
    if result == 0:
        print(f"\n🎉 Complete Python test suite PASSED!")
    else:
        print(f"\n💥 Complete Python test suite FAILED!")
    
    return result


@tests()
@angreal.command(name="python-check", about="check Python package status")
def python_check():
    """Check status of Python packages and test environment."""
    
    print(f"\n{'='*60}")
    print("Checking Python package status")
    print(f"{'='*60}")
    
    # Check Python version
    print(f"🐍 Python: {sys.version}")
    print(f"   Executable: {sys.executable}")
    
    # Check if maturin is available
    try:
        result = subprocess.run(["maturin", "--version"], capture_output=True, text=True, check=True)
        print(f"🔨 Maturin: {result.stdout.strip()}")
    except (subprocess.CalledProcessError, FileNotFoundError):
        print("❌ Maturin: Not available (run: pip install maturin)")
    
    # Check if pytest is available
    try:
        result = subprocess.run([sys.executable, "-m", "pytest", "--version"], capture_output=True, text=True, check=True)
        print(f"🧪 Pytest: {result.stdout.strip()}")
    except (subprocess.CalledProcessError, FileNotFoundError):
        print("❌ Pytest: Not available")
    
    # Check backend packages
    backend_status = {}
    for backend in ["cloacina_postgres", "cloacina_sqlite"]:
        try:
            result = subprocess.run([
                sys.executable, "-c", 
                f"import {backend}; print(f'{{backend.__version__}}')"
            ], capture_output=True, text=True, check=True)
            backend_status[backend] = f"✅ {result.stdout.strip()}"
        except subprocess.CalledProcessError:
            backend_status[backend] = "❌ Not installed"
    
    print(f"\n📦 Backend packages:")
    for backend, status in backend_status.items():
        print(f"   {backend}: {status}")
    
    # Check if dispatcher package exists
    dispatcher_dir = PROJECT_ROOT / "cloacina-dispatcher"
    if dispatcher_dir.exists():
        print(f"📦 Dispatcher package: ✅ Directory exists")
    else:
        print(f"📦 Dispatcher package: ❌ Directory not found")
    
    # Check test directory
    test_dir = PROJECT_ROOT / "python-tests"
    if test_dir.exists():
        test_files = list((test_dir / "tests").rglob("test_*.py"))
        print(f"🧪 Test suite: ✅ {len(test_files)} test files found")
    else:
        print(f"🧪 Test suite: ❌ Directory not found")
    
    # Check recent wheels
    wheels_dir = PROJECT_ROOT / "target" / "wheels"
    if wheels_dir.exists():
        wheels = list(wheels_dir.glob("cloacina*.whl"))
        if wheels:
            print(f"\n📦 Recent wheels ({len(wheels)}):")
            for wheel in sorted(wheels, key=lambda p: p.stat().st_mtime, reverse=True)[:5]:
                mtime = time.ctime(wheel.stat().st_mtime)
                print(f"   {wheel.name} ({mtime})")
        else:
            print(f"\n📦 No wheels found in {wheels_dir}")
    
    print(f"\n{'='*60}")
    print("Status check complete")
    print(f"{'='*60}")
    
    return 0


@tests()
@angreal.command(name="python-venv-setup", about="setup isolated virtual environments for testing")
@angreal.argument(
    name="clean",
    long="clean", 
    help="Remove existing venvs before creating new ones",
    takes_value=False,
    is_flag=True
)
def python_venv_setup(clean=False):
    """Setup isolated virtual environments for Python testing."""
    
    print(f"\n{'='*60}")
    print("Setting up Python virtual environments")
    print(f"{'='*60}")
    
    if clean and VENV_DIR.exists():
        print("Cleaning existing virtual environments...")
        shutil.rmtree(VENV_DIR)
        print("Cleaned existing venvs")
    
    # Define test environments
    venv_configs = {
        "postgres": {
            "desc": "PostgreSQL backend only",
            "packages": ["pytest", "pytest-asyncio", "maturin"]
        },
        "sqlite": {
            "desc": "SQLite backend only", 
            "packages": ["pytest", "pytest-asyncio", "maturin"]
        },
        "dispatcher": {
            "desc": "Dispatcher with both backends",
            "packages": ["pytest", "pytest-asyncio", "maturin"]
        },
        "minimal": {
            "desc": "No backends (test error handling)",
            "packages": ["pytest", "pytest-asyncio"]
        }
    }
    
    for venv_name, config in venv_configs.items():
        print(f"\nSetting up venv: {venv_name} ({config['desc']})")
        
        venv_path = ensure_venv(venv_name)
        if not venv_path:
            return 1
        
        # Install base packages
        print(f"   Installing base packages...")
        for package in config["packages"]:
            result = run_in_venv(venv_path, [
                "pip", "install", "-q", package
            ], capture_output=True, text=True)
            if result.returncode != 0:
                print(f"   Failed to install {package}: {result.stderr}")
                return 1
        
        print(f"   {venv_name} venv ready")
    
    print(f"\nAll virtual environments created successfully!")
    print(f"Location: {VENV_DIR}")
    return 0


@tests()
@angreal.command(name="python-venv-test", about="run tests in isolated virtual environments")
@angreal.argument(
    name="backend",
    long="backend",
    help="Test specific backend: postgres, sqlite, dispatcher, minimal, or all (default: all)",
    required=False
)
@angreal.argument(
    name="test_type",
    long="type",
    help="Type of tests to run: unit, integration, dispatcher, all (default: all)",
    required=False
)
@angreal.argument(
    name="verbose",
    long="verbose",
    help="Verbose test output",
    takes_value=False,
    is_flag=True
)
@angreal.argument(
    name="keep_venvs",
    long="keep-venvs",
    help="Keep virtual environments after testing",
    takes_value=False,
    is_flag=True
)
def python_venv_test(backend=None, test_type=None, verbose=False, keep_venvs=False):
    """Run Python tests in isolated virtual environments."""
    
    print(f"\n{'='*60}")
    print("Running Python tests in isolated environments")
    if backend:
        print(f"Backend: {backend}")
    if test_type:
        print(f"Type: {test_type}")
    print(f"{'='*60}")
    
    # Define test matrix
    test_matrix = {
        "postgres": {
            "venv": "postgres",
            "backends_to_install": ["cloacina-postgres"],
            "dispatcher": False
        },
        "sqlite": {
            "venv": "sqlite", 
            "backends_to_install": ["cloacina-sqlite"],
            "dispatcher": False
        },
        "dispatcher": {
            "venv": "dispatcher",
            "backends_to_install": ["cloacina-postgres", "cloacina-sqlite"],
            "dispatcher": True
        },
        "minimal": {
            "venv": "minimal",
            "backends_to_install": [],
            "dispatcher": True
        }
    }
    
    # Determine which environments to test
    if backend == "all" or backend is None:
        envs_to_test = list(test_matrix.keys())
    elif backend in test_matrix:
        envs_to_test = [backend]
    else:
        print(f"Invalid backend '{backend}'. Use: {', '.join(test_matrix.keys())}")
        return 1
    
    overall_success = True
    results = {}
    
    for env_name in envs_to_test:
        config = test_matrix[env_name]
        print(f"\n{'='*40}")
        print(f"Testing environment: {env_name}")
        print(f"{'='*40}")
        
        venv_path = VENV_DIR / config["venv"]
        if not venv_path.exists():
            print(f"Virtual environment {config['venv']} not found. Run 'angreal tests python-venv-setup' first.")
            overall_success = False
            results[env_name] = "VENV_MISSING"
            continue
        
        # Install backends for this environment
        print(f"Installing backends: {config['backends_to_install']}")
        install_success = True
        for backend_name in config["backends_to_install"]:
            # Build the backend
            backend_path = PROJECT_ROOT / backend_name
            if not backend_path.exists():
                print(f"Backend directory not found: {backend_path}")
                overall_success = False
                results[env_name] = "BACKEND_MISSING"
                install_success = False
                break
            
            print(f"   Building {backend_name}...")
            result = run_in_venv(venv_path, [
                "maturin", "develop", "--release"
            ], cwd=str(backend_path), capture_output=True, text=True)
            
            if result.returncode != 0:
                print(f"   Failed to build {backend_name}: {result.stderr}")
                overall_success = False
                results[env_name] = "BUILD_FAILED"
                install_success = False
                break
            print(f"   {backend_name} installed")
        
        if not install_success:
            continue
            
        # Install dispatcher if needed
        if config["dispatcher"]:
            print(f"   Installing dispatcher...")
            dispatcher_path = PROJECT_ROOT / "cloacina-dispatcher"
            result = run_in_venv(venv_path, [
                "pip", "install", "-e", "."
            ], cwd=str(dispatcher_path), capture_output=True, text=True)
            
            if result.returncode != 0:
                print(f"   Failed to install dispatcher: {result.stderr}")
                overall_success = False
                results[env_name] = "DISPATCHER_FAILED"
                continue
            print(f"   Dispatcher installed")
        
        # Run tests
        print(f"Running tests...")
        test_cmd = ["python", "-m", "pytest", str(PROJECT_ROOT / "python-tests" / "tests")]
        
        if verbose:
            test_cmd.append("-v")
        else:
            test_cmd.append("-q")
        
        # Add test type filters
        if test_type == "unit":
            test_cmd.extend(["-m", "unit"])
        elif test_type == "integration":
            test_cmd.extend(["-m", "integration"])
        elif test_type == "dispatcher":
            test_cmd.extend(["-m", "dispatcher"])
        
        # Add backend markers based on environment
        if env_name == "postgres":
            test_cmd.extend(["-m", "postgres"])
        elif env_name == "sqlite":
            test_cmd.extend(["-m", "sqlite"])
        
        test_cmd.extend(["--disable-warnings"])
        
        start_time = time.time()
        result = run_in_venv(venv_path, test_cmd, cwd=str(PROJECT_ROOT))
        end_time = time.time()
        
        if result.returncode == 0:
            print(f"{env_name} tests passed in {end_time - start_time:.2f}s")
            results[env_name] = "PASSED"
        else:
            print(f"{env_name} tests failed in {end_time - start_time:.2f}s")
            overall_success = False
            results[env_name] = "FAILED"
    
    # Summary
    print(f"\n{'='*60}")
    print("Test Results Summary:")
    print(f"{'='*60}")
    
    for env_name, result in results.items():
        print(f"   {env_name}: {result}")
    
    # Cleanup virtual environments unless requested to keep them
    if not keep_venvs:
        print(f"\nCleaning up virtual environments...")
        for env_name in envs_to_test:
            config = test_matrix[env_name]
            venv_path = VENV_DIR / config["venv"]
            if venv_path.exists():
                print(f"   Removing {config['venv']} venv...")
                shutil.rmtree(venv_path)
        print("Virtual environments cleaned up")
    else:
        print(f"\nVirtual environments preserved at: {VENV_DIR}")
    
    if overall_success:
        print(f"\nAll environment tests passed!")
        return 0
    else:
        print(f"\nSome environment tests failed!")
        return 1


@tests()
@angreal.command(name="python-venv-clean", about="clean virtual environments")
def python_venv_clean():
    """Clean virtual environments."""
    
    print(f"\n{'='*60}")
    print("Cleaning Python virtual environments")
    print(f"{'='*60}")
    
    if VENV_DIR.exists():
        print(f"Removing {VENV_DIR}")
        shutil.rmtree(VENV_DIR)
        print("Virtual environments cleaned")
    else:
        print("No virtual environments found")
    
    return 0


@tests()
@angreal.command(name="python-isolated-test", about="run tests with ephemeral isolated environments")
@angreal.argument(
    name="backend",
    long="backend",
    help="Test specific backend: postgres, sqlite, dispatcher, minimal (default: dispatcher)",
    required=False
)
@angreal.argument(
    name="test_type",
    long="type",
    help="Type of tests to run: unit, integration, dispatcher, all (default: unit)",
    required=False
)
@angreal.argument(
    name="verbose",
    long="verbose",
    help="Verbose test output",
    takes_value=False,
    is_flag=True
)
def python_isolated_test(backend=None, test_type=None, verbose=False):
    """Run Python tests in ephemeral isolated environments (auto-cleanup)."""
    
    backend = backend or "dispatcher"
    test_type = test_type or "unit"
    
    print(f"\n{'='*60}")
    print(f"Running {test_type} tests for {backend} backend in ephemeral environment")
    print(f"{'='*60}")
    
    # Define test configurations
    test_configs = {
        "postgres": {
            "backends_to_install": ["cloacina-postgres"],
            "dispatcher": False
        },
        "sqlite": {
            "backends_to_install": ["cloacina-sqlite"],
            "dispatcher": False
        },
        "dispatcher": {
            "backends_to_install": ["cloacina-postgres", "cloacina-sqlite"],
            "dispatcher": True
        },
        "minimal": {
            "backends_to_install": [],
            "dispatcher": True
        }
    }
    
    if backend not in test_configs:
        print(f"Invalid backend '{backend}'. Use: {', '.join(test_configs.keys())}")
        return 1
    
    config = test_configs[backend]
    venv_name = f"temp_{backend}_{int(time.time())}"
    venv_path = None
    
    try:
        # Create temporary environment
        print(f"Creating temporary environment: {venv_name}")
        venv_path = ensure_venv(venv_name)
        if not venv_path:
            return 1
        
        # Install base packages
        print("Installing base packages...")
        base_packages = ["pytest", "pytest-asyncio", "maturin"]
        for package in base_packages:
            result = run_in_venv(venv_path, [
                "pip", "install", "-q", package
            ], capture_output=True, text=True)
            if result.returncode != 0:
                print(f"Failed to install {package}: {result.stderr}")
                return 1
        
        # Install backends
        print(f"Installing backends: {config['backends_to_install']}")
        for backend_name in config["backends_to_install"]:
            backend_path = PROJECT_ROOT / backend_name
            if not backend_path.exists():
                print(f"Backend directory not found: {backend_path}")
                return 1
            
            print(f"   Building {backend_name}...")
            result = run_in_venv(venv_path, [
                "maturin", "develop", "--release"
            ], cwd=str(backend_path), capture_output=True, text=True)
            
            if result.returncode != 0:
                print(f"   Failed to build {backend_name}: {result.stderr}")
                return 1
            print(f"   {backend_name} installed")
        
        # Install dispatcher if needed
        if config["dispatcher"]:
            print("Installing dispatcher...")
            dispatcher_path = PROJECT_ROOT / "cloacina-dispatcher"
            result = run_in_venv(venv_path, [
                "pip", "install", "-e", "."
            ], cwd=str(dispatcher_path), capture_output=True, text=True)
            
            if result.returncode != 0:
                print(f"Failed to install dispatcher: {result.stderr}")
                return 1
            print("Dispatcher installed")
        
        # Run tests
        print(f"Running {test_type} tests...")
        test_cmd = ["python", "-m", "pytest", str(PROJECT_ROOT / "python-tests" / "tests")]
        
        if verbose:
            test_cmd.append("-v")
        else:
            test_cmd.append("-q")
        
        # Add test type filters
        if test_type == "unit":
            test_cmd.extend(["-m", "unit"])
        elif test_type == "integration":
            test_cmd.extend(["-m", "integration"])
        elif test_type == "dispatcher":
            test_cmd.extend(["-m", "dispatcher"])
        
        # Add backend markers
        if backend == "postgres":
            test_cmd.extend(["-m", "postgres"])
        elif backend == "sqlite":
            test_cmd.extend(["-m", "sqlite"])
        
        test_cmd.extend(["--disable-warnings"])
        
        start_time = time.time()
        result = run_in_venv(venv_path, test_cmd, cwd=str(PROJECT_ROOT))
        end_time = time.time()
        
        if result.returncode == 0:
            print(f"\nTests passed in {end_time - start_time:.2f}s")
            return 0
        else:
            print(f"\nTests failed in {end_time - start_time:.2f}s")
            return 1
            
    finally:
        # Always cleanup the temporary environment
        if venv_path and venv_path.exists():
            print(f"\nCleaning up temporary environment: {venv_name}")
            shutil.rmtree(venv_path)
            print("Cleanup complete")