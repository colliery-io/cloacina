import angreal  # type: ignore


cloaca = angreal.command_group(name="cloaca", about="commands for Python binding tests")


@cloaca()
@angreal.command(
    name="package",
    about="build unified cloaca wheel",
    when_to_use=["DEPRECATED — cloaca is now embedded in cloacina core via PyO3"],
    when_not_to_use=["all cases — there is no separate wheel to build"],
)
def package():
    """[DEPRECATED] The cloaca Python module is now embedded in cloacina core.

    There is no separate wheel to build. The cloaca module is registered
    in sys.modules at runtime by cloacina's ensure_cloaca_module().
    """
    print("=" * 50)
    print("DEPRECATED: cloaca is now embedded in cloacina core")
    print()
    print("The cloaca Python module is no longer a separate wheel.")
    print("It is provided natively by cloacina via PyO3.")
    print()
    print("To verify Python integration works:")
    print("  angreal cloaca smoke")
    print("=" * 50)
