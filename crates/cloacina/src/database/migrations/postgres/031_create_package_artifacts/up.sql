-- CLOACI-T-0780: per-target compiled artifacts for multi-arch dispatch.
-- workflow_packages holds the PRIMARY (host-arch) build; this table holds EXTRA
-- cdylibs, one per target triple, so one package can be handed to agents of
-- differing architectures. Dispatch picks the row matching the agent's
-- target_triple and falls back to the primary build for the host arch.
CREATE TABLE package_artifacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    package_name VARCHAR(255) NOT NULL,
    version VARCHAR(100) NOT NULL,
    tenant_id TEXT,
    target_triple TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    compiled_data BYTEA NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX idx_package_artifacts_key
    ON package_artifacts (package_name, version, COALESCE(tenant_id, ''), target_triple);
CREATE INDEX idx_package_artifacts_lookup
    ON package_artifacts (package_name, target_triple);
