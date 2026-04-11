# cloacina::dal::unified::api_keys <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


API key DAL — Postgres only.

The server uses API keys for authentication. Keys are stored as SHA-256
hashes — plaintext is never persisted. This module provides CRUD operations
for the `api_keys` table.

## Structs

### `cloacina::dal::unified::api_keys::ApiKeyInfo`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::api_keys::ApiKeyDAL`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

DAL for API key operations. Postgres only.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |
