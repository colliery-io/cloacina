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

//! CLOACI-T-0872 — certification harness for `cloacina-provider-kafka`.
//!
//! Adapted from `crates/cloacina/tests/constructor_provider_kafka_native.rs`
//! with two deliberate differences: cloacina resolves from CRATES.IO (this is
//! the point — proving the ship-form provider against RELEASED core), and an
//! unreachable broker is a hard FAILURE, not a skip — a certification that
//! can silently pass vacuously is worthless.

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
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn broker() -> String {
    std::env::var("CLOACINA_KAFKA_BROKER").unwrap_or_else(|_| "localhost:9092".to_string())
}

fn kafka_container() -> String {
    std::env::var("CLOACINA_KAFKA_CONTAINER").unwrap_or_else(|_| "cloacina-kafka".to_string())
}

fn broker_reachable(broker: &str) -> bool {
    use std::net::ToSocketAddrs;
    broker
        .to_socket_addrs()
        .ok()
        .and_then(|mut a| a.next())
        .map(|addr| std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok())
        .unwrap_or(false)
}

/// Produce newline-delimited payloads via the broker container's console
/// producer (core carries no kafka client, T-0898).
fn produce(topic: &str, payloads: &[String]) {
    let mut child = std::process::Command::new("docker")
        .args([
            "exec",
            "-i",
            &kafka_container(),
            "/opt/kafka/bin/kafka-console-producer.sh",
            "--bootstrap-server",
            "localhost:9092",
            "--topic",
            topic,
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn kafka-console-producer via docker exec");
    {
        use std::io::Write as _;
        let stdin = child.stdin.as_mut().expect("producer stdin");
        for p in payloads {
            writeln!(stdin, "{p}").expect("write payload");
        }
    }
    let out = child.wait_with_output().expect("producer exit");
    assert!(
        out.status.success(),
        "kafka-console-producer failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[tokio::main]
async fn main() {
    let broker = broker();
    assert!(
        broker_reachable(&broker),
        "CERTIFICATION FAILED: no Kafka broker reachable at {broker} \
         (set CLOACINA_KAFKA_BROKER; the wave workflow must provide one)"
    );

    let work = tempfile::TempDir::new().unwrap();

    // (1) Package SIGNED native — the `cloacinactl constructor package
    // --native --sign-key` path, via the PUBLISHED packaging API.
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
    println!("==> packaged + signed native provider via published API");

    // (2) Unpack with signature verification (fail-closed seam).
    let dest = tempfile::TempDir::new().unwrap();
    unpack_provider_archive(&result.archive, dest.path(), &[verifying])
        .expect("unpack + verify signed provider archive");
    println!("==> signature verified on unpack");

    // (3) Load the stream member natively; fresh topic + group per run.
    let run_id = uuid::Uuid::new_v4().simple().to_string();
    let topic = format!("certify-{run_id}");
    let source = load_stream_accumulator_source(
        dest.path(),
        PROVIDER,
        "kafka_source",
        &KafkaSourceConfig {
            broker: broker.clone(),
            topic: topic.clone(),
            group: format!("certify-group-{run_id}"),
        },
    )
    .await
    .expect("load kafka stream accumulator source");
    println!("==> native member loaded via published loader");

    // (4) Drive it through the runtime and stream real messages.
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

    let payloads: Vec<String> = (1..=3)
        .map(|n| serde_json::json!({ "n": n }).to_string())
        .collect();
    produce(&topic, &payloads);

    // Boundary frames are the fidius wire (bincode) wrapping the JSON payload
    // — decode the frame first, exactly as the in-repo native test does.
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
    assert_eq!(ns, vec![1, 2, 3], "all three payloads crossed the stream");

    let _ = shutdown_tx.send(true);
    tokio::time::timeout(Duration::from_secs(15), handle)
        .await
        .expect("runtime joins after shutdown")
        .expect("runtime task did not panic");

    println!("== CERTIFIED: cloacina-provider-kafka streamed 3/3 real messages");
    println!("   through the signed native package, loaded by PUBLISHED core.");
}
