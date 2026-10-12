---
title: "cloacina::dal::unified::reactor_owner_addresses"
description: "DAL for reactor owner addresses (CLOACI-T-0851 / A-0012 Amendment 3)."
sidebar:
  order: 96
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


DAL for reactor owner addresses (CLOACI-T-0851 / A-0012 Amendment 3).

**A routing hint, not ownership.** The advisory lock is the sole authority
on who owns a reactor; rows here only say where the last successful claimant
said it could be reached. Reads answer "where should I redirect this
inject", and a stale answer costs one wasted hop — the fallback (redirect
again, or the delivery outbox) covers it. No reader may treat presence or
absence of a row as evidence about ownership.

## Structs

### <span id="struct.ReactorOwnerAddressesDAL"></span>`struct ReactorOwnerAddressesDAL<'a>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


DAL accessor for the `reactor_owner_addresses` routing-hint table.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



