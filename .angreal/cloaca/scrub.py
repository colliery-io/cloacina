import angreal # type: ignore
import shutil
import subprocess
from pathlib import Path


# Define command group
cloaca = angreal.command_group(name="cloaca", about="commands for Python binding tests")



@cloaca()
@angreal.command(
    name="scrub",
    about="clean build artifacts and test environments",
    when_to_use=["cleaning development environment", "resetting after testing", "freeing disk space"],
    when_not_to_use=["active development", "before running tests"]
)
@angreal.argument(
    name="deep",
    long="deep",
    help="include cargo clean for thorough cleanup",
    takes_value=False,
    is_flag=True
)
def scrub(deep=False):
    """Clean build artifacts and test environments."""
    try:
        project_root = Path(angreal.get_root()).parent

        # Clean test/debug environments
        print("Cleaning test environments...")
        envs_cleaned = 0
        for env_pattern in ["smoke-test-*", "test-env-*", "debug-env-*", "tutorial-*"]:
            for env_dir in project_root.glob(env_pattern):
                if env_dir.is_dir():
                    shutil.rmtree(env_dir)
                    envs_cleaned += 1
                    print(f"  Removed: {env_dir.name}")

        if envs_cleaned > 0:
            print(f"Cleaned {envs_cleaned} test environments")
        else:
            print("No test environments to clean")

        # Clean build artifacts
        print("Cleaning build artifacts...")
        artifacts_cleaned = 0

        backend_dir = project_root / "bindings" / "cloaca-backend"
        if backend_dir.exists():
            # Remove compiled extensions
            for pattern in ["*.so", "*.pyd"]:
                for artifact in backend_dir.rglob(pattern):
                    artifact.unlink()
                    artifacts_cleaned += 1
                    print(f"  Removed {artifact}")

            # Remove Python cache directories
            for cache_dir in backend_dir.rglob("__pycache__"):
                shutil.rmtree(cache_dir)
                artifacts_cleaned += 1
                print(f"  Removed {cache_dir}")

            # Remove target directories
            target_dir = backend_dir / "target"
            if target_dir.exists():
                shutil.rmtree(target_dir)
                artifacts_cleaned += 1
                print(f"  Removed {target_dir}")

            # Remove dist directories
            dist_dir = backend_dir / "dist"
            if dist_dir.exists():
                shutil.rmtree(dist_dir)
                artifacts_cleaned += 1
                print(f"  Removed {dist_dir}")

        if artifacts_cleaned > 0:
            print(f"Cleaned {artifacts_cleaned} build artifacts")
        else:
            print("No build artifacts to clean")

        # Clean SQLite database files
        print("Cleaning SQLite database files...")
        db_files_cleaned = 0
        for db_file in project_root.glob("*.db*"):
            db_file.unlink()
            db_files_cleaned += 1
            print(f"  Removed {db_file.name}")

        if db_files_cleaned > 0:
            print(f"Cleaned {db_files_cleaned} database files")
        else:
            print("No database files to clean")

        # Deep clean: run cargo clean
        if deep:
            print("\nPerforming deep clean with cargo clean...")
            try:
                result = subprocess.run(
                    ["cargo", "clean"],
                    cwd=backend_dir,
                    capture_output=True,
                    text=True
                )
                if result.returncode == 0:
                    print("Deep clean completed successfully!")
                else:
                    print(f"Deep clean warning: {result.stderr}")
            except Exception as e:
                print(f"Deep clean failed: {e}")

        print("Cleanup completed!")

    except Exception as e:
        print(f"Clean failed: {e}")
        raise RuntimeError(f"Failed to clean: {e}")
