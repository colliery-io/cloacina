# cloacina::dispatcher::work_distributor <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Work distribution abstraction for efficient task notification.

This module provides the `WorkDistributor` trait that abstracts how workers
wait for new work to become available. Different backends can implement
different notification mechanisms:
- PostgreSQL: Uses LISTEN/NOTIFY for instant notifications with poll fallback
- SQLite: Uses periodic polling since SQLite lacks notification support

**Examples:**

```rust,ignore
use cloacina::dispatcher::{WorkDistributor, SqliteDistributor};

let distributor = SqliteDistributor::new();

loop {
    // Wait for work signal
    distributor.wait_for_work().await;

    // Try to claim and execute tasks
    let tasks = dal.task_execution().claim_ready_task(10).await?;
    for task in tasks {
        executor.execute(task).await?;
    }
}
```

## Structs

### `cloacina::dispatcher::work_distributor::PostgresDistributor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


PostgreSQL work distributor using LISTEN/NOTIFY.

Uses PostgreSQL's LISTEN/NOTIFY mechanism for instant work notifications.
Falls back to periodic polling every 30 seconds in case notifications are missed.
The NOTIFY is triggered by a database trigger on the `task_outbox` table:
```sql
CREATE TRIGGER task_outbox_notify
AFTER INSERT ON task_outbox
FOR EACH ROW EXECUTE FUNCTION notify_task_ready();
```

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `database_url` | `String` | The database URL for creating a dedicated listen connection |
| `notify` | `Arc < Notify >` | Notification channel for waking waiters |
| `shutdown` | `Arc < std :: sync :: atomic :: AtomicBool >` | Shutdown signal |
| `listener_handle` | `Option < tokio :: task :: JoinHandle < () > >` | Background listener task handle |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn new (database_url : & str) -> Result < Self , Box < dyn std :: error :: Error + Send + Sync > >
```

Creates a new PostgreSQL work distributor.

Spawns a background task that listens for `task_ready` notifications
and wakes any waiters.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database_url` | `-` | PostgreSQL connection URL for the listen connection |


**Returns:**

A new `PostgresDistributor` that is ready to receive notifications.

<details>
<summary>Source</summary>

```rust
    pub async fn new(database_url: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let notify = Arc::new(Notify::new());
        let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));

        // Spawn background listener
        let listener_handle =
            Self::spawn_listener(database_url.to_string(), notify.clone(), shutdown.clone())
                .await?;

        Ok(Self {
            database_url: database_url.to_string(),
            notify,
            shutdown,
            listener_handle: Some(listener_handle),
        })
    }
```

</details>



##### `spawn_listener` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn spawn_listener (database_url : String , notify : Arc < Notify > , shutdown : Arc < std :: sync :: atomic :: AtomicBool > ,) -> Result < tokio :: task :: JoinHandle < () > , Box < dyn std :: error :: Error + Send + Sync > >
```

Spawns the background listener task.

<details>
<summary>Source</summary>

```rust
    async fn spawn_listener(
        database_url: String,
        notify: Arc<Notify>,
        shutdown: Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<tokio::task::JoinHandle<()>, Box<dyn std::error::Error + Send + Sync>> {
        use futures::StreamExt;
        use tokio::sync::mpsc;

        // Parse URL and connect using tokio-postgres
        let (client, mut connection) =
            tokio_postgres::connect(&database_url, tokio_postgres::NoTls).await?;

        // Create channel to receive notifications from connection
        let (tx, mut rx) = mpsc::unbounded_channel();

        // Spawn connection driver that forwards messages to channel
        let conn_shutdown = shutdown.clone();
        tokio::spawn(async move {
            // Use poll_fn to poll the connection for messages
            let stream = futures::stream::poll_fn(move |cx| connection.poll_message(cx));

            futures::pin_mut!(stream);

            while !conn_shutdown.load(std::sync::atomic::Ordering::SeqCst) {
                match stream.next().await {
                    Some(Ok(msg)) => {
                        if tx.send(msg).is_err() {
                            // Receiver dropped
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        error!("PostgreSQL listener connection error: {}", e);
                        break;
                    }
                    None => {
                        // Connection closed
                        break;
                    }
                }
            }
        });

        // Start listening
        client.execute("LISTEN task_ready", &[]).await?;
        info!("PostgreSQL LISTEN/NOTIFY listener started on channel 'task_ready'");

        // Spawn notification handler that receives from channel
        let handle = tokio::spawn(async move {
            // Keep client alive for the duration
            let _client = client;

            loop {
                if shutdown.load(std::sync::atomic::Ordering::SeqCst) {
                    debug!("PostgreSQL listener shutting down");
                    break;
                }

                // Wait for notification with timeout
                match tokio::time::timeout(Self::POLL_FALLBACK, rx.recv()).await {
                    Ok(Some(tokio_postgres::AsyncMessage::Notification(notification))) => {
                        debug!(
                            "Received NOTIFY on channel '{}': {}",
                            notification.channel(),
                            notification.payload()
                        );
                        notify.notify_waiters();
                    }
                    Ok(Some(_)) => {
                        // Other message types (e.g., Notice) - ignore
                    }
                    Ok(None) => {
                        // Channel closed - connection dropped
                        warn!("PostgreSQL listener channel closed");
                        break;
                    }
                    Err(_) => {
                        // Timeout - trigger fallback poll
                        debug!("LISTEN timeout, triggering fallback poll");
                        notify.notify_waiters();
                    }
                }
            }
        });

        Ok(handle)
    }
```

</details>





### `cloacina::dispatcher::work_distributor::SqliteDistributor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


SQLite work distributor using periodic polling.

Since SQLite lacks notification capabilities, this implementation
uses simple periodic polling at a configurable interval.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `poll_interval` | `Duration` | Poll interval |
| `shutdown` | `Arc < std :: sync :: atomic :: AtomicBool >` | Shutdown signal |
| `notify` | `Arc < Notify >` | Notify for shutdown signal |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new () -> Self
```

Creates a new SQLite work distributor with default poll interval (500ms).

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Self {
        Self::with_poll_interval(Self::DEFAULT_POLL_INTERVAL)
    }
```

</details>



##### `with_poll_interval` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_poll_interval (poll_interval : Duration) -> Self
```

Creates a new SQLite work distributor with custom poll interval.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `poll_interval` | `-` | How often to wake up and check for work |


<details>
<summary>Source</summary>

```rust
    pub fn with_poll_interval(poll_interval: Duration) -> Self {
        Self {
            poll_interval,
            shutdown: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            notify: Arc::new(Notify::new()),
        }
    }
```

</details>





## Functions

### `cloacina::dispatcher::work_distributor::create_work_distributor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
async fn create_work_distributor (database : & crate :: Database ,) -> Result < Box < dyn WorkDistributor > , Box < dyn std :: error :: Error + Send + Sync > >
```

Creates the appropriate work distributor based on database backend.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database` | `-` | The database instance to detect backend from |


**Returns:**

A boxed `WorkDistributor` appropriate for the database backend.

<details>
<summary>Source</summary>

```rust
pub async fn create_work_distributor(
    database: &crate::Database,
) -> Result<Box<dyn WorkDistributor>, Box<dyn std::error::Error + Send + Sync>> {
    match database.backend() {
        #[cfg(feature = "postgres")]
        crate::database::BackendType::Postgres => {
            // Extract the database URL - this is tricky since we don't store it
            // For now, return an error suggesting the caller should use PostgresDistributor::new directly
            Err("PostgreSQL distributor requires database URL. Use PostgresDistributor::new() directly.".into())
        }
        #[cfg(feature = "sqlite")]
        crate::database::BackendType::Sqlite => Ok(Box::new(SqliteDistributor::new())),
        #[allow(unreachable_patterns)]
        _ => Err("Unsupported database backend".into()),
    }
}
```

</details>
