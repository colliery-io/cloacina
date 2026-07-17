-- SQLite 3.35+ supports DROP COLUMN (the workspace already requires it for
-- RETURNING clauses).
DROP INDEX idx_package_providers_key;
ALTER TABLE package_providers DROP COLUMN runtime;
ALTER TABLE package_providers DROP COLUMN target_triple;
CREATE UNIQUE INDEX idx_package_providers_key
    ON package_providers (package_name, version, COALESCE(tenant_id, ''), provider_name);
