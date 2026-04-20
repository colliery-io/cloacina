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

    /// Get the creation future for a backend type without holding the lock across await.
    /// Returns the future that will create the backend, or None if the type isn't registered.
    pub fn create_future(
        &self,
        type_name: &str,
        config: StreamConfig,
    ) -> Option<Pin<Box<dyn Future<Output = Result<Box<dyn StreamBackend>, StreamError>> + Send>>>
    {
        let factory = self.backends.get(type_name)?;
        Some(factory(config))
    }
}

impl Default for StreamBackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// The process-global stream backend registry was removed in CLOACI-T-0509.
// Stream backend factories are now owned by `crate::Runtime` and seeded from
// the `inventory` entries emitted by the stream-accumulator macros.

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

// ---------------------------------------------------------------------------
// Kafka backend (behind "kafka" feature flag)
// ---------------------------------------------------------------------------

#[cfg(feature = "kafka")]
pub mod kafka {
    use super::*;
    use futures::StreamExt;
    use rdkafka::config::ClientConfig;
    use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
    use rdkafka::message::Message;

    /// Kafka stream backend using rdkafka (librdkafka wrapper).
    ///
    /// Implements `StreamBackend` for consuming from a Kafka topic with
    /// consumer group offset tracking. Supports KRaft (KIP-500) brokers.
    pub struct KafkaStreamBackend {
        consumer: StreamConsumer,
        topic: String,
        offset: u64,
        committed_offset: u64,
    }

    #[async_trait::async_trait]
    impl StreamBackend for KafkaStreamBackend {
        async fn connect(config: &StreamConfig) -> Result<Self, StreamError>
        where
            Self: Sized,
        {
            let mut client_config = ClientConfig::new();
            client_config
                .set("bootstrap.servers", &config.broker_url)
                .set("group.id", &config.group)
                .set("enable.auto.commit", "false")
                .set("auto.offset.reset", "earliest");

            // Apply extra config overrides
            for (key, value) in &config.extra {
                client_config.set(key, value);
            }

            let consumer: StreamConsumer = client_config.create().map_err(|e| {
                StreamError::Connection(format!("Failed to create Kafka consumer: {}", e))
            })?;

            consumer.subscribe(&[&config.topic]).map_err(|e| {
                StreamError::Connection(format!(
                    "Failed to subscribe to topic '{}': {}",
                    config.topic, e
                ))
            })?;

            tracing::info!(
                topic = %config.topic,
                group = %config.group,
                broker = %config.broker_url,
                "Kafka stream backend connected"
            );

            Ok(Self {
                consumer,
                topic: config.topic.clone(),
                offset: 0,
                committed_offset: 0,
            })
        }

        async fn recv(&mut self) -> Result<RawMessage, StreamError> {
            let msg = self
                .consumer
                .stream()
                .next()
                .await
                .ok_or_else(|| StreamError::Receive("Kafka stream ended".to_string()))?
                .map_err(|e| StreamError::Receive(format!("Kafka receive error: {}", e)))?;

            let payload = msg
                .payload()
                .ok_or_else(|| StreamError::Receive("Kafka message has no payload".to_string()))?
                .to_vec();

            let offset = msg.offset() as u64;
            let timestamp = msg.timestamp().to_millis();

            self.offset = offset;

            Ok(RawMessage {
                payload,
                offset,
                timestamp,
            })
        }

        async fn commit(&mut self) -> Result<(), StreamError> {
            self.consumer
                .commit_consumer_state(CommitMode::Sync)
                .map_err(|e| StreamError::Commit(format!("Kafka commit failed: {}", e)))?;

            self.committed_offset = self.offset;

            tracing::debug!(
                topic = %self.topic,
                offset = self.committed_offset,
                "Kafka offset committed"
            );

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

    /// Create a factory for the Kafka backend. Register it on a `Runtime` via
    /// [`crate::Runtime::register_stream_backend`].
    pub fn kafka_backend_factory() -> super::StreamBackendFactory {
        Box::new(|config| {
            Box::pin(async move {
                let backend = KafkaStreamBackend::connect(&config).await?;
                Ok(Box::new(backend) as Box<dyn super::StreamBackend>)
            })
        })
    }
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
