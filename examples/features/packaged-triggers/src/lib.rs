/*
 *  Copyright 2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

/*!
# Packaged Triggers Example

This example demonstrates how to create a distributable workflow package
that includes **trigger definitions**. Triggers are user-defined polling
functions that fire workflows when conditions are met.

## Packaged Triggers vs Library-API Triggers

The `event-triggers` example shows triggers used via the library API.
This example shows the same concept in a **packageable** form:

- Tasks and triggers are defined in a cdylib using `#[packaged_workflow]`
  and `#[trigger]` macros
- When the cdylib is loaded by the reconciler, triggers are automatically
  registered in the global trigger registry via `ctor`
- The `Manifest` manifest declares trigger metadata so the reconciler
  can track and manage them

## Manifest Trigger Fields

When creating a `.cloacina` package, triggers are declared in `manifest.json`:

```json
{
    "triggers": [
        {
            "name": "inbox_watcher",
            "trigger_type": "rust",
            "workflow": "file_pipeline",
            "poll_interval": "5s",
            "allow_concurrent": false,
            "config": { "path": "/data/inbox/" }
        }
    ]
}
```

### Field Reference

| Field             | Type              | Description |
|-------------------|-------------------|-------------|
| `name`            | `string`          | Unique trigger name within the package |
| `trigger_type`    | `string`          | Discriminator: `"rust"`, `"python"`, or any custom type |
| `workflow`        | `string`          | Workflow to fire (package name or task ID) |
| `poll_interval`   | `string`          | How often to poll: `"100ms"`, `"5s"`, `"2m"`, `"1h"` |
| `allow_concurrent`| `bool`            | Allow parallel executions with same context hash |
| `config`          | `object` (opt)    | Trigger-specific configuration (URL, path, etc.) |

## Usage

```bash
# Compile as a shared library
cargo build --release

# The resulting .so/.dylib is loaded by the reconciler at runtime
```
*/

use cloacina_workflow::{packaged_workflow, task, Context, TaskError};

/// File Processing Pipeline — triggered when new files arrive.
///
/// This package demonstrates a workflow that is designed to be fired
/// by a trigger rather than scheduled on a cron. The trigger polls
/// for new files and passes the filename via context.
#[packaged_workflow(
    package = "file_pipeline",
    name = "file_processing",
    description = "Process incoming files detected by trigger",
    author = "Platform Team <platform@company.com>"
)]
pub mod file_processing {
    use super::*;

    /// Validate the incoming file.
    ///
    /// The trigger passes `filename` and `source_path` in the context.
    #[task(
        id = "validate",
        dependencies = [],
        retry_attempts = 2,
        retry_backoff = "linear"
    )]
    pub async fn validate(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let filename = context
            .get("filename")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        println!("  Validating file: {}", filename);

        // Simulate validation
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        context.insert("validated", serde_json::json!(true))?;
        context.insert(
            "validated_at",
            serde_json::json!(chrono::Utc::now().to_rfc3339()),
        )?;

        println!("  File validated successfully");
        Ok(())
    }

    /// Transform and process the validated file.
    #[task(
        id = "transform",
        dependencies = ["validate"],
        retry_attempts = 3,
        retry_backoff = "exponential"
    )]
    pub async fn transform(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let filename = context
            .get("filename")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        println!("  Transforming file: {}", filename);

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        context.insert("records_processed", serde_json::json!(1500))?;
        context.insert(
            "transform_completed_at",
            serde_json::json!(chrono::Utc::now().to_rfc3339()),
        )?;

        println!("  Transformation complete: 1500 records processed");
        Ok(())
    }

    /// Archive the processed file.
    #[task(
        id = "archive",
        dependencies = ["transform"],
        retry_attempts = 1,
        retry_backoff = "fixed"
    )]
    pub async fn archive(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let filename = context
            .get("filename")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        println!("  Archiving file: {}", filename);

        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;

        context.insert("archived", serde_json::json!(true))?;
        println!("  File archived successfully");
        Ok(())
    }
}
