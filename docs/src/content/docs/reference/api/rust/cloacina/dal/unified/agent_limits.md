---
title: "cloacina::dal::unified::agent_limits"
description: "Agent capacity limits — per-tenant exceptions to the global default (CLOACI-T-0808, CLOACI-I-0127). **Postgres only.**"
sidebar:
  order: 83
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Agent capacity limits — per-tenant exceptions to the global default (CLOACI-T-0808, CLOACI-I-0127). **Postgres only.**

The global default (`CLOACINA_DEFAULT_MAX_AGENTS`) is server config; this DAL
stores ONLY the per-tenant overrides an admin grants. `effective_limit` is the
override if present, else the default — the hard ceiling that the provision
API (T-0809) and the back-pressure autoscaler (T-0811) clamp to. Setting an
exception is a god-only op; a tenant may read its own effective limit.

## Structs

### <span id="struct.AgentLimitRow"></span>`struct AgentLimitRow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


**Derives:** `Queryable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | `String` |  |
| `max_agents` | `i32` |  |
| `created_at` | `chrono :: NaiveDateTime` |  |
| `updated_at` | `chrono :: NaiveDateTime` |  |



### <span id="struct.AgentLimitsDAL"></span>`struct AgentLimitsDAL<'a>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


DAL for per-tenant agent-capacity limits. Postgres only.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



