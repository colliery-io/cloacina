-- Add per-request / per-runner / per-tenant correlation columns to
-- execution_events. CLOACI-T-0583 / OPS-16.
--
-- New rows populate all three from the surrounding context (request span,
-- per-tenant runner, AuthenticatedKey). Pre-migration rows stay NULL —
-- per the locked-decision "no backfill."
--
-- Nullable because: (a) backfill is intentionally skipped, (b) the
-- daemon path emits events without any of these (single-tenant, no
-- request context), and (c) per-tenant runner ids only come online
-- once CLOACI-T-0580 lands.

ALTER TABLE execution_events ADD COLUMN request_id UUID;
ALTER TABLE execution_events ADD COLUMN runner_id UUID;
ALTER TABLE execution_events ADD COLUMN tenant_id TEXT;

-- Tenant-scoped historical queries: e.g. forensics for "what did tenant
-- foo's workflows do over the last 24h?" Bounded cardinality on
-- `tenant_id`; the `created_at DESC` ordering matches the dashboard /
-- retention sweep access pattern (mirrors `idx_execution_events_created`).
CREATE INDEX idx_execution_events_tenant_created
    ON execution_events(tenant_id, created_at DESC)
    WHERE tenant_id IS NOT NULL;
