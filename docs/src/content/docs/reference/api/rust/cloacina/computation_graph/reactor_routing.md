---
title: "cloacina::computation_graph::reactor_routing"
description: "Routing reactive events to the replica that owns the reactor (CLOACI-T-0851, [`ADR CLOACI-A-0012`] Decision item 2)."
sidebar:
  order: 64
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Routing reactive events to the replica that owns the reactor (CLOACI-T-0851, [`ADR CLOACI-A-0012`] Decision item 2).

## Structs

### <span id="struct.AccumulatorDeliverySink"></span>`struct AccumulatorDeliverySink`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Delivers reactive events into this replica's accumulators, when it has them.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `registry` | `super :: registry :: EndpointRegistry` |  |

#### Methods

##### <span id="AccumulatorDeliverySink.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn new (registry : super :: registry :: EndpointRegistry) -> Self
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

### <span id="enum.InjectOutcome"></span>`enum InjectOutcome`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Where an inject ended up. Distinguishing these matters to callers: an operator injecting an event wants to know it was accepted, and "handed to another replica" is a different (and slower) success from "delivered here".

#### Variants

- **`Local`** - Delivered into a local accumulator on this replica.
- **`Forwarded`** - This replica does not host the accumulator, so the event was written to
the delivery outbox for the owning replica to pick up.



## Functions

### <span id="fn.accumulator_recipient"></span>`fn accumulator_recipient`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn accumulator_recipient (accumulator_name : & str) -> String
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



### <span id="fn.parse_accumulator_recipient"></span>`fn parse_accumulator_recipient`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn parse_accumulator_recipient (recipient : & str) -> Option < & str >
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



### <span id="fn.inject_event"></span>`fn inject_event`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
async fn inject_event (registry : & super :: registry :: EndpointRegistry , dal : Option < & crate :: dal :: unified :: DAL > , name : & str , tenant_id : Option < & str > , bytes : Vec < u8 > ,) -> Result < InjectOutcome , super :: registry :: RegistryError >
```

Inject an event, forwarding to the owning replica when this one does not host the accumulator.

This is the producer half of [`AccumulatorDeliverySink`]. Together they mean
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



