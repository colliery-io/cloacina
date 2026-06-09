# cloacina-workflow::trigger <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Trigger types for workflow authoring.

T-0552 (I-0102 follow-up) relocated the `Trigger` trait from `cloacina`
(engine-only) into this leaf crate so packaged cdylibs can collect
`TriggerEntry` inventory entries (which hold `Arc<dyn Trigger>`) at
link time, and the unified `cloacina::package!()` shell macro can walk
them at FFI call time. Engine paths re-export `cloacina_workflow::Trigger`.

## Enums

### `cloacina-workflow::trigger::TriggerResult` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Result of a trigger poll operation.

#### Variants

- **`Skip`** - Do not fire the workflow, continue polling on the next interval.
- **`Fire`** - Fire the workflow with an optional context.



### `cloacina-workflow::trigger::TriggerError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors that can occur during trigger polling.

#### Variants

- **`PollError`** - Error during trigger polling
- **`ContextError`** - Context creation error
