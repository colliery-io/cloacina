# Computation-Graph Feature Tour

Three computation-graph surfaces in one package, all exercised through the
primary interface:

| Surface | What it shows |
|---|---|
| **Kafka stream accumulator** | `ticks` is upgraded to a Kafka stream source; each message fires a reactor and runs a graph |
| **Typed inject/fire** | `Tick` derives `JsonSchema`, giving the accumulator a typed `accumulator inject` form |
| **Task→graph invocation** | a workflow task `invokes` a trigger-less graph and consumes its output via a `post_invocation` hook |

## Surfaces

### 1 + 2. Kafka stream accumulator with a typed boundary

```rust
#[reactor(name = "tour_rx", accumulators = [ticks], criteria = when_any(ticks))]
pub struct TourRx;

#[computation_graph(trigger = reactor("tour_rx"), graph = { enrich(ticks) -> emit })]
pub mod tour_stream_graph { ... }
```

`ticks` is declared as a plain accumulator in the macro; `package.toml`
upgrades it to a Kafka stream source:

```toml
[[metadata.accumulators]]
name = "ticks"
accumulator_type = "stream"

[metadata.accumulators.config]
broker = "{{ KAFKA_BROKER }}"   # resolves via CLOACINA_VAR_KAFKA_BROKER
topic  = "tour.ticks"
group  = "cg-feature-tour-group"
```

Because `Tick` derives `schemars::JsonSchema`, the accumulator gets a **typed**
inject/fire form — the server validates injected events against the schema.

### 3. A workflow task that invokes a trigger-less graph

```rust
#[computation_graph(graph = { normalize -> output })]   // no trigger = ... → trigger-less
pub mod tour_math_graph { ... }

#[task(
    dependencies = ["prep"],
    invokes = computation_graph("tour_math_graph"),
    post_invocation = summarize,
)]
pub async fn crunch(ctx: &mut Context<Value>) -> Result<(), TaskError> { ... }
```

The task's body runs first, then the graph, then `summarize` (which sees the
graph's terminal output merged into the context under the node name `output`).
Only trigger-less graphs are invocable — a reactor-triggered graph is rejected
at compile time.

## Run it

Automated as `angreal demos features cg-feature-tour` (the CI examples lane
runs exactly that).

### 1. Stack + CLI

```bash
angreal ui up
cloacinactl config profile set demo http://localhost:8080 \
    --api-key clk_demo_public_key_0003 --tenant public --default
```

The demo stack includes Kafka; the server resolves `{{ KAFKA_BROKER }}` to the
in-cluster broker.

### 2. Pack + upload

```bash
cloacinactl package pack . --out cg-feature-tour.cloacina
cloacinactl package upload cg-feature-tour.cloacina
cloacinactl package list   # wait for build_status: success
```

### 3. Task→graph invocation

```bash
cloacinactl workflow run tour_pipeline
cloacinactl execution list --workflow tour_pipeline
```

Completion proves the invoke bridge: `report` fails unless the
`post_invocation` hook saw the graph's terminal output.

## Operate it

### Inject a typed event (fires the reactor)

```bash
cloacinactl accumulator inject ticks '{"price": 101.5}'
cloacinactl graph accumulators
# watch the reactor's fires:
curl -s -H 'Authorization: Bearer clk_demo_public_key_0003' \
    http://localhost:8080/v1/health/reactors/tour_rx/fires
```

A malformed event (missing `price`, wrong type) is rejected — that's the typed
boundary from `JsonSchema`.

### Feed the stream from Kafka

```bash
docker exec -i cloacina-demo-kafka-1 \
  /opt/kafka/bin/kafka-console-producer.sh \
  --bootstrap-server localhost:9092 --topic tour.ticks <<'EOF'
{"price": 202.0}
EOF
```

Each message on `tour.ticks` fires `tour_rx` and runs `tour_stream_graph`.

## Status of the surfaces

- **Task→graph invocation** — works today; asserted in CI
  (`angreal demos features cg-feature-tour`).
- **Kafka stream accumulator** — the code is here, but kafka is currently a
  HOST cargo feature (rdkafka linked into core `cloacina`), so a stream
  accumulator only runs if the *server* was built `--features kafka` and
  silently degrades to passthrough otherwise. This is being migrated so kafka
  ships **in the package** as a constructor provider — a consumption connector
  belongs in a provider, not the core engine (CLOACI-T-0898). The stream
  sections above light up on that migration.
- **`polling` / `batch` accumulators** — declared surfaces
  (`#[polling_accumulator]`, `#[batch_accumulator]`) that also silently degrade
  to passthrough in packaged graphs today (CLOACI-T-0896).
