"""
Python binding test tasks for Cloaca.

Uses composable functions from file_generation, build_operations, and file_operations
for clean, testable command implementations.
"""

import sys
from pathlib import Path

import angreal  # type: ignore
from angreal.integrations.venv import VirtualEnv# type: ignore

# Import only what we need

from angreal import render_template# type: ignore

import re
import subprocess
import shutil
import time

# Import docker utilities for postgres backend
from utils import docker_up, docker_down

# Define command group
cloaca = angreal.command_group(name="cloaca", about="commands for Python binding tests")


class FileOperationError(Exception):
    """Raised when file operations fail."""
    pass


def write_file_safe(path: Path, content: str, encoding: str = "utf-8", backup: bool = False):
    """Safely write a file with error handling.
    
    Args:
        path: File path to write
        content: Content to write
        encoding: File encoding
        backup: Whether to backup existing file
        
    Returns:
        Path to backup file if backup=True and file existed, None otherwise
        
    Raises:
        FileOperationError: If file cannot be written
    """
    try:
        backup_path = None
        
        if backup and path.exists():
            backup_path = path.with_suffix(path.suffix + ".backup")
            shutil.copy2(path, backup_path)
        
        # Ensure parent directory exists
        path.parent.mkdir(parents=True, exist_ok=True)
        
        path.write_text(content, encoding=encoding)
        return backup_path
        
    except (OSError, UnicodeEncodeError) as e:
        raise FileOperationError(f"Failed to write file {path}: {e}")

def get_workspace_version() -> str:
    """Extract version from workspace Cargo.toml.
    
    Returns:
        Version string from workspace configuration
        
    Raises:
        ValueError: If version cannot be found in workspace Cargo.toml
    """
    project_root = Path(angreal.get_root()).parent
    cargo_toml = project_root / "Cargo.toml"
    
    if not cargo_toml.exists():
        raise FileNotFoundError(f"Workspace Cargo.toml not found at {cargo_toml}")
    
    content = cargo_toml.read_text()
    match = re.search(r'\[workspace\.package\].*?version\s*=\s*"([^"]+)"', content, re.DOTALL)
    
    if match:
        return match.group(1)
    
    raise ValueError("Could not find version in workspace Cargo.toml")


def _build_and_install_cloaca_backend(backend_name, venv_name):
    """Build cloaca backend wheel and install it in a test environment with dispatcher.
    
    Returns the VirtualEnv object and paths to executables.
    """
    project_root = Path(angreal.get_root()).parent
    venv_path = project_root / venv_name
    
    # Step 1: Generate files
    print("Step 1: Generating files...")
    result = generate(backend_name)
    if result != 0:
        raise Exception(f"Failed to generate files for {backend_name}")

    # Step 1.5: Setup Docker for postgres backend
    if backend_name == "postgres":
        print("Step 1.5: Setting up Docker services for postgres...")
        # Start Docker services for postgres backend
        exit_code = docker_up()
        if exit_code != 0:
            raise Exception("Failed to start Docker services")
        
        # Wait for services to be ready
        print("Waiting for services to be ready...")
        time.sleep(10)

    # Step 2: Create test environment
    print("Step 2: Creating test environment...")
    venv = VirtualEnv(path=str(venv_path), now=True)
    
    python_exe = venv.path / "bin" / "python"
    pip_exe = venv.path / "bin" / "pip3"
    
    # Install pip and dependencies
    print("Installing dependencies...")
    subprocess.run([str(python_exe), "-m", "ensurepip"], check=True, capture_output=True)
    subprocess.run([str(pip_exe), "install", "maturin", "pytest", "pytest-asyncio"], check=True, capture_output=True)
    
    # Install dispatcher package
    print("Installing dispatcher package...")
    subprocess.run([str(pip_exe), "install", "-e", str(project_root / "cloaca")], check=True, capture_output=True)
    
    # Step 3: Build and install backend wheel
    print(f"Step 3: Building and installing {backend_name} wheel...")
    backend_dir = project_root / "cloaca-backend"
    
    # Clean existing extensions
    for pattern in ["*.so", "*.pyd"]:
        for artifact in backend_dir.rglob(pattern):
            artifact.unlink()
    
    # Build wheel
    maturin_exe = venv.path / "bin" / "maturin"
    maturin_cmd = [
        str(maturin_exe), "build",
        "--no-default-features",
        "--features", backend_name,
        "--release"
    ]
    
    result = subprocess.run(
        maturin_cmd,
        cwd=str(backend_dir),
        capture_output=True,
        text=True,
        check=True
    )
    
    # Find and install the wheel
    wheel_pattern = f"cloaca_{backend_name}-*.whl"
    wheel_dir = project_root / "target" / "wheels"
    wheel_files = list(wheel_dir.glob(wheel_pattern))
    
    if not wheel_files:
        raise FileNotFoundError(f"No wheel found matching {wheel_pattern} in {wheel_dir}")
    
    wheel_file = wheel_files[0]
    print(f"Installing wheel: {wheel_file.name}")
    subprocess.run([str(pip_exe), "install", str(wheel_file)], check=True, capture_output=True)
    
    return venv, python_exe, pip_exe



@cloaca()
@angreal.command(name="generate", about="generate all configuration files from templates")
@angreal.argument(name="backend", long="backend", help="Backend to generate for: postgres or sqlite")
def generate(backend):
    """Generate all configuration files from templates."""
    try:
        version = get_workspace_version()
        
        print(f"Using version: {version}")
        print(f"Generating files for {backend} backend...")
        
        project_root = Path(angreal.get_root()).parent
        template_dir = Path(angreal.get_root()) / "templates"
        
        # Generate dispatcher pyproject.toml
        dispatcher_template = template_dir / "dispatcher_pyproject.toml.j2"
        dispatcher_content = render_template(dispatcher_template.read_text(), {"version": version})
        dispatcher_path = project_root / "cloaca" / "pyproject.toml"
        
        # Generate backend Cargo.toml
        backend_template = template_dir / "backend_cargo.toml.j2"
        backend_content = render_template(backend_template.read_text(), {"backend": backend, "version": version})
        backend_path = project_root / "cloaca-backend" / "Cargo.toml"
        
        # Generate backend pyproject.toml
        backend_pyproject_template = template_dir / "backend_pyproject.toml.j2"
        backend_pyproject_content = render_template(backend_pyproject_template.read_text(), {"backend": backend, "version": version})
        backend_pyproject_path = project_root / "cloaca-backend" / "pyproject.toml"
        
        # Write files
        files_to_write = {
            dispatcher_path: dispatcher_content,
            backend_path: backend_content,
            backend_pyproject_path: backend_pyproject_content
        }
        
        print(f"Writing {len(files_to_write)} files...")
        for file_path, content in files_to_write.items():
            write_file_safe(file_path, content, backup=False)
            print(f"  {file_path}")
        
        # Generate backend Python directory from template
        print(f"Generating Python backend directory...")
        backend_python_src = project_root / "cloaca-backend" / "python" / "cloaca_{{backend}}"
        backend_python_dst = project_root / "cloaca-backend" / "python" / f"cloaca_{backend}"
        
        if backend_python_src.exists():
            context = {"backend": backend, "version": version}
            
            # Remove existing destination directory if it exists
            if backend_python_dst.exists():
                shutil.rmtree(backend_python_dst)
            
            # Create destination directory structure
            rendered_dirs = angreal.render_directory(
                src=str(backend_python_src),
                dst=str(backend_python_dst), 
                force=True,
                context=context
            )
            print(f"  Created directory structure: {len(rendered_dirs)} directories")
            
            # Walk template directory and render each file
            rendered_files = []
            for template_file in backend_python_src.rglob("*"):
                if template_file.is_file():
                    # Calculate relative path from template source
                    rel_path = template_file.relative_to(backend_python_src)
                    
                    # Render the relative path with context (for directory names)
                    rendered_rel_path = render_template(str(rel_path), context)
                    
                    # Create destination file path
                    dst_file = backend_python_dst / rendered_rel_path
                    
                    # Ensure destination directory exists
                    dst_file.parent.mkdir(parents=True, exist_ok=True)
                    
                    # Read template content and render it
                    template_content = template_file.read_text()
                    rendered_content = render_template(template_content, context)
                    
                    # Write rendered file
                    dst_file.write_text(rendered_content)
                    rendered_files.append(dst_file)
                    
            print(f"  Rendered {len(rendered_files)} files")
            for f in rendered_files:
                print(f"    {f}")
        else:
            print(f"  Warning: Template directory not found: {backend_python_src}")
        
        print(f"Successfully generated files for {backend} backend!")
        return 0
        
    except (FileOperationError, ValueError) as e:
        print(f"Generation failed: {e}")
        return 1


@cloaca()
@angreal.command(name="scrub", about="replace generated files with placeholder content and clean build artifacts")
def scrub():
    """Replace generated files with placeholder content and clean build artifacts."""
    try:
        project_root = Path(angreal.get_root()).parent
        
        # Clean build artifacts from cloaca crates only
        print("Cleaning cloaca build artifacts...")
        artifacts_cleaned = 0
        
        # Define cloaca-specific directories
        cloaca_dirs = [
            project_root / "cloaca-backend",
            project_root / "cloaca"
        ]
        
        for cloaca_dir in cloaca_dirs:
            if not cloaca_dir.exists():
                continue
                
            # Remove compiled extensions
            for pattern in ["*.so", "*.pyd"]:
                for artifact in cloaca_dir.rglob(pattern):
                    artifact.unlink()
                    artifacts_cleaned += 1
                    print(f"  Removed {artifact}")
            
            # Remove Python cache directories
            for cache_dir in cloaca_dir.rglob("__pycache__"):
                shutil.rmtree(cache_dir)
                artifacts_cleaned += 1
                print(f"  Removed {cache_dir}")
            
            # Remove target directories
            for target_dir in cloaca_dir.rglob("target"):
                if target_dir.is_dir():
                    shutil.rmtree(target_dir)
                    artifacts_cleaned += 1
                    print(f"  Removed {target_dir}")
        
        if artifacts_cleaned > 0:
            print(f"Cleaned {artifacts_cleaned} build artifacts")
        else:
            print("No build artifacts to clean")
        
        # Replace generated files with placeholders
        placeholder_template = """# This file is generated automatically during build
# Template: {template_path}
# DO NOT EDIT - Any changes will be overwritten
# To make changes, edit the template file above
"""
        
        # Files to clean with their template paths
        files_to_clean = {
            project_root / "cloaca" / "pyproject.toml": ".angreal/templates/dispatcher_pyproject.toml.j2",
            project_root / "cloaca-backend" / "Cargo.toml": ".angreal/templates/backend_cargo.toml.j2",
            project_root / "cloaca-backend" / "pyproject.toml": ".angreal/templates/backend_pyproject.toml.j2",
            project_root / "cloaca-backend" / "python" / "cloaca_postgres" / "__init__.py": "cloaca-backend/python/cloaca_{{backend}}/__init__.py",
            project_root / "cloaca-backend" / "python" / "cloaca_sqlite" / "__init__.py": "cloaca-backend/python/cloaca_{{backend}}/__init__.py"
        }
        
        print(f"Cleaning {len(files_to_clean)} generated files...")
        for file_path, template_path in files_to_clean.items():
            if file_path.name == "__init__.py":
                # Skip individual __init__.py files - we'll handle directories below
                continue
            placeholder_content = placeholder_template.format(template_path=template_path)
            write_file_safe(file_path, placeholder_content, backup=False)
            print(f"  {file_path}")
        
        # Clean entire backend Python directories
        print("Cleaning backend Python directories...")
        python_base_dir = project_root / "cloaca-backend" / "python"
        
        for backend in ["postgres", "sqlite"]:
            backend_dir = python_base_dir / f"cloaca_{backend}"
            if backend_dir.exists():
                # Remove the entire directory
                shutil.rmtree(backend_dir)
                print(f"  Removed directory: {backend_dir}")
                
                # Create directory with placeholder file
                backend_dir.mkdir()
                placeholder_file = backend_dir / "__init__.py"
                placeholder_content = f"""# This entire directory is generated automatically during build
# Template directory: cloaca-backend/python/cloaca_{{{{backend}}}}
# DO NOT EDIT - Any changes will be overwritten
# The entire cloaca_{backend} directory is re-rendered from the template during 'angreal cloaca generate'
# To make changes, edit the template directory above
"""
                write_file_safe(placeholder_file, placeholder_content, backup=False)
                print(f"  Created placeholder: {placeholder_file}")
        
        print("Successfully cleaned generated files and build artifacts!")
        return 0
        
    except FileOperationError as e:
        print(f"Clean failed: {e}")
        return 1


@cloaca()
@angreal.command(name="package", about="generate files, build wheel, then clean")
@angreal.argument(name="backend", long="backend", help="Backend to build: postgres or sqlite")
def package(backend):
    """Generate files, build the wheel, then clean up generated files."""
    try:
        # Step 1: Generate files
        print("Step 1: Generating files...")
        result = generate(backend)
        if result != 0:
            return result
            
        # Step 2: Build wheel
        print("Step 2: Building wheel...")
        project_root = Path(angreal.get_root()).parent
        backend_dir = project_root / "cloaca-backend"
        
        # Create temporary virtual environment for building
        venv_name = f"build-env-{backend}"
        venv_path = project_root / venv_name
        
        try:
            # Create virtual environment
            print(f"Creating build environment: {venv_name}")
            venv = VirtualEnv(path=str(venv_path), now=True)
            
            # Install pip and maturin
            python_exe = venv.path / "bin" / "python"
            print("Installing build dependencies...")
            subprocess.run([str(python_exe), "-m", "ensurepip"], check=True, capture_output=True)
            
            pip_exe = venv.path / "bin" / "pip3"
            subprocess.run([str(pip_exe), "install", "maturin"], check=True, capture_output=True)
            
            # Clean up any existing .so files to avoid conflicts
            print("Cleaning existing compiled extensions...")
            for pattern in ["*.so", "*.pyd"]:
                for artifact in backend_dir.rglob(pattern):
                    artifact.unlink()
                    print(f"  Removed {artifact.name}")
            
            # Build the wheel using maturin
            print(f"Building {backend} wheel...")
            maturin_exe = venv.path / "bin" / "maturin"
            maturin_cmd = [
                str(maturin_exe), "build",
                "--no-default-features",
                "--features", backend,
                "--release"
            ]
            
            result = subprocess.run(
                maturin_cmd,
                cwd=str(backend_dir),
                capture_output=True,
                text=True,
                check=True
            )
            print(f"  Build completed successfully")
            
            # Find the built wheel
            wheel_pattern = f"cloaca_{backend}-*.whl"
            wheel_dir = project_root / "target" / "wheels"
            wheel_files = list(wheel_dir.glob(wheel_pattern))
            
            if wheel_files:
                wheel_file = wheel_files[0]
                print(f"  Built wheel: {wheel_file.name}")
            else:
                print(f"  Warning: No wheel found matching {wheel_pattern} in {wheel_dir}")
                
        except subprocess.CalledProcessError as e:
            print(f"  Build failed with exit code {e.returncode}")
            if e.stdout:
                print(f"  STDOUT: {e.stdout}")
            if e.stderr:
                print(f"  STDERR: {e.stderr}")
            return 1
        except Exception as e:
            print(f"  Build failed: {e}")
            return 1
        finally:
            # Clean up the build environment
            if venv_path.exists():
                print(f"Cleaning up build environment: {venv_name}")
                shutil.rmtree(venv_path)
        
        # Step 3: Clean up
        print("Step 3: Cleaning generated files...")
        result = scrub()
        if result != 0:
            return result
            
        print(f"Successfully built {backend} backend!")
        return 0
        
    except Exception as e:
        print(f"Build failed: {e}")
        return 1


@cloaca()
@angreal.command(name="smoke", about="run basic smoke tests to verify Python bindings work")
@angreal.argument(name="backend", long="backend", help="Test specific backend: postgres or sqlite (default: both)", required=False)
def smoke(backend=None):
    """Run basic smoke tests to verify Python bindings work."""
    
    # Define backend configurations
    backends_to_test = []
    if backend == "postgres":
        backends_to_test = ["postgres"]
    elif backend == "sqlite":
        backends_to_test = ["sqlite"]
    elif backend is None:
        backends_to_test = ["postgres", "sqlite"]
    else:
        print(f"Error: Invalid backend '{backend}'. Use 'postgres' or 'sqlite'.")
        return 1

    all_passed = True

    for backend_name in backends_to_test:
        print(f"\n{'='*50}")
        print(f"Smoke testing {backend_name.title()} backend")
        print(f"{'='*50}")

        project_root = Path(angreal.get_root()).parent
        venv_name = f"smoke-test-{backend_name}"
        venv_path = project_root / venv_name

        try:
            # Build and install cloaca backend in test environment
            venv, python_exe, pip_exe = _build_and_install_cloaca_backend(backend_name, venv_name)
            
            # Step 4: Run smoke test
            print("Step 4: Running smoke test...")
            test_script = f'''
try:
    import cloaca_{backend_name}
    print("✓ Successfully imported cloaca_{backend_name}")
    print("✓ Smoke test passed!")
except Exception as e:
    print(f"✗ Smoke test failed: {{e}}")
    import traceback
    traceback.print_exc()
    exit(1)
'''
            
            result = subprocess.run([
                str(python_exe), "-c", test_script
            ], check=True, capture_output=True, text=True)
            print(result.stdout)
            print(f"✓ {backend_name.title()} smoke test passed")

        except subprocess.CalledProcessError as e:
            print(f"✗ Smoke test failed for {backend_name}: {e}")
            if e.stdout:
                print("STDOUT:", e.stdout)
            if e.stderr:
                print("STDERR:", e.stderr)
            all_passed = False
        except Exception as e:
            print(f"✗ Failed to setup {backend_name} test environment: {e}")
            all_passed = False
        finally:
            # Clean up test environment
            if venv_path.exists():
                print(f"Cleaning up test environment: {venv_name}")
                shutil.rmtree(venv_path)
            
            # Clean up generated files
            scrub()

    if all_passed:
        print(f"\n{'='*50}")
        print("All smoke tests passed!")
        print(f"{'='*50}")
        return 0
    else:
        print(f"\n{'='*50}")
        print("Some smoke tests failed!")
        print(f"{'='*50}")
        return 1


@cloaca()
@angreal.command(name="test", about="run tests in isolated test environments")
@angreal.argument(name="backend", long="backend", help="Test specific backend: postgres or sqlite (default: both)", required=False)
@angreal.argument(name="filter", short="k", help="Filter tests by expression (pytest -k)")
def test(backend=None, filter=None):
    """Run Python binding tests in isolated virtual environments.

    Creates fresh virtual environments for each test run to ensure
    no cross-contamination between test cycles.
    """
    
    # Define backend configurations
    backends_to_test = []
    if backend == "postgres":
        backends_to_test = ["postgres"]
    elif backend == "sqlite":
        backends_to_test = ["sqlite"]
    elif backend is None:
        backends_to_test = ["postgres", "sqlite"]
    else:
        print(f"Error: Invalid backend '{backend}'. Use 'postgres' or 'sqlite'.")
        return 1

    all_passed = True

    for backend_name in backends_to_test:
        print(f"\n{'='*50}")
        print(f"Testing {backend_name.title()} backend")
        print(f"{'='*50}")

        project_root = Path(angreal.get_root()).parent
        venv_name = f"test-env-{backend_name}"
        venv_path = project_root / venv_name

        try:
            # Build and install cloaca backend in test environment
            venv, python_exe, pip_exe = _build_and_install_cloaca_backend(backend_name, venv_name)
            
            # Step 4: Run tests
            print("Step 4: Running tests...")
            cmd = [str(python_exe), "-m", "pytest", str(project_root / "python-tests"), "-v"]
            if filter:
                cmd.extend(["-k", filter])
            
            # Set environment variable for backend detection
            import os
            env = os.environ.copy()
            env["CLOACA_BACKEND"] = backend_name
            
            try:
                subprocess.run(cmd, env=env, check=True)
                print(f"\n✓ {backend_name.title()} tests passed")
            except subprocess.CalledProcessError as e:
                print(f"\n✗ {backend_name.title()} tests failed: {e}")
                all_passed = False

        except subprocess.CalledProcessError as e:
            print(f"✗ Test setup failed for {backend_name}: {e}")
            if e.stdout:
                print("STDOUT:", e.stdout)
            if e.stderr:
                print("STDERR:", e.stderr)
            all_passed = False
        except Exception as e:
            print(f"✗ Failed to setup {backend_name} test environment: {e}")
            all_passed = False
        finally:
            # Clean up test environment
            if venv_path.exists():
                print(f"Cleaning up test environment: {venv_name}")
                shutil.rmtree(venv_path)
            
            # Clean up generated files
            scrub()

    if all_passed:
        print(f"\n{'='*50}")
        print("All tests passed!")
        print(f"{'='*50}")
        return 0
    else:
        print(f"\n{'='*50}")
        print("Some tests failed!")
        print(f"{'='*50}")
        return 1


@cloaca()
@angreal.command(name="release", about="build release wheels for distribution (leaves artifacts for inspection)")
@angreal.argument(name="backend", long="backend", help="Build specific backend: postgres or sqlite (default: both)", required=False)
def release(backend=None):
    """Build release wheels for distribution without cleanup.
    
    Generates files, builds wheels, but leaves all artifacts for inspection.
    Use 'scrub' command to clean up afterward.
    """
    
    # Define backend configurations
    backends_to_build = []
    if backend == "postgres":
        backends_to_build = ["postgres"]
    elif backend == "sqlite":
        backends_to_build = ["sqlite"]
    elif backend is None:
        backends_to_build = ["postgres", "sqlite"]
    else:
        print(f"Error: Invalid backend '{backend}'. Use 'postgres' or 'sqlite'.")
        return 1

    all_passed = True
    built_wheels = []

    for backend_name in backends_to_build:
        print(f"\n{'='*50}")
        print(f"Building {backend_name.title()} release wheel")
        print(f"{'='*50}")

        project_root = Path(angreal.get_root()).parent
        venv_name = f"release-build-{backend_name}"
        venv_path = project_root / venv_name

        try:
            # Step 1: Generate files
            print("Step 1: Generating files...")
            result = generate(backend_name)
            if result != 0:
                all_passed = False
                continue

            # Step 2: Create build environment
            print("Step 2: Creating build environment...")
            venv = VirtualEnv(path=str(venv_path), now=True)
            
            python_exe = venv.path / "bin" / "python"
            pip_exe = venv.path / "bin" / "pip3"
            
            # Install pip and maturin
            print("Installing build dependencies...")
            subprocess.run([str(python_exe), "-m", "ensurepip"], check=True, capture_output=True)
            subprocess.run([str(pip_exe), "install", "maturin"], check=True, capture_output=True)
            
            # Step 3: Build wheel
            print(f"Step 3: Building {backend_name} release wheel...")
            backend_dir = project_root / "cloaca-backend"
            
            # Clean existing extensions
            for pattern in ["*.so", "*.pyd"]:
                for artifact in backend_dir.rglob(pattern):
                    artifact.unlink()
            
            # Build wheel
            maturin_exe = venv.path / "bin" / "maturin"
            maturin_cmd = [
                str(maturin_exe), "build",
                "--no-default-features",
                "--features", backend_name,
                "--release"
            ]
            
            result = subprocess.run(
                maturin_cmd,
                cwd=str(backend_dir),
                capture_output=True,
                text=True,
                check=True
            )
            
            # Find the built wheel
            wheel_pattern = f"cloaca_{backend_name}-*.whl"
            wheel_dir = project_root / "target" / "wheels"
            wheel_files = list(wheel_dir.glob(wheel_pattern))
            
            if wheel_files:
                wheel_file = wheel_files[0]
                built_wheels.append(wheel_file)
                print(f"✓ Built release wheel: {wheel_file.name}")
            else:
                print(f"✗ No wheel found matching {wheel_pattern} in {wheel_dir}")
                all_passed = False

        except subprocess.CalledProcessError as e:
            print(f"✗ Release build failed for {backend_name}: {e}")
            if e.stdout:
                print("STDOUT:", e.stdout)
            if e.stderr:
                print("STDERR:", e.stderr)
            all_passed = False
        except Exception as e:
            print(f"✗ Failed to build {backend_name} release: {e}")
            all_passed = False
        finally:
            # Clean up build environment (but leave generated files and wheels)
            if venv_path.exists():
                print(f"Cleaning up build environment: {venv_name}")
                shutil.rmtree(venv_path)

    # Summary
    if all_passed:
        print(f"\n{'='*50}")
        print("Release build completed successfully!")
        print(f"{'='*50}")
        print("Built wheels:")
        for wheel in built_wheels:
            print(f"  {wheel}")
        print(f"\nGenerated files and wheels preserved for inspection.")
        print(f"Run 'angreal cloaca scrub' to clean up when ready.")
        return 0
    else:
        print(f"\n{'='*50}")
        print("Some release builds failed!")
        print(f"{'='*50}")
        return 1
