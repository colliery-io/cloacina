# Workflow Secrets

A workflow **declares** the secrets it needs; a run **binds** each declared
name to a tenant-stored secret; the value is **resolved at execution** through
a side channel and is never written into the durable context, execution
history, or logs. All through the primary interface:

```
secret create  →  pack → upload → compile  →  run with a $secret binding  →  Completed
```

The workflow is a two-task notifier:

```
resolve_token → send_notification
```

## How secrets are declared and consumed

```rust
#[workflow(
    name = "notify_oncall",
    params( channel: String = "#ops" ),
    secrets( api_token )
)]
pub mod notify_oncall {
    #[task]
    pub async fn resolve_token(ctx: &mut Context<Value>) -> Result<(), TaskError> {
        // Side channel — the plaintext is RETURNED to the task, never stored.
        let token = ctx.secret_field("api_token", "token").await?;
        ctx.insert("token_len", json!(token.len()))?;   // derived facts only
        Ok(())
    }
}
```

`secrets(api_token)` surfaces an **encrypted input slot** in the workflow's
typed interface. A run binds it with a reference — never a literal:

```json
{ "channel": "#oncall", "api_token": { "$secret": "oncall_api" } }
```

The server routes the reference away from the plaintext context (names only),
resolves the value from the tenant's encrypted secret store at dispatch, and
delivers it wrapped to the executing agent. A **literal** value for a secret
slot is rejected before anything runs.

## Run it

Automated as `angreal demos features workflow-secrets` (the CI examples lane
runs exactly that). By hand, against the demo stack:

### 1. Stack + CLI

```bash
angreal ui up
cloacinactl config profile set demo http://localhost:8080 \
    --api-key clk_demo_public_key_0003 --tenant public --default
```

The demo stack ships with a fixed demo `CLOACINA_SECRET_KEK`. Without a KEK
the server *refuses to dispatch* secret-using tasks — fail-closed, never a
plaintext fallback.

### 2. Create the tenant secret

Values come from a file (`k=@path`), stdin (`k=-`), or a prompt — never an
argv literal (which would land in shell history):

```bash
printf 's3cr3t-demo-token-value' > /tmp/token.txt
cloacinactl secret create oncall_api --field token=@/tmp/token.txt
```

### 3. Pack + upload

```bash
cloacinactl package pack . --out workflow-secrets-demo.cloacina
cloacinactl package upload workflow-secrets-demo.cloacina
cloacinactl package list   # wait for build_status: success
```

### 4. Run with the binding

```bash
cat > bind.json <<'EOF'
{ "channel": "#oncall", "api_token": { "$secret": "oncall_api" } }
EOF
cloacinactl workflow run notify_oncall --context bind.json
cloacinactl execution list --workflow notify_oncall
```

### 5. Rotate — the next run sees the new value

```bash
printf 'r0tated-token-value' > /tmp/token.txt
cloacinactl secret rotate oncall_api --field token=@/tmp/token.txt
cloacinactl workflow run notify_oncall --context bind.json
```

### 6. Values are write-only

```bash
cloacinactl secret get oncall_api    # metadata only — never the value
cloacinactl secret list
```

And try binding a literal — it's rejected before execution:

```bash
echo '{"api_token": "plaintext-token"}' > bad.json
cloacinactl workflow run notify_oncall --context bad.json   # → 400
```
