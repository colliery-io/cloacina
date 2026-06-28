-- CLOACI-T-0809: per-tenant desired agent count (CLOACI-I-0127). Tenant
-- self-service provisioning state: the number of agents a tenant has requested,
-- bounded by the god-set `effective_limit` from T-0808. Provision = +1 (if under
-- the limit), deprovision = -1 (floor 0). This is the operational target the
-- actuator (T-0810) and back-pressure autoscaler (T-0811) reconcile/clamp to.
-- Server mode only — Postgres.
CREATE TABLE agent_desired_counts (
    tenant_id TEXT PRIMARY KEY,
    desired_count INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
