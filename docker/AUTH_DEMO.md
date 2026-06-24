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
   (JWKS signature, `iss`/`aud`/`exp`, nonce), maps the `acme.com` email domain
   to its tenants (the `CLOACINA_OIDC_MAP` allowlist), mints a short-TTL key
   **per tenant**, and redirects back to the SPA with the membership set in the
   URL fragment.
4. The demo map grants `acme.com` identities **two** tenants (`acme` admin +
   `public` read), so the SPA shows a **tenant picker** — pick one to enter; the
   other is one click away in the tenant switcher. (A single-tenant identity
   skips the picker and lands straight on the overview.)

The mapping is god-owned config; an unmapped identity is denied (403). (Dex
static-password users carry no groups, so the demo maps on email domain; the
`group:acme-admins` rule is kept for real IdPs that emit groups.)

> **If OIDC 401s with "Signature verification failed"** after you recreate the
> dex container: dex (in-memory) regenerates signing keys on each recreate, and
> the server caches JWKS at discovery. Restart the server so it re-discovers:
> `docker compose -f docker/docker-compose.demo.yml restart server`.

This full flow is verified end-to-end (login → Dex → callback → minted
`tenant=acme role=admin` key → SPA).

### Pure host-run alternative (no /etc/hosts edit)

If you'd rather not touch `/etc/hosts`, run the server binary on the host with
`CLOACINA_OIDC_ISSUER=http://localhost:5556/dex` (everything is then `localhost`
for both the browser and the server). See the commit history for the exact
invocation used during verification.

## 4. Multi-tenant individual

The same person can belong to several tenants; the subject (the bearer key)
stays single-tenant, and multi-tenancy lives in **holding one scoped key per
tenant + switching** in the UI (the tenant switcher, top-left).

**Via OIDC (one sign-in → many tenants):** the SSO flow above already does this
— `alice@acme.com` maps to `acme` (admin) + `public` (read), so a single login
mints a key for each and the picker lets her choose where to start; the switcher
holds both.

**Via local accounts (no IdP):** create the same person in two tenants and use
the switcher's **"Add tenant…"**:

1. As the `acme` admin, **Accounts** → create `alice` / a password / role
   `admin`. Then connect as the `public` admin (`clk_demo_public_key_0003`) and
   create `alice` / a different password / role `read` in `public`.
2. Sign in as `alice` in `acme` → tenant switcher → **"Add tenant…"** → sign in
   as `alice` in `public`.
3. Flip between them in the switcher. Same human, **admin in acme, read in
   public**, each key hard-isolated to its tenant (the other tenant's data is a
   403, never visible).

## Notes

- Minted keys (local + OIDC) carry a ~15 min TTL and refresh silently; pasted
  API keys don't expire and aren't refreshed.
- OIDC in-flight login state is **Postgres-backed** (multi-replica safe); the
  callback can land on any replica.
