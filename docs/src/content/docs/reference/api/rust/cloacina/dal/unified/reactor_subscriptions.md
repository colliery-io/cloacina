---
title: "cloacina::dal::unified::reactor_subscriptions"
description: "Reactor-triggered workflow subscriptions — DB-backed event log + fan-out."
editUrl: false
sidebar:
  order: 81
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Reactor-triggered workflow subscriptions — DB-backed event log + fan-out.

Implements the data layer for CLOACI-I-0100. Two tables:

- `reactor_firings` — append-only log written by the reactor runtime
  on every fire. Each row carries the same boundary cache the
  in-process CG traversal consumed.
- `reactor_trigger_subscriptions` — one row per (reactor, workflow,
  tenant) tuple. The poller advances `last_seen_fired_at` as it
  dispatches workflows from new firings.

Watermark advance is the at-least-once contract: if the dispatcher
crashes between dispatch and watermark advance, the next poll
re-dispatches. Workflow idempotency is the user's concern (same as
cron-triggered workflows).

## Structs

### <span id="struct.ReactorFiring"></span><code style="font-size: 0.85em">struct ReactorFiring</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Queryable`

One reactor firing event. Carries the boundary cache payload the in-process CG traversal consumed; subscribers receive the same data as their workflow's input context.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `reactor_name` | `String` |  |
| `tenant_id` | `String` |  |
| `payload` | `Option<UniversalBinary>` |  |
| `fired_at` | `UniversalTimestamp` |  |
| `created_at` | `UniversalTimestamp` |  |

### <span id="struct.ReactorSubscription"></span><code style="font-size: 0.85em">struct ReactorSubscription</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Queryable`

One subscription binding a workflow to a reactor's firings.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `reactor_name` | `String` |  |
| `workflow_name` | `String` |  |
| `tenant_id` | `String` |  |
| `enabled` | `UniversalBool` |  |
| `last_seen_fired_at` | `Option<UniversalTimestamp>` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |
| `predicate_expression` | `Option<String>` | CLOACI-T-0602 — optional CEL filter expression. When `Some`, the scheduler evaluates it against the firing payload before dispatch; `Some(_) && false` means "skip + advance watermark". `None` preserves the original unfiltered behavior (fire on every firing). |
| `predicate_error_count` | `i32` | CLOACI-T-0922 — consecutive predicate *evaluation errors* on the current head-of-line firing. Reset to 0 when the firing changes or a later evaluation succeeds. |
| `predicate_error_firing_id` | `Option<UniversalUuid>` | The firing `predicate_error_count` applies to. |
| `last_predicate_error` | `Option<String>` | Truncated text of the most recent predicate evaluation error. Never cleared on recovery — it is the forensic trail. |
| `last_predicate_error_at` | `Option<UniversalTimestamp>` | When `last_predicate_error` was recorded. |
| `predicate_degraded` | `UniversalBool` | True once a firing was dead-lettered (the consecutive-error bound was exceeded and the watermark was force-advanced past it). Cleared by the next successful predicate evaluation. |

### <span id="struct.ReactorSubscriptionsDAL"></span><code style="font-size: 0.85em">struct ReactorSubscriptionsDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Data access layer for reactor subscriptions + firings.

#### Methods

##### <span id="ReactorSubscriptionsDAL.new"></span><code>new</code>

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

##### <span id="ReactorSubscriptionsDAL.insert_firing"></span><code>insert_​firing</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn insert_firing(
    &self,
    reactor: &str,
    tenant: &str,
    payload: Option<Vec<u8>>,
    fired_at: UniversalTimestamp,
) -> Result<Uuid, ValidationError>
```

Insert a firing row. Called by the reactor runtime on every fire; best-effort from the caller's perspective (a DAL failure is logged but doesn't fail the in-process CG dispatch).

<details>
<summary>Source</summary>

```rust
    pub async fn insert_firing(
        &self,
        reactor: &str,
        tenant: &str,
        payload: Option<Vec<u8>>,
        fired_at: UniversalTimestamp,
    ) -> Result<Uuid, ValidationError> {
        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();
        let reactor = reactor.to_string();
        let tenant = tenant.to_string();
        let id_for_move = id;
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(reactor_firings::table)
                .values((
                    reactor_firings::id.eq(id_for_move),
                    reactor_firings::reactor_name.eq(reactor),
                    reactor_firings::tenant_id.eq(tenant),
                    reactor_firings::payload.eq(payload.map(UniversalBinary::new)),
                    reactor_firings::fired_at.eq(fired_at),
                    reactor_firings::created_at.eq(now),
                ))
                .execute(conn)
        })?;
        Ok(id.0)
    }
```

</details>

##### <span id="ReactorSubscriptionsDAL.poll_unconsumed"></span><code>poll_​unconsumed</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn poll_unconsumed(
    &self,
    tenant: &str,
    reactor: &str,
    after: Option<UniversalTimestamp>,
    limit: i64,
) -> Result<Vec<ReactorFiring>, ValidationError>
```

Poll firings for a subscription. Returns rows strictly newer than `after`, in `fired_at` order, capped at `limit`. The caller advances the watermark as it dispatches each row.

<details>
<summary>Source</summary>

```rust
    pub async fn poll_unconsumed(
        &self,
        tenant: &str,
        reactor: &str,
        after: Option<UniversalTimestamp>,
        limit: i64,
    ) -> Result<Vec<ReactorFiring>, ValidationError> {
        let tenant = tenant.to_string();
        let reactor = reactor.to_string();
        let rows: Vec<ReactorFiring> = crate::interact_on_backend!(self.dal, |conn| {
            let mut q = reactor_firings::table
                .filter(reactor_firings::tenant_id.eq(tenant))
                .filter(reactor_firings::reactor_name.eq(reactor))
                .into_boxed();
            if let Some(after) = after {
                q = q.filter(reactor_firings::fired_at.gt(after));
            }
            q.order(reactor_firings::fired_at.asc())
                .limit(limit)
                .load::<ReactorFiring>(conn)
        })?;
        Ok(rows)
    }
```

</details>

##### <span id="ReactorSubscriptionsDAL.prune_firings_older_than"></span><code>prune_​firings_​older_​than</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn prune_firings_older_than(
    &self,
    cutoff: UniversalTimestamp,
) -> Result<usize, ValidationError>
```

TTL prune. Deletes firings whose `fired_at` is older than the cutoff. Returns the row count deleted.

<details>
<summary>Source</summary>

```rust
    pub async fn prune_firings_older_than(
        &self,
        cutoff: UniversalTimestamp,
    ) -> Result<usize, ValidationError> {
        let n = crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(reactor_firings::table.filter(reactor_firings::fired_at.lt(cutoff)))
                .execute(conn)
        })?;
        Ok(n)
    }
```

</details>

##### <span id="ReactorSubscriptionsDAL.subscribe"></span><code>subscribe</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn subscribe(
    &self,
    reactor: &str,
    workflow: &str,
    tenant: &str,
    predicate: Option<&str>,
) -> Result<Uuid, ValidationError>
```

Create a subscription. Idempotent: calling twice with the same `(reactor, workflow, tenant)` upserts; the second call's `predicate` (if any) replaces the first one's.

`predicate` is an optional CEL expression (CLOACI-T-0602). When
`Some(_)`, the expression is compiled at subscribe time and any
syntax error is returned as a `ValidationError` before the row is
written, so a bad expression never lands in the DB. The scheduler
re-compiles + caches at dispatch time.

<details>
<summary>Source</summary>

```rust
    pub async fn subscribe(
        &self,
        reactor: &str,
        workflow: &str,
        tenant: &str,
        predicate: Option<&str>,
    ) -> Result<Uuid, ValidationError> {
        if let Some(expr) = predicate {
            // Compile-time validation: reject malformed expressions before
            // they reach the DB. Cheap (single parse), centralizes the
            // error message at the API boundary.
            compile_predicate(expr).map_err(ValidationError::InvalidPredicate)?;
            // CLOACI-T-0922 — and reject well-formed expressions that
            // reference variables nobody binds. Those compile fine and then
            // silently never fire (fail-closed), which is exactly the class
            // of bug the `tenant` stub incident (T-0915) produced.
            lint_predicate_variables(expr).map_err(ValidationError::InvalidPredicate)?;
        }
        let predicate = predicate.map(str::to_string);
        crate::dispatch_backend!(
            self.dal.backend(),
            self.subscribe_postgres(reactor, workflow, tenant, predicate.clone())
                .await,
            self.subscribe_sqlite(reactor, workflow, tenant, predicate)
                .await
        )
    }
```

</details>

##### <span id="ReactorSubscriptionsDAL.advance_watermark"></span><code>advance_​watermark</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn advance_watermark(
    &self,
    subscription_id: Uuid,
    new_last_seen: UniversalTimestamp,
) -> Result<(), ValidationError>
```

Advance the watermark for a subscription. Caller is the dispatcher loop; the watermark advances after each row is dispatched (at-least-once on crash).

<details>
<summary>Source</summary>

```rust
    pub async fn advance_watermark(
        &self,
        subscription_id: Uuid,
        new_last_seen: UniversalTimestamp,
    ) -> Result<(), ValidationError> {
        let sid = UniversalUuid(subscription_id);
        let now = UniversalTimestamp::now();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                reactor_trigger_subscriptions::table
                    .filter(reactor_trigger_subscriptions::id.eq(sid)),
            )
            .set((
                reactor_trigger_subscriptions::last_seen_fired_at.eq(Some(new_last_seen)),
                reactor_trigger_subscriptions::updated_at.eq(now),
            ))
            .execute(conn)
        })?;
        Ok(())
    }
```

</details>

##### <span id="ReactorSubscriptionsDAL.record_predicate_error"></span><code>record_​predicate_​error</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn record_predicate_error(
    &self,
    subscription_id: Uuid,
    firing_id: UniversalUuid,
    error: &str,
) -> Result<i32, ValidationError>
```

CLOACI-T-0922 — record a predicate evaluation error against a subscription and return the new consecutive-error count for `firing_id`.

The count is per (subscription, firing): if `firing_id` differs
from the firing the stored count applies to, the count restarts at
1. Because the watermark is held on error, the same firing stays
head-of-line until it either evaluates cleanly or gets
dead-lettered, so "consecutive errors on this firing" is exactly
"attempts spent on this firing".

Read-modify-write on a single pooled connection. The reactor poller
is the only writer of these columns and processes a subscription
serially, so no lock is needed; a lost update under a hypothetical
second poller would only make the bound approximate, never unsafe.

<details>
<summary>Source</summary>

```rust
    pub async fn record_predicate_error(
        &self,
        subscription_id: Uuid,
        firing_id: UniversalUuid,
        error: &str,
    ) -> Result<i32, ValidationError> {
        let sid = UniversalUuid(subscription_id);
        let now = UniversalTimestamp::now();
        let error = truncate_predicate_text(error);
        let count = crate::interact_on_backend!(self.dal, |conn| {
            let (prev_count, prev_firing): (i32, Option<UniversalUuid>) =
                reactor_trigger_subscriptions::table
                    .filter(reactor_trigger_subscriptions::id.eq(sid))
                    .select((
                        reactor_trigger_subscriptions::predicate_error_count,
                        reactor_trigger_subscriptions::predicate_error_firing_id,
                    ))
                    .first(conn)?;
            let next = if prev_firing == Some(firing_id) {
                prev_count.saturating_add(1)
            } else {
                1
            };
            diesel::update(
                reactor_trigger_subscriptions::table
                    .filter(reactor_trigger_subscriptions::id.eq(sid)),
            )
            .set((
                reactor_trigger_subscriptions::predicate_error_count.eq(next),
                reactor_trigger_subscriptions::predicate_error_firing_id.eq(Some(firing_id)),
                reactor_trigger_subscriptions::last_predicate_error.eq(Some(error)),
                reactor_trigger_subscriptions::last_predicate_error_at.eq(Some(now)),
                reactor_trigger_subscriptions::updated_at.eq(now),
            ))
            .execute(conn)?;
            Ok::<i32, diesel::result::Error>(next)
        })?;
        Ok(count)
    }
```

</details>

##### <span id="ReactorSubscriptionsDAL.mark_predicate_degraded"></span><code>mark_​predicate_​degraded</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn mark_predicate_degraded(
    &self,
    subscription_id: Uuid,
    firing_id: UniversalUuid,
    error: &str,
) -> Result<(), ValidationError>
```

CLOACI-T-0922 — mark a subscription degraded because a firing was dead-lettered: its predicate errored more times than the bound allows, so the caller is about to force-advance the watermark past it. The error text + firing id stay on the row as the durable record of what was dropped.

Resets the consecutive-error counter so the NEXT firing starts with
a full retry budget.

<details>
<summary>Source</summary>

```rust
    pub async fn mark_predicate_degraded(
        &self,
        subscription_id: Uuid,
        firing_id: UniversalUuid,
        error: &str,
    ) -> Result<(), ValidationError> {
        let sid = UniversalUuid(subscription_id);
        let now = UniversalTimestamp::now();
        let error = truncate_predicate_text(error);
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                reactor_trigger_subscriptions::table
                    .filter(reactor_trigger_subscriptions::id.eq(sid)),
            )
            .set((
                reactor_trigger_subscriptions::predicate_degraded.eq(UniversalBool::from(true)),
                reactor_trigger_subscriptions::predicate_error_count.eq(0),
                reactor_trigger_subscriptions::predicate_error_firing_id.eq(Some(firing_id)),
                reactor_trigger_subscriptions::last_predicate_error.eq(Some(error)),
                reactor_trigger_subscriptions::last_predicate_error_at.eq(Some(now)),
                reactor_trigger_subscriptions::updated_at.eq(now),
            ))
            .execute(conn)
        })?;
        Ok(())
    }
```

</details>

##### <span id="ReactorSubscriptionsDAL.clear_predicate_error"></span><code>clear_​predicate_​error</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn clear_predicate_error(
    &self,
    subscription_id: Uuid,
) -> Result<(), ValidationError>
```

CLOACI-T-0922 — a predicate evaluated cleanly, so clear the retry counter and the degraded flag.

`last_predicate_error` / `last_predicate_error_at` are deliberately
NOT cleared: recovery should not erase the evidence that firings
were dropped. "Currently degraded" is `predicate_degraded`; "what
went wrong last" is the `last_predicate_error*` pair.

<details>
<summary>Source</summary>

```rust
    pub async fn clear_predicate_error(
        &self,
        subscription_id: Uuid,
    ) -> Result<(), ValidationError> {
        let sid = UniversalUuid(subscription_id);
        let now = UniversalTimestamp::now();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                reactor_trigger_subscriptions::table
                    .filter(reactor_trigger_subscriptions::id.eq(sid)),
            )
            .set((
                reactor_trigger_subscriptions::predicate_error_count.eq(0),
                reactor_trigger_subscriptions::predicate_error_firing_id.eq(None::<UniversalUuid>),
                reactor_trigger_subscriptions::predicate_degraded.eq(UniversalBool::from(false)),
                reactor_trigger_subscriptions::updated_at.eq(now),
            ))
            .execute(conn)
        })?;
        Ok(())
    }
```

</details>

##### <span id="ReactorSubscriptionsDAL.get_subscription"></span><code>get_​subscription</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_subscription(
    &self,
    subscription_id: Uuid,
) -> Result<Option<ReactorSubscription>, ValidationError>
```

CLOACI-T-0922 — fetch a single subscription by id. Used by the health/inspection path so an operator (or a test) can read the degraded state without scanning a tenant's whole subscription list.

<details>
<summary>Source</summary>

```rust
    pub async fn get_subscription(
        &self,
        subscription_id: Uuid,
    ) -> Result<Option<ReactorSubscription>, ValidationError> {
        let sid = UniversalUuid(subscription_id);
        let row = crate::interact_on_backend!(self.dal, |conn| {
            reactor_trigger_subscriptions::table
                .filter(reactor_trigger_subscriptions::id.eq(sid))
                .first::<ReactorSubscription>(conn)
                .optional()
        })?;
        Ok(row)
    }
```

</details>

##### <span id="ReactorSubscriptionsDAL.unsubscribe"></span><code>unsubscribe</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn unsubscribe(
    &self,
    reactor: &str,
    workflow: &str,
    tenant: &str,
) -> Result<bool, ValidationError>
```

Remove a subscription. Returns true if a row was deleted.

<details>
<summary>Source</summary>

```rust
    pub async fn unsubscribe(
        &self,
        reactor: &str,
        workflow: &str,
        tenant: &str,
    ) -> Result<bool, ValidationError> {
        let reactor = reactor.to_string();
        let workflow = workflow.to_string();
        let tenant = tenant.to_string();
        let n = crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(
                reactor_trigger_subscriptions::table
                    .filter(reactor_trigger_subscriptions::reactor_name.eq(reactor))
                    .filter(reactor_trigger_subscriptions::workflow_name.eq(workflow))
                    .filter(reactor_trigger_subscriptions::tenant_id.eq(tenant)),
            )
            .execute(conn)
        })?;
        Ok(n > 0)
    }
```

</details>

##### <span id="ReactorSubscriptionsDAL.list_all_enabled"></span><code>list_​all_​enabled</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_all_enabled(
    &self,
) -> Result<Vec<ReactorSubscription>, ValidationError>
```

List all enabled subscriptions across every tenant. Used by the unified scheduler's reactor poll tick (CLOACI-I-0100 / T-0599).

<details>
<summary>Source</summary>

```rust
    pub async fn list_all_enabled(&self) -> Result<Vec<ReactorSubscription>, ValidationError> {
        let rows = crate::interact_on_backend!(self.dal, |conn| {
            reactor_trigger_subscriptions::table
                .filter(reactor_trigger_subscriptions::enabled.eq(UniversalBool::from(true)))
                .load::<ReactorSubscription>(conn)
        })?;
        Ok(rows)
    }
```

</details>

##### <span id="ReactorSubscriptionsDAL.list_subscriptions"></span><code>list_​subscriptions</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_subscriptions(
    &self,
    tenant: &str,
) -> Result<Vec<ReactorSubscription>, ValidationError>
```

List enabled subscriptions for a tenant.

<details>
<summary>Source</summary>

```rust
    pub async fn list_subscriptions(
        &self,
        tenant: &str,
    ) -> Result<Vec<ReactorSubscription>, ValidationError> {
        let tenant = tenant.to_string();
        let rows = crate::interact_on_backend!(self.dal, |conn| {
            reactor_trigger_subscriptions::table
                .filter(reactor_trigger_subscriptions::tenant_id.eq(tenant))
                .filter(reactor_trigger_subscriptions::enabled.eq(UniversalBool::from(true)))
                .load::<ReactorSubscription>(conn)
        })?;
        Ok(rows)
    }
```

</details>

## Functions

### <span id="fn.truncate_predicate_text"></span><code style="font-size: 0.85em">fn truncate_​predicate_​text</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn truncate_predicate_text(s: &str) -> String
```

Truncate `s` to `PREDICATE_TEXT_TRUNCATE_LEN` characters (char-safe), appending an ellipsis marker when anything was dropped.

<details>
<summary>Source</summary>

```rust
pub fn truncate_predicate_text(s: &str) -> String {
    if s.chars().count() <= PREDICATE_TEXT_TRUNCATE_LEN {
        return s.to_string();
    }
    let head: String = s.chars().take(PREDICATE_TEXT_TRUNCATE_LEN).collect();
    format!("{}…[truncated]", head)
}
```

</details>

### <span id="fn.compile_predicate"></span><code style="font-size: 0.85em">fn compile_​predicate</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn compile_predicate(
    expr: &str,
) -> Result<cel_interpreter::Program, String>
```

CLOACI-T-0922 — compile a CEL predicate, converting BOTH failure modes of the underlying parser into an error string.

`cel_interpreter::Program::compile` is documented to return
`Err(ParseErrors)` on malformed input, but its ANTLR-generated parser
panics outright on some inputs (`"payload.x >"` reaches an
`unreachable!()` in `antlr4rust`). That panic pre-dates this change —
the T-0602 subscribe path called `compile` bare — and it turns a typo
in a user-supplied predicate into an unwind in whatever task is
subscribing or polling. Catching it here keeps a bad predicate a
*validation error* instead of a crash, which is the same principle this
ticket applies to evaluation errors.

<details>
<summary>Source</summary>

```rust
pub fn compile_predicate(expr: &str) -> Result<cel_interpreter::Program, String> {
    match std::panic::catch_unwind(|| cel_interpreter::Program::compile(expr)) {
        Ok(Ok(program)) => Ok(program),
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err(format!(
            "could not parse CEL predicate '{}' (the CEL parser rejected it)",
            truncate_predicate_text(expr)
        )),
    }
}
```

</details>

### <span id="fn.lint_predicate_variables"></span><code style="font-size: 0.85em">fn lint_​predicate_​variables</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn lint_predicate_variables(expr: &str) -> Result<(), String>
```

CLOACI-T-0922 (item 4, deferred from T-0915) — reject predicates that reference identifiers outside the bound set at subscribe time.

The motivating incident: a predicate referencing a variable nobody
binds compiles fine and then evaluates false / errors forever, which —
fail-closed — silently never fires. `tennant == 'acme'` should be a
loud error at the door, not a silent no-op in production.

**Why the AST walk instead of `Program::references()`**

`cel_interpreter::Program::references()` does report referenced
variables, but CEL's comprehension macros (`exists`, `all`, `map`,
`filter`, `exists_one`) are expanded by the parser into a
`Comprehension` node whose iteration variable appears as a bare
`Ident` inside the loop body. `references()` therefore reports the
iteration variable as if it were a free variable, and a lint built on
it would reject the perfectly valid
`payload.items.exists(i, i.price > 100)`. We walk the AST ourselves so
comprehension-bound names (`iter_var`, `iter_var2`, `accu_var`) are
treated as bound. Erring toward acceptance is deliberate: a false
rejection is worse than a missed lint.

Returns `Ok(())` when every free identifier is in
[`PREDICATE_VARIABLES`](../../../../../rust/cloacina/dal/unified/reactor_subscriptions/#const.PREDICATE_VARIABLES); otherwise an error naming the offenders and
the allowed set.

<details>
<summary>Source</summary>

```rust
pub fn lint_predicate_variables(expr: &str) -> Result<(), String> {
    use cel_parser::ast::{EntryExpr, Expr, IdedExpr};
    use std::collections::BTreeSet;

    // Same panic caveat as `compile_predicate` — this is the same parser.
    let parsed = match std::panic::catch_unwind(|| cel_parser::Parser::default().parse(expr)) {
        Ok(Ok(parsed)) => parsed,
        Ok(Err(e)) => return Err(format!("{}", e)),
        Err(_) => {
            return Err(format!(
                "could not parse CEL predicate '{}' (the CEL parser rejected it)",
                truncate_predicate_text(expr)
            ));
        }
    };

    fn walk(node: &IdedExpr, bound: &mut Vec<String>, free: &mut BTreeSet<String>) {
        match &node.expr {
            Expr::Unspecified | Expr::Literal(_) => {}
            Expr::Ident(name) => {
                // `@`-prefixed names are parser-internal accumulator
                // bindings and are never user-authored.
                if !name.starts_with('@') && !bound.iter().any(|b| b == name) {
                    free.insert(name.clone());
                }
            }
            Expr::Select(select) => walk(&select.operand, bound, free),
            Expr::Call(call) => {
                if let Some(target) = &call.target {
                    walk(target, bound, free);
                }
                for arg in &call.args {
                    walk(arg, bound, free);
                }
            }
            Expr::List(list) => {
                for elem in &list.elements {
                    walk(elem, bound, free);
                }
            }
            Expr::Map(map) => {
                for entry in &map.entries {
                    walk_entry(&entry.expr, bound, free);
                }
            }
            Expr::Struct(s) => {
                for entry in &s.entries {
                    walk_entry(&entry.expr, bound, free);
                }
            }
            Expr::Comprehension(comp) => {
                // The range is evaluated in the OUTER scope.
                walk(&comp.iter_range, bound, free);
                walk(&comp.accu_init, bound, free);
                let depth = bound.len();
                bound.push(comp.iter_var.clone());
                if let Some(v2) = &comp.iter_var2 {
                    bound.push(v2.clone());
                }
                bound.push(comp.accu_var.clone());
                walk(&comp.loop_cond, bound, free);
                walk(&comp.loop_step, bound, free);
                walk(&comp.result, bound, free);
                bound.truncate(depth);
            }
        }
    }

    fn walk_entry(entry: &EntryExpr, bound: &mut Vec<String>, free: &mut BTreeSet<String>) {
        match entry {
            EntryExpr::StructField(field) => walk(&field.value, bound, free),
            EntryExpr::MapEntry(e) => {
                walk(&e.key, bound, free);
                walk(&e.value, bound, free);
            }
        }
    }

    let mut bound: Vec<String> = Vec::new();
    let mut free: BTreeSet<String> = BTreeSet::new();
    walk(&parsed, &mut bound, &mut free);

    let unknown: Vec<String> = free
        .into_iter()
        .filter(|name| !PREDICATE_VARIABLES.contains(&name.as_str()))
        .collect();

    if unknown.is_empty() {
        return Ok(());
    }

    Err(format!(
        "predicate references unknown variable(s): {}. Available variables are: {}. \
         (Did you mean a field of `payload`, e.g. `payload.{}`?)",
        unknown.join(", "),
        PREDICATE_VARIABLES.join(", "),
        unknown[0],
    ))
}
```

</details>

## Constants

| Name | Type | Value | Description |
|------|------|-------|-------------|
| <span id="const.PREDICATE_VARIABLES"></span>`PREDICATE_VARIABLES` | `[&str; 3]` | `["payload", "reactor", "tenant"]` | CLOACI-T-0922 — the complete set of variables the reactor predicate evaluator binds into the CEL context. Kept here (next to the subscribe-time validation that enforces it) so the lint and the evaluator cannot drift apart; `cron_trigger_scheduler:: eval_cel_predicate_program` binds exactly these names. |
| <span id="const.PREDICATE_TEXT_TRUNCATE_LEN"></span>`PREDICATE_TEXT_TRUNCATE_LEN` | `usize` | `240` | Max characters of predicate/error text we persist or log. Predicates are user-authored and unbounded; log lines and DB columns are not. |
