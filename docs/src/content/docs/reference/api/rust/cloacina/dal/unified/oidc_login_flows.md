---
title: "cloacina::dal::unified::oidc_login_flows"
description: "OIDC in-flight login state (CLOACI-T-0801). **Postgres only.**"
sidebar:
  order: 94
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


OIDC in-flight login state (CLOACI-T-0801). **Postgres only.**

Persists the authorization-code flow's `state -> (nonce, pkce_verifier)` so
the callback can land on any replica (NFR-003, no sticky sessions). Single
use: [`take`](OidcLoginFlowDAL::take) deletes-and-returns in one statement,
and only for an unexpired row, so a replayed/expired/unknown `state` yields
`None` (the callback then fails closed).

## Structs

### <span id="struct.NewLoginFlow"></span>`struct NewLoginFlow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


**Derives:** `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `state` | `String` |  |
| `nonce` | `String` |  |
| `pkce_verifier` | `String` |  |
| `expires_at` | `chrono :: NaiveDateTime` |  |



### <span id="struct.OidcLoginFlowDAL"></span>`struct OidcLoginFlowDAL<'a>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


DAL for the OIDC login-flow state. Postgres only.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



