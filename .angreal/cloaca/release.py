import angreal  # type: ignore


cloaca = angreal.command_group(name="cloaca", about="commands for Python binding tests")


@cloaca()
@angreal.command(
    name="release",
    about="build release wheel and sdist for distribution (leaves artifacts for inspection)",
    when_to_use=["DEPRECATED — cloaca is now embedded in cloacina core via PyO3"],
    when_not_to_use=["all cases — there is no separate wheel to build"],
)
def release():
    """[DEPRECATED] The cloaca Python module is now embedded in cloacina core.

    There is no separate release artifact to build. Python packaging
    is handled through the cloacina binary distribution.
    """
    print("=" * 50)
    print("DEPRECATED: cloaca is now embedded in cloacina core")
    print()
    print("The cloaca Python module is no longer distributed as a")
    print("separate PyPI package. It is provided natively by cloacina.")
    print()
    print("Release artifacts are managed through unified_release.yml")
    print("=" * 50)
