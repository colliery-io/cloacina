# cloacina-workflow::trigger <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Trigger types for workflow authoring.

These types are used by `#[trigger]` macro-generated code.
The full `Trigger` trait lives in `cloacina` (runtime crate).

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
