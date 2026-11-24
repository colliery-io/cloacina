---
title: "Connector Contract"
description: "The interface every Sync connector implements, and the types that cross it: record batches, checkpoints, sync and write modes, field types."
sidebar:
  order: 48
---

Every Sync connector, compiled or declarative, implements one fidius interface,
`connector`, and exchanges data in a small set of serde types defined in
`cloacina_constructor_contract::sync`. A connector is a
[constructor provider](../../explanation/primitives/constructors/) whose members
are of kind `source` or `destination`.

## The interface

Five methods, in this vtable order:

| Index | Method | Shape | Capability |
|---|---|---|---|
| 0 | `spec() -> ConnectorSpec` | typed | always |
| 1 | `check(Config) -> CheckResult` | typed | always |
| 2 | `discover(Config) -> DiscoverOutcome` | typed, optional | source |
| 3 | `read(ReadContext) -> Stream<ReadMessage>` | server streaming, optional | source |
| 4 | `write(WriteContext, Stream<RecordBatch>) -> WriteOutcome` | client streaming, optional | destination |

`config` is bound once at construction, like every other constructor: the
member's single `#[config]` field is the JSON configuration document
(`Config { json }`). `check` and `discover` take a `Config` explicitly so a
catalog can probe a connector before any instance is configured. A pure source
leaves `write` unimplemented; a pure destination leaves `discover` and `read`
unimplemented. The host never dispatches a method the member's kind does not
cover.

## The descriptor

`spec()` returns the static facts about a connector:

| Field | Meaning |
|---|---|
| `name`, `connector_version` | Identity. |
| `contract_version` | The version of this contract the connector targets. |
| `config_schema` | JSON Schema, as JSON text, of a valid `Config`. |
| `roles` | `Source`, `Destination`, `ReverseEtl`. |
| `supported_sync_modes` | The `SyncMode`s a source can read with. |

The same facts appear in the provider manifest's `connector` field so a catalog
can list connectors without loading a component.

## Record batches

A read or write moves `RecordBatch`es in one of three encodings:

| Variant | Shape | Use |
|---|---|---|
| `Rows(Vec<JsonText>)` | one JSON-object row per entry, as text | the common path |
| `Arrow(ArrowIpc)` | Arrow IPC stream bytes | bulk, native |
| `Changes(Vec<ChangeRecord>)` | rows tagged with an operation | change data capture |

A `ChangeRecord` is `{ op, data }`, with `op` one of `Insert`, `Update`,
`Delete`, and `data` the row as JSON text **including the key columns**, so a
delete carries its own key. Changes flow through mapping like rows, preserving
the operation.

## Reading

`read` takes a `ReadContext { stream, partition, state }` and yields a stream of
`ReadMessage`:

| Variant | Meaning |
|---|---|
| `Records(RecordBatch)` | Records to move. |
| `Checkpoint(StreamState)` | The records before this message may be committed as a unit. Closes a segment. |
| `Log(LogEntry)` | A diagnostic, with a level. |
| `DeadLettered(DeadLetter)` | One record the connector could not produce, with the reason. The read goes on. |
| `Fatal(ConnectorError)` | The read ends; the error's kind decides whether it is retried. |

`StreamState { cursor, opaque }` is the resume point: the cursor as JSON text
and connector-owned bytes. The host commits it; the connector never persists
anything itself. A connector should end a stream with a `Checkpoint`. If it
does not, the trailing records are written with the previous state and will be
read again next pass.

## Writing

`write` takes a `WriteContext { stream }` and a client stream of `RecordBatch`,
which the connector pulls at its own pace, and returns one `WriteOutcome`:

| Field | Meaning |
|---|---|
| `state` | The stream state after the write, usually echoed from the read. |
| `diagnostics` | `LogEntry`s. |
| `dead_letters` | Records the destination rejected, each with a reason. |
| `result` | `Ok(WriteReceipt { accepted })` or `Err(ConnectorError)`. |

## Sync modes

How a source reads a stream (`SyncMode`):

- `FullRefresh`: read everything each pass.
- `Incremental`: read what is new, tracked by a cursor field.
- `Cdc`: stream inserts, updates and deletes from the source's change log.

## Write modes

How a destination applies records (`WriteMode`):

- `Append`: insert every record.
- `Upsert { business_keys }`: insert or update, keyed on the business keys.
  Replaying a segment is a no-op update, which is what makes delivery
  effectively once.
- `Overwrite`: replace the destination's contents.

## Configured streams and mapping

A `ConfiguredStream` is one stream as a sync uses it: `stream`, `sync_mode`,
`cursor_field`, `primary_key`, `write_mode`, `mapping`. The `MappingSpec` is an
ordered, bounded list of `MappingOp`s applied between read and write: `Select`,
`Drop`, `Rename`, `Cast`, `Filter`, `Compute`. There are no joins, aggregates or
user code; richer transformation belongs downstream.

## Field types and schemas

A stream's typed schema (`StreamSchema`) is a list of `Field { name, type,
nullable }`. `FieldType` is one of `str`, `integer`, `float`, `boolean`,
`timestamp`, `json`. A schema is inferred from a sample or declared, persisted
per stream, and enforced on the record path: a missing non-nullable field or a
failed coercion dead-letters the record. Additive changes merge; a type change
is breaking and is surfaced, not applied.

## Errors

A `ConnectorError { kind, message, retryable, context }` crosses the boundary
in `DiscoverOutcome::Error`, `ReadMessage::Fatal` and `WriteResult::Err`. The
kind decides what the host does:

| Kind | Host behaviour |
|---|---|
| `Config` | Surface to the user. No retry. |
| `Transient` | Retry with backoff. |
| `RecordLevel` | Dead-letter the record. Continue. |
| `Fatal` | Fail the run. |

## The wire

Everything crosses the boundary as bincode 1.3 with default options, the
typed wire fidius uses. Two rules follow for every boundary type: free-form
JSON travels as text, never as `serde_json::Value`, and no boundary type
carries field-level serde attributes, because bincode is positional. The
`encode` and `decode` functions in `cloacina_constructor_contract::sync::codec`
are the envelope codec, and a pinned byte-vector test guards the wire.

## Sandbox

A connector runs as a `wasm32-wasip2` component under the consumer's
[capability grants](../../explanation/primitives/constructors/grants/): no
filesystem, no ambient network, egress only where a grant allows. Credentials
come from cloacina's [secrets](../../explanation/service/secrets/), resolved on
the server and delivered to the agent as for any task.
