-- CLOACI-T-0836: bundled constructor providers for hermetic packaged workflows.
-- Each row carries ONE provider's packed `.cloacina` archive (arch-neutral WASM),
-- keyed to the consumer workflow package that referenced it via
-- `constructor!(from = "<provider_name>")`. The reconciler unpacks these into a
-- `providers/` tree and resolves constructor nodes against it at load — no
-- provider directory, no network. UUID as BLOB, TIMESTAMP as TEXT (RFC3339).
CREATE TABLE package_providers (
    id BLOB PRIMARY KEY NOT NULL,
    package_name TEXT NOT NULL,
    version TEXT NOT NULL,
    tenant_id TEXT,
    provider_name TEXT NOT NULL,
    provider_version TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    provider_data BLOB NOT NULL,
    created_at TEXT NOT NULL
);
CREATE UNIQUE INDEX idx_package_providers_key
    ON package_providers (package_name, version, COALESCE(tenant_id, ''), provider_name);
CREATE INDEX idx_package_providers_lookup
    ON package_providers (package_name, version);
