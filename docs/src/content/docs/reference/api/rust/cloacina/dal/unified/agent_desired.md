---
title: "cloacina::dal::unified::agent_desired"
description: "Agent desired count — per-tenant self-service provisioning state (CLOACI-T-0809, CLOACI-I-0127). **Postgres only.**"
editUrl: false
sidebar:
  order: 66
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Agent desired count — per-tenant self-service provisioning state (CLOACI-T-0809, CLOACI-I-0127). **Postgres only.**

`desired_count` is the number of agents a tenant has requested. It is tenant
self-service provisioning state, bounded by the god-set `effective_limit`
from T-0808 (`AgentLimitsDAL`): the provision API increments it (+1) only
while under the limit, deprovision decrements it (−1, floor 0). This is the
operational target the actuator (T-0810) and the back-pressure autoscaler
(T-0811) reconcile/clamp to. Absent → 0 (no agents requested yet).

## Structs

### <span id="struct.AgentDesiredDAL"></span><code style="font-size: 0.85em">struct AgentDesiredDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

DAL for per-tenant desired agent count. Postgres only.

#### Methods

##### <span id="AgentDesiredDAL.new"></span><code>new</code>

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

##### <span id="AgentDesiredDAL.get_desired"></span><code>get_​desired</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_desired(
    &self,
    tenant_id: &str,
) -> Result<u32, ValidationError>
```

The tenant's desired agent count (`0` if no row is set yet). This is the provisioning target the actuator/autoscaler reconcile to.

<details>
<summary>Source</summary>

```rust
    pub async fn get_desired(&self, tenant_id: &str) -> Result<u32, ValidationError> {
        use crate::database::schema::postgres::agent_desired_counts as t;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let tenant = tenant_id.to_string();
        let row: Option<AgentDesiredRow> = conn
            .interact(move |conn| t::table.find(tenant).first(conn).optional())
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(row.map(|r| r.desired_count.max(0) as u32).unwrap_or(0))
    }
```

</details>

##### <span id="AgentDesiredDAL.list_all"></span><code>list_​all</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_all(&self) -> Result<Vec<(String, u32)>, ValidationError>
```

List every tenant's desired agent count as `(tenant_id, desired_count)` pairs. Used by the fleet actuator's reconcile loop (CLOACI-T-0810) to drive actual→desired convergence across all provisioned tenants. Rows with a non-positive count are clamped to `0`.

<details>
<summary>Source</summary>

```rust
    pub async fn list_all(&self) -> Result<Vec<(String, u32)>, ValidationError> {
        use crate::database::schema::postgres::agent_desired_counts as t;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let rows: Vec<AgentDesiredRow> = conn
            .interact(move |conn| t::table.load(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(rows
            .into_iter()
            .map(|r| (r.tenant_id, r.desired_count.max(0) as u32))
            .collect())
    }
```

</details>

##### <span id="AgentDesiredDAL.list_all_with_last"></span><code>list_​all_​with_​last</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_all_with_last(
    &self,
) -> Result<Vec<(String, u32, Option<chrono::NaiveDateTime>)>, ValidationError>
```

Like `list_all`, but also returns each tenant's `last_autoscaled_at` (the wall-clock of its last autoscaler scale action, `None` if never autoscaled). The back-pressure control loop (T-0811) uses this to gate the cooldown across replicas: leadership rotates per tick, so an in-memory per-replica cooldown could be bypassed by a later leader — the DB timestamp holds regardless of which replica leads (CLOACI-A-0008).

<details>
<summary>Source</summary>

```rust
    pub async fn list_all_with_last(
        &self,
    ) -> Result<Vec<(String, u32, Option<chrono::NaiveDateTime>)>, ValidationError> {
        use crate::database::schema::postgres::agent_desired_counts as t;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let rows: Vec<AgentDesiredRow> = conn
            .interact(move |conn| t::table.load(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.tenant_id,
                    r.desired_count.max(0) as u32,
                    r.last_autoscaled_at,
                )
            })
            .collect())
    }
```

</details>

##### <span id="AgentDesiredDAL.set_desired"></span><code>set_​desired</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn set_desired(
    &self,
    tenant_id: &str,
    desired_count: u32,
) -> Result<(), ValidationError>
```

Set (or replace) the tenant's desired agent count. Upserts on `tenant_id`; `updated_at` is managed by the column default (not written from Rust).

This is the **manual** write (provision/deprovision API): it deliberately
leaves `last_autoscaled_at` untouched, so manual scaling does NOT arm the
autoscaler cooldown. The autoscaler uses
`set_desired_autoscaled` instead.

<details>
<summary>Source</summary>

```rust
    pub async fn set_desired(
        &self,
        tenant_id: &str,
        desired_count: u32,
    ) -> Result<(), ValidationError> {
        use crate::database::schema::postgres::agent_desired_counts as t;
        let tenant = tenant_id.to_string();
        let desired = desired_count as i32;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        conn.interact(move |conn| {
            diesel::insert_into(t::table)
                .values((t::tenant_id.eq(&tenant), t::desired_count.eq(desired)))
                .on_conflict(t::tenant_id)
                .do_update()
                .set(t::desired_count.eq(desired))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(())
    }
```

</details>

##### <span id="AgentDesiredDAL.set_desired_autoscaled"></span><code>set_​desired_​autoscaled</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn set_desired_autoscaled(
    &self,
    tenant_id: &str,
    desired_count: u32,
) -> Result<(), ValidationError>
```

The **autoscaler's** write: set the tenant's desired agent count AND stamp `last_autoscaled_at = now()` (SQL `now()`, not a Rust-written timestamp — mirroring the `updated_at`-via-DB pattern). Upserts on `tenant_id`.

Stamping the wall-clock here is what makes the back-pressure cooldown
(T-0811) hold across replicas: the loop reads `last_autoscaled_at` back
and gates `should_act_at`, so a replica leading a later tick still honors
a peer's recent scale action (CLOACI-A-0008 refinement). Manual
provision/deprovision uses `set_desired` instead and
does NOT arm this cooldown.

<details>
<summary>Source</summary>

```rust
    pub async fn set_desired_autoscaled(
        &self,
        tenant_id: &str,
        desired_count: u32,
    ) -> Result<(), ValidationError> {
        use crate::database::schema::postgres::agent_desired_counts as t;
        let tenant = tenant_id.to_string();
        let desired = desired_count as i32;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        conn.interact(move |conn| {
            diesel::insert_into(t::table)
                .values((
                    t::tenant_id.eq(&tenant),
                    t::desired_count.eq(desired),
                    t::last_autoscaled_at.eq(diesel::dsl::now),
                ))
                .on_conflict(t::tenant_id)
                .do_update()
                .set((
                    t::desired_count.eq(desired),
                    t::last_autoscaled_at.eq(diesel::dsl::now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(())
    }
```

</details>
