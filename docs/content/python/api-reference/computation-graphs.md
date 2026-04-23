---
title: "Computation Graphs"
description: "Python API reference for computation graph builders, nodes, and accumulators"
weight: 55
---

# Computation Graphs

Computation graphs are reactive, directed acyclic graphs (DAGs) of processing nodes. Unlike workflows (which are task-based pipelines driven by a runner), computation graphs react to data arriving at accumulators and propagate results through a fixed topology of nodes.

The Python API mirrors the Rust computation graph system, using decorators and a context-manager builder instead of macros and modules.

## ComputationGraphBuilder

`ComputationGraphBuilder` is a context manager that declares a computation graph's name, reaction criteria, and topology. Inside the `with` block you define node functions with the `@cloaca.node` decorator. When the block exits, the builder validates that every node in the topology has a matching decorated function and vice versa, then registers the graph for execution.

### Constructor

```python
cloaca.ComputationGraphBuilder(name, *, react, graph)
```

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | `str` | Yes | Unique name for the computation graph. Used to identify the graph at execution time. |
| `react` | `dict` | Yes | Reaction criteria dict with keys `mode` and `accumulators`. See [Reaction Criteria](#reaction-criteria). |
| `graph` | `dict` | Yes | Topology dict mapping node names to their configuration. See [Topology Dict](#topology-dict). |

**Returns:** `ComputationGraphBuilder` instance (used as a context manager)

**Example:**
```python
import cloaca

with cloaca.ComputationGraphBuilder(
    "pricing_pipeline",
    react={"mode": "when_any", "accumulators": ["orderbook"]},
    graph={
        "ingest": {"inputs": ["orderbook"], "next": "compute_spread"},
        "compute_spread": {"next": "format_output"},
        "format_output": {},
    },
) as builder:
    # Define @cloaca.node functions here ...
    pass
```

### Reaction Criteria

The `react` parameter controls when the graph fires.

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `mode` | `str` | Yes | `"when_any"` -- fire when any named accumulator delivers new data. `"when_all"` -- fire only when every named accumulator has delivered at least one value. |
| `accumulators` | `list[str]` | Yes | List of accumulator names that feed this graph. Each name must match a registered accumulator function. |

**Example:**
```python
# Fire whenever either source delivers data
react={"mode": "when_any", "accumulators": ["orderbook", "pricing"]}

# Fire only after both sources have delivered at least once
react={"mode": "when_all", "accumulators": ["orderbook", "pricing"]}
```

### Topology Dict

The `graph` parameter is a dict mapping node names (strings) to node configuration dicts. Each key becomes a node in the DAG; the value dict controls how data flows to and from that node.

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `inputs` | `list[str]` | No | List of accumulator/source names this node reads from the input cache. Only used on entry nodes. |
| `next` | `str` | No | Name of the downstream node for linear (unconditional) edges. Mutually exclusive with `routes`. |
| `routes` | `dict[str, str]` | No | Maps variant name strings to downstream node names for conditional routing. Mutually exclusive with `next`. |

A node with neither `next` nor `routes` is a **terminal node** -- its return value becomes the graph's output.

**Linear topology example:**
```python
graph={
    "ingest":         {"inputs": ["orderbook"], "next": "compute_spread"},
    "compute_spread": {"next": "format_output"},
    "format_output":  {},  # terminal
}
```

**Routing topology example:**
```python
graph={
    "decision": {
        "inputs": ["orderbook", "pricing"],
        "routes": {
            "Trade": "signal_handler",
            "NoAction": "audit_logger",
        },
    },
    "signal_handler": {},  # terminal on Trade branch
    "audit_logger":   {},  # terminal on NoAction branch
}
```

### Methods

#### `__enter__()` / `__exit__()`

Context manager protocol. `__enter__` establishes the active graph context so `@cloaca.node` decorators can register functions. `__exit__` validates the topology against registered nodes and builds the internal executor.

**Raises:**
- `AttributeError` -- if a node name in the topology has no matching `@cloaca.node` function
- `ValueError` -- if a `@cloaca.node` function does not appear in the topology

#### `execute(inputs)`

Execute the computation graph synchronously with the given input data.

```python
builder.execute(inputs)
```

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `inputs` | `dict[str, dict]` | Yes | Maps source/accumulator names to their data dicts. Each key must match a name from the `react` accumulators list. |

**Returns:** `dict` -- the terminal node's return value

**Raises:**
- `ValueError` -- if the graph has not been built yet (called before the `with` block exits) or if execution fails

**Example:**
```python
with cloaca.ComputationGraphBuilder("pricing_pipeline", ...) as builder:
    # ... define nodes ...
    pass

# Execute after the with block
result = builder.execute({"orderbook": {"best_bid": 100.50, "best_ask": 100.55}})
print(result)  # {"message": "Mid: 100.52, Spread: 4.9 bps", ...}
```

#### `__repr__()`

Returns a string representation of the builder.

```python
repr(builder)
# "ComputationGraphBuilder(name='pricing_pipeline', nodes=3)"
```

---

## @cloaca.node

The `@cloaca.node` decorator registers a function as a node in the active `ComputationGraphBuilder` context. It must be used inside a `with ComputationGraphBuilder(...) as builder:` block.

### Signature

```python
@cloaca.node
def node_name(arg1, arg2, ...):
    ...
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `func` | `callable` | The function to register. The function name must exactly match a key in the `graph` topology dict. |

**Returns:** The original function, unchanged (transparent decorator)

**Raises:**
- `ValueError` -- if used outside a `ComputationGraphBuilder` context manager

### Node Types

The role of a node is determined by its position in the topology, not by the decorator itself. There are three types:

#### Entry Nodes

Entry nodes have `"inputs"` in their topology config. They receive one positional argument per source listed in `"inputs"`, drawn from the input cache. Arguments are `None` if the corresponding source has no data yet.

```python
# Topology: "ingest": {"inputs": ["orderbook", "pricing"], "next": "analyze"}
@cloaca.node
def ingest(orderbook, pricing):
    """Each parameter matches a source name from 'inputs'."""
    if orderbook is None:
        return {"spread": 0.0}
    spread = orderbook["best_ask"] - orderbook["best_bid"]
    return {"spread": spread}
```

#### Interior Nodes

Interior nodes have a `"next"` or `"routes"` key and no `"inputs"`. They receive a single positional argument: the dict returned by their upstream node.

**Linear interior node:**
```python
# Topology: "compute_spread": {"next": "format_output"}
@cloaca.node
def compute_spread(input_data):
    """Receives the dict returned by the upstream node."""
    spread_bps = (input_data["spread"] / input_data["mid_price"]) * 10_000
    return {"spread_bps": spread_bps, "mid_price": input_data["mid_price"]}
```

**Routing interior node (returns a tuple):**
```python
# Topology: "decision": {"inputs": [...], "routes": {"Trade": ..., "NoAction": ...}}
@cloaca.node
def decision(orderbook, pricing):
    """Routing nodes must return a (variant_name, payload_dict) tuple."""
    if orderbook is None:
        return ("NoAction", {"reason": "no data"})
    return ("Trade", {"direction": "BUY", "price": 100.0})
```

#### Terminal Nodes

Terminal nodes have neither `"next"` nor `"routes"` in their topology config. They receive the upstream node's output and return a dict that becomes the graph's final result.

```python
# Topology: "format_output": {}
@cloaca.node
def format_output(input_data):
    """Return value becomes the result of builder.execute()."""
    return {"message": f"Spread: {input_data['spread_bps']:.1f} bps"}
```

### Return Value Requirements

| Node Type | Required Return Type | Description |
|-----------|---------------------|-------------|
| Entry node | `dict` | Passed to the downstream node as its argument |
| Linear interior node | `dict` | Passed to the downstream node as its argument |
| Routing node | `tuple[str, dict]` | First element is the variant name (must match a key in `"routes"`), second element is the payload dict sent to the selected branch |
| Terminal node | `dict` | Becomes the return value of `builder.execute()` |

### Validation

When the `with` block exits, the builder performs two-way validation:

1. Every node name in the `graph` topology must have a corresponding `@cloaca.node`-decorated function with the same name.
2. Every `@cloaca.node`-decorated function must appear as a key in the `graph` topology.

Mismatches raise `AttributeError` or `ValueError`.

---

## Accumulator Decorators

Accumulators sit between raw data sources and the computation graph. They receive events, optionally transform or buffer them, and emit the processed values that entry nodes consume. In a reactive deployment the runtime manages accumulators automatically; in tutorials and tests you can call the decorated function directly.

The function name of each accumulator becomes its **source name**, which must match entries in the `react` accumulators list and the `inputs` lists in the topology.

### @cloaca.passthrough_accumulator

Registers a function as a passthrough accumulator. Each event is transformed one-to-one with no buffering.

```python
@cloaca.passthrough_accumulator
def source_name(event):
    ...
```

**Parameters:** None (bare decorator)

**Function Signature:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `event` | `dict` | The raw incoming event |

**Returns:** `dict` to pass to the graph, or `None` to suppress the event

**Example:**
```python
@cloaca.passthrough_accumulator
def pricing(event):
    """Transform raw pricing event into graph-ready format."""
    return {"price": event["mid_price"], "change_pct": 0.0}

# Manual invocation (tutorials/testing)
processed = pricing({"mid_price": 101.25})
result = builder.execute({"pricing": processed})
```

### @cloaca.stream_accumulator

Registers a function as a stream-backed accumulator. In a packaged deployment, this accumulator subscribes to a streaming backend (e.g., Kafka, Redpanda) and delivers messages to the graph.

```python
@cloaca.stream_accumulator(type=..., topic=..., group=...)
def source_name(event):
    ...
```

**Parameters (keyword-only):**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `type` | `str` | Yes | Stream backend type (e.g., `"kafka"`, `"redpanda"`) |
| `topic` | `str` | Yes | Topic name to subscribe to |
| `group` | `str` | No | Consumer group name. Defaults to the graph name if omitted. |

**Function Signature:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `event` | `dict` | A deserialized message from the stream |

**Returns:** `dict` to pass to the graph, or `None` to suppress the message

**Example:**
```python
@cloaca.stream_accumulator(type="kafka", topic="market.orderbook", group="mm-group")
def orderbook(event):
    """Receive order book updates from Kafka."""
    return {
        "best_bid": event["bid"],
        "best_ask": event["ask"],
        "timestamp": event["ts"],
    }
```

### @cloaca.polling_accumulator

Registers a function as a polling accumulator. The runtime calls the function at a fixed interval to produce data for the graph.

```python
@cloaca.polling_accumulator(interval=...)
def source_name():
    ...
```

**Parameters (keyword-only):**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `interval` | `str` | Yes | Polling interval as a duration string (e.g., `"5s"`, `"100ms"`, `"1m"`) |

**Function Signature:** The decorated function takes no arguments and returns a dict.

**Returns:** `dict` to deliver to the graph

**Example:**
```python
@cloaca.polling_accumulator(interval="5s")
def system_metrics():
    """Poll system metrics every 5 seconds."""
    return {
        "cpu_usage": get_cpu_percent(),
        "memory_mb": get_memory_usage(),
    }
```

### @cloaca.batch_accumulator

Registers a function as a batch accumulator. Events are buffered and flushed to the graph either when the buffer reaches `max_buffer_size` or after `flush_interval` elapses, whichever comes first.

```python
@cloaca.batch_accumulator(flush_interval=..., max_buffer_size=...)
def source_name(events):
    ...
```

**Parameters (keyword-only):**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `flush_interval` | `str` | Yes | Maximum time to buffer before flushing (e.g., `"10s"`, `"500ms"`) |
| `max_buffer_size` | `int` | No | Maximum number of events to buffer before flushing. If omitted, flushing is driven solely by the interval. |

**Function Signature:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `events` | `list[dict]` | The batch of buffered events |

**Returns:** `dict` to deliver to the graph (typically an aggregation of the batch)

**Example:**
```python
@cloaca.batch_accumulator(flush_interval="10s", max_buffer_size=100)
def trade_events(events):
    """Aggregate trade events into a summary every 10s or 100 events."""
    total_volume = sum(e["volume"] for e in events)
    avg_price = sum(e["price"] for e in events) / len(events)
    return {
        "total_volume": total_volume,
        "avg_price": avg_price,
        "trade_count": len(events),
    }
```

### Accumulator Comparison

| Decorator | Buffering | Trigger | Use Case |
|-----------|-----------|---------|----------|
| `@passthrough_accumulator` | None | Each event | Low-latency, one-to-one transforms |
| `@stream_accumulator` | None | Each message from stream backend | Kafka/Redpanda subscriptions |
| `@polling_accumulator` | None | Fixed interval | Periodic data fetch (APIs, sensors) |
| `@batch_accumulator` | Yes | Interval or buffer size | Aggregation, high-throughput reduction |

---

## Graph Execution

### Synchronous Execution (Tutorials and Testing)

After the `with` block exits, call `builder.execute()` to run the graph synchronously. This is the primary API for tutorials and testing.

```python
with cloaca.ComputationGraphBuilder("my_graph", ...) as builder:
    # ... define nodes ...
    pass

result = builder.execute({"source_name": {"key": "value"}})
```

The `inputs` dict maps source names to data dicts. Each key should match an accumulator name from the `react` configuration. The return value is the terminal node's output dict.

### Reactive Execution (Packaged Deployment)

In a packaged deployment, computation graphs run inside the graph scheduler. The runtime:

1. Starts each registered accumulator (subscribing to streams, setting up polling timers, etc.)
2. Feeds accumulator output into the input cache
3. Evaluates the reaction criteria (`when_any` or `when_all`) on each cache update
4. Executes the graph when criteria are met
5. Delivers terminal node output to downstream consumers

No manual `execute()` call is needed -- the runtime drives the graph automatically. See the [Packaging guide]({{< ref "/computation-graphs/tutorials/service/07-packaging/" >}}) for details on deploying computation graphs.

---

## Complete Examples

### Linear Pipeline

A three-node pipeline that ingests order book data, computes spread in basis points, and formats the output.

```python
import cloaca

# 1. Define accumulator
@cloaca.passthrough_accumulator
def orderbook(event):
    return {"best_bid": event["bid"], "best_ask": event["ask"]}

# 2. Declare graph topology and define nodes
with cloaca.ComputationGraphBuilder(
    "pricing_pipeline",
    react={"mode": "when_any", "accumulators": ["orderbook"]},
    graph={
        "ingest":         {"inputs": ["orderbook"], "next": "compute_spread"},
        "compute_spread": {"next": "format_output"},
        "format_output":  {},
    },
) as builder:

    @cloaca.node
    def ingest(orderbook):
        if orderbook is None:
            return {"spread": 0.0, "mid_price": 0.0}
        spread = orderbook["best_ask"] - orderbook["best_bid"]
        mid_price = (orderbook["best_ask"] + orderbook["best_bid"]) / 2.0
        return {"spread": spread, "mid_price": mid_price}

    @cloaca.node
    def compute_spread(input_data):
        mid = input_data["mid_price"]
        if mid == 0:
            return input_data
        spread_bps = (input_data["spread"] / mid) * 10_000
        return {"spread_bps": spread_bps, "mid_price": mid}

    @cloaca.node
    def format_output(input_data):
        return {
            "message": f"Mid: {input_data['mid_price']:.2f}, "
                       f"Spread: {input_data['spread_bps']:.1f} bps",
        }

# 3. Execute
raw_event = {"bid": 100.50, "ask": 100.55}
processed = orderbook(raw_event)
result = builder.execute({"orderbook": processed})
print(result["message"])
# Mid: 100.52, Spread: 5.0 bps
```

### Routing Graph with Multiple Sources

A market-making graph that takes two inputs, makes a trade/no-trade decision, and routes to different handlers.

```python
import cloaca

@cloaca.passthrough_accumulator
def orderbook(event):
    return event

@cloaca.passthrough_accumulator
def pricing(event):
    return event

with cloaca.ComputationGraphBuilder(
    "market_maker",
    react={"mode": "when_any", "accumulators": ["orderbook", "pricing"]},
    graph={
        "decision": {
            "inputs": ["orderbook", "pricing"],
            "routes": {
                "Trade": "signal_handler",
                "NoAction": "audit_logger",
            },
        },
        "signal_handler": {},
        "audit_logger": {},
    },
) as builder:

    @cloaca.node
    def decision(orderbook, pricing):
        if orderbook is None:
            return ("NoAction", {"reason": "no order book data"})
        spread = orderbook["best_ask"] - orderbook["best_bid"]
        if spread < 0.20:
            return ("Trade", {"direction": "BUY", "price": orderbook["best_bid"]})
        return ("NoAction", {"reason": f"spread too wide: {spread:.2f}"})

    @cloaca.node
    def signal_handler(signal):
        return {"executed": True, "message": f"{signal['direction']} @ {signal['price']:.2f}"}

    @cloaca.node
    def audit_logger(reason):
        return {"logged": True, "reason": reason["reason"]}

# Tight spread -> Trade
result = builder.execute({
    "orderbook": {"best_bid": 100.00, "best_ask": 100.10},
    "pricing": {"mid_price": 100.05},
})
print(result)  # {"executed": True, "message": "BUY @ 100.00"}

# Wide spread -> NoAction
result = builder.execute({
    "orderbook": {"best_bid": 99.50, "best_ask": 100.50},
    "pricing": {"mid_price": 100.00},
})
print(result)  # {"logged": True, "reason": "spread too wide: 1.00"}
```

---

## Error Handling

```python
import cloaca

# Error: @cloaca.node outside a builder context
try:
    @cloaca.node
    def orphan_node(data):
        return data
except ValueError as e:
    print(f"Context error: {e}")
    # "@cloaca.node must be used inside a ComputationGraphBuilder context manager"

# Error: topology references a node with no matching function
try:
    with cloaca.ComputationGraphBuilder(
        "broken_graph",
        react={"mode": "when_any", "accumulators": ["src"]},
        graph={"missing_node": {"inputs": ["src"]}},
    ) as builder:
        pass  # no @cloaca.node defined
except AttributeError as e:
    print(f"Missing node: {e}")
    # "topology references node 'missing_node' but no @cloaca.node function ..."

# Error: decorated function not in topology
try:
    with cloaca.ComputationGraphBuilder(
        "extra_graph",
        react={"mode": "when_any", "accumulators": ["src"]},
        graph={"ingest": {"inputs": ["src"]}},
    ) as builder:
        @cloaca.node
        def ingest(src):
            return src

        @cloaca.node
        def extra_function(data):
            return data
except ValueError as e:
    print(f"Extra node: {e}")
    # "function 'extra_function' was decorated with @cloaca.node but does not appear ..."

# Error: executing before the with block exits
try:
    result = builder.execute({"src": {"key": "value"}})
except ValueError as e:
    print(f"Not built: {e}")
    # "graph '...' not built yet — call execute after the 'with' block exits"

# Error: routing node returns wrong type
# Routing nodes must return a (str, dict) tuple.
# Returning a plain dict raises an error at execution time.
```

---

## Related

- **[Computation Graph Tutorials]({{< ref "/python/tutorials/computation-graphs/" >}})** -- step-by-step tutorials covering linear graphs, accumulators, and routing
- **[WorkflowBuilder]({{< ref "/python/api-reference/workflow-builder/" >}})** -- the task-based workflow builder (different execution model)
- **[DefaultRunner]({{< ref "/python/api-reference/runner/" >}})** -- runs task-based workflows (not computation graphs)
