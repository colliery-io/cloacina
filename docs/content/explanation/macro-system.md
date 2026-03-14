---
title: "Macro System"
description: "A technical deep dive into Cloacina's macro system and how it works"
date: 2024-03-19
weight: 4
reviewer: "dstorey"
review_date: "2024-03-19"
---

# Inside Cloacina's Macro System

## Introduction

Cloacina's macro system provides a powerful way to define and validate task pipelines at compile time. This article explores the core concepts and practical usage of the system.

The macro system is the preferred way to use Cloacina for several key reasons:

1. **Compile-Time Safety**: The macro system validates your entire pipeline during compilation, catching common errors like missing dependencies or circular references before your code runs.

2. **Type Safety**: By using the macro system, you get full type checking of your task inputs and outputs, ensuring data consistency across your pipeline.

3. **Automatic Registration**: Tasks and workflows are automatically registered in the global registry, eliminating the need for manual registration and reducing boilerplate code.

4. **Developer Experience**: The declarative syntax makes it easier to understand and maintain your pipelines, with clear visualization of dependencies and execution flow.

5. **Pipeline Versioning**: The system automatically generates and tracks versions for your workflows based on their content and dependencies, making it easier to manage changes and track pipeline evolution. For a detailed explanation of how versioning works, see [Workflow Versioning](workflow-versioning.md).

While Cloacina can be used without macros, the macro system provides these benefits with minimal additional complexity, making it the recommended approach for most use cases.

## Core Concepts

### Tasks and Workflows

At its heart, Cloacina's macro system revolves around two main concepts:

1. **Tasks**: Individual units of work that can be executed independently
2. **Workflows**: Compositions of tasks that define execution order and dependencies

The macro system ensures these concepts are properly structured and validated at compile time. It also handles automatic versioning of workflows based on their content and dependencies. For a detailed explanation of how versioning works, see [Workflow Versioning](workflow-versioning.md).

### Compile-Time Validation

The macro system performs several important validations:

- Task ID uniqueness
- Dependency existence
- Circular dependency detection
- Workflow structure validation

These validations happen during compilation, catching errors before runtime.

## The Task Macro

### How It Works

The `#[task]` macro transforms your async function into a fully-featured Cloacina task by:

1. **Code Generation**: Creates a task struct that implements the `Task` trait
2. **Registry Integration**: Registers the task in the compile-time registry during macro expansion
3. **Validation**: Performs compile-time validation of dependencies and configuration
4. **Fingerprinting**: Generates a unique code fingerprint for versioning

### Compile-Time Validation

The macro performs several validations during compilation:

1. **Dependency Existence**: Ensures all declared dependencies exist
2. **ID Uniqueness**: Verifies task IDs are unique within the codebase
3. **Configuration Validity**: Validates retry policies and other settings
4. **Type Safety**: Ensures context types match the task's requirements

These validations catch common errors before runtime, making your pipelines more reliable.

### Generated Code

Behind the scenes, the macro generates code that:

1. Implements the `Task` trait with proper error handling
2. Creates a task struct with the specified configuration
3. Registers the task in the compile-time registry during macro expansion
4. Implements retry logic based on the configuration

This generated code is what enables the compile-time safety and automatic registration features. Note that `ctor` is used by the `workflow!` macro (not the `#[task]` macro) for auto-registering workflows at application startup.

## The Workflow Macro

### How It Works

The `workflow!` macro creates a complete workflow implementation by:

1. **Task Validation**: Verifies all referenced tasks exist and are properly registered
2. **Graph Analysis**: Performs topological sorting and cycle detection
3. **Version Calculation**: Generates a content-based version for the workflow
4. **Registry Integration**: Registers the workflow in the global registry

### Generated Implementation

The macro generates code that:
1. Creates a workflow struct with the specified configuration
2. Implements proper task ordering and dependency management
3. Sets up automatic version calculation
4. Handles workflow registration and execution

## Implementation Details

### Required Dependencies

The macro system requires several dependencies to be explicitly included in your `Cargo.toml`:

```toml
[dependencies]
cloacina = { version = "0.1.0", features = ["macros"] }
async-trait = "0.1"    # Required for async task definitions
serde_json = "1.0"    # Required for context data
chrono = "0.4"        # Required for timestamps in errors and state tracking
ctor = "0.2"          # Required only when using the workflow! macro (for runtime workflow registration)
```

`async-trait`, `serde_json`, and `chrono` cannot be made transitive because they are referenced directly in the code generated by the `#[task]` macro. The generated code uses `#[async_trait::async_trait]`, `serde_json::Value`, and `chrono::Utc::now()` at the call site. `ctor` is only needed when using the `workflow!` macro, which uses `#[ctor::ctor]` for automatic workflow registration at startup.

### Registry System

The macro system uses a compile-time registry to track tasks and their dependencies. This registry is implemented as a global singleton using `once_cell` and `Mutex` for thread-safe access during compilation. It serves as the central source of truth for task relationships and validation.

```mermaid
graph TD
    A[Task Definition] -->|Register| B[Compile-Time Registry]
    B -->|Validate| C[Dependency Graph]
    C -->|Check| D[Cycle Detection]
    C -->|Verify| E[Task Existence]
    B -->|Auto-Register| F[Global Task Registry]

    subgraph "Compile-Time Phase"
        A
        B
        C
        D
        E
    end

    subgraph "Runtime Phase"
        F
    end
```

#### Key Components

1. **Task Registration**
   - Each task is registered with a unique ID
   - Dependencies are recorded in a directed graph
   - File paths are tracked for better error messages
   - Registration happens during macro expansion

2. **Dependency Management**
   - Maintains a directed acyclic graph (DAG) of task dependencies
   - Tracks both direct and transitive dependencies
   - Enables efficient dependency validation
   - Supports complex workflow structures

3. **Validation System**
   - **Cycle Detection**: Uses depth-first search (DFS) with a recursion stack to detect circular dependencies
   - **Task Existence**: Verifies all referenced tasks are registered
   - **ID Uniqueness**: Ensures no duplicate task IDs
   - **Dependency Completeness**: Validates all dependencies exist

4. **Error Handling**
   - Provides detailed error messages with file locations
   - Suggests similar task names for typo detection
   - Includes dependency paths in error messages
   - Supports test mode with relaxed validation


## TaskHandle Detection

The `#[task]` macro supports an optional second parameter for execution control. When a task function declares a parameter named `handle` or `task_handle`, the macro generates additional code for concurrency slot management.

### How Detection Works

During macro expansion, the macro inspects the function's parameter list:

```rust
// Handle detected — generates requires_handle() = true
#[task(id = "deferred_task")]
async fn deferred_task(
    context: &mut Context<Value>,
    handle: &mut TaskHandle,
) -> Result<(), TaskError> { ... }

// No handle — generates requires_handle() = false
#[task(id = "normal_task")]
async fn normal_task(
    context: &mut Context<Value>,
) -> Result<(), TaskError> { ... }
```

The detection is based purely on parameter **name** (`handle` or `task_handle`), not the type. This keeps the ergonomics simple — no extra attributes or traits required.

### Generated Code Differences

For handle-aware tasks, the generated `Task::execute()` implementation:

1. Calls `take_task_handle()` to retrieve the handle from task-local storage
2. Passes it as the second argument to the user function
3. Calls `return_task_handle()` after the function completes

```rust
// Generated for handle-aware task (simplified)
async fn execute(&self, mut context: Context<Value>) -> Result<Context<Value>, TaskError> {
    let mut handle = ::cloacina::take_task_handle();
    let result = deferred_task(&mut context, &mut handle).await;
    ::cloacina::return_task_handle(handle);
    // ... error handling
}

// Generated for standard task (simplified)
async fn execute(&self, mut context: Context<Value>) -> Result<Context<Value>, TaskError> {
    let result = normal_task(&mut context).await;
    // ... error handling
}
```

The generated `requires_handle()` method returns `true` for handle-aware tasks, which signals the executor to create and inject a `TaskHandle` via task-local storage before calling `execute()`.

### Python Equivalent

In the Python bindings, the same detection happens at runtime. The `TaskDecorator` inspects the decorated function's `__code__.co_varnames` to check if the second parameter is named `handle` or `task_handle`. See the [Task Handle Architecture]({{< ref "/explanation/task-handle-architecture/" >}}) for details on how the handle flows through the Python bridge.

## Conclusion

Cloacina's macro system provides a powerful way to build type-safe, validated task pipelines. By understanding the core concepts and following best practices, you can create robust and maintainable workflows that catch errors at compile time rather than runtime.
