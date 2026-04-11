# cloacina::computation_graph::stream_backend <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


StreamBackend trait and registry for pluggable broker backends.

## Structs

### `cloacina::computation_graph::stream_backend::StreamConfig`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Configuration for connecting to a stream broker.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `broker_url` | `String` |  |
| `topic` | `String` |  |
| `group` | `String` |  |
| `extra` | `HashMap < String , String >` |  |



### `cloacina::computation_graph::stream_backend::RawMessage`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

A raw message from a stream broker.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `payload` | `Vec < u8 >` |  |
| `offset` | `u64` |  |
| `timestamp` | `Option < i64 >` |  |



### `cloacina::computation_graph::stream_backend::StreamBackendRegistry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Registry of stream backend factories.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `backends` | `HashMap < String , StreamBackendFactory >` |  |

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
            backends: HashMap::new(),
        }
    }
```

</details>



##### `register` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn register (& mut self , type_name : & str , factory : StreamBackendFactory)
```

Register a backend factory by type name.

<details>
<summary>Source</summary>

```rust
    pub fn register(&mut self, type_name: &str, factory: StreamBackendFactory) {
        self.backends.insert(type_name.to_string(), factory);
    }
```

</details>



##### `create` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn create (& self , type_name : & str , config : StreamConfig ,) -> Result < Box < dyn StreamBackend > , StreamError >
```

Create a backend instance by type name.

<details>
<summary>Source</summary>

```rust
    pub async fn create(
        &self,
        type_name: &str,
        config: StreamConfig,
    ) -> Result<Box<dyn StreamBackend>, StreamError> {
        let factory = self.backends.get(type_name).ok_or_else(|| {
            StreamError::NotFound(format!("backend type '{}' not registered", type_name))
        })?;
        factory(config).await
    }
```

</details>



##### `has` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn has (& self , type_name : & str) -> bool
```

Check if a backend type is registered.

<details>
<summary>Source</summary>

```rust
    pub fn has(&self, type_name: &str) -> bool {
        self.backends.contains_key(type_name)
    }
```

</details>



##### `create_future` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn create_future (& self , type_name : & str , config : StreamConfig ,) -> Option < Pin < Box < dyn Future < Output = Result < Box < dyn StreamBackend > , StreamError > > + Send > > >
```

Get the creation future for a backend type without holding the lock across await. Returns the future that will create the backend, or None if the type isn't registered.

<details>
<summary>Source</summary>

```rust
    pub fn create_future(
        &self,
        type_name: &str,
        config: StreamConfig,
    ) -> Option<Pin<Box<dyn Future<Output = Result<Box<dyn StreamBackend>, StreamError>> + Send>>>
    {
        let factory = self.backends.get(type_name)?;
        Some(factory(config))
    }
```

</details>





### `cloacina::computation_graph::stream_backend::MockBackend`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


In-memory mock stream backend for testing without a real broker.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `receiver` | `tokio :: sync :: mpsc :: Receiver < Vec < u8 > >` |  |
| `offset` | `u64` |  |
| `committed_offset` | `u64` |  |



### `cloacina::computation_graph::stream_backend::MockBackendProducer`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Handle for pushing messages into a MockBackend.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `sender` | `tokio :: sync :: mpsc :: Sender < Vec < u8 > >` |  |

#### Methods

##### `send` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn send (& self , payload : Vec < u8 >) -> Result < () , StreamError >
```

Push a message into the mock backend.

<details>
<summary>Source</summary>

```rust
    pub async fn send(&self, payload: Vec<u8>) -> Result<(), StreamError> {
        self.sender
            .send(payload)
            .await
            .map_err(|e| StreamError::Receive(format!("mock send failed: {}", e)))
    }
```

</details>





## Enums

### `cloacina::computation_graph::stream_backend::StreamError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors from stream backend operations.

#### Variants

- **`Connection`**
- **`Receive`**
- **`Commit`**
- **`NotFound`**



## Functions

### `cloacina::computation_graph::stream_backend::global_stream_registry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn global_stream_registry () -> & 'static Mutex < StreamBackendRegistry >
```

Get a reference to the global stream backend registry.

<details>
<summary>Source</summary>

```rust
pub fn global_stream_registry() -> &'static Mutex<StreamBackendRegistry> {
    &GLOBAL_REGISTRY
}
```

</details>



### `cloacina::computation_graph::stream_backend::register_stream_backend`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn register_stream_backend (type_name : & str , factory : StreamBackendFactory)
```

Register a backend in the global registry.

<details>
<summary>Source</summary>

```rust
pub fn register_stream_backend(type_name: &str, factory: StreamBackendFactory) {
    global_stream_registry()
        .lock()
        .unwrap()
        .register(type_name, factory);
}
```

</details>



### `cloacina::computation_graph::stream_backend::mock_backend`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn mock_backend (capacity : usize) -> (MockBackend , MockBackendProducer)
```

Create a mock backend + producer pair.

<details>
<summary>Source</summary>

```rust
pub fn mock_backend(capacity: usize) -> (MockBackend, MockBackendProducer) {
    let (tx, rx) = tokio::sync::mpsc::channel(capacity);
    (
        MockBackend {
            receiver: rx,
            offset: 0,
            committed_offset: 0,
        },
        MockBackendProducer { sender: tx },
    )
}
```

</details>



### `cloacina::computation_graph::stream_backend::register_mock_backend`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn register_mock_backend ()
```

Register the mock backend in the global registry.

<details>
<summary>Source</summary>

```rust
pub fn register_mock_backend() {
    register_stream_backend(
        "mock",
        Box::new(|_config| {
            Box::pin(async {
                Err(StreamError::Connection(
                    "mock backend must be created via mock_backend(), not the registry".to_string(),
                ))
            })
        }),
    );
}
```

</details>
