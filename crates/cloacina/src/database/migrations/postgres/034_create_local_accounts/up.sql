-- CLOACI-T-0795: local accounts — self-managed username/password login with no
-- external IdP. The minimal credential entity (no profiles/org): username +
-- argon2id password hash + tenant + role + active/disabled status. The account
-- record IS the identity→tenant/role mapping (local login bypasses the OIDC
-- allowlist). `password_hash` is a PHC string; the plaintext is never stored.
CREATE TABLE local_accounts (
    id UUID PRIMARY KEY,
    username TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    tenant_id TEXT NULL,
    role TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Usernames are unique per tenant (a global account has tenant_id NULL).
CREATE UNIQUE INDEX idx_local_accounts_tenant_username
    ON local_accounts (tenant_id, username);
