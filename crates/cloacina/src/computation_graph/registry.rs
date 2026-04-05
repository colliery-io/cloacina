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

//! Endpoint registry — maps accumulator/reactor names to their channel senders.
//!
//! The WebSocket handlers look up names in this registry to route messages to
//! the correct process. Supports broadcast: multiple accumulators registered
//! under the same name all receive the message.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use super::reactor::ManualCommand;

/// Errors from registry operations.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("no accumulator registered for '{0}'")]
    AccumulatorNotFound(String),

    #[error("no reactor registered for '{0}'")]
    ReactorNotFound(String),

    #[error("failed to send to accumulator '{name}': channel closed")]
    AccumulatorSendFailed { name: String },

    #[error("failed to send to reactor '{name}': channel closed")]
    ReactorSendFailed { name: String },
}

/// Registry mapping endpoint names to channel senders.
///
/// Shared between the Reactive Scheduler (registers on spawn) and
/// WebSocket handlers (look up on message receipt).
#[derive(Clone)]
pub struct EndpointRegistry {
    inner: Arc<RwLock<RegistryInner>>,
}

struct RegistryInner {
    /// Accumulator name → list of socket senders (Vec for broadcast).
    accumulators: HashMap<String, Vec<mpsc::Sender<Vec<u8>>>>,
    /// Reactor name → manual command sender.
    reactors: HashMap<String, mpsc::Sender<ManualCommand>>,
}

impl EndpointRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RegistryInner {
                accumulators: HashMap::new(),
                reactors: HashMap::new(),
            })),
        }
    }

    /// Register an accumulator's socket sender under a name.
    ///
    /// Multiple accumulators can share the same name — messages are broadcast
    /// to all of them.
    pub async fn register_accumulator(&self, name: String, sender: mpsc::Sender<Vec<u8>>) {
        let mut inner = self.inner.write().await;
        inner
            .accumulators
            .entry(name)
            .or_insert_with(Vec::new)
            .push(sender);
    }

    /// Register a reactor's manual command sender.
    pub async fn register_reactor(&self, name: String, sender: mpsc::Sender<ManualCommand>) {
        let mut inner = self.inner.write().await;
        inner.reactors.insert(name, sender);
    }

    /// Deregister all accumulators under a name.
    pub async fn deregister_accumulator(&self, name: &str) {
        let mut inner = self.inner.write().await;
        inner.accumulators.remove(name);
    }

    /// Deregister a reactor by name.
    pub async fn deregister_reactor(&self, name: &str) {
        let mut inner = self.inner.write().await;
        inner.reactors.remove(name);
    }

    /// Send bytes to all accumulators registered under `name`.
    ///
    /// Returns error if no accumulators are registered, or if all channels
    /// are closed. Channels that are closed are pruned on send.
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

    /// Send a manual command to a reactor.
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

    /// List all registered accumulator names.
    pub async fn list_accumulators(&self) -> Vec<String> {
        let inner = self.inner.read().await;
        inner.accumulators.keys().cloned().collect()
    }

    /// List all registered reactor names.
    pub async fn list_reactors(&self) -> Vec<String> {
        let inner = self.inner.read().await;
        inner.reactors.keys().cloned().collect()
    }

    /// Get the number of accumulators registered under a name.
    pub async fn accumulator_count(&self, name: &str) -> usize {
        let inner = self.inner.read().await;
        inner.accumulators.get(name).map(|v| v.len()).unwrap_or(0)
    }
}

impl Default for EndpointRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_send_deregister_accumulator() {
        let registry = EndpointRegistry::new();
        let (tx, mut rx) = mpsc::channel(10);

        registry.register_accumulator("alpha".to_string(), tx).await;

        let data = vec![1, 2, 3];
        let sent = registry
            .send_to_accumulator("alpha", data.clone())
            .await
            .unwrap();
        assert_eq!(sent, 1);

        let received = rx.recv().await.unwrap();
        assert_eq!(received, data);

        registry.deregister_accumulator("alpha").await;

        let err = registry
            .send_to_accumulator("alpha", vec![4, 5])
            .await
            .unwrap_err();
        assert!(matches!(err, RegistryError::AccumulatorNotFound(_)));
    }

    #[tokio::test]
    async fn test_broadcast_to_multiple_accumulators() {
        let registry = EndpointRegistry::new();
        let (tx1, mut rx1) = mpsc::channel(10);
        let (tx2, mut rx2) = mpsc::channel(10);

        registry
            .register_accumulator("alpha".to_string(), tx1)
            .await;
        registry
            .register_accumulator("alpha".to_string(), tx2)
            .await;

        assert_eq!(registry.accumulator_count("alpha").await, 2);

        let data = vec![10, 20, 30];
        let sent = registry
            .send_to_accumulator("alpha", data.clone())
            .await
            .unwrap();
        assert_eq!(sent, 2);

        assert_eq!(rx1.recv().await.unwrap(), data);
        assert_eq!(rx2.recv().await.unwrap(), data);
    }

    #[tokio::test]
    async fn test_send_to_unregistered_accumulator() {
        let registry = EndpointRegistry::new();
        let err = registry
            .send_to_accumulator("nonexistent", vec![1])
            .await
            .unwrap_err();
        assert!(matches!(err, RegistryError::AccumulatorNotFound(_)));
    }

    #[tokio::test]
    async fn test_register_send_deregister_reactor() {
        let registry = EndpointRegistry::new();
        let (tx, mut rx) = mpsc::channel(10);

        registry
            .register_reactor("market_maker".to_string(), tx)
            .await;

        registry
            .send_to_reactor("market_maker", ManualCommand::ForceFire)
            .await
            .unwrap();

        let cmd = rx.recv().await.unwrap();
        assert!(matches!(cmd, ManualCommand::ForceFire));

        registry.deregister_reactor("market_maker").await;

        let err = registry
            .send_to_reactor("market_maker", ManualCommand::ForceFire)
            .await
            .unwrap_err();
        assert!(matches!(err, RegistryError::ReactorNotFound(_)));
    }

    #[tokio::test]
    async fn test_send_to_unregistered_reactor() {
        let registry = EndpointRegistry::new();
        let err = registry
            .send_to_reactor("nonexistent", ManualCommand::ForceFire)
            .await
            .unwrap_err();
        assert!(matches!(err, RegistryError::ReactorNotFound(_)));
    }

    #[tokio::test]
    async fn test_closed_accumulator_channel_pruned() {
        let registry = EndpointRegistry::new();
        let (tx1, rx1) = mpsc::channel(10);
        let (tx2, mut rx2) = mpsc::channel(10);

        registry
            .register_accumulator("alpha".to_string(), tx1)
            .await;
        registry
            .register_accumulator("alpha".to_string(), tx2)
            .await;

        // Drop rx1 — its channel is now closed
        drop(rx1);

        let data = vec![42];
        let sent = registry
            .send_to_accumulator("alpha", data.clone())
            .await
            .unwrap();
        assert_eq!(sent, 1); // only tx2 succeeded

        assert_eq!(rx2.recv().await.unwrap(), data);

        // Closed channel should have been pruned
        assert_eq!(registry.accumulator_count("alpha").await, 1);
    }

    #[tokio::test]
    async fn test_list_accumulators_and_reactors() {
        let registry = EndpointRegistry::new();
        let (tx1, _rx1) = mpsc::channel(10);
        let (tx2, _rx2) = mpsc::channel::<ManualCommand>(10);

        registry
            .register_accumulator("alpha".to_string(), tx1)
            .await;
        registry
            .register_reactor("market_maker".to_string(), tx2)
            .await;

        let accumulators = registry.list_accumulators().await;
        assert_eq!(accumulators, vec!["alpha"]);

        let reactors = registry.list_reactors().await;
        assert_eq!(reactors, vec!["market_maker"]);
    }
}
