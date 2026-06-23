-- CLOACI-T-0780: per-target compiled artifacts for multi-arch dispatch.
-- workflow_packages holds the PRIMARY (host-arch) build; this table holds EXTRA
-- cdylibs, one per target triple. UUID as BLOB, TIMESTAMP as TEXT (RFC3339).
CREATE TABLE package_artifacts (
    id BLOB PRIMARY KEY NOT NULL,
    package_name TEXT NOT NULL,
    version TEXT NOT NULL,
    tenant_id TEXT,
    target_triple TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    compiled_data BLOB NOT NULL,
    created_at TEXT NOT NULL
);
CREATE UNIQUE INDEX idx_package_artifacts_key
    ON package_artifacts (package_name, version, COALESCE(tenant_id, ''), target_triple);
CREATE INDEX idx_package_artifacts_lookup
    ON package_artifacts (package_name, target_triple);
