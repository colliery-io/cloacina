-- CLOACI-T-0836: bundled constructor providers for hermetic packaged workflows.
-- Each row carries ONE provider's packed `.cloacina` archive (arch-neutral WASM —
-- deliberately NOT per-target like package_artifacts), keyed to the consumer
-- workflow package that referenced it via `constructor!(from = "<provider_name>")`.
-- The reconciler unpacks these into a `providers/` tree and resolves constructor
-- nodes against it at load — no provider directory, no network.
CREATE TABLE package_providers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    package_name VARCHAR(255) NOT NULL,
    version VARCHAR(100) NOT NULL,
    tenant_id TEXT,
    provider_name TEXT NOT NULL,
    provider_version TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    provider_data BYTEA NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX idx_package_providers_key
    ON package_providers (package_name, version, COALESCE(tenant_id, ''), provider_name);
CREATE INDEX idx_package_providers_lookup
    ON package_providers (package_name, version);
