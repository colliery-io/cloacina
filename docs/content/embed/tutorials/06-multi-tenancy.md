---
title: "06 — Multi-Tenancy"
description: "Give each tenant its own isolated PostgreSQL schema with one runner call."
weight: 16
aliases:
  - "/python/workflows/tutorials/06-multi-tenancy/"
  - "/workflows/tutorials/service/06-multi-tenancy/"

---

# 06 — Multi-Tenancy

On PostgreSQL, Cloacina isolates tenants by **schema**: each tenant gets its own
schema, so there is no cross-tenant access and no tenant filtering in your code.
You opt in with a single runner constructor — the workflow code is unchanged.

{{< hint type=note title="Prerequisite: a running PostgreSQL" >}}
Schema-based multi-tenancy is PostgreSQL-only — the earlier tutorials' SQLite
backend has no schemas. This tutorial assumes a reachable Postgres instance and uses
`postgresql://cloacina:cloacina@localhost:5432/cloacina`; adjust the URL to yours.
The Admin-API section additionally needs a role that can `CREATE SCHEMA`/`CREATE ROLE`.
See [Database Backends]({{< ref "/service/explanation/database-backends" >}}).
{{< /hint >}}

## A runner per tenant

`with_schema` creates (or reuses) a tenant's schema, runs migrations inside it,
and scopes every read and write to that schema. The same workflow runs against
each tenant in complete isolation.

{{< tabs "mt-runner" >}}
{{< tab "Rust" >}}
```rust
use cloacina::runner::DefaultRunner;
use cloacina::executor::WorkflowStatus;
use cloacina::{task, workflow, Context, TaskError};
use serde_json::json;
use std::collections::HashMap;

#[workflow(
    name = "customer_processing",
    description = "Process customer data in isolated tenant environment"
)]
pub mod customer_processing {
    use super::*;

    #[task(id = "process_customer_data", dependencies = [])]
    pub async fn process_customer_data(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        let tenant_id = context
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();

        context.insert("processed_records", json!(1250))?;
        context.insert("processing_completed", json!(true))?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = "postgresql://cloacina:cloacina@localhost:5432/cloacina";

    let mut tenant_runners = HashMap::new();
    for tenant_id in ["acme_corp", "globex_inc", "initech"] {
        // One schema per tenant — created on first use.
        let runner = DefaultRunner::with_schema(database_url, tenant_id).await?;
        tenant_runners.insert(tenant_id.to_string(), runner);
    }

    for (tenant_id, runner) in &tenant_runners {
        let mut context = Context::new();
        context.insert("tenant_id", json!(tenant_id))?;

        let result = runner.execute("customer_processing", context).await?;
        if matches!(result.status, WorkflowStatus::Completed) {
            let records = result
                .final_context
                .get("processed_records")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            println!("Tenant {} completed: {} records", tenant_id, records);
        }
    }

    for (_tenant_id, runner) in tenant_runners {
        runner.shutdown().await?;
    }
    Ok(())
}
```
{{< /tab >}}
{{< tab "Python" >}}
```python
import cloaca

database_url = "postgresql://cloacina:cloacina@localhost:5432/cloacina"

with cloaca.WorkflowBuilder("customer_processing") as builder:
    builder.description("Process customer data in isolated tenant environment")

    @cloaca.task(id="process_customer_data")
    def process_customer_data(context):
        tenant_id = context.get("tenant_id")
        context.set("processed_records", 1250)
        context.set("processing_completed", True)
        return context

# One schema per tenant — created on first use.
tenant_runners = {
    tid: cloaca.DefaultRunner.with_schema(database_url, tid)
    for tid in ["acme_corp", "globex_inc", "initech"]
}

for tenant_id, runner in tenant_runners.items():
    context = cloaca.Context({"tenant_id": tenant_id})
    result = runner.execute("customer_processing", context)
    if result.status == "Completed":
        records = result.final_context.get("processed_records")
        print(f"Tenant {tenant_id} completed: {records} records")

for runner in tenant_runners.values():
    runner.shutdown()
```
{{< /tab >}}
{{< /tabs >}}

Each tenant's tables live under its own schema (`acme_corp.task_executions`,
`globex_inc.task_executions`, …), so no query can cross tenants. This needs
PostgreSQL — see [Database backends]({{< ref "/service/explanation/database-backends" >}}).

## Provisioning a tenant with dedicated credentials

For stronger isolation, the **Database Admin API** provisions a tenant with its
own PostgreSQL user (auto-generated password) that can only see its own schema.
Connect a normal runner with the returned connection string.

{{< tabs "mt-admin" >}}
{{< tab "Rust" >}}
```rust
use cloacina::database::{Database, DatabaseAdmin, TenantConfig};
use cloacina::runner::DefaultRunner;

let admin_db = Database::new(admin_database_url, "cloacina", 10);
let admin = DatabaseAdmin::new(admin_db);

let tenant_config = TenantConfig {
    schema_name: "tenant_demo".to_string(),
    username: "demo_user".to_string(),
    password: String::new(), // Auto-generate secure password
};

let credentials = admin.create_tenant(tenant_config).await?;

// Runner scoped to the tenant's dedicated credentials.
let tenant_runner = DefaultRunner::new(&credentials.connection_string).await?;
```
{{< /tab >}}
{{< tab "Python" >}}
```python
import cloaca

admin = cloaca.DatabaseAdmin("postgresql://admin:admin_password@localhost:5432/myapp")

tenant_config = cloaca.TenantConfig(
    schema_name="tenant_acme_corp",
    username="acme_corp_user",
    password=""  # Auto-generate secure password
)

credentials = admin.create_tenant(tenant_config)

# Runner scoped to the tenant's dedicated credentials.
tenant_runner = cloaca.DefaultRunner(credentials.connection_string)
```
{{< /tab >}}
{{< /tabs >}}

The dedicated user gives database-level access control and a per-tenant audit
trail. For the full design rationale, see
[Multi-tenancy]({{< ref "/service/explanation/multi-tenancy" >}}).

## What you learned

A single `with_schema` call gives each tenant an isolated PostgreSQL schema, and
the Admin API adds dedicated credentials on top. Next, react to external events.

- Previous: [05 — Cron scheduling]({{< ref "/embed/tutorials/05-cron-scheduling" >}})
- Next: [07 — Event triggers]({{< ref "/embed/tutorials/07-event-triggers" >}})
- Deep dive: [Multi-tenancy]({{< ref "/service/explanation/multi-tenancy" >}})
