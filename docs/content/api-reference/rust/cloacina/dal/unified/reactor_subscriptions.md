# cloacina::dal::unified::reactor_subscriptions <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


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

### `cloacina::dal::unified::reactor_subscriptions::ReactorFiring`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`

One reactor firing event. Carries the boundary cache payload the in-process CG traversal consumed; subscribers receive the same data as their workflow's input context.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `reactor_name` | `String` |  |
| `tenant_id` | `String` |  |
| `payload` | `Option < UniversalBinary >` |  |
| `fired_at` | `UniversalTimestamp` |  |
| `created_at` | `UniversalTimestamp` |  |



### `cloacina::dal::unified::reactor_subscriptions::ReactorSubscription`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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
| `last_seen_fired_at` | `Option < UniversalTimestamp >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |
| `predicate_expression` | `Option < String >` | CLOACI-T-0602 — optional CEL filter expression. When `Some`, the
scheduler evaluates it against the firing payload before dispatch;
`Some(_) && false` means "skip + advance watermark". `None`
preserves the original unfiltered behavior (fire on every firing). |



### `cloacina::dal::unified::reactor_subscriptions::ReactorSubscriptionsDAL`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Data access layer for reactor subscriptions + firings.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |
