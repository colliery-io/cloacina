# Parameterized Python Workflow

The Python peer of [`parameterized-workflow`](../parameterized-workflow): one
workflow **template**, many differently-configured **runs**, through the primary
interface. Declared params are typed, validated by the server, and bound
per-run.

```
plan_sync → execute_sync → report
```

| Param | Type | Default |
|---|---|---|
| `source` | `str` | *(required)* |
| `dst` | `str` | *(required)* |
| `mode` | `str` | `"copy"` |
| `max_files` | `int` | `100` |

## How params are declared

```python
@cloaca.workflow_params(
    source=str,            # required
    dst=str,               # required
    mode=(str, "copy"),    # optional, default "copy"
    max_files=(int, 100),  # optional, default 100
)
@cloaca.task(id="plan_sync", dependencies=[])
def plan_sync(context):
    source = context.get("source")   # bound values arrive as context keys
    ...
```

A bare type is required; a `(type, default)` tuple is optional. The compiler
parses this from source into typed input slots; the server validates every
run's provided values against them and rejects wrong types / missing required
params **before** anything executes.

## Run it

Automated as `angreal demos features python-parameterized`.

### 1. Stack + CLI

```bash
angreal ui up
cloacinactl config profile set demo http://localhost:8080 \
    --api-key clk_demo_public_key_0003 --tenant public --default
```

### 2. Pack + upload

```bash
cloacinactl package pack . --out python-parameterized.cloacina
cloacinactl package upload python-parameterized.cloacina
cloacinactl package list   # wait for build_status: success
```

### 3. Run it with different values

```bash
echo '{"source": "/data/prod", "dst": "/backup/prod"}' > prod.json
cloacinactl workflow run python_parameterized --context prod.json

echo '{"source": "/data/archive", "dst": "/cold", "mode": "move", "max_files": 10}' > archive.json
cloacinactl workflow run python_parameterized --context archive.json

cloacinactl execution list --workflow python_parameterized
```

### 4. Validation rejects a bad run

```bash
echo '{"dst": "/backup"}' > bad.json      # missing required `source`
cloacinactl workflow run python_parameterized --context bad.json   # → 400
```
