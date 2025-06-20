import angreal # type: ignore
from angreal import render_template# type: ignore

import shutil
from pathlib import Path

cloaca = angreal.command_group(name="cloaca", about="commands for Python binding tests")


from .cloaca_utils import get_workspace_version, normalize_version_for_python, write_file_safe, _build_and_install_cloaca_backend  # noqa: E402




@cloaca()
@angreal.command(
    name="generate", 
    about="generate all configuration files from templates",
    when_to_use=["setting up development environment", "switching between backends", "preparing for testing"],
    when_not_to_use=["production builds", "when files already generated", "CI/CD workflows"]
)
@angreal.argument(name="backend", long="backend", help="target backend: postgres or sqlite", required=True)
def generate(backend):
    """Generate all configuration files from templates."""

    try:
        cargo_version = get_workspace_version()
        python_version = normalize_version_for_python(cargo_version)

        print(f"Using Cargo version: {cargo_version}")
        print(f"Using Python version: {python_version}")
        print(f"Generating files for {backend} backend...")

        project_root = Path(angreal.get_root()).parent
        template_dir = Path(angreal.get_root()) / "templates"

        # Generate dispatcher pyproject.toml (uses normalized Python version for dependencies)
        dispatcher_template = template_dir / "dispatcher_pyproject.toml.tera"
        dispatcher_context = {
            "version": python_version,
            "python_version": python_version,
            "cargo_version": cargo_version
        }
        dispatcher_content = render_template(dispatcher_template.read_text(), dispatcher_context)
        dispatcher_path = project_root / "cloaca" / "pyproject.toml"

        # Generate backend Cargo.toml (uses original Cargo version)
        backend_template = template_dir / "backend_cargo.toml.tera"
        backend_content = render_template(backend_template.read_text(), {"backend": backend, "version": cargo_version})
        backend_path = project_root / "cloaca-backend" / "Cargo.toml"

        # Generate backend pyproject.toml (uses normalized Python version)
        backend_pyproject_template = template_dir / "backend_pyproject.toml.tera"
        backend_pyproject_content = render_template(backend_pyproject_template.read_text(), {"backend": backend, "version": python_version})
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
        print("Generating Python backend directory...")
        backend_python_src = project_root / "cloaca-backend" / "python" / "cloaca_{{backend}}"
        backend_python_dst = project_root / "cloaca-backend" / "python" / f"cloaca_{backend}"

        if backend_python_src.exists():
            context = {"backend": backend, "version": python_version}

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

        # Create debug virtual environment with backend installed
        print("Creating debug virtual environment...")
        venv_name = f"debug-env-{backend}"
        venv_path = project_root / venv_name

        try:
            # Remove existing debug environment if it exists
            if venv_path.exists():
                print(f"  Removing existing debug environment: {venv_name}")
                shutil.rmtree(venv_path)

            # Build and install backend explicitly (files already generated above)
            venv, python_exe, pip_exe = _build_and_install_cloaca_backend(backend, venv_name)

            print(f"✓ Debug environment ready: {venv_name}")
            print(f"  Usage: {venv_path}/bin/python your_debug_script.py")

        except Exception as e:
            print(f"  Warning: Failed to create debug environment: {e}")
            print("  You can still use the generated files manually")

        print(f"Successfully generated files for {backend} backend!")
        return 0

    except (OSError, ValueError) as e:
        print(f"Generation failed: {e}")
        return 1
