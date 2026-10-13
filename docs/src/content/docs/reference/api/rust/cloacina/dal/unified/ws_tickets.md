---
title: "cloacina::dal::unified::ws_tickets"
description: "Unified WS-ticket DAL (CLOACI-T-0916)."
editUrl: false
sidebar:
  order: 96
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Unified WS-ticket DAL (CLOACI-T-0916).

Single-use, short-TTL WebSocket auth tickets, persisted so a ticket minted
on one server replica redeems on ANY replica (no session affinity —
mirrors the OIDC login-flow precedent, T-0801). Redemption is once per WS
connect, so a straight DB round-trip is fine (no hot-path cache needed).

Single-use enforcement is an **atomic compare-and-set**:
`UPDATE ... SET redeemed_at = now WHERE ticket = ? AND redeemed_at IS NULL
AND expires_at > now` — exactly one concurrent redeemer sees
`rows_affected == 1`; every other attempt (replay, expiry, unknown ticket)
yields `None`. Works identically on both backends (no RETURNING needed:
the winner then reads the row it now exclusively owns).

## Structs

### <span id="struct.WsTicketAuth"></span><code style="font-size: 0.85em">struct WsTicketAuth</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

The authenticated-key identity a redeemed ticket carries.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `key_id` | `uuid::Uuid` |  |
| `name` | `String` |  |
| `permissions` | `String` |  |
| `tenant_id` | `Option<String>` |  |
| `is_admin` | `bool` |  |

### <span id="struct.WsTicketDAL"></span><code style="font-size: 0.85em">struct WsTicketDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Data access layer for single-use WebSocket auth tickets.

#### Methods

##### <span id="WsTicketDAL.new"></span><code>new</code>

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

##### <span id="WsTicketDAL.issue"></span><code>issue</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn issue(
    &self,
    auth: WsTicketAuth,
    ttl: std::time::Duration,
) -> Result<String, ValidationError>
```

Issue a ticket carrying `auth`, valid for `ttl`. Returns the ticket string (a fresh UUID). Expired rows are pruned opportunistically first so the table stays bounded without a dedicated sweeper.

<details>
<summary>Source</summary>

```rust
    pub async fn issue(
        &self,
        auth: WsTicketAuth,
        ttl: std::time::Duration,
    ) -> Result<String, ValidationError> {
        let ticket = uuid::Uuid::new_v4().to_string();
        let now = UniversalTimestamp::now();
        let expires_at = UniversalTimestamp(
            now.0
                + chrono::Duration::from_std(ttl).unwrap_or_else(|_| chrono::Duration::seconds(60)),
        );
        let row = NewUnifiedWsTicket {
            ticket: ticket.clone(),
            key_id: UniversalUuid(auth.key_id),
            key_name: auth.name,
            permissions: auth.permissions,
            tenant_id: auth.tenant_id,
            is_admin: UniversalBool::new(auth.is_admin),
            created_at: now,
            expires_at,
            redeemed_at: None,
        };
        crate::interact_on_backend!(self.dal, |conn| {
            // Opportunistic prune: expired tickets (redeemed or not) are dead
            // weight — delete before inserting the fresh one.
            diesel::delete(ws_tickets::table.filter(ws_tickets::expires_at.lt(now)))
                .execute(conn)?;
            diesel::insert_into(ws_tickets::table)
                .values(&row)
                .execute(conn)
        })
        .map_err(ValidationError::from)?;
        Ok(ticket)
    }
```

</details>

##### <span id="WsTicketDAL.redeem"></span><code>redeem</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn redeem(
    &self,
    ticket: &str,
) -> Result<Option<WsTicketAuth>, ValidationError>
```

Redeem a ticket (single-use). Returns the carried auth identity iff the ticket exists, has not expired, and has not been redeemed before — atomically, so exactly one of any number of concurrent redeemers wins.

<details>
<summary>Source</summary>

```rust
    pub async fn redeem(&self, ticket: &str) -> Result<Option<WsTicketAuth>, ValidationError> {
        let ticket = ticket.to_string();
        let now = UniversalTimestamp::now();
        let row: Option<UnifiedWsTicket> = crate::interact_on_backend!(self.dal, |conn| {
            // Atomic CAS: only an unredeemed, unexpired row transitions.
            let affected = diesel::update(
                ws_tickets::table
                    .filter(ws_tickets::ticket.eq(&ticket))
                    .filter(ws_tickets::redeemed_at.is_null())
                    .filter(ws_tickets::expires_at.gt(now)),
            )
            .set(ws_tickets::redeemed_at.eq(Some(now)))
            .execute(conn)?;
            if affected == 1 {
                // We won the CAS — the row is exclusively ours to read.
                ws_tickets::table
                    .filter(ws_tickets::ticket.eq(&ticket))
                    .first::<UnifiedWsTicket>(conn)
                    .optional()
            } else {
                Ok(None)
            }
        })
        .map_err(ValidationError::from)?;
        Ok(row.map(|r| WsTicketAuth {
            key_id: r.key_id.0,
            name: r.key_name,
            permissions: r.permissions,
            tenant_id: r.tenant_id,
            is_admin: r.is_admin.into(),
        }))
    }
```

</details>

##### <span id="WsTicketDAL.prune_expired"></span><code>prune_​expired</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn prune_expired(&self) -> Result<usize, ValidationError>
```

Delete all tickets whose `expires_at` has passed. Returns the count. (Also runs opportunistically inside [`issue`](../../../../../rust/cloacina/dal/unified/ws_tickets/#WsTicketDAL.issue).)

<details>
<summary>Source</summary>

```rust
    pub async fn prune_expired(&self) -> Result<usize, ValidationError> {
        let now = UniversalTimestamp::now();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(ws_tickets::table.filter(ws_tickets::expires_at.lt(now))).execute(conn)
        })
        .map_err(ValidationError::from)
    }
```

</details>
