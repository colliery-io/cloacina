---
title: "cloacina::dal::unified::api_keys"
description: "API key DAL — Postgres only."
sidebar:
  order: 84
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


API key DAL — Postgres only.

The server uses API keys for authentication. Keys are stored as SHA-256
hashes — plaintext is never persisted. This module provides CRUD operations
for the `api_keys` table.

## Structs

### <span id="struct.ApiKeyInfo"></span>`struct ApiKeyInfo`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`

Information about an API key (never includes the hash).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `uuid :: Uuid` |  |
| `name` | `String` |  |
| `permissions` | `String` |  |
| `created_at` | `chrono :: DateTime < chrono :: Utc >` |  |
| `revoked` | `bool` |  |
| `tenant_id` | `Option < String >` |  |
| `is_admin` | `bool` |  |
| `expires_at` | `Option < chrono :: DateTime < chrono :: Utc > >` | CLOACI-T-0792: minted-key expiry (`None` for manual keys = no expiry). |
| `issued_via` | `Option < String >` | CLOACI-T-0792: minted-key provenance (e.g. `oidc:<issuer>:<sub>` /
`local:<account_id>`; `None` for manual keys). |



### <span id="struct.ApiKeyDAL"></span>`struct ApiKeyDAL<'a>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Clone`

DAL for API key operations. Postgres only.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



