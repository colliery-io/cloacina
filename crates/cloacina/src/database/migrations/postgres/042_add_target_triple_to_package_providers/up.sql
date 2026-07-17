-- CLOACI-T-0908: per-arch NATIVE provider bundles.
-- A native provider bundle is a host cdylib — arch-specific — but this table
-- stored exactly one build per (package, provider). `target_triple` NULL means
-- the PRIMARY build (the compiler host's arch; arch-neutral for wasm), which is
-- exactly what every existing row is, so no backfill. Per-target compiler scans
-- add triple-keyed rows for native providers; staging/fetch selects the reader's
-- own triple with fallback to the primary. `runtime` ('wasm' | 'native') lets
-- the missing-arch scan target native rows without unpacking archives; wasm
-- bundles are arch-neutral and never get triple rows.
ALTER TABLE package_providers ADD COLUMN target_triple TEXT;
ALTER TABLE package_providers ADD COLUMN runtime TEXT NOT NULL DEFAULT 'wasm';

-- The uniqueness key gains the triple so the primary row and per-arch rows
-- COEXIST (same COALESCE-for-NULL pattern the original key used for tenant_id).
DROP INDEX idx_package_providers_key;
CREATE UNIQUE INDEX idx_package_providers_key
    ON package_providers (package_name, version, COALESCE(tenant_id, ''), provider_name, COALESCE(target_triple, ''));
