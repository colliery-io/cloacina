---
title: "cloacina::dal::unified::agent_desired"
description: "Agent desired count — per-tenant self-service provisioning state (CLOACI-T-0809, CLOACI-I-0127). **Postgres only.**"
sidebar:
  order: 82
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

### <span id="struct.AgentDesiredRow"></span>`struct AgentDesiredRow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


**Derives:** `Queryable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | `String` |  |
| `desired_count` | `i32` |  |
| `updated_at` | `chrono :: NaiveDateTime` |  |
| `last_autoscaled_at` | `Option < chrono :: NaiveDateTime >` | Wall-clock of the last autoscaler scale action (NULL = never autoscaled,
i.e. only ever touched by manual `set_desired`). Stamped via SQL `now()`
in `set_desired_autoscaled`; read by the control loop to gate the
cross-replica cooldown (CLOACI-A-0008 refinement). |



### <span id="struct.AgentDesiredDAL"></span>`struct AgentDesiredDAL<'a>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


DAL for per-tenant desired agent count. Postgres only.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



