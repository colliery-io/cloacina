---
title: "06 — Kafka-Sourced Computation Graphs"
description: "Declare stream accumulators that consume from Kafka topics — no external producer needed"
weight: 16
aliases:
  - "/computation-graphs/tutorials/service/09-kafka-stream/"

---

In the previous tutorials, events arrived via WebSocket pushed by an external process. In this tutorial you'll declare a **stream accumulator** in `package.toml`. The server reads events from a Kafka topic automatically — once the graph loads, the accumulator connects to Kafka and pulls messages without any application code changes.

## What you'll learn

- The `[[metadata.accumulators]]` configuration block in `package.toml`
- Setting `CLOACINA_VAR_KAFKA_BROKER` so the server knows where to connect
- Starting Kafka locally with `docker compose`
- Creating topics and producing test messages with `kafka-console-producer.sh`
- Verifying the graph fires after Kafka messages arrive

## Prerequisites

- Tutorial 04 complete (you know how to package and upload a CG)
- Docker and Docker Compose available
- Your Cloacina server built with the `kafka` feature flag enabled
- `curl` and `python3` available

## Time estimate

30–40 minutes

---

## Background

A **stream accumulator** delivers events to the reactor exactly like a WebSocket accumulator, except a background Kafka reader feeds messages into the channel for you. For how accumulators buffer and deliver events, see [Accumulator]({{< ref "/engine/computation-graphs/accumulator" >}}).

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
graph_name = "kafka_price_signal"
language = "rust"
description = "Price signal graph driven by a Kafka topic"

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
cloacina-computation-graph = "0.7.0"
cloacina-macros = "0.7.0"
cloacina-workflow-plugin = "0.7.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
async-trait = "0.1"
tokio = { version = "1.0", features = ["full"] }

[build-dependencies]
cloacina-build = "0.7.0"
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

#[cloacina_macros::reactor(
    name = "kafka_price_signal_reactor",
    accumulators = [orderbook],
    criteria = when_any(orderbook),
)]
pub struct KafkaPriceSignalReactor;

#[cloacina_macros::computation_graph(
    trigger = reactor("kafka_price_signal_reactor"),
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
  result=$(curl -s "${BASE_URL}/v1/health/graphs" \
    -H "Authorization: Bearer ${TOKEN}")
  if echo "$result" | python3 -c "import sys,json; d=json.load(sys.stdin); exit(0 if any(r['name']=='kafka_price_signal' for r in d['items']) else 1)" 2>/dev/null; then
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
curl -s "${BASE_URL}/v1/health/graphs/kafka_price_signal" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -m json.tool
```

Expected:

```json
{
  "name": "kafka_price_signal",
  "health": {
    "state": "live"
  },
  "accumulators": ["orderbook"],
  "paused": false
}
```

The health endpoint reports the reactor's overall state — to confirm firings, scrape `/metrics` and watch `cloacina_reactor_fires_total{graph="kafka_price_signal"}`:

```sh
curl -sf "${BASE_URL}/metrics" -H "Authorization: Bearer ${TOKEN}" \
  | grep '^cloacina_reactor_fires_total.*kafka_price_signal'
```

Produce several more messages and watch the counter increment:

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

## Variations

This tutorial covered the passthrough path, where each Kafka message fires the graph. Other accumulators can batch, deduplicate, or combine multiple sources before firing — pick one based on your event semantics. See [Choosing Accumulator Types]({{< ref "/engine/computation-graphs/how-to/accumulator-types" >}}) for the decision guide, and the [Accumulator]({{< ref "/engine/computation-graphs/accumulator" >}}) reference for the underlying primitive (including consumer-group offset and restart behavior).

---

## Troubleshooting

**Accumulator shows `"unhealthy"` and graph never fires**: The Kafka connection failed. Check the server logs for `failed to connect to Kafka` messages. Verify `CLOACINA_VAR_KAFKA_BROKER` is set correctly and that the broker is reachable from the server process. If running the server inside a container, `localhost:9092` may not resolve correctly — use the Docker network hostname instead (e.g., `cloacina-kafka:9092`).

**Messages produce but `cloacina_reactor_fires_total{graph="kafka_price_signal"}` stays at 0**: The message payload is not valid JSON matching your boundary type. Verify with `kafka-console-consumer.sh`:

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
cargo build -p cloacina-server --features kafka
```

**Topic does not exist**: The Kafka backend will log a subscription failure. Create the topic before uploading the package (topics created after the graph loads require a server restart or graph reload to pick up).

---

## Next steps

Next: [07 — Cross-Package Reactor Binding]({{< ref "/service/tutorials/07-cross-package-binding" >}})
