---
title: "09 - Kafka-Sourced Computation Graphs"
description: "Declare stream accumulators that consume from Kafka topics — no external producer needed"
weight: 30
---

In the previous tutorials, events arrived via WebSocket pushed by an external process. In this tutorial you'll declare a **stream accumulator** in `package.toml`. The server reads events from a Kafka topic automatically — once the graph loads, the accumulator connects to Kafka and pulls messages without any application code changes.

## What you'll learn

- The `[[metadata.accumulators]]` configuration block in `package.toml`
- Setting `CLOACINA_VAR_KAFKA_BROKER` so the server knows where to connect
- Starting Kafka locally with `docker compose`
- Creating topics and producing test messages with `kafka-console-producer.sh`
- Three accumulator patterns: passthrough, stateful, and batch
- Verifying the graph fires after Kafka messages arrive

## Prerequisites

- Tutorial 07 complete (you know how to package and upload a CG)
- Docker and Docker Compose available
- Your Cloacina server built with the `kafka` feature flag enabled
- `curl` and `python3` available

## Time estimate

30–40 minutes

---

## Background: how stream accumulators work

Normally, an accumulator receives events over its WebSocket channel. A **stream accumulator** is identical at the reactor boundary — it still delivers serialized `serde_json::Value` bytes — but instead of waiting for WebSocket frames, a background Kafka reader feeds bytes into the same channel automatically.

The key difference is in `package.toml`. When the reconciler loads your graph and sees `accumulator_type = "stream"`, it spawns a `StreamBackendAccumulatorFactory` instead of the default `PassthroughAccumulatorFactory`. The factory starts a background `tokio::spawn` task that connects to Kafka using the `KafkaStreamBackend` (rdkafka) and forwards every message payload into the accumulator's channel.

The server reads the broker URL from the `CLOACINA_VAR_KAFKA_BROKER` environment variable. Topic and consumer group come from the `[metadata.accumulators.config]` block.

---

## Step 1: Start Kafka

The Cloacina development environment includes Kafka (Apache Kafka 3.9 in KRaft mode — no ZooKeeper). Start it:

```bash
# From the Cloacina repository root
docker compose -f .angreal/docker-compose.yaml up -d kafka
```

Wait for the health check to pass:

```bash
docker compose -f .angreal/docker-compose.yaml ps
```

You should see `cloacina-kafka` with status `healthy`. This usually takes 20–30 seconds on first start.

Verify it's accepting connections:

```bash
docker exec cloacina-kafka \
  /opt/kafka/bin/kafka-broker-api-versions.sh \
  --bootstrap-server localhost:9092
```

If you see a list of API versions, Kafka is ready.

## Step 2: Create the Kafka topic

```bash
docker exec cloacina-kafka \
  /opt/kafka/bin/kafka-topics.sh \
  --bootstrap-server localhost:9092 \
  --create \
  --topic price.orderbook \
  --partitions 1 \
  --replication-factor 1 \
  --if-not-exists
```

Expected output:
```
Created topic price.orderbook.
```

## Step 3: Set CLOACINA_VAR_KAFKA_BROKER

The server resolves broker URLs through the `CLOACINA_VAR_` convention. The accumulator's `broker` config key names the variable to look up:

```bash
export CLOACINA_VAR_KAFKA_BROKER="localhost:9092"
```

If you're running the server as a system service, add this to the service environment file. The variable must be set before the graph is loaded — changing it after loading has no effect on already-running accumulator tasks.

## Step 4: Write `package.toml` with a stream accumulator

Create a new project directory:

```bash
mkdir kafka-price-signal
cd kafka-price-signal
```

Write `package.toml`:

```toml
[package]
name = "kafka-price-signal"
version = "0.1.0"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
package_type = ["computation_graph"]
graph_name = "kafka_price_signal"
language = "rust"
description = "Price signal graph driven by a Kafka topic"
reaction_mode = "when_any"
input_strategy = "latest"

[[metadata.accumulators]]
name = "orderbook"
accumulator_type = "stream"

[metadata.accumulators.config]
broker = "KAFKA_BROKER"
topic = "price.orderbook"
group = "kafka-price-signal-group"
```

The `[[metadata.accumulators]]` array table declares each accumulator. Fields:

| Field | Required | Meaning |
|---|---|---|
| `name` | Yes | Must match the accumulator name in the graph macro |
| `accumulator_type` | Yes | `"passthrough"` (WebSocket) or `"stream"` (Kafka) |
| `config.broker` | Yes (stream) | Variable name for the broker URL (resolved from `CLOACINA_VAR_{name}`) |
| `config.topic` | Yes (stream) | Kafka topic to consume from |
| `config.group` | No | Consumer group ID — defaults to `{name}_group` |

{{< hint type=info title="Multiple accumulators" >}}
You can mix `passthrough` and `stream` accumulators in the same graph. For example, one accumulator could receive WebSocket pushes while another pulls from a Kafka topic. Add another `[[metadata.accumulators]]` block for each additional accumulator.
{{< /hint >}}

## Step 5: Write `Cargo.toml` and `build.rs`

`Cargo.toml`:

```toml
[package]
name = "kafka-price-signal"
version = "0.1.0"
edition = "2021"

[workspace]

[features]
default = ["packaged"]
packaged = []

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
cloacina-computation-graph = "0.3"
cloacina-macros = "0.3"
cloacina-workflow-plugin = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
async-trait = "0.1"
tokio = { version = "1.0", features = ["full"] }

[build-dependencies]
cloacina-build = "0.3"
```

`build.rs`:

```rust
fn main() {
    cloacina_build::configure();
}
```

## Step 6: Write `src/lib.rs` — passthrough pattern

The simplest pattern: each Kafka message is deserialized as-is and forwarded to the reactor. The reactor fires on every message (because `reaction_mode = "when_any"`).

```rust
use serde::{Deserialize, Serialize};

/// Each Kafka message must be a JSON object matching this struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub best_bid: f64,
    pub best_ask: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceSignal {
    pub mid_price: f64,
    pub spread: f64,
    pub spread_bps: f64,
}

#[cloacina_macros::computation_graph(
    react = when_any(orderbook),
    graph = {
        compute(orderbook) -> emit,
    }
)]
pub mod kafka_price_signal {
    use super::*;

    pub async fn compute(orderbook: Option<&OrderBook>) -> PriceSignal {
        match orderbook {
            Some(ob) => {
                let mid = (ob.best_bid + ob.best_ask) / 2.0;
                let spread = ob.best_ask - ob.best_bid;
                PriceSignal {
                    mid_price: mid,
                    spread,
                    spread_bps: (spread / mid) * 10_000.0,
                }
            }
            None => PriceSignal {
                mid_price: 0.0,
                spread: 0.0,
                spread_bps: 0.0,
            },
        }
    }

    pub async fn emit(signal: &PriceSignal) -> String {
        format!(
            "mid={:.4} spread={:.4} ({:.2} bps)",
            signal.mid_price, signal.spread, signal.spread_bps
        )
    }
}
```

## Step 7: Package and upload

```bash
cd ..
tar -cjf kafka-price-signal.cloacina \
  --transform 's,^kafka-price-signal,kafka-price-signal-0.1.0,' \
  kafka-price-signal/package.toml \
  kafka-price-signal/Cargo.toml \
  kafka-price-signal/build.rs \
  kafka-price-signal/src/lib.rs

BASE_URL="http://localhost:8080"
TOKEN="clk_your_token_here"

curl -s -w "\nHTTP %{http_code}\n" \
  -X POST "${BASE_URL}/tenants/public/workflows" \
  -H "Authorization: Bearer ${TOKEN}" \
  -F "file=@kafka-price-signal.cloacina;type=application/octet-stream"
```

Wait for compilation (60–120 seconds on first build):

```bash
for i in $(seq 1 30); do
  result=$(curl -s "${BASE_URL}/v1/health/reactors" \
    -H "Authorization: Bearer ${TOKEN}")
  if echo "$result" | python3 -c "import sys,json; d=json.load(sys.stdin); exit(0 if any(r['name']=='kafka_price_signal' for r in d['reactors']) else 1)" 2>/dev/null; then
    echo "Graph loaded!"
    echo "$result" | python3 -m json.tool
    break
  fi
  echo "Waiting... ($i/30)"
  sleep 5
done
```

## Step 8: Produce messages and verify the graph fires

Use `kafka-console-producer.sh` inside the container to send a test event. Each line is one Kafka message.

```bash
echo '{"best_bid": 100.10, "best_ask": 100.15}' | \
  docker exec -i cloacina-kafka \
  /opt/kafka/bin/kafka-console-producer.sh \
  --bootstrap-server localhost:9092 \
  --topic price.orderbook
```

After a short delay (the Kafka consumer poll interval is at most a few hundred milliseconds), verify the reactor fired:

```bash
curl -s "${BASE_URL}/v1/health/reactors/kafka_price_signal" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -m json.tool
```

Expected:

```json
{
  "name": "kafka_price_signal",
  "health": {
    "state": "running",
    "last_fired_at": "2026-04-06T14:22:11.034Z",
    "fire_count": 1
  },
  "accumulators": ["orderbook"],
  "paused": false
}
```

Produce several more messages and watch `fire_count` increment:

```bash
for i in $(seq 1 10); do
  bid=$(python3 -c "import random; print(round(100 + random.uniform(-0.5, 0.5), 4))")
  ask=$(python3 -c "import random; b=${bid}; print(round(b + random.uniform(0.01, 0.20), 4))")
  echo "{\"best_bid\": ${bid}, \"best_ask\": ${ask}}" | \
    docker exec -i cloacina-kafka \
    /opt/kafka/bin/kafka-console-producer.sh \
    --bootstrap-server localhost:9092 \
    --topic price.orderbook
  sleep 0.5
done
```

---

## Pattern 2: Stateful accumulator — fire only on significant spread change

The passthrough pattern fires on every message. For a stateful pattern, you track previous state *inside the graph node* rather than in the accumulator. Use `reaction_mode = "when_all"` combined with a sliding-window input to reduce noise.

For this you change `package.toml`:

```toml
[metadata]
reaction_mode = "when_all"
input_strategy = "latest"
```

And add a second accumulator that receives a reference price — the reactor fires only when *both* the orderbook and the reference price have been updated at least once:

```toml
[[metadata.accumulators]]
name = "orderbook"
accumulator_type = "stream"

[metadata.accumulators.config]
broker = "KAFKA_BROKER"
topic = "price.orderbook"
group = "kafka-stateful-group"

[[metadata.accumulators]]
name = "reference"
accumulator_type = "passthrough"
```

The `reference` accumulator is `passthrough` — a separate process pushes reference prices via WebSocket while Kafka delivers the live order book. The graph fires only when both have values.

Update `src/lib.rs` to accept both accumulators:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferencePrice {
    pub price: f64,
}

#[cloacina_macros::computation_graph(
    react = when_all(orderbook, reference),
    graph = {
        evaluate(orderbook, reference) -> alert,
    }
)]
pub mod kafka_stateful_signal {
    use super::*;

    pub async fn evaluate(
        orderbook: Option<&OrderBook>,
        reference: Option<&ReferencePrice>,
    ) -> String {
        let (ob, ref_price) = match (orderbook, reference) {
            (Some(o), Some(r)) => (o, r),
            _ => return "missing inputs".to_string(),
        };

        let mid = (ob.best_bid + ob.best_ask) / 2.0;
        let deviation = ((mid - ref_price.price) / ref_price.price).abs();

        if deviation > 0.005 {
            format!("ALERT: mid={:.4} deviates {:.2}% from reference={:.4}",
                mid, deviation * 100.0, ref_price.price)
        } else {
            format!("OK: mid={:.4}", mid)
        }
    }

    pub async fn alert(msg: &String) -> String {
        // In a real system this would post to Slack, PagerDuty, etc.
        tracing::info!(message = %msg, "graph result");
        msg.clone()
    }
}
```

---

## Pattern 3: Batch processing — accumulate N messages before firing

Sometimes you want to batch Kafka messages before processing them — for example, to compute a VWAP over the last 100 trades rather than reacting to each individual tick.

Batching is implemented in the accumulator itself, not in the graph node. The cleanest approach for packaged graphs is to use the `reaction_mode = "when_all"` criterion paired with a custom internal buffer. Because packaged CGs are passthrough at the accumulator level, you implement the batch logic inside the entry node by maintaining state via a `Mutex<Vec<T>>` in a `lazy_static` or `once_cell` global — the graph function is called in the same process as the server and can hold state between fires.

Here is a minimal example that batches 5 messages before firing:

```rust
use std::sync::Mutex;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub price: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VwapResult {
    pub vwap: f64,
    pub sample_count: usize,
}

static BUFFER: Lazy<Mutex<Vec<Trade>>> = Lazy::new(|| Mutex::new(Vec::new()));
const BATCH_SIZE: usize = 5;

#[cloacina_macros::computation_graph(
    react = when_any(trade),
    graph = {
        batch_and_compute(trade) -> emit_vwap,
    }
)]
pub mod kafka_batch_vwap {
    use super::*;

    pub async fn batch_and_compute(trade: Option<&Trade>) -> Option<VwapResult> {
        let Some(t) = trade else { return None; };

        let mut buf = BUFFER.lock().unwrap();
        buf.push(t.clone());

        if buf.len() < BATCH_SIZE {
            // Not enough data yet — return None to suppress terminal node
            return None;
        }

        // Compute VWAP and clear the buffer
        let trades: Vec<Trade> = buf.drain(..).collect();
        drop(buf);

        let total_value: f64 = trades.iter().map(|t| t.price * t.volume).sum();
        let total_volume: f64 = trades.iter().map(|t| t.volume).sum();
        let vwap = if total_volume > 0.0 { total_value / total_volume } else { 0.0 };

        Some(VwapResult {
            vwap,
            sample_count: trades.len(),
        })
    }

    pub async fn emit_vwap(result: &Option<VwapResult>) -> String {
        match result {
            Some(r) => format!("VWAP={:.4} (n={})", r.vwap, r.sample_count),
            None => "buffering".to_string(),
        }
    }
}
```

Add `once_cell` to your `Cargo.toml` dependencies:

```toml
once_cell = "1.19"
```

Create a separate Kafka topic for trades:

```bash
docker exec cloacina-kafka \
  /opt/kafka/bin/kafka-topics.sh \
  --bootstrap-server localhost:9092 \
  --create --topic price.trades \
  --partitions 1 --replication-factor 1 --if-not-exists
```

Update `package.toml` to point at the trades topic:

```toml
[[metadata.accumulators]]
name = "trade"
accumulator_type = "stream"

[metadata.accumulators.config]
broker = "KAFKA_BROKER"
topic = "price.trades"
group = "kafka-vwap-group"
```

Produce 10 trade messages to trigger two batch fires:

```bash
for i in $(seq 1 10); do
  price=$(python3 -c "import random; print(round(100 + random.uniform(-1, 1), 4))")
  volume=$(python3 -c "import random; print(round(random.uniform(10, 500), 2))")
  echo "{\"price\": ${price}, \"volume\": ${volume}}" | \
    docker exec -i cloacina-kafka \
    /opt/kafka/bin/kafka-console-producer.sh \
    --bootstrap-server localhost:9092 \
    --topic price.trades
done
```

After 5 messages the graph fires once, producing a VWAP. After 10 messages it fires a second time.

---

## Consumer group offsets and restarts

The Kafka backend uses `enable.auto.commit = false` and `auto.offset.reset = earliest`. This means:

- **On first start**: the consumer reads from the beginning of the topic
- **On restart**: the consumer resumes from the last committed offset if the group has prior commits; otherwise it re-reads from the earliest offset

Offsets are committed after each message is successfully forwarded to the accumulator channel (the underlying Kafka backend calls `commit()` after `recv()` succeeds). If the server crashes mid-message, the message will be redelivered on restart — the graph is at-least-once.

To reset to the beginning of a topic (useful during development):

```bash
docker exec cloacina-kafka \
  /opt/kafka/bin/kafka-consumer-groups.sh \
  --bootstrap-server localhost:9092 \
  --group kafka-price-signal-group \
  --topic price.orderbook \
  --reset-offsets \
  --to-earliest \
  --execute
```

---

## Troubleshooting

**Accumulator shows `"unhealthy"` and graph never fires**: The Kafka connection failed. Check the server logs for `failed to connect to Kafka` messages. Verify `CLOACINA_VAR_KAFKA_BROKER` is set correctly and that the broker is reachable from the server process. If running the server inside a container, `localhost:9092` may not resolve correctly — use the Docker network hostname instead (e.g., `cloacina-kafka:9092`).

**Messages produce but `fire_count` stays at 0**: The message payload is not valid JSON matching your boundary type. Verify with `kafka-console-consumer.sh`:

```bash
docker exec cloacina-kafka \
  /opt/kafka/bin/kafka-console-consumer.sh \
  --bootstrap-server localhost:9092 \
  --topic price.orderbook \
  --from-beginning \
  --max-messages 5
```

**`stream` accumulator type not supported** error in server logs: The server was built without the `kafka` feature flag. Rebuild with:

```bash
cargo build -p cloacinactl --features kafka
```

**Topic does not exist**: The Kafka backend will log a subscription failure. Create the topic before uploading the package (topics created after the graph loads require a server restart or graph reload to pick up).

---

## Summary

| Pattern | `reaction_mode` | Accumulator type | Fires when |
|---|---|---|---|
| Passthrough | `when_any` | `stream` | Every Kafka message |
| Stateful (multi-source) | `when_all` | mixed `stream` + `passthrough` | All sources have values |
| Batch | `when_any` | `stream` | Every N messages (via internal buffer) |

## Next steps

You've now seen all three deployment modes for computation graphs on the Cloacina server:

1. [Tutorial 07]({{< ref "/computation-graphs/tutorials/service/07-packaging/" >}}) — packaging and uploading a computation graph
2. [Tutorial 08]({{< ref "/computation-graphs/tutorials/service/08-websocket-events/" >}}) — pushing events via WebSocket
3. Tutorial 09 (this page) — Kafka-sourced stream accumulators

For production deployments, see the reference documentation for PAK-scoped authorization on accumulator and reactor endpoints, and the explanation pages covering the `ReactiveScheduler` architecture and offset management.
