# cloacina::dal::unified::agent_limits <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Agent capacity limits — per-tenant exceptions to the global default (CLOACI-T-0808, CLOACI-I-0127). **Postgres only.**

The global default (`CLOACINA_DEFAULT_MAX_AGENTS`) is server config; this DAL
stores ONLY the per-tenant overrides an admin grants. `effective_limit` is the
override if present, else the default — the hard ceiling that the provision
API (T-0809) and the back-pressure autoscaler (T-0811) clamp to. Setting an
exception is a god-only op; a tenant may read its own effective limit.

## Structs

### `cloacina::dal::unified::agent_limits::AgentLimitRow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


**Derives:** `Queryable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | `String` |  |
| `max_agents` | `i32` |  |
| `created_at` | `chrono :: NaiveDateTime` |  |
| `updated_at` | `chrono :: NaiveDateTime` |  |



### `cloacina::dal::unified::agent_limits::AgentLimitsDAL`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


DAL for per-tenant agent-capacity limits. Postgres only.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |
