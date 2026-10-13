---
title: "cloacina::computation_graph::stream_backend"
description: "StreamBackend trait and registry for pluggable broker backends."
editUrl: false
sidebar:
  order: 51
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

StreamBackend trait and registry for pluggable broker backends.

## Structs

### <span id="struct.StreamConfig"></span><code style="font-size: 0.85em">struct StreamConfig</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Configuration for connecting to a stream broker.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `broker_url` | `String` |  |
| `topic` | `String` |  |
| `group` | `String` |  |
| `extra` | `HashMap<String, String>` |  |

### <span id="struct.RawMessage"></span><code style="font-size: 0.85em">struct RawMessage</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

A raw message from a stream broker.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `payload` | `Vec<u8>` |  |
| `offset` | `u64` |  |
| `timestamp` | `Option<i64>` |  |

### <span id="struct.StreamBackendRegistry"></span><code style="font-size: 0.85em">struct StreamBackendRegistry</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Registry of stream backend factories.

#### Methods

##### <span id="StreamBackendRegistry.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new() -> Self
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

##### <span id="StreamBackendRegistry.register"></span><code>register</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn register(&mut self, type_name: &str, factory: StreamBackendFactory)
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

##### <span id="StreamBackendRegistry.create"></span><code>create</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn create(
    &self,
    type_name: &str,
    config: StreamConfig,
) -> Result<Box<dyn StreamBackend>, StreamError>
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

##### <span id="StreamBackendRegistry.has"></span><code>has</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn has(&self, type_name: &str) -> bool
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

##### <span id="StreamBackendRegistry.create_future"></span><code>create_​future</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn create_future(
    &self,
    type_name: &str,
    config: StreamConfig,
) -> Option<StreamBackendFuture>
```

Get the creation future for a backend type without holding the lock across await. Returns the future that will create the backend, or None if the type isn't registered.

<details>
<summary>Source</summary>

```rust
    pub fn create_future(
        &self,
        type_name: &str,
        config: StreamConfig,
    ) -> Option<StreamBackendFuture> {
        let factory = self.backends.get(type_name)?;
        Some(factory(config))
    }
```

</details>

### <span id="struct.MockBackend"></span><code style="font-size: 0.85em">struct MockBackend</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

In-memory mock stream backend for testing without a real broker.

### <span id="struct.MockBackendProducer"></span><code style="font-size: 0.85em">struct MockBackendProducer</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Handle for pushing messages into a MockBackend.

#### Methods

##### <span id="MockBackendProducer.send"></span><code>send</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn send(&self, payload: Vec<u8>) -> Result<(), StreamError>
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

### <span id="enum.StreamError"></span><code style="font-size: 0.85em">enum StreamError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors from stream backend operations.

#### Variants

- **`Connection`**
- **`Receive`**
- **`Commit`**
- **`NotFound`**

## Traits

### <span id="trait.StreamBackend"></span><code style="font-size: 0.85em">trait StreamBackend</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + 'static`

Trait for pluggable stream broker backends (Kafka, Redpanda, Iggy, etc.).

#### Methods

##### <span id="StreamBackend.connect"></span><code>connect</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn connect(config: &StreamConfig) -> Result<Self, StreamError>
where
    Self: Sized,
```

Connect to the broker and subscribe to the topic.

<details>
<summary>Source</summary>

```rust
    async fn connect(config: &StreamConfig) -> Result<Self, StreamError>
```

</details>

##### <span id="StreamBackend.recv"></span><code>recv</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn recv(&mut self) -> Result<RawMessage, StreamError>
```

Receive the next message. Blocks until available.

<details>
<summary>Source</summary>

```rust
    async fn recv(&mut self) -> Result<RawMessage, StreamError>;
```

</details>

##### <span id="StreamBackend.commit"></span><code>commit</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn commit(&mut self) -> Result<(), StreamError>
```

Commit the current offset.

<details>
<summary>Source</summary>

```rust
    async fn commit(&mut self) -> Result<(), StreamError>;
```

</details>

##### <span id="StreamBackend.current_offset"></span><code>current_​offset</code>

```rust
fn current_offset(&self) -> Option<u64>
```

Get the current uncommitted offset.

<details>
<summary>Source</summary>

```rust
    fn current_offset(&self) -> Option<u64>;
```

</details>

## Functions

### <span id="fn.mock_backend"></span><code style="font-size: 0.85em">fn mock_​backend</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn mock_backend(capacity: usize) -> (MockBackend, MockBackendProducer)
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

## Type aliases

| Name | Type | Description |
|------|------|-------------|
| <span id="type.StreamBackendFuture"></span>`StreamBackendFuture` | `Pin<Box<dyn Future<Output = Result<Box<dyn StreamBackend>, StreamError>> + Send>>` | Future returned by a `StreamBackendFactory`. Boxed so the factory signature can be expressed as a plain `Fn(...) -> StreamBackendFuture`. |
| <span id="type.StreamBackendFactory"></span>`StreamBackendFactory` | `Box<dyn Fn(StreamConfig) -> StreamBackendFuture + Send + Sync>` | Factory function type for creating stream backends. |
