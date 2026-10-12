# Cloacina test suites — the `test` command group and its subgroups.
from . import (
    all,  # noqa: F401
    auth,  # noqa: F401
    coverage,  # noqa: F401
    integration,  # noqa: F401
    macros,  # noqa: F401
    metrics_format,  # noqa: F401
    unit,  # noqa: F401
)
from .e2e import cli, compiler, k8s_fleet, k8s_leader, sdk_contract, ui_e2e, ws  # noqa: F401
from .e2e import fleet as e2e_fleet  # noqa: F401
from .soak import daemon, server  # noqa: F401
from .soak import fleet as soak_fleet  # noqa: F401
from .soak import fleet_actuator as soak_fleet_actuator  # noqa: F401
from .soak import k8s_fleet as soak_k8s_fleet  # noqa: F401
