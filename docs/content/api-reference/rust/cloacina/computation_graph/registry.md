# cloacina::computation_graph::registry <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Endpoint registry — maps accumulator/reactor names to their channel senders.

The WebSocket handlers look up names in this registry to route messages to
the correct process. Supports broadcast: multiple accumulators registered
under the same name all receive the message.

## Structs

### `cloacina::computation_graph::registry::KeyContext`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Caller identity for authorization checks.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `key_id` | `& 'a uuid :: Uuid` |  |
| `tenant_id` | `Option < & 'a str >` |  |
| `is_admin` | `bool` |  |



### `cloacina::computation_graph::registry::AccumulatorAuthPolicy`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Default`

Authorization policy for an accumulator endpoint.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `allow_all_authenticated` | `bool` | If true, any authenticated key is authorized (single-tenant default). |
| `allowed_tenants` | `Vec < String >` | Tenant IDs whose keys are authorized. Checked when allow_all is false. |
| `allowed_producers` | `Vec < uuid :: Uuid >` | PAK key IDs authorized to push to this accumulator (explicit override). |

#### Methods

##### `allow_all` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn allow_all () -> Self
```

Create a policy that allows any authenticated key (global/single-tenant).

<details>
<summary>Source</summary>

```rust
    pub fn allow_all() -> Self {
        Self {
            allow_all_authenticated: true,
            allowed_tenants: Vec::new(),
            allowed_producers: Vec::new(),
        }
    }
```

</details>



##### `for_tenant` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn for_tenant (tenant_id : & str) -> Self
```

Create a policy scoped to a specific tenant.

<details>
<summary>Source</summary>

```rust
    pub fn for_tenant(tenant_id: &str) -> Self {
        Self {
            allow_all_authenticated: false,
            allowed_tenants: vec![tenant_id.to_string()],
            allowed_producers: Vec::new(),
        }
    }
```

</details>



##### `is_authorized` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_authorized (& self , ctx : & KeyContext) -> bool
```

Check if a key is authorized.

<details>
<summary>Source</summary>

```rust
    pub fn is_authorized(&self, ctx: &KeyContext) -> bool {
        if self.allow_all_authenticated || ctx.is_admin {
            return true;
        }
        if self.allowed_producers.contains(ctx.key_id) {
            return true;
        }
        if let Some(key_tenant) = ctx.tenant_id {
            return self.allowed_tenants.iter().any(|t| t == key_tenant);
        }
        false
    }
```

</details>





### `cloacina::computation_graph::registry::ReactorAuthPolicy`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Default`

Authorization policy for a reactor endpoint.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `allow_all_authenticated` | `bool` | If true, any authenticated key is authorized (single-tenant default). |
| `allowed_tenants` | `Vec < String >` | Tenant IDs whose keys are authorized. Checked when allow_all is false. |
| `allowed_operators` | `Vec < uuid :: Uuid >` | PAK key IDs authorized to connect (explicit override). |
| `operation_permissions` | `HashMap < uuid :: Uuid , Vec < ReactorOp > >` | Per-key operation restrictions. If a key is in allowed_operators
but not in this map, all operations are permitted. |

#### Methods

##### `allow_all` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn allow_all () -> Self
```

Create a policy that allows any authenticated key (global/single-tenant).

<details>
<summary>Source</summary>

```rust
    pub fn allow_all() -> Self {
        Self {
            allow_all_authenticated: true,
            allowed_tenants: Vec::new(),
            allowed_operators: Vec::new(),
            operation_permissions: HashMap::new(),
        }
    }
```

</details>



##### `for_tenant` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn for_tenant (tenant_id : & str) -> Self
```

Create a policy scoped to a specific tenant.

<details>
<summary>Source</summary>

```rust
    pub fn for_tenant(tenant_id: &str) -> Self {
        Self {
            allow_all_authenticated: false,
            allowed_tenants: vec![tenant_id.to_string()],
            allowed_operators: Vec::new(),
            operation_permissions: HashMap::new(),
        }
    }
```

</details>



##### `is_authorized` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_authorized (& self , ctx : & KeyContext) -> bool
```

Check if a key is authorized to connect.

<details>
<summary>Source</summary>

```rust
    pub fn is_authorized(&self, ctx: &KeyContext) -> bool {
        if self.allow_all_authenticated || ctx.is_admin {
            return true;
        }
        if self.allowed_operators.contains(ctx.key_id) {
            return true;
        }
        if let Some(key_tenant) = ctx.tenant_id {
            return self.allowed_tenants.iter().any(|t| t == key_tenant);
        }
        false
    }
```

</details>



##### `is_operation_permitted` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_operation_permitted (& self , ctx : & KeyContext , op : & ReactorOp) -> bool
```

Check if a key is authorized for a specific operation.

<details>
<summary>Source</summary>

```rust
    pub fn is_operation_permitted(&self, ctx: &KeyContext, op: &ReactorOp) -> bool {
        if self.allow_all_authenticated || ctx.is_admin {
            return true;
        }
        if !self.is_authorized(ctx) {
            return false;
        }
        // If no per-key restrictions, all ops are allowed
        match self.operation_permissions.get(ctx.key_id) {
            None => true,
            Some(permitted) => permitted.contains(op),
        }
    }
```

</details>





### `cloacina::computation_graph::registry::EndpointRegistry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Registry mapping endpoint names to channel senders.

Shared between the Reactive Scheduler (registers on spawn) and
WebSocket handlers (look up on message receipt).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `Arc < RwLock < RegistryInner > >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new () -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RegistryInner {
                accumulators: HashMap::new(),
                reactors: HashMap::new(),
                reactor_handles: HashMap::new(),
                accumulator_policies: HashMap::new(),
                reactor_policies: HashMap::new(),
                accumulator_health: HashMap::new(),
            })),
        }
    }
```

</details>



##### `register_accumulator` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn register_accumulator (& self , name : String , sender : mpsc :: Sender < Vec < u8 > >)
```

Register an accumulator's socket sender under a name.

Multiple accumulators can share the same name — messages are broadcast
to all of them.

<details>
<summary>Source</summary>

```rust
    pub async fn register_accumulator(&self, name: String, sender: mpsc::Sender<Vec<u8>>) {
        let mut inner = self.inner.write().await;
        inner
            .accumulators
            .entry(name)
            .or_insert_with(Vec::new)
            .push(sender);
    }
```

</details>



##### `register_reactor` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn register_reactor (& self , name : String , sender : mpsc :: Sender < ManualCommand > , handle : ReactorHandle ,)
```

Register a reactor's manual command sender and shared handle.

<details>
<summary>Source</summary>

```rust
    pub async fn register_reactor(
        &self,
        name: String,
        sender: mpsc::Sender<ManualCommand>,
        handle: ReactorHandle,
    ) {
        let mut inner = self.inner.write().await;
        inner.reactors.insert(name.clone(), sender);
        inner.reactor_handles.insert(name, handle);
    }
```

</details>



##### `deregister_accumulator` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn deregister_accumulator (& self , name : & str)
```

Deregister all accumulators under a name.

<details>
<summary>Source</summary>

```rust
    pub async fn deregister_accumulator(&self, name: &str) {
        let mut inner = self.inner.write().await;
        inner.accumulators.remove(name);
    }
```

</details>



##### `deregister_reactor` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn deregister_reactor (& self , name : & str)
```

Deregister a reactor by name.

<details>
<summary>Source</summary>

```rust
    pub async fn deregister_reactor(&self, name: &str) {
        let mut inner = self.inner.write().await;
        inner.reactors.remove(name);
        inner.reactor_handles.remove(name);
    }
```

</details>



##### `get_reactor_handle` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn get_reactor_handle (& self , name : & str) -> Option < ReactorHandle >
```

Get a reactor's shared handle (for GetState/Pause/Resume).

<details>
<summary>Source</summary>

```rust
    pub async fn get_reactor_handle(&self, name: &str) -> Option<ReactorHandle> {
        let inner = self.inner.read().await;
        inner.reactor_handles.get(name).cloned()
    }
```

</details>



##### `set_accumulator_policy` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn set_accumulator_policy (& self , name : String , policy : AccumulatorAuthPolicy)
```

Set the auth policy for an accumulator endpoint.

<details>
<summary>Source</summary>

```rust
    pub async fn set_accumulator_policy(&self, name: String, policy: AccumulatorAuthPolicy) {
        let mut inner = self.inner.write().await;
        inner.accumulator_policies.insert(name, policy);
    }
```

</details>



##### `set_reactor_policy` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn set_reactor_policy (& self , name : String , policy : ReactorAuthPolicy)
```

Set the auth policy for a reactor endpoint.

<details>
<summary>Source</summary>

```rust
    pub async fn set_reactor_policy(&self, name: String, policy: ReactorAuthPolicy) {
        let mut inner = self.inner.write().await;
        inner.reactor_policies.insert(name, policy);
    }
```

</details>



##### `check_accumulator_auth` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn check_accumulator_auth (& self , name : & str , ctx : & KeyContext < '_ > ,) -> Result < () , RegistryError >
```

Check if a key is authorized for an accumulator endpoint.

Returns Ok(()) if authorized, Err if denied.
Deny by default: no policy = no access.

<details>
<summary>Source</summary>

```rust
    pub async fn check_accumulator_auth(
        &self,
        name: &str,
        ctx: &KeyContext<'_>,
    ) -> Result<(), RegistryError> {
        let inner = self.inner.read().await;
        match inner.accumulator_policies.get(name) {
            None => Err(RegistryError::AccumulatorUnauthorized(name.to_string())),
            Some(policy) => {
                if policy.is_authorized(ctx) {
                    Ok(())
                } else {
                    Err(RegistryError::AccumulatorUnauthorized(name.to_string()))
                }
            }
        }
    }
```

</details>



##### `check_reactor_auth` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn check_reactor_auth (& self , name : & str , ctx : & KeyContext < '_ > ,) -> Result < () , RegistryError >
```

Check if a key is authorized for a reactor endpoint.

<details>
<summary>Source</summary>

```rust
    pub async fn check_reactor_auth(
        &self,
        name: &str,
        ctx: &KeyContext<'_>,
    ) -> Result<(), RegistryError> {
        let inner = self.inner.read().await;
        match inner.reactor_policies.get(name) {
            None => Err(RegistryError::ReactorUnauthorized(name.to_string())),
            Some(policy) => {
                if policy.is_authorized(ctx) {
                    Ok(())
                } else {
                    Err(RegistryError::ReactorUnauthorized(name.to_string()))
                }
            }
        }
    }
```

</details>



##### `check_reactor_op_auth` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn check_reactor_op_auth (& self , name : & str , ctx : & KeyContext < '_ > , op : & ReactorOp ,) -> Result < () , RegistryError >
```

Check if a key is authorized for a specific reactor operation.

<details>
<summary>Source</summary>

```rust
    pub async fn check_reactor_op_auth(
        &self,
        name: &str,
        ctx: &KeyContext<'_>,
        op: &ReactorOp,
    ) -> Result<(), RegistryError> {
        let inner = self.inner.read().await;
        match inner.reactor_policies.get(name) {
            None => Err(RegistryError::ReactorUnauthorized(name.to_string())),
            Some(policy) => {
                if policy.is_operation_permitted(ctx, op) {
                    Ok(())
                } else {
                    Err(RegistryError::OperationNotPermitted {
                        name: name.to_string(),
                        op: format!("{:?}", op),
                    })
                }
            }
        }
    }
```

</details>



##### `send_to_accumulator` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn send_to_accumulator (& self , name : & str , bytes : Vec < u8 > ,) -> Result < usize , RegistryError >
```

Send bytes to all accumulators registered under `name`.

Returns error if no accumulators are registered, or if all channels
are closed. Channels that are closed are pruned on send.

<details>
<summary>Source</summary>

```rust
    pub async fn send_to_accumulator(
        &self,
        name: &str,
        bytes: Vec<u8>,
    ) -> Result<usize, RegistryError> {
        let mut inner = self.inner.write().await;
        let senders = inner
            .accumulators
            .get_mut(name)
            .ok_or_else(|| RegistryError::AccumulatorNotFound(name.to_string()))?;

        if senders.is_empty() {
            return Err(RegistryError::AccumulatorNotFound(name.to_string()));
        }

        let mut sent = 0;
        let mut closed = Vec::new();

        for (i, sender) in senders.iter().enumerate() {
            match sender.try_send(bytes.clone()) {
                Ok(()) => sent += 1,
                Err(mpsc::error::TrySendError::Closed(_)) => closed.push(i),
                Err(mpsc::error::TrySendError::Full(_)) => {
                    // Channel full — log but count as sent (data will be dropped)
                    tracing::warn!(
                        accumulator = %name,
                        "accumulator channel full, dropping message"
                    );
                }
            }
        }

        // Prune closed channels (reverse order to preserve indices)
        for i in closed.into_iter().rev() {
            senders.remove(i);
        }

        if sent == 0 {
            return Err(RegistryError::AccumulatorSendFailed {
                name: name.to_string(),
            });
        }

        Ok(sent)
    }
```

</details>



##### `send_to_reactor` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn send_to_reactor (& self , name : & str , command : ManualCommand ,) -> Result < () , RegistryError >
```

Send a manual command to a reactor.

<details>
<summary>Source</summary>

```rust
    pub async fn send_to_reactor(
        &self,
        name: &str,
        command: ManualCommand,
    ) -> Result<(), RegistryError> {
        let inner = self.inner.read().await;
        let sender = inner
            .reactors
            .get(name)
            .ok_or_else(|| RegistryError::ReactorNotFound(name.to_string()))?;

        sender
            .send(command)
            .await
            .map_err(|_| RegistryError::ReactorSendFailed {
                name: name.to_string(),
            })
    }
```

</details>



##### `list_accumulators` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn list_accumulators (& self) -> Vec < String >
```

List all registered accumulator names.

<details>
<summary>Source</summary>

```rust
    pub async fn list_accumulators(&self) -> Vec<String> {
        let inner = self.inner.read().await;
        inner.accumulators.keys().cloned().collect()
    }
```

</details>



##### `list_reactors` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn list_reactors (& self) -> Vec < String >
```

List all registered reactor names.

<details>
<summary>Source</summary>

```rust
    pub async fn list_reactors(&self) -> Vec<String> {
        let inner = self.inner.read().await;
        inner.reactors.keys().cloned().collect()
    }
```

</details>



##### `accumulator_count` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn accumulator_count (& self , name : & str) -> usize
```

Get the number of accumulators registered under a name.

<details>
<summary>Source</summary>

```rust
    pub async fn accumulator_count(&self, name: &str) -> usize {
        let inner = self.inner.read().await;
        inner.accumulators.get(name).map(|v| v.len()).unwrap_or(0)
    }
```

</details>



##### `register_accumulator_health` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn register_accumulator_health (& self , name : String , health_rx : watch :: Receiver < AccumulatorHealth > ,)
```

Register a health watch receiver for an accumulator.

<details>
<summary>Source</summary>

```rust
    pub async fn register_accumulator_health(
        &self,
        name: String,
        health_rx: watch::Receiver<AccumulatorHealth>,
    ) {
        let mut inner = self.inner.write().await;
        inner.accumulator_health.insert(name, health_rx);
    }
```

</details>



##### `get_accumulator_health` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn get_accumulator_health (& self , name : & str) -> Option < AccumulatorHealth >
```

Get the current health of an accumulator.

<details>
<summary>Source</summary>

```rust
    pub async fn get_accumulator_health(&self, name: &str) -> Option<AccumulatorHealth> {
        let inner = self.inner.read().await;
        inner
            .accumulator_health
            .get(name)
            .map(|rx| rx.borrow().clone())
    }
```

</details>



##### `list_accumulators_with_health` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn list_accumulators_with_health (& self) -> Vec < (String , AccumulatorHealth) >
```

List all accumulators with their current health status.

<details>
<summary>Source</summary>

```rust
    pub async fn list_accumulators_with_health(&self) -> Vec<(String, AccumulatorHealth)> {
        let inner = self.inner.read().await;
        inner
            .accumulators
            .keys()
            .map(|name| {
                let health = inner
                    .accumulator_health
                    .get(name)
                    .map(|rx| rx.borrow().clone())
                    .unwrap_or(AccumulatorHealth::Live); // default for accumulators without health tracking
                (name.clone(), health)
            })
            .collect()
    }
```

</details>





### `cloacina::computation_graph::registry::RegistryInner`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


#### Fields

| Name | Type | Description |
|------|------|-------------|
| `accumulators` | `HashMap < String , Vec < mpsc :: Sender < Vec < u8 > > > >` | Accumulator name → list of socket senders (Vec for broadcast). |
| `reactors` | `HashMap < String , mpsc :: Sender < ManualCommand > >` | Reactor name → manual command sender. |
| `reactor_handles` | `HashMap < String , ReactorHandle >` | Reactor name → shared handle for GetState/Pause/Resume. |
| `accumulator_policies` | `HashMap < String , AccumulatorAuthPolicy >` | Accumulator name → auth policy. |
| `reactor_policies` | `HashMap < String , ReactorAuthPolicy >` | Reactor name → auth policy. |
| `accumulator_health` | `HashMap < String , watch :: Receiver < AccumulatorHealth > >` | Accumulator name → health watch receiver. |



## Enums

### `cloacina::computation_graph::registry::RegistryError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors from registry operations.

#### Variants

- **`AccumulatorNotFound`**
- **`ReactorNotFound`**
- **`AccumulatorSendFailed`**
- **`ReactorSendFailed`**
- **`AccumulatorUnauthorized`**
- **`ReactorUnauthorized`**
- **`OperationNotPermitted`**



### `cloacina::computation_graph::registry::ReactorOp` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Operations that can be performed on a reactor via WebSocket.

#### Variants

- **`ForceFire`**
- **`FireWith`**
- **`GetState`**
- **`Pause`**
- **`Resume`**
- **`GetHealth`**
