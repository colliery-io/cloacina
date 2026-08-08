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

## Named instances

Everything above configures a *single run*. `params(...)` also powers
**workflow instances** — a persistent, named binding of this template to a set
of values, optionally on a schedule. `sync_prod` = this workflow + these
parameters + this cron.

Create one:

```bash
cloacinactl instance create sync_file sync_prod \
  --param source=/data/prod \
  --param dst=/backup/prod \
  --param max_files=500 \
  --cron "0 3 * * *"
```

`--param` is repeatable. Values are parsed as JSON when they parse, so
`max_files=500` binds the *number* 500 and matches the declared `i64`; bare
text like `mode=copy` binds a string. To reuse a shared file of values and
override a couple, combine the two — explicit flags win:

```bash
cloacinactl instance create sync_file sync_staging \
  --params shared-params.json --param dst=/backup/staging --cron "0 4 * * *"
```

The values are validated against `params(...)` at **creation** time, using the
same check that validates a per-run `--context`. A missing required parameter
is rejected when you create the instance, not at 3am on the first fire.

Inspect and remove:

```bash
cloacinactl instance list sync_file
cloacinactl instance inspect sync_file sync_prod
cloacinactl instance delete sync_file sync_prod
```

Omit `--cron` to create an **unscheduled** instance: a durable named set of
bound values that never fires on its own. Useful for holding a
known-good configuration you trigger deliberately.

When an instance fires, its bound parameters arrive exactly as they do for a
per-run `--context` — as top-level context keys — so tasks read them the same
way, with no instance-specific code.

Because an instance *is* a schedule, the existing pause/resume controls apply
to it (`cloacinactl trigger pause sync_file`).

See [Workflow Instances](../../../../docs/content/engine/scheduling/workflow-instances.md)
for the full model.
