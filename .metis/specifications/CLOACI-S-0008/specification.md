---
id: horizontal-scaling-for-reactive
level: specification
title: "Horizontal Scaling for Reactive Computation Graphs"
short_code: "CLOACI-S-0008"
created_at: 2026-04-16T12:23:48.379788+00:00
updated_at: 2026-04-16T12:23:48.379788+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Horizontal Scaling for Reactive Computation Graphs

## Overview

Enable multiple Cloacina server instances to share the execution of reactive computation graphs. Today the reactive scheduler is single-instance: every instance loads every graph, the EndpointRegistry is in-memory, and there is no coordination between instances. This specification defines the claiming, heartbeat, and rebalancing protocols that allow graphs to be distributed across a fleet of instances with automatic failover and active load balancing.

The workflow scheduler already supports horizontal scaling via database-level task claiming (`FOR UPDATE SKIP LOCKED`, heartbeats, stale claim sweeper). This specification extends the same pattern to long-lived computation graphs.

## System Context

### Actors
- **Scheduler Instance**: A running `cloacinactl serve` process. Multiple instances share a Postgres database. Each instance runs a reconciler, a reactive scheduler, and WebSocket endpoints.
- **Operator**: Uploads packages, connects via WebSocket for manual operations (force fire, pause, get state). Tolerates reconnection on graph migration.
- **Kafka**: Primary data source for stream-backed accumulators. Consumer group membership follows graph ownership — when a graph moves between instances, the consumer leaves/joins the group automatically.

### External Systems
- **PostgreSQL**: Coordination backend. Holds instance registry, graph assignments, and checkpoints. Required for horizontal scaling (SQLite is single-process).
- **Kafka**: Stream backend. Consumer groups provide natural partition assignment across instances.
- **Load Balancer**: Routes WebSocket connections. No sticky sessions required — clients reconnect on graph migration.

### Boundaries
- **In scope**: Graph claiming, instance registry, heartbeats, rebalancing, failover, reconciler changes.
- **Out of scope**: Sharding the workflow task scheduler (already works). Shared reactor cache across instances (not needed — cold start is correct). Active WebSocket migration (clients reconnect).

## Requirements

### Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-01 | Each computation graph must be owned by exactly one instance at any time | Prevents duplicate execution, duplicate boundaries to reactor, and split-brain state |
| REQ-02 | Instances must register themselves in a `scheduler_instances` table with heartbeats | Enables liveness detection and instance counting for rebalancing |
| REQ-03 | Graphs must be claimed via atomic DB operations (`FOR UPDATE SKIP LOCKED`) | Prevents two instances from claiming the same graph simultaneously |
| REQ-04 | A stale instance sweeper must free graph claims when an instance's heartbeat expires | Ensures graphs are not permanently orphaned when an instance dies |
| REQ-05 | On every reconcile pass, instances must claim unclaimed graphs up to their target load | Prevents orphaned graphs — all graphs must always be running |
| REQ-06 | Active rebalancing: when `my_count > ceil(total_graphs / num_instances)`, release excess graphs | Ensures even distribution on scale-out without waiting for instance death |
| REQ-07 | Released graphs must be gracefully shut down (best effort) before unclaiming | Allows clean Kafka consumer leave and checkpoint persistence when possible |
| REQ-08 | Cold start must produce correct state — checkpoint is a performance optimization, not required | If an instance dies ungracefully, the new owner starts fresh and rebuilds from live data |
| REQ-09 | A short delay must exist between release and re-claim to prevent thrashing | Avoids two instances fighting over the same graph in the same reconcile cycle |
| REQ-10 | Kafka consumer group membership must follow graph ownership | When a graph moves, the old consumer leaves and the new consumer joins, resuming from committed offsets |

### Non-Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| NFR-01 | Instance heartbeat cadence: ~10s. Stale threshold: ~30s. Failover within 1 minute. | Long-lived graphs tolerate brief gaps; fast failover limits data loss |
| NFR-02 | Graph heartbeat cadence: ~5s. Stale threshold: ~15s. | Higher cadence than instance heartbeat for per-graph liveness |
| NFR-03 | Rebalancing must converge within 2 reconcile cycles after a scale event | Prevents prolonged load imbalance |
| NFR-04 | PostgreSQL is required for horizontal scaling | `FOR UPDATE SKIP LOCKED` is essential for safe concurrent claiming |

## Architecture

### Instance Registry

New table `scheduler_instances`:

```sql
CREATE TABLE scheduler_instances (
    instance_id UUID PRIMARY KEY,
    started_at  TIMESTAMP NOT NULL DEFAULT NOW(),
    heartbeat_at TIMESTAMP NOT NULL DEFAULT NOW(),
    graph_count INTEGER NOT NULL DEFAULT 0
);
```

Each instance:
- Inserts a row on startup with a generated UUID
- Updates `heartbeat_at` every ~10s
- Updates `graph_count` on claim/release
- Deletes its row on graceful shutdown

### Graph Assignment

New table `graph_assignments`:

```sql
CREATE TABLE graph_assignments (
    graph_name  TEXT PRIMARY KEY,
    instance_id UUID REFERENCES scheduler_instances(instance_id),
    claimed_at  TIMESTAMP NOT NULL DEFAULT NOW(),
    heartbeat_at TIMESTAMP NOT NULL DEFAULT NOW()
);
```

- Row created when a graph package is uploaded (or on first reconcile detection)
- `instance_id` is NULL when unclaimed
- Claimed via `UPDATE ... WHERE instance_id IS NULL` with `FOR UPDATE SKIP LOCKED`
- Per-graph heartbeat updated by the owning instance

### Reconcile Loop (Modified)

On each reconcile pass:

```
1. Heartbeat: UPDATE scheduler_instances SET heartbeat_at = NOW() WHERE instance_id = $me
2. Sweep: DELETE FROM scheduler_instances WHERE heartbeat_at < NOW() - stale_threshold
          UPDATE graph_assignments SET instance_id = NULL WHERE instance_id NOT IN (SELECT instance_id FROM scheduler_instances)
3. Count: active_instances = COUNT(*) FROM scheduler_instances
          total_graphs = COUNT(*) FROM graph_assignments
          my_count = COUNT(*) FROM graph_assignments WHERE instance_id = $me
          target = CEIL(total_graphs / active_instances)
4. Release (if my_count > target):
          Pick (my_count - target) graphs to release (prefer newest claims)
          For each: graceful shutdown (best effort), then UPDATE graph_assignments SET instance_id = NULL
5. Claim (if unclaimed graphs exist AND my_count < target):
          Claim up to (target - my_count) unclaimed graphs
          For each: UPDATE ... WHERE instance_id IS NULL (FOR UPDATE SKIP LOCKED)
          Load graph into local ReactiveScheduler
```

### Failover Sequence

```
Instance A dies (ungraceful)
  -> Instance A stops heartbeating scheduler_instances
  -> Next sweep by any instance detects stale heartbeat
  -> Deletes Instance A from scheduler_instances
  -> Sets instance_id = NULL on all of Instance A's graph_assignments
  -> Next reconcile pass on surviving instances sees unclaimed graphs
  -> Surviving instances claim and load them (cold start)
  -> Kafka consumers rejoin groups, resume from committed offsets
  -> Reactor caches rebuild from live accumulator data
```

### WebSocket Routing

- WebSocket connections are served by whichever instance the client connects to
- If the graph is not on this instance, the connection is rejected (existing 4404 behavior)
- On graph migration, existing WebSocket connections break
- Client reconnects; load balancer routes to a (possibly different) instance
- The new instance may or may not own the graph — client retries until it hits the right one
- Acceptable because WebSocket is for operational needs, not primary data flow

### What Stays the Same

- Upload, compile, persist pipeline — unchanged
- Package reconciler detection (DB diff) — unchanged, just adds claiming step
- Accumulator/reactor runtime — unchanged
- Checkpoint persistence — unchanged (optional warm start optimization)
- Kafka StreamBackend — unchanged (consumer group handles partition assignment)
- Auth policies — unchanged (set on load_graph as today)

## Constraints

### Technical Constraints
- PostgreSQL required — SQLite lacks `FOR UPDATE SKIP LOCKED`
- All instances must have access to the same package store (DB-backed, already true)
- Clock skew between instances must be small relative to heartbeat thresholds

### Design Decisions
- Passive WebSocket routing (reconnect, no migration) — WebSocket is operational, not primary data path
- Checkpoint is optimization, not correctness — cold start rebuilds from live data
- Active rebalancing over passive — scale-out must distribute load without waiting for death
