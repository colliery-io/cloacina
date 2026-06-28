# Cloacina test suites — the `test` command group and its subgroups.
from . import unit  # noqa: F401
from . import integration  # noqa: F401
from . import macros  # noqa: F401
from . import auth  # noqa: F401
from . import all  # noqa: F401
from . import coverage  # noqa: F401
from . import metrics_format  # noqa: F401
from .e2e import cli, compiler, fleet as e2e_fleet, k8s_fleet, k8s_leader, sdk_contract, ui_e2e, ws  # noqa: F401
from .soak import daemon, fleet as soak_fleet, fleet_actuator as soak_fleet_actuator, k8s_fleet as soak_k8s_fleet, server  # noqa: F401
