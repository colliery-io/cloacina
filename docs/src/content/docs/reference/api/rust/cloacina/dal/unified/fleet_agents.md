---
title: "cloacina::dal::unified::fleet_agents"
description: "Unified fleet-agent roster DAL (CLOACI-T-0916)."
editUrl: false
sidebar:
  order: 74
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Unified fleet-agent roster DAL (CLOACI-T-0916).

Persists execution-agent register/heartbeat state so every
cross-replica-relevant read — same-tenant selection, capacity views,
dead-agent reclaim eligibility — comes from ONE shared table instead of a
per-replica in-memory map. Liveness is heartbeat-recency: readers filter on
`last_heartbeat_at >= now - timeout` via [`FleetAgentDAL::list_live`](../../../../../rust/cloacina/dal/unified/fleet_agents/#FleetAgentDAL.list_live).

Dead-agent eviction ([`FleetAgentDAL::sweep_dead`](../../../../../rust/cloacina/dal/unified/fleet_agents/#FleetAgentDAL.sweep_dead)) claims each stale row
with a compare-and-set DELETE (`WHERE agent_id = ? AND last_heartbeat_at <
cutoff`), so when several replicas sweep concurrently exactly one owns each
evicted agent's reclaim.

Work-packet dispatch needs no connection locality: packets ride the
delivery-outbox substrate and are delivered by whichever replica holds the
agent's delivery WebSocket (connection-ownership routing, A-0006).

## Structs

### <span id="struct.FleetAgent"></span><code style="font-size: 0.85em">struct FleetAgent</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Domain view of a roster row.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `agent_id` | `String` |  |
| `tenant_id` | `Option<String>` |  |
| `target_triple` | `String` |  |
| `capabilities` | `Vec<String>` |  |
| `max_concurrency` | `u32` |  |
| `in_flight` | `u32` |  |
| `available_capacity` | `u32` |  |
| `registered_at` | `UniversalTimestamp` |  |
| `last_heartbeat_at` | `UniversalTimestamp` |  |

### <span id="struct.FleetAgentRegistration"></span><code style="font-size: 0.85em">struct FleetAgentRegistration</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Registration input (the fields an agent supplies on `POST /v1/agent/register`).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `agent_id` | `String` |  |
| `tenant_id` | `Option<String>` |  |
| `target_triple` | `String` |  |
| `capabilities` | `Vec<String>` |  |
| `max_concurrency` | `u32` |  |

### <span id="struct.FleetAgentDAL"></span><code style="font-size: 0.85em">struct FleetAgentDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Data access layer for the execution-agent fleet roster.

#### Methods

##### <span id="FleetAgentDAL.new"></span><code>new</code>

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

##### <span id="FleetAgentDAL.upsert_registration"></span><code>upsert_​registration</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn upsert_registration(
    &self,
    reg: FleetAgentRegistration,
) -> Result<(), ValidationError>
```

Insert or overwrite an agent's registration (overwrite handles clean re-registration after an agent restart with the same `agent_id`). Registration counts as a heartbeat.

<details>
<summary>Source</summary>

```rust
    pub async fn upsert_registration(
        &self,
        reg: FleetAgentRegistration,
    ) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        let row = NewUnifiedFleetAgent {
            agent_id: reg.agent_id,
            tenant_id: reg.tenant_id,
            target_triple: reg.target_triple,
            capabilities: serde_json::to_string(&reg.capabilities)
                .unwrap_or_else(|_| "[]".to_string()),
            max_concurrency: reg.max_concurrency.min(i32::MAX as u32) as i32,
            in_flight: 0,
            available_capacity: reg.max_concurrency.min(i32::MAX as u32) as i32,
            registered_at: now,
            last_heartbeat_at: now,
        };
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(fleet_agents::table)
                .values(&row)
                .on_conflict(fleet_agents::agent_id)
                .do_update()
                .set(&row)
                .execute(conn)
        })
        .map_err(ValidationError::from)?;
        Ok(())
    }
```

</details>

##### <span id="FleetAgentDAL.record_heartbeat"></span><code>record_​heartbeat</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn record_heartbeat(
    &self,
    agent_id: &str,
    in_flight: u32,
    available_capacity: u32,
) -> Result<bool, ValidationError>
```

Record a heartbeat: refresh `last_heartbeat_at` and the capacity fields. Returns `true` iff the agent is in the roster (`false` ⇒ the caller should tell the agent to re-register).

<details>
<summary>Source</summary>

```rust
    pub async fn record_heartbeat(
        &self,
        agent_id: &str,
        in_flight: u32,
        available_capacity: u32,
    ) -> Result<bool, ValidationError> {
        let agent_id = agent_id.to_string();
        let now = UniversalTimestamp::now();
        let affected = crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(fleet_agents::table.filter(fleet_agents::agent_id.eq(&agent_id)))
                .set((
                    fleet_agents::in_flight.eq(in_flight.min(i32::MAX as u32) as i32),
                    fleet_agents::available_capacity
                        .eq(available_capacity.min(i32::MAX as u32) as i32),
                    fleet_agents::last_heartbeat_at.eq(now),
                ))
                .execute(conn)
        })
        .map_err(ValidationError::from)?;
        Ok(affected == 1)
    }
```

</details>

##### <span id="FleetAgentDAL.agent_tenant"></span><code>agent_​tenant</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn agent_tenant(
    &self,
    agent_id: &str,
) -> Result<Option<Option<String>>, ValidationError>
```

The tenant an agent registered under, if it's in the roster. `Some(tenant)` when registered (the tenant may itself be `None` for a global agent); `None` when the agent is unknown.

<details>
<summary>Source</summary>

```rust
    pub async fn agent_tenant(
        &self,
        agent_id: &str,
    ) -> Result<Option<Option<String>>, ValidationError> {
        let agent_id = agent_id.to_string();
        let row: Option<Option<String>> = crate::interact_on_backend!(self.dal, |conn| {
            fleet_agents::table
                .filter(fleet_agents::agent_id.eq(&agent_id))
                .select(fleet_agents::tenant_id)
                .first::<Option<String>>(conn)
                .optional()
        })
        .map_err(ValidationError::from)?;
        Ok(row)
    }
```

</details>

##### <span id="FleetAgentDAL.list_live"></span><code>list_​live</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_live(
    &self,
    timeout: std::time::Duration,
) -> Result<Vec<FleetAgent>, ValidationError>
```

All agents whose heartbeat is fresher than `timeout` — the live fleet. Every cross-replica-relevant read (selection, capacity, operator views, autoscaler utilization) goes through this recency filter.

<details>
<summary>Source</summary>

```rust
    pub async fn list_live(
        &self,
        timeout: std::time::Duration,
    ) -> Result<Vec<FleetAgent>, ValidationError> {
        let cutoff = cutoff_for(timeout);
        let rows: Vec<UnifiedFleetAgent> = crate::interact_on_backend!(self.dal, |conn| {
            fleet_agents::table
                .filter(fleet_agents::last_heartbeat_at.ge(cutoff))
                .order(fleet_agents::agent_id.asc())
                .load(conn)
        })
        .map_err(ValidationError::from)?;
        Ok(rows.into_iter().map(to_domain).collect())
    }
```

</details>

##### <span id="FleetAgentDAL.sweep_dead"></span><code>sweep_​dead</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn sweep_dead(
    &self,
    timeout: std::time::Duration,
) -> Result<Vec<FleetAgent>, ValidationError>
```

Evict agents whose heartbeat is older than `timeout`, returning the rows THIS caller evicted. Per-row compare-and-set delete: when several replicas sweep concurrently, each stale agent is owned by exactly one sweeper — so dead-agent reclaim never double-fires.

<details>
<summary>Source</summary>

```rust
    pub async fn sweep_dead(
        &self,
        timeout: std::time::Duration,
    ) -> Result<Vec<FleetAgent>, ValidationError> {
        let cutoff = cutoff_for(timeout);
        let rows: Vec<UnifiedFleetAgent> = crate::interact_on_backend!(self.dal, |conn| {
            let stale: Vec<UnifiedFleetAgent> = fleet_agents::table
                .filter(fleet_agents::last_heartbeat_at.lt(cutoff))
                .load(conn)?;
            let mut owned = Vec::with_capacity(stale.len());
            for row in stale {
                // CAS delete: 1 row affected ⇒ we own this eviction (a
                // concurrent sweeper — or a last-instant heartbeat — makes
                // this 0 and we skip).
                let affected = diesel::delete(
                    fleet_agents::table
                        .filter(fleet_agents::agent_id.eq(&row.agent_id))
                        .filter(fleet_agents::last_heartbeat_at.lt(cutoff)),
                )
                .execute(conn)?;
                if affected == 1 {
                    owned.push(row);
                }
            }
            Ok::<_, diesel::result::Error>(owned)
        })
        .map_err(ValidationError::from)?;
        Ok(rows.into_iter().map(to_domain).collect())
    }
```

</details>

##### <span id="FleetAgentDAL.delete"></span><code>delete</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn delete(&self, agent_id: &str) -> Result<(), ValidationError>
```

Remove an agent from the roster. Idempotent.

<details>
<summary>Source</summary>

```rust
    pub async fn delete(&self, agent_id: &str) -> Result<(), ValidationError> {
        let agent_id = agent_id.to_string();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(fleet_agents::table.filter(fleet_agents::agent_id.eq(&agent_id)))
                .execute(conn)
        })
        .map_err(ValidationError::from)?;
        Ok(())
    }
```

</details>
