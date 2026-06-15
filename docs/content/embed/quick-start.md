---
title: "Quick Start"
description: "Run a Cloacina workflow in-process in about five minutes — in Rust or Python."
weight: 11
---

# Quick Start (Embedded)

Run a one-task workflow inside your own process, against a local SQLite database.
Pick your language — the engine is the same.

{{< tabs "embed-quickstart" >}}
{{< tab "Rust" >}}
Add the dependency (`cloacina = "0.7"`, plus `tokio`, `serde_json`), then:

```rust
use cloacina::{task, workflow, Context, TaskError};
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};

#[workflow(name = "greeting", description = "Say hello")]
pub mod greeting {
    use super::*;

    #[task(id = "hello", dependencies = [])]
    pub async fn hello(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        ctx.insert("message", serde_json::json!("Hello World!"))?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runner = DefaultRunner::with_config(
        "sqlite://app.db?mode=rwc",
        DefaultRunnerConfig::default(),
    ).await?;

    let result = runner.execute("greeting", Context::new()).await?;
    println!("status: {:?}", result.status);

    runner.shutdown().await?;
    Ok(())
}
```

Run with `cargo run`.
{{< /tab >}}
{{< tab "Python" >}}
Install with `pip install cloaca`, then:

```python
import cloaca

with cloaca.WorkflowBuilder("greeting") as builder:
    builder.description("Say hello")

    @cloaca.task(id="hello")
    def hello(context):
        context.set("message", "Hello World!")
        return context

if __name__ == "__main__":
    runner = cloaca.DefaultRunner("sqlite:///app.db")
    result = runner.execute("greeting", cloaca.Context())
    print("status:", result.status)
    runner.shutdown()
```

Run with `python first_workflow.py`.
{{< /tab >}}
{{< /tabs >}}

You just defined a workflow, ran it in-process, and persisted its state to SQLite.

## Next

- **[Tutorials]({{< ref "/embed/tutorials" >}})** — build up from here, step by step.
- **[Engine & Primitives]({{< ref "/engine" >}})** — what a Workflow, Task, Context, and Runner actually are.
- Going to production embedded? **[Running embedded in production]({{< ref "/embed/how-to" >}})**.
