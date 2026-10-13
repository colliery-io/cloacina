---
title: "cloacina::computation_graph::reactor_routing"
description: "Routing reactive events to the replica that owns the reactor (CLOACI-T-0851, [`ADR CLOACI-A-0012`] Decision item 2)."
editUrl: false
sidebar:
  order: 48
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Routing reactive events to the replica that owns the reactor (CLOACI-T-0851, [`ADR CLOACI-A-0012`] Decision item 2).

**How this works, and why there is no roster**

Under per-reactor leadership only ONE replica runs a given reactor and its
accumulators. An event injected over WS/REST lands on whichever replica
terminated the connection, which is frequently not the owner.

Rather than discovering the owner's address and forwarding point-to-point,
a non-owner writes the event to the **delivery outbox** addressed to the
accumulator. Every replica's relay drains that outbox and hands rows to its
sinks; the sink below tries to deliver into the LOCAL endpoint registry and
answers [`DeliveryOutcome::NoRoute`](../../../../rust/cloacina/delivery/#enum.DeliveryOutcome) when it cannot. The row then stays
pending for another replica's relay or the sweeper.

**The endpoint registry IS the routing table.** A replica has an accumulator
registered if and only if it runs the reactor hosting it — which, under
leadership, means it owns it. So "can I deliver this?" needs no ownership
query, no lock check, and no roster of peers: it is exactly "do I have this
accumulator". That also means routing cannot disagree with ownership, since
there is only one fact being consulted rather than two that could drift.

This is the mechanism the delivery substrate was built for —
`DeliveryOutcome::NoRoute` already documents itself as "how
connection-ownership routing falls out without the relay needing a roster".
Reactive events are simply another addressed recipient alongside
`agent:<uuid>` and `exec_events:<exec_id>`.

## Structs

### <span id="struct.AccumulatorDeliverySink"></span><code style="font-size: 0.85em">struct AccumulatorDeliverySink</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Delivers reactive events into this replica's accumulators, when it has them.

#### Methods

##### <span id="AccumulatorDeliverySink.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(registry: super::registry::EndpointRegistry) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(registry: super::registry::EndpointRegistry) -> Self {
        Self { registry }
    }
```

</details>

## Enums

### <span id="enum.InjectOutcome"></span><code style="font-size: 0.85em">enum InjectOutcome</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Where an inject ended up. Distinguishing these matters to callers: an operator injecting an event wants to know it was accepted, and "handed to another replica" is a different (and slower) success from "delivered here".

#### Variants

- **`Local`** - Delivered into a local accumulator on this replica.
- **`Forwarded`** - This replica does not host the accumulator, so the event was written to
  the delivery outbox for the owning replica to pick up.

## Functions

### <span id="fn.accumulator_recipient"></span><code style="font-size: 0.85em">fn accumulator_​recipient</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn accumulator_recipient(accumulator_name: &str) -> String
```

Build the outbox `recipient` key for an accumulator.

The tenant is NOT encoded here: outbox rows carry `tenant_id` as its own
column, and duplicating it in the recipient string would create two
spellings of one scope that could disagree.

<details>
<summary>Source</summary>

```rust
pub fn accumulator_recipient(accumulator_name: &str) -> String {
    format!("{ACCUMULATOR_RECIPIENT_PREFIX}{accumulator_name}")
}
```

</details>

### <span id="fn.parse_accumulator_recipient"></span><code style="font-size: 0.85em">fn parse_​accumulator_​recipient</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn parse_accumulator_recipient(recipient: &str) -> Option<&str>
```

Parse an accumulator recipient key, returning `None` for rows addressed to anything else.

Returning `None` rather than erroring matters: every replica's relay hands
EVERY pending row to EVERY sink, so this sink constantly sees rows belonging
to other subsystems. Those are not errors and must not be logged as such.

<details>
<summary>Source</summary>

```rust
pub fn parse_accumulator_recipient(recipient: &str) -> Option<&str> {
    recipient.strip_prefix(ACCUMULATOR_RECIPIENT_PREFIX)
}
```

</details>

### <span id="fn.inject_event"></span><code style="font-size: 0.85em">fn inject_​event</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn inject_event(
    registry: &super::registry::EndpointRegistry,
    dal: Option<&crate::dal::unified::DAL>,
    name: &str,
    tenant_id: Option<&str>,
    bytes: Vec<u8>,
) -> Result<InjectOutcome, super::registry::RegistryError>
```

Inject an event, forwarding to the owning replica when this one does not host the accumulator.

This is the producer half of [`AccumulatorDeliverySink`](../../../../rust/cloacina/computation_graph/reactor_routing/#struct.AccumulatorDeliverySink). Together they mean
an event injected anywhere reaches the single replica that owns the reactor.

`dal` is `None` for embedded / single-replica deployments, where there is no
outbox and no peer to forward to. There, a missing accumulator is still a
genuine error and is returned unchanged — a deployment with nowhere to
forward to must not be told its event was "forwarded".

<details>
<summary>Source</summary>

```rust
pub async fn inject_event(
    registry: &super::registry::EndpointRegistry,
    dal: Option<&crate::dal::unified::DAL>,
    name: &str,
    tenant_id: Option<&str>,
    bytes: Vec<u8>,
) -> Result<InjectOutcome, super::registry::RegistryError> {
    let scope = match tenant_id {
        Some(t) => super::registry::EndpointScope::tenant(t),
        None => super::registry::EndpointScope::untenanted(),
    };

    match registry
        .send_to_accumulator(name, scope, bytes.clone())
        .await
    {
        Ok(n) if n > 0 => Ok(InjectOutcome::Local(n)),
        // Either not hosted here, or hosted but nothing accepted. Both mean the
        // event has NOT been consumed on this replica, so it must go to the
        // outbox rather than be reported as accepted.
        Ok(_) | Err(super::registry::RegistryError::AccumulatorNotFound(_)) => {
            let Some(dal) = dal else {
                // No outbox: embedded/single-replica. The original error is the
                // truthful answer.
                return Err(super::registry::RegistryError::AccumulatorNotFound(
                    name.to_string(),
                ));
            };
            let row = crate::models::delivery_outbox::NewDeliveryOutbox {
                recipient: accumulator_recipient(name),
                kind: REACTOR_EVENT_KIND.to_string(),
                tenant_id: tenant_id.map(|t| t.to_string()),
                payload: bytes,
            };
            dal.delivery_outbox().enqueue(row).await.map_err(|e| {
                super::registry::RegistryError::AccumulatorNotFound(format!(
                    "{name}: not hosted here and forwarding to the owning replica failed: {e}"
                ))
            })?;
            tracing::debug!(
                accumulator = %name,
                tenant = ?tenant_id,
                "accumulator not hosted here; forwarded event to the owning replica via the outbox"
            );
            Ok(InjectOutcome::Forwarded)
        }
        // Ambiguity is a misconfiguration, not a routing miss — forwarding it
        // would just relocate the ambiguity to another replica.
        Err(e) => Err(e),
    }
}
```

</details>

## Constants

| Name | Type | Value | Description |
|------|------|-------|-------------|
| <span id="const.REACTOR_EVENT_KIND"></span>`REACTOR_EVENT_KIND` | `&str` | `"reactor_event"` | `kind` discriminator for reactive-event rows. |
