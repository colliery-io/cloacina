---
title: "Troubleshooting"
description: "Solutions to common problems when using Cloacina"
weight: 56
---

# Troubleshooting

This guide covers common issues encountered when developing with or deploying Cloacina, organized by category. Each entry includes the symptom you observe, the underlying cause, and a step-by-step solution.

---

## Database Issues

### 1. Migration failed / schema errors on startup

**Symptom:**

```
Database error: __diesel_schema_migrations does not exist
```

or

```
Database connection failed: connection refused
```

The runner fails to start and reports migration or schema-related errors.

**Cause:**

The database has not been initialized with the required schema, or the connection string is incorrect. This commonly happens when:
- You are pointing at an empty database that has never had migrations run.
- The PostgreSQL/SQLite service is not running.
- The `DATABASE_URL` environment variable is set to a stale or incorrect path.

**Solution:**

1. Verify the database service is running:
   ```bash
   # PostgreSQL
   pg_isready -h localhost -p 5432

   # SQLite — ensure the file path exists and is writable
   ls -la /path/to/your/database.sqlite
   ```

2. Run migrations. Cloacina applies migrations automatically on startup via the `Database::new()` constructor. If you need to run them manually:
   ```bash
   # Using the angreal task
   angreal db migrate

   # Or directly via diesel
   diesel migration run --database-url "$DATABASE_URL"
   ```

3. If you see `__diesel_schema_migrations does not exist`, the database was likely created but never had migrations applied. Drop and recreate:
   ```bash
   diesel database reset --database-url "$DATABASE_URL"
   ```

---

### 2. "Database is locked" with SQLite (concurrent access)

**Symptom:**

```
Database error: database is locked
```

Multiple operations fail intermittently with lock errors when using the SQLite backend.

**Cause:**

SQLite allows only one writer at a time. When multiple runner instances or threads attempt concurrent writes, SQLite returns `SQLITE_BUSY`. This is especially common when:
- Running multiple test processes against the same SQLite file.
- Using SQLite in a multi-runner deployment (which is not supported).
- The WAL mode is not enabled.

**Solution:**

1. **For development/testing:** Ensure each test uses its own database file. Cloacina's test harness creates temporary databases per test. Never share a SQLite file across concurrent processes.

2. **Enable WAL mode** if you must use SQLite with moderate concurrency:
   ```sql
   PRAGMA journal_mode=WAL;
   PRAGMA busy_timeout=5000;
   ```

3. **For production:** Switch to PostgreSQL. SQLite is suitable only for single-runner, single-tenant deployments:
   ```rust
   let db = Database::new("postgresql://user:pass@localhost/cloacina").await?;
   ```

---

### 3. Connection pool exhausted

**Symptom:**

```
Connection pool error: Pool::get() timed out after waiting for 30 seconds
```

or

```
Connection pool error: Unable to acquire connection from pool
```

Requests or task executions hang and then fail with pool errors.

**Cause:**

All connections in the pool are in use and none are being returned. Common causes:
- `db_pool_size` is set too low for your concurrency level.
- Long-running transactions are holding connections.
- A deadlock in application code prevents connections from being released.

**Solution:**

1. Increase the pool size in your runner configuration:
   ```rust
   let config = DefaultRunnerConfig::builder()
       .db_pool_size(20)  // Default is 10
       .build();
   ```

2. Ensure task code does not hold database connections across await points. Each DAL operation should acquire and release its connection within the same scope.

3. Monitor pool metrics. If connections are leaking, check for panics in task code that may skip cleanup. Enable `RUST_LOG=deadpool=debug` to see pool activity.

4. As a rule of thumb, set pool size to: `max_concurrent_tasks + 5` (headroom for scheduler, sweeper, and reconciler).

---

### 4. Stale claims blocking task execution

**Symptom:**

Tasks remain in "Running" state indefinitely. New executions of the same workflow are blocked waiting for the stale task to complete. Logs may show:

```
CRITICAL: Context saved but mark_completed failed — task may be re-executed by stale claim sweeper
```

**Cause:**

A runner instance crashed (or was killed with SIGKILL) while holding task claims. The heartbeat stopped updating, but the claim record was never released. Until the stale claim sweeper detects and clears these claims, those tasks block pipeline progress.

**Solution:**

1. **Wait for automatic recovery.** The stale claim sweeper runs at `stale_claim_sweep_interval` (default 30s) and marks claims as stale if the heartbeat is older than `stale_claim_threshold` (default 60s). Once released, the task will be rescheduled.

2. **Tune thresholds** for faster detection. The `stale_claim_sweep_interval` and `stale_claim_threshold` fields use defaults (30s and 60s respectively) and are not exposed on the builder. Restart the runner to pick up a fresh sweep cycle.

3. **Manual intervention** — if you need immediate recovery, reset stuck tasks:
   ```sql
   -- PostgreSQL
   UPDATE task_executions
   SET status = 'Ready', claimed_by = NULL, heartbeat_at = NULL
   WHERE status = 'Running'
     AND heartbeat_at < NOW() - INTERVAL '2 minutes';
   ```

4. **Always use fresh databases when testing packaged workflows.** Stale pipeline state from previous test runs causes misleading failures.

---

## Runtime Errors

### 5. "Workflow not found" after registration

**Symptom:**

```
Workflow not found: my_workflow
```

or

```
Workflow not found in registry: my_workflow
```

You registered a workflow but execution fails with "not found."

**Cause:**

This happens when:
- The workflow was registered in a different runner instance (multi-tenant deployments without shared state).
- The workflow package was registered in the database but the reconciler has not yet loaded it into the in-memory registry.
- There is a name mismatch between registration and execution (e.g., module prefix differences).

**Solution:**

1. **Check the reconciler interval.** After package registration, the reconciler must run before the workflow is available in-memory. Default interval is 60 seconds:
   ```rust
   let config = DefaultRunnerConfig::builder()
       .registry_reconcile_interval(Duration::from_secs(10))
       .build();
   ```

2. **Verify the exact workflow name** including any namespace prefix:
   ```rust
   // Registration name must match execution name exactly
   runner.execute("my_package::my_workflow", context).await?;
   ```

3. **Enable startup reconciliation** (on by default) to ensure packages are loaded before accepting work:
   ```rust
   let config = DefaultRunnerConfig::builder()
       .registry_enable_startup_reconciliation(true)
       .build();
   ```

4. Check logs for reconciler activity:
   ```bash
   RUST_LOG=cloacina::registry::reconciler=debug cargo run
   ```

---

### 6. Task panics not being caught (unwind safety)

**Symptom:**

The runner process crashes entirely rather than marking a task as failed. You see:

```
thread 'tokio-runtime-worker' panicked at 'index out of bounds: ...'
```

**Cause:**

By default, Cloacina executes tasks on blocking threads via `spawn_blocking` and catches panics with `std::panic::catch_unwind`. However, this only works if:
- The task's `execute` method is `UnwindSafe` (a Rust safety guarantee ensuring data remains valid after a panic).
- The panic occurs in Rust code (FFI panics are undefined behavior).
- The panic does not corrupt shared state held across the unwind boundary.

If a task holds a `&mut` reference or non-unwind-safe type across the panic point, the catch may not activate.

**Solution:**

1. **Ensure tasks are self-contained.** Avoid holding references to external mutable state within the `execute` method.

2. **Use `AssertUnwindSafe` wrappers** if you need to pass non-unwind-safe types:
   ```rust
   use std::panic::AssertUnwindSafe;

   async fn execute(&self, ctx: &mut Context<Value>) -> Result<(), TaskError> {
       let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
           // potentially panicking code
       }));
       match result {
           Ok(()) => Ok(()),
           Err(_) => Err(TaskError::ExecutionFailed {
               message: "Task panicked".to_string(),
               task_id: self.name().to_string(),
               timestamp: Utc::now(),
           }),
       }
   }
   ```

3. **For Python tasks**, panics in PyO3 code will abort the process. Ensure Python code does not trigger Rust-level panics. Use proper error handling on the Python side.

---

### 7. Context serialization failures (non-JSON types)

**Symptom:**

```
Serialization error: invalid type: ...
```

or

```
Context error in task my_task: Serialization error: key 'data' is not valid JSON
```

**Cause:**

The `Context<T>` requires all stored values to be serializable to JSON (via `serde_json::Value`). Types that cannot be represented in JSON will fail:
- Byte arrays (use base64 encoding instead)
- Function pointers or closures
- Types without `Serialize`/`Deserialize` derives
- Infinity or NaN floating point values

**Solution:**

1. **Ensure all context types derive Serde traits:**
   ```rust
   #[derive(Serialize, Deserialize)]
   struct MyData {
       name: String,
       count: u64,
   }
   ```

2. **For binary data**, encode as base64 before storing:
   ```rust
   use base64::Engine;
   let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
   ctx.insert("binary_data", encoded)?;
   ```

3. **For complex types**, implement custom serialization or store only the data needed for downstream tasks.

4. **Check for NaN/Infinity** in floating point values — JSON does not support these:
   ```rust
   if value.is_nan() || value.is_infinite() {
       return Err(TaskError::ExecutionFailed {
           message: "Cannot serialize NaN/Infinity to JSON context".to_string(),
           task_id: self.name().to_string(),
           timestamp: Utc::now(),
       });
   }
   ```

---

### 8. "Task timeout exceeded" — causes and tuning

**Symptom:**

```
Task timeout: my_task exceeded 300s
```

or

```
Pipeline timeout after 3600s
```

Tasks or entire pipelines are forcibly terminated after the timeout period.

**Cause:**

The default timeouts are:
- **Task timeout:** 300 seconds (5 minutes)
- **Pipeline timeout:** 3600 seconds (1 hour)

Tasks that perform long-running operations (large data transfers, external API calls with retries, ML training) may exceed these limits.

**Solution:**

1. **Increase task timeout:**
   ```rust
   let config = DefaultRunnerConfig::builder()
       .task_timeout(Duration::from_secs(1800))  // 30 minutes
       .build();
   ```

2. **Increase pipeline timeout:**
   ```rust
   let config = DefaultRunnerConfig::builder()
       .pipeline_timeout(Some(Duration::from_secs(7200)))  // 2 hours
       .build();
   ```

3. **Disable pipeline timeout** for unbounded workflows:
   ```rust
   let config = DefaultRunnerConfig::builder()
       .pipeline_timeout(None)
       .build();
   ```

4. **Better approach — break long tasks into smaller steps.** Save intermediate results to context keys so progress is recoverable:
   ```rust
   async fn execute(&self, ctx: &mut Context<Value>) -> Result<(), TaskError> {
       for (i, chunk) in data.chunks(1000).enumerate() {
           let result = process_chunk(chunk)?;
           ctx.insert(format!("chunk_{i}"), result)?;  // Save progress to context
       }
       Ok(())
   }
   ```

---

### 9. Deadlocked workflows (runtime circular dependencies)

**Symptom:**

A workflow runs indefinitely with no tasks progressing. All tasks show "Pending" or "Waiting" state. The scheduler log shows no tasks becoming ready.

**Cause:**

While Cloacina detects cyclic dependencies at build time via `ValidationError::CyclicDependency`, runtime deadlocks can still occur when:
- Tasks are waiting on context keys that are never written by upstream tasks (implicit dependencies).
- Trigger rules reference task states that form a logical cycle not captured in the DAG.
- External systems that tasks depend on are themselves blocked.

**Solution:**

1. **Check for implicit dependencies.** If task B reads a context key that task A writes, but there is no explicit dependency edge from A to B, add it:
   ```rust
   workflow.add_dependency("task_b", "task_a")?;
   ```

2. **Inspect the workflow graph** for logical cycles:
   ```rust
   // Validation catches explicit cycles
   let result = workflow.validate();
   if let Err(ValidationError::CyclicDependency { cycle }) = result {
       eprintln!("Cycle detected: {:?}", cycle);
   }
   ```

3. **Add timeouts to trigger rules** so workflows fail loudly rather than hanging silently.

4. **Enable debug logging** to see which tasks are blocked and why:
   ```bash
   RUST_LOG=cloacina::executor=debug,cloacina::execution_planner=debug cargo run
   ```

---

## Computation Graphs

### 10. "Unresolved module or unlinked crate cloacina_computation_graph"

**Symptom:**

```
error[E0433]: failed to resolve: use of undeclared crate or module `cloacina_computation_graph`
```

or linker errors mentioning `cloacina_computation_graph` symbols.

**Cause:**

The `#[computation_graph]` macro expands into code that references types from the `cloacina-computation-graph` crate. If your `Cargo.toml` does not include this dependency (or only depends on the top-level `cloacina` crate without the right re-exports), compilation fails.

**Solution:**

1. **For embedded mode** (using the full `cloacina` crate), ensure you import through Cloacina's re-exports:
   ```rust
   use cloacina::computation_graph::*;
   ```

2. **For packaged mode** (standalone cdylib), add the dependency explicitly:
   ```toml
   [dependencies]
   cloacina-computation-graph = { version = "0.4" }
   cloacina-macros = { version = "0.4" }
   ```

3. **Verify the feature flags** — computation graph support requires the `macros` feature:
   ```toml
   [dependencies]
   cloacina = { version = "0.4", features = ["macros"] }
   ```

---

### 11. Graph nodes not firing — reaction criteria not met

**Symptom:**

The computation graph is loaded and the accumulators are receiving data, but the graph function never executes. No output is produced.

**Cause:**

The graph scheduler fires the graph function only when the reaction criteria are satisfied:
- **`when_any`**: At least one accumulator has received a new value since the last execution.
- **`when_all`**: All declared accumulators have received at least one value.

If using `when_all` and one source never publishes, the graph will never fire.

**Solution:**

1. **Check the reaction mode** in your graph definition:
   ```rust
   #[cloacina_macros::computation_graph(
       react = when_any(source1, source2),
       graph = { ... }
   )]
   ```

2. **Verify all sources are publishing.** Enable debug logging for the scheduler:
   ```bash
   RUST_LOG=cloacina::computation_graph::scheduler=debug cargo run
   ```

3. **Check the input cache** — if a source name in the graph does not match the actual accumulator name, the cache entry will never appear:
   ```rust
   // Graph expects "market_data" but publisher sends to "market-data"
   // These do NOT match — use consistent naming
   ```

4. **For `when_all` mode**, ensure all sources produce at least one initial value. Consider using `when_any` during development for easier debugging.

---

### 12. Accumulator not receiving events — channel closed

**Symptom:**

```
missing input: source 'my_source' not found in cache
```

The accumulator exists but never receives data. Logs may show the channel was dropped or closed.

**Cause:**

The accumulator's internal channel was closed because:
- The producer (publisher) was dropped before the graph started consuming.
- The channel capacity was exhausted and the producer timed out (backpressure).
- The graph was registered after the producer started, missing the initial messages.

**Solution:**

1. **Ensure registration order:** Register the computation graph before starting producers. The graph scheduler creates channels during graph registration.

2. **Check channel capacity.** If using bounded channels, increase the buffer or switch to unbounded:
   ```rust
   // In scheduler configuration
   let scheduler = ComputationGraphScheduler::new(config);
   ```

3. **Verify source names match exactly** between the publisher and the graph declaration. Source names are case-sensitive and use the `SourceName` type.

4. **Debug serialization format mismatch.** In debug builds, data is serialized as JSON; in release builds, as bincode. Ensure producer and consumer are built with the same profile:
   ```
   // Debug: JSON wire format
   // Release: bincode wire format
   // Mixing profiles will cause deserialization failures
   ```

---

## Multi-tenancy

### 13. "Schema does not exist" — first-time setup

**Symptom:**

```
Database error: schema "tenant_acme" does not exist
```

or

```
relation "tenant_acme.task_executions" does not exist
```

**Cause:**

The tenant schema has not been provisioned. In Cloacina's multi-tenant PostgreSQL mode, each tenant operates in an isolated schema. Before a tenant can use the system, an administrator must create the schema and run migrations within it.

**Solution:**

1. **Use the DatabaseAdmin API** to provision the tenant:
   ```rust
   use cloacina::database::admin::{DatabaseAdmin, TenantConfig};

   let admin = DatabaseAdmin::new(database);
   let credentials = admin.create_tenant(TenantConfig {
       schema_name: "tenant_acme".to_string(),
       username: "acme_user".to_string(),
       password: String::new(),  // auto-generates secure password
   }).await?;
   ```

2. **Via Python bindings:**
   ```python
   from cloaca import Admin

   admin = Admin(database_url)
   creds = admin.create_tenant(
       schema_name="tenant_acme",
       username="acme_user",
   )
   ```

3. **Verify the schema exists:**
   ```sql
   SELECT schema_name FROM information_schema.schemata
   WHERE schema_name = 'tenant_acme';
   ```

---

### 14. Tenant isolation failures — using wrong runner instance

**Symptom:**

Workflows from tenant A are visible to tenant B, or tasks execute with the wrong tenant context. Data appears to "leak" between tenants.

**Cause:**

Each runner instance is bound to a specific tenant via its `search_path` or connection configuration. If runners share a connection pool or if a runner is misconfigured to use the wrong schema, isolation breaks.

**Solution:**

1. **Each tenant requires its own runner instance** with a dedicated connection string:
   ```rust
   // Tenant-specific connection
   let db = Database::new("postgresql://acme_user:pass@host/db?options=-c search_path=tenant_acme").await?;
   let runner = DefaultRunner::new(db, config).await?;
   ```

2. **Never share a `DefaultRunner` across tenants.** The runner's database pool is tied to one schema.

3. **Verify isolation** by checking the current schema:
   ```sql
   SHOW search_path;  -- Should show the tenant's schema
   ```

4. **For the `default_tenant_id` in reconciler config**, ensure it matches the actual schema the runner operates in:
   ```rust
   // ReconcilerConfig default_tenant_id must match the connection's search_path
   ```

---

### 15. Admin API permissions — PostgreSQL role requirements

**Symptom:**

```
SQL execution error: Failed to create schema 'tenant_new': permission denied for database cloacina
```

or

```
Invalid configuration: permission denied to create role
```

**Cause:**

The `DatabaseAdmin` operations require elevated PostgreSQL privileges. The admin connection must use a role that has:
- `CREATE` privilege on the database (for schema creation)
- `CREATEROLE` privilege (for creating tenant users)
- Ownership or superuser access for granting permissions

**Solution:**

1. **Create a dedicated admin role:**
   ```sql
   CREATE ROLE cloacina_admin WITH LOGIN PASSWORD 'secure_pass' CREATEROLE;
   GRANT CREATE ON DATABASE cloacina TO cloacina_admin;
   GRANT ALL ON SCHEMA public TO cloacina_admin;
   ```

2. **Use the admin role only for provisioning** — not for day-to-day runner operations:
   ```rust
   // Admin connection (elevated privileges)
   let admin_db = Database::new("postgresql://cloacina_admin:pass@host/cloacina").await?;
   let admin = DatabaseAdmin::new(admin_db);

   // Tenant runner connection (limited privileges)
   let tenant_db = Database::new(&credentials.connection_string).await?;
   ```

3. **Validate schema and username** before creation. Cloacina validates inputs to prevent SQL injection:
   - Schema names: must start with a letter or underscore, contain only alphanumerics and underscores
   - Usernames: same constraints, plus no reserved names (e.g., `postgres`, `admin`)

---

## Packaging

### 16. "No bin target available for cargo run" — library vs binary crates

**Symptom:**

```
error: a bin target must be available for `cargo run`
```

When trying to `cargo run` a workflow package.

**Cause:**

Workflow packages are compiled as `cdylib` (dynamic libraries), not binary executables. They are loaded by the runner at runtime, not run directly.

**Solution:**

1. **Packages are not meant to be run directly.** Instead, register and load them:
   ```rust
   runner.register_package("/path/to/my_package.so").await?;
   ```

2. **For testing your package**, create a separate binary crate that loads it:
   ```toml
   # In a test binary's Cargo.toml
   [[bin]]
   name = "test_runner"
   path = "src/main.rs"
   ```

3. **Ensure your Cargo.toml declares the correct crate type:**
   ```toml
   [lib]
   crate-type = ["cdylib"]
   ```

4. **To build the package:**
   ```bash
   cargo build --release
   # Output: target/release/libmy_package.so (Linux)
   # Output: target/release/libmy_package.dylib (macOS)
   ```

---

### 17. Reconciler not loading packages — timing and polling interval

**Symptom:**

You registered a package via the API, but the workflow does not appear in the runner's task registry. Logs show:

```
Registered computation graph constructor: my_graph
```

but no corresponding "loaded workflow" message.

**Cause:**

The registry reconciler polls for new packages on a fixed interval (default: 60 seconds). After registration in the database, there is a delay before the in-memory registry is updated.

**Solution:**

1. **Reduce the reconcile interval** for faster feedback during development:
   ```rust
   let config = DefaultRunnerConfig::builder()
       .registry_reconcile_interval(Duration::from_secs(5))
       .build();
   ```

2. **Ensure startup reconciliation is enabled** (default: true). This runs a full reconciliation before the runner starts accepting work:
   ```rust
   .registry_enable_startup_reconciliation(true)
   ```

3. **Check reconciler logs:**
   ```bash
   RUST_LOG=cloacina::registry::reconciler=info cargo run
   ```

4. **Verify the package is in the database:**
   ```sql
   SELECT package_name, version, status FROM workflow_packages
   WHERE package_name = 'my_package';
   ```

---

### 18. Package version conflicts — same name/version already registered

**Symptom:**

```
Package already exists: my_workflow v0.1.0
```

Attempting to register a package that has the same name and version as one already in the registry.

**Cause:**

Cloacina enforces unique `(package_name, version)` pairs. Re-registering the same version is rejected to prevent accidentally overwriting a running package.

**Solution:**

1. **Bump the version** in your package manifest:
   ```toml
   [package]
   name = "my_workflow"
   version = "0.1.1"  # Increment from 0.1.0
   ```

2. **Unregister the old version first** if you intentionally want to replace it:
   ```rust
   runner.unregister_package("my_workflow", "0.1.0").await?;
   runner.register_package("/path/to/updated_package.so").await?;
   ```

3. **Check for active executions** — a package cannot be unregistered while workflows are running:
   ```
   Package is in use: my_workflow v0.1.0 has 3 active executions
   ```
   Wait for active executions to complete, or cancel them before unregistering.

---

## Python (Cloaca)

### 19. ImportError / SIGSEGV on import — Python version mismatch, rpath issues

**Symptom:**

```python
>>> import cloaca
Segmentation fault (core dumped)
```

or

```
ImportError: /path/to/cloaca.so: undefined symbol: _Py_Dealloc
```

or immediate crash on `import cloaca` without any Python traceback.

**Cause:**

This is typically caused by:

1. **Python version mismatch:** The wheel was built with `abi3-py39` (stable ABI for Python 3.9+). Using Python 3.8 or earlier will fail.
2. **OpenSSL/libpq conflicts:** When the PostgreSQL feature is enabled, the shared library links against system OpenSSL. If the Python environment has a different OpenSSL in its runtime library path (rpath), symbol conflicts cause SIGSEGV.
3. **Fork safety with OpenSSL:** Importing `cloaca` after `fork()` (e.g., in multiprocessing) can trigger SIGSEGV due to OpenSSL's unsafe atexit handler. See [diesel#3441](https://github.com/diesel-rs/diesel/issues/3441).

**Solution:**

1. **Verify Python version:**
   ```bash
   python --version  # Must be >= 3.9
   ```

2. **Check OpenSSL linkage:**
   ```bash
   # Linux
   ldd $(python -c "import cloaca; print(cloaca.__file__)")

   # macOS
   otool -L $(python -c "import cloaca; print(cloaca.__file__)")
   ```
   Ensure the OpenSSL version matches what libpq expects.

3. **For fork-related SIGSEGV:** Import `cloaca` in the parent process before forking, or use `spawn` instead of `fork` for multiprocessing:
   ```python
   import multiprocessing
   multiprocessing.set_start_method("spawn")
   ```

4. **If building from source**, ensure system OpenSSL is used (not vendored) to match libpq:
   ```bash
   # Do NOT set OPENSSL_STATIC=1
   # Ensure openssl-sys links to system OpenSSL
   cargo build --features extension-module
   ```

5. **Applied fixes in Cloacina's codebase:**
   - OpenSSL is initialized early via `#[ctor]` in `cloacina/src/database/connection.rs` to run before any async runtime or test setup.
   - Test packages are cached with `OnceLock` to ensure package building (which forks) happens before DB initialization.

6. **Debugging tips:**
   - GDB slows execution enough to mask race conditions — if tests pass under GDB, suspect a timing issue.
   - The SIGSEGV typically occurs during program exit when OpenSSL cleanup races with connection pool threads.
   - Disable ASLR for reproducible crashes: `setarch $(uname -m) -R python -c "import cloaca"`
   - Try AddressSanitizer: `RUSTFLAGS="-Z sanitizer=address" cargo build`

---

### 20. "Backend not available" — missing feature flags in wheel

**Symptom:**

```python
>>> from cloaca import Runner
RuntimeError: Backend not available: postgres support was not compiled into this wheel
```

or

```python
>>> runner = Runner("postgresql://...")
RuntimeError: Backend not available: sqlite support was not compiled into this wheel
```

**Cause:**

The Cloaca Python wheel is built with specific Cargo feature flags. The pre-built wheels may not include all backends. The available features are:

- `postgres` — PostgreSQL support (requires libpq)
- `sqlite` — SQLite support (bundled libsqlite3)
- `kafka` — Kafka integration (requires librdkafka)
- `extension-module` — Required for building as a Python extension

**Solution:**

1. **Check which features are compiled in:**
   ```python
   import cloaca
   print(cloaca.features())  # Lists compiled features
   ```

2. **Build from source with required features:**
   ```bash
   # Install maturin
   pip install maturin

   # Build with all features
   maturin build --release --features "extension-module,postgres,sqlite,kafka"

   # Or develop mode
   maturin develop --features "extension-module,postgres,sqlite"
   ```

3. **For PostgreSQL on Linux**, ensure `libpq-dev` is installed:
   ```bash
   sudo apt-get install libpq-dev
   ```

---

### 21. Context type conversion errors between Python and Rust

**Symptom:**

```python
RuntimeError: Failed to convert Python object to Rust type: 'dict' object cannot be converted to 'String'
```

or

```
Context error in task py_task: Serialization error: ...
```

**Cause:**

The Python-Rust boundary uses `pythonize` (PyO3 + serde) for type conversion. Python types must map cleanly to JSON-compatible Rust types:

| Python | Rust/JSON |
|--------|-----------|
| `str` | `String` |
| `int` | `i64` / `u64` |
| `float` | `f64` |
| `bool` | `bool` |
| `None` | `null` |
| `dict` | `Object` |
| `list` | `Array` |

Types that fail: `datetime` (not JSON-native), `bytes` (use base64), custom classes without `__dict__`.

**Solution:**

1. **Convert Python objects to JSON-friendly types before passing to context:**
   ```python
   import json
   from datetime import datetime

   # Convert datetime to ISO string
   ctx["timestamp"] = datetime.now().isoformat()

   # Convert bytes to base64
   import base64
   ctx["data"] = base64.b64encode(raw_bytes).decode("utf-8")
   ```

2. **For custom classes**, convert to dict:
   ```python
   ctx["my_obj"] = vars(my_object)  # or my_object.__dict__
   ```

3. **Avoid numpy arrays** — convert to lists first:
   ```python
   ctx["array"] = my_numpy_array.tolist()
   ```

---

## Performance

### 22. Slow task scheduling — polling interval tuning

**Symptom:**

Tasks appear ready but take a long time (multiple seconds) to begin execution. There is visible latency between task completion and the next task starting.

**Cause:**

The scheduler polls for ready tasks at a fixed interval. The default `scheduler_poll_interval` is 100ms, which should be sufficient for most workloads. However:
- If you overrode it to a larger value, scheduling latency increases.
- High database query latency can make each poll cycle slow.
- Too many concurrent pipelines competing for poll cycles.

**Solution:**

1. **Check and tune the poll interval:**
   ```rust
   let config = DefaultRunnerConfig::builder()
       .scheduler_poll_interval(Duration::from_millis(50))  // Faster polling
       .build();
   ```

2. **Monitor database query time.** If each poll takes >50ms, the bottleneck is the database:
   ```bash
   RUST_LOG=cloacina::execution_planner=trace cargo run
   ```

3. **Ensure database indexes exist** on `task_executions.status` and `task_executions.pipeline_id`. Migrations create these, but verify:
   ```sql
   SELECT indexname FROM pg_indexes WHERE tablename = 'task_executions';
   ```

4. **For trigger-based scheduling**, the base poll interval is separate (default 1s):
   ```rust
   .trigger_base_poll_interval(Duration::from_millis(500))
   ```

---

### 23. High memory usage — large contexts being cloned

**Symptom:**

Runner memory grows continuously as workflows execute. Each active pipeline consumes significantly more memory than expected.

**Cause:**

The `Context<serde_json::Value>` is cloned for each task execution and stored with checkpoint state. If tasks insert large payloads (multi-MB JSON objects, encoded files), memory usage scales with: `context_size * active_tasks * retry_attempts`.

**Solution:**

1. **Keep context payloads small.** Store large data externally (S3, filesystem) and keep only references in context:
   ```rust
   // Instead of:
   ctx.insert("huge_dataframe", large_json)?;

   // Do:
   ctx.insert("dataframe_path", "/tmp/output/frame_001.parquet")?;
   ```

2. **Clean up intermediate results** in downstream tasks:
   ```rust
   // Remove large intermediate values no longer needed
   ctx.remove("intermediate_result");
   ```

3. **Monitor per-pipeline memory** via structured logging and metrics.

4. **Reduce max concurrent tasks** if memory is constrained:
   ```rust
   .max_concurrent_tasks(2)  // Fewer concurrent pipelines = less memory
   ```

---

### 24. Cron catchup storm after long downtime

**Symptom:**

After the runner restarts from extended downtime, hundreds or thousands of workflow executions are triggered simultaneously. The system becomes overloaded.

**Cause:**

When a cron schedule has `catchup_policy = "run_all"` (or uses the default `max_catchup_executions = MAX`), the scheduler calculates all missed execution times during the downtime window and enqueues them all.

**Solution:**

1. **Set a catchup policy of "skip"** for schedules that do not need historical backfill:
   ```rust
   Schedule::cron("hourly_report", "0 * * * *")
       .catchup_policy(CatchupPolicy::Skip)
   ```

2. **Limit catchup executions:**
   ```rust
   let config = DefaultRunnerConfig::builder()
       .cron_max_catchup_executions(5)  // At most 5 missed runs
       .build();
   ```

3. **Set maximum recovery age** to ignore ancient missed executions:
   ```rust
   .cron_max_recovery_age(Duration::from_secs(3600))  // Only catch up last hour
   ```

4. **For critical schedules that must catch up**, ensure `max_concurrent_tasks` is high enough to process the backlog without starving other work.

---

## CI/Development

### 25. Pre-commit hook failures — formatting, license headers

**Symptom:**

```
error: formatting check failed
  Diff in src/my_file.rs
```

or

```
error: missing license header in src/new_file.rs
```

Commits are rejected by pre-commit hooks.

**Cause:**

The repository enforces:
- `cargo fmt` formatting on all Rust files.
- Apache 2.0 license headers on all source files.
- Clippy lint checks.

**Solution:**

1. **Fix formatting:**
   ```bash
   cargo fmt --all
   ```

2. **Add license headers.** Every `.rs` file must start with:
   ```rust
   /*
    *  Copyright 2025-2026 Colliery Software
    *
    *  Licensed under the Apache License, Version 2.0 (the "License");
    *  ...
    */
   ```

3. **Run the full check locally before committing:**
   ```bash
   angreal ci lint
   ```

4. **For Clippy failures**, fix warnings or add targeted allow attributes:
   ```rust
   #[allow(clippy::too_many_arguments)]
   fn complex_function(...) { }
   ```

---

### 26. Tutorial/example compilation failures after API changes

**Symptom:**

```
error[E0599]: no method named `old_method` found for struct `Runner`
```

Tutorials or examples fail to compile after pulling recent changes.

**Cause:**

API changes in the core crates may not be reflected in the tutorial and example code immediately. The CI runs example tests, but there can be drift between development branches.

**Solution:**

1. **Check the latest API** in the reference documentation or by reading the relevant source:
   ```bash
   cargo doc --open -p cloacina
   ```

2. **Run the tutorial tests** to identify all breakages:
   ```bash
   angreal test tutorials
   ```

3. **Common API migration patterns:**
   - Method renamed: Check the changelog or grep for the old name to find the replacement.
   - New required parameter: Look at `DefaultRunnerConfig::builder()` for new fields with defaults.
   - Type moved to new module: Use `cargo doc` to find the new location.

4. **CI retry logic** exists for flaky tutorial tests. If a test fails intermittently but passes on retry, it is likely a timing issue rather than an API change.

---

### 27. Docker services not starting — port conflicts

**Symptom:**

```
Error starting userland proxy: listen tcp4 0.0.0.0:5432: bind: address already in use
```

Docker Compose fails to start required services (PostgreSQL, Kafka, etc.).

**Cause:**

Another process (often a local PostgreSQL or Kafka installation) is already bound to the required port.

**Solution:**

1. **Check what is using the port:**
   ```bash
   # Linux
   ss -tlnp | grep 5432

   # macOS
   lsof -i :5432
   ```

2. **Stop the conflicting service:**
   ```bash
   # Stop local PostgreSQL
   brew services stop postgresql  # macOS
   sudo systemctl stop postgresql  # Linux
   ```

3. **Or remap ports** in `docker-compose.yml`:
   ```yaml
   services:
     postgres:
       ports:
         - "5433:5432"  # Use 5433 externally
   ```
   Then update your `DATABASE_URL`:
   ```bash
   export DATABASE_URL="postgresql://user:pass@localhost:5433/cloacina"
   ```

4. **For CI environments**, ensure the service startup order is correct and previous containers are cleaned up:
   ```bash
   docker compose down -v && docker compose up -d
   ```

---

## Quick Reference: Error to Solution

| Error Message | Section |
|---|---|
| `Database error: database is locked` | [#2 SQLite concurrent access](#2-database-is-locked-with-sqlite-concurrent-access) |
| `Connection pool error: Pool::get() timed out` | [#3 Connection pool exhausted](#3-connection-pool-exhausted) |
| `Workflow not found: X` | [#5 Workflow not found](#5-workflow-not-found-after-registration) |
| `Task timeout: X exceeded Ys` | [#8 Task timeout](#8-task-timeout-exceeded--causes-and-tuning) |
| `Pipeline timeout after Xs` | [#8 Task timeout](#8-task-timeout-exceeded--causes-and-tuning) |
| `Serialization error: ...` | [#7 Context serialization](#7-context-serialization-failures-non-json-types) |
| `Circular dependency detected` | [#9 Deadlocked workflows](#9-deadlocked-workflows-runtime-circular-dependencies) |
| `missing input: source 'X' not found in cache` | [#12 Accumulator not receiving](#12-accumulator-not-receiving-events--channel-closed) |
| `Package already exists: X vY` | [#18 Package version conflicts](#18-package-version-conflicts--same-nameversion-already-registered) |
| `Segmentation fault` on Python import | [#19 SIGSEGV on import](#19-importerror--sigsegv-on-import--python-version-mismatch-rpath-issues) |
| `Backend not available` | [#20 Missing feature flags](#20-backend-not-available--missing-feature-flags-in-wheel) |
| `schema "X" does not exist` | [#13 First-time tenant setup](#13-schema-does-not-exist--first-time-setup) |
| `No bin target available` | [#16 Library vs binary crates](#16-no-bin-target-available-for-cargo-run--library-vs-binary-crates) |

---

## Getting More Help

If your issue is not covered here:

1. **Enable verbose logging:**
   ```bash
   RUST_LOG=cloacina=debug,cloacina::executor=trace cargo run
   ```

2. **Check the error type hierarchy** in `crates/cloacina/src/error.rs` for the full set of structured errors.

3. **Search existing issues** on the GitHub repository.

4. **For SIGSEGV crashes**, run under a debugger or sanitizer:
   ```bash
   # Address sanitizer (nightly only)
   RUSTFLAGS="-Z sanitizer=address" cargo +nightly test

   # Under GDB
   gdb --args cargo test my_failing_test
   ```
