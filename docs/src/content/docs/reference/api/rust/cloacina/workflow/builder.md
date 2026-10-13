---
title: "cloacina::workflow::builder"
description: "Workflow builder for fluent workflow construction."
editUrl: false
sidebar:
  order: 203
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Workflow builder for fluent workflow construction.

This module provides the `WorkflowBuilder` struct for constructing
workflows using a chainable, fluent API.

## Structs

### <span id="struct.WorkflowBuilder"></span><code style="font-size: 0.85em">struct WorkflowBuilder</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Builder pattern for convenient and fluent Workflow construction.

The WorkflowBuilder provides a chainable interface for constructing Workflows,
making it easy to set metadata, add tasks, and validate the structure
before finalizing the Workflow.

**Fields**

* `workflow`: Workflow - The workflow being constructed

**Implementation Details**

The builder pattern provides:
- Fluent interface for workflow construction
- Automatic validation during build
- Content-based version calculation
- Metadata management

**Examples:**

```rust
use cloacina::*;

let workflow = Workflow::builder("etl-pipeline")
    .description("Customer data ETL workflow")
    .tag("environment", "staging")
    .tag("owner", "data-team")
    .add_task(TestTask::new("extract_customers", vec![]))?
    .add_task(TestTask::new("validate_data", vec!["extract_customers"]))?
    .validate()?
    .build()?;

assert_eq!(workflow.name(), "etl-pipeline");
assert!(!workflow.metadata().version.is_empty());
```

#### Methods

##### <span id="WorkflowBuilder.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(name: &str) -> Self
```

Create a new workflow builder

<details>
<summary>Source</summary>

```rust
    pub fn new(name: &str) -> Self {
        Self {
            workflow: Workflow::new(name),
        }
    }
```

</details>

##### <span id="WorkflowBuilder.name"></span><code>name</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn name(&self) -> &str
```

Get the workflow name

<details>
<summary>Source</summary>

```rust
    pub fn name(&self) -> &str {
        self.workflow.name()
    }
```

</details>

##### <span id="WorkflowBuilder.get_description"></span><code>get_​description</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_description(&self) -> Option<&str>
```

Get the workflow description (if set).

<details>
<summary>Source</summary>

```rust
    pub fn get_description(&self) -> Option<&str> {
        self.workflow.metadata.description.as_deref()
    }
```

</details>

##### <span id="WorkflowBuilder.get_tags"></span><code>get_​tags</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_tags(&self) -> &std::collections::HashMap<String, String>
```

Get the workflow tags.

<details>
<summary>Source</summary>

```rust
    pub fn get_tags(&self) -> &std::collections::HashMap<String, String> {
        &self.workflow.metadata.tags
    }
```

</details>

##### <span id="WorkflowBuilder.description"></span><code>description</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn description(mut self, description: &str) -> Self
```

Set the workflow description

<details>
<summary>Source</summary>

```rust
    pub fn description(mut self, description: &str) -> Self {
        self.workflow.set_description(description);
        self
    }
```

</details>

##### <span id="WorkflowBuilder.tenant"></span><code>tenant</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn tenant(mut self, tenant: &str) -> Self
```

Set the workflow tenant

<details>
<summary>Source</summary>

```rust
    pub fn tenant(mut self, tenant: &str) -> Self {
        self.workflow.tenant = tenant.to_string();
        self
    }
```

</details>

##### <span id="WorkflowBuilder.tag"></span><code>tag</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn tag(mut self, key: &str, value: &str) -> Self
```

Add a tag to the workflow metadata

<details>
<summary>Source</summary>

```rust
    pub fn tag(mut self, key: &str, value: &str) -> Self {
        self.workflow.add_tag(key, value);
        self
    }
```

</details>

##### <span id="WorkflowBuilder.add_task"></span><code>add_​task</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn add_task(
    mut self,
    task: Arc<dyn Task>,
) -> Result<Self, WorkflowError>
```

Add a task to the workflow

<details>
<summary>Source</summary>

```rust
    pub fn add_task(mut self, task: Arc<dyn Task>) -> Result<Self, WorkflowError> {
        self.workflow.add_task(task)?;
        Ok(self)
    }
```

</details>

##### <span id="WorkflowBuilder.validate"></span><code>validate</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn validate(self) -> Result<Self, ValidationError>
```

Validate the workflow structure

<details>
<summary>Source</summary>

```rust
    pub fn validate(self) -> Result<Self, ValidationError> {
        self.workflow.validate()?;
        Ok(self)
    }
```

</details>

##### <span id="WorkflowBuilder.build"></span><code>build</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn build(self) -> Result<Workflow, ValidationError>
```

Build the final workflow with automatic version calculation

<details>
<summary>Source</summary>

```rust
    pub fn build(self) -> Result<Workflow, ValidationError> {
        self.workflow.validate()?;
        // Auto-calculate version when building
        Ok(self.workflow.finalize())
    }
```

</details>
