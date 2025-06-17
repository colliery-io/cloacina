"""
Top-level purge command for deep cleaning the entire project.
"""

import angreal  # type: ignore

from cloaca.scrub import scrub
from task_services import clean as services_clean


@angreal.command(name="purge", about="deep clean the entire project including all Cargo.toml directories and Docker services")
def purge():
    """Deep clean the entire project including all Cargo.toml directories and Docker services."""
    print("Starting complete project purge...")

    # First clean Docker services and volumes
    print("\n=== Cleaning Docker services ===")
    docker_result = services_clean()
    if docker_result != 0:
        print("Warning: Docker services clean failed, continuing with other cleanup...")

    # Then do the deep scrub
    print("\n=== Running deep scrub ===")
    scrub_result = scrub(deep=True)

    # Return non-zero if either operation failed
    if docker_result != 0 or scrub_result != 0:
        return 1

    print("\n=== Purge complete! ===")
    return 0
