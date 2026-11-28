-- CLOACINA-T-1123: who set a priority, and the limit that holds it.

-- The highest priority a key may set (0 the highest, 10 the lowest). NULL:
-- the default of the key's role.
ALTER TABLE api_keys ADD COLUMN max_priority INTEGER;

-- One row per package (kind 'package', target the package name) or schedule
-- (kind 'schedule', target the schedule id) that a principal set a priority
-- for. `set_by` is the principal: `key:<id>`, `oidc:<issuer>:<sub>` or
-- `local:<account_id>`; `set_by_role` its role then. `asked` is the priority a
-- schedule asks for (NULL for a package: its code declares it). `last_given`
-- is the priority its last run was given: a run start that changes it is the
-- one that writes the log line and the audit event.
CREATE TABLE priority_grants (
    kind VARCHAR NOT NULL,
    target VARCHAR NOT NULL,
    set_by TEXT NOT NULL,
    set_by_role TEXT NOT NULL,
    asked INTEGER,
    last_given INTEGER,
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    PRIMARY KEY (kind, target)
);
