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

//! CLOACI-I-0139 / T-0906 — `cloacina-provider-kafka`: the flagship first-party
//! NATIVE stream-accumulator provider.
//!
//! One member, `kafka_source` (`kind = accumulator, mode = stream`): a
//! loop-owning Kafka consumer configured by `#[config] broker/topic/group`,
//! bound once at load. Each Kafka message payload is yielded verbatim as one
//! boundary-JSON item into the computation-graph boundary channel — the
//! provider-shipped replacement for core's host-side `KafkaEventSource`
//! (`rdkafka` lives HERE, not in core).
//!
//! ## Blocking is safe here
//!
//! `source()` returns a synchronous iterator whose `next()` blocks on
//! `BaseConsumer::poll(POLL_TIMEOUT)`. The cloacina loader drives a native
//! stream via fidius `call_streaming`, whose cdylib bridge pumps the iterator
//! on a **dedicated OS thread** (bounded channel back to async land) — so the
//! blocking poll never touches the tokio executor.
//!
//! ## Keepalive / teardown
//!
//! On each poll timeout the iterator yields an **empty string** — the
//! stream-accumulator keepalive the host skips (see
//! `StreamAccumulatorObject::source`). The pump thread only observes a dropped
//! consumer when it sends, so the tick bounds idle-stream teardown to one
//! `POLL_TIMEOUT` instead of parking the thread (and the consumer) forever.
//!
//! Grants are ADVISORY (native = trusted, I-0139 (e)) — the provider opens its
//! own network connection; there is no sandbox to grant egress through.

// The macro's wasm-guest glue paths are cfg'd out on the host build; rdkafka
// makes this crate native-only anyway (librdkafka has no wasm32-wasip2 target).
#![allow(dead_code)]
// fidius's `#[plugin_interface]` emits a check-cfg the workspace lint flags as
// unknown — benign (mirrors the loader's own allow; CLOACI-T-0821).
#![allow(unexpected_cfgs)]

use std::time::Duration;

use cloacina_macros::{constructor, constructor_provider};

use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::Message;

/// How long one `poll` blocks before yielding the `""` keepalive tick. Bounds
/// idle-stream teardown latency (see module docs).
const POLL_TIMEOUT: Duration = Duration::from_secs(2);

/// Streams messages from a Kafka topic into the computation-graph boundary
/// channel. Each message payload is one boundary-JSON item, verbatim — the
/// producer side owns the event schema, exactly like core's old host-side
/// Kafka event source.
#[constructor(
    kind = accumulator,
    mode = stream,
    name = "kafka_source",
    version = "0.1.0",
    contract = constructor_contract,
    description = "Streams Kafka messages (one payload = one boundary event) from a configured broker/topic/group.",
    author = "CLOACI-T-0906"
)]
pub struct KafkaSource {
    /// Kafka bootstrap servers (e.g. `localhost:9092`). Bound once at load.
    #[config]
    broker: String,
    /// Topic to subscribe to.
    #[config]
    topic: String,
    /// Consumer group id (offset tracking / resume).
    #[config]
    group: String,
}

impl KafkaSource {
    /// The ONLY thing the author writes for a stream accumulator: build the
    /// consumer from the bound config and return the poll-loop iterator. The
    /// iterator owns the consumer (an `-> impl Iterator` return can't borrow
    /// `&self`), so dropping the stream drops the consumer — clean teardown.
    ///
    /// Consumer-creation or subscribe failure panics: fidius catches the panic
    /// at the stream-init FFI boundary and surfaces it as a clear load-time
    /// error (`CallError::Panic`), which is the fail-fast behavior a
    /// misconfigured provider should have.
    fn source(&self) -> impl Iterator<Item = String> {
        let consumer: BaseConsumer = ClientConfig::new()
            .set("bootstrap.servers", &self.broker)
            .set("group.id", &self.group)
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "earliest")
            .create()
            .unwrap_or_else(|e| panic!("kafka_source: create consumer for '{}': {e}", self.broker));
        consumer
            .subscribe(&[self.topic.as_str()])
            .unwrap_or_else(|e| panic!("kafka_source: subscribe '{}': {e}", self.topic));

        let topic = self.topic.clone();
        std::iter::from_fn(move || {
            match consumer.poll(POLL_TIMEOUT) {
                // Message: its payload IS the boundary JSON (empty/absent
                // payloads degrade to the keepalive, i.e. are skipped).
                Some(Ok(msg)) => Some(
                    msg.payload()
                        .map(|p| String::from_utf8_lossy(p).into_owned())
                        .unwrap_or_default(),
                ),
                // Transient consumer error: log to stderr (no tracing inside the
                // plugin) and keep the stream alive with a keepalive tick.
                Some(Err(e)) => {
                    eprintln!("kafka_source[{topic}]: poll error (continuing): {e}");
                    Some(String::new())
                }
                // Poll timeout: the keepalive tick (host skips it; bounds
                // idle teardown — see module docs).
                None => Some(String::new()),
            }
        })
    }
}

// The provider suite shell (CLOACI-A-0011): ONE stream-accumulator member behind
// one native cdylib — the `__ProviderStreamAccumulator` holder whose
// server-streaming `source` the loader drives via `call_streaming` (T-0904).
constructor_provider!(
    name = "cloacina-provider-kafka",
    version = "0.1.0",
    contract = constructor_contract,
    stream_accumulator = [KafkaSource],
);
