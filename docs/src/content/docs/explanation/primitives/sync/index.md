---
title: "Sync"
description: "Data movement as a feature: sources, destinations, connectors, and the segment-by-segment sync that moves records between them."
sidebar:
  order: 50
---

**Sync** moves records from a **source** to a **destination** through sandboxed
**connectors**, in segments, with a cursor that advances only after each segment
is durably written. It is cloacina's third feature beside Workflows and
Computation graphs, and it is built from them: a sync is a workflow instance,
each segment is a workflow execution, and connectors are constructor providers.

Sync came to cloacina from [weir](https://github.com/colliery-io/weir), whose
feature set it absorbs (COLLIERY-S-0002, CLOACINA-A-0013).

## The words

| Word | Meaning |
|---|---|
| **Sync** | The feature, and the capability of a server or runner to run syncs. |
| **A sync** | One configured movement: a source, a destination, the streams to move, a mapping, a write mode, a schedule. A sync is an instance of the built-in Sync workflow. |
| **Connector** | A provider whose members are sources and destinations, under the connector contract. Connectors are WASM components and run sandboxed. |
| **Source** | A connector member that reads: `discover` the streams it offers, then `read` records and checkpoints from a resume point. |
| **Destination** | A connector member that writes: `write` a stream of record batches and report the outcome. |
| **Stream** | A table, endpoint or object within a source: the unit of configuration, cursor and schema. |
| **Cursor** | The committed position in a stream, as JSON text, plus the connector's opaque resume bytes. One per sync, stream and partition. |
| **Checkpoint** | The connector's signal that the records before it can be committed as a unit. A checkpoint closes a segment. |
| **Segment** | The records between two checkpoints. One segment is one workflow execution. |
| **Execution** | Cloacina's word for one run of a workflow; a sync's pass is many executions, one per segment, in order. |
| **Dead-letter** | A record a sync could not process, kept with the reason so the run continues and an operator can see what was skipped. |
| **Declarative connector** | A connector defined by a YAML **connector definition** and run on a shared REST runtime, with nothing compiled. |
| **Sync definition** | What an operator writes to create a sync: the parameters of the workflow instance. |
| **Catalog** | The connectors a server has loaded. |
| **Continuous source** | A source that stays open, tailing a change log or polling at a cadence, as against a scheduled one. |

## How a sync runs

Every sync runs on an [execution agent](../../service/execution-agent-fleet/),
never in the server process. An agent only gets tasks in the tenant its API
key is scoped to, so the agents for a tenant's syncs need a key minted for that
tenant (`POST /v1/tenants/<tenant>/keys`). The server's bootstrap admin key
is in no tenant: an agent started with it gets no tasks, and syncs wait.

One pass of a sync is a series of segments:

1. The **dispatcher** on the server reads the committed cursor of the stream
   and creates an execution of the Sync workflow with the sync's parameters
   and that cursor.
2. On the agent, the **read** task opens the source with the cursor and reads
   until the connector emits a checkpoint, or until the segment budget is
   reached at a checkpoint boundary. The records and the new cursor go into the
   execution's context.
3. The **map and enforce** task applies the mapping and the stream's typed
   schema. A record that cannot be mapped or coerced becomes a dead-letter;
   the segment goes on.
4. The **write** task streams the records into the destination. When the
   destination has accepted them, the task returns the new cursor and the
   dead-letters as **completion writes**: rows the server writes in the same
   transaction that marks the task completed.
5. The dispatcher waits for the execution, reads the committed cursor, and
   creates the next execution. When the source reports its end, the pass is
   done.

```
 server                         agent
 ┌──────────────┐   execution   ┌──────────┐  ┌──────────────┐  ┌───────┐
 │ dispatcher   │ ────────────▶ │ read     │─▶│ map, enforce │─▶│ write │
 │ reads cursor │ ◀──────────── │ segment  │  │ dead-letters │  │       │
 └──────────────┘  completion   └──────────┘  └──────────────┘  └───┬───┘
        ▲          writes: cursor, dead-letters                     │
        └───────────────────────────────────────────────────────────┘
```

Because the dispatcher waits for each execution, a sync's executions are
serial and in order, and a change-data-capture stream applies inserts, updates
and deletes in the order the source produced them.

## The cursor advances after the write

The point of a sync is a copy. A cursor that moved before the destination had
written the records could lose them to a crash. So the cursor is written by
the write task, after the destination has accepted the segment, and inside the
transaction that marks that task completed. If the agent dies mid-write, the
claim expires and the segment is written again. If the server dies between the
agent's report and the commit, the same. Either way the cursor never passes an
unwritten record.

## Delivery guarantees

Delivery is **at-least-once**: a crash replays at most one segment. With the
`Upsert` write mode, keyed on the stream's business keys, replaying a segment is
a no-op update, so the result is **effectively once**. `Append` and `Overwrite`
carry the plain at-least-once guarantee.

Dead-letters never stop a run. A record that fails mapping, schema
enforcement or the destination's write is kept with its reason, visible per
sync, and the segment completes without it.

## Connectors are sandboxed

A connector is a WASM component loaded as a
[constructor provider](../constructors/) with the consumer's capability grants.
Inside the sandbox there is no filesystem and no ambient network; the only
reach outward is the egress the grants allow. Credentials come from cloacina's
tenant-scoped [secrets](../../service/secrets/), resolved for the task on the
agent. A database connector takes its password in its config. A connector that
calls an authenticated API never sees its key: the host adds the credential to
each request the grants allow. See [Credential Injection](credentials/).

The contract every connector implements is in the
[Connector Contract reference](../../../reference/connector-contract/).

## Connectors

Cloacina ships Postgres, MSSQL, S3 and Snowflake connectors. Their config,
modes, grants and credentials are in the
[Sync Connectors reference](../../../reference/connectors/). Each passes a
conformance file through the real connector host; a connector author writes
one the same way ([Test a Connector](../../../how-to/sync/test-a-connector/)).

## What is not yet here

Sync lands in order (COLLIERY-I-0632 to COLLIERY-I-0636). The contract, the
Postgres to Postgres sync, the compiled connectors, credential injection and
the conformance kit are here. The declarative connectors and their corpus,
OAuth2 and Google credentials, authoring in the web UI, and continuous sources
come next.
