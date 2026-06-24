---
title: "Configure OIDC Single Sign-On"
description: "Wire an OIDC identity provider to cloacina-server: discovery, the identity→tenant allowlist, the browser login flow, and multi-tenant memberships."
weight: 27
---

# Configure OIDC Single Sign-On

When you'd rather your operators sign in through an existing identity provider
(Keycloak, Okta, Auth0, Dex, …) than manage credentials in Cloacina, configure
the server as an **OIDC relying party**. A successful sign-in is mapped to one or
more `{tenant, role}` memberships by a **god-owned allowlist** and **mints a
scoped bearer key** per tenant — the same kind of key everything else uses.

OIDC is a **server-only** feature. For the no-IdP alternative, see
[Configure Local Accounts]({{< ref "/service/how-to/configure-local-accounts" >}}).
For the trust model, see [Security Model]({{< ref "/service/explanation/security-model" >}}).

## Prerequisites

- A running `cloacina-server` and god (`is_admin`) access to set its configuration.
- An OIDC provider with a registered confidential client. You'll need its
  **issuer URL**, a **client id** + **client secret**, and the ability to register
  a **redirect URI** of `http(s)://<server>/v1/auth/callback`.
- The provider must expose standard discovery (`/.well-known/openid-configuration`)
  and a JWKS endpoint — Cloacina uses the `openidconnect` crate's spec-compliant
  discovery, PKCE, and ID-token validation.

## 1. Configure the relying party

Set these on the server (environment variables shown; absence of `CLOACINA_OIDC_ISSUER`
leaves OIDC disabled and the login routes unmounted):

| Variable | Purpose |
| --- | --- |
| `CLOACINA_OIDC_ISSUER` | The provider's issuer URL (discovery is fetched from `<issuer>/.well-known/openid-configuration`). |
| `CLOACINA_OIDC_CLIENT_ID` | The registered client id. |
| `CLOACINA_OIDC_CLIENT_SECRET` | The client secret. |
| `CLOACINA_OIDC_REDIRECT_URI` | Must match a redirect URI registered with the provider, e.g. `https://cloacina.example.com/v1/auth/callback`. |
| `CLOACINA_OIDC_SCOPES` | Optional, comma-separated. Defaults to `openid,email,profile,groups`. |
| `CLOACINA_OIDC_MAP` | The identity→tenant allowlist (next section). |
| `CLOACINA_OIDC_SUCCESS_REDIRECT` | Where to send the browser after login — the web UI's connect URL, e.g. `https://cloacina.example.com/connect`. |

The server runs discovery once at startup. If discovery fails (issuer
unreachable, misconfigured), OIDC is **disabled** rather than failing the whole
server — API-key and local-account auth keep working, and the OIDC routes return
`501`.

> **JWKS note:** the server caches the provider's signing keys at discovery. If
> your IdP rotates keys (or you recreate a test issuer), restart the server so it
> re-discovers.

## 2. Map identities to tenants (the allowlist)

`CLOACINA_OIDC_MAP` is a `;`-separated, **god-owned** allowlist. Each rule is
`<match>=<tenant>:<role>`, where `<match>` is one of:

- `group:<name>` — the identity is a member of this group (from the `groups` claim),
- `domain:<example.com>` — the identity's email is in this domain,
- `sub:<subject>` — an exact subject match.

`<tenant>` may be `_` for a global principal; `<role>` is `read` / `write` / `admin`.

```
CLOACINA_OIDC_MAP="group:platform-admins=_:admin;group:acme-eng=acme:write;domain:acme.com=acme:read"
```

An identity matching **no** rule is denied (`403`). Rules are evaluated in order;
**the first match per tenant wins**, and an identity that matches rules for
several tenants is granted all of them (see [Multi-tenant memberships](#multi-tenant-memberships)).

> Static-password IdP users (e.g. a Dex demo) often carry no `groups` claim —
> map on `domain:` in that case. Keep `group:` rules for real IdPs that emit
> groups.

## 3. The login flow

1. The user clicks **SSO → Continue with SSO** in the web UI (or navigates to
   `GET /v1/auth/oidc/login`). The server redirects to the IdP with PKCE + state
   + nonce.
2. The user authenticates at the IdP.
3. The IdP redirects to `/v1/auth/callback`. The server validates the ID token
   (JWKS signature, `iss` / `aud` / `exp`, nonce), resolves the identity through
   the allowlist, mints a scoped key per membership, and redirects to
   `CLOACINA_OIDC_SUCCESS_REDIRECT` with the minted key(s) in the URL **fragment**
   (a fragment is never sent to a server or logged).
4. The SPA reads the fragment and lands the user authenticated, stripping the key
   from the URL.

In-flight login state (`state` / nonce / PKCE verifier) is persisted in Postgres,
so the callback can land on **any replica** — no sticky sessions.

## Multi-tenant memberships

A single sign-on can grant access to several tenants at once. When the allowlist
matches an identity to more than one tenant, the server mints **one key per
tenant** and the UI presents a **tenant picker** — the user chooses where to
start, and the others are one click away in the tenant switcher. A single-tenant
identity skips the picker and lands straight in.

The bearer key is always scoped to exactly one tenant; "multi-tenant" lives in
holding several scoped keys and switching, not in a multi-tenant subject. See the
[Security Model]({{< ref "/service/explanation/security-model" >}}#multi-tenant-individuals).

## Try it locally

The repository's demo stack ships a self-contained **Dex** issuer wired to the
server — see `docker/AUTH_DEMO.md` for an end-to-end walkthrough (the one
`/etc/hosts` line a browser needs, the demo allowlist, and the multi-tenant
picker).

## See also

- [Configure Local Accounts]({{< ref "/service/how-to/configure-local-accounts" >}}) — the no-IdP alternative.
- [Security Model]({{< ref "/service/explanation/security-model" >}}) — ABAC authorization, identity providers, session lifecycle.
- [Manage API Keys]({{< ref "/service/how-to/manage-api-keys" >}}) — directly-created keys.
