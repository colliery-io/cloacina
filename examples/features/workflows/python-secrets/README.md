# Workflow Secrets (Python)

The Python peer of [`workflow-secrets`](../workflow-secrets): a workflow
**declares** the secrets it needs, a run **binds** each to a tenant secret, and
the value is **resolved at execution** through a side channel — never written
into the durable context, history, or logs.

```
resolve_token → send_notification
```

## How secrets are declared and consumed

```python
@cloaca.workflow_secrets("api_token")
@cloaca.workflow_params(channel=(str, "#ops"))
@cloaca.task(id="resolve_token", dependencies=[])
def resolve_token(context):
    token = context.secret_field("api_token", "token")   # side channel
    context.set("token_len", len(token))                 # derived facts only
    return context
```

`@cloaca.workflow_secrets(...)` surfaces each name as an **encrypted input
slot**. A run binds it with a reference — never a literal:

```json
{ "channel": "#oncall", "api_token": { "$secret": "oncall_api" } }
```

The server routes the reference away from the plaintext context, resolves the
value from the tenant's encrypted store at execution, and hands it to
`context.secret(...)` / `context.secret_field(...)`. A **literal** value for a
secret slot is rejected before anything runs.

## Run it

Automated as `angreal demos features python-secrets`. The server needs a
secrets KEK (`CLOACINA_SECRET_KEK`); the demo stack ships one.

```bash
angreal ui up
cloacinactl config profile set demo http://localhost:8080 \
    --api-key clk_demo_public_key_0003 --tenant public --default

# create the tenant secret (value from a file — never an argv literal)
printf 's3cr3t-demo-token' > /tmp/token.txt
cloacinactl secret create oncall_api --field token=@/tmp/token.txt

cloacinactl package pack . --out python-secrets.cloacina
cloacinactl package upload python-secrets.cloacina
cloacinactl package list   # wait for build_status: success

cat > bind.json <<'EOF'
{ "channel": "#oncall", "api_token": { "$secret": "oncall_api" } }
EOF
cloacinactl workflow run python_secrets --context bind.json
cloacinactl execution list --workflow python_secrets
```

Rotate and the next run sees the new value; `secret get` returns metadata only;
a literal binding (`{"api_token": "plaintext"}`) is rejected.
