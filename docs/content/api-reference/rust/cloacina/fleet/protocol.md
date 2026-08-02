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
| `ephemeral_public_key` | `Option < String >` | CLOACI-T-0861 (superseded by `ephemeral_key_pool`) — a single ephemeral
X25519 **public** key (base64 standard). Retained for wire back-compat
with a pre-pool agent; a modern agent leaves this `None` and advertises
`ephemeral_key_pool` instead. `None` + empty pool ⇒ the agent advertised
no key and the server MUST NOT wrap secrets to it. |
| `ephemeral_key_pool` | `Vec < EphemeralKeyEntry >` | CLOACI-T-0861 / I-0133 **D-5 (one-time key pool)** — a pool of one-time
ephemeral X25519 public keys, each with a `key_id`. The server persists
this pool against the agent and CONSUMES exactly one entry per
secret-bearing dispatch (wrapping that execution's secrets to it, stamping
the `key_id` on the [`WorkPacket::secret_key_id`]); the agent holds the
paired private keys and, on receiving the packet, unwraps ONCE with the
matching key then discards it. This gives true per-execution forward
secrecy over the push protocol (no per-dispatch round-trip needed). The
agent tops the pool up via [`AgentKeyReplenishRequest`] when the server
signals low ([`AgentHeartbeatResponse::replenish_keys`]) or proactively. |



### `cloacina::fleet::protocol::EphemeralKeyEntry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`, `PartialEq`, `Eq`

CLOACI-T-0861 / D-5 — one entry in an agent's one-time ephemeral key pool.

The `key_id` is an opaque agent-minted handle (a UUID) the server stamps onto
the dispatch it wraps to that key; the agent uses it to find the matching
private key. `public_key_b64` is the serialized X25519 public key (base64
standard). A pool entry is used AT MOST ONCE end to end: the server consumes
it for a single dispatch, the agent unwraps once and discards the private key.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `key_id` | `String` | Opaque one-time handle (agent-minted UUID). |
| `public_key_b64` | `String` | Serialized X25519 public key, base64 (standard). |



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
| `replenish_keys` | `u32` | CLOACI-T-0861 / D-5 — the server's one-time-key-pool replenish signal:
how many fresh [`EphemeralKeyEntry`]s it would like the agent to top up
(because consumption has drawn the persisted pool below its low-water
mark). `0` (the serde default, so pre-pool servers read as `0`) ⇒ the pool
is healthy. The agent responds by POSTing an [`AgentKeyReplenishRequest`]. |



### `cloacina::fleet::protocol::AgentKeyReplenishRequest`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

CLOACI-T-0861 / D-5 — the agent tops up its server-side one-time key pool.

Sent either in response to a [`AgentHeartbeatResponse::replenish_keys`] signal
or proactively when the agent's local pool drops below its own threshold. Each
carried [`EphemeralKeyEntry`] is a fresh one-time public key the server appends
to the agent's unused pool.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `protocol_version` | `u32` |  |
| `agent_id` | `String` |  |
| `keys` | `Vec < EphemeralKeyEntry >` | Fresh one-time public keys to append to the agent's server-side pool. |



### `cloacina::fleet::protocol::AgentKeyReplenishResponse`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `protocol_version` | `u32` |  |
| `accepted` | `u32` | How many keys the server accepted into the pool (0 if the agent was
unknown / needs to re-register). |



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
| `language` | `Option < String >` | Package language, so the agent loads it the right way: `"rust"` (or
absent, for older servers) → `dlopen` the cdylib at `artifact`;
`"python"` → fetch the source archive and import it via PyO3. Defaults
to `"rust"` when missing so a packet from a pre-CLOACI-T-0716 server is
still handled as before. (CLOACI-T-0716) |
| `wrapped_secrets` | `Vec < WrappedSecret >` | CLOACI-T-0861 — secrets this task needs, each HPKE-wrapped to the target
agent's advertised ephemeral public key. ONLY ciphertext crosses the wire
(NFR-001/NFR-003); the agent unwraps with its ephemeral private key into
the in-memory `Secrets` accessor. Empty/absent ⇒ no secrets for this task. |
| `secret_key_id` | `Option < String >` | CLOACI-T-0861 / D-5 — which pooled one-time key the `wrapped_secrets` are
wrapped to (the [`EphemeralKeyEntry::key_id`] the server consumed for this
dispatch). The agent looks up the matching private key, unwraps ONCE, and
discards it (one-time use). `None` ⇒ no secrets / pre-pool wrap. ALL
secrets in one dispatch wrap to the SAME key (this execution's key). |



### `cloacina::fleet::protocol::WrappedSecret`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

One at-rest secret resolved by the server and HPKE-wrapped to a single agent's ephemeral public key for one dispatch (CLOACI-T-0861).

The plaintext field-map is serialized to JSON then sealed; only `enc_b64`
(the HPKE encapsulated key) and `ciphertext_b64` (the AEAD ciphertext) travel
on the wire. The wrap is bound via AEAD associated data to the execution id +
secret name (see `security::fleet_secret::secret_aad`), so a captured blob
cannot be replayed against a different execution or secret even to the same
agent key.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` | The secret name (the lookup key the task resolves via `ctx.secret(name)`).
A NAME only — never a value. |
| `enc_b64` | `String` | HPKE encapsulated key, base64 (standard). |
| `ciphertext_b64` | `String` | HPKE AEAD ciphertext of the JSON `{field: value}` map, base64 (standard). |



### `cloacina::fleet::protocol::GraphWorkPacket`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

One reactor firing shipped to an agent for whole-graph execution (CLOACI-T-0722). The server pre-converts the reactor's `InputCache` snapshot into the FFI cache shape (source name → JSON string), so the agent's job is: fetch the cdylib by digest, `execute_graph(cache)`, report the outcome via the standard `/v1/agent/result` rendezvous keyed by `firing_id`. Accumulators + reactor state never leave the server — only the compute does.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `protocol_version` | `u32` |  |
| `firing_id` | `String` | Rendezvous key: a fresh UUID per firing. The agent reports it back as
the `task_execution_id` of its `AgentResultRequest` (the coordinator
is a plain uuid→result rendezvous; graph firings reuse it). |
| `graph_name` | `String` | The graph (== reactor) name inside the package. |
| `cache` | `std :: collections :: HashMap < String , String >` | The firing's input snapshot in FFI shape: source name → UTF-8 JSON. |
| `artifact` | `ArtifactRef` | Pointer to the cdylib artifact the agent must `dlopen`. |
| `timeout_seconds` | `u32` | Per-firing execution timeout. |
| `tenant_id` | `Option < String >` | Tenant scope (same semantics as [`WorkPacket::tenant_id`]). |
| `language` | `Option < String >` | Package language (CLOACI-T-0841): `"rust"`/absent → dlopen the cdylib
and FFI `execute_graph`; `"python"` → fetch the SOURCE archive, import
it (registering the graph executors), and execute via the Python
graph executor. Mirrors [`WorkPacket::language`]. |
| `wrapped_secrets` | `Vec < WrappedSecret >` | CLOACI-T-0861 — secrets the graph needs, HPKE-wrapped to the agent's
ephemeral public key. Same semantics as [`WorkPacket::wrapped_secrets`];
the AAD binds each blob to `firing_id` + secret name. |
| `secret_key_id` | `Option < String >` | CLOACI-T-0861 / D-5 — which pooled one-time key the `wrapped_secrets` are
wrapped to. Same semantics as [`WorkPacket::secret_key_id`]; AAD uses
`firing_id` as the execution id. |



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
