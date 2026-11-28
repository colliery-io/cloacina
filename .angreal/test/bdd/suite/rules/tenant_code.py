"""Rules of the chapter "tenant_code" (CLOACINA-T-1079, CLOACINA-A-0019).

Two claims of the ADR can be judged on every pass from the database alone,
with no pod log: what each tenant's build role may touch, and which role the
server's connections of each tenant use. The other claims are judged by the
scenarios of `features/tenant_code/`, which read pod logs and run packages.

Both read Postgres's own catalogue as the admin: `pg_roles`, `pg_class` and
`has_table_privilege`, and `pg_stat_activity`. No query reads a row of a
tenant.
"""

from __future__ import annotations

from ..engine import broken, holds, unknown
from . import _shared

CHAPTER = "tenant_code"

# The tables a tenant's build role holds rights on (security-model.md, "What
# a tenant's build can reach"), all in the tenant's own schema.
BUILD_TABLES = ("package_artifacts", "package_providers", "workflow_packages", "workflow_registry")

_TABLE_KINDS = "c.relkind IN ('r', 'p', 'v', 'm')"
_NOT_CATALOGUE = ("n.nspname NOT IN ('pg_catalog', 'information_schema') "
                  "AND n.nspname NOT LIKE 'pg_toast%'")
_ANY_RIGHT = "'SELECT, INSERT, UPDATE, DELETE, TRUNCATE'"

# Each table that a build role (`<tenant>$build`) has any right on.
BUILD_ROLE_REACH_SQL = (
    "SELECT r.rolname AS role, left(r.rolname, length(r.rolname) - 6) AS tenant, "
    "n.nspname AS schema, c.relname AS name "
    "FROM pg_roles r CROSS JOIN pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace "
    "WHERE r.rolname LIKE '%$build' "
    f"AND {_TABLE_KINDS} AND {_NOT_CATALOGUE} "
    f"AND has_table_privilege(r.oid, c.oid, {_ANY_RIGHT}) "
    "ORDER BY 1, 3, 4"
)

# A tenant's role has the tenant's name, as its schema does. For each such
# role: how many tables of each schema it has a right on.
TENANT_ROLE_REACH_SQL = (
    "SELECT r.rolname AS role, n.nspname AS schema, count(*) AS tables "
    "FROM pg_roles r JOIN pg_namespace own ON own.nspname = r.rolname "
    "CROSS JOIN pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace "
    f"WHERE {_TABLE_KINDS} AND {_NOT_CATALOGUE} "
    f"AND has_table_privilege(r.oid, c.oid, {_ANY_RIGHT}) "
    "GROUP BY 1, 2 ORDER BY 1, 2"
)

# The open connections of the database, by role.
CONNECTIONS_SQL = (
    "SELECT usename AS role, count(*) AS connections FROM pg_stat_activity "
    "WHERE datname = current_database() AND usename IS NOT NULL GROUP BY usename ORDER BY 1"
)


def build_role_faults(rows):
    """What is wrong in the rows of BUILD_ROLE_REACH_SQL: a right outside the
    role's own schema, or on a table of its schema that a build does not
    need. Names only."""
    faults = []
    for row in rows:
        tenant, schema, name = row.get("tenant"), row.get("schema"), row.get("name")
        if schema != tenant:
            faults.append(f"{row.get('role')} has a right on {schema}.{name}, outside its tenant")
        elif name not in BUILD_TABLES:
            faults.append(f"{row.get('role')} has a right on {schema}.{name}, which a build does not need")
    return faults


def register(registry):
    registry.need_admin_db("build_role_reach", BUILD_ROLE_REACH_SQL)
    registry.need_admin_db("tenant_role_reach", TENANT_ROLE_REACH_SQL)
    registry.need_admin_db("connections", CONNECTIONS_SQL)

    @registry.rule("tenant_code.build_role_reach", "tenants.code_build_role", CHAPTER,
                   tolerates=("postgres_away",),
                   about="each tenant's build role has rights on its own build tables and on "
                         "nothing else: no other schema, no table of public's server data")
    def build_role_reach(snapshot):
        rows = snapshot.db(None, "build_role_reach")
        faults = build_role_faults(rows)
        roles = sorted({row.get("role") for row in rows})
        without = [tenant for tenant in _shared.workload_tenants(snapshot)
                   if not any(row.get("tenant") == tenant and row.get("schema") == tenant
                              and row.get("name") == "workflow_packages" for row in rows)]
        if faults:
            return broken(f"{len(faults)} rights of build roles outside their build tables", faults)
        if without:
            # A tenant with no build role, or one that cannot read its own
            # packages, has a compiler that the server did not make.
            return unknown("no build role that reads its own packages for: " + ", ".join(without))
        return holds(f"{len(roles)} build roles, each with rights on its own build tables only: "
                     + ", ".join(roles))

    @registry.rule("tenant_code.tenant_pool_role", "tenants.code_pool_role", CHAPTER,
                   gap="CLOACINA-T-1080", tolerates=("postgres_away", "restart_server"),
                   about="the server's connections of each tenant use the tenant's own role, and "
                         "that role has no right on another tenant's schema")
    def tenant_pool_role(snapshot):
        reach = snapshot.db(None, "tenant_role_reach")
        connections = {row.get("role"): int(row.get("connections") or 0)
                       for row in snapshot.db(None, "connections")}
        tenants = _shared.workload_tenants(snapshot)
        named = {row.get("role") for row in reach}
        faults = []
        for row in reach:
            schema = row.get("schema")
            if schema not in (row.get("role"), "public") and schema in named:
                faults.append(f"the role {row.get('role')} has rights on {row.get('tables')} "
                              f"table(s) of the tenant {schema}")
        for tenant in tenants:
            if not connections.get(tenant):
                faults.append(f"no connection uses the role {tenant}: the server reaches the "
                              f"tenant's schema with another role")
        if faults:
            return broken(f"{len(faults)} faults of the tenant roles", faults)
        return holds("connections by tenant role: "
                     + ", ".join(f"{tenant} {connections[tenant]}" for tenant in tenants))
