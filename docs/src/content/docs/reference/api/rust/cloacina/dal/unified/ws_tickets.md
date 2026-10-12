---
title: "cloacina::dal::unified::ws_tickets"
description: "Unified WS-ticket DAL (CLOACI-T-0916)."
sidebar:
  order: 112
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

### <span id="struct.WsTicketAuth"></span>`struct WsTicketAuth`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`

The authenticated-key identity a redeemed ticket carries.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `key_id` | `uuid :: Uuid` |  |
| `name` | `String` |  |
| `permissions` | `String` |  |
| `tenant_id` | `Option < String >` |  |
| `is_admin` | `bool` |  |



### <span id="struct.WsTicketDAL"></span>`struct WsTicketDAL<'a>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Clone`

Data access layer for single-use WebSocket auth tickets.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



