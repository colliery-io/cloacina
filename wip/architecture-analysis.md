# Cloacina Architecture Analysis for Python Bindings

**Analysis Date**: 2025-05-29
**Purpose**: Identify key integration points for PyO3 Python bindings

## Core Abstractions

### 1. Task Trait (`src/task.rs`)
The fundamental execution unit with async interface:

```rust
#[async_trait]
pub trait Task: Send + Sync {
    async fn execute(&self, context: Context<serde_json::Value>) -> Result<Context<serde_json::Value>, TaskError>;
    fn id(&self) -> &str;
    fn dependencies(&self) -> &[String];
    fn retry_policy(&self) -> RetryPolicy;
    fn trigger_rules(&self) -> serde_json::Value;
    fn code_fingerprint(&self) -> Option<String>;
}
```

**Python Integration Requirements**:
- Wrapper struct implementing Task trait for Python functions
- Async method bridging via `pyo3-asyncio`
- Metadata extraction from Python decorators

### 2. Context (`src/context.rs`)
Type-safe data container with serialization:

```rust
pub struct Context<T> where T: Serialize + for<'de> Deserialize<'de> + Debug {
    data: HashMap<String, T>,
    execution_scope: Option<ExecutionScope>,
    dependency_loader: Option<DependencyLoader>,
}
```

**Python Integration Requirements**:
- Dict-like interface for Python developers
- JSON marshalling via `serde_json::Value`
- Method exposure: `insert()`, `get()`, `contains_key()`

### 3. Workflow (`src/workflow.rs`)
Task graph with dependency management:

```rust
pub struct Workflow {
    name: String,
    tasks: HashMap<String, Box<dyn Task>>,
    dependency_graph: DependencyGraph,
    metadata: WorkflowMetadata,
}
```

**Python Integration Requirements**:
- Builder pattern for Python workflow creation
- Task collection and validation
- Dependency graph construction

### 4. UnifiedExecutor (`src/executor/unified_executor.rs`)
Main execution engine:

```rust
#[async_trait]
pub trait PipelineExecutor: Send + Sync {
    async fn execute(&self, workflow_name: &str, context: Context<serde_json::Value>)
        -> Result<PipelineResult, PipelineError>;
    async fn execute_async(&self, workflow_name: &str, context: Context<serde_json::Value>)
        -> Result<PipelineExecution, PipelineError>;
}
```

**Python Integration Requirements**:
- Async method exposure to Python
- Database connection string management
- Result monitoring and callbacks

## Data Types for Marshalling

### Essential Enums
```rust
// Task execution states
pub enum TaskState {
    Pending,
    Running { start_time: DateTime<Utc> },
    Completed { completion_time: DateTime<Utc> },
    Failed { error: String, failure_time: DateTime<Utc> },
    Skipped { reason: String, skip_time: DateTime<Utc> },
}

// Pipeline execution status
pub enum PipelineStatus {
    Pending, Running, Completed, Failed, Cancelled
}
```

### Complex Structures
```rust
// Execution results
pub struct PipelineResult {
    pub execution_id: Uuid,
    pub workflow_name: String,
    pub status: PipelineStatus,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration: Option<Duration>,
    pub final_context: Context<serde_json::Value>,
    pub task_results: Vec<TaskResult>,
    pub error_message: Option<String>,
}

// Retry configuration
pub struct RetryPolicy {
    pub max_attempts: i32,
    pub backoff_strategy: BackoffStrategy,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub jitter: bool,
    pub retry_conditions: Vec<RetryCondition>,
}
```

## Async Execution Patterns

### Task Execution (`src/executor/task_executor.rs`)
- **Concurrency Control**: `tokio::sync::Semaphore`
- **Polling**: Database polling for ready tasks
- **Timeout Protection**: `tokio::time::timeout`
- **Background Spawning**: `tokio::spawn`

### Pipeline Execution (`src/executor/pipeline_executor.rs`)
- **Async Traits**: All methods return Futures
- **Status Monitoring**: Polling-based progress tracking
- **Graceful Shutdown**: Cancellation handling
- **Callbacks**: Status update notifications

**Python Bridge Requirements**:
- `pyo3-asyncio` for runtime integration
- Python coroutine ↔ Rust Future conversion
- Event loop management

## Error Handling System

### Hierarchical Error Types (`src/error.rs`)
```rust
pub enum TaskError {
    ExecutionFailed{...}, Timeout{...}, ContextError{...}, ...
}
pub enum ContextError {
    Serialization(..), KeyNotFound(..), Database(..), ...
}
pub enum ExecutorError {
    Database(..), TaskNotFound(..), TaskTimeout, ...
}
pub enum PipelineError {
    DatabaseConnection{...}, WorkflowNotFound{...}, ...
}
```

**Python Exception Mapping**:
- `TaskError` → `TaskExecutionError`
- `ContextError` → `ContextError`
- `PipelineError` → `PipelineExecutionError`
- `ValidationError` → `ValidationError`

### Error Conversion Patterns
- `From` trait implementations
- Error chaining with `thiserror`
- Context-aware error messages

## Database Integration

### Data Access Layer (`src/dal/`)
- **Multi-backend**: PostgreSQL + SQLite support
- **Connection Pooling**: r2d2 integration
- **Abstract Interface**: Database-agnostic operations

### Models (`src/models/`)
- **Diesel ORM**: Database schema mapping
- **Universal Types**: Cross-database compatibility
- **Auto-timestamping**: Created/updated timestamps

## Macro System (`cloacina-macros/`)

### Current Rust Macros
```rust
// Task definition
#[task(
    id = "my_task",
    dependencies = ["other_task"],
    retry_attempts = 3,
    retry_backoff = "exponential"
)]
async fn my_task(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> { ... }

// Workflow definition
workflow! {
    name: "my_workflow",
    description: "Example workflow",
    tasks: [task1, task2, task3]
}
```

**Python Equivalent Target**:
```python
@task(id="my_task", dependencies=["other_task"], retry_attempts=3)
async def my_task(context):
    # Python implementation
    pass

pipeline = workflow(
    name="my_workflow",
    description="Example workflow",
    tasks=[my_task, other_task]
)
```

## Python Integration Architecture

### Core Python Wrappers Needed

#### 1. PyTask
```python
class PyTask:
    def __init__(self, func, id, dependencies, retry_policy):
        self.func = func
        self.id = id
        self.dependencies = dependencies
        self.retry_policy = retry_policy

    async def execute(self, context):
        # Bridge to Python async function
        pass
```

#### 2. PyContext
```python
class PyContext:
    def insert(self, key: str, value: Any) -> None: ...
    def get(self, key: str) -> Any: ...
    def contains_key(self, key: str) -> bool: ...
    def __getitem__(self, key: str) -> Any: ...
    def __setitem__(self, key: str, value: Any) -> None: ...
```

#### 3. PyWorkflow
```python
class PyWorkflow:
    def __init__(self, name: str, description: str = ""):
        self.name = name
        self.tasks = []

    def add_task(self, task: PyTask) -> None: ...
    def validate_dependencies(self) -> bool: ...
```

#### 4. PyExecutor
```python
class PyExecutor:
    def __init__(self, database_url: str): ...
    async def execute(self, workflow_name: str, context: dict) -> PipelineResult: ...
    async def execute_async(self, workflow_name: str, context: dict) -> PipelineExecution: ...
```

### Data Marshalling Strategy

#### Type Conversion Bridge
- **Python Dict** ↔ `HashMap<String, serde_json::Value>`
- **Python datetime** ↔ `DateTime<Utc>`
- **Python timedelta** ↔ `Duration`
- **Python UUID** ↔ `Uuid`
- **Python exceptions** ↔ Rust error enums

#### JSON Intermediate Format
```
Python Objects → JSON → serde_json::Value → Rust Structs
Rust Structs → serde_json::Value → JSON → Python Objects
```

### Async Integration Strategy

#### Runtime Bridge via `pyo3-asyncio`
```rust
use pyo3_asyncio::tokio::future_into_py;
use pyo3_asyncio::tokio::into_future;

// Rust Future → Python Coroutine
let py_coro = future_into_py(py, rust_future)?;

// Python Coroutine → Rust Future
let rust_future = into_future(py_coroutine)?;
```

#### Event Loop Management
- Single Tokio runtime for Rust execution
- Python asyncio integration via `pyo3-asyncio`
- Proper cleanup and shutdown handling

## Implementation Priorities

### Phase 1: Foundation
1. **PyTask** basic implementation
2. **PyContext** dict-like interface
3. **Simple sync execution** proof-of-concept

### Phase 2: Core Functionality
1. **PyWorkflow** builder pattern
2. **PyExecutor** basic execution
3. **Error handling** exception mapping

### Phase 3: Async Integration
1. **Async task execution** via `pyo3-asyncio`
2. **Future bridging** between runtimes
3. **Status monitoring** callbacks

### Phase 4: Advanced Features
1. **Complex data marshalling**
2. **Performance optimization**
3. **Retry policy** configuration
4. **Logging integration**

## Key Challenges Identified

### 1. Async Runtime Integration
- **Challenge**: Bridging Python asyncio with Tokio
- **Solution**: `pyo3-asyncio` with careful event loop management

### 2. Data Serialization Performance
- **Challenge**: Efficient cross-language data exchange
- **Solution**: JSON intermediate format with optional binary optimization

### 3. Error Context Preservation
- **Challenge**: Maintaining error chains across language boundary
- **Solution**: Custom exception hierarchy with detailed context

### 4. Memory Management
- **Challenge**: Object lifetime across Python/Rust boundary
- **Solution**: PyO3 reference counting with explicit cleanup

## Conclusion

The Cloacina architecture is well-suited for Python integration:

✅ **Clear async interfaces** ready for `pyo3-asyncio` bridge
✅ **JSON-based serialization** enables easy data marshalling
✅ **Comprehensive error handling** maps well to Python exceptions
✅ **Modular design** allows selective exposure to Python
✅ **Database abstraction** simplifies Python configuration

The existing macro system provides a clear template for the Python decorator API, and the trait-based architecture allows for clean wrapper implementations.
