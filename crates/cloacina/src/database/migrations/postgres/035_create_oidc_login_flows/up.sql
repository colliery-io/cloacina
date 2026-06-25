-- CLOACI-T-0801: OIDC in-flight login state, Postgres-backed so the
-- authorization-code flow is multi-replica safe (NFR-003) — the callback may
-- land on a different replica than the one that began the login. One row per
-- in-flight login, keyed by the CSRF `state`. `nonce` + `pkce_verifier` are the
-- single-use secrets the callback needs to validate the ID token and exchange
-- the code. They are short-lived (minutes) and single-use (the row is deleted
-- on consumption), so they are stored as-is rather than encrypted; `expires_at`
-- bounds the flow and a sweeper deletes lapsed rows.
CREATE TABLE oidc_login_flows (
    state TEXT PRIMARY KEY,
    nonce TEXT NOT NULL,
    pkce_verifier TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_oidc_login_flows_expires_at ON oidc_login_flows (expires_at);
