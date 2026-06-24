# Auth demo runbook (CLOACI-I-0118)

Demonstrates the three auth paths the initiative delivered, in the compose demo
stack: **tenant API keys**, **self-managed local accounts (no IdP)**, and
**OIDC SSO** (via a self-contained Dex sidecar). All enforced by the in-process
ABAC layer; tenant is the isolation boundary.

## 0. Bring up the stack (with this code)

The images must be rebuilt to include the auth work:

```bash
docker compose -f docker/docker-compose.demo.yml build server ui
angreal ui up        # or: docker compose -f docker/docker-compose.demo.yml up -d
```

Services: server :8080, UI :8082, Dex :5556, Postgres, Kafka, agents.
Seeded keys (`CLOACINA_DEMO_TENANT_KEYS`): `acme` + `public` tenant-admins
(`clk_demo_acme_key_0002`, `clk_demo_public_key_0003`), plus the god bootstrap
key `clk_demo_bootstrap_key_0001`.

Open the UI at <http://localhost:8082>.

## 1. Self-managed local login (no IdP)

1. Connect with the **API key** tab: server `http://localhost:8080`, key
   `clk_demo_acme_key_0002`, tenant `acme`.
2. Go to **Accounts** → create a local account (e.g. `alice` / a password / role
   `write`). It appears in the list.
3. Sign out (tenant switcher → disconnect), back to **Connect** →
   **Username & password** tab → `alice` / password / tenant `acme` →
   **Sign in**. You land on the overview — no IdP involved. The session
   silently refreshes (`/auth/refresh`) before its ~15 min key TTL lapses.

## 2. Tenant-admin key management + isolation

1. Connected as the `acme` tenant-admin, go to **API Keys** → **+ Create key**.
   The plaintext is shown **once** — copy it, then **Done**.
2. The key appears in the roster; **Revoke** it.
3. Isolation (server-enforced, ABAC): the tenant-admin cannot reach a peer
   tenant's keys (`/v1/tenants/public/keys` → 403) nor the god-only global
   surface (`/v1/auth/keys` → 403). The UI only ever exposes
   `/v1/tenants/{t}/keys`.

## 3. OIDC SSO (via the Dex sidecar)

**One-time host setup** — the browser and the in-container server must resolve
the issuer at the same URL. Add this line to your `/etc/hosts`:

```
127.0.0.1 host.docker.internal
```

Then:

1. **Connect** → **SSO** tab → server `http://localhost:8080` →
   **Continue with SSO**. The browser is sent to Dex.
2. Log in at Dex as `alice@acme.com` / `password`.
3. Dex → server `/v1/auth/callback`: the server validates the ID token
   (JWKS signature, `iss`/`aud`/`exp`, nonce), maps the `acme-admins` group →
   tenant `acme` / role `admin` (the `CLOACINA_OIDC_MAP` allowlist), mints a
   short-TTL key, and redirects back to the SPA with the key in the URL
   fragment. You land on the overview as an `acme` admin.

The mapping is god-owned config; an unmapped identity is denied (403).

### Pure host-run alternative (no /etc/hosts edit)

If you'd rather not touch `/etc/hosts`, run the server binary on the host with
`CLOACINA_OIDC_ISSUER=http://localhost:5556/dex` (everything is then `localhost`
for both the browser and the server). See the commit history for the exact
invocation used during verification.

## Notes

- Minted keys (local + OIDC) carry a ~15 min TTL and refresh silently; pasted
  API keys don't expire and aren't refreshed.
- OIDC in-flight login state is in-memory (fine for this single-replica demo);
  NFR-003 multi-replica wants it Postgres-backed — tracked as a follow-up.
