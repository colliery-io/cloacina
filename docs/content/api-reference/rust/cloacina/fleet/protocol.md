# cloacina::fleet::protocol <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Wire types for the execution-agent fleet protocol (CLOACI-T-0631).

## Structs

### `cloacina::fleet::protocol::AgentRegisterRequest`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `protocol_version` | `u32` |  |
| `agent_id` | `Option < String >` | Caller-chosen agent id (e.g. hostname + pid hash). If `None` the server
assigns a fresh one and returns it in [`AgentRegisterResponse::agent_id`]. |
| `max_concurrency` | `u32` | Maximum concurrent tasks this agent will accept. |
| `target_triple` | `String` | Target triple the agent is running on (e.g. `aarch64-apple-darwin`).
OQ-6 fail-closed: the `FleetExecutor` only assigns work whose
`ArtifactRef::build_target_triple` matches this. |
| `capabilities` | `Vec < String >` | Free-form capability tags the `FleetExecutor` can route on
(e.g. `gpu`, `large_memory`). |



### `cloacina::fleet::protocol::AgentRegisterResponse`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `protocol_version` | `u32` |  |
| `agent_id` | `String` | The id the `FleetExecutor` will use to address this agent. The agent
MUST connect to the substrate WS at
`/v1/ws/delivery/{AGENT_RECIPIENT_PREFIX}{agent_id}`. |
| `heartbeat_interval_seconds` | `u32` | Server-suggested heartbeat cadence. The agent should heartbeat at
least this often; the server marks an agent dead after a small
multiple of missed intervals. |



### `cloacina::fleet::protocol::AgentHeartbeatRequest`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `protocol_version` | `u32` |  |
| `agent_id` | `String` |  |
| `in_flight` | `u32` | Number of work packets currently in flight on this agent. |
| `available_capacity` | `u32` | Currently-available capacity (`max_concurrency - in_flight`). The
`FleetExecutor` uses this for selection. |



### `cloacina::fleet::protocol::AgentHeartbeatResponse`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `protocol_version` | `u32` |  |



### `cloacina::fleet::protocol::WorkPacket`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Fully self-contained work packet — everything a DB-less agent needs to run one task without ever touching the database.

Serialized as JSON into the substrate `Push.payload_b64`. On Postgres the
outbox row that carries this is enqueued in the same transaction as the
state change that produced the work (by the `FleetExecutor`).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `protocol_version` | `u32` |  |
| `task_execution_id` | `String` |  |
| `workflow_execution_id` | `String` |  |
| `task_name` | `String` |  |
| `attempt` | `i32` |  |
| `context` | `serde_json :: Value` | Merged dependency context the task closure consumes — eagerly
resolved by the server because the agent has no DAL. (For very large
contexts a future variant may swap inline JSON for a context-fetch
REST reference; OQ-1.) |
| `artifact` | `ArtifactRef` | Pointer to the cdylib artifact the agent must `dlopen`. |
| `timeout_seconds` | `u32` | Per-task execution timeout. |
| `tenant_id` | `Option < String >` | Tenant scope. The agent's authenticated context must match this to
even receive the packet; included here so the agent can pass it into
the runtime when constructing the task's execution scope. |



### `cloacina::fleet::protocol::ArtifactRef`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Reference to a workflow artifact (cdylib) the agent must fetch + load.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `digest` | `String` | Content-addressed digest. Matches `workflow_packages.content_hash`. |
| `fetch_url` | `String` | REST URL the agent fetches from (relative or absolute). Typically
`/v1/agent/artifact/{digest}` on the server. |
| `build_target_triple` | `String` | Target triple the cdylib was built for (OQ-6 fail-closed). The agent
MUST compare to its own `target_triple` and refuse on mismatch rather
than attempt `dlopen`. |



### `cloacina::fleet::protocol::AgentResultRequest`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `protocol_version` | `u32` |  |
| `agent_id` | `String` |  |
| `task_execution_id` | `String` |  |
| `attempt` | `i32` | Echoed from the original work packet so the server can reject stale
reports (an agent reporting attempt N on a row already retried to N+1). |
| `duration_ms` | `u64` |  |
| `outcome` | `AgentOutcome` |  |



### `cloacina::fleet::protocol::AgentResultResponse`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `protocol_version` | `u32` |  |



## Enums

### `cloacina::fleet::protocol::AgentOutcome` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Outcome of one work packet as reported by the agent. The server's `FleetExecutor` reconciliation maps these onto the shared `crate::executor::TaskResultHandler::handle_outcome` Result variant, guaranteeing the thread and fleet executors agree on status / retry / context-persist semantics.

#### Variants

- **`Success`** - Task closure returned successfully with a produced output context.
- **`Failure`** - Task closure returned an error.
- **`Refused`** - Agent refused to run the work packet (pre-execution). Server should
treat as transient and reschedule onto a different agent.



### `cloacina::fleet::protocol::FailureClassification` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Bounded classification of a task-level failure so the server's retry decision is consistent with what `TaskResultHandler::is_transient_error` would have decided locally.

#### Variants

- **`TaskError`** - Task code returned an error (analog of `ExecutorError::TaskExecution`).
Retried only if the task's `RetryPolicy` says so.
- **`Transient`** - Timeout, network, or other clearly-transient failure — preferred for retry.
- **`Validation`** - Invalid input / context / configuration. No retry.
- **`Timeout`** - Task ran past its `timeout_seconds`.



### `cloacina::fleet::protocol::RefusalReason` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Why the agent refused to even run the packet. Treated as transient by the server (reschedule to a different agent).

#### Variants

- **`TargetTripleMismatch`** - `agent.target_triple != artifact.build_target_triple` (OQ-6 fail-closed).
- **`ArtifactFetchFailed`** - Artifact REST fetch failed (server unreachable, 404, 5xx, IO error).
- **`RuntimeLoadFailed`** - `dlopen` / runtime load failed (corrupted cdylib, missing symbol).
- **`Shutdown`** - Agent is draining or shutting down.
- **`TenantMismatch`** - Server tried to route a packet whose tenant the agent isn't authorized for.



## Functions

### `cloacina::fleet::protocol::host_target_triple`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn host_target_triple () -> String
```

Best-effort host target triple. v1 simplification: `<arch>-<os>` derived from `std::env::consts` — doesn't distinguish glibc vs musl, etc. Both the server (when stamping `ArtifactRef::build_target_triple`) and the agent (when reporting its own `target_triple` and doing the fail-closed comparison) use this same function so the OQ-6 check is exact-string. Per-artifact full-triple tracking is future work.

<details>
<summary>Source</summary>

```rust
pub fn host_target_triple() -> String {
    format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS)
}
```

</details>
