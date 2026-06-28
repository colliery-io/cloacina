-- CLOACI-T-0808: per-tenant agent-capacity exceptions (CLOACI-I-0127). The
-- global default (`CLOACINA_DEFAULT_MAX_AGENTS`) is server config; this table
-- holds ONLY the per-tenant overrides an admin grants (e.g. default 4, acme 6).
-- The effective limit is the override if present, else the default. It is the
-- hard ceiling the provision API (T-0809) and the back-pressure autoscaler
-- (T-0811) clamp to. Server mode only — Postgres.
CREATE TABLE agent_capacity_limits (
    tenant_id TEXT PRIMARY KEY,
    max_agents INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
