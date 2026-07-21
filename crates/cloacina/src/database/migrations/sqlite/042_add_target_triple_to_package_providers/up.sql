-- CLOACI-T-0908: per-arch NATIVE provider bundles (see the postgres twin for
-- the full rationale). NULL target_triple = the primary build; `runtime`
-- ('wasm' | 'native') targets the missing-arch scan. The uniqueness key gains
-- the triple so primary + per-arch rows coexist.
ALTER TABLE package_providers ADD COLUMN target_triple TEXT;
ALTER TABLE package_providers ADD COLUMN runtime TEXT NOT NULL DEFAULT 'wasm';

DROP INDEX idx_package_providers_key;
CREATE UNIQUE INDEX idx_package_providers_key
    ON package_providers (package_name, version, COALESCE(tenant_id, ''), provider_name, COALESCE(target_triple, ''));
