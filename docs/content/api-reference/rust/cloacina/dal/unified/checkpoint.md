# cloacina::dal::unified::checkpoint <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Unified Checkpoint DAL for computation graph state persistence

Provides save/load operations for accumulator checkpoints, boundaries,
reactor state, and state accumulator buffers. All operations use upsert
semantics keyed by (graph_name, accumulator_name) or (graph_name).

## Structs

### `cloacina::dal::unified::checkpoint::CheckpointDAL`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Data access layer for computation graph checkpoint operations.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |
