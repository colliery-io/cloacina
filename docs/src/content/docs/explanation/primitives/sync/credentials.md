---
title: "Credential Injection"
description: "How a sync's connector reaches an authenticated API without holding the secret: the host adds the credential to each request the grants allow."
sidebar:
  order: 10
---

A connector that calls an authenticated API never holds the credential. The
connector sends plain requests. The host adds the credential to each request
on the way out, after the request has passed the connector's grants. The
secret stays outside the WASM sandbox (CLOACINA-T-0982).

## What the host does

A connector's network reach is its grants: an `http` grant names the hosts
and ports it may call. When a sync names a credential, the host wraps that
egress policy:

1. The guest sends a request.
2. The grants decide. A request they deny is denied, and no credential is
   added. A credential never widens the grants.
3. For an allowed request, the host adds the credential: a header, a query
   parameter, a signature, or a token.
4. The request leaves. The guest sees only the response.

The secret is read on the execution agent that runs the task, through the
task's secret resolver, as for any [secret](../../../service/secrets/). It is
never written to the execution's context.

## The kinds

| Kind | What the host adds | Non-secret fields | Secret fields |
|---|---|---|---|
| `header` | A header: `prefix` + the secret's field | `name`, `field`, `prefix` (optional) | the one `field` names |
| `query` | A query parameter, percent-encoded once | `param`, `field` | the one `field` names |
| `aws_sigv4` | An AWS Signature Version 4: `Authorization`, `x-amz-date`, `x-amz-content-sha256` | `region`, `service` (default `s3`) | `access_key_id`, `secret_access_key` |
| `snowflake_keypair_jwt` | A Snowflake key-pair JWT as a bearer, and `X-Snowflake-Authorization-Token-Type: KEYPAIR_JWT` | `account`, `user` | `private_key` (PKCS#8 PEM) |
| `basic` | `Authorization: Basic` of the pair | `username_field`, `password_field` (default `username`, `password`) | those two |
| `oauth2` | A bearer from an OAuth2 grant at `token_url` | `token_url`, `grant` (`client_credentials` or `refresh_token`), `scopes` | `client_id`, `client_secret`, and `refresh_token` for that grant |
| `session` | A token from a login, sent as `inject_header` with `prefix` | `login_url`, `method` (default `POST`), `login_fields` (body key to secret field), `token_path`, `inject_header`, `prefix`, `ttl_secs` (default 900) | the values of `login_fields` |
| `google_service_account` | A bearer from Google's JWT-bearer grant | `scopes` | `service_account_key` (the account's JSON key) |

A bearer token is a `header` with `name = "Authorization"` and
`prefix = "Bearer "`. The Snowflake JWT is signed on the host with RS256, is
valid for an hour, and is signed again a minute before it expires.

## Minted tokens

For `oauth2`, `session` and `google_service_account` the host makes the grant
or login request itself, on its own thread, and keeps the token for the
process (CLOACINA-T-0989). On an agent, every load of a connector with the
same credential shares it, so a sync does not mint a token per segment. A
token is minted again a minute before it expires, or when the secret changes:
the cache is keyed by a digest of who the token is for and checked against a
digest of the whole credential. A session token is kept for `ttl_secs`.

A provider that answers a refresh grant with a **new refresh token** retires
the old one. Cloacina does not store the new one yet, so that grant fails with
a message saying the refresh token was rotated; a provider that rotates
refresh tokens cannot be used with the `refresh_token` grant for now.

A declarative connector's definition names its auth kind, and the sync names
the secret (see [Declarative Connectors](../declarative/)).

## Naming a credential in a sync

A sync definition names one credential per side, in the `source_credential`
and `destination_credential` parameters. Each is a JSON object: the `kind`,
the `secret` to read, and the kind's non-secret fields. It holds no secret
value, and a field it does not know is an error, so a value pasted into it
fails instead of landing in the definition.

```json
{
  "source_provider": "cloacina-connector-s3",
  "source_member": "source",
  "source_config": "{\"endpoint\": \"https://s3.amazonaws.com\", \"bucket\": \"orders\"}",
  "source_grants": "{\"http\": [\"s3.amazonaws.com\"]}",
  "source_credential": "{\"kind\": \"aws_sigv4\", \"secret\": \"aws_keys\", \"region\": \"us-east-1\"}"
}
```

`secret` is either a secret's name or a local name the instance binds to one
with a `{"$secret": "<name>"}` parameter. Binding keeps the definition the same
across environments:

```json
{
  "aws_keys": {"$secret": "orders-prod-aws"},
  "source_credential": "{\"kind\": \"aws_sigv4\", \"secret\": \"aws_keys\", \"region\": \"us-east-1\"}"
}
```

The secret is stored as usual, with its fields named as the kind expects
(see [Manage Secrets](../../../../how-to/service/manage-secrets/)):

```bash
cloacinactl secret create orders-prod-aws \
  -f access_key_id -f secret_access_key=@/run/secrets/aws_secret_key
```

## When it fails

- A secret that does not exist, or a field it lacks, fails the read or write
  task. The error names the secret and the field, never a value.
- A request outside the grants is denied before anything is added, and the
  connector sees a denied request.
- A wrong key is the remote service's answer: S3 answers `403`, Snowflake
  `401`. A connector's `check` makes one cheap call, so a wrong key fails
  `check` before a sync runs.
- A credential that cannot be applied, such as a refused grant or a rotated
  refresh token, denies the request. The guest sees only "request denied";
  the reason is in the host's log, as a warning naming the credential and
  the URL, never a value.

## Related

- [Sync](../) for how a sync runs.
- [Sync Connectors](../../../../reference/connectors/) for the credential each
  connector takes.
- `cloacina::registry::loader::credentials` and
  `ResolvedGrants::with_credential` for the host side.
