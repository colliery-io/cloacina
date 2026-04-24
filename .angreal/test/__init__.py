# Cloacina test suites — the `test` command group and its subgroups.
from . import unit  # noqa: F401
from . import integration  # noqa: F401
from . import macros  # noqa: F401
from . import auth  # noqa: F401
from . import all  # noqa: F401
from . import coverage  # noqa: F401
from . import metrics_format  # noqa: F401
from .e2e import cli, compiler, ws  # noqa: F401
from .soak import daemon, server  # noqa: F401
