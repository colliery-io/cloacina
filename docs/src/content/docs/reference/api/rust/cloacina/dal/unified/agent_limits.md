---
title: "cloacina::dal::unified::agent_limits"
description: "Agent capacity limits — per-tenant exceptions to the global default (CLOACI-T-0808, CLOACI-I-0127). **Postgres only.**"
editUrl: false
sidebar:
  order: 67
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Agent capacity limits — per-tenant exceptions to the global default (CLOACI-T-0808, CLOACI-I-0127). **Postgres only.**

The global default (`CLOACINA_DEFAULT_MAX_AGENTS`) is server config; this DAL
stores ONLY the per-tenant overrides an admin grants. `effective_limit` is the
override if present, else the default — the hard ceiling that the provision
API (T-0809) and the back-pressure autoscaler (T-0811) clamp to. Setting an
exception is a god-only op; a tenant may read its own effective limit.

## Structs

### <span id="struct.AgentLimitsDAL"></span><code style="font-size: 0.85em">struct AgentLimitsDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

DAL for per-tenant agent-capacity limits. Postgres only.

#### Methods

##### <span id="AgentLimitsDAL.new"></span><code>new</code>

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

##### <span id="AgentLimitsDAL.set_tenant_limit"></span><code>set_​tenant_​limit</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn set_tenant_limit(
    &self,
    tenant_id: &str,
    max_agents: u32,
) -> Result<(), ValidationError>
```

Set (or replace) a tenant's agent-capacity exception (god-only op, T-0808). Upserts on `tenant_id`; `created_at`/`updated_at` are managed by the column defaults.

<details>
<summary>Source</summary>

```rust
    pub async fn set_tenant_limit(
        &self,
        tenant_id: &str,
        max_agents: u32,
    ) -> Result<(), ValidationError> {
        use crate::database::schema::postgres::agent_capacity_limits as t;
        let tenant = tenant_id.to_string();
        let max = max_agents as i32;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        conn.interact(move |conn| {
            diesel::insert_into(t::table)
                .values((t::tenant_id.eq(&tenant), t::max_agents.eq(max)))
                .on_conflict(t::tenant_id)
                .do_update()
                .set(t::max_agents.eq(max))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(())
    }
```

</details>

##### <span id="AgentLimitsDAL.get_tenant_limit"></span><code>get_​tenant_​limit</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_tenant_limit(
    &self,
    tenant_id: &str,
) -> Result<Option<u32>, ValidationError>
```

The tenant's exception, if one is set (`None` → the default applies).

<details>
<summary>Source</summary>

```rust
    pub async fn get_tenant_limit(&self, tenant_id: &str) -> Result<Option<u32>, ValidationError> {
        use crate::database::schema::postgres::agent_capacity_limits as t;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let tenant = tenant_id.to_string();
        let row: Option<AgentLimitRow> = conn
            .interact(move |conn| t::table.find(tenant).first(conn).optional())
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(row.map(|r| r.max_agents.max(0) as u32))
    }
```

</details>

##### <span id="AgentLimitsDAL.clear_tenant_limit"></span><code>clear_​tenant_​limit</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn clear_tenant_limit(
    &self,
    tenant_id: &str,
) -> Result<bool, ValidationError>
```

Remove a tenant's exception (revert to the default). Returns true if a row was removed.

<details>
<summary>Source</summary>

```rust
    pub async fn clear_tenant_limit(&self, tenant_id: &str) -> Result<bool, ValidationError> {
        use crate::database::schema::postgres::agent_capacity_limits as t;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let tenant = tenant_id.to_string();
        let n: usize = conn
            .interact(move |conn| diesel::delete(t::table.find(tenant)).execute(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(n > 0)
    }
```

</details>

##### <span id="AgentLimitsDAL.effective_limit"></span><code>effective_​limit</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn effective_limit(
    &self,
    tenant_id: &str,
    default: u32,
) -> Result<u32, ValidationError>
```

The effective limit for a tenant: the exception if set, else `default`. This is the value the provision API and autoscaler enforce.

<details>
<summary>Source</summary>

```rust
    pub async fn effective_limit(
        &self,
        tenant_id: &str,
        default: u32,
    ) -> Result<u32, ValidationError> {
        Ok(self.get_tenant_limit(tenant_id).await?.unwrap_or(default))
    }
```

</details>
