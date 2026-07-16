/*
 *  Copyright 2026 Colliery Software
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

//! CLOACI-I-0139 / T-0906 — the flagship proof: `cloacina-provider-kafka`
//! packaged as a SIGNED native provider streams REAL Kafka messages into the
//! computation-graph boundary channel.
//!
//! Full path under test (every layer of the initiative in one flow):
//!   1. `package_constructor_provider(new_native)` + Ed25519 sign — the exact
//!      code path `cloacinactl constructor package --native --sign-key ..`
//!      drives (builds the provider cdylib with rdkafka INSIDE it);
//!   2. `unpack_provider_archive` with the verifying key (fail-closed
//!      signature check) → an unpacked native provider dir;
//!   3. `load_stream_accumulator_source` → `call_streaming` on the
//!      `kafka_source` member (config-bound broker/topic/group);
//!   4. produce JSON messages to a fresh topic → each payload arrives as one
//!      boundary on the accumulator channel via
//!      `accumulator_runtime_with_source` (fidius pump thread owns the
//!      blocking `BaseConsumer::poll` loop);
//!   5. shutdown joins within the keepalive-bounded window (idle teardown).
//!
//! GATED on a reachable broker: self-skips (pass, with a note) when
//! `$CLOACINA_KAFKA_BROKER` (default `localhost:9092` — the dev stack's
//! `cloacina-kafka` container) is not accepting TCP. Requires the `kafka`
//! feature (the TEST uses rdkafka's producer; the provider ships its own).
#![cfg(all(feature = "constructors-wasm", feature = "kafka"))]

use std::path::PathBuf;
use std::time::Duration;

use ed25519_dalek::SigningKey;
use serde::Serialize;
use tokio::sync::mpsc;

use cloacina::computation_graph::accumulator::{
    accumulator_runtime_with_source, shutdown_signal, Accumulator, AccumulatorContext,
    AccumulatorError, AccumulatorRuntimeConfig, BoundarySender,
};
use cloacina::computation_graph::types::{deserialize, SourceName};
use cloacina::packaging::constructor_provider::{
    package_constructor_provider, ProviderPackageOptions,
};
use cloacina::registry::loader::constructor_loader::{
    load_stream_accumulator_source, unpack_provider_archive,
};

const PROVIDER: &str = "cloacina-provider-kafka";

/// The `kafka_source` member's `#[config]`, bound once at load.
#[derive(Serialize)]
struct KafkaSourceConfig {
    broker: String,
    topic: String,
    group: String,
}

/// Passthrough accumulator: each streamed payload IS the boundary.
struct Passthrough;

#[async_trait::async_trait]
impl Accumulator for Passthrough {
    type Output = Vec<u8>;
    fn process(&mut self, event: Vec<u8>) -> Option<Vec<u8>> {
        Some(event)
    }
    async fn init(&mut self, _ctx: &AccumulatorContext) -> Result<(), AccumulatorError> {
        Ok(())
    }
}

fn provider_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/constructor-contract/cloacina-provider-kafka")
}

fn broker() -> String {
    std::env::var("CLOACINA_KAFKA_BROKER").unwrap_or_else(|_| "localhost:9092".to_string())
}

/// TCP-probe the broker; `false` → the test self-skips (dev stack not up).
fn broker_reachable(broker: &str) -> bool {
    broker
        .to_socket_addrs_first()
        .map(|addr| std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok())
        .unwrap_or(false)
}

/// Tiny helper: first resolved SocketAddr of a `host:port` string.
trait ToFirstAddr {
    fn to_socket_addrs_first(&self) -> Option<std::net::SocketAddr>;
}
impl ToFirstAddr for str {
    fn to_socket_addrs_first(&self) -> Option<std::net::SocketAddr> {
        use std::net::ToSocketAddrs;
        self.to_socket_addrs().ok()?.next()
    }
}

#[tokio::test]
async fn kafka_provider_streams_real_messages_from_signed_native_package() {
    let broker = broker();
    if !broker_reachable(&broker) {
        eprintln!(
            "SKIP kafka_provider_streams_real_messages_from_signed_native_package: \
             no Kafka broker at {broker} (bring up the dev stack), test passes vacuously"
        );
        return;
    }

    let work = tempfile::TempDir::new().unwrap();

    // (1) Package SIGNED native — the exact `cloacinactl constructor package
    // --native --sign-key` path. Debug profile keeps the rdkafka build cached.
    let signing = SigningKey::from_bytes(&[9u8; 32]);
    let verifying = signing.verifying_key();
    let key_path = work.path().join("key.secret");
    std::fs::write(&key_path, signing.to_bytes()).unwrap();

    let archive_path = work.path().join("kafka-provider.cloacina");
    let opts = ProviderPackageOptions {
        output: Some(archive_path.clone()),
        sign_key: Some(key_path),
        release: false,
        ..ProviderPackageOptions::new_native(provider_dir())
    };
    let result = package_constructor_provider(&opts).expect("package kafka provider (native)");
    assert!(result.signed, "archive should be signed");
    assert_eq!(result.provider_name, PROVIDER);
    assert_eq!(result.constructors, vec!["kafka_source".to_string()]);

    // (2) Unpack with signature verification (fail-closed seam).
    let dest = tempfile::TempDir::new().unwrap();
    unpack_provider_archive(&result.archive, dest.path(), &[verifying])
        .expect("unpack + verify signed provider archive");

    // (3) Load the stream member natively; fresh topic + group per run so
    // `auto.offset.reset=earliest` reads exactly this run's messages.
    let run_id = uuid::Uuid::new_v4().simple().to_string();
    let topic = format!("t0906-{run_id}");
    let source = load_stream_accumulator_source(
        dest.path(),
        PROVIDER,
        "kafka_source",
        &KafkaSourceConfig {
            broker: broker.clone(),
            topic: topic.clone(),
            group: format!("t0906-group-{run_id}"),
        },
    )
    .await
    .expect("load kafka stream accumulator source");

    // (4) Drive it through the runtime and produce real messages.
    let (boundary_tx, mut boundary_rx) = mpsc::channel::<(SourceName, Vec<u8>)>(16);
    let (_socket_tx, socket_rx) = mpsc::channel::<Vec<u8>>(16);
    let (shutdown_tx, shutdown_rx) = shutdown_signal();
    let ctx = AccumulatorContext {
        output: BoundarySender::new(boundary_tx, SourceName::new("kafka_source")),
        name: "kafka_source".to_string(),
        shutdown: shutdown_rx,
        checkpoint: None,
        health: None,
    };
    let handle = tokio::spawn(accumulator_runtime_with_source(
        Passthrough,
        ctx,
        socket_rx,
        AccumulatorRuntimeConfig::default(),
        source,
    ));

    {
        use rdkafka::producer::{BaseProducer, BaseRecord, Producer};
        let producer: BaseProducer = rdkafka::config::ClientConfig::new()
            .set("bootstrap.servers", &broker)
            .set("message.timeout.ms", "10000")
            .create()
            .expect("create test producer");
        for n in 1..=3 {
            let payload = serde_json::json!({ "n": n }).to_string();
            producer
                .send(BaseRecord::<(), str>::to(&topic).payload(&payload))
                .expect("enqueue message");
        }
        producer.flush(Duration::from_secs(10)).expect("flush");
    }

    // Collect the three boundaries (keepalive ticks are filtered by the shim).
    let mut ns = Vec::new();
    for _ in 0..3 {
        let (name, bytes) = tokio::time::timeout(Duration::from_secs(30), boundary_rx.recv())
            .await
            .expect("boundary within 30s (broker reachable, topic auto-created)")
            .expect("boundary channel open");
        assert_eq!(name, SourceName::new("kafka_source"));
        let json_bytes: Vec<u8> = deserialize(&bytes).expect("decode boundary frame");
        let b: serde_json::Value = serde_json::from_slice(&json_bytes).expect("boundary json");
        ns.push(b.get("n").and_then(|v| v.as_u64()).expect("n field"));
    }
    ns.sort();
    assert_eq!(
        ns,
        vec![1, 2, 3],
        "all three real Kafka payloads crossed the native provider stream into the boundary channel"
    );

    // (5) Idle teardown: the topic has no more messages, so the pump thread is
    // parked in the poll loop — the keepalive tick must let shutdown resolve
    // within a couple of poll windows (no leaked runtime task).
    let _ = shutdown_tx.send(true);
    tokio::time::timeout(Duration::from_secs(15), handle)
        .await
        .expect("runtime task joins after shutdown (keepalive-bounded idle teardown)")
        .expect("runtime task did not panic");
}

/// CLOACI-T-0907 slice 1 — the PACKAGED-WORKFLOW declaration path: a `stream`
/// accumulator whose config map carries `provider`/`constructor` keys (the
/// `[[metadata.accumulators]]` surface) spawns through
/// `ProviderStreamAccumulatorFactory`, which resolves the provider from the
/// process-wide provider search path (the bundled-providers tree), binds the
/// string-map config BY NAME against the member's declared `#[config]` schema,
/// resolves `{{ VAR }}` templates, and streams real Kafka messages into the
/// boundary channel. This is exactly what a packaged CG's reconciled reactor
/// drives — minus the demo stack (T-0907 slice 2).
#[tokio::test]
async fn provider_stream_factory_drives_kafka_from_declaration_config() {
    use cloacina::computation_graph::accumulator::FreshnessHandle;
    use cloacina::computation_graph::packaging_bridge::ProviderStreamAccumulatorFactory;
    use cloacina::computation_graph::scheduler::{AccumulatorFactory, AccumulatorSpawnConfig};
    use cloacina::registry::loader::set_provider_search_path;

    let broker = broker();
    if !broker_reachable(&broker) {
        eprintln!(
            "SKIP provider_stream_factory_drives_kafka_from_declaration_config: \
             no Kafka broker at {broker} (bring up the dev stack), test passes vacuously"
        );
        return;
    }

    // Package (unsigned) + unpack into the process-wide provider search path —
    // standing in for the reconciler's bundled-providers unpack step.
    let work = tempfile::TempDir::new().unwrap();
    let archive_path = work.path().join("kafka-provider.cloacina");
    let opts = ProviderPackageOptions {
        output: Some(archive_path.clone()),
        release: false,
        ..ProviderPackageOptions::new_native(provider_dir())
    };
    let result = package_constructor_provider(&opts).expect("package kafka provider (native)");
    let providers_root = tempfile::TempDir::new().unwrap();
    unpack_provider_archive(&result.archive, providers_root.path(), &[])
        .expect("unpack provider archive");
    set_provider_search_path(providers_root.path());

    // The declaration config exactly as `[[metadata.accumulators]]` carries it:
    // routing keys + member #[config], broker via a `{{ VAR }}` template.
    std::env::set_var("CLOACINA_VAR_T0907_BROKER", &broker);
    let run_id = uuid::Uuid::new_v4().simple().to_string();
    let topic = format!("t0907-{run_id}");
    let group = format!("t0907-group-{run_id}");
    let decl_config: std::collections::HashMap<String, String> = [
        ("provider", "cloacina-provider-kafka"),
        ("constructor", "kafka_source"),
        ("broker", "{{ T0907_BROKER }}"),
        ("topic", topic.as_str()),
        ("group", group.as_str()),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();

    let (boundary_tx, mut boundary_rx) = mpsc::channel::<(SourceName, Vec<u8>)>(16);
    let (shutdown_tx, shutdown_rx) = shutdown_signal();
    let factory = ProviderStreamAccumulatorFactory::new(decl_config);
    let (_socket_tx, handle) = factory.spawn(
        "ticks".to_string(),
        boundary_tx,
        shutdown_rx,
        AccumulatorSpawnConfig {
            dal: None,
            health_tx: None,
            graph_name: "t0907-graph".to_string(),
            freshness: FreshnessHandle::default(),
        },
    );

    // Real messages through the declared provider.
    {
        use rdkafka::producer::{BaseProducer, BaseRecord, Producer};
        let producer: BaseProducer = rdkafka::config::ClientConfig::new()
            .set("bootstrap.servers", &broker)
            .set("message.timeout.ms", "10000")
            .create()
            .expect("create test producer");
        for n in 1..=2 {
            let payload = serde_json::json!({ "tick": n }).to_string();
            producer
                .send(BaseRecord::<(), str>::to(&topic).payload(&payload))
                .expect("enqueue message");
        }
        producer.flush(Duration::from_secs(10)).expect("flush");
    }

    let mut ticks = Vec::new();
    for _ in 0..2 {
        let (name, bytes) = tokio::time::timeout(Duration::from_secs(30), boundary_rx.recv())
            .await
            .expect("boundary within 30s")
            .expect("boundary channel open");
        assert_eq!(name, SourceName::new("ticks"));
        let json_bytes: Vec<u8> = deserialize(&bytes).expect("decode boundary frame");
        let b: serde_json::Value = serde_json::from_slice(&json_bytes).expect("boundary json");
        ticks.push(b.get("tick").and_then(|v| v.as_u64()).expect("tick field"));
    }
    ticks.sort();
    assert_eq!(
        ticks,
        vec![1, 2],
        "the [[metadata.accumulators]] declaration path streamed real Kafka into the boundary channel"
    );

    let _ = shutdown_tx.send(true);
    tokio::time::timeout(Duration::from_secs(15), handle)
        .await
        .expect("factory-spawned task joins after shutdown")
        .expect("factory-spawned task did not panic");
}
