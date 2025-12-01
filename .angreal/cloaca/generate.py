import angreal # type: ignore

cloaca = angreal.command_group(name="cloaca", about="commands for Python binding tests")


@cloaca()
@angreal.command(
    name="generate",
    about="[DEPRECATED] Template generation no longer needed with unified wheel",
    when_to_use=["This command is deprecated"],
    when_not_to_use=["All cases - use unified build instead"]
)
@angreal.argument(name="backend", long="backend", help="[DEPRECATED] No longer needed", required=False)
def generate(backend=None):
    """Generate is no longer needed - unified wheel supports both backends at runtime."""
    print("NOTE: Template generation is no longer needed!")
    print("The unified cloaca wheel now supports both PostgreSQL and SQLite at runtime.")
    print("Simply build with: maturin build --release")
    print("Or run smoke tests with: angreal cloaca smoke")
    return 0
