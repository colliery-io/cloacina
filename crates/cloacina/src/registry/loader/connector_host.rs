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

//! The connector host (COLLIERY-I-0632 / CLOACINA-T-0973): load a Sync source or
//! destination from a provider package under the consumer's grants, and drive
//! its five methods. Used by the read and write tasks of the Sync workflow on an
//! agent, and by the catalog on the server.
//!
//! A connector component implements the `connector` fidius interface directly
//! ([`Connector`]), the way weir's guests do, so they port as they are. Config
//! binds once at load as the plain [`Config`] document, not through the suite
//! shell's name-keyed configure: a connector component may serve both a
//! `source` and a `destination` member of its provider, and the two differ
//! only in which methods the host dispatches. The member's kind is checked
//! against the manifest at load and again at each dispatch.
//!
//! Streaming: `read` is server streaming (a [`fidius_host::ChunkStream`] of
//! `ReadMessage` values, pulled by the host with backpressure) and `write` is
//! client streaming (the host produces `RecordBatch`es the guest pulls at its
//! own pace). Both cross the sandbox as fidius `Value`s and are decoded here.
//!
//! No tokio runtime on the hot path: a wasmtime call blocks the thread it runs
//! on, so callers drive [`ConnectorHandle::read_blocking`] and
//! [`ConnectorHandle::write`] from a blocking thread (`spawn_blocking`), as the
//! WASM task constructor does. The async [`ConnectorHandle::read`] exists for a
//! caller that already owns a blocking context and wants a `Stream`.

// The fidius `#[plugin_interface]` macro emits a `#[cfg(host)]`-style gate that
// the workspace's check-cfg lint flags as unknown — benign (see CLOACI-T-0821).
#![allow(unexpected_cfgs)]

use std::path::Path;
use std::sync::Arc;

use cloacina_constructor_contract::sync::{
    CheckResult, Config, ConnectorSpec, DiscoverOutcome, ReadContext, ReadMessage, RecordBatch,
    WriteContext, WriteOutcome,
};
use cloacina_constructor_contract::{
    CONNECTOR_INTERFACE, CONNECTOR_INTERFACE_VERSION, ConstructorManifest, METHOD_CHECK,
    METHOD_DISCOVER, METHOD_READ, METHOD_SPEC, METHOD_WRITE, PrimitiveKind,
};
use fidius_host::{CallError, PluginHandle, PluginHost};

use super::constructor_loader::read_member_manifest;
use super::grants::ResolvedGrants;
use crate::registry::error::LoaderError;

/// The host side of the `connector` interface. Declared with
/// `crate = "fidius_core"` so the macro emits the `Connector_WASM_DESCRIPTOR`
/// the loader links against; a guest declares the identical trait with
/// `crate = "fidius_guest"`. Same signatures, same interface hash
/// ([`cloacina_constructor_contract::sync`] documents the vtable order).
// The macro's expansion trips `redundant_field_names` on the method spans; the
// lint has nothing to act on in this file.
#[allow(clippy::redundant_field_names)]
#[fidius_macro::plugin_interface(version = 1, buffer = PluginAllocated, crate = "fidius_core")]
pub trait Connector: Send + Sync {
    /// The static descriptor: name, version, roles, sync modes, config schema.
    fn spec(&self) -> ConnectorSpec;
    /// Validate configuration and connectivity. Moves no data.
    fn check(&self, config: Config) -> CheckResult;
    /// The streams a source exposes.
    #[optional(since = 1)]
    fn discover(&self, config: Config) -> DiscoverOutcome;
    /// Stream records, checkpoints, logs and dead-letters from a resume point.
    #[optional(since = 1)]
    fn read(&self, ctx: ReadContext) -> fidius_core::Stream<ReadMessage>;
    /// Write a client stream of batches and report one outcome.
    #[optional(since = 1)]
    fn write(&self, ctx: WriteContext, batches: fidius_core::Stream<RecordBatch>) -> WriteOutcome;
}

/// What the host can do wrong, or the guest can do wrong at the boundary.
#[derive(Debug, thiserror::Error)]
pub enum ConnectorHostError {
    /// The fidius call failed: a trap, a transport error, a missing export.
    #[error("connector call failed: {0}")]
    Call(#[from] CallError),
    /// A stream item did not decode as the expected type.
    #[error("connector stream item does not decode: {0}")]
    Decode(String),
    /// The member's kind does not cover this method.
    #[error("connector member '{member}' is a {kind:?}; it has no `{method}`")]
    WrongKind {
        member: String,
        kind: PrimitiveKind,
        method: &'static str,
    },
}

/// A loaded source or destination member: the configured guest instance plus
/// its manifest entry. Cheap to clone; the guest lives as long as any clone.
#[derive(Clone)]
pub struct ConnectorHandle {
    handle: Arc<PluginHandle>,
    member: ConstructorManifest,
}

impl std::fmt::Debug for ConnectorHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectorHandle")
            .field("member", &self.member.name)
            .field("kind", &self.member.primitive_kind)
            .finish()
    }
}

/// Load the member `member_name` of the connector provider `package_name`,
/// found under `search_path`, with `config` bound once and the consumer's
/// `grants` enforced by the sandbox. The member must be a `source` or a
/// `destination` on the `connector` interface at the version this host speaks.
pub fn load_connector(
    search_path: impl AsRef<Path>,
    package_name: &str,
    member_name: &str,
    config: &Config,
    grants: &ResolvedGrants,
) -> Result<ConnectorHandle, LoaderError> {
    let search_path = search_path.as_ref();
    let host = PluginHost::builder()
        .search_path(search_path)
        .build()
        .map_err(|e| LoaderError::LibraryLoad {
            path: search_path.display().to_string(),
            error: format!("build plugin host: {e}"),
        })?;
    let dir = host
        .find_wasm_package(package_name)
        .map_err(|e| LoaderError::Validation {
            reason: format!("locate connector package '{package_name}': {e}"),
        })?;

    let member = read_member_manifest(&dir, member_name)?;
    if !matches!(
        member.primitive_kind,
        PrimitiveKind::Source | PrimitiveKind::Destination
    ) {
        return Err(LoaderError::Validation {
            reason: format!(
                "constructor '{}' is {:?}, not a source or a destination",
                member.name, member.primitive_kind
            ),
        });
    }
    if member.interface != CONNECTOR_INTERFACE {
        return Err(LoaderError::Validation {
            reason: format!(
                "constructor '{}' is on interface '{}', the connector host loads '{}'",
                member.name, member.interface, CONNECTOR_INTERFACE
            ),
        });
    }
    if member.interface_version != CONNECTOR_INTERFACE_VERSION {
        return Err(LoaderError::Validation {
            reason: format!(
                "constructor '{}' declares connector interface v{}, this host speaks v{}",
                member.name, member.interface_version, CONNECTOR_INTERFACE_VERSION
            ),
        });
    }

    let handle = host
        .load_wasm_configured_with_grants(
            package_name,
            &__fidius_Connector::Connector_WASM_DESCRIPTOR,
            config,
            grants.capabilities.clone(),
            grants.egress.clone(),
        )
        .map_err(|e| LoaderError::LibraryLoad {
            path: dir.display().to_string(),
            error: format!("load_wasm_configured_with_grants (connector): {e}"),
        })?;

    Ok(ConnectorHandle {
        handle: Arc::new(handle),
        member,
    })
}

impl ConnectorHandle {
    /// The member's kind: source or destination.
    pub fn kind(&self) -> PrimitiveKind {
        self.member.primitive_kind
    }

    /// The member's manifest entry.
    pub fn member(&self) -> &ConstructorManifest {
        &self.member
    }

    fn require(&self, kind: PrimitiveKind, method: &'static str) -> Result<(), ConnectorHostError> {
        if self.member.primitive_kind == kind {
            Ok(())
        } else {
            Err(ConnectorHostError::WrongKind {
                member: self.member.name.clone(),
                kind: self.member.primitive_kind,
                method,
            })
        }
    }

    /// The static descriptor. Blocking.
    pub fn spec(&self) -> Result<ConnectorSpec, ConnectorHostError> {
        Ok(self
            .handle
            .call_method::<(), ConnectorSpec>(METHOD_SPEC, &())?)
    }

    /// Validate `config` and connectivity. Blocking.
    pub fn check(&self, config: &Config) -> Result<CheckResult, ConnectorHostError> {
        Ok(self
            .handle
            .call_method::<(Config,), CheckResult>(METHOD_CHECK, &(config.clone(),))?)
    }

    /// The streams a source exposes under `config`. Blocking. Source only.
    pub fn discover(&self, config: &Config) -> Result<DiscoverOutcome, ConnectorHostError> {
        self.require(PrimitiveKind::Source, "discover")?;
        Ok(self
            .handle
            .call_method::<(Config,), DiscoverOutcome>(METHOD_DISCOVER, &(config.clone(),))?)
    }

    /// Open a read as a `Stream` of decoded messages. Source only. The guest
    /// produces lazily; dropping the stream tears the producer down.
    pub async fn read(&self, ctx: &ReadContext) -> Result<ReadStream, ConnectorHostError> {
        use futures::StreamExt as _;
        self.require(PrimitiveKind::Source, "read")?;
        let stream = self
            .handle
            .call_streaming::<ReadContext, ReadMessage>(METHOD_READ, ctx)
            .await?;
        Ok(Box::pin(stream.map(|item| {
            item.map_err(ConnectorHostError::Call).and_then(|v| {
                fidius_core::from_value::<ReadMessage>(v)
                    .map_err(|e| ConnectorHostError::Decode(e.to_string()))
            })
        })))
    }

    /// Open a read as a blocking iterator, for a caller on a blocking thread.
    /// Source only. Each `next` blocks until the guest yields the next message
    /// or ends the stream. Dropping the iterator tears the producer down.
    pub fn read_blocking(&self, ctx: &ReadContext) -> Result<ReadIter, ConnectorHostError> {
        let inner = futures::executor::block_on(self.read(ctx))?;
        Ok(ReadIter { inner })
    }

    /// Write `batches` to the destination as a client stream and return the
    /// outcome. Blocking. Destination only. The guest pulls batches at its own
    /// pace; an unbounded iterator stays bounded in memory.
    pub fn write(
        &self,
        ctx: &WriteContext,
        batches: impl IntoIterator<Item = RecordBatch, IntoIter: Send + 'static>,
    ) -> Result<WriteOutcome, ConnectorHostError> {
        self.require(PrimitiveKind::Destination, "write")?;
        Ok(self
            .handle
            .call_client_streaming::<RecordBatch, (WriteContext,), WriteOutcome>(
                METHOD_WRITE,
                batches,
                &(ctx.clone(),),
            )?)
    }
}

/// A read as a `Stream` of decoded messages, owned: it holds the guest alive and
/// tears the producer down when dropped.
pub type ReadStream =
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<ReadMessage, ConnectorHostError>> + Send>>;

/// The blocking form of a read stream: see [`ConnectorHandle::read_blocking`].
pub struct ReadIter {
    inner: ReadStream,
}

impl Iterator for ReadIter {
    type Item = Result<ReadMessage, ConnectorHostError>;

    fn next(&mut self) -> Option<Self::Item> {
        use futures::StreamExt as _;
        futures::executor::block_on(self.inner.next())
    }
}
