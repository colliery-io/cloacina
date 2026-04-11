# cloacina::workflow::builder <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Workflow builder for fluent workflow construction.

This module provides the `WorkflowBuilder` struct for constructing
workflows using a chainable, fluent API.

## Structs

### `cloacina::workflow::builder::WorkflowBuilder`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Builder pattern for convenient and fluent Workflow construction.

The WorkflowBuilder provides a chainable interface for constructing Workflows,
making it easy to set metadata, add tasks, and validate the structure
before finalizing the Workflow.

**Examples:**

```rust,ignore
use cloacina::*;

# struct TestTask { id: String, deps: Vec<String> }
# impl TestTask { fn new(id: &str, deps: Vec<&str>) -> Self { Self { id: id.to_string(), deps: deps.into_iter().map(|s| s.to_string()).collect() } } }
# use async_trait::async_trait;
# #[async_trait]
# impl Task for TestTask {
#     async fn execute(&self, context: Context<serde_json::Value>) -> Result<Context<serde_json::Value>, TaskError> { Ok(context) }
#     fn id(&self) -> &str { &self.id }
#     fn dependencies(&self) -> &[String] { &self.deps }
# }
let workflow = Workflow::builder("etl-pipeline")
    .description("Customer data ETL pipeline")
    .tag("environment", "staging")
    .tag("owner", "data-team")
    .add_task(TestTask::new("extract_customers", vec![]))?
    .add_task(TestTask::new("validate_data", vec!["extract_customers"]))?
    .validate()?
    .build()?;

assert_eq!(workflow.name(), "etl-pipeline");
assert!(!workflow.metadata().version.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `workflow` | `Workflow` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (name : & str) -> Self
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



##### `name` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn name (& self) -> & str
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



##### `get_description` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_description (& self) -> Option < & str >
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



##### `get_tags` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_tags (& self) -> & std :: collections :: HashMap < String , String >
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



##### `description` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn description (mut self , description : & str) -> Self
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



##### `tenant` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn tenant (mut self , tenant : & str) -> Self
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



##### `tag` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn tag (mut self , key : & str , value : & str) -> Self
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



##### `add_task` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn add_task (mut self , task : Arc < dyn Task >) -> Result < Self , WorkflowError >
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



##### `validate` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn validate (self) -> Result < Self , ValidationError >
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



##### `build` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn build (self) -> Result < Workflow , ValidationError >
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
