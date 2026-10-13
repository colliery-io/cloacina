---
title: "cloacina::dal::unified::checkpoint"
description: "Unified Checkpoint DAL for computation graph state persistence"
editUrl: false
sidebar:
  order: 70
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Unified Checkpoint DAL for computation graph state persistence

Provides save/load operations for accumulator checkpoints, boundaries,
reactor state, and state accumulator buffers. All operations use upsert
semantics keyed by (graph_name, accumulator_name) or (graph_name).

## Structs

### <span id="struct.CheckpointDAL"></span><code style="font-size: 0.85em">struct CheckpointDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Data access layer for computation graph checkpoint operations.

#### Methods

##### <span id="CheckpointDAL.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: &'a DAL) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }
```

</details>

##### <span id="CheckpointDAL.save_checkpoint"></span><code>save_​checkpoint</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn save_checkpoint(
    &self,
    graph_name: &str,
    accumulator_name: &str,
    data: Vec<u8>,
) -> Result<(), ValidationError>
```

Save (upsert) an accumulator checkpoint.

<details>
<summary>Source</summary>

```rust
    pub async fn save_checkpoint(
        &self,
        graph_name: &str,
        accumulator_name: &str,
        data: Vec<u8>,
    ) -> Result<(), ValidationError> {
        let graph_name = graph_name.to_string();
        let accumulator_name = accumulator_name.to_string();
        let now = UniversalTimestamp::now();
        let id = UniversalUuid::new_v4();
        let data = UniversalBinary::from(data);

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(accumulator_checkpoints::table)
                .values(&NewUnifiedAccumulatorCheckpoint {
                    id,
                    graph_name: graph_name.clone(),
                    accumulator_name: accumulator_name.clone(),
                    checkpoint_data: data.clone(),
                    created_at: now,
                    updated_at: now,
                })
                .on_conflict((
                    accumulator_checkpoints::graph_name,
                    accumulator_checkpoints::accumulator_name,
                ))
                .do_update()
                .set((
                    accumulator_checkpoints::checkpoint_data.eq(data),
                    accumulator_checkpoints::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }
```

</details>

##### <span id="CheckpointDAL.load_checkpoint"></span><code>load_​checkpoint</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn load_checkpoint(
    &self,
    graph_name: &str,
    accumulator_name: &str,
) -> Result<Option<Vec<u8>>, ValidationError>
```

Load an accumulator checkpoint.

<details>
<summary>Source</summary>

```rust
    pub async fn load_checkpoint(
        &self,
        graph_name: &str,
        accumulator_name: &str,
    ) -> Result<Option<Vec<u8>>, ValidationError> {
        let graph_name = graph_name.to_string();
        let accumulator_name = accumulator_name.to_string();

        let result: Option<UnifiedAccumulatorCheckpoint> =
            crate::interact_on_backend!(self.dal, |conn| {
                accumulator_checkpoints::table
                    .filter(accumulator_checkpoints::graph_name.eq(&graph_name))
                    .filter(accumulator_checkpoints::accumulator_name.eq(&accumulator_name))
                    .first(conn)
                    .optional()
            })?;

        Ok(result.map(|r| r.checkpoint_data.into_inner()))
    }
```

</details>

##### <span id="CheckpointDAL.save_boundary"></span><code>save_​boundary</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn save_boundary(
    &self,
    graph_name: &str,
    accumulator_name: &str,
    data: Vec<u8>,
    sequence_number: i64,
) -> Result<(), ValidationError>
```

Save (upsert) a boundary with sequence number.

<details>
<summary>Source</summary>

```rust
    pub async fn save_boundary(
        &self,
        graph_name: &str,
        accumulator_name: &str,
        data: Vec<u8>,
        sequence_number: i64,
    ) -> Result<(), ValidationError> {
        let graph_name = graph_name.to_string();
        let accumulator_name = accumulator_name.to_string();
        let now = UniversalTimestamp::now();
        let id = UniversalUuid::new_v4();
        let data = UniversalBinary::from(data);

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(accumulator_boundaries::table)
                .values(&NewUnifiedAccumulatorBoundary {
                    id,
                    graph_name: graph_name.clone(),
                    accumulator_name: accumulator_name.clone(),
                    boundary_data: data.clone(),
                    sequence_number,
                    created_at: now,
                    updated_at: now,
                })
                .on_conflict((
                    accumulator_boundaries::graph_name,
                    accumulator_boundaries::accumulator_name,
                ))
                .do_update()
                .set((
                    accumulator_boundaries::boundary_data.eq(data),
                    accumulator_boundaries::sequence_number.eq(sequence_number),
                    accumulator_boundaries::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }
```

</details>

##### <span id="CheckpointDAL.load_boundary"></span><code>load_​boundary</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn load_boundary(
    &self,
    graph_name: &str,
    accumulator_name: &str,
) -> Result<Option<(Vec<u8>, i64)>, ValidationError>
```

Load a boundary and its sequence number.

<details>
<summary>Source</summary>

```rust
    pub async fn load_boundary(
        &self,
        graph_name: &str,
        accumulator_name: &str,
    ) -> Result<Option<(Vec<u8>, i64)>, ValidationError> {
        let graph_name = graph_name.to_string();
        let accumulator_name = accumulator_name.to_string();

        let result: Option<UnifiedAccumulatorBoundary> =
            crate::interact_on_backend!(self.dal, |conn| {
                accumulator_boundaries::table
                    .filter(accumulator_boundaries::graph_name.eq(&graph_name))
                    .filter(accumulator_boundaries::accumulator_name.eq(&accumulator_name))
                    .first(conn)
                    .optional()
            })?;

        Ok(result.map(|r| (r.boundary_data.into_inner(), r.sequence_number)))
    }
```

</details>

##### <span id="CheckpointDAL.save_reactor_state"></span><code>save_​reactor_​state</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn save_reactor_state(
    &self,
    graph_name: &str,
    cache_data: Vec<u8>,
    dirty_flags: Vec<u8>,
    sequential_queue: Option<Vec<u8>>,
) -> Result<(), ValidationError>
```

Save (upsert) reactor state.

<details>
<summary>Source</summary>

```rust
    pub async fn save_reactor_state(
        &self,
        graph_name: &str,
        cache_data: Vec<u8>,
        dirty_flags: Vec<u8>,
        sequential_queue: Option<Vec<u8>>,
    ) -> Result<(), ValidationError> {
        let graph_name = graph_name.to_string();
        let now = UniversalTimestamp::now();
        let id = UniversalUuid::new_v4();
        let cache_data = UniversalBinary::from(cache_data);
        let dirty_flags = UniversalBinary::from(dirty_flags);
        let sequential_queue = sequential_queue.map(UniversalBinary::from);

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(reactor_state::table)
                .values(&NewUnifiedReactorState {
                    id,
                    graph_name: graph_name.clone(),
                    cache_data: cache_data.clone(),
                    dirty_flags: dirty_flags.clone(),
                    sequential_queue: sequential_queue.clone(),
                    created_at: now,
                    updated_at: now,
                })
                .on_conflict(reactor_state::graph_name)
                .do_update()
                .set((
                    reactor_state::cache_data.eq(cache_data),
                    reactor_state::dirty_flags.eq(dirty_flags),
                    reactor_state::sequential_queue.eq(sequential_queue),
                    reactor_state::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }
```

</details>

##### <span id="CheckpointDAL.load_reactor_state"></span><code>load_​reactor_​state</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn load_reactor_state(
    &self,
    graph_name: &str,
) -> Result<Option<(Vec<u8>, Vec<u8>, Option<Vec<u8>>)>, ValidationError>
```

Load reactor state.

<details>
<summary>Source</summary>

```rust
    pub async fn load_reactor_state(
        &self,
        graph_name: &str,
    ) -> Result<Option<(Vec<u8>, Vec<u8>, Option<Vec<u8>>)>, ValidationError> {
        let graph_name = graph_name.to_string();

        let result: Option<UnifiedReactorState> = crate::interact_on_backend!(self.dal, |conn| {
            reactor_state::table
                .filter(reactor_state::graph_name.eq(&graph_name))
                .first(conn)
                .optional()
        })?;

        Ok(result.map(|r| {
            (
                r.cache_data.into_inner(),
                r.dirty_flags.into_inner(),
                r.sequential_queue.map(|q| q.into_inner()),
            )
        }))
    }
```

</details>

##### <span id="CheckpointDAL.save_state_buffer"></span><code>save_​state_​buffer</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn save_state_buffer(
    &self,
    graph_name: &str,
    accumulator_name: &str,
    data: Vec<u8>,
    capacity: i32,
) -> Result<(), ValidationError>
```

Save (upsert) a state accumulator buffer.

<details>
<summary>Source</summary>

```rust
    pub async fn save_state_buffer(
        &self,
        graph_name: &str,
        accumulator_name: &str,
        data: Vec<u8>,
        capacity: i32,
    ) -> Result<(), ValidationError> {
        let graph_name = graph_name.to_string();
        let accumulator_name = accumulator_name.to_string();
        let now = UniversalTimestamp::now();
        let id = UniversalUuid::new_v4();
        let data = UniversalBinary::from(data);

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(state_accumulator_buffers::table)
                .values(&NewUnifiedStateAccumulatorBuffer {
                    id,
                    graph_name: graph_name.clone(),
                    accumulator_name: accumulator_name.clone(),
                    buffer_data: data.clone(),
                    capacity,
                    created_at: now,
                    updated_at: now,
                })
                .on_conflict((
                    state_accumulator_buffers::graph_name,
                    state_accumulator_buffers::accumulator_name,
                ))
                .do_update()
                .set((
                    state_accumulator_buffers::buffer_data.eq(data),
                    state_accumulator_buffers::capacity.eq(capacity),
                    state_accumulator_buffers::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }
```

</details>

##### <span id="CheckpointDAL.load_state_buffer"></span><code>load_​state_​buffer</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn load_state_buffer(
    &self,
    graph_name: &str,
    accumulator_name: &str,
) -> Result<Option<(Vec<u8>, i32)>, ValidationError>
```

Load a state accumulator buffer.

<details>
<summary>Source</summary>

```rust
    pub async fn load_state_buffer(
        &self,
        graph_name: &str,
        accumulator_name: &str,
    ) -> Result<Option<(Vec<u8>, i32)>, ValidationError> {
        let graph_name = graph_name.to_string();
        let accumulator_name = accumulator_name.to_string();

        let result: Option<UnifiedStateAccumulatorBuffer> =
            crate::interact_on_backend!(self.dal, |conn| {
                state_accumulator_buffers::table
                    .filter(state_accumulator_buffers::graph_name.eq(&graph_name))
                    .filter(state_accumulator_buffers::accumulator_name.eq(&accumulator_name))
                    .first(conn)
                    .optional()
            })?;

        Ok(result.map(|r| (r.buffer_data.into_inner(), r.capacity)))
    }
```

</details>

##### <span id="CheckpointDAL.delete_graph_state"></span><code>delete_​graph_​state</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn delete_graph_state(
    &self) -> Result<(,
    graph_name: &str) -> Result<(,
) -> Result<(), ValidationError>
```

Delete all state for a graph (used on graph unload/removal).

<details>
<summary>Source</summary>

```rust
    pub async fn delete_graph_state(&self, graph_name: &str) -> Result<(), ValidationError> {
        let graph_name = graph_name.to_string();

        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction(|conn| {
                diesel::delete(
                    accumulator_checkpoints::table
                        .filter(accumulator_checkpoints::graph_name.eq(&graph_name)),
                )
                .execute(conn)?;
                diesel::delete(
                    accumulator_boundaries::table
                        .filter(accumulator_boundaries::graph_name.eq(&graph_name)),
                )
                .execute(conn)?;
                diesel::delete(
                    reactor_state::table.filter(reactor_state::graph_name.eq(&graph_name)),
                )
                .execute(conn)?;
                diesel::delete(
                    state_accumulator_buffers::table
                        .filter(state_accumulator_buffers::graph_name.eq(&graph_name)),
                )
                .execute(conn)
            })
        })?;

        Ok(())
    }
```

</details>
