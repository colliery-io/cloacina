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
cloacinactl trigger fire inbox_poll
```

`fire` runs everything the trigger drives — for an `on = "..."` trigger like
this one, that is its `on` workflow; for a trigger with
`#[workflow(triggers = ["t"])]` subscribers, it fans out to every subscriber
(and both at once, when both shapes are wired).

`cloacinactl workflow run file_processing` also works, with a subtly different
meaning: it runs *that workflow* directly, bypassing the trigger entirely.
`trigger fire` is "act as if the trigger fired now"; `workflow run` is "run
this workflow regardless of its trigger".
