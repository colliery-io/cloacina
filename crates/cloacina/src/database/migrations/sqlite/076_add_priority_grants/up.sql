-- CLOACINA-T-1123: who set a priority, and the limit that holds it. The
-- api_keys table is Postgres only; see the Postgres migration.
--
-- One row per package (kind 'package', target the package name) or schedule
-- (kind 'schedule', target the schedule id) that a principal set a priority
-- for. `set_by` is the principal: `key:<id>`, `oidc:<issuer>:<sub>` or
-- `local:<account_id>`; `set_by_role` its role then. `asked` is the priority a
-- schedule asks for (NULL for a package: its code declares it). `last_given`
-- is the priority its last run was given: a run start that changes it is the
-- one that writes the log line and the audit event.
CREATE TABLE priority_grants (
    kind TEXT NOT NULL,
    target TEXT NOT NULL,
    set_by TEXT NOT NULL,
    set_by_role TEXT NOT NULL,
    asked INTEGER,
    last_given INTEGER,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (kind, target)
);
