---
title: "cloacina::dal::unified"
description: "Unified Data Access Layer with runtime backend selection"
sidebar:
  order: 81
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Unified Data Access Layer with runtime backend selection

This module provides a unified DAL implementation that works with both
PostgreSQL and SQLite backends, selecting the appropriate implementation
at runtime based on the database connection type.

**Examples:**

```rust
use cloacina::dal::unified::DAL;
use cloacina::database::Database;

// Create database with runtime backend detection
let db = Database::new("postgres://localhost/mydb", "mydb", 10);
let dal = DAL::new(db);

// Operations automatically use the correct backend
let contexts = dal.context().list().await?;
```

## Structs

### <span id="struct.DAL"></span>`struct DAL`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Clone`, `Debug`

Helper macro for dispatching operations based on backend type.

This macro simplifies writing code that needs to execute different
implementations based on the database backend.
The unified Data Access Layer struct.
This struct provides access to all database operations through a single
interface that works with both PostgreSQL and SQLite backends.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `database` | `Database` | The database instance with connection pool |

#### Methods

##### <span id="DAL.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn new (database : Database) -> Self
```

Creates a new unified DAL instance.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database` | `-` | A Database instance configured for either PostgreSQL or SQLite |


**Returns:**

A new DAL instance ready for database operations.

<details>
<summary>Source</summary>

```rust
    pub fn new(database: Database) -> Self {
        DAL { database }
    }
```

</details>



##### <span id="DAL.backend"></span>`backend`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn backend (& self) -> BackendType
```

Returns the backend type for this DAL instance.

<details>
<summary>Source</summary>

```rust
    pub fn backend(&self) -> BackendType {
        self.database.backend()
    }
```

</details>



##### <span id="DAL.database"></span>`database`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn database (& self) -> & Database
```

Returns a reference to the underlying database.

<details>
<summary>Source</summary>

```rust
    pub fn database(&self) -> &Database {
        &self.database
    }
```

</details>



##### <span id="DAL.pool"></span>`pool`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn pool (& self) -> AnyPool
```

Returns the connection pool.

<details>
<summary>Source</summary>

```rust
    pub fn pool(&self) -> AnyPool {
        self.database.pool()
    }
```

</details>



##### <span id="DAL.agent_limits"></span>`agent_limits`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn agent_limits (& self) -> AgentLimitsDAL < '_ >
```

Returns an agent-capacity-limits DAL (Postgres only). CLOACI-T-0808.

<details>
<summary>Source</summary>

```rust
    pub fn agent_limits(&self) -> AgentLimitsDAL<'_> {
        AgentLimitsDAL::new(self)
    }
```

</details>



##### <span id="DAL.agent_desired"></span>`agent_desired`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn agent_desired (& self) -> AgentDesiredDAL < '_ >
```

Returns an agent-desired-count DAL (Postgres only). CLOACI-T-0809.

<details>
<summary>Source</summary>

```rust
    pub fn agent_desired(&self) -> AgentDesiredDAL<'_> {
        AgentDesiredDAL::new(self)
    }
```

</details>



##### <span id="DAL.api_keys"></span>`api_keys`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn api_keys (& self) -> ApiKeyDAL < '_ >
```

Returns an API key DAL (Postgres only).

<details>
<summary>Source</summary>

```rust
    pub fn api_keys(&self) -> ApiKeyDAL<'_> {
        ApiKeyDAL::new(self)
    }
```

</details>



##### <span id="DAL.oidc_sessions"></span>`oidc_sessions`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn oidc_sessions (& self) -> OidcSessionDAL < '_ >
```

Returns an OIDC refresh-session DAL (Postgres only). CLOACI-T-0793.

<details>
<summary>Source</summary>

```rust
    pub fn oidc_sessions(&self) -> OidcSessionDAL<'_> {
        OidcSessionDAL::new(self)
    }
```

</details>



##### <span id="DAL.local_accounts"></span>`local_accounts`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn local_accounts (& self) -> LocalAccountDAL < '_ >
```

Returns a local-accounts DAL (Postgres only). CLOACI-T-0795.

<details>
<summary>Source</summary>

```rust
    pub fn local_accounts(&self) -> LocalAccountDAL<'_> {
        LocalAccountDAL::new(self)
    }
```

</details>



##### <span id="DAL.login_throttle"></span>`login_throttle`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn login_throttle (& self) -> LoginThrottleDAL < '_ >
```

Returns the login brute-force throttle DAL. CLOACI-T-0923. Unified (both backends) so its semantics are testable without Postgres.

<details>
<summary>Source</summary>

```rust
    pub fn login_throttle(&self) -> LoginThrottleDAL<'_> {
        LoginThrottleDAL::new(self)
    }
```

</details>



##### <span id="DAL.reactor_owner_addresses"></span>`reactor_owner_addresses`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn reactor_owner_addresses (& self) -> reactor_owner_addresses :: ReactorOwnerAddressesDAL < '_ >
```

Returns the reactor owner-address DAL (CLOACI-T-0851 / A-0012 Amendment 3). A routing hint only — the advisory lock is the sole ownership authority. Unified (both backends) so the takeover-race semantics are testable without Postgres.

<details>
<summary>Source</summary>

```rust
    pub fn reactor_owner_addresses(&self) -> reactor_owner_addresses::ReactorOwnerAddressesDAL<'_> {
        reactor_owner_addresses::ReactorOwnerAddressesDAL::new(self)
    }
```

</details>



##### <span id="DAL.oidc_login_flows"></span>`oidc_login_flows`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn oidc_login_flows (& self) -> OidcLoginFlowDAL < '_ >
```

Returns an OIDC login-flow-state DAL (Postgres only). CLOACI-T-0801.

<details>
<summary>Source</summary>

```rust
    pub fn oidc_login_flows(&self) -> OidcLoginFlowDAL<'_> {
        OidcLoginFlowDAL::new(self)
    }
```

</details>



##### <span id="DAL.checkpoint"></span>`checkpoint`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn checkpoint (& self) -> CheckpointDAL < '_ >
```

Returns a checkpoint DAL for computation graph state persistence.

<details>
<summary>Source</summary>

```rust
    pub fn checkpoint(&self) -> CheckpointDAL<'_> {
        CheckpointDAL::new(self)
    }
```

</details>



##### <span id="DAL.context"></span>`context`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn context (& self) -> ContextDAL < '_ >
```

Returns a context DAL for context operations.

<details>
<summary>Source</summary>

```rust
    pub fn context(&self) -> ContextDAL<'_> {
        ContextDAL::new(self)
    }
```

</details>



##### <span id="DAL.workflow_execution"></span>`workflow_execution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn workflow_execution (& self) -> WorkflowExecutionDAL < '_ >
```

Returns a workflow execution DAL for workflow execution operations.

<details>
<summary>Source</summary>

```rust
    pub fn workflow_execution(&self) -> WorkflowExecutionDAL<'_> {
        WorkflowExecutionDAL::new(self)
    }
```

</details>



##### <span id="DAL.task_execution"></span>`task_execution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn task_execution (& self) -> TaskExecutionDAL < '_ >
```

Returns a task execution DAL for task operations.

<details>
<summary>Source</summary>

```rust
    pub fn task_execution(&self) -> TaskExecutionDAL<'_> {
        TaskExecutionDAL::new(self)
    }
```

</details>



##### <span id="DAL.task_execution_metadata"></span>`task_execution_metadata`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn task_execution_metadata (& self) -> TaskExecutionMetadataDAL < '_ >
```

Returns a task execution metadata DAL for metadata operations.

<details>
<summary>Source</summary>

```rust
    pub fn task_execution_metadata(&self) -> TaskExecutionMetadataDAL<'_> {
        TaskExecutionMetadataDAL::new(self)
    }
```

</details>



##### <span id="DAL.delivery_outbox"></span>`delivery_outbox`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn delivery_outbox (& self) -> DeliveryOutboxDAL < '_ >
```

Returns a delivery outbox DAL for the interservice communication substrate (durable, ack-tracked, recipient-addressed push delivery).

<details>
<summary>Source</summary>

```rust
    pub fn delivery_outbox(&self) -> DeliveryOutboxDAL<'_> {
        DeliveryOutboxDAL::new(self)
    }
```

</details>



##### <span id="DAL.fleet_agents"></span>`fleet_agents`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn fleet_agents (& self) -> FleetAgentDAL < '_ >
```

Returns a fleet-agent roster DAL (register/heartbeat state persisted for multi-replica selection, capacity views and reclaim). CLOACI-T-0916.

<details>
<summary>Source</summary>

```rust
    pub fn fleet_agents(&self) -> FleetAgentDAL<'_> {
        FleetAgentDAL::new(self)
    }
```

</details>



##### <span id="DAL.ws_tickets"></span>`ws_tickets`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn ws_tickets (& self) -> WsTicketDAL < '_ >
```

Returns a single-use WebSocket-ticket DAL (DB-backed so a ticket minted on one replica redeems on any other). CLOACI-T-0916.

<details>
<summary>Source</summary>

```rust
    pub fn ws_tickets(&self) -> WsTicketDAL<'_> {
        WsTicketDAL::new(self)
    }
```

</details>



##### <span id="DAL.recovery_event"></span>`recovery_event`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn recovery_event (& self) -> RecoveryEventDAL < '_ >
```

Returns a recovery event DAL for recovery operations.

<details>
<summary>Source</summary>

```rust
    pub fn recovery_event(&self) -> RecoveryEventDAL<'_> {
        RecoveryEventDAL::new(self)
    }
```

</details>



##### <span id="DAL.execution_event"></span>`execution_event`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn execution_event (& self) -> ExecutionEventDAL < '_ >
```

Returns an execution event DAL for execution event operations.

<details>
<summary>Source</summary>

```rust
    pub fn execution_event(&self) -> ExecutionEventDAL<'_> {
        ExecutionEventDAL::new(self)
    }
```

</details>



##### <span id="DAL.schedule"></span>`schedule`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn schedule (& self) -> ScheduleDAL < '_ >
```

Returns a unified schedule DAL for schedule operations.

<details>
<summary>Source</summary>

```rust
    pub fn schedule(&self) -> ScheduleDAL<'_> {
        ScheduleDAL::new(self)
    }
```

</details>



##### <span id="DAL.schedule_execution"></span>`schedule_execution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn schedule_execution (& self) -> ScheduleExecutionDAL < '_ >
```

Returns a unified schedule execution DAL for schedule execution operations.

<details>
<summary>Source</summary>

```rust
    pub fn schedule_execution(&self) -> ScheduleExecutionDAL<'_> {
        ScheduleExecutionDAL::new(self)
    }
```

</details>



##### <span id="DAL.workflow_packages"></span>`workflow_packages`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn workflow_packages (& self) -> WorkflowPackagesDAL < '_ >
```

Returns a workflow packages DAL for package operations.

<details>
<summary>Source</summary>

```rust
    pub fn workflow_packages(&self) -> WorkflowPackagesDAL<'_> {
        WorkflowPackagesDAL::new(self)
    }
```

</details>



##### <span id="DAL.reactor_subscriptions"></span>`reactor_subscriptions`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn reactor_subscriptions (& self) -> ReactorSubscriptionsDAL < '_ >
```

Returns a reactor subscriptions DAL for reactor-triggered workflow fan-out (CLOACI-I-0100 / T-0598).

<details>
<summary>Source</summary>

```rust
    pub fn reactor_subscriptions(&self) -> ReactorSubscriptionsDAL<'_> {
        ReactorSubscriptionsDAL::new(self)
    }
```

</details>



##### <span id="DAL.workflow_registry"></span>`workflow_registry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn workflow_registry < S : crate :: registry :: traits :: RegistryStorage + 'static > (& self , storage : S ,) -> crate :: registry :: workflow_registry :: WorkflowRegistryImpl < S >
```

Creates a workflow registry implementation with the given storage backend.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `storage` | `-` | A storage backend implementing `RegistryStorage` |


**Raises:**

| Exception | Description |
|-----------|-------------|
| `Panic` | Panics if the workflow registry cannot be created. Use [`try_workflow_registry`](Self::try_workflow_registry) for fallible construction. |


<details>
<summary>Source</summary>

```rust
    pub fn workflow_registry<S: crate::registry::traits::RegistryStorage + 'static>(
        &self,
        storage: S,
    ) -> crate::registry::workflow_registry::WorkflowRegistryImpl<S> {
        self.try_workflow_registry(storage)
            .expect("Failed to create workflow registry")
    }
```

</details>



##### <span id="DAL.try_workflow_registry"></span>`try_workflow_registry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn try_workflow_registry < S : crate :: registry :: traits :: RegistryStorage + 'static > (& self , storage : S ,) -> Result < crate :: registry :: workflow_registry :: WorkflowRegistryImpl < S > , crate :: registry :: error :: RegistryError , >
```

Creates a workflow registry implementation with the given storage backend.

This is the fallible version of [`workflow_registry`](Self::workflow_registry).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `storage` | `-` | A storage backend implementing `RegistryStorage` |


**Raises:**

| Exception | Description |
|-----------|-------------|
| `Error` | Returns an error if the workflow registry cannot be initialized. |


<details>
<summary>Source</summary>

```rust
    pub fn try_workflow_registry<S: crate::registry::traits::RegistryStorage + 'static>(
        &self,
        storage: S,
    ) -> Result<
        crate::registry::workflow_registry::WorkflowRegistryImpl<S>,
        crate::registry::error::RegistryError,
    > {
        crate::registry::workflow_registry::WorkflowRegistryImpl::new(
            storage,
            self.database.clone(),
        )
    }
```

</details>





