# Packaged triggers

A packaged workflow that fires **itself**. No `workflow run`, no cron — a
trigger polls for work and starts the workflow when it finds some.

This is the packaged counterpart to `event-triggers` (which shows the same idea
through the embedded library API). Here the trigger ships inside the
`.cloacina` package: the `#[trigger]` macro projects it across the plugin FFI at
load time, and the reconciler registers it in the host's trigger registry.

## What it declares

```rust
#[trigger(on = "file_processing", poll_interval = "3s")]
pub async fn inbox_poll() -> Result<TriggerResult, TriggerError> {
    // ... look for work ...
    Ok(TriggerResult::Fire(Some(ctx)))   // Fire(ctx) starts the workflow
}
```

- **`on`** names the workflow this trigger starts.
- **`poll_interval`** is how often the host calls your function.
- Returning `TriggerResult::Fire(Some(ctx))` starts `file_processing` with that
  context. Returning no-fire simply does nothing this tick.

The context you attach is what the workflow's first task receives — that is how
a trigger passes "what it found" to the work.

## Run it

Bring up the demo stack, then pack and upload as usual:

```bash
cloacinactl package pack . -o packaged-triggers.cloacina
cloacinactl package upload packaged-triggers.cloacina
```

Once the build succeeds and the reconciler loads the package, **nothing else is
required** — the trigger begins polling on its own. Watch executions appear:

```bash
cloacinactl execution list --workflow file_processing
```

## Operate it

A trigger runs unattended, which is exactly why you need controls for it. These
are the verbs for a trigger that is already live.

### See what is registered

```bash
cloacinactl trigger list
cloacinactl trigger inspect inbox_poll
```

`inspect` shows the schedule plus its recent executions — the fastest way to
answer "is it actually firing, and what happened when it did?"

### Stop it firing

```bash
cloacinactl trigger pause inbox_poll
```

Pausing gates **new** executions only; anything already running is untouched.
It is deliberately distinct from disabling or deleting the trigger — the
registration stays intact, so this is the safe thing to reach for during an
incident.

```bash
cloacinactl trigger resume inbox_poll
```

Resuming re-arms it on its normal schedule. Fires missed while paused are **not
caught up** — the policy is skip, not backfill. If you paused for ten minutes on
a 3s poll, you do not get 200 executions on resume.

Both `pause` and `resume` accept the workflow name as well as the trigger name,
which is handy when you know what stopped rather than which trigger drives it.

### Run it now, without waiting for the poll

```bash
cloacinactl workflow run file_processing
```

For a trigger declared with `on = "..."` — like this one — that is the way to
get an immediate run. The trigger owns *when* the workflow runs; running the
workflow directly bypasses the schedule for a one-off.

`cloacinactl trigger fire` does **not** apply here, and it is worth knowing why,
because the two look interchangeable and are not:

| Shape | How it is wired | `trigger fire` |
|---|---|---|
| `#[trigger(on = "wf")]` (this example) | the trigger names the workflow it drives | **no** — the trigger has no *subscribers* |
| `#[workflow(triggers = ["t"])]` | workflows subscribe to a named trigger | yes — fans out to every subscriber |

`trigger fire` resolves its targets from the *subscription* side, so firing a
trigger whose only consumer is its own `on` workflow reports
`trigger '<name>' has no subscribed workflows`. See CLOACI-T-0929.
