import angreal  # type: ignore
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
    when_not_to_use=["active development", "before running tests"],
)
@angreal.argument(
    name="deep",
    long="deep",
    help="include cargo clean for thorough cleanup",
    takes_value=False,
    is_flag=True,
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

        # Clean Python cache directories across the project
        print("Cleaning Python cache directories...")
        caches_cleaned = 0
        for cache_dir in project_root.rglob("__pycache__"):
            shutil.rmtree(cache_dir)
            caches_cleaned += 1
        if caches_cleaned > 0:
            print(f"Cleaned {caches_cleaned} __pycache__ directories")

        # Clean SQLite database files
        print("Cleaning SQLite database files...")
        db_files_cleaned = 0
        for db_file in project_root.glob("*.db*"):
            db_file.unlink()
            db_files_cleaned += 1
            print(f"  Removed {db_file.name}")
        # Also clean temp demo databases
        for tmp_db in ["/tmp/cloacina_demo.db", "/tmp/cloacina_debug.db"]:
            p = Path(tmp_db)
            if p.exists():
                p.unlink()
                db_files_cleaned += 1
                print(f"  Removed {p.name}")

        if db_files_cleaned > 0:
            print(f"Cleaned {db_files_cleaned} database files")
        else:
            print("No database files to clean")

        # Deep clean: run cargo clean on workspace
        if deep:
            print("\nPerforming deep clean with cargo clean...")
            try:
                result = subprocess.run(
                    ["cargo", "clean"],
                    cwd=str(project_root),
                    capture_output=True,
                    text=True,
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
