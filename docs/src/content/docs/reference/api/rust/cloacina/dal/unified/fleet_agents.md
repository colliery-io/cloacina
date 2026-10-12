---
title: "cloacina::dal::unified::fleet_agents"
description: "Unified fleet-agent roster DAL (CLOACI-T-0916)."
sidebar:
  order: 90
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Unified fleet-agent roster DAL (CLOACI-T-0916).

Persists execution-agent register/heartbeat state so every
cross-replica-relevant read — same-tenant selection, capacity views,
dead-agent reclaim eligibility — comes from ONE shared table instead of a
per-replica in-memory map. Liveness is heartbeat-recency: readers filter on
`last_heartbeat_at >= now - timeout` via [`FleetAgentDAL::list_live`].
Dead-agent eviction ([`FleetAgentDAL::sweep_dead`]) claims each stale row
with a compare-and-set DELETE (`WHERE agent_id = ? AND last_heartbeat_at <
cutoff`), so when several replicas sweep concurrently exactly one owns each
evicted agent's reclaim.
Work-packet dispatch needs no connection locality: packets ride the
delivery-outbox substrate and are delivered by whichever replica holds the
agent's delivery WebSocket (connection-ownership routing, A-0006).

## Structs

### <span id="struct.FleetAgent"></span>`struct FleetAgent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`

Domain view of a roster row.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `agent_id` | `String` |  |
| `tenant_id` | `Option < String >` |  |
| `target_triple` | `String` |  |
| `capabilities` | `Vec < String >` |  |
| `max_concurrency` | `u32` |  |
| `in_flight` | `u32` |  |
| `available_capacity` | `u32` |  |
| `registered_at` | `UniversalTimestamp` |  |
| `last_heartbeat_at` | `UniversalTimestamp` |  |



### <span id="struct.FleetAgentRegistration"></span>`struct FleetAgentRegistration`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`

Registration input (the fields an agent supplies on `POST /v1/agent/register`).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `agent_id` | `String` |  |
| `tenant_id` | `Option < String >` |  |
| `target_triple` | `String` |  |
| `capabilities` | `Vec < String >` |  |
| `max_concurrency` | `u32` |  |



### <span id="struct.FleetAgentDAL"></span>`struct FleetAgentDAL<'a>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Clone`

Data access layer for the execution-agent fleet roster.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



## Functions

### <span id="fn.to_domain"></span>`fn to_domain`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn to_domain (r : UnifiedFleetAgent) -> FleetAgent
```

<details>
<summary>Source</summary>

```rust
fn to_domain(r: UnifiedFleetAgent) -> FleetAgent {
    FleetAgent {
        agent_id: r.agent_id,
        tenant_id: r.tenant_id,
        target_triple: r.target_triple,
        capabilities: serde_json::from_str(&r.capabilities).unwrap_or_default(),
        max_concurrency: r.max_concurrency.max(0) as u32,
        in_flight: r.in_flight.max(0) as u32,
        available_capacity: r.available_capacity.max(0) as u32,
        registered_at: r.registered_at,
        last_heartbeat_at: r.last_heartbeat_at,
    }
}
```

</details>



### <span id="fn.cutoff_for"></span>`fn cutoff_for`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn cutoff_for (timeout : std :: time :: Duration) -> UniversalTimestamp
```

<details>
<summary>Source</summary>

```rust
fn cutoff_for(timeout: std::time::Duration) -> UniversalTimestamp {
    let now = UniversalTimestamp::now();
    UniversalTimestamp(
        now.0
            - chrono::Duration::from_std(timeout).unwrap_or_else(|_| chrono::Duration::seconds(0)),
    )
}
```

</details>



