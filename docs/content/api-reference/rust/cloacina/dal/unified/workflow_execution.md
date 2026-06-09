# cloacina::dal::unified::workflow_execution <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Unified Workflow Execution DAL with compile-time backend selection

All state transitions are transactional: the status update and execution event
are written atomically. If either fails, both are rolled back.

## Structs

### `cloacina::dal::unified::workflow_execution::ExecutionListFilter`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Default`

Filter for `WorkflowExecutionDAL::list_filtered`. CLOACI-T-0594 / API-02: closes the silent-filter-drop bug where the REST route's `--status` / `--workflow_name` query params were discarded.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `status` | `Option < String >` | Filter by status (e.g. `Pending`, `Running`, `Completed`, `Failed`).
`None` means no status filter. |
| `workflow_name` | `Option < String >` | Filter by exact workflow name. `None` means no name filter. |
| `limit` | `i64` | SQL `LIMIT`. Caller is responsible for bounding (route validates
`limit <= 1000`); the DAL trusts whatever is passed. |
| `offset` | `i64` | SQL `OFFSET`. |



### `cloacina::dal::unified::workflow_execution::WorkflowExecutionDAL`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Data access layer for workflow execution operations with compile-time backend selection.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |
