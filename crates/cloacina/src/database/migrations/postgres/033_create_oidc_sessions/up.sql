-- CLOACI-T-0793: server-side encrypted refresh-token store. One row per minted
-- login key. `refresh_enc` holds AES-256-GCM(nonce || ciphertext || tag) of the
-- IdP refresh token (or a cloacina-issued opaque token for local login). The
-- plaintext is never logged and never returned to the browser. `expires_at`
-- bounds the server-side session; a sweeper deletes lapsed rows.
CREATE TABLE oidc_sessions (
    id UUID PRIMARY KEY,
    key_id UUID NOT NULL REFERENCES api_keys(id),
    provider TEXT NOT NULL,
    refresh_enc BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NULL
);

CREATE INDEX idx_oidc_sessions_key_id ON oidc_sessions (key_id);
CREATE INDEX idx_oidc_sessions_expires_at ON oidc_sessions (expires_at);
