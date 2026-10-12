---
title: "cloacina::computation_graph::registry"
description: "Endpoint registry — maps accumulator/reactor names to their channel senders."
sidebar:
  order: 65
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Endpoint registry — maps accumulator/reactor names to their channel senders.

The WebSocket handlers look up names in this registry to route messages to
the correct process. Supports broadcast: multiple accumulators registered
under the same name all receive the message.

## Structs

### <span id="struct.KeyContext"></span>`struct KeyContext<'a>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Caller identity for authorization checks.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `key_id` | `& 'a uuid :: Uuid` |  |
| `tenant_id` | `Option < & 'a str >` |  |
| `is_admin` | `bool` |  |



### <span id="struct.AccumulatorAuthPolicy"></span>`struct AccumulatorAuthPolicy`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Default`

Authorization policy for an accumulator endpoint.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `allow_all_authenticated` | `bool` | If true, any authenticated key is authorized (single-tenant default). |
| `allowed_tenants` | `Vec < String >` | Tenant IDs whose keys are authorized. Checked when allow_all is false. |
| `allowed_producers` | `Vec < uuid :: Uuid >` | PAK key IDs authorized to push to this accumulator (explicit override). |

#### Methods

##### <span id="AccumulatorAuthPolicy.allow_all"></span>`allow_all`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="AccumulatorAuthPolicy.for_tenant"></span>`for_tenant`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="AccumulatorAuthPolicy.is_authorized"></span>`is_authorized`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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





### <span id="struct.ReactorAuthPolicy"></span>`struct ReactorAuthPolicy`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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

##### <span id="ReactorAuthPolicy.allow_all"></span>`allow_all`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="ReactorAuthPolicy.for_tenant"></span>`for_tenant`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="ReactorAuthPolicy.is_authorized"></span>`is_authorized`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="ReactorAuthPolicy.is_operation_permitted"></span>`is_operation_permitted`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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





### <span id="struct.AccumulatorDescriptor"></span>`struct AccumulatorDescriptor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`

Discoverability metadata an accumulator self-registers at graph load (CLOACI-I-0128 follow-up). The runtime channel registration ([`EndpointRegistry::register_accumulator`]) only carries the socket sender; this descriptor adds the structural context the discovery API needs — chiefly the reactor/graph the accumulator feeds — so an operator can tell what connecting to `/v1/ws/accumulator/{name}` actually drives.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `reactor` | `String` | The reactor (graph) this accumulator feeds. |
| `tenant_id` | `Option < String >` | Owning tenant, or `None` for untagged single-tenant graphs. |



### <span id="struct.EndpointKey"></span>`struct EndpointKey`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord`

Composite lookup key for every endpoint map (CLOACI-T-0921).

Tenant is THE isolation boundary, so it is part of the *key*, not payload
hanging off the value. `tenant_id: None` is the untenanted/embedded
single-tenant case (and keeps the pre-multi-tenancy allow-all behaviour).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | `Option < String >` |  |
| `name` | `String` |  |

#### Methods

##### <span id="EndpointKey.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn new (tenant_id : Option < & str > , name : & str) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(tenant_id: Option<&str>, name: &str) -> Self {
        Self {
            tenant_id: tenant_id.map(|t| t.to_string()),
            name: name.to_string(),
        }
    }
```

</details>



##### <span id="EndpointKey.tenant_label"></span>`tenant_label`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn tenant_label (& self) -> & str
```

<details>
<summary>Source</summary>

```rust
    fn tenant_label(&self) -> &str {
        self.tenant_id.as_deref().unwrap_or("<untenanted>")
    }
```

</details>





### <span id="struct.EndpointOwner"></span>`struct EndpointOwner`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `PartialEq`, `Eq`

Full ownership identity recorded on each registration (CLOACI-T-0921).

The key is `(tenant, name)`; this is the rest of the provenance. It exists
so that (a) a same-tenant same-name claim by a *different* package/reactor
is rejected loudly at load time instead of silently cross-wiring, and
(b) deregistration only ever removes the entry its own owner installed.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | `Option < String >` | Owning tenant, or `None` for untenanted/embedded graphs. |
| `package` | `Option < String >` | Owning package, when the loader knows it. `None` for hand-wired and
embedded graphs. |
| `reactor` | `String` | Owning reactor (the graph this endpoint belongs to). |

#### Methods

##### <span id="EndpointOwner.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn new (tenant_id : Option < String > , package : Option < String > , reactor : impl Into < String > ,) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(
        tenant_id: Option<String>,
        package: Option<String>,
        reactor: impl Into<String>,
    ) -> Self {
        Self {
            tenant_id,
            package,
            reactor: reactor.into(),
        }
    }
```

</details>



##### <span id="EndpointOwner.embedded"></span>`embedded`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn embedded (reactor : impl Into < String >) -> Self
```

An owner with no tenant and no package — embedded/hand-wired graphs.

<details>
<summary>Source</summary>

```rust
    pub fn embedded(reactor: impl Into<String>) -> Self {
        Self::new(None, None, reactor)
    }
```

</details>



##### <span id="EndpointOwner.key"></span>`key`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn key (& self , name : & str) -> EndpointKey
```

The map key this owner registers `name` under.

<details>
<summary>Source</summary>

```rust
    pub fn key(&self, name: &str) -> EndpointKey {
        EndpointKey {
            tenant_id: self.tenant_id.clone(),
            name: name.to_string(),
        }
    }
```

</details>



##### <span id="EndpointOwner.describe"></span>`describe`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn describe (& self) -> String
```

Operator-facing provenance string for conflict errors.

<details>
<summary>Source</summary>

```rust
    fn describe(&self) -> String {
        match &self.package {
            Some(pkg) => format!("package '{}' (reactor '{}')", pkg, self.reactor),
            None => format!("reactor '{}'", self.reactor),
        }
    }
```

</details>





### <span id="struct.EndpointScope"></span>`struct EndpointScope<'a>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Copy`, `Default`

Tenant scope of a *caller* performing an endpoint lookup (CLOACI-T-0921).

The WebSocket and REST routes build this from the authenticated key so a
bare `{name}` path segment resolves inside the caller's own tenant.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | `Option < & 'a str >` |  |
| `is_admin` | `bool` |  |



### <span id="struct.AccumulatorEntry"></span>`struct AccumulatorEntry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


An accumulator registration: its owner plus the socket senders fanned out under this `(tenant, name)` key.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `owner` | `EndpointOwner` |  |
| `senders` | `Vec < mpsc :: Sender < Vec < u8 > > >` |  |



### <span id="struct.ReactorEntry"></span>`struct ReactorEntry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


A reactor registration: its owner, manual command channel, and shared handle.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `owner` | `EndpointOwner` |  |
| `sender` | `mpsc :: Sender < ManualCommand >` |  |
| `handle` | `ReactorHandle` |  |



### <span id="struct.EndpointRegistry"></span>`struct EndpointRegistry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Clone`

Registry mapping `(tenant, endpoint name)` to channel senders.

Shared between the computation graph scheduler (registers on spawn) and
WebSocket handlers (look up on message receipt). One instance serves the
whole server process, so every map is keyed by [`EndpointKey`] —
CLOACI-T-0921: keying by bare name let same-named accumulators from two
tenants broadcast into each other's boundary channels.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `Arc < RwLock < RegistryInner > >` |  |

#### Methods

##### <span id="EndpointRegistry.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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
                accumulator_policies: HashMap::new(),
                reactor_policies: HashMap::new(),
                accumulator_health: HashMap::new(),
                accumulator_freshness: HashMap::new(),
                accumulator_meta: HashMap::new(),
                accumulator_injects: HashMap::new(),
            })),
        }
    }
```

</details>



##### <span id="EndpointRegistry.note_accumulator_operator_inject"></span>`note_accumulator_operator_inject`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn note_accumulator_operator_inject (& self , name : & str , scope : EndpointScope < '_ >)
```

Record an operator inject into an accumulator (CLOACI-T-0776). Called only from the REST inject endpoint — the true human-operator path. (The WS accumulator-push path also goes through `send_to_accumulator` but is the data-source feed, e.g. the demo producer, so it must NOT count here.)

<details>
<summary>Source</summary>

```rust
    pub async fn note_accumulator_operator_inject(&self, name: &str, scope: EndpointScope<'_>) {
        let now = chrono::Utc::now().timestamp_millis();
        let mut inner = self.inner.write().await;
        let Ok(key) = resolve_key(&inner.accumulators, scope, name) else {
            return;
        };
        let entry = inner.accumulator_injects.entry(key).or_insert((0, 0));
        entry.0 += 1;
        entry.1 = now;
    }
```

</details>



##### <span id="EndpointRegistry.accumulator_inject_stat"></span>`accumulator_inject_stat`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn accumulator_inject_stat (& self , name : & str , scope : EndpointScope < '_ > ,) -> Option < (u64 , i64) >
```

`(operator-inject count, last-inject epoch ms)` for an accumulator, or `None` if it has never been operator-injected (CLOACI-T-0776).

<details>
<summary>Source</summary>

```rust
    pub async fn accumulator_inject_stat(
        &self,
        name: &str,
        scope: EndpointScope<'_>,
    ) -> Option<(u64, i64)> {
        let inner = self.inner.read().await;
        let key = resolve_key(&inner.accumulators, scope, name).ok()?;
        inner.accumulator_injects.get(&key).copied()
    }
```

</details>



##### <span id="EndpointRegistry.register_accumulator"></span>`register_accumulator`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn register_accumulator (& self , owner : & EndpointOwner , name : String , sender : mpsc :: Sender < Vec < u8 > > ,) -> Result < () , RegistryError >
```

Register an accumulator's socket sender under `(owner.tenant, name)`.

Several accumulator *instances* belonging to the same owner may share a
name — messages are broadcast to all of them, and the restart path
re-registers this way. A *different* owner claiming an already-taken
`(tenant, name)` is rejected with
[`RegistryError::EndpointOwnershipConflict`] (CLOACI-T-0921): silently
appending is what let two packages cross-wire each other's boundary
channels.

<details>
<summary>Source</summary>

```rust
    pub async fn register_accumulator(
        &self,
        owner: &EndpointOwner,
        name: String,
        sender: mpsc::Sender<Vec<u8>>,
    ) -> Result<(), RegistryError> {
        let key = owner.key(&name);
        let mut inner = self.inner.write().await;
        match inner.accumulators.get_mut(&key) {
            Some(entry) => {
                if &entry.owner != owner {
                    return Err(RegistryError::EndpointOwnershipConflict {
                        kind: "accumulator",
                        name,
                        tenant: key.tenant_label().to_string(),
                        existing: entry.owner.describe(),
                        incoming: owner.describe(),
                    });
                }
                entry.senders.push(sender);
            }
            None => {
                inner.accumulators.insert(
                    key,
                    AccumulatorEntry {
                        owner: owner.clone(),
                        senders: vec![sender],
                    },
                );
            }
        }
        Ok(())
    }
```

</details>



##### <span id="EndpointRegistry.register_reactor"></span>`register_reactor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn register_reactor (& self , owner : & EndpointOwner , name : String , sender : mpsc :: Sender < ManualCommand > , handle : ReactorHandle ,) -> Result < () , RegistryError >
```

Register a reactor's manual command sender and shared handle under `(owner.tenant, name)`. Same ownership rule as [`register_accumulator`](Self::register_accumulator): a different owner claiming a live `(tenant, name)` is a load-time error.

<details>
<summary>Source</summary>

```rust
    pub async fn register_reactor(
        &self,
        owner: &EndpointOwner,
        name: String,
        sender: mpsc::Sender<ManualCommand>,
        handle: ReactorHandle,
    ) -> Result<(), RegistryError> {
        let key = owner.key(&name);
        let mut inner = self.inner.write().await;
        if let Some(existing) = inner.reactors.get(&key) {
            if &existing.owner != owner {
                return Err(RegistryError::EndpointOwnershipConflict {
                    kind: "reactor",
                    name,
                    tenant: key.tenant_label().to_string(),
                    existing: existing.owner.describe(),
                    incoming: owner.describe(),
                });
            }
        }
        inner.reactors.insert(
            key,
            ReactorEntry {
                owner: owner.clone(),
                sender,
                handle,
            },
        );
        Ok(())
    }
```

</details>



##### <span id="EndpointRegistry.register_accumulator_meta"></span>`register_accumulator_meta`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn register_accumulator_meta (& self , owner : & EndpointOwner , name : String , descriptor : AccumulatorDescriptor ,)
```

Register an accumulator's discoverability descriptor (its parent reactor + tenant), self-supplied by the graph at load. Keyed by `(tenant, name)`; a re-load by the same owner overwrites. (CLOACI-I-0128 follow-up.)

<details>
<summary>Source</summary>

```rust
    pub async fn register_accumulator_meta(
        &self,
        owner: &EndpointOwner,
        name: String,
        descriptor: AccumulatorDescriptor,
    ) {
        let mut inner = self.inner.write().await;
        inner.accumulator_meta.insert(owner.key(&name), descriptor);
    }
```

</details>



##### <span id="EndpointRegistry.accumulator_descriptor"></span>`accumulator_descriptor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn accumulator_descriptor (& self , name : & str , scope : EndpointScope < '_ > ,) -> Option < AccumulatorDescriptor >
```

The discoverability descriptor for an accumulator visible to `scope`.

<details>
<summary>Source</summary>

```rust
    pub async fn accumulator_descriptor(
        &self,
        name: &str,
        scope: EndpointScope<'_>,
    ) -> Option<AccumulatorDescriptor> {
        let inner = self.inner.read().await;
        let key = resolve_key(&inner.accumulator_meta, scope, name).ok()?;
        inner.accumulator_meta.get(&key).cloned()
    }
```

</details>



##### <span id="EndpointRegistry.deregister_accumulator"></span>`deregister_accumulator`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn deregister_accumulator (& self , owner : & EndpointOwner , name : & str)
```

Deregister the accumulator `owner` registered under `name`, with all of its side tables. A no-op when the live entry belongs to somebody else — CLOACI-T-0921: unloading tenant A's package must not tear down tenant B's same-named accumulator.

<details>
<summary>Source</summary>

```rust
    pub async fn deregister_accumulator(&self, owner: &EndpointOwner, name: &str) {
        let key = owner.key(name);
        let mut inner = self.inner.write().await;
        match inner.accumulators.get(&key) {
            Some(entry) if &entry.owner == owner => {}
            Some(entry) => {
                tracing::warn!(
                    accumulator = %name,
                    tenant = %key.tenant_label(),
                    owner = %entry.owner.describe(),
                    requester = %owner.describe(),
                    "refusing to deregister an accumulator owned by another package"
                );
                return;
            }
            None => return,
        }
        inner.accumulators.remove(&key);
        inner.accumulator_meta.remove(&key);
        inner.accumulator_health.remove(&key);
        inner.accumulator_freshness.remove(&key);
        inner.accumulator_policies.remove(&key);
        inner.accumulator_injects.remove(&key);
    }
```

</details>



##### <span id="EndpointRegistry.deregister_reactor"></span>`deregister_reactor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn deregister_reactor (& self , owner : & EndpointOwner , name : & str)
```

Deregister the reactor `owner` registered under `name`. A no-op when the live entry belongs to somebody else (CLOACI-T-0921).

<details>
<summary>Source</summary>

```rust
    pub async fn deregister_reactor(&self, owner: &EndpointOwner, name: &str) {
        let key = owner.key(name);
        let mut inner = self.inner.write().await;
        match inner.reactors.get(&key) {
            Some(entry) if &entry.owner == owner => {}
            Some(entry) => {
                tracing::warn!(
                    reactor = %name,
                    tenant = %key.tenant_label(),
                    owner = %entry.owner.describe(),
                    requester = %owner.describe(),
                    "refusing to deregister a reactor owned by another package"
                );
                return;
            }
            None => return,
        }
        inner.reactors.remove(&key);
        inner.reactor_policies.remove(&key);
    }
```

</details>



##### <span id="EndpointRegistry.get_reactor_handle"></span>`get_reactor_handle`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn get_reactor_handle (& self , name : & str , scope : EndpointScope < '_ > ,) -> Option < ReactorHandle >
```

Get a reactor's shared handle (for GetState/Pause/Resume), resolved within the caller's tenant scope.

<details>
<summary>Source</summary>

```rust
    pub async fn get_reactor_handle(
        &self,
        name: &str,
        scope: EndpointScope<'_>,
    ) -> Option<ReactorHandle> {
        let inner = self.inner.read().await;
        let key = resolve_key(&inner.reactors, scope, name).ok()?;
        inner.reactors.get(&key).map(|e| e.handle.clone())
    }
```

</details>



##### <span id="EndpointRegistry.set_accumulator_policy"></span>`set_accumulator_policy`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn set_accumulator_policy (& self , owner : & EndpointOwner , name : String , policy : AccumulatorAuthPolicy ,)
```

Set the auth policy for an accumulator endpoint.

<details>
<summary>Source</summary>

```rust
    pub async fn set_accumulator_policy(
        &self,
        owner: &EndpointOwner,
        name: String,
        policy: AccumulatorAuthPolicy,
    ) {
        let mut inner = self.inner.write().await;
        inner.accumulator_policies.insert(owner.key(&name), policy);
    }
```

</details>



##### <span id="EndpointRegistry.set_reactor_policy"></span>`set_reactor_policy`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn set_reactor_policy (& self , owner : & EndpointOwner , name : String , policy : ReactorAuthPolicy ,)
```

Set the auth policy for a reactor endpoint.

<details>
<summary>Source</summary>

```rust
    pub async fn set_reactor_policy(
        &self,
        owner: &EndpointOwner,
        name: String,
        policy: ReactorAuthPolicy,
    ) {
        let mut inner = self.inner.write().await;
        inner.reactor_policies.insert(owner.key(&name), policy);
    }
```

</details>



##### <span id="EndpointRegistry.check_accumulator_auth"></span>`check_accumulator_auth`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


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
        let scope = EndpointScope::from_key_context(ctx);
        // CLOACI-T-0921: an unresolvable name (including another tenant's) is
        // indistinguishable from "no policy" — deny by default either way, and
        // never leak that the name exists in a tenant the caller can't see.
        let Ok(key) = resolve_key(&inner.accumulator_policies, scope, name) else {
            return Err(RegistryError::AccumulatorUnauthorized(name.to_string()));
        };
        match inner.accumulator_policies.get(&key) {
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



##### <span id="EndpointRegistry.check_reactor_auth"></span>`check_reactor_auth`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


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
        let scope = EndpointScope::from_key_context(ctx);
        let Ok(key) = resolve_key(&inner.reactor_policies, scope, name) else {
            return Err(RegistryError::ReactorUnauthorized(name.to_string()));
        };
        match inner.reactor_policies.get(&key) {
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



##### <span id="EndpointRegistry.check_reactor_op_auth"></span>`check_reactor_op_auth`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


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
        let scope = EndpointScope::from_key_context(ctx);
        let Ok(key) = resolve_key(&inner.reactor_policies, scope, name) else {
            return Err(RegistryError::ReactorUnauthorized(name.to_string()));
        };
        match inner.reactor_policies.get(&key) {
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



##### <span id="EndpointRegistry.send_to_accumulator"></span>`send_to_accumulator`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn send_to_accumulator (& self , name : & str , scope : EndpointScope < '_ > , bytes : Vec < u8 > ,) -> Result < usize , RegistryError >
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
        scope: EndpointScope<'_>,
        bytes: Vec<u8>,
    ) -> Result<usize, RegistryError> {
        let mut inner = self.inner.write().await;
        let key = match resolve_key(&inner.accumulators, scope, name) {
            Ok(k) => k,
            Err(ResolveMiss::NotFound) => {
                return Err(RegistryError::AccumulatorNotFound(name.to_string()))
            }
            Err(ResolveMiss::Ambiguous(tenants)) => {
                return Err(RegistryError::AmbiguousEndpoint {
                    kind: "accumulator",
                    name: name.to_string(),
                    tenants,
                })
            }
        };
        let senders = &mut inner
            .accumulators
            .get_mut(&key)
            .ok_or_else(|| RegistryError::AccumulatorNotFound(name.to_string()))?
            .senders;

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



##### <span id="EndpointRegistry.send_to_reactor"></span>`send_to_reactor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn send_to_reactor (& self , name : & str , scope : EndpointScope < '_ > , command : ManualCommand ,) -> Result < () , RegistryError >
```

Send a manual command to a reactor.

<details>
<summary>Source</summary>

```rust
    pub async fn send_to_reactor(
        &self,
        name: &str,
        scope: EndpointScope<'_>,
        command: ManualCommand,
    ) -> Result<(), RegistryError> {
        let inner = self.inner.read().await;
        let key = match resolve_key(&inner.reactors, scope, name) {
            Ok(k) => k,
            Err(ResolveMiss::NotFound) => {
                return Err(RegistryError::ReactorNotFound(name.to_string()))
            }
            Err(ResolveMiss::Ambiguous(tenants)) => {
                return Err(RegistryError::AmbiguousEndpoint {
                    kind: "reactor",
                    name: name.to_string(),
                    tenants,
                })
            }
        };
        let sender = &inner
            .reactors
            .get(&key)
            .ok_or_else(|| RegistryError::ReactorNotFound(name.to_string()))?
            .sender;

        sender
            .send(command)
            .await
            .map_err(|_| RegistryError::ReactorSendFailed {
                name: name.to_string(),
            })
    }
```

</details>



##### <span id="EndpointRegistry.list_accumulators"></span>`list_accumulators`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn list_accumulators (& self) -> Vec < String >
```

List all registered accumulator names, across every tenant. Operator/ diagnostic surface only — the tenant-filtered listing is [`list_accumulators_with_health_for_key`](Self::list_accumulators_with_health_for_key).

<details>
<summary>Source</summary>

```rust
    pub async fn list_accumulators(&self) -> Vec<String> {
        let inner = self.inner.read().await;
        inner.accumulators.keys().map(|k| k.name.clone()).collect()
    }
```

</details>



##### <span id="EndpointRegistry.list_reactors"></span>`list_reactors`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn list_reactors (& self) -> Vec < String >
```

List all registered reactor names, across every tenant.

<details>
<summary>Source</summary>

```rust
    pub async fn list_reactors(&self) -> Vec<String> {
        let inner = self.inner.read().await;
        inner.reactors.keys().map(|k| k.name.clone()).collect()
    }
```

</details>



##### <span id="EndpointRegistry.list_accumulator_keys"></span>`list_accumulator_keys`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn list_accumulator_keys (& self) -> Vec < EndpointKey >
```

List every registered accumulator as its full `(tenant, name)` key.

<details>
<summary>Source</summary>

```rust
    pub async fn list_accumulator_keys(&self) -> Vec<EndpointKey> {
        let inner = self.inner.read().await;
        inner.accumulators.keys().cloned().collect()
    }
```

</details>



##### <span id="EndpointRegistry.list_reactor_keys"></span>`list_reactor_keys`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn list_reactor_keys (& self) -> Vec < EndpointKey >
```

List every registered reactor as its full `(tenant, name)` key.

<details>
<summary>Source</summary>

```rust
    pub async fn list_reactor_keys(&self) -> Vec<EndpointKey> {
        let inner = self.inner.read().await;
        inner.reactors.keys().cloned().collect()
    }
```

</details>



##### <span id="EndpointRegistry.accumulator_count"></span>`accumulator_count`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn accumulator_count (& self , name : & str , scope : EndpointScope < '_ >) -> usize
```

Get the number of accumulator senders registered under a name, as visible to `scope`.

<details>
<summary>Source</summary>

```rust
    pub async fn accumulator_count(&self, name: &str, scope: EndpointScope<'_>) -> usize {
        let inner = self.inner.read().await;
        let Ok(key) = resolve_key(&inner.accumulators, scope, name) else {
            return 0;
        };
        inner
            .accumulators
            .get(&key)
            .map(|e| e.senders.len())
            .unwrap_or(0)
    }
```

</details>



##### <span id="EndpointRegistry.register_accumulator_health"></span>`register_accumulator_health`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn register_accumulator_health (& self , owner : & EndpointOwner , name : String , health_rx : watch :: Receiver < AccumulatorHealth > ,)
```

Register a health watch receiver for an accumulator.

<details>
<summary>Source</summary>

```rust
    pub async fn register_accumulator_health(
        &self,
        owner: &EndpointOwner,
        name: String,
        health_rx: watch::Receiver<AccumulatorHealth>,
    ) {
        let mut inner = self.inner.write().await;
        inner.accumulator_health.insert(owner.key(&name), health_rx);
    }
```

</details>



##### <span id="EndpointRegistry.register_accumulator_freshness"></span>`register_accumulator_freshness`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn register_accumulator_freshness (& self , owner : & EndpointOwner , name : String , handle : FreshnessHandle ,)
```

Register a freshness probe for an accumulator (CLOACI-T-0765): the shared events_total + last-event handle off its `BoundarySender`.

<details>
<summary>Source</summary>

```rust
    pub async fn register_accumulator_freshness(
        &self,
        owner: &EndpointOwner,
        name: String,
        handle: FreshnessHandle,
    ) {
        let mut inner = self.inner.write().await;
        inner.accumulator_freshness.insert(owner.key(&name), handle);
    }
```

</details>



##### <span id="EndpointRegistry.get_accumulator_health"></span>`get_accumulator_health`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn get_accumulator_health (& self , name : & str , scope : EndpointScope < '_ > ,) -> Option < AccumulatorHealth >
```

Get the current health of an accumulator visible to `scope`.

<details>
<summary>Source</summary>

```rust
    pub async fn get_accumulator_health(
        &self,
        name: &str,
        scope: EndpointScope<'_>,
    ) -> Option<AccumulatorHealth> {
        let inner = self.inner.read().await;
        let key = resolve_key(&inner.accumulator_health, scope, name).ok()?;
        inner
            .accumulator_health
            .get(&key)
            .map(|rx| rx.borrow().clone())
    }
```

</details>



##### <span id="EndpointRegistry.list_accumulators_with_health"></span>`list_accumulators_with_health`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn list_accumulators_with_health (& self ,) -> Vec < (String , AccumulatorHealth , Option < FreshnessHandle >) >
```

List all accumulators with their current health + freshness (CLOACI-T-0765).

<details>
<summary>Source</summary>

```rust
    pub async fn list_accumulators_with_health(
        &self,
    ) -> Vec<(String, AccumulatorHealth, Option<FreshnessHandle>)> {
        let inner = self.inner.read().await;
        inner
            .accumulators
            .keys()
            .map(|key| {
                let health = inner
                    .accumulator_health
                    .get(key)
                    .map(|rx| rx.borrow().clone())
                    .unwrap_or(AccumulatorHealth::Live); // default for accumulators without health tracking
                let fresh = inner.accumulator_freshness.get(key).cloned();
                (key.name.clone(), health, fresh)
            })
            .collect()
    }
```

</details>



##### <span id="EndpointRegistry.list_accumulators_with_health_for_key"></span>`list_accumulators_with_health_for_key`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>


```rust
async fn list_accumulators_with_health_for_key (& self , ctx : & KeyContext < '_ > ,) -> Vec < (String , AccumulatorHealth , Option < FreshnessHandle >) >
```

CLOACI-T-0579: list accumulators authorized for the given caller. Filters by each accumulator's `AccumulatorAuthPolicy::is_authorized` against the caller's `KeyContext`. Admin keys see everything; tenant-scoped keys see only their own accumulators; producer-pin keys see whichever they're explicitly allowed on.

Closes SEC-05 (cross-tenant health enumeration). The route handler
at `/v1/health/accumulators` uses this instead of the unfiltered
`list_accumulators_with_health` so tenant B can't enumerate tenant
A's accumulator names.

<details>
<summary>Source</summary>

```rust
    pub async fn list_accumulators_with_health_for_key(
        &self,
        ctx: &KeyContext<'_>,
    ) -> Vec<(String, AccumulatorHealth, Option<FreshnessHandle>)> {
        let inner = self.inner.read().await;
        inner
            .accumulators
            .keys()
            .filter(|key| {
                // CLOACI-T-0921: the key's own tenant is the first gate — an
                // entry owned by another tenant is invisible regardless of
                // policy. Untenanted entries stay globally visible (that is the
                // pre-tenancy single-tenant deployment).
                let tenant_visible = match (&key.tenant_id, ctx.tenant_id) {
                    (None, _) => true,
                    (Some(_), _) if ctx.is_admin => true,
                    (Some(owner), Some(caller)) => owner == caller,
                    (Some(_), None) => false,
                };
                if !tenant_visible {
                    return false;
                }
                // Then apply the per-accumulator policy. Accumulators without a
                // policy entry default to `allow_all_authenticated` so the
                // pre-tenancy single-tenant deployments aren't suddenly
                // empty after this change.
                match inner.accumulator_policies.get(*key) {
                    Some(policy) => policy.is_authorized(ctx),
                    None => AccumulatorAuthPolicy::allow_all().is_authorized(ctx),
                }
            })
            .map(|key| {
                let health = inner
                    .accumulator_health
                    .get(key)
                    .map(|rx| rx.borrow().clone())
                    .unwrap_or(AccumulatorHealth::Live);
                let fresh = inner.accumulator_freshness.get(key).cloned();
                (key.name.clone(), health, fresh)
            })
            .collect()
    }
```

</details>





### <span id="struct.RegistryInner"></span>`struct RegistryInner`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


#### Fields

| Name | Type | Description |
|------|------|-------------|
| `accumulators` | `HashMap < EndpointKey , AccumulatorEntry >` | `(tenant, accumulator name)` → owner + socket senders (Vec for broadcast). |
| `reactors` | `HashMap < EndpointKey , ReactorEntry >` | `(tenant, reactor name)` → owner + manual command sender + shared handle. |
| `accumulator_policies` | `HashMap < EndpointKey , AccumulatorAuthPolicy >` | `(tenant, accumulator name)` → auth policy. |
| `reactor_policies` | `HashMap < EndpointKey , ReactorAuthPolicy >` | `(tenant, reactor name)` → auth policy. |
| `accumulator_health` | `HashMap < EndpointKey , watch :: Receiver < AccumulatorHealth > >` | `(tenant, accumulator name)` → health watch receiver. |
| `accumulator_freshness` | `HashMap < EndpointKey , FreshnessHandle >` | `(tenant, accumulator name)` → freshness probe (events_total + last-event), CLOACI-T-0765. |
| `accumulator_meta` | `HashMap < EndpointKey , AccumulatorDescriptor >` | `(tenant, accumulator name)` → discoverability descriptor (CLOACI-I-0128 follow-up). |
| `accumulator_injects` | `HashMap < EndpointKey , (u64 , i64) >` | `(tenant, accumulator name)` → `(operator-inject count, last-inject epoch ms)`
(CLOACI-T-0776). Every `send_to_accumulator` is an operator inject — real
source events arrive on the accumulator's own socket, not here — so this
counts manual interventions for the UI to mark. |



## Enums

### <span id="enum.RegistryError"></span>`enum RegistryError`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Errors from registry operations.

#### Variants

- **`AccumulatorNotFound`**
- **`ReactorNotFound`**
- **`AccumulatorSendFailed`**
- **`ReactorSendFailed`**
- **`AccumulatorUnauthorized`**
- **`ReactorUnauthorized`**
- **`OperationNotPermitted`**
- **`EndpointOwnershipConflict`** - CLOACI-T-0921: two different owners (packages/reactors) inside the SAME
tenant tried to claim the same endpoint name. This is a load-time
rejection, not a silent cross-wire — the second package must rename.
- **`AmbiguousEndpoint`** - CLOACI-T-0921: an admin (tenant-less) caller named an endpoint that
exists in more than one tenant. Refuse rather than guess.



### <span id="enum.ReactorOp"></span>`enum ReactorOp`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Operations that can be performed on a reactor via WebSocket.

#### Variants

- **`ForceFire`**
- **`FireWith`**
- **`GetState`**
- **`Pause`**
- **`Resume`**
- **`GetHealth`**



### <span id="enum.ResolveMiss"></span>`enum ResolveMiss`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


Why a scoped lookup failed to land on exactly one key.

#### Variants

- **`NotFound`**
- **`Ambiguous`**



## Functions

### <span id="fn.resolve_key"></span>`fn resolve_key`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn resolve_key < V > (map : & HashMap < EndpointKey , V > , scope : EndpointScope < '_ > , name : & str ,) -> Result < EndpointKey , ResolveMiss >
```

Resolve a bare `name` to a concrete [`EndpointKey`] within `scope`.

Order:
1. the caller's own tenant — the normal multi-tenant path;
2. the untenanted entry (`tenant_id: None`) — embedded/pre-tenancy graphs,
which carry an allow-all policy and were always globally addressable;
3. admins only: a unique cross-tenant match. Two tenants owning the same
name is [`ResolveMiss::Ambiguous`], never a guess.
A non-admin caller can therefore never resolve another tenant's endpoint,
which is the whole point of CLOACI-T-0921.

<details>
<summary>Source</summary>

```rust
fn resolve_key<V>(
    map: &HashMap<EndpointKey, V>,
    scope: EndpointScope<'_>,
    name: &str,
) -> Result<EndpointKey, ResolveMiss> {
    if let Some(tenant) = scope.tenant_id {
        let key = EndpointKey::new(Some(tenant), name);
        if map.contains_key(&key) {
            return Ok(key);
        }
    }

    let untenanted = EndpointKey::new(None, name);
    if map.contains_key(&untenanted) {
        return Ok(untenanted);
    }

    if scope.is_admin {
        let matches: Vec<&EndpointKey> = map.keys().filter(|k| k.name == name).collect();
        match matches.len() {
            0 => return Err(ResolveMiss::NotFound),
            1 => return Ok(matches[0].clone()),
            _ => {
                return Err(ResolveMiss::Ambiguous(
                    matches
                        .iter()
                        .map(|k| k.tenant_label().to_string())
                        .collect(),
                ))
            }
        }
    }

    Err(ResolveMiss::NotFound)
}
```

</details>



