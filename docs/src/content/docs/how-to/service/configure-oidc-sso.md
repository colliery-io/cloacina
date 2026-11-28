---
title: "Configure OIDC Single Sign-On"
description: "Wire an OIDC identity provider to cloacina-server: discovery, the identity→tenant allowlist, the browser login flow, and multi-tenant memberships."
sidebar:
  order: 27
---

When you'd rather your operators sign in through an existing identity provider
(Keycloak, Okta, Auth0, Dex, …) than manage credentials in Cloacina, configure
the server as an **OIDC relying party**. A successful sign-in is mapped to one or
more `{tenant, role}` memberships by a **god-owned allowlist** and **mints a
scoped bearer key** per tenant — the same kind of key everything else uses.

OIDC is a **server-only** feature. For the no-IdP alternative, see
[Configure Local Accounts](../configure-local-accounts/).
For the trust model, see [Security Model](../../../explanation/service/security-model/).

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
- `domain:<example.com>` — the identity's email is in this domain, and the
  provider says the email is verified (see [What a `domain:` rule needs](#what-a-domain-rule-needs)),
- `sub:<subject>` — an exact subject match.

`<tenant>` is a tenant name; `<role>` is `read` / `write` / `admin`.

```
CLOACINA_OIDC_MAP="group:acme-admins=acme:admin;group:acme-eng=acme:write;domain:acme.com=acme:read"
```

### What a `_` rule grants

`<tenant>` may be `_`. Such a rule mints a key with **no tenant**. It does not
make a platform admin: no OIDC rule sets `is_admin`, and "no tenant" grants
nothing by itself. Whatever its role, that key:

- can sign in, read `GET /v1/auth/whoami`, refresh and log out;
- reaches no tenant route (`/v1/tenants/{tenant}/...`) and no platform route;
- sees no computation graph, reactor or accumulator, and cannot inject, fire,
  pause or resume one;
- is refused by the delivery WebSocket (`403`), so it gets no operations stream
  and no execution events.

To give a person access to a tenant's work, map them to that tenant. To give
one the `public` tenant, write `public`, not `_`. Platform administration uses
the bootstrap key. See
[A key with no tenant](../../../explanation/service/security-model/#a-key-with-no-tenant).

An identity matching **no** rule is denied (`403`). Rules are evaluated in order;
**the first match per tenant wins**, and an identity that matches rules for
several tenants is granted all of them (see [Multi-tenant memberships](#multi-tenant-memberships)).

> Static-password IdP users (e.g. a Dex demo) often carry no `groups` claim —
> map on `domain:` in that case. Keep `group:` rules for real IdPs that emit
> groups.

### What a `domain:` rule needs

A `domain:` rule matches only when the ID token carries `email_verified: true`.
An identity whose token says `email_verified: false`, or carries no
`email_verified` claim, matches no `domain:` rule. If no `group:` or `sub:` rule
matches it either, the login is denied (`403 identity_not_mapped`), and the
server logs a warning that names the subject.

A provider that lets a person set an email address it has not checked would
otherwise let that person pick a domain and take the role of its rule. Before
you write a `domain:` rule, check that your provider:

- releases the `email` and `email_verified` claims in the ID token for the
  `email` scope, and
- sets `email_verified` to `true` only for an address it has checked.

Dex sets `email_verified: true` for its static-password users. For a provider
that sends no `email_verified` claim, use `group:` or `sub:` rules.

`group:` and `sub:` rules do not read the email.

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

## 4. Refresh, group changes and map changes

A minted key lives 15 minutes; the UI refreshes it with `POST /v1/auth/refresh`.
The login session behind it has an absolute deadline,
`CLOACINA_OIDC_SESSION_MAX_AGE_S` (default `28800`, 8 hours). Past it the user
signs in through the IdP again.

The server stores the login's claims (`sub`, `email`, `email_verified`,
`groups`) on the session. At each refresh it runs the **current** allowlist on
them:

- a rule that now gives a lower (or higher) role gives that role to the new key;
- a tenant the identity no longer maps to ends that tenant's session: the
  refresh answers `401` and the key is revoked.

**Group changes at the IdP.** When the IdP's token response at login has a
refresh token and `CLOACINA_SECRET_KEK` is set, the server stores the token,
encrypted (AES-256-GCM, bound to the login). Once per
`CLOACINA_OIDC_GROUP_CHECK_INTERVAL_S` (default `3600`, one hour) a refresh
spends it at the IdP's token endpoint and reads the groups again, from the new
ID token or else from the userinfo endpoint. A person removed from a group
loses the role at the first refresh after the next check: within the interval
plus 15 minutes.

- The IdP refuses the refresh token (the person was disabled, or the token
  was revoked): the session ends.
- The IdP cannot be reached: the session keeps its last claims and runs to its
  deadline. The server logs a warning and tries again after the next interval.
- The IdP gives no refresh token (some need the `offline_access` scope in
  `CLOACINA_OIDC_SCOPES`), or no `CLOACINA_SECRET_KEK` is set: the groups are
  read only at login, and the deadline is the bound on a group removal.

Not every IdP puts `groups` in a refreshed ID token or in userinfo. Check that
yours does before you rely on the hourly check.

**Changing the allowlist.** `CLOACINA_OIDC_MAP` is read at start. A change
applies without downtime: a rolling restart, then live sessions follow at
their next refresh (at most 15 minutes). A map with a role that is not `read`,
`write` or `admin` stops the start of the new server, and the log says why;
with a rolling update the old servers keep serving with the old map.

Sessions opened before this release have no stored claims: they keep their
role until their deadline.

## Multi-tenant memberships

A single sign-on can grant access to several tenants at once. When the allowlist
matches an identity to more than one tenant, the server mints **one key per
tenant** and the UI presents a **tenant picker** — the user chooses where to
start, and the others are one click away in the tenant switcher. A single-tenant
identity skips the picker and lands straight in.

The bearer key is always scoped to exactly one tenant; "multi-tenant" lives in
holding several scoped keys and switching, not in a multi-tenant subject. See the
[Security Model](../../../explanation/service/security-model/#multi-tenant-individuals).

## Try it locally

The repository's demo stack ships a self-contained **Dex** issuer wired to the
server — see `docker/AUTH_DEMO.md` for an end-to-end walkthrough (the one
`/etc/hosts` line a browser needs, the demo allowlist, and the multi-tenant
picker).

## See also

- [Configure Local Accounts](../configure-local-accounts/) — the no-IdP alternative.
- [Security Model](../../../explanation/service/security-model/) — ABAC authorization, identity providers, session lifecycle.
- [Manage API Keys](../manage-api-keys/) — directly-created keys.
