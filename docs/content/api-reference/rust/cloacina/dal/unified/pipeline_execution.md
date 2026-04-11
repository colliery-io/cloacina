# cloacina::dal::unified::pipeline_execution <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Unified Pipeline Execution DAL with compile-time backend selection

All state transitions are transactional: the status update and execution event
are written atomically. If either fails, both are rolled back.

## Structs

### `cloacina::dal::unified::pipeline_execution::WorkflowExecutionDAL`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Data access layer for workflow execution operations with compile-time backend selection.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |
