/*
 *  Copyright 2025-2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! StreamBackend trait and registry for pluggable broker backends.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

use once_cell::sync::Lazy;

/// Configuration for connecting to a stream broker.
#[derive(Debug, Clone)]
pub struct StreamConfig {
    pub broker_url: String,
    pub topic: String,
    pub group: String,
    pub extra: HashMap<String, String>,
}

/// A raw message from a stream broker.
#[derive(Debug, Clone)]
pub struct RawMessage {
    pub payload: Vec<u8>,
    pub offset: u64,
    pub timestamp: Option<i64>,
}

/// Errors from stream backend operations.
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("connection failed: {0}")]
    Connection(String),
    #[error("receive failed: {0}")]
    Receive(String),
    #[error("commit failed: {0}")]
    Commit(String),
    #[error("backend not found: {0}")]
    NotFound(String),
}

/// Trait for pluggable stream broker backends (Kafka, Redpanda, Iggy, etc.).
#[async_trait::async_trait]
pub trait StreamBackend: Send + 'static {
    /// Connect to the broker and subscribe to the topic.
    async fn connect(config: &StreamConfig) -> Result<Self, StreamError>
    where
        Self: Sized;

    /// Receive the next message. Blocks until available.
    async fn recv(&mut self) -> Result<RawMessage, StreamError>;

    /// Commit the current offset.
    async fn commit(&mut self) -> Result<(), StreamError>;

    /// Get the current uncommitted offset.
    fn current_offset(&self) -> Option<u64>;
}

/// Factory function type for creating stream backends.
pub type StreamBackendFactory = Box<
    dyn Fn(
            StreamConfig,
        )
            -> Pin<Box<dyn Future<Output = Result<Box<dyn StreamBackend>, StreamError>> + Send>>
        + Send
        + Sync,
>;

/// Registry of stream backend factories.
pub struct StreamBackendRegistry {
    backends: HashMap<String, StreamBackendFactory>,
}

impl StreamBackendRegistry {
    pub fn new() -> Self {
        Self {
            backends: HashMap::new(),
        }
    }

    /// Register a backend factory by type name.
    pub fn register(&mut self, type_name: &str, factory: StreamBackendFactory) {
        self.backends.insert(type_name.to_string(), factory);
    }

    /// Create a backend instance by type name.
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

    /// Check if a backend type is registered.
    pub fn has(&self, type_name: &str) -> bool {
        self.backends.contains_key(type_name)
    }
}

impl Default for StreamBackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global stream backend registry.
static GLOBAL_REGISTRY: Lazy<Mutex<StreamBackendRegistry>> =
    Lazy::new(|| Mutex::new(StreamBackendRegistry::new()));

/// Get a reference to the global stream backend registry.
pub fn global_stream_registry() -> &'static Mutex<StreamBackendRegistry> {
    &GLOBAL_REGISTRY
}

/// Register a backend in the global registry.
pub fn register_stream_backend(type_name: &str, factory: StreamBackendFactory) {
    global_stream_registry()
        .lock()
        .unwrap()
        .register(type_name, factory);
}

// ---------------------------------------------------------------------------
// Mock backend for testing
// ---------------------------------------------------------------------------

/// In-memory mock stream backend for testing without a real broker.
pub struct MockBackend {
    receiver: tokio::sync::mpsc::Receiver<Vec<u8>>,
    offset: u64,
    committed_offset: u64,
}

/// Handle for pushing messages into a MockBackend.
#[derive(Clone)]
pub struct MockBackendProducer {
    sender: tokio::sync::mpsc::Sender<Vec<u8>>,
}

impl MockBackendProducer {
    /// Push a message into the mock backend.
    pub async fn send(&self, payload: Vec<u8>) -> Result<(), StreamError> {
        self.sender
            .send(payload)
            .await
            .map_err(|e| StreamError::Receive(format!("mock send failed: {}", e)))
    }
}

/// Create a mock backend + producer pair.
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

#[async_trait::async_trait]
impl StreamBackend for MockBackend {
    async fn connect(_config: &StreamConfig) -> Result<Self, StreamError> {
        // MockBackend is created via mock_backend(), not connect()
        Err(StreamError::Connection(
            "use mock_backend() to create a MockBackend".to_string(),
        ))
    }

    async fn recv(&mut self) -> Result<RawMessage, StreamError> {
        let payload = self
            .receiver
            .recv()
            .await
            .ok_or_else(|| StreamError::Receive("mock channel closed".to_string()))?;
        self.offset += 1;
        Ok(RawMessage {
            payload,
            offset: self.offset,
            timestamp: None,
        })
    }

    async fn commit(&mut self) -> Result<(), StreamError> {
        self.committed_offset = self.offset;
        Ok(())
    }

    fn current_offset(&self) -> Option<u64> {
        if self.offset > 0 {
            Some(self.offset)
        } else {
            None
        }
    }
}

/// Register the mock backend in the global registry.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_backend_recv() {
        let (mut backend, producer) = mock_backend(10);

        producer.send(b"hello".to_vec()).await.unwrap();
        producer.send(b"world".to_vec()).await.unwrap();

        let msg1 = backend.recv().await.unwrap();
        assert_eq!(msg1.payload, b"hello");
        assert_eq!(msg1.offset, 1);

        let msg2 = backend.recv().await.unwrap();
        assert_eq!(msg2.payload, b"world");
        assert_eq!(msg2.offset, 2);
    }

    #[tokio::test]
    async fn test_mock_backend_commit() {
        let (mut backend, producer) = mock_backend(10);

        producer.send(b"data".to_vec()).await.unwrap();
        let _ = backend.recv().await.unwrap();

        assert_eq!(backend.current_offset(), Some(1));

        backend.commit().await.unwrap();
        assert_eq!(backend.committed_offset, 1);
    }

    #[tokio::test]
    async fn test_registry_lookup() {
        let mut registry = StreamBackendRegistry::new();
        assert!(!registry.has("mock"));

        registry.register(
            "mock",
            Box::new(|_config| {
                Box::pin(async { Err(StreamError::Connection("test".to_string())) })
            }),
        );

        assert!(registry.has("mock"));
        assert!(!registry.has("kafka"));
    }

    #[tokio::test]
    async fn test_registry_not_found() {
        let registry = StreamBackendRegistry::new();
        let result = registry
            .create(
                "nonexistent",
                StreamConfig {
                    broker_url: String::new(),
                    topic: String::new(),
                    group: String::new(),
                    extra: HashMap::new(),
                },
            )
            .await;
        assert!(result.is_err());
    }
}
