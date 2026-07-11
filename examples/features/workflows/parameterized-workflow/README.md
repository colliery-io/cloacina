# Parameterized Workflow

One workflow **template**, many differently-configured **runs**. This example
demonstrates `params(...)` — the declared, typed, configurable surface of a
workflow — through the primary interface:

```
declare params  →  pack → upload → compile  →  run with values  →  observe
```

The template is a file-sync pipeline:

```
plan_sync → execute_sync → report
```

with four declared params:

| Param | Type | Default |
|---|---|---|
| `source` | `String` | *(required)* |
| `dst` | `String` | *(required)* |
| `mode` | `String` | `"copy"` |
| `max_files` | `i64` | `100` |

## How params are declared

```rust
#[workflow(
    name = "sync_file",
    params(
        source: String,
        dst: String,
        mode: String = "copy",
        max_files: i64 = 100,
    )
)]
pub mod sync_file { ... }
```

The compiler extracts this as the workflow's **typed input interface**; the
server validates every run's provided values against it — wrong types and
missing required params are rejected *before* anything executes. Bound values
arrive in tasks as flat top-level context keys (`context.get("source")`).

## Run it

The steps below are automated as `angreal demos features parameterized-workflow`
(the CI examples lane runs exactly that).

### 1. Stack + CLI

```bash
angreal ui up
cloacinactl config profile set demo http://localhost:8080 \
    --api-key clk_demo_public_key_0003 --tenant public --default
```

### 2. Pack + upload

```bash
cloacinactl package pack . --out parameterized-workflow-demo.cloacina
cloacinactl package upload parameterized-workflow-demo.cloacina
cloacinactl package list   # wait for build_status: success
```

### 3. Run it twice, with different values

```bash
cat > prod.json <<'EOF'
{"source": "/data/prod", "dst": "/backup/prod"}
EOF
cloacinactl workflow run sync_file --context prod.json
```

```bash
cat > archive.json <<'EOF'
{"source": "/data/archive", "dst": "/cold", "mode": "move", "max_files": 10}
EOF
cloacinactl workflow run sync_file --context archive.json
```

Same template, two independent executions with different behavior — watch
both complete:

```bash
cloacinactl execution list --workflow sync_file
```

### 4. See the validation reject a bad run

```bash
echo '{"dst": "/backup"}' > bad.json          # missing required `source`
cloacinactl workflow run sync_file --context bad.json
```

The server rejects it with a typed error before any task runs — that's the
declared interface doing its job.

## About named instances

`params(...)` also powers **workflow instances** — persistent, named,
scheduled bindings (`sync_prod` = this template + these values + this cron).
See [Workflow Instances](../../../../docs/content/engine/scheduling/workflow-instances.md).
Instance *registration* is currently an embedded-runner API
(`register_cron_workflow_instance`); a server-side registration surface is
tracked separately. Per-run parameterization — everything above — is fully
supported on the primary interface today.
