"""
Cleanup tasks for Cloaca.
"""

import shutil
from pathlib import Path

import angreal  # type: ignore

# Define command group
cloaca = angreal.command_group(name="cloaca", about="commands for Python binding tests")

from .utils import write_file_safe

@cloaca()
@angreal.command(name="scrub", about="replace generated files with placeholder content and clean build artifacts")
def scrub():
    """Replace generated files with placeholder content and clean build artifacts."""
    try:
        project_root = Path(angreal.get_root()).parent

        # Clean debug environments
        print("Cleaning debug environments...")
        debug_envs_cleaned = 0
        for backend in ["postgres", "sqlite"]:
            debug_env = project_root / f"debug-env-{backend}"
            if debug_env.exists():
                shutil.rmtree(debug_env)
                debug_envs_cleaned += 1
                print(f"  Removed debug environment: {debug_env}")

        if debug_envs_cleaned > 0:
            print(f"Cleaned {debug_envs_cleaned} debug environments")
        else:
            print("No debug environments to clean")

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

            # Remove dist directories
            for dist_dir in cloaca_dir.rglob("dist"):
                if dist_dir.is_dir():
                    shutil.rmtree(dist_dir)
                    artifacts_cleaned += 1
                    print(f"  Removed {dist_dir}")

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
