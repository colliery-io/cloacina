---
title: "Rust SDK"
description: "cloacina-client — typed Rust client for cloacina-server"
weight: 10
---

# Rust SDK (`cloacina-client`)

The same client `cloacinactl` is built on, published as a crate. DTOs come from `cloacina-api-types` — the crate the server's handlers build their responses from, so request/response shapes cannot drift.

## Tutorial: execute a workflow and follow its events

```toml
[dependencies]
cloacina-client = "0.7"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
futures-util = "0.3"
serde_json = "1"
```

```rust
use cloacina_client::ClientBuilder;
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClientBuilder::new("http://localhost:8080")
        .api_key(std::env::var("CLOACINA_API_KEY")?)
        .tenant("public")
        .build()?;

    let accepted = client
        .execute_workflow("my_workflow", serde_json::json!({"input": 42}))
        .await?;
    println!("scheduled {}", accepted.execution_id);

    let mut events = std::pin::pin!(client.follow_execution_events(&accepted.execution_id));
    while let Some(event) = events.next().await {
        println!("{}", event?);
    }
    Ok(())
}
```

## How-to

**Authenticate from a cloacinactl profile** (resolves `env:`/`file:` key schemes exactly like the CLI):

```rust
let client = cloacina_client::ClientBuilder::from_cloacinactl_profile(None, Some("prod"))?
    .build()?;
```

**Paginate executions:**

```rust
use cloacina_client::types::ListExecutionsQuery;

let page = client
    .list_executions(
        &ListExecutionsQuery { status: Some("Failed".into()), limit: Some(100), ..Default::default() },
        None,
    )
    .await?;
```

**Handle errors by kind** — `ClientError` maps the canonical `{error, code}` envelope:

```rust
use cloacina_client::ClientError;

match client.get_workflow("missing", None).await {
    Err(ClientError::NotFound(msg)) => eprintln!("not found: {msg}"),
    Err(ClientError::Auth(msg)) => eprintln!("check the API key: {msg}"),
    other => { other?; }
}
```

**Subscribe to raw delivery pushes** (any recipient, not just execution events):

```rust
use cloacina_client::SubscribeOptions;
let stream = client.subscribe_delivery("exec_events:<id>", SubscribeOptions::default());
```

Reconnection, dedup-on-row-id, and acks are handled inside the stream; a `4426` close (protocol version mismatch) surfaces as the terminal `ClientError::ProtocolVersion`.

## Reference

- API docs: `cargo doc -p cloacina-client` / [docs.rs](https://docs.rs/cloacina-client)
- Wire contract: [OpenAPI document](/openapi.json), [WebSocket protocol](/platform/reference/websocket-protocol/)
- Error/exit semantics shared with the CLI: [API error envelope](/platform/reference/api-error-envelope/)
