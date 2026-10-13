---
title: "cloacina::dal::unified::oidc_login_flows"
description: "OIDC in-flight login state (CLOACI-T-0801). **Postgres only.**"
editUrl: false
sidebar:
  order: 78
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

OIDC in-flight login state (CLOACI-T-0801). **Postgres only.**

Persists the authorization-code flow's `state -> (nonce, pkce_verifier)` so
the callback can land on any replica (NFR-003, no sticky sessions). Single
use: [`take`](../../../../../rust/cloacina/dal/unified/oidc_login_flows/#OidcLoginFlowDAL.take) deletes-and-returns in one statement,
and only for an unexpired row, so a replayed/expired/unknown `state` yields
`None` (the callback then fails closed).

## Structs

### <span id="struct.OidcLoginFlowDAL"></span><code style="font-size: 0.85em">struct OidcLoginFlowDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

DAL for the OIDC login-flow state. Postgres only.

#### Methods

##### <span id="OidcLoginFlowDAL.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: &'a DAL) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }
```

</details>

##### <span id="OidcLoginFlowDAL.put"></span><code>put</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn put(
    &self,
    state: String,
    nonce: String,
    pkce_verifier: String,
    expires_at: DateTime<Utc>,
) -> Result<(), ValidationError>
```

Stash an in-flight login keyed by `state`, valid until `expires_at`.

<details>
<summary>Source</summary>

```rust
    pub async fn put(
        &self,
        state: String,
        nonce: String,
        pkce_verifier: String,
        expires_at: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let row = NewLoginFlow {
            state,
            nonce,
            pkce_verifier,
            expires_at: expires_at.naive_utc(),
        };
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        conn.interact(move |conn| {
            diesel::insert_into(crate::database::schema::postgres::oidc_login_flows::table)
                .values(&row)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(())
    }
```

</details>

##### <span id="OidcLoginFlowDAL.take"></span><code>take</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn take(
    &self) -> Result<Option<(String, String,
    state: &str) -> Result<Option<(String, String,
) -> Result<Option<(String, String)>, ValidationError>
```

Consume the flow for `state` (single use): delete-and-return its `(nonce, pkce_verifier)` iff the row exists and has not expired. A missing/expired/replayed `state` returns `None`.

<details>
<summary>Source</summary>

```rust
    pub async fn take(&self, state: &str) -> Result<Option<(String, String)>, ValidationError> {
        use crate::database::schema::postgres::oidc_login_flows as t;
        let state = state.to_string();
        let now = Utc::now().naive_utc();
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let row: Option<(String, String)> = conn
            .interact(move |conn| {
                diesel::delete(t::table.filter(t::state.eq(state).and(t::expires_at.gt(now))))
                    .returning((t::nonce, t::pkce_verifier))
                    .get_result(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(row)
    }
```

</details>

##### <span id="OidcLoginFlowDAL.sweep_expired"></span><code>sweep_​expired</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn sweep_expired(&self) -> Result<usize, ValidationError>
```

Delete all flows whose `expires_at` has passed. Returns the count. Run periodically by the server sweeper.

<details>
<summary>Source</summary>

```rust
    pub async fn sweep_expired(&self) -> Result<usize, ValidationError> {
        use crate::database::schema::postgres::oidc_login_flows as t;
        let now = Utc::now().naive_utc();
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let n: usize = conn
            .interact(move |conn| {
                diesel::delete(t::table.filter(t::expires_at.lt(now))).execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(n)
    }
```

</details>
