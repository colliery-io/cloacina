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

These are the verbs you use against a **running** graph — feeding it, forcing
it, and looking at what it is doing.

### Look at what is loaded

```bash
cloacinactl graph list                  # graphs the runtime has loaded
cloacinactl graph status tour_stream_graph
cloacinactl graph accumulators          # every accumulator + its current depth
```

`graph accumulators` is the quickest answer to "is my data arriving at all?" —
if the depth never moves, the problem is upstream of the graph.

### Inject a typed event (fires the reactor)

```bash
cloacinactl accumulator inject ticks --event '{"price": 101.5}'
cloacinactl graph accumulators
```

The payload goes in `--event`. It is parsed as JSON, or taken as a JSON string
if it does not parse — so `--event hello` works unquoted.

A malformed event (missing `price`, wrong type) is rejected — that's the typed
boundary from `JsonSchema` doing its job, and it is worth trying once so you
recognise the error later.

Watch the reactor's fires:

```bash
curl -s -H 'Authorization: Bearer clk_demo_public_key_0003' \
    http://localhost:8080/v1/health/reactors/tour_rx/fires
```

### Fire the reactor directly

Injecting adds to what the accumulator already holds. Sometimes you want to run
the graph against an exact set of inputs instead — a full replace of the
reactor's cache, then fire:

```bash
cloacinactl reactor fire tour_rx --input ticks='{"price": 250.0}'
```

`--input` is `source=<json>` and repeats once per accumulator the reactor
consumes. This is full-replace only: sources you omit are cleared, not kept.

To fire on whatever is already cached, without supplying inputs:

```bash
cloacinactl reactor force-fire tour_rx
```

Use `fire` when you are testing the graph's logic against known inputs, and
`force-fire` when you want to shake loose a reactor whose criteria have not
tripped yet.

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

Everything in this README works on the primary interface today, and both
surfaces this example teaches are asserted in CI by
`angreal demos features cg-feature-tour`:

- **Task→graph invocation** — runs `tour_pipeline` to completion and checks the
  terminal outputs merged back into the invoking task's context.
- **Kafka stream accumulator** — the lane produces real messages onto the
  dev-stack broker and observes `tour_rx` fire. Kafka now ships **in the
  package** as a bundled native constructor provider, so it no longer depends
  on the server being built with a `kafka` cargo feature (CLOACI-T-0898,
  proven by CLOACI-T-0907).

The other accumulator kinds have their own examples rather than being crammed
in here — packaged graphs dispatch every kind explicitly since CLOACI-T-0896,
which fixed them silently degrading to passthrough:

| Kind | Example |
|---|---|
| `polling` | [`python-polling-graph`](../python-polling-graph) |
| `batch` | [`python-batch-graph`](../python-batch-graph) |
| `state` | [`python-stateful-graph`](../python-stateful-graph) |

Python's typed boundaries (`@cloaca.boundary_schema(...)`) are taught in
[`python-packaged-graph`](../python-packaged-graph) and
[`python-stateful-graph`](../python-stateful-graph).
