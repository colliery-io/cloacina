-- CLOACI-T-0923: `oidc_sessions` becomes the OIDC **login-session** record.
--
-- T-0793 built this table to hold an encrypted IdP refresh token, but nothing
-- ever wrote a row (the callback minted keys and stored no session), so
-- `/v1/auth/refresh` returned 501 for `oidc:` provenance and an SSO session
-- died hard at the 15-minute key TTL.
--
-- T-0923 implements refresh with a **server-side session record** instead of
-- IdP token custody (see routes/session.rs for the full rationale):
--   * `expires_at` is now the session's ABSOLUTE deadline — the wall past which
--     no amount of refreshing keeps you in and a full OIDC re-auth (fresh IdP
--     check + fresh allowlist resolution) is forced.
--   * `key_id` is the currently-valid minted key; refresh rotates it onto the
--     newly-minted key in place, so the row's identity survives key rotation.
--   * `refresh_enc` therefore holds NOTHING for this design and becomes
--     NULLable. The column (and its `encrypt_token`/`decrypt_token` helpers)
--     is kept so a deployment that later wants true IdP-token custody can fill
--     it without another migration.
--
-- No sqlite twin: the whole auth strand (api_keys, local_accounts,
-- oidc_login_flows, oidc_sessions) is Postgres-only — the API server runs on
-- Postgres. Cf. migrations 033/034/035, which likewise have no sqlite sibling.
ALTER TABLE oidc_sessions ALTER COLUMN refresh_enc DROP NOT NULL;

-- Refresh looks the session up by its current key; logout deletes by key.
-- One live session per minted key, so make that a uniqueness invariant rather
-- than a convention (the old index was non-unique).
DROP INDEX IF EXISTS idx_oidc_sessions_key_id;
CREATE UNIQUE INDEX idx_oidc_sessions_key_id ON oidc_sessions (key_id);
